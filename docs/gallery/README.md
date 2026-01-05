# ruviz Gallery

Comprehensive visual showcase of ruviz plotting capabilities with **30+ plot types**.

## Plot Type Categories

### Basic Plots
Fundamental visualization types for everyday use.

| Plot Type | Description | Use Case |
|-----------|-------------|----------|
| Line | Connected data points | Time series, trends |
| Scatter | Individual markers | Correlations, point clouds |
| Bar | Categorical bars | Comparisons |
| Histogram | Frequency bins | Distributions |
| Box Plot | Statistical summary | Quartiles, outliers |
| Heatmap | 2D color matrix | Correlations, matrices |

[View Basic Examples →](basic/README.md)

---

### Distribution Plots
Statistical distribution visualization.

| Plot Type | Description | Use Case |
|-----------|-------------|----------|
| Violin | KDE + box plot | Distribution shape comparison |
| KDE | Smooth density curve | Continuous distributions |
| Boxen | Letter-value plot | Large datasets |
| ECDF | Step cumulative | Distribution comparison |
| Strip | Jittered categorical | Individual points by category |
| Swarm | Non-overlapping | Dense categorical data |

[View Distribution Examples →](statistical/README.md)

---

### Categorical Plots
Comparisons across categories.

| Plot Type | Description | Use Case |
|-----------|-------------|----------|
| Grouped Bar | Side-by-side bars | Multi-series comparison |
| Stacked Bar | Stacked segments | Part-to-whole by category |
| Horizontal Bar | Rotated bars | Long category labels |

---

### Composition Plots
Part-to-whole relationships.

| Plot Type | Description | Use Case |
|-----------|-------------|----------|
| Pie | Circular wedges | Proportions |
| Donut | Pie with center hole | Modern proportions |
| Area | Filled line plot | Cumulative over time |
| Stacked Area | Multiple areas | Part-to-whole over time |

---

### Continuous Plots
2D density and field visualization.

| Plot Type | Description | Use Case |
|-----------|-------------|----------|
| Contour | Level curves | 2D density, topography |
| Hexbin | Hexagonal bins | Large scatter datasets |
| Fill Between | Shaded region | Ranges, uncertainty |

---

### Error Visualization
Uncertainty and confidence.

| Plot Type | Description | Use Case |
|-----------|-------------|----------|
| Error Bars | Symmetric/asymmetric | Measurement uncertainty |

---

### Discrete Plots
Discrete and step data.

| Plot Type | Description | Use Case |
|-----------|-------------|----------|
| Step | Step function | Discrete signals |
| Stem | Lollipop markers | Discrete sequences |

---

### Regression Plots
Statistical modeling visualization.

| Plot Type | Description | Use Case |
|-----------|-------------|----------|
| Regplot | Scatter + fit line + CI | Linear/polynomial regression |
| Residplot | Residual scatter | Model diagnostics |

---

### Polar Plots
Circular coordinate systems.

| Plot Type | Description | Use Case |
|-----------|-------------|----------|
| Polar | Circular line/scatter | Angular data |
| Radar | Multi-axis radial | Multi-variable comparison |

---

### Composite Plots
Multi-panel visualizations.

| Plot Type | Description | Use Case |
|-----------|-------------|----------|
| Joint Plot | Center + marginals | Bivariate distributions |
| Pair Plot | Scatterplot matrix | Multi-variable exploration |

---

### Vector Plots
Directional data.

| Plot Type | Description | Use Case |
|-----------|-------------|----------|
| Quiver | Arrow field | Vector fields, flow |

---

### Hierarchical Plots
Tree and cluster structures.

| Plot Type | Description | Use Case |
|-----------|-------------|----------|
| Dendrogram | Clustering tree | Hierarchical clustering |

---

## Gallery Sections

### Basic Plots
Fundamental plot types for everyday visualization.

[View Basic Plots Examples →](basic/README.md)

### Statistical Plots
Statistical analysis and distributions including violin, KDE, boxen, and ECDF plots.

[View Statistical Plots Examples →](statistical/README.md)

### Publication Quality
Professional figures for journals and presentations.

[View Publication Quality Examples →](publication/README.md)

### Performance
Large dataset handling and optimization demonstrations.

[View Performance Examples →](performance/README.md)

### Advanced Techniques
Complex visualizations including polar plots, composite plots, and custom styling.

[View Advanced Techniques Examples →](advanced/README.md)

---

## Quick Start Examples

### Violin Plot
```rust
use ruviz::plots::distribution::{ViolinConfig, ViolinData};

let violin = ViolinData::from_data(&data, &ViolinConfig::default());
```

### KDE Plot
```rust
use ruviz::plots::distribution::{KdePlotConfig, compute_kde_plot};

let kde = compute_kde_plot(&data, &KdePlotConfig::default());
```

### Pie Chart
```rust
use ruviz::plots::composition::{PieConfig, compute_pie};

let pie = compute_pie(&values, &PieConfig::default());
```

### Contour Plot
```rust
use ruviz::plots::continuous::{ContourConfig, compute_contour};

let contour = compute_contour(&z_data, x_range, y_range, &ContourConfig::default());
```

### Radar Chart
```rust
use ruviz::plots::polar::{RadarConfig, compute_radar};

let radar = compute_radar(&values, &categories, &RadarConfig::default());
```

---

## Regenerate Gallery

To regenerate all gallery images:

```bash
cargo run --bin generate_gallery --release
```

---

**Total Plot Types**: 30+
**Gallery Examples**: 50+
