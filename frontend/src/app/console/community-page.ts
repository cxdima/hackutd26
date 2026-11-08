import { Component, computed, inject, signal } from '@angular/core';
import { LucideChevronDown, LucideClock3, LucideShieldAlert, LucideUnplug } from '@lucide/angular';
import { ConsoleStore } from '../core/console-store';
import { formatDateTime, relativeAge, relativeUntil } from '../core/format';
import {
  ACTION_LABEL,
  humanize,
  PRESSURE_LABEL,
  PRETEXT_LABEL,
  TACTIC_LABEL,
} from '../core/language';
import type { Tactic } from '../core/schemas';

@Component({
  selector: 'app-community-page',
  imports: [LucideShieldAlert, LucideClock3, LucideChevronDown, LucideUnplug],
  template: `
    @let state = store.state()!;
    <div class="grid gap-10 lg:grid-cols-12">
      <div class="lg:col-span-8">
        <h1
          class="text-[clamp(2rem,1.6rem+1.6vw,2.9rem)] leading-[1.05] font-extrabold tracking-[-0.03em]"
        >
          Community
        </h1>
        <p class="mt-3 max-w-[60ch] text-lede text-ink-3">
          Shared warnings describe a trick, not a person. Each one lists what was asked for, the
          pretext and the pressure, then expires.
        </p>

        <section class="mt-10" aria-labelledby="active-heading">
          <h2 id="active-heading" class="border-b-2 border-ink pb-3 text-[1.5rem] font-extrabold">
            Active shared warnings
          </h2>
          <ul class="mt-5 space-y-5">
            @for (tactic of tactics(); track tactic.id) {
              <li class="overflow-hidden rounded-[var(--radius-card)] border-2 border-paper-3">
                <div
                  class="flex flex-wrap items-center justify-between gap-3 bg-ink px-5 py-3 text-paper"
                >
                  <h3 class="flex items-center gap-2 text-[1.1875rem] font-extrabold">
                    <svg
                      lucideShieldAlert
                      [size]="20"
                      class="text-magenta-on-ink"
                      aria-hidden="true"
                    ></svg>
                    {{ label(tactic.tactic, TACTIC) }}
                  </h3>
                  <span class="text-[1rem] font-semibold text-paper/85"
                    >Warning v{{ tactic.version }}</span
                  >
                </div>
                <dl class="grid gap-x-6 gap-y-3 p-5 sm:grid-cols-2">
                  <div>
                    <dt class="text-[0.9375rem] font-bold text-muted">Asks the person to</dt>
                    <dd class="text-[1.0625rem] font-semibold">
                      {{ label(tactic.requested_action, ACTION) }}
                    </dd>
                  </div>
                  <div>
                    <dt class="text-[0.9375rem] font-bold text-muted">Pretext</dt>
                    <dd class="text-[1.0625rem] font-semibold">
                      {{ label(tactic.pretext_category, PRETEXT) }}
                    </dd>
                  </div>
                  <div>
                    <dt class="text-[0.9375rem] font-bold text-muted">Pressure</dt>
                    <dd class="text-[1.0625rem] font-semibold">
                      {{ label(tactic.pressure_category, PRESSURE) }}
                    </dd>
                  </div>
                  <div>
                    <dt class="text-[0.9375rem] font-bold text-muted">Safer next step</dt>
                    <dd class="text-[1.0625rem] font-semibold">
                      {{ label(tactic.recommended_action ?? 'independent_contact', ACTION) }}
                    </dd>
                  </div>
                </dl>
                <div class="border-t border-paper-3 px-5 py-4 text-[1rem] text-ink-3">
                  <p><strong class="text-ink">Why it was flagged:</strong> {{ why(tactic) }}</p>
                  <p class="mt-1 flex items-center gap-2">
                    <svg lucideClock3 [size]="16" aria-hidden="true"></svg> Expires
                    {{ expiry(tactic.expires_at)
                    }}{{
                      tactic.provenance
                        ? ' · source: ' + tactic.provenance.replaceAll('_', ' ')
                        : ''
                    }}
                  </p>
                </div>
              </li>
            } @empty {
              <li class="rounded-xl bg-paper-2 p-6">
                <p class="text-[1.125rem] font-bold">No active shared warnings.</p>
                <p class="mt-1 text-[1.0625rem] text-ink-3">
                  Warnings appear when a blocked call produces a tactic capsule, and disappear when
                  they expire.
                </p>
              </li>
            }
          </ul>
        </section>

        <section class="mt-12" aria-labelledby="reputation-heading">
          <div class="flex flex-wrap items-center justify-between gap-3 border-b-2 border-ink pb-3">
            <h2 id="reputation-heading" class="text-[1.5rem] font-extrabold">Reported numbers</h2>
            <span class="bug bug-example"
              ><svg lucideUnplug [size]="15" aria-hidden="true"></svg> Not connected</span
            >
          </div>
          <p class="mt-4 max-w-[64ch] text-[1.0625rem] text-ink-3">
            The server does not keep reported-number reputation yet, so this dashboard shows none.
            Number warnings, duplicate checks, corrections and expiry need the backend endpoints
            described in
            <code class="rounded bg-paper-2 px-1.5 py-0.5 text-[0.95rem]"
              >frontend/docs/reputation-contract.md</code
            >.
          </p>
          <p class="mt-3 max-w-[64ch] text-[1.0625rem] text-ink-3">
            Reporting a mistake isn’t available yet either: a report would have nowhere to be saved.
            Until then, shared tactic warnings expire on their own.
          </p>
          <button
            type="button"
            class="btn btn-quiet mt-5"
            [attr.aria-expanded]="showExample()"
            aria-controls="reputation-example"
            (click)="showExample.set(!showExample())"
          >
            {{ showExample() ? 'Hide' : 'Show' }} the example progression
            <svg
              lucideChevronDown
              [size]="18"
              [class.rotate-180]="showExample()"
              aria-hidden="true"
            ></svg>
          </button>
          @if (showExample()) {
            <div
              id="reputation-example"
              class="mt-5 rounded-[var(--radius-card)] border-2 border-dashed border-ink-3 p-5"
            >
              <p
                class="flex items-center gap-2 text-[1rem] font-extrabold uppercase tracking-[0.08em]"
              >
                Example · fictional number (555) 010-2234
              </p>
              <ol class="mt-4 grid gap-3 sm:grid-cols-4">
                @for (stage of exampleStages; track stage.title; let i = $index) {
                  <li
                    class="rounded-xl p-4"
                    [class]="i === exampleStages.length - 1 ? 'bg-magenta-wash' : 'bg-paper-2'"
                  >
                    <p class="text-[0.95rem] font-extrabold tabular text-muted">
                      Stage {{ i + 1 }}
                    </p>
                    <p class="mt-1 text-[1.0625rem] font-extrabold">{{ stage.title }}</p>
                    <p class="mt-1 text-[1rem] text-ink-3">{{ stage.text }}</p>
                  </li>
                }
              </ol>
              <p class="mt-4 text-[1rem] text-ink-3">
                Caller ID can be faked, so even at the last stage a warning never claims who placed
                the call.
              </p>
            </div>
          }
        </section>
      </div>

      <aside class="space-y-6 lg:col-span-4" aria-label="Participating households">
        <section
          class="rounded-[var(--radius-card)] bg-paper-2 p-5 sm:p-6"
          aria-labelledby="households-heading"
        >
          <h2 id="households-heading" class="text-[1.1875rem] font-extrabold">
            Participating households
          </h2>
          <p class="mt-1 text-[1rem] text-ink-3">
            The version each household’s FirstRing has actually applied, from its stored receipt.
          </p>
          <ul class="mt-4 space-y-3">
            @for (household of state.community; track household.household_id) {
              <li class="flex items-center justify-between gap-3 rounded-xl bg-paper px-4 py-3">
                <span class="text-[1.0625rem] font-bold">{{ household.display_name }}</span>
                <span class="tabular text-[1rem] font-semibold">{{
                  household.last_applied_version
                    ? 'Applied v' + household.last_applied_version
                    : 'Nothing applied yet'
                }}</span>
              </li>
            }
          </ul>
        </section>
        <section
          class="rounded-[var(--radius-card)] border-2 border-paper-3 p-5 sm:p-6"
          aria-labelledby="receipts-heading"
        >
          <h2 id="receipts-heading" class="text-[1.1875rem] font-extrabold">
            Recent community receipts
          </h2>
          <ul class="mt-3 space-y-2.5">
            @for (event of receipts(); track event.id) {
              <li class="text-[1rem]">
                <strong>{{
                  event.type === 'tactic.created'
                    ? 'Warning shared'
                    : event.type === 'community.applied'
                      ? 'Applied by ' + household(event.payload['household_id'])
                      : 'Similar tactic matched on a call'
                }}</strong>
                <span class="block text-muted"
                  >{{ age(event.received_at) }} · #{{ event.sequence }}</span
                >
              </li>
            } @empty {
              <li class="text-[1rem] text-ink-3">
                No community receipts received in this session.
              </li>
            }
          </ul>
        </section>
      </aside>
    </div>
  `,
})
export class CommunityPage {
  readonly store = inject(ConsoleStore);
  readonly TACTIC = TACTIC_LABEL;
  readonly ACTION = ACTION_LABEL;
  readonly PRETEXT = PRETEXT_LABEL;
  readonly PRESSURE = PRESSURE_LABEL;
  readonly showExample = signal(false);
  readonly exampleStages = [
    {
      title: 'One report',
      text: 'A blocked call creates one minimized report. Nothing changes for anyone else yet.',
    },
    {
      title: 'Independent agreement',
      text: 'Two other households report the same tactic. Each household counts once.',
    },
    {
      title: 'Warning',
      text: 'New calls from the number are labeled “reported number” and screened carefully.',
    },
    {
      title: 'Stronger screening',
      text: 'Participating FirstRing services decline its requests sooner. Carriers are not involved.',
    },
  ];
  readonly tactics = computed(() =>
    [...(this.store.state()?.tactics ?? [])].sort((a, b) => b.version - a.version),
  );
  readonly receipts = computed(() =>
    this.store
      .events()
      .filter(
        (e) =>
          e.type === 'tactic.created' ||
          e.type === 'community.applied' ||
          e.type === 'tactic.matched',
      )
      .slice(-8)
      .reverse(),
  );

  label(value: string, table: Record<string, string>) {
    return humanize(value, table);
  }
  why(tactic: Tactic) {
    return `On a call, someone asked the person to “${humanize(tactic.requested_action, ACTION_LABEL).toLowerCase()}”, which FirstRing never does for a caller. The request was reduced to these categories before sharing; no words, names or numbers were kept.`;
  }
  expiry(at: string) {
    return `${formatDateTime(at, this.store.state()?.profile?.timezone)} (${relativeUntil(at, this.store.now())})`;
  }
  household(id: unknown) {
    return (
      this.store.state()?.community.find((h) => h.household_id === id)?.display_name ?? String(id)
    );
  }
  age(at: string) {
    return relativeAge(at, this.store.now());
  }
}
