from __future__ import annotations

import importlib.util
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("smoke_native_gui_examples.py")
SPEC = importlib.util.spec_from_file_location("smoke_native_gui_examples", SCRIPT)
assert SPEC is not None
assert SPEC.loader is not None
smoke = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = smoke
SPEC.loader.exec_module(smoke)


def executable_script(directory: Path, name: str, contents: str) -> Path:
    path = directory / name
    path.write_text("#!/bin/sh\n" + contents, encoding="utf-8")
    path.chmod(0o755)
    return path


def artifact_message(manifest: Path, example: str, executable: Path) -> str:
    return json.dumps(
        {
            "reason": "compiler-artifact",
            "manifest_path": str(manifest),
            "target": {"kind": ["example"], "name": example},
            "executable": str(executable),
        }
    )


class DiscoveryTests(unittest.TestCase):
    def test_discovers_exact_executables_from_multiple_cargo_streams(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            egui = executable_script(root, "egui-dashboard", "sleep 30\n")
            gpui = executable_script(root, "gpui-static", "sleep 30\n")
            egui_messages = root / "egui.json"
            gpui_messages = root / "gpui.json"
            egui_messages.write_text(
                artifact_message(
                    root / "ruviz-egui/Cargo.toml", "dashboard", egui
                )
                + "\n",
                encoding="utf-8",
            )
            gpui_messages.write_text(
                artifact_message(
                    root / "ruviz-gpui/Cargo.toml", "static_embed", gpui
                )
                + "\n",
                encoding="utf-8",
            )

            discovered = smoke.discover_examples(
                [egui_messages, gpui_messages], root
            )
            selected = smoke.select_expected_examples(
                discovered, ["ruviz-egui:dashboard", "ruviz-gpui:static_embed"]
            )

            self.assertEqual(
                [artifact.identity for artifact in selected],
                ["ruviz-egui:dashboard", "ruviz-gpui:static_embed"],
            )
            self.assertEqual(selected[0].executable, egui.resolve())
            self.assertEqual(selected[1].executable, gpui.resolve())

    def test_rejects_missing_and_unreviewed_examples(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = executable_script(root, "dashboard", "sleep 30\n")
            artifacts = {
                "ruviz-egui:dashboard": smoke.ExampleArtifact(
                    "ruviz-egui", "dashboard", binary
                )
            }

            with self.assertRaisesRegex(
                smoke.SmokeError, "missing: ruviz-iced:dashboard"
            ):
                smoke.select_expected_examples(
                    artifacts, ["ruviz-iced:dashboard"]
                )

    def test_rejects_cargo_output_collisions_between_examples(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = executable_script(root, "dashboard", "sleep 30\n")
            artifacts = {
                "ruviz-egui:dashboard": smoke.ExampleArtifact(
                    "ruviz-egui", "dashboard", binary
                ),
                "ruviz-iced:dashboard": smoke.ExampleArtifact(
                    "ruviz-iced", "dashboard", binary
                ),
            }

            with self.assertRaisesRegex(smoke.SmokeError, "same executable"):
                smoke.select_expected_examples(artifacts, artifacts)

    def test_staging_preserves_package_specific_binaries_and_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            first = executable_script(root, "egui-dashboard", "sleep 30\n")
            second = executable_script(root, "iced-dashboard", "sleep 30\n")
            artifacts = [
                smoke.ExampleArtifact("ruviz-egui", "dashboard", first),
                smoke.ExampleArtifact("ruviz-iced", "dashboard", second),
            ]
            stage = root / "stage"

            smoke.stage_examples(artifacts, stage)
            discovered = smoke.discover_staged_examples(stage)
            selected = smoke.select_expected_examples(
                discovered, ["ruviz-egui:dashboard", "ruviz-iced:dashboard"]
            )

            self.assertEqual(len(selected), 2)
            self.assertNotEqual(selected[0].executable, selected[1].executable)
            self.assertEqual(
                selected[0].executable.read_text(encoding="utf-8"),
                first.read_text(encoding="utf-8"),
            )

    def test_rejects_non_json_build_output(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            messages = root / "cargo.json"
            messages.write_text("Compiling is not JSON\n", encoding="utf-8")
            with self.assertRaisesRegex(smoke.SmokeError, "not a Cargo JSON"):
                smoke.discover_examples([messages], root)


class RuntimeTests(unittest.TestCase):
    def test_early_exit_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = executable_script(root, "exits", "exit 7\n")
            artifact = smoke.ExampleArtifact("test", "exits", binary)

            with self.assertRaisesRegex(
                smoke.SmokeError, "status 7 before 2.0s"
            ):
                smoke.smoke_example(
                    artifact, 2.0, os.environ.copy(), wrapper=()
                )

    def test_surviving_window_loop_is_terminated_and_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = executable_script(root, "survives", "sleep 30\n")
            artifact = smoke.ExampleArtifact("test", "survives", binary)

            smoke.smoke_example(
                artifact, 0.1, os.environ.copy(), wrapper=()
            )


class WorkflowContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.repository = SCRIPT.parent.parent

    def test_linux_action_installs_headless_software_rendering_stack(self) -> None:
        action = (
            self.repository
            / ".github/actions/install-linux-desktop-build-deps/action.yml"
        ).read_text(encoding="utf-8")
        for package in (
            "libgl1-mesa-dri",
            "mesa-vulkan-drivers",
            "xauth",
            "xvfb",
        ):
            with self.subTest(package=package):
                self.assertIn(package, action)

    def test_ci_builds_before_smoking_every_native_example(self) -> None:
        workflow = (self.repository / ".github/workflows/ci.yml").read_text(
            encoding="utf-8"
        )
        identities = (
            "ruviz-egui:dashboard",
            "ruviz-egui:three_d",
            "ruviz-egui:mixed_dashboard",
            "ruviz-iced:dashboard_2d",
            "ruviz-iced:dashboard",
            "ruviz-iced:static_dashboard",
            "ruviz-slint:dashboard",
            "ruviz-slint:mixed_3d_dashboard",
            "ruviz-gpui:coordinate_events",
            "ruviz-gpui:fixed_bounds_dashboard",
            "ruviz-gpui:movable_annotation",
            "ruviz-gpui:observable_embed",
            "ruviz-gpui:plot3d_embed",
            "ruviz-gpui:static_embed",
            "ruviz-gpui:streaming_embed",
        )
        expected_fragments = (
            "Build all native GUI examples for smoke testing",
            "adapters/gui/Cargo.toml --package ruviz-egui --examples --all-features --locked",
            "adapters/gui/Cargo.toml --package ruviz-iced --examples --all-features --locked",
            "adapters/gui/Cargo.toml --package ruviz-slint --examples --all-features --locked",
            "adapters/gpui/Cargo.toml --examples --all-features --locked",
            "--message-format=json-render-diagnostics",
            "smoke_native_gui_examples.py stage",
            "Smoke all native GUI examples under Xvfb",
            "smoke_native_gui_examples.py run",
            *identities,
        )
        for expected in expected_fragments:
            with self.subTest(expected=expected):
                self.assertIn(expected, workflow)

        build = workflow.index("Build all native GUI examples for smoke testing")
        runtime = workflow.index("Smoke all native GUI examples under Xvfb")
        self.assertLess(build, runtime)
        self.assertEqual(
            workflow.count("python3 scripts/smoke_native_gui_examples.py stage"), 4
        )
        self.assertEqual(
            workflow.count("python3 scripts/smoke_native_gui_examples.py run"), 1
        )

        runtime_block = workflow[runtime : workflow.index("\n  test-fast:", runtime)]
        self.assertNotIn("cargo build", runtime_block)
        for identity in identities:
            with self.subTest(runtime_identity=identity):
                self.assertIn(f"--expect {identity}", runtime_block)


if __name__ == "__main__":
    unittest.main()
