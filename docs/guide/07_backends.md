# Backend Selection Guide

ruviz provides multiple rendering backends optimized for different use cases. This guide helps you choose the right backend for your needs.

## TL;DR - Quick Decision

| Your Situation | Use This |
|----------------|----------|
| Learning ruviz, small datasets | **Default** - just use ruviz, nothing extra |
| 10K-100K points | **Parallel** - add `features = ["parallel"]` |
| 100K-1M points | **Parallel + SIMD** - add `features = ["parallel", "simd"]` |
| >1M points | **DataShader** - automatically enabled for large data |
| Real-time/interactive | **GPU** - add `features = ["gpu", "interactive"]` |
| Memory constrained | **Pooled** - optimizes memory usage |

## Available Backends

### 1. Default (Skia) Backend

**What it is**: CPU-based rendering using tiny-skia

**When to use**:
- Getting started with ruviz
- Small to medium datasets (<10K points)
- Simple plots without performance requirements
- When compile time matters

**Pros**:
- ✅ Fast compilation
- ✅ No extra dependencies
- ✅ Always available
- ✅ Predictable behavior

**Cons**:
- ❌ Slower for large datasets
- ❌ Single-threaded

**How to use**:
```rust
// Nothing special needed - this is the default!
Plot::new()
    .line(&x, &y)
    .save("plot.png")?;
```

**Performance**: ~5ms for 1K points, ~18ms for 10K points

---

### 2. Parallel Backend

**What it is**: Multi-threaded rendering using rayon

**When to use**:
- 10K-100K data points
- Multiple data series
- Multi-core system available
- Performance matters more than compile time

**Pros**:
- ✅ 2-4x faster for suitable workloads
- ✅ Scales with CPU cores
- ✅ No visual difference from default
- ✅ Automatic work distribution

**Cons**:
- ❌ +3s compile time
- ❌ Slight overhead for small datasets
- ❌ Requires rayon dependency

**How to use**:
```toml
[dependencies]
ruviz = { version = "0.1", features = ["parallel"] }
```

```rust
// Automatically uses parallel backend when beneficial
Plot::new()
    .line(&x, &y)  // Parallel processing for large data
    .save("plot.png")?;
```

**Performance**: ~85ms for 100K points (vs ~300ms single-threaded)

**Configuration**:
```rust
// Control parallelism (advanced)
Plot::new()
    .parallel_threads(8)  // Limit to 8 threads
    .line(&x, &y)
    .save("plot.png")?;
```

---

### 3. SIMD Backend

**What it is**: Vectorized coordinate transformations

**When to use**:
- >100K data points
- Modern CPU with SIMD support
- Maximum performance needed
- Scientific computing workloads

**Pros**:
- ✅ 2-4x faster coordinate transforms
- ✅ Low memory overhead
- ✅ Works with parallel backend
- ✅ Minimal compile time impact

**Cons**:
- ❌ Requires CPU SIMD support
- ❌ Complex to debug if issues arise
- ❌ Benefit only for large datasets

**How to use**:
```toml
[dependencies]
ruviz = { version = "0.1", features = ["parallel", "simd"] }
```

```rust
// SIMD used automatically for coordinate transforms
Plot::new()
    .line(&huge_x, &huge_y)  // 100K+ points
    .save("plot.png")?;
```

**Performance**: ~720ms for 1M points (vs ~1.5s without SIMD)

**Requirements**:
- x86_64 CPU with SSE2/AVX (most modern CPUs)
- ARM CPU with NEON (most modern ARM CPUs)

---

### 4. GPU Backend (Experimental)

**What it is**: GPU-accelerated rendering using wgpu

**When to use**:
- Real-time plotting (animations, live data)
- Interactive plots with zoom/pan
- Very large datasets with high frame rates
- When you have a discrete GPU

**Pros**:
- ✅ Extremely fast for real-time rendering
- ✅ Enables smooth interactions
- ✅ Can handle millions of points at 60 FPS
- ✅ Offloads work from CPU

**Cons**:
- ❌ +30s compile time
- ❌ Requires GPU and drivers
- ❌ Larger binary size (+10MB)
- ❌ Initialization overhead (~100ms)
- ❌ Experimental - API may change

**How to use**:
```toml
[dependencies]
ruviz = { version = "0.1", features = ["gpu", "interactive"] }
```

```rust
use ruviz::prelude::*;

// Static plot with GPU acceleration
Plot::new()
    .gpu_accelerated(true)
    .line(&x, &y)
    .save("plot.png")?;

// Interactive plot (requires winit)
let window = InteractiveWindow::new()
    .title("Interactive Plot")
    .build()?;

window.plot()
    .line(&x, &y)
    .show()?;  // Opens window with zoom/pan
```

**Performance**: 60 FPS for 1M+ points with interactions

**Requirements**:
- GPU with Vulkan/Metal/DirectX support
- GPU drivers installed
- For interactive: Display server running

---

### 5. DataShader Backend

**What it is**: Canvas-based aggregation for extreme datasets

**When to use**:
- >1M data points
- 10M-100M+ point datasets
- When individual points aren't visible
- Heatmap-style visualizations

**Pros**:
- ✅ Handles 100M+ points efficiently
- ✅ Constant memory usage
- ✅ Fast rendering (<2s for 100M points)
- ✅ Automatically enabled when needed

**Cons**:
- ❌ Loses individual point detail
- ❌ Aggregation introduces artifacts
- ❌ Not suitable for sparse data

**How to use**:
```rust
// Automatically uses DataShader for very large data
let huge_x: Vec<f64> = (0..10_000_000).map(|i| i as f64).collect();
let huge_y: Vec<f64> = huge_x.iter().map(|&x| x.sin()).collect();

Plot::new()
    .line(&huge_x, &huge_y)  // Auto DataShader for 10M points
    .save("huge_plot.png")?;

// Or explicitly enable
Plot::new()
    .datashader(true)
    .line(&x, &y)
    .save("plot.png")?;
```

**Performance**: ~1.8s for 100M points

**Configuration**:
```rust
// Control aggregation resolution
Plot::new()
    .datashader_resolution(2048, 1536)  // Canvas size
    .line(&huge_x, &huge_y)
    .save("plot.png")?;
```

---

### 6. Pooled Backend

**What it is**: Memory-pooled rendering for constrained environments

**When to use**:
- Memory-limited systems (<2GB RAM)
- Embedded systems
- When creating many plots
- Memory profiling shows high allocation

**Pros**:
- ✅ Minimal memory overhead
- ✅ Reduces allocations
- ✅ Good for batch processing
- ✅ Predictable memory usage

**Cons**:
- ❌ Slightly slower than default
- ❌ Requires manual enablement
- ❌ Complexity for small benefit

**How to use**:
```rust
// Enable pooled rendering
Plot::new()
    .enable_pooled_rendering(true)
    .line(&x, &y)
    .save("plot.png")?;
```

**Performance**: Similar to default, lower memory usage

---

## Decision Flowchart

```
┌─────────────────────────────────────────────────────┐
│ Start: What are you trying to do?                  │
└─────────────────────────────────────────────────────┘
                        │
                        ▼
         ┌──────────────────────────────┐
         │ How many data points?        │
         └──────────────────────────────┘
                        │
        ┌───────────────┼───────────────┐
        ▼               ▼               ▼
    < 10K          10K-100K         100K-1M         >1M
      │               │               │              │
      ▼               ▼               ▼              ▼
  Default         Parallel      Parallel+SIMD   DataShader

                        │
                        ▼
         ┌──────────────────────────────┐
         │ Need interactivity?          │
         └──────────────────────────────┘
                        │
                ┌───────┴───────┐
                ▼               ▼
              Yes              No
                │               │
                ▼               ▼
         GPU+Interactive    Continue above

                        │
                        ▼
         ┌──────────────────────────────┐
         │ Memory constrained?          │
         └──────────────────────────────┘
                        │
                ┌───────┴───────┐
                ▼               ▼
              Yes              No
                │               │
                ▼               ▼
            Pooled       Use performance choice
```

## Combining Backends

Some backends work together:

### Parallel + SIMD (Recommended for large data)
```toml
[dependencies]
ruviz = { version = "0.1", features = ["parallel", "simd"] }
```
Best for: 100K-1M points, multi-core CPU with SIMD

### Parallel + Pooled (Memory-efficient performance)
```rust
Plot::new()
    .enable_pooled_rendering(true)
    // Parallel automatically used when beneficial
    .line(&x, &y)
    .save("plot.png")?;
```
Best for: Large datasets on memory-limited systems

### GPU + Interactive (Real-time visualization)
```toml
[dependencies]
ruviz = { version = "0.1", features = ["gpu", "interactive"] }
```
Best for: Interactive dashboards, live data visualization

## Backend Comparison Table

| Backend | Compile Time | Memory | Speed (100K) | Best For |
|---------|--------------|--------|--------------|----------|
| Default | Fast (~5s) | Low | 300ms | Learning, small data |
| Parallel | Medium (~8s) | Medium | 85ms | Medium data, multi-core |
| SIMD | Medium (~9s) | Low | 60ms | Large data, modern CPU |
| Parallel+SIMD | Medium (~12s) | Medium | 40ms | Large data, performance |
| GPU | Slow (~35s) | GPU mem | <16ms | Real-time, interactive |
| DataShader | Fast (~5s) | Low | 200ms* | Extreme data (>1M) |
| Pooled | Fast (~5s) | Very Low | 320ms | Memory constrained |

*For 100K points. DataShader excels at 10M+ points.

## Auto-Selection (Coming in v0.2)

Future versions will automatically select the best backend:

```rust
// v0.2 feature
Plot::new()
    .auto_optimize()  // Chooses based on data size and system
    .line(&x, &y)
    .save("plot.png")?;

// With logging
Plot::new()
    .auto_optimize_verbose(true)
    .line(&x, &y)
    .save("plot.png")?;

// Output: [INFO] Selected: Parallel (reason: 85K points benefits from parallelism)
```

## Performance Testing

Test which backend works best for your data:

```rust
use std::time::Instant;

let start = Instant::now();
Plot::new()
    .line(&x, &y)
    .save("plot.png")?;
println!("Rendered in {:?}", start.elapsed());
```

Run with different features enabled and compare times.

## Troubleshooting

### "Parallel rendering is slower"
- You have <10K points (overhead exceeds benefit)
- Single-core CPU
- Try default backend instead

### "GPU initialization failed"
- No GPU or drivers not installed
- Falls back to parallel backend automatically
- Check GPU with: `cargo run --example gpu_debug_test`

### "Out of memory"
- Use DataShader for very large data
- Enable pooled rendering
- Reduce data resolution

### "Compilation takes forever"
- Disable GPU feature if not needed
- Use `default-features = false` for minimal build
- Consider using default backend only

## Recommendations by Use Case

### Scientific Computing
```toml
ruviz = { version = "0.1", features = ["parallel", "simd", "ndarray_support"] }
```

### Data Analysis
```toml
ruviz = { version = "0.1", features = ["parallel", "polars_support"] }
```

### Web Server (Static Plots)
```toml
ruviz = { version = "0.1", default-features = false }
```

### Interactive Dashboard
```toml
ruviz = { version = "0.1", features = ["gpu", "interactive"] }
```

### Embedded Systems
```toml
ruviz = { version = "0.1", default-features = false }
```

## Further Reading

- [Performance Guide](../performance/PERFORMANCE.md) - Detailed benchmarks
- [API Documentation](https://docs.rs/ruviz) - Complete reference
- [Examples](../../examples/) - Working code samples

## Questions?

If you're unsure which backend to use, start with **default** (no features) and only add features if you measure a performance problem.

> **Remember**: Premature optimization is the root of all evil. Start simple, measure, then optimize if needed.
