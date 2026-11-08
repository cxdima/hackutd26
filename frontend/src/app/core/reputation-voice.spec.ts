import { TestBed } from '@angular/core/testing';
import { ConsoleStore } from './console-store';
import { buildDemo } from './demo-data';
import { bandText, demoReputation, hasNoReports, reputationSchema } from './reputation';
import { checkSample } from './voice';

describe('reported-number history', () => {
  it('says “No reports yet”, never safe, when a number has no history', () => {
    const rep = demoReputation('unknown-call')!;
    expect(hasNoReports(rep)).toBe(true);
    expect(bandText(rep)).toBe('No reports yet');
    expect(bandText(rep).toLowerCase()).not.toContain('safe');
    expect(rep.explanation).toContain('does not mean the caller is safe');
  });

  it('shows the server’s own band label and never computes a score', () => {
    const rep = demoReputation('demo-call-giftcard')!;
    expect(bandText(rep)).toBe('Reported by 2 demo profiles');
    expect(rep.verified_independent_households).toBe(0);
    expect(rep.score).toBeNull();
  });

  it('rejects a response without the server’s explanation', () => {
    const rest: Record<string, unknown> = { ...demoReputation('demo-call-giftcard')! };
    delete rest['explanation'];
    expect(reputationSchema.safeParse(rest).success).toBe(false);
  });

  it('never requests reputation from a server that does not advertise it', async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
    TestBed.configureTestingModule({});
    const store = TestBed.inject(ConsoleStore);
    const { state } = buildDemo();
    // Simulate a connected live server without a reputation capability.
    store.source.set('live');
    store.state.set({ ...state, execution_mode: 'live', capabilities: [] });
    const result = await store.loadReputation('s1');
    expect(result.kind).toBe('unavailable');
    expect(fetchMock).not.toHaveBeenCalled();
    store.lock();
    vi.unstubAllGlobals();
  });
});

describe('voice sample checks', () => {
  it('accepts a supported sample within the length limits', () => {
    expect(checkSample('audio/webm;codecs=opus', 30).ok).toBe(true);
  });
  it('explains what to fix', () => {
    expect(checkSample('video/mp4', 30).message).toContain('audio file');
    expect(checkSample('audio/wav', 5).message).toContain('at least 20 seconds');
    expect(checkSample('audio/wav', 500).message).toContain('under 120 seconds');
    expect(checkSample('audio/wav', Number.NaN).ok).toBe(false);
  });
});
