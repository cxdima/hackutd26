import { Component, computed, effect, HostListener, inject, signal } from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import {
  ActivatedRoute,
  NavigationEnd,
  Router,
  RouterLink,
  RouterLinkActive,
  RouterOutlet,
} from '@angular/router';
import {
  LucideContactRound,
  LucideLayoutDashboard,
  LucideMonitorPlay,
  LucideNetwork,
  LucidePanelRightOpen,
  LucidePause,
  LucidePhone,
  LucidePlay,
  LucideSettings2,
  LucideX,
} from '@lucide/angular';
import { Access } from '../core/access';
import { CallerDirectory } from '../core/caller-directory';
import { ConsoleStore } from '../core/console-store';
import { formatPhone, formatTime, isE164 } from '../core/format';
import { household } from '../core/household';
import { Motion } from '../shared/motion';
import { Wordmark } from '../shared/wordmark';
import { ConnectPanel } from './connect-panel';
import { OperatorPanel } from './operator-panel';

const ART_KEY = 'firstring.rail-art-paused';
function readArtPaused(): boolean {
  try {
    return localStorage.getItem(ART_KEY) === '1';
  } catch {
    return false;
  }
}

type NavIcon = 'overview' | 'calls' | 'callers' | 'community' | 'settings';

@Component({
  selector: 'app-console-shell',
  imports: [
    RouterOutlet,
    RouterLink,
    RouterLinkActive,
    Wordmark,
    ConnectPanel,
    OperatorPanel,
    LucideX,
    LucidePanelRightOpen,
    LucideMonitorPlay,
    LucideLayoutDashboard,
    LucidePhone,
    LucideContactRound,
    LucideNetwork,
    LucideSettings2,
    LucidePause,
    LucidePlay,
  ],
  template: `
    @if (presenting()) {
      <a class="skip-link" href="#present-main">Skip to the presentation</a>
      <router-outlet />
    } @else {
      <a class="skip-link" href="#console-main">Skip to dashboard content</a>
      <div class="min-h-dvh lg:grid lg:grid-cols-[16.5rem_minmax(0,1fr)]">
        <aside
          class="console-rail on-ink sticky top-0 z-40 text-paper lg:flex lg:h-dvh lg:flex-col"
          aria-label="FirstRing menu"
        >
          <div
            class="relative z-10 flex min-h-16 items-center gap-3 px-4 lg:min-h-0 lg:px-6 lg:pt-7"
          >
            <a routerLink="/" class="shrink-0 rounded-lg" aria-label="FirstRing home"
              ><app-wordmark [inverse]="true"
            /></a>
            @if (connected()) {
              <div class="ml-auto flex items-center gap-2 lg:hidden">
                <a
                  routerLink="/console/present"
                  queryParamsHandling="preserve"
                  class="btn btn-quiet !min-h-11 !px-3 no-underline"
                  ><svg lucideMonitorPlay [size]="18" aria-hidden="true"></svg
                  ><span class="hidden sm:inline">Present</span
                  ><span class="sr-only sm:hidden">Present</span></a
                >
                <button
                  type="button"
                  class="btn btn-quiet !min-h-11 !px-3"
                  [attr.aria-expanded]="operatorOpen()"
                  aria-controls="operator-panel"
                  (click)="toggleOperator()"
                >
                  <svg lucidePanelRightOpen [size]="18" aria-hidden="true"></svg>
                  <span class="sr-only">Operator detail</span>
                </button>
                @if (store.isDemo()) {
                  <button type="button" class="btn btn-quiet !min-h-11 !px-3" (click)="exitDemo()">
                    <svg lucideX [size]="18" aria-hidden="true"></svg>
                    <span class="sr-only">Exit demo</span>
                  </button>
                }
              </div>
            }
          </div>

          @if (connected()) {
            <div class="relative z-10 hidden px-6 pt-7 lg:block">
              <p class="text-[1.1875rem] leading-tight font-extrabold">{{ lineName() }}</p>
              @if (lineNumber(); as n) {
                <p class="tabular mt-0.5 text-[1rem] font-semibold text-paper/75">{{ n }}</p>
              }
              <p class="mt-2 text-[0.95rem] leading-snug text-muted-on-ink" role="status">
                {{ connectionText() }}
              </p>
            </div>

            <nav class="relative z-10 lg:mt-6" aria-label="Dashboard">
              <ul class="rail-nav">
                @for (item of nav(); track item.path) {
                  <li>
                    <a
                      [routerLink]="item.path"
                      routerLinkActive="nav-active"
                      [routerLinkActiveOptions]="{ exact: item.exact }"
                      queryParamsHandling="preserve"
                      class="nav-item"
                    >
                      @switch (item.icon) {
                        @case ('overview') {
                          <svg lucideLayoutDashboard [size]="20" aria-hidden="true"></svg>
                        }
                        @case ('calls') {
                          <svg lucidePhone [size]="20" aria-hidden="true"></svg>
                        }
                        @case ('callers') {
                          <svg lucideContactRound [size]="20" aria-hidden="true"></svg>
                        }
                        @case ('community') {
                          <svg lucideNetwork [size]="20" aria-hidden="true"></svg>
                        }
                        @default {
                          <svg lucideSettings2 [size]="20" aria-hidden="true"></svg>
                        }
                      }
                      <span class="nav-label">{{ item.label }}</span>
                      @if (item.count !== undefined) {
                        <span class="nav-count tabular"
                          >{{ item.count }}<span class="sr-only"> {{ item.countLabel }}</span></span
                        >
                      }
                    </a>
                  </li>
                }
              </ul>
            </nav>

            <div class="relative z-10 mt-auto hidden px-3 pt-6 pb-3 lg:block">
              <a
                routerLink="/console/present"
                queryParamsHandling="preserve"
                class="btn btn-primary !min-h-11 w-full no-underline"
              >
                <svg lucideMonitorPlay [size]="18" aria-hidden="true"></svg> Present
              </a>
              <button
                type="button"
                class="nav-item mt-1 w-full !min-h-10 !text-[1rem]"
                [attr.aria-expanded]="operatorOpen()"
                aria-controls="operator-panel"
                (click)="toggleOperator()"
              >
                <svg lucidePanelRightOpen [size]="18" aria-hidden="true"></svg> Operator detail
              </button>
              @if (store.isDemo()) {
                <button
                  type="button"
                  class="nav-item w-full !min-h-10 !text-[1rem]"
                  (click)="exitDemo()"
                >
                  <svg lucideX [size]="18" aria-hidden="true"></svg> Exit demo
                </button>
              }
            </div>

            <div class="rail-art">
              <picture aria-hidden="true">
                <source
                  type="image/avif"
                  srcset="media/console-sidebar-512.avif 512w, media/console-sidebar-752.avif 752w"
                  sizes="264px"
                />
                <img
                  src="media/console-sidebar-512.webp"
                  srcset="media/console-sidebar-512.webp 512w, media/console-sidebar-752.webp 752w"
                  sizes="264px"
                  width="752"
                  height="784"
                  alt=""
                  decoding="async"
                />
              </picture>
              @if (artMoves()) {
                <video
                  class="rail-video"
                  [muted]="true"
                  autoplay
                  loop
                  playsinline
                  disablePictureInPicture
                  preload="auto"
                  poster="media/console-sidebar-loop-poster.jpg"
                  aria-hidden="true"
                  tabindex="-1"
                >
                  <source src="media/console-sidebar-loop.webm" type="video/webm" />
                  <source src="media/console-sidebar-loop.mp4" type="video/mp4" />
                </video>
              }
              @if (!motion.reduced()) {
                <button
                  type="button"
                  class="rail-pause"
                  [attr.aria-label]="
                    artPaused() ? 'Play the Ringo animation' : 'Pause the Ringo animation'
                  "
                  [attr.aria-pressed]="artPaused()"
                  (click)="toggleArt()"
                >
                  @if (artPaused()) {
                    <svg lucidePlay [size]="15" aria-hidden="true"></svg>
                  } @else {
                    <svg lucidePause [size]="15" aria-hidden="true"></svg>
                  }
                </button>
              }
            </div>
          }
        </aside>

        <main id="console-main" tabindex="-1" class="min-w-0 outline-none">
          <div class="mx-auto max-w-[96rem] px-4 pt-6 pb-16 sm:px-6 sm:pt-8 lg:px-12 lg:pt-11">
            @if (connected()) {
              @if (store.connection() !== 'live') {
                <p
                  class="mb-4 text-[1rem] font-semibold lg:hidden"
                  [class]="store.connection() === 'paused' ? 'text-amber-ink' : 'text-muted'"
                  role="status"
                >
                  {{ connectionText() }}
                </p>
              }
              <router-outlet />
            } @else {
              <app-connect-panel (demo)="enterDemo()" />
            }
          </div>
        </main>
      </div>

      @if (operatorOpen() && connected()) {
        <app-operator-panel id="operator-panel" (closed)="closeOperator()" />
      }
    }
  `,
})
export class ConsoleShell {
  readonly store = inject(ConsoleStore);
  readonly access = inject(Access);
  private readonly directory = inject(CallerDirectory);
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  readonly connected = computed(() => this.store.source() !== 'none' && !!this.store.state());
  readonly nav = computed(() => {
    const calls = this.store.calls().length;
    const flagged = this.directory
      .callers()
      .filter((c) => c.standing === 'reported' || c.standing === 'warned').length;
    const items: {
      path: string;
      label: string;
      exact: boolean;
      icon: NavIcon;
      count?: number;
      countLabel?: string;
      gap?: boolean;
    }[] = [
      { path: '/console', label: 'Overview', exact: true, icon: 'overview' },
      {
        path: '/console/calls',
        label: 'Calls',
        exact: false,
        icon: 'calls',
        count: calls,
        countLabel: 'calls',
      },
      {
        path: '/console/callers',
        label: 'Callers',
        exact: false,
        icon: 'callers',
        count: flagged || undefined,
        countLabel: 'reported',
      },
      { path: '/console/community', label: 'Community', exact: true, icon: 'community', gap: true },
      { path: '/console/settings', label: 'Settings', exact: true, icon: 'settings', gap: true },
    ];
    return items;
  });
  readonly lineName = computed(() => {
    const state = this.store.state();
    const id = state?.profile?.household_id ?? state?.selected_household;
    const name = state?.profile?.display_name ?? household(id)?.name ?? 'Your';
    return `${name}’s line`;
  });
  readonly lineNumber = computed(() => {
    const raw = this.store.state()?.phone_display;
    return isE164(raw) ? formatPhone(raw) : null;
  });
  readonly connectionText = computed(() => {
    switch (this.store.connection()) {
      case 'demo':
        return 'Demo data. No phone line or model is connected.';
      case 'paused':
        return 'Reconnecting. The last snapshot stays on screen.';
      case 'replay':
        return 'Replay mode: authored scenarios, no real calls.';
      default: {
        const at = this.store.lastReceiptAt();
        const view = this.access.readOnly ? ' View only.' : '';
        return at
          ? `Live, updated ${formatTime(at, this.store.state()?.profile?.timezone)}.${view}`
          : `Live, waiting for the first update.${view}`;
      }
    }
  });
  readonly operatorOpen = signal(false);
  readonly motion = inject(Motion);
  /** The rail animation can be paused; the choice is remembered on this browser. */
  readonly artPaused = signal(readArtPaused());
  readonly artMoves = computed(() => !this.motion.reduced() && !this.artPaused());
  private readonly url = signal(this.router.url);
  /** The presentation view is chrome-less: it draws its own bug, banners and controls. */
  readonly presenting = computed(
    () => this.connected() && this.url().split('?')[0].startsWith('/console/present'),
  );
  private returnFocus?: HTMLElement;

  constructor() {
    this.router.events.pipe(takeUntilDestroyed()).subscribe((event) => {
      if (event instanceof NavigationEnd) this.url.set(event.urlAfterRedirects);
    });
    const query = this.route.snapshot.queryParamMap;
    if (query.get('view') === 'demo' && this.store.source() === 'none') this.store.enterDemo();
    // The server trusts its own computer: no login, connect straight away.
    else if (this.store.source() === 'none') void this.store.connectLocal();
    effect(() => {
      if (!this.connected()) this.operatorOpen.set(false);
    });
  }

  @HostListener('document:keydown.escape')
  onEscape() {
    if (this.operatorOpen()) this.closeOperator();
  }

  enterDemo() {
    this.store.enterDemo();
    void this.router.navigate(['/console'], { queryParams: { view: 'demo' } });
  }

  exitDemo() {
    this.store.lock();
    void this.store.retryLocal();
    void this.router.navigate(['/console']);
  }

  toggleArt() {
    const paused = !this.artPaused();
    this.artPaused.set(paused);
    try {
      localStorage.setItem(ART_KEY, paused ? '1' : '0');
    } catch {
      /* storage unavailable: the choice lasts for this page only */
    }
  }

  toggleOperator() {
    if (this.operatorOpen()) this.closeOperator();
    else {
      this.returnFocus = document.activeElement as HTMLElement;
      this.operatorOpen.set(true);
    }
  }

  closeOperator() {
    this.operatorOpen.set(false);
    queueMicrotask(() => this.returnFocus?.focus());
  }
}
