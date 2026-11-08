import { Component, computed, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { LucideLoader, LucidePlus, LucideTrash2, LucideUserCheck } from '@lucide/angular';
import { Access, ReadOnlyNote } from '../core/access';
import { ConsoleStore } from '../core/console-store';
import { relativeAge } from '../core/format';
import { household } from '../core/household';
import { normalizeNumber, TrustedDirectory } from '../core/trusted';

/**
 * The trusted list for the line FirstRing answers. Adding or removing writes
 * to the server on the presenting laptop only; the public copy and the demo
 * show the list read-only.
 */
@Component({
  selector: 'app-trusted-callers',
  imports: [FormsModule, ReadOnlyNote, LucideUserCheck, LucideTrash2, LucidePlus, LucideLoader],
  template: `
    @if (trusted.unavailable()) {
      <p class="rounded-xl bg-paper-2 px-4 py-3.5 text-[1.0625rem] text-ink-3">
        This FirstRing server does not offer trusted callers yet.
      </p>
    } @else {
      <div class="flex flex-wrap items-baseline justify-between gap-3">
        <p class="text-[1.0625rem] font-bold">
          {{ lineName() }}’s list
          <span class="font-semibold text-muted"
            >· {{ trusted.items().length }}
            {{ trusted.items().length === 1 ? 'number' : 'numbers' }}</span
          >
        </p>
        @if (trusted.list()?.connects_to_enrolled_phone === false) {
          <p class="text-[1rem] font-semibold text-amber-ink">
            No phone is enrolled, so trusted calls cannot ring through yet.
          </p>
        }
      </div>

      @if (trusted.error()) {
        <p
          class="mt-3 rounded-xl bg-magenta-wash px-4 py-3 text-[1rem] font-semibold text-magenta-ink"
          role="alert"
        >
          {{ trusted.error() }}
        </p>
      }

      <ul class="mt-3 border-t-2 border-ink" aria-label="Trusted callers">
        @for (item of trusted.items(); track item.id) {
          <li class="flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-paper-3 py-3">
            <span
              class="grid size-10 shrink-0 place-items-center rounded-full bg-lime-wash text-lime-ink"
            >
              <svg lucideUserCheck [size]="19" aria-hidden="true"></svg>
            </span>
            <span class="min-w-0 flex-1">
              <span class="block text-[1.0625rem] font-bold">
                {{ item.name }}
                @if (item.relation) {
                  <span class="font-semibold text-ink-3"> · {{ item.relation }}</span>
                }
              </span>
              <span class="tabular block text-[0.95rem] text-muted"
                >Number {{ item.number_label
                }}{{ item.added_at ? ' · added ' + age(item.added_at) : '' }}</span
              >
            </span>
            @if (canEdit()) {
              @if (confirming() === item.id) {
                <span class="flex flex-wrap gap-2">
                  <button
                    type="button"
                    class="btn btn-ink !min-h-11"
                    [disabled]="trusted.busy()"
                    (click)="remove(item.id)"
                  >
                    Remove {{ item.name }}
                  </button>
                  <button type="button" class="btn btn-quiet" (click)="confirming.set(null)">
                    Keep
                  </button>
                </span>
              } @else {
                <button
                  type="button"
                  class="btn btn-quiet"
                  [attr.aria-label]="'Remove ' + item.name + ' from the trusted list'"
                  (click)="confirming.set(item.id)"
                >
                  <svg lucideTrash2 [size]="16" aria-hidden="true"></svg> Remove
                </button>
              }
            }
          </li>
        } @empty {
          <li class="border-b border-paper-3 py-4 text-[1.0625rem] text-ink-3">
            {{
              trusted.loaded()
                ? 'No trusted callers. Every caller is screened first.'
                : 'Loading the trusted list…'
            }}
          </li>
        }
      </ul>

      @if (canEdit()) {
        <form
          class="mt-6 rounded-[var(--radius-card)] bg-paper-2 p-5"
          (ngSubmit)="add()"
          novalidate
        >
          <h3 class="text-[1.125rem] font-extrabold">Add a trusted caller</h3>
          <div class="mt-4 grid gap-4 sm:grid-cols-2 xl:grid-cols-[1fr_1fr_1.2fr]">
            <div>
              <label for="trusted-name" class="field-label">Name</label>
              <input
                id="trusted-name"
                name="name"
                class="field"
                maxlength="40"
                autocomplete="off"
                [ngModel]="name()"
                (ngModelChange)="name.set($event)"
              />
            </div>
            <div>
              <label for="trusted-relation" class="field-label">Relation</label>
              <input
                id="trusted-relation"
                name="relation"
                class="field"
                maxlength="40"
                placeholder="Son, neighbour, doctor’s office"
                autocomplete="off"
                [ngModel]="relation()"
                (ngModelChange)="relation.set($event)"
              />
            </div>
            <div class="sm:col-span-2 xl:col-span-1">
              <label for="trusted-number" class="field-label">Phone number</label>
              <input
                id="trusted-number"
                name="number"
                class="field tabular"
                type="tel"
                inputmode="tel"
                autocomplete="off"
                placeholder="(214) 555-0147"
                [ngModel]="number()"
                (ngModelChange)="number.set($event)"
                aria-describedby="trusted-number-help"
              />
              <p id="trusted-number-help" class="field-help">
                Only the last four digits are shown again.
              </p>
            </div>
          </div>
          @if (formError()) {
            <p class="mt-3 text-[1rem] font-semibold text-magenta-ink" role="alert">
              {{ formError() }}
            </p>
          }
          @if (added()) {
            <p class="mt-3 text-[1rem] font-semibold text-lime-ink" role="status">{{ added() }}</p>
          }
          <button type="submit" class="btn btn-ink mt-4" [disabled]="trusted.busy()">
            @if (trusted.busy()) {
              <svg lucideLoader [size]="17" class="animate-spin" aria-hidden="true"></svg>
            } @else {
              <svg lucidePlus [size]="17" aria-hidden="true"></svg>
            }
            Add to trusted list
          </button>
        </form>
      } @else if (store.isDemo()) {
        <p class="mt-3 text-[1rem] text-muted">Fictional demo list. It cannot be changed here.</p>
      } @else if (access.readOnly) {
        <app-read-only-note class="mt-3 block" />
      }
    }
  `,
})
export class TrustedCallers {
  readonly store = inject(ConsoleStore);
  readonly access = inject(Access);
  readonly trusted = inject(TrustedDirectory);
  readonly name = signal('');
  readonly relation = signal('');
  readonly number = signal('');
  readonly formError = signal('');
  readonly added = signal('');
  readonly confirming = signal<string | null>(null);

  readonly canEdit = computed(
    () => this.store.source() === 'live' && !this.access.readOnly && !this.store.isDemo(),
  );
  readonly lineName = computed(() => household(this.trusted.household())?.name ?? 'This line');

  age(at: string) {
    return relativeAge(at, this.store.now());
  }

  async add() {
    this.added.set('');
    const name = this.name().trim();
    const number = normalizeNumber(this.number());
    if (!name) return this.formError.set('Enter a name.');
    if (!number)
      return this.formError.set('Enter a full phone number, for example (214) 555-0147.');
    this.formError.set('');
    const failure = await this.trusted.add({ name, relation: this.relation().trim(), number });
    if (failure) return this.formError.set(`Not added. ${failure}`);
    this.added.set(`${name} was added. Their calls now ring straight through.`);
    this.name.set('');
    this.relation.set('');
    this.number.set('');
  }

  async remove(id: string) {
    const failure = await this.trusted.remove(id);
    this.confirming.set(null);
    this.formError.set(failure ? `Not removed. ${failure}` : '');
  }
}
