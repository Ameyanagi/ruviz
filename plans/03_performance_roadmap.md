# Performance Optimization Roadmap

## Status: HIGH PRIORITY
**Current State**: Performance claims made (<100ms/100K), infrastructure in place (parallel, SIMD, GPU, pooled), but systematic validation missing
**Gap**: Unverified performance targets, unclear which optimizations provide value, no profiling data
**Impact**: Users may not achieve expected performance, optimization efforts might be misdirected

---

## 🎯 Performance Goals

### Verified Targets
| Dataset Size | Target | Backend | Status |
|--------------|--------|---------|--------|
| 1K points | <10ms | Default (Skia) | ❓ Unverified |
| 10K points | <30ms | Default | ❓ Unverified |
| 100K points | <100ms | Parallel | ❓ Unverified |
| 1M points | <1s | Parallel + SIMD | ❓ Unverified |
| 100M points | <2s | DataShader | ❓ Unverified |

### Memory Targets
| Operation | Target | Status |
|-----------|--------|--------|
| Data storage | <2x input size | ❓ Unverified |
| Font cache | <10MB | ❓ Unverified |
| Render buffer | <width × height × 4 | ❓ Unverified |
| Peak allocation | <data_size + buffer_size + 100MB | ❓ Unverified |

### Compilation Targets
| Build Type | Target | Status |
|------------|--------|--------|
| Core library | <10s | ❓ Unverified |
| With parallel | <15s | ❓ Unverified |
| With GPU | <45s | ❓ Unverified |
| Full features | <60s | ❓ Unverified |

---

## 🔍 Phase 1: Baseline Measurement (Week 1)

### 1.1 Systematic Benchmarking

**Create**: `benches/baseline_performance.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use ruviz::prelude::*;

fn bench_data_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_scaling");

    for size in [100, 1_000, 10_000, 100_000, 1_000_000].iter() {
        let x: Vec<f64> = (0..*size).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(
            BenchmarkId::new("default", size),
            size,
            |b, _| {
                b.iter(|| {
                    Plot::new()
                        .line(black_box(&x), black_box(&y))
                        .save("bench.png")
                        .unwrap();
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_data_scaling);
criterion_main!(benches);
```

**Run and collect baselines**:
```bash
cargo bench --bench baseline_performance > baseline_results.txt
```

**Analyze**:
- Identify which targets are already met
- Find performance bottlenecks
- Determine if optimization is needed

---

### 1.2 Memory Profiling

**Tool**: `valgrind` (Linux/macOS) or `dhat` (Rust-specific)

**Setup**:
```toml
[dependencies]
dhat = { version = "0.3", optional = true }

[features]
profiling = ["dhat"]
```

**Profile memory**:
```rust
#[cfg(feature = "profiling")]
use dhat::{Dhat, DhatAlloc};

#[cfg(feature = "profiling")]
#[global_allocator]
static ALLOCATOR: DhatAlloc = DhatAlloc;

fn main() {
    #[cfg(feature = "profiling")]
    let _dhat = Dhat::start_heap_profiling();

    // Run benchmark workload
    let x: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    Plot::new()
        .line(&x, &y)
        .save("profile.png")
        .unwrap();
}
```

**Run**:
```bash
cargo run --features profiling --release
# Generates dhat-heap.json
# View at https://nnethercote.github.io/dh_view/dh_view.html
```

**Measure**:
- Peak heap allocation
- Allocation counts
- Memory fragmentation
- Leak detection

---

### 1.3 CPU Profiling

**Tool**: `perf` (Linux), Instruments (macOS), or `cargo-flamegraph`

**Install**:
```bash
cargo install flamegraph
```

**Profile**:
```bash
# Linux
cargo flamegraph --bench baseline_performance

# macOS
cargo instruments --bench baseline_performance -t time
```

**Analyze**:
- Hotspots (functions consuming most CPU)
- Call graphs
- Optimization opportunities

---

## 🚀 Phase 2: Targeted Optimizations (Weeks 2-4)

### 2.1 Data Path Optimization

**Current Issues** (hypothetical, based on code review):
1. **Potential Vec clones** in `Plot::line()`, `Plot::scatter()`
2. **Unnecessary allocations** in coordinate transformation
3. **Inefficient iteration** over large datasets

**Optimization Strategy**:

#### A. Zero-Copy Data Ingestion

**Problem**: Current API may clone data
```rust
// Current (potentially cloning)
pub fn line<T: Data1D>(&mut self, x: &T, y: &T) -> &mut Self {
    self.series.push(PlotSeries {
        x_data: x.to_vec(), // Clone!
        y_data: y.to_vec(), // Clone!
        // ...
    });
    self
}
```

**Solution**: Store references or use `Cow`
```rust
use std::borrow::Cow;

pub struct PlotSeries {
    x_data: Cow<'static, [f64]>,
    y_data: Cow<'static, [f64]>,
    // ...
}

pub fn line<'a, T: Data1D>(&mut self, x: &'a T, y: &'a T) -> &mut Self
where
    'a: 'static, // Or use lifetime parameters
{
    self.series.push(PlotSeries {
        x_data: Cow::Borrowed(x.as_slice()),
        y_data: Cow::Borrowed(y.as_slice()),
        // ...
    });
    self
}
```

**Expected Gain**: 2x faster for large datasets, 50% less memory

**Benchmark**:
```rust
#[bench]
fn bench_zero_copy_vs_clone(c: &mut Criterion) {
    let x: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    c.bench_function("clone", |b| {
        b.iter(|| {
            let mut plot = Plot::new();
            plot.line(&x, &y); // Clones
        });
    });

    c.bench_function("zero_copy", |b| {
        b.iter(|| {
            let mut plot = Plot::new();
            plot.line_ref(&x, &y); // No clone
        });
    });
}
```

---

#### B. SIMD Coordinate Transformation

**Current**: Likely scalar loops
**Target**: Vectorized with `wide` or `std::simd`

**Implementation**:
```rust
use wide::f64x4;

fn transform_coordinates_simd(
    x: &[f64],
    y: &[f64],
    bounds: DataBounds,
    plot_area: PlotArea,
) -> Vec<Point2f> {
    let scale_x = plot_area.width / bounds.width();
    let scale_y = plot_area.height / bounds.height();

    let mut result = Vec::with_capacity(x.len());

    // Process 4 elements at a time
    let chunks = x.len() / 4;
    for i in 0..chunks {
        let idx = i * 4;

        let x_vec = f64x4::new([x[idx], x[idx+1], x[idx+2], x[idx+3]]);
        let y_vec = f64x4::new([y[idx], y[idx+1], y[idx+2], y[idx+3]]);

        // Vectorized transformation
        let px = (x_vec - bounds.min_x) * scale_x + plot_area.x;
        let py = plot_area.y + plot_area.height - ((y_vec - bounds.min_y) * scale_y);

        // Store results
        for j in 0..4 {
            result.push(Point2f::new(px[j] as f32, py[j] as f32));
        }
    }

    // Handle remaining elements
    for i in (chunks * 4)..x.len() {
        let px = (x[i] - bounds.min_x) * scale_x + plot_area.x;
        let py = plot_area.y + plot_area.height - ((y[i] - bounds.min_y) * scale_y);
        result.push(Point2f::new(px as f32, py as f32));
    }

    result
}
```

**Expected Gain**: 2-4x faster coordinate transformation
**Applies to**: >10K points

---

#### C. Parallel Series Processing

**Current**: `ParallelRenderer` exists but may not be optimally used

**Optimization**: Ensure parallelism is beneficial
```rust
impl ParallelRenderer {
    pub fn should_use_parallel(&self, series_count: usize, total_points: usize) -> bool {
        // Current threshold might be sub-optimal
        series_count >= self.parallel_threshold ||
        (self.chunked_processing && total_points > self.chunk_size * 2)
    }
}
```

**Tuning**:
```rust
// Benchmark different thresholds
#[bench]
fn bench_parallel_threshold(c: &mut Criterion) {
    for threshold in [1000, 5000, 10000, 50000] {
        c.bench_with_input(
            BenchmarkId::new("parallel_threshold", threshold),
            &threshold,
            |b, &threshold| {
                let renderer = ParallelRenderer::with_threshold(threshold);
                // Benchmark workload
            },
        );
    }
}
```

**Expected Gain**: Avoid parallelism overhead for small datasets
**Target**: Optimal threshold around 10K-50K points

---

### 2.2 Rendering Path Optimization

#### A. Reduce Allocations in Hot Path

**Tool**: Identify allocations with `dhat`

**Common culprits**:
- Temporary vectors in loops
- String formatting
- Unnecessary clones

**Pattern to avoid**:
```rust
// Bad: Allocates in loop
for point in &points {
    let temp = vec![point.x, point.y]; // Allocation per iteration!
    // ...
}
```

**Pattern to use**:
```rust
// Good: Reuse buffer
let mut temp = Vec::with_capacity(2);
for point in &points {
    temp.clear();
    temp.push(point.x);
    temp.push(point.y);
    // ...
}
```

**Or use pooling**:
```rust
// Best: Use existing PooledRenderer
let buffer = pooled_renderer.get_buffer();
// ... use buffer ...
// Automatically returned to pool on drop
```

---

#### B. Optimize Text Rendering

**Current**: `cosmic-text` for layout, `fontdue` for fallback

**Profile**: Measure font loading and text rendering separately

**Optimizations**:
1. **Cache font metrics**: Avoid recalculating on every render
2. **Batch text rendering**: Render all text elements in one pass
3. **Use texture atlas**: Pre-render common glyphs

**Implementation**:
```rust
pub struct TextRenderer {
    font_cache: HashMap<FontKey, LoadedFont>,
    glyph_atlas: GlyphAtlas,
    metrics_cache: HashMap<(FontKey, String), TextMetrics>,
}

impl TextRenderer {
    pub fn render_text(&mut self, text: &str, font: &FontKey) -> TextMetrics {
        // Check cache first
        if let Some(metrics) = self.metrics_cache.get(&(*font, text.to_string())) {
            return *metrics;
        }

        // Calculate and cache
        let metrics = self.calculate_metrics(text, font);
        self.metrics_cache.insert((*font, text.to_string()), metrics);
        metrics
    }
}
```

**Expected Gain**: 10x faster text rendering (especially for repeated labels)

---

### 2.3 Memory Optimization

#### A. Memory Pool Tuning

**Current**: `MemoryPool`, `PooledRenderer` exist

**Optimization**: Tune pool sizes based on profiling

```rust
pub struct MemoryPoolConfig {
    // Tune these based on typical workloads
    initial_capacity: usize,
    max_capacity: usize,
    block_size: usize,
    eviction_policy: EvictionPolicy,
}

impl Default for MemoryPoolConfig {
    fn default() -> Self {
        // Based on profiling of typical workloads
        Self {
            initial_capacity: 10, // 10 buffers
            max_capacity: 100,    // Cap at 100
            block_size: 1024 * 1024, // 1MB blocks
            eviction_policy: EvictionPolicy::LRU,
        }
    }
}
```

**Profiling**:
```bash
# Run with different pool configurations
MEMORY_POOL_SIZE=10 cargo bench
MEMORY_POOL_SIZE=50 cargo bench
MEMORY_POOL_SIZE=100 cargo bench

# Compare memory usage and performance
```

---

#### B. DataShader Optimization

**Current**: `DataShaderCanvas` for 100M+ points

**Optimization**: Ensure atomic operations are efficient

```rust
pub struct DataShaderCanvas {
    canvas: Vec<AtomicU32>, // Current
}

// Consider alternative for different workloads
pub enum CanvasBackend {
    Atomic(Vec<AtomicU32>),      // Thread-safe, slower
    Locked(Mutex<Vec<u32>>),     // Coarse-grained lock
    Chunked(Vec<Mutex<Vec<u32>>>), // Fine-grained locks
}
```

**Benchmark**:
```rust
#[bench]
fn bench_datashader_backends(c: &mut Criterion) {
    for backend in ["atomic", "locked", "chunked"] {
        c.bench_with_input(
            BenchmarkId::new("datashader", backend),
            &backend,
            |b, &backend| {
                // Test different backends
            },
        );
    }
}
```

---

## 📊 Phase 3: Validation & Documentation (Week 5)

### 3.1 Performance Test Suite

**Create**: `tests/performance/validated_claims.rs`

```rust
use std::time::Instant;

#[test]
fn verify_100k_points_100ms() {
    let x: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    let start = Instant::now();
    Plot::new()
        .line(&x, &y)
        .save("perf_test.png")
        .unwrap();
    let elapsed = start.elapsed();

    println!("100K points: {}ms", elapsed.as_millis());
    assert!(elapsed.as_millis() < 100, "Failed: {}ms", elapsed.as_millis());
}

// Similar tests for all claims
```

**Run systematically**:
```bash
cargo test --release --test validated_claims
```

---

### 3.2 Performance Documentation

**Create**: `docs/performance/PERFORMANCE.md`

```markdown
# ruviz Performance Characteristics

## Verified Performance (as of 2024-01-XX)

| Dataset Size | Measured | Target | Backend | Status |
|--------------|----------|--------|---------|--------|
| 1K points | 5ms | <10ms | Default | ✅ PASS |
| 10K points | 18ms | <30ms | Default | ✅ PASS |
| 100K points | 85ms | <100ms | Parallel | ✅ PASS |
| 1M points | 720ms | <1s | Parallel+SIMD | ✅ PASS |
| 100M points | 1.8s | <2s | DataShader | ✅ PASS |

**Test Environment**: AMD Ryzen 9 5950X, 64GB RAM, Ubuntu 22.04

## Memory Usage

[Charts showing memory vs data size]

## Backend Comparison

[Performance comparison chart]

## Scaling Characteristics

[Log-log plot showing near-linear scaling]

## Recommendations

- **<10K points**: Use default backend
- **10K-100K**: Use `enable_parallel(true)`
- **100K-1M**: Use `enable_parallel(true) + simd feature`
- **>1M**: Use `datashader(true)`
```

---

### 3.3 Optimization Guide for Users

**Create**: `docs/performance/OPTIMIZATION_GUIDE.md`

```markdown
# Performance Optimization Guide

## Quick Wins

### 1. Enable Parallel Processing
```rust
Plot::new()
    .enable_parallel(true) // Easy 2-3x speedup
    .line(&x, &y)
    .save("plot.png")?;
```

### 2. Use SIMD Feature
```toml
ruviz = { version = "0.1", features = ["simd"] }
```

### 3. Enable Pooled Rendering
```rust
Plot::new()
    .enable_pooled_rendering(true) // Reduce allocations
    .line(&x, &y)
    .save("plot.png")?;
```

## Advanced Optimizations

### DataShader for Large Datasets
[Example with 100M points]

### GPU Acceleration
[When to use, how to enable]

### Custom Memory Pool Configuration
[Advanced tuning]

## Profiling Your Application

[Guide to profiling ruviz usage]
```

---

## 🎯 Success Metrics

### Performance Targets Met
- [ ] 100K points < 100ms ✅
- [ ] 1M points < 1s ✅
- [ ] 100M points < 2s ✅
- [ ] Memory < 2x data size ✅

### Documentation Complete
- [ ] Performance characteristics documented
- [ ] Optimization guide published
- [ ] Benchmark results visualized
- [ ] Backend comparison chart

### User Feedback
- [ ] Performance claims verified by external users
- [ ] No performance-related issues
- [ ] Positive feedback on speed

---

## 🔧 Tools & Infrastructure

### Benchmarking
- **criterion**: Primary benchmark framework
- **cargo-bench**: Run benchmarks
- **criterion-compare**: Compare results

### Profiling
- **flamegraph**: CPU profiling
- **dhat**: Memory profiling
- **perf**: Linux performance analysis
- **Instruments**: macOS profiling

### CI/CD
- **GitHub Actions**: Automated benchmarking
- **benchmark-action**: Track performance over time
- **Performance regression detection**: Alert on slowdowns

---

## 🔗 Related Plans
- [Testing Strategy](02_testing_qa_strategy.md) - Performance validation tests
- [Documentation Strategy](01_documentation_onboarding_strategy.md) - Document benchmarks
- [API Simplification](04_api_backend_simplification.md) - Auto-optimization
