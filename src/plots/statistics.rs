//! Statistical utility functions for plot calculations
//!
//! This module provides common statistical functions used across different plot types.

/// Calculate the mean of a slice of values
pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// Calculate the standard deviation of a slice of values
pub fn std_dev(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let m = mean(values);
    let variance = values.iter().map(|x| (x - m).powi(2)).sum::<f64>() / values.len() as f64;
    variance.sqrt()
}

/// Calculate a percentile from sorted data
///
/// Uses linear interpolation between adjacent values.
/// `percentile` should be in range 0-100.
pub fn percentile(sorted_data: &[f64], percentile: f64) -> f64 {
    if sorted_data.is_empty() {
        return 0.0;
    }

    let n = sorted_data.len();
    let pos = percentile / 100.0 * (n - 1) as f64;
    let lower = pos.floor() as usize;
    let upper = pos.ceil() as usize;

    if lower == upper || upper >= n {
        sorted_data[lower.min(n - 1)]
    } else {
        let weight = pos - lower as f64;
        sorted_data[lower] * (1.0 - weight) + sorted_data[upper] * weight
    }
}

/// Calculate the interquartile range (IQR) from sorted data
pub fn iqr(sorted_data: &[f64]) -> f64 {
    let q1 = percentile(sorted_data, 25.0);
    let q3 = percentile(sorted_data, 75.0);
    q3 - q1
}

/// Calculate the median from sorted data
pub fn median(sorted_data: &[f64]) -> f64 {
    percentile(sorted_data, 50.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mean() {
        assert_eq!(mean(&[1.0, 2.0, 3.0, 4.0, 5.0]), 3.0);
        assert_eq!(mean(&[]), 0.0);
    }

    #[test]
    fn test_std_dev() {
        let values = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let sd = std_dev(&values);
        assert!((sd - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_percentile() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(percentile(&data, 0.0), 1.0);
        assert_eq!(percentile(&data, 100.0), 5.0);
        assert_eq!(percentile(&data, 50.0), 3.0);
    }

    #[test]
    fn test_iqr() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let result = iqr(&data);
        assert!(result > 0.0);
    }

    #[test]
    fn test_median() {
        assert_eq!(median(&[1.0, 2.0, 3.0, 4.0, 5.0]), 3.0);
        assert_eq!(median(&[1.0, 2.0, 3.0, 4.0]), 2.5);
    }
}
