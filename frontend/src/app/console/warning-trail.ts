import { Component, computed, input } from '@angular/core';
import { RouterLink } from '@angular/router';
import { LucideCheck, LucideCircle, LucideCircleSlash } from '@lucide/angular';
import type { CommunityStory, StoryStatus } from '../core/community-story';
import { formatTime } from '../core/format';
import { ACTION_LABEL, humanize, PRETEXT_LABEL, PRESSURE_LABEL } from '../core/language';

/**
 * Dashboard-sized version of "how the latest warning travelled": one line of
 * two household rings and the shared warning between them, then the five
 * receipt-backed stages as a compact rundown. The presentation screen keeps
 * the large three-ring diagram.
 */
@Component({
  selector: 'app-warning-trail',
  imports: [RouterLink, LucideCheck, LucideCircle, LucideCircleSlash],
  template: `
    @let s = story();
    <div class="grid gap-x-10 gap-y-6 lg:grid-cols-[minmax(0,22rem)_minmax(0,1fr)]">
      <div>
        <p class="text-[1.375rem] leading-snug font-extrabold tracking-[-0.015em] text-balance">
          {{ s.headline }}
        </p>
        <p class="mt-2 text-[1rem] text-ink-3">
          Same tactic, different number. Lines match what a caller asks for, never who they claim to
          be or which number they use.
        </p>

        <svg
          viewBox="0 0 320 72"
          class="trail-art mt-5 block w-full max-w-[22rem]"
          aria-hidden="true"
        >
          <line
            x1="62"
            y1="36"
            x2="122"
            y2="36"
            [attr.stroke]="done('committed') ? '#76b900' : '#b8bfb6'"
            stroke-width="4"
            stroke-linecap="round"
            [attr.stroke-dasharray]="done('committed') ? null : '1 9'"
          />
          <line
            x1="198"
            y1="36"
            x2="258"
            y2="36"
            [attr.stroke]="done('available') ? '#76b900' : '#b8bfb6'"
            stroke-width="4"
            stroke-linecap="round"
            [attr.stroke-dasharray]="done('available') ? null : '1 9'"
          />
          <circle
            cx="36"
            cy="36"
            r="22"
            fill="none"
            [attr.stroke]="s.sourceName ? '#76b900' : '#b8bfb6'"
            stroke-width="8"
          />
          @if (done('identified')) {
            <path
              d="M51.6 20.4 A 22 22 0 0 1 58 36"
              fill="none"
              stroke="#e20074"
              stroke-width="8"
            />
          }
          <rect
            x="122"
            y="18"
            width="76"
            height="36"
            rx="18"
            [attr.fill]="done('committed') ? '#171c18' : 'none'"
            [attr.stroke]="done('committed') ? '#e20074' : '#b8bfb6'"
            stroke-width="3"
            [attr.stroke-dasharray]="done('committed') ? null : '5 6'"
          />
          <path
            d="M160 27 L 169 44 L 151 44 Z"
            [attr.fill]="done('committed') ? '#e20074' : 'none'"
            [attr.stroke]="done('committed') ? '#e20074' : '#b8bfb6'"
            stroke-width="2.5"
            stroke-linejoin="round"
          />
          <circle
            cx="284"
            cy="36"
            r="22"
            fill="none"
            [attr.stroke]="done('available') ? '#76b900' : '#b8bfb6'"
            stroke-width="8"
          />
          @if (done('consulted')) {
            <path
              d="M268.4 20.4 A 22 22 0 0 0 262 36"
              fill="none"
              stroke="#e20074"
              stroke-width="8"
            />
          }
        </svg>
        <div class="mt-2 grid max-w-[22rem] grid-cols-3 text-[0.95rem] leading-snug">
          <p>
            <span class="block font-extrabold">{{ s.sourceName ?? 'Source' }}</span>
            <span class="text-muted">{{ s.sourceName ? 'spotted it' : sourceNote() }}</span>
          </p>
          <p class="text-center">
            <span class="block font-extrabold">Warning v{{ s.version }}</span>
            <span class="text-muted">{{ done('committed') ? 'shared' : 'not yet' }}</span>
          </p>
          <p class="text-right">
            <span class="block font-extrabold">{{ s.targetName ?? 'Other line' }}</span>
            <span class="text-muted">{{ targetNote() }}</span>
          </p>
        </div>
      </div>

      <div class="min-w-0">
        <ol class="border-t-2 border-ink" aria-label="Community warning stages">
          @for (stage of s.stages; track stage.id) {
            <li
              class="grid grid-cols-[1.75rem_minmax(0,1fr)_auto] items-baseline gap-x-3 border-b border-paper-3 py-2.5"
            >
              <span
                class="grid size-6 translate-y-1 place-items-center rounded-full"
                [class]="stage.status === 'done' ? 'bg-lime text-ink' : 'bg-paper-3 text-muted'"
              >
                @switch (stage.status) {
                  @case ('done') {
                    <svg lucideCheck [size]="14" aria-hidden="true"></svg>
                  }
                  @case ('unavailable') {
                    <svg lucideCircleSlash [size]="13" aria-hidden="true"></svg>
                  }
                  @default {
                    <svg lucideCircle [size]="9" aria-hidden="true"></svg>
                  }
                }
              </span>
              <p class="min-w-0">
                <span class="text-[1.0625rem] font-bold">{{ stage.label }}</span>
                <span class="ml-2 text-[0.95rem] font-semibold text-muted">{{
                  word(stage.status)
                }}</span>
                <span class="block text-[0.95rem] text-ink-3">{{ stage.detail }}</span>
              </p>
              @if (stage.at) {
                <time
                  class="tabular text-[0.95rem] font-semibold text-muted"
                  [attr.datetime]="stage.at"
                  >{{ time(stage.at) }}</time
                >
              } @else {
                <span></span>
              }
            </li>
          }
        </ol>
        @if (s.tactic) {
          <p class="mt-3 text-[0.95rem] text-muted">
            What was shared: {{ warningLine() }}. No words, names or numbers from the call.
          </p>
        }
        @if (s.sourceCallId) {
          <a
            [routerLink]="['/console/calls', s.sourceCallId]"
            queryParamsHandling="preserve"
            class="more-link"
            >Open the call that started it</a
          >
        }
      </div>
    </div>
  `,
})
export class WarningTrail {
  readonly story = input.required<CommunityStory>();
  readonly timezone = input<string>();

  readonly sourceNote = computed(() =>
    this.story().provenance === 'designated_synthetic_test' ? 'a designated test' : 'not linked',
  );
  readonly targetNote = computed(() => {
    const plural = / and /.test(this.story().targetName ?? '');
    if (this.done('consulted')) return plural ? 'were ready' : 'was ready';
    if (this.done('available')) return plural ? 'are ready' : 'is ready';
    return 'not yet';
  });
  readonly warningLine = computed(() => {
    const t = this.story().tactic;
    if (!t) return '';
    return `${humanize(t.requested_action, ACTION_LABEL)}, ${humanize(t.pretext_category, PRETEXT_LABEL).toLowerCase()}, ${humanize(t.pressure_category, PRESSURE_LABEL).toLowerCase()}`;
  });

  done(id: string) {
    return this.story().stages.some((s) => s.id === id && s.status === 'done');
  }
  word(status: StoryStatus) {
    return status === 'done'
      ? 'Receipt observed'
      : status === 'pending'
        ? 'Not yet'
        : 'Not available';
  }
  time(at: string) {
    return formatTime(at, this.timezone(), true);
  }
}
