# FirstRing demo runbook

## Current 90-second caller-only demonstration

Use the already working inbound line and local console. No Phone B or family call is part of this version. Start as a clinic changing an appointment; let Nemotron ask for a missing detail. Branch to a supplied ordinary appointment reference (continue normally), or change the request to Sarah's sign-in digits (accepted risky assessment, Rust blocks handoff, native `take_message` saves a categorical note, truthful goodbye). Show the actual caller transcript, current evidence quote, native proposal, accepted result and persisted note. If a model request fails, show that failure; do not narrate a saved note unless it exists.

For an evidence walkthrough without another manual call: show the22:10 correlated live receipt (listener confirmed3–5s replies/refusal/closing), then the newer noisy-call receipts and actual failures. Any private-choice, stale-key, family or community replay must retain its replay label. Those phone capabilities remain unverified.

Automated testing is the default next step after the account gate. The first originator reservation was rejected by Twilio because its primary compliance profile is unapproved; the exact action is Communications → Trust Hub → Profiles → Primary profile. Do not repeat credentials/setup or spend Sarah's attempt. Preserve attempt1; at most six automated attempts including retries are authorized. See [automated caller instructions](audio-caller-canary.md). No phone-audio or noise pass can be claimed from the current failed canary.

**Current priority after the verified 22:10 Central exchange: caller A only, in the actual noisy hackathon room.** Runtime is `live` / `relay`; all three outbound flags remain false. Phone B is unavailable and the ledger contains 0 outbound actions/reservations. Keep outbound controls unavailable. Initial submission is due at 23:00 Central; submit recorded evidence without waiting for the deferred two-phone gate.

**Observed phone proof:** the 22:10 call completed four accepted assessments, three questions and one saved incident note using eight responses from the exact NVIDIA model. The caller confirmed contextual replies, protective refusal, full goodbye and automatic hangup. HTTP latency was 1120–2892ms, median 1796ms; perceived replies were 3–5 seconds. No interrupt or recovery event occurred. Rachel-v4 supplies Relay speech, Joanna the closing. Earlier failures remain recorded. [Live receipt](evidence/nvidia-relay-call-2026-11-07.json), [inbound keypad proof](evidence/inbound-probe-2026-11-07.json), [failure history](evidence/relay-attempts-2026-11-07.json).

## Next caller-only check

Keep the deployed 4s inactivity / 8s repeated-partial recovery and signature validation. Verify public health against the configured origin; local health is insufficient. The successful baseline helped diagnosis, but quiet surroundings are not an acceptance condition. For the requested current-room manual call:

1. Hear the greeting, give a clinic detail and confirm a relevant audible follow-up despite room chatter. Record false interruptions, redundant questions, silence and recovery prompts separately.
2. Deliberately speak over a reply with a fictional request for sign-in digits. Capture actual `interrupt`/prompt, cancellation or obsolete-output rejection, then the new accepted assessment, tool result and audible protective response. A model HTTP completion alone is not playback.
3. Confirm the complete goodbye and automatic hangup after the minimized incident note. Do not inject a benign-message branch into an already closed call; that branch needs a separate manual inbound test.
4. Record approximate heard delay and room conditions, alongside instrumented HTTP/dispatch timings. No outbound leg is enabled; a stale result must never authorize one. Keep failure evidence if the room noise prevents a usable exchange.

Caller-only development passed 3/3; frozen held-out behavior failed at 7/12. Six voice-kit actor tests pass; continued interactive model evaluation remains pending implementation/results. The 36 generated clips remain unlistened, with no new rendering. See [evaluation](evaluation.md), [audio fixtures](audio-fixtures.md), [kit](voice-test-kit.md) and [submission talk track](submission.md). Label any replay explicitly.

## Current speech settings and noise limits

Twilio accepted the requested Deepgram `nova-3-general`, `speechTimeout=1000` and Rachel-v4 configuration; internal ASR version was not reported. The watchdog keeps uncertain speech uncommitted and consent held. No noise-control tuning has been applied yet. Review this room's result before a small configuration experiment; lower TTS interruption sensitivity does not suppress every partial transcript or isolate the caller from other speakers. [Researched options and limits](provider-preflight.md#noisy-room-options--researched-not-applied). No Phone B attempt is consumed.

## Start locally

1. Copy `.env.example` to `.env` only if `.env` does not already exist. Populate secrets locally; never paste them into chat or commit them. Keep `DEMO_EXECUTION_MODE=replay` for development. Set a random `DEMO_ADMIN_TOKEN` of at least 24 characters.
2. Run `bash scripts/cargo.sh run --bin firstring` from the repository. The helper uses an existing Cargo or the isolated `.tools` installation.
3. For frontend development run `npm ci` and `npm run dev` from `web/`. The Vite proxy connects `/api` to port 8080. For the packaged console build with `npm run build`; the backend serves `web/dist` once built.
4. Open the console, enter the local admin token and inspect readiness. A running server, configured credential or replay receipt is not a successful telephone test.

These commands are the intended application entry points; use the current README/verification log for the actual build status.

## Deferred two-phone gate

The historical single Sarah smoke attempt is unspent. This section is deferred until a separate consenting Phone B is enrolled and the operator chooses to use that scope; it is not the current caller-only procedure. Before that attempt:

- Configure Photon beta voice profile → the authenticated Twilio SIP Domain → `POST https://HOST/voice/incoming`. Use separate Photon outbound credentials and the assigned SIP endpoint. Verify TLS, compatible SRTP, digest **MD5 (legacy)** and eligible Photon caller number against `provider-preflight.md` and the actual profile.
- Set `PUBLIC_BASE_URL` to the exact public HTTPS origin; make HTTPS/WSS reachable. Do not disable signature validation to work around a tunnel or URL mismatch.
- Keep `VOICE_TRANSPORT=probe`, `DEMO_EXECUTION_MODE=live`, `TEST_CALLS_AUTHORIZED=false`, `HANDOFF_CALLS_ENABLED=false`, and `FAMILY_CALLS_ENABLED=false` for A-only testing. Leave `FAMILY_TEST_E164` empty.
- Designate the separate consenting Phone B and configure `SARAH_TEST_E164`. Only when B is ready for the one authorized smoke attempt, set `TEST_CALLS_AUTHORIZED=true` and restart to load it; keep the other calling flags false. The console accepts role IDs, never a dialed number. Unmute and charge both phones; allow the configured caller number through filtering.

Run once:

1. A manually calls the Photon number. Confirm the disclosed assistant greeting is audible. Press the requested keypad digit and verify `transport.keypad` belongs to the caller leg.
2. Select that inbound session in the console. Trigger the authenticated Phone B probe once: `POST /api/demo/probe` with `{"session_id":"<that session>","contact_id":"sarah"}`. The operator token goes in the Authorization header, never a URL.
3. Observe action reservation, provider acceptance/requested, ringing and answer as separate states. Record the opaque action/session IDs for reconciliation.
4. B hears the fictional private test message and presses 1. Confirm `contact.acknowledged` belongs to B. A's keypad must not acknowledge B. Verify A does not hear B's private prompt.
5. End the test phones and wait for terminal callbacks. A provider create response alone does not prove ringing, answer, audible audio or acknowledgement.

If create times out, the action outcome is **unknown**. Keep its reservation, wait for the signed callback containing the action ID, and reconcile the actual provider call. Do not click again, delete the ledger, reset the database or redial. Even a failed/ambiguous attempt consumes the current single-attempt scope. Record the observed result before seeking additional call scope.

Phone proof record:

| Check | Observation / receipt |
|---|---|
| A reached Photon/Twilio and heard greeting | Passed Nov 7, 20:42 Central; user confirmed audio, initial voice rated poor |
| A keypad accepted on A only | Passed; caller key 1 at Nov 8 `02:42:43.779833Z` |
| Single outbound reservation/submission | Unspent; 0 reservations/actions, B designation missing |
| Photon-routed B ringing / answered | Pending |
| B heard private fictional message | Pending |
| B's 1 acknowledged B only | Pending |
| A did not hear B's private message | Pending |
| Terminal statuses reconciled | A passed: Twilio completed inbound, 15s; `call.ended` at Nov 8 `02:42:49.144009Z`. B pending |

## Conversation and full branch gates

This section is a checklist for later authorized tests, not permission for additional calls.

- Relay: signed setup, native tool/result exchange and heard closing passed for the 22:10 call. Still capture deliberate interruption/obsolete-result rejection and a usable noisy-room exchange; improve redundant questioning. Optional speaker events remain disabled until their envelope is verified. Do not infer broader conversational accuracy from one call.
- Timebox an actual Relay onboarding blocker to 30–45 minutes. Switch explicitly to `VOICE_TRANSPORT=gather` and label the lost streaming-interruption capability. Do not replace it with raw audio infrastructure.
- Benign consent: A stays screened while B hears the private summary. Test current 1 → exactly A+B two-way audio; 2/silence/expired offer → no connection. The UI says connected only after both intended participant joins. Each repetition is another outbound attempt.
- Revocation: while B's Gather remains active, A changes the request to an authentication secret. Wait for the risky assessment and revocation receipt, then press B's old 1 within the bounded rejection window. Hear the withdrawal explanation; verify no bridge. A physical key after B has already hung up is not this proof.
- Family: enroll and authorize a separate family endpoint before enabling it. Verify independent request → ringing → answer → correct-leg 1 acknowledgement. A never enters this call. Also test no-answer without inventing acknowledgement.
- Parent lifetime: end A while an authorized child action is pending; verify child answer/acknowledgement/terminal state still reconciles.
- Community: after a risky call, wait for Maya's stored `community.applied` version. Select Maya before a new call from a different phone. Use a paraphrase; show matched action/context/version. Follow with a benign new-number reminder. No caller-ID match, person identification or retraining claim.

## Deferred three-phone demonstration

### Next bounded rehearsal scope — prepared, not yet authorized

After M1 and the hosted model/Relay gates pass, the minimum two-branch rehearsal needs **two new manual inbound A calls, two additional outbound Sarah/B handoff attempts, and one independent outbound family/C attempt**. Run benign consent first (B accepts, confirm exactly A+B), then risky revocation (B leaves its private Gather open, A requests sign-in digits, B presses the stale 1 within five seconds, C independently answers/acknowledges while A ends). Use three distinct consenting endpoints; C is not enrolled yet. The existing single Sarah smoke authorization does not cover these three additional outbound attempts. Obtain that exact additional scope and C's role designation locally before enabling either calling flag; no automatic retry/no-answer repeat is included. Set a fixed two-minute cap per call and stop after these attempts. Community recurrence and a benign new-number phone control are separate later scope. Keep all ledger reservations through any failure or restart.

Invite the caller to start as the fictional clinic and improvise. Show the contextual follow-up and private offer. Let their speech choose benign acceptance or risky revocation. Narrate only observed states: requested, ringing, answered, acknowledged. On a risky branch, show the independent contact; then, after the applied receipt, show the different-number recurrence. If the first branch was benign, the later risky call is the first incident, not recurrence.

Use a three-minute cut to show both endings and the benign control. Say “preset assistant voice” unless an approved reference-conditioned clone generated fresh audible output. Skip optional cloning and messaging unless already tested. Core screening ends when the approved bridge commits.

## Recovery, freeze and submission

- Auth/signature failure: inspect configured URL and exact request representation; keep validation enabled.
- Model failure: preserve isolation and report dependency failure; after accepted current risk, refuse and close without claiming a saved note; for an unblocked turn, offer message-taking; never replay a cached decision or retry an ambiguous call action.
- Carrier/network failure: show the actual pending/failed state. Switch to a visibly labeled replay for explanation; do not call it a live phone proof.
- Rehearsal: reserve three full runs once authorized, including one dependency failure. Capture measured audio/privacy observations and a clearly labeled backup recording.
- Submission schedule from verified event page: initial entry Saturday Nov 7 at 23:00 Central; final Sunday Nov 8 at 12:00 Central. Preserve Sunday 10:00–12:00 for rehearsal/submission/recovery if available. Organizer changes take precedence.
- Final package: actual capability matrix, tested revision/time, setup instructions, sanitized architecture screenshot, evaluation counts/errors, 90-second evidence recording, known limits, reused components/attribution and portal confirmation. Do not mark the package submitted until the portal confirms it.
