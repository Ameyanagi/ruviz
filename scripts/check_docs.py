#!/usr/bin/env python3
"""Validate repository-facing Markdown documentation.

The checks are intentionally narrow and deterministic:
- Markdown fences must be balanced.
- Local Markdown links/images in README.md and docs/ must resolve.
- The README's first Rust quick-start block must be a complete binary that
  type-checks against the local crate.
"""

from __future__ import annotations

import re
import subprocess
import sys
import tempfile
from pathlib import Path
from urllib.parse import unquote


ROOT = Path(__file__).resolve().parents[1]
MARKDOWN_ROOTS = [ROOT / "README.md", ROOT / "docs"]
LINK_RE = re.compile(r"!?\[[^\]]*\]\(([^)]+)\)")
FENCE_RE = re.compile(r"^```", re.MULTILINE)
RUST_BLOCK_RE = re.compile(r"```rust\n(.*?)\n```", re.DOTALL)


def markdown_files() -> list[Path]:
    files: list[Path] = []
    for root in MARKDOWN_ROOTS:
        if root.is_file():
            files.append(root)
        else:
            files.extend(sorted(root.rglob("*.md")))
    return sorted(files)


def is_external_link(target: str) -> bool:
    return target.startswith(("http://", "https://", "mailto:", "tel:"))


def normalized_local_target(raw: str) -> str | None:
    target = raw.strip().strip("<>")
    if not target or target.startswith("#") or is_external_link(target):
        return None
    target = target.split("#", 1)[0]
    if not target:
        return None
    return unquote(target)


def check_fences(files: list[Path]) -> list[str]:
    errors: list[str] = []
    for path in files:
        count = len(FENCE_RE.findall(path.read_text(encoding="utf-8")))
        if count % 2 != 0:
            errors.append(f"{path.relative_to(ROOT)} has an unbalanced Markdown fence")
    return errors


def check_local_links(files: list[Path]) -> list[str]:
    errors: list[str] = []
    for path in files:
        text = path.read_text(encoding="utf-8")
        for match in LINK_RE.finditer(text):
            target = normalized_local_target(match.group(1))
            if target is None:
                continue
            full_path = (path.parent / target).resolve()
            if not full_path.exists():
                line = text.count("\n", 0, match.start()) + 1
                errors.append(
                    f"{path.relative_to(ROOT)}:{line} points to missing local target {target!r}"
                )
    return errors


def readme_quickstart_block() -> str:
    readme = (ROOT / "README.md").read_text(encoding="utf-8")
    match = RUST_BLOCK_RE.search(readme)
    if match is None:
        raise RuntimeError("README.md does not contain a Rust quick-start block")
    code = match.group(1)
    if "fn main" not in code:
        raise RuntimeError("README.md first Rust block must include fn main")
    return code


def check_readme_quickstart() -> list[str]:
    try:
        code = readme_quickstart_block()
    except RuntimeError as error:
        return [str(error)]

    with tempfile.TemporaryDirectory(prefix="ruviz-readme-check-") as temp:
        temp_path = Path(temp)
        (temp_path / "src").mkdir()
        (temp_path / "src" / "main.rs").write_text(code, encoding="utf-8")
        (temp_path / "Cargo.toml").write_text(
            "\n".join(
                [
                    "[package]",
                    'name = "ruviz-readme-check"',
                    'version = "0.0.0"',
                    'edition = "2024"',
                    "",
                    "[dependencies]",
                    f'ruviz = {{ path = "{ROOT}" }}',
                    "",
                ]
            ),
            encoding="utf-8",
        )
        result = subprocess.run(
            ["cargo", "check", "--quiet", "--manifest-path", str(temp_path / "Cargo.toml")],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        if result.returncode != 0:
            return [
                "README.md first Rust block failed cargo check:\n"
                + result.stdout
                + result.stderr
            ]
    return []


def main() -> int:
    files = markdown_files()
    errors = check_fences(files)
    errors.extend(check_local_links(files))
    errors.extend(check_readme_quickstart())

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    print(f"Validated {len(files)} Markdown files and README quick-start syntax.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
