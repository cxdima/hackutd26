import { OUTCOME_LABEL, type CallRecord } from './calls';
import { displayName } from './evidence';
import type { CallEvent, ConsoleState, Tactic } from './schemas';

/**
 * "Sarah spotted it. Maya was ready." built only from receipts. Each stage is
 * done (a receipt proves it), pending (it could still happen) or unavailable
 * (the data cannot show it, e.g. a capsule created by a test rather than a
 * call). A tactic match is about the request pattern, never the caller's number
 * or identity.
 */

export type StoryStageId = 'identified' | 'committed' | 'available' | 'consulted' | 'outcome';
export type StoryStatus = 'done' | 'pending' | 'unavailable';

export interface StoryStage {
  id: StoryStageId;
  label: string;
  status: StoryStatus;
  detail: string;
  at?: string;
}

export interface CommunityStory {
  tacticId: string;
  version: number;
  tactic?: Tactic;
  sourceHousehold?: string;
  sourceName?: string;
  sourceCallId?: string;
  targetHousehold?: string;
  targetName?: string;
  laterCallId?: string;
  provenance?: string;
  stages: StoryStage[];
  headline: string;
}

function names(state: ConsoleState): Map<string, string> {
  const map = new Map<string, string>();
  for (const h of state.community) map.set(h.household_id, h.display_name);
  return map;
}

function capsuleIdOf(event: CallEvent): string | undefined {
  const id = event.payload['id'] ?? event.payload['capsule_id'];
  return typeof id === 'string' ? id : undefined;
}

function matchIds(event: CallEvent): string[] {
  const matches = event.payload['matches'];
  if (!Array.isArray(matches)) return [];
  return matches
    .map((m) => (m && typeof m === 'object' ? (m as Record<string, unknown>)['id'] : undefined))
    .filter((id): id is string => typeof id === 'string');
}

export function buildCommunityStories(
  state: ConsoleState | null,
  events: readonly CallEvent[],
  calls: readonly CallRecord[],
): CommunityStory[] {
  if (!state) return [];
  const householdNames = names(state);
  const nameOf = (id?: string) => (id ? (householdNames.get(id) ?? displayName(id)) : undefined);
  const created = events.filter((e) => e.type === 'tactic.created');
  const ids = new Set<string>();
  for (const e of created) {
    const id = capsuleIdOf(e);
    if (id) ids.add(id);
  }
  for (const t of state.tactics) ids.add(t.id);

  const stories: CommunityStory[] = [];
  for (const id of ids) {
    const tactic = state.tactics.find((t) => t.id === id);
    const createdEvent = [...created].reverse().find((e) => capsuleIdOf(e) === id);
    const version =
      tactic?.version ??
      (typeof createdEvent?.payload['version'] === 'number'
        ? (createdEvent.payload['version'] as number)
        : 1);
    const provenance =
      tactic?.provenance ??
      (typeof createdEvent?.payload['provenance'] === 'string'
        ? (createdEvent.payload['provenance'] as string)
        : undefined);
    const sourceCall = createdEvent?.session_id
      ? calls.find((c) => c.id === createdEvent.session_id)
      : undefined;
    const sourceHousehold = sourceCall?.session.household_id;

    // 1. Identified on a call
    let identified: StoryStage;
    if (sourceCall && sourceCall.outcome === 'blocked')
      identified = {
        id: 'identified',
        label: `${sourceCall.name} spotted it`,
        status: 'done',
        detail: `A call for ${sourceCall.name} asked for something protected, and FirstRing blocked it.`,
        at: sourceCall.startedAt,
      };
    else if (sourceCall)
      identified = {
        id: 'identified',
        label: 'Spotted on a call',
        status: 'pending',
        detail: 'The source call has no protected-request receipt yet.',
      };
    else
      identified = {
        id: 'identified',
        label: 'Spotted on a call',
        status: 'unavailable',
        detail:
          provenance === 'designated_synthetic_test'
            ? 'This warning came from a designated synthetic test, not from a call.'
            : 'This warning is not linked to a call in the current data.',
      };

    // 2. Minimized report committed
    const committed: StoryStage = createdEvent
      ? {
          id: 'committed',
          label: 'A minimized warning was saved',
          status: 'done',
          detail:
            'Only the request type, pretext and pressure were kept, with an expiry. No phone number, name or recording.',
          at: createdEvent.occurred_at,
        }
      : tactic
        ? {
            id: 'committed',
            label: 'A minimized warning was saved',
            status: 'done',
            detail: 'Listed by the server. Its creation receipt has not arrived in this session.',
          }
        : {
            id: 'committed',
            label: 'A minimized warning was saved',
            status: 'pending',
            detail: 'No warning receipt.',
          };

    // 3. Available to another household
    const applied = events.filter(
      (e) =>
        e.type === 'community.applied' &&
        e.payload['capsule_id'] === id &&
        typeof e.payload['version'] === 'number' &&
        (e.payload['version'] as number) >= version,
    );
    const others = applied.filter((e) => e.payload['household_id'] !== sourceHousehold);
    const targetEvent = others.find((e) => e.payload['household_id'] === 'maya') ?? others[0];
    const targetHousehold =
      typeof targetEvent?.payload['household_id'] === 'string'
        ? (targetEvent.payload['household_id'] as string)
        : undefined;
    const available: StoryStage = targetEvent
      ? {
          id: 'available',
          label: `${nameOf(targetHousehold)} was ready`,
          status: 'done',
          detail: `${nameOf(targetHousehold)}’s FirstRing applied warning version ${String(targetEvent.payload['version'])}.`,
          at: targetEvent.occurred_at,
        }
      : {
          id: 'available',
          label: 'Shared with another household',
          status: 'pending',
          detail: 'No other household has an application receipt for this warning yet.',
        };

    // 4. A later call consulted it (a match receipt in another household's call)
    const created_at = Date.parse(createdEvent?.occurred_at ?? '');
    const laterCall = calls.find(
      (c) =>
        c.id !== sourceCall?.id &&
        (!sourceHousehold || c.session.household_id !== sourceHousehold) &&
        (Number.isNaN(created_at) || Date.parse(c.startedAt ?? '') >= created_at) &&
        c.events.some((e) => e.type === 'tactic.matched' && matchIds(e).includes(id)),
    );
    const matchEvent = laterCall?.events.find(
      (e) => e.type === 'tactic.matched' && matchIds(e).includes(id),
    );
    const consulted: StoryStage = matchEvent
      ? {
          id: 'consulted',
          label: 'A later call matched it',
          status: 'done',
          detail: `A different call for ${laterCall!.name} matched the same request pattern. A similar tactic, not the same person or number.`,
          at: matchEvent.occurred_at,
        }
      : {
          id: 'consulted',
          label: 'A later call matched it',
          status: 'pending',
          detail: 'No later call has matched this warning yet.',
        };

    // 5. That call's outcome
    const finished = laterCall && laterCall.outcome !== 'in_progress';
    const outcome: StoryStage =
      laterCall && finished
        ? {
            id: 'outcome',
            label: OUTCOME_LABEL[laterCall.outcome],
            status: 'done',
            detail: laterCall.headline,
            at: laterCall.endedAt,
          }
        : {
            id: 'outcome',
            label: 'Outcome recorded',
            status: 'pending',
            detail: laterCall ? 'That call is still in progress.' : 'Waits for a matching call.',
          };

    const sourceName = sourceCall?.name;
    const targetName = nameOf(targetHousehold ?? laterCall?.session.household_id);
    const headline =
      sourceName && consulted.status === 'done' && targetName
        ? `${sourceName} spotted it. ${targetName} was ready.`
        : available.status === 'done' && targetName
          ? `A shared warning is ready for ${targetName}.`
          : 'A shared warning is being prepared.';

    stories.push({
      tacticId: id,
      version,
      tactic,
      sourceHousehold,
      sourceName,
      sourceCallId: sourceCall?.id,
      targetHousehold: targetHousehold ?? laterCall?.session.household_id,
      targetName,
      laterCallId: laterCall?.id,
      provenance,
      stages: [identified, committed, available, consulted, outcome],
      headline,
    });
  }
  // Most complete first, then newest.
  const score = (s: CommunityStory) => s.stages.filter((x) => x.status === 'done').length;
  return stories.sort((a, b) => score(b) - score(a));
}
