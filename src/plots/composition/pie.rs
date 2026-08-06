//! Pie chart implementation
//!
//! Provides pie and donut chart functionality with labels, percentages, and explode.
//!
//! # Trait-Based API
//!
//! Pie charts implement the core plot traits:
//! - [`PlotConfig`] for `PieConfig`
//! - [`PlotCompute`] for `Pie` marker struct
//! - [`PlotData`] for `PieData`
//! - [`PlotRender`] for `PieData`

use crate::core::Result;
use crate::plots::traits::{PlotArea, PlotCompute, PlotConfig, PlotData, PlotRender};
use crate::render::primitives::{Wedge, pie_wedges};
use crate::render::{Color, SkiaRenderer, Theme};
use std::f64::consts::PI;

/// Inner radius, as a fraction of the outer radius, that makes a pie a donut.
///
/// The one number `Plot::donut` means by "a donut": a hole wide enough to read
/// as one, narrow enough to leave the wedges comparable by angle. Callers who
/// want a different hole say so with [`PieConfig::donut`].
pub const DEFAULT_DONUT_INNER_RADIUS: f64 = 0.4;

/// Configuration for pie chart
#[derive(Debug, Clone)]
pub struct PieConfig {
    /// Labels for each wedge
    pub labels: Vec<String>,
    /// Colors for each wedge (None for auto-colors from palette)
    pub colors: Option<Vec<Color>>,
    /// Explode offset for each wedge (0.0 = no explode)
    pub explode: Vec<f64>,
    /// Show percentage labels
    pub show_percentages: bool,
    /// Show value labels
    pub show_values: bool,
    /// Show labels (category names)
    pub show_labels: bool,
    /// Inner radius for donut chart (0.0 = full pie)
    pub inner_radius: f64,
    /// Start angle in degrees, measured counter-clockwise from 3 o'clock
    /// (0 = 3 o'clock, 90 = 12 o'clock), as in matplotlib
    pub start_angle: f64,
    /// Whether wedges advance counter-clockwise from `start_angle`.
    ///
    /// The default is `false`: wedges run clockwise in input order from
    /// `start_angle`, so reading the chart the way a clock is read follows the
    /// order the values were given (plotly's and d3's convention).
    pub counter_clockwise: bool,
    /// Label color, or `None` to pick per wedge for contrast against its fill.
    pub text_color: Option<Color>,
    /// Font size for labels
    pub label_font_size: f32,
    /// How far out labels sit: 0 is the inner edge, 1 the rim
    pub label_distance: f64,
    /// Shadow offset (0 = no shadow)
    pub shadow: f64,
    /// Edge color for wedges (None for no edge)
    pub edge_color: Option<Color>,
    /// Edge width
    pub edge_width: f32,
}

impl Default for PieConfig {
    fn default() -> Self {
        Self {
            labels: vec![],
            colors: None,
            explode: vec![],
            show_percentages: true,
            show_values: false,
            show_labels: true,
            inner_radius: 0.0,
            start_angle: 90.0, // Start at top (12 o'clock)
            counter_clockwise: false,
            text_color: None,
            label_font_size: 10.0,
            label_distance: 0.6,
            shadow: 0.0,
            edge_color: Some(Color::from_rgb(255, 255, 255)),
            edge_width: 1.0,
        }
    }
}

impl PieConfig {
    /// Create a new pie config with labels
    pub fn new(labels: Vec<String>) -> Self {
        Self {
            labels,
            ..Default::default()
        }
    }

    /// Set colors for wedges
    pub fn colors(mut self, colors: Vec<Color>) -> Self {
        self.colors = Some(colors);
        self
    }

    /// Set explode offsets for wedges
    pub fn explode(mut self, explode: Vec<f64>) -> Self {
        self.explode = explode;
        self
    }

    /// Create a donut chart with inner radius
    ///
    /// See [`DEFAULT_DONUT_INNER_RADIUS`] for the ratio `Plot::donut` uses.
    pub fn donut(mut self, inner_radius: f64) -> Self {
        self.inner_radius = inner_radius.clamp(0.0, 0.95);
        self
    }

    /// Set start angle in degrees
    pub fn start_angle(mut self, angle: f64) -> Self {
        self.start_angle = angle;
        self
    }

    /// Sweep the wedges clockwise from `start_angle` (the default).
    pub fn clockwise(mut self) -> Self {
        self.counter_clockwise = false;
        self
    }

    /// Sweep the wedges counter-clockwise from `start_angle`, as matplotlib does.
    pub fn counter_clockwise(mut self) -> Self {
        self.counter_clockwise = true;
        self
    }

    /// Force one label color for every wedge.
    ///
    /// Left unset, each label takes black or white by the luminance of the
    /// wedge it sits on — see [`label_color_on`].
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    /// Show/hide percentage labels
    pub fn percentages(mut self, show: bool) -> Self {
        self.show_percentages = show;
        self
    }

    /// Show/hide value labels
    pub fn values(mut self, show: bool) -> Self {
        self.show_values = show;
        self
    }

    /// Show/hide category labels
    pub fn labels(mut self, show: bool) -> Self {
        self.show_labels = show;
        self
    }

    /// Set label font size
    pub fn font_size(mut self, size: f32) -> Self {
        self.label_font_size = size;
        self
    }

    /// Set how far out the labels sit, 0 at the inner edge and 1 at the rim
    pub fn label_distance(mut self, distance: f64) -> Self {
        self.label_distance = distance;
        self
    }

    /// Set edge color
    pub fn edge_color(mut self, color: Color) -> Self {
        self.edge_color = Some(color);
        self
    }

    /// Remove edge
    pub fn no_edge(mut self) -> Self {
        self.edge_color = None;
        self
    }
}

// Implement PlotConfig marker trait
impl PlotConfig for PieConfig {}

/// Marker struct for Pie plot type
pub struct Pie;

/// Pie chart data and computed wedges
#[derive(Debug, Clone)]
pub struct PieData {
    /// Original values
    pub values: Vec<f64>,
    /// Computed wedges
    pub wedges: Vec<Wedge>,
    /// Total sum of values
    pub total: f64,
    /// Percentages for each wedge
    pub percentages: Vec<f64>,
    /// Start angles for each wedge (radians)
    pub start_angles: Vec<f64>,
    /// End angles for each wedge (radians)
    pub end_angles: Vec<f64>,
    /// Configuration used
    pub(crate) config: PieConfig,
}

impl PieData {
    /// Create pie data from values
    pub fn from_values(values: &[f64], cx: f64, cy: f64, radius: f64, config: &PieConfig) -> Self {
        let kept: Vec<usize> = values
            .iter()
            .enumerate()
            .filter(|&(_, &v)| v > 0.0)
            .map(|(index, _)| index)
            .collect();
        let positive_values: Vec<f64> = kept.iter().map(|&index| values[index]).collect();
        let total: f64 = positive_values.iter().sum();

        // Per-value vectors the caller supplied are indexed by wedge, so they have
        // to be filtered alongside the values: otherwise every entry after a
        // dropped value binds to the wrong wedge.
        let config = filter_per_value_config(config, &kept, values.len());

        let percentages: Vec<f64> = if total > 0.0 {
            positive_values.iter().map(|v| v / total * 100.0).collect()
        } else {
            vec![0.0; positive_values.len()]
        };

        // Wedge angles live in screen space, where +y points down, so a positive
        // sweep advances clockwise. `start_angle` is measured counter-clockwise
        // from 3 o'clock (matplotlib's convention), hence the negation: the
        // default of 90° starts at 12 o'clock.
        let start_angle_rad = -config.start_angle.to_radians();

        let mut wedges = pie_wedges(&positive_values, cx, cy, radius, Some(start_angle_rad));

        // `pie_wedges` always sweeps clockwise on screen. Mirroring each wedge
        // about the start angle keeps the wedge order but sweeps the other way.
        if config.counter_clockwise {
            for wedge in &mut wedges {
                let (start, end) = (wedge.start_angle, wedge.end_angle);
                wedge.start_angle = 2.0 * start_angle_rad - end;
                wedge.end_angle = 2.0 * start_angle_rad - start;
            }
        }

        // Compute normalized angles
        let mut start_angles = Vec::with_capacity(wedges.len());
        let mut end_angles = Vec::with_capacity(wedges.len());

        // Apply configuration to wedges
        for (i, wedge) in wedges.iter_mut().enumerate() {
            start_angles.push(wedge.start_angle);
            end_angles.push(wedge.end_angle);

            // Apply inner radius for donut
            if config.inner_radius > 0.0 {
                *wedge = wedge.inner_radius(radius * config.inner_radius);
            }

            // Apply explode
            if i < config.explode.len() && config.explode[i] > 0.0 {
                *wedge = wedge.explode(config.explode[i] * radius * 0.1);
            }
        }

        Self {
            values: positive_values,
            wedges,
            total,
            percentages,
            start_angles,
            end_angles,
            config,
        }
    }

    /// Create normalized pie data (without specific coordinates)
    /// Used by PlotCompute trait
    pub fn compute(values: &[f64], config: &PieConfig) -> Self {
        // Use unit circle coordinates for normalized computation
        Self::from_values(values, 0.5, 0.5, 0.5, config)
    }
}

/// Format a wedge share as a percentage label.
///
/// One decimal is kept only when the share actually has one: a fifth of the pie
/// reads "20%", not "20.0%".
pub(crate) fn format_percentage(percentage: f64) -> String {
    let rounded = format!("{percentage:.1}");
    let trimmed = rounded.strip_suffix(".0").unwrap_or(&rounded);
    format!("{trimmed}%")
}

/// Relative luminance at or above which a wedge is light enough for dark text.
///
/// Placed at the CIE lightness midpoint (`L* = 60`, i.e. `((60 + 16) / 116)^3`,
/// rounded to 0.30) rather than at the WCAG contrast crossover (0.179), which
/// would leave dark saturated fills — tab10 blue (0.168), green (0.259) and red
/// (0.159) — carrying black text that barely reads.
const DARK_LABEL_LUMINANCE: f64 = 0.30;

/// Label color used on a dark wedge.
const LIGHT_LABEL: Color = Color::from_rgb(255, 255, 255);

/// Label color used on a light wedge: near-black, so it does not read as a hole.
const DARK_LABEL: Color = Color::from_rgb(26, 26, 26);

/// Relative luminance of an sRGB color, per WCAG 2.x.
///
/// Alpha is ignored: a wedge label sits on the wedge's own hue.
pub(crate) fn relative_luminance(color: Color) -> f64 {
    fn linearize(channel: u8) -> f64 {
        let value = f64::from(channel) / 255.0;
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    0.2126 * linearize(color.r) + 0.7152 * linearize(color.g) + 0.0722 * linearize(color.b)
}

/// How far out a wedge label sits, in pixels from the centre.
///
/// `label_distance` runs the label from the inner edge to the outer edge, so a
/// donut keeps its labels on the ring. Multiplying the ring midpoint by
/// `label_distance` instead dropped them back into the hole, where they were
/// half-eaten by the background.
pub(crate) fn label_radius(radius: f64, inner_radius: f64, label_distance: f64) -> f64 {
    radius * (inner_radius + (1.0 - inner_radius) * label_distance)
}

/// Pick the label color that reads on `fill`.
///
/// Dark fills take white, light fills near-black, split at 0.30 relative
/// luminance — the CIE lightness midpoint, `L* = 60`.
///
/// ```
/// use ruviz::plots::composition::pie::label_color_on;
/// use ruviz::render::Color;
///
/// // tab10 blue is dark enough to need white text
/// assert_eq!(label_color_on(Color::from_rgb(31, 119, 180)), Color::from_rgb(255, 255, 255));
/// // tab10 orange is not
/// assert_eq!(label_color_on(Color::from_rgb(255, 127, 14)), Color::from_rgb(26, 26, 26));
/// ```
pub fn label_color_on(fill: Color) -> Color {
    if relative_luminance(fill) >= DARK_LABEL_LUMINANCE {
        DARK_LABEL
    } else {
        LIGHT_LABEL
    }
}

/// Drop the entries of the per-value config vectors whose value was filtered out.
///
/// `kept` holds the indices of the values that became wedges. Labels and explode
/// offsets are read with a bounds check, so a short vector filters cleanly; colors
/// wrap modulo their length, so they are only realigned when the caller supplied
/// exactly one per value.
fn filter_per_value_config(config: &PieConfig, kept: &[usize], value_count: usize) -> PieConfig {
    let mut filtered = config.clone();
    if kept.len() == value_count {
        return filtered;
    }

    filtered.labels = kept
        .iter()
        .filter_map(|&index| config.labels.get(index).cloned())
        .collect();
    filtered.explode = kept
        .iter()
        .filter_map(|&index| config.explode.get(index).copied())
        .collect();
    if let Some(colors) = config.colors.as_ref().filter(|c| c.len() == value_count) {
        filtered.colors = Some(kept.iter().map(|&index| colors[index]).collect());
    }

    filtered
}

/// Render a pie chart
///
/// # Arguments
/// * `renderer` - The Skia renderer
/// * `values` - Numeric values for each wedge
/// * `cx` - Center X coordinate
/// * `cy` - Center Y coordinate
/// * `radius` - Outer radius
/// * `config` - Pie chart configuration
/// * `theme` - Color theme
///
/// # Returns
/// Result with PieData containing computed wedges
pub fn render_pie(
    renderer: &mut SkiaRenderer,
    values: &[f64],
    cx: f64,
    cy: f64,
    radius: f64,
    config: &PieConfig,
    theme: &Theme,
) -> crate::core::Result<PieData> {
    let pie_data = PieData::from_values(values, cx, cy, radius, config);

    if pie_data.wedges.is_empty() {
        return Ok(pie_data);
    }

    // Use the resolved config: its per-wedge vectors are aligned with the wedges,
    // which the caller's are not once a non-positive value has been dropped.
    let config = &pie_data.config;

    // Get colors from config or theme palette
    let colors = if let Some(ref colors) = config.colors {
        colors.clone()
    } else {
        let palette = theme.color_palette.clone();
        (0..pie_data.wedges.len())
            .map(|i| palette[i % palette.len()])
            .collect()
    };

    // Number of segments per arc for smooth curves
    let segments = 64;

    let render_scale = renderer.render_scale();
    let edge_width_px = render_scale.points_to_pixels(config.edge_width);
    let label_font_size_px = render_scale.points_to_pixels(config.label_font_size);
    let shadow_offset_px = render_scale.points_to_pixels(config.shadow as f32) as f64;

    // Draw shadow if configured
    if config.shadow > 0.0 {
        let shadow_color = Color::from_rgb(100, 100, 100).with_alpha(0.3);
        for wedge in &pie_data.wedges {
            let mut shadow_wedge = *wedge;
            // Offset shadow
            let polygon = shadow_wedge.as_polygon(segments);
            let shadow_polygon: Vec<(f32, f32)> = polygon
                .iter()
                .map(|(x, y)| {
                    (
                        (*x + shadow_offset_px) as f32,
                        (*y + shadow_offset_px) as f32,
                    )
                })
                .collect();
            renderer.draw_filled_polygon(&shadow_polygon, shadow_color)?;
        }
    }

    // Draw wedges
    for (i, wedge) in pie_data.wedges.iter().enumerate() {
        let color = colors[i % colors.len()];
        let polygon = wedge.as_polygon(segments);
        let polygon_f32: Vec<(f32, f32)> = polygon
            .iter()
            .map(|(x, y)| (*x as f32, *y as f32))
            .collect();

        // Draw filled wedge
        renderer.draw_filled_polygon(&polygon_f32, color)?;

        // Draw edge if configured
        if let Some(edge_color) = config.edge_color {
            renderer.draw_polygon_outline(&polygon_f32, edge_color, edge_width_px)?;
        }
    }

    // Draw labels
    if config.show_labels || config.show_percentages || config.show_values {
        for (i, wedge) in pie_data.wedges.iter().enumerate() {
            let label_parts: Vec<String> = [
                if config.show_labels && i < config.labels.len() {
                    Some(config.labels[i].clone())
                } else {
                    None
                },
                if config.show_percentages {
                    Some(format_percentage(pie_data.percentages[i]))
                } else {
                    None
                },
                if config.show_values {
                    Some(format!("{:.1}", pie_data.values[i]))
                } else {
                    None
                },
            ]
            .into_iter()
            .flatten()
            .collect();

            if !label_parts.is_empty() {
                let label = label_parts.join("\n");

                // Calculate label position
                let label_r = label_radius(radius, config.inner_radius, config.label_distance);

                let (lx, ly) = wedge.centroid();
                let mid_angle = (wedge.start_angle + wedge.end_angle) / 2.0;
                let label_x = cx + label_r * mid_angle.cos();
                let label_y = cy + label_r * mid_angle.sin();

                let text_color = config
                    .text_color
                    .unwrap_or_else(|| label_color_on(colors[i % colors.len()]));
                renderer.draw_text_centered(
                    &label,
                    label_x as f32,
                    label_y as f32,
                    label_font_size_px,
                    text_color,
                )?;
            }
        }
    }

    Ok(pie_data)
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl PlotCompute for Pie {
    type Input<'a> = &'a [f64];
    type Config = PieConfig;
    type Output = PieData;

    fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output> {
        let positive_count = input.iter().filter(|&&v| v > 0.0).count();
        if positive_count == 0 {
            return Err(crate::core::PlottingError::EmptyDataSet);
        }

        Ok(PieData::compute(input, config))
    }
}

impl PlotData for PieData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        // Pie charts use a normalized 0-1 coordinate space
        ((0.0, 1.0), (0.0, 1.0))
    }

    fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl PlotRender for PieData {
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        theme: &Theme,
        color: Color,
    ) -> Result<()> {
        self.render_styled(renderer, area, theme, color, 1.0, None)
    }

    /// # The `color` argument is deliberately ignored
    ///
    /// A pie needs one colour *per wedge*, so a single series colour cannot
    /// describe it: honouring it would paint every wedge the same and erase the
    /// chart. Wedge colours come from [`PieConfig::colors`], falling back to the
    /// theme palette. This means a generic `.color(..)` on the pie builder is a
    /// no-op — use `PieConfig::colors` instead.
    fn render_styled(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        theme: &Theme,
        _color: Color,
        alpha: f32,
        line_width: Option<f32>,
    ) -> Result<()> {
        if self.wedges.is_empty() {
            return Ok(());
        }

        let config = &self.config;

        // Calculate center and radius from plot area
        let (cx, cy) = area.data_to_screen(0.5, 0.5);
        let (edge_x, _) = area.data_to_screen(1.0, 0.5);
        let (_, edge_y) = area.data_to_screen(0.5, 1.0);
        let radius = ((edge_x - cx).abs().min((edge_y - cy).abs())) * 0.9;

        // Recompute wedges with screen coordinates
        let screen_data =
            PieData::from_values(&self.values, cx as f64, cy as f64, radius as f64, config);

        // Get colors from config or theme palette
        let colors = if let Some(ref colors) = config.colors {
            colors.clone()
        } else {
            let palette = theme.color_palette.clone();
            (0..screen_data.wedges.len())
                .map(|i| palette[i % palette.len()])
                .collect()
        }
        .into_iter()
        .map(|color| color.with_alpha((f32::from(color.a) / 255.0) * alpha.clamp(0.0, 1.0)))
        .collect::<Vec<_>>();

        // Number of segments per arc for smooth curves
        let segments = 64;
        let render_scale = renderer.render_scale();
        let edge_width_px = render_scale.points_to_pixels(line_width.unwrap_or(config.edge_width));
        let label_font_size_px = render_scale.points_to_pixels(config.label_font_size);
        let shadow_offset_px = render_scale.points_to_pixels(config.shadow as f32) as f64;

        // Draw shadow if configured
        if config.shadow > 0.0 {
            let shadow_color = Color::from_rgb(100, 100, 100).with_alpha(0.3 * alpha);
            for wedge in &screen_data.wedges {
                let polygon = wedge.as_polygon(segments);
                let shadow_polygon: Vec<(f32, f32)> = polygon
                    .iter()
                    .map(|(x, y)| {
                        (
                            (*x + shadow_offset_px) as f32,
                            (*y + shadow_offset_px) as f32,
                        )
                    })
                    .collect();
                renderer.draw_filled_polygon(&shadow_polygon, shadow_color)?;
            }
        }

        // Draw wedges
        for (i, wedge) in screen_data.wedges.iter().enumerate() {
            let color = colors[i % colors.len()];
            let polygon = wedge.as_polygon(segments);
            let polygon_f32: Vec<(f32, f32)> = polygon
                .iter()
                .map(|(x, y)| (*x as f32, *y as f32))
                .collect();

            // Draw filled wedge
            renderer.draw_filled_polygon(&polygon_f32, color)?;

            // Draw edge if configured
            if let Some(edge_color) = config.edge_color {
                let edge_color = edge_color
                    .with_alpha((f32::from(edge_color.a) / 255.0) * alpha.clamp(0.0, 1.0));
                renderer.draw_polygon_outline(&polygon_f32, edge_color, edge_width_px)?;
            }
        }

        // Draw labels
        if config.show_labels || config.show_percentages || config.show_values {
            for (i, wedge) in screen_data.wedges.iter().enumerate() {
                let label_parts: Vec<String> = [
                    if config.show_labels && i < config.labels.len() {
                        Some(config.labels[i].clone())
                    } else {
                        None
                    },
                    if config.show_percentages {
                        Some(format_percentage(screen_data.percentages[i]))
                    } else {
                        None
                    },
                    if config.show_values {
                        Some(format!("{:.1}", screen_data.values[i]))
                    } else {
                        None
                    },
                ]
                .into_iter()
                .flatten()
                .collect();

                if !label_parts.is_empty() {
                    let label = label_parts.join("\n");

                    // Calculate label position
                    let label_r =
                        label_radius(radius as f64, config.inner_radius, config.label_distance);

                    let mid_angle = (wedge.start_angle + wedge.end_angle) / 2.0;
                    let label_x = cx as f64 + label_r * mid_angle.cos();
                    let label_y = cy as f64 + label_r * mid_angle.sin();

                    let text_color = config
                        .text_color
                        .unwrap_or_else(|| label_color_on(colors[i % colors.len()]));
                    renderer.draw_text_centered(
                        &label,
                        label_x as f32,
                        label_y as f32,
                        label_font_size_px,
                        text_color,
                    )?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pie_data_basic() {
        let values = vec![30.0, 20.0, 50.0];
        let config = PieConfig::default();
        let data = PieData::from_values(&values, 100.0, 100.0, 50.0, &config);

        assert_eq!(data.wedges.len(), 3);
        assert!((data.total - 100.0).abs() < 1e-10);
        assert!((data.percentages[0] - 30.0).abs() < 1e-10);
        assert!((data.percentages[1] - 20.0).abs() < 1e-10);
        assert!((data.percentages[2] - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_pie_config_donut() {
        let config = PieConfig::default().donut(0.5);
        assert!((config.inner_radius - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_pie_data_with_explode() {
        let values = vec![25.0, 25.0, 50.0];
        let config = PieConfig::default().explode(vec![0.1, 0.0, 0.0]);
        let data = PieData::from_values(&values, 100.0, 100.0, 50.0, &config);

        assert_eq!(data.wedges.len(), 3);
        // First wedge should be exploded
        assert!(data.wedges[0].explode > 0.0);
        assert!((data.wedges[1].explode - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_pie_ignores_negative() {
        let values = vec![30.0, -10.0, 20.0];
        let config = PieConfig::default();
        let data = PieData::from_values(&values, 100.0, 100.0, 50.0, &config);

        // Should only have 2 wedges (negative value filtered)
        assert_eq!(data.wedges.len(), 2);
    }

    #[test]
    fn test_pie_per_value_config_tracks_filtered_values() {
        let red = Color::from_rgb(255, 0, 0);
        let green = Color::from_rgb(0, 255, 0);
        let blue = Color::from_rgb(0, 0, 255);
        let values = vec![30.0, -10.0, 20.0];
        let config = PieConfig::new(vec!["a".to_string(), "b".to_string(), "c".to_string()])
            .explode(vec![0.0, 0.5, 0.0])
            .colors(vec![red, green, blue]);
        let data = PieData::from_values(&values, 100.0, 100.0, 50.0, &config);

        assert_eq!(data.values, vec![30.0, 20.0]);
        // The dropped value must take its label, explode offset and color with it.
        assert_eq!(data.config.labels, vec!["a".to_string(), "c".to_string()]);
        assert_eq!(data.config.explode, vec![0.0, 0.0]);
        assert_eq!(data.config.colors, Some(vec![red, blue]));
        assert!(data.wedges.iter().all(|wedge| wedge.explode.abs() < 1e-10));

        // Recomputing from already-filtered data is idempotent.
        let again = PieData::from_values(&data.values, 0.0, 0.0, 1.0, &data.config);
        assert_eq!(again.config.labels, data.config.labels);
        assert_eq!(again.config.colors, data.config.colors);
    }

    #[test]
    fn test_pie_all_values_positive_keeps_config_untouched() {
        let values = vec![30.0, 20.0, 50.0];
        let config = PieConfig::new(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        let data = PieData::from_values(&values, 100.0, 100.0, 50.0, &config);

        assert_eq!(data.config.labels, config.labels);
    }

    #[test]
    fn test_pie_starts_at_twelve_oclock_clockwise() {
        let values = vec![30.0, 26.0, 24.0, 20.0];
        let config = PieConfig::default();
        let (cx, cy) = (100.0, 100.0);
        let data = PieData::from_values(&values, cx, cy, 50.0, &config);

        // Default start_angle of 90° is 12 o'clock: the first wedge *begins*
        // there. Screen space has +y down, so "up" is -PI/2.
        let start = data.wedges[0].start_angle;
        assert!((start + PI / 2.0).abs() < 1e-9, "start: {start}");

        // The first input occupies the range immediately clockwise of 12
        // o'clock: 30 of 100 is 108°, so it ends past 3 o'clock.
        let sweep = data.wedges[0].end_angle - data.wedges[0].start_angle;
        assert!(sweep > 0.0, "first wedge must sweep clockwise, got {sweep}");
        assert!((sweep - 2.0 * PI * 0.3).abs() < 1e-9, "sweep: {sweep}");

        // Clockwise from the top puts the first wedge's midpoint in the
        // upper-right quadrant.
        let (mx, my) = data.wedges[0].centroid();
        assert!(mx > cx, "midpoint x {mx} should be right of {cx}");
        assert!(my < cy, "midpoint y {my} should be above {cy}");

        // Each wedge picks up where the previous one stopped, so input order is
        // read clockwise.
        for pair in data.wedges.windows(2) {
            assert!((pair[0].end_angle - pair[1].start_angle).abs() < 1e-9);
        }

        // Wedges cover the full turn.
        let swept: f64 = data
            .wedges
            .iter()
            .map(|wedge| (wedge.end_angle - wedge.start_angle).abs())
            .sum();
        assert!((swept - 2.0 * PI).abs() < 1e-9);
    }

    #[test]
    fn test_pie_counter_clockwise_reverses_direction() {
        let values = vec![30.0, 20.0, 50.0];
        let config = PieConfig::default().counter_clockwise();
        let (cx, cy) = (100.0, 100.0);
        let data = PieData::from_values(&values, cx, cy, 50.0, &config);

        // Still bounded by 12 o'clock, but sweeps toward the upper-left.
        assert!((data.wedges[0].end_angle + PI / 2.0).abs() < 1e-9);
        let (mx, my) = data.wedges[0].centroid();
        assert!(mx < cx, "midpoint x {mx} should be left of {cx}");
        assert!(my < cy, "midpoint y {my} should be above {cy}");
    }

    #[test]
    fn test_pie_start_angle_zero_is_three_oclock() {
        let values = vec![25.0, 75.0];
        let config = PieConfig::default().start_angle(0.0);
        let (cx, cy) = (0.0, 0.0);
        let data = PieData::from_values(&values, cx, cy, 1.0, &config);

        // start_angle 0 is the +x axis; the first wedge sweeps clockwise from
        // there, into the lower-right quadrant on screen.
        assert!(data.wedges[0].start_angle.abs() < 1e-9);
        let (mx, my) = data.wedges[0].centroid();
        assert!(mx > 0.0 && my > 0.0, "midpoint ({mx}, {my})");
    }

    /// A wedge label has to read on the wedge it sits on. Black on tab10 blue
    /// or red is the case that prompted this; the pale end of a palette still
    /// has to take dark text.
    #[test]
    fn test_label_color_follows_wedge_luminance() {
        let white = Color::from_rgb(255, 255, 255);
        let near_black = Color::from_rgb(26, 26, 26);

        // tab10 blue, green and red are dark enough that black text crushes.
        assert_eq!(label_color_on(Color::from_rgb(31, 119, 180)), white);
        assert_eq!(label_color_on(Color::from_rgb(44, 160, 44)), white);
        assert_eq!(label_color_on(Color::from_rgb(214, 39, 40)), white);
        // tab10 orange is above the split: 0.365 relative luminance.
        assert_eq!(label_color_on(Color::from_rgb(255, 127, 14)), near_black);
        // A pale yellow is nowhere near needing white text.
        assert_eq!(label_color_on(Color::from_rgb(255, 255, 179)), near_black);

        // The computed luminances the split above relies on.
        for (color, expected) in [
            (Color::from_rgb(31, 119, 180), 0.1678),
            (Color::from_rgb(255, 127, 14), 0.3647),
            (Color::from_rgb(214, 39, 40), 0.1590),
        ] {
            let luminance = relative_luminance(color);
            assert!(
                (luminance - expected).abs() < 5e-4,
                "{color:?} luminance {luminance} != {expected}"
            );
        }
    }

    /// A donut label has to land on the ring, not in the hole: the hole is
    /// background, and a white label there is invisible.
    #[test]
    fn test_donut_labels_land_on_the_ring() {
        let radius = 100.0;
        let inner = DEFAULT_DONUT_INNER_RADIUS;
        let label_r = label_radius(radius, inner, PieConfig::default().label_distance);
        assert!(
            label_r > radius * inner && label_r < radius,
            "donut label radius {label_r} must sit between the hole ({}) and the rim ({radius})",
            radius * inner
        );

        // A full pie is the `inner_radius = 0` case of the same expression.
        assert_eq!(label_radius(radius, 0.0, 0.6), 60.0);
    }

    /// An explicit `text_color` still wins over the automatic choice.
    #[test]
    fn test_explicit_text_color_overrides_the_automatic_choice() {
        let config = PieConfig::default().text_color(Color::from_rgb(1, 2, 3));
        assert_eq!(config.text_color, Some(Color::from_rgb(1, 2, 3)));
        assert_eq!(PieConfig::default().text_color, None);
    }

    #[test]
    fn test_pie_config_implements_plot_config() {
        fn assert_plot_config<T: PlotConfig>() {}
        assert_plot_config::<PieConfig>();
    }

    /// The direction setters were reported as inert (plan item 2.4). They are
    /// not: `PieData::from_values` reads `counter_clockwise`, and
    /// `render_styled` re-derives the wedges from the stored config, so the
    /// direction reaches the pixels. This test guards the render path that
    /// `test_pie_counter_clockwise_reverses_direction` does not cover.
    #[test]
    fn test_clockwise_changes_the_rendered_image() {
        fn render(config: PieConfig) -> Vec<u8> {
            let values = vec![10.0, 20.0, 70.0];
            let data = PieData::compute(&values, &config);
            let mut renderer = SkiaRenderer::new(160, 160, Theme::default()).unwrap();
            let area = PlotArea::new(0.0, 0.0, 160.0, 160.0, 0.0, 1.0, 0.0, 1.0);
            data.render(
                &mut renderer,
                &area,
                &Theme::default(),
                Color::from_rgb(0, 0, 0),
            )
            .unwrap();
            renderer.into_image().pixels
        }

        let cw = render(PieConfig::default().labels(false).percentages(false));
        let ccw = render(
            PieConfig::default()
                .labels(false)
                .percentages(false)
                .counter_clockwise(),
        );

        assert_ne!(
            ccw, cw,
            "PieConfig::counter_clockwise produced a byte-identical image"
        );
    }

    #[test]
    fn test_pie_plot_compute_trait() {
        use crate::plots::traits::PlotCompute;

        let values = vec![30.0, 20.0, 50.0];
        let config = PieConfig::default();
        let result = Pie::compute(&values, &config);

        assert!(result.is_ok());
        let pie_data = result.unwrap();
        assert_eq!(pie_data.wedges.len(), 3);
        assert!((pie_data.total - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_pie_plot_compute_empty() {
        use crate::plots::traits::PlotCompute;

        let values: Vec<f64> = vec![];
        let config = PieConfig::default();
        let result = Pie::compute(&values, &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_pie_plot_compute_all_negative() {
        use crate::plots::traits::PlotCompute;

        let values = vec![-10.0, -20.0];
        let config = PieConfig::default();
        let result = Pie::compute(&values, &config);

        assert!(result.is_err());
    }

    /// A whole percentage used to render "20.0%"; the trailing ".0" is noise on
    /// a wedge label, but a real fraction still has to survive.
    #[test]
    fn test_percentage_labels_drop_a_trailing_zero_decimal() {
        assert_eq!(format_percentage(20.0), "20%");
        assert_eq!(format_percentage(100.0), "100%");
        assert_eq!(format_percentage(20.5), "20.5%");
        assert_eq!(format_percentage(22.222), "22.2%");
        assert_eq!(format_percentage(0.04), "0%");
    }

    #[test]
    fn test_pie_plot_data_trait() {
        use crate::plots::traits::{PlotCompute, PlotData};

        let values = vec![30.0, 20.0, 50.0];
        let config = PieConfig::default();
        let pie_data = Pie::compute(&values, &config).unwrap();

        // Test data_bounds (pie uses normalized 0-1 space)
        let ((x_min, x_max), (y_min, y_max)) = pie_data.data_bounds();
        assert!((x_min - 0.0).abs() < 1e-10);
        assert!((x_max - 1.0).abs() < 1e-10);
        assert!((y_min - 0.0).abs() < 1e-10);
        assert!((y_max - 1.0).abs() < 1e-10);

        // Test is_empty
        assert!(!pie_data.is_empty());
    }
}
