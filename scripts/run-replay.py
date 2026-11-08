#!/usr/bin/env python3
"""Start an inert local checkpoint without changing the user's .env."""
from pathlib import Path
import os
import secrets
import subprocess
root = Path(__file__).resolve().parents[1]
os.chdir(root)
root.joinpath('.tools').mkdir(exist_ok=True)
token = root / '.tools/replay-admin-token'
if not token.exists():
    token.write_text(secrets.token_urlsafe(32))
    token.chmod(0o600)
env = os.environ.copy()
env.update(DEMO_EXECUTION_MODE='replay', DEMO_ADMIN_TOKEN=token.read_text().strip(), DATABASE_URL='sqlite:.tools/replay.db', PUBLIC_BASE_URL='http://127.0.0.1:8080', PORT='8080', VOICE_TRANSPORT='probe', TEST_CALLS_AUTHORIZED='false', HANDOFF_CALLS_ENABLED='false', FAMILY_CALLS_ENABLED='false')
print('Replay console: http://127.0.0.1:8080 — login token is in .tools/replay-admin-token', flush=True)
subprocess.run([str(root / 'target/debug/firstring')], env=env, check=True)
