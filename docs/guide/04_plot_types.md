# Plot Types

Comprehensive guide to all plot types available in ruviz.

## Overview

ruviz provides **29 drawable plot types** from the `Plot` builder, plus 4 more from
`Plot3D` when the `3d` feature is enabled.

`ruviz::plots::*` additionally exposes compute helpers for a couple of chart
families. Those helpers are correct and usable, but they have **no renderer and
no `Plot` builder method**, so they cannot produce an image. They are marked
⚠️ **compute only** below. The canonical list is
[the `ruviz::plots` module docs](https://docs.rs/ruviz/latest/ruviz/plots/),
which a test keeps in step with the builder's own source.

The examples further down mix the high-level `Plot` builder with those low-level
helpers; the symbol names and signatures match the current exported APIs.

### Quick Reference by Category

| Category | Drawable from `Plot` | ⚠️ Compute only — no `Plot` builder |
|----------|----------------------|--------------------------------------|
| **Basic** | Line, Scatter, Bar, Histogram, Box Plot, Heatmap | — |
| **Distribution** | Violin, KDE (1D), Boxen, ECDF, Rug | KDE 2D |
| **Categorical** | Strip, Swarm, Grouped Bar, Stacked Bar | — |
| **Composition** | Pie, Donut, Area | — |
| **Continuous** | Contour, Fill Between, Hexbin, Stacked Area | — |
| **Error** | Error Bars (symmetric/asymmetric) | — |
| **Discrete** | Step, Stem | — |
| **Polar** | Polar Plot, Radar/Spider Chart | — |
| **Vector** | Quiver Plot | — |
| **Regression** | — | Regression Plot, Residual Plot |
| **Composite** | Joint Plot, Pair Plot (figures, via `plots::composite`) | — |
| **Hierarchical** | Dendrogram | — |
| **3D** (`3d` feature) | Scatter3D, Line3D, Surface3D, Wireframe3D | — |

Joint plots and pair plots are *figures*, not series: `plots::composite::{jointplot,
pairplot}` return a `SubplotFigure`, the same type `subplots` returns, so they are
not part of the 29 above.

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

let data: Vec<f64> = (0..1000).map(|i| /* sample data */).collect();

Plot::new()
    .histogram(&data)
    .bins(30)
    .title("Distribution")
    .save("histogram.png")?;
```

Already holding a `HistogramConfig`? `histogram_with` takes one by value:

```rust
use ruviz::prelude::*;
use ruviz::plots::histogram::HistogramConfig;

let data: Vec<f64> = (0..1000).map(|i| /* sample data */).collect();

Plot::new()
    .histogram_with(&data, HistogramConfig::new().bins(30))
    .save("histogram.png")?;
```

### Box Plots

**Use for**: Statistical summary, outlier detection

```rust
use ruviz::prelude::*;

let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 35.0];

Plot::new()
    .boxplot(&data)
    .show_mean(true)
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
use ruviz::prelude::*;

Plot::new()
    .boxen(&data)
    .k_depth(5)
    .show_outliers(true)
    .save("boxen.png")?;
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
use ruviz::prelude::*;

Plot::new()
    .strip(&["a", "a", "b", "b"], &vec![1.0, 2.0, 3.0, 4.0])
    .jitter(0.1)
    .label("samples")
    .save("strip.png")?;
```

The compute helper behind it is `ruviz::plots::categorical::compute_strip_points`,
if you want the jittered positions without a figure.

### Swarm Plots (Beeswarm)

**Use for**: Non-overlapping categorical scatter

```rust
use ruviz::prelude::*;

Plot::new()
    .swarm(&["a", "a", "b", "b"], &vec![1.0, 2.0, 3.0, 4.0])
    .marker_size(5.0)
    .label("samples")
    .save("swarm.png")?;
```

---

## Categorical Plots

### Grouped Bar Charts

**Use for**: Side-by-side comparison of multiple series

Grouped bars, stacked bars and stacked areas are the crate's only *multi*-series
plot types: they take N `(name, values)` pairs over one shared axis and push one
ordinary series per pair. Each column therefore gets its own palette colour, its
own legend entry, and the same chain as everything else.

```rust
use ruviz::prelude::*;

let last: Vec<f64> = vec![10.0, 20.0, 30.0];
let this: Vec<f64> = vec![15.0, 25.0, 35.0];

Plot::new()
    .grouped_bar(&["Q1", "Q2", "Q3"], &[("2023", &last), ("2024", &this)])
    .group_width(0.8)
    .bar_gap(0.05)
    .legend_best()
    .save("grouped_bar.png")?;
```

### Stacked Bar Charts

**Use for**: Part-to-whole relationships across categories

Positive contributions stack upwards from the baseline and negative ones
downwards, so a column that dips below zero does not eat into the stack above it.

```rust
use ruviz::prelude::*;

let hardware: Vec<f64> = vec![10.0, 20.0, 30.0];
let services: Vec<f64> = vec![5.0, 8.0, 12.0];

Plot::new()
    .stacked_bar(&["Q1", "Q2", "Q3"], &[("hardware", &hardware), ("services", &services)])
    .bar_width(0.8)
    .legend_best()
    .save("stacked_bar.png")?;
```

### Horizontal Bar Charts

**Use for**: Long category labels, ranked data

`.horizontal()` is available on both bar builders. Note that the shared category
axis is the x axis, so a horizontal chart's categories are not yet labelled —
the same gap `strip` and `swarm` have in their horizontal orientation.

```rust
use ruviz::prelude::*;

let last: Vec<f64> = vec![10.0, 20.0, 30.0];

Plot::new()
    .grouped_bar(&["Q1", "Q2", "Q3"], &[("2023", &last)])
    .horizontal()
    .save("horizontal_bar.png")?;
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

The numeric twin of the stacked bar chart: same `(name, values)` pairs, numeric x
positions in place of category names.

```rust
use ruviz::prelude::*;
use ruviz::plots::continuous::StackBaseline;

let years: Vec<f64> = vec![2020.0, 2021.0, 2022.0];
let solar: Vec<f64> = vec![1.0, 2.0, 4.0];
let wind: Vec<f64> = vec![3.0, 3.5, 4.5];

Plot::new()
    .stacked_area(&years, &[("solar", &solar), ("wind", &wind)])
    .baseline(StackBaseline::Zero)
    .legend_best()
    .save("stacked_area.png")?;
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
use ruviz::prelude::*;

Plot::new()
    .hexbin(&x, &y)
    .gridsize(20)
    .label("density")
    .save("hexbin.png")?;
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
use ruviz::prelude::*;

Plot::new()
    .step(&x, &y, StepWhere::Pre)
    .line_width(2.0)
    .save("step.png")?;
```

### Stem Plots (Lollipop Charts)

**Use for**: Discrete sequences, emphasizing individual values

```rust
use ruviz::prelude::*;

Plot::new()
    .stem(&x, &y, 0.0)
    .marker_size(6.0)
    .save("stem.png")?;
```

---

## Regression Plots

### Regression Plot

> ⚠️ **Compute only.** This returns data, not a drawable plot — there is no
> `Plot` builder method and no renderer behind it.

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

> ⚠️ **Compute only.** This returns data, not a drawable plot — there is no
> `Plot` builder method and no renderer behind it.

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

### Joint Plots

**Use for**: A bivariate panel with its two marginal distributions

`jointplot` returns a `SubplotFigure` — the same figure type `subplots` returns —
so it is finished with `.suptitle(..)` / `.theme(..)` / `.save(..)` rather than
with the series chain.

```rust
use ruviz::plots::composite::jointplot;

jointplot(&x, &y, 800, 800)?
    .suptitle("x vs y")
    .save("jointplot.png")?;
```

#### Joint plot layout helpers

The geometry those composers use, if you want to place the panels yourself with
[`SubplotFigure::add_axes`](https://docs.rs/ruviz/latest/ruviz/core/subplot/struct.SubplotFigure.html#method.add_axes).

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
// layout plus x_hist / y_hist is what `jointplot(&x, &y, 800, 800)?` assembles
// for you; use them directly only if you want a different arrangement.
```

### Pair Plots

**Use for**: A scatterplot matrix over several variables

Like `jointplot`, `pairplot` returns a `SubplotFigure`.

```rust
use ruviz::plots::composite::pairplot;

let columns = vec![
    vec![1.0, 2.0, 3.0],
    vec![4.0, 5.0, 6.0],
    vec![7.0, 8.0, 9.0],
];

pairplot(&columns, 900, 900)?
    .suptitle("pairwise")
    .save("pairplot.png")?;
```

#### Pair plot layout helpers

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
use ruviz::prelude::*;

Plot::new()
    .quiver(&x, &y, &u, &v)
    .arrow_scale(1.0)
    .arrow_width(1.5)
    .pivot(QuiverPivot::Tail)
    .arrow_head_width(0.25)
    .arrow_head_length(0.35)
    .color_by_magnitude(true)
    .save("quiver.png")?;
```

---

## Hierarchical Plots

### Dendrograms

**Use for**: Hierarchical clustering visualization

`Plot::new().dendrogram(&linkage_result).save("tree.png")?` draws one; the
compute helpers below give you the raw link segments instead.

```rust
use ruviz::plots::hierarchical::{DendrogramConfig, DendrogramOrientation, compute_dendrogram};
use ruviz::stats::clustering::{linkage, pdist_euclidean, LinkageMethod};

// Compute hierarchical clustering
let distances = pdist_euclidean(&points);
let linkage_result = linkage(&distances, LinkageMethod::Single);

let config = DendrogramConfig::new()
    .orientation(DendrogramOrientation::Top)
    .labels(sample_labels);

let dendro = compute_dendrogram(&linkage_result, &config);
// dendro.links contains line segments for the tree
```

---

## Performance Considerations

### Small Datasets (< 1K points)
Default rendering is optimal. No special configuration needed.

### Medium Datasets (1K - 100K points)
Use release builds first. 2D rendering is single-threaded and needs no feature
flags; benchmark your actual plot before adding any.

### Large Datasets (20K+ points)
Consider downsampling or aggregating where visual density is already saturated.
Use `performance` only after benchmarking a path that benefits from SIMD support.

```toml
[dependencies]
ruviz = { version = "0.10.0", features = ["simd"] }
```

### Very Large Datasets (> 100K points)
The crate contains DataShader-related code and backend metadata, but the current
public `render()` and `save()` paths should not be documented as automatic
DataShader output. See [Performance](08_performance.md).

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
| Categorical | Grouped Bar | Side-by-side series comparison |
| Categorical | Stacked Bar | Part-to-whole across categories |
| Composition | Pie/Donut | Part-to-whole |
| Composition | Area | Cumulative over time |
| Continuous | Contour | 2D level curves |
| Continuous | Hexbin | Large scatter binning |
| Continuous | Stacked Area | Part-to-whole over a numeric axis |
| Error | Error Bars | Uncertainty visualization |
| Discrete | Step | Discrete sequences |
| Discrete | Stem | Lollipop charts |
| Regression | Regplot | Fitted line + CI |
| Polar | Polar | Circular coordinates |
| Polar | Radar | Multi-axis radial |
| Composite | Joint plot (figure) | Bivariate panel + marginals |
| Composite | Pair plot (figure) | Scatterplot matrix |
| Vector | Quiver | Vector fields |
| Hierarchical | Dendrogram | Clustering trees |

---

**Ready to customize?** → [Styling & Themes](05_styling.md)
