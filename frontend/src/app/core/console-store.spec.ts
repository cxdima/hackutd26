import { TestBed } from '@angular/core/testing';
import { ConsoleStore, responseError } from './console-store';
import { buildDemo } from './demo-data';

function json(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

describe('ConsoleStore', () => {
  let store: ConsoleStore;
  const fetchMock = vi.fn();

  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal('fetch', fetchMock);
    localStorage.clear();
    sessionStorage.clear();
    TestBed.configureTestingModule({});
    store = TestBed.inject(ConsoleStore);
  });
  afterEach(() => {
    store.lock();
    vi.unstubAllGlobals();
  });

  it('rejects a bad token with a useful message and stays locked', async () => {
    fetchMock.mockResolvedValueOnce(new Response('', { status: 401 }));
    expect(await store.connect('wrong')).toBe(false);
    expect(store.connection()).toBe('locked');
    expect(store.error()).toBe(responseError(401));
  });

  it('keeps the operator token in memory only and sends it as a bearer header', async () => {
    const { state } = buildDemo();
    const live = { ...state, execution_mode: 'live', data_origin: 'synthetic', sessions: [] };
    fetchMock.mockImplementation((url: string) =>
      url === '/api/state'
        ? Promise.resolve(json(live))
        : Promise.resolve(new Response(new ReadableStream(), { status: 200 })),
    );
    expect(await store.connect('  secret-token  ')).toBe(true);
    expect(store.connection()).toBe('live');
    const [, init] = fetchMock.mock.calls[0];
    expect((init as RequestInit).headers).toEqual({ Authorization: 'Bearer secret-token' });
    expect(localStorage.length + sessionStorage.length).toBe(0);
    expect(document.cookie).not.toContain('secret-token');
    expect(location.href).not.toContain('secret-token');
  });

  it('rejects an incompatible server contract instead of guessing', async () => {
    fetchMock.mockResolvedValueOnce(json({ execution_mode: 'live' }));
    expect(await store.connect('token')).toBe(false);
    expect(store.error()).toContain('incompatible');
  });

  it('never sends operator actions from the explicitly selected demo view', async () => {
    store.enterDemo();
    expect(store.connection()).toBe('demo');
    expect(await store.action('/api/demo/reset', {})).toBe(false);
    await expect(store.saveProfile('sarah', store.state()!.profile!)).rejects.toThrow('Demo view');
    expect(fetchMock).not.toHaveBeenCalled();
    expect(store.calls().length).toBeGreaterThan(0);
  });

  it('plays a fictional rehearsal call in the demo view without any network request', () => {
    vi.useFakeTimers();
    store.enterDemo();
    const before = store.calls().length;
    expect(store.playStaged('branch')).toBe(true);
    vi.advanceTimersByTime(5000);
    const live = store.calls().find((c) => c.id.startsWith('staged-branch'));
    expect(live?.outcome).toBe('in_progress');
    store.finishStaged();
    const done = store.calls().find((c) => c.id.startsWith('staged-branch'))!;
    expect(store.calls().length).toBe(before + 1);
    expect(done.outcome).toBe('blocked');
    expect(done.events.every((e) => e.data_origin === 'demo_fixture')).toBe(true);
    // Skipped receipts are re-timed to the moment of skipping, never left in the future.
    expect(Math.max(...done.events.map((e) => Date.parse(e.occurred_at)))).toBeLessThanOrEqual(
      Date.now() + 1000,
    );
    expect(fetchMock).not.toHaveBeenCalled();
    vi.useRealTimers();
  });

  it('refuses rehearsal playback on a live server', async () => {
    const { state } = buildDemo();
    fetchMock.mockImplementation((url: string) =>
      url === '/api/state'
        ? Promise.resolve(json({ ...state, execution_mode: 'live', sessions: [] }))
        : Promise.resolve(new Response(new ReadableStream(), { status: 200 })),
    );
    await store.connect('token');
    expect(store.playStaged('connected')).toBe(false);
  });

  it('marks the view stale when the stream drops, then resumes after the last receipt without duplicates', async () => {
    vi.useFakeTimers();
    const { state } = buildDemo();
    const live = { ...state, execution_mode: 'live', sessions: [] };
    const sse = (ids: number[]) =>
      new Response(
        ids
          .map(
            (id) =>
              `id: ${id}\ndata: ${JSON.stringify({ id: `e${id}`, sequence: id, session_id: null, leg_role: 'system', occurred_at: '2026-11-08T14:00:00Z', received_at: '2026-11-08T14:00:00Z', policy_epoch: 0, execution_mode: 'live', data_origin: 'synthetic', type: 'tactic.created', payload: {} })}\n\n`,
          )
          .join(''),
        { status: 200, headers: { 'Content-Type': 'text/event-stream' } },
      );
    const streams = [sse([1, 2]), sse([2, 3])];
    const lastEventIds: string[] = [];
    fetchMock.mockImplementation((url: string, init: RequestInit) => {
      if (url === '/api/state') return Promise.resolve(json(live));
      lastEventIds.push((init.headers as Record<string, string>)['Last-Event-ID']);
      return Promise.resolve(
        streams.shift() ?? new Response(new ReadableStream(), { status: 200 }),
      );
    });
    await store.connect('token');
    await vi.advanceTimersByTimeAsync(300);
    expect(store.connection()).toBe('paused');
    await vi.advanceTimersByTimeAsync(2500);
    expect(lastEventIds.slice(0, 2)).toEqual(['0', '2']);
    expect(store.events().map((e) => e.sequence)).toEqual([1, 2, 3]);
    vi.useRealTimers();
  });
});
