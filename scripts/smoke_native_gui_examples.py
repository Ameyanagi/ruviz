#!/usr/bin/env python3
"""Run every built native GUI example long enough to prove its window loop starts.

The Cargo builds intentionally happen outside this script. Their JSON message
streams are the source of truth for the exact executables to run, so time spent
compiling cannot be mistaken for runtime survival.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence


class SmokeError(RuntimeError):
    """A native GUI smoke-test contract or runtime failure."""


@dataclass(frozen=True)
class ExampleArtifact:
    package: str
    example: str
    executable: Path

    @property
    def identity(self) -> str:
        return f"{self.package}:{self.example}"


IDENTITY_PATTERN = re.compile(r"^[A-Za-z0-9_-]+:[A-Za-z0-9_-]+$")


def discover_examples(
    cargo_json_paths: Iterable[Path], repository: Path
) -> dict[str, ExampleArtifact]:
    """Read Cargo JSON streams and return executable example artifacts by identity."""

    artifacts: dict[str, ExampleArtifact] = {}
    for cargo_json_path in cargo_json_paths:
        try:
            lines = cargo_json_path.read_text(encoding="utf-8").splitlines()
        except OSError as error:
            raise SmokeError(
                f"cannot read Cargo messages from {cargo_json_path}: {error}"
            ) from error

        for line_number, line in enumerate(lines, start=1):
            if not line.strip():
                continue
            try:
                message = json.loads(line)
            except json.JSONDecodeError as error:
                raise SmokeError(
                    f"{cargo_json_path}:{line_number} is not a Cargo JSON message: "
                    f"{error.msg}"
                ) from error

            if message.get("reason") != "compiler-artifact":
                continue
            target = message.get("target", {})
            if "example" not in target.get("kind", []):
                continue
            executable_value = message.get("executable")
            manifest_value = message.get("manifest_path")
            example = target.get("name")
            if not executable_value or not manifest_value or not example:
                continue

            manifest = Path(manifest_value)
            package = manifest.parent.name
            executable = Path(executable_value)
            if not executable.is_absolute():
                executable = repository / executable
            artifact = ExampleArtifact(package, example, executable.resolve())

            previous = artifacts.get(artifact.identity)
            if previous is not None and previous.executable != artifact.executable:
                raise SmokeError(
                    f"Cargo reported multiple executables for {artifact.identity}: "
                    f"{previous.executable} and {artifact.executable}"
                )
            artifacts[artifact.identity] = artifact

    return artifacts


def select_expected_examples(
    artifacts: dict[str, ExampleArtifact], expected: Iterable[str]
) -> list[ExampleArtifact]:
    """Require the built example set to match the explicitly reviewed contract."""

    expected_values = list(expected)
    expected_set = set(expected_values)
    if not expected_set:
        raise SmokeError("at least one --expect package:example value is required")
    if len(expected_set) != len(expected_values):
        raise SmokeError("duplicate --expect package:example value")
    invalid = sorted(
        identity
        for identity in expected_set
        if IDENTITY_PATTERN.fullmatch(identity) is None
    )
    if invalid:
        raise SmokeError(f"invalid expected example identities: {', '.join(invalid)}")

    actual_set = set(artifacts)
    missing = sorted(expected_set - actual_set)
    unexpected = sorted(actual_set - expected_set)
    problems = []
    if missing:
        problems.append(f"missing: {', '.join(missing)}")
    if unexpected:
        problems.append(f"unexpected: {', '.join(unexpected)}")
    if problems:
        raise SmokeError("built example contract mismatch (" + "; ".join(problems) + ")")

    selected = [artifacts[identity] for identity in sorted(expected_set)]
    executable_owners: dict[Path, str] = {}
    for artifact in selected:
        if not artifact.executable.is_file():
            raise SmokeError(
                f"Cargo executable for {artifact.identity} is missing: "
                f"{artifact.executable}"
            )
        if not os.access(artifact.executable, os.X_OK):
            raise SmokeError(
                f"Cargo executable for {artifact.identity} is not executable: "
                f"{artifact.executable}"
            )
        previous = executable_owners.get(artifact.executable)
        if previous is not None:
            raise SmokeError(
                f"{previous} and {artifact.identity} resolve to the same executable: "
                f"{artifact.executable}"
            )
        executable_owners[artifact.executable] = artifact.identity
    return selected


def stage_examples(
    artifacts: Sequence[ExampleArtifact], stage_directory: Path
) -> None:
    """Copy collision-prone Cargo outputs into stable package-specific paths."""

    stage_directory.mkdir(parents=True, exist_ok=True)
    packages: dict[str, list[dict[str, str]]] = {}
    for artifact in artifacts:
        package_directory = stage_directory / artifact.package
        package_directory.mkdir(parents=True, exist_ok=True)
        destination = package_directory / artifact.example
        if destination.exists():
            raise SmokeError(f"refusing to overwrite staged executable: {destination}")
        try:
            shutil.copy2(artifact.executable, destination)
        except OSError as error:
            raise SmokeError(
                f"cannot stage {artifact.identity} at {destination}: {error}"
            ) from error
        destination.chmod(destination.stat().st_mode | 0o111)
        relative = destination.relative_to(stage_directory)
        packages.setdefault(artifact.package, []).append(
            {"identity": artifact.identity, "executable": str(relative)}
        )

    for package, records in packages.items():
        manifest = stage_directory / f"{package}.json"
        if manifest.exists():
            raise SmokeError(f"refusing to overwrite staged manifest: {manifest}")
        manifest.write_text(
            json.dumps({"examples": sorted(records, key=lambda item: item["identity"])})
            + "\n",
            encoding="utf-8",
        )


def discover_staged_examples(stage_directory: Path) -> dict[str, ExampleArtifact]:
    """Load only the package manifests written by :func:`stage_examples`."""

    root = stage_directory.resolve()
    artifacts: dict[str, ExampleArtifact] = {}
    manifests = sorted(stage_directory.glob("*.json"))
    if not manifests:
        raise SmokeError(f"no staged package manifests found in {stage_directory}")

    for manifest in manifests:
        try:
            payload = json.loads(manifest.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            raise SmokeError(f"cannot read staged manifest {manifest}: {error}") from error
        records = payload.get("examples")
        if not isinstance(records, list):
            raise SmokeError(f"staged manifest has no examples list: {manifest}")
        for record in records:
            if not isinstance(record, dict):
                raise SmokeError(f"invalid example record in {manifest}")
            identity = record.get("identity")
            executable_value = record.get("executable")
            if (
                not isinstance(identity, str)
                or IDENTITY_PATTERN.fullmatch(identity) is None
                or not isinstance(executable_value, str)
            ):
                raise SmokeError(f"invalid example record in {manifest}")
            package, example = identity.split(":", maxsplit=1)
            if package != manifest.stem:
                raise SmokeError(
                    f"{identity} is stored in the wrong staged manifest: {manifest}"
                )
            executable_path = Path(executable_value)
            if executable_path.is_absolute():
                raise SmokeError(f"absolute staged executable path in {manifest}")
            executable = (root / executable_path).resolve()
            try:
                executable.relative_to(root)
            except ValueError as error:
                raise SmokeError(
                    f"staged executable escapes {stage_directory}: {executable_value}"
                ) from error
            artifact = ExampleArtifact(package, example, executable)
            if identity in artifacts:
                raise SmokeError(f"duplicate staged example identity: {identity}")
            artifacts[identity] = artifact
    return artifacts


def _terminate_process_group(process: subprocess.Popen[bytes]) -> None:
    """Stop the example and every Xvfb/window child in its process group."""

    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        return

    try:
        process.wait(timeout=2)
    except subprocess.TimeoutExpired:
        pass

    # The group leader may exit before one of its children. Always make a final
    # best-effort group kill so no window loop or X server leaks into the next run.
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    try:
        process.wait(timeout=2)
    except subprocess.TimeoutExpired as error:
        raise SmokeError(f"process group {process.pid} did not terminate") from error


def smoke_example(
    artifact: ExampleArtifact,
    grace_seconds: float,
    environment: dict[str, str],
    wrapper: Sequence[str],
) -> None:
    """Fail if an example exits before the grace period; stop it if it survives."""

    command = [*wrapper, str(artifact.executable)]
    with tempfile.TemporaryFile() as output:
        try:
            process = subprocess.Popen(
                command,
                env=environment,
                stdout=output,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
        except OSError as error:
            raise SmokeError(
                f"{artifact.identity} could not start ({' '.join(command)}): {error}"
            ) from error

        early_exit: int | None = None
        try:
            early_exit = process.wait(timeout=grace_seconds)
        except subprocess.TimeoutExpired:
            pass
        finally:
            _terminate_process_group(process)

        if early_exit is not None:
            output.seek(0)
            captured = output.read(16_384).decode("utf-8", errors="replace").strip()
            details = f"\n{captured}" if captured else ""
            raise SmokeError(
                f"{artifact.identity} exited after startup with status "
                f"{early_exit} before {grace_seconds:.1f}s{details}"
            )


def smoke_environment() -> dict[str, str]:
    environment = os.environ.copy()
    environment.update(
        {
            "GALLIUM_DRIVER": "llvmpipe",
            "LIBGL_ALWAYS_SOFTWARE": "1",
            "RUST_BACKTRACE": "1",
            "WGPU_BACKEND": "vulkan",
            "WGPU_POWER_PREF": "low",
            "WINIT_UNIX_BACKEND": "x11",
            "XDG_SESSION_TYPE": "x11",
        }
    )
    environment.pop("WAYLAND_DISPLAY", None)
    return environment


def run_smokes(
    artifacts: Sequence[ExampleArtifact],
    grace_seconds: float,
    wrapper: Sequence[str],
) -> None:
    failures: list[str] = []
    environment = smoke_environment()
    for artifact in artifacts:
        print(
            f"[smoke] {artifact.identity}: must remain alive for "
            f"{grace_seconds:.1f}s",
            flush=True,
        )
        try:
            smoke_example(artifact, grace_seconds, environment, wrapper)
        except SmokeError as error:
            failures.append(str(error))
            print(f"[failed] {error}", flush=True)
        else:
            print(f"[ok] {artifact.identity}", flush=True)
    if failures:
        raise SmokeError(
            f"{len(failures)} native GUI example(s) failed runtime smoke:\n"
            + "\n\n".join(failures)
        )


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)

    stage = commands.add_parser(
        "stage", help="discover freshly built Cargo examples and copy them safely"
    )
    stage.add_argument(
        "--cargo-json",
        action="append",
        required=True,
        type=Path,
        help="Cargo --message-format=json-render-diagnostics output (repeatable)",
    )
    stage.add_argument(
        "--expect",
        action="append",
        required=True,
        help="required package:example identity (repeatable)",
    )
    stage.add_argument("--stage-dir", required=True, type=Path)

    run = commands.add_parser(
        "run", help="run all explicitly expected staged examples under Xvfb"
    )
    run.add_argument("--stage-dir", required=True, type=Path)
    run.add_argument(
        "--expect",
        action="append",
        required=True,
        help="required package:example identity (repeatable)",
    )
    run.add_argument(
        "--grace-seconds",
        type=float,
        default=3.0,
        help="minimum time each native window loop must remain alive (default: 3)",
    )
    run.add_argument(
        "--xvfb-run",
        default="xvfb-run",
        help="xvfb-run executable (default: xvfb-run)",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv if argv is not None else sys.argv[1:])
    if args.command == "stage":
        repository = Path.cwd().resolve()
        discovered = discover_examples(args.cargo_json, repository)
        selected = select_expected_examples(discovered, args.expect)
        stage_examples(selected, args.stage_dir)
        print(f"Staged {len(selected)} native GUI example executable(s).")
        return 0

    if args.grace_seconds <= 0:
        raise SmokeError("--grace-seconds must be greater than zero")
    xvfb_run = shutil.which(args.xvfb_run)
    if xvfb_run is None:
        raise SmokeError(f"xvfb-run executable not found: {args.xvfb_run}")

    discovered = discover_staged_examples(args.stage_dir)
    selected = select_expected_examples(discovered, args.expect)
    wrapper = (
        xvfb_run,
        "--auto-servernum",
        "--server-args=-screen 0 1280x1024x24 -nolisten tcp",
    )
    run_smokes(selected, args.grace_seconds, wrapper)
    print(f"All {len(selected)} native GUI examples survived startup under Xvfb.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except SmokeError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
