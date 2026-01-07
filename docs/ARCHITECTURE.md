# Architecture Overview

This document describes the internal architecture of ruviz.

## Core Components

### Plot Structure

The `Plot` struct is the main entry point for creating visualizations. It has been decomposed into focused component managers:

```mermaid
classDiagram
    class Plot {
        -PlotConfiguration display
        -SeriesManager series_mgr
        -LayoutManager layout
        -RenderPipeline render
        -Vec~Annotation~ annotations
        +new() Plot
        +line() PlotBuilder
        +scatter() PlotBuilder
        +save() Result
        +render() Result
    }

    class PlotConfiguration {
        -Option~String~ title
        -Option~String~ xlabel
        -Option~String~ ylabel
        -(u32, u32) dimensions
        -u32 dpi
        -Theme theme
    }

    class SeriesManager {
        -Vec~PlotSeries~ series
        -usize auto_color_index
        +push(PlotSeries)
        +next_auto_color() Color
        +validate() Result
    }

    class LayoutManager {
        -LegendConfig legend
        -GridStyle grid_style
        -TickConfig tick_config
        -Option~f32~ margin
        -AxisScale x_scale
        -AxisScale y_scale
    }

    class RenderPipeline {
        -ParallelRenderer parallel_renderer
        -Option~PooledRenderer~ pooled_renderer
        -Option~BackendType~ backend
        -bool enable_gpu
    }

    Plot *-- PlotConfiguration
    Plot *-- SeriesManager
    Plot *-- LayoutManager
    Plot *-- RenderPipeline
```

### Component Responsibilities

| Component | File | Purpose |
|-----------|------|---------|
| `PlotConfiguration` | `src/core/plot/configuration.rs` | Display settings (title, xlabel, ylabel, dimensions, dpi, theme) |
| `SeriesManager` | `src/core/plot/series_manager.rs` | Stores data series, handles auto-color assignment |
| `LayoutManager` | `src/core/plot/layout_manager.rs` | Legend config, grid style, tick marks, margins, axis limits/scales |
| `RenderPipeline` | `src/core/plot/render_pipeline.rs` | Backend selection, parallel/pooled rendering, GPU settings |

## Builder Pattern

### PlotBuilder Flow

```mermaid
flowchart LR
    A[Plot::new] --> B[.line/scatter/bar]
    B --> C[PlotBuilder&lt;C&gt;]
    C --> D{Config Methods}
    D --> |.color/.label/.line_width| C
    D --> |.title/.xlabel/.theme| C
    C --> E{Terminal Methods}
    E --> |.save| F[File Output]
    E --> |.render| G[Image]
    E --> |.line/.scatter| H[New PlotBuilder]
    H --> C
```

### PlotBuilder<C>

The generic `PlotBuilder<C>` provides a fluent API for configuring plots:

```rust
Plot::new()
    .line(&x, &y)           // Returns PlotBuilder<LineConfig>
    .color(Color::RED)      // Series-specific method
    .line_width(2.0)        // Series-specific method
    .title("My Plot")       // Forwards to Plot
    .save("plot.png")?;     // Terminal method
```

Key features:
- **Ownership-based transitions**: Series methods consume Plot and return PlotBuilder
- **Auto-finalization**: No explicit `.end()` needed - series finalize on save/render
- **Method forwarding**: Plot-level methods (title, xlabel, theme) forward to inner Plot

### Terminal Methods Macro

The `impl_terminal_methods!` macro generates common terminal methods for all config types:

```rust
impl_terminal_methods!(LineConfig);
impl_terminal_methods!(ScatterConfig);
impl_terminal_methods!(BarConfig);
// ... etc
```

Generated methods: `save()`, `render()`, `render_to_svg()`, `export_svg()`, `save_pdf()`, `save_with_size()`

## Rendering Pipeline

### Backend Selection

```mermaid
flowchart TD
    A[Data Size] --> B{< 10K?}
    B --> |Yes| C[Skia Default]
    B --> |No| D{< 100K?}
    D --> |Yes| E[Parallel]
    D --> |No| F{< 1M?}
    F --> |Yes| G[Parallel + SIMD]
    F --> |No| H[DataShader]

    I[Interactive Mode?] --> |Yes| J[GPU Backend]

    style C fill:#90EE90
    style E fill:#87CEEB
    style G fill:#DDA0DD
    style H fill:#FFB6C1
    style J fill:#FFD700
```

| Data Size | Backend | Features Required |
|-----------|---------|-------------------|
| < 10K points | Skia (default) | none |
| 10K - 100K | Parallel | `parallel` |
| 100K - 1M | Parallel + SIMD | `parallel`, `simd` |
| > 1M | DataShader | automatic |
| Interactive | GPU | `gpu`, `interactive` |

### Coordinate Transforms

The `CoordinateTransform` struct (in `src/core/transform.rs`) handles all coordinate conversions:

```mermaid
flowchart LR
    A[Data Space<br/>x, y: f64] --> |data_to_screen| B[Screen Space<br/>px, py: f32]
    B --> |screen_to_data| A

    subgraph CoordinateTransform
        C[data_bounds<br/>x_min, x_max, y_min, y_max]
        D[screen_bounds<br/>left, right, top, bottom]
    end
```

```rust
pub struct CoordinateTransform {
    data_bounds: (f64, f64, f64, f64),   // x_min, x_max, y_min, y_max
    screen_bounds: (f32, f32, f32, f32), // left, right, top, bottom
}

impl CoordinateTransform {
    pub fn data_to_screen(&self, x: f64, y: f64) -> (f32, f32);
    pub fn screen_to_data(&self, px: f32, py: f32) -> (f64, f64);
}
```

## Module Organization

```mermaid
graph TD
    subgraph src
        subgraph core
            A[plot/mod.rs<br/>Plot struct]
            B[plot/builder.rs<br/>PlotBuilder]
            C[plot/configuration.rs]
            D[plot/series_manager.rs]
            E[plot/layout_manager.rs]
            F[plot/render_pipeline.rs]
            G[transform.rs<br/>CoordinateTransform]
        end

        subgraph plots
            H[basic/<br/>Line, Scatter, Bar]
            I[statistical/<br/>KDE, Violin, Box]
            J[polar/<br/>Polar, Radar]
        end

        subgraph render
            K[mod.rs<br/>Backends]
            L[color.rs<br/>Color, ColorMap]
            M[theme.rs<br/>Themes]
            N[style.rs<br/>LineStyle, MarkerStyle]
        end

        O[style/mod.rs<br/>Unified re-exports]
    end

    A --> B
    A --> C
    A --> D
    A --> E
    A --> F
    B --> H
    B --> I
    B --> J
    O --> K
    O --> L
    O --> M
    O --> N
```

## Style System

The `ruviz::style` module provides unified access to all styling types:

```rust
use ruviz::style::{Color, Theme, LineStyle, MarkerStyle, GridStyle};
```

This re-exports from:
- `ruviz::render` - Color, Theme, LineStyle, MarkerStyle
- `ruviz::core` - GridStyle, PlotStyle, StyleResolver

## Data Flow

```mermaid
flowchart TB
    A[User Data<br/>Vec, ndarray, polars] --> B[PlotBuilder&lt;C&gt;]
    B --> |.finalize| C[Plot]
    B --> |Config methods| B
    C --> |.save / .render| D[RenderPipeline]

    D --> E{Backend Selection}
    E --> F[Skia Backend]
    E --> G[Parallel Backend]
    E --> H[GPU Backend]
    E --> I[DataShader Backend]

    F --> J[Image / File Output]
    G --> J
    H --> J
    I --> J

    style A fill:#E8F5E9
    style J fill:#E3F2FD
```

## Error Handling

All fallible operations return `Result<T, PlotError>`:

```rust
pub type Result<T> = std::result::Result<T, PlotError>;

pub enum PlotError {
    InvalidData(String),
    RenderError(String),
    IoError(std::io::Error),
    // ...
}
```

## Feature Flags

```mermaid
graph LR
    subgraph Features
        A[parallel] --> B[ParallelRenderer<br/>rayon]
        C[simd] --> D[SIMD transforms]
        E[gpu] --> F[GPU backend<br/>wgpu]
        G[interactive] --> H[winit window<br/>events]
        I[pdf] --> J[PDF export<br/>svg2pdf]
        K[ndarray_support] --> L[ndarray integration]
        M[polars_support] --> N[DataFrame integration]
    end
```

| Feature | Components Enabled |
|---------|-------------------|
| `parallel` | ParallelRenderer, rayon integration |
| `simd` | SIMD coordinate transforms |
| `gpu` | GPU backend, wgpu integration |
| `interactive` | winit window, event handling |
| `pdf` | PDF export via svg2pdf |
| `ndarray_support` | ndarray data integration |
| `polars_support` | polars DataFrame integration |
