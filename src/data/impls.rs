use crate::core::{PlottingError, Result};
use crate::data::{NullPolicy, NumericData1D, NumericData2D};

/// Collect 1D numeric data into an owned `Vec<f64>` using the requested null policy.
#[inline]
pub fn collect_numeric_data_1d<D>(data: &D, null_policy: NullPolicy) -> Result<Vec<f64>>
where
    D: NumericData1D + ?Sized,
{
    data.try_collect_f64_with_policy(null_policy)
}

/// Collect 2D numeric data into a row-major flat vector with shape metadata.
#[inline]
pub fn collect_numeric_data_2d<D>(data: &D) -> Result<(Vec<f64>, usize, usize)>
where
    D: NumericData2D + ?Sized,
{
    let (rows, cols) = data.shape();
    let values = data.try_collect_row_major_f64()?;
    let Some(expected_len) = rows.checked_mul(cols) else {
        return Err(PlottingError::DataExtractionFailed {
            source: "NumericData2D".to_string(),
            message: format!("shape {}x{} causes integer overflow", rows, cols),
        });
    };
    if expected_len != values.len() {
        return Err(PlottingError::DataExtractionFailed {
            source: "NumericData2D".to_string(),
            message: format!(
                "shape {}x{} does not match collected length {}",
                rows,
                cols,
                values.len()
            ),
        });
    }
    Ok((values, rows, cols))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct OverflowShapeData;

    impl NumericData2D for OverflowShapeData {
        fn shape(&self) -> (usize, usize) {
            (usize::MAX, 2)
        }

        fn try_collect_row_major_f64(&self) -> Result<Vec<f64>> {
            Ok(vec![])
        }
    }

    struct MismatchShapeData;

    impl NumericData2D for MismatchShapeData {
        fn shape(&self) -> (usize, usize) {
            (2, 2)
        }

        fn try_collect_row_major_f64(&self) -> Result<Vec<f64>> {
            Ok(vec![1.0, 2.0, 3.0])
        }
    }

    #[test]
    fn test_collect_numeric_data_2d_overflow_shape() {
        let err = collect_numeric_data_2d(&OverflowShapeData).unwrap_err();
        match err {
            PlottingError::DataExtractionFailed { source, message } => {
                assert_eq!(source, "NumericData2D");
                assert!(message.contains("integer overflow"));
            }
            other => panic!("expected DataExtractionFailed, got {other:?}"),
        }
    }

    #[test]
    fn test_collect_numeric_data_2d_shape_mismatch() {
        let err = collect_numeric_data_2d(&MismatchShapeData).unwrap_err();
        match err {
            PlottingError::DataExtractionFailed { source, message } => {
                assert_eq!(source, "NumericData2D");
                assert!(message.contains("does not match collected length"));
            }
            other => panic!("expected DataExtractionFailed, got {other:?}"),
        }
    }
}
