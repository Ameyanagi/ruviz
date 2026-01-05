//! Quantile and percentile calculations
//!
//! Provides quantile calculations for boxen plots and statistical summaries.

/// Calculate quantiles at specified probabilities
///
/// # Arguments
/// * `data` - Input data (will be sorted internally)
/// * `probs` - Probabilities between 0 and 1
///
/// # Returns
/// Quantile values at each probability
pub fn quantiles(data: &[f64], probs: &[f64]) -> Vec<f64> {
    if data.is_empty() {
        return vec![f64::NAN; probs.len()];
    }

    let mut sorted: Vec<f64> = data.iter().copied().filter(|x| x.is_finite()).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    if sorted.is_empty() {
        return vec![f64::NAN; probs.len()];
    }

    probs.iter().map(|&p| quantile_sorted(&sorted, p)).collect()
}

/// Calculate quantile from sorted data
fn quantile_sorted(sorted: &[f64], p: f64) -> f64 {
    let p = p.clamp(0.0, 1.0);
    let n = sorted.len();

    if n == 1 {
        return sorted[0];
    }

    // Linear interpolation method (type 7 in R)
    let h = (n - 1) as f64 * p;
    let h_floor = h.floor() as usize;
    let h_ceil = h.ceil().min((n - 1) as f64) as usize;

    if h_floor == h_ceil {
        sorted[h_floor]
    } else {
        let frac = h - h_floor as f64;
        sorted[h_floor] * (1.0 - frac) + sorted[h_ceil] * frac
    }
}

/// Calculate letter values for boxen (letter-value) plots
///
/// Letter values are a sequence of quantiles: median, fourths, eighths, etc.
/// Each level halves the tail probability.
///
/// # Arguments
/// * `data` - Input data
/// * `k` - Optional maximum depth (number of letter value levels)
///
/// # Returns
/// Vec of (lower, upper) pairs for each letter value level,
/// starting from median (center) outward
pub fn letter_values(data: &[f64], k: Option<usize>) -> Vec<(f64, f64)> {
    let mut sorted: Vec<f64> = data.iter().copied().filter(|x| x.is_finite()).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    if sorted.is_empty() {
        return vec![];
    }

    let n = sorted.len();

    // Determine maximum depth k based on data size
    // Rule: stop when expected outliers > 1
    let max_k = ((n as f64).log2() - 1.0).floor() as usize;
    let k = k.unwrap_or(max_k).min(max_k).max(1);

    let mut result = Vec::with_capacity(k);

    // First level is median (p = 0.5)
    let median = quantile_sorted(&sorted, 0.5);
    result.push((median, median));

    // Subsequent levels: 1/4, 3/4; 1/8, 7/8; 1/16, 15/16; ...
    for i in 1..k {
        let denom = 2.0_f64.powi((i + 1) as i32);
        let p_lower = 1.0 / denom;
        let p_upper = 1.0 - p_lower;

        let lower = quantile_sorted(&sorted, p_lower);
        let upper = quantile_sorted(&sorted, p_upper);

        result.push((lower, upper));
    }

    result
}

/// Five-number summary: min, Q1, median, Q3, max
pub fn five_number_summary(data: &[f64]) -> [f64; 5] {
    let qs = quantiles(data, &[0.0, 0.25, 0.5, 0.75, 1.0]);
    [qs[0], qs[1], qs[2], qs[3], qs[4]]
}

/// Calculate interquartile range
pub fn iqr(data: &[f64]) -> f64 {
    let qs = quantiles(data, &[0.25, 0.75]);
    qs[1] - qs[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantiles_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let qs = quantiles(&data, &[0.0, 0.5, 1.0]);

        assert!((qs[0] - 1.0).abs() < 1e-10);
        assert!((qs[1] - 3.0).abs() < 1e-10);
        assert!((qs[2] - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_quantiles_interpolation() {
        let data = vec![1.0, 2.0, 3.0, 4.0];
        let qs = quantiles(&data, &[0.25, 0.5, 0.75]);

        assert!((qs[0] - 1.75).abs() < 1e-10);
        assert!((qs[1] - 2.5).abs() < 1e-10);
        assert!((qs[2] - 3.25).abs() < 1e-10);
    }

    #[test]
    fn test_letter_values() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let lv = letter_values(&data, Some(4));

        // Should have 4 levels
        assert_eq!(lv.len(), 4);

        // First level is median
        assert!((lv[0].0 - 50.5).abs() < 1.0);
        assert!((lv[0].1 - 50.5).abs() < 1.0);

        // Outer levels should be wider
        for i in 1..lv.len() {
            assert!(lv[i].0 < lv[i - 1].0);
            assert!(lv[i].1 > lv[i - 1].1);
        }
    }

    #[test]
    fn test_five_number_summary() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let summary = five_number_summary(&data);

        assert!((summary[0] - 1.0).abs() < 1e-10); // min
        assert!((summary[2] - 5.0).abs() < 1e-10); // median
        assert!((summary[4] - 9.0).abs() < 1e-10); // max
    }

    #[test]
    fn test_iqr() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let iqr_val = iqr(&data);
        assert!(iqr_val > 0.0);
    }

    #[test]
    fn test_empty_data() {
        let qs = quantiles(&[], &[0.5]);
        assert!(qs[0].is_nan());

        let lv = letter_values(&[], None);
        assert!(lv.is_empty());
    }
}
