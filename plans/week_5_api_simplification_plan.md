# Week 5: API Simplification - Detailed Plan

**Status**: Ready to start
**Phase**: Phase 2 - User Experience
**Dates**: Following Week 4 completion

## Objective

Create intelligent backend selection and beginner-friendly API to make ruviz accessible to new users while maintaining power-user flexibility.

## TDD Approach

Following strict Test-Driven Development:
1. **Write failing tests first** for auto-optimization logic
2. **Implement minimal code** to pass tests
3. **Refactor** for clarity and maintainability
4. **Verify** all functionality works correctly

## Tasks Breakdown

### Task 1: Auto-Optimization API (TDD)

**Test First**:
```rust
// tests/auto_optimization_test.rs
#[test]
fn test_small_dataset_uses_skia() {
    // GIVEN: Plot with <1K points
    let x: Vec<f64> = (0..500).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    let plot = Plot::new()
        .line(&x, &y)
        .auto_optimize();

    // THEN: Should select Skia backend
    assert_eq!(plot.get_backend(), Backend::Skia);
}

#[test]
fn test_medium_dataset_uses_parallel() {
    // GIVEN: Plot with 10K-100K points
    let x: Vec<f64> = (0..50_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    let plot = Plot::new()
        .line(&x, &y)
        .auto_optimize();

    // THEN: Should select Parallel backend
    assert_eq!(plot.get_backend(), Backend::Parallel);
}

#[test]
fn test_large_dataset_uses_gpu() {
    // GIVEN: Plot with >1M points
    let x: Vec<f64> = (0..2_000_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    let plot = Plot::new()
        .line(&x, &y)
        .auto_optimize();

    // THEN: Should select GPU backend if available
    let backend = plot.get_backend();
    assert!(backend == Backend::GPU || backend == Backend::DataShader);
}

#[test]
fn test_manual_override_respected() {
    // GIVEN: Plot with manual backend selection
    let x: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    let plot = Plot::new()
        .line(&x, &y)
        .backend(Backend::Skia)  // Manual override
        .auto_optimize();

    // THEN: Manual selection should be respected
    assert_eq!(plot.get_backend(), Backend::Skia);
}
```

**Implementation**:
- Add `auto_optimize()` method to Plot
- Implement backend selection logic based on data size
- Create decision tree: <1K → Skia, 1K-100K → Parallel, >100K → GPU/DataShader
- Respect manual backend overrides
- Add `get_backend()` method for testing

### Task 2: Backend Decision Logic (TDD)

**Test First**:
```rust
// tests/backend_selector_test.rs
#[test]
fn test_backend_selection_decision_tree() {
    use ruviz::backend_selector::select_backend;

    // Test thresholds
    assert_eq!(select_backend(500, false), Backend::Skia);
    assert_eq!(select_backend(10_000, false), Backend::Parallel);
    assert_eq!(select_backend(1_000_000, false), Backend::DataShader);
    assert_eq!(select_backend(1_000_000, true), Backend::GPU); // GPU available
}

#[test]
fn test_workload_profiling() {
    // GIVEN: Plot configuration
    let config = PlotConfig {
        data_points: 50_000,
        plot_types: vec![PlotType::Line, PlotType::Scatter],
        has_gradients: false,
        has_transparency: false,
    };

    // THEN: Should calculate appropriate backend
    let backend = profile_workload(&config);
    assert_eq!(backend, Backend::Parallel);
}
```

**Implementation**:
- Create `backend_selector` module
- Implement `select_backend()` function with decision tree
- Add workload profiling for complex plots
- Consider plot complexity factors (gradients, transparency, etc.)

### Task 3: Simple API Module (TDD)

**Test First**:
```rust
// tests/simple_api_test.rs
use ruviz::simple::*;

#[test]
fn test_line_plot_one_liner() {
    // GIVEN: Simple data
    let x = vec![0.0, 1.0, 2.0, 3.0];
    let y = vec![0.0, 1.0, 4.0, 9.0];

    // WHEN: Using simple API
    let result = line_plot(&x, &y, "test_line.png");

    // THEN: Should succeed
    assert!(result.is_ok());
    assert!(Path::new("test_line.png").exists());
}

#[test]
fn test_scatter_plot_with_title() {
    let x = vec![1.0, 2.0, 3.0];
    let y = vec![1.0, 4.0, 9.0];

    let result = scatter_plot_with_title(
        &x, &y,
        "Scatter Test",
        "test_scatter.png"
    );

    assert!(result.is_ok());
}

#[test]
fn test_bar_chart_simple() {
    let categories = vec!["A", "B", "C"];
    let values = vec![10.0, 20.0, 15.0];

    let result = bar_chart(&categories, &values, "test_bar.png");

    assert!(result.is_ok());
}
```

**Implementation**:
- Create `src/simple.rs` module
- Implement one-liner functions:
  - `line_plot(x, y, path)`
  - `scatter_plot(x, y, path)`
  - `bar_chart(categories, values, path)`
  - `histogram(data, path)`
- Add variants with titles: `*_with_title()`
- Auto-optimize by default in simple API

### Task 4: Documentation Updates (TDD)

**Test**: Integration tests verify examples work

**Implementation**:
- Update docs/guide/03_first_plot.md with auto_optimize()
- Add docs/guide/12_simple_api.md for beginners
- Update all examples to use .auto_optimize()
- Document manual override options
- Add backend selection guide

## Backend Selection Decision Tree

```
Data Analysis:
├─ Point count < 1K
│  └─ Backend: Skia (fast, simple)
├─ Point count 1K-10K
│  ├─ Complex (gradients/transparency)
│  │  └─ Backend: Skia (quality)
│  └─ Simple
│     └─ Backend: Parallel (efficient)
├─ Point count 10K-100K
│  └─ Backend: Parallel (multi-threaded)
├─ Point count 100K-1M
│  ├─ GPU available
│  │  └─ Backend: GPU (hardware acceleration)
│  └─ GPU not available
│     └─ Backend: DataShader (aggregation)
└─ Point count > 1M
   ├─ GPU available
   │  └─ Backend: GPU
   └─ GPU not available
      └─ Backend: DataShader
```

## Simple API Design

### Module Structure
```
src/simple.rs
├── line_plot()
├── line_plot_with_title()
├── scatter_plot()
├── scatter_plot_with_title()
├── bar_chart()
├── bar_chart_with_title()
├── histogram()
└── histogram_with_title()
```

### Design Principles
1. **Zero configuration**: Sensible defaults for everything
2. **Auto-optimization**: Always use auto_optimize()
3. **Minimal parameters**: Only data and output path required
4. **Progressive disclosure**: Basic → with_title → full Plot API
5. **Consistent naming**: verb_noun pattern

## Deliverables

1. **Auto-optimization API**
   - `.auto_optimize()` method on Plot
   - Intelligent backend selection
   - Manual override support

2. **Backend selector module**
   - Decision tree implementation
   - Workload profiling
   - GPU availability detection

3. **Simple API module**
   - `ruviz::simple::*` with one-liners
   - 8-10 convenience functions
   - Beginner-friendly documentation

4. **Test suite** (`tests/auto_optimization_test.rs`, `tests/simple_api_test.rs`)
   - Backend selection tests
   - Simple API integration tests
   - Manual override verification

5. **Documentation updates**
   - Updated user guide with auto_optimize()
   - New simple API guide
   - Backend selection guide

## Testing Strategy

### Unit Tests
- Backend selection logic for different data sizes
- Workload profiling calculations
- Manual override behavior

### Integration Tests
- End-to-end auto-optimization
- Simple API function calls
- Backend switching verification

### Performance Tests
- Verify backend selection improves performance
- Benchmark auto-optimization overhead (<1ms)

## Success Criteria

- [ ] All tests pass (15-20 new tests)
- [ ] Auto-optimization correctly selects backends
- [ ] Manual overrides work correctly
- [ ] Simple API functions are one-liners
- [ ] Documentation updated with auto_optimize()
- [ ] Backend selection guide complete
- [ ] Auto-optimization adds <1ms overhead
- [ ] Simple API examples in docs/guide/

## Timeline

**Day 1**: Write tests + implement auto-optimization
**Day 2**: Implement simple API (TDD)
**Day 3**: Documentation updates + integration
**Day 4**: Polish + verify all examples

**Total**: 3-4 days

## Next Steps After Week 5

Week 6: Performance Validation
- Baseline benchmarking
- Memory profiling
- Optimization opportunities
