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
});
