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
import { RouterLink } from '@angular/router';
import {
  LucideArrowDown,
  LucideArrowRight,
  LucideAudioLines,
  LucideBan,
  LucideCheck,
  LucideCode,
  LucideExternalLink,
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
    LucideCode,
    LucideExternalLink,
    LucideCircleDashed,
    LucideAudioLines,
    LucideFingerprint,
    LucideMessageSquareText,
    LucideBan,
    LucidePhoneOff,
    LucideEarOff,
  ],
  templateUrl: './landing-page.html',
  styles: `
    .hero-title {
      font-size: clamp(2.75rem, 0.9rem + 3.4vw, 4.6rem);
      line-height: 1;
      letter-spacing: -0.035em;
      text-wrap: balance;
    }
    .hero-art-mask {
      -webkit-mask-image:
        linear-gradient(to right, transparent 0, #000 32%),
        linear-gradient(to bottom, transparent 0, #000 5%, #000 78%, transparent 100%);
      -webkit-mask-composite: source-in;
      mask-image:
        linear-gradient(to right, transparent 0, #000 32%),
        linear-gradient(to bottom, transparent 0, #000 5%, #000 78%, transparent 100%);
      mask-composite: intersect;
    }
    .hero-mark {
      background-image: linear-gradient(var(--color-lime), var(--color-lime));
      background-repeat: no-repeat;
      background-size: 100% 0.2em;
      background-position: 0 86%;
      padding-inline: 0.03em;
    }
  `,
})
export class LandingPage {
  readonly motion = inject(Motion);
  readonly heroPoints = [
    { icon: 'shield', text: 'Never shares codes, PINs or SSNs' },
    { icon: 'rules', text: 'Nemotron proposes; fixed rules decide' },
    { icon: 'check', text: 'Trusted callers skip screening' },
  ];
  readonly communityVideo: VideoSource = {
    mp4: 'media/community-loop.mp4',
    webm: 'media/community-loop.webm',
    poster: 'media/community-loop-poster.jpg',
  };
  readonly exampleNumbers = ['(555) 010-2234', '(555) 014-7781', '(555) 019-3305'];
  /** Only what has receipts today; everything else is listed under "Next". */
  readonly today = [
    'Answers calls on a real phone number and asks what the call is about, tested with automated test callers.',
    'Judges each turn with NVIDIA Nemotron. FirstRing’s own rules decide what is allowed, not the model.',
    'Refuses requests for codes, passwords, Social Security numbers, payments or device access, and saves a private note.',
    'Takes ordinary messages, asks a follow-up when a detail is missing, and shows every decision in the dashboard.',
    'Reports numbers that ask for codes; the next call from that number starts with the history.',
    'Shares expiring tactic warnings between two fictional households.',
  ];
  readonly next = [
    {
      title: 'Ringing your own phone first',
      text: 'A private summary, then press 1 to connect. Built and armed on demand; not yet shown on a real handset.',
    },
    {
      title: 'Trusted callers',
      text: 'Numbers you list skip screening and ring straight through. Built and tested; not yet shown on a real call.',
    },
    {
      title: 'An assistant voice that sounds like you',
      text: 'Record it on one call, with consent and an off switch. Built; not yet recorded.',
    },
    {
      title: 'Call recordings',
      text: 'Announced at the start of each call and playable in the dashboard. Built; no recording yet.',
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
        'Any audio played to another person can be recorded, including a voice assistant’s. FirstRing uses its own synthetic voice unless you choose to record yours, with consent, and you can switch it off or delete it at any time.',
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
        'Shared tactic warnings expire after 24 hours and number reports after 14 days. The server keeps an audit trail and supports correcting a number report.',
      ],
    },
    {
      q: 'What is available in this prototype?',
      a: [
        'Working today on a demonstration line: answering calls, asking follow-up questions, blocking requests for codes, passwords, Social Security numbers, payments or remote access, saving private summaries, reporting numbers that ask for codes, and sharing expiring tactic warnings between two fictional households.',
        'Built but not yet shown on a real call: ringing your phone first, trusted callers, your own assistant voice and call recordings. Not built: carrier-level blocking and mobile notifications.',
      ],
    },
  ];

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
        {
          scale: 1.5,
          opacity: 0,
          duration: 2.6,
          ease: 'sine.out',
          repeat: 1,
          stagger: 1.3,
          delay: 0.8,
        },
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
  }
}
