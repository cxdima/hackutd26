import {
  afterNextRender,
  Component,
  computed,
  effect,
  ElementRef,
  inject,
  input,
  signal,
  untracked,
  viewChild,
} from '@angular/core';
import { LucideCheck, LucideCircle, LucideCircleSlash } from '@lucide/angular';
import type { CommunityStory } from '../../core/community-story';
import { formatTime } from '../../core/format';
import { ACTION_LABEL, humanize, PRETEXT_LABEL, PRESSURE_LABEL } from '../../core/language';
import { gsap, Motion } from '../../shared/motion';

/**
 * "Sarah spotted it. Maya was ready." Two household rings and one shared
 * warning. A connector or ring segment is drawn only when its stage is done.
 */
@Component({
  selector: 'app-community-story',
  imports: [LucideCheck, LucideCircle, LucideCircleSlash],
  template: `
    @let s = story();
    @let ink = variant() === 'ink';
    <figure class="m-0">
      <figcaption>
        <p
          class="text-balance font-extrabold leading-[1.05] tracking-[-0.03em]"
          [class]="
            ink
              ? 'text-[clamp(2.25rem,1.4rem+3.2vw,4.5rem)]'
              : 'text-[clamp(1.75rem,1.3rem+1.6vw,2.6rem)]'
          "
        >
          {{ s.headline }}
        </p>
        <p
          class="mt-3 max-w-[60ch] text-[1.125rem]"
          [class]="ink ? 'text-muted-on-ink' : 'text-ink-3'"
        >
          Same tactic, different number. FirstRing matches what a caller asks for, never who they
          claim to be or which number they use.
        </p>
        <ng-content />
      </figcaption>

      <div class="grid gap-x-10" [class.lg:grid-cols-12]="ink" [class.lg:items-start]="ink">
        <div class="relative mt-8" [class.lg:col-span-7]="ink" #art>
          <svg
            viewBox="0 0 1000 300"
            class="block w-full"
            [class.lg:max-h-[30vh]]="ink"
            aria-hidden="true"
          >
            @for (r of arcs; track r) {
              <circle
                [attr.cx]="220"
                cy="150"
                [attr.r]="r"
                fill="none"
                [attr.stroke]="ink ? '#303832' : '#e3e4da'"
                stroke-width="2"
              />
              <circle
                [attr.cx]="780"
                cy="150"
                [attr.r]="r"
                fill="none"
                [attr.stroke]="ink ? '#303832' : '#e3e4da'"
                stroke-width="2"
              />
            }
            <!-- connectors: solid when the stage is done, dashed otherwise -->
            <path
              class="story-link-a"
              d="M330 150 C 380 150, 400 150, 430 150"
              fill="none"
              stroke-width="6"
              stroke-linecap="round"
              [attr.stroke]="done('committed') ? '#76b900' : ink ? '#59615b' : '#b8bfb6'"
              [attr.stroke-dasharray]="done('committed') ? null : '2 14'"
            />
            <path
              class="story-link-b"
              d="M570 150 C 600 150, 620 150, 670 150"
              fill="none"
              stroke-width="6"
              stroke-linecap="round"
              [attr.stroke]="done('available') ? '#76b900' : ink ? '#59615b' : '#b8bfb6'"
              [attr.stroke-dasharray]="done('available') ? null : '2 14'"
            />
            <!-- source household ring -->
            <circle
              cx="220"
              cy="150"
              r="96"
              fill="none"
              [attr.stroke]="sourceKnown() ? '#76b900' : ink ? '#59615b' : '#b8bfb6'"
              stroke-width="20"
            />
            @if (done('identified')) {
              <path
                class="story-spot"
                d="M288 82 A 96 96 0 0 1 316 150"
                fill="none"
                stroke="#e20074"
                stroke-width="20"
              />
            }
            <!-- shared warning capsule -->
            <rect
              x="430"
              y="112"
              width="140"
              height="76"
              rx="38"
              [attr.fill]="done('committed') ? (ink ? '#f7f7f2' : '#171c18') : 'none'"
              [attr.stroke]="done('committed') ? '#e20074' : ink ? '#59615b' : '#b8bfb6'"
              stroke-width="5"
              [attr.stroke-dasharray]="done('committed') ? null : '8 10'"
            />
            <path
              d="M500 132 L 520 168 L 480 168 Z"
              [attr.fill]="done('committed') ? '#e20074' : 'none'"
              [attr.stroke]="done('committed') ? '#e20074' : ink ? '#59615b' : '#b8bfb6'"
              stroke-width="4"
              stroke-linejoin="round"
            />
            <!-- receiving household ring -->
            <circle
              class="story-target"
              cx="780"
              cy="150"
              r="96"
              fill="none"
              [attr.stroke]="done('available') ? '#76b900' : ink ? '#59615b' : '#b8bfb6'"
              stroke-width="20"
            />
            @if (done('consulted')) {
              <path
                class="story-match"
                d="M712 82 A 96 96 0 0 0 684 150"
                fill="none"
                stroke="#e20074"
                stroke-width="20"
              />
              <circle
                class="story-pulse"
                cx="780"
                cy="150"
                r="96"
                fill="none"
                stroke="#76b900"
                stroke-width="4"
                opacity="0"
              />
            }
          </svg>
          <div class="mt-4 grid grid-cols-3 gap-3 text-center">
            <p>
              <span class="block text-[1.375rem] font-extrabold">{{
                s.sourceName ?? 'Source'
              }}</span>
              <span class="block text-[1rem]" [class]="ink ? 'text-muted-on-ink' : 'text-muted'">{{
                s.sourceName ? 'spotted it' : sourceNote()
              }}</span>
            </p>
            <p>
              <span class="block text-[1.125rem] font-extrabold">Shared warning</span>
              <span class="block text-[1rem]" [class]="ink ? 'text-muted-on-ink' : 'text-muted'">{{
                warningLine()
              }}</span>
            </p>
            <p>
              <span class="block text-[1.375rem] font-extrabold">{{
                s.targetName ?? 'Another household'
              }}</span>
              <span class="block text-[1rem]" [class]="ink ? 'text-muted-on-ink' : 'text-muted'">{{
                done('consulted') ? 'was ready' : done('available') ? 'is ready' : 'not yet'
              }}</span>
            </p>
          </div>
        </div>

        <ol
          class="mt-8 border-t-2"
          [class]="ink ? 'border-paper/40 lg:col-span-5' : 'border-ink'"
          aria-label="Community warning stages"
        >
          @for (stage of s.stages; track stage.id) {
            <li
              class="grid grid-cols-[2rem_1fr] gap-3 border-b py-3 sm:grid-cols-[2rem_1fr_auto]"
              [class]="ink ? 'border-white/10' : 'border-paper-3'"
            >
              <span
                class="mt-0.5 grid size-7 place-items-center rounded-full"
                [class]="
                  stage.status === 'done'
                    ? 'bg-lime text-ink'
                    : ink
                      ? 'bg-ink-3 text-muted-on-ink'
                      : 'bg-paper-2 text-muted'
                "
              >
                @switch (stage.status) {
                  @case ('done') {
                    <svg lucideCheck [size]="16" aria-hidden="true"></svg>
                  }
                  @case ('unavailable') {
                    <svg lucideCircleSlash [size]="15" aria-hidden="true"></svg>
                  }
                  @default {
                    <svg lucideCircle [size]="11" aria-hidden="true"></svg>
                  }
                }
              </span>
              <div>
                <p class="text-[1.125rem] font-bold">
                  {{ stage.label
                  }}<span
                    class="ml-2 text-[0.95rem] font-semibold"
                    [class]="ink ? 'text-muted-on-ink' : 'text-muted'"
                    >{{ statusWord(stage.status) }}</span
                  >
                </p>
                <p
                  class="max-w-[65ch] text-[1rem]"
                  [class]="ink ? 'text-muted-on-ink' : 'text-ink-3'"
                >
                  {{ stage.detail }}
                </p>
              </div>
              @if (stage.at) {
                <time
                  class="tabular col-start-2 text-[0.95rem] font-semibold sm:col-start-3"
                  [class]="ink ? 'text-muted-on-ink' : 'text-muted'"
                  >{{ time(stage.at) }}</time
                >
              }
            </li>
          }
        </ol>
      </div>
    </figure>
  `,
})
export class CommunityStoryView {
  readonly story = input.required<CommunityStory>();
  readonly variant = input<'ink' | 'paper'>('paper');
  readonly timezone = input<string>();
  private readonly motion = inject(Motion);
  private readonly art = viewChild<ElementRef<HTMLElement>>('art');
  private readonly rendered = signal(false);
  readonly arcs = [124, 150];
  private previous = new Set<string>();

  readonly sourceKnown = computed(() => !!this.story().sourceName);
  readonly sourceNote = computed(() =>
    this.story().provenance === 'designated_synthetic_test' ? 'a designated test' : 'not linked',
  );
  readonly warningLine = computed(() => {
    const t = this.story().tactic;
    if (!t) return `version ${this.story().version}`;
    return `${humanize(t.requested_action, ACTION_LABEL)} · ${humanize(t.pretext_category, PRETEXT_LABEL)} · ${humanize(t.pressure_category, PRESSURE_LABEL)}`;
  });

  constructor() {
    afterNextRender(() => this.rendered.set(true));
    effect(() => {
      const stages = this.story().stages;
      if (!this.rendered()) return;
      const done = new Set(stages.filter((s) => s.status === 'done').map((s) => s.id));
      const fresh = [...done].filter((id) => !this.previous.has(id));
      const first = this.previous.size === 0;
      this.previous = done;
      untracked(() => {
        const root = this.art()?.nativeElement;
        if (!root || first || this.motion.reduced() || !fresh.length) return;
        queueMicrotask(() => {
          if (fresh.includes('consulted'))
            gsap.fromTo(
              root.querySelector('.story-pulse'),
              { attr: { r: 96 }, opacity: 0.9 },
              { attr: { r: 150 }, opacity: 0, duration: 1.1, ease: 'expo.out' },
            );
          for (const [id, cls] of [
            ['committed', '.story-link-a'],
            ['available', '.story-link-b'],
          ] as const)
            if (fresh.includes(id)) {
              const el = root.querySelector<SVGPathElement>(cls);
              if (!el) continue;
              const length = el.getTotalLength();
              gsap.fromTo(
                el,
                { strokeDasharray: length, strokeDashoffset: length },
                {
                  strokeDashoffset: 0,
                  duration: 0.7,
                  ease: 'expo.out',
                  clearProps: 'strokeDasharray,strokeDashoffset',
                },
              );
            }
        });
      });
    });
  }

  done(id: string) {
    return this.story().stages.some((s) => s.id === id && s.status === 'done');
  }
  statusWord(status: string) {
    return status === 'done'
      ? 'Receipt observed'
      : status === 'pending'
        ? 'Not yet'
        : 'Not available';
  }
  time(at: string) {
    return formatTime(at, this.timezone(), true);
  }
}
