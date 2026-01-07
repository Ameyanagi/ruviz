//! Interpolation and easing for smooth animations
//!
//! Provides the `Interpolate` trait for smooth value transitions
//! and a collection of standard easing functions.

use palette::Srgba;

/// Trait for types that can be interpolated between two values
///
/// Used for smooth transitions in animations.
///
/// # Example
///
/// ```rust
/// use ruviz::animation::Interpolate;
///
/// let start = 0.0_f64;
/// let end = 10.0_f64;
///
/// assert_eq!(start.interpolate(&end, 0.0), 0.0);
/// assert_eq!(start.interpolate(&end, 0.5), 5.0);
/// assert_eq!(start.interpolate(&end, 1.0), 10.0);
/// ```
pub trait Interpolate: Clone {
    /// Interpolate between self and target at progress t (0.0 to 1.0)
    fn interpolate(&self, target: &Self, t: f64) -> Self;
}

impl Interpolate for f64 {
    fn interpolate(&self, target: &Self, t: f64) -> Self {
        self + (target - self) * t
    }
}

impl Interpolate for f32 {
    fn interpolate(&self, target: &Self, t: f64) -> Self {
        self + (target - self) * t as f32
    }
}

impl Interpolate for i32 {
    fn interpolate(&self, target: &Self, t: f64) -> Self {
        (*self as f64 + (*target - *self) as f64 * t).round() as i32
    }
}

impl Interpolate for u32 {
    fn interpolate(&self, target: &Self, t: f64) -> Self {
        let start = *self as f64;
        let end = *target as f64;
        (start + (end - start) * t).round().max(0.0) as u32
    }
}

impl<T: Interpolate> Interpolate for Vec<T> {
    fn interpolate(&self, target: &Self, t: f64) -> Self {
        // Interpolate element-wise up to the shorter length
        let len = self.len().min(target.len());
        self.iter()
            .zip(target.iter())
            .take(len)
            .map(|(a, b)| a.interpolate(b, t))
            .collect()
    }
}

impl<T: Interpolate, const N: usize> Interpolate for [T; N] {
    fn interpolate(&self, target: &Self, t: f64) -> Self {
        std::array::from_fn(|i| self[i].interpolate(&target[i], t))
    }
}

impl Interpolate for Srgba {
    fn interpolate(&self, target: &Self, t: f64) -> Self {
        Srgba::new(
            self.red.interpolate(&target.red, t),
            self.green.interpolate(&target.green, t),
            self.blue.interpolate(&target.blue, t),
            self.alpha.interpolate(&target.alpha, t),
        )
    }
}

/// Easing function type
pub type EasingFn = fn(f64) -> f64;

/// Standard easing functions for smooth animations
///
/// Easing functions transform a linear progress value (0.0 to 1.0)
/// into a curved progress that creates more natural-looking motion.
///
/// # Categories
///
/// - `linear`: No easing, constant speed
/// - `ease_in_*`: Starts slow, accelerates
/// - `ease_out_*`: Starts fast, decelerates
/// - `ease_in_out_*`: Slow at both ends, fast in middle
///
/// # Example
///
/// ```rust
/// use ruviz::animation::easing;
///
/// let t = 0.5;
///
/// // Linear is unchanged
/// assert_eq!(easing::linear(t), 0.5);
///
/// // Quadratic ease-in is slower at start
/// assert!(easing::ease_in_quad(t) < 0.5);
///
/// // Quadratic ease-out is slower at end
/// assert!(easing::ease_out_quad(t) > 0.5);
/// ```
pub mod easing {
    use std::f64::consts::PI;

    /// Linear easing (no easing)
    pub fn linear(t: f64) -> f64 {
        t
    }

    // Quadratic easing

    /// Quadratic ease-in: t²
    pub fn ease_in_quad(t: f64) -> f64 {
        t * t
    }

    /// Quadratic ease-out: 1-(1-t)²
    pub fn ease_out_quad(t: f64) -> f64 {
        1.0 - (1.0 - t).powi(2)
    }

    /// Quadratic ease-in-out
    pub fn ease_in_out_quad(t: f64) -> f64 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
        }
    }

    // Cubic easing

    /// Cubic ease-in: t³
    pub fn ease_in_cubic(t: f64) -> f64 {
        t * t * t
    }

    /// Cubic ease-out: 1-(1-t)³
    pub fn ease_out_cubic(t: f64) -> f64 {
        1.0 - (1.0 - t).powi(3)
    }

    /// Cubic ease-in-out
    pub fn ease_in_out_cubic(t: f64) -> f64 {
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
        }
    }

    // Quartic easing

    /// Quartic ease-in: t⁴
    pub fn ease_in_quart(t: f64) -> f64 {
        t * t * t * t
    }

    /// Quartic ease-out: 1-(1-t)⁴
    pub fn ease_out_quart(t: f64) -> f64 {
        1.0 - (1.0 - t).powi(4)
    }

    /// Quartic ease-in-out
    pub fn ease_in_out_quart(t: f64) -> f64 {
        if t < 0.5 {
            8.0 * t.powi(4)
        } else {
            1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
        }
    }

    // Sinusoidal easing

    /// Sinusoidal ease-in
    pub fn ease_in_sine(t: f64) -> f64 {
        1.0 - (t * PI / 2.0).cos()
    }

    /// Sinusoidal ease-out
    pub fn ease_out_sine(t: f64) -> f64 {
        (t * PI / 2.0).sin()
    }

    /// Sinusoidal ease-in-out
    pub fn ease_in_out_sine(t: f64) -> f64 {
        -(((t * PI).cos() - 1.0) / 2.0)
    }

    // Exponential easing

    /// Exponential ease-in
    pub fn ease_in_expo(t: f64) -> f64 {
        if t == 0.0 {
            0.0
        } else {
            2.0_f64.powf(10.0 * t - 10.0)
        }
    }

    /// Exponential ease-out
    pub fn ease_out_expo(t: f64) -> f64 {
        if t == 1.0 {
            1.0
        } else {
            1.0 - 2.0_f64.powf(-10.0 * t)
        }
    }

    /// Exponential ease-in-out
    pub fn ease_in_out_expo(t: f64) -> f64 {
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else if t < 0.5 {
            2.0_f64.powf(20.0 * t - 10.0) / 2.0
        } else {
            (2.0 - 2.0_f64.powf(-20.0 * t + 10.0)) / 2.0
        }
    }

    // Elastic easing

    /// Elastic ease-in (bouncy at start)
    pub fn ease_in_elastic(t: f64) -> f64 {
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else {
            let c4 = (2.0 * PI) / 3.0;
            -(2.0_f64.powf(10.0 * t - 10.0)) * ((t * 10.0 - 10.75) * c4).sin()
        }
    }

    /// Elastic ease-out (bouncy at end)
    pub fn ease_out_elastic(t: f64) -> f64 {
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else {
            let c4 = (2.0 * PI) / 3.0;
            2.0_f64.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
        }
    }

    /// Elastic ease-in-out
    pub fn ease_in_out_elastic(t: f64) -> f64 {
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else {
            let c5 = (2.0 * PI) / 4.5;
            if t < 0.5 {
                -(2.0_f64.powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * c5).sin()) / 2.0
            } else {
                (2.0_f64.powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * c5).sin()) / 2.0 + 1.0
            }
        }
    }

    // Bounce easing

    /// Bounce ease-out (ball bounce at end)
    pub fn ease_out_bounce(t: f64) -> f64 {
        let n1 = 7.5625;
        let d1 = 2.75;

        if t < 1.0 / d1 {
            n1 * t * t
        } else if t < 2.0 / d1 {
            let t = t - 1.5 / d1;
            n1 * t * t + 0.75
        } else if t < 2.5 / d1 {
            let t = t - 2.25 / d1;
            n1 * t * t + 0.9375
        } else {
            let t = t - 2.625 / d1;
            n1 * t * t + 0.984375
        }
    }

    /// Bounce ease-in (ball bounce at start)
    pub fn ease_in_bounce(t: f64) -> f64 {
        1.0 - ease_out_bounce(1.0 - t)
    }

    /// Bounce ease-in-out
    pub fn ease_in_out_bounce(t: f64) -> f64 {
        if t < 0.5 {
            (1.0 - ease_out_bounce(1.0 - 2.0 * t)) / 2.0
        } else {
            (1.0 + ease_out_bounce(2.0 * t - 1.0)) / 2.0
        }
    }

    // Back easing (overshoots)

    /// Back ease-in (pulls back before moving forward)
    pub fn ease_in_back(t: f64) -> f64 {
        let c1 = 1.70158;
        let c3 = c1 + 1.0;
        c3 * t * t * t - c1 * t * t
    }

    /// Back ease-out (overshoots then returns)
    pub fn ease_out_back(t: f64) -> f64 {
        let c1 = 1.70158;
        let c3 = c1 + 1.0;
        1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
    }

    /// Back ease-in-out
    pub fn ease_in_out_back(t: f64) -> f64 {
        let c1 = 1.70158;
        let c2 = c1 * 1.525;

        if t < 0.5 {
            ((2.0 * t).powi(2) * ((c2 + 1.0) * 2.0 * t - c2)) / 2.0
        } else {
            ((2.0 * t - 2.0).powi(2) * ((c2 + 1.0) * (t * 2.0 - 2.0) + c2) + 2.0) / 2.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f64_interpolate() {
        assert_eq!(0.0_f64.interpolate(&10.0, 0.0), 0.0);
        assert_eq!(0.0_f64.interpolate(&10.0, 0.5), 5.0);
        assert_eq!(0.0_f64.interpolate(&10.0, 1.0), 10.0);
    }

    #[test]
    fn test_vec_interpolate() {
        let a = vec![0.0, 0.0];
        let b = vec![10.0, 20.0];

        let result = a.interpolate(&b, 0.5);
        assert_eq!(result, vec![5.0, 10.0]);
    }

    #[test]
    fn test_array_interpolate() {
        let a = [0.0, 0.0, 0.0];
        let b = [10.0, 20.0, 30.0];

        let result = a.interpolate(&b, 0.25);
        assert_eq!(result, [2.5, 5.0, 7.5]);
    }

    #[test]
    fn test_linear_easing() {
        assert_eq!(easing::linear(0.0), 0.0);
        assert_eq!(easing::linear(0.5), 0.5);
        assert_eq!(easing::linear(1.0), 1.0);
    }

    #[test]
    fn test_ease_in_quad() {
        assert_eq!(easing::ease_in_quad(0.0), 0.0);
        assert_eq!(easing::ease_in_quad(0.5), 0.25);
        assert_eq!(easing::ease_in_quad(1.0), 1.0);
    }

    #[test]
    fn test_ease_out_quad() {
        assert_eq!(easing::ease_out_quad(0.0), 0.0);
        assert_eq!(easing::ease_out_quad(0.5), 0.75);
        assert_eq!(easing::ease_out_quad(1.0), 1.0);
    }

    #[test]
    fn test_easing_boundary_values() {
        // All easing functions should return 0 at t=0 and 1 at t=1
        let easings: Vec<(&str, fn(f64) -> f64)> = vec![
            ("linear", easing::linear),
            ("ease_in_quad", easing::ease_in_quad),
            ("ease_out_quad", easing::ease_out_quad),
            ("ease_in_out_quad", easing::ease_in_out_quad),
            ("ease_in_cubic", easing::ease_in_cubic),
            ("ease_out_cubic", easing::ease_out_cubic),
            ("ease_in_out_cubic", easing::ease_in_out_cubic),
            ("ease_in_sine", easing::ease_in_sine),
            ("ease_out_sine", easing::ease_out_sine),
            ("ease_in_out_sine", easing::ease_in_out_sine),
            ("ease_in_expo", easing::ease_in_expo),
            ("ease_out_expo", easing::ease_out_expo),
            ("ease_in_out_expo", easing::ease_in_out_expo),
            ("ease_in_elastic", easing::ease_in_elastic),
            ("ease_out_elastic", easing::ease_out_elastic),
            ("ease_in_out_elastic", easing::ease_in_out_elastic),
            ("ease_in_bounce", easing::ease_in_bounce),
            ("ease_out_bounce", easing::ease_out_bounce),
            ("ease_in_out_bounce", easing::ease_in_out_bounce),
            ("ease_in_back", easing::ease_in_back),
            ("ease_out_back", easing::ease_out_back),
            ("ease_in_out_back", easing::ease_in_out_back),
        ];

        for (name, f) in easings {
            let at_0 = f(0.0);
            let at_1 = f(1.0);
            assert!(
                (at_0 - 0.0).abs() < 1e-10,
                "{} at 0.0 = {} (expected 0.0)",
                name,
                at_0
            );
            assert!(
                (at_1 - 1.0).abs() < 1e-10,
                "{} at 1.0 = {} (expected 1.0)",
                name,
                at_1
            );
        }
    }

    #[test]
    fn test_ease_in_slower_than_linear() {
        // Ease-in functions should be below linear in the first half
        let t = 0.25;
        assert!(easing::ease_in_quad(t) < t);
        assert!(easing::ease_in_cubic(t) < t);
        assert!(easing::ease_in_quart(t) < t);
    }

    #[test]
    fn test_ease_out_faster_than_linear() {
        // Ease-out functions should be above linear in the first half
        let t = 0.25;
        assert!(easing::ease_out_quad(t) > t);
        assert!(easing::ease_out_cubic(t) > t);
        assert!(easing::ease_out_quart(t) > t);
    }
}
