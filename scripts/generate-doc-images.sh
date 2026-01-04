#!/bin/bash
# Generate documentation images from gallery examples
#
# Usage: ./scripts/generate-doc-images.sh
#
# This script runs all gallery examples prefixed with 'doc_' and generates
# PNG images in docs/images/ for use in rustdoc documentation.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
IMAGES_DIR="$PROJECT_ROOT/docs/images"

# Ensure the images directory exists
mkdir -p "$IMAGES_DIR"

echo "Generating documentation images..."
echo "Output directory: $IMAGES_DIR"
echo ""

# Track statistics
total=0
success=0
failed=0

# Find all doc_*.rs examples in the examples directory
for example in "$PROJECT_ROOT"/examples/doc_*.rs; do
    if [ ! -f "$example" ]; then
        echo "No doc_*.rs examples found in examples/"
        exit 0
    fi

    name=$(basename "$example" .rs)
    total=$((total + 1))

    echo "[$total] Generating: $name"
    if cargo run --example "$name" --release 2>&1; then
        success=$((success + 1))
        echo "    ✓ Success"
    else
        failed=$((failed + 1))
        echo "    ✗ Failed"
    fi
    echo ""
done

echo "================================"
echo "Summary: $success/$total succeeded"
if [ $failed -gt 0 ]; then
    echo "Warning: $failed examples failed"
    exit 1
fi
echo "Done. Images saved to $IMAGES_DIR"
