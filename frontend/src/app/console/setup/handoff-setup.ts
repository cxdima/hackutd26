import { Access, ReadOnlyNote } from '../../core/access';
import { Component, computed, inject } from '@angular/core';
import { RouterLink } from '@angular/router';
import { LucideCheck, LucideCircleDashed, LucideLockKeyhole } from '@lucide/angular';
import { handoffView } from '../../core/live';
import { ConsoleStore } from '../../core/console-store';
import { formatTime } from '../../core/format';
import { PhoneDemoControl } from '../phone-demo-control';

/**
 * Explains the private permission call and shows the server's own readiness.
 * Nothing here places a call. "Ring my phone on the next call" arms the next
 * real inbound call only, for ten minutes, by an explicit click.
 */
@Component({
  selector: 'app-handoff-setup',
  imports: [
    RouterLink,
    LucideCheck,
    LucideCircleDashed,
    LucideLockKeyhole,
    PhoneDemoControl,
    ReadOnlyNote,
  ],
  template: `
    <section
      class="rounded-[var(--radius-card)] border-2 border-ink"
      aria-labelledby="handoff-setup-heading"
    >
      <div
        class="on-ink flex flex-wrap items-center justify-between gap-3 rounded-t-[1.1rem] bg-ink px-5 py-3 text-paper sm:px-7"
      >
        <h2 id="handoff-setup-heading" class="text-[1.25rem] font-extrabold">
          Private permission calls
        </h2>
      </div>
      <div class="grid gap-8 p-5 sm:p-7 lg:grid-cols-2">
        <div>
          <p class="max-w-[60ch] text-[1.0625rem]">
            When a caller asks for {{ name() }}, FirstRing can privately ring {{ name() }}’s
            enrolled phone, play a short summary, and connect the caller only if
            {{ name() }} presses 1.
            <strong>Nothing on this page places a call.</strong>
          </p>
          <ol class="mt-5 border-t-2 border-ink" aria-label="What happens on a call">
            @for (step of steps(); track step; let i = $index) {
              <li
                class="grid grid-cols-[2rem_1fr] gap-3 border-b border-paper-3 py-2.5 text-[1.0625rem]"
              >
                <span class="tabular font-extrabold text-muted">{{ i + 1 }}</span
                ><span>{{ step }}</span>
              </li>
            }
          </ol>
          <p class="mt-3 text-[1rem] text-ink-3">
            If the caller switches to asking for a code, payment or device access, the offer is
            withdrawn at once, and a “1” pressed afterwards is rejected.
          </p>
        </div>
        <div>
          <h3 class="text-[1.125rem] font-extrabold">What the server reports</h3>
          <ul class="mt-3 border-t border-paper-3" aria-label="Handoff readiness">
            @for (item of readiness(); track item.label) {
              <li class="grid grid-cols-[1.75rem_1fr] gap-2 border-b border-paper-3 py-3">
                @if (item.ok) {
                  <svg
                    lucideCheck
                    [size]="20"
                    class="mt-0.5 text-lime-ink"
                    aria-hidden="true"
                  ></svg>
                } @else {
                  <svg
                    lucideCircleDashed
                    [size]="20"
                    class="mt-0.5 text-amber-ink"
                    aria-hidden="true"
                  ></svg>
                }
                <div>
                  <p class="text-[1.0625rem] font-bold">
                    {{ item.label
                    }}<span class="visually-hidden">{{ item.ok ? ': ready' : ': not ready' }}</span>
                  </p>
                  <p class="text-[1rem] text-ink-3">{{ item.detail }}</p>
                </div>
              </li>
            }
          </ul>
          @if (access.readOnly) {
            <app-read-only-note class="mt-4 block" />
          } @else if (phoneDemo(); as demo) {
            @if (demo.can_arm || demo.state === 'armed') {
              <div class="mt-4">
                <div class="flex flex-wrap items-center gap-x-3 gap-y-2">
                  <app-phone-demo-control />
                </div>
                <p class="mt-2 text-[1rem] text-ink-3">
                  Arms only the next real call to the FirstRing number, for 10 minutes. Your phone
                  rings with a private summary; press 1 to connect.
                </p>
              </div>
            }
          }
          <p class="mt-3 flex items-start gap-2 text-[1rem] text-muted">
            <svg lucideLockKeyhole [size]="16" class="mt-1 shrink-0" aria-hidden="true"></svg>
            The enrolled number stays on the server and is never shown here. The private summary
            stays in this household’s dashboard.
          </p>
          @if (lastHandoff(); as call) {
            <a
              class="btn btn-quiet mt-4 no-underline"
              [routerLink]="['/console/calls', call.id]"
              queryParamsHandling="preserve"
            >
              See the latest permission call’s receipts
            </a>
          }
        </div>
      </div>
    </section>
  `,
})
export class HandoffSetup {
  readonly access = inject(Access);
  private readonly store = inject(ConsoleStore);
  readonly name = computed(() => this.store.state()?.profile?.display_name ?? 'Sarah');
  readonly steps = computed(() => [
    'A caller asks to speak with ' + this.name() + '.',
    'FirstRing checks what the caller wants. A request for a code, payment or device access is never passed on.',
    this.name() + '’s phone rings. The caller stays on hold with FirstRing.',
    this.name() + ' hears a private summary. The caller cannot hear it.',
    'Press 1 to talk with the caller, or 2 to decline.',
  ]);
  private readonly ok = (id: string) =>
    ['configured', 'verified'].includes(this.store.capability(id)?.status ?? '');
  readonly armed = computed(() => !this.store.isDemo() && this.ok('handoff'));
  readonly readiness = computed(() => {
    const state = this.store.state();
    if (!state) return [];
    if (this.store.isDemo())
      return [
        {
          label: 'Demo view',
          ok: false,
          detail:
            'No phone line. The presentation’s “Ordinary call, connected” rehearsal shows the steps with fictional receipts.',
        },
      ];
    const phoneB = state.phone_b;
    const demo = state.phone_demo;
    const enrolled = phoneB?.enrolled ?? demo?.recipient_enrolled;
    const outbound = this.ok('outbound') || !!demo?.outbound_configured;
    return [
      {
        label: `${this.name()}’s phone enrolled`,
        ok: !!enrolled,
        detail:
          enrolled === undefined
            ? 'The server does not report enrollment.'
            : enrolled
              ? 'A consenting endpoint is enrolled on the server.'
              : 'No phone is enrolled for this household on the server.',
      },
      {
        label: 'Outbound calling',
        ok: outbound,
        detail: outbound
          ? 'Configured. Configured is not the same as tested.'
          : 'Not configured on the server.',
      },
      {
        label: 'Ring my phone on the next call',
        ok: this.phoneArmed() || (!demo && this.ok('handoff')),
        detail: this.phoneDetail(),
      },
      {
        label: 'Live calls',
        ok: state.execution_mode === 'live',
        detail:
          state.execution_mode === 'live'
            ? 'The server is in live mode.'
            : 'Replay mode cannot place calls.',
      },
    ];
  });
  readonly phoneDemo = computed(() =>
    this.store.isDemo() ? undefined : this.store.state()?.phone_demo,
  );
  readonly phoneArmed = computed(() => {
    const demo = this.phoneDemo();
    if (demo?.state !== 'armed') return false;
    return !demo.expires_at || Date.parse(demo.expires_at) > this.store.now();
  });
  /** Plain words for the server's phone_demo status. */
  readonly phoneDetail = computed(() => {
    const demo = this.phoneDemo();
    const handoff = this.store.capability('handoff');
    if (!demo) return handoff ? handoff.detail : 'Not reported by this server.';
    const zone = this.store.state()?.profile?.timezone;
    const until = demo.expires_at ? formatTime(demo.expires_at, zone) : '';
    if (this.phoneArmed())
      return `Armed${until ? ` until ${until}` : ''}. The next real call can ring your phone.`;
    switch (demo.state) {
      case 'reserved':
        return 'Used: a call is ringing, or has rung, your phone.';
      case 'armed':
      case 'expired':
        return 'The last arm expired before a call came in.';
      case 'disarmed':
        return 'Disarmed. The next call will not ring your phone.';
    }
    if (demo.can_arm) return 'Not armed. Arm it below for the next call.';
    if (demo.recipient_enrolled === false) return 'Not available: no phone is enrolled.';
    if (demo.outbound_configured === false) return 'Not available: outbound calling is off.';
    if (demo.remaining_attempts === 0) return 'Not available: no test calls remain.';
    return 'Not available on this server right now.';
  });
  readonly lastHandoff = computed(() => this.store.calls().find((c) => !!handoffView(c)));
}
