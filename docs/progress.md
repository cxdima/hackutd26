# Progress

Real phone tests: **DEPENDENT ON SETUP**. No real phone call or live model request has been made. Current external scope is **one** outbound smoke-test attempt to local `SARAH_TEST_E164`; A calls manually. Family and handoff calling remain disabled.

| Gate | Implemented / verified locally | Outstanding live proof |
|---|---|---|
| Bootstrap | Both guides compared; contract installed; all eleven pinned skills present; local Rust and browser runtimes verified | Provider credentials/public origin not configured |
| M1 transport | Signed inbound greeting/keypad, authenticated Phone B action, durable one-attempt reservation, SDK signature vectors, wrong-leg/reorder/unknown-response/replay guards | A greeting/key, B Photon ringing/audio/key, terminal statuses |
| M2 conversation | Relay text WebSocket binding/parser, native Nemotron tool/result loop, evidence validation, six-second budget/cancellation, explicit Gather fallback | Real model access and contextual replies; signed Relay speaker-envelope capture |
| M3 private consent | Separate private Gather, atomic acceptance, B-first conference, two exact participant receipts, independent family adapter | Authorized benign bridge/privacy and independently enrolled family phone |
| M4 revocation | Speech hold, epochs, obsolete result rejection, five-second stale-key window, late status/retry/terminal bridge regressions | Physical old 1 during the still-active Gather; child completion after A hangs up |
| M5 console/community | Actual Rust-to-browser replay/SSE; expiring categorical SQLite memory; separate subscriber receipt; Maya lookup and benign new-number control | Fresh model paraphrase and two-household phone demonstration |
| M6 rehearsal | Runbook, authored development/held-out cases, non-dialing probe/evaluator, 58-second labeled replay recording and submission draft | Phone rehearsals, held-out model run, live evidence recording, portal submission |

Verified 2026-11-07 18:40 Central: Rust fmt/Clippy/build and all 91 tests passed; frontend lint/typecheck/build/format, all 10 unit tests and 4 browser tests passed. Actual Rust-to-browser replay/SSE integration passed. See [verification](verification.md) for tested boundaries and remaining limitations. The runnable replay is at `http://127.0.0.1:8080`; its video and screenshot are under ignored `.tools/artifacts/`.

No ready clone endpoint or text channel is established, so those enhancements are omitted. Native speech is a preset voice. Gather pauses A during private choice and cannot demonstrate concurrent revocation. Unknown Relay speaker-event envelopes hold permission conservatively; this can prevent handoff until live capture resolves the protocol gap.

Concrete phone checklist: A hears disclosure → A presses 1 → operator reserves the one authorized Phone B call → Photon B rings/answers → B hears fictional private test → only B's 1 acknowledges → terminal callbacks reconcile. Do not spend the attempt before exact SIP profile values/public URL are checked. No-answer or ambiguous timeout does not grant another attempt; reset preserves the ledger.

Next gate: follow [environment setup](environment-setup.md), configure `.env` and public ingress, start `VOICE_TRANSPORT=probe` in live mode, receive A's manual inbound call, then execute the already-authorized single B action. User confirmed setup is still in progress; no alternative env file was supplied. Meanwhile local verification remains inert. Preserve Sunday 10:00–12:00 Central for rehearsal/submission/recovery; initial Devpost entry is due Saturday 23:00, final Sunday noon.
