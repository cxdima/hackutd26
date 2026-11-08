import { computed, effect, inject, Injectable, signal, untracked } from '@angular/core';
import { z } from 'zod';
import { ConsoleStore } from './console-store';

/**
 * Trusted callers per household. A call from a listed number skips AI
 * screening, recording and reputation and rings the enrolled phone directly.
 * The server never returns the full number; only "ending 1234".
 */
export const trustedItemSchema = z.object({
  id: z.string(),
  name: z.string(),
  relation: z.string().nullable().optional(),
  number_label: z.string(),
  added_at: z.string().nullable().optional(),
});
export type TrustedItem = z.infer<typeof trustedItemSchema>;

export const trustedListSchema = z.object({
  household_id: z.string(),
  items: z.array(trustedItemSchema),
  connects_to_enrolled_phone: z.boolean().optional().default(false),
  meaning: z.string().optional().default(''),
});
export type TrustedList = z.infer<typeof trustedListSchema>;

export class TrustedUnavailable extends Error {}

async function parse(response: Response): Promise<TrustedList> {
  const body: unknown = await response.json().catch(() => ({}));
  if (response.status === 404)
    throw new TrustedUnavailable('This FirstRing server does not offer trusted callers yet.');
  if (!response.ok) {
    const message = body && typeof body === 'object' && 'error' in body ? String(body.error) : '';
    throw new Error(message || 'The FirstRing server did not accept that.');
  }
  return trustedListSchema.parse(body);
}

export async function fetchTrusted(household: string, signal?: AbortSignal): Promise<TrustedList> {
  return parse(
    await fetch(`/api/trusted?household=${encodeURIComponent(household)}`, {
      cache: 'no-store',
      signal,
    }),
  );
}

async function post(path: string, body: unknown): Promise<TrustedList> {
  return parse(
    await fetch(path, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
      cache: 'no-store',
    }),
  );
}

export const addTrusted = (entry: {
  household_id: string;
  name: string;
  relation: string;
  number: string;
}) => post('/api/trusted', entry);

export const removeTrusted = (household_id: string, id: string) =>
  post('/api/trusted/remove', { household_id, id });

/** Digits only, US numbers assumed when ten digits are given. */
export function normalizeNumber(raw: string): string | null {
  const digits = raw.replace(/[^\d+]/g, '');
  if (/^\+\d{8,15}$/.test(digits)) return digits;
  const plain = digits.replace(/\+/g, '');
  if (plain.length === 10) return `+1${plain}`;
  if (plain.length === 11 && plain.startsWith('1')) return `+${plain}`;
  return null;
}

export const TRUSTED_MEANING =
  'Calls from these numbers skip screening, recording and reputation and ring straight through. Caller ID can be faked, so only add numbers you would answer anyway.';

/** Fictional trusted lists for demo mode. Read-only. */
export function demoTrusted(household: string): TrustedList {
  const at = new Date(Date.now() - 3 * 24 * 3600 * 1000).toISOString();
  const items: TrustedItem[] =
    household === 'maya'
      ? [
          {
            id: 'demo-sarah',
            name: 'Sarah',
            relation: 'Jimmy’s mother',
            number_label: 'ending 0613',
            added_at: at,
          },
        ]
      : [
          {
            id: 'demo-jimmy',
            name: 'Jimmy',
            relation: 'Son',
            number_label: 'ending 4471',
            added_at: at,
          },
          {
            id: 'demo-maya',
            name: 'Maya',
            relation: 'Daughter-in-law',
            number_label: 'ending 2208',
            added_at: at,
          },
        ];
  return {
    household_id: household,
    items,
    connects_to_enrolled_phone: true,
    meaning: TRUSTED_MEANING,
  };
}

/**
 * The trusted list for the household FirstRing currently answers for. Live
 * servers are read with GET; demo mode shows a fictional read-only list.
 */
@Injectable({ providedIn: 'root' })
export class TrustedDirectory {
  private readonly store = inject(ConsoleStore);
  /** Settings can pick another line to manage; otherwise the line being answered. */
  readonly chosen = signal<string | null>(null);
  readonly household = computed(() => {
    const state = this.store.state();
    return this.chosen() ?? state?.profile?.household_id ?? state?.selected_household ?? 'sarah';
  });
  readonly list = signal<TrustedList | null>(null);
  readonly loaded = signal(false);
  /** Set when the server has no trusted-callers endpoint (an older build). */
  readonly unavailable = signal(false);
  readonly error = signal('');
  readonly busy = signal(false);
  readonly items = computed(() => this.list()?.items ?? []);
  private readonly version = computed(
    () => this.store.events().filter((e) => e.type.startsWith('trusted.')).length,
  );

  constructor() {
    effect((onCleanup) => {
      const source = this.store.source();
      const household = this.household();
      this.version();
      if (source === 'demo') {
        untracked(() => {
          this.list.set(demoTrusted(household));
          this.loaded.set(true);
          this.unavailable.set(false);
          this.error.set('');
        });
        return;
      }
      if (source !== 'live') return;
      const controller = new AbortController();
      onCleanup(() => controller.abort());
      untracked(() => void this.load(household, controller.signal));
    });
  }

  async load(household = this.household(), signal?: AbortSignal) {
    try {
      const list = await fetchTrusted(household, signal);
      if (signal?.aborted) return;
      this.list.set(list);
      this.unavailable.set(false);
      this.error.set('');
    } catch (failure) {
      if (signal?.aborted) return;
      if (failure instanceof TrustedUnavailable) this.unavailable.set(true);
      else
        this.error.set(
          failure instanceof Error ? failure.message : 'Trusted callers could not be loaded.',
        );
    } finally {
      if (!signal?.aborted) this.loaded.set(true);
    }
  }

  /** Returns an error message, or '' when the server accepted the change. */
  add(entry: { name: string; relation: string; number: string }): Promise<string> {
    return this.change(() => addTrusted({ household_id: this.household(), ...entry }));
  }

  remove(id: string): Promise<string> {
    return this.change(() => removeTrusted(this.household(), id));
  }

  private async change(run: () => Promise<TrustedList>): Promise<string> {
    if (this.store.isDemo()) return 'The demo list is read-only.';
    this.busy.set(true);
    try {
      this.list.set(await run());
      return '';
    } catch (failure) {
      return failure instanceof Error ? failure.message : 'The change was not saved.';
    } finally {
      this.busy.set(false);
    }
  }
}
