# Contributing

## Workflow

- Keep pull requests focused and small enough to review.
- Discuss substantial API changes in an issue or the pull request.
- Update docs and examples when public behavior changes.
- Add tests for changed behavior; run examples when rendering or input changes.

## Setup and local checks

The repository has three Rust workspaces. The root contains the core and
bindings; `adapters/gpui` and `adapters/gui` are independent. Root Cargo commands
do not check either adapter workspace. This keeps ordinary core builds from
resolving the Zed dependency graph.

```sh
make fmt
make clippy
make check-docs
```

`make fmt` checks formatting in all three workspaces. To apply formatting:

```sh
cargo fmt --all
cargo fmt --all --manifest-path adapters/gpui/Cargo.toml
cargo fmt --all --manifest-path adapters/gui/Cargo.toml
```

Choose checks for the code you changed:

| Changed code | Check |
| --- | --- |
| Core Rust | `make clippy`, relevant `cargo test --test … --features …`, and `cargo test --doc` |
| 3D rendering | `cargo test --features 3d,gpu --test three_d_spheres_test --test three_d_parity_test` (requires a GPU adapter) |
| GPUI adapter | `make clippy-gpui` and `cargo test --manifest-path adapters/gpui/Cargo.toml --all-features` |
| egui / Iced / Slint adapters | `make clippy-gui` and `cargo test --manifest-path adapters/gui/Cargo.toml --workspace --all-features` |
| Web SDK / demo | `make check-web`, `bun run build:web-demo`, and `bun run test:web` |
| Documentation | `make check-docs` |
| New Rust test target | Assign it to a CI lane, then run `make check-ci-test-coverage` |

`make check` runs the shared checks. Adapter lint remains explicit so a core-only
change does not build every GUI framework. GPU and browser tests need an available
GPU/browser; CI has dedicated lanes. `bun run install:web-browsers` installs the
Playwright browsers used by the web suite.

For release-facing docs and media refreshes, use `make release-docs` on the
dedicated release docs branch. See [docs/BUILD_OUTPUTS.md](docs/BUILD_OUTPUTS.md)
for the artifact layout and regeneration workflow.

## Pull requests

Describe the resulting behavior, required feature flags, and validation. Include
screenshots or generated output when a visual behavior changes.

## Documentation

- Keep [README.md](README.md) and [docs](docs) consistent with the current API.
- Prefer examples that compile against the current public surface.
- Keep published assets in their canonical committed paths.
- Use `generated/` for local previews and CI artifacts; published docs must not
  depend on that tree.
