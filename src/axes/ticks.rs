//! Tick generation algorithms for axis labeling
//!
//! Implements matplotlib-compatible tick generation with "nice" number selection.

use super::AxisScale;

/// Generate tick positions using matplotlib's MaxNLocator algorithm
///
/// Produces ticks at "nice" numbers (1, 2, 5 multiples) for scientific plotting.
///
/// # Arguments
/// * `min` - Minimum data value
/// * `max` - Maximum data value
/// * `target_count` - Target number of ticks (clamped to 3-10)
///
/// # Returns
/// Vector of tick positions
pub fn generate_ticks(min: f64, max: f64, target_count: usize) -> Vec<f64> {
    if min >= max || target_count == 0 {
        return vec![min, max];
    }

    let max_ticks = target_count.clamp(3, 10);
    generate_nice_ticks(min, max, max_ticks)
}

/// Generate minor ticks between major tick positions
pub fn generate_minor_ticks(major_ticks: &[f64], count: usize) -> Vec<f64> {
    if major_ticks.len() < 2 || count == 0 {
        return Vec::new();
    }

    let mut minor_ticks = Vec::new();
    for window in major_ticks.windows(2) {
        let start = window[0];
        let end = window[1];
        let step = (end - start) / (count + 1) as f64;

        for i in 1..=count {
            minor_ticks.push(start + step * i as f64);
        }
    }
    minor_ticks
}

/// Generate tick positions for a specific axis scale type
///
/// Automatically selects the appropriate tick generation algorithm based on the scale.
///
/// # Arguments
/// * `min` - Minimum data value
/// * `max` - Maximum data value
/// * `target_count` - Target number of ticks
/// * `scale` - The axis scale type
///
/// # Returns
/// Vector of tick positions in data coordinates
pub fn generate_ticks_for_scale(
    min: f64,
    max: f64,
    target_count: usize,
    scale: &AxisScale,
) -> Vec<f64> {
    match scale {
        AxisScale::Linear => generate_ticks(min, max, target_count),
        AxisScale::Log => generate_log_ticks(min, max, target_count),
        AxisScale::SymLog { linthresh } => {
            generate_symlog_ticks(min, max, *linthresh, target_count)
        }
    }
}

/// Generate logarithmic tick positions at powers of 10
///
/// # Arguments
/// * `min` - Minimum data value (must be > 0)
/// * `max` - Maximum data value (must be > 0)
/// * `target_count` - Target number of ticks
///
/// # Returns
/// Vector of tick positions at powers of 10 (1, 10, 100, 1000, ...)
pub fn generate_log_ticks(min: f64, max: f64, target_count: usize) -> Vec<f64> {
    if min <= 0.0 || max <= 0.0 || min >= max {
        return vec![min.max(f64::EPSILON), max.max(f64::EPSILON)];
    }

    let log_min = min.log10().floor() as i32;
    let log_max = max.log10().ceil() as i32;
    let decades = (log_max - log_min) as usize;

    let mut ticks = Vec::new();

    if decades <= target_count {
        // Few decades: include all powers of 10
        for exp in log_min..=log_max {
            let tick = 10.0_f64.powi(exp);
            if tick >= min && tick <= max {
                ticks.push(tick);
            }
        }

        // If we have room, add intermediate ticks (2, 5) for better resolution
        if decades <= target_count / 2 {
            for exp in log_min..log_max {
                let base = 10.0_f64.powi(exp);
                for &mult in &[2.0, 5.0] {
                    let tick = base * mult;
                    if tick >= min && tick <= max {
                        ticks.push(tick);
                    }
                }
            }
        }
    } else {
        // Many decades: show every Nth decade
        let step = ((decades as f64) / (target_count as f64)).ceil() as i32;
        let start_exp = log_min;
        let mut exp = start_exp;
        while exp <= log_max {
            let tick = 10.0_f64.powi(exp);
            if tick >= min && tick <= max {
                ticks.push(tick);
            }
            exp += step;
        }
    }

    ticks.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ticks.dedup_by(|a, b| (*a - *b).abs() < f64::EPSILON);
    ticks
}

/// Generate symmetric logarithmic tick positions
///
/// Creates ticks that work well with symlog scale: linear ticks near zero,
/// logarithmic ticks outside the linear threshold.
///
/// # Arguments
/// * `min` - Minimum data value
/// * `max` - Maximum data value
/// * `linthresh` - Linear threshold
/// * `target_count` - Target number of ticks
pub fn generate_symlog_ticks(min: f64, max: f64, linthresh: f64, target_count: usize) -> Vec<f64> {
    if min >= max {
        return vec![min, max];
    }

    let mut ticks = Vec::new();

    // Linear region ticks: within ±linthresh
    let lin_min = min.max(-linthresh);
    let lin_max = max.min(linthresh);
    if lin_min < lin_max {
        // Generate a few linear ticks including 0
        if min < 0.0 && max > 0.0 {
            ticks.push(0.0);
        }
        if lin_min < 0.0 {
            ticks.push(lin_min);
        }
        if lin_max > 0.0 {
            ticks.push(lin_max);
        }
    }

    // Positive logarithmic region
    if max > linthresh {
        let log_ticks = generate_log_ticks(linthresh, max, target_count / 2);
        for tick in log_ticks {
            if tick > linthresh && tick <= max {
                ticks.push(tick);
            }
        }
    }

    // Negative logarithmic region
    if min < -linthresh {
        let log_ticks = generate_log_ticks(linthresh, -min, target_count / 2);
        for tick in log_ticks {
            let neg_tick = -tick;
            if neg_tick < -linthresh && neg_tick >= min {
                ticks.push(neg_tick);
            }
        }
    }

    ticks.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ticks.dedup_by(|a, b| (*a - *b).abs() < f64::EPSILON);
    ticks
}

/// Generate minor ticks for logarithmic scales
///
/// Creates ticks at 2, 3, 4, 5, 6, 7, 8, 9 × 10^n between major ticks
pub fn generate_log_minor_ticks(major_ticks: &[f64]) -> Vec<f64> {
    if major_ticks.len() < 2 {
        return Vec::new();
    }

    let mut minor_ticks = Vec::new();
    for window in major_ticks.windows(2) {
        let start = window[0];
        let end = window[1];

        // Check if these are consecutive decades
        if (end / start - 10.0).abs() < 0.01 {
            // Add minor ticks at 2, 3, 4, 5, 6, 7, 8, 9
            for mult in 2..=9 {
                let tick = start * mult as f64;
                if tick > start && tick < end {
                    minor_ticks.push(tick);
                }
            }
        }
    }
    minor_ticks
}

/// Internal function implementing nice number selection
fn generate_nice_ticks(min: f64, max: f64, max_ticks: usize) -> Vec<f64> {
    let range = max - min;
    if range <= 0.0 {
        return vec![min];
    }

    let rough_step = range / (max_ticks - 1) as f64;
    if rough_step <= f64::EPSILON {
        return vec![min, max];
    }

    // Round to "nice" numbers using powers of 10
    let magnitude = 10.0_f64.powf(rough_step.log10().floor());
    let normalized_step = rough_step / magnitude;

    // Select nice step sizes: prefer 1, 2, 5, 10 sequence
    let nice_step = if normalized_step <= 1.0 {
        1.0
    } else if normalized_step <= 2.0 {
        2.0
    } else if normalized_step <= 5.0 {
        5.0
    } else {
        10.0
    };

    let step = nice_step * magnitude;

    // Find optimal start point
    let start = (min / step).floor() * step;
    let end = (max / step).ceil() * step;

    // Generate ticks
    let mut ticks = Vec::new();
    let mut tick = start;
    let epsilon = step * 1e-10;

    while tick <= end + epsilon {
        if tick >= min - epsilon && tick <= max + epsilon {
            // Clean up floating point errors by rounding to appropriate precision
            let clean_tick = clean_float(tick, step);
            ticks.push(clean_tick);
        }
        tick += step;
    }

    ticks
}

/// Clean up floating point errors by rounding to appropriate precision based on step size
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_ticks_basic() {
        let ticks = generate_ticks(0.0, 10.0, 5);
        assert!(!ticks.is_empty());
        assert!(ticks.len() <= 10);
        assert!(ticks[0] >= 0.0);
        assert!(*ticks.last().unwrap() <= 10.0);
    }

    #[test]
    fn test_generate_ticks_nice_numbers() {
        let ticks = generate_ticks(0.0, 100.0, 6);
        // Should produce nice numbers like 0, 20, 40, 60, 80, 100
        for tick in &ticks {
            let tick_int = *tick as i64;
            // Nice numbers are divisible by 1, 2, 5, or 10
            assert!(tick_int % 10 == 0 || tick_int % 5 == 0 || tick_int % 2 == 0);
        }
    }

    #[test]
    fn test_generate_minor_ticks() {
        let major = vec![0.0, 10.0, 20.0];
        let minor = generate_minor_ticks(&major, 4);
        assert_eq!(minor.len(), 8); // 4 between each pair of major ticks
        assert!(minor[0] > 0.0 && minor[0] < 10.0);
    }

    #[test]
    fn test_invalid_range() {
        let ticks = generate_ticks(10.0, 5.0, 5);
        assert_eq!(ticks, vec![10.0, 5.0]);
    }

    #[test]
    fn test_log_ticks_powers_of_10() {
        let ticks = generate_log_ticks(1.0, 10000.0, 10);
        // Should include 1, 10, 100, 1000, 10000
        assert!(ticks.contains(&1.0));
        assert!(ticks.contains(&10.0));
        assert!(ticks.contains(&100.0));
        assert!(ticks.contains(&1000.0));
        assert!(ticks.contains(&10000.0));
    }

    #[test]
    fn test_log_ticks_few_decades() {
        let ticks = generate_log_ticks(1.0, 100.0, 10);
        // With few decades, should include intermediate ticks (2, 5)
        assert!(ticks.len() > 3); // More than just 1, 10, 100
    }

    #[test]
    fn test_log_ticks_invalid_range() {
        // Negative values should be handled gracefully
        let ticks = generate_log_ticks(-10.0, 100.0, 5);
        // Should return a fallback
        assert!(!ticks.is_empty());
    }

    #[test]
    fn test_symlog_ticks_includes_zero() {
        let ticks = generate_symlog_ticks(-100.0, 100.0, 1.0, 10);
        // Should include 0 in the linear region
        assert!(ticks.contains(&0.0));
    }

    #[test]
    fn test_symlog_ticks_both_regions() {
        let ticks = generate_symlog_ticks(-1000.0, 1000.0, 1.0, 10);
        // Should have both positive and negative ticks
        let has_positive = ticks.iter().any(|&t| t > 1.0);
        let has_negative = ticks.iter().any(|&t| t < -1.0);
        assert!(has_positive);
        assert!(has_negative);
    }

    #[test]
    fn test_log_minor_ticks() {
        let major = vec![1.0, 10.0, 100.0];
        let minor = generate_log_minor_ticks(&major);
        // Should have 8 minor ticks between each pair (2,3,4,5,6,7,8,9)
        assert_eq!(minor.len(), 16); // 8 * 2 pairs
        assert!(minor.contains(&2.0));
        assert!(minor.contains(&5.0));
        assert!(minor.contains(&20.0));
        assert!(minor.contains(&50.0));
    }

    #[test]
    fn test_generate_ticks_for_scale() {
        // Linear
        let linear_ticks = generate_ticks_for_scale(0.0, 100.0, 5, &AxisScale::Linear);
        assert!(!linear_ticks.is_empty());

        // Log
        let log_ticks = generate_ticks_for_scale(1.0, 1000.0, 5, &AxisScale::Log);
        assert!(log_ticks.contains(&10.0));
        assert!(log_ticks.contains(&100.0));

        // SymLog
        let symlog_ticks = generate_ticks_for_scale(-100.0, 100.0, 10, &AxisScale::symlog(1.0));
        assert!(symlog_ticks.contains(&0.0) || symlog_ticks.iter().any(|&t| t.abs() < 0.1));
    }
}
