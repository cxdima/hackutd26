import {
  afterNextRender,
  Component,
  computed,
  DestroyRef,
  effect,
  ElementRef,
  inject,
  input,
  signal,
  viewChild,
} from '@angular/core';
import { LucidePause, LucidePlay } from '@lucide/angular';
import { Motion } from './motion';

export interface VideoSource {
  mp4: string;
  webm: string;
  poster: string;
}

/**
 * Silent decorative loop. Never autoplays audio (there is none), pauses off-screen,
 * respects reduced motion (poster only until the visitor presses play) and always
 * shows a visible pause control.
 */
@Component({
  selector: 'app-loop-video',
  imports: [LucidePause, LucidePlay],
  template: `
    <div class="relative h-full w-full">
      <video
        #video
        class="h-full w-full object-cover"
        [class]="videoClass()"
        [poster]="active().poster"
        muted
        loop
        playsinline
        disablePictureInPicture
        [attr.preload]="eager() ? 'auto' : 'none'"
        aria-hidden="true"
        tabindex="-1"
      >
        <source [src]="active().mp4" type="video/mp4" />
        <source [src]="active().webm" type="video/webm" />
      </video>
      @if (showControl()) {
        <button
          type="button"
          class="on-ink absolute z-10 inline-flex min-h-11 items-center gap-2 rounded-full bg-ink px-4 py-2 text-[0.95rem] font-bold text-paper shadow-[var(--shadow-soft)] transition-colors hover:bg-ink-3"
          [class]="controlClass()"
          (click)="toggle()"
        >
          @if (wantsPlay()) {
            <svg lucidePause [size]="16" aria-hidden="true"></svg> Pause {{ label() }}
          } @else {
            <svg lucidePlay [size]="16" aria-hidden="true"></svg> Play {{ label() }}
          }
        </button>
      }
    </div>
  `,
})
export class LoopVideo {
  readonly wide = input.required<VideoSource>();
  readonly narrow = input<VideoSource | undefined>(undefined);
  readonly label = input('animation');
  readonly eager = input(false);
  readonly videoClass = input('');
  readonly controlClass = input('bottom-4 right-4');
  /** When set, a parent owns the play/pause state (and its visible control). */
  readonly playingOverride = input<boolean | undefined>(undefined);
  readonly showControl = input(true);
  readonly narrowQuery = input('(max-width: 767px)');

  private readonly motion = inject(Motion);
  private readonly video = viewChild.required<ElementRef<HTMLVideoElement>>('video');
  private readonly visible = signal(false);
  private readonly userPaused = signal<boolean | null>(null);
  private readonly isNarrow = signal(false);
  readonly active = computed(() => (this.isNarrow() && this.narrow()) || this.wide());
  /** The visitor's explicit choice wins; otherwise reduced motion starts paused. */
  readonly wantsPlay = computed(() => {
    const override = this.playingOverride();
    if (override !== undefined) return override;
    return this.userPaused() === null ? !this.motion.reduced() : !this.userPaused();
  });
  readonly playing = signal(false);
  private lastSource = '';

  constructor() {
    const destroy = inject(DestroyRef);
    afterNextRender(() => {
      const element = this.video().nativeElement;
      const observer = new IntersectionObserver(
        ([entry]) => this.visible.set(entry.isIntersecting),
        { threshold: 0.15 },
      );
      observer.observe(element);
      const narrowQuery = matchMedia(this.narrowQuery());
      this.isNarrow.set(narrowQuery.matches);
      const onNarrow = (event: MediaQueryListEvent) => this.isNarrow.set(event.matches);
      narrowQuery.addEventListener('change', onNarrow);
      destroy.onDestroy(() => {
        observer.disconnect();
        narrowQuery.removeEventListener('change', onNarrow);
        element.pause();
      });
    });
    effect(() => {
      const source = this.active();
      const element = this.video().nativeElement;
      if (this.lastSource && this.lastSource !== source.mp4) element.load();
      this.lastSource = source.mp4;
      const shouldPlay = this.wantsPlay() && this.visible();
      if (shouldPlay) {
        if (element.preload === 'none') element.preload = 'auto';
        // The static muted attribute only sets defaultMuted on script-created elements;
        // browsers block autoplay unless the muted property itself is true.
        element.muted = true;
        element.play().then(
          () => this.playing.set(true),
          () => this.playing.set(false),
        );
      } else {
        element.pause();
        this.playing.set(false);
      }
    });
  }

  toggle() {
    this.userPaused.set(this.wantsPlay());
    if (!this.wantsPlay()) this.playing.set(false);
  }
}
