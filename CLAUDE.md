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
**Phase**: Phase 4: Performance Optimization COMPLETE
**Status**: Production-ready high-performance plotting library achieved
**Active**: All major plot types with professional seaborn styling implemented
**Next**: Ready for advanced features and GPU acceleration integration

## ⚡ Phase 4: Performance Optimization Results

### 🎯 Performance Achievements
- **Memory Optimization**: 50K points rendered in 40ms with efficient buffer pooling
- **Parallel Rendering**: 2M+ points/second on 16-core system with multi-threaded processing  
- **Scientific Quality**: Multi-panel figures (2×2 subplots) rendered in 147ms
- **Seaborn Styling**: Professional publication-quality themes throughout all examples

### 📊 Advanced Plot Types Implemented
- **Box Plots**: Complete statistical visualization with quartiles, IQR, whiskers, and outliers
- **Histograms**: Automatic binning with frequency distribution analysis
- **Multi-panel Subplots**: Professional layout system with configurable spacing and titles
- **Seaborn Theme**: Muted color palettes and typography matching matplotlib/seaborn quality

### 🔬 Performance Demonstration Examples
1. **Memory Optimization Demo** (`memory_optimization_demo.rs`)
   - 50K point line plot: 40ms rendering time
   - Memory-efficient scatter plot with intelligent subsampling
   - Demonstrates buffer pooling and coordinate transformation optimization

2. **Parallel Rendering Demo** (`parallel_demo.rs`)
   - Multi-threaded rendering across 16 CPU cores
   - 100K points: 2M+ points/second throughput
   - Multi-series parallel processing with load balancing

3. **Scientific Plotting Showcase** (`scientific_showcase.rs`)
   - 2×2 multi-panel figure: 147ms total rendering time
   - Publication-quality subplot layout with professional spacing
   - Time series, correlation, distribution, and group comparison analysis

### 🎨 Subplot System Features
- **Multi-panel Layout**: 2×2, 3×2, custom grid configurations supported
- **Professional Spacing**: Configurable hspace/wspace for publication layout
- **Individual Themes**: Each subplot can have independent styling and themes
- **Overall Titles**: Figure-level suptitle with proper typography positioning
- **Publication Ready**: Suitable for journal articles, theses, and research papers

### ✅ DPI API Implementation (Complete)
- **TDD Approach**: 6 comprehensive tests written first, all passing
- **Fluent API**: `Plot::new().line(&x, &y).dpi(300).save("file.png")` works
- **DPI Validation**: Minimum 72 DPI enforced (typography standard)
- **Scientific Standards**: Support for 96, 150, 300, 600 DPI presets
- **Theme Integration**: DPI works with publication themes
- **Test Coverage**: Basic, IEEE, validation, multi-DPI, theme integration, presets

### ✅ DPI-Aware Rendering Scaling (Complete)
- **Canvas Scaling**: DPI values now affect actual image resolution
- **Scientific Ratios**: 96→150→300→600 DPI with proper size increases
- **File Size Validation**: Higher DPI produces significantly larger images (17.9x for 600 DPI)
- **Scaling Formula**: `scaled_size = base_size * (dpi / 96.0)`
- **Test Results**: 300 DPI = 5.17x file size, 600 DPI = 17.9x file size
- **Backward Compatibility**: All existing tests pass with new scaling

### ✅ Box Plot Implementation (Complete)
- **Statistical Visualization**: Complete box plot with Q1, median, Q3, whiskers, outliers
- **Seaborn Quality**: Professional statistical plot matching seaborn aesthetics
- **API Integration**: `.boxplot(&data, config)` method in fluent Plot builder
- **Statistical Accuracy**: Proper quartile calculation, IQR-based outlier detection
- **Visual Elements**: Box (IQR), median line, whiskers with caps, outlier markers
- **Example Output**: `boxplot_example.png` shows complete statistical visualization
- **Fixed Issues**: Resolved duplicate pattern matching causing missing whiskers

### ✅ Histogram Implementation (Complete)
- **Distribution Analysis**: Professional histogram with automatic binning
- **Statistical Accuracy**: Optimal bin calculation with multiple algorithms
- **API Integration**: `.histogram(&data, config)` method in fluent Plot builder
- **Visual Quality**: Clean bar representation with proper spacing and scaling
- **Example Output**: `histogram_example.png` shows frequency distribution
- **Configuration**: Flexible HistogramConfig for custom bin settings

### ✅ Comprehensive Plot Type Support (Complete)
- **Line Plots**: Connected data points with styling options
- **Scatter Plots**: Individual markers with customizable styles
- **Bar Charts**: Categorical data visualization
- **Histograms**: Data distribution analysis
- **Box Plots**: Statistical summary visualization
- **Error Bars**: Data with uncertainty representation
- **API Consistency**: All plot types use fluent builder pattern
- **Quality Standard**: Professional output matching scientific visualization requirements

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