#!/usr/bin/env python3
"""Import reviewed evidence into the local ledger's metadata; never make requests.

Usage: python3 scripts/import-verification.py
Only the checked-in model protocol and manual inbound receipts are imported.
Relay evidence must be recorded separately after the real rehearsal, including
human audible confirmation; a socket connection alone is insufficient.
"""
import hashlib
import json
from pathlib import Path
import sqlite3

ROOT = Path(__file__).resolve().parents[1]
values = {}
for line in (ROOT / '.env').read_text().splitlines():
    if '=' in line and not line.lstrip().startswith('#'):
        key, value = line.split('=', 1)
        values[key.strip()] = value.strip().strip('"').strip("'")
url = values.get('DATABASE_URL', 'sqlite:firstring.db')
if not url.startswith('sqlite:') or '?' in url or url.endswith(':memory:'):
    raise SystemExit('An existing local SQLite file is required')
path = Path(url.removeprefix('sqlite:').removeprefix('//'))
if not path.is_absolute():
    path = ROOT / path
db = sqlite3.connect(path.resolve().as_uri() + '?mode=rw', uri=True)

def save(key, record):
    db.execute('INSERT INTO metadata(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value',
               ('verification.' + key, json.dumps(record)))
    print('Imported reviewed evidence:', key)

model = json.loads((ROOT / 'docs/evidence/nvidia-model-probe-2026-11-07.json').read_text())
samples = model.get('request_observations', [])
if (model.get('status') == 'model_probe_passed'
        and model.get('requested_model') == values.get('NEMOTRON_MODEL')
        and model.get('native_result_roundtrips', 0) >= 2
        and len(samples) == model.get('requests_submitted') and len(samples) >= 4
        and all(s.get('outcome') == 'success' and s.get('response_model') == model['requested_model'] for s in samples)):
    save('model_protocol', {'status':'passed', 'recorded_at':model['recorded_at'],
         'source':'hosted_model_probe', 'checks':dict.fromkeys(['authenticated','expected_model','native_tool_results'], True)})

inbound = json.loads((ROOT / 'docs/evidence/inbound-probe-2026-11-07.json').read_text())
prefix = inbound.get('provider_readback', {}).get('call_id_sha256_prefix', '')
bound = any(hashlib.sha256(sid.encode()).hexdigest()[:12] == prefix
            for (sid,) in db.execute('SELECT inbound_sid FROM sessions'))
events = inbound.get('application_events', [])
types = {e.get('type') for e in events if e.get('execution_mode') == 'live' and e.get('leg_role') == 'caller'}
if (bound and inbound.get('result') == 'PASS' and inbound.get('execution_mode') == 'live'
        and {'call.inbound.received','transport.keypad','call.ended'} <= types
        and inbound['provider_readback'].get('status') == 'completed'
        and inbound.get('audible_evidence', {}).get('greeting_heard') is True):
    save('inbound_phone', {'status':'passed', 'recorded_at':events[-1]['occurred_at'],
         'source':'manual_inbound_probe', 'checks':dict.fromkeys(['signed_inbound','caller_keypad','audible_greeting','terminal_reconciled'], True)})
db.commit()
db.close()
