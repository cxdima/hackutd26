# Provider preflight

Current at **Nov 7, 21:54 Central**: `live` / `relay`, caller A only, all three outbound flags false and no Phone B. Signed Relay setup and the heard Rachel-v4 greeting now have limited live proof. Two final clinic transcripts arrived, but trailing partial speech cancelled both model jobs and left silence; no native completion or spoken reply was observed. Full Relay conversation remains unverified. Inbound greeting/keypad/terminal and the separate hosted model protocol passed earlier. The first Relay attempt returned 64105; two retries at 21:43 returned 11200 / HTTP 502 at an expired Quick Tunnel. The replacement HTTP/2 origin reached signed setup. [Inbound proof](evidence/inbound-probe-2026-11-07.json), [earlier Relay failures](evidence/relay-attempts-2026-11-07.json)

Caller-only model development passed 3/3. Fresh frozen held-out behavior passed 7/12 and remains a failed gate, not an accuracy claim. Thirty-six development audio clips were synthesized successfully but have not been listened to or phone-tested. See [evaluation](evaluation.md) and [audio fixtures](audio-fixtures.md). Prior records remain intact.

## Selected route and gates

Photon **Beta / 2.0** voice-eligible SMS number → authenticated TLS SIP → Twilio SIP Domain → signed Rust webhook. Say/Gather proves transport first; ConversationRelay is the interactive route, with explicit Gather fallback after a 30–45 minute onboarding timebox. No raw-audio replacement.

| Capability | Current gate | Evidence required to pass |
|---|---|---|
| Public HTTPS/WSS | Old Quick Tunnel expired, causing two 502 attempts; replacement reached signed Relay upgrade/setup at 21:54 | Continued reachability; console SSE stays localhost |
| Photon inbound | **Passed Nov 7, 20:42 Central:** A heard disclosed greeting, pressed 1, completed inbound confirmed | Initial voice rated poor; `Polly.Joanna-Neural` / `en-US` selected for Say/Gather, listening comparison pending |
| Photon outbound | MD5/TLS credentials and caller line configured; `SARAH_TEST_E164` missing; no reservations/submissions | Designated B rings, hears private test, presses 1, and reaches terminal status |
| Relay | Signed setup, Rachel-v4 greeting heard, two final transcripts; trailing partials cancelled both model jobs | Complete model/audio loop after speech-finalization fix, interruption and goodbye; optional speaker subscription remains disabled |
| Nemotron | Native protocol PASS; A-only development 3/3; fresh frozen v2 FAIL at 7/12 | Useful phone interaction and remaining held-out failures; protocol success is not semantic accuracy |
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

`--evaluate-held-out` now uses the new frozen `fixtures/language-held-out-v2.json`; the original set is regression data. General evaluation is bounded to 12 cases / 48 HTTP requests / 120 seconds, with at most four native steps and six seconds per turn. A-only development has three cases. The evaluator shares runtime prompts, capability context and native tool/result handling, but executes only an in-memory policy simulation: no dialing, application-ledger writes, provider receipts or delivered-message claims. Expected answers stay outside prompts. See [current commands, metrics and failures](evaluation.md); do not rerun the frozen set as tuning data.

Latest model measurements (21:13–21:18 Central): A-only development 3/3 in six HTTP requests; fresh frozen v2 7/12 in 23 requests, with one evidence rejection and zero API/deadline errors. V2 retains five behavioral failures. Its HTTP samples span 630–1760ms, median 998ms; these are not audible latency. Historical 20:05 protocol proof used four requests and two native round trips; the original 3/12 evaluation remains preserved. No phone/audio result follows from either run.

The adapter and console distinguish model HTTP observations, native proposals, accepted/rejected tool results and actual phone outcomes. Request/connect/turn limits remain 3500/1500/6000ms. `NEMOTRON_PROBE_TIMEOUT_MS` changes only the explicit protocol diagnostic (maximum 15 seconds/request), not runtime or evaluation. Model traces, Relay output telemetry and `audio.requested` do not prove playback. Saved-message closing via signed Connect action → TwiML `Say` then `Hangup` is locally verified; phone goodbye delivery remains pending. Relay uses the Rachel-v4 preset; Say/Gather keeps `Polly.Joanna-Neural`. Neither is a clone.

`/health/ready` reports configuration separately from recorded evidence; HTTP 200 describes selected-transport configuration, not current public availability. Inbound/model verification comes from their historical receipts, while Relay/outbound remain unverified. Credential presence never upgrades a proof badge. Actual outbound actions still require enrolled destinations, enabled flags, authorization and the durable attempt budget.

No architectural change to the master guide is necessary. Clarifications above cover SIP caller ID, Relay's string `handoffData`, and unresolved speaker/WSS capture gates. Skills' generic SDK/Host-header examples do not override the project contract.

The [official event schedule](https://zeroday.hackutd.co/#schedule) currently confirms **Nov 7, 23:00 CT initial Devpost; Nov 8, 12:00 CT final submission; 13:00–16:00 CT judging**. Protect 10:00–12:00 Sunday for rehearsal/submission/recovery. Portal-specific requirements and organizer changes still need checking.

Current runtime is `VOICE_TRANSPORT=relay`, `DEMO_EXECUTION_MODE=live`, with `TEST_CALLS_AUTHORIZED=false`, `HANDOFF_CALLS_ENABLED=false`, and `FAMILY_CALLS_ENABLED=false`. Keep all three false for caller-only testing. Phone B is unavailable; its single historical smoke-attempt budget is unspent. The console must leave that action unavailable. No family endpoint or later outbound scope is established.

Next live gate: verify the speech-finalization and orphan-partial recovery changes, then capture actual model results and human-heard replies during a deliberate manual call. Preserve public health and signature checks. B is not a prerequisite. Whole M1, Relay voice, private consent and independent contact remain unverified; Brev remains deferred. Keep existing credentials and ledgers intact. See the [current caller-only runbook](demo-runbook.md).

## Account setup history — 2026-11-07 20:28 Central

Twilio now reports Full/active. Created `hackutd26-firstring-d232a271.sip.twilio.com`, Secure=true, SIP registration/emergency calling disabled, Voice POST to `/voice/incoming`, status POST to `/voice/status`, one generated inbound credential and its call-auth mapping. Photon default profile inbound now matches that SIP destination/login, and mediaEncryption=srtp. All values were read back without exposing credentials. No phone number was bought. Photon outbound PUT with MD5 returned 402 ENTITLEMENT_REQUIRED; its generated outbound credential/assigned endpoint remain unavailable. The profile has zero lines and the app action ledger remains empty.

Read-only Photon billing follow-up confirms cardOnFile=false and existing USD credit of10000 cents ($100). This identifies the outbound entitlement blocker: add a payment method through Photon Billing. No extra credit purchase is indicated. SMS allocation is a separate purchase/assignment; no number quote or order was submitted.

## Account setup history — 2026-11-07 20:37 Central

The user added a Photon payment method. Reused the reserved configuration idempotency key after reading the unchanged profile; outbound setup succeeded. Generated credentials were saved without display, and MD5/TLS endpoint plus inbound TLS/SRTP were verified by readback. No phone calls or application reservations occurred. A non-mutating billing preview quotes one US local SMS/voice number at $2/month on Messaging Free: $2 existing credit applied, $0 tax/card due now, $98 credit remaining after purchase, next renewal December7 at $2 before credits. No number purchase performed; profile lineCount remains0.
