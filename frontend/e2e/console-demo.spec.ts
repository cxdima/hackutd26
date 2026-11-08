import { expect, noHorizontalScroll, test } from './support/fixtures';
import { ConsolePage } from './support/pages';

test.describe('demo dashboard', () => {
  test('opens only when the visitor asks, sends nothing, and offers a way out', async ({
    page,
  }) => {
    const requests: string[] = [];
    page.on('request', (r) => r.url().includes('/api/') && requests.push(r.url()));
    await page.goto('/');
    await page.getByRole('link', { name: 'Explore the demo' }).first().click();
    await new ConsolePage(page).connected();
    await expect(page.getByRole('button', { name: 'Exit demo' }).first()).toBeVisible();
    await expect(page.getByRole('heading', { level: 1, name: 'Sarah’s line' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Most important call' })).toBeVisible();
    expect(requests).toEqual([]);
  });

  test('overview keeps one primary call and puts details one click away', async ({ page }) => {
    await page.goto('/console?view=demo');
    await new ConsolePage(page).connected();
    await expect(
      page.getByRole('link', { name: 'See what the AI heard and why it acted' }),
    ).toBeVisible();
    await expect(page.getByRole('heading', { name: 'What happened to calls' })).toBeVisible();
    // Details live behind tabs: only one panel shows at a time.
    const tabs = page.getByRole('tablist', { name: 'More about this line' });
    await expect(tabs.getByRole('tab', { name: /Callers to watch/ })).toHaveAttribute(
      'aria-selected',
      'true',
    );
    await expect(page.getByRole('tabpanel')).toHaveCount(1);
    await tabs.getByRole('tab', { name: /Trusted callers/ }).click();
    await expect(page.getByRole('tabpanel')).toContainText('Jimmy');
    await expect(page.getByRole('tabpanel')).toContainText('skip screening');
    await page.keyboard.press('ArrowRight');
    await expect(tabs.getByRole('tab', { name: 'How this line runs' })).toBeFocused();
  });

  test('filters and searches calls, then shows what FirstRing heard and why', async ({ page }) => {
    await page.goto('/console?view=demo');
    const console = new ConsolePage(page);
    await console.nav('Calls').click();
    await expect(page.getByRole('heading', { level: 1, name: 'Calls' })).toBeVisible();
    await console.choose('Outcome', /Message saved/);
    await expect(page.getByRole('combobox', { name: 'Outcome' })).toHaveText(/Message saved/);
    const rows = page.locator('table.data-table tbody tr.row-link');
    for (const row of await rows.all()) await expect(row).toContainText('Message saved');
    await console.choose('Outcome', /All outcomes/);
    await page.getByLabel('Search calls').fill('gift cards');
    await expect(rows).toHaveCount(1);
    await rows.first().locator('a').first().click();
    await expect(page.getByRole('heading', { level: 1 })).toContainText(
      'money sent an unusual way',
    );
    await expect(
      page.getByRole('heading', { name: 'What the AI saw and decided, turn by turn' }),
    ).toBeVisible();
    await page.getByText('Every receipt, in order').click();
    await expect(page.getByText('Proposed · executed below').first()).toBeVisible();
    await page.getByText(/Raw receipt #\d+ · policy.denied/).click();
    await expect(page.locator('pre').filter({ hasText: '"type": "policy.denied"' })).toBeVisible();
  });

  test('operator detail opens as a focused drawer and Escape returns focus', async ({ page }) => {
    await page.goto('/console?view=demo');
    const opener = page.getByRole('button', { name: 'Operator detail' }).last();
    await opener.click();
    const drawer = page.getByRole('dialog', { name: 'Operator detail' });
    await expect(drawer).toBeFocused();
    await expect(drawer.getByText('Demo fixtures in this browser')).toBeVisible();
    await expect(drawer.getByRole('heading', { name: 'Test controls' })).toHaveCount(0);
    await page.keyboard.press('Escape');
    await expect(drawer).toHaveCount(0);
    await expect(opener).toBeFocused();
  });

  test('settings explain demo limits instead of pretending to save', async ({ page }) => {
    await page.goto('/console/settings?view=demo');
    await page.getByLabel('Opening greeting').fill('Hello, you’ve reached the Lee household.');
    await expect(page.getByRole('button', { name: 'Save for new calls' })).toBeDisabled();
    await expect(page.getByText('saving needs a connected FirstRing server')).toBeVisible();
    await page.getByLabel('Opening greeting').fill('Call me at 555-010-2234');
    await expect(page.getByText('Remove links and phone numbers from the greeting.')).toBeVisible();
    // The demo trusted list is fictional and read-only.
    const trusted = page.getByRole('list', { name: 'Trusted callers' });
    await expect(trusted).toContainText('Jimmy');
    await expect(trusted).toContainText('Maya');
    await expect(page.getByRole('button', { name: 'Add to trusted list' })).toHaveCount(0);
    await expect(page.getByText('Fictional demo list. It cannot be changed here.')).toBeVisible();
  });

  test('community shows the latest warning compactly, then the active warnings', async ({
    page,
  }) => {
    await page.goto('/console/community?view=demo');
    await expect(
      page.getByRole('heading', { name: 'How the latest warning travelled' }),
    ).toBeVisible();
    await expect(page.getByRole('list', { name: 'Community warning stages' })).toBeVisible();
    // Compact: the diagram never dominates the page.
    const art = await page.locator('app-warning-trail svg.trail-art').boundingBox();
    expect(art?.height ?? 0).toBeLessThan(120);
    await expect(page.getByRole('heading', { name: 'Active warnings' })).toBeVisible();
    await page.getByText('How shared warnings differ from caller reputation').click();
    await expect(page.getByText('A phone number, kept as a one-way code')).toBeVisible();
  });

  test('call detail works on a phone-width screen @mobile', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto('/console/calls/demo-call-code?view=demo');
    await expect(page.getByRole('link', { name: 'All calls' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Conversation' })).toBeVisible();
    await noHorizontalScroll(page);
  });

  test('every console page fits a phone without sideways scrolling @mobile', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    for (const path of ['', '/calls', '/callers', '/community', '/settings']) {
      await page.goto(`/console${path}?view=demo`);
      await new ConsolePage(page).connected();
      await noHorizontalScroll(page);
    }
  });
});

test.describe('custom dropdown', () => {
  test('opens from the keyboard, chooses with Enter, and closes with Escape', async ({ page }) => {
    await page.goto('/console/calls?view=demo');
    const outcome = page.getByRole('combobox', { name: 'Outcome' });
    await expect(outcome).toHaveAttribute('aria-expanded', 'false');
    await outcome.focus();
    await page.keyboard.press('ArrowDown');
    await expect(outcome).toHaveAttribute('aria-expanded', 'true');
    const list = page.getByRole('listbox', { name: 'Outcome' });
    await expect(list).toBeVisible();
    await expect(list.getByRole('option', { name: /All outcomes/ })).toHaveAttribute(
      'aria-selected',
      'true',
    );
    // No native select anywhere on the page.
    await expect(page.locator('select')).toHaveCount(0);
    await page.keyboard.press('ArrowDown');
    await expect(outcome).toHaveAttribute('aria-activedescendant', /-option-1$/);
    await expect(list.getByRole('option').nth(1)).toHaveText(/Blocked/);
    await page.keyboard.press('Enter');
    await expect(list).toBeHidden();
    await expect(outcome).toBeFocused();
    await expect(outcome).toHaveText(/Blocked/);

    await page.keyboard.press('Enter');
    await expect(list).toBeVisible();
    await page.keyboard.press('End');
    await page.keyboard.press('Escape');
    await expect(list).toBeHidden();
    await expect(outcome).toBeFocused();
    await expect(outcome).toHaveText(/Blocked/);

    // A click outside closes it without changing the choice.
    await outcome.click();
    await expect(list).toBeVisible();
    await page.getByRole('heading', { name: 'Calls', level: 1 }).click();
    await expect(list).toBeHidden();
    await expect(outcome).toHaveText(/Blocked/);
  });

  test('the calls list never scrolls the page sideways', async ({ page }) => {
    for (const width of [1440, 1280, 1024, 768, 390]) {
      await page.setViewportSize({ width, height: 900 });
      await page.goto('/console/calls?view=demo');
      await expect(page.getByRole('combobox', { name: 'Outcome' })).toBeVisible();
      await noHorizontalScroll(page);
    }
  });
});
