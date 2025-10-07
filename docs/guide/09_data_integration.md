# Data Integration

Comprehensive guide to integrating ruviz with ndarray, polars, CSV files, and other data sources.

## Overview

ruviz works seamlessly with Rust's data ecosystem:

| Data Source | Use Case | Feature Flag |
|-------------|----------|--------------|
| **Vec/Arrays** | Basic Rust data | Built-in |
| **ndarray** | Scientific computing | `ndarray_support` |
| **polars** | DataFrame analysis | `polars_support` |
| **CSV files** | File I/O | Standard library |
| **JSON** | Web data | `serde_json` |

## Basic Rust Data Types

### Vectors

```rust
use ruviz::prelude::*;

let x: Vec<f64> = vec![0.0, 1.0, 2.0, 3.0, 4.0];
let y: Vec<f64> = vec![0.0, 1.0, 4.0, 9.0, 16.0];

Plot::new()
    .line(&x, &y)
    .save("vec_plot.png")?;
```

### Arrays

```rust
use ruviz::prelude::*;

let x = [0.0, 1.0, 2.0, 3.0, 4.0];
let y = [0.0, 1.0, 4.0, 9.0, 16.0];

Plot::new()
    .line(&x, &y)
    .save("array_plot.png")?;
```

### Iterators

```rust
use ruviz::prelude::*;

let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();

Plot::new()
    .line(&x, &y)
    .save("iterator_plot.png")?;
```

## ndarray Integration

**ndarray** provides n-dimensional arrays similar to NumPy.

### Setup

```toml
[dependencies]
ruviz = { version = "0.1", features = ["ndarray_support"] }
ndarray = "0.15"
```

### Basic Usage

```rust
use ruviz::prelude::*;
use ndarray::Array1;

let x = Array1::linspace(0.0, 10.0, 100);
let y = x.mapv(|v| v.sin());

Plot::new()
    .line(&x, &y)
    .title("ndarray Integration")
    .xlabel("X")
    .ylabel("sin(X)")
    .save("ndarray_plot.png")?;
```

### Scientific Computing Example

```rust
use ruviz::prelude::*;
use ndarray::{Array1, Array2};
use std::f64::consts::PI;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate mesh grid
    let n = 100;
    let x = Array1::linspace(0.0, 2.0 * PI, n);

    // Compute multiple functions
    let y_sin = x.mapv(|v| v.sin());
    let y_cos = x.mapv(|v| v.cos());
    let y_tan = x.mapv(|v| (v / 2.0).tan());

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
        .ylim(-2.0, 2.0)
        .legend(Position::TopRight)
        .grid(true)
        .save("trig_functions.png")?;

    Ok(())
}
```

### Matrix Operations

```rust
use ruviz::prelude::*;
use ndarray::{Array1, Array2};

// Create 2D data
let matrix = Array2::from_shape_fn((10, 10), |(i, j)| {
    (i as f64 * j as f64).sin()
});

// Extract column for plotting
let col_5 = matrix.column(5).to_owned();
let indices = Array1::linspace(0.0, 9.0, 10);

Plot::new()
    .scatter(&indices, &col_5)
    .marker(MarkerStyle::Circle)
    .marker_size(8.0)
    .title("Matrix Column 5")
    .xlabel("Row Index")
    .ylabel("Value")
    .save("matrix_column.png")?;
```

### Statistical Analysis

```rust
use ruviz::prelude::*;
use ndarray::{Array1, Array};

// Generate normal distribution (Box-Muller transform)
let n = 1000;
let mut rng = rand::thread_rng();
let data: Array1<f64> = Array::from_shape_fn(n, |_| {
    let u1: f64 = rand::random();
    let u2: f64 = rand::random();
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
});

Plot::new()
    .histogram(&data.to_vec(), None)  // Convert to Vec
    .title("Normal Distribution")
    .xlabel("Value")
    .ylabel("Frequency")
    .theme(Theme::seaborn())
    .save("normal_dist.png")?;
```

## polars Integration

**polars** provides high-performance DataFrames similar to pandas.

### Setup

```toml
[dependencies]
ruviz = { version = "0.1", features = ["polars_support"] }
polars = "0.35"
```

### Basic DataFrame Plotting

```rust
use ruviz::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create DataFrame
    let df = df! {
        "x" => &[1.0, 2.0, 3.0, 4.0, 5.0],
        "y" => &[2.0, 4.0, 3.0, 5.0, 4.5],
    }?;

    // Extract columns
    let x = df.column("x")?.f64()?;
    let y = df.column("y")?.f64()?;

    Plot::new()
        .scatter(x, y)
        .marker(MarkerStyle::Circle)
        .marker_size(10.0)
        .title("DataFrame Scatter Plot")
        .xlabel("X")
        .ylabel("Y")
        .save("polars_scatter.png")?;

    Ok(())
}
```

### DataFrame Aggregation

```rust
use ruviz::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load data
    let df = df! {
        "category" => &["A", "B", "C", "A", "B", "C", "A", "B", "C"],
        "value" => &[10.0, 15.0, 12.0, 14.0, 18.0, 16.0, 12.0, 20.0, 14.0],
    }?;

    // Group by category and calculate mean
    let grouped = df
        .lazy()
        .groupby([col("category")])
        .agg([col("value").mean()])
        .collect()?;

    // Extract for plotting
    let categories: Vec<&str> = grouped
        .column("category")?
        .utf8()?
        .into_iter()
        .map(|opt| opt.unwrap_or(""))
        .collect();

    let values: Vec<f64> = grouped
        .column("value")?
        .f64()?
        .into_iter()
        .map(|opt| opt.unwrap_or(0.0))
        .collect();

    Plot::new()
        .bar(&categories, &values)
        .title("Average Values by Category")
        .xlabel("Category")
        .ylabel("Average Value")
        .save("polars_grouped.png")?;

    Ok(())
}
```

### Time Series Analysis

```rust
use ruviz::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create time series data
    let dates: Vec<i64> = (0..100).collect();
    let values: Vec<f64> = (0..100).map(|i| {
        (i as f64 * 0.1).sin() * 10.0 + (rand::random::<f64>() - 0.5) * 2.0
    }).collect();

    let df = df! {
        "date" => dates,
        "value" => values,
    }?;

    // Rolling average
    let rolling = df
        .clone()
        .lazy()
        .select([col("value").rolling_mean(RollingOptionsFixedWindow {
            window_size: 10,
            ..Default::default()
        })])
        .collect()?;

    let x_values: Vec<f64> = (0..100).map(|i| i as f64).collect();
    let original: Vec<f64> = df.column("value")?.f64()?.into_iter()
        .filter_map(|v| v)
        .collect();
    let smoothed: Vec<f64> = rolling.column("value")?.f64()?.into_iter()
        .filter_map(|v| v)
        .collect();

    Plot::new()
        .line(&x_values, &original)
            .label("Original")
            .color(Color::from_rgba(0, 0, 255, 100))
        .line(&x_values, &smoothed)
            .label("Rolling Average")
            .color(Color::from_rgb(255, 0, 0))
            .line_width(2.0)
        .title("Time Series with Rolling Average")
        .xlabel("Time")
        .ylabel("Value")
        .legend(Position::TopRight)
        .save("timeseries_polars.png")?;

    Ok(())
}
```

## CSV File Integration

### Reading CSV with std

```rust
use ruviz::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read CSV file
    let file = File::open("data.csv")?;
    let reader = BufReader::new(file);

    let mut x_values = Vec::new();
    let mut y_values = Vec::new();

    for (i, line) in reader.lines().enumerate() {
        if i == 0 { continue; } // Skip header
        let line = line?;
        let parts: Vec<&str> = line.split(',').collect();

        x_values.push(parts[0].parse::<f64>()?);
        y_values.push(parts[1].parse::<f64>()?);
    }

    Plot::new()
        .scatter(&x_values, &y_values)
        .marker(MarkerStyle::Circle)
        .title("CSV Data")
        .xlabel("X Column")
        .ylabel("Y Column")
        .save("csv_plot.png")?;

    Ok(())
}
```

### Reading CSV with polars

```rust
use ruviz::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read CSV with polars
    let df = CsvReader::from_path("data.csv")?
        .finish()?;

    // Extract columns
    let x = df.column("x")?.f64()?;
    let y = df.column("y")?.f64()?;

    Plot::new()
        .line(x, y)
        .title("CSV Data (polars)")
        .xlabel("X")
        .ylabel("Y")
        .save("csv_polars_plot.png")?;

    Ok(())
}
```

### CSV with Filtering

```rust
use ruviz::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read and filter CSV
    let df = CsvReader::from_path("measurements.csv")?
        .finish()?;

    // Filter data: value > 10
    let filtered = df.filter(
        &df.column("value")?.gt(10.0)?
    )?;

    let x = filtered.column("time")?.f64()?;
    let y = filtered.column("value")?.f64()?;

    Plot::new()
        .scatter(x, y)
        .marker(MarkerStyle::Circle)
        .marker_size(8.0)
        .title("Filtered Measurements (value > 10)")
        .xlabel("Time")
        .ylabel("Value")
        .save("filtered_data.png")?;

    Ok(())
}
```

## JSON Integration

### Basic JSON Parsing

```toml
[dependencies]
ruviz = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

```rust
use ruviz::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct DataPoint {
    x: f64,
    y: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read JSON file
    let json_data = std::fs::read_to_string("data.json")?;
    let data: Vec<DataPoint> = serde_json::from_str(&json_data)?;

    let x: Vec<f64> = data.iter().map(|p| p.x).collect();
    let y: Vec<f64> = data.iter().map(|p| p.y).collect();

    Plot::new()
        .line(&x, &y)
        .title("JSON Data")
        .xlabel("X")
        .ylabel("Y")
        .save("json_plot.png")?;

    Ok(())
}
```

## Complete Data Analysis Example

### End-to-End Workflow

```rust
use ruviz::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load data
    println!("Loading data from CSV...");
    let df = CsvReader::from_path("experiment_results.csv")?
        .finish()?;

    // 2. Data cleaning
    println!("Cleaning data...");
    let clean_df = df
        .lazy()
        .filter(col("measurement").is_not_null())
        .collect()?;

    // 3. Statistical analysis
    println!("Computing statistics...");
    let stats = clean_df
        .lazy()
        .groupby([col("condition")])
        .agg([
            col("measurement").mean().alias("mean"),
            col("measurement").std(1).alias("std"),
        ])
        .collect()?;

    // 4. Extract for visualization
    let conditions: Vec<&str> = stats
        .column("condition")?
        .utf8()?
        .into_iter()
        .map(|opt| opt.unwrap_or(""))
        .collect();

    let means: Vec<f64> = stats
        .column("mean")?
        .f64()?
        .into_iter()
        .filter_map(|v| v)
        .collect();

    // 5. Create visualization
    println!("Creating plot...");
    Plot::new()
        .bar(&conditions, &means)
        .color(Color::from_rgb(70, 130, 180))
        .title("Experimental Results by Condition")
        .xlabel("Condition")
        .ylabel("Mean Measurement")
        .theme(Theme::publication())
        .dpi(300)
        .save("experiment_analysis.png")?;

    println!("✅ Analysis complete!");
    Ok(())
}
```

### Multi-Panel Analysis

```rust
use ruviz::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load experiment data
    let df = CsvReader::from_path("experiment.csv")?.finish()?;

    // Panel A: Raw time series
    let time = df.column("time")?.f64()?;
    let signal = df.column("signal")?.f64()?;

    let panel_a = Plot::new()
        .line(time, signal)
        .title("A) Raw Signal")
        .xlabel("Time (s)")
        .ylabel("Amplitude")
        .end_series()
        .theme(Theme::seaborn());

    // Panel B: Distribution
    let signal_vec: Vec<f64> = df.column("signal")?
        .f64()?
        .into_iter()
        .filter_map(|v| v)
        .collect();

    let panel_b = Plot::new()
        .histogram(&signal_vec, None)
        .title("B) Distribution")
        .xlabel("Amplitude")
        .ylabel("Frequency")
        .end_series()
        .theme(Theme::seaborn());

    // Panel C: Group comparison
    let group_a: Vec<f64> = df
        .filter(&df.column("group")?.equal("A")?)?
        .column("value")?
        .f64()?
        .into_iter()
        .filter_map(|v| v)
        .collect();

    let panel_c = Plot::new()
        .boxplot(&group_a, None)
        .title("C) Group Analysis")
        .xlabel("Group")
        .ylabel("Value")
        .end_series()
        .theme(Theme::seaborn());

    // Panel D: Correlation
    let x_var = df.column("variable_x")?.f64()?;
    let y_var = df.column("variable_y")?.f64()?;

    let panel_d = Plot::new()
        .scatter(x_var, y_var)
        .marker(MarkerStyle::Circle)
        .marker_size(4.0)
        .title("D) Correlation")
        .xlabel("Variable X")
        .ylabel("Variable Y")
        .end_series()
        .theme(Theme::seaborn());

    // Compose figure
    subplots(2, 2, 1600, 1200)?
        .suptitle("Comprehensive Data Analysis")
        .hspace(0.3)
        .wspace(0.3)
        .subplot(0, 0, panel_a)?
        .subplot(0, 1, panel_b)?
        .subplot(1, 0, panel_c)?
        .subplot(1, 1, panel_d)?
        .save("comprehensive_analysis.png")?;

    Ok(())
}
```

## Data Conversion Reference

### Python → Rust

| Python | Rust | Note |
|--------|------|------|
| `list` | `Vec<f64>` | Standard Rust vector |
| `np.array` | `Array1<f64>` | Requires `ndarray` |
| `pd.DataFrame` | `DataFrame` | Requires `polars` |
| `pd.Series` | `Series` | Requires `polars` |
| `dict` | `HashMap` | Standard library |

### Type Conversions

```rust
// Vec to ndarray
let vec_data = vec![1.0, 2.0, 3.0];
let array_data = Array1::from(vec_data);

// ndarray to Vec
let array = Array1::from(vec![1.0, 2.0, 3.0]);
let vec_data = array.to_vec();

// Polars Series to Vec
let series = df.column("col")?.f64()?;
let vec_data: Vec<f64> = series.into_iter().filter_map(|v| v).collect();
```

## Best Practices

### 1. Choose the Right Tool

- **Vec**: Simple data, small datasets
- **ndarray**: Scientific computing, linear algebra
- **polars**: Large datasets, complex operations

### 2. Handle Missing Data

```rust
use polars::prelude::*;

// Filter out nulls
let clean_data: Vec<f64> = df.column("value")?
    .f64()?
    .into_iter()
    .filter_map(|v| v)  // Removes None values
    .collect();

// Or fill with default
let filled_data: Vec<f64> = df.column("value")?
    .f64()?
    .into_iter()
    .map(|v| v.unwrap_or(0.0))  // Use 0.0 for nulls
    .collect();
```

### 3. Optimize Memory

```rust
// Don't clone unnecessarily
let df = CsvReader::from_path("large_file.csv")?.finish()?;
let x = df.column("x")?.f64()?;  // Good: uses reference

// Avoid
let x_copy = x.clone();  // Bad: unnecessary copy
```

## Next Steps

- **[Export Formats](10_export.md)** - High-quality output options
- **[Advanced Techniques](11_advanced.md)** - Complex visualizations
- **[Performance Guide](08_performance.md)** - Large dataset optimization

---

**Ready for export options?** → [Export Guide](10_export.md)
