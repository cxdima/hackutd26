import { computed, Injectable, OnDestroy, signal } from '@angular/core';
import { buildCallRecords, type CallRecord } from './calls';
import { buildDemo } from './demo-data';
import { consumeEvents } from './event-stream';
import { mergeEvents } from './evidence';
import {
  fixtureStatusSchema,
  profileSchema,
  stateSchema,
  type CallEvent,
  type ConsoleState,
  type FixtureStatus,
  type Profile,
} from './schemas';

export type Source = 'none' | 'live' | 'demo';
export type Connection = 'locked' | 'connecting' | 'live' | 'replay' | 'demo' | 'paused';

export function responseError(status: number): string {
  if (status === 401 || status === 403)
    return 'The console token was not accepted. Check the operator token configured on the FirstRing server.';
  if (status === 409)
    return 'That action is not available in the current call state. The latest state has been refreshed.';
  if (status === 503)
    return 'The service is waiting for provider setup. Open Operator detail to see what is missing.';
  return 'The request could not be completed. Check that the FirstRing server is running.';
}

function timeout(ms: number, signal?: AbortSignal): AbortSignal {
  return signal ? AbortSignal.any([signal, AbortSignal.timeout(ms)]) : AbortSignal.timeout(ms);
}

/**
 * Authenticated console state. The operator token lives only in this service's
 * memory: never localStorage, sessionStorage, cookies or a URL. Reloading or
 * locking clears it. Demo data is only loaded when the visitor explicitly asks.
 */
@Injectable({ providedIn: 'root' })
export class ConsoleStore implements OnDestroy {
  private token = '';
  private controller?: AbortController;
  private reconnectTimer?: ReturnType<typeof setTimeout>;
  private refreshTimer?: ReturnType<typeof setTimeout>;
  private flushTimer?: ReturnType<typeof setTimeout>;
  private pending: CallEvent[] = [];
  private lastSequence = 0;
  private generation = 0;
  private clock?: ReturnType<typeof setInterval>;

  readonly source = signal<Source>('none');
  readonly state = signal<ConsoleState | null>(null);
  readonly events = signal<CallEvent[]>([]);
  readonly stale = signal(false);
  readonly connecting = signal(false);
  readonly busy = signal<string | null>(null);
  readonly error = signal('');
  readonly notice = signal('');
  readonly lastReceiptAt = signal<string | null>(null);
  readonly now = signal(Date.now());

  readonly connection = computed<Connection>(() => {
    const source = this.source();
    if (source === 'demo') return 'demo';
    if (this.connecting() && !this.state()) return 'connecting';
    if (source !== 'live' || !this.state()) return 'locked';
    if (this.stale()) return 'paused';
    return this.state()?.execution_mode === 'replay' ? 'replay' : 'live';
  });
  readonly calls = computed<CallRecord[]>(() => {
    const state = this.state();
    return state ? buildCallRecords(state, this.events(), this.now()) : [];
  });
  readonly isDemo = computed(() => this.source() === 'demo');

  ngOnDestroy(): void {
    this.stop();
  }

  private startClock() {
    clearInterval(this.clock);
    this.clock = setInterval(() => this.now.set(Date.now()), 30000);
  }

  async connect(rawToken: string): Promise<boolean> {
    const token = rawToken.trim();
    if (!token || this.connecting()) return false;
    this.stop();
    const generation = ++this.generation;
    this.connecting.set(true);
    this.error.set('');
    this.notice.set('');
    try {
      const initial = await this.loadState(token);
      if (generation !== this.generation) return false;
      this.token = token;
      this.source.set('live');
      this.state.set(initial);
      this.events.set([]);
      this.lastSequence = 0;
      this.stale.set(false);
      this.startClock();
      this.controller = new AbortController();
      void this.stream(this.controller.signal, generation);
      return true;
    } catch (failure) {
      if (generation === this.generation)
        this.error.set(failure instanceof Error ? failure.message : 'Connection failed.');
      return false;
    } finally {
      if (generation === this.generation) this.connecting.set(false);
    }
  }

  enterDemo(): void {
    this.stop();
    this.generation++;
    this.token = '';
    const demo = buildDemo();
    this.source.set('demo');
    this.state.set(demo.state);
    this.events.set(demo.events);
    this.stale.set(false);
    this.error.set('');
    this.notice.set('');
    this.now.set(Date.now());
    this.startClock();
  }

  lock(): void {
    this.generation++;
    this.stop();
    this.token = '';
    this.source.set('none');
    this.state.set(null);
    this.events.set([]);
    this.stale.set(false);
    this.busy.set(null);
    this.error.set('');
    this.notice.set('');
    this.lastReceiptAt.set(null);
  }

  private stop() {
    this.controller?.abort();
    this.controller = undefined;
    clearTimeout(this.reconnectTimer);
    clearTimeout(this.refreshTimer);
    clearTimeout(this.flushTimer);
    clearInterval(this.clock);
    this.pending = [];
  }

  private async loadState(token: string, signal?: AbortSignal): Promise<ConsoleState> {
    let response: Response;
    try {
      response = await fetch('/api/state', {
        headers: { Authorization: `Bearer ${token}` },
        signal: timeout(10000, signal),
        cache: 'no-store',
      });
    } catch {
      throw new Error(responseError(0));
    }
    if (!response.ok) throw new Error(responseError(response.status));
    const parsed = stateSchema.safeParse(await response.json());
    if (!parsed.success)
      throw new Error(
        'The server returned an incompatible console state. Check that the server and console versions match.',
      );
    return parsed.data;
  }

  /** Coalesces bursts of receipts into one snapshot request. */
  private scheduleRefresh(signal: AbortSignal) {
    clearTimeout(this.refreshTimer);
    this.refreshTimer = setTimeout(async () => {
      try {
        const fresh = await this.loadState(this.token, signal);
        if (!signal.aborted) this.state.set(fresh);
      } catch {
        if (!signal.aborted) this.stale.set(true);
      }
    }, 250);
  }

  private queue(event: CallEvent, signal: AbortSignal) {
    this.pending.push(event);
    if (this.flushTimer) return;
    this.flushTimer = setTimeout(() => {
      this.flushTimer = undefined;
      if (signal.aborted) return;
      const batch = this.pending;
      this.pending = [];
      this.events.update((events) => mergeEvents(events, batch));
      this.lastReceiptAt.set(batch.at(-1)?.received_at ?? null);
    }, 120);
  }

  private async stream(signal: AbortSignal, generation: number) {
    const opening = new AbortController();
    const headerTimeout = setTimeout(() => opening.abort(), 10000);
    try {
      const response = await fetch('/api/events', {
        headers: {
          Authorization: `Bearer ${this.token}`,
          'Last-Event-ID': String(this.lastSequence),
          Accept: 'text/event-stream',
        },
        signal: AbortSignal.any([signal, opening.signal]),
        cache: 'no-store',
      });
      clearTimeout(headerTimeout);
      if (!response.ok) throw new Error(responseError(response.status));
      if (!signal.aborted) this.stale.set(false);
      this.scheduleRefresh(signal);
      await consumeEvents(response, (event) => {
        if (signal.aborted) return;
        this.lastSequence = Math.max(this.lastSequence, event.sequence);
        this.queue(event, signal);
        this.scheduleRefresh(signal);
      });
    } catch {
      /* The stream is recoverable; the last snapshot stays visible and is marked paused. */
    } finally {
      clearTimeout(headerTimeout);
    }
    if (!signal.aborted && generation === this.generation) {
      this.stale.set(true);
      this.reconnectTimer = setTimeout(() => void this.stream(signal, generation), 2000);
    }
  }

  /** POSTs an operator control. Never available in demo; success means "request sent", not "done". */
  async action(path: string, body: Record<string, unknown>): Promise<boolean> {
    if (this.busy() || !this.token || this.source() !== 'live') return false;
    const generation = this.generation;
    this.busy.set(path);
    this.error.set('');
    this.notice.set('');
    try {
      const response = await fetch(path, {
        method: 'POST',
        headers: { Authorization: `Bearer ${this.token}`, 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
        signal: AbortSignal.timeout(15000),
      });
      if (!response.ok) throw new Error(responseError(response.status));
      const fresh = await this.loadState(this.token);
      if (generation !== this.generation) return false;
      this.state.set(fresh);
      this.notice.set('Request sent. The console shows the outcome only when its receipt arrives.');
      return true;
    } catch (failure) {
      if (generation === this.generation)
        this.error.set(
          failure instanceof Error
            ? failure.message
            : 'The action did not return a result. Check the received events before trying again.',
        );
      return false;
    } finally {
      if (generation === this.generation) this.busy.set(null);
    }
  }

  async loadProfile(household: string, signal?: AbortSignal): Promise<Profile> {
    if (this.source() === 'demo') {
      const profile = this.state()?.profile;
      if (profile && profile.household_id === household) return structuredClone(profile);
      const fromSession = this.state()?.sessions.find((s) => s.household_id === household)?.profile;
      if (fromSession) return structuredClone(fromSession);
      throw new Error('Demo profile unavailable.');
    }
    const response = await fetch(`/api/profile/${encodeURIComponent(household)}`, {
      headers: { Authorization: `Bearer ${this.token}` },
      cache: 'no-store',
      signal: timeout(10000, signal),
    });
    if (!response.ok) throw new Error('Profile unavailable. Check the server and operator token.');
    const profile = profileSchema.parse(await response.json());
    if (profile.household_id !== household)
      throw new Error('Profile household did not match. Refresh before editing.');
    return profile;
  }

  async saveProfile(household: string, draft: Profile, signal?: AbortSignal): Promise<Profile> {
    if (this.source() === 'demo')
      throw new Error(
        'Demo view: settings are shown but not saved. Connect to a FirstRing server to save.',
      );
    const fields = {
      display_name: draft.display_name,
      greeting: draft.greeting,
      tts_provider: draft.tts_provider,
      voice_id: draft.voice_id,
      locale: draft.locale,
      timezone: draft.timezone,
    };
    const response = await fetch(`/api/profile/${encodeURIComponent(household)}`, {
      method: 'PUT',
      headers: { Authorization: `Bearer ${this.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify(fields),
      cache: 'no-store',
      signal: timeout(10000, signal),
    });
    if (!response.ok)
      throw new Error(
        'Not saved. Use a supported voice preset and a short greeting without links or phone numbers, then try again.',
      );
    const profile = profileSchema.parse(await response.json());
    if (profile.household_id !== household)
      throw new Error('Profile household did not match. Refresh before editing.');
    return profile;
  }

  /** Read-only automated caller status. Refreshing it never starts or retries a call. */
  async loadFixtureStatus(): Promise<FixtureStatus> {
    const response = await fetch('/api/demo/audio-fixture', {
      headers: { Authorization: `Bearer ${this.token}` },
      cache: 'no-store',
      signal: AbortSignal.timeout(10000),
    });
    if (!response.ok) throw new Error('Status unavailable. Check the server and operator token.');
    const parsed = fixtureStatusSchema.safeParse(await response.json());
    if (!parsed.success) throw new Error('Runner status format changed; no result inferred.');
    return parsed.data;
  }

  /** Fetches the latest automated-caller recording as a private blob URL, only on request. */
  async loadFixtureRecording(): Promise<string> {
    const response = await fetch('/api/demo/audio-fixture/recording', {
      headers: { Authorization: `Bearer ${this.token}` },
      cache: 'no-store',
      signal: AbortSignal.timeout(20000),
    });
    if (!response.ok) throw new Error('No recording is ready for the latest automated test call.');
    const blob = await response.blob();
    if (!blob.size || !blob.type.startsWith('audio/'))
      throw new Error('The server did not return audio.');
    return URL.createObjectURL(blob);
  }
}
