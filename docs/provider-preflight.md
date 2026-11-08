# Provider preflight

**Latest update,22:54Central:** the user completed the individual profile; read-only TrustHub API confirms `twilio-approved`. Automated attempt2 then connected through Photon, delivered a real caller clip to production Relay ASR, completed two native NVIDIA requests, persisted a note and ended with a captured recording. The scripted conversation failed because it ended before collecting the missing appointment time. The prior KYC block below is resolved, not a current credential/setup request. [Batch evidence](evidence/automated-caller-batch-2026-11-07.json).

**Automated originator gate, Nov 7 22:47 Central:** the first reserved fixture attempt was rejected before a CallSid: HTTP401/error20003. A deliberately unroutable empty POST established the exact cause: unapproved primary compliance profile (Trust Hub KYC). Authenticated Rust/Python GETs succeed and the running signature validator matches the local token/account. No rotation is indicated. The failed fixture reservation is retained; Sarah's probe remains unused. No generated clip reached phone ASR and no recording exists for this attempt. [Receipt](evidence/audio-caller-canary-2026-11-07.json).

**Verified live exchange, Nov 7 at 22:10 Central:** signed Relay → actual Nemotron tools/results → audible contextual replies and protective refusal → minimized incident note → complete goodbye and automatic hangup. Eight in-call responses returned `nvidia/nemotron-3.5-lightning-30b-a3b`; HTTP latency **1120–2892ms, median 1796ms**. Rust accepted four assessments, three questions and one `take_message`. The caller estimated 3–5 seconds per reply, not an instrumented audio measurement. [Live receipt](evidence/nvidia-relay-call-2026-11-07.json).

Runtime remains `live` / `relay`, caller A only; all three outbound flags are false, no Phone B is enrolled and no outbound action occurred. Full Relay rehearsal remains pending for deliberate interruption/obsolete-response rejection and reliability in the noisy hackathon room. The successful baseline was useful, not a quiet-environment requirement. Earlier 64105 startup, expired-tunnel 11200/502 and orphan-partial failures remain in the [attempt history](evidence/relay-attempts-2026-11-07.json).

Caller-only model development passed 3/3; frozen held-out behavior passed 7/12, a failed gate. Thirty-six existing synthesized clips remain unlistened/untested on the phone; none were regenerated. Six inert actor voice-kit tests pass, while continued interactive model evaluation remains pending implementation/results. See [evaluation](evaluation.md), [audio fixtures](audio-fixtures.md) and [kit reconciliation](voice-test-kit.md).

## Selected route and gates

Photon **Beta / 2.0** voice-eligible SMS number → authenticated TLS SIP → Twilio SIP Domain → signed Rust webhook. Say/Gather proves transport first; ConversationRelay is the interactive route, with explicit Gather fallback after a 30–45 minute onboarding timebox. No raw-audio replacement.

| Capability | Current gate | Evidence required to pass |
|---|---|---|
| Public HTTPS/WSS | Replacement HTTP/2 ingress carried the verified 22:10 exchange after the old tunnel expired | Continued reachability; console SSE stays localhost |
| Photon inbound | **Passed Nov 7, 20:42 Central:** A heard disclosed greeting, pressed 1, completed inbound confirmed | Initial voice rated poor; `Polly.Joanna-Neural` / `en-US` selected for Say/Gather, listening comparison pending |
| Photon outbound | MD5/TLS credentials and caller line configured; `SARAH_TEST_E164` missing; no reservations/submissions | Designated B rings, hears private test, presses 1, and reaches terminal status |
| Relay | 22:10 native model/tool exchange, heard replies/refusal, saved note and heard goodbye/automatic hangup | Deliberate interruption, obsolete response rejection and noisy-room reliability; optional speaker subscription remains disabled |
| Nemotron | Exact model returned in eight live-call requests; A-only development 3/3; frozen v2 FAIL at 7/12 | Redundant-question and premature code-classification repair; remaining held-out failures; no general accuracy claim |
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

`RELAY_SPEECH_MODEL` now selects `nova-3-general` (default) or `flux`; invalid values fail startup. The selection stays fixed until process restart. TwiML, API speech diagnostics and each call's requested speech telemetry use that same setting. `speechTimeout="1000"`, partial prompts, interruption controls, and closing Gather remain unchanged. Flux's documented default `eotThreshold=0.8` is retained by omitting the attribute. This configuration capability is not a completed Flux phone comparison.

For a controlled comparison, replay the same cached appointment clip with Flux and record caller-audio end → final prompt → model completion → first response audio. Nova uses the timeout as a final-prompt delay; Flux uses it as a maximum silence duration and may end a turn earlier based on confidence. Check complete fact transcription, orphan partials and interruptions as well as latency; revert on regression. No speculative action runs on partial prompts. [Current Twilio attributes](https://www.twilio.com/docs/voice/twiml/connect/conversationrelay#attributes). The `speaker-events`/`tokens-played` subscription names do not establish a documented completion-acknowledgment envelope in the [message reference](https://www.twilio.com/docs/voice/conversationrelay/websocket-messages); they remain disabled.

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

The current configured-origin signature check and setup binding have now accepted actual Twilio handshakes. Twilio accepted the requested Deepgram/nova-3-general, en-US, 1000ms speech timeout and ElevenLabs Rachel-v4 configuration for the 22:10 exchange; it did not report an internal ASR version. The 4s inactivity / 8s repeated-partial watchdog requests repetition while leaving uncertain speech uncommitted and consent held. That successful call had zero interrupt/recovery events, so those live paths remain pending.

### Noisy-room options — researched, not applied

The current-room call is the acceptance test; no quiet-location requirement applies. `interruptSensitivity="medium"` is a small candidate if false TTS cutoffs dominate. It changes TTS interruption sensitivity, not all partial-prompt delivery while the model is thinking. `ignoreBackchannel` affects brief feedback during agent speech; `echoPromptFiltering` targets the agent's own speech leaking into the microphone, not other people. Keep input reporting and consent holds intact. [Attribute contract](https://www.twilio.com/docs/voice/twiml/connect/conversationrelay#attributes), [echo filtering](https://www.twilio.com/docs/voice/conversationrelay/best-practices#preventing-false-interruptions).

If turn finalization remains the main failure, `speechModel="flux"` is a documented Relay alternative combining transcription and turn detection. Its `speechTimeout` becomes a maximum silence duration; `eotThreshold` is 0.5–0.9, default 0.8. A separate noisy-call comparison is required before choosing it. Twilio's small vacuum/drill experiment is not proof of foreground-speaker separation in hackathon chatter. No documented noise-isolation toggle guarantees that unrelated speech will be excluded. [Twilio noise tutorial](https://www.twilio.com/en-us/blog/developers/tutorials/product/handle-background-noise-when-using-conversation-relay). No noise-control change has been deployed from this research.

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

The adapter and console distinguish model HTTP observations, native proposals, accepted/rejected tool results and actual phone outcomes. Request/connect/turn limits remain 3500/1500/6000ms. `NEMOTRON_PROBE_TIMEOUT_MS` changes only the explicit protocol diagnostic (maximum 15 seconds/request), not runtime or evaluation. Traces and `audio.requested` alone do not prove playback; the 22:10 caller confirmation establishes heard replies and the full goodbye for that call. Closing uses signed Connect action → Joanna `Say` → `Hangup`. Relay Rachel-v4 and Joanna are presets, not clones.

`/health/ready` reports configuration separately from recorded evidence; HTTP 200 describes selected-transport configuration, not public availability. The live exchange receipt establishes a specific conversation/closing pass; full Relay rehearsal and outbound remain pending. Credential presence never upgrades a proof badge. Outbound still requires enrolled destinations, enabled flags, authorization and the durable attempt budget.

No architectural change to the master guide is necessary. Clarifications above cover SIP caller ID, Relay's string `handoffData`, and unresolved speaker/WSS capture gates. Skills' generic SDK/Host-header examples do not override the project contract.

The [official event schedule](https://zeroday.hackutd.co/#schedule) currently confirms **Nov 7, 23:00 CT initial Devpost; Nov 8, 12:00 CT final submission; 13:00–16:00 CT judging**. Protect 10:00–12:00 Sunday for rehearsal/submission/recovery. Portal-specific requirements and organizer changes still need checking.

Current runtime is `VOICE_TRANSPORT=relay`, `DEMO_EXECUTION_MODE=live`, with `TEST_CALLS_AUTHORIZED=false`, `HANDOFF_CALLS_ENABLED=false`, and `FAMILY_CALLS_ENABLED=false`. Keep all three false for caller-only testing. Phone B is unavailable; its single historical smoke-attempt budget is unspent. The console must leave that action unavailable. No family endpoint or later outbound scope is established.

Next live gate: the requested deliberate interruption in the current noisy room, with actual interrupt/cancellation, current-generation model result, audible response and clean ending correlated. Preserve public health and signature checks. B is not a prerequisite. Full two-phone M1, private consent and independent contact remain unverified; Brev Nemotron deployment is authorized; the current account and deployment gates are tracked separately. Keep credentials and ledgers intact. See the [caller-only runbook](demo-runbook.md).

## Account setup history — 2026-11-07 20:28 Central

Twilio now reports Full/active. Created `hackutd26-firstring-d232a271.sip.twilio.com`, Secure=true, SIP registration/emergency calling disabled, Voice POST to `/voice/incoming`, status POST to `/voice/status`, one generated inbound credential and its call-auth mapping. Photon default profile inbound now matches that SIP destination/login, and mediaEncryption=srtp. All values were read back without exposing credentials. No phone number was bought. Photon outbound PUT with MD5 returned 402 ENTITLEMENT_REQUIRED; its generated outbound credential/assigned endpoint remain unavailable. The profile has zero lines and the app action ledger remains empty.

Read-only Photon billing follow-up confirms cardOnFile=false and existing USD credit of10000 cents ($100). This identifies the outbound entitlement blocker: add a payment method through Photon Billing. No extra credit purchase is indicated. SMS allocation is a separate purchase/assignment; no number quote or order was submitted.

## Account setup history — 2026-11-07 20:37 Central

The user added a Photon payment method. Reused the reserved configuration idempotency key after reading the unchanged profile; outbound setup succeeded. Generated credentials were saved without display, and MD5/TLS endpoint plus inbound TLS/SRTP were verified by readback. No phone calls or application reservations occurred. A non-mutating billing preview quotes one US local SMS/voice number at $2/month on Messaging Free: $2 existing credit applied, $0 tax/card due now, $98 credit remaining after purchase, next renewal December7 at $2 before credits. No number purchase performed; profile lineCount remains0.
