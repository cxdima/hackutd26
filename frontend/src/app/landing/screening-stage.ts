import {
  afterNextRender,
  Component,
  DestroyRef,
  effect,
  ElementRef,
  inject,
  signal,
  untracked,
  viewChild,
} from '@angular/core';
import { LucideFastForward, LucidePhoneIncoming, LucideRotateCcw } from '@lucide/angular';
import { gsap, Motion, ScrollTrigger } from '../shared/motion';

/**
 * Staged explanation of one ordinary screened call (written example).
 * Plays once when scrolled into view; reduced motion shows every step at once.
 * The "now" marker always shows which station the call is at.
 */
@Component({
  selector: 'app-screening-stage',
  imports: [LucidePhoneIncoming, LucideRotateCcw, LucideFastForward],
  template: `
    <figure
      #root
      class="caption-band on-ink relative overflow-hidden shadow-[var(--shadow-lift)]"
      aria-labelledby="stage-title"
    >
      <div
        class="flex items-center justify-between gap-3 border-b border-white/10 px-5 py-3.5 sm:px-6"
      >
        <figcaption id="stage-title" class="text-[1.0625rem] font-bold">
          One call, step by step
        </figcaption>
        <span class="bug bug-demo">Example</span>
      </div>

      <ol
        class="relative grid grid-cols-4 border-b border-white/10"
        aria-label="Where the call is now"
      >
        @for (station of stations; track station; let i = $index) {
          <li
            class="flex flex-col items-start gap-1 px-3 py-3 text-[0.95rem] font-bold sm:px-5"
            [class.text-paper]="i <= step()"
            [class.text-muted-on-ink]="i > step()"
            [attr.aria-current]="i === step() ? 'step' : null"
          >
            <span
              class="tabular text-[0.8rem] font-extrabold tracking-[0.08em] uppercase"
              [class.text-lime]="i <= step()"
              >{{ i + 1 }}</span
            >
            {{ station }}
          </li>
        }
        <span
          #now
          class="absolute bottom-0 left-0 h-1 w-1/4 rounded-full bg-lime"
          aria-hidden="true"
        ></span>
      </ol>

      <div class="space-y-3.5 px-5 py-6 sm:px-6" aria-live="off">
        <p data-line="0" class="flex items-center gap-3 text-[1.0625rem] text-muted-on-ink">
          <span class="relative grid size-10 shrink-0 place-items-center rounded-full bg-white/8">
            <svg lucidePhoneIncoming [size]="19" class="text-lime" aria-hidden="true"></svg>
            <span data-ripple class="absolute inset-0 rounded-full border-2 border-lime/60"></span>
          </span>
          <span
            ><strong class="text-paper">Incoming call.</strong> Unfamiliar number. FirstRing answers
            first.</span
          >
        </p>
        <p data-line="1" class="text-[1.125rem] leading-snug">
          <span class="caption-speaker mr-2">&gt;&gt; FirstRing</span>Hi, thanks for calling. What’s
          the call about?
        </p>
        <p data-line="1" class="text-[1.125rem] leading-snug">
          <span class="caption-speaker mr-2">&gt;&gt; Caller</span>Hi, it’s Northside Pharmacy,
          about Sarah’s refill.
        </p>
        <p data-line="1" class="text-[1.125rem] leading-snug">
          <span class="caption-speaker mr-2">&gt;&gt; FirstRing</span>Thanks. What should I tell
          her?
        </p>
        <p data-line="1" class="text-[1.125rem] leading-snug">
          <span class="caption-speaker mr-2">&gt;&gt; Caller</span>It’s ready for pickup after three
          today.
        </p>
        <div
          data-line="2"
          class="grid gap-2 rounded-xl bg-white/6 p-4 sm:grid-cols-[auto_1fr] sm:gap-x-5"
        >
          <span class="caption-speaker">Asked for</span><span>Nothing private</span>
          <span class="caption-speaker">Shared</span><span>A pickup time</span>
          <span class="caption-speaker">Check</span
          ><span class="font-bold text-lime">Ordinary message. Save it.</span>
        </div>
        <div data-line="3" class="overflow-hidden rounded-xl bg-paper text-ink">
          <span
            class="block bg-lime px-4 py-1.5 text-[0.8125rem] font-extrabold uppercase tracking-[0.08em]"
            >Saved for Sarah</span
          >
          <p class="px-4 py-3 text-[1.0625rem] font-semibold leading-snug">
            Northside Pharmacy: refill ready for pickup after 3 today.
          </p>
        </div>
      </div>

      @if (!motion.reduced()) {
        <div class="flex flex-wrap gap-2 border-t border-white/10 px-5 py-3.5 sm:px-6">
          <button type="button" class="btn btn-quiet" (click)="replay()">
            <svg lucideRotateCcw [size]="16" aria-hidden="true"></svg> Replay
          </button>
          <button
            type="button"
            class="btn btn-quiet"
            (click)="finish()"
            [disabled]="step() === 3 && done()"
          >
            <svg lucideFastForward [size]="16" aria-hidden="true"></svg> Show all steps
          </button>
        </div>
      }
    </figure>
  `,
})
export class ScreeningStage {
  readonly motion = inject(Motion);
  readonly stations = ['Answer', 'Listen', 'Check', 'Decide'];
  readonly step = signal(3);
  readonly done = signal(true);
  private readonly root = viewChild.required<ElementRef<HTMLElement>>('root');
  private readonly now = viewChild.required<ElementRef<HTMLElement>>('now');
  private timeline?: gsap.core.Timeline;
  private context?: gsap.Context;

  private readonly rendered = signal(false);

  constructor() {
    afterNextRender(() => this.rendered.set(true));
    effect(() => {
      if (!this.rendered()) return;
      const reduced = this.motion.reduced();
      untracked(() => this.setup(reduced));
    });
    inject(DestroyRef).onDestroy(() => this.context?.revert());
  }

  private setup(reduced: boolean) {
    this.context?.revert();
    this.timeline = undefined;
    const root = this.root().nativeElement;
    const marker = this.now().nativeElement;
    if (reduced) {
      this.step.set(3);
      this.done.set(true);
      gsap.set(marker, { xPercent: 300 });
      return;
    }
    gsap.set(marker, { xPercent: 300 });
    this.context = gsap.context(() => {
      ScrollTrigger.create({
        trigger: root,
        start: 'top 70%',
        once: true,
        onEnter: () => this.replay(),
      });
    }, root);
  }

  private build(): gsap.core.Timeline {
    const root = this.root().nativeElement;
    const marker = this.now().nativeElement;
    let tl!: gsap.core.Timeline;
    this.context?.add(() => {
      const lines = (n: number) => root.querySelectorAll(`[data-line="${n}"]`);
      tl = gsap.timeline({ defaults: { ease: 'expo.out', duration: 0.7 } });
      const at = (station: number) => () => this.step.set(station);
      tl.call(() => this.done.set(false))
        .set(root.querySelectorAll('[data-line]'), { autoAlpha: 0, y: 14 })
        .set(marker, { xPercent: 0 })
        .call(at(0))
        .to(lines(0), { autoAlpha: 1, y: 0 })
        .fromTo(
          root.querySelector('[data-ripple]'),
          { scale: 1, opacity: 0.9 },
          { scale: 1.9, opacity: 0, duration: 1.1, repeat: 1, ease: 'sine.out' },
          '<',
        )
        .call(at(1), undefined, '+=0.2')
        .to(marker, { xPercent: 100, duration: 0.6 }, '<')
        .to(lines(1), { autoAlpha: 1, y: 0, stagger: 0.9 }, '<')
        .call(at(2), undefined, '+=0.4')
        .to(marker, { xPercent: 200, duration: 0.6 }, '<')
        .to(lines(2), { autoAlpha: 1, y: 0 }, '<')
        .call(at(3), undefined, '+=0.8')
        .to(marker, { xPercent: 300, duration: 0.6 }, '<')
        .to(lines(3), { autoAlpha: 1, y: 0 }, '<')
        .call(() => this.done.set(true));
    });
    return tl;
  }

  replay() {
    this.timeline?.progress(1).kill();
    this.timeline = this.build();
  }

  finish() {
    this.timeline?.progress(1);
  }
}
