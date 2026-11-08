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
import { LucideMonitorPlay, LucidePanelRightOpen, LucideX } from '@lucide/angular';
import { ConsoleStore } from '../core/console-store';
import { formatTime } from '../core/format';
import { Wordmark } from '../shared/wordmark';
import { ConnectPanel } from './connect-panel';
import { OperatorPanel } from './operator-panel';

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
  ],
  template: `
    @if (presenting()) {
      <a class="skip-link" href="#present-main">Skip to the presentation</a>
      <router-outlet />
    } @else {
      <a class="skip-link" href="#console-main">Skip to dashboard content</a>
      <header class="on-ink sticky top-0 z-40 bg-ink text-paper">
        <div
          class="mx-auto flex min-h-[4.25rem] max-w-[90rem] items-center gap-4 px-4 sm:px-6 lg:px-8"
        >
          <a routerLink="/" class="shrink-0 rounded-lg" aria-label="FirstRing home"
            ><app-wordmark [inverse]="true"
          /></a>
          @if (connected()) {
            <nav class="ml-4 hidden lg:block" aria-label="Dashboard">
              <ul class="flex items-center gap-1">
                @for (item of nav; track item.path) {
                  <li>
                    <a
                      [routerLink]="item.path"
                      routerLinkActive="!bg-paper !text-ink"
                      [routerLinkActiveOptions]="{ exact: item.exact }"
                      queryParamsHandling="preserve"
                      class="block rounded-full px-4 py-2.5 text-[1.0625rem] font-bold text-paper/85 no-underline hover:bg-white/10 hover:text-paper"
                      >{{ item.label }}</a
                    >
                  </li>
                }
              </ul>
            </nav>
          }
          <div class="ml-auto flex items-center gap-2 sm:gap-3">
            @if (store.connection() === 'paused') {
              <span class="text-[0.95rem] font-semibold text-paper/70" role="status"
                >Reconnecting…</span
              >
            }
            @if (connected()) {
              <a
                routerLink="/console/present"
                queryParamsHandling="preserve"
                class="btn btn-quiet hidden no-underline sm:inline-flex"
              >
                <svg lucideMonitorPlay [size]="17" aria-hidden="true"></svg> Present
              </a>
              <button
                type="button"
                class="btn btn-quiet hidden whitespace-nowrap sm:inline-flex lg:max-xl:!px-3.5"
                [attr.aria-expanded]="operatorOpen()"
                aria-controls="operator-panel"
                (click)="toggleOperator()"
              >
                <svg lucidePanelRightOpen [size]="17" aria-hidden="true"></svg>
                <span class="lg:max-xl:sr-only">Operator detail</span>
              </button>
              @if (store.isDemo()) {
                <button
                  type="button"
                  class="btn btn-quiet whitespace-nowrap !px-3 sm:!px-4"
                  (click)="exitDemo()"
                >
                  <svg lucideX [size]="17" aria-hidden="true"></svg
                  ><span class="hidden sm:inline">Exit demo</span
                  ><span class="sm:hidden">Exit</span>
                </button>
              }
            }
          </div>
        </div>
        @if (connected()) {
          <nav class="border-t border-white/10 lg:hidden" aria-label="Dashboard">
            <ul class="mx-auto grid max-w-[90rem] grid-cols-4">
              @for (item of nav; track item.path) {
                <li>
                  <a
                    [routerLink]="item.path"
                    routerLinkActive="!text-ink !bg-lime"
                    [routerLinkActiveOptions]="{ exact: item.exact }"
                    queryParamsHandling="preserve"
                    class="flex min-h-13 items-center justify-center px-1 text-[1rem] font-bold text-paper/85 no-underline"
                    >{{ item.label }}</a
                  >
                </li>
              }
            </ul>
            <div class="grid grid-cols-2 border-t border-white/10 sm:hidden">
              <a
                routerLink="/console/present"
                queryParamsHandling="preserve"
                class="flex min-h-12 items-center justify-center gap-2 text-[1rem] font-bold text-paper no-underline"
              >
                <svg lucideMonitorPlay [size]="17" aria-hidden="true"></svg> Present
              </a>
              <button
                type="button"
                class="flex min-h-12 items-center justify-center gap-2 border-l border-white/10 text-[1rem] font-bold"
                [attr.aria-expanded]="operatorOpen()"
                aria-controls="operator-panel"
                (click)="toggleOperator()"
              >
                <svg lucidePanelRightOpen [size]="17" aria-hidden="true"></svg> Operator detail
              </button>
            </div>
          </nav>
        }
      </header>

      <main
        id="console-main"
        tabindex="-1"
        class="mx-auto min-h-[70vh] max-w-[90rem] px-4 py-8 outline-none sm:px-6 sm:py-10 lg:px-8"
      >
        @if (connected()) {
          <router-outlet />
        } @else {
          <app-connect-panel (demo)="enterDemo()" />
        }
      </main>

      @if (operatorOpen() && connected()) {
        <app-operator-panel id="operator-panel" (closed)="closeOperator()" />
      }
    }
  `,
})
export class ConsoleShell {
  readonly store = inject(ConsoleStore);
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  readonly nav = [
    { path: '/console', label: 'Overview', exact: true },
    { path: '/console/calls', label: 'Calls', exact: false },
    { path: '/console/community', label: 'Community', exact: true },
    { path: '/console/settings', label: 'Settings', exact: true },
  ];
  readonly connected = computed(() => this.store.source() !== 'none' && !!this.store.state());
  readonly operatorOpen = signal(false);
  private readonly url = signal(this.router.url);
  /** The presentation view is chrome-less: it draws its own bug, banners and controls. */
  readonly presenting = computed(
    () => this.connected() && this.url().split('?')[0].startsWith('/console/present'),
  );
  readonly lastReceipt = computed(() => {
    const at = this.store.lastReceiptAt();
    return at ? formatTime(at, this.store.state()?.profile?.timezone, true) : '';
  });
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

  lock() {
    this.store.lock();
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
