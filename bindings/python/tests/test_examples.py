from __future__ import annotations

import importlib.util
import sys
from pathlib import Path


EXAMPLES_DIR = Path(__file__).resolve().parents[1] / "examples"


def _load_example(path: Path):
    spec = importlib.util.spec_from_file_location(path.stem, path)
    assert spec is not None
    assert spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[path.stem] = module
    sys.path.insert(0, str(EXAMPLES_DIR))
    spec.loader.exec_module(module)
    return module


def test_gallery_examples_render_svg() -> None:
    for path in sorted(EXAMPLES_DIR.glob("*.py")):
        if path.name.startswith("_"):
            continue
        module = _load_example(path)
        if getattr(module, "META", None) is None or not module.META.gallery:
            continue

        svg = module.build_plot().render_svg()
        assert svg.startswith("<?xml")
