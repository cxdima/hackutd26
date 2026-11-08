"""Bounded experiment cleanup, using the official CLI for deletion by exact ID.

Keep the control machine awake and online. This is an operational safeguard,
not a provider-enforced credit limit. Never prints credentials or account IDs.
"""
import json
import datetime
import math
import os
import pathlib
import re
import subprocess
import sys
import time
import urllib.request
import urllib.error

BASE = "https://brevapi.us-west-2-prod.control-plane.brev.dev"
IMMUTABLE = ["name", "created_at", "deadline", "warmup_deadline", "hourly_usd", "balance_usd", "max_spend_usd"]
IMMUTABLE += ["campaign_authorized", "campaign_started_at", "campaign_work_deadline", "prior_spend_usd", "total_budget_usd"]
IMMUTABLE += ["budget_policy_version", "reserve_usd", "cleanup_allowance_usd"]


def budget_v2_limit(receipt):
    """Explicit rehearsal policy; keep validation aligned with remote_guard.py."""
    for field in ["created_at", "deadline", "warmup_deadline", "hourly_usd",
                  "balance_usd", "max_spend_usd", "total_budget_usd",
                  "prior_spend_usd", "reserve_usd", "cleanup_allowance_usd"]:
        value = receipt.get(field)
        if (isinstance(value, bool) or not isinstance(value, (int, float))
                or not math.isfinite(value) or value < 0):
            raise ValueError("Invalid v2 budget or deadline field")
    if any(field in receipt for field in
           ["campaign_authorized", "campaign_started_at", "campaign_work_deadline"]):
        raise ValueError("Cannot combine campaign and rehearsal budget policies")
    duration = receipt["deadline"] - receipt["created_at"]
    if not math.isfinite(duration) or not 0 < duration <= 8 * 3600:
        raise ValueError("Rehearsal exceeds eight hours")
    if not 0 <= receipt["prior_spend_usd"] < receipt["total_budget_usd"] <= 60:
        raise ValueError("Rehearsal exceeds total authorization")
    if receipt["cleanup_allowance_usd"] != 2:
        raise ValueError("Rehearsal requires the fixed two-dollar cleanup allowance")
    return min(receipt["balance_usd"] - receipt["reserve_usd"],
               receipt["total_budget_usd"] - receipt["prior_spend_usd"],
               receipt["max_spend_usd"])


def log_event(operation, error=None, **fields):
    """Log metadata only: never stringify exceptions, responses, or requests."""
    event = {"observed_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
             "operation": getattr(error, "cleanup_operation", operation), **fields}
    if error is not None:
        event["error_class"] = type(error).__name__
        if isinstance(error, urllib.error.HTTPError) and 100 <= error.code <= 599:
            event["http_status"] = error.code
    print(json.dumps(event), flush=True)


def expiry(receipt):
    if not re.fullmatch(r"firstring-nemotron-\d{8}-\d{6}", receipt["name"]):
        raise ValueError("Not a dedicated experiment name")
    for field in ["created_at", "deadline", "warmup_deadline", "hourly_usd", "balance_usd", "max_spend_usd"]:
        value = receipt.get(field)
        if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
            raise ValueError("Invalid numeric receipt field")
    if not isinstance(receipt.get("ready", False), bool):
        raise ValueError("Invalid readiness state")
    if "budget_policy_version" in receipt and (
            type(receipt["budget_policy_version"]) is not int or receipt["budget_policy_version"] != 2):
        raise ValueError("Unsupported explicit budget policy version")
    duration = receipt["deadline"] - receipt["created_at"]
    if receipt.get("budget_policy_version") == 2:
        limit = budget_v2_limit(receipt)
    elif receipt.get("campaign_authorized") is True:
        for field in ["campaign_started_at", "campaign_work_deadline", "prior_spend_usd", "total_budget_usd"]:
            value = receipt.get(field)
            if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
                raise ValueError("Invalid campaign field")
        start, work = receipt["campaign_started_at"], receipt["campaign_work_deadline"]
        if not (0 <= start - receipt["created_at"] <= 3600 and 0 < work - start <= 7200
                and 0 < receipt["deadline"] - work <= 1800):
            raise ValueError("Campaign exceeds authorized work or cleanup window")
        if not 0 <= receipt["prior_spend_usd"] < receipt["total_budget_usd"] <= 60:
            raise ValueError("Campaign exceeds total authorization")
        limit = min(10, receipt["balance_usd"], receipt["total_budget_usd"] - receipt["prior_spend_usd"],
                    receipt["max_spend_usd"])
    else:
        if not 0 < duration <= 5400:
            raise ValueError("Experiment exceeds ninety minutes")
        limit = min(10, 60, receipt["balance_usd"] - 50, receipt["max_spend_usd"])
    if not receipt["created_at"] < receipt["warmup_deadline"] <= min(receipt["created_at"] + 1800, receipt["deadline"]):
        raise ValueError("Warmup exceeds thirty minutes")
    if receipt["hourly_usd"] <= 0 or receipt["hourly_usd"] * duration / 3600 + 2 > limit:
        raise ValueError("Quote lacks cleanup reserve or would consume protected credits")
    return receipt["deadline"] if receipt.get("ready") else receipt["warmup_deadline"]


def safe_deadline(initial, current):
    """Invalid mutable state cannot switch off the original cleanup timer."""
    fallback = expiry(initial)
    if not isinstance(current, dict) or any(current.get(k) != initial.get(k) for k in IMMUTABLE):
        return fallback
    try:
        return expiry(current)
    except (ValueError, KeyError, TypeError):
        return fallback


def select_instance(instances, bound_id, name):
    if not isinstance(instances, list):
        raise ValueError("Unexpected instance response")
    matches = [x for x in instances if (x.get("id") == bound_id if bound_id else x.get("name") == name)]
    if len(matches) > 1:
        raise ValueError("Ambiguous experiment identity")
    return matches[0] if matches else None


def delete_instance(binary, instance_id, key):
    if not re.fullmatch(r"[A-Za-z0-9_-]{1,128}", instance_id):
        raise ValueError("Invalid provider identity")
    try:
        result = subprocess.run([str(binary), "delete", instance_id], capture_output=True,
                                text=True, timeout=45, stdin=subprocess.DEVNULL)
        code = result.returncode
    except subprocess.TimeoutExpired as error:
        code = -1
        log_event("delete_cli", error, cli_exit=code)
    except OSError as error:
        code = -1
        log_event("delete_cli", error, cli_exit=code)
    if code == 0:
        return {"cleanup_requested": True, "cli_exit": 0}
    if code != -1:
        log_event("delete_cli", cli_exit=code, error_class="CommandExitFailure")
    # Official CLI store.DeleteWorkspace uses this exact provider DELETE route.
    # Its UI-derived setup-phase guard can reject before sending the request.
    # Ask the provider to cancel the one bound resource; never suppress a refusal.
    request = urllib.request.Request(BASE + "/api/workspaces/" + instance_id, method="DELETE",
                                     headers={"Authorization": "Bearer " + key})
    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            return {"cleanup_requested": True, "cli_exit": code, "provider_delete_status": response.status}
    except OSError as error:
        error.cleanup_operation = "delete_provider"
        raise


def exact_instance_exists(instance_id, key):
    """Only an authenticated exact-resource 404 proves this ID is absent."""
    if not re.fullmatch(r"[A-Za-z0-9_-]{1,128}", instance_id):
        raise ValueError("Invalid provider identity")
    request = urllib.request.Request(BASE + "/api/workspaces/" + instance_id,
                                     headers={"Authorization": "Bearer " + key})
    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            resource = json.load(response)
        if not isinstance(resource, dict) or resource.get("id") != instance_id:
            raise ValueError("Exact resource identity mismatch")
        return True
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return False
        raise


def run(path):
    repo = pathlib.Path(__file__).resolve().parents[2]
    binary = repo / ".tools/brev/brev-main"
    bound_id = None
    deletion_started = False
    identity_path = path.with_suffix(".identity.private.json")
    operation = "read_configuration"
    try:
        key = os.environ["BREV_API_KEY"]
        org = os.environ["BREV_ORG_ID"]
        initial = json.loads(path.read_text())
        operation = "validate_receipt"
        expiry(initial)
        operation = "read_identity"
        if identity_path.exists():
            identity = json.loads(identity_path.read_text())
            if identity.get("name") == initial["name"]:
                bound_id = identity.get("instance_id")
                if not isinstance(bound_id, str) or not re.fullmatch(r"[A-Za-z0-9_-]{1,128}", bound_id):
                    raise ValueError("Invalid bound instance identity")
    except (OSError, ValueError, KeyError) as error:
        log_event(operation, error, operator_review_needed=True, fatal=True)
        raise SystemExit(1)
    while True:
        try:
            operation = "read_receipt"
            try:
                receipt = json.loads(path.read_text())
            except (OSError, ValueError) as error:
                log_event(operation, error, original_deadline_retained=True)
                receipt = None
            operation = "validate_receipt"
            if receipt is not None and (not isinstance(receipt, dict) or
                    any(receipt.get(k) != initial.get(k) for k in IMMUTABLE)):
                log_event(operation, ValueError(), original_deadline_retained=True)
            deadline = safe_deadline(initial, receipt)
            # Listing is discovery only. Once bound, neither listing errors nor
            # a missing list entry can prevent exact-ID deletion or prove it.
            if bound_id is None:
                operation = "read_instances"
                request = urllib.request.Request(BASE + "/api/organizations/" + org + "/workspaces",
                                                 headers={"Authorization": "Bearer " + key})
                with urllib.request.urlopen(request, timeout=20) as response:
                    instances = json.load(response)
                operation = "select_instance"
                found = select_instance(instances, None, initial["name"])
                if found:
                    bound_id = found["id"]
                    if not isinstance(bound_id, str) or not re.fullmatch(r"[A-Za-z0-9_-]{1,128}", bound_id):
                        raise ValueError("Invalid provider identity")
                    operation = "write_identity"
                    identity_path.write_text(json.dumps({"instance_id": bound_id, "name": initial["name"]}))
                    identity_path.chmod(0o600)
                elif time.time() >= deadline:
                    log_event("read_instances", identity_unresolved_after_expiry=True,
                              operator_review_needed=True)
            if bound_id is not None:
                if time.time() >= deadline:
                    deletion_started = True
                    operation = "delete_provider"
                    try:
                        log_event(operation, **delete_instance(binary, bound_id, key))
                    except (OSError, ValueError, subprocess.TimeoutExpired) as error:
                        log_event(operation, error, cleanup_monitor_read_or_delete_failed=True,
                                  operator_review_needed=True)
                operation = "read_exact_instance"
                if not exact_instance_exists(bound_id, key):
                    operation = "write_identity"
                    identity_path.write_text(json.dumps({"instance_id": bound_id, "name": initial["name"],
                                                         "deletion_verified_at": int(time.time())}))
                    identity_path.chmod(0o600)
                    log_event("read_exact_instance", http_status=404, deletion_verified=True,
                              watchdog_initiated=deletion_started)
                    return
        except (OSError, ValueError, KeyError, subprocess.TimeoutExpired) as error:
            log_event(operation, error, cleanup_monitor_read_or_delete_failed=True,
                      operator_review_needed=True)
        time.sleep(10)


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("Usage: watchdog.py PRIVATE_EXPERIMENT_RECEIPT.json")
    run(pathlib.Path(sys.argv[1]))
