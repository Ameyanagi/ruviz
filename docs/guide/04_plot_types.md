# Plot Types

Comprehensive guide to all plot types available in ruviz.

## Overview

ruviz supports common scientific and data visualization plot types:

| Plot Type | Use Case | Method |
|-----------|----------|--------|
| **Line** | Continuous data, time series | `.line(&x, &y)` |
| **Scatter** | Correlation, discrete points | `.scatter(&x, &y)` |
| **Bar** | Categorical comparison | `.bar(&categories, &values)` |
| **Histogram** | Distribution analysis | `.histogram(&data, config)` |
| **Box Plot** | Statistical summary | `.boxplot(&data, config)` |

## Line Plots

**Use for**: Time series, continuous functions, trends

### Basic Line Plot

```rust
use ruviz::prelude::*;

let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];

Plot::new()
    .line(&x, &y)
    .title("Basic Line Plot")
    .xlabel("X")
    .ylabel("Y")
    .save("line_basic.png")?;
```

### Multiple Lines

```rust
use ruviz::prelude::*;
use std::f64::consts::PI;

let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
let y_sin: Vec<f64> = x.iter().map(|v| v.sin()).collect();
let y_cos: Vec<f64> = x.iter().map(|v| v.cos()).collect();
let y_tan: Vec<f64> = x.iter().map(|v| (v/2.0).tan()).collect();

Plot::new()
    .line(&x, &y_sin)
        .label("sin(x)")
        .color(Color::from_rgb(255, 0, 0))
    .line(&x, &y_cos)
        .label("cos(x)")
        .color(Color::from_rgb(0, 0, 255))
        .line_style(LineStyle::Dashed)
    .line(&x, &y_tan)
        .label("tan(x/2)")
        .color(Color::from_rgb(0, 128, 0))
        .line_style(LineStyle::Dotted)
    .title("Trigonometric Functions")
    .xlabel("x (radians)")
    .ylabel("y")
    .xlim(0.0, 2.0 * PI)
    .ylim(-2.0, 2.0)
    .legend(Position::TopRight)
    .grid(true)
    .save("line_multiple.png")?;
```

### Styled Lines

```rust
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y)
    .line_width(3.0)                    // Thick line
    .line_style(LineStyle::Dashed)      // Dashed pattern
    .color(Color::from_rgb(255, 128, 0)) // Orange
    .label("Styled Line")
    .save("line_styled.png")?;
```

**Available Line Styles**:
- `LineStyle::Solid` (default)
- `LineStyle::Dashed`
- `LineStyle::Dotted`
- `LineStyle::DashDot`

### Line with Markers

```rust
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y)
    .marker(MarkerStyle::Circle)
    .marker_size(6.0)
    .line_width(2.0)
    .color(Color::from_rgb(0, 128, 255))
    .save("line_with_markers.png")?;
```

## Scatter Plots

**Use for**: Correlations, discrete measurements, point clouds

### Basic Scatter

```rust
use ruviz::prelude::*;

let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
let y = vec![2.3, 3.1, 2.8, 4.5, 5.2, 4.9];

Plot::new()
    .scatter(&x, &y)
    .marker(MarkerStyle::Circle)
    .marker_size(8.0)
    .title("Scatter Plot")
    .xlabel("X Variable")
    .ylabel("Y Variable")
    .save("scatter_basic.png")?;
```

### Multiple Scatter Series

```rust
use ruviz::prelude::*;

// Group A
let x1 = vec![1.0, 2.0, 3.0, 4.0, 5.0];
let y1 = vec![2.0, 4.0, 3.0, 5.0, 4.5];

// Group B
let x2 = vec![1.5, 2.5, 3.5, 4.5, 5.5];
let y2 = vec![3.0, 2.5, 4.0, 3.5, 5.0];

Plot::new()
    .scatter(&x1, &y1)
        .label("Group A")
        .marker(MarkerStyle::Circle)
        .marker_size(10.0)
        .color(Color::from_rgb(255, 0, 0))
    .scatter(&x2, &y2)
        .label("Group B")
        .marker(MarkerStyle::Square)
        .marker_size(10.0)
        .color(Color::from_rgb(0, 0, 255))
    .title("Multiple Groups")
    .legend(Position::TopLeft)
    .save("scatter_groups.png")?;
```

**Available Marker Styles**:
- `MarkerStyle::Circle`
- `MarkerStyle::Square`
- `MarkerStyle::Triangle`
- `MarkerStyle::Diamond`
- `MarkerStyle::Cross`
- `MarkerStyle::Plus`

### Scatter with Different Marker Sizes

```rust
use ruviz::prelude::*;

Plot::new()
    .scatter(&x, &y)
    .marker(MarkerStyle::Circle)
    .marker_size(4.0)   // Small points
    .color(Color::from_rgb(0, 128, 0))
    .save("scatter_small.png")?;

Plot::new()
    .scatter(&x, &y)
    .marker(MarkerStyle::Circle)
    .marker_size(12.0)  // Large points
    .color(Color::from_rgb(128, 0, 128))
    .save("scatter_large.png")?;
```

## Bar Charts

**Use for**: Categorical comparisons, grouped data

### Basic Bar Chart

```rust
use ruviz::prelude::*;

let categories = ["Mon", "Tue", "Wed", "Thu", "Fri"];
let values = vec![23.0, 45.0, 32.0, 51.0, 38.0];

Plot::new()
    .bar(&categories, &values)
    .title("Sales by Day")
    .xlabel("Day of Week")
    .ylabel("Sales ($)")
    .save("bar_basic.png")?;
```

### Styled Bar Chart

```rust
use ruviz::prelude::*;

let categories = ["Product A", "Product B", "Product C", "Product D"];
let values = vec![120.0, 250.0, 180.0, 310.0];

Plot::new()
    .bar(&categories, &values)
    .color(Color::from_rgb(70, 130, 180))  // Steel blue
    .title("Product Sales Comparison")
    .xlabel("Product")
    .ylabel("Units Sold")
    .grid(true)
    .save("bar_styled.png")?;
```

### Multiple Bar Series (Side-by-Side)

```rust
use ruviz::prelude::*;

let categories = ["Q1", "Q2", "Q3", "Q4"];
let revenue_2023 = vec![100.0, 120.0, 140.0, 160.0];
let revenue_2024 = vec![110.0, 135.0, 155.0, 180.0];

// Note: Multi-series bars require manual positioning
// Create separate plots for each series with offset categories
Plot::new()
    .bar(&categories, &revenue_2023)
        .label("2023")
        .color(Color::from_rgb(100, 149, 237))
    .bar(&categories, &revenue_2024)
        .label("2024")
        .color(Color::from_rgb(255, 140, 0))
    .title("Quarterly Revenue Comparison")
    .xlabel("Quarter")
    .ylabel("Revenue ($K)")
    .legend(Position::TopLeft)
    .save("bar_comparison.png")?;
```

## Histograms

**Use for**: Distribution analysis, frequency analysis

### Basic Histogram

```rust
use ruviz::prelude::*;
use ruviz::plots::histogram::HistogramConfig;

let data = vec![
    1.2, 1.5, 1.8, 2.1, 2.3, 2.7, 2.9, 3.1, 3.4, 3.6,
    3.8, 4.0, 4.2, 4.5, 4.7, 4.9, 5.1, 5.3, 5.6, 5.8,
    6.0, 6.2, 6.5, 6.7, 6.9, 7.1, 7.4, 7.6, 7.8, 8.0,
];

Plot::new()
    .histogram(&data, None)  // Auto bin count
    .title("Distribution")
    .xlabel("Value")
    .ylabel("Frequency")
    .save("histogram_basic.png")?;
```

### Custom Bin Count

```rust
use ruviz::prelude::*;
use ruviz::plots::histogram::HistogramConfig;

let config = HistogramConfig {
    bins: 15,  // Specific number of bins
    ..Default::default()
};

Plot::new()
    .histogram(&data, Some(config))
    .title("Distribution (15 bins)")
    .xlabel("Value")
    .ylabel("Frequency")
    .save("histogram_custom_bins.png")?;
```

### Normal Distribution Example

```rust
use ruviz::prelude::*;
use ruviz::plots::histogram::HistogramConfig;
use rand::distributions::{Distribution, Normal};

// Generate normally distributed data
let normal = Normal::new(100.0, 15.0).unwrap();
let mut rng = rand::thread_rng();
let data: Vec<f64> = (0..1000)
    .map(|_| normal.sample(&mut rng))
    .collect();

Plot::new()
    .histogram(&data, Some(HistogramConfig::new()))
    .title("Normal Distribution (μ=100, σ=15)")
    .xlabel("Value")
    .ylabel("Frequency")
    .theme(Theme::publication())
    .save("histogram_normal.png")?;
```

Add to `Cargo.toml`:
```toml
[dependencies]
ruviz = "0.1"
rand = "0.8"
```

### Styled Histogram

```rust
use ruviz::prelude::*;
use ruviz::plots::histogram::HistogramConfig;

Plot::new()
    .theme(Theme::seaborn())  // Clean, professional style
    .histogram(&data, Some(HistogramConfig::new()))
    .color(Color::from_rgb(76, 114, 176))  // Muted blue
    .title("Distribution Analysis")
    .xlabel("Measurement")
    .ylabel("Count")
    .grid(true)
    .save("histogram_styled.png")?;
```

## Box Plots

**Use for**: Statistical summary, outlier detection, group comparison

### Basic Box Plot

```rust
use ruviz::prelude::*;
use ruviz::plots::boxplot::BoxPlotConfig;

let data = vec![
    1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0,
    11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0,
    // Add some outliers
    35.0, 40.0, -5.0
];

Plot::new()
    .boxplot(&data, Some(BoxPlotConfig::new()))
    .title("Box Plot Example")
    .xlabel("Distribution")
    .ylabel("Values")
    .save("boxplot_basic.png")?;
```

**Box Plot Components**:
- **Box**: Interquartile range (IQR) from Q1 to Q3
- **Line in box**: Median (Q2)
- **Whiskers**: Extend to 1.5 × IQR or data extremes
- **Points**: Outliers beyond whiskers

### Statistical Information

```rust
use ruviz::prelude::*;
use ruviz::plots::boxplot::BoxPlotConfig;

// Example data with clear statistics
let data = vec![
    10.0, 12.0, 14.0, 15.0, 16.0,  // Lower quartile
    17.0, 18.0, 19.0, 20.0,         // Around median
    21.0, 22.0, 23.0, 24.0,         // Upper quartile
    25.0, 26.0, 35.0,               // High values + outlier
];

Plot::new()
    .boxplot(&data, Some(BoxPlotConfig::new()))
    .title("Statistical Distribution")
    .ylabel("Values")
    .grid(true)
    .theme(Theme::publication())
    .save("boxplot_statistical.png")?;
```

### Multiple Box Plots (Comparison)

```rust
use ruviz::prelude::*;
use ruviz::plots::boxplot::BoxPlotConfig;

// Note: Currently requires manual positioning or subplots
// See Subplots guide for multi-group box plot examples

let group_a = vec![5.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 15.0];
let group_b = vec![10.0, 12.0, 14.0, 15.0, 16.0, 18.0, 20.0, 25.0];

// Create subplot with multiple box plots
// (See chapter 06_subplots.md for details)
```

## Combining Plot Types

### Line + Scatter

```rust
use ruviz::prelude::*;

let x = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
let y_theory = vec![0.0, 1.0, 4.0, 9.0, 16.0, 25.0];
let y_measured = vec![0.2, 1.3, 3.8, 9.5, 15.7, 24.9];

Plot::new()
    .line(&x, &y_theory)
        .label("Theory")
        .color(Color::from_rgb(0, 0, 255))
        .line_width(2.0)
    .scatter(&x, &y_measured)
        .label("Measured")
        .marker(MarkerStyle::Circle)
        .marker_size(8.0)
        .color(Color::from_rgb(255, 0, 0))
    .title("Theory vs Measurement")
    .xlabel("Input")
    .ylabel("Output")
    .legend(Position::TopLeft)
    .grid(true)
    .save("combined_line_scatter.png")?;
```

### Multiple Data Types

```rust
use ruviz::prelude::*;

// Time series data
let time = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
let signal = vec![1.0, 1.5, 1.2, 1.8, 1.4, 1.6];

// Event markers
let event_times = vec![1.5, 3.2, 4.8];
let event_values = vec![1.5, 1.8, 1.4];

Plot::new()
    .line(&time, &signal)
        .label("Signal")
        .color(Color::from_rgb(0, 0, 255))
    .scatter(&event_times, &event_values)
        .label("Events")
        .marker(MarkerStyle::Triangle)
        .marker_size(12.0)
        .color(Color::from_rgb(255, 0, 0))
    .title("Signal with Events")
    .xlabel("Time (s)")
    .ylabel("Amplitude")
    .legend(Position::TopRight)
    .save("signal_with_events.png")?;
```

## Common Patterns

### Time Series

```rust
use ruviz::prelude::*;

// Simulated daily measurements
let days: Vec<f64> = (0..30).map(|i| i as f64).collect();
let temperature: Vec<f64> = days.iter()
    .map(|&d| 20.0 + 5.0 * (d * 0.2).sin() + (rand::random::<f64>() - 0.5))
    .collect();

Plot::new()
    .line(&days, &temperature)
    .marker(MarkerStyle::Circle)
    .marker_size(4.0)
    .color(Color::from_rgb(255, 100, 0))
    .title("Daily Temperature")
    .xlabel("Day")
    .ylabel("Temperature (°C)")
    .grid(true)
    .save("timeseries.png")?;
```

### Categorical Summary

```rust
use ruviz::prelude::*;

let categories = ["North", "South", "East", "West"];
let sales = vec![145.0, 210.0, 178.0, 195.0];

Plot::new()
    .bar(&categories, &sales)
    .color(Color::from_rgb(70, 130, 180))
    .title("Regional Sales Summary")
    .xlabel("Region")
    .ylabel("Sales ($K)")
    .grid(true)
    .save("categorical_summary.png")?;
```

### Distribution Comparison

```rust
use ruviz::prelude::*;
use ruviz::plots::histogram::HistogramConfig;

// Before optimization
let data_before = vec![/* measurement data */];

// After optimization
let data_after = vec![/* measurement data */];

// Create two separate histograms or use subplots
// (See chapter 06_subplots.md for side-by-side comparison)
```

## Performance Considerations

### Small Datasets (< 1K points)

```rust
// Default rendering is optimal
Plot::new()
    .scatter(&x, &y)
    .save("small_dataset.png")?;
```

### Medium Datasets (1K - 10K points)

```rust
// Parallel feature automatically optimizes
// Ensure Cargo.toml has: features = ["parallel"]
Plot::new()
    .line(&x, &y)  // Auto-parallelized
    .save("medium_dataset.png")?;
```

### Large Datasets (10K - 100K points)

```rust
// Use release mode for best performance
// cargo run --release

Plot::new()
    .line(&x, &y)  // Parallel + SIMD if enabled
    .save("large_dataset.png")?;
```

### Very Large Datasets (> 1M points)

```rust
// DataShader automatically activates
// Intelligently aggregates data for rendering

let x: Vec<f64> = (0..10_000_000).map(|i| i as f64).collect();
let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();

Plot::new()
    .line(&x, &y)  // DataShader aggregation
    .save("huge_dataset.png")?;
```

See [Performance Guide](08_performance.md) for detailed optimization strategies.

## Next Steps

- **[Styling & Themes](05_styling.md)** - Customize colors, markers, themes
- **[Subplots](06_subplots.md)** - Multi-panel figures
- **[Performance](08_performance.md)** - Optimize for large datasets
- **[Examples](../../examples/)** - Working code samples

## Quick Reference

| Plot Type | Basic Usage | Configuration |
|-----------|-------------|---------------|
| **Line** | `.line(&x, &y)` | `.line_width()`, `.line_style()` |
| **Scatter** | `.scatter(&x, &y)` | `.marker()`, `.marker_size()` |
| **Bar** | `.bar(&cats, &vals)` | `.color()` |
| **Histogram** | `.histogram(&data, cfg)` | `HistogramConfig { bins, .. }` |
| **Box Plot** | `.boxplot(&data, cfg)` | `BoxPlotConfig::new()` |

---

**Ready to customize?** → [Styling & Themes](05_styling.md)
