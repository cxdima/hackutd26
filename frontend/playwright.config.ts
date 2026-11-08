import { defineConfig, devices } from '@playwright/test';

/**
 * Browser checks run against the Angular dev server on port 4310.
 * Every /api request is mocked or aborted by the fixtures in e2e/support, so no
 * test can reach or mutate a live FirstRing server.
 */
export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env['CI'],
  retries: 0,
  // First loads of lazily compiled dev-server chunks can be slow under parallel load.
  expect: { timeout: 10000 },
  timeout: 60000,
  reporter: [['list'], ['html', { open: 'never', outputFolder: 'test-results/html' }]],
  outputDir: 'test-results/artifacts',
  use: {
    baseURL: 'http://127.0.0.1:4310',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  projects: [
    {
      name: 'desktop',
      use: { ...devices['Desktop Chrome'], viewport: { width: 1440, height: 900 } },
    },
    {
      name: 'mobile',
      use: { ...devices['Desktop Chrome'], viewport: { width: 390, height: 844 }, hasTouch: true },
      grep: /@mobile/,
    },
  ],
  webServer: {
    command: 'npm start',
    url: 'http://127.0.0.1:4310',
    reuseExistingServer: !process.env['CI'],
    timeout: 120000,
  },
});
