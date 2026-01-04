//! Data validation utilities for plot types
//!
//! This module provides shared validation functions used across different plot types
//! to eliminate code duplication and ensure consistent error handling.

use crate::core::error::PlottingError;
use crate::data::traits::Data1D;

/// Collects finite (non-NaN, non-Infinite) values from a data source.
///
/// This function iterates through the data, filters out non-finite values,
/// and returns the collected finite values. Returns an error if:
/// - The input data is empty
/// - No finite values are found
///
/// # Type Parameters
/// * `T` - The input data element type, must be convertible to f64
/// * `D` - The data source type implementing `Data1D<T>`
///
/// # Arguments
/// * `data` - The data source to collect from
///
/// # Returns
/// * `Ok(Vec<f64>)` - Vector of finite values
/// * `Err(PlottingError)` - If data is empty or contains no finite values
///
/// # Example
/// ```ignore
/// use ruviz::data::validation::collect_finite_values;
///
/// let data = vec![1.0, 2.0, f64::NAN, 3.0, f64::INFINITY];
/// let finite = collect_finite_values(&data)?;
/// assert_eq!(finite, vec![1.0, 2.0, 3.0]);
/// ```
pub fn collect_finite_values<T, D>(data: &D) -> Result<Vec<f64>, PlottingError>
where
    T: Into<f64> + Copy,
    D: Data1D<T>,
{
    if data.len() == 0 {
        return Err(PlottingError::EmptyDataSet);
    }

    let values: Vec<f64> = (0..data.len())
        .filter_map(|i| data.get(i))
        .map(|v| (*v).into())
        .filter(|v: &f64| v.is_finite())
        .collect();

    if values.is_empty() {
        return Err(PlottingError::InvalidData {
            message: "No finite values in data".to_string(),
            position: None,
        });
    }

    Ok(values)
}

/// Collects finite values and returns them sorted.
///
/// Same as `collect_finite_values` but additionally sorts the result
/// in ascending order. Useful for statistical calculations that require
/// sorted data (percentiles, quartiles, etc.).
///
/// # Arguments
/// * `data` - The data source to collect from
///
/// # Returns
/// * `Ok(Vec<f64>)` - Sorted vector of finite values
/// * `Err(PlottingError)` - If data is empty or contains no finite values
pub fn collect_finite_values_sorted<T, D>(data: &D) -> Result<Vec<f64>, PlottingError>
where
    T: Into<f64> + Copy,
    D: Data1D<T>,
{
    let mut values = collect_finite_values(data)?;
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_finite_values_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = collect_finite_values(&data).unwrap();
        assert_eq!(result, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn test_collect_finite_values_filters_nan() {
        let data = vec![1.0, f64::NAN, 3.0, f64::NAN, 5.0];
        let result = collect_finite_values(&data).unwrap();
        assert_eq!(result, vec![1.0, 3.0, 5.0]);
    }

    #[test]
    fn test_collect_finite_values_filters_infinity() {
        let data = vec![1.0, f64::INFINITY, 3.0, f64::NEG_INFINITY, 5.0];
        let result = collect_finite_values(&data).unwrap();
        assert_eq!(result, vec![1.0, 3.0, 5.0]);
    }

    #[test]
    fn test_collect_finite_values_empty_data() {
        let data: Vec<f64> = vec![];
        let result = collect_finite_values(&data);
        assert!(matches!(result, Err(PlottingError::EmptyDataSet)));
    }

    #[test]
    fn test_collect_finite_values_all_nan() {
        let data = vec![f64::NAN, f64::NAN, f64::NAN];
        let result = collect_finite_values(&data);
        assert!(matches!(result, Err(PlottingError::InvalidData { .. })));
    }

    #[test]
    fn test_collect_finite_values_sorted() {
        let data = vec![5.0, 2.0, 8.0, 1.0, 9.0];
        let result = collect_finite_values_sorted(&data).unwrap();
        assert_eq!(result, vec![1.0, 2.0, 5.0, 8.0, 9.0]);
    }

    #[test]
    fn test_collect_finite_values_with_integers() {
        let data: Vec<i32> = vec![1, 2, 3, 4, 5];
        let result = collect_finite_values(&data).unwrap();
        assert_eq!(result, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    }
}
