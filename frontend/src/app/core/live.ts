import { buildTrace, type CallRecord } from './calls';
import { describeObservation, isProtectedRequest, toolLabel } from './language';
import {
  modelObservationSchema,
  observationSchema,
  type CallEvent,
  type ConsoleState,
  type Session,
} from './schemas';

/**
 * The presentation layer's reading of a call. Every stage, line and outcome
 * here comes from a server receipt. A stage the receipts do not prove is never
 * shown: no "ringing" without an outbound.ringing receipt, no "connected"
 * without bridge.connected, and a failed or rejected proposal adds no stage.
 */

export type StageId =
  | 'answered'
  | 'speaking'
  | 'checking'
  | 'held'
  | 'follow_up'
  | 'request_changed'
  | 'blocked'
  | 'saved'
  | 'handoff_offered'
  | 'handoff_calling'
  | 'handoff_ringing'
  | 'handoff_answered'
  | 'handoff_accepted'
  | 'handoff_declined'
  | 'connected'
  | 'handoff_withdrawn'
  | 'stale_rejected'
  | 'handoff_ended'
  | 'handoff_unknown'
  | 'problem'
  | 'ended';

export type StageTone = 'neutral' | 'active' | 'ok' | 'protect' | 'warn' | 'fail';

export interface Stage {
  id: StageId;
  label: string;
  detail?: string;
  tone: StageTone;
  at: string;
  sequence: number;
  event: CallEvent;
}

function str(value: unknown): string | undefined {
  return typeof value === 'string' && value.length ? value : undefined;
}

/** Action ids of the private-handoff phone legs, from the snapshot (not from the model). */
export function handoffLegIds(session: Session): Set<string> {
  const ids = new Set<string>();
  for (const leg of session.legs) if (leg.kind === 'handoff') ids.add(leg.action_id);
  for (const offer of [session.offer, ...(session.offer_history ?? [])])
    if (offer?.leg_id) ids.add(offer.leg_id);
  return ids;
}

/** Ordered, evidence-backed stages. Consecutive repeats collapse into one stage. */
export function liveStages(record: CallRecord): Stage[] {
  const who = record.name;
  const legs = handoffLegIds(record.session);
  const stages: Stage[] = [];
  let lastVerdict: 'allowed' | 'protected' | undefined;
  let bridged = false;
  const staleOffers = new Set(
    record.events
      .filter((e) => e.type === 'handoff.stale_rejected')
      .map((e) => String(e.payload['offer_id'])),
  );
  const push = (event: CallEvent, id: StageId, label: string, tone: StageTone, detail?: string) => {
    const last = stages.at(-1);
    if (last && last.id === id) {
      // Keep the first receipt's time but the latest wording.
      last.label = label;
      last.detail = detail ?? last.detail;
      return;
    }
    stages.push({
      id,
      label,
      tone,
      detail,
      at: event.occurred_at,
      sequence: event.sequence,
      event,
    });
  };

  for (const event of record.events) {
    const p = event.payload;
    const leg = str(p['action_id']);
    const isHandoffLeg = !!leg && legs.has(leg);
    switch (event.type) {
      case 'call.inbound.received':
        push(event, 'answered', 'FirstRing answered', 'neutral', 'Caller ID is a clue, not proof.');
        break;
      case 'transcript.partial':
      case 'transcript.final':
        push(event, 'speaking', 'Caller speaking', 'active');
        break;
      case 'model.request.completed': {
        const obs = modelObservationSchema.safeParse(p['observation']);
        if (obs.success && obs.data.outcome !== 'success')
          push(
            event,
            'problem',
            'Decision model did not answer',
            'warn',
            `Outcome: ${obs.data.outcome.replaceAll('_', ' ')}. FirstRing keeps its safe fallback.`,
          );
        else push(event, 'checking', 'Checking the request', 'active');
        break;
      }
      case 'agent.tool.proposed':
        if (stages.at(-1)?.id === 'speaking' || stages.at(-1)?.id === 'answered')
          push(event, 'checking', 'Checking the request', 'active');
        break;
      case 'policy.allowed':
      case 'policy.denied': {
        if (p['operation'] !== 'assessment') break;
        const obs = observationSchema.safeParse(p['observation']);
        const protectedReq =
          event.type === 'policy.denied' && obs.success && isProtectedRequest(obs.data);
        if (protectedReq) {
          if (lastVerdict === 'allowed')
            push(
              event,
              'request_changed',
              'The request changed',
              'protect',
              obs.success ? describeObservation(obs.data, who) : undefined,
            );
          push(
            event,
            'blocked',
            'Suspicious request blocked',
            'protect',
            'FirstRing will not share codes, payments or device access.',
          );
          lastVerdict = 'protected';
        } else if (event.type === 'policy.denied') {
          push(
            event,
            'held',
            'Held for safety',
            'warn',
            'The request was unclear, so FirstRing keeps asking.',
          );
        } else {
          if (lastVerdict === undefined) lastVerdict = 'allowed';
          push(event, 'checking', 'Checking the request', 'active');
        }
        break;
      }
      case 'agent.tool.result': {
        // Only an ok result can add a stage. A failed result is a rejected proposal.
        if (p['ok'] !== true) break;
        if (p['name'] === 'ask_caller')
          push(event, 'follow_up', 'Asking a follow-up', 'active', 'One short question.');
        break;
      }
      case 'message.recorded':
        push(event, 'saved', 'Message saved', 'ok', `Saved privately for ${who}.`);
        break;
      case 'handoff.offered':
        push(
          event,
          'handoff_offered',
          'Permission requested',
          'active',
          `FirstRing is privately asking ${who} whether to connect.`,
        );
        break;
      case 'outbound.requested':
        if (isHandoffLeg)
          push(
            event,
            'handoff_calling',
            `Calling ${who}’s phone`,
            'active',
            'Requested from the phone provider. Not ringing yet.',
          );
        break;
      case 'outbound.ringing':
        if (isHandoffLeg) push(event, 'handoff_ringing', `${who}’s phone is ringing`, 'active');
        break;
      case 'outbound.answered':
        if (isHandoffLeg)
          push(
            event,
            'handoff_answered',
            `${who} picked up`,
            'active',
            'The private summary plays on answer. Only a keypad choice confirms someone is there.',
          );
        break;
      case 'outbound.unknown':
        if (isHandoffLeg)
          push(
            event,
            'handoff_unknown',
            'Waiting for the phone provider',
            'warn',
            'Outcome unknown. FirstRing does not retry until it is reconciled.',
          );
        break;
      case 'handoff.choice':
        if (p['choice'] === 'accept' && staleOffers.has(String(p['offer_id'])))
          push(
            event,
            'handoff_accepted',
            `${who} pressed 1 after the offer was withdrawn`,
            'neutral',
            'Not connected.',
          );
        else if (p['choice'] === 'accept')
          push(event, 'handoff_accepted', `${who} pressed 1`, 'ok', 'Connecting.');
        else push(event, 'handoff_declined', `${who} declined`, 'neutral', 'No connection made.');
        break;
      case 'bridge.connected':
        bridged = true;
        push(
          event,
          'connected',
          `Connected to ${who}`,
          'ok',
          'Both people joined the call. Screening ended.',
        );
        break;
      case 'handoff.revoked':
        push(
          event,
          'handoff_withdrawn',
          'Permission withdrawn',
          'protect',
          'The request changed, so the pending offer was revoked.',
        );
        break;
      case 'handoff.stale_rejected':
        push(
          event,
          'stale_rejected',
          'Old keypad approval rejected',
          'protect',
          'A late “1” cannot revive a withdrawn offer.',
        );
        break;
      case 'outbound.ended':
        if (isHandoffLeg && !bridged)
          push(event, 'handoff_ended', `Call to ${who} ended`, 'neutral');
        break;
      case 'dependency.failed':
        push(
          event,
          'problem',
          `A service problem: ${providerLabel(str(p['provider']))}`,
          'fail',
          'Nothing was shared. Details are in the evidence drawer.',
        );
        break;
      case 'model.failure.injected':
        push(event, 'problem', 'Deliberate test failure', 'warn', 'Injected by a designated test.');
        break;
      case 'call.ended':
        push(event, 'ended', 'Call ended', 'neutral');
        break;
    }
  }
  return stages;
}

export function providerLabel(provider: string | undefined): string {
  switch (provider) {
    case 'nemotron':
      return 'decision model';
    case 'relay':
      return 'voice connection';
    case 'twilio':
      return 'phone provider';
    case 'fixture_injection':
      return 'injected test';
    default:
      return provider ?? 'unknown service';
  }
}

/* ------------------------------------------------------------------------ */
/* What is on screen now                                                     */
/* ------------------------------------------------------------------------ */

export type PresentOutcomeKind =
  | 'connected'
  | 'blocked'
  | 'permission'
  | 'saved'
  | 'declined'
  | 'withdrawn'
  | 'none'
  | 'ended'
  | 'incomplete';

export interface PresentOutcome {
  kind: PresentOutcomeKind;
  label: string;
  detail: string;
}

export interface CaptionLine {
  text: string;
  partial: boolean;
  at: string;
}

export interface TurnVerdict {
  index: number;
  text: string;
  verdict: 'protected' | 'allowed' | 'unclear';
  summary?: string;
}

export interface PresentFocus {
  caller?: CaptionLine;
  firstring?: CaptionLine;
  request: string;
  step?: Stage;
  outcome: PresentOutcome;
  turns: TurnVerdict[];
  /** Index into turns where the request switched from ordinary to protected. */
  branchAt?: number;
  live: boolean;
}

export function presentFocus(record: CallRecord): PresentFocus {
  const events = record.events;
  let finalLine: CaptionLine | undefined;
  let finalSeq = -1;
  let partialLine: CaptionLine | undefined;
  let partialSeq = -1;
  let reply: CaptionLine | undefined;
  for (const event of events) {
    const text = str(event.payload['text']);
    if (!text) continue;
    if (event.type === 'transcript.final') {
      finalLine = { text, partial: false, at: event.occurred_at };
      finalSeq = event.sequence;
    } else if (event.type === 'transcript.partial') {
      partialLine = { text, partial: true, at: event.occurred_at };
      partialSeq = event.sequence;
    } else if (event.type === 'audio.requested') {
      reply = { text, partial: false, at: event.occurred_at };
    }
  }
  const live = record.active;
  // A partial caption only leads while the call is live and it is newer than the final.
  const caller = live && partialLine && partialSeq > finalSeq ? partialLine : finalLine;
  const stages = liveStages(record);
  const rounds = buildTrace(record).filter((round) => round.callerText);
  const turns: TurnVerdict[] = rounds.map((round, index) => ({
    index,
    text: round.callerText ?? '',
    verdict: round.verdict ?? 'unclear',
    summary: round.verdictText,
  }));
  const firstProtected = turns.findIndex((turn) => turn.verdict === 'protected');
  const branchAt =
    firstProtected > 0 && turns.slice(0, firstProtected).some((t) => t.verdict === 'allowed')
      ? firstProtected
      : undefined;
  // After the call, the lower third keeps the decisive step rather than "Call ended".
  const settled = live
    ? stages.at(-1)
    : ([...stages]
        .reverse()
        .find(
          (s) => !['ended', 'handoff_ended', 'stale_rejected', 'handoff_accepted'].includes(s.id),
        ) ?? stages.at(-1));
  return {
    caller,
    firstring: reply,
    request: record.request,
    step: settled,
    outcome: presentOutcome(record, stages),
    turns,
    branchAt,
    live,
  };
}

export function presentOutcome(record: CallRecord, stages = liveStages(record)): PresentOutcome {
  const who = record.name;
  const has = (id: StageId) => stages.some((stage) => stage.id === id);
  const lastOf = (...ids: StageId[]) => [...stages].reverse().find((s) => ids.includes(s.id))?.id;
  if (has('connected'))
    return {
      kind: 'connected',
      label: `Connected to ${who}`,
      detail: `${who} heard a private summary, pressed 1, and both people joined.`,
    };
  if (record.outcome === 'blocked')
    return {
      kind: 'blocked',
      label: 'Suspicious request blocked',
      detail: record.session.message
        ? `Nothing was shared. A private note was saved for ${who}.`
        : 'Nothing was shared.',
    };
  const handoff = lastOf(
    'handoff_offered',
    'handoff_calling',
    'handoff_ringing',
    'handoff_answered',
    'handoff_accepted',
    'handoff_declined',
    'handoff_withdrawn',
    'stale_rejected',
    'handoff_ended',
    'handoff_unknown',
  );
  if (handoff === 'handoff_withdrawn' || handoff === 'stale_rejected')
    return {
      kind: 'withdrawn',
      label: 'Permission withdrawn',
      detail: 'The request changed, so the private offer was revoked.',
    };
  if (handoff === 'handoff_declined')
    return {
      kind: 'declined',
      label: `${who} declined`,
      detail: record.session.message
        ? 'A private message was saved instead.'
        : 'No connection made.',
    };
  if (
    record.active &&
    handoff &&
    handoff !== 'handoff_ended' &&
    record.session.offer?.status !== 'expired'
  )
    return {
      kind: 'permission',
      label: 'Permission requested',
      detail: `Waiting for ${who} to choose on their own phone.`,
    };
  if (record.outcome === 'message_saved' || has('saved'))
    return {
      kind: 'saved',
      label: 'Message saved',
      detail: `A private summary is waiting for ${who}.`,
    };
  if (record.outcome === 'in_progress')
    return { kind: 'none', label: 'Screening in progress', detail: 'No outcome yet.' };
  if (record.outcome === 'incomplete')
    return {
      kind: 'incomplete',
      label: 'Call ended early',
      detail: 'The call ended before FirstRing could finish. Nothing was shared.',
    };
  return { kind: 'ended', label: 'Call ended', detail: record.headline };
}

/* ------------------------------------------------------------------------ */
/* NVIDIA / model evidence                                                   */
/* ------------------------------------------------------------------------ */

export interface ToolTrail {
  name: string;
  label: string;
  generation?: number;
  /** undefined: no result receipt yet; true: Rust executed it; false: Rust rejected it. */
  executed?: boolean;
  rejection?: string;
  at: string;
}

export interface ModelEvidence {
  configuredLabel?: string;
  configured: boolean;
  protocolVerified: boolean;
  requestedModel?: string;
  requests: number;
  succeeded: number;
  failed: number;
  lastMs?: number;
  medianMs?: number;
  backends: string[];
  models: string[];
  tools: ToolTrail[];
  allowed: number;
  denied: number;
  /** A one-line, truthful credit for the main stage. */
  credit: string;
  /** Authored demo receipts: no model was called. */
  demo: boolean;
}

export function backendLabel(backend: string | undefined): string | undefined {
  if (backend === 'brev_hosted') return 'Brev GPU';
  if (backend === 'nvidia_hosted') return 'NVIDIA-hosted';
  return backend;
}

export function isNemotron(model: string | undefined | null): boolean {
  return !!model && model.toLowerCase().includes('nemotron');
}

export function modelEvidence(
  record: CallRecord | undefined,
  state: ConsoleState | null,
): ModelEvidence {
  const backend = state?.model_backend;
  const times: number[] = [];
  const backends = new Set<string>();
  const models = new Set<string>();
  let succeeded = 0;
  let failed = 0;
  let allowed = 0;
  let denied = 0;
  const tools: ToolTrail[] = [];
  const open = new Map<string, ToolTrail>();
  for (const event of record?.events ?? []) {
    const p = event.payload;
    if (event.type === 'model.request.completed') {
      const obs = modelObservationSchema.safeParse(p['observation']);
      if (!obs.success) continue;
      times.push(obs.data.elapsed_ms);
      if (obs.data.outcome === 'success') succeeded++;
      else failed++;
      const label = backendLabel(obs.data.backend ?? undefined);
      if (label) backends.add(label);
      const model = obs.data.response_model ?? obs.data.requested_model;
      if (model) models.add(model);
    } else if (event.type === 'agent.tool.proposed') {
      const name = str(p['name']) ?? 'unknown';
      const trail: ToolTrail = {
        name,
        label: toolLabel(name),
        generation: typeof p['generation_id'] === 'number' ? p['generation_id'] : undefined,
        at: event.occurred_at,
      };
      tools.push(trail);
      open.set(`${name}#${String(p['generation_id'])}`, trail);
    } else if (event.type === 'agent.tool.result') {
      const key = `${String(p['name'])}#${String(p['generation_id'])}`;
      const trail = open.get(key);
      if (trail) {
        trail.executed = p['ok'] === true;
        open.delete(key);
      }
    } else if (event.type === 'policy.allowed') allowed++;
    else if (event.type === 'policy.denied') {
      denied++;
      const tool = str(p['tool']);
      if (tool) {
        const trail = [...tools].reverse().find((t) => t.name === tool && t.executed === false);
        if (trail) trail.rejection = str(p['detail']);
      }
    }
  }
  const sorted = [...times].sort((a, b) => a - b);
  const median = sorted.length ? sorted[Math.floor((sorted.length - 1) / 2)] : undefined;
  const requested = backend?.requested_model;
  const nemotron = [...models].some(isNemotron) || isNemotron(requested);
  const demo =
    state?.data_origin === 'demo_fixture' ||
    (record?.events.some((e) => e.data_origin === 'demo_fixture') ?? false);
  let credit: string;
  if (demo) credit = 'Demo data: no model was called';
  else if (succeeded > 0)
    credit = `Decision model: ${nemotron ? 'NVIDIA Nemotron' : [...models][0]}`;
  else if (record && failed > 0) credit = 'Decision model did not answer on this call';
  else if (backend?.configured)
    credit = `Decision model configured: ${nemotron ? 'NVIDIA Nemotron' : backend.label}`;
  else credit = 'Decision model not configured on this server';
  return {
    configuredLabel: backend?.label,
    configured: backend?.configured ?? false,
    protocolVerified: backend?.protocol_verified ?? false,
    requestedModel: requested,
    requests: times.length,
    succeeded,
    failed,
    lastMs: times.at(-1),
    medianMs: median,
    backends: [...backends],
    models: [...models],
    tools,
    allowed,
    denied,
    credit,
    demo,
  };
}

/* ------------------------------------------------------------------------ */
/* Private handoff                                                           */
/* ------------------------------------------------------------------------ */

export type HandoffStepId = 'offered' | 'ringing' | 'summary' | 'withdrawn' | 'keypad' | 'bridge';

export interface HandoffStep {
  id: HandoffStepId;
  label: string;
  detail: string;
  observed: boolean;
  /** ok: it happened as intended; protect: a withdrawal or rejection; neutral: e.g. declined. */
  tone: 'ok' | 'protect' | 'neutral';
  at?: string;
}

export interface HandoffTerminal {
  kind: 'declined' | 'withdrawn' | 'stale_rejected' | 'expired' | 'failed' | 'ended';
  label: string;
  detail: string;
  at?: string;
}

export interface HandoffView {
  /** The model proposed a handoff (a proposal, not an offer). */
  proposed: boolean;
  /** Rust refused the proposal (for example, handoff calls are not authorized). */
  refused?: string;
  offerStatus?: string;
  providerAccepted: boolean;
  steps: HandoffStep[];
  terminal?: HandoffTerminal;
}

const FAILED_LEG = ['failed', 'busy', 'no-answer', 'canceled'];

/** The handoff as receipts prove it, or undefined when no handoff was ever proposed. */
export function handoffView(record: CallRecord): HandoffView | undefined {
  const who = record.name;
  const legs = handoffLegIds(record.session);
  const first = (pred: (e: CallEvent) => boolean) => record.events.find(pred);
  const last = (pred: (e: CallEvent) => boolean) => [...record.events].reverse().find(pred);
  const onLeg = (type: string) => (e: CallEvent) =>
    e.type === type && legs.has(String(e.payload['action_id']));

  const proposal = first(
    (e) => e.type === 'agent.tool.proposed' && e.payload['name'] === 'offer_handoff',
  );
  const offered = first((e) => e.type === 'handoff.offered');
  if (!proposal && !offered && !legs.size) return undefined;
  const refusal = last((e) => e.type === 'policy.denied' && e.payload['tool'] === 'offer_handoff');
  const requested = first(onLeg('outbound.requested'));
  const ringing = first(onLeg('outbound.ringing'));
  const answered = first(onLeg('outbound.answered'));
  const choice = last((e) => e.type === 'handoff.choice');
  const bridge = first((e) => e.type === 'bridge.connected');
  const stale = last((e) => e.type === 'handoff.stale_rejected');
  const revoked = last((e) => e.type === 'handoff.revoked');
  const declined = !!choice && choice.payload['choice'] !== 'accept';
  const steps: HandoffStep[] = [
    {
      id: 'offered',
      label: 'Permission requested',
      detail: offered
        ? `Rust created a private offer for ${who}. The caller stays on hold with FirstRing.`
        : 'No offer receipt. A model proposal alone does not create an offer.',
      observed: !!offered,
      tone: 'ok',
      at: offered?.occurred_at,
    },
    {
      id: 'ringing',
      label: `${who}’s phone rang`,
      detail: ringing
        ? 'The phone provider reported ringing.'
        : requested
          ? 'The provider accepted the call request; no ringing receipt yet.'
          : 'No ringing receipt.',
      observed: !!ringing,
      tone: 'ok',
      at: ringing?.occurred_at,
    },
    {
      id: 'summary',
      label: 'Private summary requested',
      detail: answered
        ? 'The phone answered and FirstRing requested the private summary. Hearing it is not verified.'
        : 'No answer receipt.',
      observed: !!answered,
      tone: 'ok',
      at: answered?.occurred_at,
    },
    {
      id: 'keypad',
      label: stale
        ? 'Pressed 1 after permission was withdrawn: rejected'
        : declined
          ? `${who} declined`
          : 'Keypad response received',
      detail: choice
        ? declined
          ? `${who} declined or did not choose.`
          : stale
            ? `${who} pressed 1 after the offer was withdrawn. The old approval was rejected.`
            : `${who} pressed 1 to connect.`
        : 'No keypad receipt.',
      observed: !!choice,
      tone: stale ? 'protect' : declined ? 'neutral' : 'ok',
      at: choice?.occurred_at,
    },
    {
      id: 'bridge',
      label: 'Both people connected',
      detail: bridge
        ? 'Both intended participants joined. Screening ended.'
        : 'No connection receipt.',
      observed: !!bridge,
      tone: 'ok',
      at: bridge?.occurred_at,
    },
  ];
  if (revoked) {
    // The withdrawal sits in time order among the observed steps.
    const at = Date.parse(revoked.occurred_at);
    const index = steps.findIndex((s) => !s.observed || Date.parse(s.at ?? '') > at);
    steps.splice(index < 0 ? steps.length : index, 0, {
      id: 'withdrawn',
      label: 'Permission withdrawn',
      detail: 'The caller’s request changed, so the pending offer was revoked.',
      observed: true,
      tone: 'protect',
      at: revoked.occurred_at,
    });
  }

  let terminal: HandoffTerminal | undefined;
  const failedEvent = last(
    (e) => e.type === 'dependency.failed' && legs.has(String(e.payload['action_id'])),
  );
  const failedLeg = record.session.legs.find(
    (l) => legs.has(l.action_id) && FAILED_LEG.includes(l.status),
  );
  const offerStatus = record.session.offer?.status;
  if (bridge) terminal = undefined;
  else if (stale)
    terminal = {
      kind: 'stale_rejected',
      label: 'Old keypad approval rejected',
      detail: 'A “1” pressed after the offer was withdrawn cannot reconnect the call.',
      at: stale.occurred_at,
    };
  else if (revoked)
    terminal = {
      kind: 'withdrawn',
      label: 'Permission withdrawn',
      detail: 'The caller’s request changed, so the pending offer was revoked.',
      at: revoked.occurred_at,
    };
  else if (choice && choice.payload['choice'] !== 'accept')
    terminal = {
      kind: 'declined',
      label: `${who} declined`,
      detail: 'No connection made.',
      at: choice.occurred_at,
    };
  else if (offerStatus === 'expired')
    terminal = {
      kind: 'expired',
      label: 'Offer expired',
      detail: 'No valid choice arrived before the offer expired.',
    };
  else if (failedEvent || failedLeg)
    terminal = {
      kind: 'failed',
      label: 'Call to the phone failed',
      detail: failedLeg
        ? `Provider status: ${failedLeg.status.replaceAll('-', ' ')}.`
        : 'The phone provider rejected the request.',
      at: failedEvent?.occurred_at,
    };
  else if (last(onLeg('outbound.ended')))
    terminal = {
      kind: 'ended',
      label: `Call to ${who} ended`,
      detail: 'Ended without a connection.',
      at: last(onLeg('outbound.ended'))?.occurred_at,
    };

  return {
    proposed: !!proposal,
    refused:
      refusal && !offered ? (str(refusal.payload['detail']) ?? 'Rust refused it.') : undefined,
    offerStatus,
    providerAccepted: !!requested,
    steps,
    terminal,
  };
}

/* ------------------------------------------------------------------------ */
/* Does the person need to do anything?                                      */
/* ------------------------------------------------------------------------ */

export function personAction(record: CallRecord): { needed: boolean; text: string } {
  const who = record.name;
  const focus = presentOutcome(record);
  if (focus.kind === 'permission')
    return {
      needed: true,
      text: `Yes: ${who}’s phone may ring. Press 1 to connect or 2 to decline.`,
    };
  switch (record.outcome) {
    case 'in_progress':
      return { needed: false, text: 'Not yet. FirstRing is still talking with the caller.' };
    case 'blocked':
      return {
        needed: false,
        text: 'No. Nothing was shared. If you are worried, call the organization at a number you already know.',
      };
    case 'connected':
      return { needed: false, text: `No. ${who} already chose to talk with the caller.` };
    case 'trusted':
      return {
        needed: false,
        text: 'No. The number is on the trusted list, so the call rang straight through.',
      };
    case 'message_saved':
      return {
        needed: true,
        text: 'Read the message when convenient. Call back only at a number you already know.',
      };
    case 'needs_review':
      return {
        needed: true,
        text: 'Have a look. FirstRing could not tell what the caller needed.',
      };
    default:
      return { needed: false, text: 'No. Nothing was shared.' };
  }
}

const AVAILABILITY_REASON: Record<string, string> = {
  model_not_configured: 'not configured on the server',
  current_native_cycle_unverified: 'this deployment’s tool-call cycle is not verified yet',
  model_endpoint_unavailable: 'the model endpoint is not reachable',
  deployment_deadline_passed: 'the GPU deployment window has ended',
  current_model_loaded_and_native_cycle_verified: 'model loaded and its tool-call cycle verified',
};

/** The server's live availability check, when it reports one. Configured is not available. */
export function modelAvailability(state: ConsoleState | null): {
  known: boolean;
  available: boolean;
  text: string;
} {
  const backend = state?.model_backend;
  const current = backend?.current;
  if (!backend) return { known: false, available: false, text: 'Not reported by the server' };
  if (!current)
    return {
      known: false,
      available: false,
      text: backend.configured
        ? `Configured: ${backend.label}. The server does not report live availability.`
        : 'Not configured on the server right now',
    };
  const reason = current.reason
    ? (AVAILABILITY_REASON[current.reason] ?? current.reason.replaceAll('_', ' '))
    : '';
  return {
    known: true,
    available: current.available,
    text: current.available
      ? `Available now: ${backend.label}${reason ? ` (${reason})` : ''}`
      : `Not available now${reason ? `: ${reason}` : ''}`,
  };
}
