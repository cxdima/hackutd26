// Sanitized walkthrough video. Every scene is fixture data and carries an on-screen label;
// the dashboard scene uses a mocked server with synthetic receipts. Usage:
//   node scripts/walkthrough.mjs [outDir] [baseUrl]   (then compress the .webm with ffmpeg)
import { chromium } from '@playwright/test';
const dir = process.argv[2] ?? 'review/walkthrough';
const base = process.argv[3] ?? 'http://127.0.0.1:4310';
const b = await chromium.launch();
const ctx = await b.newContext({
  viewport: { width: 1280, height: 760 },
  recordVideo: { dir, size: { width: 1280, height: 760 } },
});
const p = await ctx.newPage();
const pause = (ms) => p.waitForTimeout(ms);
const label = (text) =>
  p.evaluate((t) => {
    let el = document.getElementById('walk-label');
    if (!el) {
      el = document.createElement('div');
      el.id = 'walk-label';
      document.body.appendChild(el);
    }
    el.textContent = t;
    Object.assign(el.style, {
      position: 'fixed',
      left: '50%',
      bottom: '18px',
      transform: 'translateX(-50%)',
      zIndex: 99999,
      background: '#f7f7f2',
      color: '#171c18',
      font: '800 15px Manrope Variable, system-ui',
      letterSpacing: '0.04em',
      textTransform: 'uppercase',
      padding: '8px 14px',
      borderRadius: '8px',
      boxShadow: '0 0 0 2px #171c18',
      pointerEvents: 'none',
    });
  }, text);
// Synthetic, mocked server for the dashboard scene only.
const now = Date.now();
const iso = (s) => new Date(now - 90_000 + s * 1000).toISOString();
const profile = {
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
let seq = 0;
const ev = (type, payload, s) => ({
  id: `w${++seq}`,
  sequence: seq,
  session_id: 'mock-1',
  leg_role: 'system',
  occurred_at: iso(s),
  received_at: iso(s),
  policy_epoch: 0,
  execution_mode: 'live',
  data_origin: 'synthetic',
  type,
  payload,
});
const events = [
  ev(
    'call.inbound.received',
    { caller_identity: 'unverified', transport: 'relay', requested_profile: profile },
    0,
  ),
  ev(
    'transcript.final',
    {
      text: 'This is the bank fraud team. Please read me the code we just texted to Sarah.',
      turn_id: 't1',
    },
    6,
  ),
  ev(
    'model.request.completed',
    {
      generation_id: 2,
      turn_version: 1,
      boundary: 'model_http',
      heard: false,
      observation: {
        elapsed_ms: 1140,
        outcome: 'success',
        completion_tokens: 198,
        response_model: 'nvidia/nemotron-3.5-lightning-30b-a3b',
        backend: 'brev_hosted',
      },
    },
    7,
  ),
  ev(
    'agent.tool.proposed',
    {
      name: 'record_assessment',
      generation_id: 2,
      turn_version: 1,
      arguments: { observation: risky },
    },
    7.1,
  ),
  ev(
    'policy.denied',
    {
      operation: 'assessment',
      observation: risky,
      quote: 'read me the code we just texted',
      handoff_allowed: false,
    },
    7.2,
  ),
  ev(
    'agent.tool.result',
    {
      name: 'record_assessment',
      generation_id: 2,
      turn_version: 1,
      ok: true,
      result: { blocked: true },
    },
    7.3,
  ),
  ev(
    'audio.requested',
    { text: 'I can’t help with codes. I’ll let Sarah know you called.', heard: false },
    7.6,
  ),
  ev('call.ended', {}, 14),
];
const state = {
  execution_mode: 'live',
  data_origin: 'synthetic',
  transport: 'relay',
  phone_display: '+15555550134',
  selected_household: 'sarah',
  profile,
  capabilities: [
    { id: 'inbound', status: 'configured', detail: 'Mocked' },
    { id: 'voice_clone', status: 'disabled', detail: 'Preset assistant voice' },
  ],
  sessions: [
    {
      id: 'mock-1',
      household_id: 'sarah',
      profile,
      state: 'protected',
      policy_epoch: 1,
      generation_id: 2,
      speech_pending: false,
      inbound_ended: true,
      current_action: 'Call ended',
      evidence_quote: 'read me the code we just texted',
      turn_version: 1,
      assessed_version: 1,
      blocked: true,
      assessments: [
        {
          turn_version: 1,
          generation_id: 2,
          observation: risky,
          evidence_refs: [{ turn_id: 't1', quote: 'read me the code we just texted' }],
        },
      ],
      message: null,
      turns: [
        {
          id: 't1',
          speaker: 'caller',
          text: 'This is the bank fraud team. Please read me the code we just texted to Sarah.',
        },
        {
          id: 'a1',
          speaker: 'assistant',
          text: 'I can’t help with codes. I’ll let Sarah know you called.',
        },
      ],
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
};
await p.route('**/api/**', (route) => {
  const path = new URL(route.request().url()).pathname;
  if (route.request().method() !== 'GET') return route.abort();
  if (path === '/api/state') return route.fulfill({ json: state });
  if (path === '/api/events') return new Promise(() => void route); // a quiet, open stream
  if (path === '/api/session/mock-1/events')
    return route.fulfill({
      json: { session_id: 'mock-1', events, next_after: events.at(-1).sequence, has_more: false },
    });
  if (path === '/api/profile/sarah') return route.fulfill({ json: profile });
  return route.fulfill({ status: 404, json: { error: 'Not found' } });
});
await p.goto(`${base}/`);
await p.evaluate(() => document.fonts.ready);
await label('Landing page');
await pause(3500);
await p
  .locator('#today')
  .evaluate((el) => el.scrollIntoView({ behavior: 'smooth', block: 'start' }));
await pause(3500);
await p.goto(`${base}/console`);
await label('Dashboard · mocked server, synthetic receipts — not a real call');
await p.getByLabel('Operator token').fill('fixture-token');
await p.getByRole('button', { name: 'Connect' }).click();
await pause(700);
await label('Dashboard · mocked server, synthetic receipts — not a real call');
await pause(2000);
await p.getByRole('link', { name: /See what FirstRing heard and why/ }).click();
await pause(2600);
await p.mouse.wheel(0, 900);
await pause(2400);
// Presentation rehearsal (DEMO, in-browser only)
await p.goto(`${base}/console/present?view=demo`);
await label('Presentation view · fictional rehearsal');
await pause(2500);
await p.getByRole('button', { name: 'The request changes' }).click();
await pause(22500);
await p.getByRole('button', { name: 'Evidence' }).click();
await pause(3000);
await p.keyboard.press('Escape');
await pause(800);
await p.getByRole('button', { name: 'Maya was ready' }).click();
await pause(1500);
await p.getByRole('button', { name: 'Skip to end' }).click();
await pause(2500);
await p.getByRole('button', { name: 'Community', exact: true }).click();
await label('Community protection · fictional rehearsal');
await pause(4500);
await p.mouse.wheel(0, 600);
await pause(2000);
await p.goto(`${base}/console/settings?view=demo`);
await label('Phone and voice setup · demo view, nothing is sent');
await p
  .getByRole('region', { name: 'Private permission calls' })
  .evaluate((el) => el.scrollIntoView({ behavior: 'smooth', block: 'start' }));
await pause(3500);
await p.getByRole('button', { name: 'See how enrollment would work (example)' }).click();
await p.getByLabel('Use my own voice').check();
await p
  .getByRole('region', { name: 'Your own voice (optional)' })
  .evaluate((el) => el.scrollIntoView({ behavior: 'smooth', block: 'start' }));
await pause(3500);
await ctx.close();
await b.close();
