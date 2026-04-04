# Testing Guide

This guide reflects the current split between the default fast CI lane and the visual/heavy lane.

## Test Lanes

### Fast deterministic lane (default CI)
Use this lane for PR gating and quick local iteration.

```bash
# 1) Unit tests
cargo test --lib

# 2) Integration compile gate
cargo test --tests --no-run

# 3) Fast deterministic integration suites
cargo test --test simple_api_test --test data_format_compatibility_test --test backend_parity_test

# 4) Doctests
cargo test --doc
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
