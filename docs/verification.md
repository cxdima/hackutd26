# Local verification — 2026-11-07 18:40 Central

Real phone tests: **DEPENDENT ON SETUP**. No real outbound call or live NVIDIA inference request has been made. User configuration is still in progress; `.env` is absent. One Sarah / Phone B smoke attempt is authorized, not yet spent. Family and additional handoff calls remain disabled.

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
- **M2 DEPENDENT ON SETUP:** real Nemotron access/native tools/contextual follow-ups, Relay onboarding and signed speaker-event envelope capture. Unknown envelopes conservatively hold consent; this may prevent live handoff until resolved.
- **M3–M4 DEPENDENT ON SETUP AND SCOPE:** additional private handoff tests, enrolled/authorized family endpoint, two-way bridge audio/privacy, physical stale key during the bounded old Gather, child completion after A hangs up.
- **M5–M6 DEPENDENT ON SETUP:** fresh model paraphrase/categorical retrieval on phones, held-out language evaluation, live rehearsal recording and submission receipt.

Gather is a turn-based fallback that pauses A during private choice; it cannot prove simultaneous caller speech and stale-key revocation. Native TTS is not cloning. No clone/text endpoint is ready. The single-instance demo is not a production carrier deployment and does not monitor the human conversation after approved bridge commitment.
