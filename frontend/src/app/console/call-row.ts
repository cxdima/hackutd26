import { Component, computed, inject, input } from '@angular/core';
import { RouterLink } from '@angular/router';
import { LucideChevronRight } from '@lucide/angular';
import type { CallRecord } from '../core/calls';
import { ConsoleStore } from '../core/console-store';
import { formatDuration, formatShortDateTime, relativeAge } from '../core/format';
import { OutcomeChip } from './outcome-chip';

/** One rundown row: time, outcome, plain-language bulletin. The whole row is one link. */
@Component({
  selector: 'app-call-row',
  imports: [RouterLink, OutcomeChip, LucideChevronRight],
  template: `
    <a
      [routerLink]="['/console/calls', call().id]"
      queryParamsHandling="preserve"
      class="group grid gap-x-5 gap-y-2 rounded-xl px-4 py-4 text-ink no-underline transition-colors hover:bg-paper-2 sm:grid-cols-[8.5rem_1fr_auto] sm:items-center"
      [class.!bg-paper-3]="selected()"
      [attr.aria-current]="selected() ? 'page' : null"
    >
      <span class="flex flex-col">
        <time class="text-[1rem] font-bold" [attr.datetime]="call().startedAt">{{ when() }}</time>
        <span class="text-[0.95rem] text-muted"
          >{{ age() }}{{ call().durationSeconds !== undefined ? ' · ' + duration() : '' }}</span
        >
      </span>
      <span class="min-w-0">
        <span class="flex flex-wrap items-center gap-2">
          <app-outcome-chip [outcome]="call().outcome" />
          <span class="text-[0.95rem] font-semibold text-muted">for {{ call().name }}</span>
        </span>
        <span class="mt-1.5 block text-[1.0625rem] leading-snug" [class.line-clamp-2]="compact()">{{
          call().headline
        }}</span>
      </span>
      <svg
        lucideChevronRight
        [size]="20"
        class="hidden text-muted transition-transform group-hover:translate-x-0.5 sm:block"
        aria-hidden="true"
      ></svg>
    </a>
  `,
})
export class CallRow {
  readonly call = input.required<CallRecord>();
  readonly selected = input(false);
  readonly compact = input(true);
  private readonly store = inject(ConsoleStore);
  readonly when = computed(() =>
    formatShortDateTime(this.call().startedAt ?? this.call().lastActivityAt, this.call().timezone),
  );
  readonly age = computed(() =>
    relativeAge(this.call().startedAt ?? this.call().lastActivityAt, this.store.now()),
  );
  readonly duration = computed(() => formatDuration(this.call().durationSeconds));
}
