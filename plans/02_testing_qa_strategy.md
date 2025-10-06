# Testing & Quality Assurance Strategy

## Status: HIGH PRIORITY
**Current State**: 237 unit tests across 33 files, 29 examples, no formal integration tests, no visual regression, no systematic benchmarks
**Gap**: Performance claims unverified, no visual quality assurance, backends untested comparatively
**Impact**: Can't guarantee claimed performance, risk of visual regressions, backend feature parity unknown

---

## 🎯 Goals

1. **Verify performance claims**: <100ms/100K points, <1s/1M points, <2s/100M points
2. **Visual regression prevention**: Catch rendering changes automatically
3. **Backend parity testing**: Ensure all backends produce equivalent output
4. **Comprehensive coverage**: Unit + integration + performance + visual
5. **CI/CD integration**: Automated testing on every commit

---

## 📋 Current Test Inventory

### Unit Tests (237 occurrences)
**Coverage**: Good module-level testing
**Modules with tests**:
- `src/core/` - Plot API, validation
- `src/data/` - Data traits, memory management
- `src/render/` - Backends, styles, themes
- `src/plots/` - Plot types
- `src/interactive/` - Event handling

**Gaps**:
- No cross-module integration tests
- No full render pipeline tests
- No error path testing
- No edge case coverage (empty data, NaN, infinite values)

### Examples (29 files)
**Purpose**: Demonstration + ad-hoc testing
**Problem**: Not formal tests, no assertions, no CI validation

**Examples serving as tests**:
- `basic_example.rs` - Basic rendering
- `scientific_showcase.rs` - Multi-panel
- `memory_optimization_demo.rs` - Performance
- `parallel_demo.rs` - Parallel rendering
- `gpu_*_test.rs` - GPU backend

**Gap**: Examples aren't executable tests with pass/fail criteria

---

## 🏗️ Testing Architecture

### Layer 1: Unit Tests (Existing ✅, Enhance 📝)

**Keep current structure**: `#[cfg(test)]` modules in source files

**Enhancements needed**:

#### 1.1 Error Path Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_data_error() {
        let empty: Vec<f64> = vec![];
        let result = Plot::new().line(&empty, &empty).save("test.png");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PlottingError::EmptyData));
    }

    #[test]
    fn test_mismatched_data_length() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 2.0]; // Too short
        let result = Plot::new().line(&x, &y).save("test.png");
        assert!(result.is_err());
    }

    #[test]
    fn test_nan_handling() {
        let x = vec![1.0, 2.0, f64::NAN, 4.0];
        let y = vec![1.0, 4.0, 9.0, 16.0];
        // Should either filter NaN or error gracefully
        let result = Plot::new().line(&x, &y).save("test.png");
        assert!(result.is_ok()); // Document behavior
    }
}
```

#### 1.2 Property-Based Testing
```toml
[dev-dependencies]
proptest = "1.0"
```

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_any_valid_data_renders(
        x in prop::collection::vec(any::<f64>(), 1..1000),
        y in prop::collection::vec(any::<f64>(), 1..1000)
    ) {
        prop_assume!(x.len() == y.len());
        let result = Plot::new().line(&x, &y).save("test.png");
        prop_assert!(result.is_ok());
    }
}
```

---

### Layer 2: Integration Tests (NEW 📝)

**Create**: `tests/` directory with formal integration tests

**Structure**:
```
tests/
├── integration/
│   ├── full_pipeline_test.rs
│   ├── backend_parity_test.rs
│   ├── data_format_test.rs
│   └── subplot_composition_test.rs
├── visual/
│   ├── golden_image_test.rs
│   ├── regression_detector.rs
│   └── perceptual_diff.rs
├── performance/
│   ├── benchmark_validator.rs
│   ├── memory_profiler.rs
│   └── scaling_test.rs
└── fixtures/
    ├── golden_images/
    ├── test_data/
    └── reference_outputs/
```

#### 2.1 Full Pipeline Tests

**File**: `tests/integration/full_pipeline_test.rs`

```rust
/// Test complete render pipeline from API to PNG output
#[test]
fn test_basic_line_plot_pipeline() {
    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    let result = Plot::new()
        .line(&x, &y)
        .title("Test Plot")
        .xlabel("x")
        .ylabel("y")
        .save("test_output/integration_test.png");

    assert!(result.is_ok());

    // Verify file exists and has content
    let metadata = std::fs::metadata("test_output/integration_test.png").unwrap();
    assert!(metadata.len() > 0);

    // Verify PNG validity
    let img = image::open("test_output/integration_test.png").unwrap();
    assert_eq!(img.width(), 800); // Default width
    assert_eq!(img.height(), 600); // Default height
}

#[test]
fn test_multi_series_pipeline() {
    // Test multiple series rendering
    let result = Plot::new()
        .line(&[1.0, 2.0, 3.0], &[1.0, 4.0, 9.0])
        .line(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0])
        .scatter(&[1.5, 2.5], &[2.0, 6.0])
        .save("test_output/multi_series.png");

    assert!(result.is_ok());
}

#[test]
fn test_subplot_composition() {
    let (mut fig, axes) = subplots(2, 2)?;

    axes[0][0].line(&[1, 2, 3], &[1, 4, 9]);
    axes[0][1].scatter(&[1, 2, 3], &[1, 2, 3]);
    axes[1][0].bar(&["A", "B", "C"], &[1, 2, 3]);
    axes[1][1].histogram(&[1, 2, 2, 3, 3, 3, 4, 4, 5]);

    let result = fig.save("test_output/subplots.png");
    assert!(result.is_ok());
}
```

#### 2.2 Backend Parity Tests

**File**: `tests/integration/backend_parity_test.rs`

**Goal**: Ensure all backends produce visually equivalent output

```rust
use image::GenericImageView;

/// Compare outputs from different backends
#[test]
fn test_backend_parity_line_plot() {
    let x = vec![0.0, 1.0, 2.0, 3.0];
    let y = vec![0.0, 1.0, 4.0, 9.0];

    // Default (Skia) backend
    Plot::new()
        .line(&x, &y)
        .save("test_output/parity_default.png")?;

    // Parallel backend
    Plot::new()
        .enable_parallel(true)
        .line(&x, &y)
        .save("test_output/parity_parallel.png")?;

    // Pooled backend
    Plot::new()
        .enable_pooled_rendering(true)
        .line(&x, &y)
        .save("test_output/parity_pooled.png")?;

    #[cfg(feature = "gpu")]
    {
        // GPU backend
        Plot::new()
            .gpu_accelerated(true)
            .line(&x, &y)
            .save("test_output/parity_gpu.png")?;
    }

    // Compare images - should be pixel-perfect or perceptually identical
    let img1 = image::open("test_output/parity_default.png")?;
    let img2 = image::open("test_output/parity_parallel.png")?;

    assert_images_equivalent(&img1, &img2, 0.01); // 1% tolerance
}

fn assert_images_equivalent(img1: &DynamicImage, img2: &DynamicImage, tolerance: f64) {
    assert_eq!(img1.dimensions(), img2.dimensions());

    let (width, height) = img1.dimensions();
    let mut diff_pixels = 0;
    let total_pixels = (width * height) as f64;

    for y in 0..height {
        for x in 0..width {
            let p1 = img1.get_pixel(x, y);
            let p2 = img2.get_pixel(x, y);

            if p1 != p2 {
                diff_pixels += 1;
            }
        }
    }

    let diff_ratio = diff_pixels as f64 / total_pixels;
    assert!(
        diff_ratio < tolerance,
        "Images differ by {}%, tolerance is {}%",
        diff_ratio * 100.0,
        tolerance * 100.0
    );
}
```

---

### Layer 3: Visual Regression Tests (NEW 📝)

**Strategy**: Golden image comparison with perceptual diff

#### 3.1 Golden Image Generation

**File**: `tests/visual/golden_image_test.rs`

```rust
use std::path::Path;
use image::{DynamicImage, GenericImageView};

const GOLDEN_DIR: &str = "tests/fixtures/golden_images";
const TEST_OUTPUT_DIR: &str = "test_output/visual";

#[test]
fn test_basic_line_plot_visual() {
    let test_name = "basic_line_plot";
    let output_path = format!("{}/{}.png", TEST_OUTPUT_DIR, test_name);
    let golden_path = format!("{}/{}.png", GOLDEN_DIR, test_name);

    // Generate current output
    Plot::new()
        .line(&[0, 1, 2, 3], &[0, 1, 4, 9])
        .title("Test Plot")
        .save(&output_path)?;

    // Compare with golden image
    if !Path::new(&golden_path).exists() {
        // First run - copy as golden
        std::fs::copy(&output_path, &golden_path)?;
        println!("Generated golden image: {}", golden_path);
        return;
    }

    let current = image::open(&output_path)?;
    let golden = image::open(&golden_path)?;

    let diff = perceptual_diff(&current, &golden);
    assert!(
        diff < 0.001, // 0.1% perceptual difference
        "Visual regression detected: {}% difference",
        diff * 100.0
    );
}

/// Perceptual difference using structural similarity
fn perceptual_diff(img1: &DynamicImage, img2: &DynamicImage) -> f64 {
    // Simplified SSIM implementation
    // For production: use `image_compare` crate

    if img1.dimensions() != img2.dimensions() {
        return 1.0; // Completely different
    }

    let (width, height) = img1.dimensions();
    let mut total_diff = 0.0;

    for y in 0..height {
        for x in 0..width {
            let p1 = img1.get_pixel(x, y);
            let p2 = img2.get_pixel(x, y);

            let r_diff = (p1[0] as f64 - p2[0] as f64).abs() / 255.0;
            let g_diff = (p1[1] as f64 - p2[1] as f64).abs() / 255.0;
            let b_diff = (p1[2] as f64 - p2[2] as f64).abs() / 255.0;

            total_diff += (r_diff + g_diff + b_diff) / 3.0;
        }
    }

    total_diff / (width * height) as f64
}
```

**Workflow**:
1. First run: Generate golden images
2. Subsequent runs: Compare against golden
3. On intentional changes: `UPDATE_GOLDEN=1 cargo test --test visual`

#### 3.2 Visual Test Cases

**Comprehensive coverage**:
- Basic plots (line, scatter, bar)
- Styled plots (themes, colors, fonts)
- Complex plots (subplots, multi-series)
- Edge cases (single point, many points)
- Text rendering (titles, labels, legends)
- DPI variations (72, 96, 300, 600)

---

### Layer 4: Performance Tests (NEW 📝)

#### 4.1 Performance Validator

**File**: `tests/performance/benchmark_validator.rs`

**Goal**: Verify performance claims are met

```rust
use std::time::Instant;

#[test]
fn test_100k_points_under_100ms() {
    let x: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x * x).collect();

    let start = Instant::now();

    Plot::new()
        .line(&x, &y)
        .save("test_output/perf_100k.png")
        .unwrap();

    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 100,
        "Failed: 100K points took {}ms (target: <100ms)",
        elapsed.as_millis()
    );
}

#[test]
fn test_1m_points_under_1s() {
    let x: Vec<f64> = (0..1_000_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| (x / 1000.0).sin()).collect();

    let start = Instant::now();

    Plot::new()
        .enable_parallel(true) // Need optimization for 1M
        .line(&x, &y)
        .save("test_output/perf_1m.png")
        .unwrap();

    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 1000,
        "Failed: 1M points took {}ms (target: <1000ms)",
        elapsed.as_millis()
    );
}

#[test]
#[ignore] // Slow test - run explicitly
fn test_100m_points_under_2s_datashader() {
    let x: Vec<f64> = (0..100_000_000).map(|i| (i as f64) / 1e6).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    let start = Instant::now();

    Plot::new()
        .datashader(true) // Must use DataShader
        .line(&x, &y)
        .save("test_output/perf_100m.png")
        .unwrap();

    let elapsed = start.elapsed();

    assert!(
        elapsed.as_secs() < 2,
        "Failed: 100M points took {}s (target: <2s)",
        elapsed.as_secs()
    );
}
```

#### 4.2 Criterion Benchmark Suite

**Extend**: `benches/` directory

**File**: `benches/comprehensive_benchmarks.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use ruviz::prelude::*;

fn bench_render_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_scaling");

    for size in [100, 1_000, 10_000, 100_000, 1_000_000].iter() {
        let x: Vec<f64> = (0..*size).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, &_size| {
                b.iter(|| {
                    Plot::new()
                        .line(black_box(&x), black_box(&y))
                        .save("bench_output.png")
                        .unwrap();
                });
            },
        );
    }

    group.finish();
}

fn bench_backend_comparison(c: &mut Criterion) {
    let x: Vec<f64> = (0..50_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    c.bench_function("backend_default", |b| {
        b.iter(|| {
            Plot::new()
                .line(black_box(&x), black_box(&y))
                .save("bench.png")
                .unwrap();
        });
    });

    c.bench_function("backend_parallel", |b| {
        b.iter(|| {
            Plot::new()
                .enable_parallel(true)
                .line(black_box(&x), black_box(&y))
                .save("bench.png")
                .unwrap();
        });
    });

    c.bench_function("backend_pooled", |b| {
        b.iter(|| {
            Plot::new()
                .enable_pooled_rendering(true)
                .line(black_box(&x), black_box(&y))
                .save("bench.png")
                .unwrap();
        });
    });
}

criterion_group!(benches, bench_render_scaling, bench_backend_comparison);
criterion_main!(benches);
```

**Run**:
```bash
cargo bench --all-features
# Generates HTML reports in target/criterion/
```

---

## 🚀 CI/CD Integration

### GitHub Actions Workflow

**File**: `.github/workflows/test.yml`

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    name: Test Suite
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable, beta]
    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: ${{ matrix.rust }}
          components: clippy, rustfmt

      - name: Cache
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo
            target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Check formatting
        run: cargo fmt -- --check

      - name: Clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

      - name: Unit tests
        run: cargo test --all-features --lib

      - name: Integration tests
        run: cargo test --all-features --test integration

      - name: Visual regression tests
        run: cargo test --all-features --test visual

      - name: Performance validation
        run: cargo test --all-features --test performance -- --ignored

      - name: Doc tests
        run: cargo test --all-features --doc

      - name: Build examples
        run: cargo build --examples --all-features

  benchmark:
    name: Benchmark
    runs-on: ubuntu-latest
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'

    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run benchmarks
        run: cargo bench --all-features

      - name: Store benchmark results
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'criterion'
          output-file-path: target/criterion/*/new/estimates.json
          gh-pages-branch: gh-pages
          auto-push: true
```

---

## 📊 Coverage Targets

### Unit Test Coverage
**Target**: >80% line coverage
**Tool**: `cargo tarpaulin`

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --all-features --out Html --output-dir coverage/
```

### Integration Test Coverage
**Target**: All public API paths tested
**Checklist**:
- [ ] All plot types (line, scatter, bar, histogram, boxplot, heatmap)
- [ ] All backends (default, parallel, pooled, SIMD, GPU)
- [ ] All themes
- [ ] All data input formats (Vec, ndarray, polars)
- [ ] Subplots and multi-panel
- [ ] Export formats (PNG, SVG)

### Visual Regression Coverage
**Target**: Representative examples of all visual features
**Golden images needed**:
- [ ] Basic plots (6 types × 1 = 6 images)
- [ ] Themes (4 themes × 1 plot = 4 images)
- [ ] DPI variations (4 DPI × 1 plot = 4 images)
- [ ] Subplots (3 layouts = 3 images)
- [ ] Styling variations (colors, fonts, markers = 5 images)
**Total**: ~25 golden images

### Performance Test Coverage
**Target**: All performance claims verified
**Tests**:
- [ ] 100K points < 100ms
- [ ] 1M points < 1s
- [ ] 100M points < 2s (DataShader)
- [ ] Memory usage < 2x data size
- [ ] Font loading < 100ms
- [ ] Text rendering < 100ms (1000 elements)

---

## 🛠️ Testing Tools & Dependencies

### Add to `Cargo.toml`:
```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
approx = "0.5"
tempfile = "3.8"
rand = "0.8"
proptest = "1.0"
image = "0.24"
# image_compare = "0.3" # For better perceptual diff
```

### External Tools:
- **cargo-tarpaulin**: Coverage reports
- **cargo-criterion**: Benchmark visualization
- **nextest**: Faster test runner

---

## 📅 Implementation Timeline

### Week 1: Foundation
- [ ] Create `tests/` directory structure
- [ ] Set up CI/CD workflow
- [ ] Add error path unit tests

### Week 2: Integration
- [ ] Full pipeline tests
- [ ] Backend parity tests
- [ ] Data format tests

### Week 3: Visual Regression
- [ ] Golden image infrastructure
- [ ] Generate initial golden set
- [ ] Perceptual diff implementation

### Week 4: Performance
- [ ] Performance validator tests
- [ ] Comprehensive criterion benchmarks
- [ ] Memory profiling tests

### Week 5: Polish
- [ ] Coverage analysis
- [ ] CI optimization
- [ ] Documentation of test strategy

---

## 🔗 Related Plans
- [Documentation Strategy](01_documentation_onboarding_strategy.md) - Test all doc examples
- [Performance Roadmap](03_performance_roadmap.md) - Validate optimizations
- [API Simplification](04_api_backend_simplification.md) - Test auto-selection
