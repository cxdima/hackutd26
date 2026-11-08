import {
  afterNextRender,
  Component,
  computed,
  effect,
  ElementRef,
  inject,
  signal,
  untracked,
  viewChild,
} from '@angular/core';
import { NgTemplateOutlet } from '@angular/common';
import { RouterLink } from '@angular/router';
import {
  LucideBan,
  LucideCheck,
  LucideChevronDown,
  LucideCircleDashed,
  LucideHourglass,
  LucidePanelRightOpen,
  LucidePhoneForwarded,
  LucidePhoneOff,
  LucidePlay,
  LucideQrCode,
  LucideRotateCcw,
  LucideShieldAlert,
  LucideSkipForward,
  LucideWifiOff,
  LucideX,
} from '@lucide/angular';
import { OUTCOME_LABEL } from '../../core/calls';
import { buildCommunityStories } from '../../core/community-story';
import { ConsoleStore } from '../../core/console-store';
import { STAGED_INFO, type StagedKind } from '../../core/demo-staged';
import { formatDuration, formatPhone, formatTime, isE164 } from '../../core/format';
import {
  handoffView,
  liveStages,
  modelAvailability,
  modelEvidence,
  presentFocus,
} from '../../core/live';
import { resolvePresentation } from '../../core/present-selection';
import { gsap, Motion } from '../../shared/motion';
import { Wordmark } from '../../shared/wordmark';
import { DecisionSpine } from '../decision-spine';
import { HandoffTimeline } from '../handoff-timeline';
import { ModelEvidence } from '../model-evidence';
import { ReputationPanel } from '../reputation-panel';
import { StatusBug } from '../status-bug';
import { CommunityStoryView } from './community-story';
import { PhoneQr } from './phone-qr';
import { PresentState } from './present-state';
import { StageRail } from './stage-rail';

type Screen = 'unavailable' | 'waiting' | 'missing' | 'call' | 'community';

/**
 * The projected screen for a live demonstration. Caller words as a giant
 * caption, FirstRing's current step as the lower third, and the outcome as the
 * one coloured graphic. Everything comes from receipts; the evidence drawer
 * holds the engineering detail.
 */
@Component({
  selector: 'app-present-page',
  imports: [
    NgTemplateOutlet,
    RouterLink,
    LucideChevronDown,
    Wordmark,
    StatusBug,
    StageRail,
    PhoneQr,
    CommunityStoryView,
    ModelEvidence,
    HandoffTimeline,
    ReputationPanel,
    DecisionSpine,
    LucideShieldAlert,
    LucideCheck,
    LucidePhoneForwarded,
    LucideHourglass,
    LucideBan,
    LucidePhoneOff,
    LucideCircleDashed,
    LucidePanelRightOpen,
    LucideRotateCcw,
    LucideX,
    LucideWifiOff,
    LucideQrCode,
    LucidePlay,
    LucideSkipForward,
  ],
  host: { class: 'block' },
  template: `
    <div class="on-ink flex min-h-dvh flex-col bg-ink text-paper">
      <header class="border-b border-white/10">
        <div class="flex flex-wrap items-center gap-x-4 gap-y-3 px-4 py-3 sm:px-8">
          <a routerLink="/console" class="shrink-0 rounded-lg" aria-label="Back to the dashboard"
            ><app-wordmark [inverse]="true"
          /></a>
          <div class="flex rounded-full bg-ink-3 p-1" role="group" aria-label="Scene">
            <button
              type="button"
              class="min-h-10 rounded-full px-4 text-[1rem] font-bold"
              [class]="ui.scene() === 'call' ? 'bg-paper text-ink' : 'text-paper/85'"
              [attr.aria-pressed]="ui.scene() === 'call'"
              (click)="ui.scene.set('call')"
            >
              Call
            </button>
            <button
              type="button"
              class="min-h-10 rounded-full px-4 text-[1rem] font-bold"
              [class]="ui.scene() === 'community' ? 'bg-paper text-ink' : 'text-paper/85'"
              [attr.aria-pressed]="ui.scene() === 'community'"
              (click)="ui.scene.set('community')"
            >
              Community
            </button>
          </div>
          <div class="hidden sm:flex">
            <ng-container [ngTemplateOutlet]="showing" />
          </div>
          <div class="ml-auto flex flex-wrap items-center gap-2">
            <span class="present-bug"><app-status-bug [connection]="store.connection()" /></span>
            <button
              type="button"
              class="btn btn-quiet"
              [disabled]="!shown()"
              (click)="openEvidence()"
              aria-haspopup="dialog"
            >
              <svg lucidePanelRightOpen [size]="17" aria-hidden="true"></svg> Evidence
            </button>
            <div class="hidden gap-2 sm:flex">
              <ng-container [ngTemplateOutlet]="resetExit" />
            </div>
          </div>
        </div>
        <details class="border-t border-white/10 px-4 sm:hidden">
          <summary
            class="flex min-h-12 cursor-pointer items-center justify-between text-[1.0625rem] font-bold"
          >
            Presenter controls
            <svg lucideChevronDown [size]="20" aria-hidden="true"></svg>
          </summary>
          <div class="space-y-3 pb-4">
            <ng-container [ngTemplateOutlet]="showing" />
            @if (store.isDemo()) {
              <div class="flex flex-wrap gap-2">
                <ng-container [ngTemplateOutlet]="stagedButtons" />
              </div>
            }
            <div class="flex flex-wrap gap-2"><ng-container [ngTemplateOutlet]="resetExit" /></div>
          </div>
        </details>
        <ng-template #showing>
          <label class="flex min-w-0 flex-1 items-center gap-2">
            <span class="text-[0.95rem] font-bold text-muted-on-ink">Showing</span>
            <select
              class="min-h-11 min-w-0 flex-1 rounded-full border-2 border-white/20 bg-ink px-4 text-[1rem] font-bold text-paper sm:w-[19rem] sm:flex-none"
              [value]="selectValue()"
              (change)="choose($any($event.target).value)"
              aria-describedby="follow-help"
            >
              <option value="follow">Follow newest call</option>
              @for (call of recent(); track call.id) {
                <option [value]="call.id">{{ optionLabel(call) }}</option>
              }
            </select>
          </label>
        </ng-template>
        <ng-template #resetExit>
          <button type="button" class="btn btn-quiet" (click)="reset()">
            <svg lucideRotateCcw [size]="17" aria-hidden="true"></svg> Reset display
          </button>
          <a routerLink="/console" class="btn btn-quiet no-underline">
            <svg lucideX [size]="17" aria-hidden="true"></svg> Exit
          </a>
        </ng-template>
        <ng-template #stagedButtons>
          @for (kind of stagedKinds; track kind) {
            <button
              type="button"
              class="btn btn-quiet"
              [disabled]="kind === 'maya' && !store.stagedPlayed().includes('branch')"
              [attr.title]="info[kind].description"
              (click)="play(kind)"
            >
              <svg lucidePlay [size]="16" aria-hidden="true"></svg> {{ info[kind].title }}
            </button>
          }
          @if (store.staged()) {
            <button type="button" class="btn btn-quiet" (click)="store.finishStaged()">
              <svg lucideSkipForward [size]="16" aria-hidden="true"></svg> Skip to end
            </button>
          }
        </ng-template>
        <p id="follow-help" class="visually-hidden">
          Follow newest shows only calls that start after the screen was reset, or calls that are
          live now. Earlier calls never take over the screen.
        </p>
        @if (store.isDemo()) {
          <div class="border-t border-white/10 bg-ink-2 px-4 py-3 sm:px-8">
            <div class="flex flex-wrap items-center gap-x-4 gap-y-2">
              <p class="text-[1rem]">
                <strong>Fictional rehearsal.</strong>
                <span class="text-muted-on-ink"> No phone line, model or server is involved.</span>
              </p>
              <div class="hidden flex-wrap gap-2 sm:flex">
                <ng-container [ngTemplateOutlet]="stagedButtons" />
              </div>
              @if (!store.stagedPlayed().includes('branch')) {
                <p class="hidden w-full text-[0.95rem] text-muted-on-ink sm:block">
                  “Maya was ready” plays after “The request changes”, which shares the warning.
                </p>
              }
            </div>
          </div>
        }
        @if (store.connection() === 'paused') {
          <div
            class="flex items-center gap-3 bg-amber-wash px-4 py-3 text-amber-ink sm:px-8"
            role="status"
          >
            <svg lucideWifiOff [size]="20" class="shrink-0" aria-hidden="true"></svg>
            <p class="text-[1.0625rem] font-semibold">
              Signal lost. Showing the last snapshot{{
                lastReceipt() ? ' from ' + lastReceipt() : ''
              }}. Reconnecting automatically.
            </p>
          </div>
        }
        @if (resolution().newer; as newer) {
          <div
            class="flex flex-wrap items-center gap-3 bg-paper px-4 py-3 text-ink sm:px-8"
            role="status"
          >
            <p class="text-[1.0625rem] font-semibold">
              A new call started at {{ clock(newer.startedAt) }}. This screen stays on the pinned
              call.
            </p>
            <button type="button" class="btn btn-ink" (click)="ui.pin(newer.id)">
              Show the new call
            </button>
          </div>
        }
      </header>

      <main id="present-main" class="flex-1 px-4 py-8 sm:px-8 sm:py-10" #stage>
        @switch (screen()) {
          @case ('unavailable') {
            <section class="mx-auto max-w-[60rem] py-10" aria-labelledby="unavailable-heading">
              <p class="bug bug-lost">
                <svg lucideWifiOff [size]="15" aria-hidden="true"></svg> Signal lost
              </p>
              <h1
                id="unavailable-heading"
                class="mt-6 text-[clamp(2.5rem,1.5rem+4vw,5rem)] font-extrabold leading-[1.02] tracking-[-0.035em]"
              >
                Service unavailable
              </h1>
              <p
                class="mt-4 max-w-[48ch] text-[clamp(1.25rem,1rem+0.8vw,1.75rem)] text-muted-on-ink"
              >
                The dashboard lost its connection to the FirstRing server. This is not the same as
                waiting for a call. It reconnects automatically.
              </p>
            </section>
          }
          @case ('waiting') {
            <section
              class="grid items-center gap-10 lg:grid-cols-12"
              aria-labelledby="waiting-heading"
            >
              <div class="lg:col-span-7">
                <p class="flex items-center gap-3 text-[1.25rem] font-bold">
                  <span class="relative flex size-4" aria-hidden="true">
                    <span
                      class="absolute inset-0 rounded-full bg-lime"
                      [class.waiting-ping]="!motion.reduced()"
                    ></span>
                    <span class="relative size-4 rounded-full bg-lime"></span>
                  </span>
                  {{
                    store.isDemo()
                      ? 'Rehearsal ready. Play a fictional call above.'
                      : 'Connected. Waiting for a call.'
                  }}
                </p>
                <h1
                  id="waiting-heading"
                  class="mt-8 text-[clamp(2.5rem,1.4rem+4.4vw,5.5rem)] font-extrabold leading-[1] tracking-[-0.035em]"
                >
                  Call FirstRing
                </h1>
                @if (number(); as n) {
                  <p
                    class="tabular mt-4 text-[clamp(2.25rem,1.2rem+4.2vw,5.25rem)] font-extrabold leading-[1.05] tracking-[-0.02em] text-lime"
                  >
                    {{ n.display }}
                  </p>
                } @else {
                  <p class="mt-4 max-w-[40ch] text-[1.375rem] text-muted-on-ink">
                    {{
                      store.isDemo()
                        ? 'The demo view has no phone line.'
                        : 'No inbound number is configured on this server.'
                    }}
                  </p>
                }
                <ul
                  class="mt-8 max-w-[44rem] border-t border-white/15"
                  aria-label="Service readiness"
                >
                  @for (item of readiness(); track item.label) {
                    <li
                      class="flex flex-wrap items-baseline justify-between gap-x-6 gap-y-1 border-b border-white/10 py-3"
                    >
                      <span class="flex items-center gap-2 text-[1.125rem] font-bold">
                        @if (item.ok) {
                          <svg lucideCheck [size]="18" class="text-lime" aria-hidden="true"></svg>
                        } @else {
                          <svg
                            lucideCircleDashed
                            [size]="18"
                            class="text-[#f4c46b]"
                            aria-hidden="true"
                          ></svg>
                        }
                        {{ item.label }}
                      </span>
                      <span class="text-[1rem] text-muted-on-ink">{{ item.detail }}</span>
                    </li>
                  }
                </ul>
                @if (resolution().latest; as latest) {
                  <p class="mt-6 text-[1.0625rem] text-muted-on-ink">
                    Earlier calls stay off this screen.
                    <button
                      type="button"
                      class="ml-1 font-bold text-paper underline underline-offset-4"
                      (click)="ui.pin(latest.id)"
                    >
                      Show the latest call ({{ clock(latest.startedAt) }})
                    </button>
                  </p>
                }
              </div>
              <div class="flex flex-col items-center gap-6 lg:col-span-5">
                @if (number(); as n) {
                  @if (ui.showQr()) {
                    <div
                      class="w-full max-w-[22rem] rounded-[var(--radius-card)] bg-white p-5 text-ink"
                    >
                      <app-phone-qr [number]="n.e164" />
                      <p class="mt-3 text-center text-[1rem] font-bold">Scan to call FirstRing</p>
                    </div>
                  }
                  <button
                    type="button"
                    class="btn btn-quiet"
                    [attr.aria-pressed]="ui.showQr()"
                    (click)="ui.showQr.set(!ui.showQr())"
                  >
                    <svg lucideQrCode [size]="17" aria-hidden="true"></svg>
                    {{ ui.showQr() ? 'Hide QR code' : 'Show QR code' }}
                  </button>
                }
                @if (!ui.showQr()) {
                  <img
                    src="media/ringo-rest-480.webp"
                    alt=""
                    width="480"
                    height="480"
                    class="w-[min(18rem,60vw)] opacity-95"
                  />
                }
              </div>
            </section>
          }
          @case ('missing') {
            <section class="mx-auto max-w-[60rem] py-10">
              <h1 class="text-[clamp(2rem,1.4rem+2.6vw,3.5rem)] font-extrabold">
                That call is not in the current data.
              </h1>
              <p class="mt-3 text-[1.25rem] text-muted-on-ink">
                It may belong to a different data source or was reset on the server.
              </p>
              <button type="button" class="btn btn-primary mt-6" (click)="reset()">
                Follow newest call
              </button>
            </section>
          }
          @case ('community') {
            <section class="mx-auto max-w-[80rem]" aria-label="Community protection">
              @if (story(); as s) {
                <app-community-story [story]="s" variant="ink" [timezone]="timezone()">
                  @if (storyIsFixture()) {
                    <p class="mt-4 text-[1.0625rem] font-bold">
                      <span class="bug bug-demo mr-2">Demo</span> Fictional rehearsal data.
                    </p>
                  }
                </app-community-story>
              } @else {
                <h1
                  class="text-[clamp(2.25rem,1.4rem+3.2vw,4.5rem)] font-extrabold leading-[1.05] tracking-[-0.03em]"
                >
                  No shared warning yet.
                </h1>
                <p class="mt-4 max-w-[52ch] text-[1.25rem] text-muted-on-ink">
                  When a protected request is blocked, FirstRing can share a minimized warning
                  (request type, pretext and pressure, with an expiry) with other participating
                  households.
                  {{ store.isDemo() ? 'Play “The request changes” to rehearse it.' : '' }}
                </p>
              }
            </section>
          }
          @case ('call') {
            @let call = shown()!;
            @let f = focus()!;
            <article
              class="grid gap-x-10 gap-y-8 lg:grid-cols-12"
              aria-labelledby="present-caption"
            >
              <div class="lg:col-span-8">
                <p
                  class="flex flex-wrap items-center gap-x-3 gap-y-2 text-[1.0625rem] font-bold text-muted-on-ink"
                >
                  @if (f.live) {
                    <span class="chip chip-live">On the line</span>
                  } @else {
                    <span class="chip bg-ink-3 text-paper">{{ outcomeLabel(call.outcome) }}</span>
                  }
                  <span>Call for {{ call.name }}</span>
                  <span class="tabular whitespace-nowrap"
                    ><span aria-hidden="true">· </span>{{ clock(call.startedAt) }}</span
                  >
                  @if ((call.durationSeconds ?? 0) >= 1) {
                    <span class="tabular whitespace-nowrap"
                      ><span aria-hidden="true">· </span>{{ duration(call.durationSeconds) }}</span
                    >
                  }
                  @if (ui.selection().mode === 'pinned') {
                    <span class="bug bug-off">Pinned</span>
                  }
                </p>

                <div class="mt-6">
                  <p class="caption-speaker !text-[0.95rem]">
                    &gt;&gt; Caller{{ f.caller?.partial ? ' (speaking)' : '' }}
                  </p>
                  <p
                    id="present-caption"
                    class="present-caption mt-2 font-extrabold leading-[1.12] tracking-[-0.025em]"
                    [class]="captionClass()"
                    [class.opacity-80]="f.caller?.partial"
                    aria-live="polite"
                  >
                    {{ f.caller?.text ?? 'Waiting for the caller to speak…'
                    }}{{ f.caller?.partial ? '…' : '' }}
                  </p>
                </div>

                @if (f.branchAt !== undefined) {
                  @let before = f.turns[f.branchAt - 1];
                  @let after = f.turns[f.branchAt];
                  <ol
                    class="mt-8 max-w-[52rem] border-t-2 border-paper/40"
                    aria-label="How the request changed"
                  >
                    <li class="grid grid-cols-[2.25rem_1fr] gap-3 border-b border-white/10 py-3">
                      <span
                        class="mt-1.5 size-5 rounded-full border-2 border-muted-on-ink bg-ink-3"
                        aria-hidden="true"
                      ></span>
                      <p class="text-[1.125rem]">
                        <strong>Before:</strong> an ordinary request.
                        <span class="text-muted-on-ink line-through decoration-paper/50"
                          >“{{ clip(before.text) }}”</span
                        >
                      </p>
                    </li>
                    <li class="grid grid-cols-[2.25rem_1fr] gap-3 border-b border-white/10 py-3">
                      <span
                        class="mt-0.5 grid size-8 place-items-center rounded-md bg-magenta-on-ink text-ink"
                        ><svg lucideShieldAlert [size]="17" aria-hidden="true"></svg
                      ></span>
                      <p class="text-[1.125rem]">
                        <strong class="text-magenta-on-ink">Then the request changed.</strong>
                        {{ after.summary }}
                      </p>
                    </li>
                  </ol>
                }

                <div
                  class="lower-third mt-8 max-w-[56rem] overflow-hidden rounded-[var(--radius-band)] bg-paper text-ink shadow-[var(--shadow-lift)]"
                >
                  <div
                    class="lower-tally flex items-center justify-between gap-3 px-5 py-2"
                    [class]="tallyClass()"
                  >
                    <span class="text-[0.95rem] font-extrabold uppercase tracking-[0.06em]">{{
                      tallyLabel()
                    }}</span>
                    @if (f.step) {
                      <time class="tabular text-[0.95rem] font-bold">{{ clock(f.step.at) }}</time>
                    }
                  </div>
                  <div class="px-5 py-5">
                    <p
                      class="text-[clamp(1.5rem,1.1rem+1.5vw,2.5rem)] font-extrabold leading-[1.12] tracking-[-0.02em]"
                    >
                      {{ f.step?.label ?? 'Answering' }}
                    </p>
                    @if (f.step?.detail; as detail) {
                      <p class="mt-1 text-[1.1875rem] text-ink-3">{{ detail }}</p>
                    }
                    @if (f.firstring) {
                      <p class="mt-4 border-t border-paper-3 pt-3 text-[1.125rem] leading-snug">
                        <span class="caption-speaker !text-[0.95rem] !text-lime-ink"
                          >&gt;&gt; FirstRing</span
                        >
                        {{ f.firstring.text }}
                      </p>
                    }
                  </div>
                </div>
              </div>

              <aside class="flex flex-col gap-6 lg:col-span-4" aria-label="Request and outcome">
                <div>
                  <h2 class="text-[1.0625rem] font-bold text-muted-on-ink">
                    What the caller wants
                  </h2>
                  <p class="mt-2 text-[clamp(1.25rem,1rem+0.8vw,1.75rem)] font-bold leading-snug">
                    {{ f.request }}
                  </p>
                </div>
                <div
                  class="outcome-graphic rounded-[var(--radius-card)] p-6"
                  [class]="outcomeClass()"
                  role="status"
                  aria-live="polite"
                >
                  <h2
                    class="flex items-start gap-3 text-[clamp(1.6rem,1.2rem+1.4vw,2.6rem)] font-extrabold leading-[1.08] tracking-[-0.02em]"
                  >
                    <span class="visually-hidden">Outcome: </span>
                    @switch (f.outcome.kind) {
                      @case ('blocked') {
                        <svg
                          lucideShieldAlert
                          [size]="36"
                          class="mt-1 shrink-0"
                          aria-hidden="true"
                        ></svg>
                      }
                      @case ('saved') {
                        <svg lucideCheck [size]="36" class="mt-1 shrink-0" aria-hidden="true"></svg>
                      }
                      @case ('connected') {
                        <svg
                          lucidePhoneForwarded
                          [size]="36"
                          class="mt-1 shrink-0"
                          aria-hidden="true"
                        ></svg>
                      }
                      @case ('permission') {
                        <svg
                          lucideHourglass
                          [size]="36"
                          class="mt-1 shrink-0"
                          aria-hidden="true"
                        ></svg>
                      }
                      @case ('withdrawn') {
                        <svg lucideBan [size]="36" class="mt-1 shrink-0" aria-hidden="true"></svg>
                      }
                      @case ('declined') {
                        <svg
                          lucidePhoneOff
                          [size]="36"
                          class="mt-1 shrink-0"
                          aria-hidden="true"
                        ></svg>
                      }
                      @default {
                        <svg
                          lucideCircleDashed
                          [size]="36"
                          class="mt-1 shrink-0"
                          aria-hidden="true"
                        ></svg>
                      }
                    }
                    <span>{{ f.outcome.label }}</span>
                  </h2>
                  <p class="mt-2 text-[1.125rem] leading-snug opacity-90">{{ f.outcome.detail }}</p>
                </div>
                @if (handoff(); as h) {
                  <div>
                    <h2 class="text-[1.125rem] font-bold text-muted-on-ink">Private permission</h2>
                    <ol class="mt-2 space-y-1.5">
                      @for (step of h.steps; track step.id) {
                        <li
                          class="flex items-start gap-2 text-[1.0625rem]"
                          [class]="
                            !step.observed
                              ? 'text-muted-on-ink'
                              : step.tone === 'protect'
                                ? 'font-bold text-magenta-on-ink'
                                : 'font-bold text-paper'
                          "
                        >
                          @if (!step.observed) {
                            <svg
                              lucideCircleDashed
                              [size]="17"
                              class="mt-1 shrink-0"
                              aria-hidden="true"
                            ></svg>
                          } @else if (step.tone === 'protect') {
                            <svg
                              lucideBan
                              [size]="17"
                              class="mt-1 shrink-0"
                              aria-hidden="true"
                            ></svg>
                          } @else if (step.tone === 'neutral') {
                            <svg
                              lucidePhoneOff
                              [size]="17"
                              class="mt-1 shrink-0"
                              aria-hidden="true"
                            ></svg>
                          } @else {
                            <svg
                              lucideCheck
                              [size]="17"
                              class="mt-1 shrink-0 text-lime"
                              aria-hidden="true"
                            ></svg>
                          }
                          {{ step.label
                          }}<span class="visually-hidden">{{
                            step.observed ? ': receipt observed' : ': no receipt'
                          }}</span>
                        </li>
                      }
                    </ol>
                    @if (h.terminal; as t) {
                      <p
                        class="mt-2 text-[1.0625rem] font-bold"
                        [class]="
                          t.kind === 'withdrawn' || t.kind === 'stale_rejected'
                            ? 'text-magenta-on-ink'
                            : 'text-paper'
                        "
                      >
                        {{ t.label }}
                      </p>
                    }
                  </div>
                }
              </aside>

              <div class="lg:col-span-12">
                <app-stage-rail [stages]="stages()" [timezone]="timezone()" [max]="5" />
              </div>
            </article>
          }
        }
      </main>

      <footer class="border-t border-white/10 px-4 py-4 sm:px-8">
        <div class="flex flex-wrap items-center justify-between gap-x-8 gap-y-2 text-[1rem]">
          <p class="font-bold">{{ credit() }}</p>
          <p class="text-muted-on-ink">
            The model proposes; FirstRing’s own rules decide. Caller ID is never proof.
          </p>
        </div>
      </footer>

      <dialog
        #drawer
        class="evidence-drawer m-0 ml-auto h-dvh max-h-dvh w-full max-w-[46rem] bg-paper p-0 text-ink shadow-[var(--shadow-lift)]"
        aria-labelledby="evidence-heading"
        (close)="drawerOpen.set(false)"
      >
        @if (drawerOpen() && shown(); as call) {
          <div
            class="on-ink sticky top-0 z-10 flex items-center justify-between gap-4 bg-ink px-5 py-4 text-paper sm:px-7"
          >
            <h2 id="evidence-heading" class="text-[1.375rem] font-extrabold">
              Evidence for this call
            </h2>
            <button type="button" class="btn btn-quiet" (click)="closeEvidence()">
              <svg lucideX [size]="18" aria-hidden="true"></svg> Close
            </button>
          </div>
          <div class="space-y-10 px-5 py-6 sm:px-7">
            @if (store.isDemo()) {
              <p class="rounded-xl bg-paper-2 p-4 text-[1.0625rem]">
                <strong>Demo data.</strong> Authored, fictional receipts. No model, phone line or
                server produced them.
              </p>
            }
            <section aria-labelledby="ev-model">
              <h3 id="ev-model" class="text-[1.25rem] font-extrabold">NVIDIA decision model</h3>
              <div class="mt-3"><app-model-evidence [call]="call" /></div>
            </section>
            @if (handoff()) {
              <section aria-labelledby="ev-handoff">
                <h3 id="ev-handoff" class="text-[1.25rem] font-extrabold">
                  Private permission (handoff)
                </h3>
                <app-handoff-timeline [call]="call" />
              </section>
            }
            <app-reputation-panel [sessionId]="call.id" />
            <section aria-labelledby="ev-spine">
              <h3 id="ev-spine" class="text-[1.25rem] font-extrabold">Every receipt</h3>
              <p class="mt-1 text-[1rem] text-muted">
                What FirstRing heard, what the model proposed, which rule decided, and what was
                carried out. Each step opens its raw JSON.
              </p>
              <div class="mt-4"><app-decision-spine [call]="call" /></div>
            </section>
            <a class="btn btn-ink no-underline" [routerLink]="['/console/calls', call.id]"
              >Open the full call page</a
            >
          </div>
        }
      </dialog>
    </div>
  `,
})
export class PresentPage {
  readonly store = inject(ConsoleStore);
  readonly ui = inject(PresentState);
  readonly motion = inject(Motion);
  private readonly drawer = viewChild.required<ElementRef<HTMLDialogElement>>('drawer');
  private readonly stage = viewChild<ElementRef<HTMLElement>>('stage');
  readonly drawerOpen = signal(false);
  private readonly rendered = signal(false);
  private returnFocus?: HTMLElement;
  readonly info = STAGED_INFO;
  readonly stagedKinds: StagedKind[] = ['connected', 'branch', 'maya'];

  readonly resolution = computed(() =>
    resolvePresentation(this.store.calls(), this.ui.selection(), this.store.now()),
  );
  readonly shown = computed(() => this.resolution().call);
  readonly focus = computed(() => {
    const call = this.shown();
    return call ? presentFocus(call) : undefined;
  });
  readonly stages = computed(() => {
    const call = this.shown();
    return call ? liveStages(call) : [];
  });
  readonly handoff = computed(() => {
    const call = this.shown();
    return call ? handoffView(call) : undefined;
  });
  readonly recent = computed(() => this.store.calls().slice(0, 15));
  readonly timezone = computed(() => this.store.state()?.profile?.timezone);
  readonly credit = computed(() => modelEvidence(this.shown(), this.store.state()).credit);
  readonly selectValue = computed(() => {
    const s = this.ui.selection();
    return s.mode === 'pinned' && s.pinnedId ? s.pinnedId : 'follow';
  });
  readonly number = computed(() => {
    const raw = this.store.state()?.phone_display;
    return isE164(raw) ? { e164: raw, display: formatPhone(raw) } : undefined;
  });
  readonly stories = computed(() =>
    buildCommunityStories(this.store.state(), this.store.events(), this.store.calls()),
  );
  readonly story = computed(() => this.stories()[0]);
  readonly storyIsFixture = computed(() => this.store.isDemo());
  readonly lastReceipt = computed(() => {
    const at = this.store.lastReceiptAt();
    return at ? formatTime(at, this.timezone(), true) : '';
  });

  readonly screen = computed<Screen>(() => {
    if (this.ui.scene() === 'community') return 'community';
    if (this.shown()) return 'call';
    if (this.store.connection() === 'paused') return 'unavailable';
    if (this.resolution().missing) return 'missing';
    return 'waiting';
  });

  readonly readiness = computed(() => {
    const state = this.store.state();
    if (!state) return [];
    if (this.store.isDemo())
      return [
        { label: 'Demo data', ok: true, detail: 'Fictional calls play in this browser only' },
      ];
    const cap = (id: string) => state.capabilities.find((c) => c.id === id);
    const ok = (id: string) => ['configured', 'verified'].includes(cap(id)?.status ?? '');
    const transport =
      state.transport === 'relay'
        ? { ok: true, detail: 'Live conversation (ConversationRelay)' }
        : state.transport === 'gather'
          ? { ok: true, detail: 'Turn-by-turn speech' }
          : {
              ok: false,
              detail: `Phone-line test mode (${state.transport}); callers hear a keypad test`,
            };
    const handoff = cap('handoff');
    return [
      {
        label: 'Phone line',
        ok: ok('inbound'),
        detail: ok('inbound') ? 'Configured' : 'Not configured',
      },
      { label: 'Screening conversation', ...transport },
      (() => {
        const model = modelAvailability(state);
        return {
          label: 'Decision model',
          ok: model.known ? model.available : !!state.model_backend?.configured,
          detail: model.text,
        };
      })(),
      {
        label: 'Private permission calls',
        ok: !!handoff && ok('handoff'),
        detail: handoff
          ? ok('handoff')
            ? 'Armed on the server'
            : 'Not armed on the server'
          : 'The server does not report this yet',
      },
    ];
  });

  /** Long captions step down so the outcome and rundown stay on a projected screen. */
  readonly captionClass = computed(() => {
    const length = this.focus()?.caller?.text.length ?? 0;
    return length > 110
      ? 'max-w-[32ch] text-[clamp(1.75rem,1rem+2.2vw,3.25rem)]'
      : 'max-w-[28ch] text-[clamp(1.875rem,1.1rem+2.9vw,4.25rem)]';
  });
  readonly tallyLabel = computed(() => {
    const kind = this.focus()?.outcome.kind;
    if (kind === 'blocked') return 'FirstRing · request blocked';
    if (kind === 'withdrawn') return 'FirstRing · permission withdrawn';
    return 'FirstRing · now';
  });
  readonly tallyClass = computed(() => {
    const tone = this.focus()?.step?.tone;
    const kind = this.focus()?.outcome.kind;
    if (kind === 'blocked' || kind === 'withdrawn' || tone === 'protect')
      return 'bg-magenta-ink text-paper';
    if (tone === 'warn' || tone === 'fail') return 'bg-amber-wash text-amber-ink';
    return 'bg-lime text-ink';
  });
  readonly outcomeClass = computed(() => {
    switch (this.focus()?.outcome.kind) {
      case 'blocked':
      case 'withdrawn':
        return 'bg-magenta-wash text-magenta-ink';
      case 'saved':
      case 'connected':
        return 'bg-lime text-ink';
      case 'permission':
        return 'bg-amber-wash text-amber-ink';
      case 'none':
        return 'border-2 border-dashed border-white/30 text-paper';
      default:
        return 'bg-ink-3 text-paper';
    }
  });

  constructor() {
    afterNextRender(() => this.rendered.set(true));
    // Motion only when evidence arrives: a caption wipe and an outcome settle.
    let lastCaption = '';
    let lastOutcome = '';
    effect(() => {
      const f = this.focus();
      if (!this.rendered() || !f) return;
      const caption = `${this.shown()?.id}:${f.caller?.partial ? '' : (f.caller?.text ?? '')}`;
      const outcome = `${this.shown()?.id}:${f.outcome.kind}`;
      untracked(() => {
        const root = this.stage()?.nativeElement;
        const reduced = this.motion.reduced();
        if (
          root &&
          !reduced &&
          lastCaption &&
          caption !== lastCaption &&
          f.caller &&
          !f.caller.partial
        )
          queueMicrotask(() =>
            gsap.fromTo(
              root.querySelector('.present-caption'),
              { clipPath: 'inset(0 100% 0 0)' },
              {
                clipPath: 'inset(0 0% 0 0)',
                duration: 0.45,
                ease: 'expo.out',
                clearProps: 'clipPath',
              },
            ),
          );
        if (root && !reduced && lastOutcome && outcome !== lastOutcome)
          queueMicrotask(() =>
            gsap.fromTo(
              root.querySelector('.outcome-graphic'),
              { scale: 0.96, opacity: 0.5 },
              {
                scale: 1,
                opacity: 1,
                duration: 0.5,
                ease: 'expo.out',
                clearProps: 'transform,opacity',
              },
            ),
          );
        lastCaption = caption;
        lastOutcome = outcome;
      });
    });
    // A pinned call that the presenter is looking at keeps the drawer in sync; nothing else.
    effect(() => {
      if (!this.shown() && this.drawerOpen()) untracked(() => this.closeEvidence());
    });
  }

  choose(value: string) {
    if (value === 'follow') this.ui.follow();
    else this.ui.pin(value);
  }

  play(kind: StagedKind) {
    if (this.store.playStaged(kind)) {
      this.ui.follow();
      this.ui.scene.set('call');
    }
  }

  reset() {
    this.closeEvidence();
    this.ui.reset(Date.now());
  }

  openEvidence() {
    this.returnFocus = document.activeElement as HTMLElement;
    this.drawerOpen.set(true);
    this.drawer().nativeElement.showModal();
  }

  closeEvidence() {
    const dialog = this.drawer().nativeElement;
    if (dialog.open) dialog.close();
    this.drawerOpen.set(false);
    this.returnFocus?.focus();
  }

  optionLabel(call: { startedAt?: string; name: string; outcome: keyof typeof OUTCOME_LABEL }) {
    return `${this.clock(call.startedAt)} · ${call.name} · ${OUTCOME_LABEL[call.outcome]}`;
  }
  outcomeLabel(outcome: keyof typeof OUTCOME_LABEL) {
    return OUTCOME_LABEL[outcome];
  }
  clock(at: string | undefined) {
    return at ? formatTime(at, this.timezone()) : '—';
  }
  duration(seconds: number | undefined) {
    return formatDuration(seconds);
  }
  clip(text: string) {
    return text.length > 90 ? `${text.slice(0, 88).trimEnd()}…` : text;
  }
}
