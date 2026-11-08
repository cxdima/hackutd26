# Verification — 2026-11-07 19:24 Central

## Continuation from e9cdfd3

Fresh checks after the three focused fixes: Cargo fmt, Clippy (`--workspace --all-targets -- -D warnings`), build and **106 workspace tests pass**. Frontend lint/typecheck/build/format, **11 unit tests and 5 Chromium tests pass**. Existing actual Rust API/SSE regression integration also passes after rebuilding/restarting; old replay state/artifacts were preserved.

Observed failing regressions before fixes:

- Relay assistant diagnostics invalidated a current model result, held private consent, and subscribed to an unverified speaker envelope. Fixed by removing that optional subscription and keeping diagnostics inert. Documented partial prompts/interruptions still hold consent; valid signatures/setup binding remain mandatory. Tested shapes are defensive fixtures, not live captures.
- Inbound-only probe configuration returned 503 because outbound SIP was absent. Health and console now split inbound, outbound, Relay and model configuration. Selected probe readiness is independent of outbound/NVIDIA; outbound action still rejects before reservation when SIP setup is incomplete. No configuration produces a verified badge.
- `take_message` dropped its summary and claimed success even on storage failure. It now minimizes/bounds the validated summary, saves a candidate aggregate before adopting it, emits a metadata receipt before caller audio, and displays it privately. Database failure/recovery leaves no phantom saved message; old snapshots remain readable. Numeric/contact literals are redacted and sensitive wording rejected conservatively; this is not a complete semantic PII scrubber. Desktop/mobile screenshots inspected.

The existing native model probe gained per-request elapsed time, requested/returned model identifiers and sanitized failure classes; request and conversation deadlines are unchanged. Local fake-provider tests prove these measurements retain returned identity and distinguish authentication, authorization, rate-limit/quota and schema errors without logging response bodies. They do not measure hosted inference.

| Live integration gate | Actual outcome |
|---|---|
| Local `.env` / non-network preflight | Created from example, mode600, ignored by Git; local operator token generated without displaying it, authenticated API200. No provider secrets invented. Default preflight exits0 with zero requests. |
| Public ingress | Project-local checksum-verified cloudflared2026.10.7; HTTPS health200, unsigned voice POST403 and WS upgrade403. No browser authentication. This is reachability/rejection evidence only. |
| Hosted Nemotron protocol probe | Exact command executed; exit1: `Pending: NVIDIA_API_KEY is missing; no model requests submitted`. Identity, native round trips, follow-ups and hosted latency: unobserved. |
| Held-out evaluation | Not run: protocol gate has not passed. No scores or accuracy claims. |
| Inbound A | **DEPENDENT ON SETUP**: no signed real provider webhook, audible greeting or physical keypad evidence. |
| Outbound Sarah/B | **DEPENDENT ON SETUP**: one authorized attempt unspent; no request/ring/answer/audio/ack/terminal live evidence. |
| Live Relay | **DEPENDENT ON SETUP**: signed upgrade/setup and real prompt/interrupt ordering unobserved. Optional speaker events disabled. |
| Brev speech | User explicitly deferred provisioning. Actual GPU inventory/rate, NGC entitlement/gated access and supported tag unverified; no GPU allocated, no synthesis/playback measurement. |

Current temporary ingress is recorded in ignored `.env`; operator SSE stays on localhost because Quick Tunnels lack SSE. At19:27 the normal Rust process was restarted from `.env` in replay mode; its configured database has zero sessions/actions, and the old isolated replay database/artifacts are preserved. Additional handoff/family flags stay false. The next exact live test is manual A greeting/key, then the single existing app-controlled Sarah/B attempt, with no separate terminal dial. Later rehearsal scope is in [the runbook](demo-runbook.md).

## Preserved e9cdfd3 baseline — 18:40 Central

At that checkpoint, `.env` was absent and real phone tests were **DEPENDENT ON SETUP**. No real outbound call or live NVIDIA inference request had been made. The one Sarah / Phone B smoke attempt was unspent. These historical results are preserved separately from the continuation above.

| Executed command | Observed outcome |
|---|---|
| `bash scripts/cargo.sh fmt --all -- --check` | Exit 0 |
| `bash scripts/cargo.sh clippy --workspace --all-targets -- -D warnings` | Exit 0, no warnings |
| `bash scripts/cargo.sh test --workspace` | 91 passed, 0 failed |
| `bash scripts/cargo.sh build --workspace` | Exit 0 |
| `npm --prefix web run lint` | Exit 0 |
| `npm --prefix web run typecheck` | Exit 0 |
| `npm --prefix web test` | 10 passed, 0 failed |
| `npm --prefix web run build` | Exit 0 |
| `npm --prefix web run format:check` | Exit 0 |
| `npm --prefix web run test:browser` | 4 Chromium tests passed |
| `npm --prefix web run test:integration` | Actual local Rust API/SSE integration passed; no mocked API |
| `node web/scripts/record-replay.mjs` | Authored replay capture completed, exit 0 |
| `git check-ignore --no-index .env .env.local .tools/replay-admin-token` | All three ignored; `.env.example` remains trackable |
| `cmp FirstRing_Master_Guide.md docs/FirstRing_Master_Guide.md` | Copies identical |

The staged application/documentation diff passes whitespace checks. Unmodified upstream skill license files retain their original CRLF/trailing whitespace, so an unrestricted `git diff --cached --check` reports those vendor-only warnings. The staged-path review excludes `.idea`, `.env`, `.tools`, build output and dependencies; a credential-pattern scan found no matches. Existing `.idea` files remain untracked and untouched.

The Rust suite covers official signature vectors and tampering, signed HTTP ingress and actual local WebSocket upgrades, correct-leg keypad behavior, enrolled-contact restrictions, one-attempt durable reservation, unknown create reconciliation, replay isolation, reordered statuses, current/stale acceptance, revocation, cancellation, private summaries, two-participant conference confirmation and terminal callback races. Generated sequences exercise revoked-offer and terminal-status invariants. Payment-diversion tests distinguish an ordinary copay request. Native model-loop tests use a local fake HTTP provider; they verify tool/result protocol and cancellation, not Nemotron understanding.

Browser checks cover authentication, memory-only token handling, stale SSE state, desktop/mobile layout and real actor events. The integration check observes benign bridge receipts, rejected old acceptance, independent contact acknowledgement, subscriber-applied memory, Maya's tactic lookup, and the benign new-number control. All simulated call states remain labeled replay. Screenshot reviewed directly after capture.

The saved `.tools/artifacts/firstring-replay.webm` is 57.68 seconds at 1440×1040; `.tools/artifacts/firstring-replay.png` shows the resulting Maya match. The recording permanently identifies authored replay, no phone calls and no inference. It demonstrates actual local actor/SQLite/subscriber/SSE execution; it is not a recording of real telephone audio. Artifacts and the operator token are Git-ignored.

## Outstanding evidence

- **M1 DEPENDENT ON SETUP:** exact Photon outbound SIP assignment/credentials, Twilio account and signed public ingress, manual A greeting/key, one authorized B ringing/answer/private audio/key, terminal callbacks.
- **M2 DEPENDENT ON SETUP:** real Nemotron access/native tools/contextual follow-ups, Relay onboarding and signed prompt/interrupt capture. The continuation disables optional unverified speaker events and fixes assistant diagnostics holding consent.
- **M3–M4 DEPENDENT ON SETUP AND SCOPE:** additional private handoff tests, enrolled/authorized family endpoint, two-way bridge audio/privacy, physical stale key during the bounded old Gather, child completion after A hangs up.
- **M5–M6 DEPENDENT ON SETUP:** fresh model paraphrase/categorical retrieval on phones, held-out language evaluation, live rehearsal recording and submission receipt.

Gather is a turn-based fallback that pauses A during private choice; it cannot prove simultaneous caller speech and stale-key revocation. Native TTS is not cloning. No clone/text endpoint is ready. The single-instance demo is not a production carrier deployment and does not monitor the human conversation after approved bridge commitment.
