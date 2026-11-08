import { Component, input } from '@angular/core';

/**
 * FirstRing mark: the lime ring with its magenta "on air" segment, broadcasting
 * two signal arcs. The arcs ripple once when the mark appears and again on hover
 * of the surrounding link; reduced-motion users see the still mark.
 */
@Component({
  selector: 'app-wordmark',
  host: { class: 'fr-wordmark inline-flex' },
  template: `
    <span
      class="inline-flex items-center gap-2.5 font-extrabold tracking-[-0.025em]"
      [class.text-paper]="inverse()"
    >
      <svg viewBox="0 0 40 40" class="fr-mark size-10 shrink-0 -my-1" aria-hidden="true">
        <path
          class="fr-arc fr-arc-2"
          d="M23.01 2.73A20.5 20.5 0 0 1 35.27 14.99"
          fill="none"
          stroke="#76b900"
          stroke-width="2.4"
          stroke-linecap="round"
        />
        <path
          class="fr-arc fr-arc-1"
          d="M20.01 7.03A15.5 15.5 0 0 1 30.97 17.99"
          fill="none"
          stroke="#76b900"
          stroke-width="3"
          stroke-linecap="round"
        />
        <circle cx="16" cy="22" r="10" fill="none" stroke="#76b900" stroke-width="5.4" />
        <path
          class="fr-tally"
          d="M21 13.34A10 10 0 0 1 24.66 17"
          fill="none"
          stroke="#e20074"
          stroke-width="5.4"
        />
      </svg>
      <span class="text-[1.45rem] leading-none">FirstRing</span>
    </span>
  `,
  styles: `
    .fr-arc {
      transform-box: view-box;
      transform-origin: 16px 22px;
    }
    .fr-arc-1 {
      opacity: 0.9;
    }
    .fr-arc-2 {
      opacity: 0.5;
    }
    @media (prefers-reduced-motion: no-preference) {
      .fr-arc {
        animation: fr-ripple 1.6s cubic-bezier(0.16, 1, 0.3, 1) both;
      }
      .fr-arc-2 {
        animation-delay: 0.18s;
      }
      .fr-tally {
        animation: fr-tally 1.6s ease-out 0.1s both;
      }
      :host-context(a:hover) .fr-arc,
      :host-context(a:focus-visible) .fr-arc {
        animation: fr-ripple-again 1.2s cubic-bezier(0.16, 1, 0.3, 1) both;
      }
      :host-context(a:hover) .fr-arc-2,
      :host-context(a:focus-visible) .fr-arc-2 {
        animation-delay: 0.14s;
      }
    }
    @keyframes fr-ripple {
      0% {
        transform: scale(0.55);
        opacity: 0;
      }
      60% {
        opacity: 1;
      }
    }
    @keyframes fr-ripple-again {
      0% {
        transform: scale(0.7);
        opacity: 0;
      }
      55% {
        opacity: 1;
      }
    }
    @keyframes fr-tally {
      0%,
      100% {
        opacity: 1;
      }
      35% {
        opacity: 0.35;
      }
    }
  `,
})
export class Wordmark {
  readonly inverse = input(false);
}
