"""Instance-local exact-ID cleanup, invoked by a root-owned systemd timer."""
import json
import datetime
import math
import pathlib
import re
import time
import urllib.request
import urllib.error

BASE = "https://brevapi.us-west-2-prod.control-plane.brev.dev"
ROOT = pathlib.Path("/opt/firstring-brev-guard")


def log_event(operation, error=None, **fields):
    event = {"observed_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
             "operation": operation, **fields}
    if error is not None:
        event["error_class"] = type(error).__name__
        if isinstance(error, urllib.error.HTTPError) and 100 <= error.code <= 599:
            event["http_status"] = error.code
    print(json.dumps(event), flush=True)


def cleanup_due(config, now, ready):
    if not re.fullmatch(r"[A-Za-z0-9_-]{1,128}", config["instance_id"]):
        raise ValueError("Invalid instance identity")
    for field in ["created_at", "deadline", "warmup_deadline"]:
        value = config.get(field)
        if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
            raise ValueError("Invalid deadline field")
    if config.get("campaign_authorized") is True:
        for field in ["campaign_started_at", "campaign_work_deadline"]:
            value = config.get(field)
            if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
                raise ValueError("Invalid campaign deadline field")
        start, work = config["campaign_started_at"], config["campaign_work_deadline"]
        if not (0 <= start - config["created_at"] <= 3600 and 0 < work - start <= 7200
                and 0 < config["deadline"] - work <= 1800):
            raise ValueError("Invalid campaign expiry")
    elif not 0 < config["deadline"] - config["created_at"] <= 5400:
        raise ValueError("Invalid expiry")
    if not config["created_at"] < config["warmup_deadline"] <= min(config["created_at"] + 1800, config["deadline"]):
        raise ValueError("Invalid warmup expiry")
    deadline = config["deadline"] if ready else config["warmup_deadline"]
    return now >= deadline


def main():
    operation = "read_configuration"
    try:
        config = json.loads((ROOT / "config.private.json").read_text())
        operation = "validate_deadline"
        if not cleanup_due(config, time.time(), (ROOT / "ready").exists()):
            return
        operation = "delete_provider"
        request = urllib.request.Request(BASE + "/api/workspaces/" + config["instance_id"],
                                         method="DELETE", headers={"Authorization": "Bearer " + config["api_key"]})
        with urllib.request.urlopen(request, timeout=20) as response:
            log_event(operation, dedicated_instance_delete_status=response.status)
    except (OSError, ValueError, KeyError) as error:
        # systemd repeats at thirty seconds; no body/header/credential reaches logs.
        log_event(operation, error, dedicated_instance_cleanup_failed=True)
        raise SystemExit(1)


if __name__ == "__main__":
    main()
