import { Component, DestroyRef, effect, inject, input, signal } from '@angular/core';
import { LucidePlay } from '@lucide/angular';
import { ConsoleStore } from '../core/console-store';
import { formatDuration } from '../core/format';
import { recordingAudio, recordingInfo } from '../core/owner-api';

/** Listen back to a recorded call (caller and FirstRing), loaded only on request. */
@Component({
  selector: 'app-call-recording',
  imports: [LucidePlay],
  template: `
    @if (available()) {
      <div class="mt-4 flex flex-wrap items-center gap-3">
        @if (url(); as src) {
          <audio class="w-full max-w-[32rem]" [src]="src" controls autoplay></audio>
        } @else {
          <button type="button" class="btn btn-ink" [disabled]="loading()" (click)="load()">
            <svg lucidePlay [size]="16" aria-hidden="true"></svg>
            {{ loading() ? 'Loading…' : 'Listen to the call' }}
            @if (seconds(); as s) {
              <span class="tabular font-semibold opacity-80">{{ duration(s) }}</span>
            }
          </button>
        }
        @if (error()) {
          <p class="text-[1rem] text-magenta-ink" role="alert">{{ error() }}</p>
        }
      </div>
    }
  `,
})
export class CallRecording {
  readonly sessionId = input.required<string>();
  private readonly store = inject(ConsoleStore);
  readonly available = signal(false);
  readonly seconds = signal<number | null>(null);
  readonly url = signal<string | null>(null);
  readonly loading = signal(false);
  readonly error = signal('');

  constructor() {
    effect((onCleanup) => {
      const id = this.sessionId();
      let cancelled = false;
      onCleanup(() => (cancelled = true));
      this.available.set(false);
      this.url.set(null);
      if (this.store.isDemo() || this.store.source() !== 'live') return;
      void recordingInfo(id).then((info) => {
        if (cancelled) return;
        this.available.set(info.available);
        this.seconds.set('recording' in info ? (info.recording?.duration_seconds ?? null) : null);
      });
    });
    inject(DestroyRef).onDestroy(() => {
      const url = this.url();
      if (url) URL.revokeObjectURL(url);
    });
  }

  duration(seconds: number) {
    return formatDuration(seconds);
  }

  async load() {
    this.loading.set(true);
    this.error.set('');
    try {
      this.url.set(await recordingAudio(this.sessionId()));
    } catch (failure) {
      this.error.set(failure instanceof Error ? failure.message : 'Not available.');
    } finally {
      this.loading.set(false);
    }
  }
}
