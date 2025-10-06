# Repository Guidelines

## Project Structure & Module Organization
Code lives in `src/`; regression suites sit in `tests/` (`visual/`, `unit/`, GPU, DPI, export). Reference plots and demos are under `gallery/` and `examples/`, with outputs landing in `examples_output/`, `test_output/`, or `export_output/`. Supporting directories include `assets/`, `fonts/`, `docs/`, `scripts/`, and `benches/`.

## Build, Test, and Development Commands
Use `cargo build` for iterative development and `cargo build --release --features full` before performance validation. Run the core suite with `cargo test`, or target a file such as `cargo test --test export_tests_fixed`. Example binaries compile via `cargo run --example subplot_example` and other names in `gallery/`. The Makefile bundles workflows: `make examples` renders the full demo gallery, while `make test-compile` checks interactive builds without running them. Benchmarks execute with `cargo bench --bench memory_pool_benchmarks`.

## Coding Style & Naming Conventions
The project follows `rustfmt` (`rustfmt.toml` enforces 4-space indentation, max width 100, reordered imports). Always run `cargo fmt` before committing. Prefer snake_case for modules/functions, PascalCase for types, and SCREAMING_SNAKE_CASE for constants. Keep public APIs documented with `///` comments when behavior is not obvious. Lint with `cargo clippy --all-targets --all-features`, fixing warnings or justifying them inline.

## Testing Guidelines
Tests rely on the Rust test harness plus visual regression outputs. When adding image-producing tests, ensure they write under `test_output/` or `export_output/` and avoid committing generated artifacts. Follow the patterns in `tests/visual_output_tests_fixed.rs` and `tests/export_tests_fixed.rs` for naming (`test_<behavior>_renders`). Prefer deterministic data to keep pixel diffs stable. Update `TESTING_GUIDE.md` if directories or file conventions change. For performance-sensitive work, include a Criterion entry in `benches/` and capture summary metrics.

## Commit & Pull Request Guidelines
Commits use imperative subject lines (see `git log`: “Fix title centering…”). Group topical changes into focused commits and reference issues with `#<id>` when applicable. Pull requests should describe the motivation, list key changes, note any feature flags used, and include screenshots or generated plots when visuals change. State the commands you ran (`cargo test`, `make examples`, benchmarks) so reviewers can reproduce the results.

## Feature Flags & Runtime Profiles
Feature gating is heavy: `interactive` pulls in winit/wgpu, `gpu` enables hardware acceleration, and `full` bundles every optional capability. Document any new feature flags in `Cargo.toml` comments and expose representative examples in `gallery/`. Use `cargo run --release --features interactive --example basic_interaction` to validate interactive paths, and mention required hardware/driver considerations in your PR if GPU paths are affected.
