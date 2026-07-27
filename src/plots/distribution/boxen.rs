//! Boxen (Letter-Value) plot implementations
//!
//! Provides enhanced box plots that show more quantile information.
//!
//! # Trait-Based API
//!
//! Boxen plots implement the core plot traits:
//! - [`PlotConfig`] for `BoxenConfig`
//! - [`PlotCompute`] for `Boxen` marker struct
//! - [`PlotData`] for `BoxenData`
//! - [`PlotRender`] for `BoxenData`

use crate::core::Result;
use crate::plots::traits::{PlotArea, PlotCompute, PlotConfig, PlotData, PlotRender};
use crate::render::skia::SkiaRenderer;
use crate::render::{Color, LineStyle, Theme};
use crate::stats::quantile::{letter_values_sorted, quantile_sorted};

/// Configuration for boxen plot
#[derive(Debug, Clone)]
pub struct BoxenConfig {
    /// Maximum number of letter value levels to show
    pub k_depth: Option<usize>,
    /// Width of boxes (fraction of category spacing)
    pub width: f64,
    /// Colors for boxes (None for auto)
    pub color: Option<Color>,
    /// Saturation gradient (darken toward center)
    pub saturation: f32,
    /// Show outliers
    pub show_outliers: bool,
    /// Outlier marker size
    pub outlier_size: f32,
    /// Line width for box edges
    pub line_width: f32,
    /// Orientation
    pub orient: BoxenOrientation,
    /// Category label written under this boxen on the x axis.
    ///
    /// Set with [`category`](Self::category()). Bars, box plots, violins and
    /// boxen plots share one category axis: slot *i* is centred on `i` and one
    /// data unit wide, so `width` is a fraction of that slot no
    /// matter how many boxes the figure holds.
    pub category: Option<String>,
    /// Explicit centre on the category axis; `None` claims the next free slot.
    ///
    /// Set with [`x_position`](Self::x_position()).
    pub x_position: Option<f64>,
}

/// Orientation for boxen plots
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BoxenOrientation {
    #[default]
    Vertical,
    Horizontal,
}

impl Default for BoxenConfig {
    fn default() -> Self {
        Self {
            k_depth: None, // Auto-determine based on data size
            width: 0.8,
            color: None,
            saturation: 0.75,
            show_outliers: true,
            outlier_size: 4.0,
            line_width: 1.0,
            orient: BoxenOrientation::Vertical,
            category: None,
            x_position: None,
        }
    }
}

impl BoxenConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set k-depth
    pub fn k_depth(mut self, k: usize) -> Self {
        self.k_depth = Some(k.max(1));
        self
    }

    /// Set width
    pub fn width(mut self, width: f64) -> Self {
        self.width = width.clamp(0.1, 1.0);
        self
    }

    /// Set color
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set saturation gradient
    pub fn saturation(mut self, saturation: f32) -> Self {
        self.saturation = saturation.clamp(0.0, 1.0);
        self
    }

    /// Show outliers
    pub fn show_outliers(mut self, show: bool) -> Self {
        self.show_outliers = show;
        self
    }

    /// Set outlier marker size
    pub fn outlier_size(mut self, size: f32) -> Self {
        self.outlier_size = size.max(0.0);
        self
    }

    /// Set box edge line width
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width.max(0.0);
        self
    }

    /// Set horizontal orientation
    pub fn horizontal(mut self) -> Self {
        self.orient = BoxenOrientation::Horizontal;
        self
    }

    /// Set vertical orientation
    pub fn vertical(mut self) -> Self {
        self.orient = BoxenOrientation::Vertical;
        self
    }
}

// Implement PlotConfig marker trait
impl PlotConfig for BoxenConfig {}

/// Marker struct for Boxen plot type (used with PlotCompute trait)
pub struct Boxen;

/// A single box in a boxen plot
#[derive(Debug, Clone)]
pub struct BoxenBox {
    /// Level (0 = outermost, increasing toward center)
    pub level: usize,
    /// Lower bound of box
    pub lower: f64,
    /// Upper bound of box
    pub upper: f64,
    /// Width (relative, increases toward the center)
    pub width: f64,
}

/// Computed boxen data for a single distribution
#[derive(Debug, Clone)]
pub struct BoxenData {
    /// All boxes from outer to inner
    pub boxes: Vec<BoxenBox>,
    /// Median value
    pub median: f64,
    /// Outlier values
    pub outliers: Vec<f64>,
    /// Original data range
    pub data_range: (f64, f64),
    /// Configuration used to compute this data
    pub(crate) config: BoxenConfig,
}

impl BoxenData {
    /// Half-width of the median line.
    ///
    /// The median line spans the innermost band, which is the widest one, so it is
    /// sized from that band rather than from a fixed fraction of the nominal width.
    /// `compute_boxen` never stores the degenerate `(median, median)` letter value as
    /// a band, so `boxes.last()` is always a band with real height and the line can
    /// never overhang it.
    pub(crate) fn median_half_width(&self) -> f64 {
        self.boxes
            .last()
            .map_or(self.config.width / 4.0, |innermost| innermost.width / 2.0)
    }
}

/// Compute boxen plot data
///
/// # Arguments
/// * `data` - Input data
/// * `config` - Boxen configuration
///
/// # Returns
/// BoxenData for rendering
pub fn compute_boxen(data: &[f64], config: &BoxenConfig) -> BoxenData {
    if data.is_empty() {
        return BoxenData {
            boxes: vec![],
            median: 0.0,
            outliers: vec![],
            data_range: (0.0, 1.0),
            config: config.clone(),
        };
    }

    let mut sorted: Vec<f64> = data.iter().copied().filter(|x| x.is_finite()).collect();
    sorted.sort_by(f64::total_cmp);

    if sorted.is_empty() {
        return BoxenData {
            boxes: vec![],
            median: 0.0,
            outliers: vec![],
            data_range: (0.0, 1.0),
            config: config.clone(),
        };
    }

    let n = sorted.len();

    // Determine k depth
    let k = config.k_depth.unwrap_or_else(|| {
        // Tukey's criterion: use log2(n) levels
        ((n as f64).log2().floor() as usize).clamp(1, 10)
    });

    // Get letter values. `letter_values_sorted` yields the median first — as the
    // degenerate pair `(median, median)`, which is the honest depth-1 letter value —
    // and widens outward from there.
    let lvs = letter_values_sorted(&sorted, Some(k));

    // The median is drawn as a line, not as a band, so it must not occupy a band
    // slot: keeping it produced a zero-height rectangle that consumed the full-width
    // step of the taper, capping the visible plot at `(k-1)/k` of `config.width` and
    // making the median line (sized from the widest band) overhang the widest band
    // that actually has height. Drop it, then reverse so `boxes[0]` is the outermost
    // band — required by both the width taper and the outlier test below.
    let bands = if lvs.len() > 1 { &lvs[1..] } else { &[][..] };

    // Create boxes
    let mut boxes = Vec::with_capacity(bands.len());
    let num_levels = bands.len();

    for (level, (lower, upper)) in bands.iter().rev().enumerate() {
        // seaborn's `_LVPlotter` linear width function: the band at index `i`,
        // counted outermost-first, is `(i + 1) / k` of the nominal width. The
        // innermost band (which brackets the median) is therefore full width and
        // the tails taper to a spike — the "wedding cake" silhouette. Tapering
        // the other way would put the widest slab at the extreme letter values,
        // which is exactly backwards.
        let width_factor = (level + 1) as f64 / num_levels as f64;

        boxes.push(BoxenBox {
            level,
            lower: *lower,
            upper: *upper,
            width: config.width * width_factor,
        });
    }

    // A sample too small for a second letter value (n < 8) leaves no band at all.
    // Degenerate to a plain box on the fourths so something still renders.
    if boxes.is_empty() {
        boxes.push(BoxenBox {
            level: 0,
            lower: quantile_sorted(&sorted, 0.25),
            upper: quantile_sorted(&sorted, 0.75),
            width: config.width,
        });
    }

    // Compute median
    let median = if n.is_multiple_of(2) {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    };

    // Find outliers (outside the outermost band). The degenerate fourths fallback
    // above is not a letter-value tail, so it flags nothing — otherwise roughly half
    // the sample would be drawn as outliers.
    let outliers = if config.show_outliers && lvs.len() > 1 {
        let outer_lower = boxes[0].lower;
        let outer_upper = boxes[0].upper;
        sorted
            .iter()
            .copied()
            .filter(|&x| x < outer_lower || x > outer_upper)
            .collect()
    } else {
        vec![]
    };

    BoxenData {
        boxes,
        median,
        outliers,
        data_range: (sorted[0], sorted[n - 1]),
        config: config.clone(),
    }
}

/// Generate box rectangle vertices
pub fn boxen_rect(boxen: &BoxenBox, center: f64, orient: BoxenOrientation) -> Vec<(f64, f64)> {
    let half_width = boxen.width / 2.0;

    match orient {
        BoxenOrientation::Vertical => {
            vec![
                (center - half_width, boxen.lower),
                (center + half_width, boxen.lower),
                (center + half_width, boxen.upper),
                (center - half_width, boxen.upper),
            ]
        }
        BoxenOrientation::Horizontal => {
            vec![
                (boxen.lower, center - half_width),
                (boxen.upper, center - half_width),
                (boxen.upper, center + half_width),
                (boxen.lower, center + half_width),
            ]
        }
    }
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl PlotCompute for Boxen {
    type Input<'a> = &'a [f64];
    type Config = BoxenConfig;
    type Output = BoxenData;

    fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output> {
        let result = compute_boxen(input, config);
        if result.boxes.is_empty() && input.iter().any(|x| x.is_finite()) {
            // Data exists but boxes couldn't be computed - very unusual
            Ok(result)
        } else if result.boxes.is_empty() {
            Err(crate::core::PlottingError::EmptyDataSet)
        } else {
            Ok(result)
        }
    }
}

impl PlotData for BoxenData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        // The category axis carries one unit-wide slot per boxen, centred on
        // the position the series was assigned; the other axis spans the data.
        let slot = crate::plots::boxplot::category_slot_span(self.config.x_center());
        match self.config.orient {
            BoxenOrientation::Vertical => (slot, self.data_range),
            BoxenOrientation::Horizontal => (self.data_range, slot),
        }
    }

    fn is_empty(&self) -> bool {
        self.boxes.is_empty()
    }
}

impl PlotRender for BoxenData {
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        _theme: &Theme,
        color: Color,
    ) -> Result<()> {
        self.render_styled(renderer, area, _theme, color, 1.0, None)
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
        if self.boxes.is_empty() {
            return Ok(());
        }

        let config = &self.config;
        let render_scale = renderer.render_scale();
        let line_width_points = line_width.unwrap_or(config.line_width);
        let line_width_px = render_scale.points_to_pixels(line_width_points);
        let median_line_width_px = render_scale.points_to_pixels(2.0);
        let outlier_size_px = render_scale.points_to_pixels(config.outlier_size);
        // The stack of boxes straddles the centre of its own category slot.
        let center = config.x_center();
        let base_color = config.color.unwrap_or(color);
        let base_color =
            base_color.with_alpha((f32::from(base_color.a) / 255.0) * alpha.clamp(0.0, 1.0));

        // Draw boxes from outermost to innermost (so inner boxes overlay outer)
        for (i, boxen_box) in self.boxes.iter().enumerate() {
            // Generate saturation gradient (lighter toward outside)
            let saturation_factor = boxen_saturation_factor(i, self.boxes.len(), config.saturation);
            let adjusted_color = adjust_saturation(base_color, saturation_factor);

            // Get rectangle vertices
            let rect = boxen_rect(boxen_box, center, config.orient);

            // Convert to screen coordinates
            let screen_points: Vec<(f32, f32)> = rect
                .iter()
                .map(|(x, y)| area.data_to_screen(*x, *y))
                .collect();

            // Draw filled rectangle
            if screen_points.len() >= 3 {
                renderer.draw_filled_polygon(&screen_points, adjusted_color)?;
            }

            // Draw outline
            if line_width_points > 0.0 {
                let mut outline = screen_points.clone();
                outline.push(screen_points[0]); // Close the path
                renderer.draw_polyline(&outline, base_color, line_width_px, LineStyle::Solid)?;
            }
        }

        // Draw median line, spanning the innermost (widest) band
        let median_half = self.median_half_width();
        match config.orient {
            BoxenOrientation::Vertical => {
                let (x1, y) = area.data_to_screen(center - median_half, self.median);
                let (x2, _) = area.data_to_screen(center + median_half, self.median);
                renderer.draw_line(
                    x1,
                    y,
                    x2,
                    y,
                    Color::from_rgb(255, 255, 255).with_alpha(alpha),
                    median_line_width_px,
                    LineStyle::Solid,
                )?;
            }
            BoxenOrientation::Horizontal => {
                let (x, y1) = area.data_to_screen(self.median, center - median_half);
                let (_, y2) = area.data_to_screen(self.median, center + median_half);
                renderer.draw_line(
                    x,
                    y1,
                    x,
                    y2,
                    Color::from_rgb(255, 255, 255).with_alpha(alpha),
                    median_line_width_px,
                    LineStyle::Solid,
                )?;
            }
        }

        // Draw outliers
        if config.show_outliers {
            for &outlier in &self.outliers {
                let (px, py) = match config.orient {
                    BoxenOrientation::Vertical => area.data_to_screen(center, outlier),
                    BoxenOrientation::Horizontal => area.data_to_screen(outlier, center),
                };
                renderer.draw_marker(
                    px,
                    py,
                    outlier_size_px,
                    crate::render::MarkerStyle::Circle,
                    base_color,
                )?;
            }
        }

        Ok(())
    }
}

/// Saturation factor for the `index`-th band, counted outermost-first.
///
/// The innermost band keeps full saturation and outer bands fade toward gray,
/// which is the gradient seaborn's letter-value plots use.
pub(crate) fn boxen_saturation_factor(index: usize, count: usize, saturation: f32) -> f32 {
    if count == 0 {
        return 1.0;
    }
    let steps_from_center = (count - 1 - index.min(count - 1)) as f32;
    1.0 - (steps_from_center / count as f32) * saturation
}

/// Adjust color saturation (simple approximation)
///
/// Shared by the raster and SVG boxen paths so both blend identically.
pub(crate) fn adjust_saturation(color: Color, factor: f32) -> Color {
    // Blend toward gray for lower saturation
    let gray = ((color.r as f32 + color.g as f32 + color.b as f32) / 3.0) as u8;
    let blend = |c: u8| -> u8 {
        ((c as f32 * factor + gray as f32 * (1.0 - factor)).clamp(0.0, 255.0)) as u8
    };
    Color::from_rgba(blend(color.r), blend(color.g), blend(color.b), color.a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boxen_basic() {
        let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let config = BoxenConfig::default();
        let boxen = compute_boxen(&data, &config);

        assert!(!boxen.boxes.is_empty());
        assert!((boxen.median - 49.5).abs() < 1e-10);
    }

    #[test]
    fn test_boxen_nested_boxes() {
        let data: Vec<f64> = (0..1000).map(|i| i as f64).collect();
        let config = BoxenConfig::default().k_depth(5);
        let boxen = compute_boxen(&data, &config);

        // 5 letter-value levels = the median plus 4 bands; the median is a line.
        assert_eq!(boxen.boxes.len(), 4);

        // Each inner box should be wider (seaborn's wedding-cake taper)
        for i in 1..boxen.boxes.len() {
            assert!(boxen.boxes[i].width >= boxen.boxes[i - 1].width);
        }
    }

    #[test]
    fn test_boxen_boxes_ordered_outermost_first() {
        let data: Vec<f64> = (0..1000).map(|i| i as f64).collect();
        let config = BoxenConfig::default().k_depth(5);
        let boxen = compute_boxen(&data, &config);

        assert_eq!(boxen.boxes.len(), 4);

        // boxes[0] is the tallest band and each following band nests inside it,
        // so the paint order (index 0 first) leaves the median band on top.
        for i in 1..boxen.boxes.len() {
            assert!(boxen.boxes[i].lower >= boxen.boxes[i - 1].lower);
            assert!(boxen.boxes[i].upper <= boxen.boxes[i - 1].upper);
            assert!(boxen.boxes[i].width > boxen.boxes[i - 1].width);
        }

        // seaborn's linear taper over the 4 bands: the innermost is full width and
        // the outermost is 1/4 of it.
        let innermost = boxen.boxes.last().unwrap();
        assert!((innermost.width - config.width).abs() < 1e-10);
        assert!((boxen.boxes[0].width - config.width / 4.0).abs() < 1e-10);

        // The innermost band brackets the median and has real height, so the median
        // line drawn across it cannot overhang.
        assert!(innermost.lower < innermost.upper);
        assert!(innermost.lower <= boxen.median && boxen.median <= innermost.upper);
    }

    #[test]
    fn test_boxen_no_degenerate_zero_height_band() {
        // The `(median, median)` letter value must never become a band: it used to
        // land at `boxes.last()` carrying the full-width taper step, which both threw
        // away the widest step and made the median line overhang its box.
        for k in 2..=8 {
            let data: Vec<f64> = (0..1000).map(|i| i as f64).collect();
            let boxen = compute_boxen(&data, &BoxenConfig::default().k_depth(k));
            for b in &boxen.boxes {
                assert!(b.lower < b.upper, "k={k} produced a zero-height band");
            }
            let widest = boxen
                .boxes
                .iter()
                .map(|b| b.width)
                .fold(f64::NEG_INFINITY, f64::max);
            assert!(
                (boxen.median_half_width() * 2.0 - widest).abs() < 1e-10,
                "k={k}: median line must match the widest band exactly"
            );
        }
    }

    #[test]
    fn test_boxen_small_sample_still_renders_a_box() {
        // n < 8 admits only the degenerate median letter value; fall back to the
        // fourths rather than emitting nothing, and flag no outliers.
        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let boxen = compute_boxen(&data, &BoxenConfig::default());

        assert_eq!(boxen.boxes.len(), 1);
        assert!(boxen.boxes[0].lower < boxen.boxes[0].upper);
        assert!((boxen.boxes[0].width - BoxenConfig::default().width).abs() < 1e-10);
        assert!(boxen.outliers.is_empty());
    }

    #[test]
    fn test_boxen_outliers_are_the_extreme_tail_only() {
        // 1000 uniform points, 5 levels: the outermost band spans the
        // 1/32..31/32 quantiles, so ~6.25% of the sample falls outside it.
        let data: Vec<f64> = (0..1000).map(|i| i as f64).collect();
        let config = BoxenConfig::default().k_depth(5);
        let boxen = compute_boxen(&data, &config);

        let ratio = boxen.outliers.len() as f64 / data.len() as f64;
        assert!(
            ratio < 0.10,
            "expected only the extreme tail to be flagged, got {ratio}"
        );

        // Every flagged point really is outside the outermost band.
        let outer = &boxen.boxes[0];
        for &outlier in &boxen.outliers {
            assert!(outlier < outer.lower || outlier > outer.upper);
        }
    }

    #[test]
    fn test_boxen_saturation_darkest_at_center() {
        // Outer bands fade toward gray; the innermost band keeps full saturation.
        let outer = boxen_saturation_factor(0, 5, 0.75);
        let inner = boxen_saturation_factor(4, 5, 0.75);
        assert!(outer < inner);
        assert!((inner - 1.0).abs() < 1e-6);
        assert!((outer - (1.0 - 0.8 * 0.75)).abs() < 1e-6);
        // Degenerate inputs stay in range.
        assert!((boxen_saturation_factor(0, 0, 0.75) - 1.0).abs() < 1e-6);
        assert!((boxen_saturation_factor(0, 1, 0.75) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_boxen_median_line_spans_innermost_band() {
        let data: Vec<f64> = (0..1000).map(|i| i as f64).collect();
        let boxen = compute_boxen(&data, &BoxenConfig::default().k_depth(5));

        let innermost = boxen.boxes.last().unwrap();
        assert!((boxen.median_half_width() - innermost.width / 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_boxen_rect_vertical() {
        let box_data = BoxenBox {
            level: 0,
            lower: 10.0,
            upper: 20.0,
            width: 0.8,
        };
        let rect = boxen_rect(&box_data, 0.0, BoxenOrientation::Vertical);

        assert_eq!(rect.len(), 4);
        // Check that rectangle covers the right range
        assert!((rect[0].1 - 10.0).abs() < 1e-10); // lower
        assert!((rect[2].1 - 20.0).abs() < 1e-10); // upper
    }

    #[test]
    fn test_boxen_empty() {
        let data: Vec<f64> = vec![];
        let config = BoxenConfig::default();
        let boxen = compute_boxen(&data, &config);

        assert!(boxen.boxes.is_empty());
    }

    #[test]
    fn test_boxen_config_implements_plot_config() {
        fn assert_plot_config<T: PlotConfig>() {}
        assert_plot_config::<BoxenConfig>();
    }

    #[test]
    fn test_boxen_plot_compute_trait() {
        use crate::plots::traits::PlotCompute;

        let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let config = BoxenConfig::default();
        let result = Boxen::compute(&data, &config);

        assert!(result.is_ok());
        let boxen_data = result.unwrap();
        assert!(!boxen_data.boxes.is_empty());
    }

    #[test]
    fn test_boxen_plot_compute_empty() {
        use crate::plots::traits::PlotCompute;

        let data: Vec<f64> = vec![];
        let config = BoxenConfig::default();
        let result = Boxen::compute(&data, &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_boxen_plot_data_trait() {
        use crate::plots::traits::PlotData;

        let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let config = BoxenConfig::default();
        let boxen_data = compute_boxen(&data, &config);

        // Test data_bounds
        let ((x_min, x_max), (y_min, y_max)) = boxen_data.data_bounds();
        assert!(x_min <= x_max);
        assert!(y_min <= y_max);

        // Test is_empty
        assert!(!boxen_data.is_empty());
    }

    #[test]
    fn test_adjust_saturation() {
        let color = Color::from_rgb(100, 150, 200);
        let adjusted = super::adjust_saturation(color, 0.5);
        // Should be blended toward gray
        assert!(adjusted.r > 0 && adjusted.r < 255);
        assert!(adjusted.g > 0 && adjusted.g < 255);
        assert!(adjusted.b > 0 && adjusted.b < 255);
    }

    #[test]
    fn test_boxen_sits_in_its_own_category_slot() {
        use crate::plots::traits::PlotData as _;

        let data: Vec<f64> = (0..50).map(|i| i as f64).collect();

        let first = compute_boxen(&data, &BoxenConfig::new());
        assert_eq!(first.config.x_center(), 0.0);
        assert_eq!(first.data_bounds().0, (-0.5, 0.5));

        let second = compute_boxen(&data, &BoxenConfig::new().x_position(1.0));
        assert_eq!(second.data_bounds().0, (0.5, 1.5));
    }

    #[test]
    fn test_horizontal_boxen_puts_its_slot_on_the_y_axis() {
        use crate::plots::traits::PlotData as _;

        let data: Vec<f64> = (0..50).map(|i| i as f64).collect();
        let config = BoxenConfig::new().horizontal().x_position(2.0);
        let boxen = compute_boxen(&data, &config);

        assert_eq!(boxen.data_bounds().1, (1.5, 2.5));
    }

    #[test]
    fn test_boxen_rect_straddles_the_slot_centre() {
        let data: Vec<f64> = (0..50).map(|i| i as f64).collect();
        let config = BoxenConfig::new().x_position(1.0);
        let boxen = compute_boxen(&data, &config);
        let outermost = &boxen.boxes[0];

        let rect = boxen_rect(outermost, config.x_center(), config.orient);
        let xs: Vec<f64> = rect.iter().map(|(x, _)| *x).collect();
        let min = xs.iter().copied().fold(f64::INFINITY, f64::min);
        let max = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        assert!((((min + max) / 2.0) - 1.0).abs() < 1e-12);
        assert!(max - min <= 1.0, "a boxen must fit inside its own slot");
    }
}
