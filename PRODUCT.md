# Product

<!-- impeccable:product-schema 1 -->

> Source: the user's written frontend brief (Nov 8, 2026), `AGENTS.md`, `docs/progress.md`, and the running Rust console contract. The user was away and explicitly asked for decisions to be made without further questions, so every fact below is inferred from that brief and repository evidence, not from an interview.

## Platform

web

## Stack

Angular + Tailwind CSS (user decision, Nov 8, 2026, overriding the earlier React/Vite console). TypeScript, Zod validation of server contracts, GSAP for a small number of authored motion moments, Playwright for browser checks. The Rust/Axum server remains the only backend; Terraform is installed and an empty AWS scaffold is kept in `infra/` for possible future server functions.

## Users

- Older adults (often 65+) who receive unfamiliar calls and want time and support before responding. They are capable and in charge of their own decisions.
- Their family members and caregivers, who set the service up, read call summaries, and want to know what happened and why.
- An operator/demo presenter at HackUTD 2026 who needs model, provider and event evidence on demand.

## Product Purpose

FirstRing answers unfamiliar callers before the customer has to. It listens to what the caller wants, asks useful follow-up questions, protects private information, and helps the customer decide what happens next. An ordinary appointment message stays easy to receive; a request for a login code triggers protective behavior. Success: a visitor understands this within ten seconds, and a customer can see every call's outcome and the reasoning behind it.

## Positioning

Two ideas: (A) personal protection before you answer, and (B) shared knowledge that makes repeat scam tactics harder to carry out ("same tactic, different number"). FirstRing judges the request, not the caller ID: caller ID is a clue, never proof. Mission: "Our goal: help put phone scammers out of business by making their tricks harder to repeat."

## Operating Context

- Live path: Photon SIP number → Twilio ConversationRelay → Rust → NVIDIA Nemotron (Brev-hosted GPU or NVIDIA hosted endpoint). Speech: ElevenLabs voices, Deepgram transcription on Relay.
- The console reads `/api/state`, `/api/session/:id`, `/api/profile/:household`, and an authenticated SSE `/api/events` stream with a bearer operator token kept in memory only.
- All people and households are fictional (Sarah, Maya). Data origin is synthetic even on live calls.

## Capabilities and Constraints

Verified/implemented today: inbound answering, model assessment with quoted evidence, protective blocking of authentication-secret, payment-diversion and remote-access requests, minimized private message saving, profile editing (name, greeting, preset voice, timezone) for new calls, expiring categorical tactic capsules shared between the two fictional households, replay mode.

Not available or disabled: private handoff/family calls (flags off), voice cloning (disabled; preset synthetic voices only), per-call audio recordings for ordinary sessions, number reputation scoring, report corrections/disputes, carrier blocking. These must be labeled as concepts or unavailable, never as live.

## Brand Commitments

- Name: FirstRing. Mascot: Ringo, a calm, observant sea lion holding a lime ring with a restrained magenta detail.
- Palette tokens from the brief: warm white #F7F7F2, charcoal #171C18, lime #76B900, magenta #E20074, muted neutral #626B64. Avoid a lime-to-magenta gradient. No implication that NVIDIA or T-Mobile operates FirstRing.
- Ring motif: the phone ringing, a protective boundary, a community connection.
- Voice: plain, respectful, calm, casually professional. Banned words: revolutionary, seamless, cutting-edge, "AI-powered peace of mind".

## Evidence on Hand

- Real call data in the running server: transcripts, assessments with quoted evidence, tool proposals/results, policy decisions, model latency, saved messages, tactic capsules and household application receipts.
- No customers, testimonials, savings figures, detection accuracy, carrier partnerships, certifications or blocked-scam counts exist. Never fabricate them.

## Product Principles

1. Judge the request, not the caller ID.
2. The person stays in control; FirstRing adds time and support, not decisions made for them.
3. Show the evidence: every outcome links to what was heard and why the system acted.
4. Proposed is not completed: requested, ringing, answered and acknowledged stay distinct.
5. Respect, not fear: never portray age as incompetence.

## Accessibility & Inclusion

Design for comfortable use by older adults: 18–20px landing body copy, generous line height, large labeled controls, visible focus, strong contrast, meaning never by color alone, no hover-only essentials, 200% zoom and reduced-motion support, small-screen reflow.
