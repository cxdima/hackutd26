import { Access, ReadOnlyNote } from '../core/access';
import {
  afterNextRender,
  Component,
  computed,
  DestroyRef,
  ElementRef,
  inject,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { FormsModule } from '@angular/forms';
import { LucideRefreshCw, LucideX } from '@lucide/angular';
import { NOISE_EVENTS } from '../core/calls';
import { ConsoleStore } from '../core/console-store';
import { shortId } from '../core/evidence';
import { formatDateTime, formatTime } from '../core/format';
import type { FixtureStatus } from '../core/schemas';

/**
 * Progressive-disclosure operator detail: provider readiness, model/speech
 * configuration, recorded evidence, test controls and the raw receipt stream.
 * Configured never means tested; a sent request never means completed.
 */
@Component({
  selector: 'app-operator-panel',
  imports: [FormsModule, LucideX, LucideRefreshCw, ReadOnlyNote],
  host: { class: 'contents' },
  template: `
    <div class="fixed inset-0 z-40 bg-ink/40" aria-hidden="true" (click)="closed.emit()"></div>
    <aside
      #panel
      tabindex="-1"
      role="dialog"
      aria-modal="true"
      (keydown)="trap($event)"
      class="fixed inset-y-0 right-0 z-50 flex w-full max-w-[44rem] flex-col bg-paper shadow-[var(--shadow-lift)] outline-none"
      aria-labelledby="operator-heading"
    >
      <div
        class="on-ink flex items-center justify-between gap-4 bg-ink px-5 py-4 text-paper sm:px-7"
      >
        <h2 id="operator-heading" class="text-[1.375rem] font-extrabold">Operator detail</h2>
        <button type="button" class="btn btn-quiet" (click)="closed.emit()">
          <svg lucideX [size]="18" aria-hidden="true"></svg> Close
        </button>
      </div>
      @let state = store.state()!;
      <div class="flex-1 space-y-8 overflow-y-auto px-5 py-6 sm:px-7">
        <section aria-labelledby="op-connection">
          <h3 id="op-connection" class="text-[1.1875rem] font-extrabold">Connection</h3>
          <dl class="mt-3 grid grid-cols-[auto_1fr] gap-x-5 gap-y-1.5 text-[1rem]">
            <dt class="font-bold text-muted">Source</dt>
            <dd>{{ sourceLabel() }}</dd>
            <dt class="font-bold text-muted">Execution mode</dt>
            <dd>{{ state.execution_mode }} · data origin {{ state.data_origin }}</dd>
            <dt class="font-bold text-muted">Transport</dt>
            <dd>{{ state.transport }}</dd>
            <dt class="font-bold text-muted">Phone line</dt>
            <dd>{{ state.phone_display }}</dd>
            <dt class="font-bold text-muted">Receipts held</dt>
            <dd class="tabular">
              {{ store.events().length
              }}{{ store.lastReceiptAt() ? ' · last ' + time(store.lastReceiptAt()!) : '' }}
            </dd>
          </dl>
        </section>

        @if (state.model_backend; as model) {
          <section aria-labelledby="op-model">
            <h3 id="op-model" class="text-[1.1875rem] font-extrabold">Decision model</h3>
            <dl class="mt-3 grid grid-cols-[auto_1fr] gap-x-5 gap-y-1.5 text-[1rem]">
              <dt class="font-bold text-muted">Backend</dt>
              <dd>{{ model.label }}</dd>
              <dt class="font-bold text-muted">Requested model</dt>
              <dd class="break-all">{{ model.requested_model }}</dd>
              <dt class="font-bold text-muted">Configured</dt>
              <dd>{{ model.configured ? 'Yes' : 'No' }}</dd>
              <dt class="font-bold text-muted">Native protocol verified</dt>
              <dd>{{ model.protocol_verified ? 'Yes, recorded' : 'Not verified' }}</dd>
            </dl>
            <p class="mt-2 text-[0.95rem] text-ink-3">{{ model.detail }}</p>
          </section>
        }

        @if (state.speech; as speech) {
          <section aria-labelledby="op-speech">
            <h3 id="op-speech" class="text-[1.1875rem] font-extrabold">Speech configuration</h3>
            <dl class="mt-3 grid grid-cols-[auto_1fr] gap-x-5 gap-y-1.5 text-[1rem]">
              <dt class="font-bold text-muted">Voice</dt>
              <dd>{{ speech.provider }} · {{ speech.voice_label }}</dd>
              <dt class="font-bold text-muted">Closing voice</dt>
              <dd>{{ speech.closing_provider }} · {{ speech.closing_voice }}</dd>
              <dt class="font-bold text-muted">Transcription</dt>
              <dd>
                {{ speech.transcription_provider ?? 'not configured' }}
                {{ speech.transcription_model ?? '' }}
              </dd>
            </dl>
            <p class="mt-2 text-[0.95rem] text-ink-3">
              Configuration only. It does not prove how a call sounded.
            </p>
          </section>
        }

        @if (state.evidence; as evidence) {
          <section aria-labelledby="op-evidence">
            <h3 id="op-evidence" class="text-[1.1875rem] font-extrabold">Recorded evidence</h3>
            <ul class="mt-3 space-y-2">
              @for (proof of proofs(); track proof.label) {
                <li class="rounded-xl bg-paper-2 px-4 py-3">
                  <p
                    class="flex flex-wrap items-center justify-between gap-2 text-[1.0625rem] font-bold"
                  >
                    {{ proof.label }}
                    <span
                      class="chip"
                      [class]="
                        proof.status === 'passed'
                          ? 'chip-saved'
                          : proof.status === 'failed'
                            ? 'chip-blocked'
                            : 'chip-none'
                      "
                      >{{ proof.status }}</span
                    >
                  </p>
                  <p class="mt-1 text-[0.95rem] text-ink-3">
                    {{ proof.detail }}{{ proof.recorded_at ? ' · ' + date(proof.recorded_at) : '' }}
                  </p>
                </li>
              }
            </ul>
          </section>
        }

        <section aria-labelledby="op-capabilities">
          <h3 id="op-capabilities" class="text-[1.1875rem] font-extrabold">Provider readiness</h3>
          <ul class="mt-3 divide-y divide-paper-3">
            @for (capability of state.capabilities; track capability.id) {
              <li class="flex items-start justify-between gap-4 py-2.5">
                <div>
                  <p class="text-[1.0625rem] font-bold">{{ capability.id.replaceAll('_', ' ') }}</p>
                  <p class="text-[0.95rem] text-ink-3">{{ capability.detail }}</p>
                </div>
                <span
                  class="chip shrink-0"
                  [class]="
                    capability.status === 'verified'
                      ? 'chip-saved'
                      : capability.status === 'configured'
                        ? 'chip-live'
                        : 'chip-none'
                  "
                  >{{ capability.status }}</span
                >
              </li>
            }
          </ul>
          <p class="mt-2 text-[0.95rem] text-ink-3">
            “Configured” means settings are present, not that a phone test passed.
          </p>
        </section>

        @if (state.phone_b; as phoneB) {
          <section aria-labelledby="op-phoneb">
            <h3 id="op-phoneb" class="text-[1.1875rem] font-extrabold">
              Designated test phone (Phone B)
            </h3>
            <p class="mt-2 text-[1rem]">
              Enrolled {{ phoneB.enrolled ? 'yes' : 'no' }} · enabled
              {{ phoneB.enabled ? 'yes' : 'no' }} · authorized
              {{ phoneB.authorized ? 'yes' : 'no' }} · attempts remaining
              {{ phoneB.remaining_attempts }}
            </p>
            <p class="mt-1 text-[0.95rem] text-ink-3">
              {{ phoneB.reason }} The call button lives in a live call’s Technical details.
            </p>
          </section>
        }

        @if (store.source() === 'live' && access.readOnly) {
          <app-read-only-note />
        } @else if (store.source() === 'live') {
          <section aria-labelledby="op-controls">
            <h3 id="op-controls" class="text-[1.1875rem] font-extrabold">Test controls</h3>
            @if (state.execution_mode === 'replay') {
              <p class="mt-2 text-[1rem] text-ink-3">
                Authored fixtures on the server. No calls or model requests.
              </p>
              <div class="mt-3 flex flex-wrap gap-2">
                <button
                  type="button"
                  class="btn btn-quiet"
                  [disabled]="disabled()"
                  (click)="send('/api/demo/replay', { scenario: 'benign' })"
                >
                  Replay a benign call
                </button>
                <button
                  type="button"
                  class="btn btn-quiet"
                  [disabled]="disabled()"
                  (click)="send('/api/demo/replay', { scenario: 'revocation' })"
                >
                  Replay revocation
                </button>
                <button
                  type="button"
                  class="btn btn-quiet"
                  [disabled]="disabled()"
                  (click)="send('/api/demo/replay', { scenario: 'benign_new_number' })"
                >
                  New-number control
                </button>
              </div>
            }
            <div class="mt-4 flex flex-wrap items-center gap-2">
              @if (confirmReset()) {
                <button type="button" class="btn btn-ink" [disabled]="disabled()" (click)="reset()">
                  Confirm: end test sessions and reset
                </button>
                <button type="button" class="btn btn-quiet" (click)="confirmReset.set(false)">
                  Cancel
                </button>
              } @else {
                <button
                  type="button"
                  class="btn btn-quiet"
                  [disabled]="disabled()"
                  (click)="confirmReset.set(true)"
                >
                  End test sessions and reset…
                </button>
              }
            </div>
            <p class="mt-2 text-[0.95rem] text-ink-3">
              Reset keeps audit events. It asks the server to end owned test sessions.
            </p>
          </section>

          @if (state.execution_mode === 'live') {
            <section aria-labelledby="op-fixture">
              <h3 id="op-fixture" class="text-[1.1875rem] font-extrabold">
                Automated test caller · read-only
              </h3>
              <p class="mt-1 text-[0.95rem] text-ink-3">
                Owned FirstRing number only. Refreshing never starts or retries a call.
              </p>
              <div class="mt-3 flex flex-wrap gap-2">
                <button
                  type="button"
                  class="btn btn-quiet"
                  [disabled]="fixtureLoading()"
                  (click)="refreshFixture()"
                >
                  <svg
                    lucideRefreshCw
                    [size]="16"
                    [class.animate-spin]="fixtureLoading()"
                    aria-hidden="true"
                  ></svg>
                  {{ fixtureLoading() ? 'Refreshing…' : 'Refresh status' }}
                </button>
                @if (fixture()) {
                  <button
                    type="button"
                    class="btn btn-quiet"
                    [disabled]="recordingLoading()"
                    (click)="loadRecording()"
                  >
                    {{ recordingLoading() ? 'Loading…' : 'Load latest recording' }}
                  </button>
                }
              </div>
              @if (fixtureError()) {
                <p class="mt-2 text-[1rem] font-semibold text-magenta-ink" role="alert">
                  {{ fixtureError() }}
                </p>
              }
              @if (fixture(); as f) {
                <dl class="mt-3 grid grid-cols-[auto_1fr] gap-x-5 gap-y-1.5 text-[1rem]">
                  <dt class="font-bold text-muted">Scenario</dt>
                  <dd>
                    {{ f.scenario_id.replaceAll('_', ' ') }} · {{ f.voice }} ·
                    {{ f.clip_count }} clips
                  </dd>
                  <dt class="font-bold text-muted">Provider status</dt>
                  <dd>
                    {{ f.provider_status ?? 'none received' }} ·
                    {{ (f.outcome ?? f.state ?? 'not reported').replaceAll('_', ' ') }}
                  </dd>
                  <dt class="font-bold text-muted">Attempts</dt>
                  <dd class="tabular">
                    {{ f.remaining_attempts }} remaining{{
                      f.attempt_count !== undefined && f.max_attempts !== undefined
                        ? ' · ' + f.attempt_count + ' of ' + f.max_attempts + ' reserved'
                        : ''
                    }}
                    · {{ f.time_limit_seconds }} s limit
                  </dd>
                  <dt class="font-bold text-muted">Heard by ASR</dt>
                  <dd class="tabular">{{ f.observations?.length ?? 0 }} final observations</dd>
                </dl>
                <p class="mt-2 text-[0.95rem] text-ink-3">
                  Ungraded. Human listening and noise testing are not verified.
                </p>
              }
              @if (recordingUrl()) {
                <audio
                  class="mt-3 w-full"
                  controls
                  [src]="recordingUrl()"
                  aria-label="Latest automated test call recording"
                ></audio>
                <p class="mt-1 text-[0.95rem] text-ink-3">
                  Caller-side recording of the latest automated test call. Plays only when you press
                  play.
                </p>
              }
            </section>
          }
        }

        <section aria-labelledby="op-events">
          <h3 id="op-events" class="text-[1.1875rem] font-extrabold">Raw receipt stream</h3>
          <div class="mt-3 flex flex-wrap items-end gap-3">
            <div class="min-w-48 flex-1">
              <label for="event-filter" class="field-label !text-[1rem]">Filter by type</label>
              <input
                id="event-filter"
                class="field !min-h-11"
                placeholder="e.g. policy, model, tool"
                [ngModel]="filter()"
                (ngModelChange)="filter.set($event)"
              />
            </div>
            <label class="inline-flex min-h-11 items-center gap-2 text-[1rem] font-semibold">
              <input
                type="checkbox"
                class="size-5 accent-ink"
                [checked]="showNoise()"
                (change)="showNoise.set(!showNoise())"
              />
              Include partials and telemetry
            </label>
          </div>
          <ol class="mt-4 space-y-2">
            @for (event of recentEvents(); track event.id) {
              <li>
                <details class="rounded-xl bg-paper-2">
                  <summary
                    class="flex min-h-12 cursor-pointer flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2 text-[0.95rem]"
                  >
                    <span class="tabular font-bold">#{{ event.sequence }}</span>
                    <span class="font-bold">{{ event.type }}</span>
                    <span class="text-muted"
                      >{{ event.leg_role }} · {{ event.execution_mode
                      }}{{ event.session_id ? ' · ' + short(event.session_id) : '' }} ·
                      {{ time(event.received_at) }}</span
                    >
                  </summary>
                  <pre
                    class="max-h-64 overflow-auto rounded-b-xl bg-ink p-3 text-[0.8125rem] leading-relaxed text-paper"
                    >{{ json(event.payload) }}</pre>
                </details>
              </li>
            } @empty {
              <li class="text-[1rem] text-ink-3">No receipts match.</li>
            }
          </ol>
        </section>
      </div>
    </aside>
  `,
})
export class OperatorPanel {
  readonly store = inject(ConsoleStore);
  readonly access = inject(Access);
  readonly closed = output<void>();
  private readonly panel = viewChild.required<ElementRef<HTMLElement>>('panel');
  readonly filter = signal('');
  readonly showNoise = signal(false);
  readonly confirmReset = signal(false);
  readonly fixture = signal<FixtureStatus | null>(null);
  readonly fixtureLoading = signal(false);
  readonly fixtureError = signal('');
  readonly recordingUrl = signal('');
  readonly recordingLoading = signal(false);

  readonly disabled = computed(() => !!this.store.busy() || this.store.stale());
  readonly sourceLabel = computed(() =>
    this.store.isDemo()
      ? 'Demo fixtures in this browser'
      : this.store.session() === 'local'
        ? 'Connected FirstRing server · local operator session (the token stays in the dev server)'
        : 'Connected FirstRing server · operator token in this tab’s memory',
  );
  readonly proofs = computed(() => {
    const evidence = this.store.state()?.evidence;
    if (!evidence) return [];
    return [
      { label: 'Model protocol', ...evidence.model_protocol },
      { label: 'Inbound phone (Gather)', ...evidence.inbound_phone },
      { label: 'Relay phone', ...evidence.relay_phone },
    ];
  });
  readonly recentEvents = computed(() => {
    const filter = this.filter().trim().toLowerCase();
    return this.store
      .events()
      .filter(
        (e) =>
          (this.showNoise() || !NOISE_EVENTS.has(e.type)) &&
          (!filter || e.type.toLowerCase().includes(filter)),
      )
      .slice(-60)
      .reverse();
  });

  constructor() {
    afterNextRender(() => this.panel().nativeElement.focus());
    inject(DestroyRef).onDestroy(() => {
      if (this.recordingUrl()) URL.revokeObjectURL(this.recordingUrl());
    });
  }

  /** Keeps keyboard focus inside the open drawer; Escape is handled by the shell. */
  trap(event: KeyboardEvent) {
    if (event.key !== 'Tab') return;
    const focusable = [
      ...this.panel().nativeElement.querySelectorAll<HTMLElement>(
        'button:not([disabled]), [href], input, select, textarea, summary, audio[controls]',
      ),
    ].filter((el) => el.offsetParent !== null);
    if (!focusable.length) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (
      event.shiftKey &&
      (document.activeElement === first || document.activeElement === this.panel().nativeElement)
    ) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  send(path: string, body: Record<string, unknown>) {
    void this.store.action(path, body);
  }

  reset() {
    this.confirmReset.set(false);
    void this.store.action('/api/demo/reset', {});
  }

  async refreshFixture() {
    if (this.fixtureLoading()) return;
    this.fixtureLoading.set(true);
    this.fixtureError.set('');
    try {
      this.fixture.set(await this.store.loadFixtureStatus());
    } catch (failure) {
      this.fixtureError.set(
        failure instanceof Error ? failure.message : 'Status could not be refreshed.',
      );
    } finally {
      this.fixtureLoading.set(false);
    }
  }

  async loadRecording() {
    if (this.recordingLoading()) return;
    this.recordingLoading.set(true);
    this.fixtureError.set('');
    try {
      const url = await this.store.loadFixtureRecording();
      if (this.recordingUrl()) URL.revokeObjectURL(this.recordingUrl());
      this.recordingUrl.set(url);
    } catch (failure) {
      this.fixtureError.set(failure instanceof Error ? failure.message : 'Recording unavailable.');
    } finally {
      this.recordingLoading.set(false);
    }
  }

  time(at: string) {
    return formatTime(at, this.store.state()?.profile?.timezone, true);
  }
  date(at: string) {
    return formatDateTime(at, this.store.state()?.profile?.timezone);
  }
  short(id: string) {
    return shortId(id);
  }
  json(value: unknown) {
    return JSON.stringify(value, null, 2);
  }
}
