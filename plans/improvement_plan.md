# Codebase Improvement Plan

1. **Modularize Core Plotting API**
   - Extract `Plot` series management, layout, and rendering orchestration into focused modules under `src/core/` (e.g., `series.rs`, `layout.rs`) to replace the 2k+ LOC monolith in `src/core/plot.rs` and activate the placeholder builder utilities in `src/core/builder.rs`.
   - Introduce a non-consuming builder (`&mut self` API) plus `PlotSpec` structs so plot creation no longer clones every dataset (`Plot::line`, `Plot::scatter`) and can surface validation errors as `Result` instead of comments.

2. **Solidify Rendering Backends**
   - Finish the tiny-skia path by wiring the embedded `src/dejavu-sans.ttf` as a real fallback, remove `println!` diagnostics in `src/render/skia.rs`, and add font cache configuration to `CosmicTextRenderer` for predictable text metrics.
   - Align CPU (`SkiaRenderer`, `PooledRenderer`, `ParallelRenderer`) and GPU (`src/render/gpu/`) pipelines behind a common trait that accepts typed render tasks, eliminating duplicated stroke/marker code and enabling feature-flag parity tests.

3. **Data & Memory Layer Hardening**
   - Audit `Data1D` usage to support zero-copy slices and `ndarray` views, ensuring adapters in `src/data/impls.rs` and `src/data/zero_copy.rs` cover common numeric types without falling back to `Vec` copies.
   - Integrate the adaptive memory manager (`src/data/adaptive.rs`) with plot lifecycles, wiring telemetry from `MemoryProfiler` into optional logging to catch leaks and fragmentation when rendering large scenes.

4. **Plot Type Implementations & Composition**
   - Replace stubs in `src/plots/*.rs` with reusable primitives (line, scatter, bar, box, heatmap) that emit `PlotElementStorage` entries; ensure stacked/composed plots share scales and legends.
   - Add high-level figure helpers (`SubplotFigure`, `GridSpec`) that synthesize shared axes, colorbars, and spacing rules akin to Matplotlib/Makie, leaning on `src/layout/` utilities.

5. **Testing & Validation Enhancements**
   - Build deterministic golden-image tests via headless renders (PNG & SVG) with pixel hashing and structured metadata under `test_output/`, expanding the current `*_fixed.rs` coverage to include legends, DPI scaling, and GPU parity.
   - Add Criterion benchmarks for new render paths (`plot.render().save`) and memory-pool adaptations, wiring them into CI via the existing `Makefile` targets.

6. **User-Facing Ergonomics & Documentation**
   - Revise `README.md` and add narrative guides in `docs/` that demonstrate Matplotlib-like workflows (e.g., `figure.subplots().plot(...)`), including end-to-end examples per feature flag.
   - Provide template scripts in `examples/` showcasing DataFrame (`polars`, `ndarray`) ingestion, interactive GPU rendering (`--features interactive,gpu`), and publication-quality exports, keeping outputs tracked via `examples_output/` previews.

7. **CI, Tooling, and Packaging**
   - Define a cross-platform CI matrix (Linux/macOS/Windows, CPU & optional GPU features) running `cargo fmt`, `cargo clippy --all-targets --all-features`, tests, and selective examples to guard regressions.
   - Prepare for crates.io release: audit feature gating defaults, trim binary artifacts from the crate (`src/dejavu-sans.ttf` via `include_bytes!`), and script license/compliance checks before publishing.
