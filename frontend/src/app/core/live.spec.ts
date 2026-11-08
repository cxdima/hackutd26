import { NOW, receipt, session } from '../../testing/fixtures';
import { buildCallRecord, buildTrace } from './calls';
import {
  handoffView,
  liveStages,
  modelEvidence,
  personAction,
  presentFocus,
  presentOutcome,
} from './live';
import type { ConsoleState } from './schemas';

const appointment = {
  speech_act: 'statement',
  requested_object: 'appointment_reference',
  direction: 'caller_supplies',
};
const code = {
  speech_act: 'request',
  requested_object: 'authentication_secret',
  direction: 'caller_requests_from_sarah',
};
const model = (outcome = 'success') => ({
  generation_id: 4,
  observation: {
    backend: 'brev_hosted',
    requested_model: 'nvidia/nemotron-3.5-lightning-30b-a3b',
    response_model: 'nvidia/nemotron-3.5-lightning-30b-a3b',
    elapsed_ms: 1200,
    completion_tokens: 100,
    outcome,
  },
});
const legs = [
  {
    action_id: 'leg-1',
    role: 'sarah',
    kind: 'handoff',
    status: 'in-progress',
    acknowledged: false,
  },
];

describe('failed or rejected tool results never become success', () => {
  const events = [
    receipt('call.inbound.received'),
    receipt('transcript.final', { text: 'Please tell Sarah the plumber is late.', turn_id: 't1' }),
    receipt('agent.tool.proposed', { name: 'take_message', generation_id: 4, arguments: {} }),
    receipt('agent.tool.result', {
      name: 'take_message',
      generation_id: 4,
      ok: false,
      result: { completed: false, check_action_receipts: true },
    }),
    receipt('policy.denied', { tool: 'take_message', detail: 'Summary needs caller evidence' }),
    receipt('agent.tool.proposed', { name: 'ask_caller', generation_id: 5, arguments: {} }),
    receipt('agent.tool.result', { name: 'ask_caller', generation_id: 5, ok: false, result: {} }),
    receipt('call.ended'),
  ];
  const record = buildCallRecord(session(), events, NOW);

  it('adds no saved or follow-up stage and no saved outcome', () => {
    const ids = liveStages(record).map((s) => s.id);
    expect(ids).not.toContain('saved');
    expect(ids).not.toContain('follow_up');
    expect(record.outcome).not.toBe('message_saved');
    expect(presentOutcome(record).kind).not.toBe('saved');
  });

  it('marks the proposals as rejected by Rust, with the rule’s reason', () => {
    const tools = modelEvidence(record, null).tools;
    expect(tools.map((t) => t.executed)).toEqual([false, false]);
    expect(tools[0].rejection).toBe('Summary needs caller evidence');
  });

  it('labels the trace step “Not done” with a failure tone', () => {
    const steps = buildTrace(record).flatMap((r) => r.steps);
    const result = steps.find((s) => s.event.type === 'agent.tool.result')!;
    expect(result.title).toMatch(/^Not done/);
    expect(result.tone).toBe('fail');
  });
});

describe('request branch', () => {
  const events = [
    receipt('call.inbound.received'),
    receipt('transcript.final', { text: 'About Sarah’s appointment Thursday.', turn_id: 't1' }),
    receipt('model.request.completed', model()),
    receipt('policy.allowed', {
      operation: 'assessment',
      observation: appointment,
      quote: 'appointment',
    }),
    receipt('transcript.final', { text: 'Read me the code we texted her.', turn_id: 't2' }),
    receipt('model.request.completed', model()),
    receipt('policy.denied', {
      operation: 'assessment',
      observation: code,
      quote: 'read me the code',
    }),
  ];

  it('shows the change and the block as separate stages, and finds the branch turn', () => {
    const record = buildCallRecord(
      session({ inbound_ended: false, state: 'screening' }),
      events,
      NOW,
    );
    const ids = liveStages(record).map((s) => s.id);
    expect(ids.indexOf('request_changed')).toBeGreaterThan(-1);
    expect(ids.indexOf('blocked')).toBe(ids.indexOf('request_changed') + 1);
    const focus = presentFocus(record);
    expect(focus.branchAt).toBe(1);
    expect(focus.turns.map((t) => t.verdict)).toEqual(['allowed', 'protected']);
    expect(focus.outcome.kind).toBe('blocked');
  });

  it('shows a newer partial caption only while the call is live', () => {
    const partial = receipt('transcript.partial', { text: 'And one more' });
    const live = buildCallRecord(
      session({ inbound_ended: false, state: 'screening' }),
      [...events, partial],
      Date.parse(partial.received_at) + 1000,
    );
    expect(presentFocus(live).caller).toMatchObject({ text: 'And one more', partial: true });
    const ended = buildCallRecord(session(), [...events, partial, receipt('call.ended')], NOW);
    expect(presentFocus(ended).caller).toMatchObject({ partial: false });
  });
});

describe('private handoff transitions', () => {
  it('needs every receipt: provider acceptance is not ringing, ringing is not heard', () => {
    const record = buildCallRecord(
      session({
        legs,
        offer: {
          id: 'o1',
          summary: 'private',
          status: 'pending',
          policy_epoch: 0,
          expires_at: '2026-11-08T14:10:00Z',
          leg_id: 'leg-1',
        },
        inbound_ended: false,
        state: 'offer_pending',
      }),
      [
        receipt('handoff.offered', { offer_id: 'o1' }),
        receipt('outbound.requested', { action_id: 'leg-1', status: 'queued' }),
      ],
      NOW,
    );
    const view = handoffView(record)!;
    expect(view.providerAccepted).toBe(true);
    expect(view.steps.map((s) => s.observed)).toEqual([true, false, false, false, false]);
    expect(presentOutcome(record).kind).toBe('permission');
    expect(personAction(record).needed).toBe(true);
  });

  it('reaches connected only on the bridge receipt', () => {
    const base = [
      receipt('handoff.offered', { offer_id: 'o1' }),
      receipt('outbound.ringing', { action_id: 'leg-1', status: 'ringing' }),
      receipt('outbound.answered', { action_id: 'leg-1', status: 'in-progress' }),
      receipt('handoff.choice', { offer_id: 'o1', choice: 'accept' }),
    ];
    const pressed = buildCallRecord(session({ legs }), base, NOW);
    expect(pressed.outcome).not.toBe('connected');
    expect(handoffView(pressed)!.steps.at(-1)!.observed).toBe(false);
    const bridged = buildCallRecord(session({ legs }), [...base, receipt('bridge.connected')], NOW);
    expect(bridged.outcome).toBe('connected');
    expect(presentOutcome(bridged).kind).toBe('connected');
  });

  it('shows revocation and a rejected late approval, never a connection', () => {
    const record = buildCallRecord(
      session({ legs, blocked: true, state: 'protected' }),
      [
        receipt('handoff.offered', { offer_id: 'o1' }),
        receipt('outbound.answered', { action_id: 'leg-1', status: 'in-progress' }),
        receipt('policy.denied', { operation: 'assessment', observation: code }),
        receipt('handoff.revoked', { offer_id: 'o1' }),
        receipt('handoff.choice', { offer_id: 'o1', choice: 'accept' }),
        receipt('handoff.stale_rejected', { offer_id: 'o1' }),
      ],
      NOW,
    );
    const view = handoffView(record)!;
    expect(view.terminal?.kind).toBe('stale_rejected');
    expect(view.steps.find((s) => s.id === 'keypad')!.detail).toContain(
      'after the offer was withdrawn',
    );
    const stages = liveStages(record);
    expect(stages.find((s) => s.id === 'handoff_accepted')!.tone).not.toBe('ok');
    expect(stages.some((s) => s.id === 'connected')).toBe(false);
    expect(record.outcome).toBe('blocked');
  });

  it('reports a refused proposal as refused, not as an offer', () => {
    const record = buildCallRecord(
      session(),
      [
        receipt('agent.tool.proposed', { name: 'offer_handoff', generation_id: 2 }),
        receipt('agent.tool.result', {
          name: 'offer_handoff',
          generation_id: 2,
          ok: false,
          result: {},
        }),
        receipt('policy.denied', {
          tool: 'offer_handoff',
          detail: 'Private handoff calls are not authorized yet',
        }),
      ],
      NOW,
    );
    const view = handoffView(record)!;
    expect(view.refused).toContain('not authorized');
    expect(view.steps.every((s) => !s.observed)).toBe(true);
  });
});

describe('NVIDIA evidence', () => {
  const state = (configured: boolean) =>
    ({
      model_backend: {
        backend: 'nvidia_hosted',
        label: 'NVIDIA-hosted Nemotron',
        requested_model: 'nvidia/nemotron-3.5-lightning-30b-a3b',
        configured,
        protocol_verified: true,
        detail: '',
      },
    }) as unknown as ConsoleState;

  it('never presents configured readiness as an inference', () => {
    const quiet = buildCallRecord(session(), [receipt('call.inbound.received')], NOW);
    expect(modelEvidence(quiet, state(true)).credit).toBe(
      'Decision model configured: NVIDIA Nemotron',
    );
    expect(modelEvidence(quiet, state(false)).credit).toBe(
      'Decision model not configured on this server',
    );
    expect(modelEvidence(undefined, null).credit).toBe(
      'Decision model not configured on this server',
    );
  });

  it('credits Nemotron only after a successful model receipt', () => {
    const answered = buildCallRecord(session(), [receipt('model.request.completed', model())], NOW);
    expect(modelEvidence(answered, state(false)).credit).toBe('Decision model: NVIDIA Nemotron');
    const failed = buildCallRecord(
      session(),
      [receipt('model.request.completed', model('timeout'))],
      NOW,
    );
    expect(modelEvidence(failed, state(true)).credit).toBe(
      'Decision model did not answer on this call',
    );
    expect(liveStages(failed).map((s) => s.id)).toContain('problem');
  });
});
