#!/usr/bin/env python3
"""Validate repository-facing Markdown documentation.

The checks are intentionally narrow and deterministic:
- Markdown fences must be balanced.
- Local Markdown links/images in repository-facing docs must resolve.
- The README's first Rust quick-start block must be a complete binary that
  type-checks against the local crate.
- Markdown code fences marked with `check` are validated:
  - `rust,check` fences are compiled against the local crate.
  - `ts,check` / `typescript,check` fences are type-checked against the local Web SDK.
  - `python,check` fences are syntax-checked.
"""

from __future__ import annotations

import json
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import unquote


ROOT = Path(__file__).resolve().parents[1]
WEB_PACKAGE = ROOT / "packages" / "ruviz-web"
WEB_SRC = WEB_PACKAGE / "src"
MARKDOWN_ROOTS = [
    ROOT / "README.md",
    ROOT / "docs",
    ROOT / "python" / "README.md",
    ROOT / "python" / "docs",
    ROOT / "python" / "examples" / "README.md",
    ROOT / "packages" / "ruviz-web" / "README.md",
    ROOT / "packages" / "ruviz-web" / "docs",
    ROOT / "packages" / "ruviz-web" / "examples" / "README.md",
]
SNIPPET_MARKDOWN_ROOTS = MARKDOWN_ROOTS
LINK_RE = re.compile(r"!?\[[^\]]*\]\(([^)]+)\)")
FENCE_RE = re.compile(r"^```", re.MULTILINE)
FENCE_BLOCK_RE = re.compile(
    r"^```(?P<info>[^\n`]*)\n(?P<code>.*?)^```\s*$",
    re.DOTALL | re.MULTILINE,
)
RAW_MODULE_STUB = """\
export default function init(input?: unknown): Promise<void>;

export enum WebBackendPreference {
  Auto = 0,
  Cpu = 1,
  Svg = 2,
  Gpu = 3,
}

export class JsPlot {
  [key: string]: any;
  constructor();
}

export class ObservableVecF64 {
  [key: string]: any;
  constructor(values: Float64Array);
}

export class SignalVecF64 {
  [key: string]: any;
  static sineWave(...args: number[]): SignalVecF64;
}

export class WebCanvasSession {
  [key: string]: any;
  constructor(canvas: HTMLCanvasElement);
}

export class OffscreenCanvasSession {
  [key: string]: any;
  constructor(canvas: OffscreenCanvas);
}

export function register_default_browser_fonts_js(): void;
export function register_font_bytes_js(bytes: Uint8Array): void;
export function web_runtime_capabilities(): Record<string, boolean>;
"""


@dataclass(frozen=True)
class CodeFence:
    path: Path
    line: int
    info: str
    lang: str
    flags: frozenset[str]
    code: str

    def label(self) -> str:
        return f"{self.path.relative_to(ROOT)}:{self.line}"


def markdown_files(roots: list[Path] | None = None) -> list[Path]:
    roots = roots or MARKDOWN_ROOTS
    files: list[Path] = []
    seen: set[Path] = set()
    for root in roots:
        if root.is_file():
            candidates = [root]
        else:
            candidates = sorted(root.rglob("*.md"))
        for candidate in candidates:
            resolved = candidate.resolve()
            if resolved not in seen:
                seen.add(resolved)
                files.append(candidate)
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


def parse_fence_info(info: str) -> tuple[str, frozenset[str]]:
    tokens = [token for token in re.split(r"[\s,]+", info.strip()) if token]
    if not tokens:
        return "", frozenset()

    lang = tokens[0].lower()
    if lang in {"typescript", "tsx"}:
        lang = "ts"
    elif lang in {"py", "python3"}:
        lang = "python"
    return lang, frozenset(token.lower() for token in tokens[1:])


def extract_code_fences(files: list[Path]) -> list[CodeFence]:
    fences: list[CodeFence] = []
    for path in files:
        text = path.read_text(encoding="utf-8")
        for match in FENCE_BLOCK_RE.finditer(text):
            info = match.group("info").strip()
            lang, flags = parse_fence_info(info)
            fences.append(
                CodeFence(
                    path=path,
                    line=text.count("\n", 0, match.start()) + 1,
                    info=info,
                    lang=lang,
                    flags=flags,
                    code=match.group("code"),
                )
            )
    return fences


def checked_fences() -> list[CodeFence]:
    return [
        fence
        for fence in extract_code_fences(markdown_files(SNIPPET_MARKDOWN_ROOTS))
        if "check" in fence.flags or "compile" in fence.flags
    ]


def readme_quickstart_fence() -> CodeFence:
    rust_fences = [
        fence
        for fence in extract_code_fences([ROOT / "README.md"])
        if fence.lang == "rust"
    ]
    if not rust_fences:
        raise RuntimeError("README.md does not contain a Rust quick-start block")
    fence = rust_fences[0]
    if "fn main" not in fence.code:
        raise RuntimeError("README.md first Rust block must include fn main")
    return fence


def cargo_project_manifest(crate_name: str) -> str:
    return "\n".join(
        [
            "[package]",
            f'name = "{crate_name}"',
            'version = "0.0.0"',
            'edition = "2024"',
            "",
            "[dependencies]",
            f'ruviz = {{ path = "{ROOT}" }}',
            "",
        ]
    )


def check_readme_quickstart() -> list[str]:
    try:
        fence = readme_quickstart_fence()
    except RuntimeError as error:
        return [str(error)]

    if "check" in fence.flags or "compile" in fence.flags:
        # readme_quickstart_fence has already enforced the reader-facing fn main.
        return []

    with tempfile.TemporaryDirectory(prefix="ruviz-readme-check-") as temp:
        temp_path = Path(temp)
        (temp_path / "src").mkdir()
        (temp_path / "src" / "main.rs").write_text(fence.code, encoding="utf-8")
        (temp_path / "Cargo.toml").write_text(
            cargo_project_manifest("ruviz-readme-check"),
            encoding="utf-8",
        )
        result = subprocess.run(
            [
                "cargo",
                "check",
                "--quiet",
                "--manifest-path",
                str(temp_path / "Cargo.toml"),
            ],
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


def rust_snippet_source(code: str) -> str:
    if re.search(r"\bfn\s+main\s*\(", code):
        return code

    return "\n".join(
        [
            "fn main() -> ruviz::prelude::Result<()> {",
            code,
            "Ok(())",
            "}",
            "",
        ]
    )


def numbered_source(source: str) -> str:
    return "\n".join(
        f"{line_number:>4} | {line}"
        for line_number, line in enumerate(source.splitlines(), start=1)
    )


def check_rust_snippets(fences: list[CodeFence]) -> list[str]:
    if not fences:
        return []

    with tempfile.TemporaryDirectory(prefix="ruviz-rust-doc-snippets-") as temp:
        temp_path = Path(temp)
        src_bin = temp_path / "src" / "bin"
        src_bin.mkdir(parents=True)
        generated_snippets: list[tuple[int, CodeFence, str]] = []
        for index, fence in enumerate(fences):
            source = rust_snippet_source(fence.code)
            (src_bin / f"snippet_{index}.rs").write_text(
                source,
                encoding="utf-8",
            )
            generated_snippets.append((index, fence, source))
        (temp_path / "Cargo.toml").write_text(
            cargo_project_manifest("ruviz-rust-doc-snippets"),
            encoding="utf-8",
        )
        result = subprocess.run(
            [
                "cargo",
                "check",
                "--quiet",
                "--bins",
                "--manifest-path",
                str(temp_path / "Cargo.toml"),
            ],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        if result.returncode != 0:
            snippets = "\n\n".join(
                "\n".join(
                    [
                        f"--- snippet_{index}: {fence.label()}",
                        f"    generated as src/bin/snippet_{index}.rs ---",
                        numbered_source(source),
                    ]
                )
                for index, fence, source in generated_snippets
            )
            return [
                "checked Rust Markdown snippets failed cargo check:\n"
                + snippets
                + "\n\ncargo output:\n"
                + result.stdout
                + result.stderr
            ]
    return []


def check_python_snippets(fences: list[CodeFence]) -> list[str]:
    errors: list[str] = []
    for fence in fences:
        try:
            compile(fence.code, fence.label(), "exec")
        except SyntaxError as error:
            errors.append(f"{fence.label()} Python snippet has invalid syntax: {error}")
    return errors


def check_typescript_snippets(fences: list[CodeFence]) -> list[str]:
    if not fences:
        return []

    if not WEB_SRC.is_dir():
        return [
            "packages/ruviz-web/src is required to type-check TypeScript "
            "Markdown snippets"
        ]

    with tempfile.TemporaryDirectory(prefix="ruviz-ts-doc-snippets-") as temp:
        temp_path = Path(temp)
        web_package = temp_path / "ruviz-web"
        raw_dir = web_package / "generated" / "raw"
        raw_dir.mkdir(parents=True)
        shutil.copytree(WEB_SRC, web_package / "src")
        (raw_dir / "ruviz_web_raw.d.ts").write_text(
            RAW_MODULE_STUB,
            encoding="utf-8",
        )
        (raw_dir / "ruviz_web_raw.js").write_text("", encoding="utf-8")

        files = []
        for index, fence in enumerate(fences):
            snippet_path = temp_path / f"snippet_{index}.ts"
            snippet_path.write_text(fence.code, encoding="utf-8")
            files.append(str(snippet_path))

        tsconfig_path = temp_path / "tsconfig.json"
        tsconfig_path.write_text(
            json.dumps(
                {
                    "compilerOptions": {
                        "target": "ES2022",
                        "lib": ["DOM", "DOM.Iterable", "ES2022"],
                        "module": "ES2022",
                        "moduleResolution": "Bundler",
                        "strict": True,
                        "noEmit": True,
                        "skipLibCheck": True,
                        "baseUrl": str(temp_path),
                        "paths": {
                            "ruviz": [str(web_package / "src" / "index.ts")],
                            "ruviz/raw": [str(raw_dir / "ruviz_web_raw.d.ts")],
                        },
                    },
                    "files": files,
                },
                indent=2,
            ),
            encoding="utf-8",
        )

        try:
            result = subprocess.run(
                [
                    "bun",
                    "run",
                    "--cwd",
                    str(WEB_PACKAGE),
                    "tsc",
                    "-p",
                    str(tsconfig_path),
                ],
                cwd=ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
        except FileNotFoundError:
            return ["bun is required to type-check TypeScript Markdown snippets"]

        if result.returncode != 0:
            labels = "\n".join(f"  snippet_{index}: {fence.label()}" for index, fence in enumerate(fences))
            return [
                "checked TypeScript Markdown snippets failed tsc:\n"
                + labels
                + "\n"
                + result.stdout
                + result.stderr
            ]
    return []


def check_markdown_snippets() -> tuple[list[str], int]:
    fences = checked_fences()
    unsupported = sorted({fence.lang for fence in fences if fence.lang not in {"rust", "python", "ts"}})
    errors = [
        f"unsupported checked Markdown code fence language: {lang!r}"
        for lang in unsupported
    ]
    errors.extend(check_rust_snippets([fence for fence in fences if fence.lang == "rust"]))
    errors.extend(check_python_snippets([fence for fence in fences if fence.lang == "python"]))
    errors.extend(check_typescript_snippets([fence for fence in fences if fence.lang == "ts"]))
    return errors, len(fences)


def main() -> int:
    files = markdown_files()
    errors = check_fences(files)
    errors.extend(check_local_links(files))
    errors.extend(check_readme_quickstart())
    snippet_errors, checked_snippet_count = check_markdown_snippets()
    errors.extend(snippet_errors)

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    print(
        f"Validated {len(files)} Markdown files, README quick-start syntax, "
        f"and {checked_snippet_count} checked Markdown code fences."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
