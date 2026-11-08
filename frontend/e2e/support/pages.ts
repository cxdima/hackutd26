import type { Locator, Page } from '@playwright/test';

export class LandingPage {
  readonly hero: Locator;
  constructor(readonly page: Page) {
    this.hero = page.getByRole('heading', {
      level: 1,
      name: 'Before you answer, FirstRing checks.',
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
  bug() {
    return this.page.locator('app-status-bug .bug');
  }
  async connect(token: string) {
    await this.page.getByLabel('Operator token').fill(token);
    await this.page.getByRole('button', { name: 'Connect' }).click();
  }
}
