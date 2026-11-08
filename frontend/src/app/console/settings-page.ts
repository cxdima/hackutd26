import { Component, computed, DestroyRef, effect, inject, signal, untracked } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { LucideCheck, LucideLoader, LucideRotateCw } from '@lucide/angular';
import { ConsoleStore } from '../core/console-store';
import type { Profile } from '../core/schemas';
import { HandoffSetup } from './setup/handoff-setup';
import { VoiceEnrollment } from './setup/voice-enrollment';

type Status = 'idle' | 'loading' | 'saving' | 'saved' | 'error';

export const VOICE_PRESETS = [
  { id: 'cgSgspJ2msm6clMCkdW9-v4', label: 'Jessica · warm, conversational' },
  { id: 'JBFqnCBsd6RMkjVDRZzb-v4', label: 'George · warm storyteller' },
  { id: '21m00Tcm4TlvDq8ikWAM-v4', label: 'Janet · legacy voice' },
];
export const TIMEZONES = [
  { id: 'America/Chicago', label: 'Central · America/Chicago' },
  { id: 'America/New_York', label: 'Eastern · America/New_York' },
  { id: 'America/Denver', label: 'Mountain · America/Denver' },
  { id: 'America/Los_Angeles', label: 'Pacific · America/Los_Angeles' },
  { id: 'UTC', label: 'UTC' },
];
const LINK_OR_NUMBER = /(https?:\/\/|www\.|\b\d{3}[\s.-]?\d{3}[\s.-]?\d{4}\b)/i;

/**
 * Profile editor ported from web/src/ProfileEditor.tsx: same endpoint, same fields,
 * saved values apply to new calls only. Client checks mirror the server's rules;
 * the server remains the authority.
 */
@Component({
  selector: 'app-settings-page',
  imports: [FormsModule, LucideCheck, LucideLoader, LucideRotateCw, HandoffSetup, VoiceEnrollment],
  template: `
    @let state = store.state()!;
    <div class="grid gap-10 lg:grid-cols-12">
      <div class="lg:col-span-7">
        <h1
          class="text-[clamp(2rem,1.6rem+1.6vw,2.9rem)] leading-[1.05] font-extrabold tracking-[-0.03em]"
        >
          Settings
        </h1>
        <p class="mt-3 max-w-[56ch] text-lede text-ink-3">
          How FirstRing greets callers and which voice it uses. Changes apply to new calls; a call
          in progress keeps the settings it started with.
        </p>

        <div class="mt-8 flex flex-wrap gap-2" role="group" aria-label="Choose a profile to edit">
          @for (h of state.community; track h.household_id) {
            <button
              type="button"
              class="min-h-12 rounded-full border-2 px-5 text-[1.0625rem] font-bold"
              [class]="
                household() === h.household_id
                  ? 'border-ink bg-ink text-paper'
                  : 'border-rule bg-paper hover:border-ink'
              "
              [attr.aria-pressed]="household() === h.household_id"
              (click)="choose(h.household_id)"
            >
              {{ h.display_name }}
            </button>
          }
        </div>

        <section
          class="mt-6 rounded-[var(--radius-card)] border-2 border-ink p-5 sm:p-8"
          aria-labelledby="profile-heading"
        >
          <h2 id="profile-heading" class="text-[1.5rem] font-extrabold">Call profile</h2>
          @if (status() === 'loading' && !draft()) {
            <p class="mt-4 flex items-center gap-2 text-[1.0625rem]" role="status">
              <svg lucideLoader [size]="18" class="animate-spin" aria-hidden="true"></svg> Loading
              profile…
            </p>
          }
          @if (error()) {
            <div class="mt-4 rounded-xl bg-magenta-wash p-4 text-magenta-ink" role="alert">
              <p class="text-[1.0625rem] font-semibold">{{ error() }}</p>
              @if (!draft()) {
                <button type="button" class="btn btn-quiet mt-3" (click)="load()">
                  <svg lucideRotateCw [size]="16" aria-hidden="true"></svg> Try again
                </button>
              }
            </div>
          }
          @if (status() === 'saved') {
            <p
              class="mt-4 flex items-center gap-2 rounded-xl bg-lime-wash p-4 text-[1.0625rem] font-semibold text-lime-ink"
              role="status"
            >
              <svg lucideCheck [size]="18" aria-hidden="true"></svg> Saved as version
              {{ draft()?.version }}. New calls will use it.
            </p>
          }
          @if (draft(); as d) {
            <form class="mt-6 space-y-6" (ngSubmit)="save()" novalidate>
              <div>
                <label for="profile-name" class="field-label">Name FirstRing answers for</label>
                <input
                  id="profile-name"
                  name="display_name"
                  class="field"
                  maxlength="60"
                  required
                  [ngModel]="d.display_name"
                  (ngModelChange)="patch({ display_name: $event })"
                  [attr.aria-invalid]="!!nameError()"
                  aria-describedby="name-help name-error"
                />
                <p id="name-help" class="field-help">
                  Shown in summaries. FirstRing never pretends to be this person.
                </p>
                @if (nameError()) {
                  <p id="name-error" class="mt-1 text-[1rem] font-semibold text-magenta-ink">
                    {{ nameError() }}
                  </p>
                }
              </div>
              <div>
                <label for="profile-greeting" class="field-label">Opening greeting</label>
                <textarea
                  id="profile-greeting"
                  name="greeting"
                  class="field min-h-28"
                  maxlength="240"
                  rows="3"
                  required
                  [ngModel]="d.greeting"
                  (ngModelChange)="patch({ greeting: $event })"
                  [attr.aria-invalid]="!!greetingError()"
                  aria-describedby="greeting-help greeting-error"
                ></textarea>
                <p id="greeting-help" class="field-help tabular">
                  {{ d.greeting.length }}/240 characters. Keep it neutral; no links or phone
                  numbers.
                </p>
                @if (greetingError()) {
                  <p id="greeting-error" class="mt-1 text-[1rem] font-semibold text-magenta-ink">
                    {{ greetingError() }}
                  </p>
                }
              </div>
              <div class="grid gap-6 sm:grid-cols-2">
                <div>
                  <label for="profile-voice" class="field-label">Assistant voice</label>
                  <select
                    id="profile-voice"
                    name="voice_id"
                    class="field"
                    [ngModel]="d.voice_id"
                    (ngModelChange)="patch({ voice_id: $event })"
                    aria-describedby="voice-help"
                  >
                    @for (voice of voicesFor(d.voice_id); track voice.id) {
                      <option [value]="voice.id">{{ voice.label }}</option>
                    }
                  </select>
                  <p id="voice-help" class="field-help">
                    A preset synthetic voice. Previews aren’t available in this prototype.
                  </p>
                </div>
                <div>
                  <label for="profile-timezone" class="field-label">Timezone</label>
                  <select
                    id="profile-timezone"
                    name="timezone"
                    class="field"
                    [ngModel]="d.timezone"
                    (ngModelChange)="patch({ timezone: $event })"
                    aria-describedby="tz-help"
                  >
                    @for (zone of zonesFor(d.timezone); track zone.id) {
                      <option [value]="zone.id">{{ zone.label }}</option>
                    }
                  </select>
                  <p id="tz-help" class="field-help">
                    Used for times in summaries and this dashboard.
                  </p>
                </div>
              </div>
              <p class="text-[1rem] text-muted">
                {{ d.tts_provider }} · {{ d.locale }} · configured version {{ d.version }}.
                Selecting a voice is configuration, not proof of how it sounds on a call.
              </p>
              <div class="flex flex-wrap items-center gap-3">
                <button
                  type="submit"
                  class="btn btn-primary !min-h-14 !px-7 text-[1.125rem]"
                  [disabled]="status() === 'saving' || !dirty() || store.isDemo()"
                >
                  @if (status() === 'saving') {
                    <svg lucideLoader [size]="18" class="animate-spin" aria-hidden="true"></svg>
                    Saving…
                  } @else {
                    Save for new calls
                  }
                </button>
                @if (dirty()) {
                  <button
                    type="button"
                    class="btn btn-quiet"
                    [disabled]="status() === 'saving'"
                    (click)="revert()"
                  >
                    Undo changes
                  </button>
                }
              </div>
              @if (store.isDemo()) {
                <p class="text-[1rem] text-muted">
                  Demo view: you can try the form, but saving needs a connected FirstRing server.
                </p>
              }
            </form>
          }
        </section>
      </div>

      <aside class="space-y-6 lg:col-span-5 lg:pt-[7.5rem]" aria-label="Call routing and voice">
        <section
          class="rounded-[var(--radius-card)] bg-paper-2 p-5 sm:p-6"
          aria-labelledby="next-call-heading"
        >
          <h2 id="next-call-heading" class="text-[1.1875rem] font-extrabold">
            Who answers the next call
          </h2>
          <p class="mt-1 text-[1rem] text-ink-3">
            Selects the profile for the next incoming call only. Calls already in progress are not
            changed.
          </p>
          <div class="mt-4 flex flex-wrap gap-2">
            @for (h of state.community; track h.household_id) {
              <button
                type="button"
                class="min-h-12 rounded-full border-2 px-5 text-[1.0625rem] font-bold disabled:cursor-not-allowed"
                [class]="
                  state.selected_household === h.household_id
                    ? 'border-ink bg-lime text-ink'
                    : 'border-rule bg-paper hover:border-ink'
                "
                [attr.aria-pressed]="state.selected_household === h.household_id"
                [disabled]="store.isDemo() || !!store.busy() || store.stale()"
                (click)="selectNext(h.household_id)"
              >
                {{ h.display_name }}
              </button>
            }
          </div>
          @if (store.isDemo()) {
            <p class="mt-3 text-[1rem] text-muted">Available when connected to a server.</p>
          }
        </section>
        <section
          class="rounded-[var(--radius-card)] border-2 border-paper-3 p-5 sm:p-6"
          aria-labelledby="voice-privacy-heading"
        >
          <h2 id="voice-privacy-heading" class="text-[1.1875rem] font-extrabold">
            Voice and privacy
          </h2>
          <dl class="mt-3 space-y-3 text-[1.0625rem]">
            <div>
              <dt class="font-bold">Custom voice cloning</dt>
              <dd class="text-ink-3">{{ cloneStatus() }}</dd>
            </div>
            <div>
              <dt class="font-bold">Familiar voices</dt>
              <dd class="text-ink-3">
                Never treated as permission to share a code, send money or give access.
              </dd>
            </div>
          </dl>
        </section>
      </aside>
    </div>
    <div class="mt-14 space-y-8">
      <app-handoff-setup />
      <app-voice-enrollment />
    </div>
  `,
})
export class SettingsPage {
  readonly store = inject(ConsoleStore);
  readonly household = signal(this.store.state()?.selected_household ?? 'sarah');
  readonly draft = signal<Profile | null>(null);
  private readonly original = signal<Profile | null>(null);
  readonly status = signal<Status>('idle');
  readonly error = signal('');
  private controller?: AbortController;

  readonly dirty = computed(() => JSON.stringify(this.draft()) !== JSON.stringify(this.original()));
  readonly nameError = computed(() => {
    const name = this.draft()?.display_name.trim() ?? '';
    return !name ? 'Enter a name.' : '';
  });
  readonly greetingError = computed(() => {
    const greeting = this.draft()?.greeting.trim() ?? '';
    if (!greeting) return 'Enter a greeting.';
    if (LINK_OR_NUMBER.test(greeting)) return 'Remove links and phone numbers from the greeting.';
    return '';
  });
  readonly cloneStatus = computed(() => {
    const clone = this.store.state()?.capabilities.find((c) => c.id === 'voice_clone');
    return clone && clone.status !== 'disabled'
      ? `Status: ${clone.status}. ${clone.detail}`
      : 'Off. Not available in this prototype; FirstRing uses preset synthetic voices.';
  });

  constructor() {
    effect(() => {
      const household = this.household();
      untracked(() => this.load(household));
    });
    inject(DestroyRef).onDestroy(() => this.controller?.abort());
  }

  voicesFor(current: string) {
    return VOICE_PRESETS.some((v) => v.id === current)
      ? VOICE_PRESETS
      : [{ id: current, label: `${current} (current)` }, ...VOICE_PRESETS];
  }
  zonesFor(current: string) {
    return TIMEZONES.some((z) => z.id === current)
      ? TIMEZONES
      : [{ id: current, label: `${current} (current)` }, ...TIMEZONES];
  }

  choose(id: string) {
    if (id === this.household()) return;
    this.household.set(id);
  }

  async load(household = this.household()) {
    this.controller?.abort();
    const controller = new AbortController();
    this.controller = controller;
    this.status.set('loading');
    this.error.set('');
    this.draft.set(null);
    try {
      const profile = await this.store.loadProfile(household, controller.signal);
      if (controller.signal.aborted) return;
      this.draft.set(profile);
      this.original.set(structuredClone(profile));
      this.status.set('idle');
    } catch (failure) {
      if (controller.signal.aborted) return;
      this.status.set('error');
      this.error.set(failure instanceof Error ? failure.message : 'Profile could not be loaded.');
    }
  }

  patch(change: Partial<Profile>) {
    const current = this.draft();
    if (!current) return;
    this.draft.set({ ...current, ...change });
    if (this.status() === 'saved' || this.status() === 'error') {
      this.status.set('idle');
      this.error.set('');
    }
  }

  revert() {
    const original = this.original();
    if (original) this.draft.set(structuredClone(original));
    this.status.set('idle');
    this.error.set('');
  }

  async save() {
    const draft = this.draft();
    if (!draft || this.status() === 'saving') return;
    if (this.nameError() || this.greetingError()) {
      this.error.set('Fix the highlighted fields, then save again.');
      document.getElementById(this.nameError() ? 'profile-name' : 'profile-greeting')?.focus();
      return;
    }
    this.controller?.abort();
    const controller = new AbortController();
    this.controller = controller;
    this.status.set('saving');
    this.error.set('');
    try {
      const saved = await this.store.saveProfile(this.household(), draft, controller.signal);
      if (controller.signal.aborted) return;
      this.draft.set(saved);
      this.original.set(structuredClone(saved));
      this.status.set('saved');
    } catch (failure) {
      if (controller.signal.aborted) return;
      this.status.set('error');
      this.error.set(failure instanceof Error ? failure.message : 'Profile not saved.');
    }
  }

  selectNext(id: string) {
    void this.store.action('/api/demo/household', { household_id: id });
  }
}
