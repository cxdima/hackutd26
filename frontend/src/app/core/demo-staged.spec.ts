import { buildCallRecord } from './calls';
import { buildDemo } from './demo-data';
import { applyFrame, baseSession, stagedScript, type StagedKind } from './demo-staged';
import { handoffView, presentOutcome } from './live';
import { sessionSchema } from './schemas';

function finish(kind: StagedKind) {
  const { state } = buildDemo();
  const profile = {
    ...state.profile!,
    household_id: (kind === 'maya' ? 'maya' : 'sarah') as 'sarah' | 'maya',
  };
  const start = Date.UTC(2026, 10, 8, 14);
  const script = stagedScript(kind, profile, start, 100, 1);
  let session = baseSession(script.sessionId, profile);
  for (const frame of script.frames) session = applyFrame(session, frame);
  expect(sessionSchema.safeParse(session).success).toBe(true);
  expect(script.frames.every((f) => f.event.data_origin === 'demo_fixture')).toBe(true);
  return buildCallRecord(
    session,
    script.frames.map((f) => f.event),
    start + 120_000,
  );
}

describe('fictional rehearsal scripts', () => {
  it('connects the ordinary call only through the full permission sequence', () => {
    const record = finish('connected');
    expect(record.outcome).toBe('connected');
    expect(handoffView(record)!.steps.every((s) => s.observed)).toBe(true);
  });
  it('blocks the changed request and withdraws the pending offer', () => {
    const record = finish('branch');
    expect(record.outcome).toBe('blocked');
    expect(handoffView(record)!.terminal?.kind).toBe('stale_rejected');
    expect(record.session.message?.summary).toContain('Nothing was shared');
  });
  it('blocks Maya’s call after a tactic match', () => {
    const record = finish('maya');
    expect(record.outcome).toBe('blocked');
    expect(record.events.some((e) => e.type === 'tactic.matched')).toBe(true);
    expect(presentOutcome(record).kind).toBe('blocked');
  });
});
