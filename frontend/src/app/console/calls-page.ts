import { Component, computed, effect, inject, input, signal, untracked } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { RouterLink } from '@angular/router';
import { LucideArrowLeft, LucideSearch } from '@lucide/angular';
import { OUTCOME_LABEL, type Outcome } from '../core/calls';
import { ConsoleStore } from '../core/console-store';
import { CallDetail } from './call-detail';
import { CallRow } from './call-row';

type Filter = 'all' | Outcome;

@Component({
  selector: 'app-calls-page',
  imports: [FormsModule, RouterLink, CallRow, CallDetail, LucideSearch, LucideArrowLeft],
  template: `
    <div class="grid gap-8 lg:grid-cols-12">
      <section
        class="scroll-cue lg:sticky lg:top-[5.5rem] lg:col-span-5 lg:block lg:max-h-[calc(100dvh-6.5rem)] lg:self-start lg:overflow-y-auto lg:pr-2"
        [class.hidden]="!!id()"
        aria-labelledby="calls-heading"
      >
        @if (id()) {
          <h2
            id="calls-heading"
            class="text-[clamp(2rem,1.6rem+1.6vw,2.9rem)] leading-[1.05] font-extrabold tracking-[-0.03em]"
          >
            Calls
          </h2>
        } @else {
          <h1
            id="calls-heading"
            class="text-[clamp(2rem,1.6rem+1.6vw,2.9rem)] leading-[1.05] font-extrabold tracking-[-0.03em]"
          >
            Calls
          </h1>
        }
        <div class="mt-5 space-y-4">
          <div>
            <label for="call-search" class="field-label">Search calls</label>
            <div class="relative">
              <svg
                lucideSearch
                [size]="19"
                class="pointer-events-none absolute left-3.5 top-1/2 -translate-y-1/2 text-muted"
                aria-hidden="true"
              ></svg>
              <input
                id="call-search"
                type="search"
                class="field !pl-11"
                placeholder="Words from the call or summary"
                [ngModel]="query()"
                (ngModelChange)="query.set($event)"
              />
            </div>
          </div>
          <fieldset>
            <legend class="field-label">Show</legend>
            <div class="flex flex-wrap gap-2">
              @for (option of filters(); track option.value) {
                <button
                  type="button"
                  class="min-h-11 rounded-full border-2 px-4 text-[1rem] font-bold transition-colors"
                  [class]="
                    filter() === option.value
                      ? 'border-ink bg-ink text-paper'
                      : 'border-rule bg-paper text-ink hover:border-ink'
                  "
                  [attr.aria-pressed]="filter() === option.value"
                  (click)="filter.set(option.value)"
                >
                  {{ option.label }}
                  <span class="tabular font-semibold opacity-80">{{ option.count }}</span>
                </button>
              }
            </div>
          </fieldset>
          @if (households().length > 1) {
            <div>
              <label for="household-filter" class="field-label">Answered for</label>
              <select
                id="household-filter"
                class="field"
                [ngModel]="household()"
                (ngModelChange)="household.set($event)"
              >
                <option value="all">Everyone</option>
                @for (h of households(); track h.id) {
                  <option [value]="h.id">{{ h.name }}</option>
                }
              </select>
            </div>
          }
        </div>
        <p class="mt-6 text-[1rem] font-semibold text-muted" role="status">
          {{ visible().length }} of {{ store.calls().length }} calls
        </p>
        <ul class="mt-2 divide-y divide-paper-3">
          @for (call of visible(); track call.id) {
            <li><app-call-row [call]="call" [selected]="call.id === id()" /></li>
          } @empty {
            <li class="rounded-xl bg-paper-2 p-5 text-[1.0625rem]">
              @if (store.calls().length) {
                No calls match.
                <button type="button" class="font-bold underline" (click)="reset()">
                  Clear filters
                </button>
              } @else {
                No calls have been received yet.
              }
            </li>
          }
        </ul>
        <!-- Overflow cue: the sticky list scrolls on its own; the fade shows that more calls follow. -->
        <div
          class="pointer-events-none sticky bottom-0 -mt-12 hidden h-12 bg-linear-to-t from-paper to-transparent lg:block"
          aria-hidden="true"
        ></div>
        <div class="hidden h-6 lg:block" aria-hidden="true"></div>
      </section>

      <section class="lg:col-span-7 lg:block" [class.hidden]="!id()" aria-label="Call detail">
        @if (id()) {
          <a
            routerLink="/console/calls"
            queryParamsHandling="preserve"
            class="btn btn-quiet mb-6 lg:hidden"
            ><svg lucideArrowLeft [size]="17" aria-hidden="true"></svg> All calls</a
          >
        }
        @if (selected(); as call) {
          <app-call-detail [call]="call" />
        } @else if (id()) {
          <div class="rounded-[var(--radius-card)] bg-paper-2 p-8">
            <p class="text-[1.25rem] font-extrabold">This call isn’t in the current snapshot.</p>
            <p class="mt-2 text-[1.0625rem] text-ink-3">
              It may have been reset on the server, or it belongs to a different data source.
            </p>
          </div>
        } @else {
          <div
            class="hidden rounded-[var(--radius-card)] border-2 border-dashed border-rule p-10 text-center lg:block"
          >
            <p class="text-[1.25rem] font-extrabold">Choose a call</p>
            <p class="mt-2 text-[1.0625rem] text-ink-3">
              You’ll see the transcript, what FirstRing heard, the rule it applied, and every
              receipt.
            </p>
          </div>
        }
      </section>
    </div>
  `,
})
export class CallsPage {
  readonly store = inject(ConsoleStore);
  readonly id = input<string>();
  readonly query = signal('');
  readonly filter = signal<Filter>('all');
  readonly household = signal('all');

  readonly households = computed(() => {
    const seen = new Map<string, string>();
    for (const call of this.store.calls()) seen.set(call.session.household_id, call.name);
    return [...seen].map(([id, name]) => ({ id, name }));
  });
  readonly filters = computed(() => {
    const calls = this.store.calls();
    const order: Outcome[] = [
      'in_progress',
      'blocked',
      'connected',
      'message_saved',
      'needs_review',
      'no_message',
      'incomplete',
    ];
    const options: { value: Filter; label: string; count: number }[] = [
      { value: 'all', label: 'All', count: calls.length },
    ];
    for (const outcome of order) {
      const count = calls.filter((c) => c.outcome === outcome).length;
      if (count || outcome === 'blocked' || outcome === 'message_saved')
        options.push({ value: outcome, label: OUTCOME_LABEL[outcome], count });
    }
    return options;
  });
  readonly visible = computed(() => {
    const q = this.query().trim().toLowerCase();
    return this.store.calls().filter((call) => {
      if (this.filter() !== 'all' && call.outcome !== this.filter()) return false;
      if (this.household() !== 'all' && call.session.household_id !== this.household())
        return false;
      if (!q) return true;
      const haystack = [
        call.headline,
        call.request,
        call.name,
        ...call.session.turns.map((t) => t.text),
        call.session.message?.summary ?? '',
      ]
        .join(' ')
        .toLowerCase();
      return haystack.includes(q);
    });
  });
  readonly selected = computed(() => this.store.calls().find((call) => call.id === this.id()));

  constructor() {
    // Opening a live call fetches its own receipts directly; the stream remains the source of truth.
    effect((onCleanup) => {
      const id = this.id();
      if (!id || this.store.source() !== 'live') return;
      const controller = new AbortController();
      onCleanup(() => controller.abort());
      untracked(() => void this.store.backfillSession(id, controller.signal));
    });
  }

  reset() {
    this.query.set('');
    this.filter.set('all');
    this.household.set('all');
  }
}
