# FirstRing console

React/TypeScript/Vite console for the Rust service. It consumes `../docs/event-contract.md`; there is no frontend scenario engine or phone-control policy.

Run from this directory with Node 22.12+ (verified with Node 24.21):

```sh
npm ci
npm run dev
```

Vite proxies `/api` to `http://127.0.0.1:8080`. Start the Rust server separately. Enter its `DEMO_ADMIN_TOKEN` in the browser; the token remains in memory, never localStorage, sessionStorage or a URL. Reloading or locking clears access. `npm run build` creates `dist/` for the Rust static-file service.

Checks:

```sh
npm run lint
npm run typecheck
npm test
npm run build
PLAYWRIGHT_BROWSERS_PATH=0 npx --no-install playwright install chromium
npm run test:browser
```

The browser suite uses **explicit synthetic replay responses**, checks desktop/tablet/phone layouts, token rejection and storage isolation, withdrawn offers, honest phone progression, and a stalled event connection. Screenshots are under `test-results/`. Browser fixtures do not prove provider integration or real phone audio. The installed browser stays beneath this project's `node_modules`.

`src/types.ts` validates incoming state/events. The SSE client sends the bearer token in a header, resumes with `Last-Event-ID`, deduplicates receipts and refreshes authoritative snapshots. A completed call is not displayed as acknowledged without `acknowledged: true`. Household memory is displayed as applied only from the server's stored version receipt. Historical replay sessions cannot trigger the live Phone B control.

The local minimatch 11.0.1 override fixes GHSA-g3hv-75wg-m249 in lint tooling; lint/build/tests are verified with this override. No dependency audit findings were reported after the override.

For the separate **actual local service integration** check, start the Rust server in replay mode at `http://127.0.0.1:8080` and place its generated operator token in `../.tools/replay-admin-token`. Then run `npm run test:integration`. The script refuses live mode, does not print the token, and exercises real API/SSE/actor/subscriber behavior via the browser. It creates local replay sessions and leaves the next household set to Maya. Screenshots are under `test-results/actual-local-replay/`. This is a local implementation check, not a real-phone or fresh-model test.

To record a roughly 56-second **authored replay demonstration** against that same running local service:

```sh
node scripts/record-replay.mjs
```

The recording shows Sarah’s benign consent, revocation and stale acceptance rejection, an independent family acknowledgement, a new actual `community.applied` receipt for Maya, Maya’s matching tactic, and a benign new-number control. Inputs are authored; the Rust actor, durable state, event stream and subscriber run normally. No responses are mocked. This is **not a backup recording of live phone proof** and contains no phone calls or fresh model inference. A video-only caption states that throughout.

The script requires replay mode and a synthetic data origin, permits only localhost, blocks unexpected browser destinations/actions, and refuses a displayed phone number. It reads the operator token silently from `../.tools/replay-admin-token`, authenticates before recording, and checks that browser storage stays empty. It creates `../.tools/artifacts/firstring-replay.webm` and `../.tools/artifacts/firstring-replay.png`, outside Git. Earlier artifacts receive a timestamped `.previous` suffix on reruns. The PNG shows the actual Maya match state; the video ends on the benign control. It creates replay sessions and leaves the next household set to Maya. Keep the local service running without restarts during capture. `FIRSTRING_TEST_URL` may select another localhost port.
