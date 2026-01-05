//! Error bar implementations
//!
//! Provides error bar visualization for uncertainty representation.
//!
//! # Trait-Based API
//!
//! Error bar plots implement the core plot traits:
//! - [`PlotConfig`] for `ErrorBarConfig`
//! - [`PlotCompute`] for `ErrorBarPlot` marker struct
//! - [`PlotData`] for `ErrorBarData`
//! - [`PlotRender`] for `ErrorBarData`

use crate::core::Result;
use crate::plots::traits::{PlotArea, PlotCompute, PlotConfig, PlotData, PlotRender};
use crate::render::skia::SkiaRenderer;
use crate::render::{Color, LineStyle, Theme};

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

// Implement PlotConfig marker trait
impl PlotConfig for ErrorBarConfig {}

/// Marker struct for ErrorBar plot type
pub struct ErrorBarPlot;

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

// ============================================================================
// Trait-Based API
// ============================================================================

/// Computed error bar plot data
#[derive(Debug, Clone)]
pub struct ErrorBarData {
    /// All error bars
    pub bars: Vec<ErrorBar>,
    /// Data bounds including error extents
    pub bounds: ((f64, f64), (f64, f64)),
    /// Configuration used
    pub(crate) config: ErrorBarConfig,
}

/// Input for error bar plot computation
pub struct ErrorBarInput<'a> {
    /// X coordinates
    pub x: &'a [f64],
    /// Y coordinates
    pub y: &'a [f64],
    /// Y error values (optional)
    pub yerr: Option<&'a ErrorValues>,
    /// X error values (optional)
    pub xerr: Option<&'a ErrorValues>,
}

impl<'a> ErrorBarInput<'a> {
    /// Create new error bar input with y errors
    pub fn new(x: &'a [f64], y: &'a [f64], yerr: &'a ErrorValues) -> Self {
        Self {
            x,
            y,
            yerr: Some(yerr),
            xerr: None,
        }
    }

    /// Create error bar input with both x and y errors
    pub fn with_both(
        x: &'a [f64],
        y: &'a [f64],
        yerr: &'a ErrorValues,
        xerr: &'a ErrorValues,
    ) -> Self {
        Self {
            x,
            y,
            yerr: Some(yerr),
            xerr: Some(xerr),
        }
    }

    /// Create error bar input with x errors only
    pub fn x_only(x: &'a [f64], y: &'a [f64], xerr: &'a ErrorValues) -> Self {
        Self {
            x,
            y,
            yerr: None,
            xerr: Some(xerr),
        }
    }
}

impl PlotCompute for ErrorBarPlot {
    type Input<'a> = ErrorBarInput<'a>;
    type Config = ErrorBarConfig;
    type Output = ErrorBarData;

    fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output> {
        if input.x.is_empty() || input.y.is_empty() {
            return Err(crate::core::PlottingError::EmptyDataSet);
        }

        let bars = compute_error_bars(input.x, input.y, input.yerr, input.xerr);
        let bounds = error_bar_range(&bars);

        Ok(ErrorBarData {
            bars,
            bounds,
            config: config.clone(),
        })
    }
}

impl PlotData for ErrorBarData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        self.bounds
    }

    fn is_empty(&self) -> bool {
        self.bars.is_empty()
    }
}

impl PlotRender for ErrorBarData {
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        _theme: &Theme,
        color: Color,
    ) -> Result<()> {
        if self.bars.is_empty() {
            return Ok(());
        }

        let config = &self.config;
        let bar_color = config.color.unwrap_or(color).with_alpha(config.alpha);

        // Convert ErrorLineStyle to LineStyle (using a helper fn to avoid move issues)
        let get_line_style = || match config.line_style {
            ErrorLineStyle::Solid => LineStyle::Solid,
            ErrorLineStyle::Dashed => LineStyle::Dashed,
        };

        // Calculate cap size in data units
        let ((x_min, x_max), _) = self.bounds;
        let x_range = x_max - x_min;
        let cap_size = x_range * config.cap_size;

        for bar in &self.bars {
            // Draw Y error bar if enabled
            if config.yerr {
                let ((x1, y1), (x2, y2)) = bar.y_line();
                let (sx1, sy1) = area.data_to_screen(x1, y1);
                let (sx2, sy2) = area.data_to_screen(x2, y2);
                renderer.draw_line(
                    sx1,
                    sy1,
                    sx2,
                    sy2,
                    bar_color,
                    config.line_width,
                    get_line_style(),
                )?;

                // Draw caps
                let (lower_cap, upper_cap) = bar.y_caps(cap_size);
                let (lc1, lc2) = lower_cap;
                let (uc1, uc2) = upper_cap;

                let (slc1x, slc1y) = area.data_to_screen(lc1.0, lc1.1);
                let (slc2x, slc2y) = area.data_to_screen(lc2.0, lc2.1);
                renderer.draw_line(
                    slc1x,
                    slc1y,
                    slc2x,
                    slc2y,
                    bar_color,
                    config.line_width,
                    get_line_style(),
                )?;

                let (suc1x, suc1y) = area.data_to_screen(uc1.0, uc1.1);
                let (suc2x, suc2y) = area.data_to_screen(uc2.0, uc2.1);
                renderer.draw_line(
                    suc1x,
                    suc1y,
                    suc2x,
                    suc2y,
                    bar_color,
                    config.line_width,
                    get_line_style(),
                )?;
            }

            // Draw X error bar if enabled
            if config.xerr {
                let ((x1, y1), (x2, y2)) = bar.x_line();
                let (sx1, sy1) = area.data_to_screen(x1, y1);
                let (sx2, sy2) = area.data_to_screen(x2, y2);
                renderer.draw_line(
                    sx1,
                    sy1,
                    sx2,
                    sy2,
                    bar_color,
                    config.line_width,
                    get_line_style(),
                )?;

                // Draw caps
                let (left_cap, right_cap) = bar.x_caps(cap_size);
                let (lc1, lc2) = left_cap;
                let (rc1, rc2) = right_cap;

                let (slc1x, slc1y) = area.data_to_screen(lc1.0, lc1.1);
                let (slc2x, slc2y) = area.data_to_screen(lc2.0, lc2.1);
                renderer.draw_line(
                    slc1x,
                    slc1y,
                    slc2x,
                    slc2y,
                    bar_color,
                    config.line_width,
                    get_line_style(),
                )?;

                let (src1x, src1y) = area.data_to_screen(rc1.0, rc1.1);
                let (src2x, src2y) = area.data_to_screen(rc2.0, rc2.1);
                renderer.draw_line(
                    src1x,
                    src1y,
                    src2x,
                    src2y,
                    bar_color,
                    config.line_width,
                    get_line_style(),
                )?;
            }
        }

        Ok(())
    }
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

    #[test]
    fn test_errorbar_config_implements_plot_config() {
        fn assert_plot_config<T: PlotConfig>() {}
        assert_plot_config::<ErrorBarConfig>();
    }

    #[test]
    fn test_errorbar_plot_compute_trait() {
        use crate::plots::traits::PlotCompute;

        let x = vec![1.0, 2.0, 3.0];
        let y = vec![10.0, 20.0, 15.0];
        let yerr = ErrorValues::symmetric(vec![1.0, 2.0, 1.5]);
        let config = ErrorBarConfig::default();
        let input = ErrorBarInput::new(&x, &y, &yerr);
        let result = ErrorBarPlot::compute(input, &config);

        assert!(result.is_ok());
        let error_data = result.unwrap();
        assert_eq!(error_data.bars.len(), 3);
    }

    #[test]
    fn test_errorbar_plot_compute_empty() {
        use crate::plots::traits::PlotCompute;

        let x: Vec<f64> = vec![];
        let y: Vec<f64> = vec![];
        let yerr = ErrorValues::symmetric(vec![]);
        let config = ErrorBarConfig::default();
        let input = ErrorBarInput::new(&x, &y, &yerr);
        let result = ErrorBarPlot::compute(input, &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_errorbar_plot_data_trait() {
        use crate::plots::traits::{PlotCompute, PlotData};

        let x = vec![1.0, 2.0, 3.0];
        let y = vec![10.0, 20.0, 15.0];
        let yerr = ErrorValues::symmetric(vec![1.0, 2.0, 1.5]);
        let config = ErrorBarConfig::default();
        let input = ErrorBarInput::new(&x, &y, &yerr);
        let error_data = ErrorBarPlot::compute(input, &config).unwrap();

        // Test data_bounds
        let ((x_min, x_max), (y_min, y_max)) = error_data.data_bounds();
        assert!((x_min - 1.0).abs() < 1e-10);
        assert!((x_max - 3.0).abs() < 1e-10);
        assert!(y_min <= y_max);

        // Test is_empty
        assert!(!error_data.is_empty());
    }

    #[test]
    fn test_errorbar_with_both_errors() {
        use crate::plots::traits::PlotCompute;

        let x = vec![1.0, 2.0];
        let y = vec![10.0, 20.0];
        let yerr = ErrorValues::symmetric(vec![1.0, 2.0]);
        let xerr = ErrorValues::symmetric(vec![0.1, 0.2]);
        let config = ErrorBarConfig::default().xerr(true);
        let input = ErrorBarInput::with_both(&x, &y, &yerr, &xerr);
        let result = ErrorBarPlot::compute(input, &config);

        assert!(result.is_ok());
        let error_data = result.unwrap();
        assert_eq!(error_data.bars.len(), 2);
    }
}
