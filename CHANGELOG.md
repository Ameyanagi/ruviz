# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

- _None yet._

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

[Unreleased]: https://github.com/Ameyanagi/ruviz/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/Ameyanagi/ruviz/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Ameyanagi/ruviz/compare/v0.1.5...v0.2.0
[0.1.5]: https://github.com/Ameyanagi/ruviz/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/Ameyanagi/ruviz/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/Ameyanagi/ruviz/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/Ameyanagi/ruviz/compare/v0.1.1...v0.1.2
