//! Strip plot implementations
//!
//! Provides strip plots (jittered scatter for categorical data).
//!
//! # Trait-Based API
//!
//! Strip plots implement the core plot traits:
//! - [`PlotConfig`] for `StripConfig`
//! - [`PlotCompute`] for `Strip` marker struct
//! - [`PlotData`] for `StripData`
//! - [`PlotRender`] for `StripData`

use crate::core::Result;
use crate::plots::traits::{
    AxisScaleSupport, ComputedSeries, ComputedStyle, LegendKey, PlotArea, PlotCompute, PlotConfig,
    PlotData, PlotPrimitive, PlotRender, draw_primitives,
};
use crate::render::skia::SkiaRenderer;
use crate::render::{Color, MarkerStyle, Theme};

/// Configuration for strip plot
#[derive(Debug, Clone)]
pub struct StripConfig {
    /// Jitter amount (0.0-1.0 as fraction of category spacing)
    pub jitter: f64,
    /// Marker size, in **points**
    pub size: f32,
    /// Marker color (None for auto)
    pub color: Option<Color>,
    /// Marker alpha
    pub alpha: f32,
    /// Orientation
    pub orientation: StripOrientation,
    /// Dodge groups (for grouped strip plots)
    pub dodge: bool,
    /// Random seed for jitter
    pub seed: u64,
}

/// Orientation for strip plots
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StripOrientation {
    #[default]
    Vertical,
    Horizontal,
}

impl Default for StripConfig {
    fn default() -> Self {
        Self {
            jitter: 0.3,
            size: 5.0,
            color: None,
            alpha: 0.8,
            orientation: StripOrientation::Vertical,
            dodge: false,
            seed: 42,
        }
    }
}

impl StripConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set jitter amount
    pub fn jitter(mut self, jitter: f64) -> Self {
        self.jitter = jitter.clamp(0.0, 1.0);
        self
    }

    /// Set marker size, in points
    pub fn size(mut self, size: f32) -> Self {
        self.size = size.max(0.1);
        self
    }

    /// Set color
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set alpha
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Set horizontal orientation
    pub fn horizontal(mut self) -> Self {
        self.orientation = StripOrientation::Horizontal;
        self
    }

    /// Set vertical orientation (the default)
    pub fn vertical(mut self) -> Self {
        self.orientation = StripOrientation::Vertical;
        self
    }

    /// Enable dodging for groups
    pub fn dodge(mut self, dodge: bool) -> Self {
        self.dodge = dodge;
        self
    }

    /// Set random seed
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }
}

// Implement PlotConfig marker trait
impl PlotConfig for StripConfig {}

/// Marker struct for Strip plot type (used with PlotCompute trait)
pub struct Strip;

/// A single point in a strip plot
#[derive(Debug, Clone, Copy)]
pub struct StripPoint {
    /// Category index
    pub category: usize,
    /// Value
    pub value: f64,
    /// Jittered x position
    pub x: f64,
    /// Y position (value for vertical, category for horizontal)
    pub y: f64,
    /// Optional group index
    pub group: Option<usize>,
}

/// Simple pseudo-random number generator
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_f64(&mut self) -> f64 {
        // xorshift64
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        (self.state as f64) / (u64::MAX as f64)
    }
}

/// Compute strip plot points
///
/// # Arguments
/// * `categories` - Category indices for each point
/// * `values` - Values for each point
/// * `groups` - Optional group indices
/// * `config` - Strip plot configuration
///
/// # Returns
/// Vec of StripPoint
pub fn compute_strip_points(
    categories: &[usize],
    values: &[f64],
    groups: Option<&[usize]>,
    config: &StripConfig,
) -> Vec<StripPoint> {
    let n = categories.len().min(values.len());
    if n == 0 {
        return vec![];
    }

    let mut rng = SimpleRng::new(config.seed);
    let mut points = Vec::with_capacity(n);

    // Find number of groups for dodging
    let num_groups = groups.map_or(1, |g| g.iter().max().map_or(1, |&m| m + 1));

    for i in 0..n {
        let cat = categories[i];
        let val = values[i];
        let grp = groups.map(|g| g.get(i).copied().unwrap_or(0));

        // Compute jitter
        let jitter = (rng.next_f64() - 0.5) * config.jitter;

        // Compute position with optional dodging
        let base_x = cat as f64;
        let x = if config.dodge && num_groups > 1 {
            let grp_idx = grp.unwrap_or(0);
            let dodge_width = 0.8 / num_groups as f64;
            let dodge_offset = (grp_idx as f64 - (num_groups - 1) as f64 / 2.0) * dodge_width;
            base_x + dodge_offset + jitter * dodge_width
        } else {
            base_x + jitter
        };

        let (x, y) = match config.orientation {
            StripOrientation::Vertical => (x, val),
            StripOrientation::Horizontal => (val, x),
        };

        points.push(StripPoint {
            category: cat,
            value: val,
            x,
            y,
            group: grp,
        });
    }

    points
}

/// Compute data range for strip plot
pub fn strip_range(
    points: &[StripPoint],
    num_categories: usize,
    orientation: StripOrientation,
) -> ((f64, f64), (f64, f64)) {
    if points.is_empty() {
        return ((0.0, 1.0), (0.0, 1.0));
    }

    let val_min = points.iter().map(|p| p.value).fold(f64::INFINITY, f64::min);
    let val_max = points
        .iter()
        .map(|p| p.value)
        .fold(f64::NEG_INFINITY, f64::max);

    let cat_range = (-0.5, num_categories as f64 - 0.5);

    match orientation {
        StripOrientation::Vertical => (cat_range, (val_min, val_max)),
        StripOrientation::Horizontal => ((val_min, val_max), cat_range),
    }
}

// ============================================================================
// Trait-Based API
// ============================================================================

/// Computed strip plot data
#[derive(Debug, Clone)]
pub struct StripData {
    /// All computed points
    pub points: Vec<StripPoint>,
    /// Number of categories
    pub num_categories: usize,
    /// Name of each category, in slot order. Empty when the caller supplied
    /// bare slot indices, in which case the axis has nothing to print.
    pub category_names: Vec<String>,
    /// Configuration used to compute this data
    pub(crate) config: StripConfig,
}

/// Input for strip plot computation
pub struct StripInput<'a> {
    /// Category indices
    pub categories: &'a [usize],
    /// Values
    pub values: &'a [f64],
    /// Optional group indices
    pub groups: Option<&'a [usize]>,
    /// Optional category names, in slot order, for the category axis.
    pub names: Option<&'a [String]>,
}

impl<'a> StripInput<'a> {
    /// Create new strip input
    pub fn new(categories: &'a [usize], values: &'a [f64]) -> Self {
        Self {
            categories,
            values,
            groups: None,
            names: None,
        }
    }

    /// Add groups
    pub fn with_groups(mut self, groups: &'a [usize]) -> Self {
        self.groups = Some(groups);
        self
    }

    /// Name each category slot, so the x axis prints the names the caller used
    /// instead of the slot numbers.
    pub fn with_names(mut self, names: &'a [String]) -> Self {
        self.names = Some(names);
        self
    }
}

impl PlotCompute for Strip {
    type Input<'a> = StripInput<'a>;
    type Config = StripConfig;
    type Output = StripData;

    fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output> {
        let points = compute_strip_points(input.categories, input.values, input.groups, config);

        if points.is_empty() {
            return Err(crate::core::PlottingError::EmptyDataSet);
        }

        // Calculate number of categories
        let num_categories = input.categories.iter().max().map_or(0, |&m| m + 1);

        Ok(StripData {
            points,
            num_categories,
            category_names: input.names.map(<[String]>::to_vec).unwrap_or_default(),
            config: config.clone(),
        })
    }
}

impl PlotData for StripData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        strip_range(&self.points, self.num_categories, self.config.orientation)
    }

    fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

impl ComputedSeries for StripData {
    fn kind(&self) -> &'static str {
        "strip"
    }

    fn point_count(&self) -> usize {
        self.points.len()
    }

    /// One slot per category, centred on its index — the same unit-wide slots
    /// bars and box plots use, so the names the caller passed get printed under
    /// the columns instead of raw numbers.
    fn category_slots(&self) -> Vec<(String, f64)> {
        crate::plots::boxplot::category_slots(&self.category_names, self.num_categories)
    }

    fn category_orientation(&self) -> crate::core::Orientation {
        match self.config.orientation {
            StripOrientation::Vertical => crate::core::Orientation::Vertical,
            StripOrientation::Horizontal => crate::core::Orientation::Horizontal,
        }
    }

    /// A cloud of markers, so the key is a marker — a line swatch would claim
    /// the observations are joined up, which is the one thing a strip plot is
    /// deliberately not doing.
    fn legend_key(&self) -> LegendKey {
        LegendKey::Marker
    }

    /// The category axis carries ordinal slots, so it has no quantitative
    /// spacing to take a logarithm of; the value axis is projected and scales
    /// freely. Same rule, same wording, as a bar chart's.
    fn axis_scale_support(&self) -> (AxisScaleSupport, AxisScaleSupport) {
        match self.config.orientation {
            StripOrientation::Vertical => (AxisScaleSupport::ORDINAL, AxisScaleSupport::Scaled),
            StripOrientation::Horizontal => (AxisScaleSupport::Scaled, AxisScaleSupport::ORDINAL),
        }
    }

    /// One marker per observation, at the position the layout already chose.
    fn primitives(&self, area: &PlotArea, style: &ComputedStyle) -> Vec<PlotPrimitive> {
        let config = &self.config;
        let base = config.color.unwrap_or(style.color);
        // The configured alpha and the series alpha compose, so a translucent
        // cloud stays translucent when the series is faded further.
        let color = base
            .with_alpha((f32::from(base.a) / 255.0) * config.alpha * style.alpha.clamp(0.0, 1.0));
        // `size` is in points, like every other marker size in the crate; the
        // render scale is what keeps the dots the same physical size at any DPI.
        let size_px = style.scale.points_to_pixels(config.size);

        self.points
            .iter()
            .filter_map(|point| {
                // A point the axes cannot place has no position, so it is
                // dropped rather than drawn at a NaN pixel.
                let at = area.try_data_to_screen(point.x, point.y)?;
                Some(PlotPrimitive::Marker {
                    at,
                    size_px,
                    style: MarkerStyle::Circle,
                    color,
                })
            })
            .collect()
    }
}

impl PlotRender for StripData {
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
        theme: &Theme,
        color: Color,
        alpha: f32,
        _line_width: Option<f32>,
    ) -> Result<()> {
        let style = ComputedStyle {
            scale: renderer.render_scale(),
            color,
            alpha,
            line_width: None,
            patch_edge_color: theme.patch_edge_color,
        };
        draw_primitives(renderer, &self.primitives(area, &style))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_basic() {
        let categories = vec![0, 0, 0, 1, 1, 1, 2, 2, 2];
        let values = vec![1.0, 1.5, 2.0, 3.0, 3.5, 4.0, 2.0, 2.5, 3.0];
        let config = StripConfig::default();
        let points = compute_strip_points(&categories, &values, None, &config);

        assert_eq!(points.len(), 9);
        // Points should be jittered around their categories
        for point in &points {
            let expected_x = point.category as f64;
            assert!((point.x - expected_x).abs() < 0.5);
        }
    }

    #[test]
    fn test_strip_horizontal() {
        let categories = vec![0, 1, 2];
        let values = vec![1.0, 2.0, 3.0];
        let config = StripConfig::default().horizontal();
        let points = compute_strip_points(&categories, &values, None, &config);

        // For horizontal, x should be value, y should be category
        for point in &points {
            assert!((point.x - point.value).abs() < 1e-10);
        }
    }

    #[test]
    fn test_strip_with_groups() {
        let categories = vec![0, 0, 1, 1];
        let values = vec![1.0, 2.0, 1.0, 2.0];
        let groups = vec![0, 1, 0, 1];
        let config = StripConfig::default().dodge(true);
        let points = compute_strip_points(&categories, &values, Some(&groups), &config);

        assert_eq!(points.len(), 4);
        // Each point should have a group
        for point in &points {
            assert!(point.group.is_some());
        }
    }

    #[test]
    fn test_strip_range() {
        let categories = vec![0, 1, 2];
        let values = vec![1.0, 5.0, 3.0];
        let config = StripConfig::default();
        let points = compute_strip_points(&categories, &values, None, &config);
        let ((x_min, x_max), (y_min, y_max)) = strip_range(&points, 3, StripOrientation::Vertical);

        assert!(x_min < 0.0);
        assert!(x_max > 2.0);
        assert!((y_min - 1.0).abs() < 1e-10);
        assert!((y_max - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_strip_empty() {
        let categories: Vec<usize> = vec![];
        let values: Vec<f64> = vec![];
        let config = StripConfig::default();
        let points = compute_strip_points(&categories, &values, None, &config);

        assert!(points.is_empty());
    }

    #[test]
    fn test_strip_config_implements_plot_config() {
        fn assert_plot_config<T: PlotConfig>() {}
        assert_plot_config::<StripConfig>();
    }

    #[test]
    fn test_strip_plot_compute_trait() {
        use crate::plots::traits::PlotCompute;

        let categories = vec![0, 0, 1, 1, 2, 2];
        let values = vec![1.0, 1.5, 2.0, 2.5, 3.0, 3.5];
        let config = StripConfig::default();
        let input = StripInput::new(&categories, &values);
        let result = Strip::compute(input, &config);

        assert!(result.is_ok());
        let strip_data = result.unwrap();
        assert_eq!(strip_data.points.len(), 6);
        assert_eq!(strip_data.num_categories, 3);
    }

    #[test]
    fn test_strip_plot_compute_with_groups() {
        use crate::plots::traits::PlotCompute;

        let categories = vec![0, 0, 1, 1];
        let values = vec![1.0, 2.0, 1.0, 2.0];
        let groups = vec![0, 1, 0, 1];
        let config = StripConfig::default().dodge(true);
        let input = StripInput::new(&categories, &values).with_groups(&groups);
        let result = Strip::compute(input, &config);

        assert!(result.is_ok());
        let strip_data = result.unwrap();
        assert_eq!(strip_data.points.len(), 4);
    }

    #[test]
    fn test_strip_plot_compute_empty() {
        use crate::plots::traits::PlotCompute;

        let categories: Vec<usize> = vec![];
        let values: Vec<f64> = vec![];
        let config = StripConfig::default();
        let input = StripInput::new(&categories, &values);
        let result = Strip::compute(input, &config);

        assert!(result.is_err());
    }

    fn strip_ink(dpi_scale: f32) -> usize {
        let categories = vec![0, 1, 2];
        let values = vec![1.0, 5.0, 3.0];
        let data = Strip::compute(
            StripInput::new(&categories, &values),
            &StripConfig::default(),
        )
        .unwrap();

        let mut renderer = SkiaRenderer::new(200, 200, Theme::default()).unwrap();
        renderer.set_dpi_scale(dpi_scale);
        let ((x_min, x_max), (y_min, y_max)) = data.data_bounds();
        // Inset so every marker is fully on canvas at both render scales.
        let area = PlotArea::new(20.0, 20.0, 160.0, 160.0, x_min, x_max, y_min, y_max);
        data.render(
            &mut renderer,
            &area,
            &Theme::default(),
            Color::from_rgb(200, 0, 0),
        )
        .unwrap();

        renderer
            .into_image()
            .pixels
            .chunks_exact(4)
            .filter(|p| p[3] > 0 && (p[0] < 250 || p[1] < 250 || p[2] < 250))
            .count()
    }

    #[test]
    fn test_strip_markers_keep_their_physical_size_at_higher_dpi() {
        // `size` is in points like every other marker size in the crate, so
        // doubling the render scale must double the dot diameter. Passing the
        // raw number through as pixels left the dots the same size at 300 DPI.
        let single = strip_ink(1.0);
        let double = strip_ink(2.0);

        assert!(
            double > single * 2,
            "strip markers did not grow with DPI ({double} vs {single} inked pixels)"
        );
    }

    #[test]
    fn test_strip_plot_data_trait() {
        use crate::plots::traits::{PlotCompute, PlotData};

        let categories = vec![0, 1, 2];
        let values = vec![1.0, 5.0, 3.0];
        let config = StripConfig::default();
        let input = StripInput::new(&categories, &values);
        let strip_data = Strip::compute(input, &config).unwrap();

        // Test data_bounds
        let ((x_min, x_max), (y_min, y_max)) = strip_data.data_bounds();
        assert!(x_min <= x_max);
        assert!(y_min <= y_max);

        // Test is_empty
        assert!(!strip_data.is_empty());
    }
}
