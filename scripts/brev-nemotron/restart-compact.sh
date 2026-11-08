#!/usr/bin/env bash
# One reversible, instance-local restart; no allocation, credentials or deadline changes.
set -euo pipefail
umask 077
task_root="${FIRSTRING_NEMOTRON_DIR:-/ephemeral/firstring-nemotron}"
systemctl is-active --quiet firstring-brev-guard.timer
[[ -f /opt/firstring-brev-guard/ready ]]
[[ -f "$task_root/runtime.env" ]]
[[ -f "$task_root/firstring_compact_tools.py" ]]
[[ "$(docker inspect firstring-nemotron --format '{{.Config.Image}}')" == 'vllm/vllm-openai@sha256:0a51ea5b4ae2dc5d81890e5173f54203d2a3ae0cfffe51b8fd2afd4391bfd967' ]]
if docker container inspect firstring-nemotron-before-compact >/dev/null 2>&1; then
  printf '%s\n' 'Prior compact restart already reserved; inspect before repeating' >&2
  exit 1
fi
mkdir -p "$task_root/runtime-cache"
for entry in 'vllm:/root/.cache/vllm' 'flashinfer:/root/.cache/flashinfer' 'triton:/root/.triton' 'nv:/root/.nv'; do
  cache_name="${entry%%:*}"
  cache_source="${entry#*:}"
  mkdir -p "$task_root/runtime-cache/$cache_name"
  docker cp "firstring-nemotron:$cache_source/." "$task_root/runtime-cache/$cache_name/"
done
docker stop --time 30 firstring-nemotron >/dev/null
docker rename firstring-nemotron firstring-nemotron-before-compact
FIRSTRING_COMPACT_TOOL_GRAMMAR=1 FIRSTRING_NEMOTRON_DIR="$task_root" bash /opt/firstring-brev-guard/serve.sh
