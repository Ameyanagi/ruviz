from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path
from unittest import mock


SCRIPT = Path(__file__).with_name("check_no_new_production_unwraps.py")
SPEC = importlib.util.spec_from_file_location("check_no_new_production_unwraps", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)


class ProductionUnwrapCheckerTests(unittest.TestCase):
    def test_mask_preserves_code_braces_and_hides_literal_comment_braces(self) -> None:
        source = '''
fn production() {
    let text = r##"} not code {"##;
    // }
    /* { nested /* } */ } */
}
'''
        masked = CHECKER.rust_code_mask(source)
        self.assertEqual(masked.count("{"), 1)
        self.assertEqual(masked.count("}"), 1)
        self.assertEqual(masked.count("\n"), source.count("\n"))

    def test_only_cfg_test_module_lines_are_excluded(self) -> None:
        source = """
fn production() {
    value.expect("must be reported");
}

#[cfg(test)]
mod tests {
    #[test]
    fn unit() {
        result.expect("test-only");
    }
}

fn later_production() {
    other.unwrap();
}
"""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "sample.rs"
            path.write_text(source, encoding="utf-8")
            with mock.patch.object(CHECKER, "ROOT", root):
                excluded = CHECKER.test_only_lines("sample.rs")

        lines = source.splitlines()
        test_expect = next(
            index for index, line in enumerate(lines, start=1) if "test-only" in line
        )
        production_expect = next(
            index for index, line in enumerate(lines, start=1) if "must be reported" in line
        )
        later_unwrap = next(
            index for index, line in enumerate(lines, start=1) if "other.unwrap" in line
        )
        self.assertIn(test_expect, excluded)
        self.assertNotIn(production_expect, excluded)
        self.assertNotIn(later_unwrap, excluded)


if __name__ == "__main__":
    unittest.main()
