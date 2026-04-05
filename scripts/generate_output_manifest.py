#!/usr/bin/env python3
"""Generate a deterministic manifest for tracked preview artifacts."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import struct
from pathlib import Path
from typing import Any
from xml.etree import ElementTree

DEFAULT_INCLUDE_PREFIXES = ("examples/", "python/site/", "web/docs/")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default="generated", help="generated output root")
    parser.add_argument(
        "--output",
        default="generated/manifest.json",
        help="manifest file path",
    )
    parser.add_argument(
        "--include-prefix",
        action="append",
        dest="include_prefixes",
        help=(
            "relative path prefix to include in the manifest; defaults to the "
            "tracked docs-facing preview trees"
        ),
    )
    return parser.parse_args()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_png_dimensions(path: Path) -> dict[str, int] | None:
    with path.open("rb") as handle:
        header = handle.read(24)
    if len(header) < 24 or header[:8] != b"\x89PNG\r\n\x1a\n":
        return None
    width, height = struct.unpack(">II", header[16:24])
    return {"width": width, "height": height}


def parse_gif_dimensions(path: Path) -> dict[str, int] | None:
    with path.open("rb") as handle:
        header = handle.read(10)
    if len(header) < 10 or header[:6] not in {b"GIF87a", b"GIF89a"}:
        return None
    width, height = struct.unpack("<HH", header[6:10])
    return {"width": width, "height": height}


def parse_numeric(value: str | None) -> int | float | None:
    if value is None:
        return None
    match = re.match(r"^\s*([0-9]+(?:\.[0-9]+)?)", value)
    if not match:
        return None
    number = float(match.group(1))
    if number.is_integer():
        return int(number)
    return number


def parse_svg_dimensions(path: Path) -> dict[str, int | float] | None:
    try:
        root = ElementTree.parse(path).getroot()
    except ElementTree.ParseError:
        return None

    width = parse_numeric(root.attrib.get("width"))
    height = parse_numeric(root.attrib.get("height"))
    if width is not None and height is not None:
        return {"width": width, "height": height}

    view_box = root.attrib.get("viewBox")
    if not view_box:
        return None

    parts = re.split(r"[\s,]+", view_box.strip())
    if len(parts) != 4:
        return None

    try:
        view_width = float(parts[2])
        view_height = float(parts[3])
    except ValueError:
        return None

    if view_width.is_integer():
        view_width = int(view_width)
    if view_height.is_integer():
        view_height = int(view_height)

    return {"width": view_width, "height": view_height}


def detect_dimensions(path: Path) -> dict[str, Any] | None:
    suffix = path.suffix.lower()
    if suffix == ".png":
        return parse_png_dimensions(path)
    if suffix == ".gif":
        return parse_gif_dimensions(path)
    if suffix == ".svg":
        return parse_svg_dimensions(path)
    return None


def classify(relative_path: str) -> str:
    if relative_path.startswith("examples/"):
        return "examples"
    if relative_path.startswith("tests/render/"):
        return "tests-render"
    if relative_path.startswith("tests/visual/"):
        return "tests-visual"
    if relative_path.startswith("tests/visual-diff/"):
        return "tests-visual-diff"
    if relative_path.startswith("tests/export/"):
        return "tests-export"
    if relative_path.startswith("bench/"):
        return "bench"
    if relative_path.startswith("python/site/"):
        return "python-site"
    if relative_path.startswith("web/docs/"):
        return "web-docs"
    if relative_path.startswith("reports/"):
        return "reports"
    return "other"


def main() -> int:
    args = parse_args()
    root = Path(args.root)
    output = Path(args.output)
    include_prefixes = tuple(args.include_prefixes or DEFAULT_INCLUDE_PREFIXES)

    files: list[dict[str, Any]] = []
    if root.exists():
        for path in sorted(root.rglob("*")):
            if not path.is_file():
                continue

            relative_path = path.relative_to(root).as_posix()
            if relative_path in {"README.md", "manifest.json"}:
                continue
            if include_prefixes and not any(
                relative_path.startswith(prefix) for prefix in include_prefixes
            ):
                continue

            entry: dict[str, Any] = {
                "path": relative_path,
                "group": classify(relative_path),
                "size_bytes": path.stat().st_size,
                "sha256": sha256_file(path),
            }
            dimensions = detect_dimensions(path)
            if dimensions is not None:
                entry["image"] = dimensions
            files.append(entry)

    output.parent.mkdir(parents=True, exist_ok=True)
    manifest = {
        "schema_version": 1,
        "root": root.as_posix(),
        "file_count": len(files),
        "files": files,
    }
    output.write_text(json.dumps(manifest, indent=2, sort_keys=False) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
