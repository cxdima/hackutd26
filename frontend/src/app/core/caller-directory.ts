import { computed, effect, inject, Injectable, signal, untracked } from '@angular/core';
import {
  buildCallers,
  demoCallerItems,
  fetchCallers,
  type Caller,
  type CallerItem,
} from './callers';
import { ConsoleStore } from './console-store';

/**
 * Callers known to this console, refreshed from the server whenever a call
 * arrives or a reputation receipt changes. Demo mode uses fictional callers.
 */
@Injectable({ providedIn: 'root' })
export class CallerDirectory {
  private readonly store = inject(ConsoleStore);
  readonly items = signal<CallerItem[]>([]);
  readonly loaded = signal(false);
  readonly error = signal('');

  readonly callers = computed<Caller[]>(() => buildCallers(this.items(), this.store.calls()));
  /** Bumps when a new call or a reputation change is received. */
  private readonly version = computed(
    () =>
      this.store
        .events()
        .filter((e) => e.type === 'call.inbound.received' || e.type.startsWith('reputation.'))
        .length,
  );

  constructor() {
    effect((onCleanup) => {
      const source = this.store.source();
      this.version();
      if (source === 'demo') {
        untracked(() => {
          this.items.set(demoCallerItems());
          this.loaded.set(true);
          this.error.set('');
        });
        return;
      }
      if (source !== 'live') {
        untracked(() => {
          this.items.set([]);
          this.loaded.set(false);
        });
        return;
      }
      const controller = new AbortController();
      onCleanup(() => controller.abort());
      untracked(
        () =>
          void fetchCallers(controller.signal)
            .then((items) => {
              this.items.set(items);
              this.loaded.set(true);
              this.error.set('');
            })
            .catch((failure: unknown) => {
              if (controller.signal.aborted) return;
              this.loaded.set(true);
              this.error.set(
                failure instanceof Error ? failure.message : 'Callers could not be loaded.',
              );
            }),
      );
    });
  }

  byId(id: string | undefined): Caller | undefined {
    return id ? this.callers().find((c) => c.id === id) : undefined;
  }

  forCall(sessionId: string): Caller | undefined {
    return this.callers().find((c) => c.calls.some((call) => call.id === sessionId));
  }
}
