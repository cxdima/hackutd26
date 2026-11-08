import { Component, computed, input } from '@angular/core';
import { LucideCheck, LucideCircle, LucideCircleX, LucideLockKeyhole } from '@lucide/angular';
import type { CallRecord } from '../core/calls';
import { formatTime } from '../core/format';
import { handoffView } from '../core/live';

/**
 * The private handoff as receipts prove it: each step is either observed (with
 * its receipt time) or explicitly "no receipt". Provider acceptance is never
 * shown as ringing, and ringing is never shown as heard.
 */
@Component({
  selector: 'app-handoff-timeline',
  imports: [LucideCheck, LucideCircle, LucideCircleX, LucideLockKeyhole],
  template: `
    @if (view(); as v) {
      <div>
        @if (v.refused) {
          <p class="rounded-xl bg-amber-wash p-4 text-[1.0625rem] text-amber-ink">
            <strong>The model proposed a handoff; Rust refused it.</strong> {{ v.refused }}
          </p>
        } @else if (v.proposed && !v.steps[0].observed) {
          <p class="rounded-xl bg-paper-2 p-4 text-[1.0625rem]">
            The model proposed a handoff. No offer receipt has arrived, so nothing was created.
          </p>
        }
        <ol class="mt-4 border-t-2 border-ink" aria-label="Private handoff receipts">
          @for (step of v.steps; track step.id) {
            <li
              class="grid grid-cols-[2rem_1fr_auto] items-start gap-3 border-b border-paper-3 py-3"
            >
              <span
                class="mt-0.5 grid size-7 place-items-center rounded-full"
                [class]="
                  !step.observed
                    ? 'bg-paper-2 text-muted'
                    : step.tone === 'protect'
                      ? 'rounded-md bg-magenta-wash text-magenta-ink'
                      : step.tone === 'neutral'
                        ? 'bg-paper-3 text-ink-3'
                        : 'bg-lime-wash text-lime-ink'
                "
              >
                @if (!step.observed) {
                  <svg lucideCircle [size]="11" aria-hidden="true"></svg>
                } @else if (step.tone === 'ok') {
                  <svg lucideCheck [size]="16" aria-hidden="true"></svg>
                } @else {
                  <svg lucideCircleX [size]="16" aria-hidden="true"></svg>
                }
              </span>
              <div>
                <p
                  class="text-[1.0625rem] font-bold"
                  [class.text-magenta-ink]="step.observed && step.tone === 'protect'"
                >
                  {{ step.label
                  }}<span class="visually-hidden">{{
                    step.observed ? ': receipt observed' : ': no receipt'
                  }}</span>
                </p>
                <p class="text-[1rem] text-ink-3">{{ step.detail }}</p>
              </div>
              <time class="tabular pt-0.5 text-[0.95rem] font-semibold text-muted">{{
                step.at ? time(step.at) : '—'
              }}</time>
            </li>
          }
        </ol>
        @if (v.terminal; as t) {
          <p
            class="mt-4 flex items-start gap-2 rounded-xl p-4 text-[1.0625rem]"
            [class]="
              t.kind === 'withdrawn' || t.kind === 'stale_rejected'
                ? 'bg-magenta-wash text-magenta-ink'
                : 'bg-paper-2 text-ink'
            "
          >
            <svg lucideCircleX [size]="20" class="mt-0.5 shrink-0" aria-hidden="true"></svg>
            <span
              ><strong>{{ t.label }}.</strong> {{ t.detail }}</span
            >
          </p>
        }
        <p class="mt-3 flex items-center gap-1.5 text-[0.95rem] text-muted">
          <svg lucideLockKeyhole [size]="14" aria-hidden="true"></svg>
          The private summary stays in this household’s dashboard. It is never shared with the
          community.
        </p>
      </div>
    }
  `,
})
export class HandoffTimeline {
  readonly call = input.required<CallRecord>();
  readonly view = computed(() => handoffView(this.call()));
  time(at: string) {
    return formatTime(at, this.call().timezone, true);
  }
}
