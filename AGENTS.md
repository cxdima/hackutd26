# FirstRing project contract

FirstRing is a Rust application for a live HackUTD 2026 voice demonstration. Follow the current user request and preserve existing work. A request to inspect or plan is not a request to implement; the unified starting prompt explicitly requests implementation after a brief audit.

## Current post-campaign implementation pass (Nov 8)

The campaign at `44ecc40` is complete. The current user request supersedes its expired timer and A-only stopping point: restore a new bounded Nemotron deployment, reproduce attempt 19 through the actual runtime contract, enable an explicitly armed physical-phone handoff when the recipient is enrolled and ready, and implement durable scoped reputation with real lookup receipts. Preserve the existing 26-call total ledger and unused Sarah probe. Family calling remains disabled. No new training/NeMo/LoRA project in this pass.

Claude owns `frontend/`, assets and visual design in the separate `frontend/angular-console` worktree; backend agents own Rust, additive contracts, SQLite and deployment. Do not overwrite Claude's uncommitted work. Angular production artifacts belong at `frontend/dist/firstring-web/browser/`; retain the prior console until integration is verified.

Use a new Brev resource identity and versioned budget policy within the original $60 lifetime authorization. Never repurpose a deleted instance pointer or expired campaign identity. Historical model receipts must not make an offline backend appear available. A private handoff requires explicit short-lived operator arming; page loads, refreshes and reconnects never initiate calls.

## Specification and sources

Read `docs/FirstRing_Master_Guide.md`, or the root `FirstRing_Master_Guide.md` if the docs copy is absent. The master guide replaces the older Perplexity/Claude blueprint as the product and implementation specification. `FirstRing_Codex_Start.md` replaces the earlier initialization-only, transport-only and kickoff prompts.

Current primary provider documentation determines API syntax and account capabilities. Correct a documented mismatch narrowly and record the reason. Third-party skills are advisory. Preserve unrelated repository instructions when merging this file into an existing `AGENTS.md`.

## Product and technical boundaries

- The fictional household profile is configurable. Per the Nov8 caller-experience correction, use its neutral greeting without an automatic assistant-disclosure preamble; if asked, accurately identify the automated assistant. Never impersonate the person. Ordinary calls can reach the enrolled person after a private summary and keypad consent when that path is authorized.
- The defining interaction is a pending offer revoked when the caller requests an authentication secret. Old acceptance cannot revive that offer.
- Independent help calls use designated, enrolled endpoints. The suspicious caller never joins that contact call.
- Neighborhood Firebreak shares expiring categorical tactic capsules among fictional households. Match tactics, not people; show the subscriber's actual applied version.
- Rust owns policy, state, provider adapters, authorization and events. Default to Axum/Tokio, typed errors, tracing, `reqwest` and SQLite. Preserve an existing suitable stack.
- The primary voice route is Photon SMS-number SIP → Twilio Programmable Voice/ConversationRelay → Rust. Prove transport first with Say/Gather and independent outbound DTMF. Then use Relay for the live Nemotron conversation; Gather is the fallback if Relay setup blocks.
- Current authorized continuation: inbound Say/Gather transport is proven and Phone B is unavailable. Proceed with A-only Relay/Nemotron testing while all outbound flags remain false. Do not wait for a second handset, spend the unused Sarah attempt, or claim the private handoff/family phone gates passed.
- Automated caller exception: the original six-attempt allowance is preserved. On Nov 8 the user authorized **20 additional** synthetic calls, recorded as a separate durable grant (26 total including all historical failures/reservations). Target only FirstRing's verified own Photon number, one at a time, maximum90seconds each; preserve the Sarah attempt and disabled handoff/family flags. Reconcile unknown outcomes, stop on routing loops/repeated infrastructure failures, and clean up both legs. After three identical call failures, fix and record the underlying defect before another attempt; never clear history to resume. Cached caller clips must travel through real telephony/ASR; scripted scheduling is not evidence that a caller agent heard the response.
- Current Brev authorization: actual GPU-hosted Nemotron inference is authorized up to the lower of $60 total or remaining usable existing credits, including compute/storage/other charges. Verify account balance and actual quote, reserve rehearsal resources, set a shutdown deadline and cleanup, and never top up or enable auto-recharge. Preserve hosted inference as fallback; a forwarding proxy is not GPU proof. This supersedes earlier Brev deferral. No standalone NeMo Framework transcription/training project is requested; preserve legitimate NVIDIA-NeMo model resources. This does not expand phone authorization.
- Current campaign priority: pause optional dataset expansion and fine-tuning until audible conversation/recovery/profile failures are addressed. Focused synthetic audio needed to reproduce those failures remains authorized.
- A feasible official Nemotron LoRA pilot is now authorized within that same total budget, after a valid native-tool baseline. Verify exact-model training memory/precision, adapter export and serving compatibility first; inference fit alone is insufficient. Use reviewed synthetic trajectories with scenario-family splits and untouched frozen held-out data. Record actual optimizer steps and artifacts; prompt changes/inference are not weight training. Reuse useful allocated compute, export results, and stop/delete by the budget deadline.
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

Commit and push verified runnable checkpoints to the existing GitHub repository so the user can request independent AI code reviews. Include current verification evidence and known limitations. Keep credentials, local databases, generated runtime artifacts and unrelated IDE files out of commits. Report the pushed commit and review link; do not force-push over other contributors' work.

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

## Recorded implementation corrections

Read `docs/contract-corrections.md` alongside the preserved master guide. It records narrow, tested contract fixes; it does not replace the product specification.
