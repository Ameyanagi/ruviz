#!/usr/bin/env python3
"""Poll GitHub check runs until required checks succeed."""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from typing import Any

API_VERSION = "2022-11-28"
USER_AGENT = "ruviz-release-workflow/1.0 (https://github.com/Ameyanagi/ruviz)"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Wait for one or more GitHub check runs to complete successfully.",
    )
    parser.add_argument("--repo", required=True, help="GitHub repository in owner/name form")
    parser.add_argument("--sha", required=True, help="Commit SHA to inspect")
    parser.add_argument(
        "--check",
        action="append",
        required=True,
        dest="checks",
        help="Exact check run name to require; may be passed multiple times",
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=1800,
        help="Maximum number of seconds to wait before failing",
    )
    parser.add_argument(
        "--interval",
        type=int,
        default=15,
        help="Polling interval in seconds",
    )
    return parser.parse_args()


def github_token() -> str:
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if not token:
        raise SystemExit("Set GITHUB_TOKEN or GH_TOKEN before running this script.")
    return token


def fetch_check_runs(repo: str, sha: str, token: str) -> list[dict[str, Any]]:
    page = 1
    runs: list[dict[str, Any]] = []

    while True:
        query = urllib.parse.urlencode({"per_page": 100, "page": page})
        url = f"https://api.github.com/repos/{repo}/commits/{sha}/check-runs?{query}"
        request = urllib.request.Request(
            url,
            headers={
                "Authorization": f"Bearer {token}",
                "Accept": "application/vnd.github+json",
                "User-Agent": USER_AGENT,
                "X-GitHub-Api-Version": API_VERSION,
            },
        )
        with urllib.request.urlopen(request) as response:
            payload = json.load(response)

        page_runs = payload.get("check_runs", [])
        runs.extend(page_runs)
        if len(page_runs) < 100:
            return runs
        page += 1


def latest_completed_success(runs: list[dict[str, Any]]) -> dict[str, Any] | None:
    successes = [
        run
        for run in runs
        if run.get("status") == "completed" and run.get("conclusion") == "success"
    ]
    if not successes:
        return None
    return max(successes, key=lambda run: run.get("completed_at") or "")


def summarize_check(name: str, runs: list[dict[str, Any]]) -> tuple[str, str]:
    matching = [run for run in runs if run.get("name") == name]
    success = latest_completed_success(matching)
    if success is not None:
        completed_at = success.get("completed_at", "unknown time")
        details_url = success.get("details_url", "")
        summary = f"success at {completed_at}"
        if details_url:
            summary += f" ({details_url})"
        return "success", summary

    pending = [run for run in matching if run.get("status") != "completed"]
    if pending or not matching:
        pending_statuses = sorted({run.get("status", "unknown") for run in pending})
        if pending_statuses:
            summary = f"pending ({', '.join(pending_statuses)})"
        else:
            summary = "waiting for check run to appear"
        return "pending", summary

    conclusions = sorted({run.get("conclusion", "unknown") for run in matching})
    details_url = next((run.get("details_url") for run in reversed(matching) if run.get("details_url")), "")
    summary = f"completed without success ({', '.join(conclusions)})"
    if details_url:
        summary += f" ({details_url})"
    return "failure", summary


def main() -> int:
    args = parse_args()
    token = github_token()
    deadline = time.monotonic() + args.timeout

    while True:
        try:
            runs = fetch_check_runs(args.repo, args.sha, token)
        except urllib.error.HTTPError as exc:
            message = exc.read().decode("utf-8", "replace").strip()
            print(f"GitHub API error: {exc.code} {message}", file=sys.stderr)
            return 1
        except urllib.error.URLError as exc:
            if time.monotonic() >= deadline:
                print(f"Timed out after transient GitHub API error: {exc}", file=sys.stderr)
                return 1
            print(f"GitHub API unavailable ({exc}); retrying in {args.interval}s...", file=sys.stderr)
            time.sleep(args.interval)
            continue

        states: dict[str, tuple[str, str]] = {
            name: summarize_check(name, runs) for name in args.checks
        }

        print("Current check states:")
        for name in args.checks:
            status, summary = states[name]
            print(f"- {name}: {status} - {summary}")

        failures = {name: info for name, info in states.items() if info[0] == "failure"}
        if failures:
            print("Required checks failed.", file=sys.stderr)
            return 1

        if all(status == "success" for status, _ in states.values()):
            print("All required checks succeeded.")
            return 0

        if time.monotonic() >= deadline:
            print(
                f"Timed out after waiting {args.timeout}s for required checks.",
                file=sys.stderr,
            )
            return 1

        time.sleep(args.interval)


if __name__ == "__main__":
    raise SystemExit(main())
