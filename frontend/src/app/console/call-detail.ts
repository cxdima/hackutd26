import { Component, computed, inject, input, signal } from '@angular/core';
import {
  LucideCheck,
  LucideCircle,
  LucideLockKeyhole,
  LucidePhone,
  LucidePhoneOff,
  LucideUsers,
} from '@lucide/angular';
import { RouterLink } from '@angular/router';
import { CallerDirectory } from '../core/caller-directory';
import { STANDING_LABEL, type Standing } from '../core/callers';
import { traceStats, type CallRecord } from '../core/calls';
import { ringsFirstName } from '../core/household';
import { turnRows } from '../core/turns';
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
import { handoffView, personAction } from '../core/live';
import { CallRecording } from './call-recording';
import { DecisionSpine } from './decision-spine';
import { HandoffTimeline } from './handoff-timeline';
import { ModelEvidence } from './model-evidence';
import { OutcomeText } from './outcome-chip';
import { ReputationPanel } from './reputation-panel';

@Component({
  selector: 'app-call-detail',
  imports: [
    RouterLink,
    OutcomeText,
    DecisionSpine,
    CallRecording,
    HandoffTimeline,
    ModelEvidence,
    ReputationPanel,
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
      <header>
        <div class="flex flex-wrap items-center gap-x-3 gap-y-1">
          <app-outcome-text [outcome]="c.outcome" [short]="false" />
          <p class="text-[1rem] text-muted">
            <time [attr.datetime]="c.startedAt">{{ when() }}</time> {{ zone() }} · {{ c.name }}’s
            line · {{ modeLabel() }}
          </p>
        </div>
        <h1
          id="call-title"
          class="mt-3 max-w-[40ch] text-[2rem] leading-[1.12] font-extrabold tracking-[-0.02em] text-balance"
        >
          {{ c.headline }}
        </h1>
        <dl
          class="mt-6 grid grid-cols-2 gap-x-8 gap-y-4 border-y-2 border-ink py-4 md:grid-cols-3 xl:grid-cols-6"
        >
          <div>
            <dt class="text-[0.95rem] font-semibold text-muted">Caller</dt>
            <dd class="font-bold">
              @if (caller(); as who) {
                @if (who.captured) {
                  <a
                    [routerLink]="['/console/callers', who.id]"
                    queryParamsHandling="preserve"
                    class="text-ink"
                    >{{ who.label }}</a
                  >
                  <span
                    class="block text-[0.95rem]"
                    [class]="
                      who.standing === 'reported' || who.standing === 'warned'
                        ? 'text-magenta-ink'
                        : 'font-semibold text-muted'
                    "
                    >{{ standing(who.standing)
                    }}{{ who.calls.length > 1 ? ' · ' + who.calls.length + ' calls' : '' }}</span
                  >
                } @else {
                  Number not captured
                }
              } @else {
                Unverified
              }
            </dd>
          </div>
          <div>
            <dt class="text-[0.95rem] font-semibold text-muted">Length</dt>
            <dd class="tabular font-bold">{{ duration() }}</dd>
          </div>
          <div>
            <dt class="text-[0.95rem] font-semibold text-muted">Caller turns</dt>
            <dd class="tabular font-bold">{{ c.callerTurns }}</dd>
          </div>
          <div>
            <dt class="text-[0.95rem] font-semibold text-muted">AI requests</dt>
            <dd class="tabular font-bold">
              {{ stats().modelRequests }}
              @if (stats().modelRequests) {
                <span class="block text-[0.95rem] font-semibold text-muted"
                  >{{ ms(stats().modelMs / stats().modelRequests) }} average</span
                >
              }
            </dd>
          </div>
          <div>
            <dt class="text-[0.95rem] font-semibold text-muted">Rings first</dt>
            <dd class="font-bold">{{ recipient() }}’s phone</dd>
          </div>
          <div>
            <dt class="text-[0.95rem] font-semibold text-muted">Receipts</dt>
            <dd class="tabular font-bold">{{ c.events.length }}</dd>
          </div>
        </dl>
      </header>

      <div class="mt-8 grid gap-x-10 gap-y-10 xl:grid-cols-[minmax(0,1fr)_26rem]">
        <section class="min-w-0" aria-labelledby="transcript-heading">
          <h2 id="transcript-heading" class="text-[1.375rem] font-extrabold">Conversation</h2>
          <p class="mt-1 text-[1rem] text-muted">
            What the speech recognizer heard and what FirstRing said back. Lines marked “quoted as
            evidence” are the ones the AI cited for its decision.
          </p>
          <ol class="mt-4 divide-y divide-paper-3 border-y border-paper-3">
            @for (turn of s.turns; track turn.id) {
              <li class="grid gap-x-4 gap-y-1 py-3 sm:grid-cols-[7.5rem_minmax(0,1fr)]">
                <span
                  class="text-[0.95rem] font-extrabold"
                  [class]="turn.speaker === 'caller' ? 'text-ink' : 'text-lime-ink'"
                  >{{ turn.speaker === 'caller' ? 'Caller' : 'FirstRing' }}</span
                >
                <p class="max-w-[70ch] text-[1.0625rem] leading-snug">
                  {{ turn.text }}
                  @if (evidenceIds().has(turn.id)) {
                    <span class="ml-1 whitespace-nowrap text-[0.95rem] font-bold text-magenta-ink"
                      >· quoted as evidence</span
                    >
                  }
                </p>
              </li>
            } @empty {
              <li class="py-4 text-[1.0625rem] text-ink-3">
                No words were transcribed on this call.
              </li>
            }
            @if (partial(); as text) {
              <li class="grid gap-x-4 py-3 text-ink-3 sm:grid-cols-[7.5rem_minmax(0,1fr)]">
                <span class="text-[0.95rem] font-extrabold">Caller, still speaking</span>
                <p class="text-[1.0625rem] leading-snug">{{ text }}</p>
              </li>
            }
          </ol>
          <app-call-recording [sessionId]="c.id" />
        </section>

        <aside class="min-w-0 space-y-8" aria-label="Summary">
          <section aria-labelledby="summary-heading">
            <h2 id="summary-heading" class="text-[1.375rem] font-extrabold">Summary</h2>
            <dl class="mt-3 divide-y divide-paper-3 border-y border-paper-3">
              <div class="py-3">
                <dt class="text-[0.95rem] font-semibold text-muted">The caller wanted</dt>
                <dd class="text-[1.0625rem] font-semibold leading-snug">{{ requestSentence() }}</dd>
                @if (understanding(); as u) {
                  @if (u.claimed_identity) {
                    <dd class="mt-1 text-[1rem] text-ink-3">
                      Said they were “{{ u.claimed_identity }}”. Not verified.
                    </dd>
                  }
                  @if (u.missing_information.length && c.outcome !== 'blocked') {
                    <dd class="mt-1 text-[1rem] text-ink-3">
                      Still unclear: {{ u.missing_information.join('; ') }}
                    </dd>
                  }
                }
              </div>
              <div class="py-3">
                <dt class="text-[0.95rem] font-semibold text-muted">FirstRing</dt>
                <dd class="text-[1.0625rem] font-semibold leading-snug">{{ didSentence() }}</dd>
              </div>
              @if (s.message; as message) {
                <div class="py-3">
                  <dt class="text-[0.95rem] font-semibold text-muted">
                    Saved message{{ earlierMessage() ? ' (earlier turn)' : '' }}
                  </dt>
                  <dd>
                    <blockquote class="text-[1.125rem] font-semibold leading-snug">
                      “{{ message.summary }}”
                    </blockquote>
                    <p class="mt-1 text-[0.95rem] text-muted">
                      Private to the family. Saved for review; not a delivery receipt.
                    </p>
                  </dd>
                </div>
              }
              <div class="py-3">
                <dt class="text-[0.95rem] font-semibold text-muted">Does anyone need to act?</dt>
                <dd
                  class="text-[1.0625rem] font-semibold leading-snug"
                  [class.text-magenta-ink]="action().needed && c.outcome === 'in_progress'"
                >
                  {{ action().text }}
                </dd>
              </div>
            </dl>
          </section>

          @if (matches().length) {
            <section aria-labelledby="matches-heading">
              <h2 id="matches-heading" class="text-[1.125rem] font-extrabold text-magenta-ink">
                Matches a shared warning
              </h2>
              @for (match of matches(); track match.id) {
                <p class="mt-2 text-[1rem]">
                  {{ actionLabel(match.requested_action) }} ·
                  {{ pretextLabel(match.pretext_category) }} · warning v{{ match.version }} ({{
                    match.supporting_count
                  }}
                  matching capsule{{ match.supporting_count === 1 ? '' : 's' }}). A similar tactic,
                  not an identified person.
                </p>
              }
            </section>
          }

          <app-reputation-panel [sessionId]="c.id" />
        </aside>
      </div>

      <section class="mt-12" aria-labelledby="turns-heading">
        <h2 id="turns-heading" class="text-[1.375rem] font-extrabold">
          What the AI saw and decided, turn by turn
        </h2>
        <p class="mt-1 max-w-[75ch] text-[1rem] text-muted">
          Each caller turn goes to NVIDIA Nemotron, which proposes tools. Rust checks every proposal
          against fixed rules before anything happens, so the AI cannot hand out a code, take a
          payment or call anyone on its own.
        </p>
        <div class="table-scroll mt-4">
          <table class="data-table min-w-[64rem]">
            <thead>
              <tr>
                <th scope="col" class="num">#</th>
                <th scope="col" class="w-[22%]">Caller said</th>
                <th scope="col" class="w-[24%]">AI understood</th>
                <th scope="col" class="w-[18%]">Safety rule</th>
                <th scope="col" class="w-[24%]">FirstRing did</th>
                <th scope="col" class="num">AI time</th>
              </tr>
            </thead>
            <tbody>
              @for (t of turns(); track t.index) {
                <tr>
                  <td class="num font-bold">{{ t.index || '—' }}</td>
                  <td>
                    @if (t.said) {
                      <q class="text-[1rem]">{{ t.said }}</q>
                    } @else {
                      <span class="text-muted">Call answered</span>
                    }
                  </td>
                  <td>
                    @for (line of t.understood; track line) {
                      <p class="text-[1rem]">{{ line }}</p>
                    } @empty {
                      <span class="text-muted">—</span>
                    }
                    @if (t.proposed.length) {
                      <p class="mt-1 text-[0.95rem] text-muted">
                        Proposed: {{ t.proposed.join(', ') }}
                      </p>
                    }
                  </td>
                  <td>
                    @if (t.decision; as d) {
                      <p
                        class="text-[1rem] font-bold"
                        [class]="
                          d.verdict === 'protected'
                            ? 'text-magenta-ink'
                            : d.verdict === 'allowed'
                              ? 'text-lime-ink'
                              : 'text-amber-ink'
                        "
                      >
                        {{
                          d.verdict === 'protected'
                            ? 'Protected request: refused'
                            : d.verdict === 'allowed'
                              ? 'Nothing protected: help continued'
                              : 'Not assessed'
                        }}
                      </p>
                    } @else {
                      <span class="text-muted">—</span>
                    }
                  </td>
                  <td>
                    @for (line of t.did; track line) {
                      <p class="text-[1rem]">{{ line }}</p>
                    }
                    @for (line of t.replied; track line) {
                      <p class="text-[1rem] text-ink-3">Said: “{{ line }}”</p>
                    }
                    @for (line of t.failures; track line) {
                      <p class="text-[1rem] font-semibold text-amber-ink">{{ line }}</p>
                    }
                    @if (!t.did.length && !t.replied.length && !t.failures.length) {
                      <span class="text-muted">—</span>
                    }
                  </td>
                  <td class="num">
                    @if (t.modelRequests) {
                      {{ ms(t.modelMs) }}
                      <span class="block text-[0.95rem] text-muted"
                        >{{ t.modelRequests }} request{{ t.modelRequests === 1 ? '' : 's' }}</span
                      >
                    } @else {
                      —
                    }
                  </td>
                </tr>
              } @empty {
                <tr>
                  <td colspan="6" class="!py-6 text-[1.0625rem] text-ink-3">
                    No turns were recorded for this call.
                  </td>
                </tr>
              }
            </tbody>
          </table>
        </div>
      </section>

      @if (handoff()) {
        <section class="mt-12" aria-labelledby="handoff-heading">
          <h2 id="handoff-heading" class="text-[1.5rem] font-extrabold">Private permission</h2>
          <p class="mt-1 max-w-[65ch] text-[1rem] text-muted">
            FirstRing asks {{ recipient() }} on their own phone before connecting anyone. Each step
            below appears only when its receipt arrives.
          </p>
          @if (s.offer; as offer) {
            <div class="mt-4 rounded-xl bg-paper-2 p-4">
              <p class="text-[0.95rem] font-bold text-muted">
                Private summary for {{ recipient() }} · offer {{ offer.status }}
              </p>
              <p class="mt-1 text-[1.0625rem] font-semibold">{{ offer.summary }}</p>
              <p class="mt-1 text-[0.95rem] text-muted">
                Policy epoch {{ offer.policy_epoch }} · expires {{ time(offer.expires_at) }}. A
                withdrawn offer cannot be revived by an old acceptance.
              </p>
            </div>
          }
          <app-handoff-timeline [call]="c" />
        </section>
      }

      @if (otherLegs().length) {
        <section class="mt-12" aria-labelledby="phones-heading">
          <h2 id="phones-heading" class="text-[1.5rem] font-extrabold">Other phone actions</h2>
          <div class="mt-4 grid gap-4 sm:grid-cols-2">
            @for (leg of otherLegs(); track leg.action_id) {
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
                  {{ leg.kind === 'probe' ? 'Phone B test call' : legName(leg.role) }}
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
          Every receipt, in order
        </summary>
        <div class="px-5 pb-6"><app-decision-spine [call]="c" /></div>
      </details>

      <details class="mt-4 rounded-[var(--radius-card)] border-2 border-paper-3">
        <summary
          class="flex min-h-14 cursor-pointer items-center justify-between gap-4 px-5 text-[1.125rem] font-extrabold"
        >
          Technical details
        </summary>
        <section class="px-5 pb-8" aria-labelledby="nvidia-heading">
          <h3 id="nvidia-heading" class="text-[1.125rem] font-extrabold">NVIDIA decision model</h3>
          <div class="mt-2"><app-model-evidence [call]="c" /></div>
        </section>
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

  private readonly directory = inject(CallerDirectory);
  readonly understanding = computed(() => this.call().session.understanding?.understanding);
  readonly caller = computed(() => this.directory.forCall(this.call().id));
  readonly recipient = computed(() =>
    ringsFirstName(this.call().session.household_id, this.call().name),
  );
  readonly turns = computed(() => turnRows(this.call()));
  readonly action = computed(() => personAction(this.call()));
  readonly handoff = computed(() => handoffView(this.call()));
  /** Handoff legs are shown in the permission timeline; the rest (tests, family) here. */
  readonly otherLegs = computed(() =>
    this.call().session.legs.filter(
      (leg) => leg.kind !== 'handoff' && leg.action_id !== this.call().session.offer?.leg_id,
    ),
  );
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
      case 'connected':
        return `Rang ${this.recipient()} privately, played a summary, and connected the call only after ${this.recipient()} pressed 1.`;
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

  standing(value: Standing) {
    return STANDING_LABEL[value];
  }
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
