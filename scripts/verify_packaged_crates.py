#!/usr/bin/env python3
"""Verify packaged ruviz crates from a fresh external Cargo consumer."""

from __future__ import annotations

import argparse
import json
import os
import re
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


LEGACY_CRATES_IO_SOURCE = "registry+https://github.com/rust-lang/crates.io-index"
SPARSE_CRATES_IO_SOURCE = "registry+sparse+https://index.crates.io/"
SUPPORTED_GPUI_TARGETS = {
    'cfg(target_os = "linux")',
    'cfg(target_os = "macos")',
    'cfg(target_os = "windows")',
}
WORKSPACE_ONLY_DEV_DEPENDENCIES = {"gpui_macos", "gpui_platform"}


class VerificationError(RuntimeError):
    """A packaged-crate invariant was not satisfied."""


@dataclass(frozen=True)
class WorkspaceContract:
    version: str
    gpui_version: str
    gpui_patch_git: str
    gpui_patch_rev: str
    workspace_only_dev_dependencies: frozenset[str]


@dataclass(frozen=True)
class ExtractedCrate:
    name: str
    version: str
    archive: Path
    root: Path
    manifest: dict[str, Any]
    vcs_sha: str | None
    vcs_dirty: bool | None = None


def load_toml(path: Path) -> dict[str, Any]:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def is_crates_io_source(source: Any) -> bool:
    """Accept Cargo's canonical git and sparse spellings for crates.io."""
    if not isinstance(source, str):
        return False
    return source.rstrip("/") in {
        LEGACY_CRATES_IO_SOURCE.rstrip("/"),
        SPARSE_CRATES_IO_SOURCE.rstrip("/"),
        "sparse+https://index.crates.io",
        "registry+https://index.crates.io",
    }


def dependency_table(manifest: dict[str, Any], table: str, name: str) -> Any:
    return manifest.get(table, {}).get(name)


def require_registry_dependency(dependency: Any, *, name: str, version: str) -> None:
    if not isinstance(dependency, dict):
        raise VerificationError(f"{name} must be a detailed dependency table")
    if dependency.get("version") != version:
        raise VerificationError(
            f"{name} must require version {version}, got {dependency.get('version')!r}"
        )
    forbidden = sorted(key for key in ("git", "path", "registry") if key in dependency)
    if forbidden:
        raise VerificationError(
            f"{name} must resolve from crates.io; found {', '.join(forbidden)}"
        )


def iter_dependency_tables(
    manifest: dict[str, Any],
) -> list[tuple[str, dict[str, Any]]]:
    tables: list[tuple[str, dict[str, Any]]] = []
    for table_name in ("dependencies", "dev-dependencies", "build-dependencies"):
        table = manifest.get(table_name, {})
        if isinstance(table, dict):
            tables.append((table_name, table))

    for target, target_manifest in manifest.get("target", {}).items():
        if not isinstance(target_manifest, dict):
            continue
        for table_name in ("dependencies", "dev-dependencies", "build-dependencies"):
            table = target_manifest.get(table_name, {})
            if isinstance(table, dict):
                tables.append((f"target.{target}.{table_name}", table))
    return tables


def validate_active_dependency_sources(
    crate: ExtractedCrate,
    workspace_only_dev_dependencies: frozenset[str] = frozenset(
        WORKSPACE_ONLY_DEV_DEPENDENCIES
    ),
) -> None:
    for table_name, dependencies in iter_dependency_tables(crate.manifest):
        for name, dependency in dependencies.items():
            if name in workspace_only_dev_dependencies:
                raise VerificationError(
                    f"normalized {crate.name} manifest leaked workspace-only "
                    f"dependency {name} in {table_name}"
                )
            if not isinstance(dependency, dict):
                continue
            forbidden = sorted(
                key for key in ("git", "path", "registry") if key in dependency
            )
            if forbidden:
                raise VerificationError(
                    f"normalized {crate.name} dependency {name} in {table_name} "
                    f"retains non-crates.io source keys: {', '.join(forbidden)}"
                )


def inspect_workspace(workspace: Path) -> WorkspaceContract:
    root_manifest = load_toml(workspace / "Cargo.toml")
    gpui_manifest = load_toml(workspace / "crates/ruviz-gpui/Cargo.toml")

    members = root_manifest.get("workspace", {}).get("members", [])
    if "crates/ruviz-gpui" not in members:
        raise VerificationError("ruviz-gpui is not a workspace member")

    root_package = root_manifest.get("package", {})
    gpui_package = gpui_manifest.get("package", {})
    if root_package.get("name") != "ruviz":
        raise VerificationError("the root package must be named ruviz")
    if gpui_package.get("name") != "ruviz-gpui":
        raise VerificationError("the GPUI package must be named ruviz-gpui")

    version = root_package.get("version")
    if not isinstance(version, str) or gpui_package.get("version") != version:
        raise VerificationError("ruviz and ruviz-gpui versions must match")

    local_ruviz = dependency_table(gpui_manifest, "dependencies", "ruviz")
    if not isinstance(local_ruviz, dict):
        raise VerificationError("ruviz-gpui must declare ruviz with version and path")
    if local_ruviz.get("version") != version or local_ruviz.get("path") != "../..":
        raise VerificationError(
            "ruviz-gpui must use the matching ruviz version and workspace path"
        )

    patch = root_manifest.get("patch", {}).get("crates-io", {}).get("gpui")
    if not isinstance(patch, dict) or not patch.get("git") or not patch.get("rev"):
        raise VerificationError(
            "the workspace GPUI override must be a pinned git patch"
        )

    gpui_versions: set[str] = set()
    gpui_targets: set[str] = set()
    for target, target_manifest in gpui_manifest.get("target", {}).items():
        dependency = target_manifest.get("dependencies", {}).get("gpui", {})
        if dependency:
            gpui_targets.add(target)
            if isinstance(dependency, str):
                gpui_versions.add(dependency)
            elif isinstance(dependency, dict) and isinstance(
                dependency.get("version"), str
            ):
                if any(key in dependency for key in ("git", "path", "registry")):
                    raise VerificationError(
                        f"ruviz-gpui target {target} must declare GPUI from crates.io"
                    )
                gpui_versions.add(dependency["version"])
            else:
                raise VerificationError(f"invalid GPUI dependency for target {target}")

    if gpui_targets != SUPPORTED_GPUI_TARGETS or len(gpui_versions) != 1:
        raise VerificationError(
            "ruviz-gpui must use one GPUI version on Linux, macOS, and Windows"
        )

    workspace_only_dev_dependencies = {
        name
        for table_name, dependencies in iter_dependency_tables(gpui_manifest)
        if table_name.endswith("dev-dependencies")
        for name, dependency in dependencies.items()
        if isinstance(dependency, dict)
        and any(key in dependency for key in ("git", "path"))
    }
    return WorkspaceContract(
        version=version,
        gpui_version=gpui_versions.pop(),
        gpui_patch_git=str(patch["git"]),
        gpui_patch_rev=str(patch["rev"]),
        workspace_only_dev_dependencies=frozenset(workspace_only_dev_dependencies),
    )


def exact_archive_path(directory: Path, name: str, version: str) -> Path:
    expected = directory / f"{name}-{version}.crate"
    if not expected.is_file():
        available = ", ".join(path.name for path in sorted(directory.glob("*.crate")))
        detail = f"; found: {available}" if available else ""
        raise VerificationError(f"expected archive {expected}{detail}")
    return expected.resolve()


def resolve_archive_argument(path: Path, name: str, version: str) -> Path:
    path = path.expanduser().resolve()
    if path.is_dir():
        return exact_archive_path(path, name, version)
    if not path.is_file():
        raise VerificationError(f"archive does not exist: {path}")
    return path


def run_command(
    command: list[str],
    *,
    cwd: Path,
    env: dict[str, str] | None = None,
    capture: bool = False,
    check: bool = True,
) -> subprocess.CompletedProcess[str]:
    print(f"+ {' '.join(command)}", flush=True)
    result = subprocess.run(
        command,
        cwd=cwd,
        env=env,
        check=False,
        text=True,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.PIPE if capture else None,
    )
    if check and result.returncode != 0:
        detail = ""
        if capture and result.stderr:
            detail = f"\n{result.stderr.strip()}"
        raise VerificationError(
            f"command failed with exit code {result.returncode}: "
            f"{' '.join(command)}{detail}"
        )
    return result


def package_crate(
    *,
    workspace: Path,
    target_dir: Path,
    name: str,
    version: str,
    locked: bool,
    registry_attempts: int = 1,
    registry_delay: float = 0,
    registry_package: str | None = None,
    registry_version: str | None = None,
) -> Path:
    command = [
        "cargo",
        "package",
        "--package",
        name,
        "--allow-dirty",
        "--no-verify",
        "--locked" if locked else "--exclude-lockfile",
        "--target-dir",
        str(target_dir),
    ]
    last_error = ""
    for attempt in range(1, registry_attempts + 1):
        result = run_command(command, cwd=workspace, capture=True, check=False)
        if result.returncode == 0:
            return exact_archive_path(target_dir / "package", name, version)

        last_error = "\n".join(
            output.strip()
            for output in (result.stdout, result.stderr)
            if output.strip()
        )
        retryable = (
            registry_package is not None
            and registry_version is not None
            and is_registry_propagation_error(
                last_error, registry_package, registry_version
            )
        )
        if not retryable:
            raise VerificationError(
                f"cargo package for {name} failed with a deterministic or "
                f"non-propagation error; not retrying:\n{last_error}"
            )
        if attempt < registry_attempts:
            print(
                f"crates.io has not indexed {registry_package} {registry_version} "
                f"for packaging (attempt {attempt}/{registry_attempts}); "
                f"retrying in {registry_delay:g}s",
                flush=True,
            )
            time.sleep(registry_delay)

    raise VerificationError(
        f"crates.io did not expose {registry_package} {registry_version} for "
        f"packaging after {registry_attempts} attempt(s):\n{last_error}"
    )


def safe_extract_archive(
    archive_path: Path, destination: Path, name: str, version: str
) -> ExtractedCrate:
    expected_root = f"{name}-{version}"
    destination.mkdir(parents=True, exist_ok=True)
    extraction_root = (destination / expected_root).resolve()
    seen: set[PurePosixPath] = set()

    try:
        archive = tarfile.open(archive_path, mode="r:gz")
    except (OSError, tarfile.TarError) as exc:
        raise VerificationError(f"cannot read archive {archive_path}: {exc}") from exc

    with archive:
        for member in archive.getmembers():
            relative = PurePosixPath(member.name)
            windows_parts = [PureWindowsPath(part) for part in relative.parts]
            if (
                "\\" in member.name
                or relative.is_absolute()
                or ".." in relative.parts
                or not relative.parts
                or relative.parts[0] != expected_root
                or any(part.drive or part.root for part in windows_parts)
            ):
                raise VerificationError(
                    f"archive {archive_path} has unsafe or unexpected member {member.name!r}"
                )
            if relative in seen:
                raise VerificationError(
                    f"archive {archive_path} contains duplicate member {member.name!r}"
                )
            seen.add(relative)
            output = destination.joinpath(*relative.parts).resolve()
            if not is_within(output, extraction_root):
                raise VerificationError(
                    f"archive {archive_path} member escapes extraction root: "
                    f"{member.name!r}"
                )
            if member.isdir():
                output.mkdir(parents=True, exist_ok=True)
            elif member.isfile():
                output.parent.mkdir(parents=True, exist_ok=True)
                source = archive.extractfile(member)
                if source is None:
                    raise VerificationError(f"cannot extract {member.name!r}")
                with source, output.open("wb") as target:
                    shutil.copyfileobj(source, target)
            else:
                raise VerificationError(
                    f"archive {archive_path} contains unsupported member {member.name!r}"
                )

    root = extraction_root
    manifest_path = root / "Cargo.toml"
    if not manifest_path.is_file():
        raise VerificationError(f"archive {archive_path} has no normalized Cargo.toml")
    manifest = load_toml(manifest_path)
    package = manifest.get("package", {})
    if package.get("name") != name or package.get("version") != version:
        raise VerificationError(
            f"archive manifest identifies {package.get('name')} {package.get('version')}, "
            f"expected {name} {version}"
        )
    if "workspace" in manifest or "patch" in manifest:
        raise VerificationError(
            f"normalized {name} manifest unexpectedly retains workspace-only tables"
        )

    vcs_sha, vcs_dirty = read_vcs_metadata(root, f"archive {archive_path}")

    return ExtractedCrate(
        name, version, archive_path, root, manifest, vcs_sha, vcs_dirty
    )


def read_vcs_metadata(root: Path, artifact: str) -> tuple[str | None, bool | None]:
    vcs_info_path = root / ".cargo_vcs_info.json"
    if vcs_info_path.is_file():
        try:
            vcs_info = json.loads(vcs_info_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            raise VerificationError(
                f"{artifact} has invalid .cargo_vcs_info.json: {exc}"
            ) from exc
        if not isinstance(vcs_info, dict):
            raise VerificationError(f"{artifact} has invalid VCS metadata")
        git_info = vcs_info.get("git", {})
        if not isinstance(git_info, dict):
            raise VerificationError(f"{artifact} has invalid git VCS metadata")
        candidate = git_info.get("sha1")
        if candidate is not None and not isinstance(candidate, str):
            raise VerificationError(f"{artifact} has a non-string VCS SHA")
        dirty = git_info.get("dirty", False)
        if not isinstance(dirty, bool):
            raise VerificationError(f"{artifact} has a non-boolean VCS dirty flag")
        return candidate, dirty
    return None, None


def require_archive_vcs_sha(crate: ExtractedCrate, expected_sha: str) -> None:
    if crate.vcs_sha != expected_sha:
        raise VerificationError(
            f"{crate.name} archive was produced from VCS SHA {crate.vcs_sha!r}; "
            f"expected exact release SHA {expected_sha}"
        )
    if crate.vcs_dirty is not False:
        raise VerificationError(
            f"{crate.name} archive VCS metadata reports dirty={crate.vcs_dirty!r}; "
            "expected a clean exact-release archive"
        )


def require_registry_vcs_sha(package: dict[str, Any], expected_sha: str) -> None:
    manifest_path = package.get("manifest_path")
    if not isinstance(manifest_path, str):
        raise VerificationError("registry ruviz package has no manifest path")
    vcs_sha, vcs_dirty = read_vcs_metadata(
        Path(manifest_path).resolve().parent, "registry ruviz artifact"
    )
    if vcs_sha != expected_sha:
        raise VerificationError(
            f"registry ruviz artifact was produced from VCS SHA {vcs_sha!r}; "
            f"expected exact release SHA {expected_sha}"
        )
    if vcs_dirty is not False:
        raise VerificationError(
            "registry ruviz artifact VCS metadata reports "
            f"dirty={vcs_dirty!r}; expected a clean exact-release artifact"
        )


def validate_normalized_manifests(
    ruviz: ExtractedCrate,
    ruviz_gpui: ExtractedCrate,
    contract: WorkspaceContract,
) -> None:
    validate_active_dependency_sources(ruviz, contract.workspace_only_dev_dependencies)
    validate_active_dependency_sources(
        ruviz_gpui, contract.workspace_only_dev_dependencies
    )

    dependency = dependency_table(ruviz_gpui.manifest, "dependencies", "ruviz")
    require_registry_dependency(dependency, name="ruviz", version=contract.version)

    gpui_targets: set[str] = set()
    for target, target_manifest in ruviz_gpui.manifest.get("target", {}).items():
        gpui = target_manifest.get("dependencies", {}).get("gpui")
        if gpui is not None:
            gpui_targets.add(target)
            require_registry_dependency(
                gpui, name=f"gpui ({target})", version=contract.gpui_version
            )
    if gpui_targets != SUPPORTED_GPUI_TARGETS:
        raise VerificationError(
            "normalized ruviz-gpui manifest is missing a supported-target GPUI dependency"
        )

    if ruviz.manifest.get("package", {}).get("version") != contract.version:
        raise VerificationError("normalized ruviz manifest version changed")


def toml_string(value: str | Path) -> str:
    return json.dumps(str(value))


def write_consumer(
    directory: Path,
    *,
    mode: str,
    ruviz: ExtractedCrate,
    ruviz_gpui: ExtractedCrate,
) -> None:
    dependencies = [
        f'ruviz = {{ version = "={ruviz.version}" }}',
        (
            "ruviz-gpui = { path = "
            f"{toml_string(ruviz_gpui.root)}, default-features = false }}"
        ),
    ]
    patch = ""
    if mode == "ci":
        patch = f"\n[patch.crates-io]\nruviz = {{ path = {toml_string(ruviz.root)} }}\n"

    manifest = (
        "[package]\n"
        'name = "ruviz-package-consumer"\n'
        'version = "0.0.0"\n'
        'edition = "2024"\n'
        "\n[dependencies]\n" + "\n".join(dependencies) + "\n" + patch
    )
    source = """use ruviz::prelude::*;
use ruviz_gpui::{
    gpui::{Context, Entity},
    plot_builder, PresentationMode, RuvizPlot,
};

fn embed<V: 'static>(cx: &mut Context<V>) -> Entity<RuvizPlot> {
    let plot: Plot = Plot::new()
        .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0])
        .title("packaged consumer")
        .into();
    plot_builder(plot)
        .interactive()
        .presentation(PresentationMode::Image)
        .build(cx)
}

fn main() {
    let _ = embed::<RuvizPlot>;
}
"""
    (directory / "src").mkdir(parents=True)
    (directory / "Cargo.toml").write_text(manifest, encoding="utf-8")
    (directory / "src/main.rs").write_text(source, encoding="utf-8")


def is_within(path: Path, parent: Path) -> bool:
    try:
        path.resolve().relative_to(parent.resolve())
    except ValueError:
        return False
    return True


def validate_metadata(
    metadata: dict[str, Any],
    *,
    mode: str,
    workspace: Path,
    consumer: Path,
    ruviz: ExtractedCrate,
    ruviz_gpui: ExtractedCrate,
    contract: WorkspaceContract,
    expected_vcs_sha: str | None = None,
) -> None:
    if Path(metadata.get("workspace_root", "")).resolve() != consumer.resolve():
        raise VerificationError(
            "Cargo metadata did not use the fresh external consumer"
        )

    packages = metadata.get("packages")
    if not isinstance(packages, list):
        raise VerificationError("Cargo metadata has no package list")
    expected_local = {
        "ruviz-package-consumer": consumer.resolve(),
        "ruviz-gpui": ruviz_gpui.root.resolve(),
    }
    if mode == "ci":
        expected_local["ruviz"] = ruviz.root.resolve()

    for package in packages:
        if not isinstance(package, dict) or not isinstance(
            package.get("manifest_path"), str
        ):
            raise VerificationError("Cargo metadata contains an invalid package")
        manifest_path = Path(package["manifest_path"]).resolve()
        if is_within(manifest_path, workspace):
            raise VerificationError(
                f"workspace source leaked into consumer: {manifest_path}"
            )
        source = package.get("source")
        if source is None:
            expected_root = expected_local.get(package.get("name"))
            if expected_root is None or not is_within(manifest_path, expected_root):
                raise VerificationError(
                    f"unexpected path package {package.get('name')} at {manifest_path}"
                )
        elif not isinstance(source, str):
            raise VerificationError(
                f"Cargo metadata has an invalid source for {package.get('name')}: "
                f"{source!r}"
            )
        elif source.startswith("git+") and (
            contract.gpui_patch_git in source or contract.gpui_patch_rev in source
        ):
            raise VerificationError(f"workspace GPUI git patch leaked: {source}")
        elif source.startswith("git+"):
            raise VerificationError(
                f"unexpected git dependency leaked into consumer: {source}"
            )

        if package.get("name") in contract.workspace_only_dev_dependencies:
            raise VerificationError(
                f"workspace-only dev dependency resolved in consumer: "
                f"{package.get('name')}"
            )

    def one_package(name: str, version: str) -> dict[str, Any]:
        matches = [
            package
            for package in packages
            if package.get("name") == name and package.get("version") == version
        ]
        if len(matches) != 1:
            raise VerificationError(
                f"expected exactly one {name} {version}, found {len(matches)}"
            )
        return matches[0]

    adapter = one_package("ruviz-gpui", contract.version)
    if adapter.get("source") is not None or not is_within(
        Path(adapter["manifest_path"]), ruviz_gpui.root
    ):
        raise VerificationError("ruviz-gpui did not resolve from its unpacked archive")

    core = one_package("ruviz", contract.version)
    if mode == "ci":
        if core.get("source") is not None or not is_within(
            Path(core["manifest_path"]), ruviz.root
        ):
            raise VerificationError(
                "CI ruviz did not resolve from its unpacked archive"
            )
    elif not is_crates_io_source(core.get("source")):
        raise VerificationError(
            f"release ruviz must resolve from crates.io, got {core.get('source')!r}"
        )
    elif expected_vcs_sha is not None:
        require_registry_vcs_sha(core, expected_vcs_sha)

    gpui = one_package("gpui", contract.gpui_version)
    if not is_crates_io_source(gpui.get("source")):
        raise VerificationError(
            f"GPUI must resolve from crates.io, got {gpui.get('source')!r}"
        )

    resolve = metadata.get("resolve")
    if not isinstance(resolve, dict) or not isinstance(resolve.get("nodes"), list):
        raise VerificationError("Cargo metadata has no usable resolve graph")
    nodes_by_id = {
        node.get("id"): node
        for node in resolve["nodes"]
        if isinstance(node, dict) and isinstance(node.get("id"), str)
    }

    def node_for(package: dict[str, Any]) -> dict[str, Any]:
        node = nodes_by_id.get(package.get("id"))
        if not isinstance(node, dict) or not isinstance(node.get("deps"), list):
            raise VerificationError(
                f"Cargo resolve graph has no usable node for {package['name']}"
            )
        return node

    def require_normal_edge(
        package: dict[str, Any], dependency_name: str, expected_id: str
    ) -> None:
        node = node_for(package)
        edges = []
        for dependency in node["deps"]:
            if not isinstance(dependency, dict):
                continue
            kinds = dependency.get("dep_kinds")
            if not isinstance(kinds, list):
                continue
            if (
                isinstance(dependency.get("name"), str)
                and dependency["name"].replace("_", "-") == dependency_name
                and dependency.get("pkg") == expected_id
                and any(
                    isinstance(kind, dict) and kind.get("kind") is None
                    for kind in kinds
                )
            ):
                edges.append(dependency)
        if len(edges) != 1:
            raise VerificationError(
                f"{package['name']} does not resolve one normal {dependency_name} "
                "edge to the expected package"
            )

    consumer_package = one_package("ruviz-package-consumer", "0.0.0")
    require_normal_edge(consumer_package, "ruviz", core["id"])
    require_normal_edge(consumer_package, "ruviz-gpui", adapter["id"])
    require_normal_edge(adapter, "ruviz", core["id"])
    require_normal_edge(adapter, "gpui", gpui["id"])

    for package in (core, adapter):
        node = node_for(package)
        for dependency in node["deps"]:
            if not isinstance(dependency, dict):
                raise VerificationError(
                    f"Cargo resolve graph has an invalid edge for {package['name']}"
                )
            kinds = dependency.get("dep_kinds")
            if not isinstance(kinds, list):
                raise VerificationError(
                    f"Cargo resolve edge has no dependency kinds for {package['name']}"
                )
            if any(
                isinstance(kind, dict) and kind.get("kind") == "dev" for kind in kinds
            ):
                raise VerificationError(
                    f"dev dependency {dependency.get('name')!r} from "
                    f"{package['name']} leaked into the consumer resolve graph"
                )


def is_registry_propagation_error(stderr: str, name: str, version: str) -> bool:
    """Identify the exact-version-not-yet-indexed Cargo failures worth retrying."""
    lowered = stderr.lower()
    if "crates.io index" not in lowered or name.lower() not in lowered:
        return False
    missing_exact_version = (
        "failed to select a version for the requirement" in lowered
        or f"no matching package named `{name.lower()}` found" in lowered
    )
    return missing_exact_version and version.lower() in lowered


def generate_lockfile_with_registry_retries(
    consumer: Path,
    *,
    env: dict[str, str],
    attempts: int,
    delay: float,
    registry_package: str,
    registry_version: str,
) -> None:
    command = ["cargo", "generate-lockfile"]
    last_error = ""
    for attempt in range(1, attempts + 1):
        result = run_command(command, cwd=consumer, env=env, capture=True, check=False)
        if result.returncode == 0:
            return
        last_error = result.stderr.strip()
        retryable = is_registry_propagation_error(
            last_error, registry_package, registry_version
        )
        if not retryable:
            raise VerificationError(
                "cargo generate-lockfile failed with a deterministic or non-propagation "
                f"error; not retrying:\n{last_error}"
            )
        if attempt < attempts:
            print(
                f"crates.io has not indexed {registry_package} {registry_version} "
                f"(attempt {attempt}/{attempts}); "
                f"retrying in {delay:g}s",
                flush=True,
            )
            time.sleep(delay)
    raise VerificationError(
        f"crates.io did not expose {registry_package} {registry_version} after "
        f"{attempts} attempt(s):\n{last_error}"
    )


def verify(args: argparse.Namespace) -> None:
    workspace = args.workspace.expanduser().resolve()
    contract = inspect_workspace(workspace)

    system_temp = Path(tempfile.gettempdir()).resolve()
    if is_within(system_temp, workspace):
        raise VerificationError(
            f"system temporary directory is inside workspace: {system_temp}"
        )

    with tempfile.TemporaryDirectory(prefix="ruviz-package-verifier-") as temporary:
        temp = Path(temporary).resolve()
        if is_within(temp, workspace):
            raise VerificationError(
                "verifier temporary directory must be outside workspace"
            )

        package_target = temp / "package-target"
        extracted = temp / "archives"

        if args.ruviz_archive is None:
            ruviz_archive = package_crate(
                workspace=workspace,
                target_dir=package_target / "ruviz",
                name="ruviz",
                version=contract.version,
                locked=True,
            )
        else:
            ruviz_archive = resolve_archive_argument(
                args.ruviz_archive, "ruviz", contract.version
            )
        ruviz = safe_extract_archive(
            ruviz_archive, extracted, "ruviz", contract.version
        )

        if args.ruviz_gpui_archive is None:
            gpui_archive = package_crate(
                workspace=workspace,
                target_dir=package_target / "ruviz-gpui",
                name="ruviz-gpui",
                version=contract.version,
                locked=args.mode == "release",
                registry_attempts=(
                    args.registry_attempts if args.mode == "release" else 1
                ),
                registry_delay=args.registry_delay,
                registry_package="ruviz" if args.mode == "release" else None,
                registry_version=(contract.version if args.mode == "release" else None),
            )
        else:
            gpui_archive = resolve_archive_argument(
                args.ruviz_gpui_archive, "ruviz-gpui", contract.version
            )
        ruviz_gpui = safe_extract_archive(
            gpui_archive, extracted, "ruviz-gpui", contract.version
        )
        validate_normalized_manifests(ruviz, ruviz_gpui, contract)
        if args.expected_vcs_sha is not None:
            require_archive_vcs_sha(ruviz, args.expected_vcs_sha)
            require_archive_vcs_sha(ruviz_gpui, args.expected_vcs_sha)

        consumer = temp / "consumer"
        consumer.mkdir()
        write_consumer(
            consumer,
            mode=args.mode,
            ruviz=ruviz,
            ruviz_gpui=ruviz_gpui,
        )

        cargo_env = os.environ.copy()
        cargo_env["CARGO_TARGET_DIR"] = str(temp / "consumer-target")
        registry_package = "ruviz" if args.mode == "release" else "gpui"
        registry_version = (
            contract.version if args.mode == "release" else contract.gpui_version
        )
        generate_lockfile_with_registry_retries(
            consumer,
            env=cargo_env,
            attempts=args.registry_attempts,
            delay=args.registry_delay,
            registry_package=registry_package,
            registry_version=registry_version,
        )
        if not (consumer / "Cargo.lock").is_file():
            raise VerificationError("cargo generate-lockfile did not create Cargo.lock")
        metadata_result = run_command(
            ["cargo", "metadata", "--format-version", "1", "--locked"],
            cwd=consumer,
            env=cargo_env,
            capture=True,
        )
        try:
            metadata = json.loads(metadata_result.stdout)
        except json.JSONDecodeError as exc:
            raise VerificationError(
                f"cargo metadata returned invalid JSON: {exc}"
            ) from exc
        validate_metadata(
            metadata,
            mode=args.mode,
            workspace=workspace,
            consumer=consumer,
            ruviz=ruviz,
            ruviz_gpui=ruviz_gpui,
            contract=contract,
            expected_vcs_sha=args.expected_vcs_sha,
        )
        run_command(["cargo", "check", "--locked"], cwd=consumer, env=cargo_env)

        print(
            f"Verified ruviz {contract.version} and ruviz-gpui {contract.version} "
            f"in {args.mode} mode: GPUI {contract.gpui_version} is registry-sourced."
        )


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=("ci", "release"), required=True)
    parser.add_argument(
        "--workspace", type=Path, default=Path(__file__).resolve().parents[1]
    )
    parser.add_argument(
        "--ruviz-archive",
        type=Path,
        help="ruviz .crate file or directory containing the exact versioned archive",
    )
    parser.add_argument(
        "--ruviz-gpui-archive",
        type=Path,
        help="ruviz-gpui .crate file or directory containing the exact archive",
    )
    parser.add_argument(
        "--expected-vcs-sha",
        help="require both local archives to carry this exact cargo VCS SHA",
    )
    parser.add_argument("--registry-attempts", type=int, default=1)
    parser.add_argument("--registry-delay", type=float, default=20.0)
    args = parser.parse_args(argv)
    if args.registry_attempts < 1:
        parser.error("--registry-attempts must be at least 1")
    if args.registry_delay < 0:
        parser.error("--registry-delay cannot be negative")
    if (
        args.expected_vcs_sha is not None
        and re.fullmatch(r"[0-9a-f]{40}", args.expected_vcs_sha) is None
    ):
        parser.error("--expected-vcs-sha must be a full lowercase 40-character SHA")
    return args


def main(argv: list[str] | None = None) -> int:
    try:
        verify(parse_args(argv))
    except (OSError, subprocess.SubprocessError, VerificationError, ValueError) as exc:
        print(f"packaged-crate verification failed: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
