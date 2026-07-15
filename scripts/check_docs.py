#!/usr/bin/env python3
"""Validate repository-facing Markdown documentation.

The checks are intentionally narrow and deterministic:
- All tracked Markdown files are checked.
- Markdown fences must be balanced.
- Local Markdown links/images in repository-facing docs must resolve.
- The README's first Rust quick-start block must be a complete binary that
  type-checks against the local crate.
- Root README Rust snippets must be complete `main.rs` examples, so checked
  docs cannot pass with partial code that fails when copied into a binary.
- Markdown Rust snippets cannot use `?` inside `fn main()` unless `main`
  returns a fallible type.
- Checked Rust snippets that use `?` must include their own fallible `main`,
  instead of relying on the checker to wrap partial code.
- Markdown code fences marked with `check` are validated:
  - `rust,check` fences are compiled against the local crate.
  - `rust,check,features=gpu+interactive` selects a deterministic feature profile.
  - `ts,check` / `typescript,check` fences are type-checked against the local Web SDK.
  - `python,check` fences are syntax-checked.
- All shell fences are syntax-checked with `bash -n` and are never executed.
- Ignored Rust/TypeScript/shell fences require an explicit `reason=...`.
- Every runnable `examples/**/*.rs` and `gallery/**/*.rs` program must resolve
  to exactly one uniquely named Cargo example target.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from collections import defaultdict
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
MAIN_RE = re.compile(r"\b(?:async\s+)?fn\s+main\s*\([^)]*\)\s*(?P<return>->)?")
RUST_SNIPPET_ASSET_FIXTURES = {
    "../assets/dejavu-sans.ttf": ROOT / "src" / "dejavu-sans.ttf",
}


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
    if roots is None:
        try:
            result = subprocess.run(
                ["git", "ls-files", "*.md"],
                cwd=ROOT,
                check=True,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
        except (FileNotFoundError, subprocess.CalledProcessError):
            roots = MARKDOWN_ROOTS
        else:
            return sorted(
                (ROOT / line).resolve() for line in result.stdout.splitlines() if line
            )

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
        for fence in extract_code_fences(markdown_files())
        if "check" in fence.flags or "compile" in fence.flags
    ]


def fence_parameter(fence: CodeFence, name: str) -> str | None:
    prefix = f"{name.lower()}="
    return next(
        (flag[len(prefix) :] for flag in fence.flags if flag.startswith(prefix)),
        None,
    )


def rust_feature_profile(fence: CodeFence) -> tuple[str, ...]:
    value = fence_parameter(fence, "features")
    if value is None:
        return ()
    return tuple(sorted(feature for feature in value.split("+") if feature))


def check_fence_classification(fences: list[CodeFence]) -> list[str]:
    errors: list[str] = []
    classified_languages = {"rust", "ts", "bash", "sh", "shell"}
    for fence in fences:
        if fence.lang not in classified_languages:
            continue
        checked = "check" in fence.flags or "compile" in fence.flags
        ignored = "ignore" in fence.flags
        if checked and ignored:
            errors.append(f"{fence.label()} cannot be both checked and ignored")
        reason = fence_parameter(fence, "reason")
        if ignored and (reason is None or not reason.strip()):
            errors.append(
                f"{fence.label()} ignored {fence.lang or 'code'} fence requires a non-empty reason=..."
            )
        if (
            fence.lang == "rust"
            and MAIN_RE.search(fence.code)
            and not (checked or ignored)
        ):
            errors.append(
                f"{fence.label()} complete Rust program must be marked check or "
                "ignore with reason=..."
            )
    return errors


def runnable_example_sources() -> set[str]:
    sources = {
        path.relative_to(ROOT).as_posix()
        for directory in (ROOT / "examples", ROOT / "gallery")
        for path in directory.rglob("*.rs")
        if path.name != "mod.rs"
    }
    return sources


def cargo_example_targets(
    manifest: dict | None = None,
    sources: set[str] | None = None,
) -> list[tuple[str, str]]:
    if manifest is None:
        manifest = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    if sources is None:
        sources = runnable_example_sources()

    declared = [
        (example.get("name"), example.get("path"))
        for example in manifest.get("example", [])
        if isinstance(example, dict)
    ]
    targets = [
        (name, path)
        for name, path in declared
        if isinstance(name, str) and isinstance(path, str)
    ]
    declared_paths = {path for _, path in targets}

    package = manifest.get("package", {})
    autoexamples = (
        not isinstance(package, dict) or package.get("autoexamples") is not False
    )
    if autoexamples:
        for source in sorted(sources):
            parts = Path(source).parts
            inferred_name: str | None = None
            if len(parts) == 2 and parts[0] == "examples":
                inferred_name = Path(parts[1]).stem
            elif len(parts) == 3 and parts[0] == "examples" and parts[2] == "main.rs":
                inferred_name = parts[1]

            # A declaration for the same source customizes Cargo's inferred target.
            if inferred_name is not None and source not in declared_paths:
                targets.append((inferred_name, source))
    return targets


def check_example_target_coverage(
    manifest: dict | None = None,
    sources: set[str] | None = None,
) -> list[str]:
    if sources is None:
        sources = runnable_example_sources()
    targets = cargo_example_targets(manifest, sources)

    names: dict[str, list[str]] = defaultdict(list)
    paths: dict[str, list[str]] = defaultdict(list)
    for name, path in targets:
        names[name].append(path)
        paths[path].append(name)

    errors: list[str] = []
    for name, target_paths in sorted(names.items()):
        if len(target_paths) > 1:
            errors.append(
                f"Cargo example target name {name!r} resolves to multiple sources: "
                + ", ".join(sorted(target_paths))
            )
    for path, target_names in sorted(paths.items()):
        if len(target_names) > 1:
            errors.append(
                f"Cargo example source {path!r} is registered by multiple targets: "
                + ", ".join(sorted(target_names))
            )

    covered_paths = set(paths)
    errors.extend(
        f"{source} is runnable Rust but is not covered by a Cargo example target"
        for source in sorted(sources - covered_paths)
    )
    return errors


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


def cargo_project_manifest(
    crate_name: str,
    features: tuple[str, ...] = (),
    binaries: tuple[tuple[str, str], ...] = (),
) -> str:
    dependencies = [f'ruviz = {{ path = "{ROOT}" }}']
    if "ndarray_support" in features:
        dependencies.append('ndarray = "0.17"')
    if "interactive" in features or "interactive-gpu" in features:
        dependencies.append('tokio = { version = "1", features = ["rt", "macros"] }')
    if "serde" in features:
        dependencies.extend(
            [
                'serde = { version = "1", features = ["derive"] }',
                'serde_json = "1"',
            ]
        )
    if "polars_support" in features:
        dependencies.append(
            'polars = { version = "0.50", features = ["lazy", "rolling_window"] }'
        )

    lines = [
        "[package]",
        f'name = "{crate_name}"',
        'version = "0.0.0"',
        'edition = "2024"',
        "",
        "[dependencies]",
        *dependencies,
        "",
    ]
    if features:
        lines.extend(
            [
                "[features]",
                "default = []",
                *(f'{feature} = ["ruviz/{feature}"]' for feature in features),
                "",
            ]
        )
    for name, path in binaries:
        lines.extend(
            [
                "[[bin]]",
                f'name = "{name}"',
                f'path = "{path}"',
                "",
            ]
        )
    return "\n".join(lines)


def cargo_check_environment(cargo_target_dir: Path) -> dict[str, str]:
    environment = os.environ.copy()
    environment["CARGO_TARGET_DIR"] = str(cargo_target_dir.resolve())
    return environment


def docs_cargo_target_dir(environment: dict[str, str] | None = None) -> Path:
    if environment is None:
        environment = os.environ
    configured = environment.get("CARGO_TARGET_DIR")
    if configured:
        target = Path(configured).expanduser()
        return target if target.is_absolute() else ROOT / target
    return ROOT / "target"


def check_readme_quickstart(
    snippet_fences: list[CodeFence], cargo_target_dir: Path
) -> list[str]:
    try:
        fence = readme_quickstart_fence()
    except RuntimeError as error:
        return [str(error)]

    if "check" in fence.flags or "compile" in fence.flags:
        if fence not in snippet_fences:
            return [
                "README.md first Rust block is marked for checking but was not "
                "included in checked Markdown snippets"
            ]
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
            env=cargo_check_environment(cargo_target_dir),
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


def check_readme_rust_snippets_are_complete() -> list[str]:
    errors: list[str] = []
    for fence in extract_code_fences([ROOT / "README.md"]):
        if fence.lang != "rust":
            continue
        if not re.search(r"\bfn\s+main\s*\(", fence.code):
            errors.append(
                f"{fence.label()} README Rust snippet must be a complete "
                "main.rs example with fn main"
            )
            continue
        if "?" in fence.code and not re.search(
            r"\bfn\s+main\s*\([^)]*\)\s*->", fence.code
        ):
            errors.append(
                f"{fence.label()} README Rust snippet uses ? but fn main "
                "does not return Result/Option"
            )
    return errors


def rust_block_after_header(code: str, start: int) -> str | None:
    brace_start = code.find("{", start)
    if brace_start == -1:
        return None

    depth = 0
    for index in range(brace_start, len(code)):
        char = code[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return code[brace_start + 1 : index]
    return None


def check_rust_fence_error_propagation(fences: list[CodeFence]) -> list[str]:
    errors: list[str] = []
    for fence in fences:
        if fence.lang != "rust" or "?" not in fence.code:
            continue

        main_match = MAIN_RE.search(fence.code)
        if main_match is not None:
            main_body = rust_block_after_header(fence.code, main_match.end())
            if (
                main_match.group("return") is None
                and main_body is not None
                and "?" in main_body
            ):
                errors.append(
                    f"{fence.label()} Rust snippet uses ? inside fn main, "
                    "but fn main does not return Result/Option"
                )
            continue

        if "check" in fence.flags or "compile" in fence.flags:
            errors.append(
                f"{fence.label()} checked Rust snippet uses ? but is not a "
                "complete main.rs example"
            )
    return errors


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


def stage_rust_snippet_asset(fence: CodeFence, source_directory: Path) -> list[str]:
    asset = fence_parameter(fence, "asset")
    if asset is None:
        return []

    fixture = RUST_SNIPPET_ASSET_FIXTURES.get(asset)
    if fixture is None:
        return [f"{fence.label()} requests unsupported checked asset {asset!r}"]

    project_directory = source_directory.parent
    destination = (source_directory / asset).resolve()
    if project_directory.resolve() not in destination.parents:
        return [
            f"{fence.label()} checked asset {asset!r} escapes its temporary project"
        ]

    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(fixture, destination)
    return []


def check_rust_snippets(fences: list[CodeFence], cargo_target_dir: Path) -> list[str]:
    profiles: dict[tuple[str, ...], list[CodeFence]] = defaultdict(list)
    for fence in fences:
        profiles[rust_feature_profile(fence)].append(fence)

    errors: list[str] = []
    for profile_index, (features, profile_fences) in enumerate(
        sorted(profiles.items())
    ):
        with tempfile.TemporaryDirectory(
            prefix=f"ruviz-rust-doc-snippets-{profile_index}-"
        ) as temp:
            temp_path = Path(temp)
            generated_snippets: list[tuple[int, CodeFence, str]] = []
            binaries: list[tuple[str, str]] = []
            asset_errors: list[str] = []
            for index, fence in enumerate(profile_fences):
                source = rust_snippet_source(fence.code)
                snippet_directory = temp_path / f"snippet_{index}"
                source_directory = snippet_directory / "src"
                source_directory.mkdir(parents=True)
                (source_directory / "main.rs").write_text(
                    source,
                    encoding="utf-8",
                )
                asset_errors.extend(stage_rust_snippet_asset(fence, source_directory))
                binaries.append((f"snippet_{index}", f"snippet_{index}/src/main.rs"))
                generated_snippets.append((index, fence, source))
            (temp_path / "Cargo.toml").write_text(
                cargo_project_manifest(
                    "ruviz-rust-doc-snippets", features, tuple(binaries)
                ),
                encoding="utf-8",
            )
            if asset_errors:
                errors.extend(asset_errors)
                continue
            command = [
                "cargo",
                "check",
                "--quiet",
                "--bins",
                "--manifest-path",
                str(temp_path / "Cargo.toml"),
            ]
            if features:
                command.extend(["--features", ",".join(features)])
            result = subprocess.run(
                command,
                cwd=ROOT,
                env=cargo_check_environment(cargo_target_dir),
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            if result.returncode == 0:
                continue

            snippets = "\n\n".join(
                "\n".join(
                    [
                        f"--- snippet_{index}: {fence.label()}",
                        f"    generated as snippet_{index}/src/main.rs ---",
                        numbered_source(source),
                    ]
                )
                for index, fence, source in generated_snippets
            )
            profile_label = "+".join(features) if features else "default"
            errors.append(
                f"checked Rust Markdown snippets for profile {profile_label!r} "
                "failed cargo check:\n"
                + snippets
                + "\n\ncargo output:\n"
                + result.stdout
                + result.stderr
            )
    return errors


def check_python_snippets(fences: list[CodeFence]) -> list[str]:
    errors: list[str] = []
    for fence in fences:
        try:
            compile(fence.code, fence.label(), "exec")
        except SyntaxError as error:
            errors.append(f"{fence.label()} Python snippet has invalid syntax: {error}")
    return errors


def check_shell_snippets(fences: list[CodeFence]) -> list[str]:
    errors: list[str] = []
    for fence in fences:
        result = subprocess.run(
            ["bash", "-n"],
            input=fence.code,
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        if result.returncode != 0:
            errors.append(
                f"{fence.label()} shell snippet has invalid syntax:\n{result.stderr}"
            )
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
                        "baseUrl": ".",
                        "paths": {
                            "ruviz": ["ruviz-web/src/index.ts"],
                            "ruviz/raw": ["ruviz-web/generated/raw/ruviz_web_raw.d.ts"],
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
            labels = "\n".join(
                f"  snippet_{index}: {fence.label()}"
                for index, fence in enumerate(fences)
            )
            return [
                "checked TypeScript Markdown snippets failed tsc:\n"
                + labels
                + "\n"
                + result.stdout
                + result.stderr
            ]
    return []


def check_markdown_snippets(
    fences: list[CodeFence], cargo_target_dir: Path
) -> tuple[list[str], int]:
    supported = {"rust", "python", "ts", "bash", "sh", "shell"}
    unsupported = sorted(
        {fence.lang for fence in fences if fence.lang not in supported}
    )
    errors = [
        f"unsupported checked Markdown code fence language: {lang!r}"
        for lang in unsupported
    ]
    errors.extend(
        check_rust_snippets(
            [fence for fence in fences if fence.lang == "rust"], cargo_target_dir
        )
    )
    errors.extend(
        check_python_snippets([fence for fence in fences if fence.lang == "python"])
    )
    errors.extend(
        check_typescript_snippets([fence for fence in fences if fence.lang == "ts"])
    )
    return errors, len(fences)


def main() -> int:
    files = markdown_files()
    all_snippet_fences = extract_code_fences(files)
    snippet_fences = [
        fence
        for fence in all_snippet_fences
        if "check" in fence.flags or "compile" in fence.flags
    ]
    errors = check_fences(files)
    errors.extend(check_local_links(files))
    errors.extend(check_fence_classification(all_snippet_fences))
    errors.extend(check_example_target_coverage())
    errors.extend(check_readme_rust_snippets_are_complete())
    errors.extend(check_rust_fence_error_propagation(all_snippet_fences))
    errors.extend(
        check_shell_snippets(
            [
                fence
                for fence in all_snippet_fences
                if fence.lang in {"bash", "sh", "shell"} and "ignore" not in fence.flags
            ]
        )
    )
    cargo_target_dir = docs_cargo_target_dir()
    errors.extend(check_readme_quickstart(snippet_fences, cargo_target_dir))
    snippet_errors, checked_snippet_count = check_markdown_snippets(
        snippet_fences, cargo_target_dir
    )
    errors.extend(snippet_errors)

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    print(
        f"Validated {len(files)} Markdown files, README Rust snippets, "
        f"and {checked_snippet_count} checked Markdown code fences."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
