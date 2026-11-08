import { Component, computed, DestroyRef, effect, inject, signal, untracked } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { LucideCheck, LucideLoader, LucideRotateCw } from '@lucide/angular';
import { ConsoleStore } from '../core/console-store';
import { household } from '../core/household';
import type { Profile } from '../core/schemas';
import { SelectMenu, type SelectOption } from '../shared/select-menu';
import { Access, ReadOnlyNote } from '../core/access';
import { HandoffSetup } from './setup/handoff-setup';
import { TrustedCallers } from './trusted-callers';
import { VoiceEnrollment } from './setup/voice-enrollment';

type Status = 'idle' | 'loading' | 'saving' | 'saved' | 'error';

export const VOICE_PRESETS = [
  { id: 'cgSgspJ2msm6clMCkdW9-v4', label: 'Jessica · warm, conversational' },
  { id: 'JBFqnCBsd6RMkjVDRZzb-v4', label: 'George · warm storyteller' },
  { id: '21m00Tcm4TlvDq8ikWAM-v4', label: 'Janet · legacy voice' },
];
export const TIMEZONES = [
  { id: 'America/Chicago', label: 'Central time · America/Chicago' },
  { id: 'America/New_York', label: 'Eastern time · America/New_York' },
  { id: 'America/Denver', label: 'Mountain time · America/Denver' },
  { id: 'America/Los_Angeles', label: 'Pacific time · America/Los_Angeles' },
  { id: 'UTC', label: 'UTC' },
];
/** "Jessica · warm, conversational" becomes a name with a quieter second line. */
function toOptions(items: { id: string; label: string }[]): SelectOption[] {
  return items.map(({ id, label }) => {
    const [name, ...rest] = label.split(' · ');
    const detail = rest.join(' · ');
    return {
      value: id,
      label: name,
      detail: detail ? detail[0].toUpperCase() + detail.slice(1) : undefined,
    };
  });
}
const LINK_OR_NUMBER = /(https?:\/\/|www\.|\b\d{3}[\s.-]?\d{3}[\s.-]?\d{4}\b)/i;

/**
 * Profile editor ported from web/src/ProfileEditor.tsx: same endpoint, same fields,
 * saved values apply to new calls only. Client checks mirror the server's rules;
 * the server remains the authority.
 */
@Component({
  selector: 'app-settings-page',
  imports: [
    FormsModule,
    SelectMenu,
    LucideCheck,
    LucideLoader,
    LucideRotateCw,
    HandoffSetup,
    VoiceEnrollment,
    TrustedCallers,
    ReadOnlyNote,
  ],
  template: `
    @let state = store.state()!;
    <header>
      <h1
        class="text-[clamp(2rem,1.6rem+1.4vw,2.75rem)] leading-[1.05] font-extrabold tracking-[-0.03em]"
      >
        Settings
      </h1>
      <p class="mt-2 max-w-[64ch] text-[1.0625rem] text-ink-3">
        How FirstRing greets callers, who skips screening, and whose phone rings. Changes apply to
        new calls; a call in progress keeps the settings it started with.
      </p>
      @if (access.readOnly) {
        <app-read-only-note class="mt-2 block" />
      }
    </header>

    <nav class="settings-index" aria-label="Settings sections">
      @for (link of sections; track link.id) {
        <a [href]="'#' + link.id" (click)="jump($event, link.id)">{{ link.label }}</a>
      }
    </nav>

    <section id="profile" class="settings-section" aria-labelledby="profile-heading">
      <div class="settings-aside">
        <h2 id="profile-heading" class="text-[1.375rem] font-extrabold">Call profile</h2>
        <p class="mt-1 text-[1rem] text-ink-3">
          The name and greeting FirstRing uses, per line. FirstRing never pretends to be the person
          it answers for.
        </p>
        <label id="next-call-label" for="next-call" class="field-label mt-6"
          >Next call is answered for</label
        >
        <app-select-menu
          inputId="next-call"
          labelledBy="next-call-label"
          describedBy="next-call-help"
          [options]="lineOptions()"
          [value]="state.selected_household"
          [disabled]="store.isDemo() || access.readOnly || !!store.busy() || store.stale()"
          (valueChange)="selectNext($event)"
        />
        <p id="next-call-help" class="field-help">
          {{
            store.isDemo()
              ? 'Available when connected to a server.'
              : 'Applies to the next incoming call only.'
          }}
        </p>
        <dl class="mt-6 space-y-3 text-[1rem]">
          <div>
            <dt class="font-bold">Familiar voices</dt>
            <dd class="text-ink-3">
              Never treated as permission to share a code, send money or give access.
            </dd>
          </div>
        </dl>
      </div>

      <div class="min-w-0">
        <div class="flex flex-wrap gap-2" role="group" aria-label="Choose a profile to edit">
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
        <div class="mt-5">
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
                  <label for="profile-voice" id="profile-voice-label" class="field-label"
                    >Assistant voice</label
                  >
                  <app-select-menu
                    inputId="profile-voice"
                    labelledBy="profile-voice-label"
                    describedBy="voice-help"
                    name="voice_id"
                    [options]="voiceOptions()"
                    [ngModel]="d.voice_id"
                    (ngModelChange)="patch({ voice_id: $event })"
                  />
                  <p id="voice-help" class="field-help">
                    A preset synthetic voice. Previews aren’t available in this prototype.
                  </p>
                </div>
                <div>
                  <label for="profile-timezone" id="profile-timezone-label" class="field-label"
                    >Timezone</label
                  >
                  <app-select-menu
                    inputId="profile-timezone"
                    labelledBy="profile-timezone-label"
                    describedBy="tz-help"
                    name="timezone"
                    [options]="zoneOptions()"
                    [ngModel]="d.timezone"
                    (ngModelChange)="patch({ timezone: $event })"
                  />
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
                  [disabled]="
                    status() === 'saving' || !dirty() || store.isDemo() || access.readOnly
                  "
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
        </div>
      </div>
    </section>

    <section id="trusted" class="settings-section" aria-labelledby="trusted-heading">
      <div class="settings-aside">
        <h2 id="trusted-heading" class="text-[1.375rem] font-extrabold">Trusted callers</h2>
        <p class="mt-1 text-[1rem] text-ink-3">
          Calls from these numbers skip AI screening, recording and reputation and ring straight
          through to {{ ringsFirst() }}’s enrolled phone.
        </p>
        <p class="mt-3 text-[1rem] font-semibold text-amber-ink">
          Caller ID can be faked. Only add numbers you would answer anyway.
        </p>
      </div>
      <div class="min-w-0">
        <app-trusted-callers />
      </div>
    </section>

    <div id="handoff" class="settings-block">
      <app-handoff-setup />
    </div>
    <div id="voice" class="settings-block">
      <app-voice-enrollment />
    </div>
  `,
})
export class SettingsPage {
  readonly store = inject(ConsoleStore);
  readonly access = inject(Access);
  readonly sections = [
    { id: 'profile', label: 'Call profile' },
    { id: 'trusted', label: 'Trusted callers' },
    { id: 'handoff', label: 'Ring the phone' },
    { id: 'voice', label: 'Owner voice' },
  ];
  readonly lineOptions = computed<SelectOption[]>(() =>
    (this.store.state()?.community ?? []).map((h) => ({
      value: h.household_id,
      label: `${h.display_name}’s line`,
    })),
  );
  readonly ringsFirst = computed(
    () => household(this.store.state()?.selected_household)?.ringsFirst.name ?? 'the owner',
  );
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
  constructor() {
    effect(() => {
      const household = this.household();
      untracked(() => this.load(household));
    });
    inject(DestroyRef).onDestroy(() => this.controller?.abort());
  }

  readonly voiceOptions = computed(() => toOptions(this.voicesFor(this.draft()?.voice_id ?? '')));
  readonly zoneOptions = computed(() => toOptions(this.zonesFor(this.draft()?.timezone ?? '')));

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

  jump(event: Event, id: string) {
    event.preventDefault();
    const target = document.getElementById(id);
    if (!target) return;
    target.scrollIntoView({
      behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth',
      block: 'start',
    });
    history.replaceState(history.state, '', `${location.pathname}${location.search}#${id}`);
  }

  selectNext(id: string | null) {
    if (!id || id === this.store.state()?.selected_household) return;
    void this.store.action('/api/demo/household', { household_id: id });
  }
}
