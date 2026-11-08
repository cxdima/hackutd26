import { buildDemo } from './demo-data';
import { buildCallRecords } from './calls';
import { replayDuration, replayRecord } from './replay';

describe('recorded call replay', () => {
  const now = Date.UTC(2026, 10, 8, 15);
  const { state, events } = buildDemo(now);
  const blocked = buildCallRecords(state, events, now).find((c) => c.id === 'demo-call-code')!;

  it('shows no outcome before its receipt and the recorded outcome at the end', () => {
    const early = replayRecord(blocked, 6500);
    expect(early.outcome).toBe('in_progress');
    expect(early.session.turns.map((t) => t.speaker)).toEqual(['caller']);
    const done = replayRecord(blocked, replayDuration(blocked));
    expect(done.outcome).toBe('blocked');
    expect(done.session.message?.summary).toBe(blocked.session.message?.summary);
    expect(done.events.length).toBe(blocked.events.length);
  });

  it('never adds receipts the recording does not contain', () => {
    const mid = replayRecord(blocked, 10_000);
    const ids = new Set(blocked.events.map((e) => e.id));
    expect(mid.events.every((e) => ids.has(e.id))).toBe(true);
  });
});
