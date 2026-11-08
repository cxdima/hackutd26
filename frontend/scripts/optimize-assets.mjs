// Converts downloaded Higgsfield sources (asset-sources/higgsfield) into the
// web-optimized files the app ships from public/media. Re-run after replacing a source.
// Stills use sharp; video uses the system ffmpeg (libx264 + libvpx-vp9).
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, statSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import sharp from 'sharp';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const src = join(root, 'asset-sources', 'higgsfield');
const out = join(root, 'public', 'media');
mkdirSync(out, { recursive: true });

const report = [];
async function still(input, name, { widths, crop, avif = false, alpha = false, quality = 80 }) {
  for (const width of widths) {
    let pipeline = sharp(join(src, input));
    if (crop) pipeline = pipeline.extract(crop);
    pipeline = pipeline.resize({ width, withoutEnlargement: true });
    const webp = join(out, `${name}-${width}.webp`);
    await pipeline
      .clone()
      .webp({ quality: alpha ? 86 : quality, alphaQuality: 90, effort: 6, smartSubsample: true })
      .toFile(webp);
    report.push(webp);
    if (avif) {
      const file = join(out, `${name}-${width}.avif`);
      await pipeline
        .clone()
        .avif({ quality: quality > 80 ? 70 : 58, effort: 6 })
        .toFile(file);
      report.push(file);
    }
  }
}

function video(input, name, { crop, width, crf = 24, vp9crf = 36 }) {
  const filters = [crop, `scale=${width}:-2:flags=lanczos`, 'format=yuv420p']
    .filter(Boolean)
    .join(',');
  const mp4 = join(out, `${name}.mp4`);
  execFileSync('ffmpeg', [
    '-loglevel',
    'error',
    '-y',
    '-i',
    join(src, input),
    '-an',
    '-vf',
    filters,
    '-c:v',
    'libx264',
    '-preset',
    'slow',
    '-crf',
    String(crf),
    '-movflags',
    '+faststart',
    mp4,
  ]);
  const webm = join(out, `${name}.webm`);
  execFileSync('ffmpeg', [
    '-loglevel',
    'error',
    '-y',
    '-i',
    join(src, input),
    '-an',
    '-vf',
    filters,
    '-c:v',
    'libvpx-vp9',
    '-b:v',
    '0',
    '-crf',
    String(vp9crf),
    '-row-mt',
    '1',
    webm,
  ]);
  report.push(mp4, webm);
}

// Hero still: 2688x1520 source. Ringo occupies roughly x 1210–2513.
await still('hero/hero-desktop-2k.png', 'ringo-hero', {
  widths: [1280, 1920, 2688],
  avif: true,
  quality: 92,
});
await still('hero/hero-desktop-2k.png', 'ringo-hero-square', {
  widths: [720, 1200],
  crop: { left: 1100, top: 0, width: 1520, height: 1520 },
  avif: true,
  quality: 92,
});
for (const pose of ['greeting', 'listening', 'rest']) {
  await still(`poses/${pose}.png`, `ringo-${pose}`, { widths: [480, 960], alpha: true });
}
await still('concepts/concept-1-vinyl.png', 'ringo-reference', { widths: [640] });
await still('community/community-rings-2k.png', 'community-rings', {
  widths: [960, 1600, 2400],
  avif: true,
});

if (existsSync(join(src, 'video', 'hero-loop-kling.mp4'))) {
  video('video/hero-loop-kling.mp4', 'ringo-hero-loop', { width: 1276, crf: 20, vp9crf: 28 });
  // Square crop around Ringo for narrow screens (source is 1276x720).
  video('video/hero-loop-kling.mp4', 'ringo-hero-loop-square', {
    crop: 'crop=720:720:522:0',
    width: 720,
    crf: 20,
    vp9crf: 28,
  });
  execFileSync('ffmpeg', [
    '-loglevel',
    'error',
    '-y',
    '-i',
    join(src, 'video', 'hero-loop-kling.mp4'),
    '-vf',
    'select=eq(n\\,0)',
    '-frames:v',
    '1',
    '-q:v',
    '2',
    join(out, 'ringo-hero-loop-poster.jpg'),
  ]);
  report.push(join(out, 'ringo-hero-loop-poster.jpg'));
}
if (existsSync(join(src, 'video', 'community-loop-kling.mp4'))) {
  video('video/community-loop-kling.mp4', 'community-loop', { width: 1024, crf: 29, vp9crf: 42 });
  execFileSync('ffmpeg', [
    '-loglevel',
    'error',
    '-y',
    '-i',
    join(src, 'video', 'community-loop-kling.mp4'),
    '-vf',
    'select=eq(n\\,0),scale=1024:-2',
    '-frames:v',
    '1',
    '-q:v',
    '4',
    join(out, 'community-loop-poster.jpg'),
  ]);
  report.push(join(out, 'community-loop-poster.jpg'));
}

for (const file of report) {
  const meta = file.match(/\.(webp|avif|jpg)$/) ? await sharp(file).metadata() : null;
  console.log(
    file.replace(`${root}/`, ''),
    `${Math.round(statSync(file).size / 1024)} KB`,
    meta ? `${meta.width}x${meta.height}${meta.hasAlpha ? ' alpha' : ''}` : '',
  );
}
