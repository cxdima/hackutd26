import { Component, computed, input } from '@angular/core';
import {
  LucideCircleDot,
  LucideLock,
  LucidePlay,
  LucideRadio,
  LucideWifiOff,
  LucideLoader,
} from '@lucide/angular';
import type { Connection } from '../core/console-store';

/** The station bug: one per view. Word + shape, never colour alone. */
@Component({
  selector: 'app-status-bug',
  imports: [LucideRadio, LucidePlay, LucideCircleDot, LucideWifiOff, LucideLock, LucideLoader],
  template: `
    <span class="bug" [class]="cls()" role="status" [attr.aria-label]="'Data source: ' + text()">
      @switch (connection()) {
        @case ('live') {
          <svg lucideRadio [size]="15" aria-hidden="true"></svg>
        }
        @case ('replay') {
          <svg lucidePlay [size]="15" aria-hidden="true"></svg>
        }
        @case ('demo') {
          <svg lucideCircleDot [size]="15" aria-hidden="true"></svg>
        }
        @case ('paused') {
          <svg lucideWifiOff [size]="15" aria-hidden="true"></svg>
        }
        @case ('connecting') {
          <svg lucideLoader [size]="15" class="animate-spin" aria-hidden="true"></svg>
        }
        @default {
          <svg lucideLock [size]="15" aria-hidden="true"></svg>
        }
      }
      {{ text() }}
    </span>
  `,
})
export class StatusBug {
  readonly connection = input.required<Connection>();
  readonly text = computed(
    () =>
      ({
        live: 'Live',
        replay: 'Replay',
        demo: 'Demo',
        paused: 'Signal lost',
        connecting: 'Connecting',
        locked: 'Locked',
      })[this.connection()],
  );
  readonly cls = computed(
    () =>
      ({
        live: 'bug-live',
        replay: 'bug-replay',
        demo: 'bug-demo',
        paused: 'bug-lost',
        connecting: 'bug-off',
        locked: 'bug-off',
      })[this.connection()],
  );
}
