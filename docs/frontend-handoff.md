# Frontend handoff: Angular landing page and dashboard (Nov 8, 2026)

**Branch:** `frontend/angular-console`, created from `0632568`, in a separate worktree (`../hackutd26-frontend`). It touches only `frontend/`, `infra/terraform/`, docs and design records. No Rust, telephony, model, Brev, credential or call-allowance files were changed, and the running phone process was never restarted.

**Stack decision:** the user asked for Angular + Tailwind instead of React. The new app lives in `frontend/`. The React console in `web/` is left untouched, because `main` has uncommitted Codex edits there. Retire it after integration.

## Run

```sh
cd frontend && npm ci && npm start   # http://127.0.0.1:4310, /api proxied to http://127.0.0.1:8080
```

- `/`: landing page.
- `/console`: operator token form. The token is held in memory only.
- `/console?view=demo`: labeled fictional demo.

## Integration with the Rust server (backend owner)

1. **Serve the new build.** `npm run build` outputs `frontend/dist/firstring-web/browser/`. `server/src/http.rs` currently serves `web/dist` through `ServeDir::new("web/dist").append_index_html_on_directories(true)`.
2. **Add an SPA fallback.** Deep links such as `/console/calls/<id>` must return `index.html`, for example `ServeDir::new(...).fallback(ServeFile::new(".../index.html"))`. Without it, reloading a dashboard URL returns 404. The React console had no client routes, so it never needed this.
3. No API changes are required. The app uses the existing endpoints:
   - `GET /api/state`
   - `GET /api/events` (fetch-based SSE with `Last-Event-ID`)
   - `GET/PUT /api/profile/:household`
   - `POST /api/demo/{household,probe,end-call,reset,replay}`
   - `GET /api/demo/audio-fixture[/recording]`
4. Verified against the running server (read-only GETs, token never printed). `/api/state`, `/api/profile/sarah` and `/api/demo/audio-fixture` all validate against the new Zod schemas. Through the dev proxy the console showed **LIVE** with 31 real sessions classified as 17 messages saved, 6 suspicious requests blocked and 8 incomplete. One real blocked call rendered its full decision trace, receipts #2272–#2397. The browser sent no write requests. Those screenshots contain real test-call content and were not committed.

### Recommended (optional) backend improvements

- `GET /api/session/:id/events`: each connect currently replays the whole event history (about 4,000 receipts) to build per-call traces. A per-session endpoint would make call detail load faster.
- `started_at` / `ended_at` on the session snapshot. Times are currently derived from `call.inbound.received` and `call.ended` receipts.
- A per-session recording endpoint, if recordings are ever stored. The dashboard currently says "No audio recording is available for this call". Only the latest automated test caller recording can be loaded, on request, from Operator detail.
- Number reputation and corrections: see `frontend/docs/reputation-contract.md`. They are labeled "Not connected" or "Concept" everywhere.

## Completed

**Landing (`/`):**
- Navigation, with a mobile menu below 1024px.
- Hero with the Ringo loop, signal arcs and a screened-call lower third.
- Why it matters.
- How it works, with a staged call (GSAP, replay and "show all steps").
- An interactive two-call example.
- A ring transition from personal to community (scroll-linked, no pinning).
- Community: same tactic with different numbers, the water loop, spam vs fraud vs unwanted call, the labeled reputation concept and a fairness drawer covering duplicates, corrections, expiry and spoofing.
- The mission statement, privacy and voice section, and a 6-question FAQ.
- Final action, and a footer with the technology credit.

**Dashboard (`/console`):**
- **Overview:** truthful service state, most important call, recent rundown, counts (explicitly not a protection score), community update, and who answers the next call.
- **Calls:** search, outcome and household filters, and a sticky list. Each call shows a bulletin, what the caller wanted, what FirstRing did, the private saved message, a closed-caption transcript with evidence marks, and the decision spine (every receipt by caller turn, with raw JSON). Technical details cover model time, tokens, proposals vs executed, versions and epochs. Phone legs and offers show their receipts. Operator actions (End session with confirmation, Phone B with the original gating) are live-only.
- **Community:** active shared warnings from `state.tactics`, explaining why each was flagged and when it expires. It shows each household's applied version and recent community receipts. Number reputation is labeled "Not connected", with an optional labeled example.
- **Settings:** the profile editor (name, greeting, voice preset, timezone) with loading, saving, saved-version, error and undo states and client validation (no links or phone numbers), and the selector for who answers the next call. It notes that voice cloning is off and voice previews are unavailable.
- **Operator detail drawer:** connection, model backend, speech configuration, recorded evidence, provider readiness ("configured is not tested"), Phone B status, replay and reset controls (reset needs confirmation), the read-only automated caller status with an on-request recording, and the filterable raw receipt stream.
- **States:** locked, connecting, LIVE, REPLAY, DEMO, signal lost (paused banner, last snapshot kept), rejected token, unreachable server, empty calls, and a call missing from the snapshot.

**Motion:**
- GSAP: hero entrance (decorative layers only; text is readable from the first paint), layer depth, the ring transition and the staged call.
- One visible pause control covers the hero loop and the ring pulse. The community video has its own control.
- Video pauses off-screen.
- Reduced motion shows complete static alternatives.

## Generated assets (Higgsfield)

There were 10 jobs in a 12-job budget. The balance went from 270 to 240.5 credits (29.5 spent). One `kling3_0` "pro" request was refused before submission because it needs the "plus" plan; it was retried as "std". The full manifest (job IDs, prompts, models, settings, dimensions, verification notes) is in `frontend/asset-sources/higgsfield/manifest.json`.

| Asset | Shipping files |
|---|---|
| Ringo reference (concept B, chosen from 3) | `public/media/ringo-reference-640.webp` |
| Hero still (desktop) + square crop (mobile) | `ringo-hero-{1280,1920,2688}.{avif,webp}`, `ringo-hero-square-{720,1200}.{avif,webp}` |
| Hero loop, 5 s, seamless (start = end frame) | `ringo-hero-loop{,-square}.{mp4,webm}`, `ringo-hero-loop-poster.jpg` |
| Poses: greeting, listening, rest (transparent) | `ringo-{greeting,listening,rest}-{480,960}.webp` |
| Community still + 5 s ripple loop | `community-rings-*.{avif,webp}`, `community-loop.{mp4,webm}`, `community-loop-poster.jpg` |

## Live vs demo-only

- **Live (real API):** state, sessions, transcripts, the decision trace from real receipts, the profile editor, next-call household, tactic capsules and household application receipts, and operator controls.
- **Demo-only:** `/console?view=demo` (five authored fictional calls, labeled DEMO, never sent to a server) and the landing page's written examples.
- **Concept-only:** number reputation, corrections, and the "stronger screening" progression.

## Verification (Nov 8, 03:32 Central)

- `npm run format:check`, `npm run lint` (angular-eslint incl. template a11y) and `npm run typecheck` (strict + strictTemplates): pass.
- `npm test`: 20 Vitest tests pass. They cover the SSE parser, contract and demo validity, outcome classification, the decision trace, receipt dedupe and leg receipts, tactic matching, timezone formatting, and the store (memory-only token, incompatible contract, demo never posts).
- `npm run test:browser`: 24 Playwright tests pass, at 1440 desktop plus 390 mobile-tagged. They cover:
  - first-viewport actions;
  - the interactive example;
  - the concept, privacy and FAQ truthfulness;
  - reduced motion and the pause control;
  - the skip link, mobile menu and Escape;
  - no horizontal scroll at 390, 768 and 1440;
  - the demo labeling, filters, search, decision trace and raw receipt;
  - the operator drawer's focus and Escape handling;
  - demo settings limits and validation;
  - mocked live flows: rejected token, unreachable server, LIVE with a bearer token and empty storage, signal lost, profile save success and rejection, and confirmed end-session payload.
- `npm run build`: initial bundle 305 KB (80 KB transferred).
- Impeccable: `detect` returned 0 findings after fixes, against the rendered landing and console at 1280 and 390. The finish reviewer's first disposition was **fix** (8 items). The verdict pass resolved 7. Item 5 (sticky list overflow cue) was partial, and two tablet issues were raised (header wrap at 768, a faint hero image box and compression specks). All three were fixed afterwards and verified by screenshot, but not re-reviewed: the two-round review budget was used.
- Screenshots are in `docs/frontend/screenshots/`. A 37-second walkthrough with motion is in `docs/frontend/walkthrough-demo.webm`.

## Known limitations

- The finish reviewer judged reduced-motion captures. Motion was checked by tests (the video plays and pauses) and by the walkthrough, not by the reviewer.
- On connect, the full event history streams in. The trace fills in within a few seconds on the current database.
- The demo uses times relative to "now", so its dates change between visits.
- No audio examples ship. No approved FirstRing voice sample exists to play, so the page doesn't imitate one with browser speech.
- `AGENTS.md` still says to use React for a new frontend. A one-line note recording the user's Angular decision was added to the stack bullet.
