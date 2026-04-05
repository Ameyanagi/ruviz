#!/bin/bash
# Clean generated output artifacts from ruviz.
#
# Generated preview outputs now live under `generated/`. This script also
# removes a small set of retired legacy output roots if they still exist locally.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "Cleaning ruviz output files..."

if [ -d "$PROJECT_ROOT/generated" ]; then
    find "$PROJECT_ROOT/generated" -mindepth 1 ! -name "README.md" -exec rm -rf {} +
    echo "  Cleared generated/ (preserved generated/README.md)"
fi

for legacy_dir in \
    "$PROJECT_ROOT/examples/output" \
    "$PROJECT_ROOT/tests/output" \
    "$PROJECT_ROOT/test_output" \
    "$PROJECT_ROOT/export_output" \
    "$PROJECT_ROOT/export_test_output" \
    "$PROJECT_ROOT/python/site" \
    "$PROJECT_ROOT/packages/ruviz-web/docs/.vitepress/dist"
do
    if [ -e "$legacy_dir" ]; then
        rm -rf "$legacy_dir"
        local_path="${legacy_dir#"$PROJECT_ROOT"/}"
        echo "  Removed legacy output ${local_path:-$legacy_dir}"
    fi
done

# Clean any scattered files in root (legacy)
find "$PROJECT_ROOT" -maxdepth 1 -type f \
    \( -name "*.png" -o -name "*.pdf" -o -name "*.svg" -o -name "*.gif" -o -name "*.txt" -o -name "*.html" -o -name "*.bin" -o -name "*.csv" \) \
    -delete 2>/dev/null || true

echo "Done!"
