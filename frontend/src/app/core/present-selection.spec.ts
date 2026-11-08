import { NOW, receipt, session } from '../../testing/fixtures';
import { buildCallRecord, type CallRecord } from './calls';
import { resolvePresentation, type PresentSelection } from './present-selection';

function call(id: string, startedAt: number, live = false, lastAt = startedAt): CallRecord {
  const at = new Date(startedAt).toISOString();
  const last = new Date(lastAt).toISOString();
  const events = [
    receipt('call.inbound.received', {}, { session_id: id, occurred_at: at, received_at: at }),
    live
      ? receipt(
          'transcript.partial',
          { text: 'still talking' },
          { session_id: id, occurred_at: last, received_at: last },
        )
      : receipt('call.ended', {}, { session_id: id, occurred_at: at, received_at: at }),
  ];
  return buildCallRecord(
    session({ id, inbound_ended: !live, state: live ? 'screening' : 'ended' }),
    events,
    NOW,
  );
}

const armed = NOW - 60_000;
const follow: PresentSelection = { mode: 'follow', pinnedId: null, since: armed };

describe('presentation call selection', () => {
  it('never lets history replayed on connect take over the screen', () => {
    const history = [call('old-1', NOW - 3_600_000), call('old-2', NOW - 7_200_000)];
    const result = resolvePresentation(history, follow, NOW);
    expect(result.call).toBeUndefined();
    expect(result.latest?.id).toBe('old-1');
  });

  it('follows a call that starts after arming, even while older events stream in', () => {
    const fresh = call('new', NOW - 5_000, true);
    const calls = [call('old', NOW - 3_600_000), fresh, call('older', NOW - 9_000_000)];
    expect(resolvePresentation(calls, follow, NOW).call?.id).toBe('new');
  });

  it('includes a call that started before arming but is live right now', () => {
    const ongoing = call('ongoing', NOW - 5 * 60_000, true, NOW - 10_000);
    expect(resolvePresentation([ongoing], follow, NOW).call?.id).toBe('ongoing');
    // An open call with no receipt for minutes is not "live now".
    const silent = call('silent', NOW - 5 * 60_000, true, NOW - 4 * 60_000);
    expect(resolvePresentation([silent], follow, NOW).call).toBeUndefined();
  });

  it('does not bring back a call that finished just before a reset', () => {
    const finished = call('finished', NOW - 3_000);
    expect(resolvePresentation([finished], { ...follow, since: NOW }, NOW).call).toBeUndefined();
  });

  it('keeps a pinned call and offers the newer one without switching', () => {
    const pinned = call('pinned', NOW - 3_600_000);
    const fresh = call('new', NOW - 2_000, true);
    const result = resolvePresentation(
      [fresh, pinned],
      { mode: 'pinned', pinnedId: 'pinned', since: armed },
      NOW,
    );
    expect(result.call?.id).toBe('pinned');
    expect(result.newer?.id).toBe('new');
  });

  it('reports a pinned call that disappeared instead of silently switching', () => {
    const result = resolvePresentation(
      [call('other', NOW - 1_000, true)],
      { mode: 'pinned', pinnedId: 'gone', since: armed },
      NOW,
    );
    expect(result.call).toBeUndefined();
    expect(result.missing).toBe(true);
  });
});
