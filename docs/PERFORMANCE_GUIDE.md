# Performance Guide

Comprehensive guide to ruviz performance characteristics, optimization strategies, and best practices based on verified benchmarking results.

For the current cross-runtime large-dataset reference numbers, including Python `ruviz` vs
`matplotlib` and matching Rust/wasm runs, see
[docs/benchmarks/large-dataset-plotting.md](benchmarks/large-dataset-plotting.md).

## Why Large PNG Exports Are Faster Now

The current large-dataset benchmark page reflects a renderer change, not just a new benchmark.

For raster PNG export, `ruviz` now applies a few automatic fast paths when the output is visually equivalent at the target canvas size:

- Large monotonic solid line series without markers or error bars are reduced to a per-column envelope before stroking.
- Static histograms cache prepared `HistogramData` and reuse those bins for repeated exports.
- Nearest, non-annotated heatmaps render the final output surface directly and blit that image instead of drawing one rectangle per source cell.
- The parallel line backend now draws one polyline instead of thousands of two-point segments.

That changes the cost model:

- line export is closer to output-column work than raw point-count work
- histogram `render_only` avoids repeated binning
- heatmap export is closer to output-pixel work than source-cell work

In the current reference run this is why Rust `render_only` medians moved to roughly:

- line `1m`: `30.30 ms`
- histogram `5m`: `5.57 ms`
- heatmap `2048x2048`: `7.71 ms`

These improvements are specific to eligible raster PNG exports. SVG/vector output still uses the exact path. Python host-side rendering now also avoids the old snapshot JSON round-trip by keeping a native plot/prepared-plot handle alive across renders, while notebook widgets still use snapshots for browser transport.

## Verified Performance Metrics

**Based on Week 6 comprehensive benchmarking** (2025-10-07)

- **Hardware**: 16-core system
- **Benchmark Tool**: Criterion 0.5
- **Rust Edition**: 2024

### Rendering Performance Summary

| Operation | Dataset Size | Performance | vs Target |
|-----------|--------------|-------------|-----------|
| Line plot | 100K points | **34.6ms** | 2.9x faster ✅ |
| Histogram | 1M points | **87ms** | 5.7x faster ✅ |
| Box plot | 100K points | **28ms** | 7.1x faster ✅ |
| Multi-series | 50K total | **28.7ms** | 5.2x faster ✅ |

**Throughput**: 3.17 million elements/second

### Auto-Optimization Performance

Backend selection decision time: **< 142µs** (worst case)

| Dataset Size | Decision Time | Status |
|--------------|---------------|--------|
| 100 points | 218 nanoseconds | ✅ Excellent |
| 1K points | 1.5 microseconds | ✅ Excellent |
| 100K points | 142 microseconds | ✅ Excellent |

**Conclusion**: Zero user-facing overhead for auto-optimization.

### Small Dataset Performance

**Note**: 1K points take ~250ms on first plot (cold start)

**Time Breakdown**:
- Font initialization: 50-100ms
- Canvas setup: 20-50ms
- Rendering pipeline: 50-100ms
- File I/O: 30-50ms
- **Actual plotting: < 10ms** ✅

**Key Insight**: This is not algorithmic slowness - it's fixed overhead. The system excels at large datasets where this overhead is amortized.

## Performance Best Practices

### 1. Choose the Right Output Path

For large datasets, start with the output path you actually need:

```rust
use ruviz::prelude::*;

// PNG export: save() already applies its own large-dataset thresholds
Plot::new()
    .line(&x, &y)
    .save("plot.png")?;
```

**Why?**
- Decision time: < 142µs (negligible)
- `save()` already switches to its large-dataset export paths internally
- `render()` is the in-memory path that can use the parallel renderer

**Stored backend metadata (`auto_optimize()`)**:
- < 1K points → Skia
- 1,000 to 99,999 points → Parallel when built with the `parallel` feature, otherwise Skia
- 100,000+ points → GPU when built with the `gpu` feature, otherwise DataShader

### 2. Choose the Right Plot Type

| Use Case | Best Plot Type | Performance Characteristic |
|----------|----------------|----------------------------|
| Trends | Line plot | Fast, clear visualization |
| Distribution | Histogram | Optimized binning algorithm |
| Statistics | Box plot | Efficient quartile calculation |
| Comparison | Bar chart | Simple, fast rendering |
| Correlation | Scatter plot | Good for < 50K points |

**Example**: For 1M data points showing distribution:
```rust
// Good - histogram with automatic binning
Plot::new()
    .histogram(&data, None)  // 87ms for 1M points ✅
    .save("distribution.png")?;

// Slower - scatter plot for same purpose
Plot::new()
    .scatter(&x, &y)  // Would be much slower
    .save("distribution.png")?;
```

### 3. Understand Cold Start Overhead

**First plot in application**: ~250ms
- Font initialization: one-time cost
- Canvas setup: required for quality output
- Pipeline init: necessary setup
- **This is normal and expected!**

**Subsequent plots**: Much faster (fonts cached)

**Recommendation**:
- For applications creating multiple plots, cold start amortizes across all plots
- For single-plot scripts, 250ms is acceptable for publication-quality output
- Future batch API will reduce to < 10ms per plot after warmup

### 4. Large Dataset Optimization

For datasets with 100,000 points or more:

```rust
// Large PNG export
let x: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();

Plot::new()
    .line(&x, &y)
    .save("large_plot.png")?;
```

**Expected performance**:
- 100K points: ~35ms
- 1M points: ~87ms (histogram)

**Tips**:
- Consider DataShader for > 1M points
- Use histogram for distributions (faster than scatter)
- Use `render()` for in-memory images and `save()` for PNG export

### 5. Memory Efficiency

ruviz memory usage: typically < 2x data size

**Example**:
- 100K points (1.6MB data) → < 3.2MB peak memory
- Efficient buffer pooling
- Automatic cleanup on Plot drop

**For memory-constrained environments**:
```rust
// Process data in batches if needed
for batch in data.chunks(50_000) {
    Plot::new()
        .line(batch_x, batch_y)
        .save(&format!("batch_{}.png", i))?;
}
```

### 6. Multi-Series Plots

Multi-series plotting scales linearly:

```rust
// Efficient multi-series rendering
let plot = Plot::new()
    .line(&x, &y1)  // Series 1
    .line(&x, &y2)  // Series 2
    .line(&x, &y3)  // Series 3
    .line(&x, &y4)  // Series 4
    .line(&x, &y5); // Series 5

plot.save("multi_series.png")?;
```

**Performance**: 5 series × 10K points = 28.7ms total

**Tips**:
- Chain series methods before calling `.save()`
- Use distinct colors/styles for clarity
- Consider subplots for > 5 series

### 7. DPI and Output Quality

Higher DPI = larger files and longer rendering:

| DPI | File Size Multiplier | Use Case |
|-----|----------------------|----------|
| 96 | 1.0x (baseline) | Screen display |
| 150 | ~2.5x | Presentations |
| 300 | ~5.2x | IEEE publications |
| 600 | ~17.9x | High-res printing |

```rust
// For publications, use 300 DPI
Plot::new()
    .line(&x, &y)
    .dpi(300)  // IEEE standard
    .save("publication_plot.png")?;
```

**Rendering time impact**: Minimal (< 10% increase for 2x DPI)

## Benchmarking Your Application

To benchmark ruviz in your application:

```rust
use std::time::Instant;
use ruviz::prelude::*;

fn main() -> Result<()> {
    let x: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();

    let start = Instant::now();
    Plot::new()
        .line(&x, &y)
        .save("benchmark_plot.png")?;
    let duration = start.elapsed();

    println!("Plot rendered in {:?}", duration);
    println!("Throughput: {:.2} Melem/s",
             (x.len() as f64 / duration.as_secs_f64()) / 1_000_000.0);

    Ok(())
}
```

**Expected results** (100K points):
- Cold start: ~250-280ms
- Warm (2nd+ plot): ~30-40ms
- Throughput: 2.5-3.5 Melem/s

## Performance Debugging

If performance is slower than expected:

### Step 1: Verify the Path You Are Measuring

```rust
// In-memory rendering path
let _image = Plot::new()
    .line(&x, &y)
    .render()?;
```

### Step 2: Measure with Warmup

```rust
// Warmup run (ignore timing)
Plot::new().line(&x_small, &y_small).save("warmup.png")?;

// Actual measurement
let start = Instant::now();
Plot::new().line(&x, &y).save("actual.png")?;
let duration = start.elapsed();

println!("Warm performance: {:?}", duration);
```

### Step 3: Check Dataset Size

```rust
println!("Dataset size: {} points", x.len());
println!("Expected time: {} ms",
    if x.len() < 1_000 { 250 }
    else if x.len() < 100_000 { 50 }
    else { 35 });
```

### Step 4: Profile I/O

```rust
// Measure file write separately
let image = Plot::new().line(&x, &y).render()?;
let write_start = Instant::now();
image::save_buffer("plot.png", &image.pixels,
                   image.width, image.height,
                   image::ColorType::Rgba8)?;
let write_time = write_start.elapsed();

println!("I/O time: {:?}", write_time);
```

### Step 5: Report Issue

If performance is still unexpected, report with:
- System specifications (CPU cores, RAM)
- Dataset size and characteristics
- Timing measurements
- Whether you used `render()` or `save()`, and whether `.gpu(true)` was enabled
- Operating system

## Performance Expectations by Use Case

### Real-Time Visualization

**Goal**: < 50ms updates for interactive feel

**Strategy**:
```rust
// Cache Plot configuration, update data only
let mut current_data = initial_data;

loop {
    update_data(&mut current_data);

    let start = Instant::now();
    Plot::new()
        .line(&x, &current_data)
        .save("realtime_plot.png")?;

    if start.elapsed().as_millis() > 50 {
        eprintln!("Warning: frame took {:?}", start.elapsed());
    }

    std::thread::sleep(Duration::from_millis(100));
}
```

**Achievable**: 100K points in ~35ms ✅

### Batch Processing

**Goal**: Process thousands of plots efficiently

**Strategy**:
```rust
// Single warmup, then batch processing
Plot::new().line(&[1.0], &[1.0]).save("warmup.png")?;

for (i, dataset) in datasets.iter().enumerate() {
    Plot::new()
        .line(&dataset.x, &dataset.y)
        .save(&format!("plot_{:04}.png", i))?;
}
```

**Expected**: ~30-40ms per plot after warmup

### Scientific Publishing

**Goal**: High-quality output, correctness > speed

**Strategy**:
```rust
Plot::new()
    .line(&x, &y)
    .title("Figure 1: Experimental Results")
    .xlabel("Time (s)")
    .ylabel("Amplitude (V)")
    .dpi(300)  // IEEE standard
    .save("figure1.png")?;
```

**Expected**: ~250-300ms (acceptable for quality)

### Web Services

**Goal**: Responsive plot generation for users

**Strategy**:
```rust
use ruviz::{core::plot::Image, prelude::*};

// Async endpoint example
async fn generate_plot(data: Vec<(f64, f64)>) -> Result<Image> {
    let (x, y): (Vec<_>, Vec<_>) = data.into_iter().unzip();

    Plot::new()
        .line(&x, &y)
        .render()
}
```

**Expected**: 30-50ms response time for 10K points, plus any external PNG encoding you add on top of `Image { width, height, pixels }`

## Future Performance Work

Planned optimizations (future releases):

### Batch Rendering API (v0.2)

```rust
// Future API - amortized overhead
let renderer = BatchRenderer::new()?;  // One-time setup

for dataset in datasets {
    renderer.plot_line(&dataset.x, &dataset.y,
                      &format!("plot_{}.png", dataset.id))?;
}
// Expected: < 10ms per plot after warmup
```

### Static Font Cache (v0.2)

Reduce cold start by 50-100ms:
- Persistent font system across plots
- System-wide font cache
- One-time initialization

### SIMD Marker Rendering (v0.3)

Faster scatter plots:
- Vectorized marker drawing
- 2-3x improvement for scatter plots
- Platform-optimized (SSE, AVX, NEON)

### GPU Acceleration (experimental today)

Available today behind the `gpu` feature and explicit `.gpu(true)`:
- GPU-assisted PNG export in `save()` for sufficiently large datasets
- Automatic CPU fallback if GPU initialization fails
- `render()` continues to use the in-memory CPU/DataShader/parallel paths today

Future GPU work:
- Broader plot-type coverage
- Lower-latency interactive workflows
- Browser/WebGPU deployment

## Comparison with Other Libraries

**Note**: Benchmarks are approximate and workload-dependent. Always benchmark your specific use case.

### 100K Point Line Plot

| Library | Performance | Language | Backend/Mode |
|---------|-------------|----------|--------------|
| **ruviz** | **34.6ms** | Rust | Auto-optimized |
| matplotlib | ~500ms | Python | Multiple |
| plotters | ~100ms | Rust | Native CPU |
| plotly | ~200ms | Python | Browser (D3.js) |
| Chart.js | ~150ms | JavaScript | Browser (Canvas) |

**ruviz advantages**:
- Pure Rust performance
- Parallel rendering for large datasets
- Zero-overhead auto-optimization
- Publication-quality output

### 1M Point Histogram

| Library | Performance | Language | Notes |
|---------|-------------|----------|-------|
| **ruviz** | **87ms** | Rust | Includes binning |
| matplotlib | ~1.2s | Python | Standard binning |
| plotly | ~800ms | Python | With Dash |
| ggplot2 | ~600ms | R | Statistical focus |

**ruviz advantages**:
- 5.7x faster than target
- Efficient statistical computation
- Optimized binning algorithm

## Summary

### Performance Highlights

✅ **Excellent large dataset performance** (2.9-5.7x faster than targets)
✅ **Zero-overhead auto-optimization** (< 142µs decision time)
✅ **3.17 Melem/s throughput** (sustained across benchmarks)
✅ **Sub-100ms for 100K points** (34.6ms actual)
✅ **Production-ready performance** (verified with comprehensive benchmarks)

### Optimization Guidelines

1. **Use `render()` for in-memory images and `save()` for PNG export**
2. **Accept 250ms cold start as normal** (amortizes for multiple plots)
3. **Choose appropriate plot types** (histogram > scatter for distributions)
4. **Use 300 DPI for publications** (minimal performance impact)
5. **Profile before optimizing** (measure actual performance first)

### When to Use ruviz

**Ideal for**:
- Scientific data visualization
- Real-time monitoring (100K points in 35ms)
- Batch plot generation
- Publication-quality output
- Performance-critical applications

**Consider alternatives if**:
- Need production-grade interactive plots
- Require extensive plot customization (ruviz prioritizes performance)
- Working with web-only applications (WASM support planned)

## Additional Resources

- **Benchmark Results**: See `docs/BENCHMARK_RESULTS.md` for detailed metrics
- **Troubleshooting**: See `docs/TROUBLESHOOTING.md` for common issues
- **Optimization Findings**: See `docs/OPTIMIZATION_FINDINGS.md` for architectural analysis
- **GitHub Issues**: https://github.com/Ameyanagi/ruviz/issues for performance questions

---

**Last Updated**: 2025-10-07 (Week 8: Quality Polish)
**Benchmark Version**: Week 6 comprehensive validation
**Status**: Production-ready, verified performance
