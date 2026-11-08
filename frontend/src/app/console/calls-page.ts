import { Component, computed, effect, inject, input, signal, untracked } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router, RouterLink } from '@angular/router';
import { LucideArrowLeft, LucideChevronRight, LucideSearch } from '@lucide/angular';
import { CallerDirectory } from '../core/caller-directory';
import { STANDING_LABEL } from '../core/callers';
import type { CallRecord, Outcome } from '../core/calls';
import { ConsoleStore } from '../core/console-store';
import { formatDateTime, formatDuration } from '../core/format';
import { SelectMenu, type SelectOption } from '../shared/select-menu';
import { CallDetail } from './call-detail';
import { OUTCOME_SHORT, OutcomeText } from './outcome-chip';

type Filter = 'all' | Outcome;

const ORDER: Outcome[] = [
  'in_progress',
  'blocked',
  'trusted',
  'connected',
  'message_saved',
  'needs_review',
  'no_message',
  'incomplete',
];

@Component({
  selector: 'app-calls-page',
  imports: [
    FormsModule,
    RouterLink,
    CallDetail,
    SelectMenu,
    OutcomeText,
    LucideSearch,
    LucideArrowLeft,
    LucideChevronRight,
  ],
  template: `
    @if (id()) {
      <a routerLink="/console/calls" queryParamsHandling="preserve" class="btn btn-quiet mb-6"
        ><svg lucideArrowLeft [size]="17" aria-hidden="true"></svg> All calls</a
      >
      @if (selected(); as call) {
        <app-call-detail [call]="call" />
      } @else {
        <div class="rounded-[var(--radius-card)] bg-paper-2 p-8">
          <p class="text-[1.25rem] font-extrabold">This call isn’t in the current snapshot.</p>
          <p class="mt-2 text-[1.0625rem] text-ink-3">
            It may have been reset on the server, or it belongs to a different data source.
          </p>
        </div>
      }
    } @else {
      <header class="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 class="text-[2.25rem] leading-[1.05] font-extrabold tracking-[-0.03em]">Calls</h1>
          <p class="mt-2 text-[1.0625rem] text-ink-3">
            Every call FirstRing answered, newest first. Open one to see what the AI heard, what it
            decided and why.
          </p>
        </div>
      </header>

      <div class="mt-6 grid gap-3 md:grid-cols-[minmax(0,1fr)_14rem_14rem]">
        <div class="relative">
          <label for="call-search" class="visually-hidden">Search calls</label>
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
            placeholder="Search words, names or outcomes"
            [ngModel]="query()"
            (ngModelChange)="query.set($event)"
          />
        </div>
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
        <div class="min-w-0">
          <span id="household-filter-label" class="visually-hidden">Line</span>
          <app-select-menu
            inputId="household-filter"
            labelledBy="household-filter-label"
            [options]="householdOptions()"
            [value]="household()"
            (valueChange)="household.set($event ?? 'all')"
          />
        </div>
      </div>

      <p class="mt-4 text-[0.95rem] font-semibold text-muted" role="status">
        Showing {{ visible().length }} of {{ store.calls().length }} calls
      </p>

      <ul class="mt-2 divide-y divide-paper-3 border-t-2 border-ink md:hidden" aria-label="Calls">
        @for (call of visible(); track call.id) {
          <li>
            <a
              [routerLink]="['/console/calls', call.id]"
              queryParamsHandling="preserve"
              class="block py-3.5 text-ink no-underline"
            >
              <span class="flex items-baseline justify-between gap-3">
                <app-outcome-text [outcome]="call.outcome" />
                <span class="tabular text-[0.95rem] font-semibold text-muted">{{
                  when(call)
                }}</span>
              </span>
              <span class="mt-1 line-clamp-2 block text-[1.0625rem]">{{ call.request }}</span>
              <span class="block text-[0.95rem] text-muted"
                >{{ call.name }}’s line · {{ duration(call) }}</span
              >
            </a>
          </li>
        } @empty {
          <li class="py-6 text-[1.0625rem] text-ink-3">No calls match.</li>
        }
      </ul>

      <div class="table-scroll mt-2 hidden md:block">
        <table class="data-table min-w-[56rem]">
          <caption class="visually-hidden">
            Calls answered by FirstRing
          </caption>
          <thead>
            <tr>
              <th scope="col">When</th>
              <th scope="col">Line</th>
              <th scope="col">Caller</th>
              <th scope="col" class="w-[38%]">What the caller wanted</th>
              <th scope="col">Outcome</th>
              <th scope="col" class="num">Turns</th>
              <th scope="col" class="num">Length</th>
              <th scope="col"><span class="visually-hidden">Open</span></th>
            </tr>
          </thead>
          <tbody>
            @for (call of visible(); track call.id) {
              <tr class="row-link" (click)="open(call)">
                <td class="whitespace-nowrap">
                  <a
                    [routerLink]="['/console/calls', call.id]"
                    queryParamsHandling="preserve"
                    class="font-bold text-ink no-underline hover:underline"
                    (click)="$event.stopPropagation()"
                    >{{ when(call) }}</a
                  >
                </td>
                <td class="whitespace-nowrap">{{ call.name }}</td>
                <td class="whitespace-nowrap">
                  @if (call.trusted; as t) {
                    <span class="font-semibold">{{ t.name }}</span>
                    <span class="block text-[0.95rem] text-lime-ink">On the trusted list</span>
                  } @else if (caller(call.id); as c) {
                    <span class="font-semibold">{{ c.captured ? c.label : 'Not captured' }}</span>
                    @if (c.captured) {
                      <span
                        class="block text-[0.95rem]"
                        [class]="
                          c.standing === 'reported' || c.standing === 'warned'
                            ? 'font-bold text-magenta-ink'
                            : 'text-muted'
                        "
                        >{{ standing(c.standing) }}</span
                      >
                    }
                  } @else {
                    <span class="text-muted">—</span>
                  }
                </td>
                <td>
                  <span class="line-clamp-2 max-w-[60ch]">{{ call.request }}</span>
                </td>
                <td><app-outcome-text [outcome]="call.outcome" /></td>
                <td class="num">{{ call.callerTurns }}</td>
                <td class="num">{{ duration(call) }}</td>
                <td class="w-8 text-muted">
                  <svg lucideChevronRight [size]="18" aria-hidden="true"></svg>
                </td>
              </tr>
            } @empty {
              <tr>
                <td colspan="8" class="!py-8 text-center text-[1.0625rem]">
                  @if (store.calls().length) {
                    No calls match.
                    <button type="button" class="font-bold underline" (click)="reset()">
                      Clear filters
                    </button>
                  } @else {
                    No calls yet. When someone calls the line, it appears here within a second.
                  }
                </td>
              </tr>
            }
          </tbody>
        </table>
      </div>
    }
  `,
})
export class CallsPage {
  readonly store = inject(ConsoleStore);
  private readonly directory = inject(CallerDirectory);
  private readonly router = inject(Router);
  readonly id = input<string>();
  readonly query = signal('');
  readonly filter = signal<Filter>('all');
  readonly household = signal('all');

  readonly summary = computed(() => {
    const calls = this.store.calls();
    const count = (o: Outcome) => calls.filter((c) => c.outcome === o).length;
    return [
      { label: 'Calls', value: calls.length, cls: '' },
      { label: 'Blocked', value: count('blocked'), cls: 'text-magenta-ink' },
      {
        label: 'Messages saved',
        value: count('message_saved'),
        cls: 'text-lime-ink',
      },
      { label: 'Connected', value: count('connected'), cls: 'text-lime-ink' },
    ];
  });
  readonly households = computed(() => {
    const seen = new Map<string, string>();
    for (const call of this.store.calls()) seen.set(call.session.household_id, call.name);
    return [...seen].map(([id, name]) => ({ id, name }));
  });
  readonly filters = computed(() => {
    const calls = this.store.calls();
    const options: SelectOption[] = [
      { value: 'all', label: 'All outcomes', meta: String(calls.length) },
    ];
    for (const outcome of ORDER) {
      const count = calls.filter((c) => c.outcome === outcome).length;
      if (count || outcome === 'blocked' || outcome === 'message_saved')
        options.push({ value: outcome, label: OUTCOME_SHORT[outcome], meta: String(count) });
    }
    return options;
  });
  readonly householdOptions = computed<SelectOption[]>(() => [
    { value: 'all', label: 'All lines' },
    ...this.households().map((h) => ({ value: h.id, label: `${h.name}’s line` })),
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
        OUTCOME_SHORT[call.outcome],
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

  caller(id: string) {
    return this.directory.forCall(id);
  }
  standing(value: keyof typeof STANDING_LABEL) {
    return STANDING_LABEL[value];
  }
  when(call: CallRecord) {
    return formatDateTime(call.startedAt ?? call.lastActivityAt, call.timezone);
  }
  duration(call: CallRecord) {
    return formatDuration(call.durationSeconds);
  }
  open(call: CallRecord) {
    void this.router.navigate(['/console/calls', call.id], { queryParamsHandling: 'preserve' });
  }
  reset() {
    this.query.set('');
    this.filter.set('all');
    this.household.set('all');
  }
}
