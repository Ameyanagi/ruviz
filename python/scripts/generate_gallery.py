from __future__ import annotations

import importlib.util
import inspect
import sys
from collections import defaultdict
from pathlib import Path
from types import ModuleType

ROOT = Path(__file__).resolve().parents[1]
EXAMPLES_DIR = ROOT / "examples"
DOCS_DIR = ROOT / "docs"
ASSETS_DIR = DOCS_DIR / "assets" / "gallery"


def discover_example_paths() -> list[Path]:
    return sorted(
        path
        for path in EXAMPLES_DIR.glob("*.py")
        if not path.name.startswith("_")
    )


def load_module(path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(path.stem, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"failed to load example module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[path.stem] = module
    spec.loader.exec_module(module)
    return module


def build_gallery_page(modules: list[ModuleType]) -> str:
    grouped: dict[str, list[ModuleType]] = defaultdict(list)
    for module in modules:
        grouped[module.META.section].append(module)

    lines = [
        "# Gallery",
        "",
        "This page is generated from `python/examples/` by `scripts/generate_gallery.py`.",
        "",
    ]

    for section in sorted(grouped):
        lines.append(f"## {section}")
        lines.append("")
        for module in sorted(grouped[section], key=lambda item: item.META.title):
            meta = module.META
            lines.append(f"### {meta.title}")
            lines.append("")
            lines.append(meta.summary)
            lines.append("")
            if meta.gallery:
                lines.append(f"![{meta.title}](assets/gallery/{meta.slug}.png)")
                lines.append("")
            lines.append(f"`examples/{Path(module.__file__).name}`")
            lines.append("")
            source = inspect.getsource(module).strip()
            lines.append("```python")
            lines.append(source)
            lines.append("```")
            lines.append("")

    return "\n".join(lines)


def main() -> None:
    sys.path.insert(0, str(EXAMPLES_DIR))
    ASSETS_DIR.mkdir(parents=True, exist_ok=True)

    modules: list[ModuleType] = []
    for path in discover_example_paths():
        module = load_module(path)
        if getattr(module, "META", None) is None or not hasattr(module, "build_plot"):
            continue
        modules.append(module)

        if module.META.gallery:
            output = ASSETS_DIR / f"{module.META.slug}.png"
            module.build_plot().save(output)

    (DOCS_DIR / "gallery.md").write_text(build_gallery_page(modules), encoding="utf-8")


if __name__ == "__main__":
    main()
