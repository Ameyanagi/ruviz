//! Axis scale transformations
//!
//! Provides linear and logarithmic scale transformations for axis mapping.

/// Scale transformation trait
pub trait Scale {
    /// Transform a value from data space to normalized [0, 1] space
    fn transform(&self, value: f64) -> f64;

    /// Inverse transform from normalized space back to data space
    fn inverse(&self, normalized: f64) -> f64;

    /// Get the data range
    fn range(&self) -> (f64, f64);
}

/// Linear scale transformation
#[derive(Debug, Clone)]
pub struct LinearScale {
    min: f64,
    max: f64,
}

impl LinearScale {
    /// Create a new linear scale with the given range
    pub fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }
}

impl Scale for LinearScale {
    fn transform(&self, value: f64) -> f64 {
        if (self.max - self.min).abs() < f64::EPSILON {
            return 0.5;
        }
        (value - self.min) / (self.max - self.min)
    }

    fn inverse(&self, normalized: f64) -> f64 {
        normalized * (self.max - self.min) + self.min
    }

    fn range(&self) -> (f64, f64) {
        (self.min, self.max)
    }
}

/// Logarithmic scale transformation (base 10)
#[derive(Debug, Clone)]
pub struct LogScale {
    min: f64,
    max: f64,
    log_min: f64,
    log_max: f64,
}

impl LogScale {
    /// Create a new log scale with the given range
    ///
    /// # Panics
    /// Panics if min <= 0 or max <= 0
    pub fn new(min: f64, max: f64) -> Self {
        assert!(min > 0.0, "Log scale requires positive values");
        assert!(max > 0.0, "Log scale requires positive values");
        Self {
            min,
            max,
            log_min: min.log10(),
            log_max: max.log10(),
        }
    }
}

impl Scale for LogScale {
    fn transform(&self, value: f64) -> f64 {
        if value <= 0.0 {
            return 0.0;
        }
        let log_range = self.log_max - self.log_min;
        if log_range.abs() < f64::EPSILON {
            return 0.5;
        }
        (value.log10() - self.log_min) / log_range
    }

    fn inverse(&self, normalized: f64) -> f64 {
        let log_value = normalized * (self.log_max - self.log_min) + self.log_min;
        10.0_f64.powf(log_value)
    }

    fn range(&self) -> (f64, f64) {
        (self.min, self.max)
    }
}

/// Symmetric logarithmic scale transformation
///
/// This scale is linear around zero (within ±linthresh) and logarithmic outside.
/// Useful for data that spans positive and negative values or includes zero.
#[derive(Debug, Clone)]
pub struct SymLogScale {
    min: f64,
    max: f64,
    /// Linear threshold: values between -linthresh and +linthresh are scaled linearly
    linthresh: f64,
    /// Precomputed log of linthresh for efficiency
    log_linthresh: f64,
}

impl SymLogScale {
    /// Create a new symmetric log scale with the given range and linear threshold
    ///
    /// # Arguments
    /// * `min` - Minimum data value
    /// * `max` - Maximum data value
    /// * `linthresh` - Linear threshold (must be > 0). Values within ±linthresh are linear.
    ///
    /// # Panics
    /// Panics if linthresh <= 0
    pub fn new(min: f64, max: f64, linthresh: f64) -> Self {
        assert!(linthresh > 0.0, "SymLog scale requires positive linthresh");
        Self {
            min,
            max,
            linthresh,
            log_linthresh: linthresh.log10(),
        }
    }

    /// Transform a single value using symlog
    fn symlog(&self, value: f64) -> f64 {
        if value.abs() <= self.linthresh {
            // Linear region: scale to match log at threshold
            value / self.linthresh
        } else {
            // Logarithmic region
            let sign = value.signum();
            let abs_val = value.abs();
            sign * (1.0 + (abs_val / self.linthresh).log10())
        }
    }

    /// Inverse symlog transform
    fn inv_symlog(&self, transformed: f64) -> f64 {
        if transformed.abs() <= 1.0 {
            // Linear region
            transformed * self.linthresh
        } else {
            // Logarithmic region
            let sign = transformed.signum();
            let abs_t = transformed.abs();
            sign * self.linthresh * 10.0_f64.powf(abs_t - 1.0)
        }
    }
}

impl Scale for SymLogScale {
    fn transform(&self, value: f64) -> f64 {
        let t_min = self.symlog(self.min);
        let t_max = self.symlog(self.max);
        let t_value = self.symlog(value);

        let range = t_max - t_min;
        if range.abs() < f64::EPSILON {
            return 0.5;
        }
        (t_value - t_min) / range
    }

    fn inverse(&self, normalized: f64) -> f64 {
        let t_min = self.symlog(self.min);
        let t_max = self.symlog(self.max);
        let t_value = normalized * (t_max - t_min) + t_min;
        self.inv_symlog(t_value)
    }

    fn range(&self) -> (f64, f64) {
        (self.min, self.max)
    }
}

/// User-facing axis scale configuration
///
/// This enum provides a simple API for setting axis scales on plots.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum AxisScale {
    /// Linear scale (default)
    #[default]
    Linear,
    /// Logarithmic scale (base 10)
    /// Only valid for positive data values
    Log,
    /// Symmetric logarithmic scale
    /// Linear within ±linthresh, logarithmic outside
    SymLog {
        /// Linear threshold (values within ±linthresh are scaled linearly)
        linthresh: f64,
    },
}

#[inline]
pub(crate) fn linear_range_is_degenerate(range: f64) -> bool {
    range.abs() < f64::EPSILON
}

#[inline]
pub(crate) fn linear_normalized_position_with_range(
    value: f64,
    min: f64,
    max: f64,
    range: f64,
) -> f64 {
    if range.is_infinite() && value.is_finite() && min.is_finite() && max.is_finite() {
        (value / 2.0 - min / 2.0) / (max / 2.0 - min / 2.0)
    } else {
        (value - min) / range
    }
}

#[inline]
fn linear_inverse_normalized_position(normalized: f64, min: f64, max: f64) -> f64 {
    let range = max - min;
    if range.is_infinite() && min.is_finite() && max.is_finite() {
        (1.0 - normalized) * min + normalized * max
    } else {
        normalized * range + min
    }
}

#[inline]
fn log_normalization_bounds(min: f64, max: f64) -> (f64, f64) {
    if min.is_finite() && min > 0.0 && max.is_finite() && max > 0.0 {
        (min, max)
    } else {
        (min.max(f64::EPSILON), max.max(f64::EPSILON))
    }
}

#[inline]
fn log_ratio(value: f64, base: f64) -> f64 {
    let ratio = value / base;
    if ratio.is_finite() && ratio > 0.5 && ratio < 2.0 {
        ((value - base) / base).ln_1p()
    } else {
        value.ln() - base.ln()
    }
}

impl AxisScale {
    /// Create a logarithmic scale
    pub fn log() -> Self {
        AxisScale::Log
    }

    /// Create a symmetric logarithmic scale with the given linear threshold
    pub fn symlog(linthresh: f64) -> Self {
        AxisScale::SymLog { linthresh }
    }

    /// Normalize a value into `[0, 1]` for the provided range.
    ///
    /// This preserves range direction, so reversed ranges produce inverted
    /// normalized coordinates.
    pub fn normalized_position(&self, value: f64, min: f64, max: f64) -> f64 {
        match self {
            AxisScale::Linear => {
                let range = max - min;
                if linear_range_is_degenerate(range) {
                    0.5
                } else {
                    linear_normalized_position_with_range(value, min, max, range)
                }
            }
            AxisScale::Log => {
                if value <= 0.0 {
                    return 0.0;
                }

                if min.is_finite() && min > 0.0 && max.is_finite() && max > 0.0 {
                    if min == max {
                        return 0.5;
                    }
                    return log_ratio(value, min) / log_ratio(max, min);
                }

                let (min, max) = log_normalization_bounds(min, max);
                let log_min = min.log10();
                let log_max = max.log10();
                let log_range = log_max - log_min;
                if log_range.abs() <= f64::EPSILON {
                    0.5
                } else {
                    (value.log10() - log_min) / log_range
                }
            }
            AxisScale::SymLog { linthresh } => {
                let symlog = |input: f64| {
                    if input.abs() <= *linthresh {
                        input / *linthresh
                    } else {
                        input.signum() * (1.0 + (input.abs() / *linthresh).log10())
                    }
                };

                let transformed_min = symlog(min);
                let transformed_max = symlog(max);
                let transformed_value = symlog(value);
                let range = transformed_max - transformed_min;
                if range.abs() <= f64::EPSILON {
                    0.5
                } else {
                    (transformed_value - transformed_min) / range
                }
            }
        }
    }

    /// Convert a normalized scale position back into a value in the provided range.
    ///
    /// This is the mathematical inverse of [`Self::normalized_position`] for valid,
    /// non-degenerate ranges. Range direction is preserved, so `0.0` maps to
    /// `min` and `1.0` maps to `max` even when the range is reversed.
    pub fn inverse_normalized_position(&self, normalized: f64, min: f64, max: f64) -> f64 {
        match self {
            AxisScale::Linear => linear_inverse_normalized_position(normalized, min, max),
            AxisScale::Log => {
                if min.is_finite() && min > 0.0 && max.is_finite() && max > 0.0 {
                    if normalized == 0.0 {
                        return min;
                    }
                    if normalized == 1.0 {
                        return max;
                    }
                    if min == max {
                        return min;
                    }

                    let log_range = log_ratio(max, min);
                    if log_range.abs() < 0.5 {
                        return min * (normalized * log_range).exp();
                    }
                }

                let (min, max) = log_normalization_bounds(min, max);
                let log_min = min.log10();
                let log_max = max.log10();
                10.0_f64.powf(normalized * (log_max - log_min) + log_min)
            }
            AxisScale::SymLog { linthresh } => {
                let symlog = |input: f64| {
                    if input.abs() <= *linthresh {
                        input / *linthresh
                    } else {
                        input.signum() * (1.0 + (input.abs() / *linthresh).log10())
                    }
                };
                let inverse_symlog = |input: f64| {
                    if input.abs() <= 1.0 {
                        input * *linthresh
                    } else {
                        input.signum() * *linthresh * 10.0_f64.powf(input.abs() - 1.0)
                    }
                };

                let transformed_min = symlog(min);
                let transformed_max = symlog(max);
                inverse_symlog(normalized * (transformed_max - transformed_min) + transformed_min)
            }
        }
    }

    /// Create a scale instance for the given data range
    pub fn create_scale(&self, min: f64, max: f64) -> Box<dyn Scale> {
        match self {
            AxisScale::Linear => Box::new(LinearScale::new(min, max)),
            AxisScale::Log => Box::new(LogScale::new(min.max(f64::EPSILON), max.max(f64::EPSILON))),
            AxisScale::SymLog { linthresh } => Box::new(SymLogScale::new(min, max, *linthresh)),
        }
    }

    /// Check if this scale is valid for the given data range
    pub fn validate_range(&self, min: f64, max: f64) -> Result<(), String> {
        match self {
            AxisScale::Linear => Ok(()),
            AxisScale::Log => {
                if min <= 0.0 || max <= 0.0 {
                    Err("Logarithmic scale requires positive values. Use SymLog for data with zero or negative values.".to_string())
                } else {
                    Ok(())
                }
            }
            AxisScale::SymLog { linthresh } => {
                if *linthresh <= 0.0 {
                    Err("SymLog scale requires positive linthresh value.".to_string())
                } else {
                    Ok(())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_scale() {
        let scale = LinearScale::new(0.0, 100.0);

        assert!((scale.transform(0.0) - 0.0).abs() < 1e-10);
        assert!((scale.transform(50.0) - 0.5).abs() < 1e-10);
        assert!((scale.transform(100.0) - 1.0).abs() < 1e-10);

        assert!((scale.inverse(0.0) - 0.0).abs() < 1e-10);
        assert!((scale.inverse(0.5) - 50.0).abs() < 1e-10);
        assert!((scale.inverse(1.0) - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_log_scale() {
        let scale = LogScale::new(1.0, 1000.0);

        assert!((scale.transform(1.0) - 0.0).abs() < 1e-10);
        assert!((scale.transform(1000.0) - 1.0).abs() < 1e-10);

        // 10^1.5 ≈ 31.6 should be at ~0.5
        let mid = scale.inverse(0.5);
        assert!((mid.log10() - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_scale_range() {
        let scale = LinearScale::new(10.0, 20.0);
        assert_eq!(scale.range(), (10.0, 20.0));
    }

    #[test]
    fn test_symlog_scale_linear_region() {
        let scale = SymLogScale::new(-10.0, 10.0, 1.0);

        // Values within ±linthresh should be linear
        let t_zero = scale.transform(0.0);
        let t_half = scale.transform(0.5);
        let t_neg_half = scale.transform(-0.5);

        // Zero should be at center
        assert!((t_zero - 0.5).abs() < 0.1);
        // Symmetric around zero in linear region
        assert!((t_half - t_zero - (t_zero - t_neg_half)).abs() < 0.01);
    }

    #[test]
    fn test_symlog_scale_log_region() {
        let scale = SymLogScale::new(1.0, 100.0, 1.0);

        // At threshold, t=1
        let t_1 = scale.transform(1.0);
        // At 10, t = 1 + log10(10) = 2
        let t_10 = scale.transform(10.0);
        // At 100, t = 1 + log10(100) = 3
        let t_100 = scale.transform(100.0);

        assert!(t_1 < t_10);
        assert!(t_10 < t_100);
        assert!((t_1 - 0.0).abs() < 0.01); // min should map to 0
        assert!((t_100 - 1.0).abs() < 0.01); // max should map to 1
    }

    #[test]
    fn test_symlog_scale_inverse() {
        let scale = SymLogScale::new(-100.0, 100.0, 1.0);

        for value in [-50.0, -1.0, -0.5, 0.0, 0.5, 1.0, 50.0] {
            let normalized = scale.transform(value);
            let back = scale.inverse(normalized);
            assert!(
                (back - value).abs() < 0.01,
                "Inverse failed for {}: got {}",
                value,
                back
            );
        }
    }

    #[test]
    fn test_axis_scale_enum() {
        assert_eq!(AxisScale::default(), AxisScale::Linear);
        assert_eq!(AxisScale::log(), AxisScale::Log);
        assert_eq!(AxisScale::symlog(1.0), AxisScale::SymLog { linthresh: 1.0 });
    }

    #[test]
    fn test_axis_scale_validation() {
        // Linear is always valid
        assert!(AxisScale::Linear.validate_range(-10.0, 10.0).is_ok());

        // Log requires positive
        assert!(AxisScale::Log.validate_range(1.0, 100.0).is_ok());
        assert!(AxisScale::Log.validate_range(-1.0, 100.0).is_err());
        assert!(AxisScale::Log.validate_range(0.0, 100.0).is_err());

        // SymLog requires positive linthresh
        assert!(AxisScale::symlog(1.0).validate_range(-100.0, 100.0).is_ok());
        assert!(
            AxisScale::symlog(0.0)
                .validate_range(-100.0, 100.0)
                .is_err()
        );
    }

    #[test]
    fn test_axis_scale_create_scale() {
        let linear = AxisScale::Linear.create_scale(0.0, 100.0);
        assert!((linear.transform(50.0) - 0.5).abs() < 0.01);

        let log = AxisScale::Log.create_scale(1.0, 1000.0);
        assert!((log.transform(1.0) - 0.0).abs() < 0.01);
        assert!((log.transform(1000.0) - 1.0).abs() < 0.01);

        let symlog = AxisScale::symlog(1.0).create_scale(-100.0, 100.0);
        assert!((symlog.transform(0.0) - 0.5).abs() < 0.1);
    }

    #[test]
    fn test_axis_scale_normalized_position_preserves_reversed_ranges() {
        assert!((AxisScale::Linear.normalized_position(4.0, 4.0, 0.0) - 0.0).abs() < 1e-10);
        assert!((AxisScale::Linear.normalized_position(0.0, 4.0, 0.0) - 1.0).abs() < 1e-10);

        let log_mid = AxisScale::Log.normalized_position(10.0, 100.0, 1.0);
        assert!((log_mid - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_axis_scale_inverse_normalization_endpoints_and_midpoints() {
        let cases = [
            (AxisScale::Linear, 0.0, 10.0, 5.0),
            (AxisScale::Log, 1.0, 100.0, 10.0),
            (AxisScale::symlog(1.0), -100.0, 100.0, 0.0),
        ];

        for (scale, min, max, midpoint) in cases {
            assert!((scale.inverse_normalized_position(0.0, min, max) - min).abs() < 1e-10);
            assert!((scale.inverse_normalized_position(0.5, min, max) - midpoint).abs() < 1e-10);
            assert!((scale.inverse_normalized_position(1.0, min, max) - max).abs() < 1e-10);
        }
    }

    #[test]
    fn test_axis_scale_inverse_normalization_roundtrips_reversed_ranges() {
        let cases = [
            (AxisScale::Linear, 20.0, -10.0, vec![20.0, 7.0, -10.0]),
            (AxisScale::Log, 1000.0, 1.0, vec![1000.0, 10.0, 1.0]),
            (
                AxisScale::symlog(2.0),
                200.0,
                -50.0,
                vec![200.0, 10.0, 0.0, -2.0, -50.0],
            ),
        ];

        for (scale, min, max, values) in cases {
            for value in values {
                let normalized = scale.normalized_position(value, min, max);
                let recovered = scale.inverse_normalized_position(normalized, min, max);
                let tolerance = value.abs().max(1.0) * 1e-10;
                assert!(
                    (recovered - value).abs() <= tolerance,
                    "{scale:?} failed to round-trip {value} in {min}..{max}: {recovered}"
                );
            }
        }
    }

    #[test]
    fn test_axis_scale_log_invalid_value_behavior_is_preserved() {
        assert_eq!(AxisScale::Log.normalized_position(0.0, 1.0, 100.0), 0.0);
        assert_eq!(AxisScale::Log.normalized_position(-1.0, 1.0, 100.0), 0.0);
        assert!(AxisScale::Log.inverse_normalized_position(0.5, 1.0, 100.0) > 0.0);
        assert!(AxisScale::Log.validate_range(0.0, 100.0).is_err());
    }

    #[test]
    fn test_axis_scale_log_normalization_supports_sub_epsilon_domains() {
        let low = f64::EPSILON / 16.0;
        let high = f64::EPSILON / 2.0;
        let midpoint = 10.0_f64.powf(low.log10() + (high.log10() - low.log10()) * 0.5);

        for (min, max) in [(low, high), (high, low)] {
            assert_eq!(AxisScale::Log.normalized_position(min, min, max), 0.0);
            assert_eq!(AxisScale::Log.normalized_position(max, min, max), 1.0);

            for value in [min, midpoint, max] {
                let normalized = AxisScale::Log.normalized_position(value, min, max);
                let recovered = AxisScale::Log.inverse_normalized_position(normalized, min, max);
                assert!(
                    ((recovered - value) / value).abs() <= 1e-12,
                    "failed to round-trip {value} in {min}..{max}: {recovered}"
                );
            }
        }
    }

    #[test]
    fn test_axis_scale_log_normalization_supports_close_positive_bounds() {
        let min = 1.0_f64;
        let midpoint = f64::from_bits(min.to_bits() + 1);
        let max = f64::from_bits(min.to_bits() + 2);

        assert_eq!(AxisScale::Log.normalized_position(min, min, max), 0.0);
        assert_eq!(AxisScale::Log.normalized_position(max, min, max), 1.0);
        let normalized = AxisScale::Log.normalized_position(midpoint, min, max);
        assert!((normalized - 0.5).abs() <= f64::EPSILON);
        assert_eq!(
            AxisScale::Log.inverse_normalized_position(normalized, min, max),
            midpoint
        );
    }

    #[test]
    fn test_axis_scale_linear_exact_epsilon_span_is_not_degenerate() {
        let min = 0.0;
        let max = f64::EPSILON;

        assert_eq!(AxisScale::Linear.normalized_position(min, min, max), 0.0);
        assert_eq!(
            AxisScale::Linear.normalized_position(max / 2.0, min, max),
            0.5
        );
        assert_eq!(AxisScale::Linear.normalized_position(max, min, max), 1.0);

        let translated_min = 1.0;
        let translated_max = translated_min + f64::EPSILON;
        assert_eq!(
            AxisScale::Linear.normalized_position(translated_min, translated_min, translated_max),
            0.0
        );
        assert_eq!(
            AxisScale::Linear.normalized_position(translated_max, translated_min, translated_max),
            1.0
        );
    }

    #[test]
    fn test_axis_scale_linear_normalization_supports_finite_extreme_ranges() {
        for (min, max) in [(-f64::MAX, f64::MAX), (f64::MAX, -f64::MAX)] {
            assert_eq!(AxisScale::Linear.normalized_position(min, min, max), 0.0);
            assert_eq!(AxisScale::Linear.normalized_position(0.0, min, max), 0.5);
            assert_eq!(AxisScale::Linear.normalized_position(max, min, max), 1.0);

            assert_eq!(
                AxisScale::Linear.inverse_normalized_position(0.0, min, max),
                min
            );
            assert_eq!(
                AxisScale::Linear.inverse_normalized_position(0.5, min, max),
                0.0
            );
            assert_eq!(
                AxisScale::Linear.inverse_normalized_position(1.0, min, max),
                max
            );
        }
    }
}
