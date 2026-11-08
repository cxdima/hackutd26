import { Component, computed, effect, inject, input, signal, untracked } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { RouterLink } from '@angular/router';
import { LucideArrowLeft, LucideSearch } from '@lucide/angular';
import type { Outcome } from '../core/calls';
import { ConsoleStore } from '../core/console-store';
import { SelectMenu, type SelectOption } from '../shared/select-menu';
import { CallDetail } from './call-detail';
import { CallRow } from './call-row';

type Filter = 'all' | Outcome;

/** Short outcome names for the filter; the full wording stays on each row. */
const FILTER_LABEL: Record<Outcome, string> = {
  in_progress: 'On the line now',
  blocked: 'Blocked',
  connected: 'Connected',
  message_saved: 'Message saved',
  needs_review: 'Needs review',
  no_message: 'No message',
  incomplete: 'Incomplete',
};

@Component({
  selector: 'app-calls-page',
  imports: [
    FormsModule,
    RouterLink,
    CallRow,
    CallDetail,
    SelectMenu,
    LucideSearch,
    LucideArrowLeft,
  ],
  template: `
    <div class="grid grid-cols-1 gap-x-10 gap-y-8 lg:grid-cols-12">
      <!-- The list scrolls on its own at lg; the inline padding keeps focus rings inside its clip box. -->
      <section
        class="scroll-cue min-w-0 lg:sticky lg:top-[5.5rem] lg:col-span-4 lg:-ml-2 lg:block lg:max-h-[calc(100dvh-6.5rem)] lg:self-start lg:overflow-x-hidden lg:overflow-y-auto lg:pr-2 lg:pl-2"
        [class.hidden]="!!id()"
        aria-labelledby="calls-heading"
      >
        @if (id()) {
          <h2
            id="calls-heading"
            class="text-[2rem] leading-[1.05] font-extrabold tracking-[-0.03em]"
          >
            Calls
          </h2>
        } @else {
          <h1
            id="calls-heading"
            class="text-[2rem] leading-[1.05] font-extrabold tracking-[-0.03em]"
          >
            Calls
          </h1>
        }
        <div class="mt-4 space-y-3">
          <div>
            <label for="call-search" class="visually-hidden">Search calls</label>
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
                placeholder="Search calls"
                [ngModel]="query()"
                (ngModelChange)="query.set($event)"
              />
            </div>
          </div>
          <div class="@container">
            <div
              class="grid gap-2"
              [class]="households().length > 1 ? 'grid-cols-1 @[23rem]:grid-cols-2' : 'grid-cols-1'"
            >
              <div class="min-w-0">
                <span id="outcome-filter-label" class="visually-hidden">Outcome</span>
                <app-select-menu
                  inputId="outcome-filter"
                  labelledBy="outcome-filter-label"
                  [options]="filters()"
                  [value]="filter()"
                  (valueChange)="filter.set($any($event))"
                />
              </div>
              @if (households().length > 1) {
                <div class="min-w-0">
                  <span id="household-filter-label" class="visually-hidden">Answered for</span>
                  <app-select-menu
                    inputId="household-filter"
                    labelledBy="household-filter-label"
                    [options]="householdOptions()"
                    [value]="household()"
                    (valueChange)="household.set($event ?? 'all')"
                  />
                </div>
              }
            </div>
          </div>
        </div>
        <p class="mt-4 text-[0.95rem] font-semibold text-muted" role="status">
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

      <section
        class="min-w-0 lg:col-span-8 lg:block"
        [class.hidden]="!id()"
        aria-label="Call detail"
      >
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
    const options: SelectOption[] = [
      { value: 'all', label: 'All calls', meta: String(calls.length) },
    ];
    for (const outcome of order) {
      const count = calls.filter((c) => c.outcome === outcome).length;
      if (count || outcome === 'blocked' || outcome === 'message_saved')
        options.push({ value: outcome, label: FILTER_LABEL[outcome], meta: String(count) });
    }
    return options;
  });
  readonly householdOptions = computed<SelectOption[]>(() => [
    { value: 'all', label: 'For everyone' },
    ...this.households().map((h) => ({ value: h.id, label: `For ${h.name}` })),
  ]);
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
