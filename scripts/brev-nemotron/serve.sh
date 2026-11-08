#!/usr/bin/env bash
# Run only on the authorized, newly allocated H100 instance. No cloud allocation.
set -euo pipefail
umask 077
command -v docker >/dev/null
command -v nvidia-smi >/dev/null
command -v python3 >/dev/null

gpu_info="$(nvidia-smi --query-gpu=name,memory.total --format=csv,noheader,nounits)"
if [[ "$gpu_info" != *H100* ]] || [[ "$(printf '%s\n' "$gpu_info" | wc -l | tr -d ' ')" != 1 ]]; then
  printf '%s\n' 'This pinned recipe requires exactly one H100. Refusing another profile.' >&2
  exit 1
fi
gpu_memory="${gpu_info##*,}"
if (( ${gpu_memory// /} < 79000 )); then
  printf '%s\n' 'H100 memory is below the verified 80GB profile.' >&2
  exit 1
fi
if docker container inspect firstring-nemotron >/dev/null 2>&1; then
  printf '%s\n' 'Named container already exists; inspect it before any replacement.' >&2
  exit 1
fi

task_root="${FIRSTRING_NEMOTRON_DIR:-$HOME/firstring-nemotron}"
mkdir -p "$task_root/huggingface"
chmod 700 "$task_root"
if [[ ! -f "$task_root/runtime.env" ]]; then
  python3 - "$task_root/runtime.env" <<'PY'
import os, secrets, sys
fd = os.open(sys.argv[1], os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
with os.fdopen(fd, 'w') as out:
    out.write('VLLM_API_KEY=' + secrets.token_urlsafe(48) + '\n')
PY
fi

# The registry digest and model revision are immutable pins verified before deployment.
# Only this host's loopback is reachable; access from FirstRing uses authenticated SSH.
docker run --detach --name firstring-nemotron --restart=no \
  --gpus all --ipc=host --network=host \
  --env-file "$task_root/runtime.env" \
  --env VLLM_NO_USAGE_STATS=1 --env VLLM_DO_NOT_TRACK=1 \
  --mount "type=bind,source=$task_root/huggingface,target=/root/.cache/huggingface" \
  --log-driver json-file --log-opt max-size=10m --log-opt max-file=2 \
  vllm/vllm-openai@sha256:0a51ea5b4ae2dc5d81890e5173f54203d2a3ae0cfffe51b8fd2afd4391bfd967 \
  nvidia/NVIDIA-Nemotron-3.5-Lightning-30B-A3B-NVFP4 \
  --revision bee7596271d1495f6992ae224aefde4410e816b8 \
  --tokenizer-revision bee7596271d1495f6992ae224aefde4410e816b8 \
  --served-model-name nvidia/nemotron-3.5-lightning-30b-a3b \
  --host 127.0.0.1 --port 8000 \
  --max-model-len 8192 --max-num-seqs 4 --max-num-batched-tokens 8192 \
  --enable-prefix-caching --async-scheduling \
  --mamba-backend flashinfer --moe-backend humming --linear-backend humming \
  --mamba-ssu-algorithm horizontal --mamba-cache-mode align \
  --mamba-ssm-cache-dtype float16 --enable-mamba-cache-stochastic-rounding \
  --mamba-cache-philox-rounds 5 --reasoning-parser nemotron_v3 \
  --tool-call-parser qwen3_coder --enable-auto-tool-choice
printf '%s\n' 'Container requested. Readiness and native tool validation remain required.'
