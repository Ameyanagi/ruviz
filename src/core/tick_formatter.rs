//! Tick label formatting with nice numbers algorithm
//!
//! Provides clean tick label formatting matching matplotlib conventions. Tick
//! *positions* are not decided here: step selection is delegated to the one
//! canonical generator in [`crate::axes::ticks`], so the values this type
//! produces can never drift from the ticks the renderers draw.
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

use crate::axes::ticks::{MAX_TICK_STEPS, clean_float, select_nice_step};

/// Nice numbers for tick selection (powers of 10 multiplied by these)
const NICE_NUMBERS: [f64; 4] = [1.0, 2.0, 5.0, 10.0];

/// Mantissa decimals a scientific axis starts from: `1.00e-6`.
const SCIENTIFIC_MANTISSA_DIGITS: usize = 2;

/// Hard ceiling on digits in a tick label.
///
/// An `f64` carries about 17 significant decimal digits; past that, widening a
/// label cannot separate two values that the type itself cannot separate.
const MAX_LABEL_DIGITS: usize = 17;

/// Do these labels tell the ticks apart?
///
/// An axis whose labels repeat is unreadable: the reader cannot tell which
/// gridline is which, and the repetition hides how fine the range really is.
/// Ticks that are genuinely equal (a degenerate range collapses to one value)
/// may of course share a label — only *different* values sharing one is a
/// defect.
///
/// This is the single test both notations are held to, so the plain branch and
/// the scientific branch cannot disagree about what counts as legible.
fn labels_separate_values(values: &[f64], labels: &[String]) -> bool {
    // Tick counts are capped well under a hundred, so the quadratic scan is
    // cheaper than allocating a map and needs no float hashing.
    for (index, (&value, label)) in values.iter().zip(labels).enumerate() {
        for (&earlier_value, earlier_label) in values.iter().zip(labels).take(index) {
            if earlier_label == label && earlier_value != value {
                return false;
            }
        }
    }
    true
}

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

    /// Generate nice tick values covering a range
    ///
    /// Step selection is delegated to the one canonical tick generator in
    /// [`crate::axes::ticks`], so this cannot drift from the ticks the
    /// renderers draw. The only difference is the emission: this method
    /// *covers* the range (the first tick is at or below `min`, the last at or
    /// above `max`), whereas the renderers trim to the data range.
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

        let target_ticks = (self.min_ticks + self.max_ticks) / 2;
        let step = select_nice_step(min, max, target_ticks).unwrap_or_else(|| {
            let rough_step = range / (target_ticks.max(2) - 1) as f64;
            Self::nice_number(rough_step, true)
        });
        if !step.is_finite() || step <= 0.0 {
            return vec![min, max];
        }

        // Nice bounds that cover the data range.
        let first_index = (min / step).floor();
        let last_index = (max / step).ceil();
        let steps = last_index - first_index;
        if !steps.is_finite() || steps < 0.0 || steps > MAX_TICK_STEPS as f64 {
            return vec![min, max];
        }

        // Bounded integer-index emission: terminates by construction and does
        // not accumulate drift the way repeated addition does.
        let start = first_index * step;
        if !start.is_finite() || start + step <= start {
            return vec![min, max];
        }
        let steps = steps.round() as usize;
        let mut ticks: Vec<f64> = (0..=steps)
            .map(|i| clean_float(start + (i as f64) * step, step))
            .filter(|tick| tick.is_finite())
            .collect();
        ticks.dedup();
        if ticks.is_empty() {
            return vec![min, max];
        }

        // Ensure we don't exceed max_ticks
        if ticks.len() > self.max_ticks {
            // Take every nth tick to reduce count
            let skip = (ticks.len() as f64 / self.max_ticks as f64).ceil() as usize;
            ticks = ticks.into_iter().step_by(skip.max(1)).collect();
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
            return format!("{:.0}", Self::normalize_zero(value.round()));
        }

        // Format with minimal decimal places
        let formatted = format!("{:.prec$}", value, prec = self.max_decimals);

        // Trim trailing zeros after decimal point
        Self::trim_trailing_zeros(&formatted)
    }

    /// Format a whole axis worth of tick values.
    ///
    /// This is the axis-level formatter: **notation and precision are decided
    /// once for the whole slice**, never per value. All ticks use the same
    /// number of decimal places (driven by the tick needing the most), and the
    /// axis switches to scientific notation as a unit or not at all — a single
    /// axis can never show "1.0e5" next to "80000".
    pub fn format_ticks(&self, values: &[f64]) -> Vec<String> {
        if values.is_empty() {
            return Vec::new();
        }

        // Plain decimals are what a reader expects, so they are tried first and
        // kept as long as they can still tell the ticks apart.
        let plain_allowed = !self.use_scientific || self.plain_precision_is_sufficient(values);
        if plain_allowed {
            let labels = self.plain_labels(values, self.max_decimals);
            if labels_separate_values(values, &labels) {
                return labels;
            }
            if !self.use_scientific {
                // The caller asked for plain decimals; widen them rather than
                // switching notation behind its back.
                return self.plain_labels(values, MAX_LABEL_DIGITS);
            }
        }

        self.scientific_labels(values)
    }

    /// Format a whole axis with the notation decided by the caller instead of
    /// the automatic switch: `Some(true)` forces scientific labels,
    /// `Some(false)` plain decimals (widened as far as distinctness needs),
    /// `None` keeps the automatic choice of [`Self::format_ticks`].
    pub fn format_ticks_with_notation(
        &self,
        values: &[f64],
        scientific: Option<bool>,
    ) -> Vec<String> {
        if values.is_empty() {
            return Vec::new();
        }
        match scientific {
            Some(true) => self.scientific_labels(values),
            Some(false) => {
                let mut plain_only = self.clone();
                plain_only.use_scientific = false;
                plain_only.format_ticks(values)
            }
            None => self.format_ticks(values),
        }
    }

    /// Format every tick as a plain decimal with one shared precision.
    ///
    /// The precision is the largest any tick needs to be represented exactly,
    /// capped at `max_decimals` — so all labels line up and none of them
    /// invents digits the data does not have.
    fn plain_labels(&self, values: &[f64], max_decimals: usize) -> Vec<String> {
        let precision = values
            .iter()
            .map(|&v| Self::required_precision(v))
            .max()
            .unwrap_or(0)
            .min(max_decimals);

        values
            .iter()
            .map(|&v| {
                let v = Self::normalize_zero(v);
                if precision == 0 || (v - v.round()).abs() < 1e-9 {
                    format!("{:.0}", Self::normalize_zero(v.round()))
                } else {
                    let formatted = format!("{:.prec$}", v, prec = precision);
                    Self::trim_trailing_zeros(&formatted)
                }
            })
            .collect()
    }

    /// Format every tick in scientific notation, with just enough mantissa
    /// digits to keep the labels distinct.
    ///
    /// Three significant digits used to be hard-coded here, which is why a
    /// narrow range like `1e-6 .. 1.0001e-6` collapsed onto eleven identical
    /// `1.00e-6` labels: the axis had correctly decided that plain decimals
    /// could not resolve it, then threw away the resolution anyway. The digit
    /// count is now driven by the same requirement the notation switch is —
    /// that two ticks with different values never print the same string.
    fn scientific_labels(&self, values: &[f64]) -> Vec<String> {
        let render = |mantissa_digits: usize| -> Vec<String> {
            values
                .iter()
                .map(|&v| {
                    if v == 0.0 {
                        // Zero has no exponent and reads the same in either
                        // notation, so it never forces a mantissa on the axis.
                        "0".to_string()
                    } else {
                        format!("{:.prec$e}", v, prec = mantissa_digits)
                    }
                })
                .collect()
        };

        // Two decimals is the customary axis look; widen only as far as the
        // data actually demands, and never past what an f64 can distinguish.
        for mantissa_digits in SCIENTIFIC_MANTISSA_DIGITS..MAX_LABEL_DIGITS {
            let labels = render(mantissa_digits);
            if labels_separate_values(values, &labels) {
                return labels;
            }
        }
        render(MAX_LABEL_DIGITS)
    }

    /// Can every value be rendered faithfully with at most `max_decimals`
    /// places?
    ///
    /// "Faithfully" means within 0.1% — plain decimals below the cap do not
    /// merely lose a digit, they misstate the tick (1.5e-6 renders as
    /// "0.000002", a 33% error, and neighbouring ticks can collide on one
    /// label). Ordinary float noise is orders of magnitude under this bound, so
    /// only genuinely sub-resolution axes trip it.
    fn plain_precision_is_sufficient(&self, values: &[f64]) -> bool {
        const MAX_RELATIVE_LABEL_ERROR: f64 = 1e-3;

        let mult = 10.0_f64.powi(self.max_decimals as i32);
        if !mult.is_finite() {
            return true;
        }

        values.iter().all(|&value| {
            if !value.is_finite() {
                return true;
            }
            let rounded = (value * mult).round() / mult;
            (rounded - value).abs() <= value.abs() * MAX_RELATIVE_LABEL_ERROR
        })
    }

    /// Map `-0.0` onto `0.0` so no tick ever renders as "-0".
    fn normalize_zero(value: f64) -> f64 {
        if value == 0.0 { 0.0 } else { value }
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
    pub(crate) fn trim_trailing_zeros(s: &str) -> String {
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
        assert_eq!(formatter.format_tick(157.0 / 50.0), "3.14");
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
    fn test_format_ticks_never_mixes_notations() {
        let formatter = TickFormatter::default();

        // An axis fine enough to exhaust `max_decimals` switches as a whole
        // rather than per value: plain formatting would render 1.5e-6 and
        // 2.0e-6 as the same label.
        let labels = formatter.format_ticks(&[0.0, 1.5e-6, 3.0e-6, 4.5e-6]);
        let scientific = labels.iter().filter(|l| l.contains('e')).count();
        assert_eq!(
            scientific, 3,
            "expected the whole axis in scientific notation: {labels:?}"
        );
        assert_eq!(labels[0], "0");

        // Distinct labels either way.
        let unique: std::collections::BTreeSet<_> = labels.iter().collect();
        assert_eq!(unique.len(), labels.len(), "duplicate labels: {labels:?}");

        // Ranges that plain decimals can represent stay plain end to end.
        for values in [
            vec![0.0, 200000.0, 400000.0, 600000.0],
            vec![0.0, 0.25, 0.5, 0.75, 1.0],
            vec![-1e-5, 0.0, 1e-5],
        ] {
            let labels = formatter.format_ticks(&values);
            assert!(
                labels.iter().all(|l| !l.contains('e')),
                "unexpected scientific notation in {labels:?}"
            );
        }
    }

    #[test]
    fn test_format_never_renders_negative_zero() {
        let formatter = TickFormatter::default();

        assert_eq!(formatter.format_tick(-0.0), "0");
        assert_eq!(formatter.format_ticks(&[-0.0, 0.5])[0], "0");
        assert_eq!(formatter.format_ticks(&[-0.0, 1.0])[0], "0");
    }

    #[test]
    fn test_generate_ticks_uses_canonical_step_selection() {
        let formatter = TickFormatter::default();

        // Same step the renderers would pick for this range, emitted so that
        // it covers the range instead of being trimmed to it.
        assert_eq!(
            formatter.generate_ticks(0.7, 9.3),
            vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0]
        );
        // Nine ticks against a target of six is a 50% overshoot, which the
        // asymmetric density term now rejects: `MaxNLocator(nbins=6)` picks
        // step 2 for this range too. The assertion above is unaffected —
        // `select_nice_step` returns that same step 2 directly instead of
        // returning step 1 and leaning on `max_ticks` decimation afterwards.
        assert_eq!(
            crate::axes::generate_ticks(0.7, 9.3, 6),
            vec![2.0, 4.0, 6.0, 8.0]
        );
    }

    #[test]
    fn test_generate_ticks_terminates_at_extreme_magnitudes() {
        let formatter = TickFormatter::default();

        // Steps below one ULP of the axis start used to stall the emission
        // loop; the bounded index walk and the ULP guard must both hold.
        for (min, max) in [
            (1e16, 1e16 + 2.0),
            (1e16, 1e16 + 5.0),
            (0.0, f64::MAX),
            (f64::MIN_POSITIVE, 1.0),
        ] {
            let ticks = formatter.generate_ticks(min, max);
            assert!(!ticks.is_empty(), "no ticks for ({min}, {max})");
            assert!(
                ticks.len() <= 101,
                "unbounded tick count for ({min}, {max}): {}",
                ticks.len()
            );
        }
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
