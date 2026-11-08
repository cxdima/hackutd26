// Renders the README banners and diagrams from docs/readme/src/*.html.
// Reuses the frontend's pinned Playwright install; no extra dependencies.
//   node docs/readme/render.mjs            # everything
//   node docs/readme/render.mjs hero arch  # selected targets
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';

const here = path.dirname(fileURLToPath(import.meta.url));
const require = createRequire(path.join(here, '../../frontend/package.json'));
const { chromium } = require('playwright');
const sharp = require('sharp');

// name -> [source html, output file, css width]
const targets = {
  hero: ['hero.html', 'hero.webp', 1280],
  'banner-voice': ['banner.html?k=voice', 'banner-voice.webp', 1280],
  'banner-nvidia': ['banner.html?k=nvidia', 'banner-nvidia.webp', 1280],
  'banner-safety': ['banner.html?k=safety', 'banner-safety.webp', 1280],
  'banner-community': ['banner.html?k=community', 'banner-community.webp', 1280],
  arch: ['architecture.html', 'architecture.png', 1080],
  turn: ['turn-anatomy.html', 'turn-anatomy.png', 1080],
  voice: ['voice-pipeline.html', 'voice-pipeline.png', 1080],
  brev: ['brev-stack.html', 'brev-stack.png', 1080],
  console: ['console.html', 'console.webp', 1080],
};

const wanted = process.argv.slice(2);
const names = wanted.length ? wanted : Object.keys(targets);
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1360, height: 900 }, deviceScaleFactor: 2 });
for (const name of names) {
  const [src, out, width] = targets[name] ?? [];
  if (!src) throw new Error(`unknown target ${name}`);
  const [file, query] = src.split('?');
  const url = pathToFileURL(path.join(here, 'src', file)).href + (query ? `?${query}` : '');
  await page.setViewportSize({ width: width + 80, height: 900 });
  await page.goto(url);
  await page.evaluate(() => document.fonts.ready);
  await page.waitForFunction(() => window.__ready !== false);
  // Fail loudly on clipped copy instead of shipping a truncated figure.
  const clipped = await page.evaluate(() => [...document.querySelectorAll('.box')].flatMap((box) => {
    const title = box.querySelector('.b-title')?.textContent ?? '?';
    const limit = box.getBoundingClientRect().bottom - 4;
    const body = box.querySelector('.b-body');
    const deepest = Math.max(...[...box.querySelectorAll('.b-body *, .b-body')].map((n) => n.getBoundingClientRect().bottom));
    const wide = body ? [...body.children].some((c) => c.scrollWidth > body.clientWidth + 1) : false;
    const head = box.querySelector('.b-head');
    return [
      ...(deepest > limit ? [`${title}: text runs ${Math.ceil(deepest - limit)}px past the bottom`] : []),
      ...(wide || (head && head.scrollWidth > head.clientWidth + 1) ? [`${title}: a line is too wide`] : []),
    ];
  }));
  if (clipped.length) console.warn(`  ! ${name} overflow:\n    ${[...new Set(clipped)].join('\n    ')}`);
  // Rounded corners need alpha, so capture PNG and encode per target:
  // photographic banners -> WebP, text-heavy diagrams -> palette PNG.
  const png = await page.locator('#frame').screenshot({ type: 'png', omitBackground: true });
  const encoded = out.endsWith('.webp')
    ? sharp(png).webp({ quality: 86, alphaQuality: 100, effort: 6 })
    : sharp(png).png({ palette: true, quality: 95, effort: 10, compressionLevel: 9 });
  await encoded.toFile(path.join(here, out));
  console.log(`rendered ${out}`);
}
await browser.close();
