//! Polar plot implementations
//!
//! Provides polar scatter, line, and bar plots with configurable axis labels.
//!
//! # Axis Labels
//!
//! Polar plots support angular (theta) and radial (r) axis labels:
//!
//! ```rust,ignore
//! use ruviz::plots::polar::PolarPlotConfig;
//!
//! // Default: show all labels
//! let config = PolarPlotConfig::default();
//!
//! // Customize label appearance
//! let config = PolarPlotConfig::new()
//!     .show_theta_labels(true)      // Show 0°, 30°, 60°, etc.
//!     .show_r_labels(true)          // Show radial scale
//!     .r_label_position(22.5)       // Position at 22.5° from right
//!     .label_font_size(10.0);       // Font size in points
//!
//! // Hide labels for cleaner appearance
//! let minimal = PolarPlotConfig::new()
//!     .show_theta_labels(false)
//!     .show_r_labels(false);
//! ```
//!
//! # Trait-Based API
//!
//! Polar plots implement the core plot traits:
//! - [`PlotConfig`] for `PolarPlotConfig`
//! - [`PlotCompute`] for `PolarPlot` marker struct
//! - [`PlotData`] for `PolarPlotData`
//! - [`PlotRender`] for `PolarPlotData`

use crate::core::Result;
use crate::plots::traits::{PlotArea, PlotCompute, PlotConfig, PlotData, PlotRender};
use crate::render::skia::SkiaRenderer;
use crate::render::{Color, LineStyle, MarkerStyle, Theme};

/// Configuration for polar plots
#[derive(Debug, Clone)]
pub struct PolarPlotConfig {
    /// Start angle in radians (0 = right, counter-clockwise)
    pub theta_offset: f64,
    /// Direction of theta (true = counter-clockwise)
    pub theta_direction: bool,
    /// Show radial grid
    pub show_rgrid: bool,
    /// Show angular grid
    pub show_thetgrid: bool,
    /// Number of radial grid lines
    pub rgrid_count: usize,
    /// Number of angular grid lines
    pub thetgrid_count: usize,
    /// Line color (None for auto)
    pub color: Option<Color>,
    /// Line width
    pub line_width: f32,
    /// Marker size
    pub marker_size: f32,
    /// Fill area under curve
    pub fill: bool,
    /// Fill alpha
    pub fill_alpha: f32,
    /// Show angular axis labels (0°, 45°, 90°, etc.)
    pub show_theta_labels: bool,
    /// Show radial axis labels
    pub show_r_labels: bool,
    /// Position of radial labels in degrees from right (default: 22.5)
    pub r_label_position: f64,
    /// Font size for axis labels
    pub label_font_size: f32,
}

impl Default for PolarPlotConfig {
    fn default() -> Self {
        Self {
            theta_offset: 0.0,
            theta_direction: true,
            show_rgrid: true,
            show_thetgrid: true,
            rgrid_count: 5,
            thetgrid_count: 12,
            color: None,
            line_width: 1.5,
            marker_size: 0.0,
            fill: false,
            fill_alpha: 0.3,
            show_theta_labels: true,
            show_r_labels: true,
            r_label_position: 22.5, // degrees
            label_font_size: 10.0,
        }
    }
}

impl PolarPlotConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set theta offset
    pub fn theta_offset(mut self, offset: f64) -> Self {
        self.theta_offset = offset;
        self
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

    /// Set marker size
    pub fn marker_size(mut self, size: f32) -> Self {
        self.marker_size = size.max(0.0);
        self
    }

    /// Enable fill
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// Set fill alpha
    pub fn fill_alpha(mut self, alpha: f32) -> Self {
        self.fill_alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Show/hide angular labels (0°, 45°, 90°, etc.)
    pub fn show_theta_labels(mut self, show: bool) -> Self {
        self.show_theta_labels = show;
        self
    }

    /// Show/hide radial labels
    pub fn show_r_labels(mut self, show: bool) -> Self {
        self.show_r_labels = show;
        self
    }

    /// Set position of radial labels (degrees from right)
    pub fn r_label_position(mut self, degrees: f64) -> Self {
        self.r_label_position = degrees;
        self
    }

    /// Set label font size
    pub fn label_font_size(mut self, size: f32) -> Self {
        self.label_font_size = size.max(1.0);
        self
    }
}

// Implement PlotConfig marker trait
impl PlotConfig for PolarPlotConfig {}

/// Marker struct for Polar plot type
pub struct PolarPlot;

/// A point in polar coordinates
#[derive(Debug, Clone, Copy)]
pub struct PolarPoint {
    /// Radius (distance from center)
    pub r: f64,
    /// Theta (angle in radians)
    pub theta: f64,
    /// Cartesian x
    pub x: f64,
    /// Cartesian y
    pub y: f64,
}

impl PolarPoint {
    /// Create from polar coordinates
    pub fn from_polar(r: f64, theta: f64) -> Self {
        Self {
            r,
            theta,
            x: r * theta.cos(),
            y: r * theta.sin(),
        }
    }

    /// Create from cartesian coordinates
    pub fn from_cartesian(x: f64, y: f64) -> Self {
        Self {
            r: (x * x + y * y).sqrt(),
            theta: y.atan2(x),
            x,
            y,
        }
    }
}

/// A label with position and text
#[derive(Debug, Clone)]
pub struct PositionedLabel {
    /// Screen-relative x position (in data coordinates)
    pub x: f64,
    /// Screen-relative y position (in data coordinates)
    pub y: f64,
    /// Label text
    pub text: String,
}

/// Computed polar plot data
#[derive(Debug, Clone)]
pub struct PolarPlotData {
    /// Points in the plot
    pub points: Vec<PolarPoint>,
    /// Maximum radius
    pub r_max: f64,
    /// Polygon vertices for fill (closed path)
    pub fill_polygon: Vec<(f64, f64)>,
    /// Angular axis labels (0°, 90°, etc.)
    pub theta_labels: Vec<PositionedLabel>,
    /// Radial axis labels
    pub r_labels: Vec<PositionedLabel>,
    /// Configuration used
    pub(crate) config: PolarPlotConfig,
}

/// Input for polar plot computation
pub struct PolarPlotInput<'a> {
    /// Radius values
    pub r: &'a [f64],
    /// Theta values (in radians)
    pub theta: &'a [f64],
}

impl<'a> PolarPlotInput<'a> {
    /// Create new polar plot input
    pub fn new(r: &'a [f64], theta: &'a [f64]) -> Self {
        Self { r, theta }
    }
}

/// Compute polar plot points
///
/// # Arguments
/// * `r` - Radius values
/// * `theta` - Theta values (in radians)
/// * `config` - Polar plot configuration
///
/// # Returns
/// PolarPlotData with converted points
pub fn compute_polar_plot(r: &[f64], theta: &[f64], config: &PolarPlotConfig) -> PolarPlotData {
    use std::f64::consts::PI;

    let n = r.len().min(theta.len());
    if n == 0 {
        return PolarPlotData {
            points: vec![],
            r_max: 1.0,
            fill_polygon: vec![],
            theta_labels: vec![],
            r_labels: vec![],
            config: config.clone(),
        };
    }

    let mut points = Vec::with_capacity(n);
    let mut r_max = 0.0_f64;

    for i in 0..n {
        // Apply theta offset and direction
        let adjusted_theta = if config.theta_direction {
            theta[i] + config.theta_offset
        } else {
            -theta[i] + config.theta_offset
        };

        let point = PolarPoint::from_polar(r[i], adjusted_theta);
        r_max = r_max.max(r[i].abs());
        points.push(point);
    }

    // Ensure we have a valid r_max
    let r_max = if r_max > 0.0 { r_max } else { 1.0 };

    // Generate fill polygon if enabled
    let fill_polygon = if config.fill && !points.is_empty() {
        let mut polygon: Vec<(f64, f64)> = points.iter().map(|p| (p.x, p.y)).collect();
        // Close through origin
        polygon.push((0.0, 0.0));
        polygon
    } else {
        vec![]
    };

    // Compute theta labels (0°, 45°, 90°, etc.) positioned at edge of plot
    let theta_labels = if config.show_theta_labels {
        let label_radius = r_max * 1.12; // Position slightly outside the plot
        (0..config.thetgrid_count)
            .map(|i| {
                let angle = 2.0 * PI * i as f64 / config.thetgrid_count as f64;
                let degrees = (angle * 180.0 / PI).round() as i32;
                PositionedLabel {
                    x: label_radius * angle.cos(),
                    y: label_radius * angle.sin(),
                    text: format!("{}°", degrees),
                }
            })
            .collect()
    } else {
        vec![]
    };

    // Compute radial labels positioned along r_label_position angle
    let r_labels = if config.show_r_labels {
        let label_angle = config.r_label_position * PI / 180.0; // Convert to radians
        (1..=config.rgrid_count)
            .map(|i| {
                let radius = r_max * i as f64 / config.rgrid_count as f64;
                PositionedLabel {
                    x: radius * label_angle.cos(),
                    y: radius * label_angle.sin(),
                    text: format!("{:.1}", radius),
                }
            })
            .collect()
    } else {
        vec![]
    };

    PolarPlotData {
        points,
        r_max,
        fill_polygon,
        theta_labels,
        r_labels,
        config: config.clone(),
    }
}

/// Generate polar grid lines
///
/// # Arguments
/// * `r_max` - Maximum radius
/// * `r_count` - Number of radial circles
/// * `theta_count` - Number of angular divisions
///
/// # Returns
/// (radial_circles, angular_lines) where circles are radii and lines are (start, end) points
#[allow(clippy::type_complexity)]
pub fn polar_grid(
    r_max: f64,
    r_count: usize,
    theta_count: usize,
) -> (Vec<f64>, Vec<((f64, f64), (f64, f64))>) {
    // Radial circles
    let radii: Vec<f64> = (1..=r_count)
        .map(|i| r_max * i as f64 / r_count as f64)
        .collect();

    // Angular lines (from center to edge)
    let angular_step = 2.0 * std::f64::consts::PI / theta_count as f64;
    let angular_lines: Vec<((f64, f64), (f64, f64))> = (0..theta_count)
        .map(|i| {
            let theta = i as f64 * angular_step;
            ((0.0, 0.0), (r_max * theta.cos(), r_max * theta.sin()))
        })
        .collect();

    (radii, angular_lines)
}

/// Generate circle vertices for rendering
pub fn circle_vertices(cx: f64, cy: f64, radius: f64, n_segments: usize) -> Vec<(f64, f64)> {
    let step = 2.0 * std::f64::consts::PI / n_segments as f64;
    (0..=n_segments)
        .map(|i| {
            let theta = i as f64 * step;
            (cx + radius * theta.cos(), cy + radius * theta.sin())
        })
        .collect()
}

// ============================================================================
// Trait-Based API
// ============================================================================

impl PlotCompute for PolarPlot {
    type Input<'a> = PolarPlotInput<'a>;
    type Config = PolarPlotConfig;
    type Output = PolarPlotData;

    fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output> {
        if input.r.is_empty() || input.theta.is_empty() {
            return Err(crate::core::PlottingError::EmptyDataSet);
        }

        Ok(compute_polar_plot(input.r, input.theta, config))
    }
}

impl PlotData for PolarPlotData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        // Polar plots need extra margin for axis labels positioned at r_max * 1.12
        // Use r_max * 1.5 to ensure labels have enough room with layout adjustments
        let label_margin = self.r_max * 1.5;
        ((-label_margin, label_margin), (-label_margin, label_margin))
    }

    fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

impl PlotRender for PolarPlotData {
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        theme: &Theme,
        color: Color,
    ) -> Result<()> {
        if self.points.is_empty() {
            return Ok(());
        }

        let config = &self.config;
        let line_color = config.color.unwrap_or(color);

        // Draw fill if enabled
        if config.fill && !self.fill_polygon.is_empty() {
            let fill_color = line_color.with_alpha(config.fill_alpha);
            let screen_polygon: Vec<(f32, f32)> = self
                .fill_polygon
                .iter()
                .map(|(x, y)| area.data_to_screen(*x, *y))
                .collect();
            renderer.draw_filled_polygon(&screen_polygon, fill_color)?;
        }

        // Draw lines connecting points
        if self.points.len() > 1 {
            for i in 0..self.points.len() - 1 {
                let p1 = &self.points[i];
                let p2 = &self.points[i + 1];
                let (sx1, sy1) = area.data_to_screen(p1.x, p1.y);
                let (sx2, sy2) = area.data_to_screen(p2.x, p2.y);
                renderer.draw_line(
                    sx1,
                    sy1,
                    sx2,
                    sy2,
                    line_color,
                    config.line_width,
                    LineStyle::Solid,
                )?;
            }
        }

        // Draw markers if configured
        if config.marker_size > 0.0 {
            for point in &self.points {
                let (sx, sy) = area.data_to_screen(point.x, point.y);
                renderer.draw_marker(
                    sx,
                    sy,
                    config.marker_size,
                    MarkerStyle::Circle,
                    line_color,
                )?;
            }
        }

        // Draw theta labels (angular axis labels: 0°, 30°, 60°, etc.)
        let label_color = theme.foreground;
        for label in &self.theta_labels {
            let (sx, sy) = area.data_to_screen(label.x, label.y);
            renderer.draw_text_centered(
                &label.text,
                sx,
                sy,
                config.label_font_size,
                label_color,
            )?;
        }

        // Draw radial labels
        for label in &self.r_labels {
            let (sx, sy) = area.data_to_screen(label.x, label.y);
            renderer.draw_text_centered(
                &label.text,
                sx,
                sy,
                config.label_font_size,
                label_color,
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_polar_point_from_polar() {
        let point = PolarPoint::from_polar(1.0, 0.0);
        assert!((point.x - 1.0).abs() < 1e-10);
        assert!((point.y - 0.0).abs() < 1e-10);

        let point = PolarPoint::from_polar(1.0, PI / 2.0);
        assert!((point.x - 0.0).abs() < 1e-10);
        assert!((point.y - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_polar_point_from_cartesian() {
        let point = PolarPoint::from_cartesian(1.0, 0.0);
        assert!((point.r - 1.0).abs() < 1e-10);
        assert!((point.theta - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_polar_plot() {
        let r = vec![1.0, 2.0, 3.0];
        let theta = vec![0.0, PI / 2.0, PI];
        let config = PolarPlotConfig::default();
        let data = compute_polar_plot(&r, &theta, &config);

        assert_eq!(data.points.len(), 3);
        assert!((data.r_max - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_polar_grid() {
        let (radii, lines) = polar_grid(10.0, 5, 8);

        assert_eq!(radii.len(), 5);
        assert_eq!(lines.len(), 8);
        assert!((radii[4] - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_circle_vertices() {
        let vertices = circle_vertices(0.0, 0.0, 1.0, 4);
        assert_eq!(vertices.len(), 5); // 4 segments + closing point
    }

    #[test]
    fn test_polar_config_implements_plot_config() {
        fn assert_plot_config<T: PlotConfig>() {}
        assert_plot_config::<PolarPlotConfig>();
    }

    #[test]
    fn test_polar_plot_compute_trait() {
        use crate::plots::traits::PlotCompute;

        let r = vec![1.0, 2.0, 3.0];
        let theta = vec![0.0, PI / 2.0, PI];
        let config = PolarPlotConfig::default();
        let input = PolarPlotInput::new(&r, &theta);
        let result = PolarPlot::compute(input, &config);

        assert!(result.is_ok());
        let polar_data = result.unwrap();
        assert_eq!(polar_data.points.len(), 3);
    }

    #[test]
    fn test_polar_plot_data_trait() {
        use crate::plots::traits::{PlotCompute, PlotData};

        let r = vec![1.0, 2.0, 3.0];
        let theta = vec![0.0, PI / 2.0, PI];
        let config = PolarPlotConfig::default();
        let input = PolarPlotInput::new(&r, &theta);
        let polar_data = PolarPlot::compute(input, &config).unwrap();

        // Test data_bounds
        let ((x_min, x_max), (y_min, y_max)) = polar_data.data_bounds();
        assert!(x_min < 0.0);
        assert!(x_max > 0.0);
        assert!(y_min < 0.0);
        assert!(y_max > 0.0);

        // Test is_empty
        assert!(!polar_data.is_empty());
    }
}
