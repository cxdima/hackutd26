import { Component, computed, input } from '@angular/core';
import {
  LucideCheck,
  LucideCircleDashed,
  LucideCircleMinus,
  LucideEye,
  LucidePhoneCall,
  LucidePhoneForwarded,
  LucideShieldAlert,
} from '@lucide/angular';
import { OUTCOME_CHIP, OUTCOME_LABEL, type Outcome } from '../core/calls';

/** The icon that pairs with an outcome's words. */
@Component({
  selector: 'app-outcome-icon',
  imports: [
    LucideCheck,
    LucideShieldAlert,
    LucideEye,
    LucideCircleDashed,
    LucidePhoneCall,
    LucidePhoneForwarded,
    LucideCircleMinus,
  ],
  host: { class: 'inline-flex shrink-0' },
  template: `
    @switch (outcome()) {
      @case ('blocked') {
        <svg lucideShieldAlert [size]="size()" aria-hidden="true"></svg>
      }
      @case ('connected') {
        <svg lucidePhoneForwarded [size]="size()" aria-hidden="true"></svg>
      }
      @case ('message_saved') {
        <svg lucideCheck [size]="size()" aria-hidden="true"></svg>
      }
      @case ('needs_review') {
        <svg lucideEye [size]="size()" aria-hidden="true"></svg>
      }
      @case ('in_progress') {
        <svg lucidePhoneCall [size]="size()" aria-hidden="true"></svg>
      }
      @case ('no_message') {
        <svg lucideCircleMinus [size]="size()" aria-hidden="true"></svg>
      }
      @default {
        <svg lucideCircleDashed [size]="size()" aria-hidden="true"></svg>
      }
    }
  `,
})
export class OutcomeIcon {
  readonly outcome = input.required<Outcome>();
  readonly size = input(16);
}

/** Outcome label: icon + words; colour is never the only signal. */
@Component({
  selector: 'app-outcome-chip',
  imports: [OutcomeIcon],
  template: `
    <span class="chip" [class]="cls()">
      <app-outcome-icon [outcome]="outcome()" />
      {{ label() }}
    </span>
  `,
})
export class OutcomeChip {
  readonly outcome = input.required<Outcome>();
  readonly label = computed(() => OUTCOME_LABEL[this.outcome()]);
  readonly cls = computed(() => OUTCOME_CHIP[this.outcome()]);
}
