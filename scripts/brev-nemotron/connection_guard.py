"""Supervise one private SSH forward; never restart a model or allocate compute."""
import json
import os
import pathlib
import shlex
import signal
import subprocess
import sys
import time
import urllib.request

from watchdog import expiry, log_event

FORWARD = "127.0.0.1:18080:127.0.0.1:8000"
REMOTE_CHECK = """import json,pathlib,urllib.request
env=dict(line.split('=',1) for line in pathlib.Path('/ephemeral/firstring-nemotron/runtime.env').read_text().splitlines() if '=' in line)
with urllib.request.urlopen('http://127.0.0.1:8000/health',timeout=3) as response: healthy=response.status==200
request=urllib.request.Request('http://127.0.0.1:8000/metrics',headers={'Authorization':'Bearer '+env['VLLM_API_KEY']})
with urllib.request.urlopen(request,timeout=3) as response: metrics=response.read().decode()
counts=[float(line.rsplit(' ',1)[1]) for line in metrics.splitlines() if line.startswith('vllm:num_requests_running{')]
print(json.dumps({'healthy':healthy,'running':sum(counts) if counts else None}))
"""


def restart_allowed(now, deadline, failures, remote):
    return now < deadline and failures >= 2 and remote.get("healthy") is True and remote.get("running") == 0


def reserve_restart(path, name, deadline, now):
    if now >= deadline:
        raise ValueError("Connection cleanup deadline reached")
    state = json.loads(path.read_text()) if path.exists() else {"name": name, "deadline": deadline, "restarts_reserved": 0}
    if state.get("name") != name or state.get("deadline") != deadline:
        raise ValueError("Connection reservation identity mismatch")
    count = state.get("restarts_reserved")
    if type(count) is not int or not 0 <= count < 2:
        raise ValueError("Connection restart allowance exhausted")
    state["restarts_reserved"] = count + 1
    temporary = path.with_suffix(".replace")
    temporary.write_text(json.dumps(state)); temporary.chmod(0o600)
    os.replace(temporary, path)
    return count + 1


def ssh_arguments(name):
    return ["ssh", "-T", "-oBatchMode=yes", "-oConnectTimeout=10", "-oControlPath=none", "-oControlMaster=no", name]


def stop_owned_forward(pid, name):
    result = subprocess.run(["ps", "-p", str(pid), "-o", "command="], capture_output=True, text=True, timeout=5)
    if result.returncode != 0:
        return
    command = shlex.split(result.stdout.strip())
    if not command or pathlib.Path(command[0]).name != "ssh" or FORWARD not in command or command[-1] != name:
        raise ValueError("Refusing to stop an unrelated process")
    if os.getpgid(pid) != pid:
        raise ValueError("SSH forward is not the owned process group")
    os.killpg(pid, signal.SIGTERM)


def run(directory):
    receipt = json.loads((directory / "experiment.private.json").read_text())
    expiry(receipt)
    name, deadline = receipt["name"], receipt["deadline"]
    processes_path = directory / "processes.private.json"
    budget_path = directory / "connection-restarts.private.json"
    identity_path = directory / "experiment.private.identity.private.json"
    failures = 0
    while time.time() < deadline:
        try:
            if identity_path.exists() and json.loads(identity_path.read_text()).get("deletion_verified_at"):
                break
            try:
                with urllib.request.urlopen("http://127.0.0.1:18080/health", timeout=3) as response:
                    healthy = response.status == 200
            except OSError:
                healthy = False
            failures = 0 if healthy else failures + 1
            if failures >= 2:
                result = subprocess.run(ssh_arguments(name) + ["sudo -n python3 -"], input=REMOTE_CHECK,
                                        text=True, capture_output=True, timeout=20)
                remote = json.loads(result.stdout) if result.returncode == 0 else {}
                if restart_allowed(time.time(), deadline, failures, remote):
                    reservation = reserve_restart(budget_path, name, deadline, time.time())
                    processes = json.loads(processes_path.read_text())
                    stop_owned_forward(processes["ssh_forward_pid"], name)
                    # Wait for the old socket to close; an ambiguous restart still consumes its reservation.
                    time.sleep(1)
                    if time.time() >= deadline:
                        break
                    with (directory / "ssh-forward-supervised.private.log").open("a") as log:
                        process = subprocess.Popen(ssh_arguments(name)[:-1] + ["-N", "-oExitOnForwardFailure=yes",
                                                   "-L", FORWARD, name], stdin=subprocess.DEVNULL,
                                                   stdout=log, stderr=log, start_new_session=True)
                    processes["ssh_forward_pid"] = process.pid
                    temporary = processes_path.with_suffix(".replace")
                    temporary.write_text(json.dumps(processes)); temporary.chmod(0o600)
                    os.replace(temporary, processes_path)
                    log_event("restart_owned_forward", reservation=reservation)
                    failures = 0
                else:
                    log_event("read_forward_health", local_healthy=False, remote_healthy=remote.get("healthy") is True,
                              remote_idle=remote.get("running") == 0)
        except (OSError, ValueError, KeyError, subprocess.TimeoutExpired) as error:
            log_event("supervise_owned_forward", error, operator_review_needed=True)
        time.sleep(min(15, max(0, deadline - time.time())))
    try:
        processes = json.loads(processes_path.read_text())
        stop_owned_forward(processes["ssh_forward_pid"], name)
        log_event("stop_owned_forward", cleanup_deadline_or_deletion=True)
    except (OSError, ValueError, KeyError, subprocess.TimeoutExpired) as error:
        log_event("stop_owned_forward", error, operator_review_needed=True)


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("Usage: connection_guard.py PRIVATE_EXPERIMENT_DIRECTORY")
    run(pathlib.Path(sys.argv[1]))
