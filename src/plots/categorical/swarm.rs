//! Swarm plot implementations
//!
//! Provides non-overlapping categorical scatter plots using beeswarm algorithm.
//!
//! # Trait-Based API
//!
//! Swarm plots implement the core plot traits:
//! - [`PlotConfig`] for `SwarmConfig`
//! - [`PlotCompute`] for `Swarm` marker struct
//! - [`PlotData`] for `SwarmData`
//! - [`PlotRender`] for `SwarmData`

use crate::core::Result;
use crate::plots::traits::{
    AxisScaleSupport, ComputedSeries, ComputedStyle, LegendKey, PlotArea, PlotCompute, PlotConfig,
    PlotData, PlotPrimitive, PlotRender, draw_primitives,
};
use crate::render::skia::SkiaRenderer;
use crate::render::{Color, MarkerStyle, Theme};
use crate::stats::beeswarm::beeswarm_positions;

/// Configuration for swarm plot
#[derive(Debug, Clone)]
pub struct SwarmConfig {
    /// Marker size, in **points**
    pub size: f32,
    /// Marker color (None for auto)
    pub color: Option<Color>,
    /// Marker alpha
    pub alpha: f32,
    /// Orientation
    pub orientation: SwarmOrientation,
    /// Maximum width for spread
    pub width: f64,
    /// Dodge groups
    pub dodge: bool,
    /// Gap between dodged groups
    pub dodge_gap: f64,
}

/// Orientation for swarm plots
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SwarmOrientation {
    #[default]
    Vertical,
    Horizontal,
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            size: 5.0,
            color: None,
            alpha: 0.8,
            orientation: SwarmOrientation::Vertical,
            width: 0.8,
            dodge: false,
            dodge_gap: 0.05,
        }
    }
}

impl SwarmConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
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
        self.orientation = SwarmOrientation::Horizontal;
        self
    }

    /// Set maximum spread width
    pub fn width(mut self, width: f64) -> Self {
        self.width = width.clamp(0.1, 1.0);
        self
    }

    /// Enable dodging
    pub fn dodge(mut self, dodge: bool) -> Self {
        self.dodge = dodge;
        self
    }
}

// Implement PlotConfig marker trait
impl PlotConfig for SwarmConfig {}

/// Marker struct for Swarm plot type (used with PlotCompute trait)
pub struct Swarm;

/// A single point in a swarm plot
#[derive(Debug, Clone, Copy)]
pub struct SwarmPoint {
    /// Category index
    pub category: usize,
    /// Original value
    pub value: f64,
    /// Final x position
    pub x: f64,
    /// Final y position
    pub y: f64,
    /// Optional group index
    pub group: Option<usize>,
}

/// Compute the nominal position of every swarm point.
///
/// "Nominal" means the slot the point belongs to and its value, *without* the
/// sideways nudge that keeps markers from overlapping. That nudge is deliberately
/// not applied here: whether two markers collide depends on how big they are on
/// the page and how far apart the axis puts them, so it is a property of the
/// picture, not of the numbers. Computing it here meant comparing a marker size
/// in points against a gap in data units — the same swarm came out differently
/// for a column measured in metres and the same column measured in millimetres.
/// [`SwarmData::primitives`] lays the swarm out in pixels instead.
///
/// # Arguments
/// * `categories` - Category indices for each point
/// * `values` - Values for each point
/// * `groups` - Optional group indices
/// * `config` - Swarm plot configuration
///
/// # Returns
/// Vec of SwarmPoint
pub fn compute_swarm_points(
    categories: &[usize],
    values: &[f64],
    groups: Option<&[usize]>,
    config: &SwarmConfig,
) -> Vec<SwarmPoint> {
    let n = categories.len().min(values.len());
    let num_groups = groups.map_or(1, |g| g.iter().max().map_or(1, |&m| m + 1));

    (0..n)
        .map(|i| {
            let category = categories[i];
            let value = values[i];
            let group = groups.map(|g| g.get(i).copied().unwrap_or(0));

            // Dodging *is* a nominal position: it says which sub-slot a group
            // occupies, which the caller chose, rather than resolving a collision.
            let dodge_offset = match config.dodge && num_groups > 1 {
                true => {
                    let dodge_width = config.width / num_groups as f64;
                    (group.unwrap_or(0) as f64 - (num_groups - 1) as f64 / 2.0) * dodge_width
                }
                false => 0.0,
            };
            let slot = category as f64 + dodge_offset;

            let (x, y) = match config.orientation {
                SwarmOrientation::Vertical => (slot, value),
                SwarmOrientation::Horizontal => (value, slot),
            };

            SwarmPoint {
                category,
                value,
                x,
                y,
                group,
            }
        })
        .collect()
}

/// Compute data range for swarm plot
///
/// The category axis is the standard unit-wide slot span every categorical plot
/// type shares (see [`crate::plots::boxplot::category_slot_span`]). A column can
/// never spread past the edge of its own slot — `SwarmConfig::width` is clamped
/// to at most one whole slot — so nothing has to be measured off the points; the
/// old version folded `max |p.x|`, the *absolute* coordinate rather than the
/// offset from the slot centre, so the padding grew with the number of
/// categories and pushed the swarm off centre.
pub fn swarm_range(
    points: &[SwarmPoint],
    num_categories: usize,
    orientation: SwarmOrientation,
) -> ((f64, f64), (f64, f64)) {
    if points.is_empty() {
        return ((0.0, 1.0), (0.0, 1.0));
    }

    let val_min = points.iter().map(|p| p.value).fold(f64::INFINITY, f64::min);
    let val_max = points
        .iter()
        .map(|p| p.value)
        .fold(f64::NEG_INFINITY, f64::max);

    let (low, _) = crate::plots::boxplot::category_slot_span(0.0);
    let (_, high) = crate::plots::boxplot::category_slot_span(num_categories as f64 - 1.0);
    let cat_range = (low, high);

    match orientation {
        SwarmOrientation::Vertical => (cat_range, (val_min, val_max)),
        SwarmOrientation::Horizontal => ((val_min, val_max), cat_range),
    }
}

// ============================================================================
// Trait-Based API
// ============================================================================

/// Computed swarm plot data
#[derive(Debug, Clone)]
pub struct SwarmData {
    /// All computed points
    pub points: Vec<SwarmPoint>,
    /// Number of categories
    pub num_categories: usize,
    /// Name of each category, in slot order. Empty when the caller supplied
    /// bare slot indices, in which case the axis has nothing to print.
    pub category_names: Vec<String>,
    /// Configuration used to compute this data
    pub(crate) config: SwarmConfig,
}

/// Input for swarm plot computation
pub struct SwarmInput<'a> {
    /// Category indices
    pub categories: &'a [usize],
    /// Values
    pub values: &'a [f64],
    /// Optional group indices
    pub groups: Option<&'a [usize]>,
    /// Optional category names, in slot order, for the category axis.
    pub names: Option<&'a [String]>,
}

impl<'a> SwarmInput<'a> {
    /// Create new swarm input
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

impl PlotCompute for Swarm {
    type Input<'a> = SwarmInput<'a>;
    type Config = SwarmConfig;
    type Output = SwarmData;

    fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output> {
        let points = compute_swarm_points(input.categories, input.values, input.groups, config);

        if points.is_empty() {
            return Err(crate::core::PlottingError::EmptyDataSet);
        }

        // Calculate number of categories
        let num_categories = input.categories.iter().max().map_or(0, |&m| m + 1);

        Ok(SwarmData {
            points,
            num_categories,
            category_names: input.names.map(<[String]>::to_vec).unwrap_or_default(),
            config: config.clone(),
        })
    }
}

impl PlotData for SwarmData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        swarm_range(&self.points, self.num_categories, self.config.orientation)
    }

    fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

impl ComputedSeries for SwarmData {
    fn kind(&self) -> &'static str {
        "swarm"
    }

    fn point_count(&self) -> usize {
        self.points.len()
    }

    /// One slot per category, centred on its index — the same unit-wide slots
    /// bars and box plots use, so the names the caller passed get printed under
    /// the columns instead of raw numbers.
    fn category_slots(&self) -> Vec<(String, f64)> {
        match self.config.orientation {
            SwarmOrientation::Vertical => {
                crate::plots::boxplot::category_slots(&self.category_names, self.num_categories)
            }
            // The shared category axis is the x axis; a horizontal swarm puts
            // its categories on y, which that machinery cannot place yet.
            SwarmOrientation::Horizontal => Vec::new(),
        }
    }

    /// A cloud of markers, so the key is a marker — see [`StripData`](crate::plots::categorical::StripData).
    fn legend_key(&self) -> LegendKey {
        LegendKey::Marker
    }

    /// The category axis carries ordinal slots, so it has no quantitative
    /// spacing to take a logarithm of; the value axis is projected and scales
    /// freely. Same rule, same wording, as a bar chart's.
    fn axis_scale_support(&self) -> (AxisScaleSupport, AxisScaleSupport) {
        match self.config.orientation {
            SwarmOrientation::Vertical => (AxisScaleSupport::ORDINAL, AxisScaleSupport::Scaled),
            SwarmOrientation::Horizontal => (AxisScaleSupport::Scaled, AxisScaleSupport::ORDINAL),
        }
    }

    /// One marker per observation, nudged sideways in **pixels** so none of them
    /// overlap.
    ///
    /// The beeswarm runs here, on the projected positions, because that is the
    /// only place the three quantities it compares — marker diameter, the gap
    /// between two observations, and how far a column may spread — are in the
    /// same units. Doing it on the raw values meant a marker size in points was
    /// compared against a gap in data units, so the same column swarmed
    /// differently depending on whether it was measured in metres or millimetres.
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
        let vertical = config.orientation == SwarmOrientation::Vertical;
        let width_px = self.spread_width_px(area, vertical);

        let mut primitives = Vec::with_capacity(self.points.len());
        // Each column packs independently: two observations in different slots
        // can never collide, and packing them together would let one column's
        // crowding push another column's markers out of their slot.
        for column in self.columns() {
            // A point the axes cannot place has no position, so it is dropped
            // rather than drawn at a NaN pixel — and dropped *before* the
            // layout, so an absent point cannot leave a hole in the packing.
            let projected: Vec<(f32, f32)> = column
                .iter()
                .filter_map(|point| area.try_data_to_screen(point.x, point.y))
                .collect();
            let along: Vec<f64> = projected
                .iter()
                .map(|&(x, y)| f64::from(if vertical { y } else { x }))
                .collect();
            let nudges = beeswarm_positions(&along, f64::from(size_px), width_px);

            primitives.extend(projected.iter().zip(nudges).map(|(&(x, y), nudge)| {
                let nudge = nudge as f32;
                PlotPrimitive::Marker {
                    at: if vertical {
                        (x + nudge, y)
                    } else {
                        (x, y + nudge)
                    },
                    size_px,
                    style: MarkerStyle::Circle,
                    color,
                }
            }));
        }
        primitives
    }
}

impl SwarmData {
    /// The observations of one column at a time, in slot order.
    ///
    /// A "column" is one nominal position on the category axis — one category,
    /// or one dodged group inside a category. Grouping by the nominal
    /// coordinate rather than by `category` is what makes dodged groups pack
    /// separately without a second rule.
    fn columns(&self) -> Vec<Vec<&SwarmPoint>> {
        let slot_of = |point: &SwarmPoint| match self.config.orientation {
            SwarmOrientation::Vertical => point.x,
            SwarmOrientation::Horizontal => point.y,
        };
        let mut slots: Vec<f64> = Vec::new();
        let mut columns: Vec<Vec<&SwarmPoint>> = Vec::new();
        for point in &self.points {
            let slot = slot_of(point);
            let existing = slots.iter().position(|taken| *taken == slot);
            match existing {
                Some(index) => columns[index].push(point),
                None => {
                    slots.push(slot);
                    columns.push(vec![point]);
                }
            }
        }
        columns
    }

    /// How wide one column may spread, in device pixels.
    ///
    /// `config.width` is a fraction of one category slot — the same meaning it
    /// has on every other categorical plot type — so the width in pixels is
    /// whatever the axis currently makes one slot, scaled by that fraction.
    fn spread_width_px(&self, area: &PlotArea, vertical: bool) -> f64 {
        let num_groups = self
            .points
            .iter()
            .filter_map(|point| point.group)
            .max()
            .map_or(1, |group| group + 1);
        let (origin, one_slot) = match vertical {
            true => (
                area.try_data_to_screen(0.0, 0.0),
                area.try_data_to_screen(1.0, 0.0),
            ),
            false => (
                area.try_data_to_screen(0.0, 0.0),
                area.try_data_to_screen(0.0, 1.0),
            ),
        };
        let slot_px = match (origin, one_slot) {
            (Some(origin), Some(one_slot)) => match vertical {
                true => f64::from(one_slot.0 - origin.0).abs(),
                false => f64::from(one_slot.1 - origin.1).abs(),
            },
            _ => 0.0,
        };
        slot_px * self.config.width / num_groups.max(1) as f64
    }
}

impl PlotRender for SwarmData {
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
        _line_width: Option<f32>,
    ) -> Result<()> {
        let style = ComputedStyle {
            scale: renderer.render_scale(),
            color,
            alpha,
            line_width: None,
        };
        draw_primitives(renderer, &self.primitives(area, &style))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The x of every marker `data` draws in `area`, left to right.
    fn marker_xs(data: &SwarmData, area: &PlotArea) -> Vec<f32> {
        let renderer = SkiaRenderer::new(200, 200, Theme::default()).unwrap();
        let style = ComputedStyle::opaque(renderer.render_scale(), Color::from_rgb(0, 0, 0));
        let mut xs: Vec<f32> = data
            .primitives(area, &style)
            .into_iter()
            .filter_map(|primitive| match primitive {
                PlotPrimitive::Marker { at, .. } => Some(at.0),
                _ => None,
            })
            .collect();
        xs.sort_by(f32::total_cmp);
        xs
    }

    fn swarm_of(categories: &[usize], values: &[f64]) -> SwarmData {
        Swarm::compute(SwarmInput::new(categories, values), &SwarmConfig::default()).unwrap()
    }

    fn full_area(data: &SwarmData) -> PlotArea {
        let ((x_min, x_max), (y_min, y_max)) = data.data_bounds();
        PlotArea::new(0.0, 0.0, 400.0, 300.0, x_min, x_max, y_min, y_max)
    }

    #[test]
    fn test_swarm_basic() {
        let categories = vec![0, 0, 0, 1, 1, 1];
        let values = vec![1.0, 1.0, 1.0, 2.0, 2.0, 2.0];
        let config = SwarmConfig::default();
        let points = compute_swarm_points(&categories, &values, None, &config);

        assert_eq!(points.len(), 6);
        // The nominal position is the slot centre: the sideways nudge is a
        // property of the drawn picture, not of the numbers, so it is not here.
        for point in &points {
            assert_eq!(
                point.x, point.category as f64,
                "a swarm point's nominal position is its category slot"
            );
        }
    }

    #[test]
    fn coincident_observations_are_spread_apart_when_drawn() {
        // Three identical values in one slot cannot be drawn on top of each
        // other, so the rendered markers must occupy three distinct x.
        let data = swarm_of(&[0, 0, 0], &[1.0, 1.0, 1.0]);
        let xs = marker_xs(&data, &full_area(&data));

        assert_eq!(xs.len(), 3);
        assert!(
            xs[1] - xs[0] > 1.0 && xs[2] - xs[1] > 1.0,
            "coincident observations were not spread apart: {xs:?}"
        );
    }

    #[test]
    fn observations_the_axis_already_separates_are_not_nudged() {
        // Values far enough apart on the value axis do not overlap, so there is
        // nothing to resolve and every marker stays on the slot centre. This is
        // what the old compute-time layout got wrong: it compared a marker size
        // in *points* against a gap in *data units*, so it fanned these out.
        let data = swarm_of(&[0, 0, 0], &[0.0, 50.0, 100.0]);
        let xs = marker_xs(&data, &full_area(&data));

        assert_eq!(xs.len(), 3);
        assert!(
            xs.iter().all(|x| (x - xs[0]).abs() < 1.0e-3),
            "well-separated observations were nudged sideways: {xs:?}"
        );
    }

    #[test]
    fn the_swarm_does_not_depend_on_the_unit_the_values_are_measured_in() {
        // Scaling every value by 1000 is a change of unit, not of data. The
        // picture must be identical; it used to change completely, because the
        // collision threshold was a raw number compared against the values.
        let metres = swarm_of(&[0, 0, 0, 0, 0], &[0.0, 0.1, 0.2, 0.3, 0.4]);
        let millimetres = swarm_of(&[0, 0, 0, 0, 0], &[0.0, 100.0, 200.0, 300.0, 400.0]);

        assert_eq!(
            marker_xs(&metres, &full_area(&metres)),
            marker_xs(&millimetres, &full_area(&millimetres)),
            "the same data in different units produced different swarms"
        );
    }

    #[test]
    fn no_marker_leaves_its_own_category_slot() {
        // A crowded column may spread, but never so far that it collides with
        // the neighbouring category — `width` is a fraction of one slot.
        let categories: Vec<usize> = (0..60).map(|i| i % 2).collect();
        let values: Vec<f64> = (0..60).map(|_| 1.0).collect();
        let data = swarm_of(&categories, &values);
        let area = full_area(&data);
        let xs = marker_xs(&data, &area);

        let (slot_low, _) = crate::plots::boxplot::category_slot_span(0.0);
        let (_, slot_high) = crate::plots::boxplot::category_slot_span(0.0);
        let left_edge = area.data_to_screen(slot_low, 1.0).0;
        let right_edge = area.data_to_screen(slot_high, 1.0).0;
        let slot_zero = xs.iter().filter(|&&x| x <= right_edge).count();

        assert_eq!(slot_zero, 30, "markers escaped their category slot: {xs:?}");
        assert!(
            xs.iter().all(|&x| x >= left_edge),
            "markers escaped the left edge of the axis: {xs:?}"
        );
    }

    #[test]
    fn test_swarm_horizontal() {
        let categories = vec![0, 1];
        let values = vec![1.0, 2.0];
        let config = SwarmConfig::default().horizontal();
        let points = compute_swarm_points(&categories, &values, None, &config);

        // For horizontal, y should be around category, x should be value
        for point in &points {
            assert!((point.x - point.value).abs() < 1e-10);
        }
    }

    #[test]
    fn test_swarm_with_groups() {
        let categories = vec![0, 0, 0, 0];
        let values = vec![1.0, 1.0, 2.0, 2.0];
        let groups = vec![0, 1, 0, 1];
        let config = SwarmConfig::default().dodge(true);
        let points = compute_swarm_points(&categories, &values, Some(&groups), &config);

        assert_eq!(points.len(), 4);
        for point in &points {
            assert!(point.group.is_some());
        }
    }

    #[test]
    fn test_swarm_empty() {
        let categories: Vec<usize> = vec![];
        let values: Vec<f64> = vec![];
        let config = SwarmConfig::default();
        let points = compute_swarm_points(&categories, &values, None, &config);

        assert!(points.is_empty());
    }

    #[test]
    fn test_swarm_config_implements_plot_config() {
        fn assert_plot_config<T: PlotConfig>() {}
        assert_plot_config::<SwarmConfig>();
    }

    #[test]
    fn test_swarm_plot_compute_trait() {
        use crate::plots::traits::PlotCompute;

        let categories = vec![0, 0, 1, 1, 2, 2];
        let values = vec![1.0, 1.5, 2.0, 2.5, 3.0, 3.5];
        let config = SwarmConfig::default();
        let input = SwarmInput::new(&categories, &values);
        let result = Swarm::compute(input, &config);

        assert!(result.is_ok());
        let swarm_data = result.unwrap();
        assert_eq!(swarm_data.points.len(), 6);
        assert_eq!(swarm_data.num_categories, 3);
    }

    #[test]
    fn test_swarm_plot_compute_with_groups() {
        use crate::plots::traits::PlotCompute;

        let categories = vec![0, 0, 1, 1];
        let values = vec![1.0, 2.0, 1.0, 2.0];
        let groups = vec![0, 1, 0, 1];
        let config = SwarmConfig::default().dodge(true);
        let input = SwarmInput::new(&categories, &values).with_groups(&groups);
        let result = Swarm::compute(input, &config);

        assert!(result.is_ok());
        let swarm_data = result.unwrap();
        assert_eq!(swarm_data.points.len(), 4);
    }

    #[test]
    fn test_swarm_plot_compute_empty() {
        use crate::plots::traits::PlotCompute;

        let categories: Vec<usize> = vec![];
        let values: Vec<f64> = vec![];
        let config = SwarmConfig::default();
        let input = SwarmInput::new(&categories, &values);
        let result = Swarm::compute(input, &config);

        assert!(result.is_err());
    }

    fn swarm_ink(dpi_scale: f32) -> usize {
        let categories = vec![0, 1, 2];
        let values = vec![1.0, 5.0, 3.0];
        let data = Swarm::compute(
            SwarmInput::new(&categories, &values),
            &SwarmConfig::default(),
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
    fn test_swarm_markers_keep_their_physical_size_at_higher_dpi() {
        // `size` is in points like every other marker size in the crate, so
        // doubling the render scale must double the dot diameter. Passing the
        // raw number through as pixels left the dots the same size at 300 DPI.
        let single = swarm_ink(1.0);
        let double = swarm_ink(2.0);

        assert!(
            double > single * 2,
            "swarm markers did not grow with DPI ({double} vs {single} inked pixels)"
        );
    }

    #[test]
    fn test_swarm_plot_data_trait() {
        use crate::plots::traits::{PlotCompute, PlotData};

        let categories = vec![0, 1, 2];
        let values = vec![1.0, 5.0, 3.0];
        let config = SwarmConfig::default();
        let input = SwarmInput::new(&categories, &values);
        let swarm_data = Swarm::compute(input, &config).unwrap();

        // Test data_bounds
        let ((x_min, x_max), (y_min, y_max)) = swarm_data.data_bounds();
        assert!(x_min <= x_max);
        assert!(y_min <= y_max);

        // Test is_empty
        assert!(!swarm_data.is_empty());
    }
}
