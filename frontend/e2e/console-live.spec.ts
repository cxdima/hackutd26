import { expect, hold, liveEvents, liveState, profile, sse, test } from './support/fixtures';
import { ConsolePage } from './support/pages';

/** All responses below are explicit synthetic fixtures; no live server is contacted. */
test.describe('connected dashboard (mocked server)', () => {
  test('an unreachable server is reported plainly, with a retry and the demo on offer', async ({
    page,
  }) => {
    // Every /api request is aborted by the fixture: the server is "not running".
    await page.goto('/console');
    await expect(
      page.getByRole('heading', { name: 'The FirstRing server isn’t running on this computer.' }),
    ).toBeVisible();
    await expect(page.getByRole('button', { name: 'Try again' })).toBeVisible();
    await expect(page.getByRole('navigation', { name: 'Dashboard' })).toHaveCount(0);
    await page.getByRole('button', { name: 'Explore the demo' }).click();
    await expect(page.getByRole('button', { name: 'Exit demo' })).toBeVisible();
    await expect(page).toHaveURL(/view=demo/);
  });

  test('connects by itself with no token in the page and renders live receipts', async ({
    page,
  }) => {
    const headers: (string | undefined)[] = [];
    // Stands in for the local server, which trusts its own computer.
    await page.route('**/api/state', (route) => {
      headers.push(route.request().headers()['authorization']);
      return route.fulfill({ json: liveState() });
    });
    await page.route('**/api/events', (route) => hold(route));
    await page.goto('/console');
    const console = new ConsolePage(page);
    await console.connected();
    await expect(page.getByRole('button', { name: 'Exit demo' })).toHaveCount(0);
    expect(headers.length).toBeGreaterThan(0);
    expect(headers.every((h) => h === undefined)).toBe(true);
    const storage = await page.evaluate(
      () => localStorage.length + sessionStorage.length + document.cookie.length,
    );
    expect(storage).toBe(0);
    await expect(page.getByText('Suspicious request blocked').first()).toBeVisible();
  });

  test('a dropped event stream reads as reconnecting while the snapshot stays visible', async ({
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
    await console.connected();
    await expect(page.getByRole('banner').getByText('Reconnecting…')).toBeVisible();
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
      if (saves === 1) {
        // The custom timezone dropdown feeds the same form value as the old native select.
        expect(body.timezone).toBe('America/New_York');
        return route.fulfill({ json: { ...profile, ...body, version: profile.version + 1 } });
      }
      return route.fulfill({ status: 409, json: { error: 'rejected' } });
    });
    await page.goto('/console');
    const console = new ConsolePage(page);
    await console.connected();
    await console.nav('Settings').click();
    await page.getByLabel('Opening greeting').fill('Hello, this is FirstRing answering for Sarah.');
    await expect(page.getByLabel('Timezone')).toHaveText(/Central time/);
    await console.choose('Timezone', /Eastern time/);
    await expect(page.getByLabel('Timezone')).toHaveText(/Eastern time/);
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
    await console.connected();
    await console.nav('Calls').click();
    await page.locator('app-call-row a').first().click();
    await page.getByText('Technical details').click();
    await page.getByRole('button', { name: 'End session…' }).click();
    expect(posts).toEqual([]);
    await page.getByRole('button', { name: 'Confirm: end session' }).click();
    await expect.poll(() => posts).toEqual([{ session_id: 'live-1' }]);
    await expect(page.getByRole('button', { name: 'Call Phone B' })).toBeDisabled();
  });
});
