import { Component, computed, input, signal } from '@angular/core';
import {
  LucideChevronDown,
  LucideBan,
  LucideBrain,
  LucideCheck,
  LucideCircle,
  LucideEar,
  LucideFileText,
  LucideMessageCircle,
  LucidePhone,
  LucideShieldAlert,
  LucideShieldCheck,
  LucideTriangleAlert,
  LucideUsers,
} from '@lucide/angular';
import { buildTrace, type CallRecord, type TraceStep } from '../core/calls';
import { formatTime } from '../core/format';

/**
 * The exposed decision spine: every receipt for a call, grouped by caller turn,
 * from the words FirstRing heard to the rule it applied and what actually ran.
 * Proposals are marked as proposals until a matching execution receipt exists.
 */
@Component({
  selector: 'app-decision-spine',
  imports: [
    LucideChevronDown,
    LucideEar,
    LucideBrain,
    LucideShieldCheck,
    LucideShieldAlert,
    LucideCheck,
    LucideMessageCircle,
    LucideFileText,
    LucideUsers,
    LucidePhone,
    LucideTriangleAlert,
    LucideBan,
    LucideCircle,
  ],
  template: `
    <div class="flex flex-wrap items-center justify-between gap-3">
      <p class="max-w-[62ch] text-[1.0625rem] text-ink-3">
        Each step is a receipt the server recorded, in order. “Proposed” means the model suggested
        it; “Done” means Rust executed it after its own checks.
      </p>
      <label
        class="inline-flex min-h-11 cursor-pointer items-center gap-3 text-[1rem] font-semibold"
      >
        <input
          type="checkbox"
          class="size-5 accent-ink"
          [checked]="showAll()"
          (change)="showAll.set(!showAll())"
        />
        Show low-level receipts
      </label>
    </div>

    <ol class="mt-6 space-y-8">
      @for (round of rounds(); track round.index) {
        <li>
          <details
            class="group rounded-[var(--radius-card)] border-2 border-paper-3"
            [open]="round.index > 0"
          >
            <summary
              class="flex min-h-14 cursor-pointer list-none flex-wrap items-center justify-between gap-3 rounded-[calc(var(--radius-card)-2px)] bg-paper-2 px-5 py-3 group-open:rounded-b-none"
            >
              <h3 class="flex items-center gap-2 text-[1.125rem] font-extrabold">
                <svg
                  lucideChevronDown
                  [size]="20"
                  class="-rotate-90 transition-transform group-open:rotate-0"
                  aria-hidden="true"
                ></svg>
                {{ round.label }}
                <span class="text-[0.95rem] font-semibold text-muted"
                  >· {{ round.steps.length }} receipts</span
                >
              </h3>
              @if (round.verdict === 'protected') {
                <span class="chip chip-blocked"
                  ><svg lucideShieldAlert [size]="16" aria-hidden="true"></svg> Protected
                  request</span
                >
              } @else if (round.verdict === 'allowed') {
                <span class="chip chip-saved"
                  ><svg lucideShieldCheck [size]="16" aria-hidden="true"></svg> Ordinary
                  request</span
                >
              } @else if (round.verdict === 'unclear') {
                <span class="chip chip-review"
                  ><svg lucideTriangleAlert [size]="16" aria-hidden="true"></svg> No accepted
                  assessment</span
                >
              }
            </summary>
            @if (round.verdictText) {
              <p class="px-5 pt-4 text-[1.0625rem] font-semibold">Why: {{ round.verdictText }}</p>
            }
            <ol class="relative px-5 pb-5 pt-4">
              <span
                class="absolute bottom-6 left-[2.15rem] top-6 w-0.5 bg-paper-3 sm:left-[9.15rem]"
                aria-hidden="true"
              ></span>
              @for (step of round.steps; track step.id) {
                <li
                  class="relative grid grid-cols-[2.4rem_1fr] gap-x-3 py-2.5 sm:grid-cols-[7.5rem_2.4rem_1fr]"
                >
                  <span
                    class="hidden pt-2 text-right text-[0.8125rem] font-extrabold uppercase tracking-[0.08em] text-muted sm:block"
                  >
                    {{ step.station }}
                    <time
                      class="block font-semibold normal-case tracking-normal tabular"
                      [attr.datetime]="step.at"
                      >{{ time(step.at) }}</time
                    >
                  </span>
                  <span
                    class="relative z-10 grid size-9 place-items-center rounded-full border-2"
                    [class]="nodeClass(step)"
                    aria-hidden="true"
                  >
                    @switch (step.kind) {
                      @case ('heard') {
                        <svg lucideEar [size]="16"></svg>
                      }
                      @case ('model') {
                        <svg lucideBrain [size]="16"></svg>
                      }
                      @case ('rule') {
                        @if (step.tone === 'protect') {
                          <svg lucideShieldAlert [size]="16"></svg>
                        } @else {
                          <svg lucideShieldCheck [size]="16"></svg>
                        }
                      }
                      @case ('executed') {
                        <svg lucideCheck [size]="16"></svg>
                      }
                      @case ('spoke') {
                        <svg lucideMessageCircle [size]="16"></svg>
                      }
                      @case ('saved') {
                        <svg lucideFileText [size]="16"></svg>
                      }
                      @case ('community') {
                        <svg lucideUsers [size]="16"></svg>
                      }
                      @case ('phone') {
                        <svg lucidePhone [size]="16"></svg>
                      }
                      @case ('failure') {
                        <svg lucideTriangleAlert [size]="16"></svg>
                      }
                      @case ('superseded') {
                        <svg lucideBan [size]="16"></svg>
                      }
                      @default {
                        <svg lucideCircle [size]="12"></svg>
                      }
                    }
                  </span>
                  <div class="min-w-0 pb-1">
                    <p
                      class="text-[0.8125rem] font-extrabold uppercase tracking-[0.08em] text-muted sm:hidden"
                    >
                      {{ step.station }} · {{ time(step.at) }}
                    </p>
                    <p
                      class="flex flex-wrap items-center gap-2 text-[1.0625rem] font-bold leading-snug"
                    >
                      {{ step.title }}
                      @if (step.proposal) {
                        @if (executed(step)) {
                          <span
                            class="rounded-md px-2 py-0.5 text-[0.8125rem] font-extrabold uppercase tracking-[0.06em] text-ink-3 shadow-[inset_0_0_0_1.5px_var(--color-ink-3)]"
                            >Proposed · executed below</span
                          >
                        } @else {
                          <span
                            class="rounded-md bg-amber-wash px-2 py-0.5 text-[0.8125rem] font-extrabold uppercase tracking-[0.06em] text-amber-ink"
                            >Proposed · not executed</span
                          >
                        }
                      }
                    </p>
                    @if (step.quote) {
                      <blockquote
                        class="mt-2 rounded-lg px-3.5 py-2.5 text-[1.0625rem] leading-snug"
                        [class]="step.kind === 'heard' ? 'caption-band' : 'bg-paper-2'"
                      >
                        @if (step.kind === 'heard') {
                          <span class="caption-speaker mr-1.5">&gt;&gt; Caller</span>
                        }
                        “{{ step.quote }}”
                      </blockquote>
                    }
                    @if (step.detail) {
                      <p class="mt-1.5 text-[1rem] text-ink-3">{{ step.detail }}</p>
                    }
                    @if (step.facts?.length) {
                      <dl class="mt-2 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-[0.95rem]">
                        @for (fact of step.facts; track fact.label) {
                          <dt class="font-bold text-muted">{{ fact.label }}</dt>
                          <dd class="break-words">{{ fact.value }}</dd>
                        }
                      </dl>
                    }
                    <details class="mt-1.5">
                      <summary
                        class="inline-flex min-h-9 cursor-pointer items-center text-[0.9375rem] font-semibold text-muted underline decoration-dotted underline-offset-4"
                      >
                        Raw receipt #{{ step.sequence }} · {{ step.event.type }}
                      </summary>
                      <pre
                        class="mt-2 max-h-72 overflow-auto rounded-lg bg-ink p-3 text-[0.8125rem] leading-relaxed text-paper"
                        >{{ json(step) }}</pre>
                    </details>
                  </div>
                </li>
              }
            </ol>
          </details>
        </li>
      } @empty {
        <li class="rounded-xl bg-paper-2 p-5 text-[1.0625rem]">
          No receipts have been received for this call yet.
        </li>
      }
    </ol>
  `,
})
export class DecisionSpine {
  readonly call = input.required<CallRecord>();
  readonly showAll = signal(false);
  readonly rounds = computed(() => buildTrace(this.call(), this.showAll()));
  private readonly executedKeys = computed(() => {
    const keys = new Set<string>();
    for (const event of this.call().events)
      if (event.type === 'agent.tool.result' && event.payload['ok'] === true)
        keys.add(`${event.payload['name']}#${event.payload['generation_id']}`);
    return keys;
  });

  executed(step: TraceStep): boolean {
    return this.executedKeys().has(
      `${step.event.payload['name']}#${step.event.payload['generation_id']}`,
    );
  }

  time(at: string) {
    return formatTime(at, this.call().timezone, true);
  }

  nodeClass(step: TraceStep): string {
    switch (step.tone) {
      case 'ok':
        return 'border-lime bg-lime-wash text-lime-ink';
      case 'protect':
        return 'border-magenta-ink bg-magenta-wash text-magenta-ink';
      case 'warn':
        return 'border-amber-ink bg-amber-wash text-amber-ink';
      case 'fail':
        return 'border-ink bg-ink text-paper';
      default:
        return step.proposal
          ? 'border-ink-3 bg-paper text-ink-3 border-dashed'
          : 'border-ink-3 bg-paper text-ink';
    }
  }

  json(step: TraceStep) {
    return JSON.stringify(step.event, null, 2);
  }
}
