# Testing Guide

This guide reflects the current split between the default fast CI lane and the visual/heavy lane.

## Test Lanes

### Fast deterministic lane (default CI)
Use this lane for PR gating and quick local iteration.

```bash
# 1) Unit tests
cargo test --lib --verbose

# 2) Integration compile gate
cargo test --tests --no-run --verbose

# 3) Fast deterministic integration suites
cargo test --test simple_api_test --test data_format_compatibility_test --test backend_parity_test --verbose

# 4) Doctests
cargo test --doc --verbose
```

The fast integration command proves the default `ndarray_support` + `parallel`
configuration, including the `test_ndarray_data` runtime test.

### Feature-contract matrix (default CI)

Use these focused rows when changing Cargo feature gates. The matrix avoids
rerunning the full default suite, which is already covered by the fast lane.

```bash
# No default features; Typst APIs must remain gated off
cargo test --test typst_feature_gate_ui --no-default-features typst_requires_feature -- --exact

# Canonical ndarray feature
cargo test --test data_format_compatibility_test --no-default-features --features ndarray_support test_ndarray_data -- --exact

# Backward-compatible ndarray feature alias
cargo test --test data_format_compatibility_test --no-default-features --features ndarray test_ndarray_data -- --exact

# Typst APIs enabled and a runtime error path exercised
cargo test --test typst_feature_gate_ui --no-default-features --features typst-math typst_with_feature_compiles -- --exact
cargo test --lib --no-default-features --features typst-math core::plot::tests::typst::test_invalid_typst_snippet_returns_typst_error -- --exact
```

### Visual/heavy lane (manual or scheduled)
Use this lane for output-generating and heavier suites.

```bash
# Visual output suite (writes PNGs to generated/tests/render)
cargo test --test visual_output_tests_fixed

# Export format suite (writes artifacts under generated/tests/export)
cargo test --test export_format_tests

# Heavier validation/property suites
cargo test --test performance_validation
cargo test --test property_tests -- --ignored
```

## Canonical Suites

- Visual output: `tests/visual_output_tests_fixed.rs`
- Export formats: `tests/export_format_tests.rs`

Legacy duplicate variants were retired to keep one canonical file per concern.

## Output Directories

- `generated/tests/render/`: visual plot outputs and related artifacts
- `generated/tests/export/png/`: PNG export artifacts
- `generated/tests/export/svg/`: SVG export artifacts
- `generated/tests/export/raw/`: raw RGBA export artifacts and metadata
- `generated/tests/export/direct/`: direct `SkiaRenderer` export artifacts

See [docs/BUILD_OUTPUTS.md](docs/BUILD_OUTPUTS.md) for the full repository-wide
artifact layout.

## Notes

- Tests now use current sizing APIs (`size_px`) instead of deprecated `dimensions`.
- Series finalization is implicit in normal save/render flows; deprecated `end_series` usage was removed from general test suites.
- Output-producing tests include semantic artifact checks (decode, dimensions, non-empty/non-background content).
