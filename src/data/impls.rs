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
    if rows.checked_mul(cols).unwrap_or(usize::MAX) != values.len() {
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
