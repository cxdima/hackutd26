import { buildCallRecord, type CallRecord } from './calls';
import { applyFrame, baseSession } from './demo-staged';
import type { CallEvent, Profile, Session } from './schemas';

/**
 * Replays a recorded call's own receipts on the presentation screen. The
 * snapshot at each moment is rebuilt from the receipts up to that moment, so
 * the outcome appears only when its receipt does. Nothing is invented: text
 * the receipts do not carry (the saved message summary) comes from the
 * recorded call's final snapshot, and only once its receipt is reached.
 */

export function replayStart(call: CallRecord): number {
  const first = call.events[0]?.occurred_at;
  const parsed = Date.parse(first ?? '');
  return Number.isNaN(parsed) ? 0 : parsed;
}

export function replayDuration(call: CallRecord): number {
  const last = Date.parse(call.events.at(-1)?.occurred_at ?? '');
  const start = replayStart(call);
  return Number.isNaN(last) ? 0 : Math.max(0, last - start);
}

function understandingFrom(event: CallEvent, session: Session): Partial<Session> {
  const p = event.payload;
  const text = (key: string) => (typeof p[key] === 'string' ? (p[key] as string) : '');
  const readiness = text('readiness');
  if (!['needs_information', 'ready', 'caller_declined', 'no_message'].includes(readiness))
    return {};
  return {
    understanding: {
      generation_id: Number(p['generation_id'] ?? session.generation_id),
      turn_version: Number(p['turn_version'] ?? session.turn_version ?? 0),
      policy_epoch: session.policy_epoch,
      understanding: {
        version: 1,
        claimed_identity: null,
        purpose: text('purpose').slice(0, 180),
        requested_action: text('requested_action').slice(0, 180),
        supplied_facts: [],
        missing_information: Array.isArray(p['missing_information'])
          ? (p['missing_information'] as unknown[]).map(String).slice(0, 3)
          : [],
        readiness: readiness as 'ready',
      },
    },
  };
}

/** The call as it stood `elapsedMs` after its first receipt. */
export function replayRecord(call: CallRecord, elapsedMs: number): CallRecord {
  const start = replayStart(call);
  const cutoff = start + elapsedMs;
  const profile = (call.session.profile ?? undefined) as Profile | undefined;
  let session: Session = {
    ...baseSession(call.id, profile ?? ({ household_id: call.session.household_id } as Profile)),
    household_id: call.session.household_id,
    profile: call.session.profile ?? null,
    execution_mode: call.session.execution_mode,
    data_origin: call.session.data_origin,
    legs: [],
  };
  const events: CallEvent[] = [];
  for (const event of call.events) {
    if (Date.parse(event.occurred_at) > cutoff) break;
    events.push(event);
    let patch: Partial<Session> | undefined;
    if (event.type === 'message.recorded' && call.session.message)
      patch = { message: call.session.message };
    else if (event.type === 'call.understanding.accepted')
      patch = understandingFrom(event, session);
    session = applyFrame(session, { delay: 0, event, patch });
  }
  // Phone legs keep their recorded identity; their status is the latest receipt so far.
  const status = new Map<string, string>();
  for (const event of events)
    if (event.type.startsWith('outbound.') && typeof event.payload['action_id'] === 'string')
      status.set(event.payload['action_id'], String(event.payload['status'] ?? 'reserved'));
  const acknowledged = new Set(
    events
      .filter((e) => e.type === 'contact.acknowledged')
      .map((e) => String(e.payload['action_id'])),
  );
  session = {
    ...session,
    // A leg appears only once a receipt in the replay so far refers to it.
    legs: call.session.legs
      .filter(
        (leg) =>
          status.has(leg.action_id) ||
          (leg.kind === 'handoff' && events.some((e) => e.type === 'handoff.offered')),
      )
      .map((leg) => ({
        ...leg,
        status: status.get(leg.action_id) ?? 'reserved',
        acknowledged: acknowledged.has(leg.action_id),
      })),
    offer: session.offer
      ? { ...session.offer, leg_id: call.session.offer?.leg_id ?? session.offer.leg_id }
      : session.offer,
  };
  return buildCallRecord(session, events, cutoff);
}
