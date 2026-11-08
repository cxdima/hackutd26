import type { Request } from '@playwright/test';
import { event, expect, hold, liveState, sse, test, TOKEN } from './support/fixtures';
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
    await expect(section.getByText('Not armed')).toBeVisible();
    await expect(section.getByRole('list', { name: 'What happens on a call' })).toContainText(
      'Press 1 to talk with the caller, or 2 to decline.',
    );
    expect(writes).toHaveLength(0);
  });

  test('voice enrollment runs only as a labeled in-browser example', async ({ page }) => {
    const writes: Request[] = [];
    page.on('request', (r) => r.method() !== 'GET' && writes.push(r));
    await page.goto('/console/settings?view=demo');
    const voice = page.getByRole('region', { name: 'Your own voice (optional)' });
    await expect(voice.getByText('Voice cloning is optional.')).toBeVisible();
    await voice.getByRole('button', { name: 'See how enrollment would work (example)' }).click();
    await expect(voice.getByText('Example · nothing is sent')).toBeVisible();
    await voice.getByLabel('Use my own voice').check();
    const agree = voice.getByRole('button', { name: 'I agree' });
    await expect(agree).toBeDisabled();
    for (const box of await voice.getByRole('checkbox').all()) await box.check();
    await voice.getByLabel('Your name, as the person whose voice this is').fill('Sarah Example');
    await agree.click();
    await voice.getByTestId('voice-file').setInputFiles({
      name: 'too-short.wav',
      mimeType: 'audio/wav',
      buffer: silentWav(5),
    });
    await expect(voice.getByText(/Record at least 20 seconds/)).toBeVisible();
    await voice.getByTestId('voice-file').setInputFiles({
      name: 'sample.wav',
      mimeType: 'audio/wav',
      buffer: silentWav(25),
    });
    await expect(voice.getByText('Sample length 25 seconds. Ready.')).toBeVisible();
    await voice.getByRole('button', { name: 'Create the voice (example)' }).click();
    await expect(voice.getByText(/Ready in the example/)).toBeVisible({ timeout: 6000 });
    await expect(voice.getByRole('button', { name: 'Preview the voice' })).toBeDisabled();
    await voice.getByRole('button', { name: /Use it for Sarah’s calls/ }).click();
    await expect(voice.getByText(/The real profile was not changed/)).toBeVisible();
    await voice.getByRole('button', { name: 'Turn off and delete' }).click();
    await expect(voice.getByText(/Turned off. The sample was deleted/)).toBeVisible();
    expect(writes).toHaveLength(0);
  });

  test('live server: unreported handoff arming and disabled cloning are stated plainly', async ({
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
    await new ConsolePage(page).connect(TOKEN);
    const section = page.getByRole('region', { name: 'Private permission calls' });
    await expect(
      section.getByText('No phone is enrolled for this household on the server.'),
    ).toBeVisible();
    await expect(section.getByText(/does not report it yet/)).toBeVisible();
    const voice = page.getByRole('region', { name: 'Your own voice (optional)' });
    await expect(voice.getByText('Not available on this server')).toBeVisible();
    await expect(voice.getByText(/Voice cloning is off/)).toBeVisible();
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
    await new ConsolePage(page).connect(TOKEN);
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
    test(`community page says ${configured ? 'Connected' : 'Not connected'} from the server capability`, async ({
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
      await new ConsolePage(page).connect(TOKEN);
      const section = page.getByRole('region', { name: 'Reported-number history' });
      await expect(section.getByText(configured ? 'Connected' : 'Not connected')).toBeVisible();
      await page.goto('/console/calls/live-1');
      await new ConsolePage(page).connect(TOKEN);
      const panel = page.locator('app-reputation-panel');
      if (configured) {
        await expect(panel).toContainText('Presented number ending 34');
        await expect(panel).toContainText('Demo-scoped reports');
        await expect(panel).toContainText('Verified independent households');
        await expect(panel).not.toContainText('safe');
      } else {
        await expect(panel).toContainText('Not connected.');
        expect(requests.filter((p) => p.endsWith('/reputation'))).toHaveLength(0);
      }
    });
});
