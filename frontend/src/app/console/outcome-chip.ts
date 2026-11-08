import { Component, computed, input } from '@angular/core';
import {
  LucideCheck,
  LucideCircleDashed,
  LucideCircleMinus,
  LucideEye,
  LucidePhoneCall,
  LucidePhoneForwarded,
  LucideShieldAlert,
  LucideUserCheck,
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
    LucideUserCheck,
  ],
  host: { class: 'inline-flex shrink-0' },
  template: `
    @switch (outcome()) {
      @case ('blocked') {
        <svg lucideShieldAlert [size]="size()" aria-hidden="true"></svg>
      }
      @case ('trusted') {
        <svg lucideUserCheck [size]="size()" aria-hidden="true"></svg>
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

export const OUTCOME_SHORT: Record<Outcome, string> = {
  in_progress: 'On the line now',
  trusted: 'Trusted, put through',
  blocked: 'Blocked',
  connected: 'Connected',
  message_saved: 'Message saved',
  needs_review: 'Needs review',
  no_message: 'No message',
  incomplete: 'Incomplete',
};

const OUTCOME_TEXT_CLASS: Record<Outcome, string> = {
  in_progress: 'text-ink',
  trusted: 'outcome-ok',
  blocked: 'outcome-blocked',
  connected: 'outcome-ok',
  message_saved: 'outcome-ok',
  needs_review: 'outcome-review',
  no_message: 'outcome-neutral',
  incomplete: 'outcome-neutral',
};

/** Outcome for tables: icon + short words in the outcome's text colour, no pill. */
@Component({
  selector: 'app-outcome-text',
  imports: [OutcomeIcon],
  template: `
    <span class="outcome" [class]="cls()">
      <app-outcome-icon [outcome]="outcome()" [size]="16" />
      {{ short() ? shortLabel() : label() }}
    </span>
  `,
})
export class OutcomeText {
  readonly outcome = input.required<Outcome>();
  readonly short = input(true);
  readonly label = computed(() => OUTCOME_LABEL[this.outcome()]);
  readonly shortLabel = computed(() => OUTCOME_SHORT[this.outcome()]);
  readonly cls = computed(() => OUTCOME_TEXT_CLASS[this.outcome()]);
}
