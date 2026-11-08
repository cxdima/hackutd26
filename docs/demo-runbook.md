# FirstRing demo runbook

Current live authorization: **one outbound smoke-test call to Sarah / Phone B**, resolved from local `SARAH_TEST_E164`. Caller A initiates inbound manually. Family is not enrolled or authorized; keep family calling disabled. Later handoff calls, no-answer retries and full rehearsals need their own authorized scope. Local replay/tests remain available without dialing.

**Inbound passed Nov 7 at 20:42 Central:** A heard the greeting and pressed 1; signed caller/keypad/terminal events and Twilio's completed 15-second call agree. See [inbound evidence](evidence/inbound-probe-2026-11-07.json). The original voice was rated poor; `Polly.Joanna-Neural` / `en-US` is deployed for Say/Gather, listening comparison pending. Runtime remains `live` / `probe`, all three calling flags false, with **0 outbound actions/reservations**. `SARAH_TEST_E164` is still missing; the user's personal phone is caller A only. M1 is not complete.

## Start locally

1. Copy `.env.example` to `.env` only if `.env` does not already exist. Populate secrets locally; never paste them into chat or commit them. Keep `DEMO_EXECUTION_MODE=replay` for development. Set a random `DEMO_ADMIN_TOKEN` of at least 24 characters.
2. Run `bash scripts/cargo.sh run --bin firstring` from the repository. The helper uses an existing Cargo or the isolated `.tools` installation.
3. For frontend development run `npm ci` and `npm run dev` from `web/`. The Vite proxy connects `/api` to port 8080. For the packaged console build with `npm run build`; the backend serves `web/dist` once built.
4. Open the console, enter the local admin token and inspect readiness. A running server, configured credential or replay receipt is not a successful telephone test.

These commands are the intended application entry points; use the current README/verification log for the actual build status.

## First two-phone gate

Before using the one authorized attempt:

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

- Relay: after transport and hosted model proof, complete onboarding/addendum, validate signed WebSocket connection and session token, capture sanitized actual setup/partial/final prompt/interrupt/DTMF frames. Optional speaker events are disabled until their envelope is verified; do not claim their coverage. Confirm two different caller details produce different relevant follow-ups and a native Nemotron tool/result loop.
- Timebox an actual Relay onboarding blocker to 30–45 minutes. Switch explicitly to `VOICE_TRANSPORT=gather` and label the lost streaming-interruption capability. Do not replace it with raw audio infrastructure.
- Benign consent: A stays screened while B hears the private summary. Test current 1 → exactly A+B two-way audio; 2/silence/expired offer → no connection. The UI says connected only after both intended participant joins. Each repetition is another outbound attempt.
- Revocation: while B's Gather remains active, A changes the request to an authentication secret. Wait for the risky assessment and revocation receipt, then press B's old 1 within the bounded rejection window. Hear the withdrawal explanation; verify no bridge. A physical key after B has already hung up is not this proof.
- Family: enroll and authorize a separate family endpoint before enabling it. Verify independent request → ringing → answer → correct-leg 1 acknowledgement. A never enters this call. Also test no-answer without inventing acknowledgement.
- Parent lifetime: end A while an authorized child action is pending; verify child answer/acknowledgement/terminal state still reconciles.
- Community: after a risky call, wait for Maya's stored `community.applied` version. Select Maya before a new call from a different phone. Use a paraphrase; show matched action/context/version. Follow with a benign new-number reminder. No caller-ID match, person identification or retraining claim.

## Ninety-second cut after live gates pass

### Next bounded rehearsal scope — prepared, not yet authorized

After M1 and the hosted model/Relay gates pass, the minimum two-branch rehearsal needs **two new manual inbound A calls, two additional outbound Sarah/B handoff attempts, and one independent outbound family/C attempt**. Run benign consent first (B accepts, confirm exactly A+B), then risky revocation (B leaves its private Gather open, A requests sign-in digits, B presses the stale 1 within five seconds, C independently answers/acknowledges while A ends). Use three distinct consenting endpoints; C is not enrolled yet. The existing single Sarah smoke authorization does not cover these three additional outbound attempts. Obtain that exact additional scope and C's role designation locally before enabling either calling flag; no automatic retry/no-answer repeat is included. Set a fixed two-minute cap per call and stop after these attempts. Community recurrence and a benign new-number phone control are separate later scope. Keep all ledger reservations through any failure or restart.

Invite the caller to start as the fictional clinic and improvise. Show the contextual follow-up and private offer. Let their speech choose benign acceptance or risky revocation. Narrate only observed states: requested, ringing, answered, acknowledged. On a risky branch, show the independent contact; then, after the applied receipt, show the different-number recurrence. If the first branch was benign, the later risky call is the first incident, not recurrence.

Use a three-minute cut to show both endings and the benign control. Say “preset assistant voice” unless an approved reference-conditioned clone generated fresh audible output. Skip optional cloning and messaging unless already tested. Core screening ends when the approved bridge commits.

## Recovery, freeze and submission

- Auth/signature failure: inspect configured URL and exact request representation; keep validation enabled.
- Model failure: preserve isolation, report dependency failure, offer message-taking; never replay a cached decision or retry an ambiguous call action.
- Carrier/network failure: show the actual pending/failed state. Switch to a visibly labeled replay for explanation; do not call it a live phone proof.
- Rehearsal: reserve three full runs once authorized, including one dependency failure. Capture measured audio/privacy observations and a clearly labeled backup recording.
- Submission schedule from verified event page: initial entry Saturday Nov 7 at 23:00 Central; final Sunday Nov 8 at 12:00 Central. Preserve Sunday 10:00–12:00 for rehearsal/submission/recovery if available. Organizer changes take precedence.
- Final package: actual capability matrix, tested revision/time, setup instructions, sanitized architecture screenshot, evaluation counts/errors, 90-second evidence recording, known limits, reused components/attribution and portal confirmation. Do not mark the package submitted until the portal confirms it.
