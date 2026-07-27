use std::fmt;
use std::sync::Arc;

/// Result type alias for plotting operations.
///
/// # Not in the prelude
///
/// This alias takes a **single** generic parameter, so glob-importing it shadows
/// [`std::result::Result`] and makes every ordinary two-parameter `Result<T, E>`
/// in the importing scope fail with `E0107`. It is therefore deliberately absent
/// from [`crate::prelude`]. Reach it explicitly:
///
/// ```rust
/// use ruviz::core::Result;
///
/// fn plot_something() -> Result<()> {
///     Ok(())
/// }
/// ```
///
/// or use [`PlotResult`], the identical alias under a name that does not collide
/// with `std`, which *is* re-exported from the prelude.
pub type Result<T> = std::result::Result<T, PlottingError>;

/// Non-colliding spelling of [`Result`]: `PlotResult<T> = Result<T, PlottingError>`.
///
/// Prefer this in code that does `use ruviz::prelude::*`, because it leaves
/// [`std::result::Result`] untouched:
///
/// ```rust
/// use ruviz::prelude::*;
///
/// fn plot_something() -> PlotResult<()> {
///     Ok(())
/// }
///
/// fn parse_something() -> Result<u32, std::num::ParseIntError> {
///     "42".parse()
/// }
/// ```
pub type PlotResult<T> = std::result::Result<T, PlottingError>;

/// Renders `Some(value)` as `{prefix}{value}{suffix}` and `None` as the empty
/// string.
///
/// Every message whose wording depends on an optional field goes through this
/// one helper, so a variant still carries its *whole* message in its own
/// `#[error(...)]` attribute instead of a hand-written `Display` arm that can
/// drift away from the variant it describes.
fn optional_clause<T: fmt::Display>(value: &Option<T>, prefix: &str, suffix: &str) -> String {
    match value {
        Some(value) => format!("{prefix}{value}{suffix}"),
        None => String::new(),
    }
}

/// Errors produced by plotting, validation, rendering, and export operations.
///
/// This enum is `#[non_exhaustive]`: new variants are added in minor releases,
/// so downstream `match` expressions must include a `_ => ...` arm. Constructing
/// and matching individual existing variants keeps working as before.
///
/// Every variant's message lives in its own `#[error(...)]` attribute, so adding
/// a variant cannot leave a message behind. The enum is [`Clone`], so a builder
/// that has to park an error until its terminal call can hold the real error
/// rather than a lossy copy of it.
#[derive(Clone, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PlottingError {
    /// Data arrays have mismatched lengths
    #[error(
        "Data length mismatch{}: x has {x_len} elements, y has {y_len} elements",
        optional_clause(.series_index, " in series ", "")
    )]
    DataLengthMismatch {
        x_len: usize,
        y_len: usize,
        series_index: Option<usize>,
    },
    /// Three-dimensional coordinate arrays have mismatched lengths.
    #[error(
        "{operation}{}: x, y, and z must have the same length (x={x_len}, y={y_len}, z={z_len})",
        optional_clause(.series_index, " series ", "")
    )]
    DataLengthMismatch3D {
        operation: &'static str,
        x_len: usize,
        y_len: usize,
        z_len: usize,
        series_index: Option<usize>,
    },
    /// A regular-grid z matrix does not match `(y.len(), x.len())`.
    #[error(
        "{operation}: z shape must be (y.len(), x.len()) = ({expected_rows}, {expected_columns}), got ({actual_rows}, {actual_columns})"
    )]
    GridShapeMismatch {
        operation: &'static str,
        expected_rows: usize,
        expected_columns: usize,
        actual_rows: usize,
        actual_columns: usize,
    },
    /// Rows in a two-dimensional numeric input have inconsistent lengths.
    #[error("{context}: row {row} has {actual_columns} values, expected {expected_columns}")]
    RaggedData2D {
        context: &'static str,
        row: usize,
        expected_columns: usize,
        actual_columns: usize,
    },
    /// Invalid 3D camera field.
    #[error("camera: {field} {reason}, got {value}")]
    InvalidCamera3D {
        field: &'static str,
        value: f32,
        reason: &'static str,
    },
    /// Invalid 3D mesh or grid topology.
    #[error("Invalid 3D topology: {reason}")]
    InvalidTopology3D { reason: String },
    /// Empty data set provided
    #[error("Empty data set provided - at least one data point is required")]
    EmptyDataSet,
    /// No data series added to plot
    #[error("No data series added to plot - use line(), scatter(), or bar() to add data")]
    NoDataSeries,
    /// Invalid color specification
    #[error("Invalid color specification: '{0}'")]
    InvalidColor(String),
    /// Invalid dimensions
    #[error("Invalid dimensions: {width}x{height} (minimum 100x100)")]
    InvalidDimensions { width: u32, height: u32 },
    /// Invalid DPI value
    #[error("Invalid DPI: {0} (minimum 72)")]
    InvalidDPI(u32),
    /// Invalid line width
    #[error("Invalid line width: {0} (must be positive)")]
    InvalidLineWidth(f32),
    /// Invalid alpha value
    #[error("Invalid alpha value: {0} (must be between 0.0 and 1.0)")]
    InvalidAlpha(f32),
    /// Invalid margin value
    #[error("Invalid margin: {0} (must be between 0.0 and 0.5)")]
    InvalidMargin(f32),
    /// Font not found or invalid
    #[error("Font error: {0}")]
    FontError(String),
    /// Theme configuration error
    #[error("Theme error: {0}")]
    ThemeError(String),
    /// Rendering backend error
    #[error("Rendering error: {0}")]
    RenderError(String),
    /// Export format not supported
    #[error("Unsupported export format: '{0}'")]
    UnsupportedFormat(String),
    /// File I/O error.
    ///
    /// Held behind an [`Arc`] so this enum stays [`Clone`]; the underlying
    /// [`std::io::Error`] is still reachable through
    /// [`std::error::Error::source`]. Build it with `PlottingError::from(err)`
    /// rather than naming the variant, so callers never spell the `Arc`.
    #[error("I/O error: {0}")]
    IoError(#[source] Arc<std::io::Error>),
    /// Memory allocation error
    #[error("Out of memory during plotting operation")]
    OutOfMemory,
    /// Feature not enabled (compile-time features)
    #[error("Feature '{feature}' not enabled - required for operation: {operation}")]
    FeatureNotEnabled { feature: String, operation: String },
    /// The operation exists but this build cannot carry it out.
    ///
    /// Distinct from [`PlottingError::FeatureNotEnabled`], which means "turn on
    /// a cargo feature": this one means the capability is genuinely absent, so
    /// no build flag will produce it. Returning it is preferable to inventing a
    /// plausible-looking answer.
    #[error("{operation} is not supported: {reason}")]
    UnsupportedOperation {
        /// The operation that was refused, e.g. `"RealTimeRenderer::get_points_in_region"`.
        operation: &'static str,
        /// Why it cannot be carried out.
        reason: String,
    },

    /// System-level error
    #[error("System error: {0}")]
    SystemError(String),
    /// Invalid input parameter
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    /// Invalid dynamic annotation value or style geometry
    #[error("Invalid annotation: {reason}")]
    InvalidAnnotation { reason: String },
    /// Annotation ID does not belong to this session or is no longer present
    #[error("Unknown annotation ID for this interactive session")]
    UnknownAnnotationId,
    /// Data contains invalid values (NaN, Inf)
    #[error("Invalid data{}: {message}", optional_clause(.position, " at position ", ""))]
    InvalidData {
        message: String,
        position: Option<usize>,
    },
    /// Unsupported data type for numeric plotting ingestion
    #[error("Unsupported data type from {origin}: {dtype} (expected {expected})")]
    DataTypeUnsupported {
        /// Name of the data source that produced the value, e.g. `"polars::Series"`.
        ///
        /// Spelled `origin` rather than `source`, which on an error type means
        /// the underlying [`std::error::Error::source`].
        origin: String,
        dtype: String,
        expected: String,
    },
    /// Null values are disallowed under current null policy
    #[error(
        "Null values are not allowed for {origin}{} ({null_count} null values found)",
        optional_clause(.column, ".", "")
    )]
    NullValueNotAllowed {
        /// Name of the data source that produced the nulls.
        origin: String,
        column: Option<String>,
        null_count: usize,
    },
    /// Failed to extract numeric values from an external data source
    #[error("Failed to extract numeric data from {origin}: {message}")]
    DataExtractionFailed {
        /// Name of the data source the extraction was attempted against.
        origin: String,
        message: String,
    },
    /// LaTeX rendering error (when feature enabled)
    #[error("LaTeX rendering error: {0}")]
    LatexError(String),
    /// Typst rendering error
    #[error("Typst rendering error: {0}")]
    TypstError(String),
    /// Performance limit exceeded
    #[error("{limit_type} limit exceeded: {actual} (maximum {maximum})")]
    PerformanceLimit {
        limit_type: String,
        actual: usize,
        maximum: usize,
    },

    // DataShader-specific errors
    /// DataShader initialization failed
    #[error("DataShader error: {message}{}", optional_clause(.cause, " (cause: ", ")"))]
    DataShaderError {
        message: String,
        cause: Option<String>,
    },
    /// Aggregation operation failed
    #[error("Aggregation '{operation}' failed on {data_points} points: {error}")]
    AggregationError {
        operation: String,
        data_points: usize,
        error: String,
    },
    /// Canvas resolution too high for DataShader
    #[error("DataShader canvas {width}x{height} exceeds maximum {max_pixels} pixels")]
    DataShaderCanvasError {
        width: u32,
        height: u32,
        max_pixels: usize,
    },
    /// Atomic operation failed in parallel aggregation
    #[error("Atomic operation error: {0}")]
    AtomicOperationError(String),

    // Parallel rendering errors
    /// Parallel rendering initialization failed
    #[error("Parallel rendering failed with {threads} threads: {error}")]
    ParallelRenderError { threads: usize, error: String },
    /// Thread pool configuration error
    #[error("Thread pool error: {0}")]
    ThreadPoolError(String),
    /// Parallel task synchronization error
    #[error("Thread synchronization error: {0}")]
    SynchronizationError(String),
    /// Work stealing queue error
    #[error("Work stealing queue error: {0}")]
    WorkStealingError(String),

    // GPU acceleration errors
    /// GPU backend not available
    #[error("GPU not available: {0}")]
    GpuNotAvailable(String),
    /// GPU initialization failed
    #[error("GPU initialization failed for {backend}: {error}")]
    GpuInitError { backend: String, error: String },
    /// GPU memory allocation failed
    #[error(
        "GPU memory allocation failed: requested {requested} bytes{}",
        optional_clause(.available, ", only ", " available")
    )]
    GpuMemoryError {
        requested: usize,
        available: Option<usize>,
    },
    /// GPU shader compilation failed
    #[error("Shader compilation failed for {shader_type}: {error}")]
    ShaderError { shader_type: String, error: String },
    /// GPU buffer operation failed
    #[error("GPU buffer error: {0}")]
    BufferError(String),
    /// GPU command submission failed
    #[error("GPU command error: {0}")]
    CommandError(String),
    /// GPU device lost
    #[error("GPU device lost - try restarting the application")]
    DeviceLost,
    /// GPU feature not supported
    #[error("GPU feature '{0}' not supported on this device")]
    UnsupportedGpuFeature(String),
    /// GPU operation timeout
    #[error("GPU operation timed out")]
    GpuTimeoutError,

    // SIMD optimization errors
    /// SIMD feature not available on this CPU
    #[error("SIMD instructions not available on this CPU")]
    SimdNotAvailable,
    /// SIMD operation alignment error
    #[error("SIMD alignment error: required {required}-byte alignment, got {actual}")]
    SimdAlignmentError { required: usize, actual: usize },

    // Memory pool errors
    /// Memory pool initialization failed
    #[error("Memory pool initialization failed: {0}")]
    PoolInitError(String),
    /// Memory pool exhausted
    #[error("{pool_type} pool exhausted: {requested} bytes requested")]
    PoolExhausted { pool_type: String, requested: usize },
    /// Memory pool corruption detected
    #[error("Memory pool corruption detected: {0}")]
    PoolCorruption(String),
}

impl From<std::io::Error> for PlottingError {
    fn from(err: std::io::Error) -> Self {
        PlottingError::IoError(Arc::new(err))
    }
}

impl From<crate::render::ColorError> for PlottingError {
    fn from(err: crate::render::ColorError) -> Self {
        PlottingError::InvalidColor(err.to_string())
    }
}

#[cfg(feature = "gpu")]
impl From<crate::render::gpu::GpuError> for PlottingError {
    fn from(err: crate::render::gpu::GpuError) -> Self {
        use crate::render::gpu::GpuError;
        match err {
            GpuError::InitializationFailed(msg) => PlottingError::GpuInitError {
                backend: "wgpu".to_string(),
                error: msg,
            },
            GpuError::BufferCreationFailed(msg) => PlottingError::BufferError(msg),
            GpuError::BufferOperationFailed(msg) => PlottingError::BufferError(msg),
            GpuError::OperationFailed(msg) => PlottingError::CommandError(msg),
        }
    }
}

// Helper functions for common validation
impl PlottingError {
    /// Check if data contains invalid values (NaN, Inf)
    pub fn validate_data(data: &[f64]) -> Result<()> {
        for (i, &value) in data.iter().enumerate() {
            if value.is_nan() {
                return Err(PlottingError::InvalidData {
                    message: "NaN value found in data".to_string(),
                    position: Some(i),
                });
            }
            if value.is_infinite() {
                return Err(PlottingError::InvalidData {
                    message: format!("Infinite value ({}) found in data", value),
                    position: Some(i),
                });
            }
        }
        Ok(())
    }

    /// Validate dimensions are reasonable
    pub fn validate_dimensions(width: u32, height: u32) -> Result<()> {
        const MIN_DIMENSION: u32 = 100;

        if width < MIN_DIMENSION || height < MIN_DIMENSION {
            return Err(PlottingError::InvalidDimensions { width, height });
        }

        Self::validate_subplot_dimensions(width, height)
    }

    /// Validate child subplot dimensions without the top-level 100-pixel minimum.
    pub(crate) fn validate_subplot_dimensions(width: u32, height: u32) -> Result<()> {
        const MAX_DIMENSION: u32 = 16384; // 16K pixels max

        if width == 0 || height == 0 {
            return Err(PlottingError::InvalidDimensions { width, height });
        }

        if width > MAX_DIMENSION || height > MAX_DIMENSION {
            return Err(PlottingError::PerformanceLimit {
                limit_type: "Image dimension".to_string(),
                actual: width.max(height) as usize,
                maximum: MAX_DIMENSION as usize,
            });
        }

        Ok(())
    }

    /// Validate DPI is reasonable
    pub fn validate_dpi(dpi: u32) -> Result<()> {
        const MIN_DPI: u32 = 72;
        const MAX_DPI: u32 = 2400; // Reasonable maximum for print

        if dpi < MIN_DPI {
            return Err(PlottingError::InvalidDPI(dpi));
        }

        if dpi > MAX_DPI {
            return Err(PlottingError::PerformanceLimit {
                limit_type: "DPI".to_string(),
                actual: dpi as usize,
                maximum: MAX_DPI as usize,
            });
        }

        Ok(())
    }

    /// Check for performance limits on data size
    pub fn check_performance_limits(data_points: usize) -> Result<()> {
        // These limits are approximate and can be adjusted based on performance testing
        const SOFT_LIMIT: usize = 1_000_000; // 1M points - warning threshold
        const HARD_LIMIT: usize = 100_000_000; // 100M points - absolute limit

        if data_points > HARD_LIMIT {
            return Err(PlottingError::PerformanceLimit {
                limit_type: "Data points".to_string(),
                actual: data_points,
                maximum: HARD_LIMIT,
            });
        }

        // Could add warning mechanism here for SOFT_LIMIT
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_error_display() {
        let err = PlottingError::DataLengthMismatch {
            x_len: 5,
            y_len: 3,
            series_index: None,
        };
        assert!(err.to_string().contains("mismatch"));
        assert!(err.to_string().contains("5"));
        assert!(err.to_string().contains("3"));
    }

    #[test]
    fn optional_clauses_read_the_same_with_and_without_their_field() {
        // The six variants whose wording depends on an optional field all go
        // through `optional_clause`. Both spellings of each are pinned here so
        // the one shared mechanism cannot quietly change any of them.
        assert_eq!(
            PlottingError::DataLengthMismatch {
                x_len: 5,
                y_len: 3,
                series_index: Some(2),
            }
            .to_string(),
            "Data length mismatch in series 2: x has 5 elements, y has 3 elements"
        );
        assert_eq!(
            PlottingError::DataLengthMismatch {
                x_len: 5,
                y_len: 3,
                series_index: None,
            }
            .to_string(),
            "Data length mismatch: x has 5 elements, y has 3 elements"
        );

        assert_eq!(
            PlottingError::DataLengthMismatch3D {
                operation: "scatter3",
                x_len: 1,
                y_len: 2,
                z_len: 3,
                series_index: Some(4),
            }
            .to_string(),
            "scatter3 series 4: x, y, and z must have the same length (x=1, y=2, z=3)"
        );
        assert_eq!(
            PlottingError::DataLengthMismatch3D {
                operation: "scatter3",
                x_len: 1,
                y_len: 2,
                z_len: 3,
                series_index: None,
            }
            .to_string(),
            "scatter3: x, y, and z must have the same length (x=1, y=2, z=3)"
        );

        assert_eq!(
            PlottingError::InvalidData {
                message: "NaN value found in data".to_string(),
                position: Some(7),
            }
            .to_string(),
            "Invalid data at position 7: NaN value found in data"
        );
        assert_eq!(
            PlottingError::InvalidData {
                message: "NaN value found in data".to_string(),
                position: None,
            }
            .to_string(),
            "Invalid data: NaN value found in data"
        );

        assert_eq!(
            PlottingError::NullValueNotAllowed {
                origin: "polars::DataFrame".to_string(),
                column: Some("price".to_string()),
                null_count: 3,
            }
            .to_string(),
            "Null values are not allowed for polars::DataFrame.price (3 null values found)"
        );
        assert_eq!(
            PlottingError::NullValueNotAllowed {
                origin: "polars::Series".to_string(),
                column: None,
                null_count: 3,
            }
            .to_string(),
            "Null values are not allowed for polars::Series (3 null values found)"
        );

        assert_eq!(
            PlottingError::DataShaderError {
                message: "canvas init".to_string(),
                cause: Some("no device".to_string()),
            }
            .to_string(),
            "DataShader error: canvas init (cause: no device)"
        );
        assert_eq!(
            PlottingError::DataShaderError {
                message: "canvas init".to_string(),
                cause: None,
            }
            .to_string(),
            "DataShader error: canvas init"
        );

        assert_eq!(
            PlottingError::GpuMemoryError {
                requested: 1024,
                available: Some(512),
            }
            .to_string(),
            "GPU memory allocation failed: requested 1024 bytes, only 512 available"
        );
        assert_eq!(
            PlottingError::GpuMemoryError {
                requested: 1024,
                available: None,
            }
            .to_string(),
            "GPU memory allocation failed: requested 1024 bytes"
        );
    }

    #[test]
    fn derived_messages_match_the_wording_callers_depend_on() {
        // A sample across the variant shapes - struct, tuple, and unit - so a
        // careless edit to an `#[error(...)]` attribute is caught here rather
        // than in a downstream doc.
        assert_eq!(
            PlottingError::GridShapeMismatch {
                operation: "surface",
                expected_rows: 2,
                expected_columns: 3,
                actual_rows: 2,
                actual_columns: 2,
            }
            .to_string(),
            "surface: z shape must be (y.len(), x.len()) = (2, 3), got (2, 2)"
        );
        assert_eq!(
            PlottingError::RaggedData2D {
                context: "surface",
                row: 1,
                expected_columns: 3,
                actual_columns: 2,
            }
            .to_string(),
            "surface: row 1 has 2 values, expected 3"
        );
        assert_eq!(
            PlottingError::InvalidCamera3D {
                field: "fov",
                value: 0.0,
                reason: "must be positive",
            }
            .to_string(),
            "camera: fov must be positive, got 0"
        );
        assert_eq!(
            PlottingError::DataExtractionFailed {
                origin: "ruviz::plot-ingestion".to_string(),
                message: "forced ingestion failure".to_string(),
            }
            .to_string(),
            "Failed to extract numeric data from ruviz::plot-ingestion: forced ingestion failure"
        );
        assert_eq!(
            PlottingError::DataTypeUnsupported {
                origin: "polars::Series".to_string(),
                dtype: "Utf8".to_string(),
                expected: "numeric dtype".to_string(),
            }
            .to_string(),
            "Unsupported data type from polars::Series: Utf8 (expected numeric dtype)"
        );
        assert_eq!(
            PlottingError::InvalidDimensions {
                width: 10,
                height: 20,
            }
            .to_string(),
            "Invalid dimensions: 10x20 (minimum 100x100)"
        );
        assert_eq!(
            PlottingError::UnsupportedFormat("gif".to_string()).to_string(),
            "Unsupported export format: 'gif'"
        );
        assert_eq!(
            PlottingError::NoDataSeries.to_string(),
            "No data series added to plot - use line(), scatter(), or bar() to add data"
        );
        assert_eq!(
            PlottingError::EmptyDataSet.to_string(),
            "Empty data set provided - at least one data point is required"
        );
    }

    #[test]
    fn test_data_validation() {
        // Valid data
        let valid_data = vec![1.0, 2.0, 3.0, 4.0];
        assert!(PlottingError::validate_data(&valid_data).is_ok());

        // Data with NaN
        let nan_data = vec![1.0, f64::NAN, 3.0];
        assert!(PlottingError::validate_data(&nan_data).is_err());

        // Data with infinity
        let inf_data = vec![1.0, f64::INFINITY, 3.0];
        assert!(PlottingError::validate_data(&inf_data).is_err());
    }

    #[test]
    fn test_dimension_validation() {
        // Valid dimensions
        assert!(PlottingError::validate_dimensions(800, 600).is_ok());

        // Too small
        assert!(PlottingError::validate_dimensions(50, 50).is_err());

        // Too large
        assert!(PlottingError::validate_dimensions(20000, 20000).is_err());

        // Child subplots accept any positive size but retain the maximum.
        assert!(PlottingError::validate_subplot_dimensions(1, 1).is_ok());
        assert!(PlottingError::validate_subplot_dimensions(0, 1).is_err());
        assert!(PlottingError::validate_subplot_dimensions(20000, 1).is_err());
    }

    #[test]
    fn test_dpi_validation() {
        // Valid DPI
        assert!(PlottingError::validate_dpi(300).is_ok());

        // Too low
        assert!(PlottingError::validate_dpi(50).is_err());

        // Too high
        assert!(PlottingError::validate_dpi(5000).is_err());
    }

    #[test]
    fn test_performance_limits() {
        // Reasonable size
        assert!(PlottingError::check_performance_limits(10000).is_ok());

        // Large but acceptable
        assert!(PlottingError::check_performance_limits(1_000_000).is_ok());

        // Too large
        assert!(PlottingError::check_performance_limits(200_000_000).is_err());
    }

    #[test]
    fn test_error_source() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let plot_err = PlottingError::from(io_err);

        assert!(plot_err.source().is_some());
        assert_eq!(plot_err.to_string(), "I/O error: file not found");
    }

    #[test]
    fn an_io_error_survives_being_cloned() {
        // `Plot`'s deferred-ingestion slot holds a real `PlottingError`, which
        // only works while every variant - including the one wrapping
        // `std::io::Error` - can be cloned without losing its identity.
        let original = PlottingError::from(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "denied",
        ));
        let copy = original.clone();

        assert!(matches!(copy, PlottingError::IoError(_)));
        assert_eq!(copy.to_string(), original.to_string());
        assert_eq!(
            copy.source().map(ToString::to_string),
            Some("denied".to_string())
        );
    }

    #[test]
    fn a_data_origin_is_a_name_and_never_an_error_source() {
        // The ingestion variants carry the *name* of the data source, so they
        // have no underlying error to report. The field is spelled `origin`
        // precisely so it is never mistaken for `Error::source`.
        let err = PlottingError::DataExtractionFailed {
            origin: "NumericData2D".to_string(),
            message: "shape 2x3 does not match collected length 5".to_string(),
        };

        assert!(err.source().is_none());
    }

    #[test]
    fn test_non_exhaustive_downstream_match_shape() {
        // `#[non_exhaustive]` cannot be observed from inside the defining crate,
        // so this pins the pattern downstream crates must use: name the variants
        // they care about and fall back to a wildcard arm.
        let err = PlottingError::EmptyDataSet;
        let described = match err {
            PlottingError::EmptyDataSet => "empty",
            PlottingError::NoDataSeries => "no series",
            _ => "other",
        };

        assert_eq!(described, "empty");
    }

    #[test]
    fn plot_result_alias_matches_result_alias() {
        // `PlotResult<T>` exists so downstream code can `use ruviz::prelude::*`
        // without shadowing `std::result::Result`. Pin that the two aliases are
        // the same type so they can never drift apart.
        fn as_result(value: PlotResult<u32>) -> PlotResult<u32> {
            value
        }

        assert_eq!(as_result(Ok(7)).unwrap(), 7);
        assert!(as_result(Err(PlottingError::EmptyDataSet)).is_err());

        // And that ordinary two-parameter `Result` still works alongside it.
        let parsed: std::result::Result<u32, std::num::ParseIntError> = "42".parse();
        assert_eq!(parsed.unwrap(), 42);
    }

    #[test]
    fn test_color_error_conversion() {
        let color_err = crate::render::ColorError::InvalidHex;
        let plot_err = PlottingError::from(color_err);

        match plot_err {
            PlottingError::InvalidColor(_) => (),
            _ => panic!("Expected InvalidColor"),
        }
    }
}
