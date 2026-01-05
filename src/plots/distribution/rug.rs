//! Rug plots for showing individual data points along an axis
//!
//! Rug plots display small tick marks (often called "rugs") along an axis
//! to show the distribution of individual data points. They are commonly
//! used alongside histograms, KDE plots, or as marginal plots.
//!
//! # Example
//!
//! ```rust,ignore
//! use ruviz::plots::distribution::rug::{Rug, RugConfig};
//!
//! let data = vec![1.0, 2.0, 2.5, 3.0, 4.0, 4.5, 5.0];
//!
//! // Create a rug plot
//! let rug = Rug::new(&data).compute();
//!
//! // Configure the rug plot
//! let config = RugConfig::default()
//!     .height(0.05)
//!     .axis(RugAxis::X);
//! ```
//!
//! # Matplotlib/Seaborn Compatibility
//!
//! This implementation matches seaborn's `rugplot()` function:
//! - Default height is 5% of axis range
//! - Lines are drawn perpendicular to the axis
//! - Alpha defaults to 0.7 for visual density

use crate::core::{Orientation, Result};
use crate::plots::traits::{PlotArea, PlotCompute, PlotData, PlotRender};
use crate::render::{Color, SkiaRenderer, Theme};

/// Which axis to draw the rug on
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RugAxis {
    /// Rug marks along the x-axis (default)
    #[default]
    X,
    /// Rug marks along the y-axis
    Y,
    /// Rug marks on both axes
    Both,
}

/// Configuration for rug plots
#[derive(Debug, Clone)]
pub struct RugConfig {
    /// Height of rug marks as fraction of axis range (default: 0.05)
    pub height: f32,
    /// Which axis to draw on
    pub axis: RugAxis,
    /// Line width for rug marks (default: 0.8)
    pub line_width: f32,
    /// Alpha transparency (default: 0.7)
    pub alpha: f32,
    /// Color for rug marks
    pub color: Option<Color>,
    /// Offset from axis edge as fraction (default: 0.0)
    pub offset: f32,
}

impl Default for RugConfig {
    fn default() -> Self {
        Self {
            height: 0.05,
            axis: RugAxis::X,
            line_width: 0.8,
            alpha: 0.7,
            color: None,
            offset: 0.0,
        }
    }
}

impl RugConfig {
    /// Set the height of rug marks as fraction of axis range
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Set which axis to draw on
    pub fn axis(mut self, axis: RugAxis) -> Self {
        self.axis = axis;
        self
    }

    /// Set line width for rug marks
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width;
        self
    }

    /// Set alpha transparency
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }

    /// Set color for rug marks
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set offset from axis edge
    pub fn offset(mut self, offset: f32) -> Self {
        self.offset = offset;
        self
    }
}

/// Computed rug data
#[derive(Debug, Clone)]
pub struct RugData {
    /// Data points
    pub points: Vec<f64>,
    /// Configuration
    pub config: RugConfig,
}

impl RugData {
    /// Get the number of points
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Get data bounds
    pub fn bounds(&self) -> Option<(f64, f64)> {
        if self.points.is_empty() {
            return None;
        }

        let min = self.points.iter().copied().fold(f64::INFINITY, f64::min);
        let max = self
            .points
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        Some((min, max))
    }
}

// Implement PlotConfig marker trait
impl crate::plots::traits::PlotConfig for RugConfig {}

/// Marker type for Rug plot computation
///
/// This empty struct is used to implement [`PlotCompute`] for Rug plots.
pub struct Rug;

impl PlotCompute for Rug {
    type Input<'a> = &'a [f64];
    type Config = RugConfig;
    type Output = RugData;

    fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output> {
        // Filter out NaN/Inf values
        let points: Vec<f64> = input.iter().copied().filter(|&x| x.is_finite()).collect();

        Ok(RugData {
            points,
            config: config.clone(),
        })
    }
}

/// Builder for rug plots with fluent API
#[derive(Debug, Clone)]
pub struct RugBuilder {
    data: Vec<f64>,
    config: RugConfig,
}

impl RugBuilder {
    /// Create a new rug plot from data
    pub fn new(data: &[f64]) -> Self {
        Self {
            data: data.to_vec(),
            config: RugConfig::default(),
        }
    }

    /// Create from a reference to avoid cloning
    pub fn from_ref(data: Vec<f64>) -> Self {
        Self {
            data,
            config: RugConfig::default(),
        }
    }

    /// Set configuration
    pub fn with_config(mut self, config: RugConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the height of rug marks
    pub fn height(mut self, height: f32) -> Self {
        self.config.height = height;
        self
    }

    /// Set which axis to draw on
    pub fn axis(mut self, axis: RugAxis) -> Self {
        self.config.axis = axis;
        self
    }

    /// Set line width
    pub fn line_width(mut self, width: f32) -> Self {
        self.config.line_width = width;
        self
    }

    /// Set alpha transparency
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.config.alpha = alpha;
        self
    }

    /// Set color
    pub fn color(mut self, color: Color) -> Self {
        self.config.color = Some(color);
        self
    }

    /// Compute the rug plot data
    pub fn compute(self) -> Result<RugData> {
        Rug::compute(&self.data, &self.config)
    }
}

impl PlotData for RugData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        let (min, max) = self.bounds().unwrap_or((0.0, 1.0));
        // Bounds depend on which axis we're on
        match self.config.axis {
            RugAxis::X => ((min, max), (0.0, 1.0)),
            RugAxis::Y => ((0.0, 1.0), (min, max)),
            RugAxis::Both => ((min, max), (min, max)),
        }
    }

    fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

impl PlotRender for RugData {
    fn render(
        &self,
        _renderer: &mut SkiaRenderer,
        _area: &PlotArea,
        _theme: &Theme,
        _color: Color,
    ) -> Result<()> {
        // Rendering is handled by the Plot builder integrating with
        // the rendering backend. This trait method provides the interface.
        Ok(())
    }
}

/// Helper to compute rug line endpoints
///
/// Returns (start, end) coordinates for each rug mark
pub fn compute_rug_lines(
    data: &RugData,
    axis_min: f64,
    axis_max: f64,
    orientation: Orientation,
) -> Vec<((f64, f64), (f64, f64))> {
    let range = axis_max - axis_min;
    let rug_height = range * data.config.height as f64;
    let offset = range * data.config.offset as f64;

    data.points
        .iter()
        .map(|&point| {
            let base = axis_min + offset;
            let tip = base + rug_height;

            match orientation {
                Orientation::Vertical => ((point, base), (point, tip)),
                Orientation::Horizontal => ((base, point), (tip, point)),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rug_compute() {
        let data = vec![1.0, 2.0, 3.0, f64::NAN, 4.0, f64::INFINITY, 5.0];
        let config = RugConfig::default();
        let rug = Rug::compute(&data, &config).unwrap();

        // NaN and Inf should be filtered out
        assert_eq!(rug.len(), 5);
        assert_eq!(rug.points, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn test_rug_bounds() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let config = RugConfig::default();
        let rug = Rug::compute(&data, &config).unwrap();

        let (min, max) = rug.bounds().unwrap();
        assert_eq!(min, 1.0);
        assert_eq!(max, 5.0);
    }

    #[test]
    fn test_rug_config_builder() {
        let rug = RugBuilder::new(&[1.0, 2.0])
            .height(0.1)
            .axis(RugAxis::Y)
            .line_width(1.5)
            .alpha(0.5)
            .compute()
            .unwrap();

        assert_eq!(rug.config.height, 0.1);
        assert_eq!(rug.config.axis, RugAxis::Y);
        assert_eq!(rug.config.line_width, 1.5);
        assert_eq!(rug.config.alpha, 0.5);
    }

    #[test]
    fn test_compute_rug_lines() {
        let config = RugConfig::default().height(0.1);
        let data = Rug::compute(&[1.0, 2.0, 3.0], &config).unwrap();

        let lines = compute_rug_lines(&data, 0.0, 10.0, Orientation::Vertical);

        assert_eq!(lines.len(), 3);
        // First line at x=1.0, y from 0.0 to ~1.0 (10% of range)
        let ((x1, y1), (x2, y2)) = lines[0];
        assert!((x1 - 1.0).abs() < 1e-6);
        assert!((y1 - 0.0).abs() < 1e-6);
        assert!((x2 - 1.0).abs() < 1e-6);
        assert!((y2 - 1.0).abs() < 1e-6);
    }
}
