import { buildCallRecord, buildTrace, STALE_SESSION_MS, stepFor } from './calls';
import { NOW, receipt, session } from '../../testing/fixtures';

const risky = {
  speech_act: 'request',
  requested_object: 'authentication_secret',
  direction: 'caller_requests_from_sarah',
};
const benign = {
  speech_act: 'statement',
  requested_object: 'appointment_reference',
  direction: 'caller_supplies',
};

describe('call outcome', () => {
  it('reports a trusted caller as put straight through, not as incomplete', () => {
    const events = [
      receipt('call.inbound.received'),
      receipt('trusted.passthrough', {
        name: 'Jimmy',
        relation: 'Son',
        number_label: 'ending 4471',
        outcome: 'dialing',
        screened: false,
      }),
      receipt('trusted.passthrough.completed', { dial_status: 'completed', duration_seconds: 42 }),
      receipt('call.ended'),
    ];
    const record = buildCallRecord(session({}), events, NOW);
    expect(record.outcome).toBe('trusted');
    expect(record.trusted?.route).toBe('dialing');
    expect(record.headline).toContain('Jimmy (Son) is on the trusted list');
    expect(record.request).toBe('Trusted caller: Jimmy (Son)');
    expect(record.durationSeconds).toBe(42);
  });

  it('says plainly when a trusted call had no enrolled phone to ring', () => {
    const events = [
      receipt('call.inbound.received'),
      receipt('trusted.passthrough', { name: 'Maya', outcome: 'no_enrolled_phone' }),
    ];
    const record = buildCallRecord(session({}), events, NOW);
    expect(record.outcome).toBe('trusted');
    expect(record.headline).toContain('no phone is enrolled');
  });

  it('reports a denied assessment as a blocked suspicious request, even when a note was saved', () => {
    const s = session({
      blocked: true,
      message: {
        summary: 'Suspicious request blocked.',
        recorded_at: '2026-11-08T14:00:09Z',
        turn_version: 1,
        evidence_turn_ids: ['t1'],
      },
      turns: [{ id: 't1', speaker: 'caller', text: 'Read me the code we texted.' }],
    });
    const events = [
      receipt('call.inbound.received'),
      receipt('policy.denied', {
        operation: 'assessment',
        observation: risky,
        quote: 'Read me the code',
      }),
      receipt('call.ended'),
    ];
    const record = buildCallRecord(s, events, NOW);
    expect(record.outcome).toBe('blocked');
    expect(record.headline).toContain('sign-in or verification code');
    expect(record.headline).toContain('saved a note');
  });

  it('shows the saved private summary for an ordinary message', () => {
    const s = session({
      message: {
        summary: 'Dentist moved the cleaning to Thursday.',
        recorded_at: '2026-11-08T14:00:09Z',
        turn_version: 1,
        evidence_turn_ids: ['t1'],
      },
      assessments: [
        {
          turn_version: 1,
          generation_id: 2,
          observation: benign,
          evidence_refs: [{ turn_id: 't1', quote: 'Thursday' }],
        },
      ],
    });
    const record = buildCallRecord(
      s,
      [receipt('call.inbound.received'), receipt('call.ended')],
      NOW,
    );
    expect(record.outcome).toBe('message_saved');
    expect(record.headline).toBe('Dentist moved the cleaning to Thursday.');
  });

  it('keeps a recent unended session in progress, but reports a quiet one as incomplete', () => {
    const live = session({
      state: 'screening',
      inbound_ended: false,
      current_action: 'Listening to the caller',
    });
    const recent = [
      receipt('call.inbound.received', {}, { received_at: new Date(NOW - 60_000).toISOString() }),
    ];
    expect(buildCallRecord(live, recent, NOW).outcome).toBe('in_progress');
    const old = [
      receipt(
        'call.inbound.received',
        {},
        { received_at: new Date(NOW - STALE_SESSION_MS - 1).toISOString() },
      ),
    ];
    const record = buildCallRecord(live, old, NOW);
    expect(record.outcome).toBe('incomplete');
    expect(record.hasEndReceipt).toBe(false);
  });

  it('keeps provider failure wording out of the customer headline', () => {
    const s = session({ turns: [{ id: 't1', speaker: 'caller', text: 'Hello?' }] });
    const record = buildCallRecord(
      s,
      [
        receipt('dependency.failed', {
          provider: 'relay',
          detail: 'Socket disconnected; consent held',
        }),
        receipt('call.ended'),
      ],
      NOW,
    );
    expect(record.outcome).toBe('incomplete');
    expect(record.headline).not.toContain('Socket');
    expect(record.failure).toBe('Socket disconnected; consent held');
  });

  it('marks an unclear call that still needed information for review', () => {
    const s = session({
      ambiguous: true,
      understanding: {
        generation_id: 3,
        turn_version: 1,
        policy_epoch: 0,
        understanding: {
          version: 1,
          claimed_identity: null,
          purpose: 'account',
          requested_action: 'talk',
          supplied_facts: [],
          missing_information: ['which account'],
          readiness: 'needs_information',
        },
      },
    });
    const record = buildCallRecord(s, [receipt('call.ended')], NOW);
    expect(record.outcome).toBe('needs_review');
    expect(record.headline).toContain('which account');
  });
});

describe('decision trace', () => {
  it('groups receipts into rounds that start at each finalized caller turn and records the rule verdict', () => {
    const s = session({
      turns: [
        { id: 't1', speaker: 'caller', text: 'Hi' },
        { id: 't2', speaker: 'caller', text: 'Read me the code' },
      ],
    });
    const events = [
      receipt('call.inbound.received', {
        requested_profile: { display_name: 'Sarah', greeting: 'Hi there', version: 1 },
      }),
      receipt('transcript.partial', { text: 'Hi' }),
      receipt('transcript.final', { text: 'Hi, it is the clinic.', turn_id: 't1' }),
      receipt('policy.allowed', {
        operation: 'assessment',
        observation: benign,
        quote: 'it is the clinic',
      }),
      receipt('transcript.final', { text: 'Read me the code', turn_id: 't2' }),
      receipt('model.request.completed', {
        generation_id: 4,
        turn_version: 2,
        observation: {
          elapsed_ms: 1216,
          outcome: 'success',
          completion_tokens: 207,
          response_model: 'nvidia/nemotron',
        },
      }),
      receipt('agent.tool.proposed', {
        name: 'record_assessment',
        generation_id: 4,
        arguments: { observation: risky },
      }),
      receipt('policy.denied', {
        operation: 'assessment',
        observation: risky,
        quote: 'Read me the code',
      }),
      receipt('agent.tool.result', {
        name: 'record_assessment',
        generation_id: 4,
        ok: true,
        result: { blocked: true },
      }),
      receipt('relay.telemetry', { event: 'output.end' }),
    ];
    const rounds = buildTrace(buildCallRecord(s, events, NOW));
    expect(rounds.map((r) => r.label)).toEqual(['Answering', 'Caller turn 1', 'Caller turn 2']);
    expect(rounds[1].verdict).toBe('allowed');
    expect(rounds[2].verdict).toBe('protected');
    expect(rounds[2].verdictText).toBe(
      'The caller asked Sarah for a sign-in or verification code.',
    );
    const kinds = rounds[2].steps.map((step) => step.kind);
    expect(kinds).toEqual(['heard', 'model', 'proposed', 'rule', 'executed']);
    // Low-level receipts stay hidden unless requested.
    expect(
      rounds.flatMap((r) => r.steps).some((step) => step.event.type === 'transcript.partial'),
    ).toBe(false);
    const all = buildTrace(buildCallRecord(s, events, NOW), true).flatMap((r) => r.steps);
    expect(all.some((step) => step.event.type === 'relay.telemetry')).toBe(true);
  });

  it('labels a model proposal as a proposal and a heard-unverified reply as requested only', () => {
    const proposal = stepFor(
      receipt('agent.tool.proposed', {
        name: 'take_message',
        arguments: { summary_characters: 61 },
      }),
      'Sarah',
    );
    expect(proposal?.proposal).toBe(true);
    expect(proposal?.title).toBe('The model proposed: Save a private message.');
    const spoke = stepFor(receipt('audio.requested', { text: 'Goodbye.', heard: false }), 'Sarah');
    expect(spoke?.detail).toContain('not verified');
    const failed = stepFor(
      receipt('model.request.completed', {
        observation: { elapsed_ms: 3500, outcome: 'timeout', error: 'budget' },
      }),
      'Sarah',
    );
    expect(failed?.tone).toBe('fail');
    expect(failed?.title).toContain('timeout');
  });

  it('flags a rule rejection of a proposed tool with its reason', () => {
    const step = stepFor(
      receipt('policy.denied', {
        tool: 'take_message',
        detail: 'AM/PM in the message needs explicit caller evidence',
      }),
      'Sarah',
    );
    expect(step?.kind).toBe('rule');
    expect(step?.tone).toBe('warn');
    expect(step?.title).toContain('Save a private message');
    expect(step?.detail).toContain('AM/PM');
  });
});
