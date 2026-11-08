"""Only the process boundary is faked; SQLite, locks and durable budgets are real."""
import importlib.util
from contextlib import closing
import json
from pathlib import Path
import sqlite3
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("supervisor", Path(__file__).with_name("campaign-supervisor.py"))
sup = importlib.util.module_from_spec(spec)
if spec.loader and Path(spec.origin).exists():
    spec.loader.exec_module(sup)


class SupervisorTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        p = Path(self.tmp.name)
        self.db = p / "calls.db"
        with closing(sqlite3.connect(self.db, isolation_level=None)) as db:
            db.executescript("CREATE TABLE sessions(data TEXT); CREATE TABLE actions(mode TEXT,data TEXT);")
        self.m = {"deadline": "2026-11-08T09:09:50Z", "database": str(self.db),
                  "state_file": str(p / "state.json"), "deploy_lock": str(p / "deploy.lock"),
                  "cwd": str(p), "env_file": str(p / "runtime.env"),
                  "rust": {"pid": 123, "command": "/trusted/firstring", "started": "old", "argv": ["/trusted/firstring"]},
                  "ingress": {"pid": 456, "command": "/trusted/cloudflared", "started": "old"}}
        (p / "runtime.env").write_text("PRIVATE_TOKEN='do not log this'\n")
        self.now = 1794124800  # 2026-11-08T08:00:00Z

    def tick(self, run=False, identity=None, launch=None):
        with patch.object(sup, "identity", side_effect=identity or (lambda pid: None if pid == 123 else ("old", "/trusted/cloudflared"))), patch.object(sup, "launch", side_effect=launch or (lambda *_: 999)), patch.object(sup, "same_command_exists", return_value=False):
            return sup.tick(self.m, run, self.now)

    def test_default_check_never_reserves_or_launches(self):
        self.assertEqual(self.tick(launch=lambda *_: self.fail("launched")), "rust_dead_dry_run")
        self.assertFalse(Path(self.m["state_file"]).exists())

    def test_live_call_and_unknown_action_block_restart(self):
        with closing(sqlite3.connect(self.db, isolation_level=None)) as db:
            db.execute("INSERT INTO sessions VALUES(?)", (json.dumps({"execution_mode": "live", "inbound_ended": False, "legs": []}),))
        self.assertEqual(self.tick(True), "active_or_unknown_call")
        with closing(sqlite3.connect(self.db, isolation_level=None)) as db:
            db.execute("DELETE FROM sessions")
            db.execute("INSERT INTO actions VALUES('live',?)", (json.dumps({"status": "unknown"}),))
        self.assertEqual(self.tick(True), "active_or_unknown_call")
        self.assertFalse(Path(self.m["state_file"]).exists())

    def test_terminal_calls_allow_two_durable_attempts_even_if_spawn_fails(self):
        with closing(sqlite3.connect(self.db, isolation_level=None)) as db:
            db.execute("INSERT INTO sessions VALUES(?)", (json.dumps({"execution_mode": "live", "inbound_ended": True, "legs": [{"status": "completed"}]}),))
        def failed(*_):
            raise OSError("secret must not appear")
        self.assertEqual(self.tick(True, launch=failed), "restart_failed")
        self.assertEqual(self.tick(True, launch=failed), "restart_failed")
        self.assertEqual(self.tick(True, launch=lambda *_: self.fail("third start")), "restart_budget_exhausted")
        self.assertEqual(json.loads(Path(self.m["state_file"]).read_text())["attempts"], 2)

    def test_pid_mismatch_and_dead_ingress_require_operator(self):
        self.assertEqual(self.tick(True, identity=lambda _: ("different", "/untrusted")), "rust_identity_mismatch")
        self.assertEqual(self.tick(True, identity=lambda _: None), "ingress_unavailable_operator")

    def test_deadline_deploy_lock_and_invalid_database_fail_closed(self):
        self.assertEqual(sup.tick(self.m, True, 1794128990), "deadline")
        lock = Path(self.m["deploy_lock"])
        lock.touch()
        self.assertEqual(self.tick(True), "deployment_locked")
        self.assertTrue(lock.exists())
        lock.unlink()
        self.db.unlink()
        self.assertEqual(self.tick(True), "active_or_unknown_call")

    def test_replay_rows_do_not_block_but_malformed_live_data_does(self):
        with closing(sqlite3.connect(self.db, isolation_level=None)) as db:
            db.execute("INSERT INTO sessions VALUES(?)", (json.dumps({"execution_mode": "replay", "inbound_ended": False}),))
        self.assertFalse(sup.active_calls(self.db))
        with closing(sqlite3.connect(self.db, isolation_level=None)) as db:
            db.execute("INSERT INTO sessions VALUES('{}')")
        self.assertTrue(sup.active_calls(self.db))

    def test_unconfirmed_launch_cannot_be_blindly_retried(self):
        Path(self.m["state_file"]).write_text(json.dumps({"attempts": 1, "pending": True}))
        self.assertEqual(self.tick(True, launch=lambda *_: self.fail("duplicate launch")), "launch_unknown_operator")

    def test_success_records_new_identity_before_followup_observation(self):
        identity = lambda pid: ("new", "/trusted/firstring") if pid == 999 else (None if pid == 123 else ("old", "/trusted/cloudflared"))
        self.assertEqual(self.tick(True, identity=identity), "rust_restarted")
        self.assertEqual(self.tick(True, identity=identity, launch=lambda *_: self.fail("duplicate launch")), "healthy")
        self.assertEqual(json.loads(Path(self.m["state_file"]).read_text())["rust"]["pid"], 999)

    def test_replaced_process_at_another_pid_cannot_be_duplicated(self):
        with patch.object(sup, "identity", side_effect=lambda pid: None if pid == 123 else ("old", "/trusted/cloudflared")), patch.object(sup, "same_command_exists", return_value=True), patch.object(sup, "launch", side_effect=AssertionError("duplicate launch")):
            self.assertEqual(sup.tick(self.m, True, self.now), "replacement_process_operator")


if __name__ == "__main__":
    unittest.main()
