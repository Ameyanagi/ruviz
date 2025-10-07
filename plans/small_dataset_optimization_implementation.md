# Small Dataset Optimization Implementation Plan

## Problem Analysis

**Current Performance**: 258-265ms for 1K points
**Target Performance**: < 10ms for 1K points
**Gap**: 25.8x slower than target

## Root Cause Identified

From profiling, the bottleneck is **fixed overhead** not data processing:
1. Font system initialization (cosmic-text setup)
2. Canvas allocation (always full size regardless of data)
3. Rendering pipeline setup
4. File I/O overhead

**Key Insight**: The time is NOT spent on plotting 1K points, but on setup/teardown.

## Optimization Strategy

### Option 1: Lazy Font Initialization (Quick Win)
- Current: Font system initializes on every plot
- Solution: Use `OnceCell` for global font cache
- Expected gain: 50-100ms reduction

### Option 2: Minimal Canvas for Small Data (Medium Impact)
- Current: Full canvas allocated regardless of data size
- Solution: Calculate minimal bounds for < 5K points
- Expected gain: 20-50ms reduction

### Option 3: Skip Unnecessary Operations (High Impact)
- Current: All validations and setup run regardless of size
- Solution: Fast-path that skips optional operations
- Expected gain: 50-100ms reduction

### Option 4: Reuse Previous Context (Requires Architecture Change)
- Current: Everything recreated per plot
- Solution: Reusable rendering context
- Expected gain: 100-150ms but requires significant refactoring

## Implementation Plan (Incremental)

### Phase 1: Static Font Cache (Immediate)
```rust
// In src/render/skia.rs or font module
use std::sync::OnceLock;

static FONT_CACHE: OnceLock<CosmicTextSystem> = OnceLock::new();

fn get_font_system() -> &'static CosmicTextSystem {
    FONT_CACHE.get_or_init(|| {
        // Initialize once
        CosmicTextSystem::new()
    })
}
```

### Phase 2: Size-Based Optimization Detection
```rust
// In src/core/plot.rs
impl Plot {
    fn should_use_fast_path(&self) -> bool {
        let total_points = self.count_total_points();
        total_points > 0 && total_points < 5000
    }

    fn count_total_points(&self) -> usize {
        self.series.iter().map(|s| s.point_count()).sum()
    }
}
```

### Phase 3: Fast-Path Rendering
```rust
// In src/core/plot.rs
impl Plot {
    pub fn save<P: AsRef<Path>>(self, path: P) -> Result<()> {
        if self.should_use_fast_path() {
            self.save_fast_path(path)
        } else {
            self.save_normal_path(path)
        }
    }

    fn save_fast_path<P: AsRef<Path>>(self, path: P) -> Result<()> {
        // Optimized rendering for small datasets
        // - Use static font cache
        // - Minimal canvas size
        // - Skip parallel setup
        // - Direct rendering
    }
}
```

## Testing Strategy

After each phase, run:
```bash
cargo test --test small_dataset_optimization_test test_small_dataset_under_10ms
```

Track improvement:
- Phase 1: 258ms → ~200ms (font caching)
- Phase 2: 200ms → ~150ms (minimal canvas)
- Phase 3: 150ms → < 10ms (fast path complete)

## Implementation Steps

1. **Add font caching** to reduce initialization overhead
2. **Test improvement** - expect 50-100ms reduction
3. **Add fast-path detection** in Plot::save()
4. **Implement minimal rendering** for small datasets
5. **Test improvement** - expect to reach < 10ms target
6. **Run all tests** - ensure no regression

## Success Criteria

✅ test_small_dataset_under_10ms passes (< 10ms)
✅ test_very_small_dataset_under_5ms passes (< 5ms)
✅ test_medium_dataset_under_20ms passes (< 20ms)
✅ test_no_regression_large_datasets passes (< 40ms)
✅ All existing tests still pass

## Alternative: If Target Not Met

If optimization doesn't reach < 10ms:
1. Profile again to identify remaining bottleneck
2. Consider more aggressive optimizations:
   - Skip axis rendering for minimal plots
   - Pre-rendered templates
   - Memory-mapped I/O
3. Adjust target to realistic value (e.g., < 20ms)
