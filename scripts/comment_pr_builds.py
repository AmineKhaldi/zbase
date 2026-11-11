"""Maintain one PR comment linking only the macOS app and Windows installer."""

import json
import os
from pathlib import Path
import re
from urllib.parse import urlencode
from urllib.request import Request, urlopen

MARKER = "<!-- zbase-pr-build-downloads -->"
PLATFORMS = (("macos-arm64", "macOS (Apple Silicon) app"),
             ("windows-x64", "Windows (x64) installer"))


class GitHub:
    def __init__(self, token, api_url):
        self.token = token
        self.api_url = api_url.rstrip("/")

    def request(self, path, method="GET", body=None):
        data = None if body is None else json.dumps(body).encode()
        request = Request(self.api_url + path, data=data, method=method, headers={
            "Authorization": f"Bearer {self.token}",
            "Accept": "application/vnd.github+json",
            "Content-Type": "application/json",
            "X-GitHub-Api-Version": "2022-11-28",
        })
        with urlopen(request, timeout=30) as response:
            return json.load(response)

    def pages(self, path, key=None):
        separator = "&" if "?" in path else "?"
        page = 1
        while True:
            result = self.request(f"{path}{separator}per_page=100&page={page}")
            items = result[key] if key else result
            yield from items
            if len(items) < 100:
                break
            page += 1


def build_comment(repository, run, pr_number, artifacts, server_url):
    """Artifact names contain the merge SHA, which differs from run.head_sha."""
    root = f"{server_url.rstrip('/')}/{repository}"
    run_url = f"{root}/actions/runs/{run['id']}"
    lines = [MARKER, "### Test builds", "",
             f"Commit `{run['head_sha'][:12]}` · [CI run]({run_url}) "
             f"(attempt {run['run_attempt']}, {run['conclusion']})", ""]
    for suffix, label in PLATFORMS:
        pattern = rf"zBase-pr-{pr_number}-[0-9a-f]{{40}}-{suffix}"
        matches = [artifact for artifact in artifacts
                   if re.fullmatch(pattern, artifact["name"])
                   and not artifact["expired"]]
        if matches:
            # Rerunning only failed jobs leaves successful sibling-job artifacts
            # from an earlier attempt. They still belong to this exact CI run.
            artifact = max(matches, key=lambda item: item["id"])
            url = f"{run_url}/artifacts/{artifact['id']}"
            lines.append(f"- [{label}]({url})")
        else:
            lines.append(f"- {label}: unavailable for this run.")
    lines += ["", "Downloads require GitHub sign-in and are retained for 7 days."]
    return "\n".join(lines)


def publish(api, repository, run_id, attempt, server_url):
    prefix = f"/repos/{repository}"
    run = api.request(f"{prefix}/actions/runs/{run_id}")
    workflow = api.request(f"{prefix}/actions/workflows/ci.yml")
    if (run["event"] != "pull_request" or run["status"] != "completed"
            or run["workflow_id"] != workflow["id"] or run["run_attempt"] != attempt
            or run["repository"]["full_name"] != repository):
        return
    head_repository = (run.get("head_repository") or {}).get("full_name")
    if not head_repository:
        return
    sha = run["head_sha"]
    if not re.fullmatch(r"[0-9a-f]{40}", sha):
        raise ValueError("Invalid workflow head SHA")

    # workflow_run.pull_requests can be empty, including on fork runs.
    pulls = list(api.pages(f"{prefix}/commits/{sha}/pulls"))
    query = urlencode({"event": "pull_request", "head_sha": sha})
    runs = api.pages(f"{prefix}/actions/workflows/ci.yml/runs?{query}", "workflow_runs")
    if any(other["id"] > run_id
           and (other.get("head_repository") or {}).get("full_name") == head_repository
           and other["head_branch"] == run["head_branch"] for other in runs):
        return
    artifacts = list(api.pages(f"{prefix}/actions/runs/{run_id}/artifacts", "artifacts"))
    for associated in pulls:
        number = associated["number"]
        pr = api.request(f"{prefix}/pulls/{number}")
        if (pr["state"] != "open" or pr["head"]["sha"] != sha
                or pr["base"]["repo"]["full_name"] != repository
                or (pr["head"].get("repo") or {}).get("full_name") != head_repository):
            continue
        body = build_comment(repository, run, number, artifacts, server_url)
        comments = api.pages(f"{prefix}/issues/{number}/comments")
        existing = next((comment for comment in comments
                         if comment["user"]["login"] == "github-actions[bot]"
                         and comment["user"]["type"] == "Bot"
                         and (comment.get("body") or "").startswith(MARKER)), None)
        # Recheck after pagination so a push during API reads doesn't publish old links.
        latest = api.request(f"{prefix}/pulls/{number}")
        if latest["state"] != "open" or latest["head"]["sha"] != sha:
            continue
        if existing:
            if existing["body"] != body:
                api.request(f"{prefix}/issues/comments/{existing['id']}", "PATCH", {"body": body})
        else:
            api.request(f"{prefix}/issues/{number}/comments", "POST", {"body": body})


def main():
    event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text())
    run = event["workflow_run"]
    api = GitHub(os.environ["GH_TOKEN"], os.environ["GITHUB_API_URL"])
    publish(api, os.environ["GITHUB_REPOSITORY"], run["id"], run["run_attempt"],
            os.environ["GITHUB_SERVER_URL"])


if __name__ == "__main__":
    main()
