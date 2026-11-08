# Recorded contract corrections

## Payment meaning (2026-11-07)

The master guide's single `requested_object=payment` value cannot distinguish “Please bring your copay to the clinic” from “Buy a gift card and read its number” while preserving their actual request speech act and direction. A regression test demonstrated that treating every directed payment request as diversion blocks the benign copay case.

The shared `Observation` enum therefore adds **`payment_diversion`**. `payment` retains ordinary on-site payment/reminder meaning. Rust protects a directed request for `payment_diversion`, `authentication_secret`, or `remote_access`; it does not flag ordinary payment merely because it is a request. Both assessment and handoff tools use the same schema, and the model prompt/evaluator use the same distinction.

This is a categorical contract repair, not proof of Nemotron's extraction accuracy. The frozen ordinary-payment and diversion cases still require real model evaluation. Other observation fields and tool arguments are unchanged; original guide copies are preserved.

## Gather retry identity

The fallback adds a server-owned `turn_id` nonce to `/voice/turn` action URLs. Signed duplicate callbacks claim work once and receive the cached TwiML response. This avoids treating a webhook retry as a new caller turn without assuming a universal Twilio event ID. The route and product behavior remain the guide's Gather fallback.

## Hosted native tool request mode (2026-11-07)

Real NVIDIA calls using `tool_choice:auto` repeatedly returned `record_assessment.observation` as a malformed string (including an extra closing bracket) instead of the required object. The adapter now sends the unchanged schema subset for a Rust-selected phase: named `record_assessment` first, `required` over the six next-step tools after Rust accepts that assessment, and named `publish_tactic` only in the existing post-call job. One-tool validation, native assistant/result IDs, evidence checks and Rust authorization remain mandatory. A `required` request over all eight tools had repeated the assessment instead of advancing; phase selection removes that invalid workflow choice without choosing the model's assessment or next action. The hosted endpoint accepted `required` and returned valid native tools; the final phase-specific two-context gate and timing results are recorded in [verification](verification.md). This is a request-format correction, not permission to repair malformed JSON or coerce tool fields.

[NVIDIA's tool-calling guide](https://docs.nvidia.com/nim/large-language-models/2.0.13/advanced-use-cases/tool-calling-and-mcp.html) refers to the [vLLM native tool modes](https://docs.vllm.ai/en/v0.28.0/features/tool_calling/), where `required` uses schema-constrained generation and `auto` extracts model output. Self-hosted documentation alone does not establish hosted behavior; the actual authenticated responses supply that evidence. Provider-returned arguments remain strictly validated, and ordinary assistant content is never spoken. No model, endpoint, prompt policy or frozen evaluation answers were changed.

## Open-ended understanding and message completion (2026-11-08)

Two actual automated calls received an incomplete opening intact, then finalized it with `take_message`. The native assessment contract now requires a bounded `understanding` alongside the unchanged risk observation and current-turn evidence. Purpose/request stay open text; claimed identity is unverified, facts must quote actual caller words, and readiness distinguishes missing information, a complete message, caller-declined detail, and no message wanted. The accepted value binds generation, turn and policy epoch. Rust exposes clarification-only or ordinary-end tools where appropriate and rejects message/offer completion from a current incomplete understanding. Accepted risky policy takes precedence, so the model cannot ask for missing credentials instead of protecting the call.

A new `end_call` tool handles an explicit wrong number/no-message case without inventing a persisted note. It cannot bypass blocked policy. Readiness remains the model's judgment, not a proof that the message is semantically complete; development scoring and actual phone evidence remain necessary. Legacy snapshots deserialize; authored replay tools now explicitly carry their fixture understanding rather than bypassing native validation.

Closing uses a signed, bounded speech Gather around the acknowledgement, allowing a correction to resume the existing actor. Relay corrections rotate the connection token and reject stale input/disconnects; Gather corrections stay on the existing Gather turn path. Two seconds of silence after the closing prompt ends the call. This is locally tested, not yet a new live interruption claim.

## Backend identity and budget (2026-11-08)

`NEMOTRON_BACKEND=nvidia_hosted|brev_hosted` selects separate credentials/endpoint/model settings. Brev uses a dedicated inference key over a loopback SSH forward or HTTPS; the hosted configuration is retained. Backend labels describe selected configuration until native-cycle evidence exists. Historical hosted protocol receipts cannot verify Brev; its receipt additionally binds the endpoint hash, loaded model and GPU/native-cycle checks. The Brev experiment ceiling is $10 with at least $50 reserved for presentation, under the user's larger total existing-credit authorization. No standalone NeMo Framework experiment was implemented or retained.
