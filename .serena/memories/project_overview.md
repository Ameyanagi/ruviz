# Ruviz Project Overview

## Purpose
High-performance 2D plotting library for Rust that combines matplotlib's comprehensiveness with performance optimization. Targets <100ms for 100K points, <1s for 1M points.

## Tech Stack
- **Language**: Rust 2024 edition
- **Rendering**: tiny-skia (4-7x faster than alternatives)
- **Math**: glam, nalgebra (optional)
- **Data**: ndarray (optional), polars (optional)
- **Image**: image crate for PNG export
- **Testing**: criterion for benchmarks, approx for float comparisons

## Architecture
- `src/core/`: Core Plot struct and error handling
- `src/render/`: Rendering backends and visual styling
- `src/data/`: Data trait abstractions and transformations
- `src/plots/`: Plot type implementations
- `src/axes/`: Axis rendering and tick generation
- `src/text/`: Text rendering and layout
- `src/layout/`: Grid, margin, and subplot management
- `src/export/`: PNG/SVG export functionality