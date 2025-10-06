# Quick Start Guide

Get started with ruviz in less than 5 minutes!

## Installation

1. **Create a new Rust project**:
```bash
cargo new my_plot
cd my_plot
```

2. **Add ruviz to your `Cargo.toml`**:
```toml
[dependencies]
ruviz = "0.1"
```

3. **Write your first plot** in `src/main.rs`:
```rust
use ruviz::prelude::*;

fn main() -> Result<()> {
    // Create some data
    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    // Create and save a plot
    Plot::new()
        .line(&x, &y)
        .title("My First Plot")
        .xlabel("x")
        .ylabel("y = x²")
        .save("plot.png")?;

    println!("Plot saved to plot.png!");
    Ok(())
}
```

4. **Run it**:
```bash
cargo run --release
```

You should now have a `plot.png` file in your project directory!

## Your First Real Plot

Let's create a more interesting plot with real data:

```rust
use ruviz::prelude::*;

fn main() -> Result<()> {
    // Generate sine wave data
    let x: Vec<f64> = (0..100)
        .map(|i| i as f64 * 0.1)
        .collect();

    let y: Vec<f64> = x.iter()
        .map(|&x| x.sin())
        .collect();

    // Create a styled plot
    Plot::new()
        .line(&x, &y)
        .title("Sine Wave")
        .xlabel("x (radians)")
        .ylabel("sin(x)")
        .theme(Theme::publication())  // Professional theme
        .dpi(300)  // High resolution for print
        .save("sine_wave.png")?;

    println!("✓ Created sine_wave.png");
    Ok(())
}
```

## Common Plot Types

### Line Plot (Continuous Data)
```rust
use ruviz::prelude::*;

let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];

Plot::new()
    .line(&x, &y)
    .title("Line Plot")
    .save("line.png")?;
```

### Scatter Plot (Discrete Points)
```rust
use ruviz::prelude::*;

let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
let y = vec![2.3, 3.1, 2.8, 4.2, 3.9];

Plot::new()
    .scatter(&x, &y)
    .marker_style(MarkerStyle::Circle)
    .marker_size(10.0)
    .title("Scatter Plot")
    .save("scatter.png")?;
```

### Bar Chart (Categories)
```rust
use ruviz::prelude::*;

let categories = vec!["A", "B", "C", "D"];
let values = vec![25.0, 40.0, 30.0, 55.0];

Plot::new()
    .bar(&categories, &values)
    .title("Bar Chart")
    .ylabel("Value")
    .save("bar.png")?;
```

### Histogram (Distribution)
```rust
use ruviz::prelude::*;
use rand::Rng;

// Generate random data
let mut rng = rand::thread_rng();
let data: Vec<f64> = (0..1000)
    .map(|_| rng.gen::<f64>() * 10.0)
    .collect();

Plot::new()
    .histogram(&data)
    .title("Data Distribution")
    .xlabel("Value")
    .ylabel("Frequency")
    .save("histogram.png")?;
```

## Multiple Series

Plot multiple datasets on the same axes:

```rust
use ruviz::prelude::*;

let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];

Plot::new()
    // Linear
    .line(&x, &x.iter().map(|&v| v).collect::<Vec<_>>())
    .label("Linear")
    .color(Color::from_rgb(0, 100, 200))

    // Quadratic
    .line(&x, &x.iter().map(|&v| v * v).collect::<Vec<_>>())
    .label("Quadratic")
    .color(Color::from_rgb(200, 0, 100))

    // Cubic
    .line(&x, &x.iter().map(|&v| v.powi(3)).collect::<Vec<_>>())
    .label("Cubic")
    .color(Color::from_rgb(0, 200, 100))

    .title("Polynomial Functions")
    .xlabel("x")
    .ylabel("y")
    .legend(Position::TopLeft)
    .save("polynomials.png")?;
```

## Styling Your Plots

### Themes
```rust
use ruviz::prelude::*;

// Professional publication theme
Plot::new()
    .theme(Theme::publication())
    .line(&x, &y)
    .save("publication.png")?;

// Dark theme
Plot::new()
    .theme(Theme::dark())
    .line(&x, &y)
    .save("dark.png")?;

// Seaborn-style
Plot::new()
    .theme(Theme::seaborn())
    .line(&x, &y)
    .save("seaborn.png")?;
```

### Custom Colors
```rust
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y)
    .color(Color::from_hex("#FF5733"))  // Hex color
    .line_width(2.5)
    .line_style(LineStyle::Dashed)
    .save("custom.png")?;
```

### High-Resolution Export
```rust
use ruviz::prelude::*;

// For print/publication (300 DPI)
Plot::new()
    .line(&x, &y)
    .dpi(300)
    .dimensions(1200, 900)  // Width x Height
    .save("high_res.png")?;

// For web (96 DPI, default)
Plot::new()
    .line(&x, &y)
    .dpi(96)
    .save("web.png")?;
```

## Working with DataFrames

### With ndarray
```rust
use ruviz::prelude::*;
use ndarray::Array1;

let x = Array1::linspace(0.0, 10.0, 100);
let y = x.mapv(|v| v.sin());

Plot::new()
    .line(&x, &y)
    .save("ndarray_plot.png")?;
```

### With polars (requires `polars_support` feature)
```toml
[dependencies]
ruviz = { version = "0.1", features = ["polars_support"] }
polars = "0.35"
```

```rust
use ruviz::prelude::*;
use polars::prelude::*;

let df = df! {
    "x" => [1, 2, 3, 4, 5],
    "y" => [2, 4, 6, 8, 10],
}?;

let x = df.column("x")?.f64()?;
let y = df.column("y")?.f64()?;

Plot::new()
    .line(x, y)
    .save("polars_plot.png")?;
```

## Performance Tips

### For Large Datasets (>10K points)
Enable parallel rendering:
```toml
[dependencies]
ruviz = { version = "0.1", features = ["parallel"] }
```

### For Very Large Datasets (>100K points)
Enable SIMD optimization:
```toml
[dependencies]
ruviz = { version = "0.1", features = ["parallel", "simd"] }
```

### Automatic Optimization (Coming in v0.2)
```rust
Plot::new()
    .auto_optimize()  // Automatically selects best backend
    .line(&huge_x, &huge_y)
    .save("optimized.png")?;
```

## Error Handling

ruviz uses `Result` types for proper error handling:

```rust
use ruviz::prelude::*;

fn create_plot() -> Result<()> {
    let x = vec![1.0, 2.0, 3.0];
    let y = vec![1.0, 4.0];  // Mismatched length!

    Plot::new()
        .line(&x, &y)  // This will fail
        .save("plot.png")?;

    Ok(())
}

fn main() {
    match create_plot() {
        Ok(_) => println!("Success!"),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

## Next Steps

Now that you've created your first plots, explore:

1. **[User Guide](guide/README.md)** - Comprehensive tutorials
2. **[API Documentation](https://docs.rs/ruviz)** - Complete reference
3. **[Gallery](gallery/README.md)** - Visual examples
4. **[Performance Guide](performance/PERFORMANCE.md)** - Optimization techniques

## Common Issues

### "Cannot find `ruviz` in the crate root"
Make sure you've added ruviz to `Cargo.toml` and run `cargo build`.

### "Plot is blurry"
Increase DPI: `.dpi(300)` for print quality.

### "Rendering is slow"
Enable parallel rendering: `features = ["parallel"]` in Cargo.toml.

### "Missing font errors"
ruviz automatically falls back to system fonts. If issues persist, check that your system has basic fonts installed.

## Getting Help

- **Issues**: [GitHub Issues](https://github.com/ruviz/ruviz/issues)
- **Discussions**: [GitHub Discussions](https://github.com/ruviz/ruviz/discussions)
- **Documentation**: [docs.rs/ruviz](https://docs.rs/ruviz)

Happy plotting! 📊
