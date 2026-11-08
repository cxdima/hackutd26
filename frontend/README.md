# FirstRing web (Angular + Tailwind)

The public landing page (`/`) and the authenticated dashboard (`/console`) for FirstRing. It replaces the earlier React console in `../web` (kept unchanged until integration) and uses the same Rust API and console contract (`../docs/event-contract.md`).

Stack: Angular 22 (standalone, zoneless, signals), Tailwind CSS 4, GSAP 3 for four authored motion moments, Zod for runtime contract validation, Lucide icons, Manrope. Tests: Vitest through `ng test`, Playwright for browser checks. All versions are pinned exactly in `package.json` and `package-lock.json`.

## Run

```sh
npm ci
npm start            # http://127.0.0.1:4310, proxies /api to http://127.0.0.1:8080
```

- `/`: landing page. It never calls the API.
- `/console`: locked until you enter the server's `DEMO_ADMIN_TOKEN`. The token is held in memory only (never localStorage, sessionStorage, cookies or URLs); reloading or locking clears it.
- `/console?view=demo`: curated fictional calls, labeled DEMO throughout, replayed in the browser. Nothing in the demo contacts a server, phone line or model.
- `/console/present`: the projected screen for a live demonstration. It shows the caller's words as a large caption, FirstRing's current step, the outcome and an evidence drawer. "Follow newest call" ignores history; you can pin a call, and "Reset display" clears only what is shown. A tel: QR code shows FirstRing's own inbound number. In `?view=demo` it adds three fictional rehearsal calls: connected, request changes, and Maya was ready.
- `/console/settings`: profile editor, private permission call readiness from the server, and the optional voice enrollment, which runs as an in-browser example only while the server reports cloning as disabled.

`npm run build` writes the production app to `dist/firstring-web/browser/` (initial bundle about 80 KB transferred; landing and console pages are lazy chunks).

## Checks

```sh
npm run lint          # angular-eslint, template accessibility rules included
npm run typecheck     # strict TS + strict templates
npm run format:check  # prettier
npm test              # Vitest unit tests (contract, decision trace, outcomes, store, SSE)
npx playwright install chromium
npm run test:browser  # Playwright: landing, demo dashboard, mocked connected dashboard
```

Every Playwright test runs with `/api/**` aborted unless the test installs an explicit synthetic response (`e2e/support/fixtures.ts`). No browser test can reach a live FirstRing server.

## How the dashboard explains a call

`src/app/core/calls.ts` turns a session snapshot plus its receipts (`/api/events`) into:

- an **outcome**: suspicious request blocked, connected after permission (only on `bridge.connected`), message saved, needs review, ended without a message, incomplete, or on the line now. Each has an icon and words, never colour alone;
- a **plain-language bulletin** and the caller's request;
- the **decision spine**: receipts grouped by caller turn. *Heard* is the final transcript. *Model* covers Nemotron latency, tokens and finish reason. *Proposed* is the model's tool call and arguments. *Rule* is Rust's `policy.allowed/denied`, with the quoted evidence and reason. *Understood* is the accepted purpose, request and readiness. *Done* is the execution receipt. *Said* is the requested audio, unverified as heard. *Saved* is the message receipt. Discarded or superseded work and failures are listed too. Every step opens its raw receipt JSON.

A proposal stays "proposed" until a matching `agent.tool.result` with `ok: true` exists. A result with `ok: false` is shown as "Not done … Rust rejected this proposal", with the rule's reason. Requested, ringing, answered and acknowledged phone receipts stay separate, as in the original console.

`src/app/core/live.ts` derives the presentation view from the same receipts:
- the evidence-backed stage rundown;
- the request-change ("branch") moment;
- the outcome;
- "Do you need to do anything?";
- the private permission (handoff) timeline (see `docs/phone-handoff.md`);
- the NVIDIA evidence, kept separate: configured, live availability, requests made, latency, proposals, Rust decisions and what was carried out.

`src/app/core/community-story.ts` builds "Sarah spotted it. Maya was ready." from tactic and application receipts. It marks each stage done, pending or not available.

## Media

Generated with Higgsfield (see `asset-sources/higgsfield/manifest.json` for job IDs, prompts, models, settings, dimensions and quoted credit costs). `node scripts/optimize-assets.mjs` (sharp + system ffmpeg) rebuilds `public/media/` from the downloaded sources. All interface text is real HTML; no generated image contains text.

## Screenshots

`node scripts/capture.mjs <outDir>` captures the landing page and the demo dashboard at 1440, 768 and 390 px with `/api` blocked. Committed review captures live in `../docs/frontend/screenshots/`.

## Not live (and labeled so)

These are not provided by the running server, and the UI says so where they would appear:
- **Reported-number history:** wired to the backend's in-progress contract (`docs/reputation-contract.md`); shown only when `/api/state` reports a `reputation` capability.
- **Voice enrollment:** `docs/voice-enrollment-contract.md`.
- **Whether permission calls are armed:** `docs/phone-handoff.md`.
- **Family callback, voice previews and per-call recordings** for ordinary sessions.

`node scripts/walkthrough.mjs <outDir>` records the labeled fixture walkthrough in `../docs/frontend/walkthrough.webm`.
