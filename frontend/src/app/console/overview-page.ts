import { Component, computed, inject, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { LucideArrowRight, LucideChevronRight, LucideMonitorPlay } from '@lucide/angular';
import { CallerDirectory } from '../core/caller-directory';
import { categoryLabel, STANDING_LABEL } from '../core/callers';
import type { CallRecord, Outcome } from '../core/calls';
import { ConsoleStore } from '../core/console-store';
import {
  formatDateTime,
  formatDay,
  formatDuration,
  formatPhone,
  isE164,
  relativeAge,
  relativeUntil,
} from '../core/format';
import { FAMILY, household } from '../core/household';
import { humanize, PRETEXT_LABEL, TACTIC_LABEL } from '../core/language';
import { personAction } from '../core/live';
import { TrustedDirectory } from '../core/trusted';
import { OutcomeIcon, OutcomeText } from './outcome-chip';

/** Outcome groups for the proportion bar, in a fixed, colour-validated order. */
const GROUPS: { id: string; label: string; icon: Outcome; members: Outcome[]; fill: string }[] = [
  { id: 'blocked', label: 'Blocked', icon: 'blocked', members: ['blocked'], fill: '#b3005c' },
  {
    id: 'review',
    label: 'Needs review',
    icon: 'needs_review',
    members: ['needs_review'],
    fill: '#d39b2a',
  },
  {
    id: 'through',
    label: 'Put through',
    icon: 'connected',
    members: ['connected', 'trusted'],
    fill: '#3a6300',
  },
  {
    id: 'saved',
    label: 'Message saved',
    icon: 'message_saved',
    members: ['message_saved'],
    fill: '#76b900',
  },
  {
    id: 'other',
    label: 'No message or incomplete',
    icon: 'incomplete',
    members: ['no_message', 'incomplete'],
    fill: '#b8bfb6',
  },
];

type Tab = 'watch' | 'warnings' | 'trusted' | 'system';

@Component({
  selector: 'app-overview-page',
  imports: [
    RouterLink,
    OutcomeText,
    OutcomeIcon,
    LucideArrowRight,
    LucideChevronRight,
    LucideMonitorPlay,
  ],
  template: `
    <header class="flex flex-wrap items-baseline justify-between gap-x-8 gap-y-1">
      <h1
        class="text-[clamp(2rem,1.6rem+1.4vw,2.75rem)] leading-[1.05] font-extrabold tracking-[-0.03em]"
      >
        {{ lineName() }}’s line
      </h1>
      <p class="text-[1rem] font-semibold text-muted">{{ today() }}</p>
    </header>
    <p class="mt-2 max-w-[70ch] text-[1.0625rem] text-ink-3">{{ intro() }}</p>

    @if (live(); as call) {
      <section
        class="on-ink mt-6 flex flex-wrap items-center gap-x-6 gap-y-3 rounded-[var(--radius-card)] bg-ink px-5 py-4 text-paper"
        aria-labelledby="live-heading"
      >
        <h2 id="live-heading" class="text-[1.125rem] font-extrabold text-lime">On the line now</h2>
        <p class="min-w-0 flex-1 text-[1.0625rem]">{{ call.session.current_action }}</p>
        <a
          routerLink="/console/present"
          queryParamsHandling="preserve"
          class="btn btn-primary no-underline"
          ><svg lucideMonitorPlay [size]="17" aria-hidden="true"></svg> Show it live</a
        >
        <a
          [routerLink]="['/console/calls', call.id]"
          queryParamsHandling="preserve"
          class="btn btn-quiet no-underline"
          >Open the call</a
        >
      </section>
    }

    <div class="mt-8 grid gap-x-8 gap-y-8 xl:grid-cols-[minmax(0,1.55fr)_minmax(20rem,1fr)]">
      @if (featured(); as call) {
        <section
          class="overflow-hidden rounded-[var(--radius-card)] border-2 border-ink bg-paper"
          aria-labelledby="featured-heading"
        >
          <div
            class="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1 bg-ink px-5 py-3 text-paper sm:px-7"
          >
            <h2 id="featured-heading" class="text-[1.0625rem] font-extrabold">
              {{ featuredLabel() }}
            </h2>
            <p class="text-[0.95rem] font-semibold text-paper/75">
              <time [attr.datetime]="call.startedAt">{{ when(call) }}</time>
            </p>
          </div>
          <div class="px-5 pt-5 pb-6 sm:px-7 sm:pt-6 sm:pb-7">
            <app-outcome-text [outcome]="call.outcome" [short]="false" />
            <p
              class="mt-3 max-w-[40ch] text-[clamp(1.375rem,1.15rem+0.8vw,1.875rem)] leading-[1.2] font-extrabold tracking-[-0.015em]"
            >
              {{ call.headline }}
            </p>
            @if (quote(); as q) {
              <figure class="caption-band mt-5 max-w-[46rem] px-5 py-3.5">
                <figcaption class="caption-speaker">&gt;&gt; Caller</figcaption>
                <blockquote class="mt-1 text-[1.125rem] leading-snug font-semibold">
                  “{{ q }}”
                </blockquote>
              </figure>
            }
            <dl class="mt-6 grid gap-x-8 border-t border-paper-3 sm:grid-cols-2">
              <div class="border-b border-paper-3 py-3 sm:border-b-0">
                <dt class="text-[0.95rem] font-semibold text-muted">The caller wanted</dt>
                <dd class="mt-0.5 text-[1.0625rem] font-semibold">{{ call.request }}</dd>
              </div>
              <div class="py-3">
                <dt class="text-[0.95rem] font-semibold text-muted">Does anyone need to act?</dt>
                <dd class="mt-0.5 text-[1.0625rem] font-semibold">{{ action() }}</dd>
              </div>
            </dl>
            <a
              [routerLink]="['/console/calls', call.id]"
              queryParamsHandling="preserve"
              class="btn btn-ink mt-4 no-underline"
            >
              See what the AI heard and why it acted
              <svg lucideArrowRight [size]="18" aria-hidden="true"></svg>
            </a>
          </div>
        </section>
      } @else {
        <section
          class="flex flex-col items-start gap-4 rounded-[var(--radius-card)] bg-paper-2 p-7"
        >
          <img src="media/ringo-rest-480.webp" width="480" height="480" alt="" class="w-36" />
          <h2 class="text-[1.375rem] font-extrabold">No calls yet</h2>
          <p class="max-w-[46ch] text-[1.0625rem] text-ink-3">
            When someone calls {{ number() ?? 'the line' }}, FirstRing answers first and the call,
            with its reasons, appears here.
          </p>
        </section>
      }

      <section class="min-w-0" aria-labelledby="mix-heading">
        <div class="flex flex-wrap items-baseline justify-between gap-3">
          <h2 id="mix-heading" class="text-[1.25rem] font-extrabold">What happened to calls</h2>
          <div class="segmented" role="group" aria-label="Period">
            <button
              type="button"
              [attr.aria-pressed]="period() === 'today'"
              (click)="chosenPeriod.set('today')"
            >
              Today
            </button>
            <button
              type="button"
              [attr.aria-pressed]="period() === 'all'"
              (click)="chosenPeriod.set('all')"
            >
              All
            </button>
          </div>
        </div>
        <p class="mt-1 text-[1rem] text-ink-3">{{ mixLine() }}</p>
        @if (mixTotal()) {
          <div class="mix-bar mt-4" role="img" [attr.aria-label]="mixLabel()">
            @for (g of mix(); track g.id) {
              @if (g.count) {
                <span
                  [style.flex-grow]="g.count"
                  [style.background]="g.fill"
                  [attr.title]="g.label + ': ' + g.count"
                ></span>
              }
            }
          </div>
        }
        <table class="data-table mt-3">
          <caption class="visually-hidden">
            Calls by outcome
          </caption>
          <tbody>
            @for (g of mix(); track g.id) {
              <tr>
                <td class="!py-2.5">
                  <span class="inline-flex items-center gap-2.5 font-semibold">
                    <span class="mix-key" [style.background]="g.fill" aria-hidden="true"></span>
                    <app-outcome-icon [outcome]="g.icon" />
                    {{ g.label }}
                  </span>
                </td>
                <td class="num !py-2.5 font-bold" [class.text-muted]="!g.count">{{ g.count }}</td>
              </tr>
            }
          </tbody>
        </table>
        <p class="mt-2 text-[0.95rem] text-muted">
          Counts of calls in this {{ store.isDemo() ? 'demo' : 'server' }} record, not a protection
          score.
        </p>
      </section>
    </div>

    <section class="mt-12" aria-labelledby="recent-heading">
      <div class="flex items-baseline justify-between gap-4 border-b-2 border-ink pb-2">
        <h2 id="recent-heading" class="text-[1.375rem] font-extrabold">Recent calls</h2>
        <a
          routerLink="/console/calls"
          queryParamsHandling="preserve"
          class="inline-flex items-center gap-1 text-[1rem] font-bold text-ink"
          >All calls<svg lucideChevronRight [size]="17" aria-hidden="true"></svg
        ></a>
      </div>
      <ul class="divide-y divide-paper-3 md:hidden">
        @for (call of recent(); track call.id) {
          <li>
            <a
              [routerLink]="['/console/calls', call.id]"
              queryParamsHandling="preserve"
              class="block py-3.5 text-ink no-underline"
            >
              <span class="flex items-baseline justify-between gap-3">
                <app-outcome-text [outcome]="call.outcome" />
                <span class="text-[0.95rem] font-semibold text-muted">{{
                  age(call.startedAt ?? call.lastActivityAt)
                }}</span>
              </span>
              <span class="mt-1 line-clamp-2 block text-[1.0625rem]">{{ call.request }}</span>
            </a>
          </li>
        }
      </ul>
      <div class="hidden md:block">
        <table class="data-table">
          <thead class="visually-hidden">
            <tr>
              <th scope="col">When</th>
              <th scope="col">Caller</th>
              <th scope="col">What they wanted</th>
              <th scope="col">Outcome</th>
              <th scope="col">Length</th>
              <th scope="col">Open</th>
            </tr>
          </thead>
          <tbody>
            @for (call of recent(); track call.id) {
              <tr class="row-link" (click)="open(call)">
                <td class="w-[7.5rem] whitespace-nowrap">
                  <a
                    [routerLink]="['/console/calls', call.id]"
                    queryParamsHandling="preserve"
                    class="font-bold text-ink no-underline hover:underline"
                    (click)="$event.stopPropagation()"
                    >{{ age(call.startedAt ?? call.lastActivityAt) }}</a
                  >
                </td>
                <td class="w-[13rem] whitespace-nowrap text-ink-3">{{ callerLabel(call) }}</td>
                <td>
                  <span class="line-clamp-1">{{ call.request }}</span>
                </td>
                <td class="w-[12rem]"><app-outcome-text [outcome]="call.outcome" /></td>
                <td class="num w-[5rem] text-ink-3">{{ duration(call) }}</td>
                <td class="w-8 text-muted">
                  <svg lucideChevronRight [size]="18" aria-hidden="true"></svg>
                </td>
              </tr>
            } @empty {
              <tr>
                <td colspan="6" class="!py-8 text-center text-[1.0625rem] text-ink-3">
                  No calls yet.
                </td>
              </tr>
            }
          </tbody>
        </table>
      </div>
    </section>

    <section class="mt-12" aria-labelledby="more-heading">
      <h2 id="more-heading" class="visually-hidden">More about this line</h2>
      <div class="tab-row" role="tablist" aria-label="More about this line">
        @for (t of tabs(); track t.id) {
          <button
            type="button"
            role="tab"
            class="tab"
            [id]="'tab-' + t.id"
            [attr.aria-selected]="tab() === t.id"
            [attr.aria-controls]="'panel-' + t.id"
            [attr.tabindex]="tab() === t.id ? 0 : -1"
            (click)="tab.set(t.id)"
            (keydown)="onTabKey($event)"
          >
            {{ t.label }}
            @if (t.count !== undefined) {
              <span class="tab-count tabular">{{ t.count }}</span>
            }
          </button>
        }
      </div>

      <div
        class="pt-2"
        role="tabpanel"
        [id]="'panel-' + tab()"
        [attr.aria-labelledby]="'tab-' + tab()"
        tabindex="0"
      >
        @switch (tab()) {
          @case ('watch') {
            @if (watch().length) {
              <div class="table-scroll">
                <table class="data-table stack-table min-w-[38rem]">
                  <thead>
                    <tr>
                      <th scope="col">Caller</th>
                      <th scope="col">Reputation</th>
                      <th scope="col">Why</th>
                      <th scope="col" class="num">Calls</th>
                      <th scope="col">Last call</th>
                    </tr>
                  </thead>
                  <tbody>
                    @for (c of watch(); track c.id) {
                      <tr class="row-link" (click)="openCaller(c.id)">
                        <td class="font-bold whitespace-nowrap">
                          <a
                            [routerLink]="['/console/callers', c.id]"
                            queryParamsHandling="preserve"
                            class="text-ink no-underline hover:underline"
                            (click)="$event.stopPropagation()"
                            >{{ c.label }}</a
                          >
                        </td>
                        <td
                          data-label="Reputation"
                          class="font-bold whitespace-nowrap text-magenta-ink"
                        >
                          {{ standing(c.standing) }}
                        </td>
                        <td data-label="Why">{{ reasons(c.reputation?.scam_categories ?? []) }}</td>
                        <td data-label="Calls" class="num">{{ c.calls.length }}</td>
                        <td data-label="Last call" class="whitespace-nowrap">
                          {{ age(c.lastAt) }}
                        </td>
                      </tr>
                    }
                  </tbody>
                </table>
              </div>
            } @else {
              <p class="py-4 text-[1.0625rem] text-ink-3">
                No reported numbers. A number is reported when FirstRing blocks it for asking for a
                code, a password, an SSN, a payment or remote access.
              </p>
            }
            <a routerLink="/console/callers" queryParamsHandling="preserve" class="more-link"
              >All callers<svg lucideChevronRight [size]="17" aria-hidden="true"></svg
            ></a>
          }
          @case ('warnings') {
            @if (tactics().length) {
              <ul class="divide-y divide-paper-3">
                @for (t of tactics(); track t.id) {
                  <li class="flex flex-wrap items-baseline justify-between gap-x-6 gap-y-1 py-3">
                    <span>
                      <span class="block text-[1.0625rem] font-bold">{{
                        tacticLabel(t.tactic)
                      }}</span>
                      <span class="text-[1rem] text-ink-3">{{
                        pretextLabel(t.pretext_category)
                      }}</span>
                    </span>
                    <span class="tabular text-[1rem] text-muted"
                      >Version {{ t.version }} · expires {{ until(t.expires_at) }}</span
                    >
                  </li>
                }
              </ul>
            } @else {
              <p class="py-4 text-[1.0625rem] text-ink-3">No active shared warnings right now.</p>
            }
            <a routerLink="/console/community" queryParamsHandling="preserve" class="more-link"
              >How warnings travel between lines<svg
                lucideChevronRight
                [size]="17"
                aria-hidden="true"
              ></svg
            ></a>
          }
          @case ('trusted') {
            @if (trusted.unavailable()) {
              <p class="py-4 text-[1.0625rem] text-ink-3">
                This server does not offer trusted callers yet.
              </p>
            } @else if (trusted.items().length) {
              <ul class="divide-y divide-paper-3">
                @for (t of trusted.items(); track t.id) {
                  <li class="flex flex-wrap items-baseline justify-between gap-x-6 gap-y-1 py-3">
                    <span class="text-[1.0625rem]"
                      ><strong>{{ t.name }}</strong>
                      @if (t.relation) {
                        <span class="text-ink-3"> · {{ t.relation }}</span>
                      }
                    </span>
                    <span class="tabular text-[1rem] text-muted">Number {{ t.number_label }}</span>
                  </li>
                }
              </ul>
            } @else {
              <p class="py-4 text-[1.0625rem] text-ink-3">No trusted callers on this line.</p>
            }
            <p class="mt-2 max-w-[70ch] text-[1rem] text-muted">
              Calls from these numbers skip screening and ring straight through. Caller ID can be
              faked, so only numbers you would answer anyway belong here.
            </p>
            <a
              routerLink="/console/settings"
              fragment="trusted"
              queryParamsHandling="preserve"
              class="more-link"
              >Manage trusted callers<svg lucideChevronRight [size]="17" aria-hidden="true"></svg
            ></a>
          }
          @case ('system') {
            <dl class="divide-y divide-paper-3 text-[1rem]">
              @for (row of system(); track row.label) {
                <div class="grid gap-x-6 gap-y-0.5 py-3 sm:grid-cols-[11rem_minmax(0,1fr)]">
                  <dt class="font-semibold text-muted">{{ row.label }}</dt>
                  <dd>
                    <span class="font-semibold">{{ row.value }}</span>
                    @if (row.note) {
                      <span class="block text-[0.95rem] text-muted">{{ row.note }}</span>
                    }
                  </dd>
                </div>
              }
            </dl>
          }
        }
      </div>
    </section>
  `,
})
export class OverviewPage {
  readonly store = inject(ConsoleStore);
  readonly trusted = inject(TrustedDirectory);
  private readonly directory = inject(CallerDirectory);
  private readonly router = inject(Router);
  /** Only this line's calls: the page is titled for one line. */
  readonly calls = computed(() =>
    this.store.calls().filter((c) => c.session.household_id === this.householdId()),
  );
  /** Today when there are calls today, otherwise everything on record. */
  private readonly chosenPeriod = signal<'today' | 'all' | null>(null);
  readonly period = computed(() => this.chosenPeriod() ?? (this.todayCount() ? 'today' : 'all'));
  readonly tab = signal<Tab>('watch');

  readonly today = computed(() =>
    formatDay(this.store.now(), this.store.state()?.profile?.timezone),
  );
  readonly householdId = computed(() => {
    const state = this.store.state();
    return state?.profile?.household_id ?? state?.selected_household ?? 'sarah';
  });
  readonly lineName = computed(() => {
    const state = this.store.state();
    return state?.profile?.display_name ?? household(this.householdId())?.name ?? 'Your';
  });
  readonly number = computed(() => {
    const raw = this.store.state()?.phone_display;
    return isE164(raw) ? formatPhone(raw) : null;
  });
  readonly intro = computed(() => {
    const h = household(this.householdId());
    const number = this.number();
    const who = h ? `${h.name}, ${h.people[0].relation}` : this.lineName();
    const first = h
      ? ` ${h.ringsFirst.name}’s phone rings only after she hears a private summary and says yes.`
      : '';
    return `FirstRing answers ${number ?? 'this line'} for ${who}, before anyone picks up.${first}`;
  });

  readonly live = computed(() => this.calls().find((c) => c.outcome === 'in_progress'));
  readonly featured = computed<CallRecord | undefined>(() => {
    const calls = this.calls();
    const now = this.store.now();
    const recent = (c: CallRecord) => now - Date.parse(c.startedAt ?? '') < 24 * 3600 * 1000;
    return (
      calls.find((c) => c.outcome === 'blocked' && recent(c)) ??
      calls.find((c) => c.outcome === 'needs_review' && recent(c)) ??
      calls.find((c) => c.outcome !== 'in_progress')
    );
  });
  /** The caller's own words: the quote the AI cited, else their last turn. */
  readonly quote = computed(() => {
    const call = this.featured();
    if (!call || call.trusted) return '';
    const said =
      call.session.evidence_quote ??
      call.session.turns.filter((t) => t.speaker === 'caller').at(-1)?.text ??
      '';
    const text = said.trim();
    return text.length > 180 ? text.slice(0, 177).trimEnd() + '…' : text;
  });
  readonly featuredLabel = computed(() => {
    const call = this.featured();
    if (call?.outcome === 'blocked') return 'Most important call';
    if (call?.outcome === 'needs_review') return 'Needs a look';
    return 'Latest call';
  });
  readonly action = computed(() => {
    const call = this.featured();
    return call ? personAction(call).text : '';
  });
  readonly recent = computed(() =>
    this.calls()
      .filter((c) => c.id !== this.featured()?.id)
      .slice(0, 6),
  );
  readonly watch = computed(() =>
    this.directory
      .callers()
      .filter((c) => c.standing === 'reported' || c.standing === 'warned')
      .slice(0, 6),
  );

  private readonly todayCount = computed(() => {
    const tz = this.store.state()?.profile?.timezone;
    const today = formatDay(this.store.now(), tz);
    return this.calls().filter((c) => formatDay(c.startedAt ?? c.lastActivityAt, tz) === today)
      .length;
  });
  private readonly periodCalls = computed(() => {
    const calls = this.calls();
    if (this.period() === 'all') return calls;
    const tz = this.store.state()?.profile?.timezone;
    const today = formatDay(this.store.now(), tz);
    return calls.filter((c) => formatDay(c.startedAt ?? c.lastActivityAt, tz) === today);
  });
  readonly mix = computed(() => {
    const calls = this.periodCalls();
    return GROUPS.map((g) => ({
      ...g,
      count: calls.filter((c) => g.members.includes(c.outcome)).length,
    }));
  });
  readonly mixTotal = computed(() => this.mix().reduce((sum, g) => sum + g.count, 0));
  readonly mixLine = computed(() => {
    const total = this.periodCalls().length;
    const live = this.periodCalls().filter((c) => c.outcome === 'in_progress').length;
    const scope = this.period() === 'today' ? 'today' : 'on record';
    if (!total) return this.period() === 'today' ? 'No calls yet today.' : 'No calls on record.';
    return `${total} ${total === 1 ? 'call' : 'calls'} ${scope}${live ? `, ${live} on the line now` : ''}.`;
  });
  readonly mixLabel = computed(
    () =>
      'Outcome proportions: ' +
      this.mix()
        .filter((g) => g.count)
        .map((g) => `${g.label} ${g.count}`)
        .join(', '),
  );

  readonly tactics = computed(() =>
    [...(this.store.state()?.tactics ?? [])].sort((a, b) => b.version - a.version).slice(0, 4),
  );
  readonly tabs = computed(() => [
    { id: 'watch' as const, label: 'Callers to watch', count: this.watch().length },
    { id: 'warnings' as const, label: 'Shared warnings', count: this.tactics().length },
    {
      id: 'trusted' as const,
      label: 'Trusted callers',
      count: this.trusted.unavailable() ? undefined : this.trusted.items().length,
    },
    { id: 'system' as const, label: 'How this line runs', count: undefined },
  ]);

  readonly system = computed(() => {
    const state = this.store.state()!;
    if (this.store.isDemo())
      return [
        {
          label: 'Data',
          value: 'Fictional demo calls',
          note: 'No phone line or model is connected.',
        },
        { label: 'AI model', value: 'NVIDIA Nemotron 3.5 Lightning', note: 'Used on live calls.' },
        {
          label: 'The family',
          value: FAMILY['sarah'].people.map((p) => `${p.name} (${p.relation})`).join(', '),
          note: 'Maya and Jimmy run their own line and share warnings with Sarah’s.',
        },
      ];
    const current = state.model_backend?.current;
    const until = current?.available_until;
    const speech = state.speech;
    const reputation = state.capabilities.find((c) => c.id === 'reputation');
    return [
      {
        label: 'Phone line',
        value: this.number() ?? 'Not configured',
        note: 'Photon SIP number into Twilio Programmable Voice',
      },
      {
        label: 'Conversation',
        value: state.transport === 'relay' ? 'Twilio ConversationRelay' : state.transport,
        note: speech
          ? `${speech.transcription_provider ?? 'Speech'} ${speech.transcription_model ?? ''} listens; ${speech.provider} speaks (${speech.voice_label ?? 'preset voice'})`
          : undefined,
      },
      {
        label: 'AI model',
        value: 'NVIDIA Nemotron 3.5 Lightning 30B-A3B',
        note: current?.available
          ? `Running on a Brev H100${until ? ` until ${formatDateTime(until, state.profile?.timezone)}` : ''}. It proposes; Rust rules decide.`
          : `Not available right now (${(current?.reason ?? 'unknown').replaceAll('_', ' ')})`,
      },
      {
        label: 'Rings first',
        value: state.phone_demo?.recipient_enrolled
          ? `${household(this.householdId())?.ringsFirst.name ?? 'Enrolled'}’s phone`
          : 'Not enrolled',
        note:
          state.phone_demo?.state === 'armed'
            ? 'Armed for the next call'
            : 'Off until armed in Settings or on the presentation screen',
      },
      {
        label: 'Reputation',
        value: reputation?.status === 'configured' ? 'On' : 'Off',
        note: 'Numbers are kept as one-way codes; reports expire after 14 days',
      },
      {
        label: 'The family',
        value: FAMILY['sarah'].people.map((p) => `${p.name} (${p.relation})`).join(', '),
        note: 'Maya and Jimmy run their own line and share warnings with Sarah’s.',
      },
    ];
  });

  callerLabel(call: CallRecord) {
    if (call.trusted) return `${call.trusted.name} · trusted`;
    const c = this.directory.forCall(call.id);
    if (!c || !c.captured) return 'Number not captured';
    return c.standing === 'reported' || c.standing === 'warned' ? `${c.label} · reported` : c.label;
  }
  standing(value: keyof typeof STANDING_LABEL) {
    return STANDING_LABEL[value];
  }
  reasons(values: string[]) {
    return values.length ? values.map(categoryLabel).join('; ') : '—';
  }
  when(call: CallRecord) {
    return formatDateTime(call.startedAt ?? call.lastActivityAt, call.timezone);
  }
  age(value: string | undefined) {
    return value ? relativeAge(value, this.store.now()) : '—';
  }
  until(at: string) {
    return relativeUntil(at, this.store.now());
  }
  duration(call: CallRecord) {
    return formatDuration(call.durationSeconds);
  }
  tacticLabel(value: string) {
    return humanize(value, TACTIC_LABEL);
  }
  pretextLabel(value: string) {
    return humanize(value, PRETEXT_LABEL);
  }
  open(call: CallRecord) {
    void this.router.navigate(['/console/calls', call.id], { queryParamsHandling: 'preserve' });
  }
  openCaller(id: string) {
    void this.router.navigate(['/console/callers', id], { queryParamsHandling: 'preserve' });
  }
  onTabKey(event: KeyboardEvent) {
    const ids = this.tabs().map((t) => t.id);
    const at = ids.indexOf(this.tab());
    let next = at;
    if (event.key === 'ArrowRight') next = (at + 1) % ids.length;
    else if (event.key === 'ArrowLeft') next = (at - 1 + ids.length) % ids.length;
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = ids.length - 1;
    else return;
    event.preventDefault();
    this.tab.set(ids[next]);
    queueMicrotask(() => document.getElementById(`tab-${ids[next]}`)?.focus());
  }
}
