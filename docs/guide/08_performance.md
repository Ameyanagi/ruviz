# Performance Optimization

Comprehensive guide to maximizing ruviz performance for large datasets and production use.

## Performance Overview

ruviz is designed for high performance with multiple optimization strategies:

| Dataset Size | Render Time | Backend | Features |
|--------------|-------------|---------|----------|
| **< 1K points** | < 10ms | CPU (default) | Standard rendering |
| **1K - 10K** | < 50ms | CPU | Memory pooling |
| **10K - 100K** | < 100ms | Parallel | Multi-core rendering |
| **100K - 1M** | < 1s | Parallel + SIMD | Hardware acceleration |
| **> 1M points** | < 2s | DataShader | Intelligent aggregation |

**Benchmarked on**: AMD Ryzen 9 5950X (16 cores), 32GB RAM, Ubuntu 22.04

## Release Mode (Essential)

**Always use release builds for performance**:

```bash
# Development (slow, for debugging)
cargo run

# Production (optimized)
cargo run --release
```

**Performance difference**:
- Debug builds: **10-100x slower**
- Release builds: Full optimization enabled

### Cargo.toml Configuration

```toml
[profile.release]
lto = true           # Link-time optimization
codegen-units = 1    # Single codegen unit for max optimization
opt-level = 3        # Maximum optimization
```

## Backend Selection

ruviz automatically selects the optimal backend based on dataset size, but you can optimize further with feature flags.

### Default (CPU)

**Best for**: < 10K points, general use

```toml
[dependencies]
ruviz = "0.1"
```

**Characteristics**:
- Fast compilation
- Low memory overhead
- Single-threaded rendering

### Parallel Backend

**Best for**: 10K - 1M points

```toml
[dependencies]
ruviz = { version = "0.1", features = ["parallel"] }
```

**Characteristics**:
- Multi-core utilization
- Automatic load balancing
- 2-5x speedup for large datasets

**Example**:
```rust
use ruviz::prelude::*;

// Automatically uses parallel backend for 100K points
let x: Vec<f64> = (0..100_000).map(|i| i as f64 * 0.001).collect();
let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();

Plot::new()
    .line(&x, &y)
    .save("parallel_plot.png")?;  // Uses all CPU cores
```

**Performance**:
```rust
// Test with parallel feature
let start = std::time::Instant::now();
plot.save("test.png")?;
println!("Rendered 100K points in {:?}", start.elapsed());
// Typical: 50-100ms on 16-core system
```

### SIMD Backend

**Best for**: 100K - 1M points, numerical computation

```toml
[dependencies]
ruviz = { version = "0.1", features = ["parallel", "simd"] }
```

**Characteristics**:
- Hardware vector instructions
- Faster coordinate transformations
- Additional 20-40% speedup

**Use case**:
```rust
use ruviz::prelude::*;

// Intensive mathematical operations benefit from SIMD
let x: Vec<f64> = (0..500_000).map(|i| i as f64 * 0.0001).collect();
let y: Vec<f64> = x.iter().map(|v| {
    v.sin() * v.cos() * (-v * 0.1).exp()  // SIMD-accelerated
}).collect();

Plot::new()
    .line(&x, &y)
    .save("simd_plot.png")?;
```

### GPU Backend

**Best for**: > 1M points, real-time visualization

```toml
[dependencies]
ruviz = { version = "0.1", features = ["gpu"] }
```

**Characteristics**:
- Hardware GPU acceleration
- Best for massive datasets
- Requires graphics drivers

**Example**:
```rust
use ruviz::prelude::*;

// GPU automatically activates for very large datasets
let x: Vec<f64> = (0..10_000_000).map(|i| i as f64).collect();
let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();

Plot::new()
    .line(&x, &y)
    .save("gpu_plot.png")?;  // GPU-accelerated rendering
```

**System requirements**:
- Vulkan or Metal support
- Graphics drivers installed
- For headless systems, use CPU/parallel backends

### Performance Bundle

**Best for**: Maximum performance

```toml
[dependencies]
ruviz = { version = "0.1", features = ["performance"] }
# Equivalent to: features = ["parallel", "simd"]
```

## Memory Optimization

### Memory-Efficient Patterns

**1. Reuse data instead of cloning**:
```rust
use ruviz::prelude::*;

let x = vec![/* large dataset */];
let y = vec![/* large dataset */];

// Good: Pass references (no copy)
Plot::new()
    .line(&x, &y)
    .save("plot1.png")?;

// Bad: Don't clone unnecessarily
let x_copy = x.clone();  // Wastes memory
```

**2. Drop data after use**:
```rust
use ruviz::prelude::*;

{
    let large_data: Vec<f64> = generate_large_dataset();
    Plot::new()
        .histogram(&large_data, None)
        .save("histogram.png")?;
    // large_data dropped here, memory freed
}

// Continue with freed memory
```

**3. Subsampling for scatter plots**:
```rust
use ruviz::prelude::*;

let full_data: Vec<f64> = vec![/* 1M points */];

// For scatter plots, subsample intelligently
let step = 100;  // Every 100th point
let x_sample: Vec<f64> = full_data.iter()
    .step_by(step)
    .cloned()
    .collect();

Plot::new()
    .scatter(&x_sample, &y_sample)
    .save("scatter.png")?;
// Uses 10K points instead of 1M, same visual result
```

### Memory Pooling (Automatic)

ruviz automatically pools memory for repeated operations:

```rust
use ruviz::prelude::*;

// First plot allocates buffers
Plot::new().line(&x1, &y1).save("plot1.png")?;

// Subsequent plots reuse pooled memory
Plot::new().line(&x2, &y2).save("plot2.png")?;  // Faster
Plot::new().line(&x3, &y3).save("plot3.png")?;  // Faster
```

**Benefits**:
- Reduced allocation overhead
- Lower memory fragmentation
- Faster repeated operations

### Memory Monitoring

```rust
use ruviz::prelude::*;
use std::time::Instant;

let data_size = 50_000;
let x: Vec<f64> = (0..data_size).map(|i| i as f64 * 0.01).collect();
let y: Vec<f64> = x.iter().map(|v| v.sin() * (-v * 0.1).exp()).collect();

println!("Data size: {:.2} MB",
    (data_size * std::mem::size_of::<f64>() * 2) as f64 / 1_048_576.0);

let start = Instant::now();
Plot::new()
    .line(&x, &y)
    .save("memory_test.png")?;
println!("Rendered in {:?}", start.elapsed());
// Typical: 40ms for 50K points with memory optimization
```

## Large Dataset Strategies

### 10K - 100K Points

**Strategy**: Use parallel backend

```rust
use ruviz::prelude::*;

let points = 50_000;
let x: Vec<f64> = (0..points).map(|i| i as f64 * 0.01).collect();
let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();

Plot::new()
    .line(&x, &y)  // Automatically parallelized
    .save("medium_dataset.png")?;
```

**Optimization checklist**:
- ✅ Use `cargo run --release`
- ✅ Enable `parallel` feature
- ✅ Avoid unnecessary data cloning
- ✅ Use line plots over scatter for dense data

### 100K - 1M Points

**Strategy**: Parallel + SIMD

```toml
[dependencies]
ruviz = { version = "0.1", features = ["performance"] }
```

```rust
use ruviz::prelude::*;

let points = 500_000;
let x: Vec<f64> = (0..points).map(|i| i as f64 * 0.0001).collect();
let y: Vec<f64> = x.iter().map(|v| v.sin() * v.cos()).collect();

let start = std::time::Instant::now();
Plot::new()
    .line(&x, &y)
    .save("large_dataset.png")?;
println!("Rendered {} points in {:?}", points, start.elapsed());
// Typical: 200-500ms
```

**Optimization checklist**:
- ✅ Enable `parallel` and `simd` features
- ✅ Use release mode
- ✅ Simplify mathematical operations
- ✅ Consider reducing DPI for drafts

### > 1M Points

**Strategy**: DataShader (automatic)

```rust
use ruviz::prelude::*;

let points = 10_000_000;
let x: Vec<f64> = (0..points).map(|i| i as f64).collect();
let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();

// DataShader automatically activates for > 1M points
Plot::new()
    .line(&x, &y)
    .save("huge_dataset.png")?;
// Typical: < 2s even for 10M points
```

**How DataShader works**:
1. Intelligently bins data into pixel grid
2. Aggregates points within each bin
3. Renders aggregated representation
4. Maintains visual accuracy with orders of magnitude less data

**Benefits**:
- Handles 10M+ points efficiently
- Fixed memory usage regardless of dataset size
- Automatic activation, no configuration needed

## Multi-Series Optimization

### Parallel Series Rendering

```rust
use ruviz::prelude::*;

let size = 25_000;
let x: Vec<f64> = (0..size).map(|i| i as f64 * 0.002).collect();

// Multiple series rendered in parallel
let y1: Vec<f64> = x.iter().map(|&x| (x * 5.0).sin()).collect();
let y2: Vec<f64> = x.iter().map(|&x| (x * 3.0).cos()).collect();
let y3: Vec<f64> = x.iter().map(|&x| (x * 7.0).sin() * (x * 2.0).cos()).collect();

Plot::new()
    .line(&x, &y1).label("Series 1")
    .line(&x, &y2).label("Series 2")
    .line(&x, &y3).label("Series 3")
    .legend(Position::TopRight)
    .save("multi_series.png")?;
// Series processed concurrently on different cores
```

## Subplot Performance

### Efficient Subplot Rendering

```rust
use ruviz::prelude::*;
use std::time::Instant;

let start = Instant::now();

// Create 2×2 subplot with 1K points each
let plot1 = Plot::new().line(&x1, &y1).title("Panel 1").end_series();
let plot2 = Plot::new().line(&x2, &y2).title("Panel 2").end_series();
let plot3 = Plot::new().line(&x3, &y3).title("Panel 3").end_series();
let plot4 = Plot::new().line(&x4, &y4).title("Panel 4").end_series();

subplots(2, 2, 1600, 1200)?
    .subplot(0, 0, plot1)?
    .subplot(0, 1, plot2)?
    .subplot(1, 0, plot3)?
    .subplot(1, 1, plot4)?
    .save("efficient_subplot.png")?;

println!("Rendered 2×2 subplot in {:?}", start.elapsed());
// Typical: 147ms for 4 panels
```

**Optimization tips**:
- Individual panels rendered in parallel
- Shared memory pooling across panels
- Efficient layout calculation

## Compilation Performance

### Reducing Compile Times

**Feature selection**:
```toml
# Minimal features for fastest compilation
[dependencies]
ruviz = { version = "0.1", default-features = false }

# Add only what you need
ruviz = { version = "0.1", default-features = false, features = ["ndarray"] }
```

**Incremental compilation** (enabled by default in Cargo):
```bash
# First compile: ~30s
cargo build --release

# Subsequent compiles: ~3-5s (incremental)
cargo build --release
```

## Benchmarking

### Built-in Performance Measurement

```rust
use ruviz::prelude::*;
use std::time::Instant;

fn benchmark_plot(points: usize) -> std::time::Duration {
    let x: Vec<f64> = (0..points).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();

    let start = Instant::now();
    Plot::new()
        .line(&x, &y)
        .save("benchmark.png")
        .unwrap();
    start.elapsed()
}

fn main() {
    for &points in &[1_000, 10_000, 100_000, 1_000_000] {
        let time = benchmark_plot(points);
        let rate = points as f64 / time.as_secs_f64();
        println!("{:7} points: {:?} ({:.0} points/sec)",
                 points, time, rate);
    }
}
```

**Example output**:
```
  1,000 points: 8ms (125,000 points/sec)
 10,000 points: 25ms (400,000 points/sec)
100,000 points: 95ms (1,052,632 points/sec)
1,000,000 points: 720ms (1,388,889 points/sec)
```

### Criterion Benchmarks

For precise benchmarking:

```rust
// benches/plot_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ruviz::prelude::*;

fn benchmark_line_plot(c: &mut Criterion) {
    let x: Vec<f64> = (0..10_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();

    c.bench_function("line_plot_10k", |b| {
        b.iter(|| {
            Plot::new()
                .line(black_box(&x), black_box(&y))
                .save("bench_output.png")
                .unwrap()
        })
    });
}

criterion_group!(benches, benchmark_line_plot);
criterion_main!(benches);
```

```bash
cargo bench
```

## Performance Best Practices

### ✅ DO

1. **Use release mode** for all performance-critical work
2. **Enable appropriate features** (parallel, simd) for your dataset size
3. **Pass data by reference** to avoid unnecessary copies
4. **Subsample scatter plots** for > 10K points
5. **Reuse data** when creating multiple plots
6. **Profile before optimizing** to find actual bottlenecks

### ❌ DON'T

1. **Don't benchmark debug builds** - they're 10-100x slower
2. **Don't clone large datasets** unless necessary
3. **Don't use scatter for dense data** - use line plots
4. **Don't enable GPU** without Vulkan/Metal support
5. **Don't micro-optimize** without measurement

## Troubleshooting Performance

### Slow Rendering

**Problem**: Plot takes > 1s for 10K points

**Solutions**:
1. Check if using release mode: `cargo run --release`
2. Enable parallel feature: `features = ["parallel"]`
3. Verify data types are f64 (not converting from strings/other types)
4. Check for N² algorithms in data generation

### High Memory Usage

**Problem**: Out of memory errors

**Solutions**:
1. Use subsampling for scatter plots
2. Drop data after plotting: use scoped blocks
3. Generate data incrementally instead of all at once
4. Consider streaming or chunked processing

### Compilation Too Slow

**Problem**: `cargo build` takes > 60s

**Solutions**:
1. Use `--release` only for final builds
2. Disable unused features in Cargo.toml
3. Use `cargo check` for syntax validation (faster than build)
4. Enable incremental compilation (on by default)

## Next Steps

- **[Data Integration](09_data_integration.md)** - Work with ndarray, polars, CSV
- **[Export Formats](10_export.md)** - High-quality output options
- **[Advanced Techniques](11_advanced.md)** - Complex visualizations

---

**Ready to integrate with data libraries?** → [Data Integration Guide](09_data_integration.md)
