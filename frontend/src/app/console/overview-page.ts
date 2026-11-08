import { Component, computed, inject } from '@angular/core';
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
import { personAction, presentFocus } from '../core/live';
import { OUTCOME_SHORT, OutcomeIcon, OutcomeText } from './outcome-chip';

const ORDER: Outcome[] = [
  'blocked',
  'connected',
  'message_saved',
  'needs_review',
  'no_message',
  'incomplete',
];

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
    <header class="flex flex-wrap items-end justify-between gap-x-8 gap-y-3">
      <div>
        <p class="text-[1rem] font-semibold text-muted">{{ today() }}</p>
        <h1 class="mt-1 text-[2.25rem] leading-[1.05] font-extrabold tracking-[-0.03em]">
          {{ lineName() }}’s line
        </h1>
        <p class="mt-2 max-w-[72ch] text-[1.0625rem] text-ink-3">{{ intro() }}</p>
      </div>
    </header>

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

    <div class="mt-8 grid gap-x-10 gap-y-10 xl:grid-cols-[minmax(0,1fr)_24rem]">
      <div class="min-w-0 space-y-10">
        @if (featured(); as call) {
          <section aria-labelledby="featured-heading">
            <div
              class="flex flex-wrap items-baseline justify-between gap-3 border-b-2 border-ink pb-2"
            >
              <h2 id="featured-heading" class="text-[1.375rem] font-extrabold">
                {{ featuredLabel() }}
              </h2>
              <p class="text-[1rem] text-muted">{{ when(call) }} · {{ call.name }}’s line</p>
            </div>
            <div class="grid gap-x-8 gap-y-4 py-5 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
              <div>
                <app-outcome-text [outcome]="call.outcome" [short]="false" />
                <p class="mt-2 text-[1.375rem] leading-snug font-extrabold tracking-[-0.01em]">
                  {{ call.headline }}
                </p>
              </div>
              <dl class="grid gap-y-3 text-[1rem]">
                <div>
                  <dt class="font-semibold text-muted">The caller wanted</dt>
                  <dd class="font-semibold">{{ call.request }}</dd>
                </div>
                <div>
                  <dt class="font-semibold text-muted">FirstRing</dt>
                  <dd class="font-semibold">{{ glance()!.did }}</dd>
                </div>
                <div>
                  <dt class="font-semibold text-muted">Does anyone need to act?</dt>
                  <dd class="font-semibold">{{ glance()!.action }}</dd>
                </div>
              </dl>
            </div>
            <a
              [routerLink]="['/console/calls', call.id]"
              queryParamsHandling="preserve"
              class="btn btn-ink no-underline"
            >
              See what the AI heard and why it acted
              <svg lucideArrowRight [size]="18" aria-hidden="true"></svg>
            </a>
          </section>
        }

        <section aria-labelledby="recent-heading">
          <div class="flex items-baseline justify-between gap-4 border-b-2 border-ink pb-2">
            <h2 id="recent-heading" class="text-[1.375rem] font-extrabold">Recent calls</h2>
            <a
              routerLink="/console/calls"
              queryParamsHandling="preserve"
              class="text-[1rem] font-bold text-ink"
              >All {{ calls().length }} calls</a
            >
          </div>
          <div class="table-scroll">
            <table class="data-table min-w-[44rem]">
              <thead>
                <tr>
                  <th scope="col">When</th>
                  <th scope="col">Caller</th>
                  <th scope="col" class="w-[45%]">What they wanted</th>
                  <th scope="col">Outcome</th>
                  <th scope="col" class="num">Length</th>
                  <th scope="col"><span class="visually-hidden">Open</span></th>
                </tr>
              </thead>
              <tbody>
                @for (call of recent(); track call.id) {
                  <tr class="row-link" (click)="open(call)">
                    <td class="whitespace-nowrap">
                      <a
                        [routerLink]="['/console/calls', call.id]"
                        queryParamsHandling="preserve"
                        class="font-bold text-ink no-underline hover:underline"
                        (click)="$event.stopPropagation()"
                        >{{ age(call.startedAt ?? call.lastActivityAt) }}</a
                      >
                    </td>
                    <td class="whitespace-nowrap">{{ callerLabel(call) }}</td>
                    <td>
                      <span class="line-clamp-2">{{ call.request }}</span>
                    </td>
                    <td><app-outcome-text [outcome]="call.outcome" /></td>
                    <td class="num">{{ duration(call) }}</td>
                    <td class="w-8 text-muted">
                      <svg lucideChevronRight [size]="18" aria-hidden="true"></svg>
                    </td>
                  </tr>
                } @empty {
                  <tr>
                    <td colspan="6" class="!py-8 text-center text-[1.0625rem] text-ink-3">
                      No calls yet. When someone calls {{ number() ?? 'the line' }}, the call and
                      its reasons appear here.
                    </td>
                  </tr>
                }
              </tbody>
            </table>
          </div>
        </section>

        <section aria-labelledby="watch-heading">
          <div class="flex items-baseline justify-between gap-4 border-b-2 border-ink pb-2">
            <h2 id="watch-heading" class="text-[1.375rem] font-extrabold">Callers to watch</h2>
            <a
              routerLink="/console/callers"
              queryParamsHandling="preserve"
              class="text-[1rem] font-bold text-ink"
              >All callers</a
            >
          </div>
          @if (watch().length) {
            <div class="table-scroll">
              <table class="data-table min-w-[40rem]">
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
                      <td class="whitespace-nowrap font-bold">
                        <a
                          [routerLink]="['/console/callers', c.id]"
                          queryParamsHandling="preserve"
                          class="text-ink no-underline hover:underline"
                          (click)="$event.stopPropagation()"
                          >{{ c.label }}</a
                        >
                      </td>
                      <td class="whitespace-nowrap font-bold text-magenta-ink">
                        {{ standing(c.standing) }}
                      </td>
                      <td>{{ reasons(c.reputation?.scam_categories ?? []) }}</td>
                      <td class="num">{{ c.calls.length }}</td>
                      <td class="whitespace-nowrap">{{ age(c.lastAt) }}</td>
                    </tr>
                  }
                </tbody>
              </table>
            </div>
          } @else {
            <p class="py-5 text-[1.0625rem] text-ink-3">
              No reported numbers. A number is reported when FirstRing blocks it for asking for a
              code, a password, an SSN, a payment or remote access.
            </p>
          }
        </section>
      </div>

      <aside class="min-w-0 space-y-10" aria-label="Summary">
        <section aria-labelledby="outcomes-heading">
          <h2
            id="outcomes-heading"
            class="border-b-2 border-ink pb-2 text-[1.375rem] font-extrabold"
          >
            Outcomes
          </h2>
          <table class="data-table">
            <thead>
              <tr>
                <th scope="col">Outcome</th>
                <th scope="col" class="num">Today</th>
                <th scope="col" class="num">All</th>
                <th scope="col" class="w-[30%]"><span class="visually-hidden">Share</span></th>
              </tr>
            </thead>
            <tbody>
              @for (row of outcomes(); track row.outcome) {
                <tr>
                  <td class="whitespace-nowrap">
                    <span class="inline-flex items-center gap-2 font-semibold">
                      <app-outcome-icon [outcome]="row.outcome" />{{ row.label }}
                    </span>
                  </td>
                  <td class="num">{{ row.today }}</td>
                  <td class="num font-bold">{{ row.all }}</td>
                  <td class="!align-middle">
                    <div
                      class="h-2 rounded-r-[4px] bg-ink-3"
                      [style.width.%]="row.share"
                      [attr.title]="row.label + ': ' + row.all + ' of ' + calls().length + ' calls'"
                    ></div>
                  </td>
                </tr>
              }
            </tbody>
          </table>
        </section>

        <section aria-labelledby="family-heading">
          <h2 id="family-heading" class="border-b-2 border-ink pb-2 text-[1.375rem] font-extrabold">
            The family
          </h2>
          <ul class="divide-y divide-paper-3">
            @for (person of family; track person.name + person.line) {
              <li class="py-3">
                <p class="text-[1.0625rem] font-extrabold">
                  {{ person.name }}
                  <span class="font-semibold text-muted">· {{ person.relation }}</span>
                </p>
                <p class="text-[1rem] text-ink-3">{{ person.role }}</p>
              </li>
            }
          </ul>
        </section>

        <section aria-labelledby="system-heading">
          <h2 id="system-heading" class="border-b-2 border-ink pb-2 text-[1.375rem] font-extrabold">
            How this line runs
          </h2>
          <dl class="divide-y divide-paper-3 text-[1rem]">
            @for (row of system(); track row.label) {
              <div class="grid grid-cols-[8.5rem_minmax(0,1fr)] gap-3 py-2.5">
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
        </section>

        <section aria-labelledby="warning-heading">
          <h2
            id="warning-heading"
            class="border-b-2 border-ink pb-2 text-[1.375rem] font-extrabold"
          >
            Shared warnings
          </h2>
          @if (tactics().length) {
            <ul class="divide-y divide-paper-3">
              @for (t of tactics(); track t.id) {
                <li class="py-3">
                  <p class="text-[1.0625rem] font-bold">{{ tacticLabel(t.tactic) }}</p>
                  <p class="text-[1rem] text-ink-3">
                    {{ pretextLabel(t.pretext_category) }} · v{{ t.version }} · expires
                    {{ until(t.expires_at) }}
                  </p>
                </li>
              }
            </ul>
          } @else {
            <p class="py-3 text-[1rem] text-ink-3">No active shared warnings right now.</p>
          }
          <a
            routerLink="/console/community"
            queryParamsHandling="preserve"
            class="mt-1 inline-block text-[1rem] font-bold text-ink"
            >How warnings travel between lines</a
          >
        </section>
      </aside>
    </div>
  `,
})
export class OverviewPage {
  readonly store = inject(ConsoleStore);
  private readonly directory = inject(CallerDirectory);
  private readonly router = inject(Router);
  readonly calls = this.store.calls;

  readonly family = [
    ...FAMILY['sarah'].people.map((p) => ({ ...p, line: 'sarah' })),
    { ...FAMILY['maya'].people[0], line: 'maya' },
  ];

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
      ? ` ${h.ringsFirst.name}’s phone rings first when a caller is ready to talk.`
      : '';
    return `FirstRing answers ${number ?? 'this line'} for ${who}. It screens every caller before anyone picks up.${first}`;
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
  readonly featuredLabel = computed(() => {
    const call = this.featured();
    if (call?.outcome === 'blocked') return 'Most important call';
    if (call?.outcome === 'needs_review') return 'Needs a look';
    return 'Latest call';
  });
  readonly glance = computed(() => {
    const call = this.featured();
    if (!call) return undefined;
    return { did: presentFocus(call).outcome.label, action: personAction(call).text };
  });
  readonly recent = computed(() => this.calls().slice(0, 8));
  readonly watch = computed(() =>
    this.directory
      .callers()
      .filter((c) => c.standing === 'reported' || c.standing === 'warned')
      .slice(0, 5),
  );

  readonly outcomes = computed(() => {
    const calls = this.calls();
    const tz = this.store.state()?.profile?.timezone;
    const today = formatDay(this.store.now(), tz);
    const max = Math.max(1, ...ORDER.map((o) => calls.filter((c) => c.outcome === o).length));
    return ORDER.map((outcome) => {
      const own = calls.filter((c) => c.outcome === outcome);
      return {
        outcome,
        label: OUTCOME_SHORT[outcome],
        all: own.length,
        today: own.filter((c) => formatDay(c.startedAt ?? c.lastActivityAt, tz) === today).length,
        share: (own.length / max) * 100,
      };
    });
  });

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
          ? `Running on a Brev H100${until ? ` until ${formatDateTime(until, state.profile?.timezone)}` : ''}; verified with a live tool cycle`
          : `Not available right now (${(current?.reason ?? 'unknown').replaceAll('_', ' ')})`,
      },
      {
        label: 'Ring first',
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
    ];
  });

  readonly tactics = computed(() =>
    [...(this.store.state()?.tactics ?? [])].sort((a, b) => b.version - a.version).slice(0, 3),
  );

  callerLabel(call: CallRecord) {
    const c = this.directory.forCall(call.id);
    if (!c || !c.captured) return '—';
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
}
