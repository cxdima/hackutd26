import { Component, computed, DestroyRef, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { LucidePhone, LucidePlay, LucideTrash2 } from '@lucide/angular';
import { ConsoleStore } from '../../core/console-store';
import { formatPhone, formatTime, isE164 } from '../../core/format';
import {
  activateClone,
  armClone,
  clonePreview,
  cloneStatus,
  deactivateClone,
  deleteClone,
  type CloneStatus,
} from '../../core/owner-api';

const CONSENT = [
  'This is my own voice, and I agree to FirstRing creating an assistant voice that sounds like me.',
  'I understand a voice can be imitated. A familiar voice never proves who is calling.',
  'I can switch it off and delete it at any time.',
];

/**
 * Owner voice, recorded on a real phone call: consent here, call the FirstRing
 * number, read the passage after the tone, then use the voice on calls.
 */
@Component({
  selector: 'app-voice-enrollment',
  imports: [FormsModule, LucidePhone, LucidePlay, LucideTrash2],
  template: `
    <section
      class="rounded-[var(--radius-card)] border-2 border-paper-3 p-5 sm:p-7"
      aria-labelledby="voice-enroll-heading"
    >
      <h2 id="voice-enroll-heading" class="text-[1.25rem] font-extrabold">Your own voice</h2>
      <p class="mt-2 max-w-[62ch] text-[1.0625rem] text-ink-3">
        FirstRing can answer calls in a voice that sounds like you. Record it with one short phone
        call. A voice can be imitated, so it never proves who is calling.
      </p>

      @if (store.isDemo()) {
        <p class="mt-4 text-[1.0625rem] text-muted">
          Available when the FirstRing server is running.
        </p>
      } @else {
        @switch (phase()) {
          @case ('start') {
            <div class="mt-5 max-w-[62ch] space-y-2">
              @for (statement of consent; track $index) {
                <label
                  class="flex cursor-pointer items-start gap-3 rounded-xl bg-paper-2 px-4 py-3"
                >
                  <input
                    type="checkbox"
                    class="mt-1 size-5 shrink-0 accent-[#171c18]"
                    [ngModel]="agreed()[$index]"
                    (ngModelChange)="agree($index, $event)"
                  />
                  <span class="text-[1.0625rem]">{{ statement }}</span>
                </label>
              }
            </div>
            <label for="speaker-name" class="field-label mt-5">Your name</label>
            <input
              id="speaker-name"
              class="field max-w-[24rem]"
              autocomplete="name"
              [ngModel]="speaker()"
              (ngModelChange)="speaker.set($event)"
            />
            @if (status()?.state === 'failed' && status()?.error) {
              <p class="mt-3 text-[1rem] font-semibold text-magenta-ink" role="alert">
                The last attempt didn’t work: {{ status()?.error }}
              </p>
            }
            <div>
              <button
                type="button"
                class="btn btn-ink mt-5"
                [disabled]="!canStart() || busy()"
                (click)="start()"
              >
                <svg lucidePhone [size]="17" aria-hidden="true"></svg> Record my voice on a call
              </button>
            </div>
          }
          @case ('waiting') {
            <div class="mt-5 rounded-[var(--radius-card)] bg-ink p-6 text-paper">
              <p class="text-[1.0625rem] text-paper/80">
                {{
                  status()?.state === 'recording'
                    ? 'Recording. Read the passage, then press #.'
                    : 'Call now. After the tone, read this aloud:'
                }}
              </p>
              @if (number(); as n) {
                <p
                  class="tabular mt-1 text-[clamp(1.75rem,1.2rem+2vw,2.75rem)] font-extrabold text-lime"
                >
                  {{ n }}
                </p>
              }
              <blockquote class="mt-4 max-w-[60ch] text-[1.25rem] leading-relaxed">
                “{{ status()?.passage }}”
              </blockquote>
              @if (status()?.state === 'armed' && status()?.expires_at) {
                <p class="mt-4 text-[0.95rem] text-paper/70">
                  Waiting for your call until {{ time(status()?.expires_at) }}.
                </p>
              }
            </div>
          }
          @case ('processing') {
            <p class="mt-5 text-[1.125rem] font-semibold" role="status">
              Creating your voice… this takes about a minute.
            </p>
          }
          @case ('ready') {
            <p class="mt-5 text-[1.125rem] font-semibold">
              {{
                status()?.active ? 'FirstRing answers calls in your voice.' : 'Your voice is ready.'
              }}
            </p>
            <div class="mt-4 flex flex-wrap items-center gap-3">
              <button type="button" class="btn btn-quiet" [disabled]="busy()" (click)="preview()">
                <svg lucidePlay [size]="16" aria-hidden="true"></svg> Hear a sample
              </button>
              @if (status()?.active) {
                <button
                  type="button"
                  class="btn btn-quiet"
                  [disabled]="busy()"
                  (click)="setActive(false)"
                >
                  Use the standard voice
                </button>
              } @else {
                <button
                  type="button"
                  class="btn btn-primary"
                  [disabled]="busy()"
                  (click)="setActive(true)"
                >
                  Use my voice on calls
                </button>
              }
              <button type="button" class="btn btn-quiet" [disabled]="busy()" (click)="remove()">
                <svg lucideTrash2 [size]="16" aria-hidden="true"></svg> Delete my voice
              </button>
            </div>
            @if (sampleUrl(); as url) {
              <audio class="mt-4 w-full max-w-[28rem]" [src]="url" controls autoplay></audio>
            }
          }
        }
        @if (error()) {
          <p class="mt-3 text-[1rem] font-semibold text-magenta-ink" role="alert">{{ error() }}</p>
        }
      }
    </section>
  `,
})
export class VoiceEnrollment {
  readonly store = inject(ConsoleStore);
  readonly consent = CONSENT;
  readonly agreed = signal(CONSENT.map(() => false));
  readonly speaker = signal('');
  readonly status = signal<CloneStatus | null>(null);
  readonly busy = signal(false);
  readonly error = signal('');
  readonly sampleUrl = signal<string | null>(null);

  readonly canStart = computed(
    () => this.agreed().every(Boolean) && this.speaker().trim().length > 1,
  );
  readonly number = computed(() => {
    const raw = this.store.state()?.phone_display;
    return isE164(raw) ? formatPhone(raw) : null;
  });
  readonly phase = computed(() => {
    const state = this.status()?.state ?? 'none';
    if (state === 'armed' || state === 'recording') return 'waiting';
    if (state === 'processing') return 'processing';
    if (state === 'ready' || state === 'active') return 'ready';
    return 'start';
  });

  constructor() {
    const destroy = inject(DestroyRef);
    if (this.store.isDemo()) return;
    void this.refresh();
    const timer = setInterval(() => {
      if (['waiting', 'processing'].includes(this.phase())) void this.refresh();
    }, 2000);
    destroy.onDestroy(() => {
      clearInterval(timer);
      const url = this.sampleUrl();
      if (url) URL.revokeObjectURL(url);
    });
  }

  agree(index: number, value: boolean) {
    this.agreed.update((list) => list.map((v, i) => (i === index ? value : v)));
  }
  time(at: string | null | undefined) {
    return at ? formatTime(at, this.store.state()?.profile?.timezone) : '';
  }

  async refresh() {
    try {
      this.status.set(await cloneStatus());
    } catch {
      /* the next poll retries */
    }
  }
  private async run(task: () => Promise<CloneStatus>) {
    this.busy.set(true);
    this.error.set('');
    try {
      this.status.set(await task());
    } catch (failure) {
      this.error.set(failure instanceof Error ? failure.message : 'Something went wrong.');
    } finally {
      this.busy.set(false);
    }
  }
  start() {
    return this.run(() => armClone(this.speaker().trim()));
  }
  setActive(active: boolean) {
    return this.run(() => (active ? activateClone() : deactivateClone()));
  }
  remove() {
    return this.run(() => deleteClone());
  }
  async preview() {
    this.busy.set(true);
    this.error.set('');
    try {
      const previous = this.sampleUrl();
      if (previous) URL.revokeObjectURL(previous);
      this.sampleUrl.set(await clonePreview());
    } catch (failure) {
      this.error.set(failure instanceof Error ? failure.message : 'No sample available.');
    } finally {
      this.busy.set(false);
    }
  }
}
