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


if __name__ == "__main__":
    unittest.main()
