import { expect, hold, liveEvents, liveState, profile, sse, test, TOKEN } from './support/fixtures';
import { ConsolePage } from './support/pages';

/** All responses below are explicit synthetic fixtures; no live server is contacted. */
test.describe('connected dashboard (mocked server)', () => {
  test('rejected token stays locked with a useful message', async ({ page }) => {
    await page.route('**/api/state', (route) => route.fulfill({ status: 401, body: '' }));
    await page.goto('/console');
    await new ConsolePage(page).connect('nope');
    await expect(page.getByRole('alert')).toContainText('token was not accepted');
    await expect(page.getByLabel('Operator token')).toBeVisible();
  });

  test('unreachable server shows a disconnected state and offers the demo explicitly', async ({
    page,
  }) => {
    await page.goto('/console');
    await new ConsolePage(page).connect(TOKEN);
    await expect(page.getByRole('alert')).toContainText(
      'Check that the FirstRing server is running',
    );
    await expect(page.getByRole('button', { name: 'Explore the demo' })).toBeVisible();
  });

  test('connects with a bearer token kept out of storage and renders receipts as LIVE', async ({
    page,
  }) => {
    const auth: string[] = [];
    await page.route('**/api/state', (route) => {
      auth.push(route.request().headers()['authorization'] ?? '');
      return route.fulfill({ json: liveState() });
    });
    await page.route('**/api/events', (route) => hold(route));
    await page.goto('/console');
    const console = new ConsolePage(page);
    await console.connect(TOKEN);
    await expect(console.bug()).toHaveText(/Live/);
    expect(auth[0]).toBe(`Bearer ${TOKEN}`);
    const storage = await page.evaluate(
      () => localStorage.length + sessionStorage.length + document.cookie.length,
    );
    expect(storage).toBe(0);
    expect(page.url()).not.toContain(TOKEN);
    await expect(page.getByText('Suspicious request blocked').first()).toBeVisible();
  });

  test('a dropped event stream is shown as signal lost while the snapshot stays visible', async ({
    page,
  }) => {
    await page.route('**/api/state', (route) => route.fulfill({ json: liveState() }));
    let streams = 0;
    await page.route('**/api/events', (route) => {
      streams += 1;
      // First stream delivers receipts and closes; reconnects fail, so the console must report the loss.
      if (streams > 1) return route.abort('connectionrefused');
      return route.fulfill({
        status: 200,
        headers: { 'Content-Type': 'text/event-stream' },
        body: sse(liveEvents()),
      });
    });
    await page.goto('/console');
    const console = new ConsolePage(page);
    await console.connect(TOKEN);
    await expect(console.bug()).toHaveText(/Signal lost/);
    await expect(page.getByText('Reconnecting automatically.', { exact: false })).toBeVisible();
    await console.nav('Calls').click();
    await page.locator('app-call-row a').first().click();
    await expect(page.getByText('Nemotron answered in 1.19 s.')).toBeVisible();
    await expect(page.getByText('Rule check: protected request.', { exact: false })).toBeVisible();
  });

  test('profile edits save a new version for new calls, and a rejected save claims nothing', async ({
    page,
  }) => {
    let saves = 0;
    await page.route('**/api/state', (route) => route.fulfill({ json: liveState() }));
    await page.route('**/api/events', (route) => hold(route));
    await page.route('**/api/profile/sarah', async (route) => {
      if (route.request().method() === 'GET') return route.fulfill({ json: profile });
      saves += 1;
      const body = route.request().postDataJSON();
      expect(Object.keys(body).sort()).toEqual([
        'display_name',
        'greeting',
        'locale',
        'timezone',
        'tts_provider',
        'voice_id',
      ]);
      if (saves === 1)
        return route.fulfill({ json: { ...profile, ...body, version: profile.version + 1 } });
      return route.fulfill({ status: 409, json: { error: 'rejected' } });
    });
    await page.goto('/console');
    const console = new ConsolePage(page);
    await console.connect(TOKEN);
    await console.nav('Settings').click();
    await page.getByLabel('Opening greeting').fill('Hello, this is FirstRing answering for Sarah.');
    await page.getByRole('button', { name: 'Save for new calls' }).click();
    await expect(page.getByText('Saved as version 5. New calls will use it.')).toBeVisible();
    await page.getByLabel('Opening greeting').fill('Hello again.');
    await page.getByRole('button', { name: 'Save for new calls' }).click();
    await expect(page.getByRole('alert')).toContainText('Not saved.');
    await expect(page.getByText('Saved as version')).toHaveCount(0);
  });

  test('ending a session needs confirmation and sends only the session id', async ({ page }) => {
    const posts: unknown[] = [];
    await page.route('**/api/state', (route) => route.fulfill({ json: liveState() }));
    await page.route('**/api/events', (route) => hold(route));
    await page.route('**/api/demo/end-call', (route) => {
      posts.push(route.request().postDataJSON());
      return route.fulfill({ json: {} });
    });
    await page.goto('/console');
    const console = new ConsolePage(page);
    await console.connect(TOKEN);
    await console.nav('Calls').click();
    await page.locator('app-call-row a').first().click();
    await page.getByText('Technical details').click();
    await page.getByRole('button', { name: 'End session…' }).click();
    expect(posts).toEqual([]);
    await page.getByRole('button', { name: 'Confirm: end session' }).click();
    await expect.poll(() => posts).toEqual([{ session_id: 'live-1' }]);
    await expect(page.getByText('Request sent.', { exact: false })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Call Phone B' })).toBeDisabled();
  });

  test('connects by itself through the local operator session, holding no token in the page', async ({
    page,
  }) => {
    await page.addInitScript(() => {
      (globalThis as { __FIRSTRING_NO_AUTOCONNECT__?: boolean }).__FIRSTRING_NO_AUTOCONNECT__ =
        false;
    });
    const headers: (string | undefined)[] = [];
    // Stands in for the dev proxy, which adds the token server-side.
    await page.route('**/api/state', (route) => {
      headers.push(route.request().headers()['authorization']);
      return route.fulfill({ json: liveState() });
    });
    await page.route('**/api/events', (route) => hold(route));
    await page.goto('/console');
    const console = new ConsolePage(page);
    await expect(console.bug()).toHaveText(/Live/);
    await expect(page.getByLabel('Operator token')).toHaveCount(0);
    expect(headers.every((h) => h === undefined)).toBe(true);
    const storage = await page.evaluate(
      () => localStorage.length + sessionStorage.length + document.cookie.length,
    );
    expect(storage).toBe(0);
  });

  test('falls back to the token form without an error when no local session exists', async ({
    page,
  }) => {
    await page.addInitScript(() => {
      (globalThis as { __FIRSTRING_NO_AUTOCONNECT__?: boolean }).__FIRSTRING_NO_AUTOCONNECT__ =
        false;
    });
    await page.route('**/api/state', (route) => route.fulfill({ status: 401, body: '' }));
    await page.goto('/console');
    await expect(page.getByLabel('Operator token')).toBeVisible();
    await expect(page.getByRole('alert')).toHaveCount(0);
  });
});
