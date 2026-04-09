# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

- _None yet._

## [0.4.8] - 2026-04-09

### Fixed

- Notebook widgets now force transparent notebook host surfaces through the VS Code Jupyter wrapper chain, preventing host-injected white backgrounds from surrounding smaller plots in notebook outputs.
- Added browser regression coverage for notebook-like white wrapper shells so the widget stays transparent and content-sized across Chromium, Firefox, and WebKit.

## [0.4.7] - 2026-04-09

### Fixed

- Notebook widgets now shrink-wrap their notebook host container instead of reserving a full-width output box, so smaller plots no longer show a large white notebook area around the figure.
- Added browser regression coverage for roomy, constrained, and manually resized notebook widget hosts so the outer widget box keeps tracking the displayed plot width.

## [0.4.6] - 2026-04-08

### Changed

- Notebook widgets can now be resized directly in notebook outputs with a drag handle; free resize is the default and `Shift` or `Ctrl` preserves the current aspect ratio while resizing.
- The default notebook widget display size now matches the default exported PNG size when `size_px(...)` is not set, while still shrinking proportionally in narrower notebook cells.

### Fixed

- Improved notebook widget context-menu contrast so the right-click export menu remains legible on light notebook surfaces.
- Hardened notebook widget pointer handling for resize drags across mouse, touch, and pen input, including better constrained-resize behavior near minimum sizes.

## [0.4.5] - 2026-04-08

### Fixed

- The `Python Packaging` CI job now installs the `wasm32-unknown-unknown` target before rebuilding the notebook widget bundle, fixing the tag-triggered packaging failure that blocked the `v0.4.4` release workflow.

## [0.4.4] - 2026-04-08

### Changed

- Python release CI/CD now builds and publishes a manylinux `x86_64` wheel so Linux `uv add ruviz` installs can use a prebuilt wheel instead of falling back to a source build.
- The GitHub release workflow now publishes from `refs/tags/<tag>` even for manual recovery runs, and the release runbook now documents GitHub Actions as the single supported publishing path.

### Fixed

- Python source distributions now include the root `benches/` tree required by the workspace `Cargo.toml`, fixing source-install failures caused by missing bench targets during Cargo manifest parsing.
- Added Python packaging CI coverage that smoke-tests both the sdist install path and the Linux wheel artifact path before release.

## [0.4.3] - 2026-04-08

### Changed

- Notebook widgets now use `plot.size_px(width, height)` as their on-screen display size in Jupyter and VS Code notebook outputs, shrinking proportionally when the cell is narrower than the configured width.
- Notebook widget exports now move behind a right-click menu with `Save PNG` and `Save SVG`, removing the always-visible download buttons while keeping right-drag box zoom intact.
- Refreshed the release-facing version snippets, quickstart, and release notes for `0.4.3`.

### Fixed

- Fixed notebook widget sizing so the displayed widget aspect ratio and clamped size now match the exported PNG/SVG output instead of stretching to the notebook width.
- Added browser regression coverage for notebook-like host sizing, right-click export menu behavior, and preserved secondary-button zoom interactions in the blob-backed widget runtime.

## [0.4.2] - 2026-04-08

### Added

- Added live `ObservableSeries` arithmetic and NumPy ufunc derivations in the Python API, with copied observables preserving independent live state.
- Added Python package extras for dataframe workflows (`dataframes`, `pandas`, and `polars`) plus new docs/examples for dataframe input, deepcopy templates, and observable math.

### Changed

- The Python notebook widget now builds from the shared web SDK frontend, and `plot.size_px(width, height)` drives the widget aspect ratio in notebook displays.
- Refreshed the release-facing version snippets, quickstart, and release notes for `0.4.2`.

### Fixed

- Fixed Python copy/deepcopy behavior so copied plots keep independent native plot handles and live observables instead of aliasing shared state.
- Fixed Python CI and preview lanes to invoke the packaged tools consistently and to avoid running the full generated-preview pipeline when unaffected paths change.

## [0.4.1] - 2026-04-07

### Added

- Added physical heatmap extents through `HeatmapConfig::extent(...)`, opt-in cell borders, heatmap-specific log colorbar subtick control, and automatic SymLog `linthresh` derivation.
- Added a dedicated `examples/montecarlo_heatmap.rs` example for a synthetic absorbed-energy style log heatmap.

### Changed

- Heatmap log colorbars now use superscript decade labels again, reserve enough right-side layout space to avoid clipping, keep decade labels centered on their ticks, and draw visible logarithmic subticks by default.
- Log-scaled heatmaps now mask `<= 0` and non-finite cells out of auto range detection instead of coloring them as the minimum valid bin, matching Matplotlib-style `LogNorm` behavior more closely.

### Fixed

- Fixed the new pixel-aligned heatmap and filled-contour fast path so translucent fills keep normal alpha compositing and subpixel-width tiles still contribute visible coverage.
- Fixed interactive heatmap hit testing so masked log-scale cells do not produce hover or selection hits and stale masked hits are dropped during viewport refresh.

## [0.4.0] - 2026-04-05

### Added

- Added empty-plot rendering support across the Rust core renderers, the Python notebook path, and the Python widget/browser session path, so a plot with zero series now renders as a valid empty chart instead of failing.

### Changed

- Reworked tick-aware layout handling so shared layout respects explicit margin modes (`Fixed`, `Auto`, and `Proportional`) instead of collapsing them into the default content-driven layout path.
- Aligned render-time tick layout with configured major-tick settings so `render()`, save/export flows, and interactive rendering use the same tick-count strategy.
- Refreshed release-facing documentation, package READMEs, and generated media for the `0.4.0` release surface across Rust, Python, and npm/web.

### Fixed

- Restored the `center_plot` content-centering option in the layout path and added regression coverage for centered versus asymmetric layouts.
- Fixed the heatmap raster parity regression introduced by the `0.3.5` output-surface fast path by restoring the normal per-cell renderer for heatmaps and adding regression coverage for downsampled narrow-feature visibility.
- Fixed `draw_datashader_image` so image-backed raster paths apply their scale-and-translate transform correctly when blitting into the plot area.
- Fixed the Python widget build bootstrap so wasm-bindgen version lookup is deterministic and concurrent installs do not race on the cached CLI directory.
- Stabilized notebook/widget media regeneration and golden-image refresh flows so release assets can be rebuilt consistently from one documented command.

## [0.3.6] - 2026-04-04

### Added

- Added a Rust-only feature-impact benchmark suite for plotting render/save paths, covering `baseline_cpu`, `default`, `parallel_only`, `parallel_simd`, `performance_alias`, and `gpu_only`, with committed reference artifacts and documentation.
- Added heatmap value scaling through `HeatmapConfig::value_scale(AxisScale)`, including log-aware color normalization, colorbar ticks, and a dedicated `examples/heatmap_scale_reversed_axes.rs` example.

### Changed

- Descending manual axis limits such as `.xlim(4.0, 0.0)` and `.ylim(16.0, 0.0)` are now preserved across static rendering and interactive viewport updates instead of being normalized away.
- Updated the README, quickstart, and guide snippets to point at the `0.3.6` release and document reversed manual limits plus log-scaled heatmaps.

### Fixed

- Fixed `Plot::xlim()` and `Plot::ylim()` dropping inverted manual bounds when `min > max`, including interactive zoom, pan, and zoom-rect flows.
- Fixed heatmap colorbar placement so non-linear heatmap value scales render ticks and labels in the correct transformed positions.

## [0.3.5] - 2026-04-04

### Added

- Added a cross-runtime large-dataset benchmark suite covering Rust, Python, and wasm `ruviz`, plus Python `matplotlib` and Rust `plotters` comparison baselines.
- Added committed benchmark reference artifacts and documentation for the large-dataset plotting suite, including methodology, environment metadata, and report generation.

### Changed

- Accelerated raster PNG export for large datasets with automatic line envelope reduction, cached static histogram bins, and output-surface heatmap rasterization fast paths.
- Reworked the Python binding to keep native plot handles and prepared render state alive across renders, removing the old JSON snapshot round-trip from the hot render/export path.
- Extended the Python benchmark path and benchmark reporting so `render_only` measures real uncached rendering from a reused built plot, with smoke outputs separated from the published reference artifacts.

### Fixed

- Fixed Python static histograms to use Rust's cached static histogram path instead of the slower source-backed reactive path.
- Fixed nearest-neighbor heatmap fast-path sampling so the last source row and column are included in downsampled raster exports.
- Hardened the benchmark report generator against partial runtime result sets and corrected the benchmark statistics and probe-timing methodology used in the published report.

## [0.3.4] - 2026-04-03

### Added

- Added a notebook-safe `ruviz-web` runtime entrypoint plus a self-contained `anywidget` frontend bundle for the Python package, so Jupyter widgets no longer depend on worker loading or `import.meta.url`.
- Added browser regression coverage for blob-backed widget loading, widget export behavior, and single-point sine-signal snapshots in the notebook runtime path.

### Changed

- Bare `Plot` output and `plot.show()` now render a static PNG by default in Jupyter notebooks, while interactive notebook rendering is explicitly opt-in via `plot.widget()`.
- Python CI now verifies the notebook widget bundle is reproducible on the Linux runner, and the release workflow rebuilds one canonical Linux widget bundle before packaging Python artifacts.

### Fixed

- Fixed Jupyter/VS Code widget loading by bundling the notebook frontend as a single Bun-built module with inline WASM bytes instead of runtime-relative imports.
- Fixed notebook widget edge cases around single-point sine signals, export download URL lifetime, and duplicate image rendering from `plot.show()` in notebook cells.
- Fixed release determinism for widget builds by pinning the Rust and `wasm-pack` toolchains used to generate the Python notebook bundle.

## [0.3.3] - 2026-04-01

### Fixed

- Stopped publishing the plain `linux_x86_64` Python wheel to PyPI, since PyPI rejects that host-native Linux platform tag for public uploads.
- Kept the Python release path on PyPI by publishing the source distribution plus macOS Intel, macOS Apple Silicon, and Windows wheels while Linux falls back to source installs for now.

## [0.3.2] - 2026-04-01

### Fixed

- Updated the Python release packaging lane to use `maturin` `1.12.6`, fixing the duplicate-README source distribution failure seen in the `0.3.1` CI/CD release.
- Switched the macOS Intel Python wheel lane to the supported `macos-15-intel` GitHub-hosted runner so the unified release workflow can produce Intel macOS wheels again.

## [0.3.1] - 2026-04-01

### Added

- Added the first tag-driven PyPI publishing path for the `ruviz` Python package, including trusted publishing from GitHub Actions and multi-platform wheel builds.
- Added dedicated Python CI coverage so `uv sync`, the native extension tests, package tests, and Ruff linting run on every PR and release tag.
- Added published Python examples, docs, and notebook widget support alongside the browser runtime bridge bundled into the Python package.

### Changed

- Aligned the unified release workflow across crates.io, npm, and PyPI with shared version validation and release-note handling.

### Fixed

- Fixed mixed named and unnamed radar-series handling in the browser/web bridge so partial labels no longer fail render or mount.
- Fixed Python observable listener cleanup so discarded plots do not stay strongly referenced by long-lived observables.

## [0.3.0] - 2026-04-01

### Breaking Changes

- Raised the repository MSRV to Rust `1.92`. Builds on older toolchains now fail earlier instead of drifting behind CI and release validation.

### Added

- Added Linux and Windows desktop bootstrap support for `ruviz-gpui` examples and integrations, alongside the existing macOS path.
- Added explicit `ruviz-gpui` example compilation checks to CI and release validation so desktop integration regressions surface before publishing.

### Changed

- Switched the workspace GPUI patching flow to an upstream `zed-industries/zed` revision while keeping the required macOS right-drag fix pinned consistently across the workspace.
- Split oversized plotting, rendering, and observable modules into focused internal submodules while preserving the public module paths and re-exports.
- Defaulted contributor toolchains to the latest stable Rust via `rust-toolchain.toml`, while retaining a dedicated CI lane that enforces the `1.92` MSRV floor.

### Fixed

- Made GPUI examples fail cleanly in headless desktop environments, with session-specific hints for Linux (`DISPLAY` / `WAYLAND_DISPLAY`) and Windows desktop sessions.
- Refreshed the committed README quickstart image so the top-level documentation matches current rendering output again.

## [0.2.0] - 2026-03-31

### Breaking Changes

- Typst selection is now compile-time gated behind `typst-math`. `Plot::typst(true)`, builder forwarding to `.typst(true)`, and `TextEngineMode::Typst` are unavailable unless the feature is enabled. If your crate makes Typst optional, guard those calls with `#[cfg(feature = "typst-math")]` instead of expecting a runtime `FeatureNotEnabled` error. Without the feature, `.typst(true)` now fails with a compile error such as `no method named 'typst' found`.

### Added

- Added experimental browser and `wasm32` support via the `ruviz-web` crate and the public npm `ruviz` SDK.
- Added mixed-coordinate inset rendering so Cartesian plots can embed polar, pie, and radar series with configurable inset layout.
- Added builder chaining parity for common continuation flows and styled annotations, removing explicit `end_series()` workarounds in the supported cases.

### Changed

- Unified browser package naming on `ruviz` and made the JS workspace Bun-first for build, lint, and packaging flows.
- Aligned raster and SVG mixed-inset rendering behavior, including clipping, DPI-scaled strokes, and auto-placement spacing.
- Restored and committed golden-image visual fixtures so release validation has stable baseline artifacts again.

### Fixed

- Stabilized interactive zoom/pan, wheel direction, context menu, and save/copy shortcuts across the interactive and GPUI paths.
- Fixed ndarray view recursion and several export-path DPI, validation, and overwrite edge cases in PNG/SVG rendering.
- Fixed browser session timing and destroy races, and kept wasm/browser builds continuously checked in CI.

## [0.1.5] - 2026-03-23

### Breaking Changes

- Default tick marks now render on all four sides of the plot frame instead of only the bottom and left axes. To preserve the previous look, call `.ticks_bottom_left()`.

### Added

- Added `ruviz-gpui`, a GPUI component adapter crate for interactive and reactive plotting integrations.
- Added GPUI interactive session support and reactive plotting hooks for embedded and streaming use cases.

### Changed

- Preserved the public `SubscriberCallback` API while moving runtime subscription dispatch to internal shared callbacks.
- Raster DPI now changes output density without intentionally enlarging fonts, line widths, marker sizes, or layout spacing. If you tuned visuals around the old DPI-coupled output, re-check explicit `.line_width(...)`, `.marker_size(...)`, and font-size settings.

### Fixed

- Fixed GPUI reactive rendering issues around interactive invalidation, streaming redraws, overlay refresh, and source setter updates.
- Fixed manual axis-limit handling in the GPUI/reactive plotting path.
- Eagerly release `lift2` cross-source subscriptions when either source is dropped.
- Prevent `lift2` source-drop cleanup hooks from accumulating on long-lived source observables.
- Validate floating-point DPI values directly before rendering, including negative and fractional out-of-range inputs.
- Keep `set_output_pixels` geometry consistent with the actual configured DPI, even on invalid pre-validation states.
- Retry atomic temp-file creation on stale collisions and document why stale-temp cleanup is safe.
- Reuse the same per-series validation for saved snapshots so reactive saves keep NaN and error-bar checks aligned with render validation.
- Validate rendered reactive snapshots after capture so render, SVG export, and external renderer paths stop re-reading live series for validation.
- Preserve existing Windows export targets on overwrite failures by using native replace semantics and keeping the temporary file for recovery.
- Evict stale Typst cache entries when a replacement grows beyond the cache byte limit, including oversized render results that skip recaching.
- Restore snapshot-based bounds calculation for heatmap, density, polar, radar, contour, and other non-Cartesian series.
- Restore series validation before DataShader and parallel render fast paths, and preserve POSIX symlink destinations during atomic export overwrites.
- Make DataShader renders consume the same validated snapshot as the main render path, and keep invalid zero-DPI pixel sizing from surfacing misleading dimension errors.
- Tighten DataShader bounds handling for the reactive/interactive rendering path.
- Apply tick-side and tick-direction settings consistently in `render()`-based outputs, keep SVG frame strokes DPI-aware when ticks are disabled, and preserve exact framebuffer sizes on fractional HiDPI interactive surfaces.

## [0.1.4] - 2026-02-11

### Added

- Added grouped series API via `Plot::group(|g| ...)` for shared styling across line/scatter/bar series.
- Added grouped legend collapse with `group_label(...)` so grouped series render as a single legend entry.

### Changed

- Group auto-color behavior now reuses one palette-generated color for all group members when no fixed group color is set.
- Updated release documentation workflow to support versioned release notes under `docs/releases/`.

### Fixed

- Made dashed line spacing DPI-independent for consistent appearance across output resolutions.

## [0.1.3] - 2026-02-10

### Breaking Changes

- Removed `Plot::latex(...)` API. Use `Plot::typst(true)` for Typst text rendering.

### Added

- Added global Typst text mode via `Plot::typst(bool)` and builder forwarding.
- Added optional `typst-math` feature for Typst-backed text rendering across PNG/SVG/PDF export.
- Added strict Typst error behavior: invalid Typst now fails render/export with `TypstError`.
- Added ecosystem data ingestion support for `polars`, `ndarray`, and `nalgebra`.

### Changed

- Improved Typst text layout fidelity by aligning baseline/anchor semantics across layout, raster, and SVG paths.
- Fixed Typst title/label clipping and spacing drift in visual outputs (no public API changes).
- Removed Typst raster text oversampling and simplified native-scale raster handling.
- Stabilized the test suite and split CI into focused lanes for more reliable export and visual checks.

## [0.1.2] - 2026-01-30

### Platform Fixes

- Fixed macOS and Windows platform build errors (#4)
- Added FreeBSD support (#1)
- Added cross-platform CI build checks for Linux, macOS, Windows, and FreeBSD
- Pinned cross to v0.2.5 with `--locked` for reproducible CI builds

### Contributors

- [@yonas](https://github.com/yonas) - FreeBSD support (#1)
- [@Ameyanagi](https://github.com/Ameyanagi) - Cross-platform build fixes (#4)

[Unreleased]: https://github.com/Ameyanagi/ruviz/compare/v0.4.8...HEAD
[0.4.8]: https://github.com/Ameyanagi/ruviz/compare/v0.4.7...v0.4.8
[0.4.7]: https://github.com/Ameyanagi/ruviz/compare/v0.4.6...v0.4.7
[0.4.6]: https://github.com/Ameyanagi/ruviz/compare/v0.4.5...v0.4.6
[0.4.5]: https://github.com/Ameyanagi/ruviz/compare/v0.4.4...v0.4.5
[0.4.4]: https://github.com/Ameyanagi/ruviz/compare/v0.4.3...v0.4.4
[0.4.3]: https://github.com/Ameyanagi/ruviz/compare/v0.4.2...v0.4.3
[0.4.2]: https://github.com/Ameyanagi/ruviz/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/Ameyanagi/ruviz/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/Ameyanagi/ruviz/compare/v0.3.6...v0.4.0
[0.3.6]: https://github.com/Ameyanagi/ruviz/compare/v0.3.5...v0.3.6
[0.3.5]: https://github.com/Ameyanagi/ruviz/compare/v0.3.4...v0.3.5
[0.3.4]: https://github.com/Ameyanagi/ruviz/compare/v0.3.3...v0.3.4
[0.3.3]: https://github.com/Ameyanagi/ruviz/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/Ameyanagi/ruviz/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/Ameyanagi/ruviz/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/Ameyanagi/ruviz/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Ameyanagi/ruviz/compare/v0.1.5...v0.2.0
[0.1.5]: https://github.com/Ameyanagi/ruviz/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/Ameyanagi/ruviz/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/Ameyanagi/ruviz/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/Ameyanagi/ruviz/compare/v0.1.1...v0.1.2
