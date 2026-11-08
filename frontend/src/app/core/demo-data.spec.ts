import { buildCallRecords } from './calls';
import { buildDemo, DEMO_ORIGIN } from './demo-data';
import { eventSchema, stateSchema } from './schemas';

it('builds a demo that satisfies the real server contract and is labeled as demo data everywhere', () => {
  const now = Date.parse('2026-11-08T14:30:00Z');
  const { state, events } = buildDemo(now);
  expect(stateSchema.safeParse(state).success).toBe(true);
  for (const event of events) expect(eventSchema.safeParse(event).success).toBe(true);
  expect(state.data_origin).toBe(DEMO_ORIGIN);
  expect(
    state.sessions.every((s) => s.data_origin === DEMO_ORIGIN && s.execution_mode === 'replay'),
  ).toBe(true);
  expect(events.every((e) => e.data_origin === DEMO_ORIGIN)).toBe(true);
  expect(state.capabilities.find((c) => c.id === 'voice_clone')?.status).toBe('disabled');
  const outcomes = buildCallRecords(state, events, now).map((r) => r.outcome);
  expect(new Set(outcomes)).toEqual(
    new Set(['blocked', 'message_saved', 'needs_review', 'incomplete', 'trusted']),
  );
});
