from __future__ import annotations

import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from scripts.check_repository_outputs import (
    gallery_output_violations,
    tracked_output_violations,
)


class TrackedOutputTests(unittest.TestCase):
    def test_rejects_outputs_in_source_oriented_paths(self) -> None:
        paths = [
            "preview.png",
            "examples/demo.svg",
            "tools/gallery/basic/render.png",
            "generated/bench/render.png",
            "generated/notes.txt",
            "scripts/__pycache__/check.cpython-312.pyc",
            "tests/output/result.pdf",
        ]

        self.assertEqual(tracked_output_violations(paths), sorted(paths))

    def test_allows_published_assets_and_test_references(self) -> None:
        paths = [
            "docs/assets/gallery/rust/basic/line.png",
            "python/docs/assets/gallery/line.png",
            "tests/fixtures/golden/line.png",
            "tests/visual/reference/matplotlib/line.png",
            "generated/manifest.json",
        ]

        self.assertEqual(tracked_output_violations(paths), [])


class GalleryDestinationTests(unittest.TestCase):
    def test_rejects_literal_save_destination_outside_generated(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary)
            source = repository / "tools" / "gallery" / "basic" / "demo.rs"
            source.parent.mkdir(parents=True)
            source.write_text(
                'plot.save("tools/gallery/basic/demo.png")?;\n'
                'plot.save_with_size(format!("output/demo.svg"), 10, 10)?;\n',
                encoding="utf-8",
            )

            violations = gallery_output_violations(repository)

        self.assertEqual(
            violations,
            [
                "tools/gallery/basic/demo.rs:1: tools/gallery/basic/demo.png",
                "tools/gallery/basic/demo.rs:2: output/demo.svg",
            ],
        )

    def test_allows_generated_save_destination(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary)
            source = repository / "tools" / "gallery" / "basic" / "demo.rs"
            source.parent.mkdir(parents=True)
            source.write_text(
                'plot.save("generated/bench/demo.png")?;\n', encoding="utf-8"
            )

            self.assertEqual(gallery_output_violations(repository), [])


class CleanupScriptTests(unittest.TestCase):
    def test_preserves_generated_control_files(self) -> None:
        source_script = Path(__file__).with_name("clean-outputs.sh")
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary)
            scripts = repository / "scripts"
            generated = repository / "generated"
            scripts.mkdir()
            generated.mkdir()
            shutil.copy2(source_script, scripts / source_script.name)

            (generated / "README.md").write_text("docs", encoding="utf-8")
            (generated / "manifest.json").write_text("{}", encoding="utf-8")
            (generated / "bench").mkdir()
            (generated / "bench" / "render.png").write_bytes(b"render")
            (generated / "temporary.txt").write_text("output", encoding="utf-8")
            (repository / "legacy.png").write_bytes(b"legacy")

            subprocess.run(
                ["bash", str(scripts / source_script.name)],
                check=True,
                capture_output=True,
                text=True,
            )

            self.assertTrue((generated / "README.md").is_file())
            self.assertTrue((generated / "manifest.json").is_file())
            self.assertFalse((generated / "bench").exists())
            self.assertFalse((generated / "temporary.txt").exists())
            self.assertFalse((repository / "legacy.png").exists())


if __name__ == "__main__":
    unittest.main()
