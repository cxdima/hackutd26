import { displayName } from './evidence';
import { describeObservation, isProtectedRequest, OBJECT_LABEL, toolLabel } from './language';
import {
  modelObservationSchema,
  observationSchema,
  type CallEvent,
  type ConsoleState,
  type Observation,
  type Session,
} from './schemas';

/**
 * Builds customer-facing call records and the full decision trace from the
 * server's session snapshot plus its receipt events. Everything shown is derived
 * from those receipts; a proposal is never presented as a completed action.
 */

export type Outcome =
  'in_progress' | 'blocked' | 'message_saved' | 'needs_review' | 'no_message' | 'incomplete';

export const OUTCOME_LABEL: Record<Outcome, string> = {
  in_progress: 'On the line now',
  blocked: 'Suspicious request blocked',
  message_saved: 'Message saved',
  needs_review: 'Needs review',
  no_message: 'Ended, no message',
  incomplete: 'Incomplete call',
};

export const OUTCOME_CHIP: Record<Outcome, string> = {
  in_progress: 'chip-live',
  blocked: 'chip-blocked',
  message_saved: 'chip-saved',
  needs_review: 'chip-review',
  no_message: 'chip-none',
  incomplete: 'chip-incomplete',
};

/** A session with no new receipt for this long and no end receipt is reported as incomplete. */
export const STALE_SESSION_MS = 15 * 60 * 1000;

export interface CallRecord {
  id: string;
  session: Session;
  events: CallEvent[];
  name: string;
  timezone?: string;
  startedAt?: string;
  endedAt?: string;
  lastActivityAt?: string;
  durationSeconds?: number;
  outcome: Outcome;
  headline: string;
  request: string;
  callerTurns: number;
  protectedRequest?: Observation;
  failure?: string;
  hasEndReceipt: boolean;
}

function eventsBySession(events: readonly CallEvent[]): Map<string, CallEvent[]> {
  const map = new Map<string, CallEvent[]>();
  for (const event of events) {
    if (!event.session_id) continue;
    const list = map.get(event.session_id);
    if (list) list.push(event);
    else map.set(event.session_id, [event]);
  }
  for (const list of map.values()) list.sort((a, b) => a.sequence - b.sequence);
  return map;
}

export function buildCallRecords(
  state: ConsoleState,
  events: readonly CallEvent[],
  now: number,
): CallRecord[] {
  const grouped = eventsBySession(events);
  const records = state.sessions.map((session) =>
    buildCallRecord(session, grouped.get(session.id) ?? [], now, state),
  );
  return records.sort((a, b) => sortKey(b) - sortKey(a));
}

function sortKey(record: CallRecord): number {
  const time = Date.parse(record.startedAt ?? record.lastActivityAt ?? '');
  return Number.isNaN(time) ? 0 : time;
}

function lastObservation(session: Session, events: CallEvent[]): Observation | undefined {
  const latest = session.assessments?.at(-1)?.observation;
  if (latest) return latest;
  for (let i = events.length - 1; i >= 0; i--) {
    const parsed = observationSchema.safeParse(events[i].payload['observation']);
    if (parsed.success) return parsed.data;
  }
  return undefined;
}

export function buildCallRecord(
  session: Session,
  events: CallEvent[],
  now: number,
  state?: ConsoleState,
): CallRecord {
  const name = session.profile?.display_name ?? displayName(session.household_id);
  const timezone =
    session.profile?.timezone ??
    (state?.profile?.household_id === session.household_id ? state.profile.timezone : undefined);
  const start = events.find((e) => e.type === 'call.inbound.received') ?? events[0];
  const end = [...events].reverse().find((e) => e.type === 'call.ended');
  const relayEnded = [...events].reverse().find((e) => e.type === 'relay.ended');
  const last = events.at(-1);
  const relayDuration = relayEnded?.payload['duration_seconds'];
  const durationSeconds =
    typeof relayDuration === 'number'
      ? relayDuration
      : start && end
        ? (Date.parse(end.occurred_at) - Date.parse(start.occurred_at)) / 1000
        : undefined;

  const deniedAssessment = [...events]
    .reverse()
    .find((e) => e.type === 'policy.denied' && e.payload['operation'] === 'assessment');
  const deniedObservation = observationSchema.safeParse(deniedAssessment?.payload['observation']);
  const assessmentProtected = session.assessments?.find((a) =>
    isProtectedRequest(a.observation),
  )?.observation;
  const protectedRequest = deniedObservation.success ? deniedObservation.data : assessmentProtected;
  const blocked =
    session.blocked === true ||
    !!deniedAssessment ||
    session.state === 'protected' ||
    !!assessmentProtected;

  const failureEvent = [...events].reverse().find((e) => e.type === 'dependency.failed');
  const failure =
    typeof failureEvent?.payload['detail'] === 'string'
      ? (failureEvent.payload['detail'] as string)
      : undefined;
  const lastActivity = last?.received_at;
  const ended = session.inbound_ended || session.state === 'ended' || !!end;
  const quiet = lastActivity ? now - Date.parse(lastActivity) > STALE_SESSION_MS : true;
  const understanding = session.understanding?.understanding;
  const callerTurns = session.turns.filter((t) => t.speaker === 'caller').length;

  let outcome: Outcome;
  if (blocked) outcome = 'blocked';
  else if (!ended && !quiet) outcome = 'in_progress';
  else if (session.message) outcome = 'message_saved';
  else if (session.ambiguous || understanding?.readiness === 'needs_information')
    outcome = 'needs_review';
  else if (
    understanding?.readiness === 'no_message' ||
    understanding?.readiness === 'caller_declined'
  )
    outcome = 'no_message';
  else outcome = 'incomplete';

  const observation = protectedRequest ?? lastObservation(session, events);
  let headline: string;
  switch (outcome) {
    case 'blocked':
      headline = `Asked for ${OBJECT_LABEL[protectedRequest?.requested_object ?? 'unknown']}. FirstRing declined${session.message ? ' and saved a note for you' : ''}.`;
      break;
    case 'message_saved':
      headline = session.message?.summary ?? 'Message saved.';
      break;
    case 'in_progress':
      headline = session.current_action;
      break;
    case 'needs_review':
      headline = understanding?.missing_information.length
        ? `FirstRing still needed: ${understanding.missing_information.join('; ')}.`
        : 'FirstRing could not confirm what the caller needed.';
      break;
    case 'no_message':
      headline =
        understanding?.readiness === 'caller_declined'
          ? 'The caller chose not to leave details.'
          : 'The caller did not need to leave a message.';
      break;
    default:
      headline = failure
        ? 'The call ended before FirstRing could finish. Nothing was shared.'
        : callerTurns
          ? 'No message was saved before the call ended.'
          : 'The caller did not say anything FirstRing could act on.';
  }

  const request = understanding?.requested_action
    ? understanding.requested_action
    : observation
      ? describeObservation(observation, name)
      : 'No request was heard.';

  return {
    id: session.id,
    session,
    events,
    name,
    timezone,
    startedAt: start?.occurred_at,
    endedAt: end?.occurred_at,
    lastActivityAt: lastActivity,
    durationSeconds,
    outcome,
    headline,
    request,
    callerTurns,
    protectedRequest,
    failure,
    hasEndReceipt: !!end,
  };
}

/* ------------------------------------------------------------------------ */
/* Decision trace                                                            */
/* ------------------------------------------------------------------------ */

export type StepKind =
  | 'call'
  | 'heard'
  | 'model'
  | 'proposed'
  | 'rule'
  | 'understood'
  | 'executed'
  | 'spoke'
  | 'saved'
  | 'community'
  | 'phone'
  | 'superseded'
  | 'failure'
  | 'closing'
  | 'ended'
  | 'raw';

export type StepTone = 'neutral' | 'ok' | 'protect' | 'warn' | 'fail';

export interface TraceStep {
  id: string;
  sequence: number;
  at: string;
  kind: StepKind;
  station: string;
  title: string;
  detail?: string;
  quote?: string;
  facts?: { label: string; value: string }[];
  tone: StepTone;
  /** True for a model proposal that Rust has not (yet) executed. */
  proposal?: boolean;
  event: CallEvent;
}

export interface TraceRound {
  index: number;
  label: string;
  callerText?: string;
  turnId?: string;
  at?: string;
  verdict?: 'protected' | 'allowed' | 'unclear';
  verdictText?: string;
  steps: TraceStep[];
}

export const NOISE_EVENTS = new Set([
  'transcript.partial',
  'relay.telemetry',
  'session.updated',
  'prompt.partial',
]);

const STATION: Record<StepKind, string> = {
  call: 'Call',
  heard: 'Heard',
  model: 'Model',
  proposed: 'Proposed',
  rule: 'Rule',
  understood: 'Understood',
  executed: 'Done',
  spoke: 'Said',
  saved: 'Saved',
  community: 'Community',
  phone: 'Phone',
  superseded: 'Discarded',
  failure: 'Problem',
  closing: 'Closing',
  ended: 'Ended',
  raw: 'Event',
};

function str(value: unknown): string | undefined {
  return typeof value === 'string' && value.length ? value : undefined;
}
function num(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined;
}

const ARG_LABEL: Record<string, string> = {
  purpose: 'Purpose',
  question_characters: 'Question length',
  summary_characters: 'Summary length',
  contact_id: 'Enrolled contact',
};
const VALUE_LABEL: Record<string, string> = {
  clarify_request: 'Clarify the request',
  missing_detail: 'Ask for a missing detail',
  confirm_detail: 'Confirm a detail',
};
function sentence(key: string): string {
  const text = key.replaceAll('_', ' ');
  return text.charAt(0).toUpperCase() + text.slice(1);
}
function plainValue(value: unknown, key = ''): string {
  if (typeof value === 'string') return VALUE_LABEL[value] ?? sentence(value);
  if (typeof value === 'number' && key.endsWith('_characters')) return `${value} characters`;
  return String(value);
}

function describeArguments(
  name: string,
  args: Record<string, unknown>,
  who: string,
): { detail?: string; facts: { label: string; value: string }[] } {
  const facts: { label: string; value: string }[] = [];
  if (name === 'record_assessment') {
    const parsed = observationSchema.safeParse(args['observation']);
    if (parsed.success) {
      facts.push(
        { label: 'Speech act', value: parsed.data.speech_act },
        { label: 'Requested', value: OBJECT_LABEL[parsed.data.requested_object] },
        {
          label: 'Direction',
          value: parsed.data.direction.replaceAll('_', ' ').replace('sarah', who),
        },
      );
      return { detail: describeObservation(parsed.data, who), facts };
    }
  }
  for (const [key, value] of Object.entries(args)) {
    if (value === null || value === undefined) continue;
    facts.push({
      label: ARG_LABEL[key] ?? sentence(key),
      value: typeof value === 'object' ? JSON.stringify(value) : plainValue(value, key),
    });
  }
  if (name === 'ask_caller' && typeof args['purpose'] === 'string')
    return {
      detail: `To ${plainValue(args['purpose']).toLowerCase()}. The question’s words are in the reply below.`,
      facts,
    };
  if (name === 'take_message' && typeof args['summary_characters'] === 'number')
    return {
      detail: `A ${args['summary_characters']}-character private summary (the text itself stays private to the saved message).`,
      facts,
    };
  return { facts };
}

function describeResult(name: string, result: Record<string, unknown>): string {
  if (name === 'record_assessment') {
    if (result['blocked'] === true)
      return 'Rust accepted the assessment and closed the protected request. No code, payment or access will be shared.';
    if (result['ambiguous'] === true)
      return 'Rust accepted the assessment as unclear. FirstRing keeps asking before anything else happens.';
    return 'Rust accepted the assessment. Ordinary help could continue.';
  }
  if (result['message_recorded'] === true)
    return 'Private message saved for review. This is not a delivery receipt.';
  if (result['audio_requested'] === true)
    return 'Reply queued for the caller. Hearing it is not verified.';
  if (result['acknowledged'] === true) return 'Contact acknowledgement recorded.';
  if (name === 'offer_handoff')
    return 'Private offer created. Phone receipts are shown separately.';
  if (name === 'start_trusted_callback')
    return 'Independent contact request accepted. No acknowledgement implied.';
  if (name === 'lookup_tactics') return 'Shared tactics looked up. A match needs its own receipt.';
  if (name === 'end_call') return 'Call ending requested.';
  return 'Completed. No phone outcome implied.';
}

export function stepFor(event: CallEvent, who: string): TraceStep | undefined {
  const p = event.payload;
  const base = { id: event.id, sequence: event.sequence, at: event.occurred_at, event };
  const make = (
    kind: StepKind,
    title: string,
    tone: StepTone,
    extra: Partial<TraceStep> = {},
  ): TraceStep => ({
    ...base,
    kind,
    station: STATION[kind],
    title,
    tone,
    ...extra,
  });
  switch (event.type) {
    case 'call.inbound.received': {
      const profile = (p['requested_profile'] ?? {}) as Record<string, unknown>;
      return make('call', 'An unfamiliar call arrived. FirstRing answered first.', 'neutral', {
        detail: 'Caller identity is unverified. Caller ID is a clue, not proof.',
        quote: str(profile['greeting']),
        facts: [
          {
            label: 'Profile',
            value: `${str(profile['display_name']) ?? who} · version ${num(profile['version']) ?? '—'}`,
          },
          { label: 'Transport', value: str(p['transport']) ?? '—' },
        ],
      });
    }
    case 'voice.connected': {
      const speech = (p['speech'] ?? {}) as Record<string, unknown>;
      return make('call', 'Conversation connected.', 'neutral', {
        facts: [
          {
            label: 'Voice',
            value: `${str(speech['provider']) ?? '—'} · ${str(speech['voice']) ?? '—'}`,
          },
          {
            label: 'Transcription',
            value:
              `${str(speech['transcription_provider']) ?? '—'} ${str(speech['transcription_model']) ?? ''}`.trim(),
          },
        ],
      });
    }
    case 'transcript.final':
      return make('heard', 'FirstRing heard the caller say:', 'neutral', { quote: str(p['text']) });
    case 'transport.keypad':
      return make(
        'heard',
        `Keypad input received${str(p['digits']) ? `: ${str(p['digits'])}` : ''}.`,
        'neutral',
      );
    case 'model.request.completed': {
      const obs = modelObservationSchema.safeParse(p['observation']);
      if (!obs.success)
        return make('model', 'Model request finished (details not readable).', 'warn');
      const o = obs.data;
      const ok = o.outcome === 'success';
      const seconds = (o.elapsed_ms / 1000).toFixed(2);
      return make(
        'model',
        ok
          ? `Nemotron answered in ${seconds} s.`
          : `Model request failed: ${o.outcome.replaceAll('_', ' ')}.`,
        ok ? 'neutral' : 'fail',
        {
          detail: ok ? undefined : (o.error ?? undefined),
          facts: [
            { label: 'Model', value: o.response_model ?? o.requested_model ?? '—' },
            {
              label: 'Backend',
              value:
                o.backend === 'brev_hosted'
                  ? 'Brev GPU'
                  : o.backend === 'nvidia_hosted'
                    ? 'NVIDIA hosted'
                    : '—',
            },
            {
              label: 'Tokens',
              value: o.completion_tokens != null ? String(o.completion_tokens) : '—',
            },
            { label: 'Finish', value: o.finish_reason ?? '—' },
            { label: 'Generation', value: String(num(p['generation_id']) ?? '—') },
          ],
        },
      );
    }
    case 'agent.tool.proposed': {
      const name = str(p['name']) ?? 'unknown';
      const args = (p['arguments'] ?? {}) as Record<string, unknown>;
      const described = describeArguments(name, args, who);
      return make('proposed', `The model proposed: ${toolLabel(name)}.`, 'neutral', {
        detail: described.detail,
        facts: described.facts,
        proposal: true,
      });
    }
    case 'policy.denied':
    case 'policy.allowed': {
      const allowed = event.type === 'policy.allowed';
      if (p['operation'] === 'assessment') {
        const obs = observationSchema.safeParse(p['observation']);
        const protectedReq = obs.success && isProtectedRequest(obs.data);
        return make(
          'rule',
          allowed
            ? 'Rule check: nothing protected was requested, so ordinary help continued.'
            : protectedReq
              ? 'Rule check: protected request. FirstRing will not share codes, payments or device access.'
              : 'Rule check: held for safety.',
          allowed ? 'ok' : 'protect',
          {
            detail: obs.success ? describeObservation(obs.data, who) : undefined,
            quote: str(p['quote']),
            facts: [
              {
                label: 'Private handoff allowed',
                value: p['handoff_allowed'] === true ? 'yes' : 'no',
              },
            ],
          },
        );
      }
      const tool = str(p['tool']);
      return make(
        'rule',
        allowed
          ? `Rule check passed${tool ? ` for “${toolLabel(tool)}”` : ''}.`
          : `Rule check rejected${tool ? ` “${toolLabel(tool)}”` : ' the proposal'}.`,
        allowed ? 'ok' : 'warn',
        {
          detail: str(p['detail']),
        },
      );
    }
    case 'call.understanding.accepted': {
      const missing = Array.isArray(p['missing_information'])
        ? (p['missing_information'] as unknown[]).map(String)
        : [];
      const readiness = str(p['readiness']);
      return make('understood', 'Understanding accepted.', 'neutral', {
        facts: [
          { label: 'Purpose (model’s words)', value: str(p['purpose']) ?? '—' },
          { label: 'Request (model’s words)', value: str(p['requested_action']) ?? '—' },
          {
            label: 'Message ready?',
            value:
              readiness === 'ready'
                ? 'Yes, enough to save'
                : readiness === 'needs_information'
                  ? 'Needs more detail'
                  : readiness === 'caller_declined'
                    ? 'Caller declined to give details'
                    : readiness === 'no_message'
                      ? 'No message wanted'
                      : '—',
          },
          ...(missing.length ? [{ label: 'Still missing', value: missing.join('; ') }] : []),
        ],
      });
    }
    case 'agent.tool.result': {
      const name = str(p['name']) ?? 'unknown';
      const ok = p['ok'] === true;
      const result = (p['result'] ?? {}) as Record<string, unknown>;
      return make(
        'executed',
        ok ? `Done: ${toolLabel(name)}.` : `Rust reported a failure: ${toolLabel(name)}.`,
        ok ? (result['blocked'] === true ? 'protect' : 'ok') : 'fail',
        {
          detail: ok ? describeResult(name, result) : str(p['error']),
        },
      );
    }
    case 'audio.requested':
      return make('spoke', 'FirstRing replied:', 'neutral', {
        quote: str(p['text']),
        detail:
          p['heard'] === false
            ? 'Requested for playback. Whether the caller heard it is not verified.'
            : undefined,
      });
    case 'audio.prepared':
      return make(
        'spoke',
        `Voice audio prepared${p['cache_hit'] === true ? ' from cache' : ''}.`,
        'neutral',
        {
          facts: [
            {
              label: 'Provider',
              value: `${str(p['provider']) ?? '—'} ${str(p['model_id']) ?? ''}`.trim(),
            },
            { label: 'Phrase', value: str(p['phrase']) ?? '—' },
          ],
        },
      );
    case 'audio.fallback':
      return make('spoke', 'Backup voice used for this reply.', 'warn', {
        detail: str(p['detail']),
      });
    case 'message.recorded':
      return make('saved', 'Private message saved for review.', 'ok', {
        detail:
          'Saved after a successful durable write. It is not a delivery receipt and never becomes community data.',
      });
    case 'tactic.matched':
      return make('community', 'A similar shared tactic was found.', 'protect', {
        detail: 'Matched on the requested action and pretext, not on a person or phone number.',
      });
    case 'handoff.offered':
      return make('phone', 'A private handoff was offered.', 'neutral');
    case 'handoff.revoked':
      return make('phone', 'The offer was withdrawn because the request changed.', 'protect');
    case 'handoff.stale_rejected':
      return make(
        'phone',
        'An old acceptance was rejected; the withdrawn offer stays closed.',
        'protect',
      );
    case 'handoff.choice':
      return make('phone', 'Private keypad choice received.', 'neutral');
    case 'action.reserved':
      return make('phone', 'A phone action was reserved before any provider request.', 'neutral');
    case 'outbound.requested':
      return make('phone', 'Outbound call requested (not yet ringing).', 'neutral');
    case 'outbound.ringing':
      return make('phone', 'Phone ringing.', 'neutral');
    case 'outbound.answered':
      return make('phone', 'Phone answered.', 'ok');
    case 'outbound.unknown':
      return make('phone', 'Awaiting provider confirmation before any retry.', 'warn');
    case 'outbound.ended':
      return make('phone', 'Outbound call ended.', 'neutral');
    case 'contact.acknowledged':
      return make('phone', 'Contact acknowledged by keypad.', 'ok');
    case 'bridge.connected':
      return make('phone', 'Both intended participants joined.', 'ok');
    case 'model.cancelled':
      return make('superseded', 'Model work discarded because the caller spoke again.', 'warn', {
        detail: `Generation ${num(p['generation_id']) ?? '—'} was superseded; its results cannot be used.`,
      });
    case 'output.discarded':
      return make('superseded', 'A stale reply was discarded before playback.', 'warn');
    case 'speech.recovery.requested':
      return make(
        'superseded',
        'Speech was interrupted; FirstRing asked the caller to repeat.',
        'warn',
      );
    case 'dependency.failed':
      return make(
        'failure',
        `A dependency failed: ${str(p['provider']) ?? 'unknown provider'}.`,
        'fail',
        { detail: str(p['detail']) },
      );
    case 'model.failure.injected':
      return make('failure', 'Deliberately injected test failure (synthetic).', 'warn', {
        detail: 'Designated fixture: zero provider requests were made.',
      });
    case 'call.closing':
    case 'closing.listen.configured':
      return make('closing', 'Closing: listening briefly for a correction.', 'neutral');
    case 'closing.interrupted':
      return make(
        'closing',
        'The caller spoke during the goodbye; the saved message was kept.',
        'neutral',
      );
    case 'closing.completed':
      return make('closing', 'Goodbye finished.', 'neutral');
    case 'relay.ended':
      return make(
        'ended',
        `Voice connection ended${num(p['duration_seconds']) !== undefined ? ` after ${num(p['duration_seconds'])} s` : ''}.`,
        'neutral',
      );
    case 'call.ended':
      return make('ended', 'Call ended.', 'neutral');
    case 'tactic.created':
      return make('community', 'A minimized tactic capsule was shared.', 'neutral');
    default:
      return make('raw', event.type, 'neutral');
  }
}

/** Groups receipts into rounds that start at each finalized caller turn. */
export function buildTrace(record: CallRecord, includeNoise = false): TraceRound[] {
  const rounds: TraceRound[] = [{ index: 0, label: 'Answering', steps: [] }];
  for (const event of record.events) {
    if (!includeNoise && NOISE_EVENTS.has(event.type)) continue;
    const step = stepFor(event, record.name);
    if (!step) continue;
    if (event.type === 'transcript.final') {
      rounds.push({
        index: rounds.length,
        label: `Caller turn ${rounds.length}`,
        callerText: step.quote,
        turnId: str(event.payload['turn_id']),
        at: event.occurred_at,
        steps: [step],
      });
      continue;
    }
    rounds[rounds.length - 1].steps.push(step);
  }
  for (const round of rounds) {
    const rule = [...round.steps]
      .reverse()
      .find((s) => s.kind === 'rule' && s.event.payload['operation'] === 'assessment');
    if (rule) {
      round.verdict = rule.tone === 'protect' ? 'protected' : 'allowed';
      round.verdictText = rule.detail;
    } else if (round.callerText) {
      round.verdict = 'unclear';
      round.verdictText = 'No accepted assessment was received for this turn.';
    }
  }
  return rounds.filter((round) => round.steps.length);
}

/** Simple counts for the operator header of a call. */
export function traceStats(record: CallRecord) {
  let modelMs = 0;
  let modelRequests = 0;
  let tokens = 0;
  let proposals = 0;
  let executed = 0;
  let failures = 0;
  for (const event of record.events) {
    if (event.type === 'model.request.completed') {
      const obs = modelObservationSchema.safeParse(event.payload['observation']);
      if (obs.success) {
        modelRequests++;
        modelMs += obs.data.elapsed_ms;
        tokens += obs.data.completion_tokens ?? 0;
        if (obs.data.outcome !== 'success') failures++;
      }
    } else if (event.type === 'agent.tool.proposed') proposals++;
    else if (event.type === 'agent.tool.result' && event.payload['ok'] === true) executed++;
    else if (event.type === 'dependency.failed') failures++;
  }
  return { modelMs, modelRequests, tokens, proposals, executed, failures };
}
