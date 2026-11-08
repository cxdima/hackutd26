import {
  afterNextRender,
  Component,
  computed,
  DestroyRef,
  effect,
  ElementRef,
  inject,
  signal,
  untracked,
  viewChild,
} from '@angular/core';
import { RouterLink } from '@angular/router';
import {
  LucideArrowDown,
  LucideArrowRight,
  LucideAudioLines,
  LucideBan,
  LucideCheck,
  LucideChevronDown,
  LucideCircleDashed,
  LucideEarOff,
  LucideFingerprint,
  LucideMessageSquareText,
  LucidePhoneOff,
  LucideShieldAlert,
  LucideShieldCheck,
} from '@lucide/angular';
import { gsap, Motion } from '../shared/motion';
import { LoopVideo, type VideoSource } from '../shared/loop-video';
import { Wordmark } from '../shared/wordmark';
import { CallExample } from './call-example';
import { ScreeningStage } from './screening-stage';
import { SiteHeader } from './site-header';

@Component({
  selector: 'app-landing-page',
  imports: [
    RouterLink,
    SiteHeader,
    Wordmark,
    LoopVideo,
    ScreeningStage,
    CallExample,
    LucideArrowDown,
    LucideArrowRight,
    LucideShieldAlert,
    LucideShieldCheck,
    LucideChevronDown,
    LucideCheck,
    LucideCircleDashed,
    LucideAudioLines,
    LucideFingerprint,
    LucideMessageSquareText,
    LucideBan,
    LucidePhoneOff,
    LucideEarOff,
  ],
  templateUrl: './landing-page.html',
})
export class LandingPage {
  readonly motion = inject(Motion);
  readonly heroVideo: VideoSource = {
    mp4: 'media/ringo-hero-loop.mp4',
    webm: 'media/ringo-hero-loop.webm',
    poster: 'media/ringo-hero-loop-poster.jpg',
  };
  readonly heroVideoNarrow: VideoSource = {
    mp4: 'media/ringo-hero-loop-square.mp4',
    webm: 'media/ringo-hero-loop-square.webm',
    poster: 'media/ringo-hero-square-720.webp',
  };
  readonly communityVideo: VideoSource = {
    mp4: 'media/community-loop.mp4',
    webm: 'media/community-loop.webm',
    poster: 'media/community-loop-poster.jpg',
  };
  readonly exampleNumbers = ['(555) 010-2234', '(555) 014-7781', '(555) 019-3305'];
  /** Only what has receipts today; everything else is listed under "Next". */
  readonly today = [
    'Answers calls on a real phone number and asks what the call is about, tested with automated test callers.',
    'Judges the request with NVIDIA Nemotron. FirstRing’s own rules decide what is allowed, not the model.',
    'Refuses requests for codes, unusual payments or device access, and saves a private note.',
    'Takes ordinary messages and shows each call’s transcript, decisions and receipts in the dashboard.',
    'Shares minimized, expiring tactic warnings between two fictional households.',
  ];
  readonly next = [
    {
      title: 'Ringing your own phone first',
      text: 'A private summary, then press 1 to connect. Built into the server; not yet shown on a real handset.',
    },
    {
      title: 'Reported-number history',
      text: 'Independent reports, expiry and corrections for a number. In development.',
    },
    {
      title: 'An assistant voice that sounds like you',
      text: 'Optional, with explicit consent and an off switch. Designed, not available.',
    },
    {
      title: 'More households',
      text: 'Today’s community is two fictional households.',
    },
  ];

  readonly faqs = [
    {
      q: 'Who is FirstRing for?',
      a: [
        'People who want a little time before answering unfamiliar calls, often adults in their 60s, 70s and beyond, and the family members or caregivers who help them stay independent.',
      ],
    },
    {
      q: 'What happens to an ordinary call?',
      a: [
        'FirstRing answers, asks a short follow-up if something is missing, saves a plain summary, and ends the call politely. You read the summary and decide whether to call back.',
      ],
    },
    {
      q: 'Could someone copy my voice?',
      a: [
        'Any audio played to another person can be recorded, including a voice assistant’s. That’s why FirstRing answers in its own distinct synthetic voice, not yours. Voice cloning is turned off in this prototype.',
        'And because voices can be imitated, sounding familiar is never treated as permission to share a code, send money or give access.',
      ],
    },
    {
      q: 'What does community sharing include?',
      a: [
        'Only a short, expiring description of the trick: what the caller asked for, what they claimed to be, and the kind of pressure used. No names, phone numbers, recordings or transcripts are shared.',
      ],
    },
    {
      q: 'What if a call is flagged incorrectly?',
      a: [
        'Every call in the dashboard shows what FirstRing heard and why it acted, word for word. A blocked call still leaves you a note, so you can call the organization back at a number you already know.',
        'In this prototype, shared tactic warnings expire automatically after 24 hours. A way to report a mistake and correct a shared warning is designed but not connected yet.',
      ],
    },
    {
      q: 'What is available in this prototype?',
      a: [
        'Working today on a demonstration line: answering calls, asking follow-up questions, blocking requests for codes, unusual payments or remote access, saving private summaries, editing the greeting, voice and timezone for new calls, and sharing expiring tactic warnings between two fictional households.',
        'Not available yet: transferring a call to you or a family member, number reputation scores, report corrections, carrier-level blocking, mobile notifications and custom voices.',
      ],
    },
  ];

  /** One visible control pauses all continuous hero motion (video loop and ring pulse). */
  private readonly heroChoice = signal<boolean | null>(null);
  readonly heroPlaying = computed(() => this.heroChoice() ?? !this.motion.reduced());
  private readonly root = viewChild.required<ElementRef<HTMLElement>>('root');
  private readonly rendered = signal(false);
  private context?: gsap.Context;

  constructor() {
    afterNextRender(() => this.rendered.set(true));
    effect(() => {
      if (!this.rendered()) return;
      const reduced = this.motion.reduced();
      untracked(() => this.animate(reduced));
    });
    effect(() => {
      const playing = this.heroPlaying();
      if (this.rendered()) untracked(() => this.syncPause(playing));
    });
    inject(DestroyRef).onDestroy(() => this.context?.revert());
  }

  /** Hero entrance, hero layer depth and the personal → community ring transition. */
  private animate(reduced: boolean) {
    this.context?.revert();
    if (reduced) return;
    const root = this.root().nativeElement;
    this.context = gsap.context(() => {
      // Entrance: only decorative layers move; headline and copy are readable from the first paint.
      gsap.from('[data-hero-art]', { autoAlpha: 0, scale: 0.97, duration: 1.2, ease: 'expo.out' });
      gsap.from('[data-hero-ring]', {
        scale: 0.6,
        autoAlpha: 0,
        duration: 1.4,
        ease: 'expo.out',
        stagger: 0.12,
        delay: 0.25,
      });
      gsap.from('[data-hero-caption]', { y: 28, duration: 1, ease: 'expo.out', delay: 0.45 });
      gsap.fromTo(
        '[data-hero-ring-pulse]',
        { scale: 1, opacity: 0.6 },
        { scale: 1.5, opacity: 0, duration: 2.6, ease: 'sine.out', repeat: -1, stagger: 1.3 },
      );

      // Depth: separately prepared layers drift at different rates while the hero scrolls away.
      const depth = { trigger: '[data-hero]', start: 'top top', end: 'bottom top', scrub: 0.6 };
      gsap.to('[data-hero-art]', { yPercent: 8, ease: 'none', scrollTrigger: depth });
      gsap.to('[data-hero-rings]', { yPercent: 16, ease: 'none', scrollTrigger: depth });
      gsap.to('[data-hero-caption]', { yPercent: -22, ease: 'none', scrollTrigger: depth });

      // Ring transition: one protective ring opens into the shared community rings.
      const bridge = gsap.timeline({
        scrollTrigger: {
          trigger: '[data-bridge]',
          start: 'top 80%',
          end: 'bottom 45%',
          scrub: 0.8,
        },
      });
      bridge
        .from('[data-bridge-home]', { scale: 0.7, transformOrigin: '50% 50%', ease: 'none' })
        .from('[data-bridge-link]', { strokeDashoffset: 1, ease: 'none', stagger: 0.08 }, '<0.1')
        .from(
          '[data-bridge-ring]',
          {
            scale: 0,
            autoAlpha: 0,
            transformOrigin: '50% 50%',
            ease: 'back.out(1.6)',
            stagger: 0.1,
          },
          '<0.15',
        );
    }, root);
    this.syncPause(this.heroPlaying());
  }

  toggleHero() {
    this.heroChoice.set(!this.heroPlaying());
  }

  private syncPause(playing: boolean) {
    const root = this.root().nativeElement;
    gsap
      .getTweensOf(root.querySelectorAll('[data-hero-ring-pulse]'))
      .forEach((tween) => (playing ? tween.resume() : tween.pause()));
  }
}
