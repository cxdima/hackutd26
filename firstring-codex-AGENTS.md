# FirstRing project contract

FirstRing is a Rust application for a live HackUTD 2026 voice demonstration. Follow the current user request and preserve existing work. A request to inspect or plan is not a request to implement; the unified starting prompt explicitly requests implementation after a brief audit.

## Specification and sources

Read `docs/FirstRing_Master_Guide.md`, or the root `FirstRing_Master_Guide.md` if the docs copy is absent. The master guide replaces the older Perplexity/Claude blueprint as the product and implementation specification. `FirstRing_Codex_Start.md` replaces the earlier initialization-only, transport-only and kickoff prompts.

Current primary provider documentation determines API syntax and account capabilities. Correct a documented mismatch narrowly and record the reason. Third-party skills are advisory. Preserve unrelated repository instructions when merging this file into an existing `AGENTS.md`.

## Product and technical boundaries

- Fictional Sarah has a disclosed assistant. Ordinary calls can reach her after a private spoken summary and keypad consent.
- The defining interaction is a pending offer revoked when the caller requests an authentication secret. Old acceptance cannot revive that offer.
- Independent help calls use designated, enrolled endpoints. The suspicious caller never joins that contact call.
- Neighborhood Firebreak shares expiring categorical tactic capsules among fictional households. Match tactics, not people; show the subscriber's actual applied version.
- Rust owns policy, state, provider adapters, authorization and events. Default to Axum/Tokio, typed errors, tracing, `reqwest` and SQLite. Preserve an existing suitable stack.
- The primary voice route is Photon SMS-number SIP → Twilio Programmable Voice/ConversationRelay → Rust. Prove transport first with Say/Gather and independent outbound DTMF. Then use Relay for the live Nemotron conversation; Gather is the fallback if Relay setup blocks.
- Use the master guide's current Nemotron model/tool contract, subject to a real access probe. The NVIDIA speech skill does not configure LLM tool calling.
- Do not build a SIP/RTP server, raw-audio pipeline, distributed workflow system, vector database or second model for the core. No post-bridge AI monitoring claim.
- Use React/TypeScript/Vite if starting a new frontend. Readable live actions and real evidence matter more than decorative animation.

## State and execution invariants

- One serialized parent actor per inbound session owns all child phone legs. It survives inbound hangup until authorized child actions and reconciliation settle.
- Offers bind session, intended leg, policy epoch and expiry. Recheck atomically before bridge commitment. Active or unassessed speech closes the gate.
- Every committed caller turn needs an evidence-backed assessment. Read-only tool results cannot clear that hold. Discard superseded model/TTS/tool results.
- Keep the revoked Sarah Gather alive for one bounded window when demonstrating physical stale-DTMF rejection, then explain and end it. Do not hang up and pretend a later keypress happened.
- Resolve contact IDs to server-enrolled numbers. Model arguments cannot contain arbitrary dial destinations, URLs or policy changes.
- Reserve side effects before provider requests. Include the reserved action ID in status callback URLs. Reconcile unknown outcomes before any retry; do not double-dial.
- Duplicate/out-of-order callbacks cannot regress terminal state or create acknowledgement. Provider callback identity is adapter-specific; do not assume a universal event ID.
- Keep private summaries on the private leg. Show connected only after the intended conference participants join.
- Interpret quotation, negation, request direction and requested object. An appointment reference or a warning about codes is not automatically an extraction attempt.
- Signature validation uses the exact configured public URL and request representation, never an arbitrary Host header. Establish Rust conformance with official test vectors or a reproducible official-validator comparison.
- Fixture/replay execution is inert. Use independent execution mode and data-origin fields: a live call may contain synthetic scenario data. Provider readiness is a separate capability state.
- Keep requested, ringing, answered and acknowledged distinct. Do not report provider acceptance as a completed intervention.
- Use only fictional scenarios and consented voice references. Keep secrets server-side and out of Git, logs and screenshots. Expire voice references as specified in the guide.

## Skill routing

Read only the relevant `.agents/skills/<name>/SKILL.md` and its necessary references. `.agents/firstring-skills.lock` records pinned provenance. Installing instructions does not install a runtime or prove their examples current.

| Task | Skills |
|---|---|
| Rust async ownership and cancellation | `rust-async-patterns` |
| TwiML and signed ingress | `twilio-voice-twiml`, `twilio-webhook-architecture` |
| Relevant ingress/outbound review | `twilio-security-hardening` |
| Behavioral tests and meaningful generated invariants | `test-driven-development`, `property-based-testing` |
| Real failures | `systematic-debugging` |
| Speech after the phone path works | `nemotron-speech` |
| UI and browser checks | `frontend-design`, `playwright-cli` |
| Completion claims | `verification-before-completion` |

Map `superpowers:NAME` references to the corresponding installed bare name. Do not install an entire orchestration framework to satisfy a cross-reference. Do not run upstream helpers, hooks, evals or benchmarks without a concrete task need and appropriate authorization.

Project-specific overrides:

- Apply test-first discipline to behavioral logic. Do not delete user work to satisfy generic TDD language. Documentation, configuration and visual exploration do not need artificial tests or extra approval merely for being TDD exceptions.
- Routine project dependencies are within an implementation request's scope; pin versions and lockfiles. Introduce `proptest` only for actual state invariants. Do not require repeated permission for ordinary dependency choices already authorized.
- Adapt Python/Node examples to Rust. Do not introduce another backend language because an upstream skill uses its SDK.
- This single-instance demo uses actor state with a durable action ledger. Generic multi-instance deployment advice is not a requirement to add Redis or rewrite the architecture.
- Use current Twilio references for Relay. Do not infer a mandatory Twilio PSTN number from a generic example when the documented Photon SIP route is being used. Native-provider limits do not mean Relay cannot play externally generated audio URLs.
- Follow the existing design brief without repeating its discovery interview. Frontend work must explain the phones' actual states.

## Autonomy and verification

Proceed through authorized implementation milestones without routine approval questions. Preserve existing authorization. Ask only for missing designated endpoints, account actions or external-test scope. Provisioning, purchases and account changes require authorization if not already given. Keep working on independent code/tests while setup is pending. Never request secret values in chat.

Do not change global model settings, install unrelated plugins, bypass sandbox restrictions, or overwrite existing skills. Treat installed skill discovery and browser availability as separate checks.

Test real behavior: signature rejection, allowed contacts, deduplication, timeout reconciliation, stale offers, cancellation, private audio, and terminal state. Use property testing only when it exercises a meaningful invariant. Run the project's relevant formatter, lint, build and tests; report failures and unrun checks honestly. Verify the rendered UI once it exists.

Maintain short bootstrap/preflight/progress documents and a live-demo runbook. A missing credential is a named live-test blocker, not a reason to fabricate success. Continue independent implementation with labeled fixtures, then return to the actual phone gates. Protect rehearsal and submission time.

## Primary references

- [HackUTD tracks](https://zeroday.hackutd.co/#tracks)
- [Photon sponsor guide](https://somethingreat.notion.site/Hackathon-Guidelines-for-Photon-2-0-3f8e443f36bb81f9893fed0d08a4c01d)
- [Photon Twilio route](https://photon.codes/docs/beta/cookbooks/voice/twilio)
- [Twilio Calls API](https://www.twilio.com/docs/voice/api/call-resource)
- [ConversationRelay TwiML](https://www.twilio.com/docs/voice/twiml/connect/conversationrelay)
- [ConversationRelay protocol](https://www.twilio.com/docs/voice/conversationrelay/websocket-messages)
- [Twilio request security](https://www.twilio.com/docs/usage/security)
- [Nemotron model API](https://docs.api.nvidia.com/nim/reference/nvidia-nemotron-3-5-lightning-30b-a3b-infer)
- [NVIDIA speech skills](https://docs.nvidia.com/nim/speech/26.10.0/resources/agent-skills.html)
