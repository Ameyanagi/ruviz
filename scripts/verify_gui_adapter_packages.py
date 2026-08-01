#!/usr/bin/env python3
"""Verify packaged native GUI adapters from a fresh Cargo consumer.

The GUI adapters intentionally form a workspace that is separate from the
repository root. This verifier keeps that boundary honest, checks Cargo's
normalized package manifests, and compiles a consumer from unpacked `.crate`
archives. The Slint consumer imports `RuvizPlot` through the packaged
`@Ruviz` external module, so omitting the Slint source or Cargo `links`
metadata is a hard failure.
"""

from __future__ import annotations

import argparse
import io
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time
import tomllib
from dataclasses import dataclass
from pathlib import Path, PurePosixPath, PureWindowsPath
from typing import Any


ADAPTER_WORKSPACE = Path("adapters/gui")
ADAPTERS = ("ruviz-egui", "ruviz-iced", "ruviz-slint")
MSRV = "1.92"
# Cargo validates a path dependency's version against crates.io even with
# `cargo package --no-verify`. CI runs before the next core release exists, so
# it stages the dependency as the latest published compatible core solely
# while Cargo creates the archive, then restores the exact next-release
# requirement in the archive. Release mode never uses this escape hatch.
CI_PUBLISHED_CORE_VERSION = "0.5.0"


class VerificationError(RuntimeError):
    """A GUI adapter packaging invariant was not satisfied."""


@dataclass(frozen=True)
class Package:
    name: str
    version: str
    archive: Path
    root: Path
    manifest: dict[str, Any]


def load_toml(path: Path) -> dict[str, Any]:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def run(
    command: list[str],
    *,
    cwd: Path,
    env: dict[str, str] | None = None,
    check: bool = True,
) -> subprocess.CompletedProcess[str]:
    print("+", " ".join(command), f"(in {cwd})", flush=True)
    return subprocess.run(
        command,
        cwd=cwd,
        env=env,
        check=check,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )


def inspect_workspace(repository: Path) -> tuple[str, dict[str, Path]]:
    root_manifest = load_toml(repository / "Cargo.toml")
    workspace_path = repository / ADAPTER_WORKSPACE
    workspace_manifest = load_toml(workspace_path / "Cargo.toml")
    root_workspace = root_manifest.get("workspace", {})
    if str(ADAPTER_WORKSPACE) not in root_workspace.get("exclude", []):
        raise VerificationError(
            f"the root workspace must exclude {ADAPTER_WORKSPACE}"
        )

    workspace = workspace_manifest.get("workspace", {})
    members = workspace.get("members")
    expected_members = [name for name in ADAPTERS]
    if members != expected_members:
        raise VerificationError(
            f"{ADAPTER_WORKSPACE} members must be {expected_members!r}, got {members!r}"
        )

    package_defaults = workspace.get("package", {})
    version = package_defaults.get("version")
    if not isinstance(version, str):
        raise VerificationError("GUI adapter workspace version is missing")
    if package_defaults.get("rust-version") != MSRV:
        raise VerificationError(f"GUI adapters must declare Rust {MSRV} as the MSRV")
    if root_manifest.get("package", {}).get("version") != version:
        raise VerificationError("core and GUI adapter workspace versions must match")

    lockfile = workspace_path / "Cargo.lock"
    if not lockfile.is_file():
        raise VerificationError(f"isolated adapter lockfile is missing: {lockfile}")
    lock = load_toml(lockfile)
    git_sources = sorted(
        str(package.get("source"))
        for package in lock.get("package", [])
        if isinstance(package, dict)
        and str(package.get("source", "")).startswith("git+")
    )
    if git_sources:
        raise VerificationError(
            "GUI adapter lockfile must resolve registry/path dependencies only: "
            + ", ".join(git_sources[:5])
        )

    manifests: dict[str, Path] = {}
    for name in ADAPTERS:
        manifest_path = workspace_path / name / "Cargo.toml"
        manifest = load_toml(manifest_path)
        package = manifest.get("package", {})
        if package.get("name") != name:
            raise VerificationError(f"{manifest_path} must package {name}")
        dependency = manifest.get("dependencies", {}).get("ruviz")
        if not isinstance(dependency, dict):
            raise VerificationError(f"{name} must use a detailed ruviz dependency")
        if dependency.get("version") != f"={version}":
            raise VerificationError(
                f"{name} must require the exact core version ={version}"
            )
        if dependency.get("path") != "../../..":
            raise VerificationError(
                f"{name} must use the repository root as its local ruviz path"
            )
        manifests[name] = manifest_path

    return version, manifests


def package_archive(
    repository: Path,
    *,
    name: str,
    manifest: Path,
    version: str,
    target_dir: Path,
    verify: bool,
    attempts: int,
    delay: float,
    ci_dependency_version: str | None = None,
) -> Path:
    command = [
        "cargo",
        "package",
        "--manifest-path",
        str(manifest),
        "--allow-dirty",
        "--target-dir",
        str(target_dir),
    ]
    command.append("--locked" if verify else "--exclude-lockfile")
    if not verify:
        command.append("--no-verify")

    output = ""
    for attempt in range(1, attempts + 1):
        result = run(command, cwd=repository, check=False)
        output = result.stdout
        if result.returncode == 0:
            break
        if attempt == attempts:
            raise VerificationError(
                f"cargo package failed for {name} after {attempts} attempt(s):\n{output}"
            )
        print(
            f"waiting for registry propagation before retrying {name} "
            f"({attempt}/{attempts})",
            flush=True,
        )
        time.sleep(delay)

    archive = target_dir / "package" / f"{name}-{version}.crate"
    if not archive.is_file():
        raise VerificationError(f"cargo package did not create {archive}")
    if ci_dependency_version is not None:
        rewrite_ci_dependency_version(
            archive,
            published_version=ci_dependency_version,
            release_version=version,
        )
    return archive.resolve()


def rewrite_ci_dependency_version(
    archive: Path, *, published_version: str, release_version: str
) -> None:
    """Restore the next release's exact core requirement in a staged archive."""

    replacement = archive.with_suffix(".crate.rewritten")
    old = f'version = "={published_version}"'
    new = f'version = "={release_version}"'
    replaced = 0
    with tarfile.open(archive, "r:gz") as source, tarfile.open(
        replacement, "w:gz"
    ) as destination:
        for member in source.getmembers():
            extracted = source.extractfile(member) if member.isfile() else None
            payload = extracted.read() if extracted is not None else None
            if payload is not None and member.name.endswith(
                ("/Cargo.toml", "/Cargo.toml.orig")
            ):
                text = payload.decode("utf-8")
                count = text.count(old)
                if count:
                    text = text.replace(old, new)
                    payload = text.encode("utf-8")
                    member.size = len(payload)
                    replaced += count
            destination.addfile(
                member,
                None if payload is None else io.BytesIO(payload),
            )
    if replaced < 1:
        replacement.unlink(missing_ok=True)
        raise VerificationError(
            f"{archive.name} did not contain the staged exact ruviz requirement"
        )
    replacement.replace(archive)


def prepare_ci_staging(
    repository: Path, temporary: Path, core: Package
) -> tuple[Path, dict[str, Path]]:
    """Create a package-only workspace resolvable before the core is published.

    Before a release, the next ruviz version is intentionally absent from the
    registry. Cargo still resolves registry dependencies while creating a
    package with `--no-verify`, so the staged path dependency temporarily uses
    the latest published compatible version. The archive is restored to the
    next exact version before its normalized manifest and consumer are checked.
    """

    staged_repository = temporary / "staged-repository"
    shutil.copytree(core.root, staged_repository)
    staged_workspace = staged_repository / ADAPTER_WORKSPACE
    shutil.copytree(
        repository / ADAPTER_WORKSPACE,
        staged_workspace,
        ignore=shutil.ignore_patterns("target"),
    )
    core_manifest = staged_repository / "Cargo.toml"
    core_text = core_manifest.read_text(encoding="utf-8")
    old_core_version = f'version = "{core.version}"'
    if old_core_version not in core_text:
        raise VerificationError("cannot stage the core package version for CI")
    core_manifest.write_text(
        core_text.replace(
            old_core_version,
            f'version = "{CI_PUBLISHED_CORE_VERSION}"',
            1,
        ),
        encoding="utf-8",
    )

    for name in ADAPTERS:
        manifest = staged_workspace / name / "Cargo.toml"
        text = manifest.read_text(encoding="utf-8")
        old_dependency = f'version = "={core.version}"'
        if old_dependency not in text:
            raise VerificationError(
                f"cannot stage {name}'s core dependency version for CI"
            )
        manifest.write_text(
            text.replace(
                old_dependency,
                f'version = "={CI_PUBLISHED_CORE_VERSION}"',
                1,
            ),
            encoding="utf-8",
        )
    manifests = {
        name: staged_workspace / name / "Cargo.toml" for name in ADAPTERS
    }
    return staged_repository, manifests


def safe_extract(
    archive: Path, destination: Path, *, name: str, version: str
) -> Package:
    expected_root = f"{name}-{version}"
    destination.mkdir(parents=True, exist_ok=True)
    with tarfile.open(archive, "r:gz") as source:
        members = source.getmembers()
        for member in members:
            posix = PurePosixPath(member.name)
            windows = PureWindowsPath(member.name)
            if (
                posix.is_absolute()
                or windows.is_absolute()
                or ".." in posix.parts
                or ".." in windows.parts
                or not posix.parts
                or posix.parts[0] != expected_root
                or member.issym()
                or member.islnk()
            ):
                raise VerificationError(
                    f"{archive.name} contains unsafe or unexpected member {member.name!r}"
                )
        source.extractall(destination, filter="data")

    root = destination / expected_root
    manifest_path = root / "Cargo.toml"
    if not manifest_path.is_file():
        raise VerificationError(f"{archive.name} has no normalized Cargo.toml")
    manifest = load_toml(manifest_path)
    package = manifest.get("package", {})
    if package.get("name") != name or package.get("version") != version:
        raise VerificationError(
            f"{archive.name} normalized manifest does not describe {name} {version}"
        )
    return Package(name, version, archive, root.resolve(), manifest)


def validate_normalized_adapter(package: Package, version: str) -> None:
    dependency = package.manifest.get("dependencies", {}).get("ruviz")
    if not isinstance(dependency, dict):
        raise VerificationError(
            f"{package.name} normalized ruviz dependency must be a table"
        )
    if dependency.get("version") != f"={version}":
        raise VerificationError(
            f"{package.name} normalized ruviz dependency is not ={version}"
        )
    forbidden = sorted(key for key in ("path", "git", "registry") if key in dependency)
    if forbidden:
        raise VerificationError(
            f"{package.name} normalized dependency retained {', '.join(forbidden)}"
        )

    missing_licenses = [
        name
        for name in ("LICENSE-MIT", "LICENSE-APACHE")
        if not (package.root / name).is_file()
    ]
    if missing_licenses:
        raise VerificationError(
            f"{package.name} archive omitted dual-license files: "
            + ", ".join(missing_licenses)
        )

    if package.name == "ruviz-slint":
        required = ("build.rs", "src/lib.rs", "ui/ruviz.slint")
        missing = [relative for relative in required if not (package.root / relative).is_file()]
        if missing:
            raise VerificationError(
                "ruviz-slint archive omitted required module files: "
                + ", ".join(missing)
            )
        if package.manifest.get("package", {}).get("links") != "Ruviz":
            raise VerificationError(
                "ruviz-slint must retain links = \"Ruviz\" in its package"
            )
        slint_build = package.manifest.get("build-dependencies", {}).get("slint-build")
        features = slint_build.get("features", []) if isinstance(slint_build, dict) else []
        if "experimental-module-builds" not in features:
            raise VerificationError(
                "ruviz-slint must enable Slint external-module build metadata"
            )


def require_release_vcs(package: Package, expected_sha: str) -> None:
    metadata_path = package.root / ".cargo_vcs_info.json"
    if not metadata_path.is_file():
        raise VerificationError(
            f"{package.name} archive has no .cargo_vcs_info.json"
        )
    try:
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise VerificationError(
            f"cannot read {package.name} VCS metadata: {error}"
        ) from error
    git = metadata.get("git", {})
    if git.get("sha1") != expected_sha or git.get("dirty", False):
        raise VerificationError(
            f"{package.name} archive is not from clean release SHA {expected_sha}"
        )


def toml_path(path: Path) -> str:
    return '"' + str(path).replace("\\", "\\\\").replace('"', '\\"') + '"'


def write_consumer(consumer: Path, core: Package, adapters: list[Package]) -> None:
    package_paths = {package.name: package.root for package in adapters}
    manifest = f"""[package]
name = "ruviz-gui-adapter-package-consumer"
version = "0.0.0"
edition = "2024"
rust-version = "{MSRV}"
build = "build.rs"

[dependencies]
ruviz = {{ path = {toml_path(core.root)}, default-features = false, features = ["3d", "gpu"] }}
ruviz-egui = {{ path = {toml_path(package_paths["ruviz-egui"])}, features = ["3d-gpu"] }}
ruviz-iced = {{ path = {toml_path(package_paths["ruviz-iced"])}, features = ["3d-gpu"] }}
ruviz-slint = {{ path = {toml_path(package_paths["ruviz-slint"])}, features = ["3d-gpu"] }}
slint = {{ version = "~1.17", default-features = false, features = ["std", "compat-1-2"] }}

[build-dependencies]
slint-build = {{ version = "~1.17", features = ["experimental-module-builds"] }}

[patch.crates-io]
ruviz = {{ path = {toml_path(core.root)} }}
"""
    (consumer / "Cargo.toml").write_text(manifest, encoding="utf-8")
    (consumer / "build.rs").write_text(
        """fn main() {
    slint_build::compile("ui/main.slint")
        .expect("the packaged @Ruviz Slint module must be importable");
}
""",
        encoding="utf-8",
    )
    (consumer / "ui").mkdir()
    (consumer / "ui/main.slint").write_text(
        """import { RuvizPlot } from "@Ruviz";

export component PackageConsumer inherits Window {
    RuvizPlot {
        slot-id: 7;
    }
}
""",
        encoding="utf-8",
    )
    (consumer / "src").mkdir()
    (consumer / "src/main.rs").write_text(
        """slint::include_modules!();

fn main() {
    let _ = core::mem::size_of::<ruviz_egui::RuvizPlot>();
    let _ = core::mem::size_of::<ruviz_egui::RuvizPlot3D>();
    let _ = core::mem::size_of::<ruviz_iced::PlotState>();
    let _ = core::mem::size_of::<ruviz_iced::Plot3DState>();
    let _ = core::mem::size_of::<ruviz_slint::RuvizController>();
    let _ = PackageConsumer::new();
}
""",
        encoding="utf-8",
    )


def verify(args: argparse.Namespace) -> None:
    repository = Path(args.repository).resolve()
    version, manifests = inspect_workspace(repository)

    with tempfile.TemporaryDirectory(prefix="ruviz-gui-packages-") as temporary:
        temp = Path(temporary)
        target = temp / "package-target"
        extracted = temp / "extracted"

        core_archive = package_archive(
            repository,
            name="ruviz",
            manifest=Path("Cargo.toml"),
            version=version,
            target_dir=target / "ruviz",
            verify=args.mode == "release",
            attempts=args.registry_attempts,
            delay=args.registry_delay,
        )
        core = safe_extract(
            core_archive, extracted, name="ruviz", version=version
        )
        if args.expected_vcs_sha is not None:
            require_release_vcs(core, args.expected_vcs_sha)

        package_repository = repository
        package_manifests = manifests
        if args.mode == "ci":
            package_repository, package_manifests = prepare_ci_staging(
                repository, temp, core
            )

        adapters: list[Package] = []
        for name in ADAPTERS:
            archive = package_archive(
                package_repository,
                name=name,
                manifest=package_manifests[name],
                version=version,
                target_dir=target / name,
                verify=args.mode == "release",
                attempts=args.registry_attempts,
                delay=args.registry_delay,
                ci_dependency_version=(
                    CI_PUBLISHED_CORE_VERSION if args.mode == "ci" else None
                ),
            )
            package = safe_extract(
                archive, extracted, name=name, version=version
            )
            validate_normalized_adapter(package, version)
            if args.expected_vcs_sha is not None:
                require_release_vcs(package, args.expected_vcs_sha)
            adapters.append(package)

        consumer = temp / "consumer"
        consumer.mkdir()
        write_consumer(consumer, core, adapters)
        cargo_env = os.environ.copy()
        cargo_env["CARGO_TARGET_DIR"] = str(temp / "consumer-target")
        run(["cargo", "generate-lockfile"], cwd=consumer, env=cargo_env)
        run(["cargo", "check", "--locked"], cwd=consumer, env=cargo_env)

    print(
        f"Verified packaged ruviz GUI adapters {version} and the Slint @Ruviz import.",
        flush=True,
    )


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", default=".")
    parser.add_argument("--mode", choices=("ci", "release"), default="ci")
    parser.add_argument("--registry-attempts", type=int, default=1)
    parser.add_argument("--registry-delay", type=float, default=20.0)
    parser.add_argument("--expected-vcs-sha")
    args = parser.parse_args(argv)
    if args.registry_attempts < 1:
        parser.error("--registry-attempts must be positive")
    if args.registry_delay < 0:
        parser.error("--registry-delay must not be negative")
    if args.mode == "release" and args.expected_vcs_sha is None:
        parser.error("--expected-vcs-sha is required in release mode")
    return args


def main(argv: list[str] | None = None) -> int:
    try:
        verify(parse_args(argv))
    except (OSError, subprocess.CalledProcessError, VerificationError) as error:
        print(f"error: {error}", file=sys.stderr)
        if isinstance(error, subprocess.CalledProcessError) and error.stdout:
            print(error.stdout, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
