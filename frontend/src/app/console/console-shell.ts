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
  LucidePhone,
  LucideSettings2,
  LucideX,
} from '@lucide/angular';
import { CallerDirectory } from '../core/caller-directory';
import { ConsoleStore } from '../core/console-store';
import { formatPhone, formatTime, isE164 } from '../core/format';
import { household } from '../core/household';
import { Wordmark } from '../shared/wordmark';
import { ConnectPanel } from './connect-panel';
import { OperatorPanel } from './operator-panel';

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
  ],
  template: `
    @if (presenting()) {
      <a class="skip-link" href="#present-main">Skip to the presentation</a>
      <router-outlet />
    } @else {
      <a class="skip-link" href="#console-main">Skip to dashboard content</a>
      <div class="min-h-dvh lg:grid lg:grid-cols-[16rem_minmax(0,1fr)]">
        <aside
          class="on-ink sticky top-0 z-40 bg-ink text-paper lg:flex lg:h-dvh lg:flex-col"
          aria-label="Dashboard"
        >
          <div class="flex min-h-16 items-center gap-3 px-4 lg:min-h-0 lg:px-5 lg:pt-6">
            <a routerLink="/" class="shrink-0 rounded-lg" aria-label="FirstRing home"
              ><app-wordmark [inverse]="true"
            /></a>
            @if (connected()) {
              <div class="ml-auto flex items-center gap-2 lg:hidden">
                <a
                  routerLink="/console/present"
                  queryParamsHandling="preserve"
                  class="btn btn-quiet !min-h-10 !px-3 no-underline"
                  ><svg lucideMonitorPlay [size]="17" aria-hidden="true"></svg
                  ><span class="hidden sm:inline">Present</span></a
                >
                <button
                  type="button"
                  class="btn btn-quiet !min-h-10 !px-3"
                  [attr.aria-expanded]="operatorOpen()"
                  aria-controls="operator-panel"
                  (click)="toggleOperator()"
                >
                  <svg lucidePanelRightOpen [size]="17" aria-hidden="true"></svg>
                  <span class="sr-only">Operator detail</span>
                </button>
                @if (store.isDemo()) {
                  <button type="button" class="btn btn-quiet !min-h-10 !px-3" (click)="exitDemo()">
                    <svg lucideX [size]="17" aria-hidden="true"></svg>
                    <span class="sr-only">Exit demo</span>
                  </button>
                }
              </div>
            }
          </div>

          @if (connected()) {
            <div class="hidden px-5 pt-6 lg:block">
              <p class="text-[0.95rem] font-semibold text-muted-on-ink">FirstRing answers for</p>
              <p class="mt-0.5 text-[1.125rem] font-extrabold">{{ lineName() }}</p>
              @if (lineNumber(); as n) {
                <p class="tabular text-[1rem] font-semibold text-paper/80">{{ n }}</p>
              }
            </div>

            <nav class="border-t border-white/10 lg:mt-6 lg:border-0" aria-label="Sections">
              <ul
                class="scrollbar-none flex overflow-x-auto px-2 lg:block lg:space-y-1 lg:overflow-visible lg:px-3"
              >
                @for (item of nav(); track item.path) {
                  <li class="shrink-0">
                    <a
                      [routerLink]="item.path"
                      routerLinkActive="nav-active"
                      [routerLinkActiveOptions]="{ exact: item.exact }"
                      queryParamsHandling="preserve"
                      class="nav-item"
                    >
                      @switch (item.icon) {
                        @case ('overview') {
                          <svg lucideLayoutDashboard [size]="19" aria-hidden="true"></svg>
                        }
                        @case ('calls') {
                          <svg lucidePhone [size]="19" aria-hidden="true"></svg>
                        }
                        @case ('callers') {
                          <svg lucideContactRound [size]="19" aria-hidden="true"></svg>
                        }
                        @case ('community') {
                          <svg lucideNetwork [size]="19" aria-hidden="true"></svg>
                        }
                        @default {
                          <svg lucideSettings2 [size]="19" aria-hidden="true"></svg>
                        }
                      }
                      <span>{{ item.label }}</span>
                      @if (item.count !== undefined) {
                        <span class="tabular ml-auto pl-3 text-[0.95rem] font-semibold opacity-70"
                          >{{ item.count }}<span class="sr-only"> {{ item.countLabel }}</span></span
                        >
                      }
                    </a>
                  </li>
                }
              </ul>
            </nav>

            <div class="mt-auto hidden space-y-2 px-3 pb-5 lg:block">
              <a
                routerLink="/console/present"
                queryParamsHandling="preserve"
                class="btn btn-primary w-full no-underline"
              >
                <svg lucideMonitorPlay [size]="17" aria-hidden="true"></svg> Present
              </a>
              <button
                type="button"
                class="btn btn-quiet w-full"
                [attr.aria-expanded]="operatorOpen()"
                aria-controls="operator-panel"
                (click)="toggleOperator()"
              >
                <svg lucidePanelRightOpen [size]="17" aria-hidden="true"></svg> Operator detail
              </button>
              @if (store.isDemo()) {
                <button type="button" class="btn btn-quiet w-full" (click)="exitDemo()">
                  <svg lucideX [size]="17" aria-hidden="true"></svg> Exit demo
                </button>
              }
              <p class="px-2 pt-2 text-[0.95rem] text-muted-on-ink" role="status">
                {{ connectionText() }}
              </p>
            </div>
          }
        </aside>

        <main id="console-main" tabindex="-1" class="min-w-0 outline-none">
          <div class="mx-auto max-w-[96rem] px-4 py-6 sm:px-6 sm:py-8 lg:px-10 lg:py-10">
            @if (connected()) {
              @if (store.connection() === 'paused') {
                <p class="mb-4 text-[1rem] font-semibold text-amber-ink lg:hidden" role="status">
                  Reconnecting…
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
      { path: '/console/community', label: 'Community', exact: true, icon: 'community' },
      { path: '/console/settings', label: 'Settings', exact: true, icon: 'settings' },
    ];
    return items;
  });
  readonly lineName = computed(() => {
    const state = this.store.state();
    const id = state?.profile?.household_id ?? state?.selected_household;
    const name = state?.profile?.display_name ?? household(id)?.name ?? 'your household';
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
        return 'Reconnecting… the last snapshot stays on screen.';
      case 'replay':
        return 'Replay mode: authored scenarios, no real calls.';
      default: {
        const at = this.store.lastReceiptAt();
        return at
          ? `Live. Last update ${formatTime(at, this.store.state()?.profile?.timezone, true)}.`
          : 'Live. Waiting for the first update.';
      }
    }
  });
  readonly operatorOpen = signal(false);
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
