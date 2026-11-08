import {
  Component,
  computed,
  contentChild,
  Directive,
  DestroyRef,
  ElementRef,
  forwardRef,
  inject,
  input,
  model,
  signal,
  TemplateRef,
  viewChild,
} from '@angular/core';
import { NgTemplateOutlet } from '@angular/common';
import { type ControlValueAccessor, NG_VALUE_ACCESSOR } from '@angular/forms';
import { LucideCheck, LucideChevronDown } from '@lucide/angular';

export interface SelectOption {
  value: string;
  /** Shown on the button when selected, and as the first line in the list. */
  label: string;
  /** Optional second line in the list. */
  detail?: string;
  /** Optional trailing figure in the list, e.g. a count. */
  meta?: string;
}

export interface SelectOptionContext {
  $implicit: SelectOption;
  selected: boolean;
  active: boolean;
}

/** Custom content for each option: `<ng-template appSelectOption let-option>…</ng-template>`. */
@Directive({ selector: 'ng-template[appSelectOption]' })
export class SelectOptionTemplate {
  readonly template = inject<TemplateRef<SelectOptionContext>>(TemplateRef);
  static ngTemplateContextGuard(
    _dir: SelectOptionTemplate,
    ctx: unknown,
  ): ctx is SelectOptionContext {
    return ctx !== null && typeof ctx === 'object';
  }
}

let nextId = 0;
const PAGE = 8;

/**
 * Select-only combobox (WAI-ARIA APG pattern). Focus stays on the button; the
 * list opens in the top layer so scroll containers and sticky headers never
 * clip it. Works with ngModel or with [value] / (valueChange).
 */
@Component({
  selector: 'app-select-menu',
  imports: [NgTemplateOutlet, LucideChevronDown, LucideCheck],
  providers: [
    { provide: NG_VALUE_ACCESSOR, useExisting: forwardRef(() => SelectMenu), multi: true },
  ],
  host: {
    class: 'block min-w-0',
    '(mousedown)': 'onHostMousedown($event)',
    '(click)': 'onHostClick($event)',
    '(pointermove)': 'onHostPointermove($event)',
  },
  template: `
    <button
      #trigger
      type="button"
      role="combobox"
      aria-haspopup="listbox"
      [id]="triggerId()"
      [attr.aria-label]="ariaLabel() || null"
      [attr.aria-labelledby]="labelledBy() || null"
      [attr.aria-describedby]="describedBy() || null"
      [attr.aria-expanded]="open()"
      [attr.aria-controls]="listId"
      [attr.aria-activedescendant]="open() && active() >= 0 ? optionId(active()) : null"
      [disabled]="isDisabled()"
      [class]="triggerClass()"
      (click)="toggle()"
      (keydown)="onKeydown($event)"
      (keyup)="onKeyup($event)"
      (blur)="close(false)"
    >
      <span class="min-w-0 flex-1 truncate text-left">{{ selectedLabel() }}</span>
      <svg
        lucideChevronDown
        [size]="18"
        class="shrink-0 transition-transform duration-150"
        [class.rotate-180]="open()"
        aria-hidden="true"
      ></svg>
    </button>
    <ul #list popover="manual" role="listbox" [id]="listId" [class]="listClass()">
      @for (option of options(); track option.value; let i = $index) {
        @let selected = option.value === value();
        <li
          role="option"
          [id]="optionId(i)"
          [attr.data-index]="i"
          [attr.aria-selected]="selected"
          class="flex min-h-11 cursor-pointer items-center gap-3 rounded-[0.625rem] px-3 py-2"
          [class]="i === active() ? activeClass() : ''"
        >
          @if (optionTemplate(); as custom) {
            <ng-container
              [ngTemplateOutlet]="custom.template"
              [ngTemplateOutletContext]="{ $implicit: option, selected, active: i === active() }"
            />
          } @else {
            <span class="min-w-0 flex-1 leading-snug">
              <span class="block" [class.font-bold]="selected">{{ option.label }}</span>
              @if (option.detail) {
                <span class="mt-0.5 block text-[0.9375rem]" [class]="mutedClass()">{{
                  option.detail
                }}</span>
              }
            </span>
            @if (option.meta) {
              <span
                class="tabular min-w-[2ch] pl-2 text-right text-[0.9375rem] font-semibold"
                [class]="mutedClass()"
                >{{ option.meta }}</span
              >
            }
          }
          <span class="inline-flex w-[18px] shrink-0" [class]="checkClass()">
            @if (selected) {
              <svg lucideCheck [size]="18" aria-hidden="true"></svg>
            }
          </span>
        </li>
      }
    </ul>
  `,
})
export class SelectMenu implements ControlValueAccessor {
  readonly options = input.required<SelectOption[]>();
  readonly value = model<string | null>(null);
  /** 'paper' for forms on paper, 'ink' for the dark presentation header. */
  readonly tone = input<'paper' | 'ink'>('paper');
  readonly inputId = input<string>();
  readonly ariaLabel = input<string>();
  readonly labelledBy = input<string>();
  readonly describedBy = input<string>();
  readonly placeholder = input('Choose…');
  readonly disabled = input(false);
  /** Extra classes for the button, e.g. a fixed width. */
  readonly buttonClass = input('');

  readonly optionTemplate = contentChild(SelectOptionTemplate);
  private readonly trigger = viewChild.required<ElementRef<HTMLButtonElement>>('trigger');
  private readonly list = viewChild.required<ElementRef<HTMLUListElement>>('list');
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

  private readonly uid = `select-menu-${++nextId}`;
  readonly listId = `${this.uid}-list`;
  readonly triggerId = computed(() => this.inputId() || `${this.uid}-button`);
  readonly open = signal(false);
  readonly active = signal(-1);
  private readonly formDisabled = signal(false);
  readonly isDisabled = computed(() => this.disabled() || this.formDisabled());

  readonly selectedIndex = computed(() =>
    this.options().findIndex((o) => o.value === this.value()),
  );
  readonly selectedLabel = computed(
    () => this.options()[this.selectedIndex()]?.label ?? this.placeholder(),
  );

  readonly triggerClass = computed(() => {
    const base =
      'flex w-full cursor-pointer items-center gap-2 text-left disabled:cursor-not-allowed';
    const tone =
      this.tone() === 'ink'
        ? 'min-h-11 rounded-full border-2 bg-ink px-4 text-[1rem] font-bold text-paper hover:border-white/45 ' +
          (this.open() ? 'border-white/60' : 'border-white/20')
        : 'field font-semibold ' + (this.open() ? '!border-ink' : '');
    return `${base} ${tone} ${this.buttonClass()}`;
  });
  readonly listClass = computed(() =>
    this.tone() === 'ink'
      ? 'select-list fixed inset-auto m-0 overflow-y-auto overscroll-contain rounded-[0.875rem] border border-white/15 bg-ink-2 p-1.5 text-[1rem] text-paper shadow-[0_18px_40px_-16px_rgb(0_0_0/0.7)]'
      : 'select-list fixed inset-auto m-0 overflow-y-auto overscroll-contain rounded-[0.875rem] border border-rule bg-white p-1.5 text-[1.0625rem] text-ink shadow-[var(--shadow-lift)]',
  );
  readonly activeClass = computed(() => (this.tone() === 'ink' ? 'bg-white/12' : 'bg-paper-3'));
  readonly mutedClass = computed(() =>
    this.tone() === 'ink' ? 'text-muted-on-ink' : 'text-muted',
  );
  readonly checkClass = computed(() => (this.tone() === 'ink' ? 'text-lime' : 'text-lime-ink'));

  private onChange: (value: string) => void = () => undefined;
  private onTouched: () => void = () => undefined;
  private typed = '';
  private typedAt = 0;
  private readonly reposition = (event?: Event) => {
    if (event?.target === this.list().nativeElement) return;
    this.place();
  };
  private readonly outside = (event: PointerEvent) => {
    if (!this.host.nativeElement.contains(event.target as Node)) this.close(false);
  };

  constructor() {
    inject(DestroyRef).onDestroy(() => this.detach());
  }

  optionId(index: number) {
    return `${this.uid}-option-${index}`;
  }

  // ControlValueAccessor
  writeValue(value: unknown): void {
    this.value.set(value == null ? null : String(value));
  }
  registerOnChange(fn: (value: string) => void): void {
    this.onChange = fn;
  }
  registerOnTouched(fn: () => void): void {
    this.onTouched = fn;
  }
  setDisabledState(disabled: boolean): void {
    this.formDisabled.set(disabled);
  }

  toggle() {
    if (this.open()) this.close(false);
    else this.show();
  }

  show(index = this.selectedIndex()) {
    if (this.isDisabled() || !this.options().length) return;
    this.active.set(index >= 0 ? index : 0);
    if (!this.open()) {
      this.open.set(true);
      const list = this.list().nativeElement;
      this.name(list, true);
      list.showPopover();
      this.place();
      window.addEventListener('scroll', this.reposition, true);
      window.addEventListener('resize', this.reposition);
      document.addEventListener('pointerdown', this.outside, true);
    }
    this.reveal();
  }

  close(refocus: boolean) {
    if (!this.open()) return;
    this.open.set(false);
    this.detach();
    this.onTouched();
    if (refocus) this.trigger().nativeElement.focus();
  }

  pick(index: number) {
    const option = this.options()[index];
    if (!option) return;
    if (option.value !== this.value()) {
      this.value.set(option.value);
      this.onChange(option.value);
    }
    this.close(true);
  }

  onKeydown(event: KeyboardEvent) {
    const count = this.options().length;
    if (!count || this.isDisabled()) return;
    const open = this.open();
    const at = this.active();
    let handled = true;
    switch (event.key) {
      case 'ArrowDown':
        if (!open) this.show();
        else if (event.altKey) this.pick(at);
        else this.move(Math.min(count - 1, at + 1));
        break;
      case 'ArrowUp':
        if (!open) this.show();
        else if (event.altKey) this.pick(at);
        else this.move(Math.max(0, at - 1));
        break;
      case 'Home':
        if (open) this.move(0);
        else this.show(0);
        break;
      case 'End':
        if (open) this.move(count - 1);
        else this.show(count - 1);
        break;
      case 'PageDown':
        if (open) this.move(Math.min(count - 1, at + PAGE));
        else handled = false;
        break;
      case 'PageUp':
        if (open) this.move(Math.max(0, at - PAGE));
        else handled = false;
        break;
      case 'Enter':
      case ' ':
        if (event.key === ' ' && open && this.typed && Date.now() - this.typedAt < 600) {
          this.typeahead(' ');
        } else if (open) this.pick(at);
        else this.show();
        break;
      case 'Escape':
        if (open) {
          this.close(true);
          event.stopPropagation();
        } else handled = false;
        break;
      case 'Tab':
        this.close(false);
        handled = false;
        break;
      default:
        if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
          if (!open) this.show();
          this.typeahead(event.key);
        } else handled = false;
    }
    if (handled) event.preventDefault();
  }

  /** Space activates a native button on keyup; the keydown handler already acted. */
  onKeyup(event: KeyboardEvent) {
    if (event.key === ' ') event.preventDefault();
  }

  onHostMousedown(event: MouseEvent) {
    // Keep focus on the button while the pointer works inside the list.
    if (this.list().nativeElement.contains(event.target as Node)) event.preventDefault();
  }

  onHostClick(event: MouseEvent) {
    const index = this.indexFrom(event.target);
    if (index !== undefined) this.pick(index);
  }

  onHostPointermove(event: PointerEvent) {
    const index = this.indexFrom(event.target);
    if (index !== undefined && index !== this.active()) this.active.set(index);
  }

  private indexFrom(target: EventTarget | null): number | undefined {
    if (!this.open() || !(target instanceof Element)) return undefined;
    const item = target.closest<HTMLElement>('[role="option"]');
    if (!item || !this.list().nativeElement.contains(item)) return undefined;
    const index = Number(item.dataset['index']);
    return Number.isInteger(index) ? index : undefined;
  }

  private move(index: number) {
    this.active.set(index);
    this.reveal();
  }

  private reveal() {
    const index = this.active();
    queueMicrotask(() =>
      document.getElementById(this.optionId(index))?.scrollIntoView({ block: 'nearest' }),
    );
  }

  private typeahead(key: string) {
    const now = Date.now();
    this.typed = now - this.typedAt > 600 ? key : this.typed + key;
    this.typedAt = now;
    const query = this.typed.toLowerCase();
    const options = this.options();
    const start = this.typed.length === 1 ? this.active() + 1 : this.active();
    for (let step = 0; step < options.length; step++) {
      const index = (start + step) % options.length;
      if (options[index].label.toLowerCase().startsWith(query)) {
        this.move(index);
        return;
      }
    }
  }

  /** Positions the top-layer list under the button, or above it when there is more room. */
  private place() {
    if (!this.open()) return;
    const button = this.trigger().nativeElement.getBoundingClientRect();
    const list = this.list().nativeElement;
    const margin = 8;
    const gap = 6;
    const width = document.documentElement.clientWidth;
    const height = window.innerHeight;
    list.style.minWidth = `${Math.round(Math.min(width - margin * 2, Math.max(button.width, 240)))}px`;
    list.style.maxWidth = `${Math.min(width - margin * 2, Math.max(button.width, 416))}px`;
    list.style.maxHeight = '';
    const natural = list.scrollHeight + 2;
    const below = height - button.bottom - gap - margin;
    const above = button.top - gap - margin;
    const up = natural > below && above > below;
    const room = Math.max(120, up ? above : below);
    list.style.maxHeight = `${Math.min(room, 22 * 16)}px`;
    const left = Math.max(margin, Math.min(button.left, width - margin - list.offsetWidth));
    list.style.left = `${Math.round(left)}px`;
    list.style.top = up
      ? `${Math.round(button.top - gap - list.offsetHeight)}px`
      : `${Math.round(button.bottom + gap)}px`;
  }

  /**
   * The list carries the field's name only while open, set synchronously so a
   * label lookup never finds the list and the button at the same moment.
   */
  private name(list: HTMLElement, open: boolean) {
    const label = this.ariaLabel();
    const labelledBy = this.labelledBy();
    if (open && label) list.setAttribute('aria-label', label);
    else list.removeAttribute('aria-label');
    if (open && labelledBy) list.setAttribute('aria-labelledby', labelledBy);
    else list.removeAttribute('aria-labelledby');
  }

  private detach() {
    window.removeEventListener('scroll', this.reposition, true);
    window.removeEventListener('resize', this.reposition);
    document.removeEventListener('pointerdown', this.outside, true);
    const list = this.list()?.nativeElement;
    if (!list) return;
    this.name(list, false);
    if (list.matches(':popover-open')) list.hidePopover();
  }
}
