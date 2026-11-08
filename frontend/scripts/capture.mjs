// Captures review screenshots of the landing page and the demo dashboard.
// Usage: node scripts/capture.mjs [outDir] [baseUrl]
// Every /api request is blocked so captures can never touch a live FirstRing server.
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';

const out = process.argv[2] ?? 'review/screenshots';
const base = process.argv[3] ?? 'http://127.0.0.1:4310';
mkdirSync(out, { recursive: true });

const browser = await chromium.launch();
const shots = [];
async function page(width, height, options = {}) {
  const context = await browser.newContext({
    viewport: { width, height },
    deviceScaleFactor: 1,
    reducedMotion: options.reducedMotion ?? 'reduce',
  });
  const p = await context.newPage();
  await p.route('**/api/**', (route) => route.abort());
  const errors = [];
  p.on('pageerror', (error) => errors.push(error.message));
  p.on('console', (msg) => msg.type() === 'error' && errors.push(msg.text()));
  return { context, p, errors };
}
async function settle(p) {
  await p.waitForLoadState('networkidle').catch(() => undefined);
  await p.evaluate(() => document.fonts.ready);
  // Walk the page so lazy images load, then return to the top for the capture.
  await p.evaluate(async () => {
    for (let y = 0; y < document.body.scrollHeight; y += 700) {
      window.scrollTo(0, y);
      await new Promise((r) => setTimeout(r, 60));
    }
    window.scrollTo(0, 0);
  });
  await p.waitForLoadState('networkidle').catch(() => undefined);
  await p.waitForTimeout(400);
}

for (const [name, width, height] of [
  ['desktop', 1440, 900],
  ['tablet', 768, 1024],
  ['mobile', 390, 844],
]) {
  const { context, p, errors } = await page(width, height);
  await p.goto(`${base}/`);
  await settle(p);
  await p.screenshot({ path: join(out, `landing-${name}-viewport.png`) });
  await p.screenshot({ path: join(out, `landing-${name}-full.png`), fullPage: true });
  shots.push(`landing-${name}`);

  await p.goto(`${base}/console?view=demo`);
  await settle(p);
  await p.screenshot({ path: join(out, `console-overview-${name}.png`), fullPage: true });
  await p.goto(`${base}/console/calls/demo-call-code?view=demo`);
  await settle(p);
  await p.screenshot({ path: join(out, `console-call-${name}.png`), fullPage: true });
  if (name === 'desktop') {
    for (const route of ['community', 'settings', 'calls']) {
      await p.goto(`${base}/console/${route}?view=demo`);
      await settle(p);
      await p.screenshot({ path: join(out, `console-${route}-${name}.png`), fullPage: true });
    }
    await p.goto(`${base}/console`);
    await settle(p);
    await p.screenshot({ path: join(out, `console-locked-${name}.png`), fullPage: true });
  }
  if (errors.length) console.log(name, 'errors:', errors.slice(0, 5));
  await context.close();
}
await browser.close();
console.log('captured', shots.join(', '));
