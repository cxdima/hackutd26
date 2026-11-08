import { Component, input } from '@angular/core';

/** FirstRing wordmark: a lime ring with one magenta segment, then the name. */
@Component({
  selector: 'app-wordmark',
  template: `
    <span
      class="inline-flex items-center gap-2.5 font-extrabold tracking-[-0.02em]"
      [class.text-paper]="inverse()"
    >
      <svg viewBox="0 0 32 32" class="size-8 shrink-0" aria-hidden="true">
        <circle cx="16" cy="16" r="11" fill="none" stroke="#76b900" stroke-width="5.5" />
        <path d="M24.2 8.6a11 11 0 0 1 2.5 4.2" fill="none" stroke="#e20074" stroke-width="5.5" />
      </svg>
      <span class="text-[1.4rem] leading-none">FirstRing</span>
    </span>
  `,
})
export class Wordmark {
  readonly inverse = input(false);
}
