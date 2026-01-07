//! Observable extensions for animation
//!
//! Provides `AnimatedObservable<T>` for smooth value transitions and
//! reactive recording utilities.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use super::easing;
use super::interpolation::Interpolate;
use crate::data::observable::{Observable, SubscriberId};

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
    /// Target value we're animating towards
    target: Arc<RwLock<Option<T>>>,
    /// Starting value of the animation
    start_value: Arc<RwLock<Option<T>>>,
    /// Animation progress (0.0 to 1.0)
    progress: Arc<RwLock<f64>>,
    /// Total animation duration in seconds
    duration_secs: Arc<RwLock<f64>>,
    /// Easing function
    easing_fn: Arc<RwLock<fn(f64) -> f64>>,
    /// Whether an animation is currently in progress
    animating: Arc<AtomicBool>,
}

impl<T: Clone> Clone for AnimatedObservable<T> {
    fn clone(&self) -> Self {
        Self {
            current: self.current.clone(),
            target: Arc::clone(&self.target),
            start_value: Arc::clone(&self.start_value),
            progress: Arc::clone(&self.progress),
            duration_secs: Arc::clone(&self.duration_secs),
            easing_fn: Arc::clone(&self.easing_fn),
            animating: Arc::clone(&self.animating),
        }
    }
}

impl<T: Interpolate + Clone + Send + Sync + 'static> AnimatedObservable<T> {
    /// Create a new animated observable with the given initial value
    pub fn new(value: T) -> Self {
        Self {
            current: Observable::new(value),
            target: Arc::new(RwLock::new(None)),
            start_value: Arc::new(RwLock::new(None)),
            progress: Arc::new(RwLock::new(0.0)),
            duration_secs: Arc::new(RwLock::new(0.0)),
            easing_fn: Arc::new(RwLock::new(easing::ease_in_out_quad)),
            animating: Arc::new(AtomicBool::new(false)),
        }
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
        // Store start value (current value)
        {
            let mut start = self.start_value.write().expect("Lock poisoned");
            *start = Some(self.current.get());
        }

        // Store target
        {
            let mut tgt = self.target.write().expect("Lock poisoned");
            *tgt = Some(target);
        }

        // Reset progress
        {
            let mut prog = self.progress.write().expect("Lock poisoned");
            *prog = 0.0;
        }

        // Set duration
        {
            let mut dur = self.duration_secs.write().expect("Lock poisoned");
            *dur = duration_ms as f64 / 1000.0;
        }

        // Set easing
        {
            let mut ease = self.easing_fn.write().expect("Lock poisoned");
            *ease = easing;
        }

        // Start animation
        self.animating.store(true, Ordering::Release);
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
        if !self.animating.load(Ordering::Acquire) {
            return false;
        }

        let duration = *self.duration_secs.read().expect("Lock poisoned");
        if duration <= 0.0 {
            self.animating.store(false, Ordering::Release);
            return false;
        }

        // Update progress
        let new_progress = {
            let mut prog = self.progress.write().expect("Lock poisoned");
            *prog += delta_time / duration;
            if *prog >= 1.0 {
                *prog = 1.0;
            }
            *prog
        };

        // Get easing function and compute eased progress
        let easing_fn = *self.easing_fn.read().expect("Lock poisoned");
        let eased_t = easing_fn(new_progress);

        // Interpolate value
        let start = self.start_value.read().expect("Lock poisoned");
        let target = self.target.read().expect("Lock poisoned");

        if let (Some(start_val), Some(target_val)) = (&*start, &*target) {
            let interpolated = start_val.interpolate(target_val, eased_t);
            self.current.set(interpolated);
        }

        // Check if complete
        if new_progress >= 1.0 {
            self.animating.store(false, Ordering::Release);
            return false;
        }

        true
    }

    /// Get the current value
    pub fn get(&self) -> T {
        self.current.get()
    }

    /// Set the value immediately without animation
    pub fn set_immediate(&self, value: T) {
        self.stop();
        self.current.set(value);
    }

    /// Check if an animation is currently in progress
    pub fn is_animating(&self) -> bool {
        self.animating.load(Ordering::Acquire)
    }

    /// Stop any current animation
    pub fn stop(&self) {
        self.animating.store(false, Ordering::Release);
    }

    /// Get the current animation progress (0.0 to 1.0)
    pub fn progress(&self) -> f64 {
        *self.progress.read().expect("Lock poisoned")
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
        let mut any_animating = false;
        for anim in &self.animations {
            if anim.tick(delta_time) {
                any_animating = true;
            }
        }
        any_animating
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
    fn test_animated_observable_zero_duration() {
        let anim = AnimatedObservable::new(0.0);

        // Zero duration should jump immediately
        anim.animate_to(100.0, 0);
        anim.tick(0.016);

        // Animation should complete immediately
        assert!(!anim.is_animating());
    }
}
