import { Component, computed, inject, input, signal } from '@angular/core';
import {
  LucideCheck,
  LucideCircle,
  LucideLockKeyhole,
  LucidePhone,
  LucidePhoneOff,
  LucideUsers,
} from '@lucide/angular';
import { traceStats, type CallRecord } from '../core/calls';
import { ConsoleStore } from '../core/console-store';
import {
  confirmedMatches,
  displayName,
  legEvidence,
  legStatus,
  modelHttpEvidence,
  shortId,
  TERMINAL_LEG_STATUSES,
} from '../core/evidence';
import { formatDateTime, formatDuration, formatMs, zoneLabel } from '../core/format';
import { describeObservation, humanize, PRETEXT_LABEL, ACTION_LABEL } from '../core/language';
import type { Leg } from '../core/schemas';
import { DecisionSpine } from './decision-spine';
import { OutcomeChip } from './outcome-chip';

@Component({
  selector: 'app-call-detail',
  imports: [
    OutcomeChip,
    DecisionSpine,
    LucideCheck,
    LucideCircle,
    LucideLockKeyhole,
    LucidePhone,
    LucidePhoneOff,
    LucideUsers,
  ],
  template: `
    @let c = call();
    @let s = c.session;
    <article aria-labelledby="call-title">
      <header class="border-b-2 border-ink pb-6">
        <div class="flex flex-wrap items-center gap-2">
          <app-outcome-chip [outcome]="c.outcome" />
          <span class="bug" [class]="modeClass()">{{ modeLabel() }}</span>
        </div>
        <h1
          id="call-title"
          class="mt-4 max-w-[40ch] text-[clamp(1.6rem,1.3rem+1.2vw,2.35rem)] leading-[1.15] font-extrabold tracking-[-0.02em]"
        >
          {{ c.headline }}
        </h1>
        <dl class="mt-4 flex flex-wrap gap-x-7 gap-y-2 text-[1.0625rem]">
          <div class="flex gap-1.5">
            <dt class="text-muted">When</dt>
            <dd class="font-semibold">
              <time [attr.datetime]="c.startedAt">{{ when() }}</time> {{ zone() }}
            </dd>
          </div>
          <div class="flex gap-1.5">
            <dt class="text-muted">Length</dt>
            <dd class="font-semibold tabular">{{ duration() }}</dd>
          </div>
          <div class="flex gap-1.5">
            <dt class="text-muted">Answered for</dt>
            <dd class="font-semibold">{{ c.name }}</dd>
          </div>
          <div class="flex gap-1.5">
            <dt class="text-muted">Caller identity</dt>
            <dd class="font-semibold">Unverified</dd>
          </div>
        </dl>
      </header>

      <section
        class="grid gap-px overflow-hidden border-b border-paper-3 sm:grid-cols-2"
        aria-label="At a glance"
      >
        <div class="py-5 sm:pr-6">
          <h2 class="text-[0.9375rem] font-extrabold uppercase tracking-[0.08em] text-muted">
            What the caller wanted
          </h2>
          <p class="mt-1.5 text-[1.1875rem] font-semibold leading-snug">{{ requestSentence() }}</p>
          @if (understanding(); as u) {
            @if (u.claimed_identity) {
              <p class="mt-2 text-[1rem] text-ink-3">
                Said they were: “{{ u.claimed_identity }}” (not verified)
              </p>
            }
            @if (u.missing_information.length) {
              <p class="mt-1 text-[1rem] text-ink-3">
                Still missing: {{ u.missing_information.join('; ') }}
              </p>
            }
          }
        </div>
        <div class="py-5 sm:border-l sm:border-paper-3 sm:pl-6">
          <h2 class="text-[0.9375rem] font-extrabold uppercase tracking-[0.08em] text-muted">
            What FirstRing did
          </h2>
          <p class="mt-1.5 text-[1.1875rem] font-semibold leading-snug">{{ didSentence() }}</p>
          @if (s.message; as message) {
            <div class="mt-3 rounded-xl bg-paper-2 p-4">
              <p class="text-[0.9375rem] font-bold text-muted">
                Saved message, private to you{{
                  earlierMessage() ? ' (from an earlier caller turn)' : ''
                }}
              </p>
              <p class="mt-1 text-[1.125rem] font-semibold">{{ message.summary }}</p>
              <p class="mt-1 text-[0.95rem] text-muted">
                Saved for review. Not a delivery receipt.
              </p>
            </div>
          }
        </div>
      </section>

      @if (matches().length) {
        <section class="mt-6 rounded-xl bg-magenta-wash p-5" aria-labelledby="matches-heading">
          <h2 id="matches-heading" class="text-[1.125rem] font-extrabold text-magenta-ink">
            Similar shared tactic on this call
          </h2>
          @for (match of matches(); track match.id) {
            <p class="mt-2 text-[1.0625rem]">
              {{ actionLabel(match.requested_action) }} ·
              {{ pretextLabel(match.pretext_category) }} · warning v{{ match.version }} ({{
                match.supporting_count
              }}
              matching capsule{{ match.supporting_count === 1 ? '' : 's' }}). A similar tactic, not
              an identified person.
            </p>
          }
        </section>
      }

      <section class="mt-10" aria-labelledby="transcript-heading">
        <h2 id="transcript-heading" class="text-[1.5rem] font-extrabold">Transcript</h2>
        <p class="mt-1 text-[1rem] text-muted">
          Closed-caption style. Lines marked “evidence” were quoted by the model to justify its
          assessment.
        </p>
        <ol class="caption-band on-ink mt-4 space-y-3 p-5 sm:p-6">
          @for (turn of s.turns; track turn.id) {
            <li class="text-[1.125rem] leading-snug">
              <span class="caption-speaker mr-2" [class.!text-lime]="turn.speaker !== 'caller'"
                >&gt;&gt; {{ turn.speaker === 'caller' ? 'Caller' : 'FirstRing' }}</span
              >
              {{ turn.text }}
              @if (evidenceIds().has(turn.id)) {
                <span
                  class="ml-2 whitespace-nowrap rounded bg-paper/12 px-1.5 py-0.5 text-[0.8125rem] font-extrabold uppercase tracking-[0.06em] text-paper"
                  >Evidence</span
                >
              }
            </li>
          } @empty {
            <li class="text-[1.0625rem] text-paper/80">No words were transcribed on this call.</li>
          }
          @if (partial(); as text) {
            <li class="text-[1.125rem] leading-snug text-paper/70">
              <span class="caption-speaker mr-2">&gt;&gt; Caller (still speaking)</span>{{ text }}
            </li>
          }
        </ol>
        <p class="mt-3 text-[1rem] text-muted">
          No audio recording is available for this call in the dashboard.
        </p>
      </section>

      <section class="mt-12" aria-labelledby="spine-heading">
        <h2 id="spine-heading" class="text-[1.5rem] font-extrabold">
          What FirstRing saw, and why it acted
        </h2>
        <div class="mt-4"><app-decision-spine [call]="c" /></div>
      </section>

      @if (s.offer || s.legs.length) {
        <section class="mt-12" aria-labelledby="phones-heading">
          <h2 id="phones-heading" class="text-[1.5rem] font-extrabold">Phone actions</h2>
          @if (s.offer; as offer) {
            <div class="mt-4 rounded-xl border-2 border-paper-3 p-5">
              <p class="text-[1.0625rem] font-bold">Private offer · {{ offer.status }}</p>
              <p class="mt-1 text-[1.0625rem]">{{ offer.summary }}</p>
              <p class="mt-1 text-[0.95rem] text-muted">
                Policy epoch {{ offer.policy_epoch }} · expires {{ time(offer.expires_at) }}. A
                withdrawn offer cannot be revived by an old acceptance.
              </p>
            </div>
          }
          <div class="mt-4 grid gap-4 sm:grid-cols-2">
            @for (leg of s.legs; track leg.action_id) {
              <article
                class="rounded-xl border-2 p-5"
                [class]="leg.acknowledged ? 'border-lime' : 'border-paper-3'"
              >
                <h3 class="flex items-center gap-2 text-[1.125rem] font-extrabold">
                  @if (leg.role === 'family') {
                    <svg lucideUsers [size]="18" aria-hidden="true"></svg>
                  } @else {
                    <svg lucidePhone [size]="18" aria-hidden="true"></svg>
                  }
                  {{ leg.role === 'sarah' ? c.name : legName(leg.role) }}
                </h3>
                <p class="mt-1 text-[1.0625rem] font-semibold">{{ legStatus(leg) }}</p>
                <ol
                  class="mt-3 flex flex-wrap gap-2"
                  [attr.aria-label]="legName(leg.role) + ' call receipts'"
                >
                  @for (label of legLabels(leg); track label; let i = $index) {
                    <li
                      class="inline-flex items-center gap-1.5 rounded-md px-2 py-1 text-[0.9375rem] font-semibold"
                      [class]="
                        legObserved(leg)[i] ? 'bg-lime-wash text-lime-ink' : 'bg-paper-2 text-muted'
                      "
                    >
                      @if (legObserved(leg)[i]) {
                        <svg lucideCheck [size]="14" aria-hidden="true"></svg>
                      } @else {
                        <svg lucideCircle [size]="11" aria-hidden="true"></svg>
                      }
                      {{ label
                      }}<span class="visually-hidden">{{
                        legObserved(leg)[i] ? ': receipt observed' : ': no receipt yet'
                      }}</span>
                    </li>
                  }
                </ol>
                @if (leg.role !== 'sarah') {
                  <p class="mt-3 flex items-center gap-1.5 text-[0.95rem] text-muted">
                    <svg lucideLockKeyhole [size]="14" aria-hidden="true"></svg> The caller is not
                    on this line.
                  </p>
                }
              </article>
            }
          </div>
        </section>
      }

      <details class="mt-12 rounded-[var(--radius-card)] border-2 border-paper-3">
        <summary
          class="flex min-h-14 cursor-pointer items-center justify-between gap-4 px-5 text-[1.125rem] font-extrabold"
        >
          Technical details
        </summary>
        <div class="grid gap-8 px-5 pb-6 lg:grid-cols-2">
          <dl class="grid grid-cols-[auto_1fr] gap-x-5 gap-y-2 text-[1rem]">
            <dt class="font-bold text-muted">Model requests</dt>
            <dd class="tabular">
              {{ stats().modelRequests }} · {{ ms(stats().modelMs) }} total{{
                stats().modelRequests
                  ? ' · ' + ms(stats().modelMs / stats().modelRequests) + ' avg'
                  : ''
              }}
            </dd>
            <dt class="font-bold text-muted">Completion tokens</dt>
            <dd class="tabular">{{ stats().tokens }}</dd>
            <dt class="font-bold text-muted">Proposals / executed</dt>
            <dd class="tabular">{{ stats().proposals }} / {{ stats().executed }}</dd>
            <dt class="font-bold text-muted">Failures</dt>
            <dd class="tabular">{{ stats().failures }}</dd>
            <dt class="font-bold text-muted">Latest model receipt</dt>
            <dd>
              {{
                http().completed
                  ? (http().completed!.observation.response_model ?? 'unknown model') +
                    ' · ' +
                    http().completed!.observation.outcome +
                    (http().current ? ' · current turn' : ' · earlier turn')
                  : 'None received'
              }}
            </dd>
            <dt class="font-bold text-muted">Session</dt>
            <dd class="break-all">
              {{ shortId(s.id) }} · {{ s.state
              }}{{ s.closing_reason ? ' · ' + s.closing_reason : '' }}
            </dd>
            <dt class="font-bold text-muted">Turn / assessed</dt>
            <dd class="tabular">
              {{ s.turn_version ?? '—' }} / {{ s.assessed_version ?? '—'
              }}{{ s.speech_pending ? ' · speech pending' : '' }}
            </dd>
            <dt class="font-bold text-muted">Generation · epoch</dt>
            <dd class="tabular">{{ s.generation_id }} · {{ s.policy_epoch }}</dd>
            <dt class="font-bold text-muted">Data</dt>
            <dd>{{ s.execution_mode }} · {{ s.data_origin }}</dd>
            <dt class="font-bold text-muted">Receipts</dt>
            <dd class="tabular">
              {{ c.events.length }}{{ c.hasEndReceipt ? '' : ' · no end receipt yet' }}
            </dd>
          </dl>
          @if (canOperate()) {
            <div>
              <h3 class="text-[1.125rem] font-extrabold">Operator actions</h3>
              <p class="mt-1 text-[1rem] text-ink-3">
                These act on the live server. A sent request is not a completed action; watch the
                receipts above.
              </p>
              <div class="mt-4 flex flex-wrap gap-3">
                <button
                  type="button"
                  class="btn btn-quiet"
                  [disabled]="!phoneBReady() || disabled()"
                  aria-describedby="phone-b-reason"
                  (click)="probe()"
                >
                  <svg lucidePhone [size]="16" aria-hidden="true"></svg> Call Phone B
                </button>
                @if (!allEnded()) {
                  @if (confirmEnd()) {
                    <button
                      type="button"
                      class="btn btn-ink"
                      [disabled]="disabled()"
                      (click)="endSession()"
                    >
                      <svg lucidePhoneOff [size]="16" aria-hidden="true"></svg> Confirm: end session
                    </button>
                    <button type="button" class="btn btn-quiet" (click)="confirmEnd.set(false)">
                      Cancel
                    </button>
                  } @else {
                    <button
                      type="button"
                      class="btn btn-quiet"
                      [disabled]="disabled()"
                      (click)="confirmEnd.set(true)"
                    >
                      <svg lucidePhoneOff [size]="16" aria-hidden="true"></svg> End session…
                    </button>
                  }
                }
              </div>
              <p id="phone-b-reason" class="mt-3 text-[1rem] text-muted">{{ phoneBReason() }}</p>
            </div>
          }
        </div>
      </details>
    </article>
  `,
})
export class CallDetail {
  readonly call = input.required<CallRecord>();
  private readonly store = inject(ConsoleStore);
  readonly confirmEnd = signal(false);
  readonly shortId = shortId;

  readonly understanding = computed(() => this.call().session.understanding?.understanding);
  readonly when = computed(() =>
    formatDateTime(this.call().startedAt ?? this.call().lastActivityAt, this.call().timezone),
  );
  readonly zone = computed(() =>
    zoneLabel(this.call().timezone, this.call().startedAt ?? Date.now()),
  );
  readonly duration = computed(() => formatDuration(this.call().durationSeconds));
  readonly stats = computed(() => traceStats(this.call()));
  readonly http = computed(() => modelHttpEvidence(this.call().session, this.call().events));
  readonly evidenceIds = computed(() => {
    const ids = new Set<string>();
    for (const assessment of this.call().session.assessments ?? [])
      for (const ref of assessment.evidence_refs) ids.add(ref.turn_id);
    for (const id of this.call().session.message?.evidence_turn_ids ?? []) ids.add(id);
    return ids;
  });
  readonly partial = computed(() => {
    const c = this.call();
    if (c.outcome !== 'in_progress') return undefined;
    const last = [...c.events]
      .reverse()
      .find((e) => e.type === 'transcript.partial' || e.type === 'transcript.final');
    return last?.type === 'transcript.partial' && typeof last.payload['text'] === 'string'
      ? (last.payload['text'] as string)
      : undefined;
  });
  readonly earlierMessage = computed(() => {
    const s = this.call().session;
    return !!s.message && s.turn_version !== undefined && s.message.turn_version < s.turn_version;
  });
  readonly requestSentence = computed(() => {
    const c = this.call();
    const u = this.understanding();
    const observation = c.protectedRequest ?? c.session.assessments?.at(-1)?.observation;
    const plain = observation
      ? describeObservation(observation, c.name)
      : 'No accepted assessment was received.';
    return u?.requested_action ? `${plain} In the model’s words: “${u.requested_action}.”` : plain;
  });
  readonly didSentence = computed(() => {
    const c = this.call();
    switch (c.outcome) {
      case 'blocked':
        return 'Declined the protected request and ended the call politely. Nothing private was shared.';
      case 'message_saved':
        return 'Took a message and ended the call politely.';
      case 'needs_review':
        return 'Asked for more detail, but the call ended before it was clear.';
      case 'no_message':
        return 'Ended the call without saving a message, as the caller didn’t leave one.';
      case 'in_progress':
        return c.session.current_action;
      default:
        return c.failure
          ? `Stopped safely after a technical problem, without sharing anything. Technical reason: ${c.failure}.`
          : 'The call ended before FirstRing could save anything.';
    }
  });
  readonly matches = computed(() => {
    const c = this.call();
    const state = this.store.state();
    if (!state || c.session.execution_mode !== state.execution_mode) return [];
    const applied =
      state.community.find((h) => h.household_id === c.session.household_id)
        ?.last_applied_version ?? 0;
    return confirmedMatches(
      c.events,
      c.session.household_id,
      c.id,
      c.session.execution_mode,
      applied,
      this.store.now(),
    );
  });
  readonly modeLabel = computed(() =>
    this.store.isDemo()
      ? 'Demo'
      : this.call().session.execution_mode === 'live'
        ? 'Live call'
        : 'Replay',
  );
  readonly modeClass = computed(() =>
    this.store.isDemo()
      ? 'bug-demo'
      : this.call().session.execution_mode === 'live'
        ? 'bug-live'
        : 'bug-replay',
  );

  readonly canOperate = computed(() => this.store.source() === 'live');
  readonly disabled = computed(() => !!this.store.busy() || this.store.stale());
  readonly allEnded = computed(() => {
    const s = this.call().session;
    return s.inbound_ended && s.legs.every((leg) => TERMINAL_LEG_STATUSES.includes(leg.status));
  });
  /** Same gate as the original console: enrollment, authorization, budget and an active live Sarah session. */
  readonly phoneBReady = computed(() => {
    const phoneB = this.store.state()?.phone_b;
    const s = this.call().session;
    return (
      !!phoneB &&
      phoneB.enrolled &&
      phoneB.enabled &&
      phoneB.authorized &&
      phoneB.remaining_attempts > 0 &&
      phoneB.can_request &&
      s.execution_mode === 'live' &&
      s.household_id === 'sarah' &&
      !s.inbound_ended &&
      !s.legs.some((leg) => leg.kind === 'probe')
    );
  });
  readonly phoneBReason = computed(() => {
    const phoneB = this.store.state()?.phone_b;
    const s = this.call().session;
    if (!phoneB) return 'Phone B: enrollment and attempt budget have not been received.';
    const budget = ` Attempts remaining: ${phoneB.remaining_attempts}.`;
    if (!phoneB.can_request) return `Phone B: ${phoneB.reason}${budget}`;
    if (s.execution_mode !== 'live' || s.household_id !== 'sarah' || s.inbound_ended)
      return `Phone B needs an active live Sarah session.${budget}`;
    if (s.legs.some((leg) => leg.kind === 'probe'))
      return `This session already has a reserved probe.${budget}`;
    return `Phone B: ${phoneB.reason}${budget}`;
  });

  time(at: string) {
    return formatDateTime(at, this.call().timezone);
  }
  ms(value: number) {
    return formatMs(value);
  }
  legName(role: string) {
    return displayName(role);
  }
  legStatus(leg: Leg) {
    return legStatus(leg);
  }
  legLabels(leg: Leg) {
    return leg.kind === 'handoff'
      ? ['Requested', 'Ringing', 'Answered']
      : ['Requested', 'Ringing', 'Answered', 'Keypad 1'];
  }
  legObserved(leg: Leg) {
    return legEvidence(leg, this.call().events);
  }
  actionLabel(value: string) {
    return humanize(value, ACTION_LABEL);
  }
  pretextLabel(value: string) {
    return humanize(value, PRETEXT_LABEL);
  }

  probe() {
    if (!this.phoneBReady()) return;
    void this.store.action('/api/demo/probe', { session_id: this.call().id, contact_id: 'sarah' });
  }

  endSession() {
    this.confirmEnd.set(false);
    void this.store.action('/api/demo/end-call', { session_id: this.call().id });
  }
}
