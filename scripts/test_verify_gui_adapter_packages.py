from __future__ import annotations

import importlib.util
import io
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("verify_gui_adapter_packages.py")
SPEC = importlib.util.spec_from_file_location("verify_gui_adapter_packages", SCRIPT)
assert SPEC is not None
assert SPEC.loader is not None
verifier = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = verifier
SPEC.loader.exec_module(verifier)


def add_file(archive: tarfile.TarFile, name: str, contents: str) -> None:
    payload = contents.encode("utf-8")
    member = tarfile.TarInfo(name)
    member.size = len(payload)
    archive.addfile(member, io.BytesIO(payload))


class ArchiveTests(unittest.TestCase):
    def test_ci_rewrite_restores_exact_release_dependency(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            archive_path = Path(temporary) / "ruviz-egui-0.6.0.crate"
            with tarfile.open(archive_path, "w:gz") as archive:
                add_file(
                    archive,
                    "ruviz-egui-0.6.0/Cargo.toml",
                    '[dependencies.ruviz]\nversion = "=0.5.0"\n',
                )

            verifier.rewrite_ci_dependency_version(
                archive_path,
                published_version="0.5.0",
                release_version="0.6.0",
            )

            with tarfile.open(archive_path, "r:gz") as archive:
                member = archive.extractfile("ruviz-egui-0.6.0/Cargo.toml")
                assert member is not None
                contents = member.read().decode("utf-8")
            self.assertIn('version = "=0.6.0"', contents)
            self.assertNotIn('version = "=0.5.0"', contents)

    def test_ci_rewrite_rejects_an_archive_without_staged_dependency(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            archive_path = Path(temporary) / "ruviz-egui-0.6.0.crate"
            with tarfile.open(archive_path, "w:gz") as archive:
                add_file(
                    archive,
                    "ruviz-egui-0.6.0/Cargo.toml",
                    '[package]\nname = "ruviz-egui"\nversion = "0.6.0"\n',
                )

            with self.assertRaisesRegex(
                verifier.VerificationError, "staged exact ruviz requirement"
            ):
                verifier.rewrite_ci_dependency_version(
                    archive_path,
                    published_version="0.5.0",
                    release_version="0.6.0",
                )

    def test_safe_extract_rejects_parent_traversal(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            archive_path = directory / "ruviz-egui-0.6.0.crate"
            with tarfile.open(archive_path, "w:gz") as archive:
                add_file(archive, "../outside", "bad")

            with self.assertRaisesRegex(
                verifier.VerificationError, "unsafe or unexpected member"
            ):
                verifier.safe_extract(
                    archive_path,
                    directory / "out",
                    name="ruviz-egui",
                    version="0.6.0",
                )

    def test_release_vcs_rejects_a_dirty_archive(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / ".cargo_vcs_info.json").write_text(
                '{"git":{"sha1":"' + "a" * 40 + '","dirty":true}}',
                encoding="utf-8",
            )
            package = verifier.Package(
                "ruviz-egui",
                "0.6.0",
                Path("archive.crate"),
                root,
                {},
            )
            with self.assertRaisesRegex(
                verifier.VerificationError, "clean release SHA"
            ):
                verifier.require_release_vcs(package, "a" * 40)


class WorkflowContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.repository = SCRIPT.parent.parent

    def test_ci_covers_three_native_platforms_and_feature_surfaces(self) -> None:
        workflow = (self.repository / ".github/workflows/ci.yml").read_text(
            encoding="utf-8"
        )
        for expected in (
            "name: Native GUI Adapters",
            "name: Native GUI Behavioral Tests (${{ matrix.platform.name }})",
            "os: ubuntu-latest",
            "os: macos-14",
            "os: windows-latest",
            "cache_key: linux",
            "cache_key: macos",
            "cache_key: windows",
            "if: runner.os == 'Linux'",
            "--features 3d --locked",
            "--all-features --locked",
            "--examples --all-features --locked",
            "cargo doc --manifest-path adapters/gui/Cargo.toml",
            "Native GUI Adapter MSRV (1.92)",
            "GPUI MSRV (1.92)",
            "hashFiles('adapters/gui/Cargo.lock')",
            "hashFiles('adapters/gpui/Cargo.lock')",
            "cargo clippy --manifest-path adapters/gui/Cargo.toml --workspace --all-targets --locked",
            "cargo clippy --manifest-path adapters/gpui/Cargo.toml --all-targets --locked",
            "cargo test --manifest-path adapters/gui/Cargo.toml --workspace --lib --all-features --locked",
            "cargo test --manifest-path adapters/gpui/Cargo.toml --lib --all-features --locked",
            "cargo test --manifest-path adapters/gui/Cargo.toml --workspace --doc --all-features --locked",
            "cargo test --manifest-path adapters/gpui/Cargo.toml --doc --all-features --locked",
            "cargo check --manifest-path adapters/gpui/Cargo.toml --examples --all-features --locked",
            "cargo check --manifest-path adapters/gpui/Cargo.toml --target ${{ matrix.platform.target }} --example plot3d_embed --all-features --locked",
            "cargo doc --manifest-path adapters/gpui/Cargo.toml --all-features --no-deps --locked",
            "Run direct 3D GPU suite with a required adapter",
            "BackgroundRenderer3D::GpuReadback",
        ):
            with self.subTest(expected=expected):
                self.assertIn(expected, workflow)

    def test_release_orders_core_before_adapter_verification_and_publish(self) -> None:
        workflow = (self.repository / ".github/workflows/release.yml").read_text(
            encoding="utf-8"
        )
        self.assertIn(
            "needs: [check-ci, build-release, publish-ruviz]",
            workflow,
        )
        self.assertIn(
            "needs: [check-ci, verify-gui-adapter-packages]",
            workflow,
        )
        self.assertIn(
            "matrix:\n        crate: [ruviz-egui, ruviz-iced, ruviz-slint]",
            workflow,
        )
        self.assertIn("needs.publish-gui-adapters.result == 'success'", workflow)
        self.assertIn("softprops/action-gh-release@v2", workflow)
        self.assertIn(
            "cargo check --manifest-path adapters/gpui/Cargo.toml --example plot3d_embed --all-features --locked",
            workflow,
        )
        self.assertIn("Test and build ruviz-gpui documentation", workflow)
        self.assertIn(
            "cargo test --manifest-path adapters/gpui/Cargo.toml --doc --all-features --locked",
            workflow,
        )

    def test_packaged_adapter_jobs_install_native_linux_dependencies(self) -> None:
        dependency_step = (
            "- name: Install Linux desktop build dependencies\n"
            "        uses: ./.github/actions/install-linux-desktop-build-deps"
        )

        ci = (self.repository / ".github/workflows/ci.yml").read_text(
            encoding="utf-8"
        )
        ci_job = ci.split("\n  gui-adapter-packages:", maxsplit=1)[1].split(
            "\n  native-gui-tests:", maxsplit=1
        )[0]
        self.assertIn(dependency_step, ci_job)

        release = (self.repository / ".github/workflows/release.yml").read_text(
            encoding="utf-8"
        )
        release_job = release.split(
            "\n  verify-gui-adapter-packages:", maxsplit=1
        )[1].split("\n  publish-gui-adapters:", maxsplit=1)[0]
        self.assertIn(dependency_step, release_job)

        publish_job = release.split(
            "\n  publish-gui-adapters:", maxsplit=1
        )[1].split("\n  publish-ruviz-web:", maxsplit=1)[0]
        self.assertIn("- name: Install Linux desktop build dependencies", publish_job)
        self.assertIn(
            "uses: ./.github/actions/install-linux-desktop-build-deps",
            publish_job,
        )
        self.assertIn(
            "if: steps.adapter-version.outputs.published != 'true'",
            publish_job,
        )

    def test_pages_build_includes_adapter_and_gpui_rustdoc(self) -> None:
        workflow = (self.repository / ".github/workflows/docs.yml").read_text(
            encoding="utf-8"
        )
        self.assertIn("Build native GUI adapter documentation", workflow)
        self.assertIn("Check GPUI documentation examples", workflow)
        self.assertIn("Build GPUI documentation", workflow)
        self.assertIn(
            "cargo test --manifest-path adapters/gpui/Cargo.toml --doc --all-features --locked",
            workflow,
        )
        self.assertIn(
            "cargo doc --manifest-path adapters/gpui/Cargo.toml --all-features --no-deps --locked",
            workflow,
        )
        self.assertIn("CARGO_TARGET_DIR: ${{ github.workspace }}/target", workflow)


if __name__ == "__main__":
    unittest.main()
