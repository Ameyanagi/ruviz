# Week 6: Performance Validation Plan

## Overview
Establish baseline performance metrics and comprehensive benchmarking infrastructure following TDD methodology.

## Success Criteria
1. ✅ Baseline benchmarks for all plot types
2. ✅ Memory profiling with allocation tracking
3. ✅ Performance regression tests
4. ✅ Benchmarking documentation
5. ✅ All benchmarks passing performance targets

## Master Roadmap Alignment
**Week 6: Performance Validation**
- Baseline performance benchmarking
- Memory profiling and optimization
- Performance regression prevention

## TDD Approach

### Part 1: Baseline Benchmarking

#### Red Phase - Write Failing Benchmark Tests
Create `benches/baseline_benchmarks.rs`:
```rust
// Expected to fail initially - no benchmark infrastructure yet

#[bench]
fn bench_line_plot_1k_points() {
    // GIVEN: 1K points
    // WHEN: Render line plot
    // THEN: < 10ms
}

#[bench]
fn bench_line_plot_100k_points() {
    // GIVEN: 100K points
    // WHEN: Render line plot
    // THEN: < 100ms
}

#[bench]
fn bench_scatter_plot_10k_points() {
    // GIVEN: 10K points
    // WHEN: Render scatter plot
    // THEN: < 50ms
}

#[bench]
fn bench_histogram_1m_points() {
    // GIVEN: 1M points
    // WHEN: Render histogram with binning
    // THEN: < 500ms
}

#[bench]
fn bench_boxplot_100k_points() {
    // GIVEN: 100K points
    // WHEN: Render box plot with quartiles
    // THEN: < 200ms
}

#[bench]
fn bench_multi_series_plot() {
    // GIVEN: 5 series, 10K points each
    // WHEN: Render multi-series line plot
    // THEN: < 150ms
}
```

Create `benches/memory_benchmarks.rs`:
```rust
// Memory allocation tracking benchmarks

#[bench]
fn bench_memory_line_plot_100k() {
    // GIVEN: 100K points
    // WHEN: Render line plot
    // THEN: Peak memory < 2x data size
}

#[bench]
fn bench_memory_multi_series() {
    // GIVEN: 10 series, 10K points each
    // WHEN: Render multi-series plot
    // THEN: Peak memory < 20MB
}
```

#### Green Phase - Implement Benchmarking Infrastructure
1. **Configure Cargo.toml**:
   ```toml
   [[bench]]
   name = "baseline_benchmarks"
   harness = false

   [[bench]]
   name = "memory_benchmarks"
   harness = false

   [dev-dependencies]
   criterion = "0.5"
   ```

2. **Implement Criterion benchmarks**:
   - Use Criterion for statistical analysis
   - Measure throughput (points/second)
   - Track memory allocations
   - Generate HTML reports

3. **Run benchmarks**:
   ```bash
   cargo bench --bench baseline_benchmarks
   cargo bench --bench memory_benchmarks
   ```

4. **Verify all benchmarks pass performance targets**

#### Refactor Phase
- Extract common benchmark utilities
- Add warmup iterations
- Implement benchmark result comparison

### Part 2: Memory Profiling

#### Red Phase - Write Memory Tests
Create `tests/memory_profiling_test.rs`:
```rust
#[test]
fn test_no_memory_leaks_1k_iterations() {
    // GIVEN: 1K plot iterations
    // WHEN: Create and drop plots repeatedly
    // THEN: Memory returns to baseline
}

#[test]
fn test_memory_pool_reuse() {
    // GIVEN: Multiple plots using same pool
    // WHEN: Sequential plot creation
    // THEN: No allocation growth after warmup
}

#[test]
fn test_large_dataset_memory_bound() {
    // GIVEN: 1M point dataset
    // WHEN: Render with DataShader
    // THEN: Memory stays < 100MB
}
```

#### Green Phase - Implement Memory Tracking
1. **Memory instrumentation**:
   - Track allocations per operation
   - Monitor peak memory usage
   - Detect leaks with repeated operations

2. **Profile tools integration**:
   - valgrind (Linux)
   - Instruments (macOS)
   - Windows Memory Profiler

3. **Run memory tests**:
   ```bash
   cargo test --test memory_profiling_test
   ```

### Part 3: Performance Regression Prevention

#### Red Phase - Write Regression Tests
Create `tests/performance_regression_test.rs`:
```rust
#[test]
fn test_performance_baseline_maintained() {
    // GIVEN: Historical baseline metrics
    // WHEN: Run current benchmarks
    // THEN: Performance within 10% of baseline
}

#[test]
fn test_auto_optimize_selection_speed() {
    // GIVEN: Various dataset sizes
    // WHEN: Call auto_optimize()
    // THEN: Decision time < 1ms
}
```

#### Green Phase - Implement Regression Detection
1. **Baseline storage**: Save benchmark results to `benches/baselines/`
2. **Comparison logic**: Compare current vs historical results
3. **CI integration**: Fail CI if regression detected

## Timeline
- Day 1: Write benchmark tests (Red phase)
- Day 2: Implement benchmarking infrastructure (Green phase)
- Day 3: Memory profiling setup and tests
- Day 4: Performance regression prevention
- Day 5: Documentation and refinement

## Expected Benchmark Results

### Rendering Performance
| Operation | Data Size | Target | Current |
|-----------|-----------|--------|---------|
| Line plot | 1K | < 10ms | TBD |
| Line plot | 100K | < 100ms | TBD |
| Scatter plot | 10K | < 50ms | TBD |
| Histogram | 1M | < 500ms | TBD |
| Box plot | 100K | < 200ms | TBD |
| Multi-series | 50K total | < 150ms | TBD |

### Memory Usage
| Operation | Data Size | Target Peak | Current |
|-----------|-----------|-------------|---------|
| Line plot | 100K | < 2x data | TBD |
| Multi-series | 100K total | < 20MB | TBD |
| Histogram | 1M | < 100MB | TBD |

## Documentation Deliverables
1. **Benchmark Guide**: How to run and interpret benchmarks
2. **Performance Targets**: Official performance claims
3. **Profiling Guide**: Memory profiling instructions
4. **CI Integration**: Automated benchmark running

## Next Steps (Week 7)
- Memory optimization based on profiling results
- CPU optimization (SIMD, parallel improvements)
- GPU acceleration validation
