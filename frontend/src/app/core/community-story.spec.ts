import { buildCallRecords } from './calls';
import { buildCommunityStories } from './community-story';
import { buildDemo } from './demo-data';
import { applyToState, stagedScript } from './demo-staged';
import type { CallEvent, ConsoleState } from './schemas';
import { receipt } from '../../testing/fixtures';

function play(state: ConsoleState, events: CallEvent[], kind: 'branch' | 'maya', at: number) {
  const household = kind === 'maya' ? 'maya' : 'sarah';
  const profile = {
    ...state.profile!,
    household_id: household as 'sarah' | 'maya',
    display_name: household === 'maya' ? 'Maya' : 'Sarah',
  };
  const script = stagedScript(kind, profile, at, Math.max(...events.map((e) => e.sequence)), 1);
  for (const frame of script.frames) {
    state = applyToState(state, script, frame, profile);
    events = [...events, frame.event];
  }
  return { state, events };
}

describe('community story', () => {
  it('completes every stage only after Sarah’s call, the shared warning and Maya’s matching call', () => {
    const demo = buildDemo(Date.UTC(2026, 10, 8, 12));
    let { state, events } = play(demo.state, demo.events, 'branch', Date.UTC(2026, 10, 8, 14));
    let stories = buildCommunityStories(
      state,
      events,
      buildCallRecords(state, events, Date.UTC(2026, 10, 8, 15)),
    );
    let story = stories.find((s) => s.tacticId === 'staged-tactic-code')!;
    expect(story.stages.map((s) => s.status)).toEqual([
      'done',
      'done',
      'done',
      'pending',
      'pending',
    ]);
    expect(story.headline).toBe('A shared warning is ready for Maya and Jimmy.');

    ({ state, events } = play(state, events, 'maya', Date.UTC(2026, 10, 8, 14, 30)));
    stories = buildCommunityStories(
      state,
      events,
      buildCallRecords(state, events, Date.UTC(2026, 10, 8, 15)),
    );
    story = stories.find((s) => s.tacticId === 'staged-tactic-code')!;
    expect(story.stages.every((s) => s.status === 'done')).toBe(true);
    expect(story.headline).toBe('Sarah spotted it. Maya and Jimmy were ready.');
    expect(story.stages.find((s) => s.id === 'consulted')!.detail).toContain(
      'not the same person or number',
    );
  });

  it('marks a warning from a designated test as not linked to a call', () => {
    const { state } = buildDemo();
    const live: ConsoleState = {
      ...state,
      sessions: [],
      tactics: [{ ...state.tactics[0], id: 'cap-1', provenance: 'designated_synthetic_test' }],
    };
    const events = [
      receipt('tactic.created', { id: 'cap-1', version: 1 }, { session_id: null }),
      receipt(
        'community.applied',
        { capsule_id: 'cap-1', household_id: 'sarah', version: 1 },
        { session_id: null },
      ),
      receipt(
        'community.applied',
        { capsule_id: 'cap-1', household_id: 'maya', version: 1 },
        { session_id: null },
      ),
    ];
    const [story] = buildCommunityStories(live, events, []);
    expect(story.stages.map((s) => [s.id, s.status])).toEqual([
      ['identified', 'unavailable'],
      ['committed', 'done'],
      ['available', 'done'],
      ['consulted', 'pending'],
      ['outcome', 'pending'],
    ]);
    expect(story.stages[0].detail).toContain('designated synthetic test');
    expect(story.headline).not.toContain('spotted');
  });

  it('does not count an application of an older version as available', () => {
    const { state } = buildDemo();
    const live: ConsoleState = {
      ...state,
      sessions: [],
      tactics: [{ ...state.tactics[0], id: 'cap-2', version: 3 }],
    };
    const events = [
      receipt(
        'community.applied',
        { capsule_id: 'cap-2', household_id: 'maya', version: 2 },
        { session_id: null },
      ),
    ];
    const [story] = buildCommunityStories(live, events, []);
    expect(story.stages.find((s) => s.id === 'available')!.status).toBe('pending');
  });
});
