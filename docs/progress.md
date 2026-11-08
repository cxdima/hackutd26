# Progress

Real phone tests: **DEPENDENT ON SETUP**. No real phone call or live model request has been made. Current external scope is **one** outbound smoke-test attempt to local `SARAH_TEST_E164`; A calls manually. Family and handoff calling remain disabled. Continued from `e9cdfd3` without architecture/console redesign; existing replay remains a regression checkpoint.

| Gate | Implemented / verified locally | Outstanding live proof |
|---|---|---|
| Bootstrap | Both guides compared; eleven pinned skills discovered; local Rust/browser verified; `.env` created; voice-only HTTPS/WSS tunnel prepared and reachable | Twilio/Photon/NVIDIA credentials still missing |
| M1 transport | Signed inbound greeting/keypad, authenticated Phone B action, durable one-attempt reservation, SDK signature vectors, wrong-leg/reorder/unknown-response/replay guards | A greeting/key, B Photon ringing/audio/key, terminal statuses |
| M2 conversation | Assistant diagnostics no longer cancel replies; unverified optional speaker subscription disabled; partial prompt/interrupt kept; saved minimized messages; independent readiness; probe measurements | Actual model access/contextual replies; signed Relay setup/prompt/interrupt capture |
| M3 private consent | Separate private Gather, atomic acceptance, B-first conference, two exact participant receipts, independent family adapter | Authorized benign bridge/privacy and independently enrolled family phone |
| M4 revocation | Speech hold, epochs, obsolete result rejection, five-second stale-key window, late status/retry/terminal bridge regressions | Physical old 1 during the still-active Gather; child completion after A hangs up |
| M5 console/community | Actual Rust-to-browser replay/SSE; expiring categorical SQLite memory; separate subscriber receipt; Maya lookup and benign new-number control | Fresh model paraphrase and two-household phone demonstration |
| M6 rehearsal | Runbook, authored development/held-out cases, non-dialing probe/evaluator, 58-second labeled replay recording and submission draft | Phone rehearsals, held-out model run, live evidence recording, portal submission |

Verified 2026-11-07 19:24 Central: Rust fmt/Clippy/build and all **106 tests** passed; frontend lint/typecheck/build/format, **11 unit tests and 5 browser tests** passed. The existing Rust-to-browser regression integration passed against the rebuilt service. See [verification](verification.md). Existing checkpoint video/database were preserved; no new replay milestone was created.

No ready clone endpoint or text channel is established. The user deferred Brev provisioning; preparation is documented but actual GPU/price, entitlement and supported Zeroshot tag remain unverified, $0 allocated. Native speech stays the fallback. Gather pauses A during private choice and cannot demonstrate concurrent revocation. Optional Relay speaker events are disabled; diagnostics neither begin nor finish caller speech. No speech-onset coverage before a documented prompt/interrupt is claimed.

Hosted model probe was invoked and exited at missing `NVIDIA_API_KEY`, zero requests. Therefore model identity, round trips, contextual replies and real latency are unobserved; held-out evaluation remains unrun. The existing probe now retains measurements/errors for the actual run without changing timeouts.

Concrete phone checklist: A hears disclosure → A presses 1 → operator reserves the one authorized Phone B call → Photon B rings/answers → B hears fictional private test → only B's 1 acknowledges → terminal callbacks reconcile. Do not spend the attempt before exact SIP profile values/public URL are checked. No-answer or ambiguous timeout does not grant another attempt; reset preserves the ledger.

Next gate: complete the four grouped [remaining setup actions](environment-setup.md#remaining-account-actions). The running tunnel's origin is in `.env`; current Rust process remains replay. Once inbound credentials are present, restart in live/probe mode for A's manual greeting/key test; only after that spend the one existing Sarah attempt through `/api/demo/probe`. Run hosted model probe as soon as its key arrives, independently of phones, then held-out evaluation if it passes. After both transport and model gates pass, test Relay and request only the next rehearsal's missing scope: two additional Sarah attempts and one newly enrolled family attempt. Preserve Sunday 10:00–12:00 Central for rehearsal/submission/recovery; initial Devpost entry is due Saturday 23:00, final Sunday noon.
