import { test as base, expect, type Page, type Route } from '@playwright/test';

let seq = 0;

export function event(
  type: string,
  payload: Record<string, unknown>,
  sessionId: string | null = 'live-1',
) {
  seq += 1;
  const at = new Date(Date.now() - 60_000 + seq * 500).toISOString();
  return {
    id: `e2e-${seq}`,
    sequence: seq,
    session_id: sessionId,
    leg_role: type.startsWith('transcript') ? 'caller' : 'system',
    occurred_at: at,
    received_at: at,
    policy_epoch: 0,
    execution_mode: 'live',
    data_origin: 'synthetic',
    type,
    payload,
  };
}

export const profile = {
  household_id: 'sarah',
  version: 4,
  display_name: 'Sarah',
  greeting: 'Hi, thanks for calling. What’s the call about?',
  tts_provider: 'ElevenLabs',
  voice_id: 'cgSgspJ2msm6clMCkdW9-v4',
  locale: 'en-US',
  timezone: 'America/Chicago',
};

const risky = {
  speech_act: 'request',
  requested_object: 'authentication_secret',
  direction: 'caller_requests_from_sarah',
};

export function liveState(overrides: Record<string, unknown> = {}) {
  return {
    execution_mode: 'live',
    data_origin: 'synthetic',
    transport: 'relay',
    phone_display: 'Test line',
    selected_household: 'sarah',
    profile,
    capabilities: [
      { id: 'inbound', status: 'configured', detail: 'Local signing inputs present' },
      { id: 'voice_clone', status: 'disabled', detail: 'Preset assistant voice' },
    ],
    evidence: {
      model_protocol: {
        status: 'passed',
        detail: 'Recorded native tool cycle',
        recorded_at: '2026-11-08T06:00:00Z',
      },
      inbound_phone: {
        status: 'passed',
        detail: 'Signed inbound probe',
        recorded_at: '2026-11-08T02:42:49Z',
      },
      relay_phone: { status: 'pending', detail: 'Not yet recorded' },
    },
    sessions: [
      {
        id: 'live-1',
        household_id: 'sarah',
        profile,
        state: 'screening',
        policy_epoch: 1,
        generation_id: 2,
        speech_pending: false,
        inbound_ended: false,
        current_action: 'Closing the call',
        evidence_quote: 'read me the code',
        turn_version: 1,
        assessed_version: 1,
        blocked: true,
        assessments: [
          {
            turn_version: 1,
            generation_id: 2,
            observation: risky,
            evidence_refs: [{ turn_id: 't1', quote: 'read me the code' }],
          },
        ],
        message: null,
        turns: [{ id: 't1', speaker: 'caller', text: 'Please read me the code we just texted.' }],
        offer: null,
        legs: [],
        execution_mode: 'live',
        data_origin: 'synthetic',
      },
    ],
    community: [
      { household_id: 'sarah', display_name: 'Sarah', last_applied_version: 1 },
      { household_id: 'maya', display_name: 'Maya', last_applied_version: 1 },
    ],
    tactics: [],
    ...overrides,
  };
}

/** The server's "ring my phone on the next call" status. */
export function phoneDemo(overrides: Record<string, unknown> = {}) {
  return {
    state: 'unarmed',
    arm_id: null,
    session_id: null,
    household_id: null,
    contact_id: 'sarah',
    expires_at: null,
    action_id: null,
    remaining_attempts: 3,
    recipient_enrolled: true,
    outbound_configured: true,
    can_arm: true,
    can_disarm: false,
    scope: 'next_inbound_call',
    probe_budget_unchanged: true,
    private_summary_provider: 'ElevenLabs',
    private_summary_voice: 'Jessica',
    private_summary_source: 'configured',
    ...overrides,
  };
}

export function liveEvents() {
  return [
    event('call.inbound.received', {
      caller_identity: 'unverified',
      transport: 'relay',
      requested_profile: profile,
    }),
    event('transcript.final', { text: 'Please read me the code we just texted.', turn_id: 't1' }),
    event('model.request.completed', {
      generation_id: 2,
      turn_version: 1,
      boundary: 'model_http',
      heard: false,
      observation: {
        elapsed_ms: 1190,
        outcome: 'success',
        completion_tokens: 210,
        response_model: 'nvidia/nemotron-3.5-lightning-30b-a3b',
        backend: 'brev_hosted',
      },
    }),
    event('agent.tool.proposed', {
      name: 'record_assessment',
      generation_id: 2,
      turn_version: 1,
      arguments: { observation: risky },
    }),
    event('policy.denied', {
      operation: 'assessment',
      observation: risky,
      quote: 'read me the code',
      handoff_allowed: false,
    }),
    event('agent.tool.result', {
      name: 'record_assessment',
      generation_id: 2,
      turn_version: 1,
      ok: true,
      result: { blocked: true },
    }),
  ];
}

export function sse(events: unknown[]) {
  return events
    .map((e) => `id: ${(e as { sequence: number }).sequence}\ndata: ${JSON.stringify(e)}\n\n`)
    .join('');
}

/** Blocks every /api request unless a test installs a handler. */
export const test = base.extend<{ apiGuard: void }>({
  apiGuard: [
    async ({ page }, use) => {
      // The console connects to the local server by itself; with /api aborted it reports
      // that the server is not running. Tests that need a server mock /api/state.
      await page.route('**/api/**', (route) => route.abort('blockedbyclient'));
      await use();
    },
    { auto: true },
  ],
});
export { expect };

export async function noHorizontalScroll(page: Page) {
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
  expect(overflow).toBeLessThanOrEqual(0);
}

/** Holds a request open (a quiet but healthy event stream) until the page closes. */
export function hold(route: Route): Promise<void> {
  return new Promise(() => void route);
}
