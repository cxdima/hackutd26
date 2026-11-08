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
    await expect(page.getByRole('button', { name: 'Exit demo' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Calls for Sarah' })).toBeVisible();
    await expect(page.getByText('Most important')).toBeVisible();
    expect(requests).toEqual([]);
  });

  test('filters and searches calls, then shows what FirstRing heard and why', async ({ page }) => {
    await page.goto('/console?view=demo');
    const console = new ConsolePage(page);
    await console.nav('Calls').click();
    await console.choose('Outcome', /Message saved/);
    await expect(page.getByRole('combobox', { name: 'Outcome' })).toHaveText(/Message saved/);
    await expect(page.getByText('1 of 5 calls')).toBeVisible();
    await console.choose('Answered for', /For Maya/);
    await expect(page.getByText('0 of 5 calls')).toBeVisible();
    await page.getByRole('button', { name: 'Clear filters' }).click();
    await expect(page.getByText('5 of 5 calls')).toBeVisible();
    await page.getByLabel('Search calls').fill('gift cards');
    await expect(page.getByText('1 of 5 calls')).toBeVisible();
    await page.locator('app-call-row a').first().click();
    await expect(page.getByRole('heading', { level: 1 })).toContainText(
      'money sent an unusual way',
    );
    await expect(
      page.getByRole('heading', { name: 'What FirstRing saw, and why it acted' }),
    ).toBeVisible();
    await expect(page.getByText('Protected request').first()).toBeVisible();
    await expect(page.getByText('Rule check: protected request.', { exact: false })).toBeVisible();
    await expect(page.getByText('Proposed · executed below').first()).toBeVisible();
    await page.getByText(/Raw receipt #\d+ · policy.denied/).click();
    await expect(page.locator('pre').filter({ hasText: '"type": "policy.denied"' })).toBeVisible();
  });

  test('operator detail opens as a focused drawer and Escape returns focus', async ({ page }) => {
    await page.goto('/console?view=demo');
    const opener = page.getByRole('button', { name: 'Operator detail' }).first();
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
  });

  test('community separates tactic warnings from labeled example number history', async ({
    page,
  }) => {
    await page.goto('/console/community?view=demo');
    await expect(page.getByRole('heading', { name: 'Asking for a sign-in code' })).toBeVisible();
    await expect(
      page.getByRole('heading', { name: 'Same tactic, different number' }),
    ).toBeVisible();
    await page.getByRole('button', { name: /Show the example progression/ }).click();
    await expect(page.getByText('Example · fictional number (555) 010-2234')).toBeVisible();
  });

  test('call detail works on a phone-width screen @mobile', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto('/console/calls/demo-call-code?view=demo');
    await expect(page.getByRole('link', { name: 'All calls' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Transcript' })).toBeVisible();
    await noHorizontalScroll(page);
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
    await expect(list.getByRole('option', { name: /All calls/ })).toHaveAttribute(
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
    await expect(page.getByText('2 of 5 calls')).toBeVisible();

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

  test('the list column never clips or scrolls sideways', async ({ page }) => {
    for (const width of [1440, 1280, 1024, 768, 390]) {
      await page.setViewportSize({ width, height: 900 });
      await page.goto(
        width >= 1024 ? '/console/calls/demo-call-code?view=demo' : '/console/calls?view=demo',
      );
      const column = page.locator('section[aria-labelledby="calls-heading"]');
      await expect(column.getByRole('combobox', { name: 'Outcome' })).toBeVisible();
      const clip = await column.evaluate((el) => el.scrollWidth - el.clientWidth);
      expect(clip, `list column at ${width}px`).toBeLessThanOrEqual(0);
      await noHorizontalScroll(page);
    }
  });
});
