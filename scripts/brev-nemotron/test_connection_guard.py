import importlib.util
import json
import pathlib
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("connection_guard", pathlib.Path(__file__).with_name("connection_guard.py"))
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)


class ConnectionGuardTests(unittest.TestCase):
    def test_only_idle_healthy_remote_after_repeated_failures_can_restart(self):
        self.assertTrue(guard.restart_allowed(100, 200, 2, {"healthy": True, "running": 0}))
        for now, failures, remote in [(200, 2, {"healthy": True, "running": 0}),
                                      (100, 1, {"healthy": True, "running": 0}),
                                      (100, 2, {"healthy": True, "running": 1}),
                                      (100, 2, {"healthy": False, "running": 0}),
                                      (100, 2, {"healthy": True})]:
            with self.subTest(now=now, failures=failures, remote=remote):
                self.assertFalse(guard.restart_allowed(now, 200, failures, remote))

    def test_two_restart_reservations_survive_process_restart(self):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "restart.json"
            self.assertEqual(guard.reserve_restart(path, "owned-name", 200, 100), 1)
            self.assertEqual(guard.reserve_restart(path, "owned-name", 200, 101), 2)
            with self.assertRaises(ValueError): guard.reserve_restart(path, "owned-name", 200, 102)
            self.assertEqual(json.loads(path.read_text())["restarts_reserved"], 2)

    def test_expiry_or_corrupt_identity_cannot_reset_budget(self):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "restart.json"
            with self.assertRaises(ValueError): guard.reserve_restart(path, "owned-name", 200, 200)
            path.write_text(json.dumps({"name": "different", "deadline": 200, "restarts_reserved": 0}))
            with self.assertRaises(ValueError): guard.reserve_restart(path, "owned-name", 200, 100)


if __name__ == "__main__":
    unittest.main()
