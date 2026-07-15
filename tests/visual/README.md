# Visual Regression Tests

This directory contains infrastructure for visual regression testing of plot types.

## Overview

There are two separate visual-test categories:

- `tests/visual_regression_test.rs` is the deterministic CI gate. It registers `src/dejavu-sans.ttf`, verifies that the bundled bytes are selected as `DejaVu Sans` for plot and figure-level text, and requires exact pixels for every committed ruviz golden fixture.
- `tests/visual_traits_test.rs` and this `tests/visual/` infrastructure are manual, environment-sensitive comparisons against matplotlib references. They are not CI coverage because the external renderer, reference generation, and review thresholds vary by environment.

Do not treat a passing manual visual-traits run as a substitute for the deterministic golden lane.

## Directory Structure

```
tests/visual/
├── README.md           # This file
├── mod.rs              # Rust test infrastructure
└── reference/
    └── matplotlib/     # Reference images from matplotlib
        ├── kde.png
        ├── ecdf.png
        ├── violin.png
        └── ...

generated/tests/render/
├── visual/             # Generated test images
│   ├── kde.png
│   └── ...
└── visual_diff/        # Diff images for failed tests
    └── kde_diff.png
```

## Quick Start

### 1. Generate Reference Images

```bash
# Generate all reference images
uv run python scripts/generate_reference.py

# Generate specific plot type
uv run python scripts/generate_reference.py kde
```

### 2. Run Visual Tests

```bash
# Run all visual tests
cargo test --test visual_traits_test -- --ignored

# Run specific test
cargo test --test visual_traits_test test_kde_visual -- --ignored
```

### 3. Review Results

- Generated images: `generated/tests/render/visual/`
- Diff images (failures): `generated/tests/visual-diff/`
- Compare visually against references
- These output directories are local debug artifacts and are gitignored; only `tests/visual/reference/` is versioned

## Adding New Visual Tests

1. Add reference image generation to `scripts/generate_reference.py`
2. Add test function to `tests/visual_traits_test.rs`
3. Generate new reference: `uv run python scripts/generate_reference.py <plot_type>`
4. Run test to verify: `cargo test --test visual_traits_test test_<plot_type>_visual -- --ignored`

## Test Template

```rust
#[test]
#[ignore = "Visual test - run with: cargo test --test visual_traits_test -- --ignored"]
fn test_myplot_visual() {
    let config = visual::VisualTestConfig::default();

    let result = visual::run_visual_test("myplot", &config, |path| {
        Plot::new()
            .myplot(&data)
            .title("My Plot")
            .save(path)?;
        Ok(())
    });

    if !result.passed {
        panic!("{}", result.assert_message());
    }
}
```

## Reference Image Requirements

- Size: 640x480 pixels (matplotlib default at 100 DPI)
- Format: PNG
- Filename: `{plot_type}.png`
- Located in: `tests/visual/reference/matplotlib/`

## Tolerance

Default tolerance is 5% pixel difference. This accounts for:
- Anti-aliasing differences
- Font rendering variations
- Minor layout adjustments

Adjust tolerance in `VisualTestConfig::default()` if needed.
