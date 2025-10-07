# Week 7: Performance Optimization Plan

## Overview
Optimize identified performance bottlenecks from Week 6 benchmarks, focusing on small dataset overhead and scatter plot efficiency.

## Success Criteria
1. ✅ Small dataset (1K points) rendering < 10ms
2. ✅ Scatter plot (10K points) rendering < 50ms
3. ✅ All optimizations validated with benchmarks
4. ✅ No performance regression on large datasets
5. ✅ Documentation updated with optimization results

## Master Roadmap Alignment
**Week 7: Performance Optimization**
- Address Week 6 benchmark findings
- Small dataset fast-path implementation
- Scatter plot rendering optimization
- Validate improvements with re-benchmarking

## Week 6 Findings Recap

### Optimization Priority 1: Small Dataset Overhead
**Current**: 1K points = 26.9ms
**Target**: < 10ms
**Gap**: 2.7x slower than target
**Root Cause**: Fixed setup overhead dominates small datasets

### Optimization Priority 2: Scatter Plot Performance
**Current**: 10K points = 54.8ms
**Target**: < 50ms
**Gap**: 1.1x slower (10% over)
**Root Cause**: Marker rendering overhead

## TDD Approach

### Part 1: Small Dataset Optimization

#### Phase 1: Profiling (Investigation)
**Goal**: Identify where time is spent in small dataset rendering

**Steps**:
1. Profile 1K point rendering with `cargo flamegraph`
2. Identify bottlenecks: setup vs rendering vs I/O
3. Measure time breakdown:
   - Font loading
   - Canvas allocation
   - Coordinate transformation
   - Actual rendering
   - File I/O

**Expected findings**:
- Setup overhead (font loading, cache) dominates
- Canvas allocation may be oversized
- Rendering itself likely fast

#### Phase 2: Red - Write Optimization Tests
Create `tests/small_dataset_optimization_test.rs`:

```rust
#[test]
fn test_small_dataset_under_10ms() {
    // GIVEN: 1K points
    let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    // WHEN: Render with optimized path
    let start = Instant::now();
    Plot::new()
        .line(&x, &y)
        .save("test_output/opt_small_1k.png")
        .expect("Failed to save plot");
    let duration = start.elapsed();

    // THEN: Should complete in < 10ms
    assert!(
        duration < Duration::from_millis(10),
        "Small dataset took {:?}, target < 10ms",
        duration
    );
}

#[test]
fn test_small_dataset_fast_path_selection() {
    // GIVEN: Small dataset (< 5K points)
    let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    // WHEN: Create plot
    let plot = Plot::new().line(&x, &y);

    // THEN: Should use fast path
    assert!(plot.is_using_fast_path());
}

#[test]
fn test_no_regression_large_datasets() {
    // GIVEN: 100K points (large dataset)
    let x: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    // WHEN: Render
    let start = Instant::now();
    Plot::new()
        .line(&x, &y)
        .auto_optimize()
        .save("test_output/opt_large_100k.png")
        .expect("Failed to save plot");
    let duration = start.elapsed();

    // THEN: Should still be fast (< 35ms)
    assert!(
        duration < Duration::from_millis(35),
        "Large dataset regression: {:?}",
        duration
    );
}
```

#### Phase 3: Green - Implement Fast Path
**Implementation Strategy**:

1. **Add fast-path detection** in `Plot::new()`:
   ```rust
   impl Plot {
       pub fn save<P: AsRef<Path>>(self, path: P) -> Result<()> {
           let total_points = self.count_total_points();

           if total_points < 5000 {
               // Fast path for small datasets
               self.render_fast_path(path)
           } else {
               // Normal path
               self.render_normal_path(path)
           }
       }
   }
   ```

2. **Optimize fast-path rendering**:
   - Skip parallel rendering setup
   - Use minimal canvas size
   - Cache font at module level
   - Pre-allocate common buffers
   - Skip unnecessary validations

3. **Fast-path optimizations**:
   ```rust
   fn render_fast_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
       // Use static font cache (one-time initialization)
       static FONT_CACHE: OnceCell<LoadedFont> = OnceCell::new();

       // Minimal canvas (don't over-allocate)
       let canvas = create_minimal_canvas(self.width, self.height);

       // Skip parallel setup
       let renderer = SkiaRenderer::new_simple(canvas)?;

       // Direct rendering (no threading overhead)
       renderer.render_series(&self.series)?;
       renderer.save(path)
   }
   ```

4. **Run tests**:
   ```bash
   cargo test --test small_dataset_optimization_test
   ```

#### Phase 4: Refactor - Benchmark Validation
Re-run baseline benchmarks:
```bash
cargo bench --bench baseline_benchmarks -- line_plot_1k
```

**Expected result**: 1K point rendering < 10ms

### Part 2: Scatter Plot Optimization

#### Phase 1: Red - Write Scatter Tests
Create `tests/scatter_optimization_test.rs`:

```rust
#[test]
fn test_scatter_10k_under_50ms() {
    // GIVEN: 10K points
    let x: Vec<f64> = (0..10_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0 + 10.0).collect();

    // WHEN: Render scatter plot
    let start = Instant::now();
    Plot::new()
        .scatter(&x, &y)
        .save("test_output/opt_scatter_10k.png")
        .expect("Failed to save plot");
    let duration = start.elapsed();

    // THEN: Should complete in < 50ms
    assert!(
        duration < Duration::from_millis(50),
        "Scatter plot took {:?}, target < 50ms",
        duration
    );
}

#[test]
fn test_marker_rendering_optimized() {
    // GIVEN: Scatter plot with default markers
    let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    // WHEN: Create plot
    let plot = Plot::new().scatter(&x, &y);

    // THEN: Should use optimized marker rendering
    assert!(plot.is_using_optimized_markers());
}
```

#### Phase 2: Green - Implement Scatter Optimization
**Implementation Strategy**:

1. **Marker caching**:
   ```rust
   struct MarkerCache {
       cached_markers: HashMap<(MarkerStyle, u32), Pixmap>,
   }

   impl MarkerCache {
       fn get_or_render(&mut self, style: MarkerStyle, size: u32) -> &Pixmap {
           self.cached_markers.entry((style, size))
               .or_insert_with(|| render_marker(style, size))
       }
   }
   ```

2. **SIMD marker placement** (if `simd` feature enabled):
   ```rust
   #[cfg(feature = "simd")]
   fn place_markers_simd(positions: &[(f32, f32)], marker: &Pixmap) {
       // Use SIMD for coordinate calculations
       // Batch marker placement operations
   }
   ```

3. **Batch rendering**:
   - Group markers by type
   - Render all same-type markers together
   - Reduce state changes

4. **Run tests**:
   ```bash
   cargo test --test scatter_optimization_test
   ```

#### Phase 3: Refactor - Benchmark Validation
Re-run scatter benchmarks:
```bash
cargo bench --bench baseline_benchmarks -- scatter_plot_10k
```

**Expected result**: 10K scatter plot < 50ms

## Timeline
- **Day 1**: Profiling and investigation (identify bottlenecks)
- **Day 2**: Small dataset optimization (TDD cycle)
- **Day 3**: Scatter plot optimization (TDD cycle)
- **Day 4**: Re-benchmark and validate improvements
- **Day 5**: Documentation and Week 7 completion

## Expected Improvements

### Before Optimization (Week 6 Baseline)
| Benchmark | Current | Target | Status |
|-----------|---------|--------|--------|
| Line 1K | 26.9ms | < 10ms | ❌ 2.7x slower |
| Scatter 10K | 54.8ms | < 50ms | ❌ 1.1x slower |

### After Optimization (Week 7 Target)
| Benchmark | Expected | Target | Status |
|-----------|----------|--------|--------|
| Line 1K | < 10ms | < 10ms | ✅ Target met |
| Scatter 10K | < 50ms | < 50ms | ✅ Target met |

**Success Metrics**:
- Line 1K: 2.7x improvement (26.9ms → < 10ms)
- Scatter 10K: 1.1x improvement (54.8ms → < 50ms)
- No regression on large datasets (100K still < 35ms)

## Optimization Techniques

### Small Dataset Fast-Path
1. **Static font caching**: One-time initialization
2. **Minimal canvas allocation**: Right-sized buffers
3. **Skip parallel overhead**: Direct rendering
4. **Skip unnecessary validation**: Trust small data
5. **Pre-warmed buffers**: Reuse common allocations

### Scatter Plot Optimization
1. **Marker caching**: Pre-render common markers
2. **Batch rendering**: Group same-type markers
3. **SIMD placement**: Vectorized coordinate math
4. **State minimization**: Reduce rendering state changes
5. **Memory locality**: Improve cache efficiency

## Validation Strategy

### Performance Tests
- Unit tests for < 10ms and < 50ms targets
- Regression tests for large datasets
- Benchmark comparison (before vs after)

### Quality Tests
- Visual regression tests (output unchanged)
- All existing tests still pass
- No memory leaks introduced

## Documentation Deliverables
1. **Optimization Results**: Before/after comparison
2. **Implementation Notes**: What was changed and why
3. **Performance Guide**: When to use which optimizations
4. **Week 7 Summary**: Complete optimization report

## Next Steps (Week 8)
- Additional profiling if targets not met
- SIMD optimization exploration
- GPU acceleration preparation
