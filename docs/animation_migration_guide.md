# Animation API Migration Guide

This guide helps you migrate from the verbose animation API to the simplified API introduced in v0.9.

## Quick Reference

| Task | Original API | Simplified API |
|------|-------------|----------------|
| Basic recording | `record(path, 0..60, \|frame, tick\| ...)` | `record_simple(path, 60, \|tick\| ...)` |
| Duration-based | `record_duration(path, 2.0, 30, \|tick\| ...)` | `record_simple(path, 2.0.secs(), \|tick\| ...)` |
| Multi-value | `AnimatedObservable` + `AnimationGroup` | `Animation::build().value(...)` |
| Time interpolation | Manual: `from + (to - from) * (tick.time / duration)` | `tick.lerp_over(from, to, duration)` |
| Eased values | Manual calculation | `tick.ease_over(easing::ease_out_bounce, from, to, duration)` |

## Migration Examples

### 1. Basic Frame Recording

**Before:**
```rust
use ruviz::animation::record;

record("out.gif", 0..60, |_frame, tick| {
    let x = tick.time;
    #[allow(deprecated)]
    Plot::new()
        .line(&[0.0, x], &[0.0, x])
        .end_series()
})?;
```

**After:**
```rust
use ruviz::animation::record_simple;

record_simple("out.gif", 60, |tick| {
    let x = tick.time;
    Plot::new()
        .line(&[0.0, x], &[0.0, x])
})?;
```

**Changes:**
- `record` → `record_simple`
- `0..60` (range) → `60` (count)
- `|_frame, tick|` → `|tick|` (no frame index needed)
- No more `#[allow(deprecated)]` or `.end_series()`

### 2. Duration-Based Recording

**Before:**
```rust
use ruviz::animation::record_duration;

record_duration("out.gif", 2.0, 30, |tick| {
    // ...
})?;
```

**After:**
```rust
use ruviz::animation::{record_simple, DurationExt};

record_simple("out.gif", 2.0.secs(), |tick| {
    // ...
})?;
```

**Changes:**
- Use `DurationExt` trait for `.secs()` syntax
- Single function handles both frame counts and durations

### 3. Value Interpolation

**Before:**
```rust
let duration = 2.0;
let progress = (tick.time / duration).clamp(0.0, 1.0);
let x = 0.0 + (100.0 - 0.0) * progress;  // Linear interpolation
```

**After:**
```rust
let x = tick.lerp_over(0.0, 100.0, 2.0);  // Same result, one line
```

### 4. Eased Interpolation

**Before:**
```rust
use ruviz::animation::easing;

let duration = 2.0;
let progress = (tick.time / duration).clamp(0.0, 1.0);
let eased = easing::ease_out_bounce(progress);
let y = 100.0 + (0.0 - 100.0) * eased;  // Bouncing from 100 to 0
```

**After:**
```rust
use ruviz::animation::easing;

let y = tick.ease_over(easing::ease_out_bounce, 100.0, 0.0, 2.0);
```

### 5. Multi-Value Animations

**Before (AnimatedObservable):**
```rust
use ruviz::animation::{AnimatedObservable, AnimationGroup, record_animated, easing};

// Create observables
let x = AnimatedObservable::new(0.0);
let y = AnimatedObservable::new(0.0);

// Clone for closure
let x_ref = x.clone();
let y_ref = y.clone();

// Create group
let mut group = AnimationGroup::new();
group.add(&x);
group.add(&y);

// Start animations
x.animate_to_with_easing(100.0, 2000, easing::ease_out_elastic);
y.animate_to_with_easing(50.0, 1500, easing::ease_in_out_quad);

// Record
record_animated("out.gif", &group, 120, |tick| {
    let x_val = x_ref.get();
    let y_val = y_ref.get();
    Plot::new().scatter(&[x_val], &[y_val])
})?;
```

**After (Animation Builder):**
```rust
use ruviz::animation::{Animation, easing};

Animation::build()
    .value("x", 0.0).to(100.0).ease(easing::ease_out_elastic).duration_secs(2.0)
    .value("y", 0.0).to(50.0).ease(easing::ease_in_out_quad).duration_secs(1.5)
    .record("out.gif", |values, tick| {
        Plot::new().scatter(&[values["x"]], &[values["y"]])
    })?;
```

**Changes:**
- 18 lines → 6 lines
- No manual observable management
- Declarative value definitions
- Auto-duration calculation

## Tick Helper Methods

New methods on `Tick` for easier time-based calculations:

| Method | Description | Example |
|--------|-------------|---------|
| `progress(start, end)` | Get 0.0-1.0 progress | `tick.progress(0.0, 2.0)` |
| `lerp(from, to, start, end)` | Linear interpolation | `tick.lerp(0.0, 100.0, 0.0, 2.0)` |
| `lerp_over(from, to, duration)` | Lerp from t=0 | `tick.lerp_over(0.0, 100.0, 2.0)` |
| `ease(fn, from, to, start, end)` | Eased interpolation | `tick.ease(ease_out_quad, 0.0, 100.0, 0.0, 2.0)` |
| `ease_over(fn, from, to, duration)` | Eased from t=0 | `tick.ease_over(ease_out_bounce, 0.0, 100.0, 2.0)` |

## When to Use Each API

### Use Simplified API (`record_simple`, `Animation::build`) when:
- Creating standard animations with known durations
- Animating multiple values with different easings
- Wanting minimal boilerplate

### Use Original API (`record`, `AnimatedObservable`) when:
- Need frame-by-frame control
- Animations change dynamically during recording
- Complex conditional animation logic
- Need to query animation state externally

## Backward Compatibility

The original API remains fully functional. All existing code will continue to work. The simplified API is additive - use whichever fits your needs.

## PlotBuilder Auto-Conversion

All recording functions now accept `impl Into<Plot>`, so you can return `PlotBuilder` directly without calling `.end_series()`:

```rust
// Both work now:
record_simple("out.gif", 60, |tick| {
    Plot::new().line(&x, &y)  // Returns PlotBuilder, auto-converts
})?;

record_simple("out.gif", 60, |tick| {
    Plot::new().line(&x, &y).into()  // Explicit conversion also works
})?;
```
