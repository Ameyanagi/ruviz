#!/bin/bash
# Clean generated output artifacts from ruviz.
#
# This preserves checked-in `.gitkeep` placeholders while removing generated
# files from the output roots used by tests, examples, and export demos.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

clean_output_root() {
    local dir="$1"
    local label="$2"

    if [ -d "$dir" ]; then
        find "$dir" -type f ! -name ".gitkeep" -delete
        find "$dir" -type l -delete
        find "$dir" -depth -mindepth 1 -type d -empty -delete
        echo "  Cleaned $label/"
    fi
}

echo "Cleaning ruviz output files..."

clean_output_root "$PROJECT_ROOT/tests/output" "tests/output"
clean_output_root "$PROJECT_ROOT/examples/output" "examples/output"
# Keep committed documentation images intact; refresh them with `make doc-images`.
clean_output_root "$PROJECT_ROOT/test_output" "test_output"
clean_output_root "$PROJECT_ROOT/export_test_output" "export_test_output"
clean_output_root "$PROJECT_ROOT/export_output" "export_output"

# Clean any scattered files in root (legacy)
find "$PROJECT_ROOT" -maxdepth 1 -type f \
    \( -name "*.png" -o -name "*.pdf" -o -name "*.svg" -o -name "*.gif" -o -name "*.txt" -o -name "*.html" -o -name "*.bin" -o -name "*.csv" \) \
    -delete 2>/dev/null || true

echo "Done!"
