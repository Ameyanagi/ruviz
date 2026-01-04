# Subplots & Composition

Multi-panel figures and complex layouts for publication-quality visualizations.

## Overview

Subplots allow you to combine multiple plots into a single figure, essential for:
- Comparative analysis
- Multi-faceted data visualization
- Publication figures with multiple panels
- Dashboard-style layouts

## Basic Subplots

### 2×2 Grid

```rust
use ruviz::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Prepare data
    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y1 = vec![0.0, 1.0, 4.0, 9.0, 16.0];
    let y2 = vec![0.0, 2.0, 4.0, 6.0, 8.0];

    let categories = vec!["A", "B", "C", "D"];
    let values = vec![10.0, 15.0, 8.0, 12.0];

    // Create individual plots
    let plot1 = Plot::new()
        .line(&x, &y1)
        .title("Quadratic")
        .end_series();

    let plot2 = Plot::new()
        .scatter(&x, &y2)
        .title("Linear")
        .end_series();

    let plot3 = Plot::new()
        .bar(&categories, &values)
        .title("Categories")
        .end_series();

    let plot4 = Plot::new()
        .line(&x, &y1)
        .title("Combined")
        .end_series();

    // Compose into 2×2 grid
    subplots(2, 2, 800, 600)?
        .subplot(0, 0, plot1)?  // Top-left
        .subplot(0, 1, plot2)?  // Top-right
        .subplot(1, 0, plot3)?  // Bottom-left
        .subplot(1, 1, plot4)?  // Bottom-right
        .save("subplot_2x2.png")?;

    Ok(())
}
```

**Grid Indexing**:
```
(0,0) | (0,1)
------|------
(1,0) | (1,1)
```

### Single Row

```rust
use ruviz::prelude::*;

let plot1 = Plot::new().line(&x, &y1).title("Plot 1").end_series();
let plot2 = Plot::new().line(&x, &y2).title("Plot 2").end_series();
let plot3 = Plot::new().line(&x, &y3).title("Plot 3").end_series();

subplots(1, 3, 1200, 400)?
    .subplot(0, 0, plot1)?
    .subplot(0, 1, plot2)?
    .subplot(0, 2, plot3)?
    .save("subplot_1x3.png")?;
```

### Single Column

```rust
use ruviz::prelude::*;

let plot1 = Plot::new().line(&x, &y1).title("Top").end_series();
let plot2 = Plot::new().line(&x, &y2).title("Middle").end_series();
let plot3 = Plot::new().line(&x, &y3).title("Bottom").end_series();

subplots(3, 1, 600, 900)?
    .subplot(0, 0, plot1)?
    .subplot(1, 0, plot2)?
    .subplot(2, 0, plot3)?
    .save("subplot_3x1.png")?;
```

## Subplot Configuration

### Figure Title

```rust
use ruviz::prelude::*;

subplots(2, 2, 800, 600)?
    .suptitle("Comprehensive Analysis")  // Overall figure title
    .subplot(0, 0, plot1)?
    .subplot(0, 1, plot2)?
    .subplot(1, 0, plot3)?
    .subplot(1, 1, plot4)?
    .save("subplot_with_title.png")?;
```

### Spacing Control

```rust
use ruviz::prelude::*;

subplots(2, 2, 800, 600)?
    .hspace(0.3)  // Horizontal spacing (between rows)
    .wspace(0.3)  // Vertical spacing (between columns)
    .subplot(0, 0, plot1)?
    .subplot(0, 1, plot2)?
    .subplot(1, 0, plot3)?
    .subplot(1, 1, plot4)?
    .save("subplot_with_spacing.png")?;
```

**Spacing Guidelines**:
- **0.0**: No spacing - subplots fill available area (default)
- **0.05-0.1**: Tight layout with small gaps
- **0.1-0.2**: Normal spacing
- **0.2-0.3**: Generous spacing

### Custom Dimensions

```rust
use ruviz::prelude::*;

// Wide figure (16:9 aspect ratio)
subplots(1, 2, 1600, 900)?
    .subplot(0, 0, plot1)?
    .subplot(0, 1, plot2)?
    .save("wide_subplot.png")?;

// Tall figure
subplots(3, 1, 600, 1200)?
    .subplot(0, 0, plot1)?
    .subplot(1, 0, plot2)?
    .subplot(2, 0, plot3)?
    .save("tall_subplot.png")?;

// Square panels
subplots(2, 2, 1000, 1000)?
    .subplot(0, 0, plot1)?
    .subplot(0, 1, plot2)?
    .subplot(1, 0, plot3)?
    .subplot(1, 1, plot4)?
    .save("square_subplot.png")?;
```

## Scientific Multi-Panel Figures

### Publication Figure (2×2)

```rust
use ruviz::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate scientific datasets
    let time: Vec<f64> = (0..1000).map(|i| i as f64 * 0.01).collect();
    let signal: Vec<f64> = time.iter()
        .map(|&t| 5.0 * (-t * 0.2).exp() * (t * 3.0).sin())
        .collect();

    let x_values: Vec<f64> = (0..500).map(|i| i as f64 * 0.02).collect();
    let y_values: Vec<f64> = x_values.iter()
        .map(|&x| 2.5 * x + 1.2)
        .collect();

    let distribution: Vec<f64> = (0..2000).map(|i| {
        let t = i as f64 / 100.0;
        if i % 3 == 0 { 5.0 + (t * 7.0).sin() * 2.0 }
        else { 12.0 + (t * 11.0).cos() * 1.5 }
    }).collect();

    let group_data: Vec<f64> = (0..100).map(|i| {
        8.0 + (i as f64 * 0.1).sin() * 2.0
    }).collect();

    // Create panels with professional styling
    let panel_a = Plot::new()
        .title("A) Experimental Time Series")
        .xlabel("Time (seconds)")
        .ylabel("Signal Amplitude")
        .line(&time, &signal)
        .end_series()
        .theme(Theme::seaborn());

    let panel_b = Plot::new()
        .title("B) Correlation Analysis")
        .xlabel("Independent Variable")
        .ylabel("Dependent Variable")
        .scatter(&x_values, &y_values)
        .end_series()
        .theme(Theme::seaborn());

    let panel_c = Plot::new()
        .title("C) Distribution")
        .xlabel("Value Bins")
        .ylabel("Frequency")
        .histogram(&distribution, None)
        .end_series()
        .theme(Theme::seaborn());

    let panel_d = Plot::new()
        .title("D) Statistical Analysis")
        .xlabel("Groups")
        .ylabel("Values")
        .boxplot(&group_data, None)
        .end_series()
        .theme(Theme::seaborn());

    // Compose publication figure
    subplots(2, 2, 1600, 1200)?
        .suptitle("Scientific Data Analysis - Multi-Panel Figure")
        .hspace(0.3)
        .wspace(0.3)
        .subplot(0, 0, panel_a)?
        .subplot(0, 1, panel_b)?
        .subplot(1, 0, panel_c)?
        .subplot(1, 1, panel_d)?
        .save("publication_figure.png")?;

    Ok(())
}
```

**Panel Labeling Convention**:
- Use **A), B), C), D)** for publication figures
- Consistent with Nature, Science, Cell journal standards
- Place labels at top-left of each panel

### Comparison Layouts

**Before/After Analysis**:
```rust
use ruviz::prelude::*;

// Before optimization
let plot_before = Plot::new()
    .line(&time, &data_before)
    .title("Before Optimization")
    .xlabel("Time")
    .ylabel("Performance")
    .end_series();

// After optimization
let plot_after = Plot::new()
    .line(&time, &data_after)
    .title("After Optimization")
    .xlabel("Time")
    .ylabel("Performance")
    .end_series();

subplots(1, 2, 1200, 600)?
    .suptitle("Performance Optimization Results")
    .hspace(0.3)
    .wspace(0.3)
    .subplot(0, 0, plot_before)?
    .subplot(0, 1, plot_after)?
    .save("before_after.png")?;
```

**Multi-Group Comparison**:
```rust
use ruviz::prelude::*;

let group_a_plot = Plot::new()
    .histogram(&group_a_data, None)
    .title("Group A")
    .end_series();

let group_b_plot = Plot::new()
    .histogram(&group_b_data, None)
    .title("Group B")
    .end_series();

let group_c_plot = Plot::new()
    .histogram(&group_c_data, None)
    .title("Group C")
    .end_series();

subplots(1, 3, 1500, 500)?
    .suptitle("Multi-Group Distribution Analysis")
    .subplot(0, 0, group_a_plot)?
    .subplot(0, 1, group_b_plot)?
    .subplot(0, 2, group_c_plot)?
    .save("multi_group.png")?;
```

## Different Plot Types in Subplots

### Mixed Plot Types

```rust
use ruviz::prelude::*;

// Time series (line)
let timeseries = Plot::new()
    .line(&time, &measurements)
    .title("Time Series")
    .xlabel("Time (s)")
    .ylabel("Amplitude")
    .end_series();

// Correlation (scatter)
let correlation = Plot::new()
    .scatter(&x_var, &y_var)
    .marker(MarkerStyle::Circle)
    .title("Correlation")
    .xlabel("X Variable")
    .ylabel("Y Variable")
    .end_series();

// Categories (bar)
let categories_plot = Plot::new()
    .bar(&categories, &values)
    .title("Category Summary")
    .xlabel("Category")
    .ylabel("Count")
    .end_series();

// Distribution (histogram)
let distribution_plot = Plot::new()
    .histogram(&data, None)
    .title("Distribution")
    .xlabel("Value")
    .ylabel("Frequency")
    .end_series();

subplots(2, 2, 1200, 1000)?
    .suptitle("Comprehensive Data Analysis")
    .hspace(0.3)
    .wspace(0.3)
    .subplot(0, 0, timeseries)?
    .subplot(0, 1, correlation)?
    .subplot(1, 0, categories_plot)?
    .subplot(1, 1, distribution_plot)?
    .save("mixed_types.png")?;
```

## Large Subplot Grids

### 3×2 Grid

```rust
use ruviz::prelude::*;

let plots: Vec<_> = (0..6).map(|i| {
    Plot::new()
        .line(&x, &data[i])
        .title(&format!("Condition {}", i + 1))
        .end_series()
}).collect();

let mut figure = subplots(3, 2, 1400, 1200)?
    .suptitle("Multi-Condition Experiment Results")
    .hspace(0.35)
    .wspace(0.3);

for row in 0..3 {
    for col in 0..2 {
        let idx = row * 2 + col;
        figure = figure.subplot(row, col, plots[idx].clone())?;
    }
}

figure.save("large_grid.png")?;
```

### 2×3 Grid

```rust
use ruviz::prelude::*;

let mut figure = subplots(2, 3, 1800, 900)?
    .suptitle("Six-Panel Analysis")
    .hspace(0.3)
    .wspace(0.25);

for row in 0..2 {
    for col in 0..3 {
        let plot = Plot::new()
            .scatter(&x_data[row][col], &y_data[row][col])
            .title(&format!("Panel ({},{})", row, col))
            .end_series();
        figure = figure.subplot(row, col, plot)?;
    }
}

figure.save("panel_2x3.png")?;
```

## Styling Subplots

### Consistent Theme

```rust
use ruviz::prelude::*;

// Apply same theme to all panels
let theme = Theme::seaborn();

let plot1 = Plot::new()
    .line(&x, &y1)
    .title("Panel 1")
    .theme(theme.clone())
    .end_series();

let plot2 = Plot::new()
    .line(&x, &y2)
    .title("Panel 2")
    .theme(theme.clone())
    .end_series();

let plot3 = Plot::new()
    .line(&x, &y3)
    .title("Panel 3")
    .theme(theme.clone())
    .end_series();

let plot4 = Plot::new()
    .line(&x, &y4)
    .title("Panel 4")
    .theme(theme)
    .end_series();

subplots(2, 2, 1000, 800)?
    .subplot(0, 0, plot1)?
    .subplot(0, 1, plot2)?
    .subplot(1, 0, plot3)?
    .subplot(1, 1, plot4)?
    .save("consistent_theme.png")?;
```

### Color-Coordinated Panels

```rust
use ruviz::prelude::*;

let colors = [
    Color::from_rgb(76, 114, 176),   // Muted blue
    Color::from_rgb(221, 132, 82),   // Muted orange
    Color::from_rgb(85, 168, 104),   // Muted green
    Color::from_rgb(196, 78, 82),    // Muted red
];

let plots: Vec<_> = (0..4).map(|i| {
    Plot::new()
        .line(&x, &data[i])
        .color(colors[i])
        .title(&format!("Series {}", i + 1))
        .end_series()
}).collect();

let mut figure = subplots(2, 2, 1000, 800)?
    .suptitle("Color-Coordinated Analysis");

figure = figure.subplot(0, 0, plots[0].clone())?;
figure = figure.subplot(0, 1, plots[1].clone())?;
figure = figure.subplot(1, 0, plots[2].clone())?;
figure = figure.subplot(1, 1, plots[3].clone())?;

figure.save("color_coordinated.png")?;
```

## Publication-Ready Figures

### IEEE Format (Two-Column)

```rust
use ruviz::prelude::*;

// IEEE two-column figure: 7.25" wide @ 300 DPI = 2175 pixels
// Height: ~5.5" @ 300 DPI = 1650 pixels

let panel_a = Plot::new()
    .line(&x, &y1)
    .title("(a) Experimental Results")
    .xlabel("Input Parameter")
    .ylabel("Output Response")
    .end_series()
    .theme(Theme::publication());

let panel_b = Plot::new()
    .scatter(&x, &y2)
    .title("(b) Validation Data")
    .xlabel("Input Parameter")
    .ylabel("Output Response")
    .end_series()
    .theme(Theme::publication());

subplots(1, 2, 2175, 1000)?
    .suptitle("Figure 1: Comprehensive System Analysis")
    .hspace(0.3)
    .wspace(0.35)
    .subplot(0, 0, panel_a)?
    .subplot(0, 1, panel_b)?
    .save("ieee_figure.png")?;
```

### Nature Format (Single-Column)

```rust
use ruviz::prelude::*;

// Nature single-column: 89mm = 3.5" @ 300 DPI = 1050 pixels
// Vertical layout for single-column

let panel_a = Plot::new()
    .line(&time, &data_a)
    .title("a")  // Nature uses lowercase letters
    .end_series()
    .theme(Theme::publication());

let panel_b = Plot::new()
    .histogram(&data_b, None)
    .title("b")
    .end_series()
    .theme(Theme::publication());

let panel_c = Plot::new()
    .boxplot(&data_c, None)
    .title("c")
    .end_series()
    .theme(Theme::publication());

subplots(3, 1, 1050, 1575)?  // 3.5" × 5.25"
    .hspace(0.35)
    .subplot(0, 0, panel_a)?
    .subplot(1, 0, panel_b)?
    .subplot(2, 0, panel_c)?
    .save("nature_figure.png")?;
```

## SubplotFigure API

### Progressive Construction

```rust
use ruviz::prelude::*;

// Create figure
let mut figure = SubplotFigure::new(2, 2, 1200, 900)?
    .suptitle("Progressive Construction")
    .hspace(0.3)
    .wspace(0.3);

// Add plots progressively
let plot1 = Plot::new().line(&x1, &y1).title("Panel 1").end_series();
figure = figure.subplot(0, 0, plot1)?;

let plot2 = Plot::new().scatter(&x2, &y2).title("Panel 2").end_series();
figure = figure.subplot(0, 1, plot2)?;

// Continue adding...
let plot3 = Plot::new().bar(&cats, &vals).title("Panel 3").end_series();
figure = figure.subplot(1, 0, plot3)?;

let plot4 = Plot::new().histogram(&data, None).title("Panel 4").end_series();
figure = figure.subplot(1, 1, plot4)?;

// Save when complete
figure.save("progressive.png")?;
```

### Querying Subplot Information

```rust
use ruviz::prelude::*;

let figure = subplots(3, 2, 1200, 900)?
    .subplot(0, 0, plot1)?
    .subplot(0, 1, plot2)?
    .subplot(1, 0, plot3)?
    .subplot(1, 1, plot4)?
    .subplot(2, 0, plot5)?
    .subplot(2, 1, plot6)?;

let count = figure.subplot_count();
println!("Figure contains {} subplots", count);  // 6

figure.save("queried_figure.png")?;
```

## Common Patterns

### Dashboard Layout

```rust
use ruviz::prelude::*;

// Top: Overview (full width)
// Bottom: Details (three panels)

// Use 2×3 grid, merge top row
let overview = Plot::new()
    .line(&time, &overall_metric)
    .title("System Overview")
    .end_series();

let detail1 = Plot::new().histogram(&metric1, None).title("Metric 1").end_series();
let detail2 = Plot::new().histogram(&metric2, None).title("Metric 2").end_series();
let detail3 = Plot::new().histogram(&metric3, None).title("Metric 3").end_series();

// For now, use separate rows (future: subplot span support)
subplots(2, 3, 1800, 900)?
    .suptitle("System Dashboard")
    .subplot(0, 0, overview.clone())?  // Top-left
    .subplot(0, 1, overview.clone())?  // Top-middle (visually merged)
    .subplot(0, 2, overview)?           // Top-right
    .subplot(1, 0, detail1)?
    .subplot(1, 1, detail2)?
    .subplot(1, 2, detail3)?
    .save("dashboard.png")?;
```

### Time Series Comparison

```rust
use ruviz::prelude::*;

// Stack time series vertically for easy comparison
let sensors = ["Sensor A", "Sensor B", "Sensor C", "Sensor D"];
let plots: Vec<_> = sensors.iter().enumerate().map(|(i, &name)| {
    Plot::new()
        .line(&time, &sensor_data[i])
        .title(name)
        .xlabel("Time (s)")
        .ylabel("Reading")
        .end_series()
}).collect();

let mut figure = subplots(4, 1, 1200, 1600)?
    .suptitle("Multi-Sensor Time Series")
    .hspace(0.25);

for (i, plot) in plots.into_iter().enumerate() {
    figure = figure.subplot(i, 0, plot)?;
}

figure.save("sensor_comparison.png")?;
```

## Performance Considerations

### Subplot Rendering Time

```rust
use ruviz::prelude::*;
use std::time::Instant;

let start = Instant::now();

// Create 2×2 subplot with 1000 points each
let figure = subplots(2, 2, 1600, 1200)?
    .subplot(0, 0, plot1)?
    .subplot(0, 1, plot2)?
    .subplot(1, 0, plot3)?
    .subplot(1, 1, plot4)?;

figure.save("performance_test.png")?;

println!("Rendered 2×2 subplot in {:?}", start.elapsed());
// Typical: 147ms for 4 panels with 1K points each
```

### Large Grid Optimization

For grids larger than 3×3, consider:
- Using `parallel` feature for independent panel rendering
- Reducing DPI for draft versions
- Simplifying individual plot complexity

## Next Steps

- **[Backend Selection](07_backends.md)** - Rendering backend options
- **[Performance](08_performance.md)** - Large dataset optimization
- **[Export Formats](10_export.md)** - High-quality output
- **[Advanced Techniques](11_advanced.md)** - Custom layouts

---

**Ready to optimize performance?** → [Performance Guide](08_performance.md)
