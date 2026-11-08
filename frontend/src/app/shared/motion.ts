import { DestroyRef, inject, Injectable, signal } from '@angular/core';
import { gsap } from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';

gsap.registerPlugin(ScrollTrigger);
export { gsap, ScrollTrigger };

/** Tracks prefers-reduced-motion live. Every animation has a complete static alternative. */
@Injectable({ providedIn: 'root' })
export class Motion {
  private readonly query =
    typeof matchMedia === 'function' ? matchMedia('(prefers-reduced-motion: reduce)') : undefined;
  readonly reduced = signal(this.query?.matches ?? false);

  constructor() {
    const destroy = inject(DestroyRef);
    const listener = (event: MediaQueryListEvent) => this.reduced.set(event.matches);
    this.query?.addEventListener('change', listener);
    destroy.onDestroy(() => this.query?.removeEventListener('change', listener));
  }
}
