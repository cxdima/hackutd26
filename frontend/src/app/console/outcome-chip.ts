import { Component, computed, input } from '@angular/core';
import {
  LucideCheck,
  LucideCircleDashed,
  LucideCircleMinus,
  LucideEye,
  LucidePhoneCall,
  LucideShieldAlert,
} from '@lucide/angular';
import { OUTCOME_CHIP, OUTCOME_LABEL, type Outcome } from '../core/calls';

/** Outcome label: icon + words; colour is never the only signal. */
@Component({
  selector: 'app-outcome-chip',
  imports: [
    LucideCheck,
    LucideShieldAlert,
    LucideEye,
    LucideCircleDashed,
    LucidePhoneCall,
    LucideCircleMinus,
  ],
  template: `
    <span class="chip" [class]="cls()">
      @switch (outcome()) {
        @case ('blocked') {
          <svg lucideShieldAlert [size]="16" aria-hidden="true"></svg>
        }
        @case ('message_saved') {
          <svg lucideCheck [size]="16" aria-hidden="true"></svg>
        }
        @case ('needs_review') {
          <svg lucideEye [size]="16" aria-hidden="true"></svg>
        }
        @case ('in_progress') {
          <svg lucidePhoneCall [size]="16" aria-hidden="true"></svg>
        }
        @case ('no_message') {
          <svg lucideCircleMinus [size]="16" aria-hidden="true"></svg>
        }
        @default {
          <svg lucideCircleDashed [size]="16" aria-hidden="true"></svg>
        }
      }
      {{ label() }}
    </span>
  `,
})
export class OutcomeChip {
  readonly outcome = input.required<Outcome>();
  readonly label = computed(() => OUTCOME_LABEL[this.outcome()]);
  readonly cls = computed(() => OUTCOME_CHIP[this.outcome()]);
}
