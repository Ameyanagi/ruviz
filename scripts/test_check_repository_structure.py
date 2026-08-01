from __future__ import annotations

import unittest

from scripts.check_repository_structure import tracked_structure_violations


class TrackedStructureTests(unittest.TestCase):
    def test_allows_root_files_and_canonical_directories(self) -> None:
        paths = [
            "Cargo.toml",
            "README.md",
            ".github/workflows/ci.yml",
            "adapters/gpui/src/lib.rs",
            "apps/web-demo/package.json",
            "benches/rendering.rs",
            "bindings/python/pyproject.toml",
            "bindings/wasm/Cargo.toml",
            "docs/REPOSITORY_STRUCTURE.md",
            "examples/basic.rs",
            "generated/README.md",
            "packages/ruviz/package.json",
            "proptest-regressions/render.txt",
            "scripts/check_docs.py",
            "src/lib.rs",
            "tests/integration.rs",
            "tools/gallery/basic/line.rs",
        ]

        self.assertEqual(tracked_structure_violations(paths), [])

    def test_rejects_retired_and_unclassified_top_level_directories(self) -> None:
        paths = [
            "benchmarks/results.csv",
            "crates/ruviz-web/Cargo.toml",
            "demo/index.html",
            "gallery/basic/line.rs",
            "python/pyproject.toml",
        ]

        self.assertEqual(
            tracked_structure_violations(paths),
            [
                "benchmarks/results.csv: non-canonical top-level directory benchmarks/",
                "crates/ruviz-web/Cargo.toml: non-canonical top-level directory crates/",
                "demo/index.html: non-canonical top-level directory demo/",
                "gallery/basic/line.rs: non-canonical top-level directory gallery/",
                "python/pyproject.toml: non-canonical top-level directory python/",
            ],
        )

    def test_rejects_retired_nested_package_path(self) -> None:
        self.assertEqual(
            tracked_structure_violations(["packages/ruviz-web/package.json"]),
            [
                "packages/ruviz-web/package.json: retired directory packages/ruviz-web/"
            ],
        )


if __name__ == "__main__":
    unittest.main()
