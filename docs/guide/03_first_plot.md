# Your First Plot

Create your first visualization with ruviz in 5 minutes.

## Quick Start

### 1. Create New Project

```bash
cargo new my_plot
cd my_plot
cargo add ruviz
```

### 2. Write Your First Plot

Edit `src/main.rs`:

```rust
use ruviz::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Data
    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    // Plot
    Plot::new()
        .line(&x, &y)
        .title("My First Plot")
        .xlabel("X axis")
        .ylabel("Y axis")
        .save("my_first_plot.png")?;

    println!("✅ Plot saved to my_first_plot.png");
    Ok(())
}
```

### 3. Run

```bash
cargo run
```

**Output**: `my_first_plot.png` with a line plot of y = x²

## Understanding the Code

### Imports

```rust
use ruviz::prelude::*;
```

The `prelude` module includes all commonly used types and traits. This gives you access to:
- `Plot` - Main plotting struct
- `Color`, `MarkerStyle`, `LineStyle` - Styling types
- `Position` - Legend positioning
- Common traits for data conversion

### Error Handling

```rust
fn main() -> Result<(), Box<dyn std::error::Error>>
```

ruviz operations return `Result` types. Use `?` operator for clean error propagation:

```rust
.save("plot.png")?;  // Propagate errors automatically
```

Or handle errors explicitly:

```rust
match plot.save("plot.png") {
    Ok(_) => println!("Success!"),
    Err(e) => eprintln!("Error: {}", e),
}
```

### Builder Pattern

ruviz uses **method chaining** for fluent API:

```rust
Plot::new()           // Create plot
    .line(&x, &y)     // Add line series
    .title("Title")   // Set title
    .xlabel("X")      // Set x label
    .ylabel("Y")      // Set y label
    .save("file.png") // Save to file
```

Each method returns `Self`, enabling chaining.

## Common Plot Types

### Line Plot

```rust
use ruviz::prelude::*;

let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];

Plot::new()
    .line(&x, &y)
    .title("Line Plot")
    .save("line.png")?;
```

### Scatter Plot

```rust
use ruviz::prelude::*;

let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
let y = vec![2.0, 4.0, 3.0, 5.0, 4.5];

Plot::new()
    .scatter(&x, &y)
    .marker(MarkerStyle::Circle)
    .marker_size(8.0)
    .title("Scatter Plot")
    .save("scatter.png")?;
```

### Bar Chart

```rust
use ruviz::prelude::*;

let categories = ["A", "B", "C", "D"];
let values = vec![10.0, 25.0, 17.0, 30.0];

Plot::new()
    .bar(&categories, &values)
    .title("Bar Chart")
    .xlabel("Category")
    .ylabel("Value")
    .save("bar.png")?;
```

### Histogram

```rust
use ruviz::prelude::*;
use rand::distributions::{Distribution, Normal};

let normal = Normal::new(100.0, 15.0).unwrap();
let mut rng = rand::thread_rng();
let data: Vec<f64> = (0..1000).map(|_| normal.sample(&mut rng)).collect();

Plot::new()
    .histogram(&data, None)  // Auto bin count
    .title("Histogram")
    .xlabel("Value")
    .ylabel("Frequency")
    .save("histogram.png")?;
```

Add to `Cargo.toml`:
```toml
[dependencies]
ruviz = "0.1"
rand = "0.8"
```

## Customization Basics

### Colors

```rust
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y)
    .color(Color::from_rgb(255, 0, 0))  // Red line
    .save("red_line.png")?;
```

### Line Styles

```rust
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y)
    .line_style(LineStyle::Dashed)
    .line_width(2.0)
    .save("dashed_line.png")?;
```

### Markers

```rust
use ruviz::prelude::*;

Plot::new()
    .scatter(&x, &y)
    .marker(MarkerStyle::Circle)
    .marker_size(10.0)
    .color(Color::from_rgb(0, 0, 255))
    .save("blue_circles.png")?;
```

### Grid

```rust
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y)
    .grid(true)  // Enable grid
    .save("grid_plot.png")?;
```

## Multiple Series

### Basic Multi-Series

```rust
use ruviz::prelude::*;

let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
let y1 = vec![0.0, 1.0, 4.0, 9.0, 16.0];
let y2 = vec![0.0, 2.0, 4.0, 6.0, 8.0];

Plot::new()
    .line(&x, &y1)
        .label("Quadratic")
        .color(Color::from_rgb(255, 0, 0))
    .line(&x, &y2)
        .label("Linear")
        .color(Color::from_rgb(0, 0, 255))
    .legend(Position::TopLeft)
    .title("Multiple Series")
    .save("multi_series.png")?;
```

### Mixing Plot Types

```rust
use ruviz::prelude::*;

let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
let y_line = vec![1.0, 2.0, 3.0, 4.0, 5.0];
let y_scatter = vec![1.5, 2.3, 2.9, 4.2, 4.8];

Plot::new()
    .line(&x, &y_line)
        .label("Theory")
        .color(Color::from_rgb(0, 0, 255))
    .scatter(&x, &y_scatter)
        .label("Measured")
        .marker(MarkerStyle::Circle)
        .color(Color::from_rgb(255, 0, 0))
    .legend(Position::TopLeft)
    .title("Theory vs Measurement")
    .save("mixed_plot.png")?;
```

## Working with Data

### From Vectors

```rust
let x: Vec<f64> = vec![0.0, 1.0, 2.0, 3.0, 4.0];
let y: Vec<f64> = vec![0.0, 1.0, 4.0, 9.0, 16.0];

Plot::new()
    .line(&x, &y)
    .save("from_vec.png")?;
```

### From Arrays

```rust
let x = [0.0, 1.0, 2.0, 3.0, 4.0];
let y = [0.0, 1.0, 4.0, 9.0, 16.0];

Plot::new()
    .line(&x, &y)
    .save("from_array.png")?;
```

### From Ranges

```rust
let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();

Plot::new()
    .line(&x, &y)
    .title("Sine Wave")
    .save("sine.png")?;
```

### With ndarray (optional)

Add to `Cargo.toml`:
```toml
[dependencies]
ruviz = { version = "0.1", features = ["ndarray_support"] }
ndarray = "0.15"
```

```rust
use ruviz::prelude::*;
use ndarray::Array1;

let x = Array1::linspace(0.0, 10.0, 100);
let y = x.mapv(|v| v.sin());

Plot::new()
    .line(&x, &y)
    .title("ndarray Example")
    .save("ndarray_plot.png")?;
```

## Configuration Options

### Figure Size

```rust
Plot::new()
    .dimensions(1200, 800)  // Width x Height pixels
    .line(&x, &y)
    .save("custom_size.png")?;
```

### DPI (Resolution)

```rust
Plot::new()
    .dpi(300)  // High resolution for publication
    .line(&x, &y)
    .save("high_res.png")?;
```

### Axis Limits

```rust
Plot::new()
    .line(&x, &y)
    .xlim(0.0, 10.0)
    .ylim(-5.0, 5.0)
    .save("custom_limits.png")?;
```

### Themes

```rust
use ruviz::prelude::*;

Plot::new()
    .theme(Theme::dark())      // Dark background
    .line(&x, &y)
    .save("dark_theme.png")?;

Plot::new()
    .theme(Theme::publication())  // Scientific publication
    .line(&x, &y)
    .save("publication.png")?;

Plot::new()
    .theme(Theme::seaborn())   // seaborn-like styling
    .line(&x, &y)
    .save("seaborn.png")?;
```

## Complete Example

```rust
use ruviz::prelude::*;
use std::f64::consts::PI;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate data
    let x: Vec<f64> = (0..200).map(|i| i as f64 * 0.05).collect();
    let y_sin: Vec<f64> = x.iter().map(|v| v.sin()).collect();
    let y_cos: Vec<f64> = x.iter().map(|v| v.cos()).collect();

    // Create plot
    Plot::new()
        .dimensions(1000, 600)
        .dpi(150)
        .theme(Theme::light())
        // Sine wave
        .line(&x, &y_sin)
            .label("sin(x)")
            .color(Color::from_rgb(0, 0, 255))
            .line_width(2.0)
        // Cosine wave
        .line(&x, &y_cos)
            .label("cos(x)")
            .color(Color::from_rgb(255, 0, 0))
            .line_style(LineStyle::Dashed)
            .line_width(2.0)
        // Configuration
        .title("Trigonometric Functions")
        .xlabel("x (radians)")
        .ylabel("y")
        .xlim(0.0, 2.0 * PI)
        .ylim(-1.5, 1.5)
        .grid(true)
        .legend(Position::TopRight)
        .save("trig_functions.png")?;

    println!("✅ Plot saved to trig_functions.png");
    Ok(())
}
```

## Troubleshooting

### Plot file not created

**Check**: Error handling
```rust
// Don't ignore errors
Plot::new().line(&x, &y).save("plot.png")?;

// Or handle explicitly
match Plot::new().line(&x, &y).save("plot.png") {
    Ok(_) => println!("Success"),
    Err(e) => eprintln!("Error: {}", e),
}
```

### Empty or blank plot

**Check**: Data validity
```rust
// Ensure x and y have same length
assert_eq!(x.len(), y.len());

// Ensure data is not empty
assert!(!x.is_empty());

// Check for NaN or infinity
assert!(y.iter().all(|v| v.is_finite()));
```

### Performance issues

**Use**: Release mode for large datasets
```bash
cargo run --release  # Much faster than debug builds
```

### Legend not showing

**Add**: Labels to series
```rust
Plot::new()
    .line(&x, &y)
        .label("My Data")  // Required for legend
    .legend(Position::TopRight)
    .save("plot.png")?;
```

## Next Steps

🎉 **Congratulations!** You've created your first plot with ruviz.

**Continue learning**:
- **[Plot Types](04_plot_types.md)** - Explore all available plot types
- **[Styling & Themes](05_styling.md)** - Advanced customization
- **[Subplots](06_subplots.md)** - Multi-panel figures
- **[Examples](../../examples/)** - Browse working examples

## Quick Reference

### Essential Pattern
```rust
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y)          // or .scatter(), .bar(), .histogram()
    .title("Title")
    .xlabel("X")
    .ylabel("Y")
    .save("plot.png")?;
```

### Common Customizations
```rust
.dimensions(width, height)     // Figure size
.dpi(resolution)               // Image resolution
.color(Color::from_rgb(r,g,b)) // Series color
.line_width(width)             // Line thickness
.marker(MarkerStyle::Circle)   // Marker shape
.marker_size(size)             // Marker size
.line_style(LineStyle::Dashed) // Line pattern
.grid(true)                    // Show grid
.xlim(min, max)                // X axis range
.ylim(min, max)                // Y axis range
.legend(Position::TopRight)    // Show legend
.theme(Theme::dark())          // Apply theme
```

---

**Ready for more?** → [Plot Types Guide](04_plot_types.md)
