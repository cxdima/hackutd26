import { Component, computed, effect, inject, input, signal, untracked } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { LucideArrowLeft, LucideChevronRight } from '@lucide/angular';
import { CallerDirectory } from '../core/caller-directory';
import {
  categoryLabel,
  demoCallerDetail,
  fetchCallerDetail,
  STANDING_LABEL,
  type Caller,
  type CallerDetail,
  type Standing,
} from '../core/callers';
import type { CallRecord } from '../core/calls';
import { ConsoleStore } from '../core/console-store';
import { formatDateTime, formatDuration, relativeAge, relativeUntil } from '../core/format';
import { household } from '../core/household';
import { OutcomeText } from './outcome-chip';

@Component({
  selector: 'app-callers-page',
  imports: [RouterLink, OutcomeText, LucideArrowLeft, LucideChevronRight],
  template: `
    @if (id()) {
      <a routerLink="/console/callers" queryParamsHandling="preserve" class="btn btn-quiet mb-6"
        ><svg lucideArrowLeft [size]="17" aria-hidden="true"></svg> All callers</a
      >
      @if (selected(); as c) {
        <header class="flex flex-wrap items-end justify-between gap-6">
          <div>
            <h1 class="text-[2.25rem] leading-[1.05] font-extrabold tracking-[-0.03em]">
              {{ c.label }}
            </h1>
            <p
              class="mt-2 text-[1.25rem] font-extrabold"
              [class]="flagged(c.standing) ? 'text-magenta-ink' : 'text-ink-3'"
            >
              {{ standing(c.standing) }}
              @if (c.reputation?.reports) {
                · {{ c.reputation!.reports }} active report{{
                  c.reputation!.reports === 1 ? '' : 's'
                }}
              }
            </p>
            <p class="mt-1 max-w-[70ch] text-[1rem] text-muted">
              Caller ID can be faked, so this describes the number, not a person. FirstRing never
              blocks a number on its own; it tells the AI about the history at the start of the next
              call.
            </p>
          </div>
        </header>

        <dl class="mt-8 grid grid-cols-2 gap-x-8 gap-y-5 border-y-2 border-ink py-5 md:grid-cols-4">
          @for (f of facts(); track f.label) {
            <div>
              <dt class="text-[0.95rem] font-semibold text-muted">{{ f.label }}</dt>
              <dd class="mt-0.5 text-[1.125rem] font-extrabold" [class.tabular]="f.numeric">
                {{ f.value }}
              </dd>
            </div>
          }
        </dl>

        @if (c.captured) {
          <section class="mt-10" aria-labelledby="history-heading">
            <h2 id="history-heading" class="text-[1.5rem] font-extrabold">Report history</h2>
            <p class="mt-1 max-w-[70ch] text-[1rem] text-ink-3">
              A report is created when FirstRing blocks a request it never fulfils for a caller.
              Each line counts once, and every report expires.
            </p>
            @if (detailError()) {
              <p class="mt-4 text-[1rem] font-semibold text-magenta-ink" role="alert">
                {{ detailError() }}
              </p>
            }
            <div class="table-scroll mt-4">
              <table class="data-table min-w-[44rem]">
                <thead>
                  <tr>
                    <th scope="col">Reported</th>
                    <th scope="col">By the line of</th>
                    <th scope="col">Why</th>
                    <th scope="col">Expires</th>
                    <th scope="col">Status</th>
                  </tr>
                </thead>
                <tbody>
                  @for (r of detail()?.reports ?? []; track r.report_id) {
                    <tr>
                      <td class="whitespace-nowrap">{{ dateTime(r.created_at) }}</td>
                      <td>{{ lineName(r.household_id) }}</td>
                      <td>{{ r.scam_category ? category(r.scam_category) : 'Not recorded' }}</td>
                      <td class="whitespace-nowrap">{{ until(r.expires_at) }}</td>
                      <td [class]="r.retracted ? 'text-muted' : 'font-bold text-magenta-ink'">
                        {{ r.retracted ? 'Withdrawn' : 'Active' }}
                      </td>
                    </tr>
                  } @empty {
                    <tr>
                      <td colspan="5" class="!py-6 text-[1.0625rem] text-ink-3">
                        @if (detailLoading()) {
                          Loading the history…
                        } @else {
                          No reports. That does not mean the caller is safe; FirstRing still screens
                          every request on its own.
                        }
                      </td>
                    </tr>
                  }
                </tbody>
              </table>
            </div>
          </section>
        }

        <section class="mt-10" aria-labelledby="caller-calls-heading">
          <h2 id="caller-calls-heading" class="text-[1.5rem] font-extrabold">
            Calls from this number
          </h2>
          <div class="table-scroll mt-4">
            <table class="data-table min-w-[48rem]">
              <thead>
                <tr>
                  <th scope="col">When</th>
                  <th scope="col">Line</th>
                  <th scope="col" class="w-[45%]">What the caller wanted</th>
                  <th scope="col">Outcome</th>
                  <th scope="col" class="num">Length</th>
                  <th scope="col"><span class="visually-hidden">Open</span></th>
                </tr>
              </thead>
              <tbody>
                @for (call of c.calls; track call.id) {
                  <tr class="row-link" (click)="openCall(call)">
                    <td class="whitespace-nowrap">
                      <a
                        [routerLink]="['/console/calls', call.id]"
                        queryParamsHandling="preserve"
                        class="font-bold text-ink no-underline hover:underline"
                        (click)="$event.stopPropagation()"
                        >{{ callWhen(call) }}</a
                      >
                    </td>
                    <td class="whitespace-nowrap">{{ call.name }}</td>
                    <td>
                      <span class="line-clamp-2">{{ call.request }}</span>
                    </td>
                    <td><app-outcome-text [outcome]="call.outcome" /></td>
                    <td class="num">{{ duration(call) }}</td>
                    <td class="w-8 text-muted">
                      <svg lucideChevronRight [size]="18" aria-hidden="true"></svg>
                    </td>
                  </tr>
                }
              </tbody>
            </table>
          </div>
        </section>
      } @else {
        <div class="rounded-[var(--radius-card)] bg-paper-2 p-8">
          <p class="text-[1.25rem] font-extrabold">This caller isn’t in the current snapshot.</p>
        </div>
      }
    } @else {
      <header>
        <h1 class="text-[2.25rem] leading-[1.05] font-extrabold tracking-[-0.03em]">Callers</h1>
        <p class="mt-2 max-w-[72ch] text-[1.0625rem] text-ink-3">
          Every number that called, how often, and what FirstRing lines have reported about it. The
          number itself never leaves the server: FirstRing keeps a one-way code of it and shows only
          the last two digits.
        </p>
      </header>

      <ol
        class="mt-8 grid gap-x-8 gap-y-5 border-y-2 border-ink py-6 md:grid-cols-3"
        aria-label="How caller reputation works"
      >
        @for (step of steps; track step.title; let i = $index) {
          <li class="flex gap-4">
            <span
              class="tabular flex size-9 shrink-0 items-center justify-center rounded-full border-2 border-ink text-[1rem] font-extrabold"
              aria-hidden="true"
              >{{ i + 1 }}</span
            >
            <div>
              <p class="text-[1.125rem] font-extrabold">{{ step.title }}</p>
              <p class="mt-1 text-[1rem] text-ink-3">{{ step.text }}</p>
            </div>
          </li>
        }
      </ol>

      <dl class="mt-8 flex flex-wrap gap-x-10 gap-y-3">
        @for (s of summary(); track s.label) {
          <div>
            <dt class="text-[0.95rem] font-semibold text-muted">{{ s.label }}</dt>
            <dd class="stat-figure" [class]="s.cls">{{ s.value }}</dd>
          </div>
        }
      </dl>

      @if (directory.error()) {
        <p class="mt-6 text-[1rem] font-semibold text-magenta-ink" role="alert">
          Callers could not be loaded: {{ directory.error() }}
        </p>
      }

      <div class="table-scroll mt-6">
        <table class="data-table min-w-[60rem]">
          <caption class="visually-hidden">
            Callers and their reported history
          </caption>
          <thead>
            <tr>
              <th scope="col">Caller</th>
              <th scope="col">Reputation</th>
              <th scope="col">Why it was reported</th>
              <th scope="col" class="num">Reports</th>
              <th scope="col" class="num">Calls</th>
              <th scope="col" class="num">Blocked</th>
              <th scope="col">Lines called</th>
              <th scope="col">Last call</th>
              <th scope="col"><span class="visually-hidden">Open</span></th>
            </tr>
          </thead>
          <tbody>
            @for (c of directory.callers(); track c.id) {
              <tr class="row-link" (click)="open(c)">
                <td class="whitespace-nowrap">
                  <a
                    [routerLink]="['/console/callers', c.id]"
                    queryParamsHandling="preserve"
                    class="font-bold text-ink no-underline hover:underline"
                    (click)="$event.stopPropagation()"
                    >{{ c.label }}</a
                  >
                  @if (!c.captured) {
                    <span class="block text-[0.95rem] text-muted"
                      >Calls before numbers were kept</span
                    >
                  }
                </td>
                <td
                  class="whitespace-nowrap"
                  [class]="flagged(c.standing) ? 'font-bold text-magenta-ink' : 'text-ink-3'"
                >
                  {{ c.captured ? standing(c.standing) : '—' }}
                </td>
                <td>
                  {{ reasons(c) }}
                </td>
                <td class="num">{{ c.reputation?.reports ?? 0 }}</td>
                <td class="num">{{ c.calls.length }}</td>
                <td class="num" [class.text-magenta-ink]="c.blocked > 0">{{ c.blocked }}</td>
                <td>{{ c.households.join(', ') }}</td>
                <td class="whitespace-nowrap">{{ age(c.lastAt) }}</td>
                <td class="w-8 text-muted">
                  <svg lucideChevronRight [size]="18" aria-hidden="true"></svg>
                </td>
              </tr>
            } @empty {
              <tr>
                <td colspan="9" class="!py-8 text-center text-[1.0625rem] text-ink-3">
                  @if (directory.loaded()) {
                    No callers yet. Each call adds its number here.
                  } @else {
                    Loading callers…
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
export class CallersPage {
  readonly store = inject(ConsoleStore);
  readonly directory = inject(CallerDirectory);
  private readonly router = inject(Router);
  readonly id = input<string>();
  readonly detail = signal<CallerDetail | null>(null);
  readonly detailLoading = signal(false);
  readonly detailError = signal('');

  readonly steps = [
    {
      title: 'A caller asks for something private',
      text: 'A sign-in code, a password, a Social Security number, a payment or control of a computer. FirstRing never hands these to a caller.',
    },
    {
      title: 'FirstRing blocks it and reports the number',
      text: 'The report records the kind of request, not the words. It expires after 14 days unless the number does it again.',
    },
    {
      title: 'The next call starts with that history',
      text: 'When the number calls any FirstRing line in the network, the AI is told about the reports before it answers.',
    },
  ];

  readonly selected = computed(() => this.directory.byId(this.id()));
  readonly summary = computed(() => {
    const callers = this.directory.callers().filter((c) => c.captured);
    return [
      { label: 'Numbers seen', value: callers.length, cls: '' },
      {
        label: 'Reported',
        value: callers.filter((c) => this.flagged(c.standing)).length,
        cls: 'text-magenta-ink',
      },
      {
        label: 'Called more than once',
        value: callers.filter((c) => c.calls.length > 1).length,
        cls: '',
      },
    ];
  });
  readonly facts = computed(() => {
    const c = this.selected();
    if (!c) return [];
    const rep = c.reputation;
    return [
      { label: 'Calls', value: String(c.calls.length), numeric: true },
      { label: 'Blocked', value: String(c.blocked), numeric: true },
      { label: 'First call', value: this.dateTime(c.firstAt), numeric: false },
      { label: 'Last call', value: this.dateTime(c.lastAt), numeric: false },
      { label: 'Lines called', value: c.households.join(', ') || '—', numeric: false },
      {
        label: 'Why it was reported',
        value: rep?.scam_categories.length
          ? rep.scam_categories.map(categoryLabel).join('; ')
          : '—',
        numeric: false,
      },
      {
        label: 'Report expires',
        value: rep?.expires_at ? this.until(rep.expires_at) : '—',
        numeric: false,
      },
      {
        label: 'Correction',
        value: !rep || rep.correction === 'none' ? 'None' : rep.correction.replaceAll('_', ' '),
        numeric: false,
      },
    ];
  });

  constructor() {
    effect((onCleanup) => {
      const caller = this.selected();
      const key = caller?.numberKey;
      untracked(() => {
        this.detail.set(null);
        this.detailError.set('');
      });
      if (!caller || !key) return;
      if (this.store.isDemo()) {
        const item = untracked(() => this.directory.items()).find((i) => i.number_key === key);
        untracked(() => this.detail.set(item ? demoCallerDetail(item) : null));
        return;
      }
      let cancelled = false;
      onCleanup(() => (cancelled = true));
      untracked(() => this.detailLoading.set(true));
      void fetchCallerDetail(key)
        .then((detail) => {
          if (!cancelled) this.detail.set(detail ?? { reports: [], audit: [] });
        })
        .catch((failure: unknown) => {
          if (!cancelled)
            this.detailError.set(
              failure instanceof Error ? failure.message : 'The history could not be loaded.',
            );
        })
        .finally(() => {
          if (!cancelled) this.detailLoading.set(false);
        });
    });
  }

  flagged(value: Standing) {
    return value === 'reported' || value === 'warned';
  }
  standing(value: Standing) {
    return STANDING_LABEL[value];
  }
  category(value: string) {
    return categoryLabel(value);
  }
  reasons(c: Caller) {
    const categories = c.reputation?.scam_categories ?? [];
    return categories.length ? categories.map(categoryLabel).join('; ') : '—';
  }
  lineName(id: string) {
    return (
      this.store.state()?.community.find((h) => h.household_id === id)?.display_name ??
      household(id)?.name ??
      id
    );
  }
  age(at: string | undefined) {
    return at ? relativeAge(at, this.store.now()) : '—';
  }
  dateTime(at: string | undefined | null) {
    return at ? formatDateTime(at, this.store.state()?.profile?.timezone) : '—';
  }
  until(at: string) {
    return `${formatDateTime(at, this.store.state()?.profile?.timezone)} (${relativeUntil(at, this.store.now())})`;
  }
  callWhen(call: CallRecord) {
    return formatDateTime(call.startedAt ?? call.lastActivityAt, call.timezone);
  }
  duration(call: CallRecord) {
    return formatDuration(call.durationSeconds);
  }
  open(c: Caller) {
    void this.router.navigate(['/console/callers', c.id], { queryParamsHandling: 'preserve' });
  }
  openCall(call: CallRecord) {
    void this.router.navigate(['/console/calls', call.id], { queryParamsHandling: 'preserve' });
  }
}
