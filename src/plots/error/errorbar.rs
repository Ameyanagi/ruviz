//! Error bar implementations
//!
//! Provides error bar visualization for uncertainty representation.

use crate::render::Color;

/// Configuration for error bars
#[derive(Debug, Clone)]
pub struct ErrorBarConfig {
    /// Color for error bars
    pub color: Option<Color>,
    /// Line width for error bar lines
    pub line_width: f32,
    /// Cap width as fraction of data spacing
    pub cap_size: f64,
    /// Line style for error bars
    pub line_style: ErrorLineStyle,
    /// Whether to show horizontal error bars
    pub xerr: bool,
    /// Whether to show vertical error bars
    pub yerr: bool,
    /// Alpha transparency
    pub alpha: f32,
}

/// Line style for error bars
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorLineStyle {
    Solid,
    Dashed,
}

impl Default for ErrorBarConfig {
    fn default() -> Self {
        Self {
            color: None,
            line_width: 1.5,
            cap_size: 0.1,
            line_style: ErrorLineStyle::Solid,
            xerr: false,
            yerr: true,
            alpha: 1.0,
        }
    }
}

impl ErrorBarConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set color
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set line width
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width.max(0.1);
        self
    }

    /// Set cap size
    pub fn cap_size(mut self, size: f64) -> Self {
        self.cap_size = size.max(0.0);
        self
    }

    /// Enable x error bars
    pub fn xerr(mut self, enable: bool) -> Self {
        self.xerr = enable;
        self
    }

    /// Enable y error bars
    pub fn yerr(mut self, enable: bool) -> Self {
        self.yerr = enable;
        self
    }

    /// Set alpha
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }
}

/// Error values for asymmetric errors
#[derive(Debug, Clone)]
pub enum ErrorValues {
    /// Symmetric error (same in both directions)
    Symmetric(Vec<f64>),
    /// Asymmetric error (lower, upper)
    Asymmetric(Vec<f64>, Vec<f64>),
}

impl ErrorValues {
    /// Create symmetric error values
    pub fn symmetric(errors: Vec<f64>) -> Self {
        Self::Symmetric(errors)
    }

    /// Create asymmetric error values
    pub fn asymmetric(lower: Vec<f64>, upper: Vec<f64>) -> Self {
        Self::Asymmetric(lower, upper)
    }

    /// Get error bounds at index
    pub fn bounds_at(&self, idx: usize) -> Option<(f64, f64)> {
        match self {
            Self::Symmetric(errors) => errors.get(idx).map(|&e| (e, e)),
            Self::Asymmetric(lower, upper) => match (lower.get(idx), upper.get(idx)) {
                (Some(&l), Some(&u)) => Some((l, u)),
                _ => None,
            },
        }
    }

    /// Get length
    pub fn len(&self) -> usize {
        match self {
            Self::Symmetric(errors) => errors.len(),
            Self::Asymmetric(lower, upper) => lower.len().min(upper.len()),
        }
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A single error bar with endpoints
#[derive(Debug, Clone, Copy)]
pub struct ErrorBar {
    /// Center X position
    pub x: f64,
    /// Center Y position
    pub y: f64,
    /// Lower X bound (for x errors)
    pub x_lower: f64,
    /// Upper X bound (for x errors)
    pub x_upper: f64,
    /// Lower Y bound (for y errors)
    pub y_lower: f64,
    /// Upper Y bound (for y errors)
    pub y_upper: f64,
}

impl ErrorBar {
    /// Create error bar with symmetric y-error
    pub fn symmetric_y(x: f64, y: f64, yerr: f64) -> Self {
        Self {
            x,
            y,
            x_lower: x,
            x_upper: x,
            y_lower: y - yerr,
            y_upper: y + yerr,
        }
    }

    /// Create error bar with symmetric x-error
    pub fn symmetric_x(x: f64, y: f64, xerr: f64) -> Self {
        Self {
            x,
            y,
            x_lower: x - xerr,
            x_upper: x + xerr,
            y_lower: y,
            y_upper: y,
        }
    }

    /// Create error bar with asymmetric y-error
    pub fn asymmetric_y(x: f64, y: f64, yerr_lower: f64, yerr_upper: f64) -> Self {
        Self {
            x,
            y,
            x_lower: x,
            x_upper: x,
            y_lower: y - yerr_lower,
            y_upper: y + yerr_upper,
        }
    }

    /// Create error bar with both x and y errors
    pub fn full(
        x: f64,
        y: f64,
        xerr_lower: f64,
        xerr_upper: f64,
        yerr_lower: f64,
        yerr_upper: f64,
    ) -> Self {
        Self {
            x,
            y,
            x_lower: x - xerr_lower,
            x_upper: x + xerr_upper,
            y_lower: y - yerr_lower,
            y_upper: y + yerr_upper,
        }
    }

    /// Get Y-error bar line endpoints
    pub fn y_line(&self) -> ((f64, f64), (f64, f64)) {
        ((self.x, self.y_lower), (self.x, self.y_upper))
    }

    /// Get X-error bar line endpoints
    pub fn x_line(&self) -> ((f64, f64), (f64, f64)) {
        ((self.x_lower, self.y), (self.x_upper, self.y))
    }

    /// Get Y-error cap endpoints (horizontal lines at top and bottom)
    #[allow(clippy::type_complexity)]
    pub fn y_caps(&self, cap_size: f64) -> (((f64, f64), (f64, f64)), ((f64, f64), (f64, f64))) {
        let half_cap = cap_size / 2.0;
        let lower_cap = (
            (self.x - half_cap, self.y_lower),
            (self.x + half_cap, self.y_lower),
        );
        let upper_cap = (
            (self.x - half_cap, self.y_upper),
            (self.x + half_cap, self.y_upper),
        );
        (lower_cap, upper_cap)
    }

    /// Get X-error cap endpoints (vertical lines at left and right)
    #[allow(clippy::type_complexity)]
    pub fn x_caps(&self, cap_size: f64) -> (((f64, f64), (f64, f64)), ((f64, f64), (f64, f64))) {
        let half_cap = cap_size / 2.0;
        let left_cap = (
            (self.x_lower, self.y - half_cap),
            (self.x_lower, self.y + half_cap),
        );
        let right_cap = (
            (self.x_upper, self.y - half_cap),
            (self.x_upper, self.y + half_cap),
        );
        (left_cap, right_cap)
    }
}

/// Compute error bars from data
///
/// # Arguments
/// * `x` - X coordinates
/// * `y` - Y coordinates
/// * `yerr` - Y error values (optional)
/// * `xerr` - X error values (optional)
///
/// # Returns
/// Vec of ErrorBar structs
pub fn compute_error_bars(
    x: &[f64],
    y: &[f64],
    yerr: Option<&ErrorValues>,
    xerr: Option<&ErrorValues>,
) -> Vec<ErrorBar> {
    let n = x.len().min(y.len());
    let mut bars = Vec::with_capacity(n);

    for i in 0..n {
        let (yerr_lower, yerr_upper) = yerr.and_then(|e| e.bounds_at(i)).unwrap_or((0.0, 0.0));

        let (xerr_lower, xerr_upper) = xerr.and_then(|e| e.bounds_at(i)).unwrap_or((0.0, 0.0));

        bars.push(ErrorBar::full(
            x[i], y[i], xerr_lower, xerr_upper, yerr_lower, yerr_upper,
        ));
    }

    bars
}

/// Compute data range including error bars
///
/// # Returns
/// ((x_min, x_max), (y_min, y_max))
pub fn error_bar_range(bars: &[ErrorBar]) -> ((f64, f64), (f64, f64)) {
    if bars.is_empty() {
        return ((0.0, 1.0), (0.0, 1.0));
    }

    let mut x_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;

    for bar in bars {
        x_min = x_min.min(bar.x_lower);
        x_max = x_max.max(bar.x_upper);
        y_min = y_min.min(bar.y_lower);
        y_max = y_max.max(bar.y_upper);
    }

    ((x_min, x_max), (y_min, y_max))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symmetric_y_error() {
        let bar = ErrorBar::symmetric_y(1.0, 5.0, 0.5);
        assert!((bar.y_lower - 4.5).abs() < 1e-10);
        assert!((bar.y_upper - 5.5).abs() < 1e-10);
    }

    #[test]
    fn test_asymmetric_y_error() {
        let bar = ErrorBar::asymmetric_y(1.0, 5.0, 0.3, 0.7);
        assert!((bar.y_lower - 4.7).abs() < 1e-10);
        assert!((bar.y_upper - 5.7).abs() < 1e-10);
    }

    #[test]
    fn test_error_values_symmetric() {
        let errors = ErrorValues::symmetric(vec![0.5, 1.0]);
        let (lower, upper) = errors.bounds_at(0).unwrap();
        assert!((lower - 0.5).abs() < 1e-10);
        assert!((upper - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_error_values_asymmetric() {
        let errors = ErrorValues::asymmetric(vec![0.3], vec![0.7]);
        let (lower, upper) = errors.bounds_at(0).unwrap();
        assert!((lower - 0.3).abs() < 1e-10);
        assert!((upper - 0.7).abs() < 1e-10);
    }

    #[test]
    fn test_compute_error_bars() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![10.0, 20.0, 15.0];
        let yerr = ErrorValues::symmetric(vec![1.0, 2.0, 1.5]);

        let bars = compute_error_bars(&x, &y, Some(&yerr), None);
        assert_eq!(bars.len(), 3);
        assert!((bars[0].y_lower - 9.0).abs() < 1e-10);
        assert!((bars[0].y_upper - 11.0).abs() < 1e-10);
    }

    #[test]
    fn test_error_bar_range() {
        let bars = vec![
            ErrorBar::symmetric_y(1.0, 5.0, 0.5),
            ErrorBar::symmetric_y(2.0, 10.0, 1.0),
        ];

        let ((x_min, x_max), (y_min, y_max)) = error_bar_range(&bars);
        assert!((x_min - 1.0).abs() < 1e-10);
        assert!((x_max - 2.0).abs() < 1e-10);
        assert!((y_min - 4.5).abs() < 1e-10);
        assert!((y_max - 11.0).abs() < 1e-10);
    }
}
