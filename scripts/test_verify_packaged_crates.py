from __future__ import annotations

import argparse
import importlib.util
import io
import re
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest import mock


SCRIPT_PATH = Path(__file__).with_name("verify_packaged_crates.py")
SPEC = importlib.util.spec_from_file_location("verify_packaged_crates", SCRIPT_PATH)
assert SPEC is not None
assert SPEC.loader is not None
verify_packaged_crates = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = verify_packaged_crates
SPEC.loader.exec_module(verify_packaged_crates)

LEGACY_CRATES_IO_SOURCE = verify_packaged_crates.LEGACY_CRATES_IO_SOURCE
SPARSE_CRATES_IO_SOURCE = verify_packaged_crates.SPARSE_CRATES_IO_SOURCE
ExtractedCrate = verify_packaged_crates.ExtractedCrate
VerificationError = verify_packaged_crates.VerificationError
WorkspaceContract = verify_packaged_crates.WorkspaceContract


def add_archive_file(archive: tarfile.TarFile, name: str, contents: str) -> None:
    payload = contents.encode("utf-8")
    member = tarfile.TarInfo(name)
    member.size = len(payload)
    archive.addfile(member, io.BytesIO(payload))


def parse_workflow_jobs(path: Path) -> dict[str, dict]:
    """Parse the small job/step subset needed by these workflow assertions."""
    jobs: dict[str, dict] = {}
    current_job: dict | None = None
    current_step: dict | None = None
    multiline_key: str | None = None
    multiline_indent = 0

    for raw_line in path.read_text(encoding="utf-8").splitlines():
        indent = len(raw_line) - len(raw_line.lstrip(" "))
        stripped = raw_line.strip()
        job_match = re.fullmatch(r"  ([A-Za-z0-9_-]+):", raw_line)
        if job_match:
            current_job = {"needs": [], "steps": []}
            jobs[job_match.group(1)] = current_job
            current_step = None
            multiline_key = None
            continue
        if current_job is None:
            continue
        if multiline_key is not None:
            if stripped and indent >= multiline_indent:
                current_step[multiline_key] += " " + stripped
                continue
            multiline_key = None
        if indent == 4 and stripped.startswith("needs:"):
            value = stripped.removeprefix("needs:").strip()
            if value.startswith("[") and value.endswith("]"):
                current_job["needs"] = [
                    item.strip() for item in value[1:-1].split(",") if item.strip()
                ]
            elif value:
                current_job["needs"] = [value]
            continue
        if indent == 6 and stripped.startswith("- name:"):
            current_step = {"name": stripped.split(":", 1)[1].strip()}
            current_job["steps"].append(current_step)
            continue
        if (
            current_step is not None
            and indent == 10
            and isinstance(current_step.get("with"), dict)
            and ":" in stripped
        ):
            key, value = stripped.split(":", 1)
            current_step["with"][key] = value.strip()
            continue
        if current_step is None or indent != 8 or ":" not in stripped:
            continue
        key, value = stripped.split(":", 1)
        value = value.strip()
        if key == "with" and not value:
            current_step[key] = {}
            continue
        if value in {"|", ">-"}:
            current_step[key] = ""
            multiline_key = key
            multiline_indent = 10
        else:
            current_step[key] = value

    return jobs


def step_named(job: dict, name: str) -> dict:
    return next(step for step in job["steps"] if step.get("name") == name)


class ArchiveSelectionTests(unittest.TestCase):
    def test_selects_only_the_exact_name_and_version(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            exact = directory / "ruviz-1.2.3.crate"
            exact.touch()
            (directory / "ruviz-1.2.2.crate").touch()

            selected = verify_packaged_crates.exact_archive_path(
                directory, "ruviz", "1.2.3"
            )

            self.assertEqual(selected, exact.resolve())

    def test_missing_exact_version_does_not_fall_back(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            (directory / "ruviz-1.2.2.crate").touch()

            with self.assertRaisesRegex(VerificationError, "ruviz-1.2.3.crate"):
                verify_packaged_crates.exact_archive_path(directory, "ruviz", "1.2.3")


class ArchiveManifestTests(unittest.TestCase):
    def test_archive_manifest_name_and_version_are_validated(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            archive_path = directory / "input.crate"
            with tarfile.open(archive_path, "w:gz") as archive:
                add_archive_file(
                    archive,
                    "ruviz-1.2.3/Cargo.toml",
                    '[package]\nname = "different"\nversion = "1.2.3"\n',
                )

            with self.assertRaisesRegex(VerificationError, "expected ruviz 1.2.3"):
                verify_packaged_crates.safe_extract_archive(
                    archive_path, directory / "out", "ruviz", "1.2.3"
                )

    def test_expected_release_sha_is_checked_from_archive_metadata(self) -> None:
        crate = ExtractedCrate(
            "ruviz-gpui",
            "1.2.3",
            Path("archive.crate"),
            Path("ruviz-gpui-1.2.3"),
            {},
            "a" * 40,
            False,
        )

        verify_packaged_crates.require_archive_vcs_sha(crate, "a" * 40)
        with self.assertRaisesRegex(VerificationError, "exact release SHA"):
            verify_packaged_crates.require_archive_vcs_sha(crate, "b" * 40)

    def test_missing_dirty_field_means_clean_archive(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            archive_path = directory / "input.crate"
            with tarfile.open(archive_path, "w:gz") as archive:
                add_archive_file(
                    archive,
                    "ruviz-1.2.3/Cargo.toml",
                    '[package]\nname = "ruviz"\nversion = "1.2.3"\n',
                )
                add_archive_file(
                    archive,
                    "ruviz-1.2.3/.cargo_vcs_info.json",
                    '{"git":{"sha1":"' + "a" * 40 + '"}}',
                )

            crate = verify_packaged_crates.safe_extract_archive(
                archive_path, directory / "out", "ruviz", "1.2.3"
            )

            self.assertIs(crate.vcs_dirty, False)
            verify_packaged_crates.require_archive_vcs_sha(crate, "a" * 40)

    def test_expected_release_sha_rejects_dirty_archive(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            archive_path = directory / "input.crate"
            with tarfile.open(archive_path, "w:gz") as archive:
                add_archive_file(
                    archive,
                    "ruviz-1.2.3/Cargo.toml",
                    '[package]\nname = "ruviz"\nversion = "1.2.3"\n',
                )
                add_archive_file(
                    archive,
                    "ruviz-1.2.3/.cargo_vcs_info.json",
                    '{"git":{"sha1":"' + "a" * 40 + '","dirty":true}}',
                )

            crate = verify_packaged_crates.safe_extract_archive(
                archive_path, directory / "out", "ruviz", "1.2.3"
            )

            self.assertIs(crate.vcs_dirty, True)
            with self.assertRaisesRegex(VerificationError, "dirty=True"):
                verify_packaged_crates.require_archive_vcs_sha(crate, "a" * 40)

    def test_archive_cannot_escape_expected_package_root(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            archive_path = directory / "input.crate"
            with tarfile.open(archive_path, "w:gz") as archive:
                add_archive_file(archive, "../outside", "bad")

            with self.assertRaisesRegex(VerificationError, "unexpected member"):
                verify_packaged_crates.safe_extract_archive(
                    archive_path, directory / "out", "ruviz", "1.2.3"
                )

    def test_archive_rejects_windows_escape_spellings(self) -> None:
        members = (
            r"ruviz-1.2.3/..\..\outside",
            r"ruviz-1.2.3/\\server\share\escaped",
            "ruviz-1.2.3/C:escaped",
        )
        for member in members:
            with (
                self.subTest(member=member),
                tempfile.TemporaryDirectory() as temporary,
            ):
                directory = Path(temporary)
                archive_path = directory / "input.crate"
                with tarfile.open(archive_path, "w:gz") as archive:
                    add_archive_file(archive, member, "bad")

                with self.assertRaisesRegex(VerificationError, "unexpected member"):
                    verify_packaged_crates.safe_extract_archive(
                        archive_path, directory / "out", "ruviz", "1.2.3"
                    )


class RegistrySourceTests(unittest.TestCase):
    def test_accepts_cargo_crates_io_source_spellings(self) -> None:
        for source in (
            LEGACY_CRATES_IO_SOURCE,
            SPARSE_CRATES_IO_SOURCE,
            "sparse+https://index.crates.io/",
            "registry+https://index.crates.io/",
        ):
            with self.subTest(source=source):
                self.assertTrue(verify_packaged_crates.is_crates_io_source(source))

    def test_rejects_non_crates_io_registry(self) -> None:
        self.assertFalse(
            verify_packaged_crates.is_crates_io_source(
                "registry+sparse+https://registry.example.invalid/"
            )
        )


class WorkspaceContractTests(unittest.TestCase):
    def test_discovers_target_specific_workspace_only_dev_dependencies(self) -> None:
        contract = verify_packaged_crates.inspect_workspace(SCRIPT_PATH.parents[1])

        self.assertEqual(
            contract.workspace_only_dev_dependencies,
            frozenset({"gpui_macos", "gpui_platform"}),
        )


class PackagingModeTests(unittest.TestCase):
    def test_package_command_uses_exactly_one_lockfile_mode(self) -> None:
        for locked, expected, rejected in (
            (True, "--locked", "--exclude-lockfile"),
            (False, "--exclude-lockfile", "--locked"),
        ):
            with (
                self.subTest(locked=locked),
                tempfile.TemporaryDirectory() as temporary,
            ):
                target = Path(temporary) / "target"
                archive = target / "package/ruviz-1.2.3.crate"
                archive.parent.mkdir(parents=True)
                archive.touch()
                success = mock.Mock(returncode=0, stdout="", stderr="")
                with mock.patch.object(
                    verify_packaged_crates, "run_command", return_value=success
                ) as run:
                    verify_packaged_crates.package_crate(
                        workspace=Path(temporary),
                        target_dir=target,
                        name="ruviz",
                        version="1.2.3",
                        locked=locked,
                    )

                command = run.call_args.args[0]
                self.assertIn(expected, command)
                self.assertNotIn(rejected, command)

    def test_locked_adapter_packaging_retries_registry_propagation(self) -> None:
        error = mock.Mock(
            returncode=101,
            stdout="",
            stderr=(
                "error: failed to select a version for the requirement "
                '`ruviz = "=1.2.3"`\nlocation searched: crates.io index'
            ),
        )
        success = mock.Mock(returncode=0, stdout="", stderr="")
        with tempfile.TemporaryDirectory() as temporary:
            target = Path(temporary) / "target"
            archive = target / "package/ruviz-gpui-1.2.3.crate"
            archive.parent.mkdir(parents=True)
            archive.touch()
            with (
                mock.patch.object(
                    verify_packaged_crates,
                    "run_command",
                    side_effect=[error, success],
                ) as run,
                mock.patch.object(verify_packaged_crates.time, "sleep") as sleep,
            ):
                verify_packaged_crates.package_crate(
                    workspace=Path(temporary),
                    target_dir=target,
                    name="ruviz-gpui",
                    version="1.2.3",
                    locked=True,
                    registry_attempts=2,
                    registry_delay=0.25,
                    registry_package="ruviz",
                    registry_version="1.2.3",
                )

        self.assertEqual(run.call_count, 2)
        sleep.assert_called_once_with(0.25)

    def verify_package_calls(self, mode: str) -> list[object]:
        contract = WorkspaceContract(
            version="1.2.3",
            gpui_version="0.2.2",
            gpui_patch_git="https://github.com/zed-industries/zed",
            gpui_patch_rev="abc123",
            workspace_only_dev_dependencies=frozenset(),
        )
        ruviz = ExtractedCrate(
            "ruviz", "1.2.3", Path("ruviz.crate"), Path("/tmp/ruviz"), {}, None
        )
        adapter = ExtractedCrate(
            "ruviz-gpui",
            "1.2.3",
            Path("ruviz-gpui.crate"),
            Path("/tmp/ruviz-gpui"),
            {},
            None,
        )
        args = argparse.Namespace(
            workspace=Path("/tmp/ruviz-package-workspace"),
            ruviz_archive=None,
            ruviz_gpui_archive=None,
            expected_vcs_sha=None,
            mode=mode,
            registry_attempts=60,
            registry_delay=20.0,
        )

        def create_lockfile(consumer: Path, **_kwargs: object) -> None:
            (consumer / "Cargo.lock").touch()

        metadata_result = mock.Mock(stdout="{}")
        with (
            mock.patch.object(
                verify_packaged_crates, "inspect_workspace", return_value=contract
            ),
            mock.patch.object(
                verify_packaged_crates,
                "package_crate",
                side_effect=[Path("ruviz.crate"), Path("ruviz-gpui.crate")],
            ) as package,
            mock.patch.object(
                verify_packaged_crates,
                "safe_extract_archive",
                side_effect=[ruviz, adapter],
            ),
            mock.patch.object(verify_packaged_crates, "validate_normalized_manifests"),
            mock.patch.object(
                verify_packaged_crates,
                "generate_lockfile_with_registry_retries",
                side_effect=create_lockfile,
            ),
            mock.patch.object(
                verify_packaged_crates,
                "run_command",
                side_effect=[metadata_result, mock.Mock()],
            ),
            mock.patch.object(verify_packaged_crates, "validate_metadata"),
        ):
            verify_packaged_crates.verify(args)

        return package.call_args_list

    def test_ci_only_excludes_lockfile_for_unpublished_gpui_adapter(self) -> None:
        core, adapter = self.verify_package_calls("ci")

        self.assertIs(core.kwargs["locked"], True)
        self.assertIs(adapter.kwargs["locked"], False)
        self.assertEqual(adapter.kwargs["registry_attempts"], 1)

    def test_release_packages_both_crates_locked_after_core_publish(self) -> None:
        core, adapter = self.verify_package_calls("release")

        self.assertIs(core.kwargs["locked"], True)
        self.assertIs(adapter.kwargs["locked"], True)
        self.assertEqual(adapter.kwargs["registry_attempts"], 60)
        self.assertEqual(adapter.kwargs["registry_package"], "ruviz")
        self.assertEqual(adapter.kwargs["registry_version"], "1.2.3")


class MetadataSourceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        root = Path(self.temporary.name)
        self.workspace = root / "workspace"
        self.consumer = root / "consumer"
        self.ruviz_root = root / "archives/ruviz-1.2.3"
        self.gpui_root = root / "archives/ruviz-gpui-1.2.3"
        for directory in (
            self.workspace,
            self.consumer,
            self.ruviz_root,
            self.gpui_root,
        ):
            directory.mkdir(parents=True)
            (directory / "Cargo.toml").touch()
        self.contract = WorkspaceContract(
            version="1.2.3",
            gpui_version="0.2.2",
            gpui_patch_git="https://github.com/zed-industries/zed",
            gpui_patch_rev="abc123",
            workspace_only_dev_dependencies=frozenset({"gpui_macos", "gpui_platform"}),
        )
        self.ruviz = ExtractedCrate(
            "ruviz", "1.2.3", root / "ruviz.crate", self.ruviz_root, {}, None
        )
        self.ruviz_gpui = ExtractedCrate(
            "ruviz-gpui",
            "1.2.3",
            root / "ruviz-gpui.crate",
            self.gpui_root,
            {},
            None,
        )

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def metadata(
        self, *, mode: str, gpui_source: str = LEGACY_CRATES_IO_SOURCE
    ) -> dict:
        core_source = None if mode == "ci" else LEGACY_CRATES_IO_SOURCE
        core_manifest = (
            self.ruviz_root / "Cargo.toml"
            if mode == "ci"
            else Path("/cargo/registry/ruviz-1.2.3/Cargo.toml")
        )
        ids = {
            "consumer": "path+file:///tmp/consumer#ruviz-package-consumer@0.0.0",
            "ruviz-gpui": "path+file:///tmp/ruviz-gpui#1.2.3",
            "ruviz": f"{core_source or 'path+file:///tmp/ruviz'}#ruviz@1.2.3",
            "gpui": f"{gpui_source}#gpui@0.2.2",
        }
        return {
            "workspace_root": str(self.consumer),
            "packages": [
                {
                    "name": "ruviz-package-consumer",
                    "version": "0.0.0",
                    "id": ids["consumer"],
                    "manifest_path": str(self.consumer / "Cargo.toml"),
                    "source": None,
                },
                {
                    "name": "ruviz-gpui",
                    "version": "1.2.3",
                    "id": ids["ruviz-gpui"],
                    "manifest_path": str(self.gpui_root / "Cargo.toml"),
                    "source": None,
                },
                {
                    "name": "ruviz",
                    "version": "1.2.3",
                    "id": ids["ruviz"],
                    "manifest_path": str(core_manifest),
                    "source": core_source,
                },
                {
                    "name": "gpui",
                    "version": "0.2.2",
                    "id": ids["gpui"],
                    "manifest_path": "/cargo/registry/gpui-0.2.2/Cargo.toml",
                    "source": gpui_source,
                },
            ],
            "resolve": {
                "nodes": [
                    {
                        "id": ids["consumer"],
                        "deps": [
                            {
                                "name": "ruviz",
                                "pkg": ids["ruviz"],
                                "dep_kinds": [{"kind": None, "target": None}],
                            },
                            {
                                "name": "ruviz-gpui",
                                "pkg": ids["ruviz-gpui"],
                                "dep_kinds": [{"kind": None, "target": None}],
                            },
                        ],
                    },
                    {"id": ids["ruviz"], "deps": []},
                    {
                        "id": ids["ruviz-gpui"],
                        "deps": [
                            {
                                "name": "gpui",
                                "pkg": ids["gpui"],
                                "dep_kinds": [{"kind": None, "target": None}],
                            },
                            {
                                "name": "ruviz",
                                "pkg": ids["ruviz"],
                                "dep_kinds": [{"kind": None, "target": None}],
                            },
                        ],
                    },
                    {"id": ids["gpui"], "deps": []},
                ]
            },
        }

    def validate(self, metadata: dict, mode: str) -> None:
        verify_packaged_crates.validate_metadata(
            metadata,
            mode=mode,
            workspace=self.workspace,
            consumer=self.consumer,
            ruviz=self.ruviz,
            ruviz_gpui=self.ruviz_gpui,
            contract=self.contract,
        )

    def test_ci_accepts_local_ruviz_and_registry_gpui(self) -> None:
        self.validate(self.metadata(mode="ci"), "ci")

    def test_release_accepts_registry_ruviz_and_gpui(self) -> None:
        self.validate(self.metadata(mode="release"), "release")

    def test_release_accepts_sparse_crates_io_sources(self) -> None:
        metadata = self.metadata(mode="release", gpui_source=SPARSE_CRATES_IO_SOURCE)
        core = next(
            package for package in metadata["packages"] if package["name"] == "ruviz"
        )
        core["source"] = SPARSE_CRATES_IO_SOURCE

        self.validate(metadata, "release")

    def test_gpui_git_source_is_rejected(self) -> None:
        metadata = self.metadata(
            mode="ci",
            gpui_source="git+https://github.com/zed-industries/zed?rev=abc123",
        )

        with self.assertRaisesRegex(VerificationError, "GPUI git patch leaked"):
            self.validate(metadata, "ci")

    def test_gpui_other_registry_is_rejected(self) -> None:
        metadata = self.metadata(
            mode="ci",
            gpui_source="registry+sparse+https://registry.example.invalid/",
        )

        with self.assertRaisesRegex(
            VerificationError, "GPUI must resolve from crates.io"
        ):
            self.validate(metadata, "ci")

    def test_release_ruviz_path_source_is_rejected(self) -> None:
        metadata = self.metadata(mode="release")
        core = next(
            package for package in metadata["packages"] if package["name"] == "ruviz"
        )
        core["source"] = None
        core["manifest_path"] = str(self.ruviz_root / "Cargo.toml")

        with self.assertRaisesRegex(VerificationError, "unexpected path package ruviz"):
            self.validate(metadata, "release")

    def test_resolved_dev_dependency_is_rejected(self) -> None:
        metadata = self.metadata(mode="ci")
        adapter_id = next(
            package["id"]
            for package in metadata["packages"]
            if package["name"] == "ruviz-gpui"
        )
        adapter_node = next(
            node for node in metadata["resolve"]["nodes"] if node["id"] == adapter_id
        )
        adapter_node["deps"].append(
            {
                "name": "test-helper",
                "pkg": "registry+example#test-helper@1.0.0",
                "dep_kinds": [{"kind": "dev", "target": None}],
            }
        )

        with self.assertRaisesRegex(VerificationError, "dev dependency"):
            self.validate(metadata, "ci")

    def test_workspace_only_dev_package_is_rejected(self) -> None:
        metadata = self.metadata(mode="ci")
        metadata["packages"].append(
            {
                "name": "gpui_platform",
                "version": "0.1.0",
                "id": "registry+example#gpui_platform@0.1.0",
                "manifest_path": "/cargo/registry/gpui_platform/Cargo.toml",
                "source": LEGACY_CRATES_IO_SOURCE,
            }
        )

        with self.assertRaisesRegex(VerificationError, "workspace-only dev"):
            self.validate(metadata, "ci")

    def test_adapter_must_resolve_the_expected_core_edge(self) -> None:
        metadata = self.metadata(mode="ci")
        adapter_id = next(
            package["id"]
            for package in metadata["packages"]
            if package["name"] == "ruviz-gpui"
        )
        adapter_node = next(
            node for node in metadata["resolve"]["nodes"] if node["id"] == adapter_id
        )
        core_edge = next(
            edge for edge in adapter_node["deps"] if edge["name"] == "ruviz"
        )
        core_edge["pkg"] = "registry+example#ruviz@1.2.3"

        with self.assertRaisesRegex(VerificationError, "normal ruviz edge"):
            self.validate(metadata, "ci")


class NormalizedDependencyTests(unittest.TestCase):
    def extracted(self, manifest: dict) -> ExtractedCrate:
        return ExtractedCrate(
            "ruviz-gpui",
            "1.2.3",
            Path("/tmp/ruviz-gpui.crate"),
            Path("/tmp/ruviz-gpui-1.2.3"),
            manifest,
            None,
        )

    def test_workspace_only_target_dev_dependency_is_rejected(self) -> None:
        crate = self.extracted(
            {
                "target": {
                    'cfg(target_os = "linux")': {
                        "dev-dependencies": {
                            "gpui_platform": {
                                "git": "https://github.com/zed-industries/zed",
                                "rev": "abc123",
                            }
                        }
                    }
                }
            }
        )

        with self.assertRaisesRegex(VerificationError, "workspace-only dependency"):
            verify_packaged_crates.validate_active_dependency_sources(crate)

    def test_path_dependency_is_rejected_in_any_dependency_table(self) -> None:
        crate = self.extracted(
            {"dev-dependencies": {"helper": {"path": "../../helper"}}}
        )

        with self.assertRaisesRegex(VerificationError, "non-crates.io source"):
            verify_packaged_crates.validate_active_dependency_sources(crate)


class RegistryRetryTests(unittest.TestCase):
    def test_exact_version_index_lag_is_retryable(self) -> None:
        error = """error: failed to select a version for the requirement `ruviz = \"=1.2.3\"`
candidate versions found which didn't match: 1.2.2
location searched: crates.io index
"""
        self.assertTrue(
            verify_packaged_crates.is_registry_propagation_error(
                error, "ruviz", "1.2.3"
            )
        )

    def test_manifest_error_is_not_retryable(self) -> None:
        error = "error: failed to parse manifest at `/tmp/consumer/Cargo.toml`"
        self.assertFalse(
            verify_packaged_crates.is_registry_propagation_error(
                error, "ruviz", "1.2.3"
            )
        )

    def test_deterministic_failure_stops_after_one_attempt(self) -> None:
        failure = mock.Mock(returncode=101, stderr="failed to parse manifest")
        with mock.patch.object(
            verify_packaged_crates, "run_command", return_value=failure
        ) as run:
            with self.assertRaisesRegex(VerificationError, "not retrying"):
                verify_packaged_crates.generate_lockfile_with_registry_retries(
                    Path("/tmp/consumer"),
                    env={},
                    attempts=60,
                    delay=0,
                    registry_package="ruviz",
                    registry_version="1.2.3",
                )

        run.assert_called_once()


class WorkflowIntegrationTests(unittest.TestCase):
    def test_ci_has_one_packaged_crate_gate(self) -> None:
        jobs = parse_workflow_jobs(SCRIPT_PATH.parents[1] / ".github/workflows/ci.yml")
        packaged = jobs["packaged-crates"]

        self.assertEqual(packaged["needs"], ["fmt", "clippy"])
        self.assertEqual(
            step_named(packaged, "Test packaged-crate verifier")["run"],
            "uv run python -m unittest scripts/test_verify_packaged_crates.py",
        )
        self.assertEqual(
            step_named(packaged, "Verify packaged crates from an external consumer")[
                "run"
            ],
            "uv run python scripts/verify_packaged_crates.py --mode ci",
        )
        self.assertTrue(
            {"test-fast", "test-feature-contract", "test-visual-heavy"}.issubset(jobs),
            "P01 feature lanes must remain present",
        )

    def test_release_gate_uses_resolved_sha_and_blocks_gpui_publish(self) -> None:
        jobs = parse_workflow_jobs(
            SCRIPT_PATH.parents[1] / ".github/workflows/release.yml"
        )
        verify_job = jobs["verify-packaged-crates"]
        gpui_job = jobs["publish-ruviz-gpui"]

        self.assertEqual(verify_job["needs"], ["check-ci", "publish-ruviz"])
        checkout = step_named(verify_job, "Checkout code")
        self.assertEqual(
            checkout["with"]["ref"], "${{ needs.check-ci.outputs.release_sha }}"
        )
        verify_run = step_named(verify_job, "Verify packaged crates against crates.io")[
            "run"
        ]
        self.assertIn("uv run python scripts/verify_packaged_crates.py", verify_run)
        self.assertIn("--mode release", verify_run)
        self.assertIn(
            "--expected-vcs-sha ${{ needs.check-ci.outputs.release_sha }}", verify_run
        )
        self.assertEqual(gpui_job["needs"], ["check-ci", "verify-packaged-crates"])

        publish_steps = [
            (job_name, step)
            for job_name, job in jobs.items()
            for step in job["steps"]
            if "cargo publish --package ruviz-gpui" in step.get("run", "")
        ]
        self.assertEqual(
            [(name, step["name"]) for name, step in publish_steps],
            [("publish-ruviz-gpui", "Publish ruviz-gpui to crates.io")],
        )
        self.assertEqual(
            step_named(gpui_job, "Skip ruviz-gpui publish")["if"],
            "steps.gpui-version.outputs.published == 'true'",
            "idempotent recovery must still pass through the verification job",
        )


if __name__ == "__main__":
    unittest.main()
