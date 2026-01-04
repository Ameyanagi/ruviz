#!/bin/bash
# Clean all generated output files from ruviz project

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "Cleaning ruviz output files..."

# Clean tests/output
if [ -d "$PROJECT_ROOT/tests/output" ]; then
    find "$PROJECT_ROOT/tests/output" -type f \( -name "*.png" -o -name "*.pdf" -o -name "*.svg" \) -delete
    echo "  Cleaned tests/output/"
fi

# Clean examples/output
if [ -d "$PROJECT_ROOT/examples/output" ]; then
    find "$PROJECT_ROOT/examples/output" -type f \( -name "*.png" -o -name "*.pdf" -o -name "*.svg" \) -delete
    echo "  Cleaned examples/output/"
fi

# Clean any scattered files in root (legacy)
find "$PROJECT_ROOT" -maxdepth 1 -type f \( -name "*.png" -o -name "*.pdf" -o -name "*.svg" \) -delete 2>/dev/null || true

echo "Done!"
