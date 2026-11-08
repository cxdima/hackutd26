# Optional Brev speech preflight

**Current authorization supersedes the historical deferral below:** up to the lower of $60 total or remaining existing Brev credits is authorized for actual GPU-hosted Nemotron serving, including storage and other charges. No extra credits or auto-recharge. Decision-model deployment now takes priority over optional speech; see [deployment evidence](brev-nemotron-deployment.md). The earlier $10 speech-only proposal below is historical, not the current spending authorization.

User screenshot confirms a **$60 Brev balance**; it does not establish a GPU quote, Magpie access or NGC entitlement. Speech provisioning has not occurred. The current [NVIDIA sponsor challenge](https://zeroday.hackutd.co/#tracks) requires Nemotron and multi-step tool/API workflows, but does not require Brev or self-hosting. Hosted Nemotron is consistent with that deployment-neutral wording; no private booth/portal amendment has been verified. Preserve credits for an optional speech experiment after the core phone gate, while retaining hosted inference as the known-working fallback.


Rechecked 2026-11-07 at 23:04 Central (2026-11-08 05:04 UTC). **The user requested Brev readiness while phone work continues. This check is read-only: no GPU provisioned, image layer pulled, account changed, purchase/top-up made, or adapter implemented.** The reported $60 Brev credits are a budget input, not verified model access or permission to spend. If later resumed, first experiment target: at most $10 including storage/other charges. Core phone work keeps native Relay speech.

## Step 1/5 — Establish access and the actual quote

| Gate | Observed status / next evidence |
|---|---|
| Brev account inventory and quote | Still unavailable: no Brev CLI/config/API key, connector or authenticated browser session accessible to these tools. The $60 screenshot establishes a displayed balance, not a current available GPU offer. No account-specific provider, region, compute rate or storage price was obtained. |
| Local tools | Docker client/daemon 29.7.2 available, daemon `aarch64 / linux`; no `nvidia-smi`. The `ngc` found on PATH is Angular's compiler, **not** NVIDIA NGC CLI. Local Docker is not a compatible GPU deployment. |
| Local secrets/config | `NVIDIA_API_KEY` is configured and already powers hosted inference. `BREV_API_KEY`, `BREV_TOKEN` and the separate `NGC_API_KEY` alias are absent. No NVIDIA NGC CLI/config or `nvcr.io` Docker auth entry was found; credential-store values were not read. The existing NVIDIA key succeeded in the registry probe below, so a replacement key is not required for manifest access. |
| NVIDIA entitlement | The registry issued the existing key a pull grant for this repository, and authenticated manifest access passed. This does not verify the model-weight download, startup/readiness or applicable self-hosted NVAIE/developer entitlement terms. |
| Accessible tag / support status | The pinned `magpie-tts-zeroshot:1.2.0` OCI index returned HTTP 200 with authenticated access and includes Linux amd64/arm64. This supersedes the earlier unauthenticated catalogue 402 observation. The public catalogue still displays an end-of-support notice despite the current tutorial/blueprint; support status remains unresolved. |
| Voice reference | A consenting performer's fictional-Sarah reference is not enrolled. No recording requested or uploaded in this audit. |

The bounded manifest probe made **three GET requests to `nvcr.io`**: anonymous manifest 401/`UNAUTHORIZED`, authenticated token exchange 200 with the requested repository pull grant, then authenticated manifest 200. It downloaded no layers or model weights and wrote no Docker login/global credential storage. The credential and issued token remained in process memory and were never printed. OCI index digest: `sha256:3e96aa19549d0a9ba7dc500f7aeb5eb22586df6640615ba65eaa2c24a14d341c`. Sanitized receipt: `.tools/magpie-registry-probe.json` (ignored). The [official blueprint](https://github.com/NVIDIA-AI-Blueprints/nemotron-voice-agent/blob/main/docker/docker-compose.magpie-zeroshot-tts.yaml) maps `NVIDIA_API_KEY` to `NGC_API_KEY`; [NGC setup](https://docs.nvidia.com/nim/speech/latest/get-started/ngc-access-setup.html) documents the registry credential convention. No raw authentication challenges, keys or bearer tokens were retained in the receipt.

The [Speech prerequisites](https://docs.nvidia.com/nim/speech/latest/get-started/prerequisites.html) and [gated model catalogue](https://catalog.ngc.nvidia.com/orgs/nim/teams/nvidia/containers/magpie-tts-zeroshot) establish separate entitlement/access requirements. The [approval link](https://developer.nvidia.com/riva-tts-zeroshot-models) currently requires NVIDIA login. Do not infer access from Brev credits, a build.nvidia.com inference key, or successful access to another model.

Remaining checks are a supported Zeroshot release and successful weight download/readiness under the applicable entitlement; authenticated access to the Brev organization with usable credits; exact single-GPU offer (provider/region, GPU, usable VRAM, CPU/RAM/disk, compute rate, storage rate while stopped, billing minimum and other fees); and where FirstRing's Rust process will run. Keep API keys local. Provisioning authorization should identify that concrete quote, a <=$10 total ceiling, maximum runtime, and cleanup of only the dedicated experiment instance.

After authenticated CLI access is independently established, these are read-only searches; they do **not** create instances:

```bash
brev search gpu --gpu-name L4 --min-vram 22 --min-capability 8.0 --stoppable --sort price --json
brev search gpu --gpu-name A10 --min-vram 22 --min-capability 8.0 --stoppable --sort price --json
brev list
```

Confirm exactly one GPU and inspect architecture/RAM/disk in the selected offer. Brev's [GPU catalogue](https://docs.nvidia.com/brev/reference/gpu-types) advertises L4/A10G with 22 GB usable memory. Its [search documentation](https://docs.nvidia.com/brev/cli/search-discovery) contains illustrative prices; those are **not** quotes for this account. Actual rate, region, availability and storage charges remain **pending**.

## Step 2/5 — Review the smallest suitable deployment

Proposed candidate: one dedicated L4 or A10G, x86_64 Ubuntu 22.04+, with no LLM/ASR sharing its GPU. Choose between actual offers after the quote. Target >=16 GB free VRAM, >=16 GB host RAM as project headroom, and enough quoted disk for image/model/cache downloads. Disk size is not yet established; inspect the accessible image layers and model download sizes before selecting it.

| Item | Verified recipe / constraint |
|---|---|
| Image pin | `nvcr.io/nim/nvidia/magpie-tts-zeroshot:1.2.0` — official blueprint pin with authenticated OCI index access verified; digest recorded above. No layers were pulled. |
| Selector | `name=magpie-tts-zeroshot,batch_size=8` |
| Profile memory | 13.06 GB GPU / 4.00 GB host; batch 32 instead needs 41.30 GB GPU / 7.08 GB host |
| GPU requirement | Compute capability >=8.0, at least 16 GB VRAM; avoid T4 even though it has 16 GB |
| Runtime | NVIDIA Container Toolkit; matrix requires NVIDIA driver >=535 and Docker >=23.0.1; prefer an actively supported driver rather than an old EOL branch |
| HTTP | Port 9000; only loopback-bound on the GPU host |

Sources: [official pinned Compose recipe](https://github.com/NVIDIA-AI-Blueprints/nemotron-voice-agent/blob/main/docker/docker-compose.magpie-zeroshot-tts.yaml), [current TTS matrix](https://docs.nvidia.com/nim/speech/latest/reference/support-matrix/tts.html). The general skill's x86-only wording is broader than today's matrix, which also lists DGX Spark; the proposed Brev path remains x86_64. No ARM workaround is needed for this experiment.

Cost worksheet, to fill from the actual quote:

```text
R = total compute dollars/hour for the selected single-GPU instance
S = storage + egress + minimum-billing/other charges for its complete lifetime
C = R * billed_hours + S
```

Proposed first run: **90 minutes maximum including provisioning/download/warm-up**, with $2 reserved for storage/other charges and $8 for compute. This requires `R <= $8 / 1.5 = $5.33/hour` and verified `S <= $2`; it is a ceiling calculation, **not a quoted GPU price**. If either cannot be bounded, do not launch. A cheaper verified offer reduces cost; the $60 balance does not expand this first-run ceiling. Review elapsed cost at 30 and 60 minutes. If readiness has not passed after its 30-minute warm-up window, stop and diagnose without keeping a GPU idle.

## Step 3/5 — Start and verify only after access and spend approval

The following commands are a reviewable remote-host runbook, **not commands executed during preparation**. No Brev create command is supplied until its exact offer is known. On the approved GPU host, first inspect:

```bash
uname -m
ldd --version
nvidia-smi --query-gpu=name,driver_version,memory.total,memory.free,compute_cap --format=csv
docker version
docker info --format '{{json .Runtimes}}'
```

Require the applicable license/access and a local secret-manager/private-file mapping of the existing `NVIDIA_API_KEY` to `NGC_API_KEY` on the approved host. The registry probe established that this existing key can read the pinned manifest; do not create another key merely because the alias is unset. Model-weight access is still a separate gate. NGC personal keys need **NGC Catalog** scope. Disable shell tracing; never put a literal key in a command, log, screenshot, or repository. The [official NGC setup](https://docs.nvidia.com/nim/speech/latest/get-started/ngc-access-setup.html) documents registry authentication; keep its username literal:

```bash
printf '%s' "$NGC_API_KEY" | docker login nvcr.io --username '$oauthtoken' --password-stdin
export FIRSTRING_NIM_IMAGE=nvcr.io/nim/nvidia/magpie-tts-zeroshot:1.2.0
docker pull "$FIRSTRING_NIM_IMAGE"
docker image inspect "$FIRSTRING_NIM_IMAGE" --format '{{json .RepoDigests}}'
```

A successful image pull does not prove model-weight permission. Record the pulled digest without credentials, and still require successful model download/readiness. If NVIDIA confirms a different supported tag, update this reviewed pin first; do not silently use `latest`. For a dedicated host cache:

```bash
export FIRSTRING_NIM_CACHE="$HOME/firstring-nim-cache"
mkdir -p "$FIRSTRING_NIM_CACHE"
sudo chown 1000:1000 "$FIRSTRING_NIM_CACHE"
docker run -d --rm --name firstring-magpie-test --restart=no \
  --runtime=nvidia --gpus '"device=0"' --shm-size=8GB \
  -e NGC_API_KEY -e NIM_TAGS_SELECTOR=name=magpie-tts-zeroshot,batch_size=8 \
  -e NIM_HTTP_API_PORT=9000 -e NIM_GRPC_API_PORT=50051 \
  -p 127.0.0.1:9000:9000 \
  -v "$FIRSTRING_NIM_CACHE:/opt/nim/.cache" "$FIRSTRING_NIM_IMAGE"
curl --fail --silent --show-error --max-time 5 http://127.0.0.1:9000/v1/health/ready
curl --fail --silent --show-error --max-time 5 http://127.0.0.1:9000/v1/audio/list_voices
```

The [official tutorial](https://docs.nvidia.com/nim/speech/latest/get-started/tutorials/tts.html) allows up to 30 minutes for the first model download/build. Recheck readiness every 30 seconds within that bound; a running container alone is insufficient. Require `status: ready`, the Zeroshot voice in discovery, then a fresh synthesis. Do not paste unrestricted container logs; inspect locally for credential-bearing URLs before sharing an error.

Use one approved 3–10 second, 16-bit mono WAV reference at >=22.05 kHz, ideally about five seconds of clear speech. A WebM recording needs decoding/resampling, not renaming. Synthesize text absent from the reference:

```bash
umask 077
curl --fail --silent --show-error --max-time 30 \
  http://127.0.0.1:9000/v1/audio/synthesize \
  -F language=en-US \
  -F 'text=I am the disclosed automated assistant for this fictional demonstration.' \
  -F voice=Magpie-ZeroShot-Multilingual \
  -F audio_prompt=@consented-reference.wav \
  --output fresh-probe.wav
file fresh-probe.wav
```

Confirm playable WAV metadata/duration, listen to the newly generated sentence, and record measured synthesis duration plus the performer's assessment. The [voice-cloning guide](https://docs.nvidia.com/nim/speech/latest/tts/voice-cloning.html) says current Magpie ignores the optional reference transcript. Built-in `.Female`/`.Male` output without the reference is preset TTS, not a clone. Only after this gate passes should a Rust adapter be implemented.

## Step 4/5 — Keep NIM private and publish only expiring output

Brev console tunnel URLs require browser authentication and redirects. They are unsuitable as Rust or Twilio media endpoints. For FirstRing running on the developer machine, use the documented SSH transport:

```bash
brev port-forward firstring-magpie-test --port 19000:9000
curl --fail --silent --show-error --max-time 5 http://127.0.0.1:19000/v1/health/ready
```

The name above must be the actual approved **Brev instance name**, not merely the Docker container name. Verify forwarding listens only on local loopback. A future Rust adapter can use the narrowly allowlisted `http://127.0.0.1:19000/v1/audio/synthesize` because the remote hop is authenticated/encrypted SSH. If Rust runs elsewhere, establish the tunnel on that server, or use a reviewed private network / HTTPS reverse proxy with service authentication and request limits. Never expose unauthenticated NIM ports publicly or follow a browser-login redirect. [Brev connectivity](https://docs.nvidia.com/brev/cli/connectivity)

Planned FirstRing integration, not yet implemented:

- Authenticated voice-reference enrollment binds consent, household and expiry. Validate decoded WAV/size/duration; keep reference private; expire after 1, 800 seconds and delete on reset or withdrawal.
- Rust submits only validated assistant text and the enrolled reference through the private NIM connection. Bound concurrency, response size and timeout; cancel/discard obsolete generation/epoch results. Keep the core six-second turn budget; slow synthesis falls back to native speech before either output is sent.
- Store validated generated WAV under a cryptographically random opaque token on FirstRing's own configured HTTPS `/audio/<token>.wav` route. Bind token to session, intended leg, generation/epoch and 300-second expiry. Recheck expiry and revocation per request; return no body after invalidation. Permit bounded provider fetch/retry while valid. Use `Content-Type: audio/wav` and `Cache-Control: private, no-store`; omit audio tokens and reference bytes from logs/UI/events.
- Send Relay `play` with that FirstRing URL, `loop:1`, `interruptible:true`, `preemptible:true`. Never send both `text` and `play` for the same reply. Keep private summary media on B; do not expose the reference to Twilio. An accepted message means `audio.requested`; an actual phone playback remains a separate gate.

These are the master guide's TTL defaults and [Relay audio contract](https://www.twilio.com/docs/voice/conversationrelay/websocket-messages), not provider retention guarantees. Current application speech remains native until synthesis, expiring delivery and real playback pass.

## Step 5/5 — Stop billing and remove experiment data

At the approved time/cost cutoff, end only this experiment's synthesis process and stop its Brev instance. Container shutdown alone does not stop GPU billing:

```bash
docker stop firstring-magpie-test
brev stop firstring-magpie-test
brev list
```

The first command runs on the GPU host; Brev commands run on the authenticated operator machine. Verify the actual instance shows **Stopped**. Brev states that stopped compute stops accruing compute charges, but retained storage can continue charging. Under the separately approved cleanup scope, delete the dedicated experiment instance and confirm its storage/snapshots are gone; never use `--all`. [Brev billing](https://docs.nvidia.com/brev/guides/console-reference), [instance lifecycle](https://docs.nvidia.com/brev/cli/instance-management)

Delete consented references/generated samples from the experiment workspace and FirstRing cache; keep only non-sensitive measurements and the image digest. Remove private secret files/session credentials from the dedicated machine. Retain model cache only if its storage cost and next use are explicitly approved. Review actual credits/usage after deletion; record compute and storage separately. Final capability stays **pending** until account access, readiness, fresh reference-conditioned WAV, and real phone playback have evidence.
