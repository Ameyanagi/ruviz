# Animation API Improvement Suggestions

## Comparison: Makie.jl vs ruviz

### Makie.jl Approach (Reference)

Makie.jl's animation system is elegant due to its **Observable-first design**:

```julia
# 1. Create a time observable
time = Observable(0.0)

# 2. Derive data reactively using @lift - auto-updates when time changes
xs = range(0, 7, length=40)
ys = @lift(sin.(xs .- $time))

# 3. Create figure ONCE with reactive data
fig = lines(xs, ys, color=:blue)

# 4. Record by simply updating the observable
record(fig, "wave.mp4", 0:0.1:10; framerate=30) do t
    time[] = t  # Just set time, everything updates automatically
end
```

**Key strengths:**
- Single `record()` function
- Figure created once, not recreated each frame
- `@lift` macro creates derived observables automatically
- Minimal boilerplate

---

## Current ruviz API Pain Points

### Problem 1: Too Many `record*()` Variants

We have **6 functions** for recording:
```rust
record()
record_with_config()
record_duration()
record_duration_with_config()
record_animated()
record_animated_with_config()
```

This creates confusion about which to use.

### Problem 2: Plot Recreation Every Frame

```rust
// Current: rebuild entire plot each frame
record("out.gif", 0..60, |_frame, tick| {
    let y: Vec<f64> = x.iter().map(|xi| (xi + tick.time).sin()).collect();
    Plot::new()
        .line(&x, &y)
        .end_series()  // Create new plot every frame!
})?;
```

Unlike Makie which creates the figure once and updates data via Observables.

### Problem 3: AnimatedObservable Boilerplate

```rust
// Current: Too much setup for simple animations
let x_pos = AnimatedObservable::new(0.0);
let x_ref = x_pos.clone();  // Must clone for closure
let mut group = AnimationGroup::new();  // Must create group
group.add(&x_pos);  // Must add to group
x_pos.animate_to_with_easing(8.0, 2000, ease_out_elastic);

record_animated_with_config("out.gif", &group, 120, config, |tick| {
    let x = x_ref.get();  // Must call .get() explicitly
    Plot::new()...
})?;
```

### Problem 4: No Time-Based Interpolation Helpers

Users must manually compute lerp/easing:
```rust
// Current: manual computation
let progress = tick.time / duration;
let eased = ease_out_quad(progress);
let x = start + (end - start) * eased;
```

---

## Proposed Improvements

### Improvement 1: Unified `record()` with Duration Support

**Current:**
```rust
record("out.gif", 0..60, |frame, tick| {...})?;
record_duration("out.gif", 2.0, 30, |tick| {...})?;
```

**Proposed:**
```rust
// Frame-based (existing)
record("out.gif", 60, |t| {...})?;

// Duration-based (new overload via trait)
record("out.gif", Duration::secs(2), |t| {...})?;
// or
record("out.gif", 2.0.secs(), |t| {...})?;
```

**Implementation approach:**
```rust
pub trait IntoFrameCount {
    fn into_frame_count(self, framerate: u32) -> usize;
}

impl IntoFrameCount for usize {
    fn into_frame_count(self, _: u32) -> usize { self }
}

impl IntoFrameCount for std::time::Duration {
    fn into_frame_count(self, framerate: u32) -> usize {
        (self.as_secs_f64() * framerate as f64).ceil() as usize
    }
}

// Single record function
pub fn record<P, D, F>(path: P, duration: D, frame_fn: F) -> Result<()>
where
    P: AsRef<Path>,
    D: IntoFrameCount,
    F: FnMut(&Tick) -> Plot,
```

### Improvement 2: Time Interpolation Helpers on Tick

**Proposed API:**
```rust
record("out.gif", 60, |t| {
    // Linear interpolation over 2 seconds
    let x = t.lerp_over(0.0, 100.0, 2.0);

    // With easing
    let y = t.ease_over(ease_out_elastic, 0.0, 50.0, 1.5);

    // Staggered animations (start at t=0.5, end at t=2.0)
    let scale = t.ease(ease_out_bounce, 1.0, 2.0, 0.5, 2.0);

    Plot::new().scatter(&[x], &[y])...
})?;
```

**Implementation (already added to tick.rs):**
```rust
impl Tick {
    pub fn progress(&self, start: f64, end: f64) -> f64;
    pub fn lerp(&self, from: f64, to: f64, start_time: f64, end_time: f64) -> f64;
    pub fn ease(&self, easing: fn(f64)->f64, from: f64, to: f64, start: f64, end: f64) -> f64;
    pub fn lerp_over(&self, from: f64, to: f64, duration: f64) -> f64;
    pub fn ease_over(&self, easing: fn(f64)->f64, from: f64, to: f64, duration: f64) -> f64;
}
```

### Improvement 3: Simplified AnimatedObservable with Auto-Grouping

**Current (verbose):**
```rust
let x = AnimatedObservable::new(0.0);
let y = AnimatedObservable::new(0.0);
let x_ref = x.clone();
let y_ref = y.clone();
let mut group = AnimationGroup::new();
group.add(&x);
group.add(&y);
x.animate_to(10.0, 1000);
y.animate_to(5.0, 500);

record_animated("out.gif", &group, 120, |tick| {
    Plot::new().scatter(&[x_ref.get()], &[y_ref.get()])...
})?;
```

**Proposed Option A: Animation Builder Pattern**
```rust
let anim = Animation::build()
    .animate("x", 0.0, 10.0, 1.0.secs(), ease_out_elastic)
    .animate("y", 0.0, 5.0, 0.5.secs(), ease_out_bounce)
    .framerate(30);

anim.record("out.gif", |values, t| {
    let x = values["x"];
    let y = values["y"];
    Plot::new().scatter(&[x], &[y])...
})?;
```

**Proposed Option B: Macro-based (Makie-style)**
```rust
let x = Animated::new(0.0);
let y = Animated::new(0.0);

x.to(10.0).over(1.0.secs()).ease(ease_out_elastic);
y.to(5.0).over(0.5.secs());

// Auto-discover animated values in closure
record_reactive("out.gif", |t| {
    Plot::new().scatter(&[x.get()], &[y.get()])...
})?;
```

**Proposed Option C: Tuple-based (minimal boilerplate)**
```rust
record_animated("out.gif",
    // Animations as tuple: (name, start, end, duration_ms, easing)
    [
        ("x", 0.0, 10.0, 1000, ease_out_elastic),
        ("y", 0.0, 5.0, 500, ease_out_bounce),
    ],
    |values, t| {
        Plot::new().scatter(&[values.x], &[values.y])...
    }
)?;
```

### Improvement 4: Makie-Style Reactive Figures (Long-term)

This would require architectural changes but would be the most elegant:

```rust
// Create time observable
let time = Observable::new(0.0);

// Derive data reactively (like @lift in Makie)
let y_data = lift(&time, |&t| {
    x.iter().map(|xi| (xi - t).sin()).collect::<Vec<_>>()
});

// Create figure ONCE with reactive data
let fig = Figure::new()
    .line(&x, &y_data)  // y_data is Observable<Vec<f64>>
    .build();

// Record by updating the time observable
fig.record("out.gif", 0..100, |frame| {
    time.set(frame as f64 * 0.1);  // Figure auto-updates
})?;
```

**Benefits:**
- Figure created once, not per-frame
- Data derivation is declarative
- Matches Makie's mental model
- Better performance (only re-render changed elements)

**Challenges:**
- Requires Plot to accept Observable data sources
- Need change detection in rendering pipeline
- Significant refactoring

---

## Priority Recommendations

### High Priority (Low effort, High impact)

1. **Add Tick interpolation helpers** - Already implemented
   - `t.lerp_over()`, `t.ease_over()`, `t.progress()`
   - Eliminates 80% of manual animation math

2. **Simplify closure signature** - Just pass time `f64`
   ```rust
   // Instead of |_frame, tick| use tick.time
   record("out.gif", 60, |t: f64| {...})?;
   ```

### Medium Priority (Medium effort)

3. **Unify record functions** with `IntoFrameCount` trait
   - Single entry point for frames or duration

4. **Animation builder** for multi-value animations
   - Replaces AnimationGroup boilerplate

### Low Priority (High effort, Future consideration)

5. **Reactive Figure** with Observable data sources
   - Major architectural change
   - Best long-term solution

---

## Example: Before and After

### Before (Current API)
```rust
use ruviz::animation::{
    AnimatedObservable, AnimationGroup, RecordConfig, Quality,
    easing, record_animated_with_config
};

let x_pos = AnimatedObservable::new(0.0_f64);
let y_pos = AnimatedObservable::new(0.0_f64);
let x_ref = x_pos.clone();
let y_ref = y_pos.clone();

let mut group = AnimationGroup::new();
group.add(&x_pos);
group.add(&y_pos);

x_pos.animate_to_with_easing(8.0, 2000, easing::ease_out_elastic);
y_pos.animate_to_with_easing(6.0, 1500, easing::ease_in_out_cubic);

let config = RecordConfig::new()
    .dimensions(800, 600)
    .framerate(30)
    .quality(Quality::Medium);

record_animated_with_config("out.gif", &group, 120, config, |tick| {
    let x = x_ref.get();
    let y = y_ref.get();
    Plot::new().scatter(&[x], &[y]).end_series()
})?;
```
**Lines of setup: 18**

### After (Proposed Simplified API)
```rust
use ruviz::animation::{record, easing};

record("out.gif", 2.0.secs(), |t| {
    let x = t.ease_over(easing::ease_out_elastic, 0.0, 8.0, 2.0);
    let y = t.ease_over(easing::ease_in_out_cubic, 0.0, 6.0, 1.5);
    Plot::new().scatter(&[x], &[y])
})?;
```
**Lines of setup: 6** (67% reduction)

---

## References

- [Makie.jl Animation Documentation](https://docs.makie.org/dev/explanations/animation)
- [Makie Observable Pattern](https://docs.makie.org/dev/explanations/observables)
