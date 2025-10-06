# Documentation & User Onboarding Strategy

## Status: CRITICAL PRIORITY
**Current State**: Minimal README (just header), good lib.rs docs, 29 examples but no cohesive guide
**Gap**: Users cannot easily discover capabilities, understand when to use which backend, or migrate from matplotlib/seaborn
**Impact**: Adoption barrier, user frustration, support burden

---

## 🎯 Goals

1. **Zero-to-First-Plot in < 5 minutes** for new Rust users
2. **Clear migration path** from Python (matplotlib/seaborn) to ruviz
3. **Progressive disclosure** of advanced features (parallel, GPU, memory optimization)
4. **Decision guides** for backend selection and configuration
5. **Visual gallery** showing capabilities at a glance

---

## 📋 Action Items

### Phase 1: Foundation (Week 1)

#### 1.1 README.md Overhaul
**Current**: Just "# ruviz"
**Target**: Comprehensive project introduction

```markdown
# ruviz

High-performance 2D plotting library for Rust combining matplotlib's usability with Makie's performance.

## Quick Start
[30-second example with output image]

## Features
- **Fast**: <100ms for 100K points
- **Safe**: Zero unsafe in public API
- **Flexible**: Line, scatter, bar, histogram, boxplot, heatmap
- **Beautiful**: Publication-quality themes

## Installation
[Cargo.toml snippet]

## Examples
[3 progressive examples: basic → styled → advanced]

## Documentation
- [User Guide](docs/guide/README.md)
- [API Docs](https://docs.rs/ruviz)
- [Gallery](docs/gallery/README.md)
- [Migration from matplotlib](docs/migration/matplotlib.md)

## Performance
[Benchmark chart: ruviz vs alternatives]

## License
[MIT/Apache-2.0]
```

**Files to create**:
- `README.md` (replace existing)
- `docs/QUICKSTART.md`
- `docs/FEATURES.md`
- `assets/readme_example.png` (generated from example)

---

#### 1.2 User Guide Structure

**Create `docs/guide/` with**:

1. **00_introduction.md** - Project vision, comparison with matplotlib/seaborn/plotters
2. **01_installation.md** - Cargo setup, feature flags explained
3. **02_first_plot.md** - Hello World walkthrough with explanations
4. **03_plot_types.md** - Comprehensive guide to line, scatter, bar, histogram, boxplot, heatmap
5. **04_styling.md** - Themes, colors, fonts, publication quality
6. **05_data_ingestion.md** - Vec, ndarray, polars integration
7. **06_subplots.md** - Multi-panel figures, GridSpec
8. **07_backends.md** - **CRITICAL**: When to use CPU vs parallel vs pooled vs GPU vs SIMD
9. **08_performance.md** - Optimization techniques, DataShader for large datasets
10. **09_export.md** - PNG, SVG, DPI settings for print
11. **10_interactive.md** - winit integration, real-time updates

**Format**: Each guide has:
- **Overview**: What you'll learn
- **Code example**: Complete, runnable
- **Output**: Visual showing result
- **Explanation**: Line-by-line breakdown
- **Common pitfalls**: FAQ for that topic
- **Next steps**: Links to related guides

---

### Phase 2: Decision Guides (Week 2)

#### 2.1 Backend Selection Flowchart

**File**: `docs/guide/07_backends.md`

```
┌─────────────────────────────────────────────┐
│ How many data points?                       │
└─────────────────────────────────────────────┘
           │
           ├─ < 10K ────────────► Default (Skia)
           │
           ├─ 10K - 100K ───────► Parallel (rayon)
           │
           ├─ 100K - 1M ────────► Pooled + SIMD
           │
           ├─ 1M - 100M ────────► DataShader
           │
           └─ Real-time? ───────► GPU + interactive

┌─────────────────────────────────────────────┐
│ What's your priority?                       │
└─────────────────────────────────────────────┘
           │
           ├─ Simplicity ────────► Default
           ├─ Speed ─────────────► Parallel + SIMD
           ├─ Memory ────────────► Pooled
           └─ Interactivity ─────► GPU + winit
```

**Decision Matrix Table**:

| Backend | Best For | Points | Memory | Compile Time | Dependencies |
|---------|----------|--------|--------|--------------|--------------|
| **Default (Skia)** | Simple plots, learning | <10K | Low | Fast | Core only |
| **Parallel** | Multi-series, CPU-bound | 10K-100K | Medium | Fast | +rayon |
| **Pooled** | Memory-constrained | Any | Minimal | Fast | Core only |
| **SIMD** | Vectorizable transforms | 100K+ | Low | Fast | +wide |
| **GPU** | Real-time, animations | Any | GPU memory | Slow | +wgpu |
| **DataShader** | Extreme datasets | 1M+ | Low | Fast | Core only |

**Auto-Selection Recommendation**:
```rust
// Proposed API addition
Plot::new()
    .auto_optimize() // Automatically selects best backend based on data
    .line(&x, &y)
    .save("output.png")?;
```

---

#### 2.2 Performance Characteristics Documentation

**File**: `docs/performance/BENCHMARKS.md`

**Contents**:
1. **Claimed Performance**: Document <100ms/100K, <1s/1M targets
2. **Verification**: Link to criterion benchmark results
3. **Comparison**: vs plotters, vs Python matplotlib (via PyO3)
4. **Scaling Charts**: Render time vs data points for each backend
5. **Memory Profiles**: Heap usage vs data size
6. **Recommendations**: When to optimize, what to optimize first

**Action**: Run systematic benchmarks and publish results:
```bash
cargo bench --all-features > docs/performance/benchmark_results.txt
# Generate charts from criterion output
python scripts/visualize_benchmarks.py
```

---

### Phase 3: Migration Guides (Week 3)

#### 3.1 Matplotlib to ruviz

**File**: `docs/migration/matplotlib.md`

**Structure**:
```markdown
# Migrating from matplotlib

## Philosophy Differences
- **Type safety**: Compile-time checking vs runtime errors
- **Performance**: Native speed vs Python overhead
- **Memory**: Explicit vs GC managed

## API Comparison

### Basic Plot
**matplotlib**:
```python
import matplotlib.pyplot as plt
plt.plot([1, 2, 3], [1, 4, 9])
plt.xlabel('x')
plt.ylabel('y')
plt.savefig('plot.png')
```

**ruviz**:
```rust
Plot::new()
    .line(&[1, 2, 3], &[1, 4, 9])
    .xlabel("x")
    .ylabel("y")
    .save("plot.png")?;
```

### Subplots
[Side-by-side examples]

### Styling
[Theme equivalents: seaborn → ruviz themes]

## Common Tasks
- Reading CSV: pandas → polars
- NumPy arrays → ndarray
- plt.show() → interactive feature

## FAQ
Q: How do I recreate plt.style.use('seaborn')?
A: Use Theme::seaborn() [link to styling guide]
```

---

#### 3.2 seaborn to ruviz

**File**: `docs/migration/seaborn.md`

Focus on:
- Statistical plot types (boxplot, histogram, heatmap)
- Color palettes mapping
- Multi-panel figures (FacetGrid equivalent)

---

### Phase 4: Visual Gallery (Week 4)

#### 4.1 Interactive Gallery Website

**File**: `docs/gallery/README.md`

**Structure**:
```
docs/gallery/
├── README.md (index with thumbnails)
├── basic/
│   ├── 01_line_plot.md
│   ├── 02_scatter_plot.md
│   └── ...
├── statistical/
│   ├── boxplot.md
│   ├── histogram.md
│   └── ...
├── publication/
│   ├── ieee_style.md
│   ├── nature_style.md
│   └── ...
├── performance/
│   ├── large_datasets.md
│   ├── real_time.md
│   └── ...
└── advanced/
    ├── custom_themes.md
    ├── gpu_acceleration.md
    └── ...
```

**Each gallery entry**:
- Thumbnail image
- Full-resolution output
- Complete source code
- Explanation of techniques
- Related examples

**Generation**:
```bash
# Script to generate gallery from examples/
cargo run --example <name> --features=gallery_output
# Captures output PNG + code + metadata
python scripts/build_gallery.py
# Generates markdown with thumbnails
```

---

### Phase 5: API Documentation (Week 5)

#### 5.1 Rustdoc Enhancement

**Current**: Basic doc comments
**Target**: Comprehensive examples in every public type

**Template for each public type**:
```rust
/// Brief description (one line)
///
/// # Overview
/// [Detailed explanation of purpose and design]
///
/// # Examples
///
/// Basic usage:
/// ```rust
/// # use ruviz::prelude::*;
/// let plot = Plot::new()
///     .line(&[1, 2, 3], &[1, 4, 9])
///     .save("plot.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Advanced usage:
/// [More complex example]
///
/// # Performance
/// [Notes on complexity, memory usage]
///
/// # See Also
/// [Links to related types/guides]
pub struct Plot { ... }
```

**Priority types for enhancement**:
- `Plot` (main API)
- `Theme`, `ThemeBuilder`
- `Data1D` trait
- Each plot type (in `plots/`)
- Backend renderers

---

#### 5.2 Feature Flag Documentation

**File**: `docs/guide/01_installation.md`

**Explain every feature**:

```toml
[dependencies]
ruviz = { version = "0.1", features = ["..."] }
```

| Feature | Adds | Use When | Compile Time | Size Impact |
|---------|------|----------|--------------|-------------|
| `default` | ndarray, parallel | General use | +5s | +2MB |
| `parallel` | rayon threading | >10K points | +3s | +500KB |
| `gpu` | wgpu acceleration | Real-time | +30s | +10MB |
| `simd` | Vectorization | >100K points | +1s | +100KB |
| `interactive` | winit window | Interactive plots | +15s | +5MB |
| `serde` | Serialization | Save/load configs | +2s | +200KB |
| `ndarray_support` | ndarray types | Scientific computing | +2s | +1MB |
| `polars_support` | DataFrame | Data analysis | +10s | +5MB |
| `full` | Everything | Power users | +45s | +20MB |

**Recommendations**:
- **Start with**: `default` (ndarray + parallel)
- **Add for data analysis**: `polars_support`
- **Add for speed**: `simd`
- **Add for real-time**: `interactive` + `gpu`
- **Minimal build**: `default-features = false` (core only)

---

## 🧪 Testing Strategy for Documentation

### Validate All Examples
```bash
# Test all code snippets in docs compile and run
cargo test --doc --all-features
```

### Link Checking
```bash
# Verify all internal links work
mdbook test docs/
```

### Visual Validation
```bash
# Ensure example outputs match docs images
cargo run --example gallery_validation
```

---

## 📊 Success Metrics

### Quantitative
- **README**: Star/watch ratio on GitHub (target: >5%)
- **Docs**: docs.rs page views (track via logs)
- **Gallery**: Example downloads (track via crates.io stats)
- **Time to First Plot**: Survey new users (target: <5 min)

### Qualitative
- **Issue Quality**: Users reference docs in issues (less "how do I...")
- **Migration**: Users report successful matplotlib→ruviz transitions
- **Contributions**: External contributors submit examples/gallery entries

---

## 🚀 Implementation Priority

### Critical (Week 1)
1. README.md overhaul
2. QUICKSTART.md
3. Backend selection guide (docs/guide/07_backends.md)

### High (Weeks 2-3)
4. Complete user guide (docs/guide/00-11)
5. Matplotlib migration guide
6. Performance benchmarks

### Medium (Week 4-5)
7. Visual gallery
8. API documentation enhancement
9. Feature flag guide

### Nice-to-Have
10. Video tutorials
11. Interactive playground (WASM)
12. Comparison benchmarks vs Python

---

## 📝 Templates & Assets

### Required Assets
- [ ] Logo/branding (SVG)
- [ ] Example plots (PNG, high-res)
- [ ] Benchmark charts (generated from criterion)
- [ ] Architecture diagrams (mermaid or SVG)

### Documentation Templates
- [ ] Example template (code + output + explanation)
- [ ] API doc template (rustdoc format)
- [ ] Guide template (consistent structure)

---

## 🔗 Related Plans
- [Testing & QA Strategy](02_testing_qa_strategy.md) - Validate documentation examples
- [Performance Optimization](03_performance_roadmap.md) - Document verified benchmarks
- [API Simplification](04_api_backend_simplification.md) - Auto-selection feature
