#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ ! -x .tools/cloudflared ]]; then
  printf '%s\n' 'Missing project-local cloudflared; see docs/environment-setup.md.' >&2
  exit 1
fi
# Voice HTTP/WebSocket ingress only. Keep the SSE console on localhost.
# Do not add --allowed-mail: Twilio cannot pass an interactive login challenge.
exec .tools/cloudflared tunnel --no-autoupdate --protocol http2 --url http://127.0.0.1:8080
