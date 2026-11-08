# Configure FirstRing locally

Real phone tests remain **DEPENDENT ON SETUP**. `.env` was created from the example on Nov 7; its public origin points to the running temporary voice tunnel and `DEMO_ADMIN_TOKEN` was generated locally without being displayed. The rebuilt service reads this file, currently in replay mode. Provider credential fields are still empty at the latest check. The current authorization permits exactly one outbound smoke-test attempt to Sarah / Phone B. Family calling stays disabled.

## Remaining account actions

1. **Inbound Photon/Twilio:** Photon **Beta → project → Voice → Lines**: use the assigned voice-capable line as `PHOTON_NUMBER` and open its SIP profile. A shared iMessage assignment cannot carry voice; do not buy another messaging plan to solve this. Twilio **Console → Account dashboard** supplies `TWILIO_ACCOUNT_SID` and `TWILIO_AUTH_TOKEN`. Twilio **Communications → Voice → SIP domains → Credential Lists**: attach the inbound login to the SIP Domain, then enter that same pair in Photon profile **Inbound calls**, destination `sips:YOURDOMAIN.sip.twilio.com`, TLS, and save. These are provider-to-provider settings, not the Photon outbound env variables. Set the SIP Domain Voice URL to `POST ${PUBLIC_BASE_URL}/voice/incoming`. The local operator token is already generated.
2. **Outbound Photon/Sarah:** the assigned Photon profile's **Outbound calls** supplies `PHOTON_SIP_USERNAME`, `PHOTON_SIP_PASSWORD`, and **SIP connection** `PHOTON_SIP_HOST`/`PHOTON_SIP_PORT`. Verify **MD5 (legacy)**, TLS and compatible secure media; use the connection host, not the authentication realm. Set designated `SARAH_TEST_E164`. Keep `HANDOFF_CALLS_ENABLED=false`, `FAMILY_CALLS_ENABLED=false`; the one existing smoke authorization uses `TEST_CALLS_AUTHORIZED=true` only when ready. Use the app probe once, not an additional terminal call.
3. **Hosted inference:** NVIDIA **API Catalog → Nemotron 3.5 Lightning → Get API Key** supplies `NVIDIA_API_KEY`. Keep the supplied model/URL constants. The existing `--model-probe` runs independently of phones; the held-out evaluator runs only after that protocol gate passes.
4. **Optional Brev/NGC: deferred by the user; no action needed for the first call.** When resumed, Brev **organization → launch/create instance → GPU selection** supplies the actual GPU/region and compute/storage quote. NVIDIA **NGC → account menu → Setup → Generate Personal Key**, with **NGC Catalog** scope, supplies `NGC_API_KEY` for the dedicated GPU host. Entitlement, Zeroshot approval and supported tag must then be confirmed. `MAGPIE_ZERO_SHOT_URL` stays blank. See [Brev preparation](brev-speech-preflight.md); no GPU has been allocated.

Only these external setup facts remain. No additional messaging purchase, model migration or GPU is needed for the initial transport test. Configure values locally; never paste credentials in chat.

Create `.env` from `.env.example` without overwriting any existing file, then edit locally. The server reads `.env` automatically on startup; restart after changes. Git ignores `.env`, `.env.*`, databases and `.tools/`; only `.env.example` is intentionally tracked.

```sh
cp -n .env.example .env
chmod 600 .env
# Verify protection without displaying contents:
git check-ignore .env
```

`DEMO_ADMIN_TOKEN` is already generated in `.env` with mode600 file permissions; an authenticated local API read succeeded without displaying it. Enter its value locally in the console; do not send it in chat. Preserve it on subsequent setup edits. This token is not a Twilio or Photon credential. The separate replay helper uses `.tools/replay-admin-token`; the currently running normal service uses `.env`.

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

No existing ingress or tunnel account was configured. A project-local **cloudflared 2026.10.7** was downloaded from the [official release](https://github.com/cloudflare/cloudflared/releases/tag/2026.10.7), its Darwin arm64 archive checked against SHA-256 `3586ccf397e98e5f437e60e7c307ca6ac463e110580503af8bb8e6f9ebf831bd`, and a free accountless voice tunnel started. No global installation or account change was made.

```sh
bash scripts/voice-tunnel.sh
# Copy the printed HTTPS origin into PUBLIC_BASE_URL, restart Rust, and
# update the Twilio SIP Domain Voice webhook. Keep this process running.
```

This is the one prepared ingress option. Reuse the running process while it is available; its current URL is already in `.env`. Do not add `--allowed-mail` or an access-login policy: provider webhooks cannot use browser authentication. [Quick Tunnels](https://developers.cloudflare.com/tunnel/get-started/quick-tunnels/) have no uptime guarantee and change hostname on restart. Their lack of SSE is handled by using the operator console at `http://127.0.0.1:8080`, while the public URL carries only voice HTTP/WSS and later generated audio. [WebSocket support](https://developers.cloudflare.com/network/websockets/) is documented; a real signed Relay session still needs verification. Public health succeeded and unsigned POST/upgrade requests returned 403; those checks do not count as a phone or valid-signature gate.

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
