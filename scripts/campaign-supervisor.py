#!/usr/bin/env python3
"""Check by default; --run permits <=2 dead-Rust recoveries until deadline.

Manifest: deadline (UTC ISO8601), database, state_file, deploy_lock, cwd,
env_file, public_base_url, local_base_url; rust:{pid,started,command,argv},
ingress:{pid,started,command}. Paths and argv[0] must be absolute. `started`
is normalized `ps -p PID -o lstart=`; `command` is exact `ps ... -o command=`.
Use a private .tools manifest/state. All deployments must exclusively create
deploy_lock (O_CREAT|O_EXCL) before mutation and remove it afterward. A stale
lock, unknown launch, changed PID/URL or tunnel failure requires an operator.
Never recreates tunnels, updates Twilio, kills services or creates GPU/calls.
"""
import argparse
from contextlib import closing
from datetime import datetime
import fcntl
import json
import os
from pathlib import Path
import shlex
import sqlite3
import subprocess
import time
import urllib.request

TERMINAL = {"completed", "busy", "failed", "no-answer", "canceled"}


def same_command_exists(command):
    p = subprocess.run(["/bin/ps", "-axo", "command="], capture_output=True, text=True, timeout=3)
    if p.returncode:
        raise ValueError("process list unavailable")
    return command in (line.strip() for line in p.stdout.splitlines())


def identity(pid):
    p = subprocess.run(["/bin/ps", "-p", str(int(pid)), "-o", "lstart=", "-o", "command="], capture_output=True, text=True, env={"LC_ALL": "C"}, timeout=3)
    if p.returncode == 1 and not p.stdout.strip():
        return None
    parts = p.stdout.strip().split(None, 5)
    if p.returncode or len(parts) != 6:
        raise ValueError("identity unavailable")
    return " ".join(parts[:5]), parts[5]


def active_calls(database):
    try:
        with closing(sqlite3.connect(Path(database).as_uri() + "?mode=ro", uri=True, timeout=1)) as db:
            for (raw,) in db.execute("SELECT data FROM sessions"):
                s = json.loads(raw)
                if s.get("execution_mode") == "replay":
                    continue
                if s.get("execution_mode") != "live" or s.get("inbound_ended") is not True or any(l.get("status") not in TERMINAL for l in s["legs"]):
                    return True
            return any(json.loads(raw).get("status") not in TERMINAL for (raw,) in db.execute("SELECT data FROM actions WHERE mode!='replay'"))
    except (OSError, sqlite3.Error, ValueError, KeyError, TypeError):
        return True


def save(path, state):
    tmp = Path(str(path) + ".tmp")
    with os.fdopen(os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600), "w") as f:
        json.dump(state, f)
        f.flush()
        os.fsync(f.fileno())
    os.replace(tmp, path)


def runtime_env(m):
    env = os.environ.copy()
    for line in Path(m["env_file"]).read_text().splitlines():
        parts = shlex.split(line, comments=True)
        if parts:
            key, value = " ".join(parts).removeprefix("export ").split("=", 1)
            env[key.strip()] = value
    database = env.get("DATABASE_URL", "").removeprefix("sqlite:").split("?", 1)[0]
    if not database or (Path(m["cwd"]) / database).resolve() != Path(m["database"]).resolve() or env.get("PUBLIC_BASE_URL") != m["public_base_url"]:
        raise ValueError("configuration changed")
    return env


def launch(m):
    p = subprocess.Popen(m["rust"]["argv"], cwd=m["cwd"], env=runtime_env(m), stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
    time.sleep(0.2)
    return p.pid


def probe(base):
    try:
        with urllib.request.urlopen(base.rstrip("/") + "/health/live", timeout=3) as r:
            return r.status == 200 and json.loads(r.read(512)).get("ok") is True
    except (OSError, ValueError):
        return False


def tick(m, run, now):
    if now >= datetime.fromisoformat(m["deadline"].replace("Z", "+00:00")).timestamp():
        return "deadline"
    lock = Path(m["deploy_lock"])
    try:
        fd = os.open(lock, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    except FileExistsError:
        return "deployment_locked"
    try:
        state_path = Path(m["state_file"])
        state = json.loads(state_path.read_text()) if state_path.exists() else {"attempts": 0}
        if not isinstance(state.get("attempts"), int) or not 0 <= state["attempts"] <= 2:
            return "invalid_state_operator"
        if state.get("pending"):
            return "launch_unknown_operator"
        rust = state.get("rust", m["rust"])
        found = identity(rust["pid"])
        if found and found != (rust["started"], rust["command"]):
            return "rust_identity_mismatch"
        ingress = m["ingress"]
        if identity(ingress["pid"]) != (ingress["started"], ingress["command"]):
            return "ingress_unavailable_operator"
        if found:
            for key in ("local_base_url", "public_base_url"):
                if key in m and not probe(m[key]):
                    return key + "_unhealthy_operator"
            return "healthy"
        if active_calls(m["database"]):
            return "active_or_unknown_call"
        if same_command_exists(m["rust"]["command"]):
            return "replacement_process_operator"
        if not run:
            return "rust_dead_dry_run"
        if state["attempts"] >= 2:
            return "restart_budget_exhausted"
        state.update(attempts=state["attempts"] + 1, pending=True)
        save(state_path, state)  # Reserve before spawning; unknown outcomes are never retried.
        try:
            pid = launch(m)
        except (OSError, ValueError):
            state["pending"] = False
            save(state_path, state)
            return "restart_failed"
        new = identity(pid)
        if not new or new[1] != m["rust"]["command"]:
            return "launch_unknown_operator"
        state.update(pending=False, rust={"pid": pid, "started": new[0], "command": new[1]})
        save(state_path, state)
        return "rust_restarted"
    finally:
        os.close(fd)
        lock.unlink()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("--run", action="store_true")
    args = parser.parse_args()
    caffeinate = None
    try:
        m = json.loads(args.manifest.read_text())
        for key in ("database", "state_file", "deploy_lock", "cwd", "env_file"):
            if not Path(m[key]).is_absolute():
                raise ValueError("absolute paths required")
        if not Path(m["rust"]["argv"][0]).is_absolute():
            raise ValueError("absolute binary required")
        runtime_env(m)
        seconds = int(datetime.fromisoformat(m["deadline"].replace("Z", "+00:00")).timestamp() - time.time())
        with open(m["state_file"] + ".supervisor.lock", "a") as singleton:
            fcntl.flock(singleton, fcntl.LOCK_EX | fcntl.LOCK_NB)
            if args.run and seconds > 0 and Path("/usr/bin/caffeinate").exists():
                caffeinate = subprocess.Popen(["/usr/bin/caffeinate", "-i", "-t", str(seconds)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            previous = None
            while True:
                result = tick(m, args.run, time.time())
                if result != previous:
                    print(json.dumps({"event": result, "at": int(time.time())}), flush=True)
                previous = result
                if not args.run or result == "deadline":
                    break
                time.sleep(min(10, max(0, seconds)))
    except Exception:
        print('{"event":"supervisor_error_operator"}', flush=True)
        return 1
    finally:
        if caffeinate:
            caffeinate.terminate()
            caffeinate.wait(timeout=3)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
