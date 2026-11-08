# FirstRing: unified Codex starting prompt

Paste the text below into Codex with `FirstRing_Master_Guide.md` available in the repository. Use the accompanying setup package if the project skills are not installed yet. This prompt replaces both the old initialization-only prompt and the old transport-only development prompt; it also replaces `FirstRing_Codex_Kickoff.md` as the starting message. Keep the master guide as the product specification.

---

You are my senior Rust and voice-systems engineer for FirstRing at HackUTD 2026. Build the working product in this repository. Start with a short setup audit, then implement immediately in small, verified vertical slices. Do not stop after initialization or merely produce a plan. Preserve existing work and keep a runnable checkpoint after each milestone.

I am a solo developer. The original plan budgeted fourteen hours; use the actual remaining time and submission deadlines rather than restarting that clock. Preserve time for real phone testing, rehearsal and submission. Prioritize the quality of the live interaction over feature count.

## 1. Read and reconcile the project

Inspect the Git working tree, existing code, root and applicable nested `AGENTS.md` files, and available tools. Read `firstring-codex-AGENTS.md` and `docs/FirstRing_Master_Guide.md`, falling back to the root `FirstRing_Master_Guide.md`. Read existing bootstrap/preflight documents if present; do not pretend missing files were read.

If both root and docs copies of the master guide exist, compare them before selecting the specification. Preserve local edits and reconcile differences; do not silently use a stale docs copy merely because it exists.

This request supersedes the earlier instruction to stop after initialization, the transport-only stopping point, and the older recommendation to make Gather the primary conversational path. Merge the supplied FirstRing project contract into an existing `AGENTS.md` while preserving unrelated repository instructions and user changes. Do not overwrite a whole existing file. Resolve routine choices yourself; flag only genuine unresolved conflicts.

Use the master guide for product, routes, events, state and tool contracts. Current primary provider documentation determines actual API syntax and availability. If evidence contradicts the guide, make the smallest necessary correction and record the reason. Third-party skills provide guidance; they do not override my request or the project contract.

## 2. Bootstrap the skills once

Inspect `.agents/firstring-skills.lock` and local skill directories. If the supplied installer has not run, review `firstring-codex-setup.sh`, run its `--dry-run`, and install into this repository. This authorizes the supplied project-local skill installation; do not change global Codex settings or install unrelated hooks, MCP servers or plugins. Preserve different existing skills and resolve conflicts without deletion. If upstream access is blocked, record the missing skills and continue using the available references; skill installation is not a reason to abandon implementation.

Confirm the eleven expected skill names and the presence of their local supporting files. Distinguish file presence from actual Codex discovery and from executable availability. Do not claim the browser runtime is installed because its skill exists. Do not load every skill into context at once or run bundled examples/evaluation harnesses merely because they are present.

Use this routing:

| Work | Relevant skills |
|---|---|
| Rust ownership, tasks, cancellation and bounded queues | `rust-async-patterns` |
| TwiML, inbound callbacks, outbound legs and private Gather | `twilio-voice-twiml`, `twilio-webhook-architecture` |
| Public ingress and outbound controls | Relevant parts of `twilio-security-hardening` |
| Meaningful behavioral and call-control tests | `test-driven-development`; add `property-based-testing` for generated state sequences |
| Actual integration failure | `systematic-debugging` plus the relevant provider reference |
| Optional NVIDIA speech integration | `nemotron-speech` |
| Live console design and browser verification | `frontend-design`, then `playwright-cli` |
| Any completion claim | `verification-before-completion` |

Map references such as `superpowers:test-driven-development` to the installed bare skill name. Do not install the full Superpowers workflow to satisfy a cross-reference. Apply test-first discipline to new behavioral logic; configuration, documentation and visual exploration do not need contrived tests. Never delete existing work because a generic skill demands starting over. Use Python/Node examples as protocol references while keeping the application backend in Rust.

The speech skill covers ASR/TTS, not Nemotron LLM tool orchestration. Use current Twilio documentation directly for ConversationRelay. Verify `setup`, `prompt`, `interrupt`, `dtmf`, speaker events, `text`, `play` and `end` against the current protocol; do not substitute Media Streams messages or copy an outdated example blindly.

Write a concise `docs/bootstrap-audit.md`, `docs/provider-preflight.md`, and `docs/progress.md`. Record available tools, missing setup, selected route, capability status and next gate. Then continue into application code in the same work session. Keep these documents short and current.

## 3. Product and architecture

FirstRing answers unfamiliar callers for fictional Sarah. A judge calls a real Photon number and improvises the conversation. Nemotron asks context-dependent follow-ups and proposes tools. Sarah receives a private spoken summary and a keypad choice for an ordinary call.

The signature interaction is revocable pending consent: while Sarah's offer is pending, the caller changes the request to an authentication secret. FirstRing withdraws that offer, rejects an old acceptance, and independently calls a designated family contact. A different number's paraphrase retrieves the same tactic, and a second fictional household receives minimized tactic memory.

Use:

- Rust, Axum/Tokio, typed provider adapters, `reqwest`, structured tracing and SQLite where appropriate.
- Photon 2.0 SMS-number voice profile → authenticated SIP → Twilio SIP Domain/Programmable Voice → ConversationRelay text WebSocket → Rust → NVIDIA Nemotron.
- `https://integrate.api.nvidia.com/v1/chat/completions`, model `nvidia/nemotron-3.5-lightning-30b-a3b`, subject to a real access/tool-call probe.
- React/TypeScript/Vite for a new frontend; preserve an existing suitable frontend.
- One parent actor per inbound session, owning all child legs, plus a durable action ledger and typed UI events.

Gather is the initial transport probe and the setup-blocked conversational fallback. ConversationRelay is the primary interactive path. Do not build a SIP/RTP server, raw Media Streams audio pipeline, second reasoning model, vector database or distributed platform for the core.

## 4. Implement in this order

**Milestone 1: smallest real transport slice.** Implement signed inbound TwiML, a disclosed assistant greeting, one keypad response, and a separately triggered authenticated operator action calling a designated Phone B through Photon outbound SIP. B hears a fictional private test message and acknowledges it. Capture actual call progression and correct-leg DTMF. No AI, clone or conference is required for this first checkpoint. Add the authenticated demo action and narrow probe routes needed for it without inventing a parallel production call-control system.

Before provider calls, verify the exact SIP credentials, endpoint, digest/TLS configuration and Calls API fields in the master guide and current official documentation. Use contact IDs resolved to enrolled numbers. Validate signatures against the configured public URL and exact request representation; verify the Rust implementation against official SDK test vectors or a reproducible comparison with the official validator. Do not disable validation to make a call work.

**Milestone 2: live conversation and real Nemotron tools.** Add Relay and native tool-call/result handling. Two different caller details should produce relevant different follow-ups. Keep tool schemas and evidence validation aligned with the master guide. Timebox blocked Relay onboarding to 30–45 minutes, then use the explicit Gather fallback. Do not turn an onboarding problem into a raw-audio rewrite.

**Milestone 3: private consent and independent contact.** Keep caller A on Relay while separate Sarah leg B receives its private summary and keypad choice. Only current consent can join the intended legs in a unique conference. Implement the independent family callback and acknowledgement. Show request accepted, ringing, answer and acknowledgement separately.

**Milestone 4: signature revocation.** New speech closes the bridge gate. A committed risky assessment revokes the offer; obsolete work and old DTMF cannot revive it. For the physical stale-key demonstration, keep B's existing Gather alive for a bounded window after revocation, reject the old 1, explain and end B. Keep the parent actor alive after A ends until authorized child actions settle.

**Milestone 5: polished evidence and community.** Build the interface from real typed events: readable transcript, current action, supporting quote, private offer, leg progression and a compact two-household strip. Once contracts are stable, frontend work may run in parallel. Add expiring tactic capsules and a real subscriber-applied receipt; wait for that receipt before claiming Maya used the update on her next call. Match requested action and context, not caller ID. Include a benign new-number control.

**Milestone 6: one ready enhancement, then rehearsal.** Add fresh voice cloning only if the approved endpoint is ready; generated audio can use Relay `play`. Preset TTS is not a clone. Optional Photon text continuity should use an already functioning channel. Prioritize the core demo and held-out evaluation over either enhancement. Finish the runbook, actual results, fallback demonstration and submission material.

Proceed between milestones without asking whether to continue. If a provider gate is blocked, finish its local implementation/tests/runbook and continue independent work with clearly labeled fixtures. Do not mark the live gate passed or quietly substitute a provider. Maintain the concrete outstanding phone-test checklist.

## 5. Call-control invariants

- Rust authorizes actions; the model cannot dial arbitrary numbers, bridge directly, change policy or choose arbitrary URLs.
- Bind offers to session, leg, version and expiry. Recheck atomically before bridge commitment; unresolved speech blocks acceptance.
- Record an assessment for every committed caller turn. Read-only tools do not clear the unassessed-turn hold. Reject stale model, audio and tool results using generation/epoch checks.
- Reserve side effects before submitting them. Include the reserved action ID in the signed status callback URL so a callback can reconcile a lost create response. Never blindly retry an ambiguous call-creation timeout.
- Duplicate or reordered callbacks cannot repeat calls, regress terminal state or invent acknowledgement. Do not assume a universal provider event ID.
- Caller A never joins the independent trusted-contact call. Keep private summaries private. Confirm both intended participants before showing a bridge as connected.
- Core screening ends at approved bridge commitment. Do not claim continuous monitoring of the later human conversation.
- Interpret meaning, direction, quotation and negation. “Never share your code” and an appointment reference are not extraction requests.
- Share categorical tactic memory, not names, numbers, full transcripts, recordings or credentials. A match means similar tactic, not same person or model retraining.
- Replay mode cannot issue real calls. Separate execution mode `live|replay` from data origin `synthetic`; provider readiness can be pending. Fictional data can participate in real execution.

## 6. Autonomy and setup

Routine local implementation, project dependencies, testing and applicable skill use are authorized by this request. Pin compatible dependencies in project lockfiles. Preserve user changes; avoid global installs and unrelated account changes. Keep secrets in environment variables, never in chat, logs, screenshots or Git.

Use existing authorization rather than asking again. Before real external tests, establish which configured contacts are the designated test endpoints and whether the run is authorized; ask only for missing facts or scope. Provisioning, purchases and account changes need their own authorization if not already supplied. Prepare the concrete action before asking. Continue independent coding while a human completes account setup or supplies endpoints.

If delegation is available, give one owner the actor/provider state, another the frontend after the event contract is fixed, and another meaningful tests/fixtures. Do not let agents independently change shared contracts. Use the smallest useful team and integrate frequently.

## 7. Verification and handoff

First test valid/invalid signatures, keypad behavior, unlisted-contact rejection, duplicate outbound actions, reordered statuses, timeout reconciliation and inert replay mode. Add current versus stale acceptance, revocation races, cancellation and conference privacy tests when those features exist. Use generated event sequences when they test a real invariant, not just to advertise property testing.

For a new Cargo workspace, run the applicable checks and report actual outcomes:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

Run the frontend's actual lint/typecheck/build/test commands once it exists. Exercise the rendered UI with available browser tooling and inspect screenshots. Skill discovery, a running dev server and passing fixtures are not evidence that a phone interaction works.

At each gate, report briefly: what changed, what was actually tested, what is still dependent on setup, and the next concrete step. Maintain a working README, `.env.example`, provider preflight, demo runbook and truthful capability matrix. Preserve the final two hours before the actual deadline for rehearsal, submission and recovery where the remaining window allows it.

Start now with repository inspection, the brief setup audit and Milestone 1 implementation. The first proof is two real phones. The interaction to protect is a live, revocable private handoff followed by a real independent contact.
