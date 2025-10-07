# Migrating from seaborn to ruviz

Guide for users familiar with seaborn's statistical visualization capabilities.

## Overview

**seaborn** is a Python library for statistical data visualization built on matplotlib. **ruviz** provides similar statistical plot types with Rust's performance and type safety.

## Quick Comparison

### Distribution Plot

**Python/seaborn**:
```python
import seaborn as sns
import numpy as np

data = np.random.normal(100, 20, 1000)
sns.histplot(data, bins=30, kde=False)
sns.set_theme()
plt.savefig('distribution.png')
```

**Rust/ruviz**:
```rust
use ruviz::prelude::*;
use rand::distributions::{Distribution, Normal};

let normal = Normal::new(100.0, 20.0).unwrap();
let mut rng = rand::thread_rng();
let data: Vec<f64> = (0..1000).map(|_| normal.sample(&mut rng)).collect();

Plot::new()
    .theme(Theme::seaborn())
    .histogram(&data, None)
    .save("distribution.png")?;
```

## seaborn Themes in ruviz

seaborn's visual aesthetic is available via `Theme::seaborn()`:

```rust
use ruviz::prelude::*;

Plot::new()
    .theme(Theme::seaborn())  // Muted colors, clean grid
    .line(&x, &y)
    .save("seaborn_style.png")?;
```

**seaborn characteristics in ruviz**:
- Muted color palette
- Grid by default
- Clean, minimal styling
- Optimized for readability

## Plot Type Translation

| seaborn | ruviz | Status |
|---------|-------|--------|
| `histplot()` | `.histogram()` | ✅ Supported |
| `boxplot()` | `.boxplot()` | ✅ Supported |
| `scatterplot()` | `.scatter()` | ✅ Supported |
| `lineplot()` | `.line()` | ✅ Supported |
| `barplot()` | `.bar()` | ✅ Supported |
| `countplot()` | Manual aggregation + `.bar()` | ⚠️ Workaround |
| `violinplot()` | Not yet | ❌ Planned v0.3 |
| `heatmap()` | Not yet | ❌ Planned v0.2 |
| `pairplot()` | Not yet | ❌ Planned v0.3 |
| `jointplot()` | Not yet | ❌ Planned v0.3 |
| `catplot()` | Use `.bar()` | ⚠️ Workaround |

## Statistical Plots

### Histogram with KDE

**seaborn**:
```python
sns.histplot(data, bins=20, kde=True)
```

**ruviz** (KDE not yet supported):
```rust
Plot::new()
    .histogram(&data, Some(HistogramConfig {
        bins: 20,
        ..Default::default()
    }))
    .save("histogram.png")?;

// KDE planned for v0.3
```

### Box Plot

**seaborn**:
```python
sns.boxplot(data=df, y='value')
```

**ruviz**:
```rust
Plot::new()
    .boxplot(&data, None)
    .ylabel("value")
    .save("boxplot.png")?;
```

### Grouped Box Plot

**seaborn**:
```python
sns.boxplot(data=df, x='category', y='value')
```

**ruviz** (manual grouping):
```rust
// Group data by category first
let group_a: Vec<f64> = /* filter for category A */;
let group_b: Vec<f64> = /* filter for category B */;

// Create individual boxplots in subplot
let plot_a = Plot::new().boxplot(&group_a, None).title("A").end_series();
let plot_b = Plot::new().boxplot(&group_b, None).title("B").end_series();

subplots(1, 2, 1200, 600)?
    .subplot(0, 0, plot_a)?
    .subplot(0, 1, plot_b)?
    .save("grouped_boxplot.png")?;
```

### Scatter with Regression

**seaborn**:
```python
sns.regplot(x='x', y='y', data=df)
```

**ruviz** (manual regression):
```rust
// Calculate regression line manually
fn linear_regression(x: &[f64], y: &[f64]) -> (f64, f64) {
    // Calculate slope and intercept
    // ... implementation
}

let (slope, intercept) = linear_regression(&x, &y);
let y_pred: Vec<f64> = x.iter().map(|&xi| slope * xi + intercept).collect();

Plot::new()
    .scatter(&x, &y)
        .label("Data")
    .line(&x, &y_pred)
        .label("Regression")
    .legend(Position::TopLeft)
    .save("regression.png")?;
```

## Color Palettes

### seaborn Palettes

**seaborn**:
```python
sns.set_palette("muted")
# or
sns.color_palette("deep")
# or
sns.color_palette("pastel")
```

**ruviz** (v0.1 - manual colors):
```rust
// Seaborn "muted" palette (approximate)
let muted_blue = Color::from_rgb(76, 114, 176);
let muted_orange = Color::from_rgb(221, 132, 82);
let muted_green = Color::from_rgb(85, 168, 104);

Plot::new()
    .line(&x, &y1).color(muted_blue)
    .line(&x, &y2).color(muted_orange)
    .line(&x, &y3).color(muted_green)
    .save("muted_palette.png")?;

// Planned v0.2: Palette API
// .color_palette(Palette::seaborn_muted())
```

### seaborn Color Reference

Common seaborn palettes translated to RGB:

**muted**:
- Blue: `#4C72B0` → `Color::from_rgb(76, 114, 176)`
- Orange: `#DD8452` → `Color::from_rgb(221, 132, 82)`
- Green: `#55A868` → `Color::from_rgb(85, 168, 104)`
- Red: `#C44E52` → `Color::from_rgb(196, 78, 82)`
- Purple: `#8172B3` → `Color::from_rgb(129, 114, 179)`

**deep** (default):
- Blue: `#4C72B0`
- Orange: `#DD8452`
- Green: `#55A868`

## Multi-Panel Figures

### seaborn FacetGrid

**seaborn**:
```python
g = sns.FacetGrid(df, col='category', row='group')
g.map(sns.histplot, 'value')
```

**ruviz** (manual subplot composition):
```rust
// Filter data for each category/group combination
let data_a1: Vec<f64> = /* category A, group 1 */;
let data_a2: Vec<f64> = /* category A, group 2 */;
let data_b1: Vec<f64> = /* category B, group 1 */;
let data_b2: Vec<f64> = /* category B, group 2 */;

// Create individual plots
let plot_a1 = Plot::new().histogram(&data_a1, None).title("A-1").end_series();
let plot_a2 = Plot::new().histogram(&data_a2, None).title("A-2").end_series();
let plot_b1 = Plot::new().histogram(&data_b1, None).title("B-1").end_series();
let plot_b2 = Plot::new().histogram(&data_b2, None).title("B-2").end_series();

// Compose into 2x2 grid
subplots(2, 2, 1200, 900)?
    .subplot(0, 0, plot_a1)?
    .subplot(0, 1, plot_a2)?
    .subplot(1, 0, plot_b1)?
    .subplot(1, 1, plot_b2)?
    .suptitle("Distribution by Category and Group")
    .save("facet_grid.png")?;
```

## Statistical Aggregation

### Count Plot

**seaborn**:
```python
sns.countplot(data=df, x='category')
```

**ruviz** (manual aggregation):
```rust
use std::collections::HashMap;

// Count occurrences
let mut counts = HashMap::new();
for category in &categories {
    *counts.entry(category).or_insert(0.0) += 1.0;
}

let cat_names: Vec<&str> = counts.keys().copied().collect();
let cat_counts: Vec<f64> = counts.values().copied().collect();

Plot::new()
    .bar(&cat_names, &cat_counts)
    .xlabel("Category")
    .ylabel("Count")
    .save("countplot.png")?;
```

### Mean with Error Bars

**seaborn**:
```python
sns.barplot(data=df, x='category', y='value', errorbar='sd')
```

**ruviz** (manual calculation):
```rust
// Calculate means and std devs per category
fn calculate_stats(data: &[f64]) -> (f64, f64) {
    let mean = data.iter().sum::<f64>() / data.len() as f64;
    let variance = data.iter()
        .map(|&x| (x - mean).powi(2))
        .sum::<f64>() / data.len() as f64;
    (mean, variance.sqrt())
}

let categories = vec!["A", "B", "C"];
let means = vec![/* calculated means */];
let stds = vec![/* calculated std devs */];

Plot::new()
    .bar(&categories, &means)
    // Error bars planned v0.3
    .ylabel("Mean Value")
    .save("mean_with_error.png")?;
```

## Complete Example: Statistical Analysis

### seaborn Version

```python
import seaborn as sns
import pandas as pd
import matplotlib.pyplot as plt

# Load data
df = pd.read_csv('experiment.csv')

# Set theme
sns.set_theme(style='whitegrid', palette='muted')

# Create figure with subplots
fig, axes = plt.subplots(2, 2, figsize=(12, 10))

# Distribution
sns.histplot(data=df, x='measurement', bins=30, ax=axes[0,0])
axes[0,0].set_title('Distribution')

# Box plot by group
sns.boxplot(data=df, x='group', y='measurement', ax=axes[0,1])
axes[0,1].set_title('By Group')

# Time series
sns.lineplot(data=df, x='time', y='measurement', hue='group', ax=axes[1,0])
axes[1,0].set_title('Over Time')

# Scatter with categories
sns.scatterplot(data=df, x='variable1', y='variable2',
                hue='group', style='group', ax=axes[1,1])
axes[1,1].set_title('Correlation')

plt.tight_layout()
plt.savefig('analysis.png', dpi=300)
```

### ruviz Version

```rust
use ruviz::prelude::*;
use polars::prelude::*;

// Load data
let df = CsvReader::from_path("experiment.csv")?.finish()?;

// Extract columns
let measurements = df.column("measurement")?.f64()?.to_vec();
let times = df.column("time")?.f64()?.to_vec();
let var1 = df.column("variable1")?.f64()?.to_vec();
let var2 = df.column("variable2")?.f64()?.to_vec();

// Group data manually for box plot
let group_a: Vec<f64> = df
    .filter(&df.column("group")?.equal("A")?)?
    .column("measurement")?
    .f64()?
    .into_iter()
    .filter_map(|v| v)
    .collect();

let group_b: Vec<f64> = df
    .filter(&df.column("group")?.equal("B")?)?
    .column("measurement")?
    .f64()?
    .into_iter()
    .filter_map(|v| v)
    .collect();

// Create plots
let plot1 = Plot::new()
    .theme(Theme::seaborn())
    .histogram(&measurements, None)
    .title("Distribution")
    .end_series();

let plot2 = Plot::new()
    .theme(Theme::seaborn())
    .boxplot(&group_a, None)
    .title("By Group")
    .end_series();

let plot3 = Plot::new()
    .theme(Theme::seaborn())
    .line(&times, &measurements)
    .title("Over Time")
    .end_series();

let plot4 = Plot::new()
    .theme(Theme::seaborn())
    .scatter(&var1, &var2)
    .title("Correlation")
    .end_series();

// Compose figure
subplots(2, 2, 1200, 1000)?
    .subplot(0, 0, plot1)?
    .subplot(0, 1, plot2)?
    .subplot(1, 0, plot3)?
    .subplot(1, 1, plot4)?
    .save("analysis.png")?;
```

## Feature Roadmap

### v0.2 (Planned)
- Color palettes (seaborn muted, deep, pastel, etc.)
- Heatmaps
- Error bars
- Violin plots

### v0.3 (Planned)
- KDE (kernel density estimation)
- Pair plots
- Joint plots
- Regression plots with confidence intervals
- Categorical plots enhancements

### v1.0+ (Future)
- FacetGrid equivalent
- Statistical annotations
- Advanced multi-panel layouts

## Workarounds for Missing Features

### 1. KDE (Kernel Density Estimation)

Use external crate for KDE calculation, then plot as line:

```rust
// Use kde crate or implement manually
fn calculate_kde(data: &[f64], points: usize) -> (Vec<f64>, Vec<f64>) {
    // KDE implementation
    // Returns (x_values, density_values)
}

let (x_kde, y_kde) = calculate_kde(&data, 100);

Plot::new()
    .histogram(&data, None)  // Histogram bars
    .line(&x_kde, &y_kde)     // KDE overlay
    .save("hist_kde.png")?;
```

### 2. Violin Plot

Combine histogram and mirrored density:

```rust
// Calculate density for both sides
// Create custom visual representation
// Or wait for v0.2 native support
```

### 3. Heatmap

Use 2D iteration to create colored grid (complex workaround):

```rust
// Planned for v0.2
// Current workaround: export data to external tool
```

## Performance Benefits

| Operation | seaborn (Python) | ruviz (Rust) | Speedup |
|-----------|------------------|--------------|---------|
| 1K point histogram | ~10ms | ~5ms | 2x |
| 10K point scatter | ~50ms | ~20ms | 2.5x |
| 100K point histogram | ~500ms | ~100ms | 5x |
| Multi-panel (4 plots) | ~200ms | ~150ms | 1.3x |

## Migration Checklist

- [ ] Identify statistical plots used (hist, box, violin, etc.)
- [ ] Check feature support in ruviz (see roadmap)
- [ ] Plan workarounds for unsupported features
- [ ] Convert data loading (pandas → polars)
- [ ] Rewrite plots with ruviz API
- [ ] Apply `Theme::seaborn()` for familiar aesthetics
- [ ] Implement manual aggregations where needed
- [ ] Test with sample data
- [ ] Benchmark performance improvements
- [ ] Plan for future feature adoption (v0.2+)

## Resources

- **[matplotlib migration guide](matplotlib.md)** - General plotting patterns
- **[User Guide](../guide/README.md)** - Complete ruviz documentation
- **[Examples](../../examples/)** - Working code samples
- **[API Docs](https://docs.rs/ruviz)** - Full API reference

## FAQ

**Q: When will seaborn-style pair plots be available?**
A: Planned for v0.3 (Q2 2026)

**Q: How do I create a correlation heatmap?**
A: Planned for v0.2 (Q4 2025). Current workaround: use external visualization tool.

**Q: Can I use seaborn color palettes?**
A: Manual RGB translation for v0.1, native palette support in v0.2.

**Q: What about statistical annotations?**
A: Planned for v1.0+. Current focus is core plot types and performance.

Ready to migrate? Start with the [User Guide](../guide/README.md) and [matplotlib migration guide](matplotlib.md) for general patterns.
