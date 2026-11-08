import { Component, inject, output, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import {
  LucideArrowRight,
  LucideKeyRound,
  LucideLoader,
  LucideLockKeyhole,
  LucidePlay,
} from '@lucide/angular';
import { ConsoleStore } from '../core/console-store';

/**
 * Locked state. The token form talks to the configured FirstRing server; the
 * demo button loads clearly labeled fictional data. No call is placed by either.
 */
@Component({
  selector: 'app-connect-panel',
  imports: [
    FormsModule,
    LucideKeyRound,
    LucideArrowRight,
    LucidePlay,
    LucideLockKeyhole,
    LucideLoader,
  ],
  template: `
    <div class="grid gap-10 py-6 lg:grid-cols-12 lg:py-12">
      <div class="lg:col-span-6">
        <h1
          class="text-[clamp(2.4rem,1.6rem+2.8vw,4rem)] leading-[1.02] font-extrabold tracking-[-0.03em]"
        >
          Your calls, with the reasons behind them.
        </h1>
        <p class="mt-6 max-w-[44ch] text-lede text-ink-3">
          See what each caller wanted, what FirstRing heard, why it acted, and what happened next.
          Every step is shown as it was received.
        </p>
        <img
          src="media/ringo-greeting-480.webp"
          width="480"
          height="480"
          alt=""
          class="mt-10 hidden w-56 lg:block"
        />
      </div>

      <div class="space-y-6 lg:col-span-5 lg:col-start-8">
        @if (store.autoConnecting()) {
          <section
            class="rounded-[var(--radius-card)] border-2 border-ink bg-paper p-6 sm:p-8"
            role="status"
          >
            <p class="flex items-center gap-3 text-[1.5rem] font-extrabold">
              <svg lucideLoader [size]="24" class="animate-spin" aria-hidden="true"></svg>
              Connecting to your FirstRing server…
            </p>
            <p class="mt-3 text-[1.0625rem] text-ink-3">
              Using this computer’s local operator session. Connecting places no call.
            </p>
          </section>
        } @else {
          <section
            class="rounded-[var(--radius-card)] border-2 border-ink bg-paper p-6 sm:p-8"
            aria-labelledby="connect-heading"
          >
            <h2 id="connect-heading" class="flex items-center gap-3 text-[1.5rem] font-extrabold">
              <svg lucideKeyRound [size]="24" aria-hidden="true"></svg> Connect to your FirstRing
              server
            </h2>
            <p class="mt-3 text-[1.0625rem] text-ink-3">
              Use the operator token configured on the server. Connecting places no call.
            </p>
            <form class="mt-6" (ngSubmit)="connect()" novalidate>
              <label for="operator-token" class="field-label">Operator token</label>
              <input
                id="operator-token"
                name="token"
                type="password"
                class="field"
                autocomplete="off"
                spellcheck="false"
                [(ngModel)]="token"
                [attr.aria-invalid]="!!store.error() && !store.connecting()"
                aria-describedby="token-help"
                required
              />
              <p id="token-help" class="field-help flex items-center gap-2">
                <svg lucideLockKeyhole [size]="15" aria-hidden="true"></svg> Kept only in this tab’s
                memory. Cleared when you lock or reload.
              </p>
              <button
                type="submit"
                class="btn btn-ink mt-6 w-full !min-h-14 text-[1.125rem]"
                [disabled]="!token().trim() || store.connecting()"
              >
                {{ store.connecting() ? 'Connecting…' : 'Connect' }}
                <svg lucideArrowRight [size]="19" aria-hidden="true"></svg>
              </button>
            </form>
            <p class="mt-5 text-[1rem] text-muted">
              On the presenting computer, start the console with
              <code class="rounded bg-paper-2 px-1.5 py-0.5">npm start</code>
              from the repository and it connects by itself. No token to paste.
            </p>
          </section>
        }

        <section
          class="rounded-[var(--radius-card)] bg-paper-2 p-6 sm:p-8"
          aria-labelledby="demo-heading"
        >
          <h2 id="demo-heading" class="text-[1.375rem] font-extrabold">No server handy?</h2>
          <p class="mt-2 text-[1.0625rem] text-ink-3">
            Open a demo with five fictional calls, labeled DEMO throughout. It runs entirely in your
            browser and is never mixed with live data.
          </p>
          <button
            type="button"
            class="btn btn-primary mt-5 w-full !min-h-14 text-[1.125rem]"
            (click)="demo.emit()"
          >
            <svg lucidePlay [size]="18" aria-hidden="true"></svg> Explore the demo
          </button>
        </section>
      </div>
    </div>
  `,
})
export class ConnectPanel {
  readonly store = inject(ConsoleStore);
  readonly demo = output<void>();
  readonly token = signal('');

  async connect() {
    const ok = await this.store.connect(this.token());
    if (ok) this.token.set('');
  }
}
