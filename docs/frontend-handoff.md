# Frontend handoff: judge-facing pass (Nov 8, 2026)

**Branch:** `frontend/angular-console` in the worktree `../hackutd26-frontend`, with `origin/main` (44ecc40) merged in.

**Commit:** `__COMMIT__` (see `git log`). Review: https://github.com/cxdima/hackutd26/compare/main...frontend/angular-console

This pass touches only:
- `frontend/`
- docs and design records: `docs/frontend*`, `DESIGN.md`, `.impeccable/`

It did not modify Rust, telephony, model serving, Brev, credentials or call allowances, and it did not restart the running server. The backend terminal's uncommitted work in the main checkout was read, never edited.

## Preview

```sh
cd frontend && npm ci && npm start     # http://127.0.0.1:4310 (proxies /api to :8080)
```

| URL | What |
|---|---|
| `/` | Landing page. New "What works today" section separates working-now from next. |
| `/console` | Dashboard. The operator token is held in memory only. |
| `/console/present` | **Presentation view** to project during the judge call. |
| `/console/present?view=demo` | Presentation with three labeled fictional rehearsal calls. Nothing leaves the browser. |
| `/console/settings` | Profile, private permission call readiness, optional voice (example only). |

## What is connected to real backend data

Verified read-only against the running server at 07:47 CT through the dev proxy:
- token read from `.env`, never printed;
- all non-GET requests blocked;
- no page errors;
- screenshots kept out of Git.

| Capability | Status |
|---|---|
| State, sessions, transcripts, decision trace, profile, tactics, household versions | **Live.** The contract updates in this pass are open capability statuses, offer `leg_id`/`offer_history`, `bridge_committed`, speech timeout and fallback fields, and tactic `schema_version`. |
| Presentation view (caption, current step, outcome, stage rundown, branch, evidence drawer) | **Live**, from receipts. It showed "Connected. Waiting for a call." The server is in `transport: probe` and its model is not configured right now, and the readiness list says exactly that. |
| Failed tool results | `agent.tool.result ok:false` (15 on the server) are shown as "Not done … Rust rejected this proposal", with the rule's reason. They never become a saved message or follow-up. |
| NVIDIA evidence | Configured backend, live availability (`model_backend.current`, read when the server sends it), requests made, median and last latency, proposals, Rust allowed/held, and what was carried out. The credit "Decision model: NVIDIA Nemotron" appears only after a successful model receipt. In DEMO it reads "Demo data: no model was called", and proposals show "Authored receipt". |
| Community story ("Sarah spotted it. Maya was ready.") | **Live**, from `tactic.created`, `community.applied` and `tactic.matched`. On the real data, "spotted on a call" is **not available**, because both capsules came from a designated synthetic test. "Saved" and "Maya was ready" are done; "later call matched" is not yet. |
| Private permission (handoff) timeline | **Wired to the real receipts** (`docs`: `frontend/docs/phone-handoff.md`). No handoff has happened on the server yet, so it has only been exercised with fixtures. |
| Per-session receipts `GET /api/session/{id}/events` | Wired. A call page backfills from it, and the live stream stays authoritative. The running binary predates it and returns 404, which is ignored quietly. |
| Reported-number history | Wired to the backend's in-progress contract (`/api/session/{id}/reputation`, the `reputation` capability). Until the server reports the capability, the console says "Not connected" and sends no request. |

## Simulated or awaiting backend support

- **Fictional rehearsal calls** (DEMO view only): "Ordinary call, connected", "The request changes" (offer withdrawn, late keypad rejected, warning shared) and "Maya was ready" (tactic matched from a different number). They use the real receipt shapes and render through the same code, labeled "Fictional rehearsal".
- **Reputation in demo:** fictional, demo-scope history on demo calls, labeled "Example · fictional reports".
- **Voice enrollment:** the server reports `voice_clone: disabled`. The flow (choose, consent, record or upload, processing and checks, preview disabled, apply, turn off and delete) runs as a labeled in-browser example. Contract: `frontend/docs/voice-enrollment-contract.md`.
- **Whether permission calls are armed:** not reported by `/api/state`. A one-line `handoff` capability is proposed in `frontend/docs/phone-handoff.md`; the console picks it up automatically.

## Production integration (exact steps)

The backend terminal is already implementing serving in **uncommitted** `server/src/console_http.rs`, wired in `http.rs`. I reviewed it read-only against the requirements:
- `/`, `/console` and `/console/{*path}` return the Angular `index.html`, so deep links survive a refresh.
- Static assets come from `frontend/dist/firstring-web/browser`.
- `/api`, `/voice`, `/health` and bare `/media` paths that don't exist return JSON 404s, never the page.
- Existing voice, webhook and WebSocket routes are unchanged.
- The React console stays at `/legacy-console/`.
- It falls back to `web/dist` if the Angular build is missing.

I had independently built and tested an equivalent patch: 3 new route tests, and the full Rust suite passed in my worktree. I dropped it to avoid conflicting with that work.

1. The backend terminal commits its `console_http.rs` work (owner: backend).
2. Merge this branch into `main` (no conflicts with 44ecc40; frontend paths only).
3. `cd frontend && npm ci && npm run build`. This writes `frontend/dist/firstring-web/browser/`, which is gitignored and built on the serving machine.
4. Restart the Rust server from the repository root when the backend owner chooses. `frontend_service()` picks the Angular build at startup.
5. Check:
   - `GET /` and `GET /console/calls/<id>` return HTML;
   - `GET /api/missing` returns a JSON 404;
   - `/legacy-console/` still serves the React console;
   - the presentation connects with the operator token.

## 90-second presentation sequence

1. **0:00. Projector on `/console/present`** (connected, LIVE bug). The waiting cover shows the number and an optional tel: QR. Say: "Before you answer, FirstRing checks."
2. **0:10. The judge calls.** Their words appear as the caption. The lower third reads "Checking the request", then "Asking a follow-up" with FirstRing's question.
3. **0:35. The judge switches** to "read me the code we just texted". The branch strip shows "First: an ordinary request → Then the request changed". The outcome turns to **Suspicious request blocked**.
   - If permission calls are armed and a phone is enrolled, the ordinary opening can instead ring the presenter's phone: private summary, press 1, **Connected**. A late "1" after a change is rejected on screen.
4. **0:55. Open Evidence:** NVIDIA Nemotron requests and latency, what the model proposed, which Rust rule held it, and what was carried out. Close with Escape.
5. **1:10. Switch to Community:** the shared warning. On live data the source stage honestly reads "not available" (test-created capsules). For the full "Sarah spotted it. Maya was ready." story, use the DEMO rehearsal, which is labeled fictional.
6. **1:25. Close** on the mission line.

**Fallback:** if the readiness list shows the phone line or model is unavailable, run `/console/present?view=demo` → "The request changes". It is labeled fictional rehearsal throughout.

## Essential remaining user actions

- **For the live phone demo, the backend must be put in conversation mode.** Right now `transport: probe` (callers hear a keypad test) and the model is not configured. That is the backend terminal's job, using your existing Brev/hosted authorization.
- **For the "my phone rings, I press 1" demonstration:** enrolling your own phone as Sarah's endpoint and enabling `HANDOFF_CALLS_ENABLED` and outbound calling require your explicit authorization to the backend terminal. AGENTS.md currently keeps outbound flags false. Nothing in the console can or will arm it.
- Merge, build and restart as above.

## Verification (this pass)

- `npm run format:check`, `npm run lint` and `npm run typecheck` (strict templates): pass.
- `npm test`: **54** Vitest tests pass. New coverage:
  - failed or rejected tool results never become success;
  - SSE reconnect with `Last-Event-ID` and duplicate receipts;
  - stale stream;
  - demo rehearsal never touches the network, and is refused on a live server;
  - missing capabilities (reputation is never requested without one; NVIDIA credit only after inference);
  - call selection (history never takes over; live calls followed; pinned stays; a call that finished before reset stays off);
  - handoff transitions (provider accepted is not ringing; connected only on bridge; revoked and stale);
  - community stages;
  - QR encodes only `tel:`;
  - voice sample checks.
- `npm run test:browser`: **41** Playwright tests pass (desktop 1440, plus mobile-tagged at 390). New:
  - rehearsal flow with zero `/api` and zero write requests;
  - evidence drawer focus and Escape;
  - community completion;
  - reset;
  - reduced motion;
  - phone width;
  - mocked live: waiting vs service unavailable, history kept off screen, number and QR, following a new call;
  - setup pages place no call and voice runs only as an example;
  - rejected proposal shown as "Not done";
  - reputation Connected vs Not connected.
- `npm run build`: initial bundle 318 KB raw, 80 KB transferred. The presentation view is a lazy chunk.
- **Impeccable:**
  - detector: 0 findings on the presentation and Settings pages.
  - Remaining notes, judged not defects:
    - "cramped padding" on vertically centred min-height pills;
    - a long Community column next to a now-sticky aside;
    - the landing display heading's deliberate 1.18 leading;
    - two roughly 86-character lines in call detail.
  - Finish review (one round, presentation surface): disposition **fix**, 10 items, all applied (see the end of this file). Confirmed by a re-capture at 1440 and 390; no second review round.
- Visual review at 390, 768 and 1440 (`docs/frontend/screenshots/`, fixture data only). Labeled 63-second walkthrough: `docs/frontend/walkthrough.webm` (landing, mocked-server dashboard, presentation rehearsal, community, setup).

## Known limitations

- The live handoff path has never run on the server. Its UI is proven against fixtures that mirror the Rust receipts.
- After "Skip to end", a rehearsal's stage times collapse to the moment of skipping (they are retimed, never left in the future).
- The presentation's "Follow newest" counts a call as live while its receipts are under 2 minutes old.
- Reputation list, report and correction actions are not exposed in the console yet (household-facing permissions undecided).
- `.impeccable/design.json` was not regenerated this pass. `DESIGN.md` documents the new components.

## Impeccable finish review (presentation view) and what changed

The reviewer judged the shipped captures against the surface brief (`.impeccable/surfaces/frontend-src-app-console-present-present-page-ts.md`). All ten fixes were applied:

1. **DEMO honesty:** the footer credit and the drawer no longer imply a model ran on authored receipts.
2. **Revoked handoff:** the permission list shows "Permission withdrawn" and "Pressed 1 after permission was withdrawn: rejected" in magenta with words, not lime checks.
3. **Lower-third tally:** turns magenta-ink and reads "FirstRing · request blocked" or "permission withdrawn" when that is the outcome.
4. **Earlier request:** shown as "Before:", struck through, with a neutral marker instead of a lime check.
5. **No eyebrow labels:** the outcome is its own `h2`; labels are sentence case at 1.0625rem or larger; no main-stage text is below 0.95rem.
6. **Station bug:** 1.0625rem on the projected screen; the duplicate DEMO bug in the rehearsal strip was removed.
7. **Fit on a projector:** long captions step down in size, the rundown shows the newest five stages, and times drop the seconds.
8. **Phone layout:** presenter controls (Showing, rehearsal buttons, Reset, Exit) collapse into one "Presenter controls" disclosure at 390.
9. **Phone rundown:** the stage list stacks vertically, with times aligned right.
10. **Community scene:** the receipt list sits beside the rings at desktop width, so the proof stays above the fold.

