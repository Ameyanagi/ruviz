#!/usr/bin/env python3
"""Fail if published docs reference generated preview assets."""

from __future__ import annotations

import re
from pathlib import Path


ROOTS = [
    Path("README.md"),
    Path("src/lib.rs"),
    Path("docs"),
    Path("python/README.md"),
    Path("python/docs"),
    Path("packages/ruviz-web/README.md"),
    Path("packages/ruviz-web/docs"),
    Path("crates/ruviz-web/README.md"),
    Path("crates/ruviz-web/src/lib.rs"),
    Path("crates/ruviz-gpui/README.md"),
    Path("crates/ruviz-gpui/src/lib.rs"),
]

TEXT_SUFFIXES = {".md", ".rs"}
PATTERNS = [
    re.compile(r"!\[[^\]]*\]\(\s*(?:\./)?generated/"),
    re.compile(r"\[[^\]]+\]\(\s*(?:\./)?generated/"),
    re.compile(r"""(?:src|href)=["'](?:\./)?generated/"""),
    re.compile(r"""https://raw\.githubusercontent\.com/[^"'\s)]+/generated/"""),
]


def iter_files(root: Path) -> list[Path]:
    if root.is_file():
        return [root]
    return sorted(
        path
        for path in root.rglob("*")
        if path.is_file() and path.suffix in TEXT_SUFFIXES
    )


def main() -> int:
    violations: list[str] = []

    for root in ROOTS:
        for path in iter_files(root):
            with path.open("r", encoding="utf-8") as handle:
                in_fence = False
                for line_number, line in enumerate(handle, start=1):
                    if path.suffix == ".md" and line.strip().startswith("```"):
                        in_fence = not in_fence
                    if in_fence:
                        continue

                    for pattern in PATTERNS:
                        if pattern.search(line):
                            violations.append(f"{path}:{line_number}: {line.rstrip()}")
                            break

    if violations:
        print("Published docs must not reference generated preview assets:")
        for violation in violations:
            print(violation)
        return 1

    print("No published docs reference generated preview assets.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
