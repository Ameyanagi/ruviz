use crate::core::PlottingError;

/// Core data abstraction trait for 1D data series
///
/// This trait allows ruviz to accept various data types (Vec, arrays, slices)
/// without forcing users to convert their data into specific types.
pub trait Data1D<T> {
    /// Get the length of the data series
    fn len(&self) -> usize;

    /// Check if the data series is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a value at the specified index
    fn get(&self, index: usize) -> Option<&T>;

    /// Create an iterator over the data
    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_>;
}

// Blanket implementations for common Rust types

impl<T> Data1D<T> for Vec<T> {
    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn get(&self, index: usize) -> Option<&T> {
        <[T]>::get(self, index)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new(<[T]>::iter(self))
    }
}

impl<T> Data1D<T> for &Vec<T> {
    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn get(&self, index: usize) -> Option<&T> {
        (**self).get(index)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new((**self).iter())
    }
}

impl<T, const N: usize> Data1D<T> for [T; N] {
    fn len(&self) -> usize {
        N
    }

    fn get(&self, index: usize) -> Option<&T> {
        <[T]>::get(self, index)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new(<[T]>::iter(self))
    }
}

impl<T, const N: usize> Data1D<T> for &[T; N] {
    fn len(&self) -> usize {
        N
    }

    fn get(&self, index: usize) -> Option<&T> {
        (**self).get(index)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new((**self).iter())
    }
}

impl<T> Data1D<T> for &[T] {
    fn len(&self) -> usize {
        <[T]>::len(self)
    }

    fn get(&self, index: usize) -> Option<&T> {
        <[T]>::get(self, index)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new(<[T]>::iter(self))
    }
}

// Optional feature-gated implementations

#[cfg(feature = "ndarray_support")]
impl<T> Data1D<T> for ndarray::Array1<T> {
    fn len(&self) -> usize {
        self.len()
    }

    fn get(&self, index: usize) -> Option<&T> {
        self.get(index)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new(self.iter())
    }
}

#[cfg(feature = "ndarray_support")]
impl<'a, T> Data1D<T> for ndarray::ArrayView1<'a, T> {
    fn len(&self) -> usize {
        self.len()
    }

    fn get(&self, index: usize) -> Option<&T> {
        self.get(index)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new(self.iter())
    }
}

#[cfg(any(feature = "nalgebra_support", feature = "nalgebra"))]
impl Data1D<f64> for nalgebra::DVector<f64> {
    fn len(&self) -> usize {
        self.len()
    }

    fn get(&self, index: usize) -> Option<&f64> {
        self.as_slice().get(index)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &f64> + '_> {
        Box::new(self.as_slice().iter())
    }
}

#[cfg(any(feature = "nalgebra_support", feature = "nalgebra"))]
impl<const N: usize> Data1D<f64> for nalgebra::SVector<f64, N> {
    fn len(&self) -> usize {
        N
    }

    fn get(&self, index: usize) -> Option<&f64> {
        self.as_slice().get(index)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &f64> + '_> {
        Box::new(self.as_slice().iter())
    }
}

/// Null-handling policy for dataframe-backed numeric data ingestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NullPolicy {
    /// Reject null-containing inputs with an explicit error.
    #[default]
    Error,
    /// Drop null values.
    Drop,
    /// Fill null values with NaN.
    FillNaN,
}

/// Fallible numeric ingestion contract for 1D plotting data.
///
/// This trait is used by plotting entry points that accept numeric sequences
/// and need explicit extraction errors for dataframe-like backends.
pub trait NumericData1D {
    /// Length of the underlying 1D data.
    fn len(&self) -> usize;

    /// Whether the data is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Convert input to a concrete f64 vector (strict null policy by default).
    fn try_collect_f64(&self) -> Result<Vec<f64>, PlottingError> {
        self.try_collect_f64_with_policy(NullPolicy::Error)
    }

    /// Convert input to a concrete f64 vector using the requested null policy.
    fn try_collect_f64_with_policy(
        &self,
        _null_policy: NullPolicy,
    ) -> Result<Vec<f64>, PlottingError>;
}

impl<T> NumericData1D for T
where
    T: Data1D<f64>,
{
    fn len(&self) -> usize {
        Data1D::len(self)
    }

    fn try_collect_f64_with_policy(
        &self,
        _null_policy: NullPolicy,
    ) -> Result<Vec<f64>, PlottingError> {
        Ok(Data1D::iter(self).copied().collect())
    }
}

macro_rules! impl_numeric_data_1d_for_primitive_collections {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl NumericData1D for Vec<$ty> {
                fn len(&self) -> usize {
                    Vec::len(self)
                }

                fn try_collect_f64_with_policy(
                    &self,
                    _null_policy: NullPolicy,
                ) -> Result<Vec<f64>, PlottingError> {
                    Ok(self.iter().map(|v| *v as f64).collect())
                }
            }

            impl NumericData1D for &Vec<$ty> {
                fn len(&self) -> usize {
                    (**self).len()
                }

                fn try_collect_f64_with_policy(
                    &self,
                    _null_policy: NullPolicy,
                ) -> Result<Vec<f64>, PlottingError> {
                    Ok((**self).iter().map(|v| *v as f64).collect())
                }
            }

            impl<const N: usize> NumericData1D for [$ty; N] {
                fn len(&self) -> usize {
                    N
                }

                fn try_collect_f64_with_policy(
                    &self,
                    _null_policy: NullPolicy,
                ) -> Result<Vec<f64>, PlottingError> {
                    Ok(self.iter().map(|v| *v as f64).collect())
                }
            }

            impl<const N: usize> NumericData1D for &[$ty; N] {
                fn len(&self) -> usize {
                    N
                }

                fn try_collect_f64_with_policy(
                    &self,
                    _null_policy: NullPolicy,
                ) -> Result<Vec<f64>, PlottingError> {
                    Ok((**self).iter().map(|v| *v as f64).collect())
                }
            }

            impl NumericData1D for &[$ty] {
                fn len(&self) -> usize {
                    (**self).len()
                }

                fn try_collect_f64_with_policy(
                    &self,
                    _null_policy: NullPolicy,
                ) -> Result<Vec<f64>, PlottingError> {
                    Ok((**self).iter().map(|v| *v as f64).collect())
                }
            }

            #[cfg(feature = "ndarray_support")]
            impl NumericData1D for ndarray::Array1<$ty> {
                fn len(&self) -> usize {
                    self.len()
                }

                fn try_collect_f64_with_policy(
                    &self,
                    _null_policy: NullPolicy,
                ) -> Result<Vec<f64>, PlottingError> {
                    Ok(self.iter().map(|v| *v as f64).collect())
                }
            }

            #[cfg(feature = "ndarray_support")]
            impl<'a> NumericData1D for ndarray::ArrayView1<'a, $ty> {
                fn len(&self) -> usize {
                    self.len()
                }

                fn try_collect_f64_with_policy(
                    &self,
                    _null_policy: NullPolicy,
                ) -> Result<Vec<f64>, PlottingError> {
                    Ok(self.iter().map(|v| *v as f64).collect())
                }
            }
        )+
    };
}

impl_numeric_data_1d_for_primitive_collections!(
    f32, i64, i32, i16, i8, u64, u32, u16, u8, isize, usize
);

/// Fallible numeric ingestion contract for 2D plotting data (heatmap-style).
pub trait NumericData2D {
    /// Returns `(rows, cols)`.
    fn shape(&self) -> (usize, usize);

    /// Convert input to row-major f64 values.
    fn try_collect_row_major_f64(&self) -> Result<Vec<f64>, PlottingError>;
}

impl NumericData2D for Vec<Vec<f64>> {
    fn shape(&self) -> (usize, usize) {
        <[Vec<f64>] as NumericData2D>::shape(self.as_slice())
    }

    fn try_collect_row_major_f64(&self) -> Result<Vec<f64>, PlottingError> {
        <[Vec<f64>] as NumericData2D>::try_collect_row_major_f64(self.as_slice())
    }
}

impl NumericData2D for [Vec<f64>] {
    fn shape(&self) -> (usize, usize) {
        (
            self.len(),
            self.first().map(std::vec::Vec::len).unwrap_or(0),
        )
    }

    fn try_collect_row_major_f64(&self) -> Result<Vec<f64>, PlottingError> {
        if self.is_empty() {
            return Ok(vec![]);
        }

        let cols = self[0].len();
        for (row_idx, row) in self.iter().enumerate() {
            if row.len() != cols {
                return Err(PlottingError::InvalidInput(format!(
                    "Heatmap rows have inconsistent lengths: row {} has {} values, expected {}",
                    row_idx,
                    row.len(),
                    cols
                )));
            }
        }

        let mut values = Vec::with_capacity(self.len() * cols);
        for row in self {
            values.extend(row.iter().copied());
        }
        Ok(values)
    }
}

#[cfg(feature = "ndarray_support")]
impl NumericData2D for ndarray::Array2<f64> {
    fn shape(&self) -> (usize, usize) {
        (self.nrows(), self.ncols())
    }

    fn try_collect_row_major_f64(&self) -> Result<Vec<f64>, PlottingError> {
        let (rows, cols) = (self.nrows(), self.ncols());
        let mut values = Vec::with_capacity(rows * cols);
        for r in 0..rows {
            for c in 0..cols {
                values.push(self[(r, c)]);
            }
        }
        Ok(values)
    }
}

#[cfg(feature = "ndarray_support")]
impl<'a> NumericData2D for ndarray::ArrayView2<'a, f64> {
    fn shape(&self) -> (usize, usize) {
        (self.nrows(), self.ncols())
    }

    fn try_collect_row_major_f64(&self) -> Result<Vec<f64>, PlottingError> {
        let (rows, cols) = (self.nrows(), self.ncols());
        let mut values = Vec::with_capacity(rows * cols);
        for r in 0..rows {
            for c in 0..cols {
                values.push(self[(r, c)]);
            }
        }
        Ok(values)
    }
}

#[cfg(any(feature = "nalgebra_support", feature = "nalgebra"))]
impl NumericData2D for nalgebra::DMatrix<f64> {
    fn shape(&self) -> (usize, usize) {
        (self.nrows(), self.ncols())
    }

    fn try_collect_row_major_f64(&self) -> Result<Vec<f64>, PlottingError> {
        let (rows, cols) = self.shape();
        let mut values = Vec::with_capacity(rows * cols);
        for r in 0..rows {
            for c in 0..cols {
                values.push(self[(r, c)]);
            }
        }
        Ok(values)
    }
}

#[cfg(any(feature = "nalgebra_support", feature = "nalgebra"))]
impl<const R: usize, const C: usize> NumericData2D for nalgebra::SMatrix<f64, R, C> {
    fn shape(&self) -> (usize, usize) {
        (R, C)
    }

    fn try_collect_row_major_f64(&self) -> Result<Vec<f64>, PlottingError> {
        let mut values = Vec::with_capacity(R * C);
        for r in 0..R {
            for c in 0..C {
                values.push(self[(r, c)]);
            }
        }
        Ok(values)
    }
}

#[cfg(feature = "polars_support")]
fn polars_extract_error(source: &str, err: impl std::fmt::Display) -> PlottingError {
    PlottingError::DataExtractionFailed {
        source: source.to_string(),
        message: err.to_string(),
    }
}

#[cfg(feature = "polars_support")]
fn collect_polars_numeric<T, I, F>(
    iter: I,
    len: usize,
    null_count: usize,
    source: &str,
    null_policy: NullPolicy,
    map: F,
) -> Result<Vec<f64>, PlottingError>
where
    I: Iterator<Item = Option<T>>,
    F: Fn(T) -> f64,
{
    if matches!(null_policy, NullPolicy::Error) && null_count > 0 {
        return Err(PlottingError::NullValueNotAllowed {
            source: source.to_string(),
            column: None,
            null_count,
        });
    }

    let mut values = Vec::with_capacity(len);
    for maybe_value in iter {
        match maybe_value {
            Some(value) => values.push(map(value)),
            None => match null_policy {
                NullPolicy::Error => {}
                NullPolicy::Drop => {}
                NullPolicy::FillNaN => values.push(f64::NAN),
            },
        }
    }

    Ok(values)
}

#[cfg(feature = "polars_support")]
fn collect_polars_series(
    series: &polars::prelude::Series,
    null_policy: NullPolicy,
) -> Result<Vec<f64>, PlottingError> {
    use polars::prelude::DataType;

    let source = "polars::Series";
    match series.dtype() {
        DataType::Float64 => {
            let chunked = series.f64().map_err(|e| polars_extract_error(source, e))?;
            collect_polars_numeric(
                chunked.into_iter(),
                chunked.len(),
                chunked.null_count(),
                source,
                null_policy,
                |v| v,
            )
        }
        DataType::Float32 => {
            let chunked = series.f32().map_err(|e| polars_extract_error(source, e))?;
            collect_polars_numeric(
                chunked.into_iter(),
                chunked.len(),
                chunked.null_count(),
                source,
                null_policy,
                |v| v as f64,
            )
        }
        DataType::Int64 => {
            let chunked = series.i64().map_err(|e| polars_extract_error(source, e))?;
            collect_polars_numeric(
                chunked.into_iter(),
                chunked.len(),
                chunked.null_count(),
                source,
                null_policy,
                |v| v as f64,
            )
        }
        DataType::Int32 => {
            let chunked = series.i32().map_err(|e| polars_extract_error(source, e))?;
            collect_polars_numeric(
                chunked.into_iter(),
                chunked.len(),
                chunked.null_count(),
                source,
                null_policy,
                |v| v as f64,
            )
        }
        DataType::Int16 => {
            let chunked = series.i16().map_err(|e| polars_extract_error(source, e))?;
            collect_polars_numeric(
                chunked.into_iter(),
                chunked.len(),
                chunked.null_count(),
                source,
                null_policy,
                |v| v as f64,
            )
        }
        DataType::Int8 => {
            let chunked = series.i8().map_err(|e| polars_extract_error(source, e))?;
            collect_polars_numeric(
                chunked.into_iter(),
                chunked.len(),
                chunked.null_count(),
                source,
                null_policy,
                |v| v as f64,
            )
        }
        DataType::UInt64 => {
            let chunked = series.u64().map_err(|e| polars_extract_error(source, e))?;
            collect_polars_numeric(
                chunked.into_iter(),
                chunked.len(),
                chunked.null_count(),
                source,
                null_policy,
                |v| v as f64,
            )
        }
        DataType::UInt32 => {
            let chunked = series.u32().map_err(|e| polars_extract_error(source, e))?;
            collect_polars_numeric(
                chunked.into_iter(),
                chunked.len(),
                chunked.null_count(),
                source,
                null_policy,
                |v| v as f64,
            )
        }
        DataType::UInt16 => {
            let chunked = series.u16().map_err(|e| polars_extract_error(source, e))?;
            collect_polars_numeric(
                chunked.into_iter(),
                chunked.len(),
                chunked.null_count(),
                source,
                null_policy,
                |v| v as f64,
            )
        }
        DataType::UInt8 => {
            let chunked = series.u8().map_err(|e| polars_extract_error(source, e))?;
            collect_polars_numeric(
                chunked.into_iter(),
                chunked.len(),
                chunked.null_count(),
                source,
                null_policy,
                |v| v as f64,
            )
        }
        dtype => Err(PlottingError::DataTypeUnsupported {
            source: source.to_string(),
            dtype: format!("{dtype:?}"),
            expected: "numeric dtype".to_string(),
        }),
    }
}

#[cfg(feature = "polars_support")]
impl NumericData1D for polars::prelude::Series {
    fn len(&self) -> usize {
        self.len()
    }

    fn try_collect_f64_with_policy(
        &self,
        null_policy: NullPolicy,
    ) -> Result<Vec<f64>, PlottingError> {
        collect_polars_series(self, null_policy)
    }
}

#[cfg(feature = "polars_support")]
macro_rules! impl_polars_numeric_data_1d {
    ($ty:ty, $source:expr, $map:expr) => {
        impl NumericData1D for $ty {
            fn len(&self) -> usize {
                self.len()
            }

            fn try_collect_f64_with_policy(
                &self,
                null_policy: NullPolicy,
            ) -> Result<Vec<f64>, PlottingError> {
                collect_polars_numeric(
                    self.into_iter(),
                    self.len(),
                    self.null_count(),
                    $source,
                    null_policy,
                    $map,
                )
            }
        }
    };
}

#[cfg(feature = "polars_support")]
impl_polars_numeric_data_1d!(
    polars::prelude::Float64Chunked,
    "polars::Float64Chunked",
    |v| v
);
#[cfg(feature = "polars_support")]
impl_polars_numeric_data_1d!(
    polars::prelude::Float32Chunked,
    "polars::Float32Chunked",
    |v| { v as f64 }
);
#[cfg(feature = "polars_support")]
impl_polars_numeric_data_1d!(polars::prelude::Int64Chunked, "polars::Int64Chunked", |v| v
    as f64);
#[cfg(feature = "polars_support")]
impl_polars_numeric_data_1d!(polars::prelude::Int32Chunked, "polars::Int32Chunked", |v| v
    as f64);
#[cfg(feature = "polars_support")]
impl_polars_numeric_data_1d!(polars::prelude::Int16Chunked, "polars::Int16Chunked", |v| v
    as f64);
#[cfg(feature = "polars_support")]
impl_polars_numeric_data_1d!(polars::prelude::Int8Chunked, "polars::Int8Chunked", |v| v
    as f64);
#[cfg(feature = "polars_support")]
impl_polars_numeric_data_1d!(
    polars::prelude::UInt64Chunked,
    "polars::UInt64Chunked",
    |v| v as f64
);
#[cfg(feature = "polars_support")]
impl_polars_numeric_data_1d!(
    polars::prelude::UInt32Chunked,
    "polars::UInt32Chunked",
    |v| v as f64
);
#[cfg(feature = "polars_support")]
impl_polars_numeric_data_1d!(
    polars::prelude::UInt16Chunked,
    "polars::UInt16Chunked",
    |v| v as f64
);
#[cfg(feature = "polars_support")]
impl_polars_numeric_data_1d!(polars::prelude::UInt8Chunked, "polars::UInt8Chunked", |v| v
    as f64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_data1d() {
        let data = vec![1.0, 2.0, 3.0];
        assert_eq!(data.len(), 3);
        assert_eq!(data.get(1), Some(&2.0));
        assert!(!data.is_empty());

        let collected: Vec<&f64> = data.iter().collect();
        assert_eq!(collected, vec![&1.0, &2.0, &3.0]);
    }

    #[test]
    fn test_array_data1d() {
        let data = [1.0, 2.0, 3.0];
        assert_eq!(<[f64]>::len(&data), 3);
        assert_eq!(data.get(0), Some(&1.0));

        let collected: Vec<&f64> = data.iter().collect();
        assert_eq!(collected, vec![&1.0, &2.0, &3.0]);
    }

    #[test]
    fn test_slice_data1d() {
        let data: &[f64] = &[1.0, 2.0, 3.0];
        assert_eq!(data.len(), 3);
        assert_eq!(data.get(2), Some(&3.0));

        let collected: Vec<&f64> = data.iter().collect();
        assert_eq!(collected, vec![&1.0, &2.0, &3.0]);
    }

    #[test]
    fn test_empty_data() {
        let empty: Vec<f64> = vec![];
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
        assert_eq!(empty.get(0), None);
    }

    #[test]
    fn test_numeric_data1d_collect() {
        let data = vec![1.0, 2.0, 3.0];
        let collected = data.try_collect_f64().unwrap();
        assert_eq!(collected, data);
    }

    #[test]
    fn test_numeric_data1d_collect_integer() {
        let data = vec![1_i32, 2, 3];
        let collected = data.try_collect_f64().unwrap();
        assert_eq!(collected, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_numeric_data2d_from_vec() {
        let data = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        assert_eq!(data.shape(), (2, 2));
        let flat = data.try_collect_row_major_f64().unwrap();
        assert_eq!(flat, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_numeric_data2d_row_mismatch_error() {
        let data = vec![vec![1.0, 2.0], vec![3.0]];
        let err = data.try_collect_row_major_f64().unwrap_err();
        assert!(matches!(err, PlottingError::InvalidInput(_)));
    }

    #[cfg(feature = "ndarray_support")]
    #[test]
    fn test_ndarray_view_data1d() {
        use ndarray::Array1;

        let arr = Array1::from_vec(vec![1.0, 2.0, 3.0]);
        let view = arr.view();
        let values = view.try_collect_f64().unwrap();
        assert_eq!(values, vec![1.0, 2.0, 3.0]);
    }

    #[cfg(feature = "ndarray_support")]
    #[test]
    fn test_ndarray_data2d() {
        use ndarray::array;

        let arr = array![[1.0, 2.0], [3.0, 4.0]];
        assert_eq!(NumericData2D::shape(&arr), (2, 2));
        let flat = arr.try_collect_row_major_f64().unwrap();
        assert_eq!(flat, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[cfg(any(feature = "nalgebra_support", feature = "nalgebra"))]
    #[test]
    fn test_nalgebra_data1d() {
        let vec = nalgebra::DVector::from_vec(vec![1.0, 2.0, 3.0]);
        let values = vec.try_collect_f64().unwrap();
        assert_eq!(values, vec![1.0, 2.0, 3.0]);
    }

    #[cfg(any(feature = "nalgebra_support", feature = "nalgebra"))]
    #[test]
    fn test_nalgebra_data2d() {
        let matrix = nalgebra::DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(matrix.shape(), (2, 2));
        let flat = matrix.try_collect_row_major_f64().unwrap();
        assert_eq!(flat, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[cfg(feature = "polars_support")]
    #[test]
    fn test_polars_series_strict_null_error() {
        use polars::prelude::*;

        let series = Series::new("x".into(), &[Some(1.0), None, Some(3.0)]);
        let err = series.try_collect_f64().unwrap_err();
        assert!(matches!(err, PlottingError::NullValueNotAllowed { .. }));
    }

    #[cfg(feature = "polars_support")]
    #[test]
    fn test_polars_series_drop_nulls() {
        use polars::prelude::*;

        let series = Series::new("x".into(), &[Some(1.0), None, Some(3.0)]);
        let values = series
            .try_collect_f64_with_policy(NullPolicy::Drop)
            .unwrap();
        assert_eq!(values, vec![1.0, 3.0]);
    }
}
