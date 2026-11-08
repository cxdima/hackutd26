# Configure FirstRing locally

The first real inbound test **passed Nov 7 at 20:42 Central**: the user heard the assistant and pressed 1; signed caller/keypad/terminal events and Twilio's completed 15-second inbound call agree. See [inbound evidence](evidence/inbound-probe-2026-11-07.json). Twilio is Full/active, Photon default profile has the user's purchased line, and `PHOTON_NUMBER` is configured. The application runs in `live` / `probe` mode through the existing tunnel.

All three calling flags remain false. The local ledger contains **0 outbound action reservations**; Sarah's single authorized attempt is unspent. The user's personal phone is caller A only, not Sarah / Phone B. Family remains unenrolled and disabled. The initial voice was rated poor; `Polly.Joanna-Neural` / `en-US` is now deployed for Say/Gather, with a listening comparison pending.

## Remaining account actions

1. **Sarah / Phone B:** designate the separate consenting second phone and set `SARAH_TEST_E164` locally. This is the only missing core phone variable. Keep `TEST_CALLS_AUTHORIZED=false` until B is designated and ready; then enable it for the one authorized smoke attempt. Keep handoff and family flags false. A-only testing does not require outbound authorization.

The added card resolved Photon outbound entitlement. MD5 outbound credentials are saved locally and read back with the assigned TLS endpoint; inbound TLS and required SRTP remain configured. The user purchased the Photon number on the default profile, which now has one line. Outbound delivery to B is still untested. Hosted model protocol passed, while held-out behavior failed (3/12 composite passes); see [verification](verification.md). Relay/model conversation has not been verified by phone. Brev remains deferred.

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

## Values needed for the first two-phone test

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
| `VOICE_TRANSPORT` | Keep `probe` for Milestone 1; no model key is required for that checkpoint. |
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

Reuse the running ingress process; its current URL is already in `.env`. Do not add `--allowed-mail` or an access-login policy: provider webhooks cannot use browser authentication. [Quick Tunnels](https://developers.cloudflare.com/tunnel/get-started/quick-tunnels/) have no uptime guarantee and change hostname on restart. Their lack of SSE is handled by using the operator console at `http://127.0.0.1:8080`, while the public URL carries voice HTTP/WSS and later generated audio. Real signed inbound HTTP is now verified. [WebSocket support](https://developers.cloudflare.com/network/websockets/) is documented, but the actual signed Relay session remains pending. Public/local health returned 200 and unsigned POST returned 403 after the preset-voice deployment.

## Later conversation values

| Variable | Where its value comes from / current setting |
|---|---|
| `NVIDIA_API_KEY` | Already authenticated for the [requested model](https://build.nvidia.com/nvidia/nemotron-3.5-lightning-30b-a3b). Native protocol passed; held-out behavior failed. No new key collection is needed. |
| `NEMOTRON_CHAT_URL` | Keep the provided NVIDIA chat-completions URL. This is an application constant, not a value to invent. |
| `NEMOTRON_MODEL` | Keep `nvidia/nemotron-3.5-lightning-30b-a3b`; do not silently substitute a model. |
| `VOICE_TRANSPORT` | After the first phone checkpoint, use `relay` after Twilio ConversationRelay onboarding/addendum. Use `gather` for the explicit setup-blocked fallback. |
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

Preserve the current live/probe service, tunnel and ledger. Follow the [two-phone checklist](demo-runbook.md) once B is designated. A calls manually; the console's authenticated Phone B action uses only `contact_id="sarah"`. Unknown create outcomes remain reserved and must not be retried blindly.
