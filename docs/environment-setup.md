# Configure FirstRing locally

Real phone tests remain **DEPENDENT ON SETUP**. No credentials or alternative env file have been supplied. The current authorization permits exactly one outbound smoke-test attempt to Sarah / Phone B. Family calling stays disabled.

Create `.env` from `.env.example` without overwriting any existing file, then edit locally. The server reads `.env` automatically on startup; restart after changes. Git ignores `.env`, `.env.*`, databases and `.tools/`; only `.env.example` is intentionally tracked.

```sh
cp -n .env.example .env
chmod 600 .env
# Verify protection without displaying contents:
git check-ignore .env
```

For `DEMO_ADMIN_TOKEN`, generate a unique value locally with a password manager or `python3 -c 'import secrets; print(secrets.token_urlsafe(32))'`, then paste it into `.env` in your editor. Do not send it in chat. This token unlocks the FirstRing console and authenticated operator API; it is not a Twilio or Photon credential.

## Values needed for the first two-phone test

| Variable | Where its value comes from |
|---|---|
| `PUBLIC_BASE_URL` | The exact public **HTTPS origin** assigned by your existing tunnel or deployment, e.g. `https://your-host.example`. No `/voice` path, query or credentials. Point that ingress at local port 8080 and keep it running. |
| `PORT` | Local choice; keep `8080` unless another service uses it. |
| `DATABASE_URL` | Local choice; keep `sqlite:firstring.db`. Preserve this database because it contains the one-attempt ledger. |
| `DEMO_ADMIN_TOKEN` | Generate locally as above; at least 24 characters. |
| `DEMO_EXECUTION_MODE` | Keep `replay` while configuring. Set `live` only for the authorized manual-A/single-B test. |
| `TEST_CALLS_AUTHORIZED` | Set `true` for the already-authorized single Sarah smoke attempt once setup is ready. This does not authorize retries or other phone roles. |
| `TWILIO_ACCOUNT_SID` | Open the Twilio Console account dashboard and copy the live Account SID (`AC…`) for the account owning your SIP Domain. |
| `TWILIO_AUTH_TOKEN` | Reveal/copy that same account's live Auth Token locally. The app uses it for API Basic auth and signature validation. An API Key Secret or Twilio test credential is not a replacement. |
| `PHOTON_NUMBER` | In the **Photon Beta / 2.0** project, open **Voice → Lines** and copy the voice-enabled number assigned to the intended SIP profile, in E.164 format. Use the SMS-number voice route in the project plan. |
| `PHOTON_SIP_HOST` / `PHOTON_SIP_PORT` | In that Photon Voice SIP profile, open **Outbound calls** and copy the assigned **SIP connection** host/port. The guide example is `sip.photon.codes:5061`; use the actual assignment, not the authentication realm. |
| `PHOTON_SIP_USERNAME` / `PHOTON_SIP_PASSWORD` | Generate/copy the **outbound** credentials in the same Photon Voice profile. Configure digest **MD5 (legacy)** for Twilio compatibility and retain TLS/compatible SRTP. |
| `SARAH_TEST_E164` | Your designated consenting Phone B's actual E.164 number, `+` followed by country code and subscriber digits. It is entered by you, never selected by the model or caller. |
| `VOICE_TRANSPORT` | Keep `probe` for Milestone 1; no model key is required for that checkpoint. |
| `MAX_CALL_SECONDS` | Local duration cap; keep `180` (range 15–600). |

Twilio documents the dashboard credentials in its [Auth Token reference](https://www.twilio.com/docs/iam/api/authtoken). Use a funded/enabled account; [trial restrictions](https://www.twilio.com/docs/usage/trials/try-out-voice) prevent the final Relay/SIP architecture. No purchase or account change has been performed by this implementation.

The Photon values and current console labels come from its [official Beta Twilio cookbook](https://photon.codes/docs/beta/cookbooks/voice/twilio.md). If HTML access fails, the `.md` version contains the same official walkthrough. Profile edits can affect other numbers using that profile; use the intended test profile.

## Inbound SIP settings are a separate credential pair

In Twilio Console, open **Communications → Voice → SIP domains → Credential Lists**. The SIP username/password you choose there are used by **Photon inbound delivery** to authenticate to Twilio. Attach that list to the intended Twilio SIP Domain. In Photon Voice's **Inbound calls** section, configure `sips:YOURDOMAIN.sip.twilio.com`, TLS, and that Twilio credential-list login. These inbound credentials are configured between the providers; they are **not** `PHOTON_SIP_USERNAME/PASSWORD` in the Rust service.

Set the SIP Domain's Voice webhook to `POST PUBLIC_BASE_URL/voice/incoming`. Configure inbound status notifications to `/voice/status` where offered. The Rust Calls API sets outbound status URLs itself, including the reserved action ID. Complete the [provider preflight](provider-preflight.md) before spending the one smoke attempt.

For public ingress, an existing HTTPS deployment or a webhook-capable tunnel works. [ngrok's localhost guide](https://ngrok.com/docs/start) explains obtaining a public URL from its CLI. Keep both HTTP callbacks and WebSocket upgrades reachable without an interactive login challenge. Cloudflare [Quick Tunnels](https://developers.cloudflare.com/tunnel/get-started/quick-tunnels/) can expose local HTTP for testing, but their documented lack of SSE support makes them unsuitable for hosting this complete live console; a changing URL also requires updating `PUBLIC_BASE_URL` and callbacks. No tunnel runtime/account has been installed or provisioned here.

## Later conversation values

| Variable | Where its value comes from / current setting |
|---|---|
| `NVIDIA_API_KEY` | Sign in to the [NVIDIA API Catalog model page](https://build.nvidia.com/nvidia/nemotron-3.5-lightning-30b-a3b), use **Get API Key**, and save it locally. NVIDIA's [quickstart](https://docs.api.nvidia.com/nim/docs/api-quickstart) describes this flow. Access/credits/tool support still require a real probe. |
| `NEMOTRON_CHAT_URL` | Keep the provided NVIDIA chat-completions URL. This is an application constant, not a value to invent. |
| `NEMOTRON_MODEL` | Keep `nvidia/nemotron-3.5-lightning-30b-a3b`; do not silently substitute a model. |
| `VOICE_TRANSPORT` | After the first phone checkpoint, use `relay` after Twilio ConversationRelay onboarding/addendum. Use `gather` for the explicit setup-blocked fallback. |
| `VOICE_OUTPUT` | Keep `relay_native`. Current output is a preset assistant voice, not a clone. |
| `HANDOFF_CALLS_ENABLED` | Keep `false`: additional Sarah handoff calls are not yet authorized. |
| `FAMILY_CALLS_ENABLED` | Keep `false`: family is not enrolled or authorized. |
| `FAMILY_TEST_E164` / `CLINIC_TEST_E164` | Leave blank until the roles have designated consenting endpoints and their test scope is authorized. |

Optional `MAGPIE_ZERO_SHOT_URL`, `PHOTON_API_KEY`, `PHOTON_PROJECT_ID`, and `PHOTON_WEBHOOK_SECRET` are reserved for future clone/messaging adapters and are not used by the current build. Leave them blank. An approved Zeroshot deployment would supply its own HTTPS endpoint; Photon messaging would use a project API key/project ID and the configured webhook signing secret, independently of voice SIP credentials. None is needed for the core transport test.

## Check without making a call

```sh
bash scripts/cargo.sh run --bin preflight
```

The default reports variable **names** that are present/missing, not their values, and performs zero external requests. Presence does not prove correct flags, provider access or authorization. With the NVIDIA key ready, the explicit `--model-probe` flag makes bounded model-only requests; it never dials.

When ready, stop the separate replay process, start `bash scripts/cargo.sh run --bin firstring`, then follow the [two-phone checklist](demo-runbook.md). A calls manually; the console's authenticated Phone B action uses only `contact_id="sarah"`. Unknown create outcomes remain reserved and must not be retried blindly.
