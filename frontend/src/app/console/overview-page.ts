import { Component, computed, inject } from '@angular/core';
import { RouterLink } from '@angular/router';
import {
  LucideArrowRight,
  LucideCircleCheck,
  LucideCircleAlert,
  LucidePause,
  LucideSettings2,
} from '@lucide/angular';
import type { CallRecord } from '../core/calls';
import { ConsoleStore } from '../core/console-store';
import { formatDateTime, formatDay, relativeAge } from '../core/format';
import { humanize, PRETEXT_LABEL, TACTIC_LABEL } from '../core/language';
import { CallRow } from './call-row';
import { personAction, presentFocus } from '../core/live';
import { OutcomeChip } from './outcome-chip';

interface Service {
  tone: 'ok' | 'paused' | 'setup' | 'demo' | 'replay';
  title: string;
  detail: string;
}

@Component({
  selector: 'app-overview-page',
  imports: [
    RouterLink,
    CallRow,
    OutcomeChip,
    LucideArrowRight,
    LucideCircleCheck,
    LucideCircleAlert,
    LucidePause,
    LucideSettings2,
  ],
  template: `
    @let state = store.state()!;
    <div class="grid gap-x-10 gap-y-8 lg:grid-cols-12">
      <div class="lg:col-span-8">
        <p class="text-[1.0625rem] font-semibold text-muted">{{ today() }}</p>
        <h1
          class="mt-1 text-[clamp(2.1rem,1.6rem+1.8vw,3.25rem)] leading-[1.05] font-extrabold tracking-[-0.03em]"
        >
          Calls for {{ profileName() }}
        </h1>

        <div
          class="mt-5 flex items-start gap-3 rounded-xl px-4 py-3.5"
          [class]="serviceClass()"
          role="status"
        >
          @switch (service().tone) {
            @case ('ok') {
              <svg lucideCircleCheck [size]="22" class="mt-0.5 shrink-0" aria-hidden="true"></svg>
            }
            @case ('paused') {
              <svg lucidePause [size]="22" class="mt-0.5 shrink-0" aria-hidden="true"></svg>
            }
            @default {
              <svg lucideCircleAlert [size]="22" class="mt-0.5 shrink-0" aria-hidden="true"></svg>
            }
          }
          <p class="max-w-[62ch] text-[1.0625rem]">
            <strong>{{ service().title }}</strong> {{ service().detail }}
          </p>
        </div>

        @if (featured(); as call) {
          <section
            class="mt-8 overflow-hidden rounded-[var(--radius-card)] border-2 border-ink"
            aria-labelledby="featured-heading"
          >
            <div
              class="flex flex-wrap items-center justify-between gap-3 bg-ink px-5 py-3 text-paper"
            >
              <h2 id="featured-heading" class="text-[1.125rem] font-extrabold">
                {{ featuredLabel() }}
              </h2>
              <time
                class="text-[1rem] font-semibold text-paper/85"
                [attr.datetime]="call.startedAt"
                >{{ when(call) }}</time
              >
            </div>
            <div class="bg-paper p-5 sm:p-7">
              <app-outcome-chip [outcome]="call.outcome" />
              <p
                class="mt-4 max-w-[48ch] text-[clamp(1.4rem,1.2rem+0.8vw,1.875rem)] leading-snug font-extrabold tracking-[-0.015em]"
              >
                {{ call.headline }}
              </p>
              @let glance = featuredGlance()!;
              <dl class="mt-5 grid max-w-[60rem] gap-x-8 border-t border-paper-3 sm:grid-cols-3">
                <div class="border-b border-paper-3 py-3 sm:border-b-0">
                  <dt class="text-[0.95rem] font-bold text-muted">The caller asked</dt>
                  <dd class="mt-0.5 text-[1.0625rem] font-semibold">{{ call.request }}</dd>
                </div>
                <div class="border-b border-paper-3 py-3 sm:border-b-0">
                  <dt class="text-[0.95rem] font-bold text-muted">FirstRing</dt>
                  <dd class="mt-0.5 text-[1.0625rem] font-semibold">{{ glance.did }}</dd>
                </div>
                <div class="py-3">
                  <dt class="text-[0.95rem] font-bold text-muted">Do you need to do anything?</dt>
                  <dd class="mt-0.5 text-[1.0625rem] font-semibold">{{ glance.action }}</dd>
                </div>
              </dl>
              <div class="mt-6 flex flex-wrap gap-3">
                <a
                  [routerLink]="['/console/calls', call.id]"
                  queryParamsHandling="preserve"
                  class="btn btn-ink"
                >
                  See what FirstRing heard and why
                  <svg lucideArrowRight [size]="18" aria-hidden="true"></svg>
                </a>
                @if (call.outcome === 'in_progress') {
                  <a
                    routerLink="/console/present"
                    queryParamsHandling="preserve"
                    class="btn btn-secondary"
                  >
                    Show it on the presentation screen
                  </a>
                }
              </div>
            </div>
          </section>
        }

        <section class="mt-10" aria-labelledby="recent-heading">
          <div class="flex items-end justify-between gap-4 border-b-2 border-ink pb-3">
            <h2 id="recent-heading" class="text-[1.5rem] font-extrabold">Recent calls</h2>
            @if (calls().length) {
              <a
                routerLink="/console/calls"
                queryParamsHandling="preserve"
                class="text-[1.0625rem] font-bold text-ink"
                >All {{ calls().length }} calls</a
              >
            }
          </div>
          @if (recent().length) {
            <ul class="mt-2 divide-y divide-paper-3">
              @for (call of recent(); track call.id) {
                <li><app-call-row [call]="call" /></li>
              }
            </ul>
          } @else {
            <div class="mt-8 flex flex-col items-center gap-4 text-center">
              <img src="media/ringo-rest-480.webp" width="480" height="480" alt="" class="w-48" />
              <p class="max-w-[40ch] text-[1.125rem] font-semibold">No calls yet.</p>
              <p class="max-w-[46ch] text-[1.0625rem] text-ink-3">
                When someone calls {{ state.phone_display }}, the call and its reasons will appear
                here.
              </p>
            </div>
          }
        </section>
      </div>

      <aside class="space-y-6 lg:col-span-4" aria-label="Summary">
        <section
          class="rounded-[var(--radius-card)] bg-paper-2 p-5 sm:p-6"
          aria-labelledby="counts-heading"
        >
          <h2 id="counts-heading" class="text-[1.1875rem] font-extrabold">On record</h2>
          <dl class="mt-4 space-y-3">
            @for (count of counts(); track count.label) {
              <div class="flex items-baseline justify-between gap-4 border-b border-rule pb-2">
                <dt class="text-[1.0625rem]">{{ count.label }}</dt>
                <dd class="tabular text-[1.25rem] font-extrabold">{{ count.value }}</dd>
              </div>
            }
          </dl>
          <p class="mt-3 text-[0.95rem] text-muted">
            Counts of calls in this {{ store.isDemo() ? 'demo' : 'server' }} snapshot, not a
            protection score.
          </p>
        </section>

        <section
          class="rounded-[var(--radius-card)] border-2 border-paper-3 p-5 sm:p-6"
          aria-labelledby="community-update"
        >
          <h2 id="community-update" class="text-[1.1875rem] font-extrabold">Community update</h2>
          @if (latestTactic(); as tactic) {
            <p class="mt-3 text-[1.125rem] font-bold">{{ tacticLabel(tactic.tactic) }}</p>
            <p class="text-[1.0625rem] text-ink-3">
              {{ pretextLabel(tactic.pretext_category) }} · shared warning v{{ tactic.version }}
            </p>
            <p class="mt-2 text-[1rem] text-muted">
              Expires {{ expiry(tactic.expires_at) }}. {{ appliedLine() }}
            </p>
          } @else {
            <p class="mt-3 text-[1.0625rem] text-ink-3">No active shared warnings right now.</p>
          }
          <a
            routerLink="/console/community"
            queryParamsHandling="preserve"
            class="mt-4 inline-block text-[1.0625rem] font-bold text-ink"
            >Open Community</a
          >
        </section>

        <section
          class="rounded-[var(--radius-card)] border-2 border-paper-3 p-5 sm:p-6"
          aria-labelledby="profile-card"
        >
          <h2 id="profile-card" class="text-[1.1875rem] font-extrabold">
            Who FirstRing answers for
          </h2>
          <p class="mt-3 text-[1.0625rem]">
            Next call: <strong>{{ profileName() }}</strong>
          </p>
          @if (state.profile; as profile) {
            <p class="mt-1 text-[1.0625rem] text-ink-3">Greeting: “{{ profile.greeting }}”</p>
          }
          <a
            routerLink="/console/settings"
            queryParamsHandling="preserve"
            class="btn btn-quiet mt-4"
            ><svg lucideSettings2 [size]="17" aria-hidden="true"></svg> Settings</a
          >
        </section>
      </aside>
    </div>
  `,
})
export class OverviewPage {
  readonly store = inject(ConsoleStore);
  readonly calls = this.store.calls;
  readonly today = computed(() =>
    formatDay(this.store.now(), this.store.state()?.profile?.timezone),
  );
  readonly profileName = computed(() => {
    const state = this.store.state();
    return (
      state?.profile?.display_name ??
      state?.community.find((h) => h.household_id === state.selected_household)?.display_name ??
      'your household'
    );
  });

  readonly service = computed<Service>(() => {
    const state = this.store.state()!;
    const connection = this.store.connection();
    if (connection === 'demo')
      return {
        tone: 'demo',
        title: 'Service status appears here when connected.',
        detail: 'In the demo, there is no phone line to report on.',
      };
    if (connection === 'paused')
      return {
        tone: 'paused',
        title: 'Updates paused.',
        detail:
          'The connection to the server dropped; the last snapshot is shown while it reconnects.',
      };
    if (state.execution_mode === 'replay')
      return {
        tone: 'replay',
        title: 'Replay mode.',
        detail: 'The server is running authored scenarios. No real calls are answered.',
      };
    const inbound = state.capabilities.find((c) => c.id === 'inbound');
    const verified =
      state.evidence?.relay_phone.status === 'passed' ||
      state.evidence?.inbound_phone.status === 'passed';
    if (!inbound || inbound.status === 'pending' || inbound.status === 'disabled')
      return {
        tone: 'setup',
        title: 'Needs setup.',
        detail: 'The phone line is not configured yet. Operator detail lists what is missing.',
      };
    return {
      tone: 'ok',
      title: 'Answering calls.',
      detail: `The phone line is configured${verified ? ' and has a recorded test call' : ', but no test call has been recorded yet'}. ${state.phone_display !== 'Not configured' ? 'Number: ' + state.phone_display + '.' : ''}`,
    };
  });
  readonly serviceClass = computed(
    () =>
      ({
        ok: 'bg-lime-wash text-lime-ink',
        paused: 'bg-amber-wash text-amber-ink',
        setup: 'bg-amber-wash text-amber-ink',
        demo: 'bg-paper-2 text-ink',
        replay: 'bg-paper-2 text-ink',
      })[this.service().tone],
  );

  /** In progress first, then the newest blocked or needs-review call from the last day, else the newest call. */
  readonly featured = computed<CallRecord | undefined>(() => {
    const calls = this.calls();
    const now = this.store.now();
    const recent = (c: CallRecord) => now - Date.parse(c.startedAt ?? '') < 24 * 3600 * 1000;
    return (
      calls.find((c) => c.outcome === 'in_progress') ??
      calls.find((c) => c.outcome === 'blocked' && recent(c)) ??
      calls.find((c) => c.outcome === 'needs_review' && recent(c)) ??
      calls[0]
    );
  });
  readonly featuredGlance = computed(() => {
    const call = this.featured();
    if (!call) return undefined;
    const focus = presentFocus(call);
    const action = personAction(call).text;
    return {
      did:
        call.outcome === 'in_progress' ? (focus.step?.label ?? 'Answering') : focus.outcome.label,
      action,
    };
  });
  readonly featuredLabel = computed(() => {
    const call = this.featured();
    if (!call) return '';
    if (call.outcome === 'in_progress') return 'On the line now';
    if (call.outcome === 'blocked') return 'Most important call';
    if (call.outcome === 'needs_review') return 'Needs your review';
    return 'Latest call';
  });
  readonly recent = computed(() =>
    this.calls()
      .filter((c) => c.id !== this.featured()?.id)
      .slice(0, 5),
  );
  readonly counts = computed(() => {
    const calls = this.calls();
    const count = (o: CallRecord['outcome']) => calls.filter((c) => c.outcome === o).length;
    return [
      { label: 'Suspicious requests blocked', value: count('blocked') },
      { label: 'Messages saved', value: count('message_saved') },
      { label: 'Needs review', value: count('needs_review') },
      { label: 'Incomplete or no message', value: count('incomplete') + count('no_message') },
    ];
  });
  readonly latestTactic = computed(
    () => [...(this.store.state()?.tactics ?? [])].sort((a, b) => b.version - a.version)[0],
  );
  readonly appliedLine = computed(() => {
    const state = this.store.state();
    const household = state?.community.find((h) => h.household_id === state.selected_household);
    return household
      ? `${household.display_name}’s FirstRing has applied warnings up to v${household.last_applied_version}.`
      : '';
  });

  when(call: CallRecord) {
    return formatDateTime(call.startedAt ?? call.lastActivityAt, call.timezone);
  }
  expiry(at: string) {
    return formatDateTime(at, this.store.state()?.profile?.timezone);
  }
  tacticLabel(value: string) {
    return humanize(value, TACTIC_LABEL);
  }
  pretextLabel(value: string) {
    return humanize(value, PRETEXT_LABEL);
  }
  age(value: string | undefined) {
    return relativeAge(value, this.store.now());
  }
}
