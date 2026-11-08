# Provider preflight

Checked 2026-11-07 against current official documentation. The first actual inbound call passed at **20:42 Central**: signed webhook, caller key 1, audible greeting confirmed by the user, and completed status confirmed through Twilio. See [inbound evidence](evidence/inbound-probe-2026-11-07.json) and [model/local results](verification.md). The purchased Photon line is assigned to the default profile; `PHOTON_NUMBER` is configured. Phone B and Relay/model conversation remain unverified.

## Selected route and gates

Photon **Beta / 2.0** voice-eligible SMS number → authenticated TLS SIP → Twilio SIP Domain → signed Rust webhook. Say/Gather proves transport first; ConversationRelay is the interactive route, with explicit Gather fallback after a 30–45 minute onboarding timebox. No raw-audio replacement.

| Capability | Current gate | Evidence required to pass |
|---|---|---|
| Public HTTPS/WSS | Real signed inbound HTTP passed; unsigned POST and WS upgrade rejected with 403 | Actual signed Relay upgrade pending; console SSE uses localhost |
| Photon inbound | **Passed Nov 7, 20:42 Central:** A heard disclosed greeting, pressed 1, completed inbound confirmed | Initial voice rated poor; `Polly.Joanna-Neural` / `en-US` selected for Say/Gather, listening comparison pending |
| Photon outbound | MD5/TLS credentials and caller line configured; `SARAH_TEST_E164` missing; no reservations/submissions | Designated B rings, hears private test, presses 1, and reaches terminal status |
| Relay | Twilio Full/active; Relay onboarding/live handshake pending; optional speaker subscription disabled | Signed upgrade, setup bound to inbound leg, fresh partial/final prompts and interruption capture |
| Nemotron | Authenticated native protocol PASS; held-out behavior FAIL (3/12 composite passes) | Native tool call → validated result with matching ID → useful follow-up; different details yield different questions |
| Handoff / revocation | Pending three-phone rehearsal | B-only summary; current acceptance connects A+B; revoked acceptance fails; family leg stays separate |
| Optional clone / text | Not established | Approved ready endpoint / functioning enrolled message channel; otherwise omit |

## Photon and Calls API

The [official Beta cookbook](https://photon.codes/docs/beta/cookbooks/voice/twilio.md) confirms two credential pairs: a Twilio credential-list login for Photon **inbound** delivery, and Photon-generated **outbound** credentials. Inbound destination is `sips:YOURDOMAIN.sip.twilio.com`, TLS selected. Configure the SIP Domain webhook as `POST PUBLIC_BASE_URL/voice/incoming`.

For outbound, select **MD5 (legacy)** in the Photon profile, retain TLS, and copy the assigned **SIP connection** host/port, not the authentication realm. `sip.photon.codes:5061` is the documented example, not proof of this account's assignment. The cookbook's HTML returned 403 during audit; its official Markdown endpoint was readable.

Form-encode `POST https://api.twilio.com/2010-04-01/Accounts/{AccountSid}/Calls.json`, with account SID/Auth Token Basic authentication for this demo:

```text
To=sip:{ENROLLED_E164}@{PHOTON_SIP_HOST}:{PHOTON_SIP_PORT};transport=tls;secure=true
From={PHOTON_NUMBER}
SipAuthUsername={PHOTON_SIP_USERNAME}
SipAuthPassword={PHOTON_SIP_PASSWORD}
Url={PUBLIC_BASE_URL}/voice/alert/{RESERVED_ACTION_ID}
StatusCallback={PUBLIC_BASE_URL}/voice/status?action_id={RESERVED_ACTION_ID}
StatusCallbackMethod=POST
StatusCallbackEvent=initiated
StatusCallbackEvent=ringing
StatusCallbackEvent=answered
StatusCallbackEvent=completed
```

Use `/voice/whisper/{offer_id}` for a private offer. `transport=tls` encrypts signaling; `secure=true` requests SRTP. For SIP destinations, `From` populates the SIP header; Photon authorizes its own number against the profile. The Twilio-verified caller-ID requirement in the cookbook's forwarding example concerns its PSTN leg. [Twilio SIP API](https://www.twilio.com/docs/voice/api/sip-making-calls)

Create acceptance is not ringing or answer. Subscribe to every progression event; `answered` callbacks carry call status `in-progress`. `SequenceNumber` orders generation, not delivery. Preserve terminal states despite reordering, and reserve before create so a callback can recover a lost response. An ambiguous timeout never authorizes another dial. [Calls API](https://www.twilio.com/docs/voice/api/call-resource)

## Signed ingress

Validate against configured public origin plus exact path/query, including the reserved action ID, not an untrusted Host header. Decode form fields once, preserving repeated fields; include all fields in signature input. The current official Python validator sorts names and distinct values, then HMAC-SHA1/Base64 with the Auth Token; it handles port variants. Requests using the newer shared-key signing configuration need their corresponding algorithm/key, and are outside this demo's default Auth Token configuration. [Security](https://www.twilio.com/docs/usage/security), [pinned official validator](https://github.com/twilio/twilio-python/blob/7e3ca781178dcaa542b403591a58e5bc58b874a8/twilio/request_validator.py)

Rust conformance targets from the [official SDK tests](https://github.com/twilio/twilio-python/blob/7e3ca781178dcaa542b403591a58e5bc58b874a8/tests/unit/test_request_validator.py):

- Token `12345`, URL `https://mycompany.com/myapp.php?foo=1&bar=2`, fields `CallSid=CA1234567890ABCDE`, `Digits=1234`, `From=+14158675309`, `To=+18005551212`, `Caller=+14158675309` → `RSOYDt4T1cUTdK1PDd93/VVr8B8=`.
- Same token/URL, fields `Sid=CA123`, `SidAccount=AC123`, `Digits=5678`, `Digits=1234`, `Digits=1234` → `IK+Dwps556ElfBT0I3Rgjkr1wJU=`.

Twilio's [SDK catalogue](https://www.twilio.com/docs/libraries) lists maintained helpers in seven languages, with Rust covered through OpenAPI generation. Registry search found community crates `twilio`, `twilio-async`, and `twilio-agnostic`; none supplies an official Rust validator. Use `reqwest` and standard RustCrypto primitives with official-vector/parity tests; retain Rust ownership of the backend as requested.

## Relay and Gather protocol

[Relay message reference](https://www.twilio.com/docs/voice/conversationrelay/websocket-messages):

| Direction/type | Required handling |
|---|---|
| In `setup` | Bind `callSid`, `accountSid`, `sessionId`, `customParameters.session_token`; no new session from a claimed ID alone |
| In `prompt` | `voicePrompt`, `lang`, `last`; only `last:true` commits the utterance |
| In `interrupt` | `utteranceUntilInterrupt`, `durationUntilInterruptMs`; cancel obsolete reply and close consent gate |
| In `dtmf` | `digit`; caller DTMF is never Sarah acceptance |
| In `error` | `description`; report dependency failure |
| Out `text` | `token`, `last:true`; optional Boolean `interruptible`/`preemptible` |
| Out `play` | `source` HTTPS audio URL, `loop:1`, optional Boolean interruption flags |
| Out `end` | Optional `handoffData` is a **string**, including when it contains encoded JSON |

Current official TwiML docs name `agentSpeaking` and `clientSpeaking` but the message reference does not specify their wire envelope/state schema. The optional `events="speaker-events"` subscription is now **disabled** until authenticated live capture verifies it. Diagnostic/info/unknown messages no longer change the caller speech state or cancel inference. Documented partial `prompt` and `interrupt` still close consent and cancel obsolete work; `reportInputDuringAgentSpeech="any"`, `partialPrompts="true"`, `interruptible="any"`, and `dtmfDetection="true"` remain enabled. Local socket regressions prove assistant diagnostics cannot invalidate a saved model result or indefinitely hold acceptance, and cannot clear a genuine caller-speech hold. These are defensive fixtures, not captured provider frames. No instantaneous speech-onset coverage is claimed before a documented prompt/interrupt arrives. [Relay TwiML](https://www.twilio.com/docs/voice/twiml/connect/conversationrelay)

Validate `X-Twilio-Signature` on the initial WebSocket handshake. Official onboarding specifies the request URL but does not explicitly resolve `wss://` versus the HTTP upgrade's `https://` representation. Use the configured external Relay URL, keep validation mandatory, and establish exact scheme/port behavior with an actual signed handshake before passing the gate. Also bind the opaque setup token and expected call/account. [Relay onboarding](https://www.twilio.com/docs/voice/conversationrelay/onboarding)

Current trials strip `ConversationRelay` and `Dial Sip`; a funded enabled account is a gate. [Trial restrictions](https://www.twilio.com/docs/usage/trials/try-out-voice). Gather DTMF should set `numDigits="1"`, explicit action, bounded timeout, and `actionOnEmptyResult="true"`. For speech fallback, don't pair explicit `speechModel` with `speechTimeout="auto"`. [Gather](https://www.twilio.com/docs/voice/twiml/gather)

## Nemotron and schedule

The [hosted API reference](https://docs.api.nvidia.com/nim/reference/nvidia-nemotron-3-5-lightning-30b-a3b-infer) confirms `https://integrate.api.nvidia.com/v1/chat/completions` and `nvidia/nemotron-3.5-lightning-30b-a3b`. [NIM Lightning documentation](https://docs.nvidia.com/nim/large-language-models/2.0.10/get-started/advanced/get-started-nemotron-3.5-lightning.html) documents native `tools` and tool-result follow-up, plus `chat_template_kwargs:{"enable_thinking":false}`. The authenticated probe now verifies hosted identity, serial native tools and matching result IDs. Mandatory assessment/publication use named choices; next-step selection uses `required` over its allowed schemas. The initial `auto` responses failed nested-object validation; see [the correction](contract-corrections.md#hosted-native-tool-request-mode-2026-11-07). Do not speak arbitrary planning content.

The Rust preflight executable reads local `.env` and never dials phones, executes proposed tools, or changes provider configuration. Run from the repository root:

```bash
bash scripts/cargo.sh run --bin preflight
bash scripts/cargo.sh run --bin preflight -- --help
# Explicitly opt in to NVIDIA requests, once NVIDIA_API_KEY is configured locally:
bash scripts/cargo.sh run --bin preflight -- --model-probe
bash scripts/cargo.sh run --bin preflight -- --evaluate-held-out
```

Default output contains environment variable **names** and presence only; presence does not validate credentials, enabled flags, authorization, or capabilities. `--model-probe` permits at most four requests: a real native assessment/result/follow-up sequence for each of two synthetic benign details. It requires distinct questions containing prewritten context anchors. This is a protocol and lexical check; a human must still assess relevance and audible interaction.

`--evaluate-held-out` permits at most 24 requests over the 12 frozen cases in `fixtures/language-held-out.json`. Both live modes use the documented endpoint/model, the adapter's 3.5-second request timeout, and a 120-second total budget. The evaluator requires `record_assessment` first, validates evidence against committed caller text, returns its matching native result ID, and checks the next tool without executing it. It compares observation flags and next-tool choices with the prewritten rubric; expected answers and rationales are never included in model prompts. Actual per-case results, confusion counts, recall, false-positive, clarification and evidence rates go to ignored `.tools/evaluation-results.json`. Missing denominators are `null`; errors are retained. The current observation schema has no instruction-rejection class, and this probe cannot establish tactic receipts, stale consent or phone outcomes; those dimensions are explicitly unscored.

Latest actual execution (Nov 7, 20:05 Central): non-network preflight exit 0; protocol probe **exit 0**, 4 requests / 2 native round trips, exact requested model, 741–2675ms per request. Held-out evaluation **exit 1**:24 successful API requests but3/12 composite passes, with interpretation and next-tool failures. Full synthetic-input/live-provider results and all earlier failures are retained in [verification](verification.md). No phone/audio result follows from these measurements.

The adapter records requested/returned model IDs, request milliseconds and sanitized failure classes. HTTP 401/403, 429, request/schema, transport, provider and timeout failures remain distinct; 429 alone cannot distinguish quota from rate limits. Production request/connect/turn limits remain3500/1500/6000ms. For diagnosis only, `NEMOTRON_PROBE_TIMEOUT_MS=15000` can extend `--model-probe` to at most 15s/request; it cannot affect held-out evaluation or runtime. Reports retain the effective limit. One earlier diagnostic response took 7864ms; final protocol and all 24 held-out requests fit normal request limits, which does not establish tail latency or audible responsiveness.

`/health/ready` now reports inbound, outbound, Relay and model configuration independently, and its HTTP status follows the selected inbound transport plus operator authentication. Probe mode does not require outbound SIP or NVIDIA. Relay/gather conversation requires model configuration. Every `live_verified` remains false; credential presence never upgrades a badge to verified. Actual outbound actions still require full Photon SIP setup, enrolled destination, authorization and the durable action reservation.

No architectural change to the master guide is necessary. Clarifications above cover SIP caller ID, Relay's string `handoffData`, and unresolved speaker/WSS capture gates. Skills' generic SDK/Host-header examples do not override the project contract.

The [official event schedule](https://zeroday.hackutd.co/#schedule) currently confirms **Nov 7, 23:00 CT initial Devpost; Nov 8, 12:00 CT final submission; 13:00–16:00 CT judging**. Protect 10:00–12:00 Sunday for rehearsal/submission/recovery. Portal-specific requirements and organizer changes still need checking.

Current runtime is `VOICE_TRANSPORT=probe`, `DEMO_EXECUTION_MODE=live`, with `TEST_CALLS_AUTHORIZED=false`, `HANDOFF_CALLS_ENABLED=false`, and `FAMILY_CALLS_ENABLED=false`. Keep all three false for A-only calls. The single authorized Sarah / Phone B attempt is unspent; the user's personal phone is caller A only. After B is designated in `SARAH_TEST_E164` and ready, enable only `TEST_CALLS_AUTHORIZED` for that attempt. Trigger `POST /api/demo/probe` with the actual inbound `session_id` and `contact_id:"sarah"`; send the operator token only in the Bearer header, never a phone number in the request. No family endpoint or additional call authorization is established.

Next live gate: populate `SARAH_TEST_E164`, the only missing core phone variable, then run the B portion of the [two-phone transport checklist](demo-runbook.md). Existing credentials and caller line are configured. This application uses `PHOTON_SIP_USERNAME` and `PHOTON_SIP_PASSWORD` for outbound Photon digest credentials, separately from `TWILIO_ACCOUNT_SID` and `TWILIO_AUTH_TOKEN`. Whole M1, Relay, private consent and independent-contact gates remain pending. Brev remains deferred.

## Account setup history — 2026-11-07 20:28 Central

Twilio now reports Full/active. Created `hackutd26-firstring-d232a271.sip.twilio.com`, Secure=true, SIP registration/emergency calling disabled, Voice POST to `/voice/incoming`, status POST to `/voice/status`, one generated inbound credential and its call-auth mapping. Photon default profile inbound now matches that SIP destination/login, and mediaEncryption=srtp. All values were read back without exposing credentials. No phone number was bought. Photon outbound PUT with MD5 returned 402 ENTITLEMENT_REQUIRED; its generated outbound credential/assigned endpoint remain unavailable. The profile has zero lines and the app action ledger remains empty.

Read-only Photon billing follow-up confirms cardOnFile=false and existing USD credit of10000 cents ($100). This identifies the outbound entitlement blocker: add a payment method through Photon Billing. No extra credit purchase is indicated. SMS allocation is a separate purchase/assignment; no number quote or order was submitted.

## Account setup history — 2026-11-07 20:37 Central

The user added a Photon payment method. Reused the reserved configuration idempotency key after reading the unchanged profile; outbound setup succeeded. Generated credentials were saved without display, and MD5/TLS endpoint plus inbound TLS/SRTP were verified by readback. No phone calls or application reservations occurred. A non-mutating billing preview quotes one US local SMS/voice number at $2/month on Messaging Free: $2 existing credit applied, $0 tax/card due now, $98 credit remaining after purchase, next renewal December7 at $2 before credits. No number purchase performed; profile lineCount remains0.
