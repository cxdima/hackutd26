# Console backend integration

Backend-only additions; no frontend source, assets or other-worktree edits. Initial handoff read at `frontend/angular-console:e8ec101`; a later read found `dbd6366` plus Claude's uncommitted work, all preserved. The current schema is `frontend/src/app/core/schemas.ts` (there is no `core/schemas/live.ts`).

## Static routes and receipts

The Rust fallback serves `frontend/dist/firstring-web/browser` when its index exists. `/` and `/console/**` return that SPA entry. `/legacy-console/` retains the React console and rebases its asset references; absent Angular falls back to `web/dist`. Missing `/api`, `/voice`, `/health`, media and asset paths do not receive HTML. Existing signed voice/WebSocket/media routes take precedence.

Authenticated `GET /api/session/{id}/events?after=0&limit=100` returns:

```json
{"session_id":"...","events":[],"next_after":0,"has_more":false}
```

Events keep the existing envelope and sort by global numeric `sequence`, not UUID `id`. `after` is an exclusive sequence cursor. Limits are 1–500, default100; invalid queries are400, unknown sessions404. Continue using `next_after`; an empty page preserves the supplied cursor. New receipts can subsequently be fetched with the same cursor. Responses are private/no-store. The existing SSE endpoint is unchanged.

## One private-handoff arm

These operator-authenticated controls do not place a call themselves:

- `GET /api/demo/phone-demo`: status below; also available as `/api/state.phone_demo`.
- `POST /api/demo/phone-demo/arm` with `{"contact_id":"sarah","session_id":"optional active session","ttl_seconds":300}`. Omit `session_id` to bind the next signed live inbound for the selected household. TTL is60–600seconds; default300.
- `POST /api/demo/phone-demo/disarm` with `{"arm_id":"current arm UUID"}`. A stale arm ID is rejected.

Status fields: `state` (`unarmed|armed|expired|disarmed|reserved`), nullable `arm_id`, `session_id`, `household_id`, `expires_at`, `action_id`; fixed `contact_id:"sarah"`, `scope:"one_private_handoff"`; `remaining_attempts` (0 or1), `recipient_enrolled`, `outbound_configured`, `can_arm`, `can_disarm`, `probe_budget_unchanged:true`.

Arming requires a live configuration, outbound setup and the enrolled Sarah recipient. Arbitrary numbers/URLs and unknown body fields are rejected. The server binds the exact configured recipient using a private fingerprint. A next-call arm binds only once after signed ingress; it is never exposed to an unrelated session. Unarmed live model contexts cannot offer handoff even if the legacy global flag is set.

The fixed durable `phone_demo.v1` grant is consumed atomically with the handoff action reservation, before the provider request. One reserved attempt cannot be rearmed or refunded after timeout, failure, disarm or process restart. An unused expired/disarmed arm may be replaced. Existing consent/version/revocation and conference-participant checks still apply. `reserved` means an attempt is reserved, not ringing, answered or connected; use actual action/offer/bridge receipts for progression. This allowance is separate from the original one-attempt Phone B transport probe and from automated caller allowances. No live call is implied by these local tests.

## Armed automated caller

`armed_handoff_e2e_george` is an additive existing-runner scenario. Starting it requires a valid arm awaiting the next signed inbound; it consumes the unchanged automated-caller allowance only when explicitly started by the operator. Cached George clip7 supplies the appointment/transportation request. After the assistant's reply, bounded silent Gathers keep caller A connected while B is pending. Only the actual session `connected` state (both intended conference participants) releases cached clip12 as an explicitly scripted bridge-test sentence. The next remote Gather ASR is retained as unattributed audio evidence, then A ends. The existing90-second provider cap remains; the waiting loop also enforces90seconds locally. No new audio is generated.

`handoff_probe` exposes `bridge_confirmed`, `test_sentence_requested`, `remote_audio_asr_captured`, `human_b_listening_confirmed:false`, and `bidirectional_audio_verified:false`. A requested Play is not heard-audio proof. Keep B's listener confirmation and the actual caller recording separate; do not label a bridge or ASR result as verified bidirectional audio.

The private summary still uses **Amazon Polly / Joanna-Neural**, separately from the configured Relay voice. `phone_demo.private_summary_provider`, `private_summary_voice`, and `private_summary_source:"configured"` make that distinction explicit. This is configuration, not an audition result.

## Current model availability

`model_backend.protocol_verified` remains historical. Use optional `model_backend.current.available` for current availability, with `protocol_current`, `endpoint_reachable`, `checked_at`, `available_until`, `deployment_id`, `reason`, `native_cycle_recorded_at`, `native_cycle_receipt`, `freshness_seconds` and `meaning` as supporting evidence. An expired deployment is unavailable even with historical success. Current availability is neither a latency nor a conversational-quality guarantee.

Claude's current schema already accepts the core optional `current` booleans/timestamps/reason; add the deployment/receipt fields to retain them if displayed. Add `phone_demo` and the receipt-page loader separately. Capability `private_handoff_demo` describes the bounded arm, not a general permission or a completed call.

## Presented-number reputation

This implementation is Rust/SQLite; it does not use AWS. Live configuration requires a persistent `REPUTATION_HMAC_KEY` (at least 32 bytes) and explicit `REPUTATION_SCOPE=demo|production`. Replay ignores environment activation; inert tests inject their own service. Changing the key requires an explicit migration. `/api/state` advertises capability `reputation` as configured or disabled; configuration alone is not live interaction proof.

Signed ingress captures an exact presented E.164 number, including the user portion of a valid `sip:`/`sips:` URI. Storage retains a domain-separated, scope-keyed HMAC and last two digits, not the raw number. Missing, anonymous or malformed identifiers have no shared number key. Replay and live records are isolated. Presented caller ID remains unverified and may be spoofed.

All routes below use the existing operator bearer authentication and no-store responses:

| Method and path | Payload/result |
|---|---|
| `GET /api/reputation?limit=50` | `{scope, execution_mode, items}`; limit 1–100. |
| `GET /api/reputation/{number_key}` | Raw snapshot plus versioned `audit`; unknown key returns 404. |
| `GET /api/session/{id}/reputation` | Current Angular-compatible projection described below; no model-use receipt is created by this console read. |
| `POST /api/reputation/reports` | `{session_id}` only. Derives identity and durable protective source server-side; 202 for new, 409 for duplicate/ineligible. Ordinary operation reports automatically. |
| `POST /api/reputation/{key}/corrections` | `{reason, expected_version}`; reason is `mistaken`, `spoofed` or `legitimate_business`; returns 202 `{correction, version, level}`. |
| `POST /api/reputation/{key}/corrections/resolve` | `{outcome, expected_version}`; outcome `upheld` keeps reports, `removed` retracts existing reports. |

Unknown body fields, including arbitrary numbers and freeform correction notes, are rejected. Stale correction versions return 409. A requested correction prevents escalation while allowing expiry to lower the level. Removal preserves the audit; a later genuinely eligible session may begin a new report cycle. Versions increase across reports, correction changes and expiry refreshes.

A report requires the current accepted risky assessment with valid caller evidence, a saved protective message and its exact durable `message.recorded` event. A proposal, timeout, benign correction or historical sticky block alone cannot report. Session deduplication and report/event/audit writes are transactional. Repeated sessions from one household renew its expiring contribution without multiplying that household's weight. Corroboration uses seven days; report expiry is fourteen days, refreshed on access. Tactic capsules remain separate; number reputation neither invents tactic matches nor authorizes blocking, bridging or dialing.

Demo Sarah/Maya profiles are **simulated independence**: `demo_household_count` can increase, while `verified_independent_households` stays zero. Production currently counts one local participation group, not multiple independently verified people. The raw snapshot exposes these counts, `independence`, `corroborating_household_count`, `level`, expiry and correction. A demo label must not be presented as verified community corroboration.

The receipt chain is `reputation.identity.captured` → accepted `message.recorded` → `reputation.reported` (source event ID/sequence and turn/epoch) → a later household's `reputation.lookup` → `reputation.applied`. The final event links `lookup_receipt_id` and `read_version` to the originating generation/turn after a native provider response is observed. It proves the minimized context was supplied, not that a decision was accepted or that the model correctly used it. Console reads do not manufacture this chain. For example, illustrative payload fragments are `{"household_id":"maya","read_version":1}` and `{"lookup_receipt_id":"<lookup UUID>","read_version":1,"decision_accepted":false}`.

Compatibility was checked against Claude's **current uncommitted** `../hackutd26-frontend/frontend/src/app/core/reputation.ts` and `console-store.ts`, not solely the older branch proposal. The session endpoint returns `number_label`, `band`, explicit `band_label`, `reports`, `independent_households:0`, timestamps, `correction:{state}` and `related_tactic_ids:[]`; its label/explanation explicitly disclose simulated demo profiles. The separate worktree and all frontend source/assets remain preserved.

Local verification: 12 reputation persistence/privacy tests and three signed-ingress/local-native-provider/API integration tests passed. They cover rollback, concurrency, restart, expiry, corrections, SIP/anonymous parsing, authorization, actual second-household request context and benign/failure controls. These results do not constitute a new phone test.
