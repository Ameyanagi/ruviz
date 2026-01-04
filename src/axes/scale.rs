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
#[derive(Debug, Clone, PartialEq)]
pub enum AxisScale {
    /// Linear scale (default)
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

impl Default for AxisScale {
    fn default() -> Self {
        AxisScale::Linear
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
}
