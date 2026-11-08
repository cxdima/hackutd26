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
      class="group block min-w-0 rounded-xl px-3.5 py-3 text-ink no-underline transition-colors hover:bg-paper-2"
      [class.!bg-paper-3]="selected()"
      [attr.aria-current]="selected() ? 'page' : null"
    >
      <span class="flex items-center justify-between gap-3">
        <app-outcome-chip [outcome]="call().outcome" />
        <svg
          lucideChevronRight
          [size]="18"
          class="shrink-0 text-muted transition-transform group-hover:translate-x-0.5"
          aria-hidden="true"
        ></svg>
      </span>
      <span class="mt-1.5 block text-[1.0625rem] leading-snug" [class.line-clamp-2]="compact()">{{
        call().headline
      }}</span>
      <span class="mt-1 block text-[0.95rem] text-muted"
        ><time
          class="tabular whitespace-nowrap font-semibold text-ink-3"
          [attr.datetime]="call().startedAt"
          >{{ when() }}</time
        >
        · <span class="whitespace-nowrap">For {{ call().name }}</span> ·
        <span class="whitespace-nowrap">{{ age() }}</span>
        @if (call().durationSeconds !== undefined) {
          · <span class="tabular whitespace-nowrap">{{ duration() }}</span>
        }
      </span>
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
