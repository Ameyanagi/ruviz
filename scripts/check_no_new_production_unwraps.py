#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RISKY_CALL = re.compile(r"\.(unwrap|expect)\s*\(|\bpanic!\s*\(|\btodo!\s*\(|\bunimplemented!\s*\(")
TEST_MODULE = re.compile(r"\bmod\s+[A-Za-z_][A-Za-z0-9_]*\s*\{")


def is_production_path(path: str) -> bool:
    parts = Path(path).parts
    if any(part in {"tests", "examples", "benches", "benchmarks", "docs", "gallery"} for part in parts):
        return False
    if path.endswith("_test.rs") or path.endswith("tests.rs") or path.endswith(".md"):
        return False
    return path.endswith(".rs")


def rust_code_mask(source: str) -> str:
    """Replace Rust comments and literals with spaces while preserving newlines."""
    output: list[str] = []
    index = 0
    block_depth = 0
    state = "code"
    raw_hashes = 0

    while index < len(source):
        char = source[index]
        following = source[index + 1] if index + 1 < len(source) else ""

        if state == "line_comment":
            if char == "\n":
                state = "code"
                output.append("\n")
            else:
                output.append(" ")
            index += 1
            continue

        if state == "block_comment":
            if char == "/" and following == "*":
                block_depth += 1
                output.extend((" ", " "))
                index += 2
            elif char == "*" and following == "/":
                block_depth -= 1
                output.extend((" ", " "))
                index += 2
                if block_depth == 0:
                    state = "code"
            else:
                output.append("\n" if char == "\n" else " ")
                index += 1
            continue

        if state == "string":
            if char == "\\":
                output.append(" ")
                if following:
                    output.append("\n" if following == "\n" else " ")
                    index += 2
                else:
                    index += 1
            elif char == '"':
                output.append(" ")
                index += 1
                state = "code"
            else:
                output.append("\n" if char == "\n" else " ")
                index += 1
            continue

        if state == "raw_string":
            terminator = '"' + ("#" * raw_hashes)
            if source.startswith(terminator, index):
                output.extend(" " for _ in terminator)
                index += len(terminator)
                state = "code"
            else:
                output.append("\n" if char == "\n" else " ")
                index += 1
            continue

        if char == "/" and following == "/":
            output.extend((" ", " "))
            index += 2
            state = "line_comment"
            continue
        if char == "/" and following == "*":
            output.extend((" ", " "))
            index += 2
            block_depth = 1
            state = "block_comment"
            continue

        raw_match = re.match(r"(?:b?r)(#*)\"", source[index:])
        if raw_match:
            token = raw_match.group(0)
            raw_hashes = len(raw_match.group(1))
            output.extend(" " for _ in token)
            index += len(token)
            state = "raw_string"
            continue

        if char == '"' or (char == "b" and following == '"'):
            if char == "b":
                output.append(" ")
                index += 1
            output.append(" ")
            index += 1
            state = "string"
            continue

        output.append(char)
        index += 1

    return "".join(output)


def test_only_lines(path: str) -> set[int]:
    source = (ROOT / path).read_text(encoding="utf-8")
    masked_lines = rust_code_mask(source).splitlines()
    excluded: set[int] = set()
    cfg_test_pending = False

    for index, line in enumerate(masked_lines):
        stripped = line.strip()
        if stripped.startswith("#[cfg") and "test" in stripped:
            cfg_test_pending = True
            continue
        if not cfg_test_pending or not stripped:
            continue
        if not TEST_MODULE.search(stripped):
            cfg_test_pending = False
            continue

        depth = 0
        opened = False
        for module_index in range(index, len(masked_lines)):
            module_line = masked_lines[module_index]
            depth += module_line.count("{")
            if "{" in module_line:
                opened = True
            depth -= module_line.count("}")
            excluded.add(module_index + 1)
            if opened and depth == 0:
                break
        cfg_test_pending = False

    return excluded


def changed_lines(base: str) -> list[tuple[str, int | None, str]]:
    diff = subprocess.check_output(
        ["git", "diff", "--unified=0", base, "--", "*.rs"],
        cwd=ROOT,
        text=True,
    )
    findings: list[tuple[str, int | None, str]] = []
    excluded_lines: dict[str, set[int]] = {}
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
            if current_path and current_path not in excluded_lines:
                excluded_lines[current_path] = test_only_lines(current_path)
            is_test_only = (
                current_line is not None
                and current_line in excluded_lines.get(current_path, set())
            )
            if (
                current_path
                and is_production_path(current_path)
                and not is_test_only
                and RISKY_CALL.search(line)
            ):
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
        excluded = test_only_lines(path)
        for line_number, line in enumerate(source.splitlines(), start=1):
            if line_number not in excluded and RISKY_CALL.search(line):
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
