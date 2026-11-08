import { Component, DestroyRef, inject, output } from '@angular/core';
import { LucideLoader, LucidePlay, LucideRefreshCw } from '@lucide/angular';
import { ConsoleStore } from '../core/console-store';

/**
 * Shown only until the console reaches the FirstRing server on this computer.
 * No login: the server trusts its own machine. If it is not running, it says so
 * and keeps retrying quietly.
 */
@Component({
  selector: 'app-connect-panel',
  imports: [LucideLoader, LucideRefreshCw, LucidePlay],
  template: `
    <div
      class="mx-auto flex min-h-[60vh] max-w-[40rem] flex-col items-center justify-center text-center"
    >
      <img src="media/ringo-greeting-480.webp" width="480" height="480" alt="" class="w-40" />
      @if (store.autoConnecting() || !store.unreachable()) {
        <p class="mt-6 flex items-center gap-3 text-[1.5rem] font-extrabold" role="status">
          <svg lucideLoader [size]="24" class="animate-spin" aria-hidden="true"></svg>
          Opening your calls…
        </p>
      } @else {
        <h1 class="mt-6 text-[1.75rem] font-extrabold leading-tight">
          The FirstRing server isn’t running on this computer.
        </h1>
        <p class="mt-2 max-w-[44ch] text-[1.0625rem] text-ink-3">
          Start it and this page connects by itself.
        </p>
        <div class="mt-6 flex flex-wrap justify-center gap-3">
          <button type="button" class="btn btn-ink" (click)="store.retryLocal()">
            <svg lucideRefreshCw [size]="17" aria-hidden="true"></svg> Try again
          </button>
          <button type="button" class="btn btn-quiet" (click)="demo.emit()">
            <svg lucidePlay [size]="16" aria-hidden="true"></svg> Explore the demo
          </button>
        </div>
      }
    </div>
  `,
})
export class ConnectPanel {
  readonly store = inject(ConsoleStore);
  readonly demo = output<void>();

  constructor() {
    // Keep trying every few seconds while the server is down.
    const timer = setInterval(() => {
      if (this.store.unreachable() && !this.store.autoConnecting()) void this.store.retryLocal();
    }, 5000);
    inject(DestroyRef).onDestroy(() => clearInterval(timer));
  }
}
