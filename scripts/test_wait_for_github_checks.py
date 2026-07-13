from __future__ import annotations

import argparse
import importlib.util
import sys
import unittest
import urllib.error
from pathlib import Path
from unittest import mock


SCRIPT_PATH = Path(__file__).with_name("wait_for_github_checks.py")
SPEC = importlib.util.spec_from_file_location("wait_for_github_checks", SCRIPT_PATH)
assert SPEC is not None
assert SPEC.loader is not None
wait_for_github_checks = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = wait_for_github_checks
SPEC.loader.exec_module(wait_for_github_checks)

ReleaseTarget = wait_for_github_checks.ReleaseTarget
PUSHED_AT = 1_700_000_000
CURRENT_RUN_CREATED_AT = "2023-11-14T22:13:20Z"


def workflow_run(
    *,
    run_id: int,
    run_number: int,
    attempt: int,
    branch: str = "v1.2.3",
    sha: str = "commit-sha",
    status: str = "completed",
    conclusion: str | None = "success",
    created_at: str = CURRENT_RUN_CREATED_AT,
) -> dict[str, object]:
    return {
        "id": run_id,
        "run_number": run_number,
        "run_attempt": attempt,
        "event": "push",
        "head_branch": branch,
        "head_sha": sha,
        "created_at": created_at,
        "status": status,
        "conclusion": conclusion,
        "html_url": f"https://github.example/actions/runs/{run_id}/attempts/{attempt}",
    }


def main_args(*, skip_ci: bool = False) -> argparse.Namespace:
    return argparse.Namespace(
        repo="owner/repo",
        workflow="ci.yml",
        tag="v1.2.3",
        ref="refs/tags/v1.2.3",
        sha=None if skip_ci else "commit-sha",
        pushed_at=None if skip_ci else PUSHED_AT,
        skip_ci=skip_ci,
        timeout=10,
        interval=0,
    )


class ReleaseTargetTests(unittest.TestCase):
    def test_lightweight_tag_resolves_directly_to_the_ci_commit(self) -> None:
        ref_payload = {
            "ref": "refs/tags/v1.2.3",
            "object": {"type": "commit", "sha": "commit-sha"},
        }

        target = wait_for_github_checks.resolve_release_target(
            "v1.2.3",
            "refs/tags/v1.2.3",
            "commit-sha",
            PUSHED_AT,
            ref_payload,
            lambda sha: self.fail(f"unexpected annotated tag lookup for {sha}"),
        )

        self.assertEqual(target.commit_sha, "commit-sha")

    def test_annotated_tag_is_peeled_to_the_ci_commit(self) -> None:
        ref_payload = {
            "ref": "refs/tags/v1.2.3",
            "object": {"type": "tag", "sha": "annotated-tag-sha"},
        }
        tag_payloads = {
            "annotated-tag-sha": {
                "object": {"type": "commit", "sha": "commit-sha"},
            }
        }

        target = wait_for_github_checks.resolve_release_target(
            "v1.2.3",
            "refs/tags/v1.2.3",
            "annotated-tag-sha",
            PUSHED_AT,
            ref_payload,
            tag_payloads.__getitem__,
        )

        self.assertEqual(
            target,
            ReleaseTarget(
                tag="v1.2.3",
                ref="refs/tags/v1.2.3",
                commit_sha="commit-sha",
                pushed_at=PUSHED_AT,
            ),
        )

    def test_annotated_tag_accepts_a_peeled_event_sha(self) -> None:
        ref_payload = {
            "ref": "refs/tags/v1.2.3",
            "object": {"type": "tag", "sha": "annotated-tag-sha"},
        }
        tag_payloads = {
            "annotated-tag-sha": {
                "object": {"type": "commit", "sha": "commit-sha"},
            }
        }

        target = wait_for_github_checks.resolve_release_target(
            "v1.2.3",
            "refs/tags/v1.2.3",
            "commit-sha",
            PUSHED_AT,
            ref_payload,
            tag_payloads.__getitem__,
        )

        self.assertEqual(target.commit_sha, "commit-sha")


class WorkflowRunSelectionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.target = ReleaseTarget(
            tag="v1.2.3",
            ref="refs/tags/v1.2.3",
            commit_sha="commit-sha",
            pushed_at=PUSHED_AT,
        )

    def test_same_sha_main_run_cannot_satisfy_the_tag_gate(self) -> None:
        runs = [
            workflow_run(
                run_id=300,
                run_number=300,
                attempt=1,
                branch="main",
            ),
            workflow_run(
                run_id=299,
                run_number=299,
                attempt=1,
                status="in_progress",
                conclusion=None,
            ),
        ]

        selected = wait_for_github_checks.select_workflow_run(runs, self.target)

        self.assertIsNotNone(selected)
        assert selected is not None
        self.assertEqual(selected["id"], 299)

    def test_previous_push_run_is_ignored_until_the_current_run_appears(self) -> None:
        stale = workflow_run(
            run_id=350,
            run_number=350,
            attempt=1,
            created_at="2023-11-14T22:13:19Z",
        )

        self.assertIsNone(
            wait_for_github_checks.select_workflow_run([stale], self.target)
        )

        current = workflow_run(
            run_id=351,
            run_number=351,
            attempt=1,
            status="queued",
            conclusion=None,
        )
        selected = wait_for_github_checks.select_workflow_run(
            [stale, current], self.target
        )

        self.assertIsNotNone(selected)
        assert selected is not None
        self.assertEqual(selected["id"], 351)

    def test_newest_exact_run_wins_over_a_stale_success(self) -> None:
        runs = [
            workflow_run(run_id=400, run_number=400, attempt=1),
            workflow_run(
                run_id=401,
                run_number=401,
                attempt=1,
                status="in_progress",
                conclusion=None,
            ),
        ]

        selected = wait_for_github_checks.select_workflow_run(runs, self.target)

        self.assertIsNotNone(selected)
        assert selected is not None
        self.assertEqual(selected["id"], 401)
        self.assertEqual(
            wait_for_github_checks.summarize_workflow_run(selected)[0],
            "pending",
        )

    def test_exact_attempt_validation_rejects_an_older_attempt(self) -> None:
        selected = workflow_run(run_id=400, run_number=400, attempt=2)
        stale_attempt = workflow_run(run_id=400, run_number=400, attempt=1)

        with self.assertRaisesRegex(ValueError, "selected attempt"):
            wait_for_github_checks.validate_workflow_attempt(
                selected,
                stale_attempt,
                self.target,
            )

    def test_current_run_detects_a_newer_attempt(self) -> None:
        selected = workflow_run(run_id=400, run_number=400, attempt=1)
        current = workflow_run(
            run_id=400,
            run_number=400,
            attempt=2,
            status="queued",
            conclusion=None,
        )

        self.assertFalse(
            wait_for_github_checks.attempt_is_current(selected, current, self.target)
        )

    def test_pending_failure_success_and_missing_states(self) -> None:
        fixtures = [
            ("missing", None, "pending"),
            (
                "pending",
                workflow_run(
                    run_id=10,
                    run_number=10,
                    attempt=1,
                    status="queued",
                    conclusion=None,
                ),
                "pending",
            ),
            (
                "nonterminal-success",
                workflow_run(
                    run_id=11,
                    run_number=11,
                    attempt=1,
                    status="in_progress",
                    conclusion="success",
                ),
                "pending",
            ),
            (
                "failure",
                workflow_run(
                    run_id=12,
                    run_number=12,
                    attempt=1,
                    conclusion="failure",
                ),
                "failure",
            ),
            (
                "success",
                workflow_run(run_id=13, run_number=13, attempt=1),
                "success",
            ),
        ]

        for name, run, expected in fixtures:
            with self.subTest(name=name):
                state, _ = wait_for_github_checks.summarize_workflow_run(run)
                self.assertEqual(state, expected)


class MainTests(unittest.TestCase):
    def setUp(self) -> None:
        self.ref_payload = {
            "ref": "refs/tags/v1.2.3",
            "object": {"type": "commit", "sha": "commit-sha"},
        }

    def test_release_ref_resolution_retries_a_transient_failure(self) -> None:
        fetch_git_ref = mock.Mock(
            side_effect=[urllib.error.URLError("temporary outage"), self.ref_payload]
        )
        write_output = mock.Mock()

        with (
            mock.patch.object(
                wait_for_github_checks,
                "parse_args",
                return_value=main_args(skip_ci=True),
            ),
            mock.patch.object(
                wait_for_github_checks, "github_token", return_value="token"
            ),
            mock.patch.object(wait_for_github_checks, "fetch_git_ref", fetch_git_ref),
            mock.patch.object(
                wait_for_github_checks,
                "write_release_sha_output",
                write_output,
            ),
        ):
            result = wait_for_github_checks.main()

        self.assertEqual(result, 0)
        self.assertEqual(fetch_git_ref.call_count, 2)
        write_output.assert_called_once_with(
            ReleaseTarget(
                tag="v1.2.3",
                ref="refs/tags/v1.2.3",
                commit_sha="commit-sha",
                pushed_at=0,
            )
        )

    def test_new_attempt_starting_during_success_check_is_polled(self) -> None:
        attempt_1 = workflow_run(run_id=400, run_number=400, attempt=1)
        attempt_2_queued = workflow_run(
            run_id=400,
            run_number=400,
            attempt=2,
            status="queued",
            conclusion=None,
        )
        attempt_2_success = workflow_run(run_id=400, run_number=400, attempt=2)
        fetch_attempt = mock.Mock(side_effect=[attempt_1, attempt_2_success])

        with (
            mock.patch.object(
                wait_for_github_checks, "parse_args", return_value=main_args()
            ),
            mock.patch.object(
                wait_for_github_checks, "github_token", return_value="token"
            ),
            mock.patch.object(
                wait_for_github_checks,
                "fetch_git_ref",
                return_value=self.ref_payload,
            ),
            mock.patch.object(
                wait_for_github_checks,
                "fetch_workflow_runs",
                side_effect=[[attempt_1], [attempt_2_success]],
            ),
            mock.patch.object(
                wait_for_github_checks, "fetch_workflow_attempt", fetch_attempt
            ),
            mock.patch.object(
                wait_for_github_checks,
                "fetch_workflow_run",
                side_effect=[attempt_2_queued, attempt_2_success],
            ),
            mock.patch.object(wait_for_github_checks, "write_release_sha_output"),
        ):
            result = wait_for_github_checks.main()

        self.assertEqual(result, 0)
        self.assertEqual(
            [call.args[2] for call in fetch_attempt.call_args_list],
            [1, 2],
        )


class ReleaseWorkflowTests(unittest.TestCase):
    def test_manual_recovery_skip_remains_explicit(self) -> None:
        workflow_path = SCRIPT_PATH.parents[1] / ".github" / "workflows" / "release.yml"
        workflow = workflow_path.read_text(encoding="utf-8")

        self.assertIn("workflow_dispatch:", workflow)
        self.assertIn("release_tag:", workflow)
        self.assertIn("- name: Manual recovery - skip tag-push CI gate", workflow)
        self.assertIn("if: github.event_name == 'workflow_dispatch'", workflow)
        self.assertIn("args+=(--skip-ci)", workflow)
        self.assertIn('--workflow "ci.yml"', workflow)
        self.assertIn('--tag "${RELEASE_TAG}"', workflow)
        self.assertIn('--ref "${RELEASE_REF}"', workflow)
        self.assertIn('--sha "${GITHUB_SHA}"', workflow)
        self.assertIn('--pushed-at "${RELEASE_PUSHED_AT}"', workflow)
        self.assertNotIn("${{ env.RELEASE_TAG }} explicitly skips", workflow)
        self.assertNotIn('--tag "${{ env.RELEASE_TAG }}"', workflow)
        self.assertNotIn('--ref "${{ env.RELEASE_REF }}"', workflow)

    def test_release_checkouts_use_the_resolved_commit_sha(self) -> None:
        workflow_path = SCRIPT_PATH.parents[1] / ".github" / "workflows" / "release.yml"
        workflow = workflow_path.read_text(encoding="utf-8")

        checkout_count = workflow.count("- name: Checkout code")
        immutable_checkout = "ref: ${{ needs.check-ci.outputs.release_sha }}"

        self.assertIn(
            "release_sha: ${{ steps.release-target.outputs.release_sha }}",
            workflow,
        )
        self.assertEqual(workflow.count(immutable_checkout), checkout_count)
        self.assertNotIn("ref: ${{ env.RELEASE_REF }}", workflow)


if __name__ == "__main__":
    unittest.main()
