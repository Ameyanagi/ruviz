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
}
