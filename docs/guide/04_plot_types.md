# Plot Types

Comprehensive guide to all plot types available in ruviz.

## Overview

ruviz provides **30+ plot types** across multiple categories, achieving parity with matplotlib, seaborn, and Makie.jl for scientific and data visualization.

### Quick Reference by Category

| Category | Plot Types |
|----------|------------|
| **Basic** | Line, Scatter, Bar, Histogram, Box Plot, Heatmap |
| **Distribution** | Violin, KDE (1D/2D), Boxen, ECDF, Strip, Swarm |
| **Categorical** | Grouped Bar, Stacked Bar, Horizontal Bar |
| **Composition** | Pie, Donut, Area, Stacked Area |
| **Continuous** | Contour, Hexbin, Fill Between |
| **Error** | Error Bars (symmetric/asymmetric) |
| **Discrete** | Step, Stem |
| **Regression** | Regression Plot, Residual Plot |
| **Polar** | Polar Plot, Radar/Spider Chart |
| **Composite** | Joint Plot, Pair Plot |
| **Vector** | Quiver Plot |
| **Hierarchical** | Dendrogram |

---

## Basic Plots

### Line Plots

**Use for**: Time series, continuous functions, trends

```rust
use ruviz::prelude::*;

let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();

Plot::new()
    .line(&x, &y)
    .title("Sine Wave")
    .xlabel("x")
    .ylabel("sin(x)")
    .save("line_plot.png")?;
```

**Styling Options**:
- `line_width(f32)` - Line thickness
- `line_style(LineStyle)` - Solid, Dashed, Dotted, DashDot
- `color(Color)` - Line color
- `marker(MarkerStyle)` - Add markers at data points

### Scatter Plots

**Use for**: Correlations, discrete measurements, point clouds

```rust
use ruviz::prelude::*;

let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
let y = vec![2.3, 3.1, 2.8, 4.5, 5.2];

Plot::new()
    .scatter(&x, &y)
    .marker(MarkerStyle::Circle)
    .marker_size(8.0)
    .title("Scatter Plot")
    .save("scatter.png")?;
```

**Marker Styles**: Circle, Square, Triangle, Diamond, Cross, Plus

### Bar Charts

**Use for**: Categorical comparisons

```rust
use ruviz::prelude::*;

let categories = ["Mon", "Tue", "Wed", "Thu", "Fri"];
let values = vec![23.0, 45.0, 32.0, 51.0, 38.0];

Plot::new()
    .bar(&categories, &values)
    .title("Daily Sales")
    .save("bar.png")?;
```

### Histograms

**Use for**: Distribution analysis, frequency analysis

```rust
use ruviz::prelude::*;
use ruviz::plots::histogram::HistogramConfig;

let data: Vec<f64> = (0..1000).map(|i| /* sample data */).collect();

Plot::new()
    .histogram(&data, Some(HistogramConfig { bins: 30, ..Default::default() }))
    .title("Distribution")
    .save("histogram.png")?;
```

### Box Plots

**Use for**: Statistical summary, outlier detection

```rust
use ruviz::prelude::*;
use ruviz::plots::boxplot::BoxPlotConfig;

let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 35.0];

Plot::new()
    .boxplot(&data, Some(BoxPlotConfig::new()))
    .title("Box Plot")
    .save("boxplot.png")?;
```

---

## Distribution Plots

### Violin Plots

**Use for**: Distribution comparison across categories, combining KDE with box plot statistics

```rust
use ruviz::plots::distribution::{ViolinConfig, ViolinData, violin_polygon};

let config = ViolinConfig::new()
    .bandwidth_method(BandwidthMethod::Scott)
    .show_box(true)
    .show_median(true);

let violin = ViolinData::from_data(&data, &config);
let polygon = violin_polygon(&violin, x_center, width, Orientation::Vertical);
```

**Features**:
- Kernel density estimation on both sides
- Optional inner box plot
- Configurable bandwidth (Scott, Silverman, custom)
- Vertical or horizontal orientation

### KDE Plots (Kernel Density Estimation)

**Use for**: Smooth distribution curves, density estimation

```rust
use ruviz::plots::distribution::{KdePlotConfig, compute_kde_plot};

// 1D KDE
let config = KdePlotConfig::new()
    .bandwidth(0.5)
    .n_points(200)
    .cumulative(false);

let kde_data = compute_kde_plot(&data, &config);
// kde_data.x and kde_data.y contain the smooth density curve

// 2D KDE for bivariate density
use ruviz::plots::distribution::{Kde2dPlotConfig, compute_kde_2d_plot};

let config_2d = Kde2dPlotConfig::new().grid_size(50);
let kde_2d = compute_kde_2d_plot(&x, &y, &config_2d);
// kde_2d.density contains the 2D density grid
```

### Boxen Plots (Letter-Value Plots)

**Use for**: Large datasets where box plots don't show enough detail

```rust
use ruviz::plots::distribution::{BoxenConfig, compute_boxen};

let config = BoxenConfig::new()
    .k(5)  // Number of letter values
    .outlier_prop(0.007);

let boxen = compute_boxen(&data, &config);
// boxen.boxes contains nested boxes from median outward
```

### ECDF Plots (Empirical Cumulative Distribution)

**Use for**: Comparing distributions, survival analysis

```rust
use ruviz::plots::distribution::{EcdfConfig, EcdfStat, compute_ecdf};

let config = EcdfConfig::new()
    .stat(EcdfStat::Proportion)  // or Count, Percent
    .complementary(false)  // true for survival function
    .show_ci(true);  // Show confidence interval

let ecdf = compute_ecdf(&data, &config);
// ecdf.step_vertices contains step function coordinates
```

### Strip Plots

**Use for**: Jittered categorical scatter plots

```rust
use ruviz::plots::categorical::{StripConfig, compute_strip};

let config = StripConfig::new()
    .jitter(0.1)
    .orientation(StripOrientation::Vertical);

let strip = compute_strip(&values, &config);
// strip.points contains jittered positions
```

### Swarm Plots (Beeswarm)

**Use for**: Non-overlapping categorical scatter

```rust
use ruviz::plots::categorical::{SwarmConfig, compute_swarm};

let config = SwarmConfig::new()
    .point_size(5.0)
    .orientation(SwarmOrientation::Vertical);

let swarm = compute_swarm(&values, &config);
// swarm.points positioned to avoid overlap
```

---

## Categorical Plots

### Grouped Bar Charts

**Use for**: Side-by-side comparison of multiple series

```rust
use ruviz::plots::categorical::{BarConfig, BarMode, compute_grouped_bars};

let config = BarConfig::new().mode(BarMode::Grouped);
let groups = vec![
    vec![10.0, 20.0, 30.0],  // Series 1
    vec![15.0, 25.0, 35.0],  // Series 2
];

let bars = compute_grouped_bars(&groups, &config);
```

### Stacked Bar Charts

**Use for**: Part-to-whole relationships across categories

```rust
use ruviz::plots::categorical::{BarConfig, BarMode, compute_stacked_bars};

let config = BarConfig::new().mode(BarMode::Stacked);
let bars = compute_stacked_bars(&groups, &config);
```

### Horizontal Bar Charts

**Use for**: Long category labels, ranked data

```rust
use ruviz::plots::categorical::{BarConfig, BarOrientation};

let config = BarConfig::new().orientation(BarOrientation::Horizontal);
```

---

## Composition Plots

### Pie Charts

**Use for**: Part-to-whole proportions

```rust
use ruviz::plots::composition::{PieConfig, compute_pie};

let values = vec![30.0, 25.0, 20.0, 15.0, 10.0];
let labels = vec!["A", "B", "C", "D", "E"];

let config = PieConfig::new()
    .start_angle(90.0)
    .labels(labels)
    .show_percent(true)
    .explode(vec![0.0, 0.1, 0.0, 0.0, 0.0]);  // Explode slice B

let pie = compute_pie(&values, &config);
// pie.wedges contains arc coordinates for each slice
```

### Donut Charts

**Use for**: Pie chart with center hole (modern aesthetic)

```rust
let config = PieConfig::new()
    .inner_radius(0.5)  // Creates donut hole
    .outer_radius(1.0);
```

### Area Charts

**Use for**: Cumulative quantities over time, filled line plots

```rust
use ruviz::plots::continuous::{AreaConfig, compute_area};

let config = AreaConfig::new()
    .fill_alpha(0.5)
    .stacked(false);

let area = compute_area(&x, &y, &config);
```

### Stacked Area Charts

**Use for**: Part-to-whole over continuous axis

```rust
let config = AreaConfig::new().stacked(true);
let areas = compute_stacked_area(&x, &series, &config);
```

---

## Continuous Plots

### Contour Plots

**Use for**: 2D density visualization, level curves

```rust
use ruviz::plots::continuous::{ContourConfig, compute_contour};

let config = ContourConfig::new()
    .levels(10)
    .filled(true);

let contour = compute_contour(&z_data, x_range, y_range, &config);
// contour.level_paths contains paths for each contour level
```

### Hexbin Plots

**Use for**: Large scatter datasets, 2D histogram with hexagonal bins

```rust
use ruviz::plots::continuous::{HexbinConfig, HexbinReduce, compute_hexbin};

let config = HexbinConfig::new()
    .gridsize(20)
    .reduce(HexbinReduce::Count);  // or Mean, Sum, Max, Min

let hexbin = compute_hexbin(&x, &y, &config);
// hexbin.hexagons contains bin positions and values
```

---

## Error Plots

### Error Bars

**Use for**: Uncertainty visualization in scientific data

```rust
use ruviz::plots::error::{ErrorBarConfig, ErrorValues, compute_error_bars};

// Symmetric errors
let config = ErrorBarConfig::new()
    .cap_size(5.0)
    .line_width(1.5);

let errors = ErrorValues::Symmetric(vec![0.5, 0.3, 0.4, 0.6]);
let bars = compute_error_bars(&x, &y, &errors, &config);

// Asymmetric errors
let errors = ErrorValues::Asymmetric {
    lower: vec![0.3, 0.2, 0.3, 0.4],
    upper: vec![0.5, 0.4, 0.5, 0.6],
};
```

---

## Discrete Plots

### Step Plots

**Use for**: Discrete data, histogram outlines, signal processing

```rust
use ruviz::plots::discrete::{StepConfig, StepMode, compute_step};

let config = StepConfig::new()
    .mode(StepMode::Pre)  // or Post, Mid
    .fill(false);

let step = compute_step(&x, &y, &config);
// step.vertices contains step function coordinates
```

### Stem Plots (Lollipop Charts)

**Use for**: Discrete sequences, emphasizing individual values

```rust
use ruviz::plots::discrete::{StemConfig, compute_stem};

let config = StemConfig::new()
    .marker_size(6.0)
    .baseline(0.0);

let stem = compute_stem(&x, &y, &config);
// stem.lines and stem.markers for rendering
```

---

## Regression Plots

### Regression Plot

**Use for**: Scatter with fitted regression line and confidence interval

```rust
use ruviz::plots::regression::{RegressionConfig, compute_regression};

let config = RegressionConfig::new()
    .order(1)  // Linear (1) or polynomial (2, 3, ...)
    .ci(0.95)  // 95% confidence interval
    .scatter(true);

let reg = compute_regression(&x, &y, &config);
// reg.fit_line, reg.ci_lower, reg.ci_upper, reg.scatter_points
```

### Residual Plot

**Use for**: Regression diagnostics, checking model fit

```rust
use ruviz::plots::regression::{ResidualConfig, compute_residuals};

let config = ResidualConfig::new()
    .lowess(true);  // Add LOWESS smoothing line

let resid = compute_residuals(&x, &y, &config);
// resid.residuals contains (x, residual) points
```

---

## Polar Plots

### Polar Line/Scatter

**Use for**: Circular data, angular measurements, wind roses

```rust
use ruviz::plots::polar::{PolarConfig, compute_polar};

let theta = vec![0.0, PI/4.0, PI/2.0, PI, 3.0*PI/2.0];  // Angles in radians
let r = vec![1.0, 2.0, 1.5, 2.5, 1.0];  // Radii

let config = PolarConfig::new()
    .direction(PolarDirection::CounterClockwise)
    .zero_location(ZeroLocation::East);

let polar = compute_polar(&theta, &r, &config);
// polar.cartesian_points for rendering
```

### Radar/Spider Charts

**Use for**: Multi-variable comparison, performance profiles

```rust
use ruviz::plots::polar::{RadarConfig, compute_radar};

let categories = vec!["Speed", "Power", "Range", "Defense", "Magic"];
let values = vec![0.8, 0.6, 0.9, 0.7, 0.5];  // Normalized 0-1

let config = RadarConfig::new()
    .fill(true)
    .fill_alpha(0.3);

let radar = compute_radar(&values, &categories, &config);
// radar.polygon_points for the filled area
```

---

## Composite Plots

### Joint Plots

**Use for**: Bivariate distribution with marginal distributions

```rust
use ruviz::plots::composite::{JointConfig, JointKind, compute_joint};

let config = JointConfig::new()
    .kind(JointKind::Scatter)  // or Hex, KDE
    .marginal(MarginalKind::Histogram);  // or KDE, Box

let joint = compute_joint(&x, &y, &config);
// joint.center, joint.marginal_x, joint.marginal_y
```

### Pair Plots

**Use for**: Multi-variable exploration, scatterplot matrix

```rust
use ruviz::plots::composite::{PairConfig, compute_pairplot};

let data = vec![
    vec![1.0, 2.0, 3.0],  // Variable 1
    vec![4.0, 5.0, 6.0],  // Variable 2
    vec![7.0, 8.0, 9.0],  // Variable 3
];

let config = PairConfig::new()
    .diag(DiagKind::Histogram)  // or KDE
    .corner(false);  // Show full matrix or just lower triangle

let pair = compute_pairplot(&data, &config);
// pair.panels[i][j] for each subplot
```

---

## Vector Plots

### Quiver Plots

**Use for**: Vector fields, flow visualization

```rust
use ruviz::plots::vector::{QuiverConfig, QuiverPivot, compute_quiver};

let config = QuiverConfig::new()
    .scale(1.0)
    .pivot(QuiverPivot::Tail)  // or Middle, Tip
    .headwidth(3.0)
    .headlength(5.0);

let quiver = compute_quiver(&x, &y, &u, &v, &config);
// quiver.arrows contains arrow geometries
```

---

## Hierarchical Plots

### Dendrograms

**Use for**: Hierarchical clustering visualization

```rust
use ruviz::plots::hierarchical::{DendrogramConfig, DendrogramOrientation, compute_dendrogram};
use ruviz::stats::clustering::{linkage, pdist_euclidean, LinkageMethod};

// Compute hierarchical clustering
let distances = pdist_euclidean(&points);
let linkage_result = linkage(&distances, LinkageMethod::Single);

let config = DendrogramConfig::new()
    .orientation(DendrogramOrientation::Top)
    .color_threshold(Some(5.0))
    .labels(sample_labels);

let dendro = compute_dendrogram(&linkage_result, &config);
// dendro.links contains line segments for the tree
```

---

## Performance Considerations

### Small Datasets (< 1K points)
Default rendering is optimal. No special configuration needed.

### Medium Datasets (1K - 100K points)
Parallel features automatically optimize rendering.

```toml
[dependencies]
ruviz = { version = "0.1", features = ["parallel"] }
```

### Large Datasets (100K - 1M points)
Use SIMD optimization and consider downsampling.

```toml
[dependencies]
ruviz = { version = "0.1", features = ["parallel", "simd"] }
```

### Very Large Datasets (> 1M points)
DataShader-style aggregation automatically activates for density-based plots.

---

## Next Steps

- **[Styling & Themes](05_styling.md)** - Customize colors, markers, themes
- **[Subplots](06_subplots.md)** - Multi-panel figures
- **[Performance](08_performance.md)** - Optimize for large datasets

## Quick Reference

| Category | Plot Type | Primary Use |
|----------|-----------|-------------|
| Distribution | Violin | Category comparison with density |
| Distribution | KDE | Smooth density estimation |
| Distribution | Boxen | Large dataset box plots |
| Distribution | ECDF | Cumulative distribution |
| Categorical | Strip | Jittered scatter by category |
| Categorical | Swarm | Non-overlapping scatter |
| Composition | Pie/Donut | Part-to-whole |
| Composition | Area | Cumulative over time |
| Continuous | Contour | 2D level curves |
| Continuous | Hexbin | Large scatter binning |
| Error | Error Bars | Uncertainty visualization |
| Discrete | Step | Discrete sequences |
| Discrete | Stem | Lollipop charts |
| Regression | Regplot | Fitted line + CI |
| Polar | Polar | Circular coordinates |
| Polar | Radar | Multi-axis radial |
| Composite | Joint | Bivariate + marginals |
| Composite | Pair | Scatterplot matrix |
| Vector | Quiver | Vector fields |
| Hierarchical | Dendrogram | Clustering trees |

---

**Ready to customize?** → [Styling & Themes](05_styling.md)
