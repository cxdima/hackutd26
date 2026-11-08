# Voice test kit reconciliation

This is **synthetic development evaluation**, not training or fine-tuning. The supplied `FirstRing_Voice_Test_Kit.zip` README, prompt and JSON files were read without extracting executable content. Its baseline is `4e9f61d`; current local code and authorization take precedence. No phone, TTS or model requests were made by this integration.

The kit's 24 `development-source-snapshot.json` records exactly match all 24 current `language-development.json` records, including expectations. Its **72 planned clips = 24 cases × three voices**. Those source texts have zero exact overlap with the newer local audio dataset's **12 segments / ten groups × three voices = 36 generated clips**. All 36 existing audio byte counts and SHA-256 receipts were rechecked successfully. Preserve those files; they are not kit renders. Listening and standalone audio→ASR evaluation remain **NOT_RUN**. The frozen held-out sets were neither read nor changed for this job.

## Interactive adapter

[`fixtures/voice-interactive-development.json`](../fixtures/voice-interactive-development.json) preserves the five supplied scenarios and ten caller prefixes, with source-file hash and an explicit mapping to current `Observation` fields. Expected labels remain separate from caller text. [`server/tests/voice_scenarios.rs`](../server/tests/voice_scenarios.rs) sends authored native tool proposals through isolated production actors using in-memory SQLite and `execution_mode=replay`; it never calls Nemotron or telephony. These are policy regressions, not measured model classification or clarification accuracy.

| Supplied scenario | Actual tool/state check |
|---|---|
| Ambiguous code → appointment reference | `record_assessment` marks unknown object; `offer_handoff` is rejected; `ask_caller(clarify_code_type)` asks for the type without soliciting its value; current reference assessment permits only a pending offer. |
| Ambiguous code → login secret | Same safe clarification, then accepted directed secret-request assessment blocks handoff. |
| Clinic → login while offer pending | A real local offer and simulated Sarah leg exist; new speech closes acceptance; the changed turn revokes the offer; actual `Choice` input with old `1` is rejected and no conference result appears. |
| Withdraw earlier secret request | Old-turn-only evidence cannot assess the withdrawal. A fresh routine-request assessment does not erase earlier blocking evidence; new handoff fails; a minimized incident note can close the call. |
| Reported speech → active request | A quotation assessment does not block; the subsequent directed request does. |

Two additional local controls cover a negated code warning and a caller supplying an appointment reference. **Six tests passed**, exercising the five kit scenarios plus those controls. They verify accepted state transitions, evidence freshness and inert execution; they do not grade the model's ability to produce the authored observations. False alarms, missed risks, model clarification/next-tool accuracy, audio phrase preservation and completed external protective actions remain **NOT_RUN** for this kit.

The withdrawal mapping deliberately distinguishes the current utterance from whole-call policy. The kit's final checkpoint cites the earlier request as historical risk; production still requires evidence from each new caller turn. Rust retains the earlier block independently. There is no `revoke` model tool: committing new evidence and accepting the risky assessment invalidate the offer. Replay leg events remain simulated; no Phone B capability or authorization is implied.

## Reuse and remaining audio work

Keep the existing [Rust audio-fixture generator](audio-fixtures.md). It currently accepts its fixed 12×3 dataset, uses `eleven_multilingual_v2`, and creates a new run for every invocation. The kit proposes `eleven_flash_v2_5`, 24×3 and staged four-canary/one-voice expansion; those are **pending adaptations**, not executed changes. Do not rerun the existing generator expecting a resumed cache.

Before any kit expansion, extend that generator rather than adding another: accept a bounded development input, implement content-addressed reuse keyed by exact text+voice+model+settings, retain provider request IDs when present, and keep unknown billable outcomes visible without retrying. Current receipts retain attempt timestamps, audio/file hashes, model/format and voice mapping. A later supplemental index now verifies all 36 cached recordings and reconstructs exact-text/render hashes; future generator output captures those hashes directly. That provenance index is not a resumable content cache. See [audio provenance](audio-fixtures.md). Existing audio must be listened to first; no additional synthesis budget was spent here. Primary API: [ElevenLabs text-to-speech](https://elevenlabs.io/docs/api-reference/text-to-speech/convert).

For a future clean-text model run, give Nemotron only each available caller prefix and production capability context. Never send gold labels, scenario IDs, filenames, voice identity, future turns or endpoint labels. Preserve actual request/result receipts and count five interactive scenarios separately from ten prefixes. For audio evaluation, use the observed ASR transcript; playing a clip while supplying its known script remains a text test. No standalone audio→ASR adapter is implemented by this kit.

## Relay transcription configuration

Twilio currently documents `Deepgram` as the default transcription provider, except accounts using Relay before September 12, 2025, and `nova-3-general` as the en-US default model. The TwiML now explicitly pins `transcriptionProvider="Deepgram" speechModel="nova-3-general"` with existing `language="en-US"`; this documents the requested configuration, not an observed provider model version. The boundary assertion failed before the pin and all nine Relay protocol tests passed afterward. Actual provider acceptance and turn completion still need a correlated phone receipt. Sources: [Relay attributes](https://www.twilio.com/docs/voice/twiml/connect/conversationrelay#attributes), [language defaults](https://www.twilio.com/docs/voice/conversationrelay/voice-configuration#default-voice-settings).

Next live gate remains one authorized A-only call: caller speech → committed transcript → Nemotron native tool → Rust result → audible contextual response, followed by a changed request and clean message ending. Outbound and family flags remain disabled; the unused Sarah smoke attempt remains untouched.
