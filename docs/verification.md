# Verification

## First real inbound phone proof — 2026-11-07 20:42 Central

**Inbound probe PASS.** The user manually called the purchased Photon line and confirmed hearing the assistant and pressing 1. Rust accepted the signed provider webhook at 02:42:33.069083 UTC, recorded `transport.keypad` with key `1` on the **caller** leg at 02:42:43.779833, and received `call.ended` at 02:42:49.144009. A read-only Twilio Calls API lookup independently returned HTTP 200, `direction=inbound`, `status=completed`, duration **15 seconds**. The sanitized [evidence receipt](evidence/inbound-probe-2026-11-07.json) contains event timestamps and a hashed call identifier; no recording was captured.

The user reported poor voice quality. This call used an unspecified `<Say>` voice. Following it, the Say/Gather prompts explicitly select `Polly.Joanna-Neural`, `en-US`, from Twilio's [supported voice inventory](https://www.twilio.com/docs/voice/twiml/say/text-speech). This configuration change still needs a listening comparison; it is preset speech, not cloning. Relay has separate voice settings and remains untested on a real call.

The service is running **live / probe**. `TEST_CALLS_AUTHORIZED`, `HANDOFF_CALLS_ENABLED` and `FAMILY_CALLS_ENABLED` remain false; the action ledger contains **zero** outbound reservations. The one Sarah attempt is unspent. Caller A is designated; `SARAH_TEST_E164` still needs a distinct consenting Phone B. Photon number, inbound SIP and outbound SIP configuration are present. **Outbound phone audio, live Relay and spoken Nemotron remain unverified.** Brev stays deferred. The NVIDIA results below remain unchanged.

After selecting the explicit voice, `cargo fmt --all -- --check`, workspace Clippy with `-D warnings`, all **113 workspace tests**, workspace build and the existing non-network preflight passed. The preflight made zero requests; its `phone_tested=false` describes that configuration-only command, not the separately observed call above. No frontend changes were made. The rebuilt service restarted at 20:48 Central; local/public readiness returned 200 and unsigned public inbound remained 403. The one live session and zero outbound reservations survived the restart.

## Real NVIDIA integration — 2026-11-07 20:05 Central

**Protocol PASS; held-out behavior FAIL; phones DEPENDENT ON SETUP.** These are actual authenticated hosted NVIDIA requests using synthetic caller text, not simulated provider responses. The reported model was exactly `nvidia/nemotron-3.5-lightning-30b-a3b`. No phone calls or tool side effects were executed. All calling flags remain false, the durable action ledger is empty, and Sarah's one attempt remains unspent. Brev is deferred.

| Executed check | Actual result |
|---|---|
| Non-network preflight | Exit 0; zero external requests. NVIDIA/Twilio keys present; missing phone variables listed in [environment setup](environment-setup.md#remaining-account-actions). |
| `--model-probe`, normal 3500ms limit | **Exit 0**, four successful requests, two matching native tool/result round trips. Request durations: **1524, 2675, 992, 741ms**. Two sequence sums: **4199ms / 1733ms**. |
| `--evaluate-held-out`, normal 3500ms limit | **Exit 1: rubric mismatches**. All 24 requests succeeded; 12/12 assessments had valid evidence references. **3/12 cases passed the available composite checks**, classification 6/11 scored, next-tool allowlist 8/12. |
| Held-out latency | HTTP through response/tool parsing: 629–1979ms, median 1049.5ms, 24 samples. Two-request sequence sums 1593–3352ms. No audible latency measured. |
| Fresh local Rust checks | `fmt --all -- --check`, Clippy workspace/all-targets with warnings denied, **113 workspace tests**, and workspace build: all exit 0. |

Final probe questions were: “Is this a routine appointment change or does it involve a payment or credential request?” and “Which entrance would you like the bags left at?” They pass the existing distinct/context-anchor gate. Human review is less favorable: the clinic question introduces unnecessary credential/payment suspicion, and the delivery question reflects back the preference the caller wanted to learn. This establishes native protocol access, not polished live interaction.

Held-out failures are retained: four of six benign cases were classified `clarify`; the ambiguous-code case was prematurely classified `risky`; all four risky cases were assessed as risky but proposed `ask_caller` instead of an allowed protective next tool. Authentication detection was 3/3 on the three positive cases, with no missed positives in this small set; that is not an end-to-end safety success. The 0/6 benign-as-risky metric hides four unnecessary clarifications. Injection rejection has no observation class and remains unscored; its composite pass covers only the available evidence/next-tool checks. Quote validity does not prove interpretation accuracy. No prompt, rubric or frozen fixture was tuned to these results.

Original evidence: [passing probe](evidence/nvidia-model-probe-2026-11-07.json), [complete held-out results](evidence/nvidia-held-out-2026-11-07.json), and [all 11 probe attempts, including failures](evidence/nvidia-probe-attempts-2026-11-07.json). Reports also remain in ignored `.tools/`. Held-out fixture SHA256: `5aa764084cbb70bd3f877a95a223073c520f2fd73c0a863b457dd5151dceed79`. Build: local changes over `89585bb`; no model/policy substitution.

Earlier probe failures are not removed from the denominator: across 24 probe requests there were 16 valid native responses, 4 schema failures and 4 request timeouts. The 10 unsuccessful runs included malformed nested observations under `auto`, a context-anchor failure, and repeated assessment under unrestricted `required`. Three diagnostic runs explicitly allowed 15000ms; one response took 7864ms. The final pass used 3500ms. There were no 401/403/429/model-access HTTP failures. Timeouts were never classified as invalid credentials. The app's 3500ms request and 6000ms turn limits remain unchanged; these samples do not establish tail latency reliability.

Fixes verified with failing regressions before implementation: schema-shaped diagnostics omit free text; malformed/stringified observations stay rejected; explicit mandatory phases send named tools and reject a valid wrong-phase response; the real actor advances only after accepted assessment and preserves native result IDs/cancellation; diagnostic timeout overrides cannot affect evaluation/runtime; failed probe questions remain inspectable. [Contract correction](contract-corrections.md#hosted-native-tool-request-mode-2026-11-07) explains the phase-specific request change. Frontend was unchanged; its last tests remain the historical 19:24 results below, not a fresh UI claim.

Provider configuration update (2026-11-07 20:28 Central): the initial read-only audit found Twilio Trial and no Photon profile. The user subsequently created the default profile and upgraded Twilio; API readback now confirms Full/active. The agent then configured/read back Twilio SIP domain, credential list/login, call-auth mapping, Voice/status URLs and secure media; Photon inbound points to that domain using TLS with SRTP required. An initial non-JSON403 from Photon was traced to HTTP-client filtering; the authenticated request with working headers succeeded without credential changes. Outbound configuration then returned a distinct JSON **402 ENTITLEMENT_REQUIRED**, retained as an account gate. Photon still has zero voice lines. Read-only billing checks then confirmed $100 USD credit and no card on file, identifying the documented outbound entitlement gate. No billing action, number purchase, phone call or application action reservation occurred.

After rebuilding, the normal Rust process was restarted from `.env` in replay mode. Local/public health returned 200; unsigned public inbound returned 403. All calling flags are false; the durable action ledger is still empty. This is reachability/rejection evidence, not a real inbound or outbound phone result. Exact remaining steps are in [environment setup](environment-setup.md#remaining-account-actions).

Latest provider check (2026-11-07 20:37 Central): after the user added the card, outbound SIP configuration succeeded using the original reserved idempotency key. MD5, actual TLS host/port, saved password and unchanged inbound TLS/SRTP were read back. Line count remains0. Non-mutating number preview: one US local number $2/month on Messaging Free; today $2 credit applied and $0 card charge, renewal December7 ($2 before credits). No number was purchased. Default preflight and ledger checks remain separate from phone evidence.

## Preserved continuation checkpoint — 19:24 Central

Fresh checks after the three focused fixes: Cargo fmt, Clippy (`--workspace --all-targets -- -D warnings`), build and **106 workspace tests pass**. Frontend lint/typecheck/build/format, **11 unit tests and 5 Chromium tests pass**. Existing actual Rust API/SSE regression integration also passes after rebuilding/restarting; old replay state/artifacts were preserved.

Observed failing regressions before fixes:

- Relay assistant diagnostics invalidated a current model result, held private consent, and subscribed to an unverified speaker envelope. Fixed by removing that optional subscription and keeping diagnostics inert. Documented partial prompts/interruptions still hold consent; valid signatures/setup binding remain mandatory. Tested shapes are defensive fixtures, not live captures.
- Inbound-only probe configuration returned 503 because outbound SIP was absent. Health and console now split inbound, outbound, Relay and model configuration. Selected probe readiness is independent of outbound/NVIDIA; outbound action still rejects before reservation when SIP setup is incomplete. No configuration produces a verified badge.
- `take_message` dropped its summary and claimed success even on storage failure. It now minimizes/bounds the validated summary, saves a candidate aggregate before adopting it, emits a metadata receipt before caller audio, and displays it privately. Database failure/recovery leaves no phantom saved message; old snapshots remain readable. Numeric/contact literals are redacted and sensitive wording rejected conservatively; this is not a complete semantic PII scrubber. Desktop/mobile screenshots inspected.

The existing native model probe gained per-request elapsed time, requested/returned model identifiers and sanitized failure classes; request and conversation deadlines are unchanged. Local fake-provider tests prove these measurements retain returned identity and distinguish authentication, authorization, rate-limit/quota and schema errors without logging response bodies. They do not measure hosted inference.

| Live integration gate | Actual outcome |
|---|---|
| Local `.env` / non-network preflight | Created from example, mode 600, ignored by Git; local operator token generated without displaying it, authenticated API200. No provider secrets invented. Default preflight exits 0 with zero requests. |
| Public ingress | Project-local checksum-verified cloudflared2026.10.7; HTTPS health200, unsigned voice POST403 and WS upgrade403. No browser authentication. This is reachability/rejection evidence only. |
| Hosted Nemotron protocol probe | Exact command executed; exit 1: `Pending: NVIDIA_API_KEY is missing; no model requests submitted`. Identity, native round trips, follow-ups and hosted latency: unobserved. |
| Held-out evaluation | Not run: protocol gate has not passed. No scores or accuracy claims. |
| Inbound A | **DEPENDENT ON SETUP**: no signed real provider webhook, audible greeting or physical keypad evidence. |
| Outbound Sarah/B | **DEPENDENT ON SETUP**: one authorized attempt unspent; no request/ring/answer/audio/ack/terminal live evidence. |
| Live Relay | **DEPENDENT ON SETUP**: signed upgrade/setup and real prompt/interrupt ordering unobserved. Optional speaker events disabled. |
| Brev speech | User explicitly deferred provisioning. Actual GPU inventory/rate, NGC entitlement/gated access and supported tag unverified; no GPU allocated, no synthesis/playback measurement. |

Current temporary ingress is recorded in ignored `.env`; operator SSE stays on localhost because Quick Tunnels lack SSE. At 19:27 the normal Rust process was restarted from `.env` in replay mode; its configured database has zero sessions/actions, and the old isolated replay database/artifacts are preserved. Additional handoff/family flags stay false. The next exact live test is manual A greeting/key, then the single existing app-controlled Sarah/B attempt, with no separate terminal dial. Later rehearsal scope is in [the runbook](demo-runbook.md).

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

- **M1 PARTIAL:** manual A greeting/key and inbound terminal callback passed. Photon outbound SIP is configured; a distinct `SARAH_TEST_E164` endpoint and the one authorized B ringing/answer/private audio/key/terminal test remain pending.
- **M2 PARTIAL:** hosted authentication/native tool protocol passed; held-out behavior failed as recorded above. Relay onboarding, signed prompt/interrupt capture and useful audible follow-ups remain pending. The continuation disables optional unverified speaker events and fixes assistant diagnostics holding consent.
- **M3–M4 DEPENDENT ON SETUP AND SCOPE:** additional private handoff tests, enrolled/authorized family endpoint, two-way bridge audio/privacy, physical stale key during the bounded old Gather, child completion after A hangs up.
- **M5–M6 DEPENDENT ON SETUP:** fresh model paraphrase/categorical retrieval on phones, improved model behavior evaluated on a new frozen set after any tuning, live rehearsal recording and submission receipt.

Gather is a turn-based fallback that pauses A during private choice; it cannot prove simultaneous caller speech and stale-key revocation. Native TTS is not cloning. No clone/text endpoint is ready. The single-instance demo is not a production carrier deployment and does not monitor the human conversation after approved bridge commitment.
