"""Retry transient ticket-service errors without bypassing validation."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("stapler_retry.sh")


class StaplerRetryTests(unittest.TestCase):
    def run_stapler(self, statuses, action="validate"):
        with tempfile.TemporaryDirectory(prefix="zbase-stapler-test-") as temp:
            root = Path(temp)
            env = dict(os.environ, PATH=f"{root}{os.pathsep}{os.environ['PATH']}",
                       TEST_ROOT=temp, TEST_STATUSES=json.dumps(statuses))
            mock = '''import json, os, pathlib, sys
root = pathlib.Path(os.environ["TEST_ROOT"])
log = root / "calls"
previous = log.read_text().splitlines() if log.exists() else []
with log.open("a") as stream:
    stream.write(json.dumps([pathlib.Path(sys.argv[0]).name, *sys.argv[1:]]) + "\\n")
if pathlib.Path(sys.argv[0]).name == "xcrun":
    attempt = sum(json.loads(line)[0] == "xcrun" for line in previous)
    statuses = json.loads(os.environ["TEST_STATUSES"])
    sys.exit(statuses[min(attempt, len(statuses) - 1)])
'''
            for name in ("xcrun", "sleep"):
                path = root / name
                path.write_text(f"#!{sys.executable}\n" + mock)
                path.chmod(0o755)
            result = subprocess.run(["bash", str(SCRIPT), action, "path with spaces/zBase.app"],
                                    env=env, text=True, capture_output=True)
            calls = [json.loads(line) for line in (root / "calls").read_text().splitlines()]
            return result, calls

    def test_success_does_not_retry(self):
        result, calls = self.run_stapler([0])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(calls, [["xcrun", "stapler", "validate", "path with spaces/zBase.app"]])

    def test_connection_loss_is_retried_and_validation_must_succeed(self):
        result, calls = self.run_stapler([68, 68, 0])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([call for call in calls if call[0] == "sleep"],
                         [["sleep", "10"], ["sleep", "20"]])
        self.assertEqual(sum(call[0] == "xcrun" for call in calls), 3)

    def test_persistent_service_failure_preserves_failure_status(self):
        result, calls = self.run_stapler([68])
        self.assertEqual(result.returncode, 68)
        self.assertEqual(sum(call[0] == "xcrun" for call in calls), 4)
        self.assertIn("failed after 4 attempts", result.stderr)
        self.assertEqual([call for call in calls if call[0] == "sleep"],
                         [["sleep", "10"], ["sleep", "20"], ["sleep", "40"]])

    def test_invalid_missing_and_revoked_tickets_fail_immediately(self):
        for status in (65, 66, 77):
            with self.subTest(status=status):
                result, calls = self.run_stapler([status])
                self.assertEqual(result.returncode, status)
                self.assertEqual(len(calls), 1)

    def test_stapling_retries_service_unavailable_and_temporary_failures(self):
        result, calls = self.run_stapler([69, 75, 0], action="staple")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(all(call[2] == "staple" for call in calls if call[0] == "xcrun"))


if __name__ == "__main__":
    unittest.main()
