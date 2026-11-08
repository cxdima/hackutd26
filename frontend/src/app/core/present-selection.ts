import type { CallRecord } from './calls';

/**
 * Which call the projected screen shows. "Follow newest" only considers calls
 * that started after the presentation was armed (or reset), plus calls that are
 * still live right now. History replayed on connect therefore cannot take over
 * the screen, and a pinned call stays put until the presenter switches.
 */

export interface PresentSelection {
  mode: 'follow' | 'pinned';
  pinnedId: string | null;
  /** Epoch ms. Calls that started before this are history in follow mode. */
  since: number;
}

/** A call that started slightly before arming still counts (clock skew, quick dialing). */
export const ARM_GRACE_MS = 10_000;
/** A still-open call with a receipt this recent is live, whenever it started. */
export const ACTIVE_WINDOW_MS = 2 * 60_000;

function time(value: string | undefined): number {
  const parsed = Date.parse(value ?? '');
  return Number.isNaN(parsed) ? Number.NaN : parsed;
}

export function startedAt(call: CallRecord): number {
  return time(call.startedAt ?? call.lastActivityAt);
}

export function isFollowable(call: CallRecord, since: number, now: number): boolean {
  const started = startedAt(call);
  if (!Number.isNaN(started) && started >= since) return true;
  // Before arming or reset: only a call that is still open counts, never a finished one.
  if (!call.active) return false;
  if (!Number.isNaN(started) && started >= since - ARM_GRACE_MS) return true;
  const last = time(call.lastActivityAt);
  return !Number.isNaN(last) && now - last < ACTIVE_WINDOW_MS;
}

export interface PresentResolution {
  call?: CallRecord;
  /** Pinned mode: the pinned id is no longer in the snapshot. */
  missing: boolean;
  /** Pinned mode: a newer followable call the presenter may switch to. */
  newer?: CallRecord;
  /** Most recent call of any age, offered as "show the latest call" while waiting. */
  latest?: CallRecord;
}

export function resolvePresentation(
  calls: readonly CallRecord[],
  selection: PresentSelection,
  now: number,
): PresentResolution {
  const sorted = [...calls].sort((a, b) => (startedAt(b) || 0) - (startedAt(a) || 0));
  const followable = sorted.filter((call) => isFollowable(call, selection.since, now));
  const newest = followable[0];
  if (selection.mode === 'pinned' && selection.pinnedId) {
    const pinned = sorted.find((call) => call.id === selection.pinnedId);
    const newer =
      newest && pinned && newest.id !== pinned.id && startedAt(newest) > startedAt(pinned)
        ? newest
        : undefined;
    return { call: pinned, missing: !pinned, newer, latest: sorted[0] };
  }
  return { call: newest, missing: false, latest: sorted[0] };
}
