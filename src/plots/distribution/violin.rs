//! Violin plot implementation
//!
//! Provides violin plots for visualizing distribution shapes,
//! combining KDE with optional box/strip components.
//!
//! # Trait-Based API
//!
//! Violin plots implement the core plot traits:
//! - [`PlotConfig`] for `ViolinConfig`
//! - [`PlotCompute`] for `Violin` marker struct
//! - [`PlotData`] for `ViolinData`
//! - [`PlotRender`] for `ViolinData`

use crate::core::Result;
use crate::core::style_utils::StyleResolver;
use crate::plots::traits::{PlotArea, PlotCompute, PlotConfig, PlotData, PlotRender};
use crate::render::skia::SkiaRenderer;
use crate::render::{Color, LineStyle, Theme};
use crate::stats::kde::{KdeResult, kde_1d, scotts_rule, silvermans_rule};

/// Configuration for violin plot
#[allow(deprecated)] // the derives touch the deprecated `scale` field
#[derive(Debug, Clone)]
pub struct ViolinConfig {
    /// Number of points for KDE evaluation
    pub n_points: usize,
    /// Bandwidth selection method
    pub bandwidth: BandwidthMethod,
    /// Show inner boxplot
    pub show_box: bool,
    /// Show inner quartile lines
    pub show_quartiles: bool,
    /// Show median line
    pub show_median: bool,
    /// Show individual data points (like strip/swarm)
    pub show_points: bool,
    /// Split violin (show half on each side for comparison)
    pub split: bool,
    /// Scale method for width
    ///
    /// Not yet implemented; every violin is width-normalised
    /// ([`ViolinScale::Width`]). Honouring `Area`/`Count` requires a normaliser
    /// shared across all violins in a figure, which the per-series render path
    /// does not have.
    #[deprecated(
        since = "0.6.0",
        note = "not yet implemented; tracked for a future release. Violins are always width-normalised (ViolinScale::Width)"
    )]
    pub scale: ViolinScale,
    /// Maximum width of violin (in plot units)
    pub width: f64,
    /// Orientation
    pub orientation: Orientation,
    /// Color for violin fill
    pub fill_color: Option<Color>,
    /// Alpha for fill
    pub fill_alpha: f32,
    /// Color for outline
    pub line_color: Option<Color>,
    /// Line width
    pub line_width: f32,
    /// Inner color (for box/quartiles)
    pub inner_color: Color,
    /// Category name for this violin (for X-axis label)
    pub category: Option<String>,
    /// X position for this violin (default: 0.5 for single violin)
    pub x_position: f64,
}

/// Bandwidth selection method
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BandwidthMethod {
    /// Scott's rule: `1.06 * sigma * n^(-1/5)` (default)
    #[default]
    Scott,
    /// Silverman's robust rule: `0.9 * min(sigma, IQR/1.34) * n^(-1/5)`
    Silverman,
    /// Fixed bandwidth value
    Fixed(f64),
}

/// A bare number is a fixed bandwidth.
///
/// This is what lets `bandwidth` mean the same thing on every builder that has
/// one: `.bandwidth(0.5)` and `.bandwidth(BandwidthMethod::Silverman)` are both
/// accepted by KDE and violin plots alike.
impl From<f64> for BandwidthMethod {
    fn from(value: f64) -> Self {
        BandwidthMethod::Fixed(value)
    }
}

impl BandwidthMethod {
    /// Resolve this method to a concrete, strictly positive bandwidth.
    ///
    /// A non-finite or non-positive [`BandwidthMethod::Fixed`] value falls back
    /// to Scott's rule rather than producing a degenerate density.
    pub fn resolve(self, data: &[f64]) -> f64 {
        match self {
            BandwidthMethod::Fixed(bw) if crate::stats::kde::is_valid_bandwidth(bw) => bw,
            BandwidthMethod::Fixed(_) | BandwidthMethod::Scott => scotts_rule(data),
            BandwidthMethod::Silverman => silvermans_rule(data),
        }
    }
}

/// Scaling method for violin width
///
/// Only [`ViolinScale::Width`] is implemented; see [`ViolinConfig::scale`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViolinScale {
    /// Same area for all violins
    Area,
    /// Same maximum width for all violins
    #[default]
    Width,
    /// Width proportional to sample count
    Count,
}

/// Orientation for violin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    #[default]
    Vertical,
    Horizontal,
}

impl Default for ViolinConfig {
    #[allow(deprecated)] // `scale` still has to be populated while it exists
    fn default() -> Self {
        Self {
            n_points: 100,
            bandwidth: BandwidthMethod::Scott,
            show_box: true,
            show_quartiles: true,
            show_median: true,
            show_points: false,
            split: false,
            scale: ViolinScale::Width,
            width: 0.8,
            orientation: Orientation::Vertical,
            fill_color: None,
            fill_alpha: 0.7,
            line_color: None,
            line_width: 1.0,
            inner_color: Color::from_rgb(51, 51, 51), // Dark gray instead of pure black
            category: None,
            x_position: 0.5, // Default center position for single violin
        }
    }
}

impl ViolinConfig {
    /// Create new config with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set number of evaluation points
    pub fn n_points(mut self, n: usize) -> Self {
        self.n_points = n.max(10);
        self
    }

    /// Set the bandwidth selection method.
    ///
    /// Takes a [`BandwidthMethod`] or, via [`From<f64>`], a fixed bandwidth:
    /// `.bandwidth(0.5)` is `.bandwidth(BandwidthMethod::Fixed(0.5))`.
    pub fn bandwidth(mut self, method: impl Into<BandwidthMethod>) -> Self {
        self.bandwidth = method.into();
        self
    }

    /// Show/hide inner boxplot
    pub fn box_plot(mut self, show: bool) -> Self {
        self.show_box = show;
        self
    }

    /// Show/hide quartile lines
    pub fn quartiles(mut self, show: bool) -> Self {
        self.show_quartiles = show;
        self
    }

    /// Show/hide median
    pub fn median(mut self, show: bool) -> Self {
        self.show_median = show;
        self
    }

    /// Show/hide the individual observations inside the violin
    ///
    /// Points are drawn on the violin's centre line, seaborn's
    /// `inner="point"` convention.
    pub fn points(mut self, show: bool) -> Self {
        self.show_points = show;
        self
    }

    /// Enable split violin mode
    pub fn split(mut self, split: bool) -> Self {
        self.split = split;
        self
    }

    /// Set scale method
    ///
    /// Currently inert: violins are always width-normalised. See
    /// [`ViolinConfig::scale`].
    #[deprecated(
        since = "0.6.0",
        note = "not yet implemented; tracked for a future release. Violins are always width-normalised (ViolinScale::Width)"
    )]
    #[allow(deprecated)]
    pub fn scale(mut self, scale: ViolinScale) -> Self {
        self.scale = scale;
        self
    }

    /// Set maximum width
    pub fn width(mut self, width: f64) -> Self {
        self.width = width.max(0.1);
        self
    }

    /// Set horizontal orientation
    pub fn horizontal(mut self) -> Self {
        self.orientation = Orientation::Horizontal;
        self
    }

    /// Set vertical orientation
    pub fn vertical(mut self) -> Self {
        self.orientation = Orientation::Vertical;
        self
    }

    /// Set fill color
    pub fn fill_color(mut self, color: Color) -> Self {
        self.fill_color = Some(color);
        self
    }

    /// Set fill alpha
    pub fn fill_alpha(mut self, alpha: f32) -> Self {
        self.fill_alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Set line color
    pub fn line_color(mut self, color: Color) -> Self {
        self.line_color = Some(color);
        self
    }

    /// Set line width
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width.max(0.0);
        self
    }

    /// Set category name for this violin
    ///
    /// The category name is displayed on the X-axis instead of numeric values.
    pub fn category<S: Into<String>>(mut self, name: S) -> Self {
        self.category = Some(name.into());
        self
    }

    /// Set X position for this violin
    ///
    /// Used when plotting multiple violins to control their horizontal positions.
    pub fn x_position(mut self, pos: f64) -> Self {
        self.x_position = pos;
        self
    }
}

// Implement PlotConfig marker trait
impl PlotConfig for ViolinConfig {}

/// Marker struct for Violin plot type (used with PlotCompute trait)
pub struct Violin;

/// Violin plot data with computed KDE
#[derive(Debug, Clone)]
pub struct ViolinData {
    /// Original data
    pub data: Vec<f64>,
    /// KDE result
    pub kde: KdeResult,
    /// Quartiles (q1, median, q3)
    pub quartiles: (f64, f64, f64),
    /// Data min and max
    pub range: (f64, f64),
    /// Configuration used to compute this data
    pub(crate) config: ViolinConfig,
}

impl ViolinData {
    /// Compute violin data from raw values
    pub fn from_values(data: &[f64], config: &ViolinConfig) -> Option<Self> {
        if data.is_empty() {
            return None;
        }

        // Filter valid values
        let mut sorted: Vec<f64> = data.iter().filter(|v| v.is_finite()).copied().collect();
        if sorted.is_empty() {
            return None;
        }
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let n = sorted.len();
        let min = sorted[0];
        let max = sorted[n - 1];

        // Compute KDE. Every arm resolves to an explicit bandwidth so the
        // selected rule is the rule that actually runs — `Silverman` used to
        // fall through to `kde_1d`'s Scott default and silently compute a
        // different estimator than the one it named.
        let bandwidth = Some(config.bandwidth.resolve(&sorted));
        let kde = kde_1d(&sorted, bandwidth, Some(config.n_points));

        // Compute quartiles
        let q1 = percentile(&sorted, 25.0);
        let median = percentile(&sorted, 50.0);
        let q3 = percentile(&sorted, 75.0);

        Some(Self {
            data: sorted,
            kde,
            quartiles: (q1, median, q3),
            range: (min, max),
            config: config.clone(),
        })
    }

    /// Get maximum density (for scaling)
    pub fn max_density(&self) -> f64 {
        self.kde.density.iter().copied().fold(0.0, f64::max)
    }

    /// Draw the individual observations on the violin's centre line.
    ///
    /// This backs [`ViolinConfig::points`] (seaborn's `inner="point"`). Points
    /// sit on the centre line rather than being jittered so the same input
    /// always produces the same image.
    fn draw_points(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        center: f64,
        color: Color,
        alpha: f32,
    ) -> Result<()> {
        if !self.config.show_points || self.data.is_empty() {
            return Ok(());
        }

        let clip_rect = (area.x, area.y, area.width, area.height);
        let size = renderer
            .render_scale()
            .points_to_pixels(VIOLIN_POINT_SIZE_PT);
        let point_color =
            color.with_alpha((f32::from(color.a) / 255.0) * alpha.clamp(0.0, 1.0) * 0.75);

        for &value in &self.data {
            if !value.is_finite() {
                continue;
            }
            let projected = match self.config.orientation {
                Orientation::Vertical => area.try_data_to_screen(center, value),
                Orientation::Horizontal => area.try_data_to_screen(value, center),
            };
            let Some((px, py)) = projected else {
                continue;
            };
            renderer.draw_marker_clipped(
                px,
                py,
                size,
                crate::render::MarkerStyle::Circle,
                point_color,
                clip_rect,
            )?;
        }

        Ok(())
    }
}

/// Marker size, in points, for the observations drawn by
/// [`ViolinConfig::points`].
const VIOLIN_POINT_SIZE_PT: f32 = 2.5;

/// Calculate percentile from sorted data
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let n = sorted.len();
    let idx = (p / 100.0) * (n - 1) as f64;
    let lower = idx.floor() as usize;
    let upper = idx.ceil() as usize;
    let frac = idx - lower as f64;

    if lower >= n || upper >= n {
        sorted[n - 1]
    } else {
        sorted[lower] * (1.0 - frac) + sorted[upper] * frac
    }
}

/// Generate violin polygon vertices
///
/// # Arguments
/// * `violin` - Violin data with KDE
/// * `center_x` - X position for vertical violin
/// * `center_y` - Y position for horizontal violin
/// * `half_width` - Half the maximum width
/// * `config` - Violin configuration
///
/// # Returns
/// Left and right polygon vertices for the violin shape
#[allow(clippy::type_complexity)]
pub fn violin_polygon(
    violin: &ViolinData,
    center: f64,
    half_width: f64,
    config: &ViolinConfig,
) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
    let max_density = violin.max_density();
    if max_density <= 0.0 {
        return (vec![], vec![]);
    }

    let scale = half_width / max_density;

    let mut left_side = Vec::with_capacity(violin.kde.x.len());
    let mut right_side = Vec::with_capacity(violin.kde.x.len());

    for (i, (&x, &d)) in violin
        .kde
        .x
        .iter()
        .zip(violin.kde.density.iter())
        .enumerate()
    {
        let width = d * scale;

        match config.orientation {
            Orientation::Vertical => {
                // x is actually y (the data value), center is the x position
                if config.split {
                    left_side.push((center, x));
                    right_side.push((center + width, x));
                } else {
                    left_side.push((center - width, x));
                    right_side.push((center + width, x));
                }
            }
            Orientation::Horizontal => {
                // x is the data value (horizontal axis), center is y position
                if config.split {
                    left_side.push((x, center));
                    right_side.push((x, center + width));
                } else {
                    left_side.push((x, center - width));
                    right_side.push((x, center + width));
                }
            }
        }
    }

    (left_side, right_side)
}

/// Create closed polygon from left and right sides
pub fn close_violin_polygon(left: &[(f64, f64)], right: &[(f64, f64)]) -> Vec<(f64, f64)> {
    if left.is_empty() || right.is_empty() {
        return vec![];
    }

    let mut polygon = Vec::with_capacity(left.len() + right.len());

    // Add left side (bottom to top)
    polygon.extend_from_slice(left);

    // Add right side (top to bottom)
    for point in right.iter().rev() {
        polygon.push(*point);
    }

    polygon
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl PlotCompute for Violin {
    type Input<'a> = &'a [f64];
    type Config = ViolinConfig;
    type Output = ViolinData;

    fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output> {
        ViolinData::from_values(input, config).ok_or(crate::core::PlottingError::EmptyDataSet)
    }
}

impl PlotData for ViolinData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        // For vertical violin: x is centered (use 0-1 range), y is KDE range
        // For horizontal violin: x is KDE range, y is centered
        // Use the KDE's actual range (which extends beyond data min/max by 3 bandwidths)
        let kde_range = if self.kde.x.is_empty() {
            self.range
        } else {
            let kde_min = self.kde.x.first().copied().unwrap_or(self.range.0);
            let kde_max = self.kde.x.last().copied().unwrap_or(self.range.1);
            (kde_min, kde_max)
        };

        match self.config.orientation {
            Orientation::Vertical => {
                let x_range = (0.0, 1.0); // Will be adjusted by position
                (x_range, kde_range)
            }
            Orientation::Horizontal => {
                let y_range = (0.0, 1.0); // Will be adjusted by position
                (kde_range, y_range)
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl PlotRender for ViolinData {
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        _theme: &Theme,
        color: Color,
    ) -> Result<()> {
        if self.data.is_empty() {
            return Ok(());
        }

        let config = &self.config;
        let half_width = config.width / 2.0;
        let render_scale = renderer.render_scale();
        let line_width_px = render_scale.points_to_pixels(config.line_width);
        let min_box_width_px = render_scale.points_to_pixels(4.0);
        let inner_line_width_px = render_scale.points_to_pixels(1.0);
        let median_marker_size_px = render_scale.points_to_pixels(4.0);

        // Generate polygon vertices (center at 0.5 for single violin)
        let (left, right) = violin_polygon(self, 0.5, half_width, config);
        let polygon = close_violin_polygon(&left, &right);

        if polygon.is_empty() {
            return Ok(());
        }

        // Convert to screen coordinates, dropping vertices the axis scales
        // cannot place (the KDE tails run past a log axis' floor).
        let screen_points: Vec<(f32, f32)> = area.project_points(polygon.iter().copied());

        // Get clip rectangle from plot area bounds
        let clip_rect = (area.x, area.y, area.width, area.height);

        // Draw filled violin with clipping to plot area
        if screen_points.len() >= 3 {
            let fill_color = config
                .fill_color
                .unwrap_or(color)
                .with_alpha(config.fill_alpha);
            renderer.draw_filled_polygon_clipped(&screen_points, fill_color, clip_rect)?;
        }

        // Draw outline with clipping
        let line_color = config.line_color.unwrap_or(color);
        if screen_points.len() >= 2 && config.line_width > 0.0 {
            let mut outline = screen_points.clone();
            outline.push(screen_points[0]); // Close the path
            renderer.draw_polyline_clipped(
                &outline,
                line_color,
                line_width_px,
                LineStyle::Solid,
                clip_rect,
            )?;
        }

        // Draw inner elements (points, box, quartiles, median)
        let center = 0.5;
        let (q1, median, q3) = self.quartiles;

        self.draw_points(renderer, area, center, line_color, 1.0)?;

        if config.show_box {
            // Draw thin box for IQR (seaborn-style: ~5% of half-width)
            let box_half_width = half_width * 0.025;
            let (x1, y1) = area.data_to_screen(center - box_half_width, q1);
            let (x2, y2) = area.data_to_screen(center + box_half_width, q3);
            // Use abs() for dimensions and min() for origin to handle y-axis inversion
            let box_x = x1.min(x2);
            let box_y = y1.min(y2);
            let box_width = (x2 - x1).abs().max(min_box_width_px);
            let box_height = (y2 - y1).abs();
            renderer.draw_rectangle(
                box_x,
                box_y,
                box_width,
                box_height,
                config.inner_color,
                true,
            )?;
        }

        if config.show_quartiles {
            // Draw quartile lines
            let line_half = half_width * 0.12;
            let (q1_x1, q1_y) = area.data_to_screen(center - line_half, q1);
            let (q1_x2, _) = area.data_to_screen(center + line_half, q1);
            renderer.draw_line(
                q1_x1,
                q1_y,
                q1_x2,
                q1_y,
                config.inner_color,
                inner_line_width_px,
                LineStyle::Solid,
            )?;

            let (q3_x1, q3_y) = area.data_to_screen(center - line_half, q3);
            let (q3_x2, _) = area.data_to_screen(center + line_half, q3);
            renderer.draw_line(
                q3_x1,
                q3_y,
                q3_x2,
                q3_y,
                config.inner_color,
                inner_line_width_px,
                LineStyle::Solid,
            )?;
        }

        if config.show_median {
            // Draw median dot or line
            let (mx, my) = area.data_to_screen(center, median);
            renderer.draw_marker(
                mx,
                my,
                median_marker_size_px,
                crate::render::MarkerStyle::Circle,
                Color::from_rgb(255, 255, 255),
            )?;
        }

        Ok(())
    }

    fn render_styled(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        theme: &Theme,
        color: Color,
        alpha: f32,
        line_width: Option<f32>,
    ) -> Result<()> {
        if self.data.is_empty() {
            return Ok(());
        }

        let config = &self.config;
        let resolver = StyleResolver::new(theme);
        let half_width = config.width / 2.0;
        let render_scale = renderer.render_scale();
        let min_box_width_px = render_scale.points_to_pixels(4.0);
        let inner_line_width_px = render_scale.points_to_pixels(1.0);
        let median_marker_size_px = render_scale.points_to_pixels(4.0);

        // Generate polygon vertices (center at 0.5 for single violin)
        let (left, right) = violin_polygon(self, 0.5, half_width, config);
        let polygon = close_violin_polygon(&left, &right);

        if polygon.is_empty() {
            return Ok(());
        }

        // Convert to screen coordinates, dropping vertices the axis scales
        // cannot place (the KDE tails run past a log axis' floor).
        let screen_points: Vec<(f32, f32)> = area.project_points(polygon.iter().copied());

        // Get clip rectangle from plot area bounds
        let clip_rect = (area.x, area.y, area.width, area.height);

        // Use StyleResolver for fill color
        let fill_base = config.fill_color.unwrap_or(color);
        let fill_alpha =
            (f32::from(fill_base.a) / 255.0) * config.fill_alpha * alpha.clamp(0.0, 1.0);
        let fill_color = fill_base.with_alpha(fill_alpha);

        // Draw filled violin with clipping to plot area
        if screen_points.len() >= 3 {
            renderer.draw_filled_polygon_clipped(&screen_points, fill_color, clip_rect)?;
        }

        // Use StyleResolver for edge/line styling
        let actual_line_width = render_scale.points_to_pixels(
            line_width.unwrap_or_else(|| resolver.line_width(Some(config.line_width))),
        );
        let line_color = resolver.edge_color(color, config.line_color);
        let line_color =
            line_color.with_alpha((f32::from(line_color.a) / 255.0) * alpha.clamp(0.0, 1.0));

        // Draw outline with clipping
        if screen_points.len() >= 2 && actual_line_width > 0.0 {
            let mut outline = screen_points.clone();
            outline.push(screen_points[0]); // Close the path
            renderer.draw_polyline_clipped(
                &outline,
                line_color,
                actual_line_width,
                LineStyle::Solid,
                clip_rect,
            )?;
        }

        // Draw inner elements (points, box, quartiles, median)
        let center = 0.5;
        let (q1, median, q3) = self.quartiles;

        // `line_color` already carries the series alpha, so do not apply it twice.
        self.draw_points(renderer, area, center, line_color, 1.0)?;

        if config.show_box {
            // Draw thin box for IQR (seaborn-style: ~5% of half-width)
            let box_half_width = half_width * 0.025;
            let (x1, y1) = area.data_to_screen(center - box_half_width, q1);
            let (x2, y2) = area.data_to_screen(center + box_half_width, q3);
            // Use abs() for dimensions and min() for origin to handle y-axis inversion
            let box_x = x1.min(x2);
            let box_y = y1.min(y2);
            let box_width = (x2 - x1).abs().max(min_box_width_px);
            let box_height = (y2 - y1).abs();
            renderer.draw_rectangle(
                box_x,
                box_y,
                box_width,
                box_height,
                config.inner_color,
                true,
            )?;
        }

        if config.show_quartiles {
            // Draw quartile lines
            let line_half = half_width * 0.12;
            let (q1_x1, q1_y) = area.data_to_screen(center - line_half, q1);
            let (q1_x2, _) = area.data_to_screen(center + line_half, q1);
            renderer.draw_line(
                q1_x1,
                q1_y,
                q1_x2,
                q1_y,
                config.inner_color,
                inner_line_width_px,
                LineStyle::Solid,
            )?;

            let (q3_x1, q3_y) = area.data_to_screen(center - line_half, q3);
            let (q3_x2, _) = area.data_to_screen(center + line_half, q3);
            renderer.draw_line(
                q3_x1,
                q3_y,
                q3_x2,
                q3_y,
                config.inner_color,
                inner_line_width_px,
                LineStyle::Solid,
            )?;
        }

        if config.show_median {
            // Draw median dot or line
            let (mx, my) = area.data_to_screen(center, median);
            renderer.draw_marker(
                mx,
                my,
                median_marker_size_px,
                crate::render::MarkerStyle::Circle,
                Color::from_rgb(255, 255, 255),
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_violin_data_basic() {
        let data: Vec<f64> = (0..100).map(|i| (i as f64 * 0.1).sin()).collect();
        let config = ViolinConfig::default();
        let violin = ViolinData::from_values(&data, &config);

        assert!(violin.is_some());
        let violin = violin.unwrap();
        assert!(!violin.kde.x.is_empty());
        assert!(violin.max_density() > 0.0);
    }

    #[test]
    fn test_violin_data_empty() {
        let data: Vec<f64> = vec![];
        let config = ViolinConfig::default();
        let violin = ViolinData::from_values(&data, &config);
        assert!(violin.is_none());
    }

    #[test]
    fn test_violin_polygon() {
        let data: Vec<f64> = (0..50).map(|i| i as f64).collect();
        let config = ViolinConfig::default();
        let violin = ViolinData::from_values(&data, &config).unwrap();

        let (left, right) = violin_polygon(&violin, 0.5, 0.3, &config);
        assert!(!left.is_empty());
        assert!(!right.is_empty());
        assert_eq!(left.len(), right.len());
    }

    #[test]
    fn test_close_polygon() {
        let left = vec![(0.0, 0.0), (0.0, 1.0), (0.0, 2.0)];
        let right = vec![(1.0, 0.0), (1.0, 1.0), (1.0, 2.0)];

        let closed = close_violin_polygon(&left, &right);
        assert_eq!(closed.len(), 6);
    }

    #[test]
    fn test_percentile() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((percentile(&data, 50.0) - 3.0).abs() < 1e-10);
        assert!((percentile(&data, 0.0) - 1.0).abs() < 1e-10);
        assert!((percentile(&data, 100.0) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_violin_config_implements_plot_config() {
        fn assert_plot_config<T: PlotConfig>() {}
        assert_plot_config::<ViolinConfig>();
    }

    #[test]
    fn test_violin_plot_compute_trait() {
        use crate::plots::traits::PlotCompute;

        let data: Vec<f64> = (0..100).map(|i| (i as f64 * 0.1).sin()).collect();
        let config = ViolinConfig::default();
        let result = Violin::compute(&data, &config);

        assert!(result.is_ok());
        let violin_data = result.unwrap();
        assert!(!violin_data.data.is_empty());
        assert!(violin_data.max_density() > 0.0);
    }

    #[test]
    fn test_violin_plot_compute_empty() {
        use crate::plots::traits::PlotCompute;

        let data: Vec<f64> = vec![];
        let config = ViolinConfig::default();
        let result = Violin::compute(&data, &config);

        assert!(result.is_err());
    }

    // ------------------------------------------------------------------
    // Bandwidth method wiring (plan item 2.4)
    // ------------------------------------------------------------------

    fn skewed_sample() -> Vec<f64> {
        // Log-normal-ish: sigma and IQR/1.34 disagree, so Scott and Silverman
        // must land on different bandwidths.
        (1..=200).map(|i| ((i as f64) / 20.0).exp()).collect()
    }

    #[test]
    fn test_silverman_is_not_silently_scott() {
        let data = skewed_sample();

        let scott = ViolinData::from_values(&data, &ViolinConfig::new()).unwrap();
        let silverman = ViolinData::from_values(
            &data,
            &ViolinConfig::new().bandwidth(BandwidthMethod::Silverman),
        )
        .unwrap();

        assert!(
            (scott.kde.bandwidth - silverman.kde.bandwidth).abs() > 1e-9,
            "Silverman ({}) collapsed onto Scott ({})",
            silverman.kde.bandwidth,
            scott.kde.bandwidth
        );
    }

    #[test]
    fn test_bandwidth_methods_resolve_to_their_named_estimator() {
        let data = skewed_sample();
        let mut sorted = data.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        assert_eq!(
            BandwidthMethod::Scott.resolve(&sorted),
            crate::stats::kde::scotts_rule(&sorted)
        );
        assert_eq!(
            BandwidthMethod::Silverman.resolve(&sorted),
            crate::stats::kde::silvermans_rule(&sorted)
        );
        assert_eq!(BandwidthMethod::Fixed(0.25).resolve(&sorted), 0.25);
    }

    #[test]
    fn test_fixed_bandwidth_reaches_the_kde() {
        let data = skewed_sample();
        let violin = ViolinData::from_values(
            &data,
            &ViolinConfig::new().bandwidth(BandwidthMethod::Fixed(0.5)),
        )
        .unwrap();
        assert!((violin.kde.bandwidth - 0.5).abs() < 1e-12);
    }

    #[test]
    fn test_degenerate_fixed_bandwidth_falls_back_instead_of_producing_nan() {
        let data = skewed_sample();
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let violin = ViolinData::from_values(
                &data,
                &ViolinConfig::new().bandwidth(BandwidthMethod::Fixed(bad)),
            )
            .unwrap();
            assert!(violin.kde.bandwidth.is_finite() && violin.kde.bandwidth > 0.0);
            assert!(violin.kde.density.iter().all(|d| d.is_finite()));
        }
    }

    // ------------------------------------------------------------------
    // show_points (plan item 2.4)
    // ------------------------------------------------------------------

    fn render_violin(config: ViolinConfig) -> crate::core::Result<crate::core::plot::Image> {
        let data: Vec<f64> = (0..60).map(|i| i as f64 / 6.0).collect();
        let violin = ViolinData::from_values(&data, &config).unwrap();
        let mut renderer = SkiaRenderer::new(200, 200, Theme::default())?;
        let ((_, _), (y_min, y_max)) = violin.data_bounds();
        let area = PlotArea::new(0.0, 0.0, 200.0, 200.0, 0.0, 1.0, y_min, y_max);
        violin.render(
            &mut renderer,
            &area,
            &Theme::default(),
            Color::from_rgb(0, 0, 255),
        )?;
        Ok(renderer.into_image())
    }

    #[test]
    fn test_show_points_changes_the_rendered_image() {
        let without = render_violin(ViolinConfig::new().points(false)).unwrap();
        let with = render_violin(ViolinConfig::new().points(true)).unwrap();

        assert_ne!(
            without.pixels, with.pixels,
            "ViolinConfig::points(true) produced a byte-identical image"
        );
    }

    #[test]
    fn test_show_points_is_off_by_default() {
        let default = render_violin(ViolinConfig::new()).unwrap();
        let explicit_off = render_violin(ViolinConfig::new().points(false)).unwrap();
        assert_eq!(default.pixels, explicit_off.pixels);
    }

    #[test]
    fn test_violin_plot_data_trait() {
        use crate::plots::traits::PlotData;

        let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let config = ViolinConfig::default();
        let violin_data = ViolinData::from_values(&data, &config).unwrap();

        // Test data_bounds
        let ((x_min, x_max), (y_min, y_max)) = violin_data.data_bounds();
        assert!(x_min <= x_max);
        assert!(y_min <= y_max);

        // Test is_empty
        assert!(!violin_data.is_empty());
    }
}
