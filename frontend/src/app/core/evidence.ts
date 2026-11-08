import {
  matchSchema,
  modelObservationSchema,
  type CallEvent,
  type Leg,
  type Session,
  type TacticMatch,
} from './schemas';

/**
 * Evidence helpers ported from web/src/model.ts. They only report receipts the
 * server actually sent; nothing here infers success from a request.
 */

/** Events that carry no timeline meaning and only prompt a snapshot refresh. */
export const REFRESH_ONLY_EVENTS = new Set(['session.updated']);
const MAX_EVENTS = 20000;

/** Deduplicates by receipt id and keeps sequence order. */
export function mergeEvents(
  events: readonly CallEvent[],
  incoming: readonly CallEvent[],
): CallEvent[] {
  if (!incoming.length) return events as CallEvent[];
  const known = new Set(events.map((event) => event.id));
  const fresh = incoming.filter((event) => {
    if (known.has(event.id) || REFRESH_ONLY_EVENTS.has(event.type)) return false;
    known.add(event.id);
    return true;
  });
  if (!fresh.length) return events as CallEvent[];
  const merged = [...events, ...fresh];
  merged.sort((a, b) => a.sequence - b.sequence);
  return merged.length > MAX_EVENTS ? merged.slice(-MAX_EVENTS) : merged;
}

export const TERMINAL_LEG_STATUSES = ['completed', 'busy', 'failed', 'no-answer', 'canceled'];

export function legStatus(leg: Leg): string {
  if (leg.acknowledged) return 'Acknowledged by keypad';
  if (leg.kind === 'handoff' && leg.status === 'completed') return 'Call ended';
  const labels: Record<string, string> = {
    reserved: 'Preparing the request',
    unknown: 'Awaiting provider confirmation',
    queued: 'Request accepted',
    initiated: 'Dialing',
    ringing: 'Ringing',
    'in-progress': 'Answered',
    completed: 'Ended · no acknowledgement',
    busy: 'Line busy',
    failed: 'Call failed',
    'no-answer': 'No answer',
    canceled: 'Canceled',
  };
  return labels[leg.status] ?? 'Awaiting a status receipt';
}

/** [requested, ringing, answered, acknowledged] — each only from a receipt or server status. */
export function legEvidence(leg: Leg, events: readonly CallEvent[]): boolean[] {
  const types = new Set(
    events.filter((event) => event.payload['action_id'] === leg.action_id).map((e) => e.type),
  );
  return [
    types.has('outbound.requested') ||
      ['queued', 'initiated', 'ringing', 'in-progress'].includes(leg.status),
    types.has('outbound.ringing') || leg.status === 'ringing',
    types.has('outbound.answered') || leg.status === 'in-progress',
    leg.acknowledged,
  ];
}

const NAMES: Record<string, string> = {
  sarah: 'Sarah',
  maya: 'Maya',
  family: 'Trusted family contact',
  clinic: 'Enrolled clinic',
  caller: 'Caller',
  assistant: 'FirstRing',
  system: 'FirstRing service',
  fixture_caller: 'Automated test caller',
};
export function displayName(role: string): string {
  return NAMES[role] ?? role.replaceAll('_', ' ');
}

export function shortId(id: string): string {
  return id.length > 12 ? `${id.slice(0, 6)}…${id.slice(-4)}` : id;
}

/** Tactic matches confirmed against the household's stored, applied version and expiry. */
export function confirmedMatches(
  events: readonly CallEvent[],
  household: string,
  sessionId: string,
  mode: string,
  applied: number,
  now: number,
): (TacticMatch & { supporting_count: number })[] {
  if (!['sarah', 'maya'].includes(household)) return [];
  const found = new Map<string, TacticMatch>();
  for (const event of events) {
    if (
      event.type !== 'tactic.matched' ||
      event.session_id !== sessionId ||
      event.execution_mode !== mode ||
      !Array.isArray(event.payload['matches'])
    )
      continue;
    for (const value of event.payload['matches'] as unknown[]) {
      const result = matchSchema.safeParse(value);
      if (!result.success) continue;
      const match = result.data;
      if (
        match.version <= applied &&
        match.version <= match.household_applied_version &&
        Date.parse(match.expires_at) > now &&
        new Set(match.matched_fields).size === 2
      )
        found.set(match.id, match);
    }
  }
  const groups = new Map<string, TacticMatch & { supporting_count: number }>();
  for (const match of found.values()) {
    const key = JSON.stringify([match.requested_action, match.pretext_category]);
    const group = groups.get(key);
    const latest = group && group.version > match.version ? group : match;
    groups.set(key, { ...latest, supporting_count: (group?.supporting_count ?? 0) + 1 });
  }
  return [...groups.values()].sort((a, b) => b.version - a.version);
}

/** Latest model HTTP observation for a session and whether it belongs to the current turn. */
export function modelHttpEvidence(session: Session, events: readonly CallEvent[]) {
  const relevant = events.filter(
    (e) => e.session_id === session.id && e.execution_mode === session.execution_mode,
  );
  let completed:
    | {
        generation_id: number;
        turn_version: number;
        sequence: number;
        observation: ReturnType<typeof modelObservationSchema.parse>;
      }
    | undefined;
  for (const event of relevant) {
    if (event.type !== 'model.request.completed') continue;
    const observation = modelObservationSchema.safeParse(event.payload['observation']);
    const generation = event.payload['generation_id'];
    const turn = event.payload['turn_version'];
    if (observation.success && typeof generation === 'number' && typeof turn === 'number')
      completed = {
        generation_id: generation,
        turn_version: turn,
        sequence: event.sequence,
        observation: observation.data,
      };
  }
  const cancelled = [...relevant]
    .reverse()
    .find(
      (event) =>
        event.type === 'model.cancelled' &&
        Number.isInteger(event.payload['generation_id']) &&
        (!completed ||
          (event.sequence > completed.sequence &&
            event.payload['generation_id'] === completed.generation_id)),
    );
  return {
    completed,
    cancelledGeneration: cancelled ? Number(cancelled.payload['generation_id']) : undefined,
    current:
      !!completed &&
      completed.generation_id === session.generation_id &&
      completed.turn_version === session.turn_version,
  };
}
