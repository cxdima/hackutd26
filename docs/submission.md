# Submission preparation — draft, not submitted

**Nov8 campaign update:** actual Brev H100 Nemotron now completes native assessment/action cycles and real phone message/protection flows. A distinct second Photon line successfully originated a call into FirstRing in attempt16; this is automated test transport, not Sarah/private-handoff proof. Attempt18 saved an interrupted time correction and closed cleanly. Matched speech-timeout trials measured4.87→4.55s caller-end→closing audio; model time~1.75s. Identity/human preference and pickup phrasing remain known behavioral failures. No weight/adapter training occurred. The GPU has a fixed09:39:50UTC cleanup deadline; check current availability before presenting.

FirstRing explores a private, revocable choice before an unfamiliar caller reaches a household. NVIDIA Nemotron proposes evidence-backed tools; Rust controls permission, private phone legs and independent enrolled contacts. A minimized, expiring tactic can be applied by another fictional household.

The selected backend must pass its own current native assessment/action cycle before a new live demonstration. Historical hosted NVIDIA receipts do not verify a Brev deployment, and an isolated successful request is not a complete conversational baseline. No model has been trained or fine-tuned for FirstRing. Sarah/Phone B and independent family calling remain disabled and unverified.

Current evidence includes a **verified live caller-only exchange at 22:10 Central on Nov 7**: Photon → Twilio Relay → NVIDIA Nemotron native tools → Rust results → audible contextual replies, protective refusal, minimized incident note and automatic hangup after the full goodbye. The caller confirmed hearing these outcomes; Twilio reports a completed 75-second inbound call. [Live call receipt](evidence/nvidia-relay-call-2026-11-07.json).

Eight in-call responses returned the exact specified model, `nvidia/nemotron-3.5-lightning-30b-a3b`. HTTP latency was **1120–2892ms, median 1796ms**; the caller estimated **3–5 seconds** per reply. Four assessments, three questions and one `take_message` were accepted. The call exposed redundant questioning and a premature appointment-reference classification before a safe code clarification. Full Relay rehearsal still needs deliberate interruption, obsolete-response rejection and reliable behavior in the noisy hackathon room. The successful baseline was useful, not a quiet-environment requirement. Scope remains caller A only, with all three household/contact calling flags false and Phone B unavailable. The [recorded automated batch](evidence/automated-caller-batch-2026-11-07.json) contains **four reservations: one provider failure and three completed caller legs; zero Sarah/family calls**. Completed transport is not a behavioral pass: the first clinic call ended prematurely, while the explicit sign-in request produced a protective saved note. Additional test allowance is separate from outcomes; read the server's durable attempt counts rather than inferring that authorized calls occurred.

Measured language behavior: caller-only development **3/3**; frozen held-out **7/12**, a failed behavioral gate, not an accuracy claim. The latter used 23 real model requests, with one rejected assessment and no API/deadline errors. See [evaluation](evaluation.md). The original **36 development audio clips** were preserved; **three additional matched-family George clips** were synthesized in a separate cache. Selected original clips reached the actual automated phone path; that does not establish listening, ASR or behavioral quality for every clip. The family opening/benign continuation reached the phone in attempt15, revealing a reversed pickup question while ultimately saving the time/place; subjective listening remains pending in their [synthesis receipt](evidence/audio-family-development-2026-11-07.json). Six inert actor voice-kit tests passed; interactive model evaluation retains both failures and conversation-context repairs. These are **synthetic development evaluation**, not training examples used to fine-tune Nemotron. Model HTTP completion, native proposals/results and phone outcomes remain separate evidence.

Sponsor mapping: Photon beta SIP has recorded inbound phone proof; distinct-line synthetic outbound transport is proven; the separate Sarah/private-handoff path remains pending. NVIDIA's specified Nemotron model has both native protocol and one live model-driven voice exchange receipt; the frozen behavioral gate still failed. T-Mobile's theme of safeguards and legitimate-call preservation is represented by the implemented policy and synthetic tests. No sponsor partnership, deployment, loss prevention, calibrated fraud score or customer traction is claimed.

Demo available now: the live exchange receipt plus clearly labeled local replay of benign consent, revoked old choice, independent contact progression and an actual local household memory receipt. The 58-second replay recording and screenshot are under `.tools/artifacts/`; see [verification](verification.md). That video is a fallback explanation, not a recording of the live phone exchange. Earlier failed attempts remain in the evidence history.

## 90-second talk track

**0–20 seconds — show the live receipt.** “FirstRing screens unfamiliar callers for a fictional household. In this real phone test, NVIDIA Nemotron asked follow-ups, clarified a code request and proposed saving a protective incident note. The caller heard the refusal and complete goodbye, then the call ended automatically.”

**20–40 seconds — show caller words and the decision panel.** “The model proposes; Rust authorizes. The console separates committed caller words, an accepted assessment and its supporting quote, the proposed tool, and the actual execution result. A request does not count as ringing, an answer, or an acknowledgement.”

**40–65 seconds — label the deferred Phone B branch as replay.** “This next section is replay: no phone calls or fresh model inference. A clinic request becomes a request for sign-in digits. The pending offer is withdrawn, and the old acceptance is rejected. The separate contact acknowledgement shown here is authored input to the real local actor.”

**65–90 seconds — show the limitation and next gate.** “Caller-only development passed three of three cases; held-out behavior passed seven of twelve, so that gate failed. We still need a deliberate interruption and reliable conversation in this noisy room. Outbound calling is disabled. Replay demonstrates a local subscriber receipt; live private handoff and family calls remain unverified.”

Update each live claim only from its recorded result and human confirmation where audibility matters. The 22:10 call had no interrupt/recovery event; its success cannot pass that separate gate. Do not infer success from a flag, queued audio or a model proposal.

Before submitting:

- [ ] Create/update the initial Devpost entry by Saturday Nov 7, 23:00 Central.
- [ ] If Phone B is later enrolled and authorized, complete the two-phone proof; otherwise keep that gate pending.
- [x] Complete one caller-only native-tool voice exchange, protective message and heard goodbye/automatic hangup (22:10 receipt).
- [ ] Complete deliberate interruption/obsolete-response rejection and noisy-room rehearsal; no Phone B request until enrollment and scope exist.
- [ ] Obtain additional scope before handoff/family phone rehearsals.
- [x] Run the frozen held-out language set with actual Nemotron: 7/12, failed behavioral gate; counts and errors are in evaluation.md.
- [ ] Record the 90-second evidence walkthrough with the current gate labels; label every replay portion explicitly.
- [x] Capture a sanitized replay console screenshot; architecture diagram is in README.
- [ ] List reused crates, npm packages and pinned skill sources/licenses.
- [ ] Verify track/portal requirements and save submission confirmation by Sunday Nov 8, noon Central.

Reserve Sunday 10:00–12:00 for recovery, rehearsal and submission. Optional cloning and text are omitted; Rachel-v4 and the audio fixtures use presets. No submission confirmation has been recorded. Account setup and the Photon line purchase are documented separately; this draft does not publish a portal entry.
