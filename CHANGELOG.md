# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Architecture Refactoring

Major internal refactoring to improve code organization and maintainability.

#### Plot Decomposition

The `Plot` struct has been decomposed into focused component managers:

- **PlotConfiguration** (`src/core/plot/configuration.rs`) - Display settings (title, labels, dimensions, theme)
- **SeriesManager** (`src/core/plot/series_manager.rs`) - Data series storage and auto-coloring
- **LayoutManager** (`src/core/plot/layout_manager.rs`) - Legend, grid, ticks, margins, axis limits
- **RenderPipeline** (`src/core/plot/render_pipeline.rs`) - Backend selection, parallel/pooled rendering

The public API remains unchanged - this is purely an internal refactoring.

#### PlotBuilder API Enhancements

Added missing methods to `PlotBuilder<C>` for better API consistency:

- `legend(Position)` - Set legend position
- `xscale(AxisScale)` / `yscale(AxisScale)` - Set axis scales
- `backend(BackendType)` - Set rendering backend
- `gpu(bool)` - Enable GPU acceleration (requires `gpu` feature)
- `style(LineStyle)` - Set line style (for `LineConfig`)
- `get_backend_name()` - Get current backend name
- `export_svg(path)` - Export to SVG file
- `save_pdf(path)` - Export to PDF file (requires `pdf` feature)
- `save_with_size(path, width, height)` - Save with specific dimensions

#### Code Quality Improvements

- Unified coordinate transform logic via `CoordinateTransform` struct
- Consolidated style re-exports via `ruviz::style` module
- Terminal method macro (`impl_terminal_methods!`) reduces boilerplate
- PlotBuilder now implements `Clone`

### API Changes

#### Deprecated (still work, will warn)

- `Plot::dimensions(w, h)` → Use `size(w, h)` or `size_px(w, h)` instead
- `PlotBuilder::end_series()` → Series finalize automatically; use `.save()` directly

#### Method Renames

- `.width()` on line builders → `.line_width()` for clarity

### Migration Guide

```rust
// Before (deprecated but still works)
Plot::new()
    .line(&x, &y)
    .width(2.0)
    .end_series()
    .dimensions(800, 600)
    .save("plot.png")?;

// After (recommended)
Plot::new()
    .line(&x, &y)
    .line_width(2.0)
    .size_px(800, 600)
    .save("plot.png")?;
```

### Testing

- 761 library tests passing with `--all-features`
- 132 doctests passing
- All benchmarks verified with no performance regression
