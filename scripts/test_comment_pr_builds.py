import copy
import unittest

from comment_pr_builds import GitHub, MARKER, build_comment, publish

REPO = "owner/zbase"
SHA = "a" * 40
MERGE_SHA = "b" * 40
RUN = dict(id=7, event="pull_request", status="completed", workflow_id=3,
           run_attempt=1, repository=dict(full_name=REPO), head_sha=SHA,
           head_repository=dict(full_name="contributor/zbase"), head_branch="feature",
           conclusion="success", run_started_at="2026-11-11T10:00:00Z", pull_requests=[])
PR = dict(number=42, state="open", head=dict(sha=SHA, repo=dict(full_name="contributor/zbase")),
          base=dict(repo=dict(full_name=REPO)))


def artifact(suffix, artifact_id=10, **changes):
    result = dict(id=artifact_id, name=f"zBase-pr-42-{MERGE_SHA}-{suffix}", expired=False,
                  created_at="2026-11-11T11:00:00Z")
    result.update(changes)
    return result


class FakeAPI:
    def __init__(self):
        self.run = copy.deepcopy(RUN)
        self.pr = copy.deepcopy(PR)
        self.other_runs = []
        self.artifacts = [artifact("macos-arm64"), artifact("windows-x64", 11)]
        self.comments = []
        self.writes = []
        self.push_during_reads = False
        self.pr_reads = 0

    def request(self, path, method="GET", body=None):
        if method != "GET":
            self.writes.append((path, method, body))
            return {}
        if path.endswith("/actions/runs/7"):
            return self.run
        if path.endswith("/actions/workflows/ci.yml"):
            return {"id": 3}
        if path.endswith("/pulls/42"):
            self.pr_reads += 1
            pr = copy.deepcopy(self.pr)
            if self.push_during_reads and self.pr_reads > 1:
                pr["head"]["sha"] = "c" * 40
            return pr
        raise AssertionError(path)

    def pages(self, path, key=None):
        if path.endswith("/pulls"):
            return iter([{"number": 42}])
        if "/ci.yml/runs?" in path:
            return iter(self.other_runs)
        if path.endswith("/artifacts"):
            return iter(self.artifacts)
        if path.endswith("/comments"):
            return iter(self.comments)
        raise AssertionError(path)

    def publish(self, attempt=1):
        publish(self, REPO, 7, attempt, "https://github.com")


class BuildCommentTests(unittest.TestCase):
    def test_lists_only_runnable_builds_and_accepts_different_merge_sha(self):
        artifacts = [artifact("macos-arm64"), artifact("windows-x64", 11),
                     artifact("macos-cargo-timings", 12),
                     artifact("windows-x64", 13, name=f"zBase-pr-99-{MERGE_SHA}-windows-x64")]
        body = build_comment(REPO, RUN, 42, artifacts, "https://github.com")
        self.assertIn("[macOS (Apple Silicon) app]", body)
        self.assertIn("[Windows (x64) installer]", body)
        self.assertIn("/actions/runs/7/artifacts/10", body)
        self.assertIn("/actions/runs/7/artifacts/11", body)
        self.assertNotIn("/artifacts/12", body)
        self.assertNotIn("/artifacts/13", body)
        self.assertNotIn("timings", body)

    def test_failed_partial_run_does_not_advertise_expired_builds(self):
        run = dict(RUN, conclusion="failure", run_attempt=2)
        body = build_comment(REPO, run, 42, [artifact("macos-arm64"),
            artifact("windows-x64", 11, expired=True)], "https://github.com")
        self.assertIn("attempt 2, failure", body)
        self.assertIn("/artifacts/10", body)
        self.assertIn("Windows (x64) installer: unavailable", body)
        self.assertNotIn("/artifacts/11", body)

    def test_rerun_keeps_successful_sibling_job_build_and_uses_newest_replacement(self):
        run = dict(RUN, run_attempt=2)
        artifacts = [artifact("windows-x64", 11, created_at="2026-11-10T11:00:00Z"),
                     artifact("macos-arm64", 12), artifact("macos-arm64", 13)]
        body = build_comment(REPO, run, 42, artifacts, "https://github.com")
        self.assertIn("/artifacts/11", body)
        self.assertIn("/artifacts/13", body)
        self.assertNotIn("/artifacts/12", body)

    def test_creates_comment_for_fork_pr_with_empty_run_pull_requests(self):
        api = FakeAPI()
        api.publish()
        self.assertEqual(len(api.writes), 1)
        path, method, body = api.writes[0]
        self.assertEqual(path, f"/repos/{REPO}/issues/42/comments")
        self.assertEqual(method, "POST")
        self.assertTrue(body["body"].startswith(MARKER))

    def test_updates_one_existing_bot_comment_and_ignores_user_marker(self):
        api = FakeAPI()
        api.comments = [dict(id=8, body=MARKER, user=dict(login="someone", type="User")),
                        dict(id=9, body=MARKER, user=dict(login="github-actions[bot]", type="Bot"))]
        api.publish()
        self.assertEqual(api.writes[0][0], f"/repos/{REPO}/issues/comments/9")
        self.assertEqual(api.writes[0][1], "PATCH")
        api.comments[1]["body"] = api.writes[0][2]["body"]
        api.writes.clear()
        api.publish()
        self.assertEqual(api.writes, [])

    def test_user_marker_does_not_get_edited(self):
        api = FakeAPI()
        api.comments = [dict(id=8, body=MARKER, user=dict(login="someone", type="User"))]
        api.publish()
        self.assertEqual(api.writes[0][1], "POST")

    def test_closed_stale_or_wrong_repository_pr_is_skipped(self):
        for change in ["closed", "stale", "wrong_base", "wrong_head"]:
            with self.subTest(change=change):
                api = FakeAPI()
                if change == "closed": api.pr["state"] = "closed"
                if change == "stale": api.pr["head"]["sha"] = "c" * 40
                if change == "wrong_base": api.pr["base"]["repo"]["full_name"] = "another/repo"
                if change == "wrong_head": api.pr["head"]["repo"]["full_name"] = "another/fork"
                api.publish()
                self.assertEqual(api.writes, [])

    def test_late_old_run_or_superseded_attempt_does_not_replace_new_links(self):
        api = FakeAPI()
        api.other_runs = [dict(RUN, id=8)]
        api.publish()
        self.assertEqual(api.writes, [])
        api.other_runs = []
        api.publish(attempt=2)
        self.assertEqual(api.writes, [])

    def test_push_while_reading_comments_prevents_stale_post(self):
        api = FakeAPI()
        api.push_during_reads = True
        api.publish()
        self.assertEqual(api.writes, [])

    def test_other_workflows_and_non_pr_events_do_not_write(self):
        for changes in [dict(event="push"), dict(workflow_id=99), dict(status="in_progress")]:
            with self.subTest(changes=changes):
                api = FakeAPI()
                api.run.update(changes)
                api.publish()
                self.assertEqual(api.writes, [])

    def test_paginates_through_large_artifact_and_comment_lists(self):
        api = GitHub("unused", "https://api.github.com")
        calls = []
        def request(path):
            calls.append(path)
            return {"artifacts": list(range(100)) if path.endswith("&page=1") else [100]}
        api.request = request
        self.assertEqual(list(api.pages("/artifacts", "artifacts")), list(range(101)))
        self.assertEqual(calls, ["/artifacts?per_page=100&page=1", "/artifacts?per_page=100&page=2"])


if __name__ == "__main__":
    unittest.main()
