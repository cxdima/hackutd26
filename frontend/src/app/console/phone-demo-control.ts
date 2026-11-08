import { Component, computed, inject, input, signal } from '@angular/core';
import { LucideLoader, LucidePhoneForwarded } from '@lucide/angular';
import { ConsoleStore, PHONE_DEMO_ARM, PHONE_DEMO_DISARM } from '../core/console-store';
import { formatTime } from '../core/format';

/**
 * "Ring my phone on the next call". Arming is one explicit click and places no
 * call by itself: only the next real call to the FirstRing number can ring the
 * enrolled phone. Live servers only; renders nothing in the demo.
 */
@Component({
  selector: 'app-phone-demo-control',
  imports: [LucidePhoneForwarded, LucideLoader],
  host: { class: 'contents' },
  template: `
    @if (status(); as s) {
      @if (armed()) {
        <span class="flex flex-wrap items-center gap-x-3 gap-y-2">
          <span class="text-[1rem] font-bold" [class]="textClass()" role="status"
            >Ready to ring your phone until {{ until() }}</span
          >
          @if (s.can_disarm && s.arm_id) {
            <button
              type="button"
              class="btn btn-quiet !min-h-10 !px-3.5 !py-1.5 !text-[0.95rem]"
              [disabled]="!!store.busy()"
              (click)="disarm(s.arm_id)"
            >
              @if (store.busy() === disarmPath) {
                <svg lucideLoader [size]="15" class="animate-spin" aria-hidden="true"></svg>
              }
              Disarm
            </button>
          }
        </span>
      } @else if (s.can_arm) {
        <button
          type="button"
          [class]="tone() === 'ink' ? 'btn btn-quiet' : 'btn btn-ink'"
          [disabled]="!!store.busy()"
          (click)="arm()"
        >
          @if (store.busy() === armPath) {
            <svg lucideLoader [size]="17" class="animate-spin" aria-hidden="true"></svg>
          } @else {
            <svg lucidePhoneForwarded [size]="17" aria-hidden="true"></svg>
          }
          Ring my phone on the next call
        </button>
      }
      @if (failed()) {
        <span class="text-[0.95rem] font-semibold" [class]="errorClass()" role="alert">{{
          failed()
        }}</span>
      }
    }
  `,
})
export class PhoneDemoControl {
  readonly store = inject(ConsoleStore);
  /** 'ink' in the presentation header, 'paper' in settings. */
  readonly tone = input<'paper' | 'ink'>('paper');
  readonly armPath = PHONE_DEMO_ARM;
  readonly disarmPath = PHONE_DEMO_DISARM;
  readonly failed = signal('');

  readonly status = computed(() =>
    this.store.source() === 'live' ? this.store.state()?.phone_demo : undefined,
  );
  readonly armed = computed(() => {
    const s = this.status();
    if (s?.state !== 'armed') return false;
    const expires = s.expires_at ? Date.parse(s.expires_at) : NaN;
    return Number.isNaN(expires) || expires > this.store.now();
  });
  readonly until = computed(() => {
    const at = this.status()?.expires_at;
    return at ? formatTime(at, this.store.state()?.profile?.timezone) : 'it expires';
  });
  readonly textClass = computed(() => (this.tone() === 'ink' ? 'text-lime' : 'text-lime-ink'));
  readonly errorClass = computed(() =>
    this.tone() === 'ink' ? 'text-magenta-on-ink' : 'text-magenta-ink',
  );

  async arm() {
    this.failed.set('');
    if (!(await this.store.armPhoneDemo()))
      this.failed.set(`Not armed. ${this.store.error() || 'Try again.'}`.trim());
  }

  async disarm(armId: string) {
    this.failed.set('');
    if (!(await this.store.disarmPhoneDemo(armId)))
      this.failed.set(`Still armed. ${this.store.error() || 'Try again.'}`.trim());
  }
}
