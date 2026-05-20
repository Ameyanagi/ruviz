#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RISKY_CALL = re.compile(r"\.(unwrap|expect)\s*\(|\bpanic!\s*\(|\btodo!\s*\(|\bunimplemented!\s*\(")


def is_production_path(path: str) -> bool:
    parts = Path(path).parts
    if any(part in {"tests", "examples", "benches", "benchmarks", "docs", "gallery"} for part in parts):
        return False
    if path.endswith("_test.rs") or path.endswith("tests.rs") or path.endswith(".md"):
        return False
    return path.endswith(".rs")


def changed_lines(base: str) -> list[tuple[str, int | None, str]]:
    diff = subprocess.check_output(
        ["git", "diff", "--unified=0", base, "--", "*.rs"],
        cwd=ROOT,
        text=True,
    )
    findings: list[tuple[str, int | None, str]] = []
    current_path = ""
    current_line: int | None = None

    for line in diff.splitlines():
        if line.startswith("+++ b/"):
            current_path = line.removeprefix("+++ b/")
            current_line = None
            continue
        if line.startswith("@@"):
            match = re.search(r"\+(\d+)", line)
            current_line = int(match.group(1)) if match else None
            continue
        if line.startswith("+") and not line.startswith("+++"):
            if current_path and is_production_path(current_path) and RISKY_CALL.search(line):
                findings.append((current_path, current_line, line[1:].strip()))
            if current_line is not None:
                current_line += 1
        elif not line.startswith("-") and current_line is not None:
            current_line += 1

    return findings


def untracked_rust_lines() -> list[tuple[str, int | None, str]]:
    output = subprocess.check_output(
        ["git", "ls-files", "--others", "--exclude-standard", "--", "*.rs"],
        cwd=ROOT,
        text=True,
    )
    findings: list[tuple[str, int | None, str]] = []

    for path in output.splitlines():
        if not is_production_path(path):
            continue
        source = (ROOT / path).read_text(encoding="utf-8")
        for line_number, line in enumerate(source.splitlines(), start=1):
            if RISKY_CALL.search(line):
                findings.append((path, line_number, line.strip()))

    return findings


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Fail when a branch adds unwrap/expect/panic-style calls in production Rust code."
    )
    parser.add_argument("--base", default="main")
    args = parser.parse_args()

    findings = changed_lines(args.base) + untracked_rust_lines()
    if not findings:
        print("No new production unwrap/expect/panic calls found.")
        return

    print("New production unwrap/expect/panic calls found:")
    for path, line, source in findings:
        location = f"{path}:{line}" if line is not None else path
        print(f"- {location}: {source}")
    raise SystemExit(1)


if __name__ == "__main__":
    main()
