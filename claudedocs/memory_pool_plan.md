# Memory Pool + Zero-Copy Rendering Implementation Plan

## Overview
Implement a high-performance memory management system for ruviz plotting library to reduce allocation overhead and enable zero-copy data processing. This foundational performance enhancement will benefit all subsequent features.

## Performance Targets
- **30-50% reduction** in allocation time for large plots (>10K points)
- **Zero memory growth** in steady-state rendering loops
- **No regression** in existing functionality or API compatibility
- **Thread-safe** operation for future parallel rendering support

## Architecture

### Core Components

#### 1. MemoryPool<T> - Typed Memory Pool
```rust
pub struct MemoryPool<T> {
    available: VecDeque<Box<[T]>>,
    in_use: HashSet<*const T>,
    chunk_size: usize,
    max_pools: usize,
}

impl<T> MemoryPool<T> {
    pub fn acquire(&mut self, len: usize) -> PooledBuffer<T>;
    pub fn release(&mut self, buffer: PooledBuffer<T>);
    fn grow_pool(&mut self);
    fn shrink_pool(&mut self);
}
```

#### 2. PooledVec<T> - Transparent Pool Integration
```rust
pub struct PooledVec<T> {
    buffer: PooledBuffer<T>,
    len: usize,
    pool_ref: Arc<Mutex<MemoryPool<T>>>,
}

impl<T> Deref for PooledVec<T> {
    type Target = [T];
    // Provides Vec-like interface using pooled memory
}
```

#### 3. DataView<T> - Zero-Copy Data Views
```rust
pub struct DataView<T> {
    data: NonNull<T>,
    len: usize,
    _phantom: PhantomData<&'static [T]>,
}

impl<T: Into<f64> + Copy> Data1D for DataView<T> {
    // Zero-copy implementation of Data1D trait
}
```

### Memory Pool Strategy

#### Pool Sizing
- **Dynamic sizing** based on usage patterns
- **Separate pools** for different data types:
  - `f64` arrays for coordinate data
  - `u8` arrays for pixel buffers  
  - `PremultipliedColorU8` for image data
- **Adaptive growth/shrink** based on demand

#### Thread Safety
- **Thread-local pools** for hot paths
- **Work-stealing** between threads when needed
- **Arc<Mutex<Pool>>** only for cross-thread coordination
- **Lock-free fast path** for single-threaded access

## Integration Points

### 1. Coordinate Transformation Pipeline
**Current**: Multiple Vec allocations during transformation
```rust
// src/core/plot.rs - current allocation hotspot
let transformed_x: Vec<f64> = x_data.iter().map(transform).collect();
let transformed_y: Vec<f64> = y_data.iter().map(transform).collect();
```

**Enhanced**: Zero-copy pooled transformation
```rust
let mut transformed_x = pool.acquire_vec::<f64>(x_data.len());
let mut transformed_y = pool.acquire_vec::<f64>(y_data.len());
transform_in_place(&x_data, &mut transformed_x);
transform_in_place(&y_data, &mut transformed_y);
```

### 2. Pixmap Buffer Allocation
**Current**: New allocation for each plot
```rust
// src/render/skia.rs - pixmap creation hotspot
let mut pixmap = Pixmap::new(width, height).unwrap();
```

**Enhanced**: Pooled pixmap buffers
```rust
let pixmap_buffer = pool.acquire_buffer::<u8>(width * height * 4);
let mut pixmap = Pixmap::from_buffer(pixmap_buffer, width, height);
```

### 3. Text Rendering Buffers
**Current**: cosmic-text creates internal buffers
```rust
// src/render/cosmic_text_renderer.rs - buffer allocation
let mut buffer = Buffer::new(&mut self.font_system, metrics);
buffer.set_size(&mut self.font_system, Some(buffer_width), Some(buffer_height));
```

**Enhanced**: Pre-sized pooled text buffers
```rust
let text_buffer = pool.acquire_text_buffer(buffer_width, buffer_height);
let mut buffer = Buffer::from_pooled(&mut self.font_system, text_buffer, metrics);
```

## API Design

### Transparent Integration
The pool system should be invisible to most ruviz users:

```rust
// User code remains unchanged
let plot = Plot::new()
    .line(&x_data, &y_data)  // Zero-copy data views
    .title("Performance Test")
    .save("output.png")?;    // Pooled pixel buffers
```

### Pool Configuration
```rust
pub struct PoolConfig {
    pub coordinate_pool_size: usize,    // Default: 1000 f64 elements
    pub pixel_pool_size: usize,         // Default: 4MB pixel buffers
    pub text_pool_size: usize,          // Default: 100KB text buffers
    pub max_pools_per_type: usize,      // Default: 10
    pub enable_cross_thread_sharing: bool, // Default: true
}

impl Plot {
    pub fn with_pool_config(config: PoolConfig) -> Self;
}
```

## Implementation Phases

### Phase 1: Core Pool Infrastructure
1. Implement `MemoryPool<T>` with basic allocation/deallocation
2. Add `PooledBuffer<T>` RAII wrapper for automatic pool return
3. Create thread-local pool registry with type-safe access
4. Implement pool growth/shrink policies based on usage

### Phase 2: Zero-Copy Data Views  
1. Implement `DataView<T>` with `Data1D` trait integration
2. Add `PooledVec<T>` with `Vec<T>`-compatible API
3. Create conversion utilities for existing data sources
4. Ensure lifetime safety with Rust's borrow checker

### Phase 3: Rendering Pipeline Integration
1. Replace coordinate transformation allocations with pooled buffers
2. Integrate pooled pixel buffers with tiny-skia `Pixmap` creation
3. Add cosmic-text buffer pooling for text rendering
4. Update `Plot::save()` to use pooled encoding buffers

### Phase 4: Optimization & Tuning
1. Add performance instrumentation and metrics collection
2. Implement adaptive pool sizing based on usage patterns
3. Optimize hot paths with SIMD-friendly memory layouts
4. Add pool defragmentation for long-running applications

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    #[test] fn test_pool_basic_allocation() { /* ... */ }
    #[test] fn test_pool_reuse_efficiency() { /* ... */ }
    #[test] fn test_zero_copy_data_view() { /* ... */ }
    #[test] fn test_thread_safety() { /* ... */ }
    #[test] fn test_memory_leak_prevention() { /* ... */ }
}
```

### Performance Benchmarks
```rust
#[bench] fn bench_allocation_overhead() { /* Before vs after pooling */ }
#[bench] fn bench_large_plot_memory() { /* 100K+ points */ }
#[bench] fn bench_steady_state_rendering() { /* Memory growth test */ }
#[bench] fn bench_concurrent_rendering() { /* Thread contention */ }
```

### Integration Tests
1. **Existing functionality preservation**: All current tests pass
2. **Memory usage validation**: No memory leaks in continuous rendering
3. **Performance regression tests**: Ensure no slowdowns in small plots
4. **Stress testing**: High-frequency plot generation under memory pressure

### Success Metrics Validation
- Measure allocation time reduction using `criterion` benchmarks  
- Track memory usage with `heaptrack` or similar profiling tools
- Validate steady-state memory with continuous rendering loops
- Ensure API compatibility with existing ruviz examples

## Implementation Files

### New Files
- `src/data/memory_pool.rs` - Core pool implementation
- `src/data/pooled_vec.rs` - Vec-compatible pooled containers
- `src/data/zero_copy.rs` - Zero-copy data view implementations
- `tests/memory_pool_tests.rs` - Comprehensive test suite
- `benches/memory_performance.rs` - Performance benchmarks

### Modified Files
- `src/data/memory.rs` - Integration with existing memory utilities
- `src/core/plot.rs` - Pool-aware coordinate transformation
- `src/render/skia.rs` - Pooled pixmap buffer integration
- `src/render/cosmic_text_renderer.rs` - Pooled text buffer integration

## Risk Mitigation

### Lifetime Management
**Risk**: Complex lifetime interactions with pooled memory
**Mitigation**: Use RAII wrappers and extensive testing with Miri

### Performance Regression
**Risk**: Pool overhead negating performance benefits for small plots  
**Mitigation**: Adaptive pooling that bypasses pools for small allocations

### Thread Safety
**Risk**: Pool contention reducing parallel performance
**Mitigation**: Thread-local pools with work-stealing fallback

### API Compatibility
**Risk**: Breaking existing user code
**Mitigation**: Transparent integration maintaining exact API compatibility

## Future Extensions

This memory pool foundation enables:
1. **SIMD Statistical Functions** - Aligned memory for vectorized operations
2. **GPU Backend Integration** - Buffer mapping for GPU memory
3. **Streaming Data Architecture** - Ring buffer support for real-time data
4. **Parallel Multi-Plot Rendering** - Shared pool coordination

## Expected Impact

**Development Velocity**: Faster iteration on performance features
**User Experience**: Smoother rendering of large datasets
**System Resource Usage**: More efficient memory utilization
**Scalability**: Foundation for high-performance scientific computing