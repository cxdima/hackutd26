import { Component, computed, DestroyRef, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { LucideCheck, LucideMic, LucideSquare, LucideTrash2, LucideUpload } from '@lucide/angular';
import { ConsoleStore } from '../../core/console-store';
import {
  checkSample,
  CONSENT_STATEMENTS,
  EXAMPLE_REQUIREMENTS,
  type EnrollmentState,
} from '../../core/voice';

/**
 * Optional voice enrollment. The server reports voice cloning as disabled, so
 * the live console shows that plainly and the steps run only as an explicit
 * in-browser example: a recorded or chosen sample never leaves this browser and
 * no voice is created.
 */
@Component({
  selector: 'app-voice-enrollment',
  imports: [FormsModule, LucideCheck, LucideMic, LucideSquare, LucideUpload, LucideTrash2],
  template: `
    <section
      class="rounded-[var(--radius-card)] border-2 border-paper-3 p-5 sm:p-7 [&_label]:max-w-[68ch] [&_p]:max-w-[68ch]"
      aria-labelledby="voice-enroll-heading"
    >
      <div class="flex flex-wrap items-center justify-between gap-3">
        <h2 id="voice-enroll-heading" class="text-[1.25rem] font-extrabold">
          Your own voice (optional)
        </h2>
        @if (example()) {
          <span class="bug bug-example">Example · nothing is sent</span>
        } @else if (available()) {
          <span class="bug bug-live">Available</span>
        } @else {
          <span class="bug bug-example">Not available on this server</span>
        }
      </div>
      <p class="mt-2 max-w-[65ch] text-[1.0625rem] text-ink-3">
        FirstRing can answer in a preset synthetic voice, or one day in a voice that resembles its
        consenting owner. Voice cloning is optional. A voice can be recorded and imitated by others,
        so a familiar-sounding voice never proves who is calling.
      </p>
      <dl class="mt-4 grid grid-cols-[auto_1fr] gap-x-5 gap-y-1.5 text-[1.0625rem]">
        <dt class="font-bold text-muted">Voice in use now</dt>
        <dd>{{ currentVoice() }}</dd>
        <dt class="font-bold text-muted">Server</dt>
        <dd>{{ serverStatus() }}</dd>
      </dl>

      @if (!example()) {
        @if (!available()) {
          <button type="button" class="btn btn-quiet mt-5" (click)="startExample()">
            See how enrollment would work (example)
          </button>
        }
      } @else {
        <div class="mt-6 rounded-xl bg-paper-2 p-4 text-[1rem]" role="note">
          <strong>Example only.</strong> Every step below runs in this browser. Nothing is uploaded,
          no voice is created, and {{ name() }}’s real profile is not changed.
        </div>

        <ol class="mt-6 space-y-6" aria-label="Voice enrollment steps">
          <!-- 1. Choose -->
          <li>
            <fieldset>
              <legend class="text-[1.125rem] font-extrabold">1. Choose a voice</legend>
              <div class="mt-3 grid gap-2 sm:grid-cols-2">
                <label
                  class="flex min-h-14 cursor-pointer items-center gap-3 rounded-xl border-2 px-4"
                  [class]="choice() === 'preset' ? 'border-ink bg-paper' : 'border-rule'"
                >
                  <input
                    type="radio"
                    name="voice-choice"
                    value="preset"
                    [ngModel]="choice()"
                    (ngModelChange)="choice.set($event)"
                    class="size-5 accent-[#171c18]"
                  />
                  <span class="text-[1.0625rem] font-semibold">Keep a preset synthetic voice</span>
                </label>
                <label
                  class="flex min-h-14 cursor-pointer items-center gap-3 rounded-xl border-2 px-4"
                  [class]="choice() === 'own' ? 'border-ink bg-paper' : 'border-rule'"
                >
                  <input
                    type="radio"
                    name="voice-choice"
                    value="own"
                    [ngModel]="choice()"
                    (ngModelChange)="choice.set($event)"
                    class="size-5 accent-[#171c18]"
                  />
                  <span class="text-[1.0625rem] font-semibold">Use my own voice</span>
                </label>
              </div>
              @if (choice() === 'preset') {
                <p class="mt-3 text-[1rem] text-ink-3">
                  Nothing more to do. The preset voice is chosen under Call profile.
                </p>
              }
            </fieldset>
          </li>

          @if (choice() === 'own') {
            <!-- 2. Consent -->
            <li>
              <fieldset [disabled]="!fresh()">
                <legend class="text-[1.125rem] font-extrabold">2. Consent</legend>
                <div class="mt-3 space-y-2">
                  @for (statement of statements; track $index) {
                    <label
                      class="flex cursor-pointer items-start gap-3 rounded-xl bg-paper px-4 py-3"
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
                <label for="speaker-name" class="field-label mt-4"
                  >Your name, as the person whose voice this is</label
                >
                <input
                  id="speaker-name"
                  class="field max-w-[28rem]"
                  [ngModel]="speaker()"
                  (ngModelChange)="speaker.set($event)"
                  autocomplete="name"
                />
                @if (fresh()) {
                  <div>
                    <button
                      type="button"
                      class="btn btn-ink mt-4"
                      [disabled]="!canConsent()"
                      (click)="state.set('consented')"
                    >
                      I agree
                    </button>
                  </div>
                } @else {
                  <p class="mt-3 flex items-center gap-2 text-[1rem] font-semibold text-lime-ink">
                    <svg lucideCheck [size]="17" aria-hidden="true"></svg> Consent recorded in this
                    example.
                  </p>
                }
              </fieldset>
            </li>
          }

          @if (choice() === 'own' && !fresh()) {
            <!-- 3. Sample -->
            <li>
              <h3 class="text-[1.125rem] font-extrabold">3. Record or choose a sample</h3>
              <p class="mt-1 text-[1rem] text-ink-3">
                {{ requirements.minSeconds }} to {{ requirements.maxSeconds }} seconds of natural
                speech in a quiet room. Example requirements; a real provider sets its own.
              </p>
              <div class="mt-3 flex flex-wrap gap-3">
                @if (canRecord) {
                  @if (recording()) {
                    <button type="button" class="btn btn-ink" (click)="stopRecording()">
                      <svg lucideSquare [size]="16" aria-hidden="true"></svg> Stop ({{
                        recordSeconds()
                      }}
                      s)
                    </button>
                  } @else {
                    <button
                      type="button"
                      class="btn btn-quiet"
                      [disabled]="busy()"
                      (click)="record()"
                    >
                      <svg lucideMic [size]="17" aria-hidden="true"></svg> Record
                    </button>
                  }
                }
                <label class="btn btn-quiet cursor-pointer" [class.opacity-55]="busy()">
                  <svg lucideUpload [size]="17" aria-hidden="true"></svg> Choose an audio file
                  <input
                    type="file"
                    accept="audio/*"
                    class="visually-hidden"
                    [disabled]="busy()"
                    (change)="choose($event)"
                    data-testid="voice-file"
                  />
                </label>
              </div>
              @if (sampleUrl(); as url) {
                <div class="mt-3">
                  <p class="text-[0.95rem] font-bold text-muted">
                    Your sample (stays in this browser)
                  </p>
                  <audio [src]="url" controls class="mt-1 w-full max-w-[28rem]"></audio>
                </div>
              }
              @if (check(); as c) {
                <p
                  class="mt-2 text-[1.0625rem] font-semibold"
                  [class]="c.ok ? 'text-lime-ink' : 'text-magenta-ink'"
                  role="status"
                >
                  {{ c.message }}
                </p>
              }
              @if (check()?.ok && state() === 'consented') {
                <button type="button" class="btn btn-ink mt-3" (click)="process()">
                  Create the voice (example)
                </button>
              }
            </li>
          }

          @if (
            ['processing', 'verifying', 'ready', 'applied', 'failed', 'revoked'].includes(state())
          ) {
            <!-- 4-6. Processing, preview, apply -->
            <li>
              <h3 class="text-[1.125rem] font-extrabold">4. Processing and checks</h3>
              <p class="mt-1 text-[1.0625rem]" role="status">{{ stateText() }}</p>
              @if (state() === 'ready' || state() === 'applied') {
                <div class="mt-3 flex flex-wrap gap-3">
                  <button
                    type="button"
                    class="btn btn-quiet"
                    disabled
                    aria-describedby="preview-note"
                  >
                    Preview the voice
                  </button>
                  @if (state() === 'ready') {
                    <button type="button" class="btn btn-primary" (click)="state.set('applied')">
                      Use it for {{ name() }}’s calls (example)
                    </button>
                  }
                </div>
                <p id="preview-note" class="mt-2 text-[1rem] text-muted">
                  Preview is unavailable in the example: no voice was generated.
                </p>
              }
            </li>
          }

          @if (choice() === 'own' && !fresh()) {
            <li>
              <h3 class="text-[1.125rem] font-extrabold">Turn it off</h3>
              <p class="mt-1 text-[1rem] text-ink-3">
                Revoking stops using the voice and deletes the sample and the voice.
              </p>
              <button type="button" class="btn btn-quiet mt-3" (click)="revoke()">
                <svg lucideTrash2 [size]="17" aria-hidden="true"></svg> Turn off and delete
              </button>
            </li>
          }
        </ol>
        <button type="button" class="btn btn-quiet mt-6" (click)="endExample()">
          Close the example
        </button>
      }
    </section>
  `,
})
export class VoiceEnrollment {
  private readonly store = inject(ConsoleStore);
  readonly statements = CONSENT_STATEMENTS;
  readonly requirements = EXAMPLE_REQUIREMENTS;
  readonly canRecord =
    typeof navigator !== 'undefined' &&
    !!navigator.mediaDevices &&
    typeof MediaRecorder !== 'undefined';

  readonly example = signal(false);
  readonly choice = signal<'preset' | 'own'>('preset');
  readonly agreed = signal<boolean[]>(CONSENT_STATEMENTS.map(() => false));
  readonly speaker = signal('');
  readonly state = signal<EnrollmentState>('none');
  readonly sampleUrl = signal<string | null>(null);
  readonly check = signal<{ ok: boolean; message: string } | null>(null);
  readonly recording = signal(false);
  readonly recordSeconds = signal(0);
  readonly busy = computed(
    () => this.recording() || ['processing', 'verifying'].includes(this.state()),
  );
  private recorder?: MediaRecorder;
  private stream?: MediaStream;
  private timers: ReturnType<typeof setTimeout>[] = [];
  private tick?: ReturnType<typeof setInterval>;

  readonly name = computed(() => this.store.state()?.profile?.display_name ?? 'Sarah');
  readonly available = computed(() =>
    ['configured', 'verified'].includes(this.store.capability('voice_clone')?.status ?? ''),
  );
  readonly currentVoice = computed(() => {
    const speech = this.store.state()?.speech;
    return speech
      ? `${speech.voice_label} · ${speech.provider} (preset, synthetic)`
      : 'Not reported by the server';
  });
  readonly serverStatus = computed(() => {
    const clone = this.store.capability('voice_clone');
    if (this.store.isDemo()) return 'Demo view: no server.';
    if (!clone) return 'The server does not report voice enrollment.';
    return `${clone.status === 'disabled' ? 'Voice cloning is off' : 'Status: ' + clone.status}. ${clone.detail}`;
  });
  /** No consent in force: before agreeing, or after turning the voice off. */
  readonly fresh = computed(() => this.state() === 'none' || this.state() === 'revoked');
  readonly canConsent = computed(
    () => this.agreed().every(Boolean) && this.speaker().trim().length > 1,
  );
  readonly stateText = computed(() => {
    switch (this.state()) {
      case 'processing':
        return 'Processing the sample… (example)';
      case 'verifying':
        return 'Checking the sample is one consenting speaker… (example)';
      case 'ready':
        return 'Ready in the example. A real voice would be checked and previewed before use.';
      case 'applied':
        return `Example only: this is where ${this.name()}’s new calls would start using the voice. The real profile was not changed.`;
      case 'revoked':
        return 'Turned off. The sample was deleted from this browser.';
      case 'failed':
        return 'Processing failed. Try a clearer sample.';
      default:
        return '';
    }
  });

  constructor() {
    inject(DestroyRef).onDestroy(() => this.cleanup());
  }

  startExample() {
    this.example.set(true);
  }
  endExample() {
    this.cleanup();
    this.example.set(false);
    this.choice.set('preset');
    this.agreed.set(CONSENT_STATEMENTS.map(() => false));
    this.speaker.set('');
    this.state.set('none');
    this.check.set(null);
  }
  agree(index: number, value: boolean) {
    this.agreed.update((list) => list.map((v, i) => (i === index ? value : v)));
  }

  async record() {
    this.check.set(null);
    try {
      this.stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    } catch {
      this.check.set({
        ok: false,
        message: 'Microphone permission was not given. You can choose a file instead.',
      });
      return;
    }
    const chunks: Blob[] = [];
    this.recorder = new MediaRecorder(this.stream);
    this.recorder.ondataavailable = (e) => chunks.push(e.data);
    this.recorder.onstop = () => {
      const type = this.recorder?.mimeType || 'audio/webm';
      this.setSample(new Blob(chunks, { type }), type, this.recordSeconds());
      this.stream?.getTracks().forEach((t) => t.stop());
    };
    this.recordSeconds.set(0);
    this.recording.set(true);
    this.recorder.start();
    this.tick = setInterval(() => {
      this.recordSeconds.update((s) => s + 1);
      if (this.recordSeconds() >= this.requirements.maxSeconds) this.stopRecording();
    }, 1000);
  }

  stopRecording() {
    clearInterval(this.tick);
    this.recording.set(false);
    if (this.recorder?.state === 'recording') this.recorder.stop();
  }

  choose(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;
    const url = URL.createObjectURL(file);
    const audio = new Audio();
    audio.preload = 'metadata';
    audio.onloadedmetadata = () => this.setSample(file, file.type, audio.duration, url);
    audio.onerror = () => {
      URL.revokeObjectURL(url);
      this.check.set({ ok: false, message: 'That file could not be read as audio.' });
    };
    audio.src = url;
  }

  private setSample(blob: Blob, type: string, seconds: number, url = URL.createObjectURL(blob)) {
    const previous = this.sampleUrl();
    if (previous && previous !== url) URL.revokeObjectURL(previous);
    this.sampleUrl.set(url);
    this.check.set(checkSample(type, seconds));
  }

  process() {
    this.state.set('processing');
    this.timers.push(setTimeout(() => this.state.set('verifying'), 1200));
    this.timers.push(setTimeout(() => this.state.set('ready'), 2400));
  }

  revoke() {
    this.cleanup();
    this.state.set('revoked');
    this.agreed.set(CONSENT_STATEMENTS.map(() => false));
    this.check.set(null);
  }

  private cleanup() {
    for (const t of this.timers) clearTimeout(t);
    this.timers = [];
    clearInterval(this.tick);
    if (this.recorder?.state === 'recording') this.recorder.stop();
    this.stream?.getTracks().forEach((t) => t.stop());
    this.recording.set(false);
    const url = this.sampleUrl();
    if (url) URL.revokeObjectURL(url);
    this.sampleUrl.set(null);
  }
}
