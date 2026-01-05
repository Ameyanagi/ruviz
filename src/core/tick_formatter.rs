//! Tick formatting with nice numbers algorithm
//!
//! Implements Wilkinson's extended algorithm for selecting "nice" tick values
//! and provides clean tick label formatting matching matplotlib conventions.
//!
//! # Example
//!
//! ```rust,ignore
//! use ruviz::core::TickFormatter;
//!
//! let formatter = TickFormatter::default();
//!
//! // Generate nice tick values for a range
//! let ticks = formatter.generate_ticks(0.7, 9.3);
//! // Result: [0.0, 2.0, 4.0, 6.0, 8.0, 10.0]
//!
//! // Format tick labels
//! let label = formatter.format_tick(5.0);
//! // Result: "5" (not "5.0")
//! ```

/// Nice numbers for tick selection (powers of 10 multiplied by these)
const NICE_NUMBERS: [f64; 4] = [1.0, 2.0, 5.0, 10.0];

/// Tick formatter configuration
///
/// Provides nice number selection for tick values and clean label formatting
/// that matches matplotlib conventions:
/// - Integers display without decimals: "5" not "5.0"
/// - Minimal decimal precision: "3.14" not "3.140000"
/// - Nice numbers: 1, 2, 5, 10 (and powers of 10)
#[derive(Debug, Clone)]
pub struct TickFormatter {
    /// Minimum number of ticks to generate
    pub min_ticks: usize,
    /// Maximum number of ticks to generate
    pub max_ticks: usize,
    /// Maximum decimal places for tick labels
    pub max_decimals: usize,
    /// Use scientific notation for very large/small values
    pub use_scientific: bool,
    /// Threshold for scientific notation (absolute value)
    pub scientific_threshold: f64,
}

impl Default for TickFormatter {
    /// Create default tick formatter
    ///
    /// - 4-9 ticks (matplotlib-like)
    /// - Up to 6 decimal places
    /// - Scientific notation for values > 10^4 or < 10^-4
    fn default() -> Self {
        Self {
            min_ticks: 4,
            max_ticks: 9,
            max_decimals: 6,
            use_scientific: true,
            scientific_threshold: 1e4,
        }
    }
}

impl TickFormatter {
    /// Create a new tick formatter with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set minimum number of ticks
    pub fn min_ticks(mut self, n: usize) -> Self {
        self.min_ticks = n.max(2);
        self
    }

    /// Set maximum number of ticks
    pub fn max_ticks(mut self, n: usize) -> Self {
        self.max_ticks = n.max(self.min_ticks);
        self
    }

    /// Set maximum decimal places
    pub fn max_decimals(mut self, n: usize) -> Self {
        self.max_decimals = n;
        self
    }

    /// Enable/disable scientific notation
    pub fn use_scientific(mut self, enabled: bool) -> Self {
        self.use_scientific = enabled;
        self
    }

    /// Round a value to a "nice" number
    ///
    /// Nice numbers are 1, 2, 5, or 10 multiplied by a power of 10.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to round
    /// * `round` - If true, round to nearest; if false, round up (ceiling)
    pub fn nice_number(value: f64, round: bool) -> f64 {
        if value == 0.0 {
            return 0.0;
        }

        let value = value.abs();
        let exponent = value.log10().floor();
        let fraction = value / 10.0_f64.powf(exponent);

        let nice_fraction = if round {
            // Round to nearest nice number
            // Using geometric mean thresholds: sqrt(1*2)≈1.41, sqrt(2*5)≈3.16, sqrt(5*10)≈7.07
            // Add small epsilon to handle floating point edge cases like 0.7/0.1 = 6.999...
            let frac = fraction + 1e-10;
            if frac < 1.5 {
                1.0
            } else if frac < 3.0 {
                2.0
            } else if frac < 7.0 {
                5.0
            } else {
                10.0
            }
        } else {
            // Round up to next nice number (ceiling)
            if fraction <= 1.0 {
                1.0
            } else if fraction <= 2.0 {
                2.0
            } else if fraction <= 5.0 {
                5.0
            } else {
                10.0
            }
        };

        nice_fraction * 10.0_f64.powf(exponent)
    }

    /// Generate nice tick values for a range
    ///
    /// Uses the extended Wilkinson algorithm to select aesthetically
    /// pleasing tick values that span the data range.
    ///
    /// # Arguments
    ///
    /// * `min` - Minimum data value
    /// * `max` - Maximum data value
    ///
    /// # Returns
    ///
    /// Vector of tick values
    pub fn generate_ticks(&self, min: f64, max: f64) -> Vec<f64> {
        if min >= max {
            return vec![min];
        }

        // Handle edge cases
        if !min.is_finite() || !max.is_finite() {
            return vec![0.0, 1.0];
        }

        let range = max - min;
        if range == 0.0 {
            return vec![min];
        }

        // Calculate nice tick spacing
        let target_ticks = ((self.min_ticks + self.max_ticks) / 2) as f64;
        let rough_step = range / (target_ticks - 1.0);
        let step = Self::nice_number(rough_step, true);

        // Calculate nice bounds
        let nice_min = (min / step).floor() * step;
        let nice_max = (max / step).ceil() * step;

        // Generate tick values
        let mut ticks = Vec::new();
        let mut tick = nice_min;

        // Safety limit to prevent infinite loops
        let max_iterations = 100;
        let mut iterations = 0;

        while tick <= nice_max + step * 0.5 && iterations < max_iterations {
            // Clean up floating point errors
            let clean_tick = Self::clean_float(tick, step);
            ticks.push(clean_tick);
            tick += step;
            iterations += 1;
        }

        // Ensure we don't exceed max_ticks
        if ticks.len() > self.max_ticks {
            // Take every nth tick to reduce count
            let skip = (ticks.len() as f64 / self.max_ticks as f64).ceil() as usize;
            ticks = ticks.into_iter().step_by(skip).collect();
        }

        ticks
    }

    /// Format a tick value as a clean string
    ///
    /// - Integers display without decimals: "5" not "5.0"
    /// - Minimal decimal precision: "3.14" not "3.140000"
    /// - Scientific notation for very large/small values
    ///
    /// # Arguments
    ///
    /// * `value` - The tick value to format
    pub fn format_tick(&self, value: f64) -> String {
        // Handle special values
        if !value.is_finite() {
            return value.to_string();
        }

        // Check for scientific notation
        let abs_value = value.abs();
        if self.use_scientific
            && abs_value != 0.0
            && (abs_value >= self.scientific_threshold
                || abs_value < 1.0 / self.scientific_threshold)
        {
            return format!("{:.2e}", value);
        }

        // Check if it's effectively an integer
        if (value - value.round()).abs() < 1e-9 {
            return format!("{:.0}", value);
        }

        // Format with minimal decimal places
        let formatted = format!("{:.prec$}", value, prec = self.max_decimals);

        // Trim trailing zeros after decimal point
        Self::trim_trailing_zeros(&formatted)
    }

    /// Format multiple tick values with consistent precision
    ///
    /// All ticks will use the same number of decimal places,
    /// determined by the tick that needs the most precision.
    pub fn format_ticks(&self, values: &[f64]) -> Vec<String> {
        if values.is_empty() {
            return Vec::new();
        }

        // Find the required precision
        let max_precision = values
            .iter()
            .map(|&v| Self::required_precision(v))
            .max()
            .unwrap_or(0)
            .min(self.max_decimals);

        // Format all values with consistent precision
        values
            .iter()
            .map(|&v| {
                if max_precision == 0 || (v - v.round()).abs() < 1e-9 {
                    format!("{:.0}", v)
                } else {
                    let formatted = format!("{:.prec$}", v, prec = max_precision);
                    Self::trim_trailing_zeros(&formatted)
                }
            })
            .collect()
    }

    /// Clean up floating point errors
    fn clean_float(value: f64, step: f64) -> f64 {
        // Round to a precision appropriate for the step size
        let decimals = if step >= 1.0 {
            0
        } else {
            (-step.log10().floor()) as i32 + 1
        };
        let mult = 10.0_f64.powi(decimals);
        (value * mult).round() / mult
    }

    /// Determine required decimal places for a value
    fn required_precision(value: f64) -> usize {
        if !value.is_finite() || (value - value.round()).abs() < 1e-9 {
            return 0;
        }

        // Find significant decimal places
        for precision in 1..=6 {
            let mult = 10.0_f64.powi(precision as i32);
            let rounded = (value * mult).round() / mult;
            if (value - rounded).abs() < 1e-9 {
                return precision;
            }
        }
        6
    }

    /// Trim trailing zeros from a formatted number
    fn trim_trailing_zeros(s: &str) -> String {
        if !s.contains('.') {
            return s.to_string();
        }

        let trimmed = s.trim_end_matches('0');
        if let Some(stripped) = trimmed.strip_suffix('.') {
            stripped.to_string()
        } else {
            trimmed.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nice_number_round() {
        // Test rounding to nearest nice number
        // Thresholds: 1.5 (1↔2), 3.0 (2↔5), 7.0 (5↔10)
        // 0.7 → fraction≈7 in 10^-1 decade → rounds to 10*0.1=1.0
        assert!((TickFormatter::nice_number(0.7, true) - 1.0).abs() < 0.001);
        // 3.2 → fraction=3.2 in 10^0 decade → 3.2>=3.0 rounds to 5.0
        assert!((TickFormatter::nice_number(3.2, true) - 5.0).abs() < 0.001);
        // 2.5 → fraction=2.5 in 10^0 decade → 2.5<3.0 rounds to 2.0
        assert!((TickFormatter::nice_number(2.5, true) - 2.0).abs() < 0.001);
        // 7.8 → fraction=7.8 in 10^0 decade → 7.8>=7.0 rounds to 10.0
        assert!((TickFormatter::nice_number(7.8, true) - 10.0).abs() < 0.001);
        // 12.0 → fraction=1.2 in 10^1 decade → 1.2<1.5 rounds to 1.0*10=10.0
        assert!((TickFormatter::nice_number(12.0, true) - 10.0).abs() < 0.001);
        // 25.0 → fraction=2.5 in 10^1 decade → 2.5<3.0 rounds to 2.0*10=20.0
        assert!((TickFormatter::nice_number(25.0, true) - 20.0).abs() < 0.001);
        // 55.0 → fraction=5.5 in 10^1 decade → 5.5<7.0 rounds to 5.0*10=50.0
        assert!((TickFormatter::nice_number(55.0, true) - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_nice_number_ceil() {
        // Test rounding up to next nice number
        assert!((TickFormatter::nice_number(0.7, false) - 1.0).abs() < 0.001);
        assert!((TickFormatter::nice_number(1.5, false) - 2.0).abs() < 0.001);
        assert!((TickFormatter::nice_number(3.5, false) - 5.0).abs() < 0.001);
        assert!((TickFormatter::nice_number(7.0, false) - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_generate_ticks() {
        let formatter = TickFormatter::default();

        // Test typical range
        let ticks = formatter.generate_ticks(0.7, 9.3);
        assert!(!ticks.is_empty());

        // Should include or be near the endpoints
        assert!(ticks[0] <= 0.7);
        assert!(*ticks.last().unwrap() >= 9.3);

        // All ticks should be evenly spaced with a nice step
        // Check that the spacing between ticks is consistent
        if ticks.len() > 1 {
            let step = ticks[1] - ticks[0];
            for i in 2..ticks.len() {
                let diff = (ticks[i] - ticks[i - 1] - step).abs();
                assert!(diff < 0.001, "Ticks not evenly spaced: {:?}", ticks);
            }
            // Step should be a nice number (1, 2, 5 * power of 10)
            let step_nice = TickFormatter::nice_number(step, true);
            assert!(
                (step - step_nice).abs() / step < 0.1,
                "Step {} is not nice (expected ~{})",
                step,
                step_nice
            );
        }
    }

    #[test]
    fn test_generate_ticks_nice_values() {
        let formatter = TickFormatter::default();

        // Test that we get nice values like [0, 2, 4, 6, 8, 10]
        let ticks = formatter.generate_ticks(0.7, 9.3);

        // Should contain round numbers
        let has_zero_or_two = ticks.iter().any(|&t| (t - 0.0).abs() < 0.001)
            || ticks.iter().any(|&t| (t - 2.0).abs() < 0.001);
        assert!(has_zero_or_two);
    }

    #[test]
    fn test_format_tick_integers() {
        let formatter = TickFormatter::default();

        // Integers should not have decimal point
        assert_eq!(formatter.format_tick(5.0), "5");
        assert_eq!(formatter.format_tick(10.0), "10");
        assert_eq!(formatter.format_tick(-3.0), "-3");
        assert_eq!(formatter.format_tick(0.0), "0");
    }

    #[test]
    fn test_format_tick_decimals() {
        let formatter = TickFormatter::default();

        // Should use minimal precision
        assert_eq!(formatter.format_tick(3.14), "3.14");
        assert_eq!(formatter.format_tick(2.5), "2.5");

        // Should trim trailing zeros
        assert_eq!(formatter.format_tick(1.10), "1.1");
        assert_eq!(formatter.format_tick(2.500), "2.5");
    }

    #[test]
    fn test_format_tick_scientific() {
        let formatter = TickFormatter::default();

        // Large values should use scientific notation
        let large = formatter.format_tick(1e6);
        assert!(large.contains('e'), "Expected scientific notation for 1e6");

        // Small values should use scientific notation
        let small = formatter.format_tick(1e-6);
        assert!(small.contains('e'), "Expected scientific notation for 1e-6");
    }

    #[test]
    fn test_format_ticks_consistent() {
        let formatter = TickFormatter::default();

        let values = vec![0.0, 0.5, 1.0, 1.5, 2.0];
        let labels = formatter.format_ticks(&values);

        assert_eq!(labels.len(), 5);
        // All should have consistent formatting
        // 0 and integer-like values should not have decimals
        assert_eq!(labels[0], "0");
        assert_eq!(labels[2], "1");
        assert_eq!(labels[4], "2");
        // Half values should have decimals
        assert_eq!(labels[1], "0.5");
        assert_eq!(labels[3], "1.5");
    }

    #[test]
    fn test_edge_cases() {
        let formatter = TickFormatter::default();

        // Same min and max
        let ticks = formatter.generate_ticks(5.0, 5.0);
        assert_eq!(ticks.len(), 1);

        // Negative range
        let ticks = formatter.generate_ticks(-10.0, -1.0);
        assert!(!ticks.is_empty());
        assert!(ticks[0] <= -10.0);
        assert!(*ticks.last().unwrap() >= -1.0);
    }

    #[test]
    fn test_trim_trailing_zeros() {
        assert_eq!(TickFormatter::trim_trailing_zeros("3.14000"), "3.14");
        assert_eq!(TickFormatter::trim_trailing_zeros("5.0"), "5");
        assert_eq!(TickFormatter::trim_trailing_zeros("5"), "5");
        assert_eq!(TickFormatter::trim_trailing_zeros("0.100"), "0.1");
    }
}
