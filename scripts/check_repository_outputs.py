#!/usr/bin/env python3
"""Enforce the boundary between source trees and generated output."""

from __future__ import annotations

import re
import subprocess
from pathlib import Path, PurePosixPath
from typing import Iterable


OUTPUT_SUFFIXES = {
    ".bin",
    ".csv",
    ".gif",
    ".html",
    ".jpeg",
    ".jpg",
    ".pdf",
    ".png",
    ".svg",
    ".webp",
}
FORBIDDEN_OUTPUT_ROOTS = (
    PurePosixPath("examples"),
    PurePosixPath("tools/gallery"),
)
ALLOWED_GENERATED_FILES = {
    PurePosixPath("generated/README.md"),
    PurePosixPath("generated/manifest.json"),
}
ALLOWED_TEST_OUTPUT_ROOTS = (
    PurePosixPath("tests/fixtures"),
    PurePosixPath("tests/visual/reference"),
)
SAVE_PATTERN = re.compile(
    r"\.(?:save|save_with_size)\s*\(\s*"
    r'(?:format!\s*\(\s*)?"(?P<path>[^"\n]+\.(?:bin|csv|gif|html|jpeg|jpg|pdf|png|svg|webp))"'
)


def is_within(path: PurePosixPath, parent: PurePosixPath) -> bool:
    """Return whether a repository-relative path is inside a parent path."""

    return path == parent or parent in path.parents


def tracked_output_violations(paths: Iterable[str]) -> list[str]:
    """Find tracked output artifacts that live in source-oriented paths."""

    violations: list[str] = []
    for raw_path in paths:
        path = PurePosixPath(raw_path)
        if "__pycache__" in path.parts or path.suffix.lower() in {".pyc", ".pyo"}:
            violations.append(raw_path)
            continue

        if path.parts and path.parts[0] == "generated" and path not in ALLOWED_GENERATED_FILES:
            violations.append(raw_path)
            continue

        if path.suffix.lower() not in OUTPUT_SUFFIXES:
            continue

        if len(path.parts) == 1:
            violations.append(raw_path)
            continue

        if any(is_within(path, root) for root in FORBIDDEN_OUTPUT_ROOTS):
            violations.append(raw_path)
            continue

        if path.parts[0] == "tests" and not any(
            is_within(path, allowed) for allowed in ALLOWED_TEST_OUTPUT_ROOTS
        ):
            violations.append(raw_path)

    return sorted(violations)


def gallery_output_violations(repository: Path) -> list[str]:
    """Find literal gallery save destinations outside generated/."""

    gallery = repository / "tools" / "gallery"
    if not gallery.is_dir():
        return []

    violations: list[str] = []
    for path in sorted(gallery.rglob("*.rs")):
        contents = path.read_text(encoding="utf-8")
        for match in SAVE_PATTERN.finditer(contents):
            destination = match.group("path")
            if destination.startswith("generated/"):
                continue
            line_number = contents.count("\n", 0, match.start()) + 1
            relative = path.relative_to(repository)
            violations.append(f"{relative}:{line_number}: {destination}")
    return violations


def tracked_files(repository: Path) -> list[str]:
    """Read the repository's tracked file list without inspecting ignored files."""

    result = subprocess.run(
        ["git", "-C", str(repository), "ls-files", "-z"],
        check=True,
        capture_output=True,
        text=True,
    )
    return [path for path in result.stdout.split("\0") if path]


def main() -> int:
    repository = Path(__file__).resolve().parent.parent
    tracked_violations = tracked_output_violations(tracked_files(repository))
    writer_violations = gallery_output_violations(repository)

    if tracked_violations:
        print("Generated output and cache files must not be tracked:")
        for violation in tracked_violations:
            print(f"  {violation}")

    if writer_violations:
        print("Gallery programs must save transient output under generated/:")
        for violation in writer_violations:
            print(f"  {violation}")

    if tracked_violations or writer_violations:
        return 1

    print("Repository output boundaries are clean.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
