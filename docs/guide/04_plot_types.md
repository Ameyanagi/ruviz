# Plot Types

Comprehensive guide to all plot types available in ruviz.

## Overview

ruviz provides **30+ plot types** across multiple categories, achieving parity with matplotlib, seaborn, and Makie.jl for scientific and data visualization.

The examples below mix the high-level `Plot` builder with lower-level helpers from
`ruviz::plots::*`. For the low-level examples, the symbol names and signatures
match the current exported APIs.

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
    .histogram(&data, Some(HistogramConfig { bins: Some(30), ..Default::default() }))
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
use ruviz::plots::distribution::{
    BandwidthMethod, ViolinConfig, ViolinData, close_violin_polygon, violin_polygon,
};

let config = ViolinConfig::new()
    .bandwidth(BandwidthMethod::Scott)
    .box_plot(true)
    .median(true);

let violin = ViolinData::from_values(&data, &config).unwrap();
let (left, right) = violin_polygon(&violin, 0.5, 0.4, &config);
let polygon = close_violin_polygon(&left, &right);
```

**Features**:
- Kernel density estimation on both sides
- Optional inner box plot
- Configurable bandwidth (Scott, Silverman, custom)
- Vertical or horizontal orientation

### KDE Plots (Kernel Density Estimation)

**Use for**: Smooth distribution curves, density estimation

```rust
use ruviz::plots::distribution::{Kde2dPlotConfig, KdeConfig, compute_kde, compute_kde_2d_plot};

// 1D KDE
let config = KdeConfig::new()
    .bandwidth(0.5)
    .n_points(200)
    .cumulative(false);

let kde_data = compute_kde(&data, &config);
// kde_data.x and kde_data.y contain the smooth density curve

// 2D KDE for bivariate density
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
use ruviz::plots::categorical::{StripConfig, compute_strip_points};

let categories = vec![0, 0, 0, 1, 1, 1];
let config = StripConfig::new()
    .jitter(0.1);

let strip_points = compute_strip_points(&categories, &values, None, &config);
// strip_points contains jittered positions
```

### Swarm Plots (Beeswarm)

**Use for**: Non-overlapping categorical scatter

```rust
use ruviz::plots::categorical::{SwarmConfig, compute_swarm_points};

let categories = vec![0, 0, 0, 1, 1, 1];
let config = SwarmConfig::new()
    .size(5.0);

let swarm_points = compute_swarm_points(&categories, &values, None, &config);
// swarm_points are positioned to avoid overlap
```

---

## Categorical Plots

### Grouped Bar Charts

**Use for**: Side-by-side comparison of multiple series

```rust
use ruviz::plots::categorical::{GroupedBarConfig, compute_grouped_bars};

let groups = vec![
    vec![10.0, 20.0, 30.0],  // Series 1
    vec![15.0, 25.0, 35.0],  // Series 2
];

let config = GroupedBarConfig::new().group_width(0.8).bar_gap(0.05);
let bars = compute_grouped_bars(&groups, 3, &config);
```

### Stacked Bar Charts

**Use for**: Part-to-whole relationships across categories

```rust
use ruviz::plots::categorical::{StackedBarConfig, compute_stacked_bars};

let config = StackedBarConfig::new().width(0.8);
let bars = compute_stacked_bars(&groups, 3, &config);
```

### Horizontal Bar Charts

**Use for**: Long category labels, ranked data

```rust
use ruviz::plots::categorical::{GroupedBarConfig, compute_grouped_bars};

let config = GroupedBarConfig::new().horizontal();
let bars = compute_grouped_bars(&groups, 3, &config);
```

---

## Composition Plots

### Pie Charts

**Use for**: Part-to-whole proportions

```rust
use ruviz::plots::composition::{PieConfig, PieData};

let values = vec![30.0, 25.0, 20.0, 15.0, 10.0];
let labels = vec!["A", "B", "C", "D", "E"]
    .into_iter()
    .map(String::from)
    .collect();

let config = PieConfig::new(labels)
    .start_angle(90.0)
    .percentages(true)
    .explode(vec![0.0, 0.1, 0.0, 0.0, 0.0]);

let pie = PieData::compute(&values, &config);
// pie.wedges contains arc coordinates for each slice
```

### Donut Charts

**Use for**: Pie chart with center hole (modern aesthetic)

```rust
let labels = vec!["A".to_string(), "B".to_string(), "C".to_string()];
let config = PieConfig::new(labels).donut(0.5);
```

### Area Charts

**Use for**: Cumulative quantities over time, filled line plots

```rust
use ruviz::plots::continuous::{AreaConfig, area_polygon};

let config = AreaConfig::new()
    .alpha(0.5)
    .baseline(0.0);

let area = area_polygon(&x, &y, config.baseline);
```

### Stacked Area Charts

**Use for**: Part-to-whole over continuous axis

```rust
use ruviz::plots::continuous::{StackBaseline, StackPlotConfig, compute_stack};

let config = StackPlotConfig::new().baseline(StackBaseline::Zero);
let areas = compute_stack(&x, &series, config.baseline);
```

---

## Continuous Plots

### Contour Plots

**Use for**: 2D density visualization, level curves

```rust
use ruviz::plots::continuous::{ContourConfig, compute_contour_plot};

let config = ContourConfig::new()
    .n_levels(10)
    .filled(true);

let contour = compute_contour_plot(&x, &y, &z_data, &config);
// contour.lines contains contour segments for each level
```

### Hexbin Plots

**Use for**: Large scatter datasets, 2D histogram with hexagonal bins

```rust
use ruviz::plots::continuous::{HexbinConfig, ReduceFunction, compute_hexbin};

let config = HexbinConfig::new()
    .gridsize(20)
    .reduce_fn(ReduceFunction::Count);  // or Mean, Sum, Max, Min, Std

let hexbin = compute_hexbin(&x, &y, None, &config);
// hexbin.bins contains bin positions and aggregated values
```

---

## Error Plots

### Error Bars

**Use for**: Uncertainty visualization in scientific data

```rust
use ruviz::plots::error::{ErrorBarConfig, ErrorValues, compute_error_bars};

let config = ErrorBarConfig::new()
    .cap_size(5.0)
    .line_width(1.5);

// Geometry helpers take optional x/y error values directly.
let y_errors = ErrorValues::symmetric(vec![0.5, 0.3, 0.4, 0.6]);
let bars = compute_error_bars(&x, &y, Some(&y_errors), None);

// Asymmetric errors
let x_errors = ErrorValues::asymmetric(
    vec![0.3, 0.2, 0.3, 0.4],
    vec![0.5, 0.4, 0.5, 0.6],
);
let bars_with_xy = compute_error_bars(&x, &y, Some(&y_errors), Some(&x_errors));
```

---

## Discrete Plots

### Step Plots

**Use for**: Discrete data, histogram outlines, signal processing

```rust
use ruviz::plots::discrete::{StepConfig, StepWhere, step_line};

let config = StepConfig::new()
    .where_step(StepWhere::Pre)  // or Post, Mid
    .fill(false);

let step = step_line(&x, &y, config.where_step);
// step contains the rendered step vertices
```

### Stem Plots (Lollipop Charts)

**Use for**: Discrete sequences, emphasizing individual values

```rust
use ruviz::plots::discrete::{StemConfig, compute_stems};

let config = StemConfig::new()
    .marker_size(6.0)
    .baseline(0.0);

let stem = compute_stems(&x, &y, &config);
// stem contains one StemElement per sample
```

---

## Regression Plots

### Regression Plot

**Use for**: Scatter with fitted regression line and confidence interval

```rust
use ruviz::plots::regression::{RegPlotConfig, compute_regplot};

let config = RegPlotConfig::new()
    .order(1)             // Linear (1) or polynomial (2, 3, ...)
    .ci(Some(95.0))       // 95% confidence interval
    .scatter_size(5.0);

let reg = compute_regplot(&x, &y, &config);
// reg.line_x / reg.line_y and optional reg.ci_lower / reg.ci_upper
```

### Residual Plot

**Use for**: Regression diagnostics, checking model fit

```rust
use ruviz::plots::regression::{ResidPlotConfig, compute_residplot};

let config = ResidPlotConfig::new()
    .lowess(true);  // Add LOWESS smoothing line

let resid = compute_residplot(&x, &y, &config);
// resid.x contains fitted values and resid.residuals contains the residuals
```

---

## Polar Plots

### Polar Line/Scatter

**Use for**: Circular data, angular measurements, wind roses

```rust
use ruviz::plots::polar::{PolarPlotConfig, compute_polar_plot};

let theta = vec![0.0, PI/4.0, PI/2.0, PI, 3.0*PI/2.0];  // Angles in radians
let r = vec![1.0, 2.0, 1.5, 2.5, 1.0];  // Radii

let config = PolarPlotConfig::new()
    .theta_offset(std::f64::consts::FRAC_PI_2)
    .show_theta_labels(true)
    .show_r_labels(true);

let polar = compute_polar_plot(&r, &theta, &config);
// polar.points contains the resolved cartesian/polar coordinates
```

### Radar/Spider Charts

**Use for**: Multi-variable comparison, performance profiles

```rust
use ruviz::plots::polar::{RadarConfig, compute_radar_chart};

let categories = vec!["Speed", "Power", "Range", "Defense", "Magic"];
let values = vec![vec![0.8, 0.6, 0.9, 0.7, 0.5]];  // One radar series

let config = RadarConfig::new()
    .labels(categories.into_iter().map(String::from).collect())
    .fill(true)
    .fill_alpha(0.3);

let radar = compute_radar_chart(&values, &config);
// radar.series contains one polygon/marker set per input series
```

---

## Composite Plots

### Joint Plot Helpers

**Use for**: Joint-plot style layout and marginal histogram helpers

```rust
use ruviz::plots::composite::{
    JointKind, JointPlotConfig, compute_marginal_histogram, joint_plot_layout,
};

let config = JointPlotConfig::new()
    .kind(JointKind::Hex)
    .marginal_hist(true)
    .marginal_kde(false);

let layout = joint_plot_layout(config.marginal_ratio);
let x_hist = compute_marginal_histogram(&x, config.bins);
let y_hist = compute_marginal_histogram(&y, config.bins);
// layout plus x_hist / y_hist can be used to assemble a joint-plot style figure
```

### Pair Plot Helpers

**Use for**: Scatterplot-matrix layout and labeling helpers

```rust
use ruviz::plots::composite::{
    DiagKind, OffDiagKind, PairPlotConfig, cell_variable_names, compute_pairplot_layout,
};

let data = vec![
    vec![1.0, 2.0, 3.0],  // Variable 1
    vec![4.0, 5.0, 6.0],  // Variable 2
    vec![7.0, 8.0, 9.0],  // Variable 3
];

let config = PairPlotConfig::new()
    .diag_kind(DiagKind::Hist)
    .off_diag_kind(OffDiagKind::Scatter)
    .lower_only()
    .vars(vec!["x".into(), "y".into(), "z".into()]);

let pair = compute_pairplot_layout(data.len(), &config);
let first_cell = &pair.cells[0];
let (x_name, y_name) = cell_variable_names(first_cell, &config.vars);
```

---

## Vector Plots

### Quiver Plots

**Use for**: Vector fields, flow visualization

```rust
use ruviz::plots::vector::{QuiverConfig, QuiverPivot, compute_quiver};

let mut config = QuiverConfig::new()
    .scale(1.0)
    .width(1.5)
    .pivot(QuiverPivot::Tail)
    .color_by_magnitude(true);

config.headwidth = 0.25;
config.headlength = 0.35;

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
    .color_threshold(5.0)
    .labels(sample_labels);

let dendro = compute_dendrogram(&linkage_result, &config);
// dendro.links contains line segments for the tree
```

---

## Performance Considerations

### Small Datasets (< 1K points)
Default rendering is optimal. No special configuration needed.

### Medium Datasets (1K - 100K points)
The `parallel` feature can speed up the in-memory `render()` path. Reactive
plots are resolved to a static snapshot first, so observable-backed series can
use the same path as static data.

```toml
[dependencies]
ruviz = { version = "0.4.2", features = ["parallel"] }
```

### Large Datasets (20K - 100K points)
Use `parallel` and `simd` for heavier in-memory rendering workloads, and
consider downsampling where visual density is already saturated.

```toml
[dependencies]
ruviz = { version = "0.4.2", features = ["parallel", "simd"] }
```

### Very Large Datasets (> 100K points)
DataShader-style aggregation can activate above `100_000` total points for
aggregation-friendly series such as scatter and histogram. The exact
`render()` vs `save()` behavior is documented in [Performance](08_performance.md).

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
| Composite | Joint helpers | Bivariate layout + marginals |
| Composite | Pair helpers | Scatterplot-matrix layout |
| Vector | Quiver | Vector fields |
| Hierarchical | Dendrogram | Clustering trees |

---

**Ready to customize?** → [Styling & Themes](05_styling.md)
