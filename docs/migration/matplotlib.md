# Migrating from matplotlib to ruviz

Complete guide for Python/matplotlib users transitioning to ruviz.

## Quick Comparison

### Basic Line Plot

**Python/matplotlib**:
```python
import matplotlib.pyplot as plt
import numpy as np

x = np.linspace(0, 10, 100)
y = np.sin(x)

plt.plot(x, y)
plt.title('Sine Wave')
plt.xlabel('X axis')
plt.ylabel('Y axis')
plt.grid(True)
plt.savefig('plot.png', dpi=300)
plt.show()
```

**Rust/ruviz**:
```rust
use ruviz::prelude::*;

let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();

Plot::new()
    .line(&x, &y)
    .title("Sine Wave")
    .xlabel("X axis")
    .ylabel("Y axis")
    .grid(true)
    .save("plot.png")?;
```

## API Translation Table

| matplotlib | ruviz | Notes |
|------------|-------|-------|
| `plt.plot(x, y)` | `Plot::new().line(&x, &y)` | Builder pattern |
| `plt.scatter(x, y)` | `Plot::new().scatter(&x, &y)` | |
| `plt.bar(x, y)` | `Plot::new().bar(&categories, &values)` | |
| `plt.hist(data)` | `Plot::new().histogram(&data, None)` | |
| `plt.title('text')` | `.title("text")` | Method chaining |
| `plt.xlabel('text')` | `.xlabel("text")` | |
| `plt.ylabel('text')` | `.ylabel("text")` | |
| `plt.legend()` | `.legend(Position::TopRight)` | Explicit position |
| `plt.grid(True)` | `.grid(true)` | |
| `plt.xlim(0, 10)` | `.xlim(0.0, 10.0)` | |
| `plt.ylim(0, 10)` | `.ylim(0.0, 10.0)` | |
| `plt.savefig('file.png')` | `.save("file.png")?` | Returns Result |
| `plt.savefig('file.png', dpi=300)` | `.dpi(300).save("file.png")?` | |
| `plt.show()` | N/A | Save to file instead |

## Common Patterns

### Multiple Series

**matplotlib**:
```python
plt.plot(x, y1, label='Linear')
plt.plot(x, y2, label='Quadratic')
plt.plot(x, y3, label='Cubic')
plt.legend()
```

**ruviz**:
```rust
Plot::new()
    .line(&x, &y1).label("Linear")
    .line(&x, &y2).label("Quadratic")
    .line(&x, &y3).label("Cubic")
    .legend(Position::TopLeft)
    .save("multi_series.png")?;
```

### Styling

**matplotlib**:
```python
plt.plot(x, y, color='red', linewidth=2, linestyle='--', marker='o')
```

**ruviz**:
```rust
Plot::new()
    .line(&x, &y)
    .color(Color::from_rgb(255, 0, 0))
    .line_width(2.0)
    .line_style(LineStyle::Dashed)
    .marker(MarkerStyle::Circle)
    .save("styled.png")?;
```

### Subplots

**matplotlib**:
```python
fig, axes = plt.subplots(2, 2, figsize=(12, 9))
axes[0, 0].plot(x, y1)
axes[0, 0].set_title('Plot 1')
axes[0, 1].scatter(x, y2)
axes[0, 1].set_title('Plot 2')
fig.suptitle('Multiple Plots')
plt.savefig('subplots.png')
```

**ruviz**:
```rust
let plot1 = Plot::new().line(&x, &y1).title("Plot 1").end_series();
let plot2 = Plot::new().scatter(&x, &y2).title("Plot 2").end_series();
let plot3 = Plot::new().bar(&cats, &vals).title("Plot 3").end_series();
let plot4 = Plot::new().histogram(&data, None).title("Plot 4").end_series();

subplots(2, 2, 1200, 900)?
    .subplot(0, 0, plot1)?
    .subplot(0, 1, plot2)?
    .subplot(1, 0, plot3)?
    .subplot(1, 1, plot4)?
    .suptitle("Multiple Plots")
    .save("subplots.png")?;
```

### Themes/Styles

**matplotlib**:
```python
plt.style.use('seaborn')
# or
plt.style.use('ggplot')
# or
import seaborn as sns
sns.set_theme()
```

**ruviz**:
```rust
Plot::new()
    .theme(Theme::seaborn())
    .line(&x, &y)
    .save("themed.png")?;

// Available themes:
// - Theme::light() (default)
// - Theme::dark()
// - Theme::publication() (for papers)
// - Theme::seaborn() (matplotlib seaborn style)
```

## Feature Comparison

### Supported in ruviz ✅

| Feature | matplotlib | ruviz |
|---------|------------|-------|
| Line plots | `plot()` | `.line()` |
| Scatter plots | `scatter()` | `.scatter()` |
| Bar charts | `bar()` | `.bar()` |
| Histograms | `hist()` | `.histogram()` |
| Box plots | `boxplot()` | `.boxplot()` |
| Multiple series | ✅ | ✅ |
| Legends | `legend()` | `.legend()` |
| Grid | `grid()` | `.grid()` |
| Titles/labels | ✅ | ✅ |
| Custom colors | ✅ | ✅ |
| Line styles | ✅ | ✅ |
| Markers | ✅ | ✅ |
| Subplots | `subplots()` | `subplots()` |
| Themes | `style.use()` | `.theme()` |
| DPI control | `dpi=` | `.dpi()` |
| Figure size | `figsize=` | `.dimensions()` |
| PNG export | `savefig()` | `.save()` |

### Not Yet Supported ⚠️

| Feature | matplotlib | ruviz Status |
|---------|------------|--------------|
| SVG export | `savefig('file.svg')` | Planned v0.2 |
| Heatmaps | `imshow()`, `pcolormesh()` | Planned v0.2 |
| Contour plots | `contour()` | Planned v0.2 |
| 3D plots | `mpl_toolkits.mplot3d` | Planned v1.0+ |
| Interactive plots | `%matplotlib notebook` | Experimental GPU backend |
| Polar plots | `projection='polar'` | Planned v0.3 |
| Animations | `FuncAnimation` | Planned v0.4 |

### Different Approach 🔄

| Feature | matplotlib | ruviz Equivalent |
|---------|------------|------------------|
| Interactive display | `plt.show()` | Save to file, view externally |
| Jupyter integration | `%matplotlib inline` | Save + display markdown cell |
| Global state | `plt.plot()` then `plt.title()` | Builder pattern (no global state) |
| Automatic figure mgmt | Implicit figure creation | Explicit `Plot::new()` |

## Data Types

### numpy arrays → Rust vectors

**matplotlib**:
```python
import numpy as np
x = np.linspace(0, 10, 100)
y = np.sin(x)
plt.plot(x, y)
```

**ruviz with ndarray** (closest to numpy):
```rust
use ndarray::Array1;

let x = Array1::linspace(0.0, 10.0, 100);
let y = x.mapv(|v| v.sin());

Plot::new()
    .line(&x, &y)
    .save("plot.png")?;
```

**ruviz with Vec** (standard Rust):
```rust
let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();

Plot::new()
    .line(&x, &y)
    .save("plot.png")?;
```

### pandas → polars

**matplotlib with pandas**:
```python
import pandas as pd
df = pd.read_csv('data.csv')
plt.plot(df['x'], df['y'])
```

**ruviz with polars**:
```rust
use polars::prelude::*;

let df = CsvReader::from_path("data.csv")?.finish()?;
let x = df.column("x")?.f64()?;
let y = df.column("y")?.f64()?;

Plot::new()
    .line(x, y)
    .save("plot.png")?;
```

## Performance Comparison

| Task | matplotlib | ruviz | Speedup |
|------|------------|-------|---------|
| 1K points | ~5ms | ~5ms | 1x |
| 10K points | ~50ms | ~18ms | 2.8x |
| 100K points | ~500ms | ~100ms | 5x |
| 1M points | ~5s | ~720ms | 7x |
| 10M points | ~60s | ~2s | 30x |

*Benchmarks on AMD Ryzen 9 5950X, Ubuntu 22.04*

## Migration Examples

### Example 1: Scientific Plot

**Before (Python)**:
```python
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 2*np.pi, 1000)
y_sin = np.sin(x)
y_cos = np.cos(x)

plt.figure(figsize=(10, 6), dpi=300)
plt.plot(x, y_sin, 'b-', label='sin(x)', linewidth=2)
plt.plot(x, y_cos, 'r--', label='cos(x)', linewidth=2)
plt.title('Trigonometric Functions', fontsize=16)
plt.xlabel('x (radians)', fontsize=12)
plt.ylabel('y', fontsize=12)
plt.legend(loc='upper right')
plt.grid(True, alpha=0.3)
plt.savefig('trig.png', dpi=300, bbox_inches='tight')
```

**After (Rust)**:
```rust
use ruviz::prelude::*;
use std::f64::consts::PI;

let x: Vec<f64> = (0..1000).map(|i| i as f64 * 2.0 * PI / 999.0).collect();
let y_sin: Vec<f64> = x.iter().map(|v| v.sin()).collect();
let y_cos: Vec<f64> = x.iter().map(|v| v.cos()).collect();

Plot::new()
    .dimensions(1000, 600)
    .dpi(300)
    .line(&x, &y_sin)
        .color(Color::from_rgb(0, 0, 255))
        .line_width(2.0)
        .label("sin(x)")
    .line(&x, &y_cos)
        .color(Color::from_rgb(255, 0, 0))
        .line_style(LineStyle::Dashed)
        .line_width(2.0)
        .label("cos(x)")
    .title("Trigonometric Functions")
    .xlabel("x (radians)")
    .ylabel("y")
    .legend(Position::TopRight)
    .grid(true)
    .save("trig.png")?;
```

### Example 2: Data Analysis

**Before (Python)**:
```python
import pandas as pd
import matplotlib.pyplot as plt

df = pd.read_csv('measurements.csv')
grouped = df.groupby('category')['value'].mean()

plt.bar(grouped.index, grouped.values)
plt.title('Average Values by Category')
plt.xlabel('Category')
plt.ylabel('Average Value')
plt.xticks(rotation=45)
plt.tight_layout()
plt.savefig('analysis.png')
```

**After (Rust)**:
```rust
use ruviz::prelude::*;
use polars::prelude::*;

let df = CsvReader::from_path("measurements.csv")?.finish()?;

let grouped = df
    .lazy()
    .groupby([col("category")])
    .agg([col("value").mean()])
    .collect()?;

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
    .save("analysis.png")?;
```

## FAQ

### Q: Can I use ruviz in Jupyter notebooks?
**A**: Not directly (no Python bindings yet). However, you can:
1. Save plots to files in Rust
2. Display them in Jupyter using `IPython.display.Image()`
3. Or use PyO3 to create Python bindings (community contribution welcome!)

### Q: How do I display plots interactively?
**A**: Current focus is file output. For interactive:
1. Use the experimental GPU backend (`features = ["gpu", "interactive"]`)
2. Or save to file and open with image viewer
3. Interactive mode planned for v0.3

### Q: What about animations?
**A**: Not yet supported. Planned for v0.4. Current workaround:
- Generate frame-by-frame PNGs
- Combine with ffmpeg or similar tool

### Q: Can I customize colors like matplotlib's colormap?
**A**: Yes, but differently:
```rust
// Custom colors
.color(Color::from_rgb(255, 128, 0))
.color(Color::from_hex("#FF8000"))

// Planned v0.2: color palettes
.color_palette(Palette::viridis())
```

### Q: Performance tips for large datasets?
**A**:
1. Use `features = ["parallel"]` for >10K points
2. Use `features = ["parallel", "simd"]` for >100K points
3. DataShader automatically activates for >1M points
4. See [Performance Guide](../guide/08_performance.md)

## Migration Checklist

- [ ] Install Rust and cargo
- [ ] Add `ruviz = "0.1"` to `Cargo.toml`
- [ ] Convert numpy arrays to `Vec<f64>` or `ndarray`
- [ ] Replace `plt.plot()` with `Plot::new().line()`
- [ ] Change `plt.savefig()` to `.save()?`
- [ ] Handle `Result` types with `?` operator
- [ ] Update data loading (pandas → polars if needed)
- [ ] Test with small dataset first
- [ ] Optimize with appropriate backend features
- [ ] Update CI/CD to compile Rust code

## Resources

- **[ruviz User Guide](../guide/README.md)** - Complete documentation
- **[Examples](../../examples/)** - Working code samples
- **[API Docs](https://docs.rs/ruviz)** - Full API reference
- **[Quickstart](../QUICKSTART.md)** - 5-minute tutorial

## Getting Help

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: Questions and community support
- **Examples Directory**: Real-world usage patterns

Ready to start? Follow the [User Guide](../guide/README.md) for step-by-step instructions.
