import type { Page, Request } from '@playwright/test';
import {
  event,
  expect,
  hold,
  liveState,
  noHorizontalScroll,
  phoneDemo,
  profile,
  sse,
  test,
} from './support/fixtures';
import { ConsolePage } from './support/pages';

/** Fictional rehearsal (DEMO) and mocked-server checks of the projected screen. */

function trackRequests(page: Page) {
  const sent: Request[] = [];
  page.on('request', (r) => sent.push(r));
  return {
    api: () => sent.filter((r) => new URL(r.url()).pathname.startsWith('/api')),
    writes: () => sent.filter((r) => r.method() !== 'GET'),
  };
}

async function play(page: Page, name: string) {
  await page.getByRole('button', { name }).click();
  await page.getByRole('button', { name: 'Skip to end' }).click();
}

test.describe('presentation view · fictional rehearsal', () => {
  test('waiting, then the request changes and the outcome lands, with no network calls', async ({
    page,
  }) => {
    const requests = trackRequests(page);
    await page.goto('/console/present?view=demo');
    await expect(page.getByText('Rehearsal ready. Play a fictional call above.')).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Call FirstRing' })).toBeVisible();
    // Demo history never takes over the screen.
    await expect(page.locator('#present-caption')).toHaveCount(0);

    await page.getByRole('button', { name: 'The request changes' }).click();
    await expect(page.locator('#present-caption')).toContainText('Northside Clinic', {
      timeout: 8000,
    });
    await expect(page.getByText('On the line', { exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Skip to end' }).click();

    await expect(page.locator('#present-caption')).toContainText('six-digit code');
    await expect(page.getByText('Then the request changed.')).toBeVisible();
    const outcome = page.locator('.outcome-graphic');
    await expect(outcome).toContainText('Suspicious request blocked');
    await expect(page.getByText('Old keypad approval rejected').first()).toBeVisible();
    // The rundown keeps the newest five stages; the withdrawal is in the permission list.
    await expect(page.getByRole('list', { name: 'Call stages' })).toContainText(
      'Old keypad approval rejected',
    );
    await expect(
      page
        .getByRole('complementary', { name: 'Request and outcome' })
        .getByText('Permission withdrawn'),
    ).toBeVisible();
    // Demo receipts are authored: the credit must not claim a model ran.
    await expect(page.getByText('Demo data: no model was called')).toBeVisible();
    expect(requests.api()).toHaveLength(0);
    expect(requests.writes()).toHaveLength(0);
  });

  test('the connected call shows every permission receipt before “Connected”', async ({ page }) => {
    await page.goto('/console/present?view=demo');
    await play(page, 'Ordinary call, connected');
    await expect(page.locator('.outcome-graphic')).toContainText('Connected to Sarah');
    const permission = page.getByRole('complementary', { name: 'Request and outcome' });
    for (const step of [
      'Permission requested',
      'Sarah’s phone rang',
      'Keypad response received',
      'Both people connected',
    ])
      await expect(permission.getByText(step)).toBeVisible();
    await expect(permission.getByText(': no receipt')).toHaveCount(0);
  });

  test('evidence drawer separates NVIDIA evidence and returns focus on Escape', async ({
    page,
  }) => {
    await page.goto('/console/present?view=demo');
    await play(page, 'The request changes');
    const opener = page.getByRole('button', { name: 'Evidence' });
    await opener.click();
    const drawer = page.getByRole('dialog', { name: 'Evidence for this call' });
    await expect(drawer).toBeVisible();
    await expect(drawer.getByText('Demo data.')).toBeVisible();
    await expect(drawer.getByRole('heading', { name: 'NVIDIA decision model' })).toBeVisible();
    await expect(drawer.getByText('Authored receipt').first()).toBeVisible();
    await expect(drawer.getByText('Carried out by Rust')).toHaveCount(0);
    await expect(drawer.getByText('Old keypad approval rejected')).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(drawer).toBeHidden();
    await expect(opener).toBeFocused();
  });

  test('community scene completes only after Maya’s matching call', async ({ page }) => {
    await page.goto('/console/present?view=demo');
    await expect(page.getByRole('button', { name: 'Maya was ready' })).toBeDisabled();
    await play(page, 'The request changes');
    await page.getByRole('button', { name: 'Community', exact: true }).click();
    await expect(page.getByText('A shared warning is ready for Maya.')).toBeVisible();
    await page.getByRole('button', { name: 'Call', exact: true }).click();
    await play(page, 'Maya was ready');
    await page.getByRole('button', { name: 'Community', exact: true }).click();
    await expect(page.getByText('Sarah spotted it. Maya was ready.')).toBeVisible();
    await expect(page.getByText('not the same person or number')).toBeVisible();
  });

  test('reset clears the display without deleting calls', async ({ page }) => {
    await page.goto('/console/present?view=demo');
    await play(page, 'Ordinary call, connected');
    await expect(page.locator('#present-caption')).toBeVisible();
    await page.getByRole('button', { name: 'Reset display' }).click();
    await expect(page.getByRole('heading', { name: 'Call FirstRing' })).toBeVisible();
    const showing = page.getByRole('combobox', { name: 'Showing' });
    await expect(showing).toHaveText(/Follow newest call/);
    await showing.click();
    const list = page.getByRole('listbox', { name: 'Showing' });
    await expect(list.getByRole('option').first()).toHaveText(/Follow newest call/);
    await expect(list.getByRole('option').first()).toHaveAttribute('aria-selected', 'true');
    await list.getByRole('option').nth(1).click();
    await expect(list).toBeHidden();
    await expect(page.locator('#present-caption')).toContainText('Lakeside Pharmacy');
    await expect(showing).not.toHaveText(/Follow newest call/);
  });

  test('replays a recorded call from its own receipts, labeled as a replay', async ({ page }) => {
    const requests = trackRequests(page);
    await page.goto('/console/present?view=demo');
    const list = page.getByRole('region', { name: 'Recorded calls to show' });
    await expect(list.getByText('The request changed, then it was blocked')).toBeVisible();
    await list.getByRole('button', { name: 'Replay' }).first().click();
    await expect(page.getByText('Replay · recorded call')).toBeVisible();
    // Early in the replay there is no outcome yet.
    await expect(page.locator('.outcome-graphic')).toContainText('Screening in progress');
    await page.getByRole('button', { name: 'Skip to end' }).click();
    await expect(page.locator('.outcome-graphic')).toContainText('Suspicious request blocked');
    await page.getByRole('button', { name: 'Stop replay' }).click();
    await expect(page.getByText('Replay · recorded call')).toHaveCount(0);
    expect(requests.api()).toHaveLength(0);
  });

  test('reduced motion keeps the waiting beacon still', async ({ page }) => {
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.goto('/console/present?view=demo');
    await expect(page.getByRole('heading', { name: 'Call FirstRing' })).toBeVisible();
    await expect(page.locator('.waiting-ping')).toHaveCount(0);
  });

  test('works on a phone-width screen with presenter controls tucked away @mobile', async ({
    page,
  }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto('/console/present?view=demo');
    await noHorizontalScroll(page);
    // The stage comes first; presenter controls live behind one disclosure.
    await expect(page.getByRole('heading', { name: 'Call FirstRing' })).toBeInViewport();
    await page.getByText('Presenter controls').click();
    await play(page, 'The request changes');
    await expect(page.locator('.outcome-graphic')).toContainText('Suspicious request blocked');
    await noHorizontalScroll(page);
  });
});

test.describe('presentation view · mocked live server', () => {
  const historical = {
    ...liveState().sessions[0],
    id: 'old-call',
    state: 'ended',
    inbound_ended: true,
  };

  async function connect(page: Page) {
    await page.goto('/console/present');
    await new ConsolePage(page).presenting();
  }

  test('history stays off screen; waiting shows the number and a tel: QR only', async ({
    page,
  }) => {
    await page.route('**/api/state', (route) =>
      route.fulfill({
        json: liveState({ phone_display: '+15555550134', sessions: [historical] }),
      }),
    );
    // A quiet, healthy stream. (Replayed history never taking over is covered by unit tests.)
    await page.route('**/api/events', (route) => hold(route));
    await connect(page);
    await expect(page.getByText('Connected. Waiting for a call.')).toBeVisible({ timeout: 15000 });
    await expect(page.getByText('+1 (555) 555-0134')).toBeVisible();
    await expect(page.locator('#present-caption')).toHaveCount(0);
    await page.getByRole('button', { name: /Show a QR code/ }).click();
    await expect(page.getByRole('img', { name: 'QR code that dials +15555550134' })).toBeVisible();
    await expect(page.getByRole('list', { name: 'Service readiness' })).toContainText(
      'Ring my phone first',
    );
  });

  test('a dropped stream reads as service unavailable, not as waiting', async ({ page }) => {
    await page.route('**/api/state', (route) =>
      route.fulfill({ json: liveState({ sessions: [historical] }) }),
    );
    let streams = 0;
    await page.route('**/api/events', (route) => {
      streams += 1;
      if (streams > 1) return route.abort('connectionrefused');
      return route.fulfill({
        status: 200,
        headers: { 'Content-Type': 'text/event-stream' },
        body: '',
      });
    });
    await connect(page);
    await expect(page.getByRole('heading', { name: 'Service unavailable' })).toBeVisible();
    await expect(page.getByRole('banner').getByText('Reconnecting…')).toBeVisible();
    await expect(page.getByText('Connected. Waiting for a call.')).toHaveCount(0);
  });

  test('follows a new call that arrives after the screen was armed', async ({ page }) => {
    const fresh = {
      ...liveState().sessions[0],
      id: 'new-call',
      blocked: false,
      assessments: [],
      turns: [{ id: 'n1', speaker: 'caller', text: 'Hi, this is the pharmacy calling for Sarah.' }],
    };
    let stateCalls = 0;
    await page.route('**/api/state', (route) => {
      stateCalls += 1;
      return route.fulfill({
        json: liveState({ sessions: stateCalls > 1 ? [fresh, historical] : [historical] }),
      });
    });
    let streams = 0;
    await page.route('**/api/events', async (route) => {
      streams += 1;
      if (streams > 1) return hold(route);
      await new Promise((r) => setTimeout(r, 600));
      const now = new Date().toISOString();
      return route.fulfill({
        status: 200,
        headers: { 'Content-Type': 'text/event-stream' },
        body: sse([
          {
            ...event('call.inbound.received', { requested_profile: profile }, 'new-call'),
            occurred_at: now,
            received_at: now,
          },
          {
            ...event(
              'transcript.final',
              { text: 'Hi, this is the pharmacy calling for Sarah.', turn_id: 'n1' },
              'new-call',
            ),
            occurred_at: now,
            received_at: now,
          },
        ]),
      });
    });
    await connect(page);
    await expect(page.locator('#present-caption')).toContainText('pharmacy calling for Sarah', {
      timeout: 8000,
    });
  });

  test('rings my phone on the next call only after an explicit click, then can disarm', async ({
    page,
  }) => {
    const expires = new Date(Date.now() + 10 * 60_000).toISOString();
    let status = phoneDemo();
    const posts: { url: string; body: string | null }[] = [];
    await page.route('**/api/state', (route) =>
      route.fulfill({ json: liveState({ phone_demo: status }) }),
    );
    await page.route('**/api/events', (route) => hold(route));
    await page.route('**/api/demo/phone-demo/**', (route) => {
      const url = new URL(route.request().url()).pathname;
      posts.push({ url, body: route.request().postData() });
      status = url.endsWith('/arm')
        ? phoneDemo({
            state: 'armed',
            arm_id: 'arm-7',
            expires_at: expires,
            can_arm: false,
            can_disarm: true,
          })
        : phoneDemo({ state: 'disarmed' });
      return route.fulfill({ json: status });
    });
    await connect(page);
    const arm = page.getByRole('button', { name: 'Ring my phone on the next call' });
    await expect(arm).toBeVisible();
    // Loading the screen never arms or calls anything.
    expect(posts).toEqual([]);
    await arm.click();
    await expect.poll(() => posts).toHaveLength(1);
    expect(posts[0].url).toBe('/api/demo/phone-demo/arm');
    expect(posts[0].body).toBe('{"contact_id":"sarah","ttl_seconds":600}');
    await expect(page.getByText(/^Ready to ring your phone until \d{1,2}:\d{2}/)).toBeVisible();
    await expect(arm).toHaveCount(0);
    await page.getByRole('button', { name: 'Disarm' }).click();
    await expect.poll(() => posts).toHaveLength(2);
    expect(posts[1]).toEqual({ url: '/api/demo/phone-demo/disarm', body: '{"arm_id":"arm-7"}' });
    await expect(page.getByText(/Ready to ring your phone/)).toHaveCount(0);
  });

  test('the demo never offers to ring a phone', async ({ page }) => {
    await page.goto('/console/present?view=demo');
    await expect(page.getByRole('heading', { name: 'Call FirstRing' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Ring my phone on the next call' })).toHaveCount(
      0,
    );
  });
});
