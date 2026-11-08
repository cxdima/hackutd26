import { Component, computed, inject, input } from '@angular/core';
import type { CallRecord } from '../core/calls';
import { ConsoleStore } from '../core/console-store';
import { formatMs } from '../core/format';
import { modelAvailability, modelEvidence } from '../core/live';

/**
 * NVIDIA's role, separated into what is configured, what was actually
 * requested, how long it took, what the model proposed, what Rust's rules
 * decided, and what was carried out. Readiness is never shown as inference.
 */
@Component({
  selector: 'app-model-evidence',
  template: `
    @let e = evidence();
    <dl class="grid grid-cols-1 gap-x-6 sm:grid-cols-[minmax(11rem,auto)_1fr]">
      <dt class="pt-3 text-[0.95rem] font-bold text-muted">Configured model</dt>
      <dd class="border-b border-paper-3 pb-3 pt-3 sm:pt-3">
        {{ e.configuredLabel ?? 'Not reported by the server' }}
        @if (e.requestedModel) {
          <span class="text-ink-3"> · {{ e.requestedModel }}</span>
        }
        <span class="block text-[0.95rem] text-muted">
          {{
            e.configured ? 'Configured on the server.' : 'Not configured on the server right now.'
          }}
          {{
            e.protocolVerified
              ? 'A recorded tool-call probe passed earlier. That is protocol evidence, not a quality score.'
              : 'No recorded protocol probe for this configuration.'
          }}
        </span>
      </dd>
      <dt class="pt-3 text-[0.95rem] font-bold text-muted">Available now</dt>
      <dd class="border-b border-paper-3 pb-3 pt-3">{{ availability().text }}</dd>
      <dt class="pt-3 text-[0.95rem] font-bold text-muted">Requests on this call</dt>
      <dd class="border-b border-paper-3 pb-3 pt-3">
        @if (e.requests) {
          @if (e.demo) {
            <span class="font-semibold">Authored demo receipts:</span>
          }
          <span class="tabular">{{ e.succeeded }} answered</span>
          @if (e.failed) {
            <span class="tabular"> · {{ e.failed }} failed</span>
          }
          @if (e.backends.length) {
            <span class="text-ink-3"> · {{ e.backends.join(', ') }}</span>
          }
          @if (e.models.length) {
            <span class="block text-[0.95rem] text-muted"
              >Response model: {{ e.models.join(', ') }}</span
            >
          }
        } @else {
          No model request receipt on this call.
        }
      </dd>
      <dt class="pt-3 text-[0.95rem] font-bold text-muted">Response time</dt>
      <dd class="tabular border-b border-paper-3 pb-3 pt-3">
        {{ e.requests ? 'median ' + ms(e.medianMs) + ' · last ' + ms(e.lastMs) : '—' }}
      </dd>
      <dt class="pt-3 text-[0.95rem] font-bold text-muted">Model proposed</dt>
      <dd class="border-b border-paper-3 pb-3 pt-3">
        @if (e.tools.length) {
          <ul class="space-y-1.5">
            @for (tool of e.tools; track $index) {
              <li class="flex flex-wrap items-baseline gap-x-2">
                <span class="font-semibold">{{ tool.label }}</span>
                @switch (e.demo ? 'demo' : tool.executed) {
                  @case ('demo') {
                    <span class="chip chip-none !py-0.5 !text-[0.875rem]">Authored receipt</span>
                  }
                  @case (true) {
                    <span class="chip chip-saved !py-0.5 !text-[0.875rem]"
                      >Carried out by Rust</span
                    >
                  }
                  @case (false) {
                    <span class="chip chip-review !py-0.5 !text-[0.875rem]">Rejected by Rust</span>
                  }
                  @default {
                    <span class="chip chip-none !py-0.5 !text-[0.875rem]">No result receipt</span>
                  }
                }
                @if (tool.rejection) {
                  <span class="w-full text-[0.95rem] text-muted">{{ tool.rejection }}</span>
                }
              </li>
            }
          </ul>
        } @else {
          No proposals.
        }
      </dd>
      <dt class="pt-3 text-[0.95rem] font-bold text-muted">Rust rule decisions</dt>
      <dd class="tabular border-b border-paper-3 pb-3 pt-3">
        {{ e.allowed }} allowed · {{ e.denied }} held or rejected
      </dd>
    </dl>
    <p class="mt-3 text-[0.95rem] text-muted">
      These are reported decisions and their receipts, not the model’s private reasoning. Prompt or
      inference changes are not model training.
    </p>
  `,
})
export class ModelEvidence {
  readonly call = input<CallRecord>();
  private readonly store = inject(ConsoleStore);
  readonly evidence = computed(() => modelEvidence(this.call(), this.store.state()));
  readonly availability = computed(() => modelAvailability(this.store.state()));
  ms(value: number | undefined) {
    return formatMs(value);
  }
}
