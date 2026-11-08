# Configure FirstRing locally

**Latest update,22:54Central:** the user completed the individual profile; read-only TrustHub API confirms `twilio-approved`. Automated attempt2 then connected through Photon, delivered a real caller clip to production Relay ASR, completed two native NVIDIA requests, persisted a note and ended with a captured recording. The scripted conversation failed because it ended before collecting the missing appointment time. The prior KYC block below is resolved, not a current credential/setup request. [Batch evidence](evidence/automated-caller-batch-2026-11-07.json).

**Current blocker (Nov 7, 22:47 Central):** automated Calls creation returns HTTP401 / 20003 with the explicit reason that the **primary compliance profile is not approved**. Existing Rust/Python read-only authentication succeeds; inbound Relay remains working. Complete **Twilio Console → Communications → Trust Hub → Profiles → Primary profile** and wait for **Twilio Approved**. Use the applicable individual/business profile; no new credentials, phone number, messaging plan or credit purchase is indicated by this error. [Provider instructions](https://www.twilio.com/docs/trust-hub/profiles/primary-compliance-profiles), [actual attempt receipt](evidence/audio-caller-canary-2026-11-07.json).

**Live exchange verified Nov 7 at 22:10 Central:** the caller heard contextual Relay replies and a protective refusal; Rust saved a minimized note, the goodbye finished and the call ended automatically. Eight in-call responses returned the exact NVIDIA model. Twilio is Full/active, the purchased Photon line and `PHOTON_NUMBER` are configured, and runtime stays `live` / `relay`. [Live receipt](evidence/nvidia-relay-call-2026-11-07.json).

All three outbound flags remain false and the ledger has **one failed fixture reservation and zero Sarah/family reservations**. Phone B is unavailable; the user's personal phone is caller A only. Rachel-v4 supplies Relay speech and Joanna supplies Say/Gather and the verified closing. Full rehearsal still needs deliberate interruption and reliability in the noisy hackathon room. A quieter baseline helped diagnosis, but quiet surroundings are not an acceptance requirement. Preset TTS is not a clone.

## Remaining live checks and deferred setup

1. **Current-room interruption check:** no new account setup is needed for caller A. Keep the configured HTTP/2 ingress, `/voice/incoming`, `/voice/status`, credentials and ledger. The deployed 1000ms speech timeout and bounded orphan-partial recovery preceded the verified 22:10 exchange, but that call did not exercise interruption/recovery. Recheck public reachability and run the requested deliberate interruption in the actual noisy room. Backchannel filtering is now requested; its effect in real noise is not yet proven. [Earlier failures](evidence/relay-attempts-2026-11-07.json), [next test](demo-runbook.md).
2. **Deferred Phone B:** no second phone is currently available. Leave `SARAH_TEST_E164` unset and every outbound flag false. Enrollment and outbound scope are only needed if that later gate becomes feasible; they do not block the caller-only trial or initial submission.

The added card resolved Photon outbound entitlement; MD5/TLS credentials and inbound TLS/SRTP remain configured. This does not authorize or prove outbound delivery. Hosted model protocol passed. A-only model development passed 3/3; fresh frozen held-out behavior failed at 7/12. See [evaluation](evaluation.md); the original 3/12 run remains history. Brev remains deferred.

The configured Twilio SIP domain is `hackutd26-firstring-d232a271.sip.twilio.com`. It requires TLS/SRTP and an attached credential list, routes Voice via POST to the current `PUBLIC_BASE_URL` plus `/voice/incoming`, and status to `/voice/status`. Locally generated inbound SIP credentials are stored in `.env` as `TWILIO_SIP_USERNAME` / `TWILIO_SIP_PASSWORD`, separate from Photon outbound credentials. No credentials were rotated. The project-local setup records retain resource IDs and prevent blind re-creation after ambiguous results.

Setup used the [official Twilio SIP API](https://www.twilio.com/docs/voice/sip/api/sip-domain-resource), [Photon Beta API](https://api.photon.codes/openapi.json) and [current cookbook](https://photon.codes/docs/beta/cookbooks/voice/twilio.md). No MCP is required: native authenticated APIs already connect the services. No Twilio/Photon plugin was found in the available catalog; no plugin/global browser configuration was installed.

Create `.env` from `.env.example` without overwriting any existing file, then edit locally. The server reads `.env` automatically on startup; restart after changes. Git ignores `.env`, `.env.*`, databases and `.tools/`; only `.env.example` is intentionally tracked.

```sh
cp -n .env.example .env
chmod 600 .env
# Verify protection without displaying contents:
git check-ignore .env
```

`DEMO_ADMIN_TOKEN` is already generated in `.env` with mode 600 file permissions; an authenticated local API read succeeded without displaying it. Enter its value locally in the console; do not send it in chat. Preserve it on subsequent setup edits. This token is not a Twilio or Photon credential. The separate replay helper uses `.tools/replay-admin-token`; the currently running normal service uses `.env`.

## Existing runtime values and deferred Phone B

| Variable | Where its value comes from |
|---|---|
| `PUBLIC_BASE_URL` | The exact public **HTTPS origin** assigned by your existing tunnel or deployment, e.g. `https://your-host.example`. No `/voice` path, query or credentials. Point that ingress at local port 8080 and keep it running. |
| `PORT` | Local choice; keep `8080` unless another service uses it. |
| `DATABASE_URL` | Local choice; keep `sqlite:firstring.db`. Preserve this database because it contains the one-attempt ledger. |
| `DEMO_ADMIN_TOKEN` | Generate locally as above; at least 24 characters. |
| `DEMO_EXECUTION_MODE` | Current phone checkpoint uses `live`; use an isolated `replay` service for development fixtures. |
| `TEST_CALLS_AUTHORIZED` | Keep `false` for A-only testing. Enable only when the designated `SARAH_TEST_E164` is ready for the single smoke attempt; retries and other roles are not covered. |
| `TWILIO_ACCOUNT_SID` | Already configured; authenticated account status is Full / active. Preserve the value. |
| `TWILIO_AUTH_TOKEN` | Already configured and authenticated with that account. Used for API Basic auth and signature validation; preserve it. |
| `PHOTON_NUMBER` | Already configured from the purchased line on Photon Beta / 2.0's default SIP profile. Preserve its E.164 value. |
| `PHOTON_SIP_HOST` / `PHOTON_SIP_PORT` | In that Photon Voice SIP profile, open **Outbound calls** and copy the assigned **SIP connection** host/port. The guide example is `sip.photon.codes:5061`; use the actual assignment, not the authentication realm. |
| `PHOTON_SIP_USERNAME` / `PHOTON_SIP_PASSWORD` | Generate/copy the **outbound** credentials in the same Photon Voice profile. Configure digest **MD5 (legacy)** for Twilio compatibility and retain TLS/compatible SRTP. |
| `SARAH_TEST_E164` | Your designated consenting Phone B's actual E.164 number, `+` followed by country code and subscriber digits. It is entered by you, never selected by the model or caller. |
| `VOICE_TRANSPORT` | Current caller-only path is `relay`; `gather` is the explicitly labeled fallback. `probe` is the earlier keypad transport checkpoint. |
| `MAX_CALL_SECONDS` | Local duration cap; keep `180` (range 15–600). |

Twilio documents the dashboard credentials in its [Auth Token reference](https://www.twilio.com/docs/iam/api/authtoken). Use a funded/enabled account; [trial restrictions](https://www.twilio.com/docs/usage/trials/try-out-voice) prevent the final Relay/SIP architecture. The user completed the account upgrade, purchased the Photon line, and initiated the verified inbound call. No outbound call has been placed.

The Photon values and current console labels come from its [official Beta Twilio cookbook](https://photon.codes/docs/beta/cookbooks/voice/twilio.md). If HTML access fails, the `.md` version contains the same official walkthrough. Profile edits can affect other numbers using that profile; use the intended test profile.

## Inbound SIP settings are a separate credential pair

In Twilio Console, open **Communications → Voice → SIP domains → Credential Lists**. The SIP username/password you choose there are used by **Photon inbound delivery** to authenticate to Twilio. Attach that list to the intended Twilio SIP Domain. In Photon Voice's **Inbound calls** section, configure `sips:YOURDOMAIN.sip.twilio.com`, TLS, and that Twilio credential-list login. These inbound credentials are configured between the providers; they are **not** `PHOTON_SIP_USERNAME/PASSWORD` in the Rust service.

Set the SIP Domain's Voice webhook to `POST PUBLIC_BASE_URL/voice/incoming`. Configure inbound status notifications to `/voice/status` where offered. The Rust Calls API sets outbound status URLs itself, including the reserved action ID. Complete the [provider preflight](provider-preflight.md) before spending the one smoke attempt.

No existing ingress or tunnel account was configured. A project-local **cloudflared 2026.10.7** was downloaded from the [official release](https://github.com/cloudflare/cloudflared/releases/tag/2026.10.7), its Darwin arm64 archive checked against SHA-256 `3586ccf397e98e5f437e60e7c307ca6ac463e110580503af8bb8e6f9ebf831bd`, and a free accountless voice tunnel started. No global installation or account change was made.

```sh
bash scripts/voice-tunnel.sh
# Copy the printed HTTPS origin into PUBLIC_BASE_URL, restart Rust, and
# update the Twilio SIP Domain Voice webhook. Keep this process running.
```

The helper selects HTTP/2. The old origin expired despite Rust remaining healthy: local 200 does not prove public availability. The replacement origin and SIP Domain URLs carried the verified 22:10 exchange; recheck health and unsigned rejection after any restart. Do not add browser-access authentication. [Quick Tunnels](https://developers.cloudflare.com/tunnel/get-started/quick-tunnels/) have no uptime guarantee and change hostname on restart. Use the console at `http://127.0.0.1:8080`; public ingress carries voice HTTP/WSS.

## Later conversation values

| Variable | Where its value comes from / current setting |
|---|---|
| `NVIDIA_API_KEY` | Already authenticated for the [requested model](https://build.nvidia.com/nvidia/nemotron-3.5-lightning-30b-a3b). Protocol passed, A-only development 3/3, fresh held-out 7/12. No new key collection is needed. |
| `NEMOTRON_CHAT_URL` | Keep the provided NVIDIA chat-completions URL. This is an application constant, not a value to invent. |
| `NEMOTRON_MODEL` | Keep `nvidia/nemotron-3.5-lightning-30b-a3b`; do not silently substitute a model. |
| `VOICE_TRANSPORT` | Current value is `relay`. Use `gather` only as the explicitly labeled setup-blocked fallback; this loses streaming interruption/concurrent revocation. |
| `VOICE_OUTPUT` | Keep `relay_native`. Current output is a preset assistant voice, not a clone. |
| `HANDOFF_CALLS_ENABLED` | Keep `false`: additional Sarah handoff calls are not yet authorized. |
| `FAMILY_CALLS_ENABLED` | Keep `false`: family is not enrolled or authorized. |
| `FAMILY_TEST_E164` / `CLINIC_TEST_E164` | Leave blank until the roles have designated consenting endpoints and their test scope is authorized. |

`PHOTON_API_KEY` is already configured and authenticated for the read-only setup audit; preserve it. Current voice call creation still uses the separate Photon SIP credentials through Twilio. `PHOTON_PROJECT_ID` and `PHOTON_WEBHOOK_SECRET` remain optional for future messaging integration. `MAGPIE_ZERO_SHOT_URL` stays blank while Brev is deferred. None of these optional adapters is needed for the core transport test.

## Check without making a call

```sh
bash scripts/cargo.sh run --bin preflight
```

The default reports variable **names** that are present/missing, not their values, and performs zero external requests. Presence does not prove correct flags, provider access or authorization. With the NVIDIA key ready, the explicit `--model-probe` flag makes bounded model-only requests; it never dials.

Preserve the current live/relay service, credentials and ledger. Follow the [noisy-room caller-only checklist](demo-runbook.md) next. Six voice-kit actor tests and 36 existing synthesized clips are development artifacts; interactive model evaluation remains pending. The console keeps model traces separate from phone outcomes. Phone B stays unavailable; any future unknown create outcome remains reserved and must not be retried blindly.
