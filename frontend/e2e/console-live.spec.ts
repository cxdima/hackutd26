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
    await expect(page.getByText('Reconnecting. The last snapshot stays on screen.')).toBeVisible();
    await console.nav('Calls').click();
    await page.locator('table.data-table tbody tr.row-link a').first().click();
    await page.getByText('Every receipt, in order').click();
    await page.getByText('Technical details').click();
    await expect(page.getByText('Nemotron answered in 1.19 s.').first()).toBeVisible();
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
    await page.locator('table.data-table tbody tr.row-link a').first().click();
    await page.getByText('Technical details').click();
    await page.getByRole('button', { name: 'End session…' }).click();
    expect(posts).toEqual([]);
    await page.getByRole('button', { name: 'Confirm: end session' }).click();
    await expect.poll(() => posts).toEqual([{ session_id: 'live-1' }]);
    await expect(page.getByRole('button', { name: 'Call Phone B' })).toBeDisabled();
  });

  test('trusted callers: add and remove send exact bodies; caller ID warning is shown', async ({
    page,
  }) => {
    const posts: { url: string; body: unknown }[] = [];
    let items = [
      { id: 't1', name: 'Jimmy', relation: 'Son', number_label: 'ending 4471', added_at: null },
    ];
    const list = () => ({
      household_id: 'sarah',
      items,
      connects_to_enrolled_phone: true,
      meaning: 'Calls from these numbers skip screening.',
    });
    await page.route('**/api/state', (route) => route.fulfill({ json: liveState() }));
    await page.route('**/api/events', (route) => hold(route));
    await page.route('**/api/profile/sarah', (route) => route.fulfill({ json: profile }));
    await page.route(
      (url) => url.pathname === '/api/trusted',
      (route) => {
        if (route.request().method() === 'GET') return route.fulfill({ json: list() });
        const body = route.request().postDataJSON();
        posts.push({ url: '/api/trusted', body });
        if (body.name === 'Dup')
          return route.fulfill({ status: 409, json: { error: 'Already listed.' } });
        items = [
          ...items,
          {
            id: 't2',
            name: body.name,
            relation: body.relation,
            number_label: 'ending 0147',
            added_at: null,
          },
        ];
        return route.fulfill({ json: list() });
      },
    );
    await page.route(
      (url) => url.pathname === '/api/trusted/remove',
      (route) => {
        const body = route.request().postDataJSON();
        posts.push({ url: '/api/trusted/remove', body });
        items = items.filter((i) => i.id !== body.id);
        return route.fulfill({ json: list() });
      },
    );
    await page.goto('/console/settings#trusted');
    await new ConsolePage(page).connected();
    await expect(
      page.getByText('Caller ID can be faked. Only add numbers you would answer anyway.'),
    ).toBeVisible();
    const trusted = page.getByRole('list', { name: 'Trusted callers' });
    await expect(trusted).toContainText('Jimmy');
    await page.getByLabel('Name', { exact: true }).fill('Ana');
    await page.getByLabel('Relation').fill('Neighbour');
    await page.getByLabel('Phone number').fill('(214) 555-0147');
    await page.getByRole('button', { name: 'Add to trusted list' }).click();
    await expect(trusted).toContainText('Ana');
    expect(posts[0]).toEqual({
      url: '/api/trusted',
      body: { household_id: 'sarah', name: 'Ana', relation: 'Neighbour', number: '+12145550147' },
    });
    await page.getByLabel('Name', { exact: true }).fill('Dup');
    await page.getByLabel('Phone number').fill('2145550147');
    await page.getByRole('button', { name: 'Add to trusted list' }).click();
    await expect(page.getByRole('alert')).toContainText('Not added. Already listed.');
    await page.getByRole('button', { name: 'Remove Jimmy from the trusted list' }).click();
    expect(posts).toHaveLength(2);
    await page.getByRole('button', { name: 'Remove Jimmy', exact: true }).click();
    await expect(trusted).not.toContainText('Jimmy');
    expect(posts[2]).toEqual({
      url: '/api/trusted/remove',
      body: { household_id: 'sarah', id: 't1' },
    });
  });

  test('read-only public view hides every write control and says where changes are made', async ({
    page,
  }) => {
    const writes: string[] = [];
    page.on('request', (r) => r.method() !== 'GET' && writes.push(r.url()));
    await page.route('**/api/state', (route) => route.fulfill({ json: liveState() }));
    await page.route('**/api/events', (route) => hold(route));
    await page.route('**/api/profile/sarah', (route) => route.fulfill({ json: profile }));
    await page.goto('/console/settings?view=public');
    await new ConsolePage(page).connected();
    await expect(
      page.getByText('View only. Changes are made on the presenting laptop.').first(),
    ).toBeVisible();
    await expect(page.getByRole('button', { name: 'Add to trusted list' })).toHaveCount(0);
    await expect(page.getByRole('button', { name: /Remove .* from the trusted list/ })).toHaveCount(
      0,
    );
    await expect(page.getByRole('button', { name: /Ring my phone/ })).toHaveCount(0);
    await expect(page.getByRole('button', { name: /Record my voice/ })).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'Save for new calls' })).toBeDisabled();
    await page.goto('/console/calls/live-1?view=public');
    await new ConsolePage(page).connected();
    await page.getByText('Technical details').click();
    await expect(page.getByRole('button', { name: 'End session…' })).toHaveCount(0);
    expect(writes).toEqual([]);
  });
});
