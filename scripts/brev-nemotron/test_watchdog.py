import importlib.util
import pathlib
import unittest
import contextlib
import datetime
import io
import json
import tempfile
import urllib.error
from unittest import mock

spec = importlib.util.spec_from_file_location("watchdog", pathlib.Path(__file__).with_name("watchdog.py"))
watchdog = importlib.util.module_from_spec(spec)
spec.loader.exec_module(watchdog)
remote_spec = importlib.util.spec_from_file_location("remote_guard", pathlib.Path(__file__).with_name("remote_guard.py"))
remote_guard = importlib.util.module_from_spec(remote_spec)
remote_spec.loader.exec_module(remote_guard)


class ExpiryTests(unittest.TestCase):
    def receipt(self):
        return {"name": "firstring-nemotron-20261108-000000", "created_at": 1000,
                "deadline": 6400, "warmup_deadline": 2800, "ready": False,
                "max_spend_usd": 10, "hourly_usd": 3, "balance_usd": 60}

    def test_failed_warmup_expires_at_thirty_minutes(self):
        self.assertEqual(watchdog.expiry(self.receipt()), 2800)

    def test_ready_cannot_extend_the_ninety_minute_hard_limit(self):
        r = self.receipt(); r["ready"] = True
        self.assertEqual(watchdog.expiry(r), 6400)
        r["deadline"] += 1
        with self.assertRaises(ValueError): watchdog.expiry(r)

    def test_other_names_and_over_budget_are_rejected(self):
        r = self.receipt(); r["name"] = "user-existing-instance"
        with self.assertRaises(ValueError): watchdog.expiry(r)
        r = self.receipt(); r["hourly_usd"] = 8
        with self.assertRaises(ValueError): watchdog.expiry(r)

    def test_bound_identity_survives_rename_and_ignores_reused_name(self):
        instances = [{"id": "owned-id", "name": "renamed"},
                     {"id": "different-id", "name": self.receipt()["name"]}]
        self.assertEqual(watchdog.select_instance(instances, "owned-id", self.receipt()["name"])["id"], "owned-id")

    def test_corrupt_or_extended_receipt_keeps_original_early_deadline(self):
        initial = self.receipt()
        self.assertEqual(watchdog.safe_deadline(initial, None), 2800)
        modified = dict(initial, deadline=999999, ready=True)
        self.assertEqual(watchdog.safe_deadline(initial, modified), 2800)
        self.assertEqual(watchdog.safe_deadline(initial, dict(initial, ready=True)), 6400)

    def test_cli_setup_refusal_uses_exact_id_provider_delete(self):
        response = mock.MagicMock()
        response.__enter__.return_value.status = 202
        with mock.patch.object(watchdog.subprocess, "run", return_value=mock.Mock(returncode=1)), \
             mock.patch.object(watchdog.urllib.request, "urlopen", return_value=response) as request, \
             contextlib.redirect_stdout(io.StringIO()):
            watchdog.delete_instance(pathlib.Path("mock-cli"), "owned-id", "fake-test-key")
        req = request.call_args.args[0]
        self.assertEqual(req.get_method(), "DELETE")
        self.assertEqual(req.full_url, watchdog.BASE + "/api/workspaces/owned-id")

    def test_remote_timer_keeps_warmup_and_hard_deadlines(self):
        r = dict(self.receipt(), instance_id="owned-id")
        self.assertFalse(remote_guard.cleanup_due(r, 2799, False))
        self.assertTrue(remote_guard.cleanup_due(r, 2800, False))
        self.assertFalse(remote_guard.cleanup_due(r, 2800, True))
        self.assertTrue(remote_guard.cleanup_due(r, 6400, True))

    def test_local_http_failure_logs_only_safe_diagnostic_fields(self):
        secret = "do-not-log-test-key"
        error = urllib.error.HTTPError("https://example.test/" + secret, 403,
                                       secret, {"Authorization": secret}, io.BytesIO(secret.encode()))
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            watchdog.log_event("delete_provider", error)
        event = json.loads(output.getvalue())
        self.assertEqual(event["operation"], "delete_provider")
        self.assertEqual(event["error_class"], "HTTPError")
        self.assertEqual(event["http_status"], 403)
        self.assertEqual(datetime.datetime.fromisoformat(event["observed_at_utc"]).utcoffset(), datetime.timedelta(0))
        self.assertNotIn(secret, output.getvalue())
        self.assertEqual(error.fp.tell(), 0)

    def test_remote_http_failure_logs_operation_status_time_without_secret(self):
        secret = "do-not-log-remote-key"
        error = urllib.error.HTTPError("https://example.test/" + secret, 409,
                                       secret, {"Authorization": secret}, io.BytesIO(secret.encode()))
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            config = dict(self.receipt(), instance_id="owned-id", api_key=secret)
            (root / "config.private.json").write_text(json.dumps(config))
            output = io.StringIO()
            with mock.patch.object(remote_guard, "ROOT", root), \
                 mock.patch.object(remote_guard.urllib.request, "urlopen", side_effect=error), \
                 contextlib.redirect_stdout(output), self.assertRaises(SystemExit):
                remote_guard.main()
        event = json.loads(output.getvalue())
        self.assertEqual(event["operation"], "delete_provider")
        self.assertEqual(event["error_class"], "HTTPError")
        self.assertEqual(event["http_status"], 409)
        self.assertEqual(datetime.datetime.fromisoformat(event["observed_at_utc"]).utcoffset(), datetime.timedelta(0))
        self.assertNotIn(secret, output.getvalue())
        self.assertEqual(error.fp.tell(), 0)

    def test_remote_config_read_failure_does_not_print_exception_message(self):
        output = io.StringIO()
        with mock.patch.object(pathlib.Path, "read_text", side_effect=PermissionError("private-key-value")), \
             contextlib.redirect_stdout(output), self.assertRaises(SystemExit):
            remote_guard.main()
        event = json.loads(output.getvalue())
        self.assertEqual(event["operation"], "read_configuration")
        self.assertEqual(event["error_class"], "PermissionError")
        self.assertNotIn("private-key-value", output.getvalue())

    def test_local_read_failure_identifies_operation_without_provider_details(self):
        secret = "do-not-log-read-key"
        error = urllib.error.HTTPError("https://example.test/" + secret, 401,
                                       secret, {"Authorization": secret}, io.BytesIO(secret.encode()))
        with tempfile.TemporaryDirectory() as directory:
            receipt = pathlib.Path(directory) / "experiment.json"
            receipt.write_text(json.dumps(self.receipt()))
            output = io.StringIO()
            with mock.patch.dict(watchdog.os.environ, {"BREV_API_KEY": secret, "BREV_ORG_ID": "test-org"}), \
                 mock.patch.object(watchdog.urllib.request, "urlopen", side_effect=error), \
                 mock.patch.object(watchdog.time, "sleep", side_effect=StopIteration), \
                 contextlib.redirect_stdout(output), self.assertRaises(StopIteration):
                watchdog.run(receipt)
        event = json.loads(output.getvalue())
        self.assertEqual(event["operation"], "read_instances")
        self.assertEqual(event["http_status"], 401)
        self.assertNotIn(secret, output.getvalue())
        self.assertEqual(error.fp.tell(), 0)

    def test_local_cli_and_provider_delete_failures_do_not_leak_output(self):
        secret = "do-not-log-delete-key"
        error = urllib.error.HTTPError("https://example.test/" + secret, 403,
                                       secret, {"Authorization": secret}, io.BytesIO(secret.encode()))
        output = io.StringIO()
        with mock.patch.object(watchdog.subprocess, "run",
                               return_value=mock.Mock(returncode=1, stdout=secret, stderr=secret)), \
             mock.patch.object(watchdog.urllib.request, "urlopen", side_effect=error), \
             contextlib.redirect_stdout(output):
            try:
                watchdog.delete_instance(pathlib.Path("mock-cli"), "owned-id", secret)
            except OSError as failure:
                watchdog.log_event("unknown", failure)
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual([event["operation"] for event in events], ["delete_cli", "delete_provider"])
        self.assertEqual(events[1]["http_status"], 403)
        self.assertNotIn(secret, output.getvalue())
        self.assertEqual(error.fp.tell(), 0)

    def test_bound_deadline_deletes_even_when_org_listing_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            receipt = pathlib.Path(directory) / "experiment.json"
            receipt.write_text(json.dumps(self.receipt()))
            receipt.with_suffix(".identity.private.json").write_text(json.dumps({
                "name": self.receipt()["name"], "instance_id": "owned-id"}))
            with mock.patch.dict(watchdog.os.environ, {"BREV_API_KEY": "test-key", "BREV_ORG_ID": "test-org"}), \
                 mock.patch.object(watchdog.time, "time", return_value=3000), \
                 mock.patch.object(watchdog.time, "sleep", side_effect=StopIteration), \
                 mock.patch.object(watchdog.urllib.request, "urlopen", side_effect=urllib.error.URLError("offline")), \
                 mock.patch.object(watchdog, "delete_instance", return_value={"cleanup_requested": True}) as delete, \
                 contextlib.redirect_stdout(io.StringIO()), self.assertRaises(StopIteration):
                watchdog.run(receipt)
            delete.assert_called_once_with(mock.ANY, "owned-id", "test-key")

    def test_missing_org_entry_cannot_verify_existing_exact_id_deleted(self):
        with tempfile.TemporaryDirectory() as directory:
            receipt = pathlib.Path(directory) / "experiment.json"
            receipt.write_text(json.dumps(self.receipt()))
            identity_path = receipt.with_suffix(".identity.private.json")
            identity_path.write_text(json.dumps({"name": self.receipt()["name"], "instance_id": "owned-id"}))
            def response(request, **_):
                content = [] if "/organizations/" in request.full_url else {"id": "owned-id", "status": "RUNNING"}
                handle = mock.MagicMock()
                handle.__enter__.return_value = io.StringIO(json.dumps(content))
                return handle
            with mock.patch.dict(watchdog.os.environ, {"BREV_API_KEY": "test-key", "BREV_ORG_ID": "test-org"}), \
                 mock.patch.object(watchdog.time, "time", return_value=1500), \
                 mock.patch.object(watchdog.time, "sleep", side_effect=StopIteration), \
                 mock.patch.object(watchdog.urllib.request, "urlopen", side_effect=response), \
                 contextlib.redirect_stdout(io.StringIO()), self.assertRaises(StopIteration):
                watchdog.run(receipt)
            self.assertNotIn("deletion_verified_at", json.loads(identity_path.read_text()))

    def test_missing_or_unexecutable_cli_falls_back_to_provider_delete(self):
        for error in [FileNotFoundError("missing"), PermissionError("unexecutable")]:
            with self.subTest(error=type(error).__name__):
                response = mock.MagicMock()
                response.__enter__.return_value.status = 202
                with mock.patch.object(watchdog.subprocess, "run", side_effect=error), \
                     mock.patch.object(watchdog.urllib.request, "urlopen", return_value=response) as request, \
                     contextlib.redirect_stdout(io.StringIO()):
                    result = watchdog.delete_instance(pathlib.Path("missing-cli"), "owned-id", "test-key")
                self.assertEqual(result["provider_delete_status"], 202)
                self.assertEqual(request.call_args.args[0].get_method(), "DELETE")

    def test_local_and_remote_reject_warmup_after_hard_deadline(self):
        r = dict(self.receipt(), deadline=2000, warmup_deadline=2800, instance_id="owned-id")
        with self.assertRaises(ValueError): watchdog.expiry(r)
        with self.assertRaises(ValueError): remote_guard.cleanup_due(r, 3000, False)

    def test_remote_rejects_missing_or_invalid_warmup_even_when_ready(self):
        for warmup in [None, 999, 2801, float("nan"), float("inf")]:
            r = dict(self.receipt(), instance_id="owned-id", warmup_deadline=warmup)
            with self.subTest(warmup=warmup), self.assertRaises(ValueError):
                remote_guard.cleanup_due(r, 7000, True)

    def test_exact_resource_404_is_required_for_absence_confirmation(self):
        for code in [401, 403, 404, 429, 500]:
            error = urllib.error.HTTPError("https://example.test/private", code,
                                           "private-message", {}, io.BytesIO(b"private-body"))
            with self.subTest(code=code), mock.patch.object(watchdog.urllib.request, "urlopen", side_effect=error):
                if code == 404:
                    self.assertFalse(watchdog.exact_instance_exists("owned-id", "test-key"))
                else:
                    with self.assertRaises(urllib.error.HTTPError):
                        watchdog.exact_instance_exists("owned-id", "test-key")
            self.assertEqual(error.fp.tell(), 0)

    def test_non_boolean_mutable_readiness_keeps_original_deadline(self):
        self.assertEqual(watchdog.safe_deadline(self.receipt(), dict(self.receipt(), ready="true")), 2800)

    def test_exact_404_records_verified_deletion_without_org_listing(self):
        with tempfile.TemporaryDirectory() as directory:
            receipt = pathlib.Path(directory) / "experiment.json"
            receipt.write_text(json.dumps(self.receipt()))
            identity_path = receipt.with_suffix(".identity.private.json")
            identity_path.write_text(json.dumps({"name": self.receipt()["name"], "instance_id": "owned-id"}))
            error = urllib.error.HTTPError("https://example.test/private", 404, "private", {}, io.BytesIO())
            with mock.patch.dict(watchdog.os.environ, {"BREV_API_KEY": "test-key", "BREV_ORG_ID": "test-org"}), \
                 mock.patch.object(watchdog.time, "time", return_value=1500), \
                 mock.patch.object(watchdog.urllib.request, "urlopen", side_effect=error) as request, \
                 contextlib.redirect_stdout(io.StringIO()):
                watchdog.run(receipt)
            self.assertEqual(json.loads(identity_path.read_text())["deletion_verified_at"], 1500)
            self.assertEqual(request.call_args.args[0].full_url, watchdog.BASE + "/api/workspaces/owned-id")

    def campaign_receipt(self):
        return dict(self.receipt(), campaign_authorized=True, campaign_started_at=1547,
                    campaign_work_deadline=8747, deadline=10547, prior_spend_usd=2.6,
                    total_budget_usd=60, balance_usd=57.4, ready=True, instance_id="owned-id")

    def test_explicit_campaign_allows_two_hours_plus_bounded_cleanup(self):
        receipt = self.campaign_receipt()
        self.assertEqual(watchdog.expiry(receipt), 10547)
        self.assertFalse(remote_guard.cleanup_due(receipt, 10546, True))
        self.assertTrue(remote_guard.cleanup_due(receipt, 10547, True))
        receipt["ready"] = False
        self.assertEqual(watchdog.expiry(receipt), 2800)
        self.assertTrue(remote_guard.cleanup_due(receipt, 2800, False))

    def test_campaign_extension_requires_explicit_authorization(self):
        for authorized in [False, "true", None]:
            receipt = dict(self.campaign_receipt(), campaign_authorized=authorized)
            with self.subTest(authorized=authorized), self.assertRaises(ValueError):
                watchdog.expiry(receipt)
            with self.subTest(remote=authorized), self.assertRaises(ValueError):
                remote_guard.cleanup_due(receipt, 10547, True)

    def test_campaign_cannot_extend_work_cleanup_or_spend_budget(self):
        for changes in [dict(campaign_work_deadline=8748), dict(deadline=10548),
                        dict(campaign_started_at=999), dict(balance_usd=9),
                        dict(prior_spend_usd=51), dict(total_budget_usd=61)]:
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                watchdog.expiry(dict(self.campaign_receipt(), **changes))
        for changes in [dict(campaign_work_deadline=8748), dict(deadline=10548),
                        dict(campaign_started_at=999)]:
            with self.subTest(remote=changes), self.assertRaises(ValueError):
                remote_guard.cleanup_due(dict(self.campaign_receipt(), **changes), 10547, True)

    def test_mutable_campaign_fields_cannot_extend_trusted_deadline(self):
        receipt = self.campaign_receipt()
        modified = dict(receipt, campaign_started_at=1647, campaign_work_deadline=8847, deadline=10647)
        self.assertEqual(watchdog.safe_deadline(receipt, modified), 10547)

    def budget_v2_receipt(self):
        return dict(self.receipt(), budget_policy_version=2, total_budget_usd=60,
                    prior_spend_usd=9.1, balance_usd=50.9, reserve_usd=0,
                    max_spend_usd=17, cleanup_allowance_usd=2, deadline=19000,
                    instance_id="owned-id")

    def test_explicit_budget_v2_fits_five_hours_and_preserves_both_deadlines(self):
        r = self.budget_v2_receipt()
        self.assertEqual(watchdog.expiry(r), 2800)
        self.assertTrue(remote_guard.cleanup_due(r, 2800, False))
        r["ready"] = True
        self.assertEqual(watchdog.expiry(r), 19000)
        self.assertFalse(remote_guard.cleanup_due(r, 18999, True))
        self.assertTrue(remote_guard.cleanup_due(r, 19000, True))

    def test_v2_explicit_reserve_does_not_silently_relax_legacy_policy(self):
        r = dict(self.budget_v2_receipt(), balance_usd=40, prior_spend_usd=20,
                 max_spend_usd=5, deadline=4600, ready=True)
        self.assertEqual(watchdog.expiry(r), 4600)
        with self.assertRaises(ValueError):
            watchdog.expiry(dict(self.receipt(), balance_usd=40, deadline=4600))

    def test_v2_spend_includes_cleanup_actual_balance_reserve_and_prior_cost(self):
        for changes in [dict(max_spend_usd=16.99), dict(balance_usd=16.99),
                        dict(prior_spend_usd=43.01), dict(reserve_usd=34),
                        dict(total_budget_usd=61), dict(cleanup_allowance_usd=1),
                        dict(cleanup_allowance_usd=3), dict(hourly_usd=0),
                        dict(max_spend_usd=0), dict(prior_spend_usd=60)]:
            for validator in [watchdog.expiry, lambda r: remote_guard.cleanup_due(r, 1000, False)]:
                with self.subTest(changes=changes, validator=validator), self.assertRaises(ValueError):
                    validator(dict(self.budget_v2_receipt(), **changes))

    def test_v2_eight_hour_boundary_and_thirty_minute_warmup_are_enforced(self):
        r = dict(self.budget_v2_receipt(), deadline=29800, max_spend_usd=26, ready=True)
        self.assertEqual(watchdog.expiry(r), 29800)
        self.assertTrue(remote_guard.cleanup_due(r, 29800, True))
        for changes in [dict(deadline=29801), dict(deadline=1000),
                        dict(warmup_deadline=2801), dict(warmup_deadline=1000),
                        dict(deadline=1500, warmup_deadline=2800)]:
            for validator in [watchdog.expiry, lambda x: remote_guard.cleanup_due(x, 30000, True)]:
                with self.subTest(changes=changes), self.assertRaises(ValueError):
                    validator(dict(r, **changes))

    def test_v2_rejects_invalid_numeric_fields_and_unknown_or_mixed_policy(self):
        fields = ["created_at", "deadline", "warmup_deadline", "hourly_usd",
                  "balance_usd", "max_spend_usd", "total_budget_usd",
                  "prior_spend_usd", "reserve_usd", "cleanup_allowance_usd"]
        for field in fields:
            for invalid in [True, False, -1, float("nan"), float("inf"), "2", None]:
                r = dict(self.budget_v2_receipt(), **{field: invalid})
                for validator in [watchdog.expiry, lambda x: remote_guard.cleanup_due(x, 1000, False)]:
                    with self.subTest(field=field, invalid=invalid), self.assertRaises(ValueError):
                        validator(r)
        for version in [None, True, False, 0, 1, 4, "2", "3", 2.0, 3.0]:
            for validator in [watchdog.expiry, lambda x: remote_guard.cleanup_due(x, 1000, False)]:
                with self.subTest(version=version), self.assertRaises(ValueError):
                    # A legacy-compatible short duration must not hide a missing
                    # policy-version check behind the older ninety-minute limit.
                    validator(dict(self.budget_v2_receipt(), budget_policy_version=version,
                                   deadline=4600, balance_usd=60, prior_spend_usd=0,
                                   max_spend_usd=5))
        for field in ["campaign_authorized", "campaign_started_at", "campaign_work_deadline"]:
            for validator in [watchdog.expiry, lambda x: remote_guard.cleanup_due(x, 1000, False)]:
                with self.subTest(mixed_field=field), self.assertRaises(ValueError):
                    validator(dict(self.budget_v2_receipt(), **{field: True}))

    def test_v2_missing_budget_fields_and_mutable_edits_keep_original_cutoff(self):
        original = self.budget_v2_receipt()
        for field in ["total_budget_usd", "prior_spend_usd", "reserve_usd", "cleanup_allowance_usd"]:
            changed = dict(original); del changed[field]
            for validator in [watchdog.expiry, lambda x: remote_guard.cleanup_due(x, 1000, False)]:
                with self.subTest(missing=field), self.assertRaises(ValueError):
                    validator(changed)
        for field in ["budget_policy_version", "total_budget_usd", "prior_spend_usd",
                      "reserve_usd", "cleanup_allowance_usd", "max_spend_usd", "balance_usd", "deadline"]:
            changed = dict(original, ready=True); changed[field] += 1
            self.assertEqual(watchdog.safe_deadline(original, changed), 2800)
        self.assertEqual(watchdog.safe_deadline(original, dict(original, ready=True)), 19000)

    def test_v3_presentation_day_allows_ten_hours_with_v2_spend_bounds(self):
        # Explicit operator authorization to keep inference up through the
        # presentations: a longer window, never a larger spend envelope.
        r = dict(self.budget_v2_receipt(), budget_policy_version=3,
                 deadline=37000, max_spend_usd=32, ready=True)
        self.assertEqual(watchdog.expiry(r), 37000)
        self.assertFalse(remote_guard.cleanup_due(r, 36999, True))
        self.assertTrue(remote_guard.cleanup_due(r, 37000, True))
        r["ready"] = False
        self.assertEqual(watchdog.expiry(r), 2800)
        for changes in [dict(deadline=37001), dict(max_spend_usd=31.99),
                        dict(balance_usd=31.99), dict(prior_spend_usd=28.01),
                        dict(total_budget_usd=61), dict(cleanup_allowance_usd=1),
                        dict(warmup_deadline=2801), dict(campaign_authorized=True)]:
            for validator in [watchdog.expiry, lambda x: remote_guard.cleanup_due(x, 1000, True)]:
                with self.subTest(changes=changes), self.assertRaises(ValueError):
                    validator(dict(r, ready=True, **changes))

    def test_v2_keeps_its_eight_hour_limit_after_v3_exists(self):
        r = dict(self.budget_v2_receipt(), deadline=29801, max_spend_usd=32, ready=True)
        for validator in [watchdog.expiry, lambda x: remote_guard.cleanup_due(x, 1000, True)]:
            with self.assertRaises(ValueError):
                validator(r)

    def test_running_receipt_cannot_switch_policy_version(self):
        original = dict(self.budget_v2_receipt(), deadline=29800, max_spend_usd=32, ready=True)
        extended = dict(original, budget_policy_version=3, deadline=37000)
        self.assertEqual(watchdog.safe_deadline(original, extended), 29800)


if __name__ == "__main__":
    unittest.main()
