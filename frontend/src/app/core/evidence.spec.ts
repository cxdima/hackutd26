import { confirmedMatches, legEvidence, legStatus, mergeEvents } from './evidence';
import type { Leg } from './schemas';
import { receipt } from '../../testing/fixtures';

const leg: Leg = {
  action_id: 'action-b',
  role: 'family',
  status: 'in-progress',
  acknowledged: false,
};

describe('phone progression', () => {
  it('does not turn an answer or an ended call into an acknowledgement', () => {
    expect(legStatus(leg)).toBe('Answered');
    expect(legStatus({ ...leg, status: 'completed' })).toBe('Ended · no acknowledgement');
    expect(legStatus({ ...leg, status: 'completed', acknowledged: true })).toBe(
      'Acknowledged by keypad',
    );
  });

  it('does not infer missing ringing or answer receipts from a terminal status', () => {
    expect(legEvidence({ ...leg, status: 'completed' }, [])).toEqual([false, false, false, false]);
    const events = [
      receipt('outbound.ringing', { action_id: 'action-b' }),
      receipt('outbound.ringing', { action_id: 'other' }),
    ];
    expect(legEvidence({ ...leg, status: 'completed' }, events)).toEqual([
      false,
      true,
      false,
      false,
    ]);
  });
});

describe('receipt history', () => {
  it('deduplicates redelivered receipts, restores order and drops refresh-only events', () => {
    const a = receipt('transcript.final');
    const b = receipt('call.ended');
    const refresh = receipt('session.updated');
    const merged = mergeEvents([b], [a, b, refresh]);
    expect(merged.map((e) => e.id)).toEqual([a.id, b.id]);
    expect(mergeEvents(merged, [a])).toBe(merged);
  });
});

describe('shared tactic matches', () => {
  const match = (id: string, version: number, extra: Record<string, unknown> = {}) => ({
    id,
    version,
    requested_action: 'share_authentication_secret',
    pretext_category: 'bank_account',
    expires_at: '2026-11-09T00:00:00Z',
    household_applied_version: version,
    matched_fields: ['requested_action', 'pretext_category'],
    ...extra,
  });
  const now = Date.parse('2026-11-08T12:00:00Z');

  it('shows a match only for this session, an applied unexpired version, and the same execution mode', () => {
    const events = [
      receipt('tactic.matched', { matches: [match('c1', 1)] }),
      receipt('tactic.matched', { matches: [match('c2', 3)] }),
      receipt('tactic.matched', {
        matches: [match('c3', 1, { expires_at: '2026-11-08T11:00:00Z' })],
      }),
      receipt('tactic.matched', { matches: [match('c4', 1)] }, { session_id: 'other' }),
      receipt('tactic.matched', { matches: [match('c5', 1)] }, { execution_mode: 'replay' }),
    ];
    const found = confirmedMatches(events, 'sarah', 's1', 'live', 2, now);
    expect(found.map((m) => m.id)).toEqual(['c1']);
    expect(confirmedMatches(events, 'unknown', 's1', 'live', 2, now)).toEqual([]);
  });
});
