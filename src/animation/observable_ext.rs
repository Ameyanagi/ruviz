//! Observable extensions for animation
//!
//! Provides `AnimatedObservable<T>` for smooth value transitions and
//! reactive recording utilities.

use std::sync::{Arc, RwLock};

use super::easing;
use super::interpolation::Interpolate;
use crate::data::observable::{Observable, SubscriberId};

struct AnimationState<T> {
    target: Option<Arc<T>>,
    start_value: Option<Arc<T>>,
    progress: f64,
    duration_secs: f64,
    easing_fn: fn(f64) -> f64,
    animating: bool,
    generation: u64,
}

/// An observable value that can animate smoothly between states
///
/// `AnimatedObservable` wraps any type that implements `Interpolate` and provides
/// smooth transitions when the value changes. Call `tick()` each frame to advance
/// the animation.
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::animation::{AnimatedObservable, easing};
///
/// // Create an animated position
/// let position = AnimatedObservable::new(0.0);
///
/// // Animate to a new value over 500ms
/// position.animate_to(100.0, 500);
///
/// // In your animation loop:
/// while position.is_animating() {
///     position.tick(1.0 / 60.0); // 60 FPS
///     let current = position.get();
///     // Use current value...
/// }
/// ```
pub struct AnimatedObservable<T> {
    /// Current interpolated value
    current: Observable<T>,
    // Animation metadata is updated atomically under this lock. Value installation
    // happens while this state is committed, but subscribers run after its release.
    state: Arc<RwLock<AnimationState<T>>>,
}

impl<T: Clone> Clone for AnimatedObservable<T> {
    fn clone(&self) -> Self {
        Self {
            current: self.current.clone(),
            state: Arc::clone(&self.state),
        }
    }
}

impl<T: Interpolate + Clone + Send + Sync + 'static> AnimatedObservable<T> {
    /// Create a new animated observable with the given initial value
    pub fn new(value: T) -> Self {
        Self {
            current: Observable::new(value),
            state: Arc::new(RwLock::new(AnimationState {
                target: None,
                start_value: None,
                progress: 0.0,
                duration_secs: 0.0,
                easing_fn: easing::ease_in_out_quad,
                animating: false,
                generation: 0,
            })),
        }
    }

    fn install_value<F>(&self, value: T, update_state: F) -> bool
    where
        F: FnOnce(&mut AnimationState<T>) -> bool,
    {
        let mut value = Some(value);
        let changed = self.current.update_if(|current| {
            let mut state = self.state.write().expect("Animation state lock poisoned");
            if !update_state(&mut state) {
                return false;
            }

            let old_value = std::mem::replace(
                current,
                value
                    .take()
                    .expect("Animation value installed more than once"),
            );
            drop(state);
            drop(old_value);
            true
        });
        drop(value);
        changed
    }

    /// Start animating to a new target value
    ///
    /// # Arguments
    ///
    /// * `target` - The value to animate to
    /// * `duration_ms` - Animation duration in milliseconds
    pub fn animate_to(&self, target: T, duration_ms: u64) {
        self.animate_to_with_easing(target, duration_ms, easing::ease_in_out_quad);
    }

    /// Start animating with a custom easing function
    ///
    /// # Arguments
    ///
    /// * `target` - The value to animate to
    /// * `duration_ms` - Animation duration in milliseconds
    /// * `easing` - Easing function (t -> t')
    pub fn animate_to_with_easing(&self, target: T, duration_ms: u64, easing: fn(f64) -> f64) {
        let duration_secs = duration_ms as f64 / 1000.0;

        if duration_secs <= 0.0 {
            self.install_value(target, |state| {
                state.generation = state.generation.wrapping_add(1);
                state.progress = 1.0;
                state.duration_secs = duration_secs;
                state.easing_fn = easing;
                state.animating = false;
                true
            });
            return;
        }

        // Hold the current-value read guard until the new state is installed so a
        // concurrent tick cannot publish between the start snapshot and generation
        // change. `T::clone` still runs before the animation-state guard is acquired.
        let current = self.current.read();
        let start_value = Arc::new(current.clone());
        let target = Arc::new(target);
        let (old_start, old_target) = {
            let mut state = self.state.write().expect("Animation state lock poisoned");
            state.generation = state.generation.wrapping_add(1);
            let old_start = state.start_value.replace(start_value);
            let old_target = state.target.replace(target);
            state.progress = 0.0;
            state.duration_secs = duration_secs;
            state.easing_fn = easing;
            state.animating = true;
            (old_start, old_target)
        };
        drop(current);
        // Replacing the last `Arc` may run user-defined `Drop` code.
        drop((old_start, old_target));
    }

    /// Advance the animation by the given time delta
    ///
    /// # Arguments
    ///
    /// * `delta_time` - Time elapsed since last tick, in seconds
    ///
    /// # Returns
    ///
    /// `true` if the animation is still in progress, `false` if complete
    pub fn tick(&self, delta_time: f64) -> bool {
        let snapshot = {
            let mut state = self.state.write().expect("Animation state lock poisoned");
            if !state.animating {
                return false;
            }

            let (Some(start), Some(target)) = (&state.start_value, &state.target) else {
                return state.animating;
            };

            let start = Arc::clone(start);
            let target = Arc::clone(target);
            let progress = (state.progress + delta_time / state.duration_secs).min(1.0);
            state.generation = state.generation.wrapping_add(1);

            (start, target, state.easing_fn, progress, state.generation)
        };

        let (start, target, easing_fn, progress, generation) = snapshot;
        // Easing and interpolation are user-defined code. Compute before taking
        // any guard that protects animation state or value publication.
        let value = start.interpolate(&target, easing_fn(progress));

        // Avoid entering Observable mutation for operations already superseded
        // while interpolation ran. `install_value` validates again atomically.
        {
            let state = self.state.read().expect("Animation state lock poisoned");
            if state.generation != generation || !state.animating {
                return state.animating;
            }
        }

        self.install_value(value, |state| {
            if state.generation != generation || !state.animating {
                return false;
            }
            state.progress = progress;
            state.animating = progress < 1.0;
            true
        });

        // Re-entrant callbacks may have started or stopped an animation.
        self.is_animating()
    }

    /// Get the current value
    pub fn get(&self) -> T {
        self.current.get()
    }

    /// Set the value immediately without animation
    pub fn set_immediate(&self, value: T) {
        self.install_value(value, |state| {
            state.generation = state.generation.wrapping_add(1);
            state.animating = false;
            true
        });
    }

    /// Check if an animation is currently in progress
    pub fn is_animating(&self) -> bool {
        self.state
            .read()
            .expect("Animation state lock poisoned")
            .animating
    }

    /// Stop any current animation
    pub fn stop(&self) {
        let mut state = self.state.write().expect("Animation state lock poisoned");
        state.generation = state.generation.wrapping_add(1);
        state.animating = false;
    }

    /// Get the current animation progress (0.0 to 1.0)
    pub fn progress(&self) -> f64 {
        self.state
            .read()
            .expect("Animation state lock poisoned")
            .progress
    }

    /// Subscribe to value changes
    pub fn subscribe<F>(&self, callback: F) -> SubscriberId
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.current.subscribe(callback)
    }

    /// Unsubscribe from value changes
    pub fn unsubscribe(&self, id: SubscriberId) -> bool {
        self.current.unsubscribe(id)
    }

    /// Get the underlying observable
    pub fn as_observable(&self) -> &Observable<T> {
        &self.current
    }

    /// Get the current version (for change detection)
    pub fn version(&self) -> u64 {
        self.current.version()
    }
}

impl<T: Interpolate + Clone + Send + Sync + Default + 'static> Default for AnimatedObservable<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

/// A collection of animated observables that can be ticked together
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::animation::{AnimatedObservable, AnimationGroup};
///
/// let x = AnimatedObservable::new(0.0);
/// let y = AnimatedObservable::new(0.0);
///
/// let mut group = AnimationGroup::new();
/// group.add(&x);
/// group.add(&y);
///
/// x.animate_to(100.0, 500);
/// y.animate_to(200.0, 500);
///
/// // Tick all animations at once
/// while group.tick(1.0 / 60.0) {
///     // Animations in progress
/// }
/// ```
pub struct AnimationGroup<'a> {
    animations: Vec<&'a dyn Tickable>,
}

/// Trait for types that can be ticked for animation
pub trait Tickable {
    /// Advance the animation by delta_time seconds
    fn tick(&self, delta_time: f64) -> bool;
    /// Check if still animating
    fn is_animating(&self) -> bool;
}

impl<T: Interpolate + Clone + Send + Sync + 'static> Tickable for AnimatedObservable<T> {
    fn tick(&self, delta_time: f64) -> bool {
        AnimatedObservable::tick(self, delta_time)
    }

    fn is_animating(&self) -> bool {
        AnimatedObservable::is_animating(self)
    }
}

impl<'a> AnimationGroup<'a> {
    /// Create a new empty animation group
    pub fn new() -> Self {
        Self {
            animations: Vec::new(),
        }
    }

    /// Add an animated observable to the group
    pub fn add<T: Interpolate + Clone + Send + Sync + 'static>(
        &mut self,
        animation: &'a AnimatedObservable<T>,
    ) {
        self.animations.push(animation);
    }

    /// Tick all animations in the group
    ///
    /// Returns `true` if any animation is still in progress
    pub fn tick(&self, delta_time: f64) -> bool {
        for anim in &self.animations {
            anim.tick(delta_time);
        }
        // A later member's callback may have started an earlier member after it
        // was ticked, so determine the result from the post-callback group state.
        self.is_animating()
    }

    /// Check if any animation in the group is still animating
    pub fn is_animating(&self) -> bool {
        self.animations.iter().any(|a| a.is_animating())
    }

    /// Get the number of animations in the group
    pub fn len(&self) -> usize {
        self.animations.len()
    }

    /// Check if the group is empty
    pub fn is_empty(&self) -> bool {
        self.animations.is_empty()
    }
}

impl<'a> Default for AnimationGroup<'a> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    #[derive(Clone)]
    struct BlockingValue {
        value: f64,
        block_next_interpolation: Arc<AtomicBool>,
        interpolation_started: Arc<Barrier>,
        continue_interpolation: Arc<Barrier>,
    }

    impl BlockingValue {
        fn new(value: f64) -> Self {
            Self {
                value,
                block_next_interpolation: Arc::new(AtomicBool::new(true)),
                interpolation_started: Arc::new(Barrier::new(2)),
                continue_interpolation: Arc::new(Barrier::new(2)),
            }
        }

        fn with_value(&self, value: f64) -> Self {
            Self {
                value,
                block_next_interpolation: Arc::clone(&self.block_next_interpolation),
                interpolation_started: Arc::clone(&self.interpolation_started),
                continue_interpolation: Arc::clone(&self.continue_interpolation),
            }
        }
    }

    impl Interpolate for BlockingValue {
        fn interpolate(&self, target: &Self, t: f64) -> Self {
            if self.block_next_interpolation.swap(false, Ordering::AcqRel) {
                self.interpolation_started.wait();
                self.continue_interpolation.wait();
            }
            self.with_value(self.value + (target.value - self.value) * t)
        }
    }

    struct CloneCountingValue {
        value: f64,
        clones: Arc<AtomicUsize>,
    }

    impl Clone for CloneCountingValue {
        fn clone(&self) -> Self {
            self.clones.fetch_add(1, Ordering::Relaxed);
            Self {
                value: self.value,
                clones: Arc::clone(&self.clones),
            }
        }
    }

    impl Interpolate for CloneCountingValue {
        fn interpolate(&self, target: &Self, t: f64) -> Self {
            Self {
                value: self.value + (target.value - self.value) * t,
                clones: Arc::clone(&self.clones),
            }
        }
    }

    #[test]
    fn test_animated_observable_new() {
        let anim = AnimatedObservable::new(0.0);
        assert_eq!(anim.get(), 0.0);
        assert!(!anim.is_animating());
    }

    #[test]
    fn test_animated_observable_set_immediate() {
        let anim = AnimatedObservable::new(0.0);
        anim.set_immediate(100.0);
        assert_eq!(anim.get(), 100.0);
        assert!(!anim.is_animating());
    }

    #[test]
    fn test_animated_observable_animate_to() {
        let anim = AnimatedObservable::new(0.0);

        // Start animation (1 second duration)
        anim.animate_to(100.0, 1000);
        assert!(anim.is_animating());
        assert_eq!(anim.get(), 0.0); // Still at start

        // Tick halfway
        anim.tick(0.5);
        let mid_value = anim.get();
        assert!(mid_value > 0.0 && mid_value < 100.0);
        assert!(anim.is_animating());

        // Tick to completion
        anim.tick(0.6);
        assert_eq!(anim.get(), 100.0);
        assert!(!anim.is_animating());
    }

    #[test]
    fn test_animated_observable_linear_easing() {
        let anim = AnimatedObservable::new(0.0);

        // Use linear easing for predictable results
        anim.animate_to_with_easing(100.0, 1000, easing::linear);

        // Tick exactly halfway
        anim.tick(0.5);
        let mid_value = anim.get();
        assert!((mid_value - 50.0_f64).abs() < 0.01);
    }

    #[test]
    fn test_animated_observable_stop() {
        let anim = AnimatedObservable::new(0.0);

        anim.animate_to(100.0, 1000);
        anim.tick(0.5);
        let mid_value = anim.get();

        anim.stop();
        assert!(!anim.is_animating());

        // Value should stay where it was stopped
        assert_eq!(anim.get(), mid_value);
    }

    #[test]
    fn test_animated_observable_vec() {
        let anim = AnimatedObservable::new(vec![0.0, 0.0, 0.0]);

        anim.animate_to_with_easing(vec![100.0, 200.0, 300.0], 1000, easing::linear);

        anim.tick(0.5);
        let mid = anim.get();
        assert!((mid[0] - 50.0_f64).abs() < 0.01);
        assert!((mid[1] - 100.0_f64).abs() < 0.01);
        assert!((mid[2] - 150.0_f64).abs() < 0.01);

        anim.tick(0.6);
        let end = anim.get();
        assert_eq!(end, vec![100.0, 200.0, 300.0]);
    }

    #[test]
    fn test_animated_observable_progress() {
        let anim = AnimatedObservable::new(0.0);

        anim.animate_to(100.0, 1000);
        assert!((anim.progress() - 0.0).abs() < 0.001);

        anim.tick(0.25);
        assert!((anim.progress() - 0.25).abs() < 0.001);

        anim.tick(0.75);
        assert!((anim.progress() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_animated_observable_version_changes() {
        let anim = AnimatedObservable::new(0.0);
        let v0 = anim.version();

        anim.animate_to(100.0, 1000);
        anim.tick(0.5);
        let v1 = anim.version();
        assert!(v1 > v0);

        anim.tick(0.6);
        let v2 = anim.version();
        assert!(v2 > v1);
    }

    #[test]
    fn test_animation_group() {
        let x = AnimatedObservable::new(0.0);
        let y = AnimatedObservable::new(0.0);

        let mut group = AnimationGroup::new();
        group.add(&x);
        group.add(&y);

        assert_eq!(group.len(), 2);
        assert!(!group.is_animating());

        x.animate_to(100.0, 1000);
        y.animate_to(200.0, 500);

        assert!(group.is_animating());

        // Tick group
        assert!(group.tick(0.3));
        assert!(x.is_animating());
        assert!(y.is_animating());

        // Y should finish first (500ms)
        group.tick(0.3);
        assert!(x.is_animating());
        assert!(!y.is_animating());
        assert_eq!(y.get(), 200.0);

        // Tick until X finishes
        group.tick(0.5);
        assert!(!x.is_animating());
        assert!(!group.is_animating());
    }

    #[test]
    fn test_animated_observable_subscribe() {
        let anim = AnimatedObservable::new(0.0);
        let change_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count_clone = std::sync::Arc::clone(&change_count);

        anim.subscribe(move || {
            count_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        });

        anim.animate_to(100.0, 1000);
        anim.tick(0.5);
        anim.tick(0.6);

        // Should have been notified twice (once per tick that updates value)
        assert!(change_count.load(std::sync::atomic::Ordering::Relaxed) >= 2);
    }

    #[test]
    fn test_animated_observable_zero_duration_installs_target_once() {
        let anim = AnimatedObservable::new(0.0);
        let change_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count_clone = std::sync::Arc::clone(&change_count);
        anim.subscribe(move || {
            count_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        });

        anim.animate_to(100.0, 0);

        assert_eq!(anim.get(), 100.0);
        assert_eq!(anim.progress(), 1.0);
        assert!(!anim.is_animating());
        assert_eq!(change_count.load(std::sync::atomic::Ordering::Relaxed), 1);
        assert!(!anim.tick(0.016));
        assert_eq!(change_count.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    #[test]
    fn test_zero_duration_subscriber_observes_completion() {
        let anim = AnimatedObservable::new(0.0);
        let observed = Arc::new(std::sync::Mutex::new(None));
        let callback_anim = anim.clone();
        let callback_observed = Arc::clone(&observed);
        anim.subscribe(move || {
            *callback_observed.lock().expect("observation lock poisoned") =
                Some((callback_anim.get(), callback_anim.is_animating()));
        });

        anim.animate_to(100.0, 0);
        assert_eq!(
            *observed.lock().expect("observation lock poisoned"),
            Some((100.0, false))
        );
    }

    #[test]
    fn test_final_tick_preserves_animation_started_by_subscriber() {
        let anim = AnimatedObservable::new(0.0);
        let callback_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let callback_count_clone = std::sync::Arc::clone(&callback_count);
        let callback_anim = anim.clone();
        anim.subscribe(move || {
            if callback_count_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed) == 0 {
                callback_anim.animate_to_with_easing(200.0, 1000, easing::linear);
            }
        });

        anim.animate_to_with_easing(100.0, 1000, easing::linear);
        assert!(anim.tick(1.0));

        assert_eq!(anim.get(), 100.0);
        assert_eq!(anim.progress(), 0.0);
        assert!(anim.is_animating());

        assert!(anim.tick(0.5));
        assert_eq!(anim.get(), 150.0);
    }

    #[test]
    fn test_stale_tick_cannot_overwrite_concurrent_immediate_value() {
        let initial = BlockingValue::new(0.0);
        let anim = AnimatedObservable::new(initial.clone());
        anim.animate_to_with_easing(initial.with_value(100.0), 1000, easing::linear);

        let tick_anim = anim.clone();
        let tick = std::thread::spawn(move || tick_anim.tick(0.5));
        initial.interpolation_started.wait();

        anim.set_immediate(initial.with_value(500.0));
        initial.continue_interpolation.wait();

        assert!(!tick.join().expect("tick thread panicked"));
        assert_eq!(anim.get().value, 500.0);
        assert!(!anim.is_animating());
    }

    #[test]
    fn test_stale_tick_cannot_overwrite_concurrent_animation() {
        let initial = BlockingValue::new(0.0);
        let anim = AnimatedObservable::new(initial.clone());
        anim.animate_to_with_easing(initial.with_value(100.0), 1000, easing::linear);

        let tick_anim = anim.clone();
        let tick = std::thread::spawn(move || tick_anim.tick(0.5));
        initial.interpolation_started.wait();

        anim.animate_to_with_easing(initial.with_value(200.0), 1000, easing::linear);
        initial.continue_interpolation.wait();

        assert!(tick.join().expect("tick thread panicked"));
        assert_eq!(anim.get().value, 0.0);
        assert_eq!(anim.progress(), 0.0);
        assert!(anim.tick(0.5));
        assert_eq!(anim.get().value, 100.0);
    }

    #[test]
    fn test_final_completion_is_not_visible_before_value_installation() {
        let initial = BlockingValue::new(0.0);
        let anim = AnimatedObservable::new(initial.clone());
        anim.animate_to_with_easing(initial.with_value(100.0), 1000, easing::linear);

        let tick_anim = anim.clone();
        let tick = std::thread::spawn(move || tick_anim.tick(1.0));
        initial.interpolation_started.wait();

        assert!(anim.is_animating());
        assert_eq!(anim.get().value, 0.0);

        initial.continue_interpolation.wait();
        assert!(!tick.join().expect("tick thread panicked"));
        assert_eq!(anim.get().value, 100.0);
        assert!(!anim.is_animating());
    }

    #[test]
    fn test_final_subscriber_observes_installed_value_and_completion() {
        let anim = AnimatedObservable::new(0.0);
        let observed = Arc::new(std::sync::Mutex::new(None));
        let callback_anim = anim.clone();
        let callback_observed = Arc::clone(&observed);
        anim.subscribe(move || {
            *callback_observed.lock().expect("observation lock poisoned") =
                Some((callback_anim.get(), callback_anim.is_animating()));
        });

        anim.animate_to_with_easing(100.0, 1000, easing::linear);
        assert!(!anim.tick(1.0));
        assert_eq!(
            *observed.lock().expect("observation lock poisoned"),
            Some((100.0, false))
        );
    }

    #[test]
    fn test_tick_does_not_clone_animation_endpoints() {
        let clones = Arc::new(AtomicUsize::new(0));
        let anim = AnimatedObservable::new(CloneCountingValue {
            value: 0.0,
            clones: Arc::clone(&clones),
        });
        anim.animate_to_with_easing(
            CloneCountingValue {
                value: 100.0,
                clones: Arc::clone(&clones),
            },
            1000,
            easing::linear,
        );
        clones.store(0, Ordering::Relaxed);

        assert!(anim.tick(0.5));
        assert_eq!(clones.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_final_tick_preserves_immediate_value_set_by_subscriber() {
        let anim = AnimatedObservable::new(0.0);
        let callback_count = Arc::new(AtomicUsize::new(0));
        let callback_anim = anim.clone();
        let callback_count_clone = Arc::clone(&callback_count);
        anim.subscribe(move || {
            if callback_count_clone.fetch_add(1, Ordering::Relaxed) == 0 {
                callback_anim.set_immediate(250.0);
            }
        });

        anim.animate_to_with_easing(100.0, 1000, easing::linear);
        assert!(!anim.tick(1.0));
        assert_eq!(anim.get(), 250.0);
        assert!(!anim.is_animating());
    }

    #[test]
    fn test_final_tick_allows_subscriber_to_stop() {
        let anim = AnimatedObservable::new(0.0);
        let callback_anim = anim.clone();
        anim.subscribe(move || callback_anim.stop());

        anim.animate_to_with_easing(100.0, 1000, easing::linear);
        assert!(!anim.tick(1.0));
        assert_eq!(anim.get(), 100.0);
        assert!(!anim.is_animating());
    }

    #[test]
    fn test_subscriber_can_recursively_tick() {
        let anim = AnimatedObservable::new(0.0);
        let callback_count = Arc::new(AtomicUsize::new(0));
        let callback_anim = anim.clone();
        let callback_count_clone = Arc::clone(&callback_count);
        anim.subscribe(move || {
            if callback_count_clone.fetch_add(1, Ordering::Relaxed) == 0 {
                callback_anim.tick(0.5);
            }
        });

        anim.animate_to_with_easing(100.0, 1000, easing::linear);
        assert!(!anim.tick(0.5));
        assert_eq!(anim.get(), 100.0);
        assert!(!anim.is_animating());
    }

    #[test]
    fn test_zero_duration_preserves_animation_started_by_subscriber() {
        let anim = AnimatedObservable::new(0.0);
        let callback_count = Arc::new(AtomicUsize::new(0));
        let callback_anim = anim.clone();
        let callback_count_clone = Arc::clone(&callback_count);
        anim.subscribe(move || {
            if callback_count_clone.fetch_add(1, Ordering::Relaxed) == 0 {
                callback_anim.animate_to_with_easing(200.0, 1000, easing::linear);
            }
        });

        anim.animate_to_with_easing(100.0, 0, easing::linear);
        assert_eq!(anim.get(), 100.0);
        assert_eq!(anim.progress(), 0.0);
        assert!(anim.is_animating());
        assert!(anim.tick(0.5));
        assert_eq!(anim.get(), 150.0);
    }

    #[test]
    fn test_animation_group_reports_animation_started_in_later_callback() {
        let earlier = AnimatedObservable::new(0.0);
        let later = AnimatedObservable::new(0.0);
        let callback_count = Arc::new(AtomicUsize::new(0));
        let callback_earlier = earlier.clone();
        let callback_count_clone = Arc::clone(&callback_count);
        later.subscribe(move || {
            if callback_count_clone.fetch_add(1, Ordering::Relaxed) == 0 {
                callback_earlier.animate_to_with_easing(100.0, 1000, easing::linear);
            }
        });

        let mut group = AnimationGroup::new();
        group.add(&earlier);
        group.add(&later);
        later.animate_to_with_easing(50.0, 1000, easing::linear);

        assert!(group.tick(1.0));
        assert!(earlier.is_animating());
        assert!(!later.is_animating());
    }
}
