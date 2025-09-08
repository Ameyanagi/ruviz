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
- `fontdue` - Text rendering
- `palette` - Color management  
- `glam` - Math/geometry
- `thiserror/anyhow` - Error handling
- Optional: `ndarray`, `polars`, `winit`

## Performance Targets
- 100K points: <100ms rendering
- 1M points: <1s with optimization
- 100M points: <2s with DataShader
- Memory: <2x data size usage
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
    .xlabel("X Axis") 
    .ylabel("Y Axis")
    .save("output.png")?
```

### Plot Types
- Line plots (connected points)
- Scatter plots (discrete markers)
- Bar charts (categorical data)
- Histograms (binned data)
- Heatmaps (2D data grid)

## Current Implementation Status
**Phase**: Planning complete, ready for implementation
**Next**: Task generation with TDD approach

## Testing Strategy
- TDD: Tests written first, must fail before implementation
- Contract tests: Core API behavior validation
- Integration tests: Real rendering pipeline
- Performance tests: Timing and memory constraints
- Visual regression: Pixel-perfect output validation

## Key Features
- Feature-gated interactivity (winit backend)
- Multiple export formats (PNG, SVG)
- DataShader-style aggregation for large datasets
- Custom themes and styling
- Cross-platform support (Linux, macOS, Windows, WASM)

## Development Guidelines
- Pure Rust implementation (no C dependencies)
- Builder pattern for fluent API
- Automatic optimization for large datasets
- Publication-ready output quality
- Memory pooling for performance
- SIMD utilization where possible

This context is updated incrementally as the project evolves. See `/specs/001-rust-plotting-library/` for detailed planning artifacts.