import { Component, computed, input } from '@angular/core';
import type { Stage } from '../../core/live';
import { formatTime } from '../../core/format';

/**
 * The rundown of evidence-backed stages, oldest to newest. Only stages with a
 * receipt appear; the newest is the "now" stage.
 */
@Component({
  selector: 'app-stage-rail',
  template: `
    <ol
      class="flex flex-col gap-2 sm:flex-row sm:flex-wrap sm:items-stretch sm:gap-x-1"
      [attr.aria-label]="label()"
    >
      @if (hidden() > 0) {
        <li class="flex items-center px-2 text-[0.95rem] font-bold text-muted-on-ink">
          +{{ hidden() }} earlier
        </li>
      }
      @for (stage of visible(); track stage.sequence; let last = $last) {
        <li class="flex items-center gap-1">
          <span
            class="stage-chip w-full sm:w-auto"
            [attr.data-tone]="stage.tone"
            [class.stage-now]="last"
            [attr.aria-current]="last ? 'step' : null"
          >
            <span class="stage-dot" aria-hidden="true"></span>
            <span>{{ stage.label }}</span>
            <time
              class="tabular ml-auto whitespace-nowrap pl-2 font-semibold opacity-75 sm:ml-0 sm:pl-0"
              [attr.datetime]="stage.at"
              >{{ time(stage.at) }}</time
            >
          </span>
          @if (!last) {
            <svg
              viewBox="0 0 16 16"
              class="hidden size-4 shrink-0 text-muted-on-ink sm:block"
              aria-hidden="true"
            >
              <path
                d="M5 3l5 5-5 5"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
              />
            </svg>
          }
        </li>
      }
    </ol>
  `,
})
export class StageRail {
  readonly stages = input.required<Stage[]>();
  readonly timezone = input<string>();
  readonly max = input(6);
  readonly label = input('Call stages');
  readonly visible = computed(() => this.stages().slice(-this.max()));
  readonly hidden = computed(() => Math.max(0, this.stages().length - this.max()));
  time(at: string) {
    return formatTime(at, this.timezone());
  }
}
