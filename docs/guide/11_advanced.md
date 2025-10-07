# Advanced Techniques

Advanced visualization techniques, custom layouts, and specialized use cases.

## Overview

This guide covers advanced topics for experienced users:
- Custom color schemes and palettes
- Complex multi-panel layouts
- Mathematical function visualization
- Real-time and interactive patterns
- Production deployment strategies

## Custom Color Schemes

### Creating Color Palettes

```rust
use ruviz::prelude::*;

// Define custom palette
struct CustomPalette;

impl CustomPalette {
    fn oceanblue() -> Color { Color::from_rgb(0, 119, 182) }
    fn deepcyan() -> Color { Color::from_rgb(0, 180, 216) }
    fn skyblue() -> Color { Color::from_rgb(144, 224, 239) }
    fn coral() -> Color { Color::from_rgb(240, 128, 128) }
    fn sunset() -> Color { Color::from_rgb(255, 99, 71) }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let datasets = vec![
        vec![1.0, 2.0, 3.0, 4.0, 5.0],
        vec![1.5, 2.5, 3.5, 4.5, 5.5],
        vec![2.0, 3.0, 4.0, 5.0, 6.0],
    ];

    let colors = vec![
        CustomPalette::oceanblue(),
        CustomPalette::deepcyan(),
        CustomPalette::skyblue(),
    ];

    let mut plot = Plot::new();
    for (i, data) in datasets.iter().enumerate() {
        plot = plot.line(&x, data)
            .color(colors[i])
            .label(&format!("Series {}", i + 1));
    }

    plot.legend(Position::TopLeft)
        .save("custom_palette.png")?;

    Ok(())
}
```

### Color Interpolation

```rust
use ruviz::prelude::*;

fn interpolate_color(color1: Color, color2: Color, t: f64) -> Color {
    // Linear interpolation between two colors
    let (r1, g1, b1) = color1.to_rgb();
    let (r2, g2, b2) = color2.to_rgb();

    let r = (r1 as f64 * (1.0 - t) + r2 as f64 * t) as u8;
    let g = (g1 as f64 * (1.0 - t) + g2 as f64 * t) as u8;
    let b = (b1 as f64 * (1.0 - t) + b2 as f64 * t) as u8;

    Color::from_rgb(r, g, b)
}

fn create_gradient_palette(n: usize) -> Vec<Color> {
    let start = Color::from_rgb(0, 0, 255);    // Blue
    let end = Color::from_rgb(255, 0, 0);      // Red

    (0..n).map(|i| {
        let t = i as f64 / (n - 1) as f64;
        interpolate_color(start, end, t)
    }).collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let colors = create_gradient_palette(5);
    // Use colors for multi-series plot
    Ok(())
}
```

## Complex Layouts

### Asymmetric Grids

```rust
use ruviz::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create overview panel and detail panels
    let overview = Plot::new()
        .line(&time, &overall_signal)
        .title("System Overview")
        .end_series();

    let detail1 = Plot::new()
        .scatter(&x1, &y1)
        .title("Detail 1")
        .end_series();

    let detail2 = Plot::new()
        .scatter(&x2, &y2)
        .title("Detail 2")
        .end_series();

    let detail3 = Plot::new()
        .scatter(&x3, &y3)
        .title("Detail 3")
        .end_series();

    // Use 2×3 grid to simulate 1×1 + 1×3 layout
    subplots(2, 3, 1800, 1000)?
        .suptitle("Hierarchical Analysis Layout")
        // Top row: span overview across all columns
        .subplot(0, 0, overview.clone())?
        .subplot(0, 1, overview.clone())?
        .subplot(0, 2, overview)?
        // Bottom row: three detail panels
        .subplot(1, 0, detail1)?
        .subplot(1, 1, detail2)?
        .subplot(1, 2, detail3)?
        .save("asymmetric_layout.png")?;

    Ok(())
}
```

### Mixed Aspect Ratios

```rust
use ruviz::prelude::*;

// Wide timeseries + tall distribution analysis
let timeseries = Plot::new()
    .line(&time, &signal)
    .title("Time Series (Wide)")
    .end_series();

let distribution = Plot::new()
    .histogram(&data, None)
    .title("Distribution (Tall)")
    .end_series();

// Use custom figure dimensions to accommodate different aspects
subplots(1, 2, 2400, 1000)?  // Extra wide for different panel shapes
    .subplot(0, 0, timeseries)?   // Left panel (wider)
    .subplot(0, 1, distribution)? // Right panel (narrower)
    .wspace(0.4)  // Extra spacing between panels
    .save("mixed_aspects.png")?;
```

## Mathematical Visualization

### Parametric Curves

```rust
use ruviz::prelude::*;
use std::f64::consts::PI;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parametric spiral
    let t: Vec<f64> = (0..1000).map(|i| i as f64 * 0.01).collect();
    let x: Vec<f64> = t.iter().map(|&t| t * (t * 2.0).cos()).collect();
    let y: Vec<f64> = t.iter().map(|&t| t * (t * 2.0).sin()).collect();

    Plot::new()
        .line(&x, &y)
        .title("Parametric Spiral: x = t·cos(2t), y = t·sin(2t)")
        .xlabel("X")
        .ylabel("Y")
        .grid(true)
        .save("parametric_spiral.png")?;

    Ok(())
}
```

### Phase Portraits

```rust
use ruviz::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simple harmonic oscillator phase portrait
    let n = 1000;
    let dt = 0.01;

    let mut x = vec![0.0; n];
    let mut v = vec![1.0; n];  // Initial velocity

    // Simulate dynamics
    for i in 1..n {
        let acceleration = -x[i-1];  // Simple harmonic oscillator
        v[i] = v[i-1] + acceleration * dt;
        x[i] = x[i-1] + v[i] * dt;
    }

    Plot::new()
        .line(&x, &v)
        .title("Phase Portrait: Simple Harmonic Oscillator")
        .xlabel("Position")
        .ylabel("Velocity")
        .grid(true)
        .save("phase_portrait.png")?;

    Ok(())
}
```

### Vector Fields (Approximation)

```rust
use ruviz::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Approximate vector field with small line segments
    let grid_size = 20;
    let step = 0.5;

    for i in 0..grid_size {
        for j in 0..grid_size {
            let x0 = i as f64 * step;
            let y0 = j as f64 * step;

            // Vector field: dx/dt = y, dy/dt = -x
            let dx = y0 * 0.1;
            let dy = -x0 * 0.1;

            let x_line = vec![x0, x0 + dx];
            let y_line = vec![y0, y0 + dy];

            // Plot each vector as a small line
            // (Accumulate into single plot for efficiency)
        }
    }

    Ok(())
}
```

## Production Patterns

### Configuration-Driven Plots

```rust
use ruviz::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct PlotConfig {
    title: String,
    xlabel: String,
    ylabel: String,
    theme: String,
    dpi: u32,
    width: u32,
    height: u32,
}

fn create_plot_from_config(
    x: &[f64],
    y: &[f64],
    config: &PlotConfig
) -> Result<(), Box<dyn std::error::Error>> {
    let theme = match config.theme.as_str() {
        "dark" => Theme::dark(),
        "publication" => Theme::publication(),
        "seaborn" => Theme::seaborn(),
        _ => Theme::light(),
    };

    Plot::new()
        .dimensions(config.width, config.height)
        .dpi(config.dpi)
        .theme(theme)
        .line(x, y)
        .title(&config.title)
        .xlabel(&config.xlabel)
        .ylabel(&config.ylabel)
        .save("output.png")?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from JSON
    let config_json = std::fs::read_to_string("plot_config.json")?;
    let config: PlotConfig = serde_json::from_str(&config_json)?;

    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    create_plot_from_config(&x, &y, &config)?;

    Ok(())
}
```

### Plot Templates

```rust
use ruviz::prelude::*;

struct PlotTemplate;

impl PlotTemplate {
    fn publication_timeseries(
        time: &[f64],
        signal: &[f64],
        title: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        Plot::new()
            .dimensions(2175, 1500)
            .dpi(300)
            .theme(Theme::publication())
            .line(time, signal)
            .color(Color::from_rgb(76, 114, 176))
            .line_width(2.0)
            .title(title)
            .xlabel("Time (s)")
            .ylabel("Signal Amplitude")
            .grid(true)
            .save(&format!("{}.png", title.replace(" ", "_")))?;
        Ok(())
    }

    fn dashboard_metric(
        data: &[f64],
        label: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        Plot::new()
            .dimensions(600, 400)
            .dpi(96)
            .theme(Theme::light())
            .histogram(data, None)
            .color(Color::from_rgb(70, 130, 180))
            .title(label)
            .xlabel("Value")
            .ylabel("Count")
            .save(&format!("{}_metric.png", label.replace(" ", "_")))?;
        Ok(())
    }
}
```

### Batch Processing

```rust
use ruviz::prelude::*;
use std::path::Path;

fn process_dataset_directory(
    input_dir: &Path,
    output_dir: &Path
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output_dir)?;

    for entry in std::fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("csv") {
            // Load CSV data
            let data = load_csv_data(&path)?;

            // Generate plot
            let filename = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("plot");

            let output_path = output_dir.join(format!("{}.png", filename));

            Plot::new()
                .line(&data.x, &data.y)
                .title(&format!("Analysis: {}", filename))
                .save(&output_path)?;

            println!("✅ Processed: {}", filename);
        }
    }

    Ok(())
}
```

## Error Handling Patterns

### Robust Plot Generation

```rust
use ruviz::prelude::*;
use std::error::Error;

fn generate_plot_safe(
    x: &[f64],
    y: &[f64],
    output: &str
) -> Result<(), Box<dyn Error>> {
    // Validation
    if x.len() != y.len() {
        return Err("Mismatched data lengths".into());
    }

    if x.is_empty() {
        return Err("Empty dataset".into());
    }

    // Check for NaN/Inf
    if x.iter().any(|v| !v.is_finite()) || y.iter().any(|v| !v.is_finite()) {
        return Err("Data contains NaN or Infinity".into());
    }

    // Create output directory
    if let Some(parent) = std::path::Path::new(output).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Generate plot
    Plot::new()
        .line(x, y)
        .save(output)?;

    Ok(())
}
```

### Graceful Degradation

```rust
use ruviz::prelude::*;

fn plot_with_fallback(
    x: &[f64],
    y: &[f64],
    output: &str
) -> Result<(), Box<dyn std::error::Error>> {
    // Try high-quality first
    match Plot::new()
        .dimensions(2175, 1500)
        .dpi(300)
        .line(x, y)
        .save(output)
    {
        Ok(_) => return Ok(()),
        Err(e) => {
            eprintln!("⚠️ High-quality render failed: {}", e);
            eprintln!("   Falling back to standard quality...");
        }
    }

    // Fallback to standard quality
    Plot::new()
        .dimensions(800, 600)
        .dpi(96)
        .line(x, y)
        .save(output)?;

    Ok(())
}
```

## Performance Optimization Patterns

### Lazy Data Generation

```rust
use ruviz::prelude::*;

fn generate_and_plot_lazy(n: usize) -> Result<(), Box<dyn std::error::Error>> {
    // Don't materialize all data at once
    let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.01).collect();

    // Generate y values on-demand during plotting
    let y: Vec<f64> = x.iter().map(|&x| {
        // Expensive calculation
        (x * 10.0).sin() * (x * 7.0).cos() * (-x * 0.1).exp()
    }).collect();

    Plot::new()
        .line(&x, &y)
        .save("lazy_plot.png")?;

    // x and y dropped immediately after plotting
    Ok(())
}
```

### Memory-Efficient Iterations

```rust
use ruviz::prelude::*;

fn process_large_dataset_efficiently() -> Result<(), Box<dyn std::error::Error>> {
    const CHUNK_SIZE: usize = 10_000;

    // Process in chunks to avoid loading all data into memory
    for chunk_idx in 0..100 {
        let start = chunk_idx * CHUNK_SIZE;
        let end = start + CHUNK_SIZE;

        let x: Vec<f64> = (start..end).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();

        Plot::new()
            .line(&x, &y)
            .save(&format!("chunk_{:03}.png", chunk_idx))?;

        // Memory freed after each chunk
    }

    Ok(())
}
```

## Testing Patterns

### Visual Regression Testing

```rust
use ruviz::prelude::*;

#[test]
fn test_plot_output_consistency() {
    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    // Generate reference plot
    Plot::new()
        .line(&x, &y)
        .save("test_output/reference.png")
        .expect("Failed to save reference");

    // Generate test plot
    Plot::new()
        .line(&x, &y)
        .save("test_output/test.png")
        .expect("Failed to save test");

    // Compare file sizes (basic check)
    let ref_size = std::fs::metadata("test_output/reference.png")
        .unwrap()
        .len();

    let test_size = std::fs::metadata("test_output/test.png")
        .unwrap()
        .len();

    // File sizes should be identical for deterministic rendering
    assert_eq!(ref_size, test_size, "Plot output changed");
}
```

## Best Practices Summary

### ✅ DO

1. **Use configuration files** for production deployments
2. **Validate data** before plotting (NaN, Inf, length checks)
3. **Handle errors gracefully** with fallback strategies
4. **Create plot templates** for consistent styling
5. **Test visual output** with regression tests
6. **Document custom palettes** for team collaboration
7. **Use lazy evaluation** for large datasets
8. **Implement batch processing** for multiple datasets

### ❌ DON'T

1. **Don't hardcode paths** - use configurable locations
2. **Don't ignore data validation** - check before plotting
3. **Don't plot untrusted data** without sanitization
4. **Don't create memory leaks** - drop data after use
5. **Don't skip error handling** - always check Results
6. **Don't mix concerns** - separate data processing from visualization

## Advanced Examples Repository

For more advanced examples, see:
- `/examples/scientific_showcase.rs` - Publication-quality multi-panel figures
- `/examples/parallel_demo.rs` - High-performance parallel rendering
- `/examples/memory_optimization_demo.rs` - Memory-efficient large datasets

## Next Steps

- **[Migration Guides](../migration/)** - Transition from Python libraries
- **[Examples Gallery](../../examples/)** - Working code samples
- **[API Documentation](https://docs.rs/ruviz)** - Complete API reference

## Getting Help

- **Issues**: [GitHub Issues](https://github.com/ruviz/ruviz/issues)
- **Discussions**: [GitHub Discussions](https://github.com/ruviz/ruviz/discussions)
- **Examples**: [examples/](../../examples/) directory

---

**Congratulations!** You've completed the ruviz user guide. Happy plotting! 📊
