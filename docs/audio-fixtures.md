# Optional development audio fixtures

This Rust utility synthesizes caller utterances for listening and later manual transport checks. It does not replace Photon → Twilio ConversationRelay → Rust → Nemotron, invoke ElevenAgents, dial a phone, send model tool calls, change live configuration or grade the model. Twilio-managed Relay TTS does not use this utility's API key.

The reviewed batch was synthesized successfully on **Nov 7 at 21:51 Central**: **36/36 MP3 clips**, using Sarah (`EXAVITQu4vr4xnSDxMaL`), George (`JBFqnCBsd6RMkjVDRZzb`) and River (`SAz9YHcvj6GT2YYXdXww`). The root implementation owner reviewed the 12 authored labels/groupings before generation and confirmed available voices and pre-run quota (0/10,000 used, extension disabled). No quota was purchased or extended. Actual post-run billing/quota was not measured by this utility.

There were four successful metadata reads, 36 synthesis attempts, 36 success receipts, zero retries and zero errors/unknown attempts. All 36 file sizes and SHA-256 values matched their receipts; the source fixture hash matched the manifest. Total audio bytes: **3,149,602**. Per-request HTTP-through-audio-transfer times: **323–814ms, median 518ms**, 36 samples (sum 19,579ms). This timing excludes metadata reads and is not phone or first-audible latency.

The ignored local run directory is `.tools/audio-fixtures/20261108T035107Z-f7c2c209-a79e-4ae9-bc1a-e8e3b27812c4/`; its manifest/results and per-clip attempt/success receipts are preserved. Source SHA-256: `b1c106ca7acfed66f312ffffe41423055af6c4faf823d52650654567cf6ff9e3`. **Listening, transcription accuracy and phone playback remain unverified.** No phone calls or application ledger changes occurred.

The [supplemental cache index](evidence/audio-development-index-2026-11-07.json) was created **after synthesis**, verifying all 36 sizes/audio hashes and the unchanged source fixture while preserving all 110 original cache files. Its text/render hashes are newly computed; voice settings are reconstructed from the reviewed generator payload, not captured by the original receipts. Future manifests/receipts include `text_sha256` (exact UTF-8, no normalization) and `render_sha256` (compact JSON with recursively sorted keys: exact text, voice ID, model, output format and voice settings). Labels/rubrics are excluded; an unmapped dry-run voice has a null render hash. No audio was regenerated, no provider request was made, and this index establishes no ASR, listening or phone proof.

`fixtures/audio-development.json` contains 12 independently authored scenario segments rendered in three voice slots: **36 planned clips**. The two multi-segment groups cover collecting a changed appointment time and an ordinary request changing into an authentication request. Other cases cover complete reminders/deliveries, caller-supplied references, negation, quotation, ambiguity, account-access requests and payment diversion. Expected classifications, acceptable next actions and policy constraints were authored before synthesis and are never sent to TTS.

All voices and segments of a group remain in the **development** split. Do not count 36 variants as 36 independent scenarios, move an alternate voice/segment into a held-out set, or modify the frozen language v2 set. The fixture's `a_only` / `fictional_enrolled` profiles describe the rubric only; they cannot enroll or authorize real contacts. A directory lookup is an intermediate step, not a completed protective action. A private offer is pending consent, not a bridge.

```bash
# No network request, even when a key is configured:
bash scripts/cargo.sh run --bin audio_fixtures -- --dry-run

# Select three distinct existing, permitted preset/library voice IDs locally.
# This command performs billable synthesis only when explicitly invoked:
bash scripts/cargo.sh run --bin audio_fixtures -- \
  --generate --voice-ids VOICE_ID_1,VOICE_ID_2,VOICE_ID_3 --limit 36
```

Save `ELEVENLABS_API_KEY` in local `.env`, never in a command or chat. Voice IDs are not credentials. Choose three audibly distinct voices from the account's available library; the calm/brisk/measured slot names are planning labels, not measured voice characteristics. No cloning, uploaded voice reference, voice creation, account purchase or model migration is performed. `--limit 1` can bound an explicitly authorized initial synthesis check; a later invocation creates a new run and can incur new charges.

Generation is sequential, capped at 36 synthesis requests plus four metadata reads (one model and three voice checks), 20 seconds per request and 180 seconds per run. It stops on the first failure; no automatic retries. A timeout can still be billable. Each request has a local attempt receipt; complete audio files and success receipts are published atomically without overwriting existing files. A crash may leave an attempt without a success receipt: treat its remote billing outcome as unknown. Existing application databases and action ledgers are untouched.

Outputs are Git-ignored under `.tools/audio-fixtures/<unique-run>/`: `manifest.json`, per-clip MP3/attempt/success receipts, and `results.json`. The manifest records source SHA-256, grouped clip IDs, voice mapping, model/format and character estimate; successful receipts record audio SHA-256, byte count and HTTP duration. Logs show IDs/counts/status, never keys or audio/text bodies. Audio remains marked unlistened and not phone-tested. The provider's normal retention policy applies; the utility does not request enterprise-only zero retention.

The earlier Nov 7 dry run planned **36 clips / 3,549 characters**, estimated **218–327 seconds** total at 120–180 spoken words/minute. Those are text-based estimates, not measured durations or a price quote. That dry run generated no provider audio. The later reviewed synthesis results are recorded above. Verification: five audio-generator tests passed after observed failing tests for missing behavior; the full Rust workspace test run and scoped Clippy with warnings denied passed.

After generation, listen to each clip and record pronunciation, clipping, pace and whether the intended quotation/negation is audible. During an authorized A-only phone test, play only an appropriate development segment through the caller's microphone, wait for the actual response, then play the next segment in the same group. Compare the real ASR transcript, accepted assessment, proposed tool and actual outcome with the prewritten rubric. A-only runs cannot satisfy fictional handoff/family completion expectations. Synthesized caller audio, model-only tests and successful MP3 downloads do not prove carrier routing, ASR, first audible latency, private handoff or a completed callback.

The adapter uses the official [create-speech endpoint](https://elevenlabs.io/docs/api-reference/text-to-speech/convert), `xi-api-key`, `eleven_multilingual_v2` and `mp3_44100_128`. It checks [available models](https://elevenlabs.io/docs/api-reference/models/list) and each [voice's metadata](https://elevenlabs.io/docs/api-reference/voices/get) before synthesis. Voice selection can use the documented [voice list](https://elevenlabs.io/docs/api-reference/voices/search). The [stream endpoint](https://elevenlabs.io/docs/api-reference/text-to-speech/stream) is available, but complete file conversion is sufficient here; this utility does not add a streaming transport to FirstRing.

## Matched family development clips

On **Nov 7 at 23:25 Central**, the separately authored `fixtures/audio-family-development.json` produced exactly **three George clips**: a shared claimed-son/pickup opening, a benign train-time/entrance continuation, and an authentication-extraction continuation. Their labels were reviewed before synthesis. All three remain in the single `family_pickup` development group; the alternatives are not independent held-out examples, and a claimed family identity is unverified.

The [synthesis receipt](evidence/audio-family-development-2026-11-07.json) records the manifest, exact text/render/audio hashes, settings and per-clip receipts. The separate cache is `.tools/audio-fixtures/20261108T052514Z-53c19a1c-2bc3-4ce0-8aa7-d16a7615235e/`. Three requests submitted **233 text characters** (59 + 85 + 89) after two metadata reads; all three succeeded without retries or unknown outcomes. Audio totals **219,557 bytes**, with measured MP3 durations **3.669s / 4.783s / 5.062s**, mono 44.1kHz. Request-through-transfer times were 526 / 565 / 610ms; these are not phone or first-audible latency. Actual provider billing/quota was not measured.

All 110 original cache files and the six preserved source/evaluation fixture files were hash-checked unchanged. These new files have **not been listened to, transcribed or phone-tested**; synthesis performed zero telephony actions. Exact command used (rerunning incurs new synthesis):

```bash
bash scripts/cargo.sh run --bin audio_fixtures -- \
  --dataset fixtures/audio-family-development.json \
  --voice-slot caller_george --voice-ids JBFqnCBsd6RMkjVDRZzb \
  --limit 3 --generate
```

The generator also accepts repeated `--scenario ID` selections from an authored development dataset. It validates the complete source and ancestry before selecting clips, preserves the 36-request maximum, and creates a fresh cache rather than overwriting existing audio.

## Focused appointment clips

On **Nov 8 at 01:29 Central**, the existing generator synthesized exactly three additional George clips from [the authored development source](../fixtures/audio-focused-appointment-development.json): “Sarah has an appointment tomorrow at four.”, “Actually, tomorrow at five, not four.”, and “Maya has an appointment tomorrow at four.” No AM/PM, clinic or appointment type was added. The [receipt](evidence/audio-focused-appointment-2026-11-08.json) records **120 characters, 3/3 successful requests, 116,739 bytes**, verified text/render/audio hashes, and 434/390/294ms HTTP-through-transfer times. Two metadata reads preceded synthesis; no retry, error or unknown attempt occurred. Billing was not measured; no listening, ASR or phone result is claimed.

The [phone plan](../fixtures/audio-focused-phone-plan.json) allowlists `exact_appointment_george` and `maya_appointment_george` as single-clip runs. The operator must select the matching fictional active profile before starting; the caller cannot change it. Existing missing-time and family scenarios reuse their caches. The correction clip is prepared but deliberately has no startable scenario: this runner waits for the defender's response and stops when that call ends, so it cannot guarantee correction before a valid message closure. An actual pre-closure interruption cue is still required. No call allowance, reservation, model configuration or telephone state changed during synthesis.
