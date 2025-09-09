# Claude Code Context: Rust Plotting Library

## Project Overview
High-performance 2D plotting library for Rust combining matplotlib's comprehensiveness with Makie's performance. Zero unsafe in public API, <100ms for 100K points, publication-quality output.

## Architecture
- **Language**: Rust 2021 Edition (1.75+)
- **Backend**: tiny-skia for CPU rendering, future GPU with wgpu
- **API**: Builder pattern with method chaining, future Grammar of Graphics
- **Modules**: core, data, render, plots, axes, layout, text, export

## Key Dependencies
- `tiny-skia` - Core rendering (4-7x faster than alternatives)
- `fontdue` - TTF font parsing and rasterization
- `reqwest` - Open font downloading
- `directories` - Cross-platform cache paths
- `palette` - Color management  
- `glam` - Math/geometry
- `thiserror/anyhow` - Error handling
- Optional: `ndarray`, `polars`, `winit`

## Performance Targets
- 100K points: <100ms rendering
- 1M points: <1s with optimization
- 100M points: <2s with DataShader
- Font loading: <50ms system fonts, <100ms open fonts
- Text rendering: <100ms for 1000+ text elements
- Memory: <2x data size usage, <10MB font cache
- Compile: <30s full library

## Core Types

### Data Traits
```rust
pub trait Data1D {
    type Item: Into<f64>;
    fn len(&self) -> usize;
    fn get(&self, idx: usize) -> Option<Self::Item>;
}
```

### Builder API
```rust
Plot::new()
    .line(&x, &y)
    .title("Plot Title")
    .title_font("Arial", 16.0)        // System font
    .xlabel("X Axis") 
    .xlabel_font("Open Sans", 12.0)   // Open font (auto-download)
    .ylabel("Y Axis")
    .ylabel_font_file(&path, 12.0)    // Custom TTF file
    .save("output.png")?
```

### Plot Types
- Line plots (connected points)
- Scatter plots (discrete markers)
- Bar charts (categorical data)
- Histograms (binned data)
- Heatmaps (2D data grid)

## TTF Font Rendering System

### Three Font Sources
1. **System Fonts**: Arial, Times New Roman, platform-specific fonts
2. **Open Fonts**: Auto-downloaded from Google Fonts (Open Sans, Roboto)
3. **Custom Fonts**: User-provided TTF files

### Font Architecture
```rust
FontSource → FontCache → LoadedFont → GlyphCache → RenderedText
```

### Key Features
- **Cross-platform discovery**: Windows registry, macOS paths, Linux fontconfig
- **Automatic fallback**: System → Open → Default embedded fonts
- **UTF-8 Unicode support**: International characters, emoji, combining marks
- **Anti-aliased rendering**: Gamma correction, proper alpha blending
- **Content-addressable cache**: SHA-256 hashing, LRU eviction
- **Baseline alignment**: Proper typography positioning
- **Memory optimization**: Glyph atlas, font instance caching

### Font Configuration
```rust
let config = FontConfig::builder()
    .system_font("Arial")
    .size(12.0)
    .weight(FontWeight::Bold)
    .style(FontStyle::Italic)
    .color(Color::new(0, 0, 0, 255))
    .alignment(TextAlignment::Center)
    .build()?;
```

## Current Implementation Status
**Phase**: Scientific plotting DPI API implementation complete
**Status**: DPI fluent API implemented using TDD methodology  
**Active**: DPI method works in fluent chain: `.dpi(u32).save("")`
**Next**: Implement DPI-aware rendering scaling for actual resolution changes

### ✅ DPI API Implementation (Complete)
- **TDD Approach**: 6 comprehensive tests written first, all passing
- **Fluent API**: `Plot::new().line(&x, &y).dpi(300).save("file.png")` works
- **DPI Validation**: Minimum 72 DPI enforced (typography standard)
- **Scientific Standards**: Support for 96, 150, 300, 600 DPI presets
- **Theme Integration**: DPI works with publication themes
- **Test Coverage**: Basic, IEEE, validation, multi-DPI, theme integration, presets

## Scientific Plotting Enhancement Plan
**Goal**: Transform ruviz into publication-ready scientific plotting library
**Key improvements**: IEEE/Nature themes, accessibility-tested color palettes, mathematical typography, DPI consistency
**TDD Requirement**: All features must follow Red-Green-Refactor cycle with failing tests first
**Directory Structure**: `test_output/` for test images (gitignored), `gallery/` for examples (committed)

## Testing Strategy
- **TDD Mandatory**: Tests written first, must fail before implementation
- **Contract tests**: Core API behavior validation  
- **Integration tests**: Real rendering pipeline
- **Performance tests**: Timing and memory constraints
- **Visual regression**: Pixel-perfect output validation
- **Test outputs**: All test images go to `test_output/` (gitignored)
- **Example outputs**: Example program outputs go to `gallery/` (committed)

## Key Features
- **Professional Typography**: TTF font rendering with system/open/custom sources
- **UTF-8 Unicode Support**: International text, emoji, combining characters
- **Feature-gated interactivity** (winit backend)
- **Multiple export formats** (PNG, SVG) with font quality preservation
- **DataShader-style aggregation** for large datasets
- **Custom themes and styling** with comprehensive font configuration
- **Cross-platform support** (Linux, macOS, Windows, WASM)

## Development Guidelines
- Pure Rust implementation (no C dependencies)
- Builder pattern for fluent API
- Automatic optimization for large datasets
- Publication-ready output quality
- Memory pooling for performance
- SIMD utilization where possible

This context is updated incrementally as the project evolves. See `/specs/001-can-you-please/` for TTF font rendering specification and `/specs/001-rust-plotting-library/` for core plotting artifacts.

## Font Rendering Implementation Notes
- **Text alignment fixed**: Proper baseline positioning with fontdue integration
- **Gamma correction**: Applied `powf(1.0/2.2)` for anti-aliasing
- **Alpha blending**: Premultiplied alpha with tiny-skia PremultipliedColorU8
- **Cross-platform fonts**: Adwaita Sans (Linux), Arial (Windows/macOS)
- **Fallback chain**: System → Open download → Default embedded
- **Performance**: Font caching, glyph atlas, memory optimization planned