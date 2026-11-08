import { expect, noHorizontalScroll, test } from './support/fixtures';
import { LandingPage } from './support/pages';

test.describe('landing page', () => {
  test('explains the product in the first viewport with working primary actions', async ({
    page,
  }) => {
    const apiCalls: string[] = [];
    page.on('request', (r) => r.url().includes('/api/') && apiCalls.push(r.url()));
    const landing = new LandingPage(page);
    await landing.goto();
    await expect(landing.hero).toBeVisible();
    await expect(page.getByText('FirstRing answers your calls first.')).toBeVisible();
    await expect(page.getByLabel('Example of a screened call')).toContainText(
      'I can’t share that. Goodbye.',
    );
    await page.getByRole('link', { name: 'See a screened call' }).click();
    await expect(
      page.getByRole('heading', { name: 'Two calls. Two very different requests.' }),
    ).toBeInViewport();
    expect(apiCalls).toEqual([]);
  });

  test('interactive example switches between an ordinary message and a blocked code request', async ({
    page,
  }) => {
    const landing = new LandingPage(page);
    await landing.goto();
    await expect(landing.example('An appointment message')).toHaveAttribute('aria-pressed', 'true');
    await expect(
      page.getByText('Lakeside Dental: cleaning moved to Thursday at 2:30.'),
    ).toBeVisible();
    await landing.example('A request for a code').click();
    await expect(landing.example('A request for a code')).toHaveAttribute('aria-pressed', 'true');
    await expect(page.locator('#example').getByText('Suspicious request blocked')).toBeVisible();
    await expect(
      page.getByText('I can’t help with codes. I’ve saved a note for Sarah. Goodbye.'),
    ).toBeVisible();
  });

  test('community concept, privacy answer and FAQ stay explicit about what is live', async ({
    page,
  }) => {
    await page.goto('/');
    await expect(page.getByText('Concept · not live yet')).toBeVisible();
    await page.getByText('How this would stay fair').click();
    await expect(page.getByText('No duplicate counting')).toBeVisible();
    await expect(
      page
        .getByText('Any audio played to another person can be recorded.', { exact: false })
        .first(),
    ).toBeVisible();
    await page.getByText('What is available in this prototype?').click();
    await expect(
      page.getByText('Built but not yet shown on a real call', { exact: false }),
    ).toBeVisible();
    await expect(
      page.getByText(
        'Our goal: help put phone scammers out of business by making their tricks harder to repeat.',
      ),
    ).toBeVisible();
  });

  test('reduced motion keeps every step readable and the hero paused until asked', async ({
    browser,
  }) => {
    const context = await browser.newContext({
      reducedMotion: 'reduce',
      viewport: { width: 1440, height: 900 },
    });
    const page = await context.newPage();
    await page.route('**/api/**', (r) => r.abort());
    await page.goto('/');
    const toggle = page.getByRole('button', { name: 'Play hero animation' });
    await expect(toggle).toBeVisible();
    const paused = await page
      .locator('[data-hero-art] video')
      .evaluate((v: HTMLVideoElement) => v.paused);
    expect(paused).toBe(true);
    await page.getByRole('heading', { name: 'How it works' }).scrollIntoViewIfNeeded();
    await expect(
      page.getByText('Northside Pharmacy: refill ready for pickup after 3 today.'),
    ).toBeVisible();
    await expect(page.getByRole('button', { name: 'Replay' })).toHaveCount(0);
    await context.close();
  });

  test('pause control stops the hero loop with motion allowed', async ({ page }) => {
    await page.goto('/');
    const video = page.locator('[data-hero-art] video');
    await expect
      .poll(() => video.evaluate((v: HTMLVideoElement) => !v.paused), { timeout: 10000 })
      .toBe(true);
    await page.getByRole('button', { name: 'Pause hero animation' }).click();
    await expect.poll(() => video.evaluate((v: HTMLVideoElement) => v.paused)).toBe(true);
    await expect(page.getByRole('button', { name: 'Play hero animation' })).toBeVisible();
  });

  test('keyboard users reach the skip link first and see focus', async ({ page }) => {
    await page.goto('/');
    await expect(new LandingPage(page).hero).toBeVisible();
    await page.keyboard.press('Tab');
    await expect(page.getByRole('link', { name: 'Skip to content' })).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(page).toHaveURL(/#main$/);
  });

  for (const width of [390, 768, 1440]) {
    test(`reflows without horizontal scrolling at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 900 });
      await page.goto('/');
      await noHorizontalScroll(page);
    });
  }

  test('mobile menu has large controls and closes with Escape @mobile', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto('/');
    const menu = page.locator('button[aria-controls="mobile-menu"]');
    await menu.click();
    await expect(menu).toHaveAttribute('aria-expanded', 'true');
    await expect(menu).toHaveText(/Close/);
    const link = page.locator('#mobile-menu').getByRole('link', { name: 'Privacy' });
    const box = await link.boundingBox();
    expect(box!.height).toBeGreaterThanOrEqual(48);
    await page.keyboard.press('Escape');
    await expect(page.locator('#mobile-menu')).toHaveCount(0);
    await expect(menu).toBeFocused();
    await menu.click();
    await page.locator('#mobile-menu').getByRole('link', { name: 'Privacy' }).click();
    await expect(
      page.getByRole('heading', { name: 'Could someone copy my voice?' }),
    ).toBeInViewport();
  });
});
