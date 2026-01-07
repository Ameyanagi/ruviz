//! Signal type for pull-based reactive animation values
//!
//! `Signal<T>` represents a time-varying value that can be evaluated at any point
//! in time. Unlike `Observable<T>` which is push-based (notifies subscribers on change),
//! `Signal` is pull-based (evaluated on demand at a specific time).
//!
//! # When to use Signal vs Observable
//!
//! - **Signal**: Best for animation recording where you control time progression.
//!   Values are computed lazily when `at(time)` is called.
//!
//! - **Observable**: Best for interactive/live updates where values change asynchronously
//!   and you need to notify listeners.
//!
//! # Examples
//!
//! ```rust,ignore
//! use ruviz::animation::signal;
//!
//! // Linear interpolation from 0 to 100 over 2 seconds
//! let x = signal::lerp(0.0, 100.0, 2.0);
//! assert_eq!(x.at(0.0), 0.0);
//! assert_eq!(x.at(1.0), 50.0);
//! assert_eq!(x.at(2.0), 100.0);
//!
//! // Custom signal function
//! let y = signal::of(|t| (t * std::f64::consts::PI).sin());
//! assert!((y.at(0.5) - 1.0).abs() < 0.01);
//!
//! // Signal composition
//! let doubled = x.map(|v| v * 2.0);
//! assert_eq!(doubled.at(1.0), 100.0);
//! ```

use std::sync::Arc;

/// A time-varying value that can be evaluated at any point in time.
///
/// `Signal<T>` wraps a function `f64 -> T` that computes a value for any given time.
/// Signals are immutable and deterministic - evaluating at the same time always
/// returns the same value.
///
/// # Type Parameters
///
/// * `T` - The type of value produced by the signal
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::animation::signal;
///
/// let phase = signal::lerp(0.0, 2.0 * std::f64::consts::PI, 1.0);
/// let y = signal::of(move |t| (phase.at(t)).sin());
/// ```
pub struct Signal<T> {
    /// The evaluation function (Arc for thread-safety)
    eval: Arc<dyn Fn(f64) -> T + Send + Sync>,
}

impl<T> Signal<T> {
    /// Create a new signal from a function.
    ///
    /// # Arguments
    ///
    /// * `f` - A function that takes time (f64) and returns a value of type T
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let sig = Signal::new(|t| t * 2.0);
    /// assert_eq!(sig.at(1.5), 3.0);
    /// ```
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(f64) -> T + Send + Sync + 'static,
    {
        Signal { eval: Arc::new(f) }
    }

    /// Evaluate the signal at a specific time.
    ///
    /// # Arguments
    ///
    /// * `time` - The time at which to evaluate (in seconds)
    ///
    /// # Returns
    ///
    /// The value of the signal at the given time
    #[inline]
    pub fn at(&self, time: f64) -> T {
        (self.eval)(time)
    }

    /// Transform the signal's output using a mapping function.
    ///
    /// # Arguments
    ///
    /// * `f` - A function to apply to the signal's output
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let x = signal::lerp(0.0, 10.0, 1.0);
    /// let x_squared = x.map(|v| v * v);
    /// assert_eq!(x_squared.at(0.5), 25.0); // 5^2 = 25
    /// ```
    pub fn map<U, F>(self, f: F) -> Signal<U>
    where
        F: Fn(T) -> U + Send + Sync + 'static,
        T: Send + 'static,
    {
        let eval = self.eval;
        Signal::new(move |t| f(eval(t)))
    }
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Signal {
            eval: self.eval.clone(),
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Signal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Signal")
            .field("eval", &"<function>")
            .finish()
    }
}

// ============================================================================
// Signal constructors (module-level functions)
// ============================================================================

/// Create a signal from a custom function.
///
/// This is the most flexible way to create a signal - you provide a function
/// that computes the value for any given time.
///
/// # Arguments
///
/// * `f` - A function `f64 -> T` that computes the value at any time
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::animation::signal;
///
/// // Sine wave signal
/// let wave = signal::of(|t| (t * 2.0 * std::f64::consts::PI).sin());
///
/// // Data that varies with time
/// let data = signal::of(|t| {
///     (0..100).map(|i| {
///         let x = i as f64 * 0.1;
///         (x + t).sin()
///     }).collect::<Vec<f64>>()
/// });
/// ```
pub fn of<T, F>(f: F) -> Signal<T>
where
    F: Fn(f64) -> T + Send + Sync + 'static,
{
    Signal::new(f)
}

/// Create a constant signal that always returns the same value.
///
/// # Arguments
///
/// * `value` - The constant value to return
///
/// # Example
///
/// ```rust,ignore
/// let c = signal::constant(42.0);
/// assert_eq!(c.at(0.0), 42.0);
/// assert_eq!(c.at(100.0), 42.0);
/// ```
pub fn constant<T: Clone + Send + Sync + 'static>(value: T) -> Signal<T> {
    Signal::new(move |_| value.clone())
}

/// Create an identity signal that returns the current time.
///
/// # Example
///
/// ```rust,ignore
/// let t = signal::time();
/// assert_eq!(t.at(1.5), 1.5);
/// assert_eq!(t.at(3.0), 3.0);
/// ```
pub fn time() -> Signal<f64> {
    Signal::new(|t| t)
}

/// Create a linear interpolation signal.
///
/// The signal linearly interpolates from `from` to `to` over the given `duration`.
/// Before t=0, returns `from`. After t=duration, returns `to`.
///
/// # Arguments
///
/// * `from` - Starting value at t=0
/// * `to` - Ending value at t=duration
/// * `duration` - Duration of the interpolation in seconds
///
/// # Example
///
/// ```rust,ignore
/// let x = signal::lerp(0.0, 100.0, 2.0);
/// assert_eq!(x.at(0.0), 0.0);
/// assert_eq!(x.at(1.0), 50.0);
/// assert_eq!(x.at(2.0), 100.0);
/// assert_eq!(x.at(3.0), 100.0); // Clamped at end
/// ```
pub fn lerp(from: f64, to: f64, duration: f64) -> Signal<f64> {
    Signal::new(move |t| {
        let progress = (t / duration).clamp(0.0, 1.0);
        from + (to - from) * progress
    })
}

/// Create an eased interpolation signal using an easing function.
///
/// The signal interpolates from `from` to `to` over the given `duration`,
/// applying the easing function to the progress.
///
/// # Arguments
///
/// * `easing_fn` - An easing function `f64 -> f64` (from the `easing` module)
/// * `from` - Starting value at t=0
/// * `to` - Ending value at t=duration
/// * `duration` - Duration of the interpolation in seconds
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::animation::{signal, easing};
///
/// let bounce = signal::ease(easing::ease_out_bounce, 100.0, 0.0, 2.0);
/// // Value bounces from 100 down to 0 over 2 seconds
/// ```
pub fn ease<F>(easing_fn: F, from: f64, to: f64, duration: f64) -> Signal<f64>
where
    F: Fn(f64) -> f64 + Send + Sync + 'static,
{
    Signal::new(move |t| {
        let progress = (t / duration).clamp(0.0, 1.0);
        let eased = easing_fn(progress);
        from + (to - from) * eased
    })
}

/// Combine two signals using a function.
///
/// # Arguments
///
/// * `a` - First signal
/// * `b` - Second signal
/// * `f` - Combiner function
///
/// # Example
///
/// ```rust,ignore
/// let angle = signal::lerp(0.0, 2.0 * std::f64::consts::PI, 1.0);
/// let radius = signal::lerp(0.0, 100.0, 1.0);
///
/// let x = signal::zip(angle.clone(), radius.clone(), |a, r| r * a.cos());
/// let y = signal::zip(angle, radius, |a, r| r * a.sin());
/// ```
pub fn zip<A, B, C, F>(a: Signal<A>, b: Signal<B>, f: F) -> Signal<C>
where
    A: Send + 'static,
    B: Send + 'static,
    F: Fn(A, B) -> C + Send + Sync + 'static,
{
    Signal::new(move |t| f(a.at(t), b.at(t)))
}

/// Combine three signals using a function.
///
/// # Arguments
///
/// * `a` - First signal
/// * `b` - Second signal
/// * `c` - Third signal
/// * `f` - Combiner function
pub fn zip3<A, B, C, D, F>(a: Signal<A>, b: Signal<B>, c: Signal<C>, f: F) -> Signal<D>
where
    A: Send + 'static,
    B: Send + 'static,
    C: Send + 'static,
    F: Fn(A, B, C) -> D + Send + Sync + 'static,
{
    Signal::new(move |t| f(a.at(t), b.at(t), c.at(t)))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_new_and_at() {
        let sig = Signal::new(|t| t * 2.0);
        assert_eq!(sig.at(0.0), 0.0);
        assert_eq!(sig.at(1.5), 3.0);
        assert_eq!(sig.at(5.0), 10.0);
    }

    #[test]
    fn test_signal_clone() {
        let sig = Signal::new(|t| t * 3.0);
        let cloned = sig.clone();
        assert_eq!(sig.at(2.0), cloned.at(2.0));
    }

    #[test]
    fn test_signal_map() {
        let sig = Signal::new(|t| t);
        let doubled = sig.map(|v| v * 2.0);
        assert_eq!(doubled.at(5.0), 10.0);
    }

    #[test]
    fn test_of() {
        let sig = of(|t| t.powi(2));
        assert_eq!(sig.at(3.0), 9.0);
    }

    #[test]
    fn test_constant() {
        let sig = constant(42.0);
        assert_eq!(sig.at(0.0), 42.0);
        assert_eq!(sig.at(100.0), 42.0);
    }

    #[test]
    fn test_time() {
        let sig = time();
        assert_eq!(sig.at(1.5), 1.5);
        assert_eq!(sig.at(99.0), 99.0);
    }

    #[test]
    fn test_lerp() {
        let sig = lerp(0.0, 100.0, 2.0);
        assert_eq!(sig.at(0.0), 0.0);
        assert!((sig.at(1.0) - 50.0).abs() < 1e-10);
        assert_eq!(sig.at(2.0), 100.0);
    }

    #[test]
    fn test_lerp_clamped() {
        let sig = lerp(0.0, 100.0, 2.0);
        assert_eq!(sig.at(-1.0), 0.0); // Before start
        assert_eq!(sig.at(5.0), 100.0); // After end
    }

    #[test]
    fn test_ease() {
        // Simple linear easing (identity)
        let sig = ease(|p| p, 0.0, 100.0, 2.0);
        assert!((sig.at(1.0) - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_ease_quad() {
        // Quadratic ease-in
        let sig = ease(|p| p * p, 0.0, 100.0, 1.0);
        assert!((sig.at(0.5) - 25.0).abs() < 1e-10); // 0.5^2 * 100 = 25
    }

    #[test]
    fn test_zip() {
        let a = constant(3.0_f64);
        let b = constant(4.0_f64);
        let c = zip(a, b, |x, y| (x * x + y * y).sqrt());
        assert_eq!(c.at(0.0), 5.0); // 3-4-5 triangle
    }

    #[test]
    fn test_zip3() {
        let a = constant(1.0);
        let b = constant(2.0);
        let c = constant(3.0);
        let sum = zip3(a, b, c, |x, y, z| x + y + z);
        assert_eq!(sum.at(0.0), 6.0);
    }

    #[test]
    fn test_signal_vec_data() {
        let data_signal = of(|t| (0..5).map(|i| i as f64 + t).collect::<Vec<f64>>());

        let at_0 = data_signal.at(0.0);
        assert_eq!(at_0, vec![0.0, 1.0, 2.0, 3.0, 4.0]);

        let at_10 = data_signal.at(10.0);
        assert_eq!(at_10, vec![10.0, 11.0, 12.0, 13.0, 14.0]);
    }
}
