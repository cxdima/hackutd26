import { Component, effect, inject, input, signal } from '@angular/core';
import { ConsoleStore } from '../core/console-store';
import { formatDateTime, relativeAge } from '../core/format';
import {
  bandText,
  correctionText,
  hasNoReports,
  isDemoScope,
  type ReputationResult,
} from '../core/reputation';

/**
 * Reported-number history for one call, kept separate from tactic matching.
 * Shows the server's band, explanation and evidence; never a browser-made score.
 */
@Component({
  selector: 'app-reputation-panel',
  template: `
    <section
      class="rounded-[var(--radius-card)] border-2 border-paper-3 p-5"
      aria-labelledby="rep-heading"
    >
      <div class="flex flex-wrap items-center justify-between gap-3">
        <h3 id="rep-heading" class="text-[1.25rem] font-extrabold">Reported-number history</h3>
        @if (result()?.kind === 'ok') {
          @if (fixture()) {
          } @else if (demoScope()) {}
        }
      </div>
      @switch (result()?.kind) {
        @case (undefined) {
          <p class="mt-2 text-[1.0625rem] text-muted" role="status">Checking…</p>
        }
        @case ('unavailable') {
          <p class="mt-2 max-w-[65ch] text-[1.0625rem] text-ink-3">{{ reason() }}</p>
        }
        @case ('error') {
          <p class="mt-2 max-w-[65ch] text-[1.0625rem] text-magenta-ink" role="alert">
            {{ reason() }}
          </p>
        }
        @case ('ok') {
          @if (data(); as rep) {
            <p class="mt-2 text-[1.0625rem] text-muted">{{ rep.number_label }}</p>
            <p class="mt-1 text-[1.5rem] font-extrabold leading-tight">
              {{ band() }}
              @if (rep.score !== null && rep.score !== undefined) {
                <span class="text-[1.0625rem] font-semibold text-ink-3"
                  >· server score {{ rep.score_label ?? rep.score }}</span
                >
              }
            </p>
            <p class="mt-2 max-w-[65ch] text-[1.0625rem]">{{ rep.explanation }}</p>
            @if (!noReports()) {
              <dl class="mt-4 grid grid-cols-[auto_1fr] gap-x-5 gap-y-1.5 text-[1rem]">
                <dt class="font-bold text-muted">Verified independent households</dt>
                <dd class="tabular">
                  {{ rep.verified_independent_households ?? rep.independent_households }}
                </dd>
                @if (rep.demo_household_count) {
                  <dt class="font-bold text-muted">Demo profiles reporting</dt>
                  <dd class="tabular">{{ rep.demo_household_count }} (simulated households)</dd>
                }
                <dt class="font-bold text-muted">Reports</dt>
                <dd class="tabular">{{ rep.reports }}</dd>
                <dt class="font-bold text-muted">Most recent</dt>
                <dd>{{ when(rep.last_reported_at) }}</dd>
                <dt class="font-bold text-muted">Expires</dt>
                <dd>{{ rep.expires_at ? date(rep.expires_at) : 'Not set' }}</dd>
              </dl>
            }
            @if (correction(); as text) {
              <p class="mt-3 rounded-xl bg-amber-wash p-3 text-[1rem] text-amber-ink">{{ text }}</p>
            }
            <p class="mt-3 text-[0.95rem] text-muted">
              A number can be spoofed or reassigned. This is history about a number, not proof of
              who called, and it never blocks a number at the carrier.
            </p>
          }
        }
      }
    </section>
  `,
})
export class ReputationPanel {
  readonly sessionId = input.required<string>();
  private readonly store = inject(ConsoleStore);
  readonly result = signal<ReputationResult | undefined>(undefined);

  constructor() {
    effect((onCleanup) => {
      const id = this.sessionId();
      const controller = new AbortController();
      onCleanup(() => controller.abort());
      this.result.set(undefined);
      void this.store.loadReputation(id, controller.signal).then((result) => {
        if (!controller.signal.aborted) this.result.set(result);
      });
    });
  }

  data() {
    const r = this.result();
    return r?.kind === 'ok' ? r.data : undefined;
  }
  fixture() {
    const r = this.result();
    return r?.kind === 'ok' && r.fixture;
  }
  reason() {
    const r = this.result();
    return r?.kind === 'unavailable' ? r.reason : r?.kind === 'error' ? r.message : '';
  }
  band() {
    const d = this.data();
    return d ? bandText(d) : '';
  }
  demoScope() {
    const d = this.data();
    return !!d && isDemoScope(d);
  }
  noReports() {
    const d = this.data();
    return d ? hasNoReports(d) : true;
  }
  correction() {
    const d = this.data();
    return d ? correctionText(d) : undefined;
  }
  when(value: string | null) {
    return value ? relativeAge(value, this.store.now()) : 'Never';
  }
  date(value: string) {
    return formatDateTime(value, this.store.state()?.profile?.timezone);
  }
}
