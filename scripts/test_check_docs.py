from __future__ import annotations

import importlib.util
import os
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path
from unittest import mock


SCRIPT = Path(__file__).with_name("check_docs.py")
SPEC = importlib.util.spec_from_file_location("check_docs", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
check_docs = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = check_docs
SPEC.loader.exec_module(check_docs)


def fence(info: str, code: str = "") -> check_docs.CodeFence:
    lang, flags = check_docs.parse_fence_info(info)
    return check_docs.CodeFence(
        path=check_docs.ROOT / "docs" / "test.md",
        line=1,
        info=info,
        lang=lang,
        flags=flags,
        code=code,
    )


def checked_fence_containing(path: Path, text: str) -> check_docs.CodeFence:
    return next(
        item
        for item in check_docs.extract_code_fences([path])
        if item.lang == "rust"
        and ("check" in item.flags or "compile" in item.flags)
        and text in item.code
    )


def documented_manifest_before(snippet: check_docs.CodeFence) -> dict:
    manifest = max(
        (
            item
            for item in check_docs.extract_code_fences([snippet.path])
            if item.lang == "toml" and item.line < snippet.line
        ),
        key=lambda item: item.line,
    )
    return tomllib.loads(manifest.code)


class CheckDocsTests(unittest.TestCase):
    def test_docs_target_defaults_to_persistent_repository_cache(self) -> None:
        environment = dict(os.environ)
        environment.pop("CARGO_TARGET_DIR", None)
        self.assertEqual(
            check_docs.docs_cargo_target_dir(environment),
            check_docs.ROOT / "target",
        )

    def test_docs_target_respects_existing_cargo_target_dir(self) -> None:
        absolute = Path(tempfile.gettempdir()) / "shared-cargo-target"
        self.assertEqual(
            check_docs.docs_cargo_target_dir({"CARGO_TARGET_DIR": str(absolute)}),
            absolute,
        )
        self.assertEqual(
            check_docs.docs_cargo_target_dir({"CARGO_TARGET_DIR": "cached-target"}),
            check_docs.ROOT / "cached-target",
        )

    def test_rust_feature_profile_uses_canonical_plus_separator(self) -> None:
        snippet = fence("rust,check,features=interactive+gpu")
        self.assertEqual(
            check_docs.rust_feature_profile(snippet),
            ("gpu", "interactive"),
        )
        manifest = check_docs.cargo_project_manifest(
            "snippet-test",
            check_docs.rust_feature_profile(snippet),
        )
        self.assertIn('gpu = ["ruviz/gpu"]', manifest)
        self.assertIn('interactive = ["ruviz/interactive"]', manifest)

    def test_polars_snippet_features_do_not_expand_public_dependency(self) -> None:
        root_manifest = tomllib.loads(
            (check_docs.ROOT / "Cargo.toml").read_text(encoding="utf-8")
        )
        snippet_manifest = tomllib.loads(
            check_docs.cargo_project_manifest(
                "snippet-test",
                ("polars_support",),
            )
        )
        self.assertNotIn("features", root_manifest["dependencies"]["polars"])
        self.assertEqual(
            set(snippet_manifest["dependencies"]["polars"]["features"]),
            {"lazy", "rolling_window"},
        )
        self.assertNotIn("rand", snippet_manifest["dependencies"])

    def test_installation_ndarray_manifest_declares_direct_dependency(self) -> None:
        path = check_docs.ROOT / "docs" / "guide" / "02_installation.md"
        snippet = checked_fence_containing(path, "use ndarray::Array1;")
        manifest = documented_manifest_before(snippet)

        self.assertEqual(manifest["dependencies"]["ndarray"], "0.17")
        self.assertEqual(
            manifest["dependencies"]["ruviz"]["features"],
            ["ndarray_support"],
        )

    def test_polars_time_series_uses_its_documented_manifest(self) -> None:
        path = check_docs.ROOT / "docs" / "guide" / "09_data_integration.md"
        snippet = checked_fence_containing(path, "RollingOptionsFixedWindow")
        manifest = documented_manifest_before(snippet)

        self.assertEqual(
            set(manifest["dependencies"]["polars"]["features"]),
            {"lazy", "rolling_window"},
        )
        self.assertNotIn("rand::", snippet.code)
        self.assertNotIn("rand", manifest["dependencies"])

    def test_ignored_fence_requires_non_empty_reason(self) -> None:
        for info in ["rust,ignore", "rust,ignore,reason=", "rust,ignore,reason=   "]:
            with self.subTest(info=info):
                errors = check_docs.check_fence_classification([fence(info)])
                self.assertEqual(len(errors), 1)
                self.assertIn("requires a non-empty reason=...", errors[0])

        self.assertEqual(
            check_docs.check_fence_classification(
                [fence("rust,ignore,reason=illustrative-fragment")]
            ),
            [],
        )

    def test_complete_rust_program_requires_classification(self) -> None:
        errors = check_docs.check_fence_classification(
            [fence("rust", "fn main() {}\n")]
        )
        self.assertEqual(len(errors), 1)
        self.assertIn("must be marked check or ignore", errors[0])

    def test_shell_check_is_syntax_only(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            marker = Path(temp) / "must-not-exist"
            snippet = fence("bash", f"touch {marker}\n")
            self.assertEqual(check_docs.check_shell_snippets([snippet]), [])
            self.assertFalse(marker.exists())

    def test_invalid_shell_syntax_fails(self) -> None:
        errors = check_docs.check_shell_snippets([fence("sh", "if true; then\n")])
        self.assertEqual(len(errors), 1)
        self.assertIn("invalid syntax", errors[0])

    def test_all_example_programs_have_unique_covered_targets(self) -> None:
        self.assertEqual(check_docs.check_example_target_coverage(), [])

        targets = dict(
            (path, name) for name, path in check_docs.cargo_example_targets()
        )
        self.assertEqual(
            targets["examples/memory_optimization_demo.rs"],
            "memory_optimization_demo",
        )
        self.assertEqual(
            targets["examples/parallel_demo.rs"],
            "parallel_demo",
        )
        self.assertEqual(
            targets["gallery/performance/memory_optimization_demo.rs"],
            "gallery_memory_optimization_demo",
        )
        self.assertEqual(
            targets["gallery/performance/parallel_demo.rs"],
            "gallery_parallel_demo",
        )

    def test_duplicate_inferred_and_declared_example_names_are_rejected(self) -> None:
        manifest = {
            "package": {},
            "example": [
                {
                    "name": "parallel_demo",
                    "path": "gallery/performance/parallel_demo.rs",
                }
            ],
        }
        sources = {
            "examples/parallel_demo.rs",
            "gallery/performance/parallel_demo.rs",
        }
        errors = check_docs.check_example_target_coverage(manifest, sources)
        self.assertEqual(len(errors), 1)
        self.assertIn("resolves to multiple sources", errors[0])

    def test_duplicate_example_paths_are_rejected(self) -> None:
        manifest = {
            "package": {"autoexamples": False},
            "example": [
                {"name": "first", "path": "gallery/basic/demo.rs"},
                {"name": "second", "path": "gallery/basic/demo.rs"},
            ],
        }
        errors = check_docs.check_example_target_coverage(
            manifest, {"gallery/basic/demo.rs"}
        )
        self.assertEqual(len(errors), 1)
        self.assertIn("registered by multiple targets", errors[0])

    def test_uncovered_example_path_is_rejected(self) -> None:
        errors = check_docs.check_example_target_coverage(
            {"package": {"autoexamples": False}},
            {"gallery/basic/unregistered.rs"},
        )
        self.assertEqual(len(errors), 1)
        self.assertIn("not covered by a Cargo example target", errors[0])

    def test_cpu_only_gpu_memory_example_is_not_feature_gated(self) -> None:
        manifest = tomllib.loads(
            (check_docs.ROOT / "Cargo.toml").read_text(encoding="utf-8")
        )
        example = next(
            target
            for target in manifest["example"]
            if target.get("path") == "examples/gpu_memory_test.rs"
        )
        self.assertNotIn("required-features", example)

    def test_checked_font_asset_is_staged_relative_to_main_source(self) -> None:
        snippet = fence(
            "rust,check,asset=../assets/dejavu-sans.ttf",
            'fn main() { let _ = include_bytes!("../assets/dejavu-sans.ttf"); }\n',
        )

        def verify_staged_asset(command: list[str], **_kwargs: object):
            manifest = Path(command[command.index("--manifest-path") + 1])
            parsed = tomllib.loads(manifest.read_text(encoding="utf-8"))
            source = manifest.parent / parsed["bin"][0]["path"]
            asset = (source.parent / "../assets/dejavu-sans.ttf").resolve()
            self.assertEqual(
                asset.read_bytes(),
                (check_docs.ROOT / "src/dejavu-sans.ttf").read_bytes(),
            )
            return check_docs.subprocess.CompletedProcess(command, 0, "", "")

        with tempfile.TemporaryDirectory() as temp:
            with mock.patch.object(
                check_docs.subprocess, "run", side_effect=verify_staged_asset
            ):
                self.assertEqual(
                    check_docs.check_rust_snippets([snippet], Path(temp)), []
                )

    def test_rust_feature_profiles_share_only_the_cargo_target(self) -> None:
        snippets = [
            fence("rust,check", "fn main() {}\n"),
            fence("rust,check,features=gpu", "fn main() {}\n"),
        ]
        completed = check_docs.subprocess.CompletedProcess([], 0, "", "")

        with tempfile.TemporaryDirectory() as temp:
            cargo_target = Path(temp) / "shared-target"
            with mock.patch.object(
                check_docs.subprocess, "run", return_value=completed
            ) as run:
                self.assertEqual(
                    check_docs.check_rust_snippets(snippets, cargo_target), []
                )

            self.assertEqual(run.call_count, 2)
            manifests: list[Path] = []
            for call in run.call_args_list:
                command = call.args[0]
                manifests.append(Path(command[command.index("--manifest-path") + 1]))
                self.assertEqual(
                    call.kwargs["env"]["CARGO_TARGET_DIR"],
                    str(cargo_target.resolve()),
                )

            self.assertNotEqual(manifests[0].parent, manifests[1].parent)
            self.assertTrue(all(not manifest.parent.exists() for manifest in manifests))

    def test_cargo_target_survives_isolated_snippet_project_cleanup(self) -> None:
        snippet = fence("rust,check", "fn main() {}\n")
        completed = check_docs.subprocess.CompletedProcess([], 0, "", "")

        with tempfile.TemporaryDirectory() as temp:
            cargo_target = Path(temp) / "persistent-target"
            cargo_target.mkdir()
            marker = cargo_target / "cached-artifact"
            marker.write_text("keep", encoding="utf-8")
            with mock.patch.object(
                check_docs.subprocess, "run", return_value=completed
            ) as run:
                self.assertEqual(
                    check_docs.check_rust_snippets([snippet], cargo_target), []
                )

            command = run.call_args.args[0]
            manifest = Path(command[command.index("--manifest-path") + 1])
            self.assertFalse(manifest.parent.exists())
            self.assertEqual(marker.read_text(encoding="utf-8"), "keep")


if __name__ == "__main__":
    unittest.main()
