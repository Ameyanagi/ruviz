#!/usr/bin/env python3
"""Wait for the exact CI workflow run and attempt for a release tag."""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from collections.abc import Callable
from dataclasses import dataclass
from datetime import datetime
from typing import Any

API_VERSION = "2022-11-28"
USER_AGENT = "ruviz-release-workflow/1.0 (https://github.com/Ameyanagi/ruviz)"


@dataclass(frozen=True)
class ReleaseTarget:
    tag: str
    ref: str
    commit_sha: str
    pushed_at: int


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Wait for the exact GitHub Actions CI run for a release tag.",
    )
    parser.add_argument(
        "--repo", required=True, help="GitHub repository in owner/name form"
    )
    parser.add_argument(
        "--workflow",
        required=True,
        help="CI workflow file name or workflow ID",
    )
    parser.add_argument("--tag", required=True, help="Release tag name")
    parser.add_argument("--ref", required=True, help="Full release tag ref")
    parser.add_argument("--sha", help="SHA from the release push event")
    parser.add_argument(
        "--pushed-at",
        type=int,
        help="Unix timestamp from the release push event",
    )
    parser.add_argument(
        "--skip-ci",
        action="store_true",
        help="Resolve the release target without enforcing the tag-push CI gate",
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


def fetch_json(url: str, token: str) -> dict[str, Any]:
    request = urllib.request.Request(
        url,
        headers={
            "Authorization": f"Bearer {token}",
            "Accept": "application/vnd.github+json",
            "User-Agent": USER_AGENT,
            "X-GitHub-Api-Version": API_VERSION,
        },
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def fetch_git_ref(repo: str, ref: str, token: str) -> dict[str, Any]:
    ref_path = urllib.parse.quote(ref.removeprefix("refs/"), safe="/")
    url = f"https://api.github.com/repos/{repo}/git/ref/{ref_path}"
    return fetch_json(url, token)


def fetch_annotated_tag(repo: str, sha: str, token: str) -> dict[str, Any]:
    url = f"https://api.github.com/repos/{repo}/git/tags/{sha}"
    return fetch_json(url, token)


def resolve_release_target(
    tag: str,
    ref: str,
    event_sha: str | None,
    pushed_at: int | None,
    ref_payload: dict[str, Any],
    load_annotated_tag: Callable[[str], dict[str, Any]],
) -> ReleaseTarget:
    expected_ref = f"refs/tags/{tag}"
    if ref != expected_ref:
        raise ValueError(f"release ref {ref!r} does not match tag {tag!r}")
    if ref_payload.get("ref") != ref:
        raise ValueError(
            f"GitHub returned {ref_payload.get('ref')!r} for release ref {ref!r}"
        )

    git_object = ref_payload.get("object")
    if not isinstance(git_object, dict):
        raise ValueError(f"release ref {ref!r} has no Git object")

    tag_chain_shas: set[str] = set()
    while git_object.get("type") == "tag":
        tag_sha = git_object.get("sha")
        if not isinstance(tag_sha, str) or not tag_sha:
            raise ValueError(f"annotated tag for {ref!r} has no SHA")
        if tag_sha in tag_chain_shas:
            raise ValueError(f"annotated tag cycle detected for {ref!r}")
        tag_chain_shas.add(tag_sha)

        tag_payload = load_annotated_tag(tag_sha)
        git_object = tag_payload.get("object")
        if not isinstance(git_object, dict):
            raise ValueError(f"annotated tag {tag_sha} has no target object")

    if git_object.get("type") != "commit":
        raise ValueError(
            f"release ref {ref!r} resolves to unsupported Git object type "
            f"{git_object.get('type')!r}"
        )

    commit_sha = git_object.get("sha")
    if not isinstance(commit_sha, str) or not commit_sha:
        raise ValueError(f"release ref {ref!r} resolves to a commit without a SHA")

    if (
        event_sha is not None
        and event_sha != commit_sha
        and event_sha not in tag_chain_shas
    ):
        raise ValueError(
            f"release event SHA {event_sha} is not part of the Git object chain for {ref!r}"
        )
    if pushed_at is not None and pushed_at <= 0:
        raise ValueError("release push timestamp must be a positive Unix timestamp")

    return ReleaseTarget(
        tag=tag,
        ref=ref,
        commit_sha=commit_sha,
        pushed_at=pushed_at or 0,
    )


def fetch_workflow_runs(
    repo: str,
    workflow: str,
    target: ReleaseTarget,
    token: str,
) -> list[dict[str, Any]]:
    workflow_path = urllib.parse.quote(workflow, safe="")
    page = 1
    runs: list[dict[str, Any]] = []

    while True:
        query = urllib.parse.urlencode(
            {
                "branch": target.tag,
                "event": "push",
                "head_sha": target.commit_sha,
                "per_page": 100,
                "page": page,
            }
        )
        url = (
            f"https://api.github.com/repos/{repo}/actions/workflows/"
            f"{workflow_path}/runs?{query}"
        )
        payload = fetch_json(url, token)
        page_runs = payload.get("workflow_runs", [])
        if not isinstance(page_runs, list):
            raise ValueError("GitHub workflow-runs response is missing workflow_runs")
        runs.extend(run for run in page_runs if isinstance(run, dict))
        if len(page_runs) < 100:
            return runs
        page += 1


def fetch_workflow_attempt(
    repo: str,
    run_id: int,
    attempt: int,
    token: str,
) -> dict[str, Any]:
    url = (
        f"https://api.github.com/repos/{repo}/actions/runs/{run_id}/attempts/{attempt}"
    )
    return fetch_json(url, token)


def fetch_workflow_run(repo: str, run_id: int, token: str) -> dict[str, Any]:
    url = f"https://api.github.com/repos/{repo}/actions/runs/{run_id}"
    return fetch_json(url, token)


def run_created_at(run: dict[str, Any]) -> int | None:
    created_at = run.get("created_at")
    if not isinstance(created_at, str):
        return None
    try:
        parsed = datetime.fromisoformat(created_at.replace("Z", "+00:00"))
    except ValueError:
        return None
    if parsed.tzinfo is None:
        return None
    return int(parsed.timestamp())


def run_matches_target(run: dict[str, Any], target: ReleaseTarget) -> bool:
    created_at = run_created_at(run)
    return (
        run.get("event") == "push"
        and run.get("head_branch") == target.tag
        and run.get("head_sha") == target.commit_sha
        and created_at is not None
        and created_at >= target.pushed_at
    )


def run_order(run: dict[str, Any]) -> tuple[int, int, int]:
    return (
        int(run.get("run_number") or 0),
        int(run.get("run_attempt") or 0),
        int(run.get("id") or 0),
    )


def select_workflow_run(
    runs: list[dict[str, Any]],
    target: ReleaseTarget,
) -> dict[str, Any] | None:
    matching = [run for run in runs if run_matches_target(run, target)]
    if not matching:
        return None
    return max(matching, key=run_order)


def selected_attempt(run: dict[str, Any]) -> tuple[int, int]:
    run_id = run.get("id")
    attempt = run.get("run_attempt")
    if not isinstance(run_id, int) or not isinstance(attempt, int):
        raise ValueError("selected workflow run has no numeric id or run_attempt")
    return run_id, attempt


def validate_workflow_attempt(
    selected: dict[str, Any],
    attempt: dict[str, Any],
    target: ReleaseTarget,
) -> None:
    selected_identity = selected_attempt(selected)
    attempt_identity = selected_attempt(attempt)
    if attempt_identity != selected_identity:
        raise ValueError(
            f"GitHub returned workflow attempt {attempt_identity} for selected attempt "
            f"{selected_identity}"
        )
    if not run_matches_target(attempt, target):
        raise ValueError(
            "selected workflow attempt no longer matches the release target"
        )


def attempt_is_current(
    selected: dict[str, Any],
    current: dict[str, Any],
    target: ReleaseTarget,
) -> bool:
    selected_run_id, selected_attempt_number = selected_attempt(selected)
    current_run_id, current_attempt_number = selected_attempt(current)
    if current_run_id != selected_run_id:
        raise ValueError(
            f"GitHub returned current run {current_run_id} for selected run {selected_run_id}"
        )
    if not run_matches_target(current, target):
        raise ValueError("current workflow run no longer matches the release target")
    if current_attempt_number < selected_attempt_number:
        raise ValueError(
            f"current workflow attempt {current_attempt_number} is older than selected "
            f"attempt {selected_attempt_number}"
        )
    return current_attempt_number == selected_attempt_number


def summarize_workflow_run(run: dict[str, Any] | None) -> tuple[str, str]:
    if run is None:
        return "pending", "waiting for the release-tag CI workflow run to appear"

    run_id, attempt = selected_attempt(run)
    status = run.get("status") or "unknown"
    conclusion = run.get("conclusion") or "none"
    details_url = run.get("html_url") or ""
    identity = f"run {run_id}, attempt {attempt}"

    if status != "completed":
        summary = f"{identity} is {status}"
        if details_url:
            summary += f" ({details_url})"
        return "pending", summary

    summary = f"{identity} completed with conclusion {conclusion}"
    if details_url:
        summary += f" ({details_url})"
    if conclusion == "success":
        return "success", summary
    return "failure", summary


def print_http_error(exc: urllib.error.HTTPError) -> None:
    message = exc.read().decode("utf-8", "replace").strip()
    print(f"GitHub API error: {exc.code} {message}", file=sys.stderr)


def is_retryable_http_error(exc: urllib.error.HTTPError) -> bool:
    return exc.code == 429 or 500 <= exc.code < 600


def retry_api_error(
    exc: urllib.error.URLError,
    *,
    deadline: float,
    interval: int,
    context: str,
) -> bool:
    if time.monotonic() >= deadline:
        print(
            f"Timed out after transient GitHub API error while {context}: {exc}",
            file=sys.stderr,
        )
        return False
    print(
        f"GitHub API unavailable while {context} ({exc}); retrying in {interval}s...",
        file=sys.stderr,
    )
    time.sleep(interval)
    return True


def write_release_sha_output(target: ReleaseTarget) -> None:
    output_path = os.environ.get("GITHUB_OUTPUT")
    if output_path:
        with open(output_path, "a", encoding="utf-8") as output:
            print(f"release_sha={target.commit_sha}", file=output)


def main() -> int:
    args = parse_args()
    token = github_token()
    deadline = time.monotonic() + args.timeout

    if not args.skip_ci and (args.sha is None or args.pushed_at is None):
        print(
            "--sha and --pushed-at are required unless --skip-ci is used",
            file=sys.stderr,
        )
        return 1

    while True:
        try:
            ref_payload = fetch_git_ref(args.repo, args.ref, token)
            target = resolve_release_target(
                args.tag,
                args.ref,
                args.sha,
                args.pushed_at,
                ref_payload,
                lambda sha: fetch_annotated_tag(args.repo, sha, token),
            )
            break
        except urllib.error.HTTPError as exc:
            if not is_retryable_http_error(exc):
                print_http_error(exc)
                return 1
            if not retry_api_error(
                exc,
                deadline=deadline,
                interval=args.interval,
                context="resolving the release ref",
            ):
                return 1
        except urllib.error.URLError as exc:
            if not retry_api_error(
                exc,
                deadline=deadline,
                interval=args.interval,
                context="resolving the release ref",
            ):
                return 1
        except ValueError as exc:
            print(f"Invalid release target: {exc}", file=sys.stderr)
            return 1

    print(f"Release target: {target.ref} -> {target.commit_sha}")
    write_release_sha_output(target)

    if args.skip_ci:
        print(
            "Manual recovery explicitly skips the tag-push CI gate after resolving the "
            "immutable release commit."
        )
        return 0

    while True:
        try:
            runs = fetch_workflow_runs(args.repo, args.workflow, target, token)
            selected = select_workflow_run(runs, target)
            if selected is None:
                attempt = None
            else:
                run_id, attempt_number = selected_attempt(selected)
                attempt = fetch_workflow_attempt(
                    args.repo,
                    run_id,
                    attempt_number,
                    token,
                )
                validate_workflow_attempt(selected, attempt, target)
                current = fetch_workflow_run(args.repo, run_id, token)
                if not attempt_is_current(selected, current, target):
                    print(
                        f"CI workflow run {run_id} advanced beyond attempt "
                        f"{attempt_number}; polling the current attempt."
                    )
                    if time.monotonic() >= deadline:
                        print(
                            f"Timed out after waiting {args.timeout}s for the "
                            "release-tag CI workflow.",
                            file=sys.stderr,
                        )
                        return 1
                    time.sleep(args.interval)
                    continue
        except urllib.error.HTTPError as exc:
            if not is_retryable_http_error(exc):
                print_http_error(exc)
                return 1
            if retry_api_error(
                exc,
                deadline=deadline,
                interval=args.interval,
                context="polling the release-tag CI workflow",
            ):
                continue
            return 1
        except urllib.error.URLError as exc:
            if retry_api_error(
                exc,
                deadline=deadline,
                interval=args.interval,
                context="polling the release-tag CI workflow",
            ):
                continue
            return 1
        except ValueError as exc:
            print(f"Invalid GitHub workflow response: {exc}", file=sys.stderr)
            return 1

        state, summary = summarize_workflow_run(attempt)
        print(f"CI workflow: {state} - {summary}")

        if state == "failure":
            print(
                "The release-tag CI workflow attempt did not succeed. Recover CI, then "
                f"manually run the Release workflow with release_tag={target.tag}; manual "
                "recovery explicitly skips the tag-push CI gate.",
                file=sys.stderr,
            )
            return 1
        if state == "success":
            print("The exact release-tag CI workflow attempt succeeded.")
            return 0
        if time.monotonic() >= deadline:
            print(
                f"Timed out after waiting {args.timeout}s for the release-tag CI workflow.",
                file=sys.stderr,
            )
            return 1

        time.sleep(args.interval)


if __name__ == "__main__":
    raise SystemExit(main())
