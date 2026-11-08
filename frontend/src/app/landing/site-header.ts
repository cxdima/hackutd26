import { Component, ElementRef, HostListener, signal, viewChild } from '@angular/core';
import { RouterLink } from '@angular/router';
import { LucideArrowRight, LucideMenu, LucideX } from '@lucide/angular';
import { Wordmark } from '../shared/wordmark';

@Component({
  selector: 'app-site-header',
  imports: [RouterLink, Wordmark, LucideMenu, LucideX, LucideArrowRight],
  template: `
    <header
      class="sticky top-0 z-40 border-b border-transparent bg-paper transition-[border-color] duration-300"
      [class.!border-rule]="scrolled()"
    >
      <nav
        class="mx-auto flex h-[4.5rem] max-w-[86rem] items-center justify-between gap-6 px-4 sm:px-6 lg:px-10"
        aria-label="Main"
      >
        <a routerLink="/" fragment="top" class="rounded-lg" aria-label="FirstRing home"
          ><app-wordmark
        /></a>
        <ul class="hidden items-center gap-1 whitespace-nowrap lg:flex">
          @for (link of links; track link.href) {
            <li>
              <a
                [href]="'#' + link.href"
                (click)="go($event, link.href)"
                class="rounded-full px-4 py-2.5 text-[1.0625rem] font-semibold text-ink-3 no-underline transition-colors hover:bg-paper-3 hover:text-ink"
                >{{ link.label }}</a
              >
            </li>
          }
          <li class="ml-3">
            <a routerLink="/console" class="btn btn-ink !min-h-12 !px-5"
              >Open dashboard <svg lucideArrowRight [size]="18" aria-hidden="true"></svg
            ></a>
          </li>
        </ul>
        <button
          #menuButton
          type="button"
          class="btn btn-quiet lg:hidden"
          [attr.aria-expanded]="open()"
          aria-controls="mobile-menu"
          (click)="toggle()"
        >
          @if (open()) {
            <svg lucideX [size]="20" aria-hidden="true"></svg> Close
          } @else {
            <svg lucideMenu [size]="20" aria-hidden="true"></svg> Menu
          }
        </button>
      </nav>
      @if (open()) {
        <div id="mobile-menu" class="border-t border-rule bg-paper px-4 pb-6 pt-2 lg:hidden">
          <ul class="flex flex-col">
            @for (link of links; track link.href) {
              <li>
                <a
                  [href]="'#' + link.href"
                  (click)="go($event, link.href)"
                  class="flex min-h-14 items-center border-b border-paper-3 text-[1.25rem] font-bold text-ink no-underline"
                  >{{ link.label }}</a
                >
              </li>
            }
          </ul>
          <a
            routerLink="/console"
            class="btn btn-ink mt-5 w-full !min-h-14 text-[1.15rem]"
            (click)="open.set(false)"
            >Open dashboard</a
          >
        </div>
      }
    </header>
  `,
})
export class SiteHeader {
  readonly links = [
    { href: 'how', label: 'How it works' },
    { href: 'community', label: 'Community' },
    { href: 'privacy', label: 'Privacy' },
  ];
  readonly open = signal(false);
  readonly scrolled = signal(false);
  private readonly menuButton = viewChild<ElementRef<HTMLButtonElement>>('menuButton');

  @HostListener('window:scroll')
  onScroll() {
    this.scrolled.set(window.scrollY > 8);
  }

  @HostListener('document:keydown.escape')
  onEscape() {
    if (!this.open()) return;
    this.open.set(false);
    this.menuButton()?.nativeElement.focus();
  }

  toggle() {
    this.open.update((value) => !value);
  }

  /** In-page links keep normal scrolling and move focus to the section heading. */
  go(event: Event, id: string) {
    const target = document.getElementById(id);
    if (!target) return;
    event.preventDefault();
    this.open.set(false);
    const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
    target.scrollIntoView({ behavior: reduced ? 'auto' : 'smooth', block: 'start' });
    history.replaceState(null, '', `#${id}`);
    const heading = target.querySelector<HTMLElement>('h2');
    if (heading) {
      heading.setAttribute('tabindex', '-1');
      heading.focus({ preventScroll: true });
    }
  }
}
