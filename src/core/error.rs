use std::fmt;

/// Result type alias for plotting operations
pub type Result<T> = std::result::Result<T, PlottingError>;

/// Comprehensive error types for plotting operations
#[derive(Debug)]
pub enum PlottingError {
    /// Data arrays have mismatched lengths
    DataLengthMismatch {
        x_len: usize,
        y_len: usize,
    },
    /// Empty data set provided
    EmptyDataSet,
    /// No data series added to plot
    NoDataSeries,
    /// Invalid color specification
    InvalidColor(String),
    /// Invalid dimensions
    InvalidDimensions {
        width: u32,
        height: u32,
    },
    /// Invalid DPI value
    InvalidDPI(u32),
    /// Invalid line width
    InvalidLineWidth(f32),
    /// Invalid alpha value
    InvalidAlpha(f32),
    /// Invalid margin value
    InvalidMargin(f32),
    /// Font not found or invalid
    FontError(String),
    /// Theme configuration error
    ThemeError(String),
    /// Rendering backend error
    RenderError(String),
    /// Export format not supported
    UnsupportedFormat(String),
    /// File I/O error
    IoError(std::io::Error),
    /// Memory allocation error
    OutOfMemory,
    /// Feature not enabled (compile-time features)
    FeatureNotEnabled {
        feature: String,
        operation: String,
    },
    /// Data contains invalid values (NaN, Inf)
    InvalidData {
        message: String,
        position: Option<usize>,
    },
    /// LaTeX rendering error (when feature enabled)
    LatexError(String),
    /// Performance limit exceeded
    PerformanceLimit {
        limit_type: String,
        actual: usize,
        maximum: usize,
    },
}

impl fmt::Display for PlottingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlottingError::DataLengthMismatch { x_len, y_len } => {
                write!(f, "Data length mismatch: x has {} elements, y has {} elements", x_len, y_len)
            }
            PlottingError::EmptyDataSet => {
                write!(f, "Empty data set provided - at least one data point is required")
            }
            PlottingError::NoDataSeries => {
                write!(f, "No data series added to plot - use line(), scatter(), or bar() to add data")
            }
            PlottingError::InvalidColor(color) => {
                write!(f, "Invalid color specification: '{}'", color)
            }
            PlottingError::InvalidDimensions { width, height } => {
                write!(f, "Invalid dimensions: {}x{} (minimum 100x100)", width, height)
            }
            PlottingError::InvalidDPI(dpi) => {
                write!(f, "Invalid DPI: {} (minimum 72)", dpi)
            }
            PlottingError::InvalidLineWidth(width) => {
                write!(f, "Invalid line width: {} (must be positive)", width)
            }
            PlottingError::InvalidAlpha(alpha) => {
                write!(f, "Invalid alpha value: {} (must be between 0.0 and 1.0)", alpha)
            }
            PlottingError::InvalidMargin(margin) => {
                write!(f, "Invalid margin: {} (must be between 0.0 and 0.5)", margin)
            }
            PlottingError::FontError(msg) => {
                write!(f, "Font error: {}", msg)
            }
            PlottingError::ThemeError(msg) => {
                write!(f, "Theme error: {}", msg)
            }
            PlottingError::RenderError(msg) => {
                write!(f, "Rendering error: {}", msg)
            }
            PlottingError::UnsupportedFormat(format) => {
                write!(f, "Unsupported export format: '{}'", format)
            }
            PlottingError::IoError(err) => {
                write!(f, "I/O error: {}", err)
            }
            PlottingError::OutOfMemory => {
                write!(f, "Out of memory during plotting operation")
            }
            PlottingError::FeatureNotEnabled { feature, operation } => {
                write!(f, "Feature '{}' not enabled - required for operation: {}", feature, operation)
            }
            PlottingError::InvalidData { message, position } => {
                match position {
                    Some(pos) => write!(f, "Invalid data at position {}: {}", pos, message),
                    None => write!(f, "Invalid data: {}", message),
                }
            }
            PlottingError::LatexError(msg) => {
                write!(f, "LaTeX rendering error: {}", msg)
            }
            PlottingError::PerformanceLimit { limit_type, actual, maximum } => {
                write!(f, "{} limit exceeded: {} (maximum {})", limit_type, actual, maximum)
            }
        }
    }
}

impl std::error::Error for PlottingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PlottingError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for PlottingError {
    fn from(err: std::io::Error) -> Self {
        PlottingError::IoError(err)
    }
}

impl From<crate::render::ColorError> for PlottingError {
    fn from(err: crate::render::ColorError) -> Self {
        PlottingError::InvalidColor(err.to_string())
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
        const MAX_DIMENSION: u32 = 16384; // 16K pixels max
        
        if width < MIN_DIMENSION || height < MIN_DIMENSION {
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
        const SOFT_LIMIT: usize = 1_000_000;    // 1M points - warning threshold
        const HARD_LIMIT: usize = 100_000_000;  // 100M points - absolute limit
        
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
        let err = PlottingError::DataLengthMismatch { x_len: 5, y_len: 3 };
        assert!(err.to_string().contains("mismatch"));
        assert!(err.to_string().contains("5"));
        assert!(err.to_string().contains("3"));
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