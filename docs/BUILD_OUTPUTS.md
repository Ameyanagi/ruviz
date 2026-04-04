# Build Outputs

This repository distinguishes between transient generated output and committed
published media.

## Transient Output

Transient files go under `generated/` and should not be committed.

Primary subdirectories:

- `generated/examples/` for Rust example outputs
- `generated/tests/render/` for test render artifacts
- `generated/tests/visual/` for visual test output
- `generated/tests/visual-diff/` for visual diff failures
- `generated/tests/export/` for export-format test artifacts
- `generated/bench/` for gallery, benchmark, and utility image output
- `generated/python/` for built Python docs output
- `generated/web/` for built web docs output
- `generated/reports/` for generated reports

## Committed Media

Committed release-facing media lives in stable paths:

- `docs/assets/readme/` for README-facing images
- `docs/assets/rustdoc/` for rustdoc and guide screenshots/GIFs
- `docs/assets/gallery/rust/` for committed Rust gallery assets
- `python/docs/assets/gallery/` for committed Python gallery assets
- `tests/fixtures/golden/` for visual regression fixtures

## Canonical Regeneration Command

Use the dedicated release-docs workflow on the release docs branch:

```sh
make release-docs
```

Supporting targets:

```sh
make release-docs-rust
make release-docs-python
make release-docs-web
make clean-generated
```

## Legacy Paths

Do not add new writes to these retired roots:

- `examples/output/`
- `tests/output/`
- `test_output/`
- `export_output/`
- `export_test_output/`

The cleanup script removes them when they still exist locally.
