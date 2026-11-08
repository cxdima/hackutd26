import type { Request } from '@playwright/test';
import { event, expect, hold, liveState, phoneDemo, sse, test } from './support/fixtures';
import { ConsolePage } from './support/pages';

/** A short, silent WAV file built in the test: nothing real is recorded or uploaded. */
function silentWav(seconds: number) {
  const rate = 8000;
  const samples = rate * seconds;
  const buffer = Buffer.alloc(44 + samples * 2);
  buffer.write('RIFF', 0);
  buffer.writeUInt32LE(36 + samples * 2, 4);
  buffer.write('WAVEfmt ', 8);
  buffer.writeUInt32LE(16, 16);
  buffer.writeUInt16LE(1, 20);
  buffer.writeUInt16LE(1, 22);
  buffer.writeUInt32LE(rate, 24);
  buffer.writeUInt32LE(rate * 2, 28);
  buffer.writeUInt16LE(2, 32);
  buffer.writeUInt16LE(16, 34);
  buffer.write('data', 36);
  buffer.writeUInt32LE(samples * 2, 40);
  return buffer;
}

test.describe('phone handoff and voice setup', () => {
  test('handoff setup explains the flow and places no call', async ({ page }) => {
    const writes: Request[] = [];
    page.on('request', (r) => r.method() !== 'GET' && writes.push(r));
    await page.goto('/console/settings?view=demo');
    const section = page.getByRole('region', { name: 'Private permission calls' });
    await expect(section.getByText('Nothing on this page places a call.')).toBeVisible();
    await expect(section.getByRole('button', { name: /Ring my phone/ })).toHaveCount(0);
    await expect(section.getByRole('list', { name: 'What happens on a call' })).toContainText(
      'Press 1 to talk with the caller, or 2 to decline.',
    );
    expect(writes).toHaveLength(0);
  });

  test('live server: missing enrollment and an unreported phone demo are stated plainly', async ({
    page,
  }) => {
    await page.route('**/api/state', (route) =>
      route.fulfill({
        json: liveState({
          capabilities: [
            { id: 'inbound', status: 'configured', detail: 'ok' },
            { id: 'outbound', status: 'configured', detail: 'ok' },
            {
              id: 'voice_clone',
              status: 'disabled',
              detail: 'Preset assistant voice; no clone endpoint verified',
            },
          ],
          phone_b: {
            enrolled: false,
            enabled: true,
            authorized: false,
            remaining_attempts: 1,
            can_request: false,
            reason: 'Designate a separate consenting Sarah / Phone B endpoint.',
          },
        }),
      }),
    );
    await page.route('**/api/profile/sarah', (route) =>
      route.fulfill({ json: liveState().profile }),
    );
    await page.route('**/api/events', (route) => hold(route));
    await page.goto('/console/settings');
    await new ConsolePage(page).connected();
    const section = page.getByRole('region', { name: 'Private permission calls' });
    await expect(
      section.getByText('No phone is enrolled for this household on the server.'),
    ).toBeVisible();
    await expect(section.getByText('Not reported by this server.')).toBeVisible();
  });
});

test.describe('ring my phone on the next call (mocked server)', () => {
  test('settings shows the real status and arms with the exact body', async ({ page }) => {
    let status = phoneDemo();
    const bodies: (string | null)[] = [];
    await page.route('**/api/state', (route) =>
      route.fulfill({ json: liveState({ phone_demo: status }) }),
    );
    await page.route('**/api/profile/sarah', (route) =>
      route.fulfill({ json: liveState().profile }),
    );
    await page.route('**/api/events', (route) => hold(route));
    await page.route('**/api/demo/phone-demo/arm', (route) => {
      bodies.push(route.request().postData());
      status = phoneDemo({
        state: 'armed',
        arm_id: 'arm-9',
        expires_at: new Date(Date.now() + 600_000).toISOString(),
        can_arm: false,
        can_disarm: true,
      });
      return route.fulfill({ json: status });
    });
    await page.goto('/console/settings');
    await new ConsolePage(page).connected();
    const section = page.getByRole('region', { name: 'Private permission calls' });
    const readiness = section.getByRole('list', { name: 'Handoff readiness' });
    await expect(readiness).toContainText('Not armed. Arm it below for the next call.');
    expect(bodies).toEqual([]);
    await section.getByRole('button', { name: 'Ring my phone on the next call' }).click();
    await expect(section.getByText(/^Ready to ring your phone until/)).toBeVisible();
    expect(bodies).toEqual(['{"contact_id":"sarah","ttl_seconds":600}']);
    await expect(readiness).toContainText('The next real call can ring your phone.');
    await expect(readiness.getByText('Ring my phone on the next call: ready')).toBeAttached();
  });
});

test.describe('call evidence on a mocked live server', () => {
  test('a rejected proposal is shown as not done, never as a saved message', async ({ page }) => {
    const session = {
      ...liveState().sessions[0],
      blocked: false,
      assessments: [],
      state: 'ended',
      inbound_ended: true,
      evidence_quote: null,
    };
    await page.route('**/api/state', (route) =>
      route.fulfill({ json: liveState({ sessions: [session] }) }),
    );
    await page.route('**/api/events', (route) =>
      route.fulfill({
        status: 200,
        headers: { 'Content-Type': 'text/event-stream' },
        body: sse([
          event('call.inbound.received', { caller_identity: 'unverified' }),
          event('transcript.final', { text: 'Tell Sarah the plumber is late.', turn_id: 't1' }),
          event('agent.tool.proposed', { name: 'take_message', generation_id: 3, arguments: {} }),
          event('agent.tool.result', {
            name: 'take_message',
            generation_id: 3,
            ok: false,
            result: { completed: false, check_action_receipts: true },
          }),
          event('policy.denied', { tool: 'take_message', detail: 'Summary needs caller evidence' }),
          event('call.ended', {}),
        ]),
      }),
    );
    await page.route('**/api/session/**', (route) =>
      route.fulfill({ status: 404, json: { error: 'Not found' } }),
    );
    await page.goto('/console/calls/live-1');
    await new ConsolePage(page).connected();
    await expect(
      page.getByText('Not done: Save a private message. Rust rejected this proposal.'),
    ).toBeVisible();
    await expect(page.locator('app-call-detail app-outcome-chip')).not.toContainText(
      'Message saved',
    );
    await page.getByText('Technical details').click();
    await expect(page.getByText('Rejected by Rust')).toBeVisible();
    await expect(page.getByText('Summary needs caller evidence').first()).toBeVisible();
  });
});

test.describe('reported-number history on a mocked live server', () => {
  for (const configured of [false, true])
    test(`community page says ${configured ? 'provided' : 'not provided'} from the server capability`, async ({
      page,
    }) => {
      const requests: string[] = [];
      page.on('request', (r) => requests.push(new URL(r.url()).pathname));
      const capabilities = [
        { id: 'inbound', status: 'configured', detail: 'ok' },
        ...(configured
          ? [{ id: 'reputation', status: 'configured', detail: 'Scoped reports' }]
          : []),
      ];
      await page.route('**/api/state', (route) =>
        route.fulfill({ json: liveState({ capabilities }) }),
      );
      await page.route('**/api/events', (route) => hold(route));
      await page.route('**/api/session/live-1/reputation', (route) =>
        route.fulfill({
          json: {
            version: 2,
            scope: 'demo',
            number_label: 'Presented number ending 34',
            band: 'reported',
            band_label: 'Demo reported; simulated profiles, no verified independence',
            explanation:
              'Demo profile reports simulate independent households; verified independence is zero.',
            independent_households: 0,
            verified_independent_households: 0,
            demo_household_count: 1,
            reports: 1,
            first_reported_at: '2026-11-08T13:00:00Z',
            last_reported_at: '2026-11-08T13:00:00Z',
            expires_at: '2026-11-22T13:00:00Z',
            correction: { state: 'none' },
            related_tactic_ids: [],
            identity_verified: false,
          },
        }),
      );
      await page.goto('/console/community');
      await new ConsolePage(page).connected();
      const section = page.getByRole('region', { name: 'Reported-number history' });
      await expect(
        section.getByText(
          configured ? 'The server provides it.' : 'This server does not provide it yet',
        ),
      ).toBeVisible();
      await page.goto('/console/calls/live-1');
      await new ConsolePage(page).connected();
      const panel = page.locator('app-reputation-panel');
      if (configured) {
        await expect(panel).toContainText('Presented number ending 34');
        await expect(panel).toContainText('Demo reported; simulated profiles');
        await expect(panel).toContainText('Verified independent households');
        await expect(panel).not.toContainText('safe');
      } else {
        await expect(panel).toContainText('Not connected.');
        expect(requests.filter((p) => p.endsWith('/reputation'))).toHaveLength(0);
      }
    });
});

test.describe('owner voice and call recordings (mocked live server)', () => {
  test('voice setup arms a recording call with consent, then offers to use the voice', async ({
    page,
  }) => {
    let state = 'none';
    const posts: { url: string; body: unknown }[] = [];
    const status = () => ({
      household_id: 'sarah',
      state,
      speaker_name: state === 'none' ? null : 'Dima',
      expires_at: '2026-11-08T16:10:00Z',
      voice_ready: state === 'ready' || state === 'active',
      active: state === 'active',
      passage: 'Hi, this is my voice for FirstRing.',
      provider_configured: true,
    });
    await page.route('**/api/state', (route) =>
      route.fulfill({ json: liveState({ phone_display: '+15555550134' }) }),
    );
    await page.route('**/api/events', (route) => hold(route));
    await page.route('**/api/profile/sarah', (route) =>
      route.fulfill({ json: liveState().profile }),
    );
    await page.route('**/api/voice/clone**', (route) => {
      const request = route.request();
      if (request.method() === 'POST') {
        posts.push({ url: new URL(request.url()).pathname, body: request.postDataJSON() });
        state = request.url().endsWith('/arm') ? 'armed' : 'active';
      }
      return route.fulfill({ json: status() });
    });
    await page.goto('/console/settings');
    const voice = page.getByRole('region', { name: 'Your own voice' });
    const start = voice.getByRole('button', { name: 'Record my voice on a call' });
    await expect(start).toBeDisabled();
    for (const box of await voice.getByRole('checkbox').all()) await box.check();
    await voice.getByLabel('Your name').fill('Dima');
    await start.click();
    await expect(voice.getByText('Hi, this is my voice for FirstRing.')).toBeVisible();
    await expect(voice.getByText('+1 (555) 555-0134')).toBeVisible();
    expect(posts[0]).toEqual({
      url: '/api/voice/clone/arm',
      body: { speaker_name: 'Dima', consent: true },
    });
    state = 'ready';
    await expect(voice.getByRole('button', { name: 'Use my voice on calls' })).toBeVisible({
      timeout: 8000,
    });
    await voice.getByRole('button', { name: 'Use my voice on calls' }).click();
    await expect(voice.getByText('FirstRing answers calls in your voice.')).toBeVisible();
  });

  test('a recorded call can be played back from its call page', async ({ page }) => {
    await page.route('**/api/state', (route) => route.fulfill({ json: liveState() }));
    await page.route('**/api/events', (route) => hold(route));
    await page.route('**/api/session/live-1/recording/info', (route) =>
      route.fulfill({
        json: { available: true, recording: { duration_seconds: 42, channels: 2 } },
      }),
    );
    let audioRequested = false;
    await page.route('**/api/session/live-1/recording', (route) => {
      audioRequested = true;
      return route.fulfill({
        body: Buffer.from([0xff, 0xfb, 0x90, 0x00]),
        contentType: 'audio/mpeg',
      });
    });
    await page.route('**/api/session/live-1/events**', (route) =>
      route.fulfill({ status: 404, json: { error: 'Not found' } }),
    );
    await page.goto('/console/calls/live-1');
    const listen = page.getByRole('button', { name: /Listen to the call/ });
    await expect(listen).toBeVisible();
    expect(audioRequested).toBe(false);
    await listen.click();
    await expect(page.locator('app-call-recording audio')).toBeVisible();
    expect(audioRequested).toBe(true);
  });
});
