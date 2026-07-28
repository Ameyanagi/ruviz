//! Rug plots for showing individual data points along an axis
//!
//! Rug plots display small tick marks (often called "rugs") along an axis
//! to show the distribution of individual data points. They are commonly
//! used alongside histograms, KDE plots, or as marginal plots.
//!
//! # Example
//!
//! ```rust
//! use ruviz::plots::distribution::{Rug, RugAxis, RugConfig};
//! use ruviz::plots::PlotCompute;
//!
//! let data = vec![1.0, 2.0, 2.5, 3.0, 4.0, 4.5, 5.0];
//!
//! let config = RugConfig::default().height(0.05).axis(RugAxis::X);
//! let rug = Rug::compute(&data, &config)?;
//!
//! // The marks the renderer draws, in data coordinates.
//! let marks = rug.segments((1.0, 5.0), (0.0, 1.0));
//! assert_eq!(marks.len(), data.len());
//! # Ok::<(), ruviz::core::PlottingError>(())
//! ```
//!
//! # Matplotlib/Seaborn Compatibility
//!
//! This implementation matches seaborn's `rugplot()` function:
//! - Default height is 5% of axis range
//! - Lines are drawn perpendicular to the axis
//! - Alpha defaults to 0.7 for visual density

use crate::core::{Orientation, Result};
use crate::plots::traits::{
    ComputedSeries, ComputedStyle, PlotArea, PlotCompute, PlotData, PlotPrimitive, PlotRender,
    draw_primitives,
};
use crate::render::{Color, LineStyle, SkiaRenderer, Theme};

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
    /// Line width for rug marks, in **points** (default: 0.8)
    ///
    /// The renderer converts it to device pixels, so a rug mark keeps its
    /// physical thickness at every DPI.
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

    /// Set line width for rug marks, in points
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

    /// The rug marks in **data** coordinates for a plot spanning `x_range` by
    /// `y_range`.
    ///
    /// This is the only place rug geometry is derived. [`PlotRender::render`]
    /// projects exactly these segments, so a caller that draws the marks itself
    /// and a plotted rug cannot disagree about where a mark is.
    ///
    /// Which ranges are consulted follows [`RugConfig::axis`]: an x-axis rug
    /// rises from the bottom of `y_range`, a y-axis rug runs in from the left of
    /// `x_range`, and [`RugAxis::Both`] emits both sets.
    pub fn segments(
        &self,
        x_range: (f64, f64),
        y_range: (f64, f64),
    ) -> Vec<((f64, f64), (f64, f64))> {
        let mut segments = Vec::new();
        if matches!(self.config.axis, RugAxis::X | RugAxis::Both) {
            segments.extend(compute_rug_lines(
                self,
                y_range.0,
                y_range.1,
                Orientation::Vertical,
            ));
        }
        if matches!(self.config.axis, RugAxis::Y | RugAxis::Both) {
            segments.extend(compute_rug_lines(
                self,
                x_range.0,
                x_range.1,
                Orientation::Horizontal,
            ));
        }
        segments
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

impl ComputedSeries for RugData {
    fn kind(&self) -> &'static str {
        "rug"
    }

    fn point_count(&self) -> usize {
        self.points.len()
    }

    /// One stroked segment per rug mark.
    ///
    /// This is the only description of rug ink in the crate: the raster path,
    /// the SVG path and a caller driving [`PlotRender`] directly all draw
    /// exactly these primitives.
    fn primitives(&self, area: &PlotArea, style: &ComputedStyle) -> Vec<PlotPrimitive> {
        let base = self.config.color.unwrap_or(style.color);
        // The configured alpha and the series alpha compose, so `.alpha(0.5)`
        // on the series halves an already-translucent rug rather than
        // replacing it.
        let color = base.with_alpha(
            (f32::from(base.a) / 255.0) * self.config.alpha * style.alpha.clamp(0.0, 1.0),
        );
        // The width is authored in points; the render scale is what makes the
        // mark keep its physical thickness at every DPI.
        let width_px = style.stroke_px(self.config.line_width);

        self.segments((area.x_min, area.x_max), (area.y_min, area.y_max))
            .into_iter()
            .filter_map(|(start, end)| {
                // A mark with an endpoint the axis cannot place (zero on a log
                // axis, a non-finite sample) has no position at all, so it is
                // dropped rather than stroked to a NaN pixel.
                let from = area.try_data_to_screen(start.0, start.1)?;
                let to = area.try_data_to_screen(end.0, end.1)?;
                Some(PlotPrimitive::Line {
                    from,
                    to,
                    color,
                    width_px,
                    style: LineStyle::Solid,
                })
            })
            .collect()
    }
}

impl PlotRender for RugData {
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        _theme: &Theme,
        color: Color,
    ) -> Result<()> {
        let style = ComputedStyle::opaque(renderer.render_scale(), color);
        draw_primitives(renderer, &self.primitives(area, &style))
    }

    fn render_styled(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        _theme: &Theme,
        color: Color,
        alpha: f32,
        line_width: Option<f32>,
    ) -> Result<()> {
        let style = ComputedStyle {
            scale: renderer.render_scale(),
            color,
            alpha,
            line_width,
        };
        draw_primitives(renderer, &self.primitives(area, &style))
    }
}

/// Helper to compute rug line endpoints along one axis
///
/// Returns (start, end) coordinates for each rug mark. This places every mark
/// against a single axis range; [`RugData::segments`] is the axis-aware wrapper
/// the renderer uses, and it is what you want unless you are laying marks
/// against an axis of your own.
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

    fn rug(config: RugConfig) -> RugData {
        Rug::compute(&[1.0, 2.0, 3.0, 4.0], &config).unwrap()
    }

    /// Number of pixels the rug put ink on.
    fn ink(image: &crate::core::plot::Image) -> usize {
        image
            .pixels
            .chunks_exact(4)
            .filter(|p| p[3] > 0 && (p[0] < 250 || p[1] < 250 || p[2] < 250))
            .count()
    }

    /// Total ink coverage, which is proportional to stroked area and therefore
    /// to stroke width — unlike a pixel count, which quantises it.
    fn coverage(image: &crate::core::plot::Image) -> u64 {
        image
            .pixels
            .chunks_exact(4)
            .map(|p| u64::from(255 - p[1]))
            .sum()
    }

    fn render(data: &RugData, area: PlotArea, dpi_scale: f32) -> crate::core::plot::Image {
        let mut renderer =
            SkiaRenderer::new(200, 200, Theme::default()).expect("renderer for a 200x200 canvas");
        renderer.set_dpi_scale(dpi_scale);
        data.render(
            &mut renderer,
            &area,
            &Theme::default(),
            Color::from_rgb(200, 0, 0),
        )
        .expect("rug render");
        renderer.into_image()
    }

    fn area() -> PlotArea {
        PlotArea::new(0.0, 0.0, 200.0, 200.0, 0.0, 5.0, 0.0, 10.0)
    }

    #[test]
    fn test_rug_render_actually_draws_its_marks() {
        // The renderer used to return Ok(()) without drawing anything, so a rug
        // plot reported success and produced a blank canvas.
        let image = render(&rug(RugConfig::default()), area(), 1.0);
        assert!(
            ink(&image) > 0,
            "rug render reported success but left the canvas blank"
        );
    }

    #[test]
    fn test_rug_render_draws_more_ink_for_more_points() {
        let few = Rug::compute(&[1.0, 2.0], &RugConfig::default()).unwrap();
        let many = Rug::compute(&[1.0, 2.0, 3.0, 4.0], &RugConfig::default()).unwrap();

        assert!(
            ink(&render(&many, area(), 1.0)) > ink(&render(&few, area(), 1.0)),
            "twice the samples drew no more marks"
        );
    }

    #[test]
    fn test_rug_marks_keep_their_physical_thickness_at_higher_dpi() {
        // The width is in points, so doubling the render scale must double the
        // stroked area; a raw-pixel width would leave the two images identical.
        let data = rug(RugConfig::default());
        let single = coverage(&render(&data, area(), 1.0));
        let double = coverage(&render(&data, area(), 2.0));

        assert!(single > 0, "the rug drew nothing at all");
        assert!(
            double > single + single / 2,
            "rug marks did not thicken with DPI ({double} vs {single} ink coverage)"
        );
    }

    #[test]
    fn test_rug_render_drops_marks_the_axis_cannot_place() {
        use crate::axes::AxisScale;

        let placeable = Rug::compute(&[1.0, 10.0], &RugConfig::default()).unwrap();
        let with_unplaceable = Rug::compute(&[-5.0, 1.0, 10.0], &RugConfig::default()).unwrap();
        let log_area = PlotArea::new(0.0, 0.0, 200.0, 200.0, 1.0, 100.0, 1.0, 10.0)
            .with_scales(AxisScale::Log, AxisScale::Linear);

        // The negative sample has no position on a log axis, so it is dropped
        // rather than stroked at pixel zero - and rendering still succeeds.
        assert_eq!(
            ink(&render(&with_unplaceable, log_area, 1.0)),
            ink(&render(&placeable, log_area, 1.0)),
            "a sample the log axis cannot place still put ink on the canvas"
        );
    }

    #[test]
    fn test_rug_segments_follow_the_configured_axis() {
        let x_marks = rug(RugConfig::default().axis(RugAxis::X)).segments((0.0, 5.0), (0.0, 10.0));
        let y_marks = rug(RugConfig::default().axis(RugAxis::Y)).segments((0.0, 5.0), (0.0, 10.0));
        let both = rug(RugConfig::default().axis(RugAxis::Both)).segments((0.0, 5.0), (0.0, 10.0));

        assert_eq!(x_marks.len(), 4);
        assert_eq!(y_marks.len(), 4);
        assert_eq!(both.len(), 8, "RugAxis::Both must mark both axes");

        // Tolerances are 1e-6, not 1e-9: `height` and `offset` are `f32`, so
        // 0.05 widened to f64 is 0.05000000074..., and a 1e-9 window is finer
        // than the config field can represent.

        // An x-axis mark stands at its sample's x and rises off the y floor.
        let ((x1, y1), (x2, y2)) = x_marks[0];
        assert!((x1 - 1.0).abs() < 1e-6 && (x2 - 1.0).abs() < 1e-6);
        assert!((y1 - 0.0).abs() < 1e-6);
        assert!((y2 - 0.5).abs() < 1e-6, "5% of a 10-unit y range");

        // A y-axis mark sits at its sample's y and runs in off the x floor.
        let ((x1, y1), (x2, y2)) = y_marks[0];
        assert!((y1 - 1.0).abs() < 1e-6 && (y2 - 1.0).abs() < 1e-6);
        assert!((x1 - 0.0).abs() < 1e-6);
        assert!((x2 - 0.25).abs() < 1e-6, "5% of a 5-unit x range");
    }

    #[test]
    fn test_rug_render_honours_the_offset() {
        let flush = rug(RugConfig::default()).segments((0.0, 5.0), (0.0, 10.0));
        let lifted = rug(RugConfig::default().offset(0.1)).segments((0.0, 5.0), (0.0, 10.0));

        let ((_, flush_base), _) = flush[0];
        let ((_, lifted_base), _) = lifted[0];
        assert!((flush_base - 0.0).abs() < 1e-6);
        assert!((lifted_base - 1.0).abs() < 1e-6, "10% of a 10-unit range");
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
