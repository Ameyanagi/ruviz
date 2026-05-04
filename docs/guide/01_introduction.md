# Introduction to ruviz

**ruviz** is a 2D plotting library for Rust with a fluent, matplotlib-inspired API.

## What is ruviz?

ruviz provides familiar builders for creating publication-quality plots in Rust,
with PNG, SVG, and optional PDF export.

### Key Features

- **🛡️ Memory Safe**: Zero unsafe code in public API
- **📊 Rich Plot Types**: Line, scatter, bar, histogram, boxplot, and more
- **🎨 Publication Quality**: Professional themes, high-DPI export, Unicode support
- **⚡ Configurable Rendering**: reference raster output plus optional feature-gated renderer code
- **🔧 Type Safe**: Strong typing prevents runtime errors
- **📦 Easy Integration**: Works with ndarray, polars, standard Vec/slices

## Why ruviz?

### Coming from Python/matplotlib?

```python
# Python/matplotlib
import matplotlib.pyplot as plt
x = [0, 1, 2, 3, 4]
y = [0, 1, 4, 9, 16]
plt.plot(x, y)
plt.title("My Plot")
plt.savefig("plot.png")
```

```rust
// Rust/ruviz
use ruviz::prelude::*;
let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];
Plot::new()
    .line(&x, &y)
    .title("My Plot")
    .save("plot.png")?;
```

**Benefits of using ruviz**:
- Compile-time error checking
- No GC pauses or runtime overhead
- Type-safe API prevents common mistakes

### Why not existing Rust libraries?

ruviz is designed specifically for:
- **Scientific computing**: Plot arrays and numeric data directly
- **Data analysis**: Integration with ndarray, polars
- **Publication**: IEEE/Nature-quality output
- **Rust applications**: Native APIs without a Python or JavaScript runtime

## Design Philosophy

### 1. Ease of Use
- **Familiar API**: If you know matplotlib, you know ruviz
- **Builder pattern**: Fluent, chainable method calls
- **Sensible defaults**: Get good results with minimal configuration

### 2. Practical Output
- **Reference raster path**: Conservative PNG/image output for visual parity
- **Vector output**: SVG export and optional PDF export
- **Feature flags**: Optional renderer/data integrations when your workload needs them

### 3. Safety & Quality
- **No unsafe code**: Memory safe by design
- **Strong typing**: Catch errors at compile time
- **Comprehensive testing**: 299+ tests with visual regression
- **Production ready**: Battle-tested rendering pipeline

## Use Cases

### Scientific Computing
```rust
use ruviz::prelude::*;
use ndarray::Array1;

let x = Array1::linspace(0.0, 10.0, 1000);
let y = x.mapv(|v| v.sin());

Plot::new()
    .line(&x, &y)
    .title("Sine Wave")
    .xlabel("x (radians)")
    .ylabel("sin(x)")
    .theme(Theme::publication())
    .dpi(300)  // Publication quality
    .save("scientific_plot.png")?;
```

### Data Analysis
```rust
use ruviz::prelude::*;
use polars::prelude::*;

let df = CsvReader::from_path("data.csv")?
    .finish()?;

let x = df.column("time")?.f64()?;
let y = df.column("value")?.f64()?;

Plot::new()
    .scatter(x, y)
    .xlabel("Time")
    .ylabel("Measurement")
    .save("analysis.png")?;
```

### Larger Datasets
```rust
use ruviz::prelude::*;

let x: Vec<f64> = (0..250_000).map(|i| i as f64 * 0.001).collect();
let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

Plot::new()
    .line(&x, &y)
    .save("large_dataset.png")?;
```

## Architecture Overview

```
┌─────────────────────────────────────────┐
│          High-Level API                 │
│   Plot, Theme, Style, Layout           │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────┴───────────────────────┐
│       Plot Configuration               │
│  Size, DPI, theme, stored backend label │
└─────────────────┬───────────────────────┘
                  │
     ┌────────────┼────────────┬──────────┐
     │            │            │          │
┌────▼─────┐ ┌───▼────┐ ┌────▼─────┐ ┌──▼────┐
│ Raster   │ │  SVG   │ │   PDF    │ │Window │
│  PNG     │ │Vector  │ │ feature  │ │feature│
└──────────┘ └────────┘ └──────────┘ └───────┘
     │            │            │          │
     └────────────┴────────────┴──────────┘
                  │
         ┌────────▼────────┐
         │ Output helpers  │
         │ save/render/... │
         └────────┬────────┘
                  │
            ┌─────▼──────┐
            │ PNG Export │
            └────────────┘
```

## Next Steps

- **[Installation Guide](02_installation.md)** - Set up ruviz in your project
- **[First Plot](03_first_plot.md)** - Create your first visualization
- **[Quick Start](../QUICKSTART.md)** - 5-minute tutorial

## Philosophy Summary

> **ruviz aims to make data visualization in Rust approachable while preserving Rust's type safety.**

We believe that:
1. Performance shouldn't require complexity
2. Type safety prevents bugs before they happen
3. Good defaults enable quick prototyping
4. Fine-grained control enables optimization

Ready to get started? Continue to [Installation →](02_installation.md)
