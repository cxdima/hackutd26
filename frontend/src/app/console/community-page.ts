import { Component, computed, inject } from '@angular/core';
import { RouterLink } from '@angular/router';
import { LucideShieldAlert } from '@lucide/angular';
import { buildCommunityStories } from '../core/community-story';
import { ConsoleStore } from '../core/console-store';
import { relativeAge, relativeUntil } from '../core/format';
import {
  ACTION_LABEL,
  humanize,
  PRESSURE_LABEL,
  PRETEXT_LABEL,
  TACTIC_LABEL,
} from '../core/language';
import { household as familyHousehold } from '../core/household';
import type { Tactic } from '../core/schemas';
import { WarningTrail } from './warning-trail';

@Component({
  selector: 'app-community-page',
  imports: [RouterLink, LucideShieldAlert, WarningTrail],
  template: `
    @let state = store.state()!;
    <header>
      <h1 class="text-[2.25rem] leading-[1.05] font-extrabold tracking-[-0.03em]">Community</h1>
      <p class="mt-2 max-w-[70ch] text-[1.0625rem] text-ink-3">
        Sarah’s line and Maya and Jimmy’s line share warnings. When one line blocks a scam, it
        shares the trick (what was asked for, the excuse, the pressure), never the words, names or
        numbers. The other line applies it, so the same trick from a new number is caught the first
        time. Warnings expire.
      </p>
    </header>

    @if (story(); as s) {
      <section class="mt-10" aria-labelledby="story-heading">
        <h2 id="story-heading" class="mb-4 text-[1.375rem] font-extrabold">
          How the latest warning travelled
        </h2>
        <app-warning-trail [story]="s" [timezone]="state.profile?.timezone" />
      </section>
    }

    <section class="mt-12" aria-labelledby="active-heading">
      <h2 id="active-heading" class="border-b-2 border-ink pb-2 text-[1.375rem] font-extrabold">
        Active warnings
      </h2>
      <div class="table-scroll">
        <table class="data-table stack-table min-w-[64rem]">
          <thead>
            <tr>
              <th scope="col">Trick</th>
              <th scope="col">Asks the person to</th>
              <th scope="col">Excuse</th>
              <th scope="col">Pressure</th>
              <th scope="col">Safer next step</th>
              <th scope="col" class="num">Version</th>
              <th scope="col">Expires</th>
            </tr>
          </thead>
          <tbody>
            @for (tactic of tactics(); track tactic.id) {
              <tr>
                <td class="font-bold">
                  <span class="inline-flex items-start gap-2 text-magenta-ink">
                    <svg
                      lucideShieldAlert
                      [size]="17"
                      class="mt-1 shrink-0"
                      aria-hidden="true"
                    ></svg>
                    {{ label(tactic.tactic, TACTIC) }}
                  </span>
                  <span class="block text-[0.95rem] font-semibold text-muted">{{
                    why(tactic)
                  }}</span>
                </td>
                <td data-label="Asks the person to">
                  {{ label(tactic.requested_action, ACTION) }}
                </td>
                <td data-label="Excuse">{{ label(tactic.pretext_category, PRETEXT) }}</td>
                <td data-label="Pressure">{{ label(tactic.pressure_category, PRESSURE) }}</td>
                <td data-label="Safer next step">
                  {{ label(tactic.recommended_action ?? 'independent_contact', ACTION) }}
                </td>
                <td data-label="Version" class="num">v{{ tactic.version }}</td>
                <td data-label="Expires" class="whitespace-nowrap">
                  {{ expiry(tactic.expires_at) }}
                </td>
              </tr>
            } @empty {
              <tr>
                <td colspan="7" class="!py-6 text-[1.0625rem] text-ink-3">
                  No active warnings. One appears when a blocked call produces a warning, and it
                  disappears when it expires.
                </td>
              </tr>
            }
          </tbody>
        </table>
      </div>
    </section>

    <div class="mt-12 grid grid-cols-1 gap-10 xl:grid-cols-2">
      <section class="min-w-0" aria-labelledby="households-heading">
        <h2
          id="households-heading"
          class="border-b-2 border-ink pb-2 text-[1.375rem] font-extrabold"
        >
          Lines in the network
        </h2>
        <div class="table-scroll">
          <table class="data-table stack-table">
            <thead>
              <tr>
                <th scope="col">Line</th>
                <th scope="col">Who</th>
                <th scope="col">Warnings applied</th>
              </tr>
            </thead>
            <tbody>
              @for (h of state.community; track h.household_id) {
                <tr>
                  <td class="font-bold whitespace-nowrap">{{ h.display_name }}</td>
                  <td data-label="Who">{{ people(h.household_id) }}</td>
                  <td data-label="Warnings applied" class="tabular">
                    {{
                      h.last_applied_version
                        ? 'Up to v' + h.last_applied_version
                        : 'Nothing applied yet'
                    }}
                  </td>
                </tr>
              }
            </tbody>
          </table>
        </div>
      </section>

      <section class="min-w-0" aria-labelledby="receipts-heading">
        <h2 id="receipts-heading" class="border-b-2 border-ink pb-2 text-[1.375rem] font-extrabold">
          Recent activity
        </h2>
        <div class="table-scroll">
          <table class="data-table stack-table">
            <thead>
              <tr>
                <th scope="col">When</th>
                <th scope="col">What happened</th>
                <th scope="col" class="num">Receipt</th>
              </tr>
            </thead>
            <tbody>
              @for (event of receipts(); track event.id) {
                <tr>
                  <td data-label="When" class="whitespace-nowrap">{{ age(event.received_at) }}</td>
                  <td data-label="What happened">
                    {{
                      event.type === 'tactic.created'
                        ? 'Warning shared'
                        : event.type === 'community.applied'
                          ? 'Applied by ' + household(event.payload['household_id'])
                          : 'Same trick matched on a call'
                    }}
                  </td>
                  <td data-label="Receipt" class="num">#{{ event.sequence }}</td>
                </tr>
              } @empty {
                <tr>
                  <td colspan="3" class="!py-6 text-ink-3">
                    No community activity received in this session.
                  </td>
                </tr>
              }
            </tbody>
          </table>
        </div>
      </section>
    </div>
    <details class="disclosure mt-12">
      <summary>How shared warnings differ from caller reputation</summary>
      <div class="table-scroll mt-3">
        <table class="data-table min-w-[40rem]">
          <thead>
            <tr>
              <th scope="col"><span class="visually-hidden">Aspect</span></th>
              <th scope="col">Shared warnings (this page)</th>
              <th scope="col">Caller reputation</th>
            </tr>
          </thead>
          <tbody>
            @for (row of compare; track row.label) {
              <tr>
                <th scope="row" class="text-left text-[1rem] font-bold whitespace-nowrap">
                  {{ row.label }}
                </th>
                <td>{{ row.warning }}</td>
                <td>{{ row.reputation }}</td>
              </tr>
            }
          </tbody>
        </table>
      </div>
      <a routerLink="/console/callers" queryParamsHandling="preserve" class="more-link"
        >See caller reputation and history</a
      >
    </details>
  `,
})
export class CommunityPage {
  readonly store = inject(ConsoleStore);
  readonly TACTIC = TACTIC_LABEL;
  readonly ACTION = ACTION_LABEL;
  readonly PRETEXT = PRETEXT_LABEL;
  readonly PRESSURE = PRESSURE_LABEL;
  readonly compare = [
    {
      label: 'Describes',
      warning: 'A trick: what was asked for, the excuse and the pressure',
      reputation: 'A phone number, kept as a one-way code',
    },
    {
      label: 'Created when',
      warning: 'A line blocks a protected request',
      reputation: 'A line blocks a protected request from that number',
    },
    {
      label: 'Catches',
      warning: 'The same trick from any number',
      reputation: 'The same number calling again, on any line',
    },
    {
      label: 'Never contains',
      warning: 'Words, names or numbers from the call',
      reputation: 'The number itself or anything the caller said',
    },
    {
      label: 'Expires',
      warning: 'After its time to live (24 hours here)',
      reputation: 'After 14 days',
    },
  ];
  /** "Latest" means the newest shared version, not the most complete story. */
  readonly story = computed(
    () =>
      [...buildCommunityStories(this.store.state(), this.store.events(), this.store.calls())].sort(
        (a, b) => b.version - a.version,
      )[0],
  );
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
    if (tactic.provenance === 'designated_synthetic_test')
      return 'From a designated test, not a real call.';
    return 'From a blocked call, reduced to categories.';
  }
  expiry(at: string) {
    return relativeUntil(at, this.store.now());
  }
  people(id: string) {
    const h = familyHousehold(id);
    return h ? h.people.map((p) => `${p.name} (${p.relation})`).join(', ') : '—';
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
