# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Breaking Changes

- Removed `Plot::latex(...)` API. Use `Plot::typst(true)` for Typst text rendering.

### Added

- Added global Typst text mode via `Plot::typst(bool)` and builder forwarding.
- Added optional `typst-math` feature for Typst-backed text rendering across PNG/SVG/PDF export.
- Added strict Typst error behavior: invalid Typst now fails render/export with `TypstError`.

## [0.1.2] - 2026-01-30

### Platform Fixes

- Fixed macOS and Windows platform build errors (#4)
- Added FreeBSD support (#1)
- Added cross-platform CI build checks for Linux, macOS, Windows, and FreeBSD
- Pinned cross to v0.2.5 with `--locked` for reproducible CI builds

### Contributors

- [@yonas](https://github.com/yonas) - FreeBSD support (#1)
- [@Ameyanagi](https://github.com/Ameyanagi) - Cross-platform build fixes (#4)
