//! Tick generation algorithms for axis labeling
//!
//! Implements matplotlib-compatible tick generation with "nice" number selection.

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
            ticks.push(tick);
        }
        tick += step;
    }

    ticks
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
}
