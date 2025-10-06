# API Simplification & Backend Auto-Selection

## Status: MEDIUM PRIORITY
**Current State**: 5 rendering backends (Skia, Parallel, Pooled, SIMD, GPU) with manual selection, complex configuration space
**Gap**: Users don't know which backend to use, manual configuration is error-prone, no auto-optimization
**Impact**: Suboptimal performance, confused users, high support burden

---

## 🎯 Goals

1. **Intelligent defaults**: Auto-select best backend based on workload characteristics
2. **Progressive disclosure**: Simple API for beginners, advanced control for experts
3. **Backend abstraction**: Hide complexity unless needed
4. **Zero-overhead**: Auto-selection should add <1ms overhead
5. **Transparent optimization**: Log decisions for debugging

---

## 🧩 Current Backend Landscape

### Backend Inventory

| Backend | Purpose | Activation | Overhead | Best For |
|---------|---------|------------|----------|----------|
| **SkiaRenderer** | Default CPU rendering | Always available | None | <10K points |
| **ParallelRenderer** | Multi-threaded | `enable_parallel(true)` | Thread pool setup ~5ms | 10K-100K points |
| **PooledRenderer** | Memory optimization | `enable_pooled_rendering(true)` | Pool init ~1ms | Memory-constrained |
| **SIMDTransformer** | Vectorization | `feature = "simd"` | None | >100K points |
| **GpuRenderer** | GPU acceleration | `feature = "gpu"` | GPU init ~100ms | Real-time, interactive |
| **DataShader** | Extreme scale | `datashader(true)` | Canvas init ~10ms | >1M points |

### Current Selection Logic (Manual)

```rust
// User must know which to use
let mut plot = Plot::new();

// Small dataset - default is OK
plot.line(&small_x, &small_y);

// Medium dataset - user must remember to enable parallel
plot.enable_parallel(true);
plot.line(&medium_x, &medium_y);

// Large dataset - user must enable multiple optimizations
plot.enable_parallel(true);
// ... and separately enable SIMD feature in Cargo.toml
plot.line(&large_x, &large_y);

// Very large - user must switch to DataShader
plot.datashader(true);
plot.line(&huge_x, &huge_y);

plot.save("output.png")?;
```

**Problems**:
1. Requires user knowledge of internals
2. Easy to forget optimizations
3. No guidance on combinations
4. Performance left on table

---

## 🚀 Proposed Solution: Auto-Optimization API

### Design Principles

1. **Smart defaults**: Auto-select backend based on data characteristics
2. **Explicit control**: Allow manual override when needed
3. **Transparent**: Log decisions for debugging
4. **Fast**: Selection logic <1ms overhead
5. **Safe**: Never choose wrong backend (fall back to safe default)

---

### Phase 1: Backend Selection Strategy

#### A. Workload Profiler

**Create**: `src/core/backend_selector.rs`

```rust
use crate::render::*;

/// Analyzes plot workload and selects optimal backend configuration
pub struct BackendSelector {
    /// Whether to log selection decisions
    verbose: bool,
}

impl BackendSelector {
    pub fn new() -> Self {
        Self { verbose: false }
    }

    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Analyze workload and select backend
    pub fn select(&self, workload: &WorkloadProfile) -> BackendConfig {
        let config = self.analyze(workload);

        if self.verbose {
            log::info!("Backend selection: {:?} (reason: {})", config, config.reason());
        }

        config
    }

    fn analyze(&self, workload: &WorkloadProfile) -> BackendConfig {
        let total_points = workload.total_points();
        let series_count = workload.series_count();
        let has_gpu = workload.interactive || workload.gpu_available;

        // Decision tree
        match (total_points, series_count, has_gpu) {
            // Extreme scale: DataShader
            (n, _, _) if n > 1_000_000 => {
                BackendConfig::DataShader {
                    canvas_resolution: self.compute_canvas_resolution(workload),
                    reason: format!("{}M points requires aggregation", n / 1_000_000),
                }
            }

            // Large scale: Parallel + SIMD
            (n, s, _) if n > 100_000 || s > 10 => {
                BackendConfig::ParallelSIMD {
                    threads: num_cpus::get(),
                    chunk_size: (n / num_cpus::get()).max(10_000),
                    simd_enabled: cfg!(feature = "simd"),
                    reason: format!("{}K points with {} series benefits from parallelism",
                                    n / 1000, s),
                }
            }

            // Interactive: GPU if available
            (_, _, true) if workload.interactive => {
                BackendConfig::Gpu {
                    fallback: Box::new(BackendConfig::Parallel {
                        threads: num_cpus::get(),
                        reason: "GPU fallback".to_string(),
                    }),
                    reason: "Interactive mode with GPU available".to_string(),
                }
            }

            // Medium scale: Parallel
            (n, _, _) if n > 10_000 => {
                BackendConfig::Parallel {
                    threads: num_cpus::get(),
                    reason: format!("{}K points benefits from parallel rendering", n / 1000),
                }
            }

            // Memory constrained: Pooled
            _ if workload.memory_limited => {
                BackendConfig::Pooled {
                    reason: "Memory-constrained environment".to_string(),
                }
            }

            // Default: Skia
            _ => {
                BackendConfig::Default {
                    reason: format!("{} points, simple workload", total_points),
                }
            }
        }
    }

    fn compute_canvas_resolution(&self, workload: &WorkloadProfile) -> (usize, usize) {
        // DataShader canvas should be ~output resolution
        // Higher resolution = more detail but slower
        let (width, height) = workload.output_dimensions;
        let dpi_factor = workload.dpi / 96.0;
        (
            (width as f64 * dpi_factor) as usize,
            (height as f64 * dpi_factor) as usize,
        )
    }
}

/// Profile of plot workload characteristics
#[derive(Debug, Clone)]
pub struct WorkloadProfile {
    /// Total data points across all series
    total_points: usize,
    /// Number of data series
    series_count: usize,
    /// Output image dimensions
    output_dimensions: (u32, u32),
    /// Output DPI
    dpi: f64,
    /// Interactive mode requested
    interactive: bool,
    /// GPU available
    gpu_available: bool,
    /// Memory-constrained environment
    memory_limited: bool,
    /// Real-time rendering required
    realtime: bool,
}

impl WorkloadProfile {
    pub fn from_plot(plot: &Plot) -> Self {
        Self {
            total_points: plot.series.iter().map(|s| s.point_count()).sum(),
            series_count: plot.series.len(),
            output_dimensions: plot.dimensions,
            dpi: plot.dpi as f64,
            interactive: cfg!(feature = "interactive"),
            gpu_available: Self::check_gpu_available(),
            memory_limited: Self::check_memory_limited(),
            realtime: false, // TODO: detect from plot configuration
        }
    }

    fn total_points(&self) -> usize {
        self.total_points
    }

    fn series_count(&self) -> usize {
        self.series_count
    }

    #[cfg(feature = "gpu")]
    fn check_gpu_available() -> bool {
        use crate::render::gpu::is_gpu_available;
        is_gpu_available()
    }

    #[cfg(not(feature = "gpu"))]
    fn check_gpu_available() -> bool {
        false
    }

    fn check_memory_limited() -> bool {
        // Heuristic: Check available system memory
        // If < 2GB available, consider memory-limited
        #[cfg(target_os = "linux")]
        {
            if let Ok(info) = sys_info::mem_info() {
                return info.avail < 2 * 1024 * 1024; // < 2GB
            }
        }
        false
    }
}

/// Backend configuration decision
#[derive(Debug, Clone)]
pub enum BackendConfig {
    Default {
        reason: String,
    },
    Parallel {
        threads: usize,
        reason: String,
    },
    ParallelSIMD {
        threads: usize,
        chunk_size: usize,
        simd_enabled: bool,
        reason: String,
    },
    Pooled {
        reason: String,
    },
    Gpu {
        fallback: Box<BackendConfig>,
        reason: String,
    },
    DataShader {
        canvas_resolution: (usize, usize),
        reason: String,
    },
}

impl BackendConfig {
    fn reason(&self) -> &str {
        match self {
            BackendConfig::Default { reason } => reason,
            BackendConfig::Parallel { reason, .. } => reason,
            BackendConfig::ParallelSIMD { reason, .. } => reason,
            BackendConfig::Pooled { reason } => reason,
            BackendConfig::Gpu { reason, .. } => reason,
            BackendConfig::DataShader { reason, .. } => reason,
        }
    }
}
```

---

#### B. Auto-Optimize API

**Modify**: `src/core/plot.rs`

```rust
impl Plot {
    /// Automatically select and configure optimal backend for this plot
    ///
    /// Analyzes the data size, series count, and system capabilities to choose
    /// the best rendering backend. For manual control, use specific backend methods.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use ruviz::prelude::*;
    /// let x: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
    /// let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();
    ///
    /// Plot::new()
    ///     .auto_optimize() // Automatically selects parallel backend
    ///     .line(&x, &y)
    ///     .save("plot.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Performance
    ///
    /// Backend selection adds <1ms overhead. The selector uses heuristics based on:
    /// - Total data points
    /// - Number of series
    /// - Available system resources (CPU, GPU, memory)
    /// - Feature flags (simd, gpu, etc.)
    pub fn auto_optimize(&mut self) -> &mut Self {
        self.auto_optimize_verbose(false)
    }

    /// Auto-optimize with logging of backend selection decisions
    ///
    /// Same as `auto_optimize()` but logs the selected backend and reason.
    /// Useful for debugging performance issues.
    pub fn auto_optimize_verbose(&mut self, verbose: bool) -> &mut Self {
        let selector = BackendSelector::new().verbose(verbose);
        let workload = WorkloadProfile::from_plot(self);
        let config = selector.select(&workload);

        self.apply_backend_config(config);
        self
    }

    fn apply_backend_config(&mut self, config: BackendConfig) {
        match config {
            BackendConfig::Default { .. } => {
                // Use default Skia renderer (no changes)
            }
            BackendConfig::Parallel { threads, .. } => {
                self.enable_parallel = true;
                #[cfg(feature = "parallel")]
                {
                    self.parallel_renderer = ParallelRenderer::with_threads(threads);
                }
            }
            BackendConfig::ParallelSIMD { threads, chunk_size, simd_enabled, .. } => {
                self.enable_parallel = true;
                #[cfg(feature = "parallel")]
                {
                    let mut renderer = ParallelRenderer::with_threads(threads);
                    renderer = renderer.with_chunking(true, chunk_size);
                    #[cfg(feature = "simd")]
                    if simd_enabled {
                        renderer = renderer.with_simd(chunk_size);
                    }
                    self.parallel_renderer = renderer;
                }
            }
            BackendConfig::Pooled { .. } => {
                self.enable_pooled_rendering = true;
            }
            BackendConfig::Gpu { fallback, .. } => {
                #[cfg(feature = "gpu")]
                {
                    if is_gpu_available() {
                        self.gpu_accelerated = true;
                    } else {
                        // Fall back
                        self.apply_backend_config(*fallback);
                    }
                }
                #[cfg(not(feature = "gpu"))]
                {
                    // Fall back
                    self.apply_backend_config(*fallback);
                }
            }
            BackendConfig::DataShader { canvas_resolution, .. } => {
                self.use_datashader = true;
                self.datashader_resolution = Some(canvas_resolution);
            }
        }
    }
}
```

---

### Phase 2: Simplified API Layers

#### Beginner API (One-Liner)

```rust
/// Simplest possible API - smart defaults for everything
pub mod simple {
    pub use crate::prelude::*;

    /// Create and save a line plot in one call
    pub fn line_plot(
        x: &[f64],
        y: &[f64],
        output: &str,
    ) -> Result<()> {
        Plot::new()
            .auto_optimize()
            .line(x, y)
            .save(output)
    }

    /// Create and save a scatter plot
    pub fn scatter_plot(
        x: &[f64],
        y: &[f64],
        output: &str,
    ) -> Result<()> {
        Plot::new()
            .auto_optimize()
            .scatter(x, y)
            .save(output)
    }

    /// Create and save a bar chart
    pub fn bar_chart(
        categories: &[&str],
        values: &[f64],
        output: &str,
    ) -> Result<()> {
        Plot::new()
            .auto_optimize()
            .bar(categories, values)
            .save(output)
    }
}
```

**Usage**:
```rust
use ruviz::simple::*;

// Ultra-simple for beginners
line_plot(&x, &y, "output.png")?;
```

---

#### Standard API (Builder Pattern with Auto-Optimization)

```rust
// Current usage - auto-optimize added
Plot::new()
    .auto_optimize() // <-- New: automatic backend selection
    .line(&x, &y)
    .title("My Plot")
    .xlabel("X")
    .ylabel("Y")
    .save("output.png")?;
```

---

#### Advanced API (Manual Control)

```rust
// Expert users can override auto-selection
Plot::new()
    .backend(BackendConfig::ParallelSIMD {
        threads: 16,
        chunk_size: 50_000,
        simd_enabled: true,
        reason: "Custom configuration".to_string(),
    })
    .line(&x, &y)
    .save("output.png")?;

// Or use specific backend methods
Plot::new()
    .enable_parallel(true)
    .parallel_threads(16)
    .parallel_chunk_size(50_000)
    .enable_simd(true)
    .line(&x, &y)
    .save("output.png")?;
```

---

### Phase 3: Backend Unification

#### Common Backend Trait

**Create**: `src/render/backend.rs` (enhance existing)

```rust
/// Common interface for all rendering backends
pub trait RenderBackend: Send + Sync {
    /// Render the plot to an image buffer
    fn render(&mut self, tasks: &[RenderTask]) -> Result<ImageBuffer>;

    /// Get backend capabilities
    fn capabilities(&self) -> BackendCapabilities;

    /// Initialize backend (may be expensive)
    fn initialize(&mut self) -> Result<()>;

    /// Cleanup backend resources
    fn shutdown(&mut self);

    /// Get performance statistics
    fn stats(&self) -> BackendStats;
}

/// Backend capabilities
#[derive(Debug, Clone)]
pub struct BackendCapabilities {
    pub parallel: bool,
    pub simd: bool,
    pub gpu: bool,
    pub max_points: usize,
    pub memory_overhead: f64, // Multiplier on data size
}

/// Backend performance statistics
#[derive(Debug, Clone)]
pub struct BackendStats {
    pub last_render_time: Duration,
    pub total_renders: u64,
    pub points_rendered: u64,
    pub memory_used: usize,
}

/// Render task abstraction
#[derive(Debug, Clone)]
pub enum RenderTask {
    Line {
        points: Vec<Point2f>,
        style: LineStyle,
        color: Color,
        width: f32,
    },
    Scatter {
        points: Vec<Point2f>,
        marker: MarkerStyle,
        color: Color,
        size: f32,
    },
    Bar {
        bars: Vec<BarGeometry>,
        color: Color,
    },
    Text {
        text: String,
        position: Point2f,
        font: FontConfig,
    },
    // ... other primitives
}
```

**Implement for all backends**:
```rust
impl RenderBackend for SkiaRenderer { /* ... */ }
impl RenderBackend for ParallelRenderer { /* ... */ }
impl RenderBackend for PooledRenderer { /* ... */ }
#[cfg(feature = "gpu")]
impl RenderBackend for GpuRenderer { /* ... */ }
```

**Benefits**:
1. Uniform interface for all backends
2. Easy to add new backends
3. Automated testing of backend parity
4. Runtime backend switching

---

### Phase 4: Smart Caching & Warmup

#### A. Backend Warmup

**Problem**: First render with GPU/parallel may be slow due to initialization

**Solution**: Lazy initialization with optional warmup

```rust
impl Plot {
    /// Pre-initialize backends for fast first render
    ///
    /// Useful when you know you'll be rendering multiple plots
    /// and want to amortize initialization cost.
    pub fn warmup(&mut self) -> Result<()> {
        // Initialize configured backends
        if self.enable_parallel {
            #[cfg(feature = "parallel")]
            {
                let _ = rayon::ThreadPoolBuilder::new()
                    .num_threads(num_cpus::get())
                    .build_global();
            }
        }

        if self.gpu_accelerated {
            #[cfg(feature = "gpu")]
            {
                self.gpu_backend.initialize()?;
            }
        }

        Ok(())
    }
}
```

**Usage**:
```rust
let mut plot = Plot::new().auto_optimize();
plot.warmup()?; // Optional: initialize backends now

// Later renders are faster
for i in 0..10 {
    plot.line(&x_data[i], &y_data[i]);
    plot.save(&format!("frame_{}.png", i))?;
}
```

---

#### B. Backend Recommendation Cache

**Problem**: Re-analyzing workload on every render is wasteful

**Solution**: Cache backend decisions

```rust
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static! {
    static ref BACKEND_CACHE: Mutex<HashMap<WorkloadProfile, BackendConfig>> =
        Mutex::new(HashMap::new());
}

impl BackendSelector {
    pub fn select(&self, workload: &WorkloadProfile) -> BackendConfig {
        // Check cache first
        if let Ok(cache) = BACKEND_CACHE.lock() {
            if let Some(config) = cache.get(workload) {
                return config.clone();
            }
        }

        // Analyze and cache
        let config = self.analyze(workload);

        if let Ok(mut cache) = BACKEND_CACHE.lock() {
            cache.insert(workload.clone(), config.clone());
        }

        config
    }
}

// WorkloadProfile needs to be hashable
impl Hash for WorkloadProfile { /* ... */ }
impl PartialEq for WorkloadProfile { /* ... */ }
impl Eq for WorkloadProfile {}
```

---

## 📊 Migration Path

### Backward Compatibility

**Ensure existing code works without changes**:

```rust
// Old code (still works)
Plot::new()
    .line(&x, &y)
    .save("output.png")?;

// Old code with manual optimization (still works)
Plot::new()
    .enable_parallel(true)
    .line(&x, &y)
    .save("output.png")?;

// New code (opt-in auto-optimization)
Plot::new()
    .auto_optimize()
    .line(&x, &y)
    .save("output.png")?;
```

**No breaking changes** - auto-optimization is opt-in

---

### Deprecation Timeline

**Phase 1 (v0.2)**: Introduce `auto_optimize()`
- Add auto-selection API
- Document recommended usage
- No deprecations

**Phase 2 (v0.3)**: Recommend auto-optimization
- Make `auto_optimize()` default in examples
- Soft-deprecate manual backend methods
- Add migration guide

**Phase 3 (v1.0)**: Make auto-optimization default
- `Plot::new()` auto-optimizes by default
- Manual control via `.manual_backend()` or specific methods
- Breaking change with clear migration path

---

## 🧪 Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_selection_small_dataset() {
        let workload = WorkloadProfile {
            total_points: 1000,
            series_count: 1,
            // ... other fields
        };

        let selector = BackendSelector::new();
        let config = selector.select(&workload);

        assert!(matches!(config, BackendConfig::Default { .. }));
    }

    #[test]
    fn test_backend_selection_large_dataset() {
        let workload = WorkloadProfile {
            total_points: 100_000,
            series_count: 5,
            // ... other fields
        };

        let selector = BackendSelector::new();
        let config = selector.select(&workload);

        assert!(matches!(config, BackendConfig::Parallel { .. }));
    }

    #[test]
    fn test_backend_selection_extreme_dataset() {
        let workload = WorkloadProfile {
            total_points: 10_000_000,
            series_count: 1,
            // ... other fields
        };

        let selector = BackendSelector::new();
        let config = selector.select(&workload);

        assert!(matches!(config, BackendConfig::DataShader { .. }));
    }
}
```

---

### Integration Tests

```rust
#[test]
fn test_auto_optimize_produces_same_output() {
    let x: Vec<f64> = (0..50_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    // Manual backend
    Plot::new()
        .enable_parallel(true)
        .line(&x, &y)
        .save("manual.png")?;

    // Auto-optimized (should select parallel)
    Plot::new()
        .auto_optimize()
        .line(&x, &y)
        .save("auto.png")?;

    // Should produce identical output
    assert_images_identical("manual.png", "auto.png");
}
```

---

## 📚 Documentation

### User Guide Section

**Add to**: `docs/guide/07_backends.md`

```markdown
## Automatic Backend Selection (Recommended)

ruviz can automatically choose the best rendering backend for your data:

```rust
Plot::new()
    .auto_optimize() // Let ruviz choose the best backend
    .line(&x, &y)
    .save("output.png")?;
```

This analyzes your data size and system capabilities to select:
- **Default** for <10K points
- **Parallel** for 10K-100K points
- **Parallel + SIMD** for >100K points
- **DataShader** for >1M points
- **GPU** for interactive plots (if available)

### Debugging Backend Selection

To see which backend was chosen and why:

```rust
Plot::new()
    .auto_optimize_verbose(true) // Logs backend decision
    .line(&x, &y)
    .save("output.png")?;
```

Output:
```
[INFO] Backend selection: Parallel(threads=16) (reason: 85K points with 3 series benefits from parallelism)
```
```

---

## 🎯 Success Metrics

### Adoption
- [ ] 80%+ of examples use `auto_optimize()`
- [ ] User issues mention auto-optimization positively
- [ ] Documentation reflects auto-optimization as primary API

### Performance
- [ ] Auto-selection overhead <1ms
- [ ] Auto-selected backend within 90% of optimal manual selection
- [ ] No performance regressions vs manual selection

### Simplification
- [ ] New users can create fast plots without backend knowledge
- [ ] Advanced users can still override for specific needs
- [ ] Backend selection code is maintainable and testable

---

## 🔗 Related Plans
- [Documentation Strategy](01_documentation_onboarding_strategy.md) - Document auto-optimization
- [Testing Strategy](02_testing_qa_strategy.md) - Test backend parity
- [Performance Roadmap](03_performance_roadmap.md) - Optimize selection heuristics
