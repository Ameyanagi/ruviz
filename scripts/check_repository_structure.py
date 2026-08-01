#!/usr/bin/env python3
"""Keep tracked files within the repository's documented directory layout."""

from __future__ import annotations

import subprocess
from pathlib import Path, PurePosixPath
from typing import Iterable


CANONICAL_TOP_LEVEL_DIRECTORIES = frozenset(
    {
        ".github",
        "adapters",
        "apps",
        "benches",
        "bindings",
        "docs",
        "examples",
        "generated",
        "packages",
        "proptest-regressions",
        "scripts",
        "src",
        "tests",
        "tools",
    }
)
RETIRED_PATH_PREFIXES = (
    PurePosixPath("packages/ruviz-web"),
)


def is_within(path: PurePosixPath, parent: PurePosixPath) -> bool:
    """Return whether a repository-relative path is inside a parent path."""

    return path == parent or parent in path.parents


def tracked_structure_violations(paths: Iterable[str]) -> list[str]:
    """Describe tracked paths that do not follow the canonical layout."""

    violations: list[str] = []
    for raw_path in paths:
        path = PurePosixPath(raw_path)
        if len(path.parts) < 2:
            # Root-level project files such as Cargo.toml and README.md are valid.
            continue

        root = path.parts[0]
        if root not in CANONICAL_TOP_LEVEL_DIRECTORIES:
            violations.append(
                f"{raw_path}: non-canonical top-level directory {root}/"
            )
            continue

        retired_prefix = next(
            (prefix for prefix in RETIRED_PATH_PREFIXES if is_within(path, prefix)),
            None,
        )
        if retired_prefix is not None:
            violations.append(
                f"{raw_path}: retired directory {retired_prefix.as_posix()}/"
            )

    return sorted(violations)


def tracked_files(repository: Path) -> list[str]:
    """Read tracked paths only, leaving ignored local directories out of scope."""

    result = subprocess.run(
        ["git", "-C", str(repository), "ls-files", "-z"],
        check=True,
        capture_output=True,
        text=True,
    )
    return [path for path in result.stdout.split("\0") if path]


def main() -> int:
    repository = Path(__file__).resolve().parent.parent
    violations = tracked_structure_violations(tracked_files(repository))
    if violations:
        print("Tracked files must follow docs/REPOSITORY_STRUCTURE.md:")
        for violation in violations:
            print(f"  {violation}")
        return 1

    print("Tracked repository structure is canonical.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
