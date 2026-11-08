// Dev-server proxy: /api goes to the local Rust server on 127.0.0.1:8080.
//
// Local operator session: when the repository's .env (or FIRSTRING_ENV_FILE)
// contains DEMO_ADMIN_TOKEN, this proxy adds it server-side to /api requests
// that come from the console's own pages (Sec-Fetch-Site: same-origin). The
// console then connects without anyone pasting a token. The token stays in this
// Node process: it never enters the browser bundle, page, URL, storage or logs.
// The dev server listens on 127.0.0.1 only. Set FIRSTRING_CONSOLE_AUTOCONNECT=off
// to require the token form.
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));

function localOperatorToken() {
  if (process.env.FIRSTRING_CONSOLE_AUTOCONNECT === 'off') return '';
  const file = process.env.FIRSTRING_ENV_FILE ?? resolve(here, '../.env');
  if (!existsSync(file)) return '';
  const line = readFileSync(file, 'utf8')
    .split(/\r?\n/)
    .find((entry) => entry.startsWith('DEMO_ADMIN_TOKEN='));
  return line
    ? line
        .slice('DEMO_ADMIN_TOKEN='.length)
        .trim()
        .replace(/^["']|["']$/g, '')
    : '';
}

const token = localOperatorToken();
console.log(
  token
    ? '[firstring] Local operator session on: the console connects automatically on this computer.'
    : '[firstring] Local operator session off: the console will ask for the operator token.',
);

export default {
  '/api': {
    target: 'http://127.0.0.1:8080',
    secure: false,
    changeOrigin: false,
    configure: (proxy) => {
      if (!token) return;
      proxy.on('proxyReq', (proxyReq, req) => {
        const sameOrigin = req.headers['sec-fetch-site'] === 'same-origin';
        if (sameOrigin && !req.headers['authorization'])
          proxyReq.setHeader('Authorization', `Bearer ${token}`);
      });
    },
  },
};
