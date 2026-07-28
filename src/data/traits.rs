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

    /// Borrow the whole series as one contiguous slice, when the backing
    /// storage is contiguous and already in series order.
    ///
    /// This is the bulk fast path. [`Data1D::iter`] returns a
    /// `Box<dyn Iterator>`, so every element that ruviz ingests through it
    /// costs a virtual `next()` call; a 1M-point `line(&x, &y)` pays that twice
    /// per point. Any implementor whose elements already live in one run of
    /// memory should override this so ingestion becomes a single `memcpy`
    /// instead.
    ///
    /// The default returns `None`, which means "no contiguous view available" —
    /// callers must fall back to [`Data1D::get`] or [`Data1D::iter`] and get
    /// exactly the same values, only slower. Overriding is therefore never
    /// required and never observable in the output.
    fn as_slice(&self) -> Option<&[T]> {
        None
    }
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

    fn as_slice(&self) -> Option<&[T]> {
        Some(&self[..])
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

    fn as_slice(&self) -> Option<&[T]> {
        Some(&self[..])
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

    fn as_slice(&self) -> Option<&[T]> {
        Some(&self[..])
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

    fn as_slice(&self) -> Option<&[T]> {
        Some(&self[..])
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

    fn as_slice(&self) -> Option<&[T]> {
        Some(&self[..])
    }
}

impl<T> Data1D<T> for [T] {
    fn len(&self) -> usize {
        <[T]>::len(self)
    }

    fn get(&self, index: usize) -> Option<&T> {
        <[T]>::get(self, index)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new(<[T]>::iter(self))
    }

    fn as_slice(&self) -> Option<&[T]> {
        Some(self)
    }
}

#[cfg(feature = "ndarray_support")]
fn ndarray_len_1d<T, S>(array: &ndarray::ArrayBase<S, ndarray::Ix1>) -> usize
where
    S: ndarray::Data<Elem = T>,
{
    <ndarray::ArrayBase<S, ndarray::Ix1>>::len(array)
}

#[cfg(feature = "ndarray_support")]
fn ndarray_get_1d<T, S>(array: &ndarray::ArrayBase<S, ndarray::Ix1>, index: usize) -> Option<&T>
where
    S: ndarray::Data<Elem = T>,
{
    let array_ref: &ndarray::ArrayRef<T, ndarray::Ix1> = std::borrow::Borrow::borrow(array);
    <ndarray::ArrayRef<T, ndarray::Ix1>>::get(array_ref, index)
}

/// Contiguous view of a 1D array, or `None` when the array is strided or
/// otherwise not in standard layout.
#[cfg(feature = "ndarray_support")]
fn ndarray_as_slice_1d<T, S>(array: &ndarray::ArrayBase<S, ndarray::Ix1>) -> Option<&[T]>
where
    S: ndarray::Data<Elem = T>,
{
    let array_ref: &ndarray::ArrayRef<T, ndarray::Ix1> = std::borrow::Borrow::borrow(array);
    <ndarray::ArrayRef<T, ndarray::Ix1>>::as_slice(array_ref)
}

#[cfg(feature = "ndarray_support")]
fn ndarray_iter_1d<'a, T, S>(
    array: &'a ndarray::ArrayBase<S, ndarray::Ix1>,
) -> impl Iterator<Item = &'a T> + 'a
where
    S: ndarray::Data<Elem = T>,
{
    let array_ref: &ndarray::ArrayRef<T, ndarray::Ix1> = std::borrow::Borrow::borrow(array);
    <ndarray::ArrayRef<T, ndarray::Ix1>>::iter(array_ref)
}

// Optional feature-gated implementations

#[cfg(feature = "ndarray_support")]
impl<T> Data1D<T> for ndarray::Array1<T> {
    fn len(&self) -> usize {
        ndarray_len_1d(self)
    }

    fn get(&self, index: usize) -> Option<&T> {
        ndarray_get_1d(self, index)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new(ndarray_iter_1d(self))
    }

    fn as_slice(&self) -> Option<&[T]> {
        ndarray_as_slice_1d(self)
    }
}

#[cfg(feature = "ndarray_support")]
impl<'a, T> Data1D<T> for ndarray::ArrayView1<'a, T> {
    fn len(&self) -> usize {
        ndarray_len_1d(self)
    }

    fn get(&self, index: usize) -> Option<&T> {
        ndarray_get_1d(self, index)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        Box::new(ndarray_iter_1d(self))
    }

    fn as_slice(&self) -> Option<&[T]> {
        ndarray_as_slice_1d(self)
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

    fn as_slice(&self) -> Option<&[f64]> {
        // `Matrix` has an inherent `as_slice`, which method resolution would
        // pick over this trait method — but spelling the borrow as `AsRef`
        // removes the name clash from the reader's path entirely.
        Some(<Self as AsRef<[f64]>>::as_ref(self))
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

    fn as_slice(&self) -> Option<&[f64]> {
        Some(<Self as AsRef<[f64]>>::as_ref(self))
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
    T: Data1D<f64> + ?Sized,
{
    fn len(&self) -> usize {
        Data1D::len(self)
    }

    fn try_collect_f64_with_policy(
        &self,
        _null_policy: NullPolicy,
    ) -> Result<Vec<f64>, PlottingError> {
        // `f64` is the crate's primary input type, and every `Vec<f64>`,
        // `&[f64]`, `[f64; N]`, `Array1<f64>` and `DVector<f64>` reaches
        // ingestion through this impl — `.line`, `.scatter`, `.bar`,
        // `.with_yerr`, all of it. Routing a million points through
        // `Data1D::iter`'s `Box<dyn Iterator>` costs a million virtual
        // `next()` calls where one `memcpy` does the same job, so take the
        // contiguous view whenever the source has one.
        //
        // This is a fast path, not a second code path: `as_slice` is defined
        // to yield exactly the elements `iter` yields, in the same order, so
        // both arms produce identical `Vec<f64>`s and no implementor can opt
        // into different values by overriding it.
        if let Some(values) = Data1D::as_slice(self) {
            return Ok(values.to_vec());
        }
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

            impl NumericData1D for [$ty] {
                fn len(&self) -> usize {
                    <[$ty]>::len(self)
                }

                fn try_collect_f64_with_policy(
                    &self,
                    _null_policy: NullPolicy,
                ) -> Result<Vec<f64>, PlottingError> {
                    Ok(self.iter().map(|v| *v as f64).collect())
                }
            }

            #[cfg(feature = "ndarray_support")]
            impl NumericData1D for ndarray::Array1<$ty> {
                fn len(&self) -> usize {
                    ndarray_len_1d(self)
                }

                fn try_collect_f64_with_policy(
                    &self,
                    _null_policy: NullPolicy,
                ) -> Result<Vec<f64>, PlottingError> {
                    Ok(ndarray_iter_1d(self).map(|value| *value as f64).collect())
                }
            }

            #[cfg(feature = "ndarray_support")]
            impl<'a> NumericData1D for ndarray::ArrayView1<'a, $ty> {
                fn len(&self) -> usize {
                    ndarray_len_1d(self)
                }

                fn try_collect_f64_with_policy(
                    &self,
                    _null_policy: NullPolicy,
                ) -> Result<Vec<f64>, PlottingError> {
                    Ok(ndarray_iter_1d(self).map(|value| *value as f64).collect())
                }
            }
        )+
    };
}

impl_numeric_data_1d_for_primitive_collections!(
    f32, i64, i32, i16, i8, u64, u32, u16, u8, isize, usize
);

/// Fallible numeric ingestion contract for regular 2D numeric data.
pub trait NumericData2D {
    /// Returns `(rows, cols)`.
    fn shape(&self) -> (usize, usize);

    /// Convert input to row-major f64 values.
    fn try_collect_row_major_f64(&self) -> Result<Vec<f64>, PlottingError>;
}

macro_rules! impl_numeric_data_2d_for_rows {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl NumericData2D for Vec<Vec<$ty>> {
                fn shape(&self) -> (usize, usize) {
                    <[Vec<$ty>] as NumericData2D>::shape(self.as_slice())
                }

                fn try_collect_row_major_f64(&self) -> Result<Vec<f64>, PlottingError> {
                    <[Vec<$ty>] as NumericData2D>::try_collect_row_major_f64(self.as_slice())
                }
            }

            impl NumericData2D for [Vec<$ty>] {
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
                            return Err(PlottingError::RaggedData2D {
                                context: "NumericData2D",
                                row: row_idx,
                                expected_columns: cols,
                                actual_columns: row.len(),
                            });
                        }
                    }

                    let capacity = self.len().checked_mul(cols).ok_or_else(|| {
                        PlottingError::DataExtractionFailed {
                            origin: "NumericData2D".to_string(),
                            message: format!(
                                "shape {}x{} causes integer overflow",
                                self.len(),
                                cols
                            ),
                        }
                    })?;
                    let mut values = Vec::with_capacity(capacity);
                    for row in self {
                        values.extend(row.iter().map(|value| *value as f64));
                    }
                    Ok(values)
                }
            }

            impl<const ROWS: usize, const COLUMNS: usize> NumericData2D
                for [[$ty; COLUMNS]; ROWS]
            {
                fn shape(&self) -> (usize, usize) {
                    (ROWS, COLUMNS)
                }

                fn try_collect_row_major_f64(&self) -> Result<Vec<f64>, PlottingError> {
                    let capacity = ROWS.checked_mul(COLUMNS).ok_or_else(|| {
                        PlottingError::DataExtractionFailed {
                            origin: "NumericData2D".to_string(),
                            message: format!(
                                "shape {}x{} causes integer overflow",
                                ROWS, COLUMNS
                            ),
                        }
                    })?;
                    let mut values = Vec::with_capacity(capacity);
                    for row in self {
                        values.extend(row.iter().map(|value| *value as f64));
                    }
                    Ok(values)
                }
            }
        )+
    };
}

impl_numeric_data_2d_for_rows!(f64, f32, i64, i32, i16, i8, u64, u32, u16, u8, isize, usize);

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

#[cfg(feature = "ndarray_support")]
impl NumericData2D for ndarray::Array2<f32> {
    fn shape(&self) -> (usize, usize) {
        (self.nrows(), self.ncols())
    }

    fn try_collect_row_major_f64(&self) -> Result<Vec<f64>, PlottingError> {
        let (rows, cols) = (self.nrows(), self.ncols());
        let mut values = Vec::with_capacity(rows * cols);
        for r in 0..rows {
            for c in 0..cols {
                values.push(f64::from(self[(r, c)]));
            }
        }
        Ok(values)
    }
}

#[cfg(feature = "ndarray_support")]
impl<'a> NumericData2D for ndarray::ArrayView2<'a, f32> {
    fn shape(&self) -> (usize, usize) {
        (self.nrows(), self.ncols())
    }

    fn try_collect_row_major_f64(&self) -> Result<Vec<f64>, PlottingError> {
        let (rows, cols) = (self.nrows(), self.ncols());
        let mut values = Vec::with_capacity(rows * cols);
        for r in 0..rows {
            for c in 0..cols {
                values.push(f64::from(self[(r, c)]));
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
        origin: source.to_string(),
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
            origin: source.to_string(),
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
            origin: source.to_string(),
            dtype: format!("{dtype:?}"),
            expected: "numeric dtype".to_string(),
        }),
    }
}

#[cfg(feature = "polars_support")]
impl NumericData1D for polars::prelude::Series {
    fn len(&self) -> usize {
        // `Series::len` is not inherent — it arrives through
        // `Deref<Target = dyn SeriesTrait>`. A bare `self.len()` therefore
        // resolves to *this* method and recurses forever, so deref explicitly.
        (**self).len()
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

    /// Values that make an element-wise walk and a `memcpy` distinguishable:
    /// `NaN` is not equal to itself, and `-0.0 == 0.0`, so both are compared
    /// through `to_bits` below.
    const TRICKY: [f64; 7] = [
        1.5,
        -0.0,
        0.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN_POSITIVE,
    ];

    fn bits(values: &[f64]) -> Vec<u64> {
        values.iter().map(|value| value.to_bits()).collect()
    }

    /// Exactly what `try_collect_f64_with_policy` did before the contiguous
    /// fast path existed.
    fn element_wise_reference<D: Data1D<f64> + ?Sized>(data: &D) -> Vec<f64> {
        Data1D::iter(data).copied().collect()
    }

    fn assert_matches_reference<D: Data1D<f64> + NumericData1D + ?Sized>(data: &D, label: &str) {
        let collected = NumericData1D::try_collect_f64(data).unwrap();
        assert_eq!(
            bits(&collected),
            bits(&element_wise_reference(data)),
            "{label} ingested different values through the contiguous fast path"
        );
        assert_eq!(
            collected.len(),
            Data1D::len(data),
            "{label} ingested the wrong number of values"
        );
    }

    /// The contiguous fast path must be bit-identical to the boxed-iterator
    /// walk it replaced, for every f64 container the public API accepts.
    #[test]
    fn f64_ingestion_matches_the_element_wise_reference() {
        let owned: Vec<f64> = TRICKY.to_vec();
        let array: [f64; 7] = TRICKY;
        let slice: &[f64] = &owned;

        assert_matches_reference(&owned, "Vec<f64>");
        assert_matches_reference(&&owned, "&Vec<f64>");
        assert_matches_reference(&array, "[f64; N]");
        assert_matches_reference(&&array, "&[f64; N]");
        assert_matches_reference(&slice, "&[f64]");
        assert_matches_reference(slice, "[f64]");

        let empty: Vec<f64> = vec![];
        assert_matches_reference(&empty, "empty Vec<f64>");
    }

    /// The perf property itself: if any of these stops reporting a contiguous
    /// view, f64 ingestion has silently fallen back to `Box<dyn Iterator>`.
    #[test]
    fn contiguous_f64_containers_expose_a_borrowed_slice() {
        let owned: Vec<f64> = TRICKY.to_vec();
        let array: [f64; 7] = TRICKY;
        let slice: &[f64] = &owned;

        assert_eq!(
            Data1D::as_slice(&owned).map(|values| values.as_ptr()),
            Some(owned.as_ptr()),
            "Vec<f64> must hand back its own buffer, not a copy"
        );
        assert_eq!(
            Data1D::as_slice(&&owned).map(|values| values.as_ptr()),
            Some(owned.as_ptr())
        );
        assert_eq!(
            Data1D::as_slice(&array).map(|values| values.as_ptr()),
            Some(array.as_ptr())
        );
        assert_eq!(
            Data1D::as_slice(&&array).map(|values| values.as_ptr()),
            Some(array.as_ptr())
        );
        assert_eq!(
            Data1D::as_slice(&slice).map(|values| values.as_ptr()),
            Some(slice.as_ptr())
        );
        assert_eq!(
            Data1D::as_slice(slice).map(|values| values.as_ptr()),
            Some(slice.as_ptr())
        );
    }

    /// Every other element of the backing buffer — deliberately not contiguous,
    /// so it takes the default `as_slice` (`None`) and the iterator fallback.
    struct EveryOtherValue {
        backing: Vec<f64>,
    }

    impl Data1D<f64> for EveryOtherValue {
        fn len(&self) -> usize {
            self.backing.len().div_ceil(2)
        }

        fn get(&self, index: usize) -> Option<&f64> {
            self.backing.get(index * 2)
        }

        fn iter(&self) -> Box<dyn Iterator<Item = &f64> + '_> {
            Box::new(self.backing.iter().step_by(2))
        }
    }

    /// Implementors that cannot offer a contiguous view — including any
    /// downstream one written before `as_slice` existed — keep working through
    /// the iterator, unchanged.
    #[test]
    fn non_contiguous_source_falls_back_to_the_iterator() {
        let data = EveryOtherValue {
            backing: vec![1.0, 99.0, 2.0, 99.0, 3.0],
        };

        assert!(
            Data1D::as_slice(&data).is_none(),
            "the default as_slice must decline rather than invent a view"
        );
        assert_eq!(data.try_collect_f64().unwrap(), vec![1.0, 2.0, 3.0]);
        assert_eq!(NumericData1D::len(&data), 3);
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
        assert!(matches!(err, PlottingError::RaggedData2D { row: 1, .. }));
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

    /// A standard-layout `Array1` is contiguous and must take the fast path; a
    /// strided view is not, and must still ingest the same values.
    #[cfg(feature = "ndarray_support")]
    #[test]
    fn ndarray_takes_the_fast_path_only_when_contiguous() {
        use ndarray::{Array1, s};

        let expected: [f64; 5] = [1.0, 9.0, 2.0, 9.0, 3.0];
        let arr = Array1::from_vec(expected.to_vec());
        assert_eq!(
            Data1D::as_slice(&arr),
            Some(&expected[..]),
            "a standard-layout Array1<f64> must expose its buffer"
        );
        assert_eq!(arr.try_collect_f64().unwrap(), expected.to_vec());

        let strided = arr.slice(s![..;2]);
        assert!(
            Data1D::as_slice(&strided).is_none(),
            "a strided view is not contiguous and must decline the fast path"
        );
        assert_eq!(strided.try_collect_f64().unwrap(), vec![1.0, 2.0, 3.0]);
    }

    #[cfg(any(feature = "nalgebra_support", feature = "nalgebra"))]
    #[test]
    fn nalgebra_vectors_expose_a_borrowed_slice() {
        let expected: [f64; 3] = [1.0, 2.0, 3.0];

        let dynamic = nalgebra::DVector::from_vec(expected.to_vec());
        assert_eq!(Data1D::as_slice(&dynamic), Some(&expected[..]));
        assert_eq!(dynamic.try_collect_f64().unwrap(), expected.to_vec());

        let fixed = nalgebra::SVector::<f64, 3>::from(expected);
        assert_eq!(Data1D::as_slice(&fixed), Some(&expected[..]));
        assert_eq!(fixed.try_collect_f64().unwrap(), expected.to_vec());
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
