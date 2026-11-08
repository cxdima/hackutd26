import { expect, type Locator, type Page } from '@playwright/test';

export class LandingPage {
  readonly hero: Locator;
  constructor(readonly page: Page) {
    this.hero = page.getByRole('heading', {
      level: 1,
      name: /Kind to every caller\.\s*Firm about codes\./,
    });
  }
  goto() {
    return this.page.goto('/');
  }
  example(name: string) {
    return this.page
      .getByRole('group', { name: 'Choose an example call' })
      .getByRole('button', { name: new RegExp(name) });
  }
}

export class ConsolePage {
  constructor(readonly page: Page) {}
  nav(name: string) {
    return this.page
      .getByRole('navigation', { name: 'Dashboard' })
      .first()
      .getByRole('link', { name });
  }
  /** The console connects by itself; it is ready once the dashboard navigation shows. */
  async connected() {
    await expect(this.page.getByRole('navigation', { name: 'Dashboard' }).first()).toBeVisible({
      timeout: 15000,
    });
  }
  /** The presentation view has no dashboard chrome; it is ready once its scene switch shows. */
  async presenting() {
    await expect(this.page.getByRole('group', { name: 'Scene' })).toBeVisible({ timeout: 15000 });
  }
  /** Opens a custom dropdown (app-select-menu) by its label and picks an option. */
  async choose(label: string, option: string | RegExp) {
    await this.page.getByRole('combobox', { name: label }).click();
    await this.page.getByRole('option', { name: option }).click();
  }
}
