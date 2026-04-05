# Build Outputs

This repository distinguishes between generated preview artifacts and
committed published media.

## Generated Output

Generated preview artifacts go under `generated/`.

The `generated/` tree is for local rebuilds and pull request preview artifacts.
Only `generated/README.md` and `generated/manifest.json` are tracked in git.
Everything else under `generated/` is ignored so frequent binary rebuilds do not
bloat repository history. PR CI uploads the tracked docs-facing preview trees
when the manifest changes.

Primary subdirectories:

- `generated/examples/` for Rust example outputs
- `generated/tests/render/` for optional test render artifacts
- `generated/tests/visual/` for optional visual test output
- `generated/tests/visual-diff/` for optional visual diff failures
- `generated/tests/export/` for optional export-format test artifacts
- `generated/bench/` for gallery, benchmark, and utility image output
- `generated/python/` for built Python docs output
- `generated/web/` for built web docs output
- `generated/reports/` for generated reports

The tracked `generated/manifest.json` intentionally covers only:

- `generated/examples/`
- `generated/python/site/`
- `generated/web/docs/`

Test output under `generated/tests/` is still useful for local debugging, but it
is not part of the default preview manifest or PR artifact upload path.

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
make build-generated-preview
make generated-manifest
make check-doc-asset-refs
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

The cleanup script removes them when they still exist locally before
regeneration.

## Packaging

`generated/` is intentionally excluded from the published root Rust crate via
the workspace package metadata in `Cargo.toml`. The adapter crates under
`crates/` do not package the repository-root `generated/` tree because it sits
outside their crate directories.
