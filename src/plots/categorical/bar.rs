//! Stacked and grouped bar chart implementations
//!
//! Provides stacked bar, grouped bar, and horizontal bar functionality.
//!
//! # Trait-Based API
//!
//! Bar plots implement the core plot traits:
//! - [`PlotConfig`] for `StackedBarConfig` and `GroupedBarConfig`
//! - [`PlotCompute`] for `StackedBar` and `GroupedBar` marker structs
//! - [`PlotData`] for `StackedBarData` and `GroupedBarData`
//! - [`PlotRender`] for `StackedBarData` and `GroupedBarData`
//!
//! # How the builder draws these
//!
//! [`Plot::grouped_bar`](crate::core::Plot::grouped_bar) and
//! [`Plot::stacked_bar`](crate::core::Plot::stacked_bar) do **not** add one
//! series holding the whole chart. They add **one series per named value
//! column** — a [`BarSeriesData`] each — because a palette slot, a legend entry
//! and a `.color()` are per-series things and a grouped chart needs one of each
//! per column. `grouped_bar_series` and `stacked_bar_series` are what perform
//! that split; `GroupedBarData`/`StackedBarData` remain the whole-chart shape
//! for callers driving [`PlotCompute`] and [`PlotRender`] directly.

use crate::core::Result;
use crate::plots::boxplot::{category_slot_span, category_slots};
use crate::plots::traits::{
    AxisScaleSupport, ComputedSeries, ComputedStyle, LegendKey, PlotArea, PlotCompute, PlotConfig,
    PlotData, PlotPrimitive, PlotRender, draw_primitives,
};
use crate::render::skia::SkiaRenderer;
use crate::render::{Color, LineStyle, Theme};

/// Bar orientation, shared with [`crate::plots::basic`].
///
/// Re-exported rather than re-declared: an identical enum lived here too, so
/// `.bar(..)` and `.grouped_bar(..)` spelled the same knob with two
/// incompatible types and nothing could convert between them.
pub use crate::plots::basic::BarOrientation;

/// Configuration for stacked bar chart
#[derive(Debug, Clone)]
pub struct StackedBarConfig {
    /// Bar width as fraction of category spacing (0.0-1.0)
    pub width: f64,
    /// Colors for each series (None for auto-colors)
    pub colors: Option<Vec<Color>>,
    /// Alpha for fill
    pub alpha: f32,
    /// Labels for each series
    pub labels: Vec<String>,
    /// Edge color for bars
    pub edge_color: Option<Color>,
    /// Edge width
    pub edge_width: f32,
    /// Orientation
    pub orientation: BarOrientation,
}

/// Configuration for grouped bar chart
#[derive(Debug, Clone)]
pub struct GroupedBarConfig {
    /// Width of each bar group as fraction of category spacing
    pub group_width: f64,
    /// Gap between bars within a group (fraction of bar width)
    pub bar_gap: f64,
    /// Colors for each series (None for auto-colors)
    pub colors: Option<Vec<Color>>,
    /// Alpha for fill
    pub alpha: f32,
    /// Labels for each series
    pub labels: Vec<String>,
    /// Edge color for bars
    pub edge_color: Option<Color>,
    /// Edge width
    pub edge_width: f32,
    /// Orientation
    pub orientation: BarOrientation,
}

impl Default for StackedBarConfig {
    fn default() -> Self {
        Self {
            width: 0.8,
            colors: None,
            alpha: 1.0,
            labels: vec![],
            edge_color: None,
            edge_width: 0.0,
            orientation: BarOrientation::Vertical,
        }
    }
}

impl StackedBarConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set bar width
    pub fn width(mut self, width: f64) -> Self {
        self.width = width.clamp(0.1, 1.0);
        self
    }

    /// Set colors
    pub fn colors(mut self, colors: Vec<Color>) -> Self {
        self.colors = Some(colors);
        self
    }

    /// Set alpha
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Set labels
    pub fn labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    /// Set edge color
    pub fn edge_color(mut self, color: Color) -> Self {
        self.edge_color = Some(color);
        self
    }

    /// Set horizontal orientation
    pub fn horizontal(mut self) -> Self {
        self.orientation = BarOrientation::Horizontal;
        self
    }

    /// Set vertical orientation
    pub fn vertical(mut self) -> Self {
        self.orientation = BarOrientation::Vertical;
        self
    }
}

impl Default for GroupedBarConfig {
    fn default() -> Self {
        Self {
            group_width: 0.8,
            bar_gap: 0.05,
            colors: None,
            alpha: 1.0,
            labels: vec![],
            edge_color: None,
            edge_width: 0.0,
            orientation: BarOrientation::Vertical,
        }
    }
}

impl GroupedBarConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set group width
    pub fn group_width(mut self, width: f64) -> Self {
        self.group_width = width.clamp(0.1, 1.0);
        self
    }

    /// Set gap between bars in group
    pub fn bar_gap(mut self, gap: f64) -> Self {
        self.bar_gap = gap.clamp(0.0, 0.5);
        self
    }

    /// Set colors
    pub fn colors(mut self, colors: Vec<Color>) -> Self {
        self.colors = Some(colors);
        self
    }

    /// Set alpha
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Set labels
    pub fn labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    /// Set edge color
    pub fn edge_color(mut self, color: Color) -> Self {
        self.edge_color = Some(color);
        self
    }

    /// Set horizontal orientation
    pub fn horizontal(mut self) -> Self {
        self.orientation = BarOrientation::Horizontal;
        self
    }
}

// Implement PlotConfig marker trait
impl PlotConfig for StackedBarConfig {}
impl PlotConfig for GroupedBarConfig {}

/// Marker struct for StackedBar plot type
pub struct StackedBar;

/// Marker struct for GroupedBar plot type
pub struct GroupedBar;

/// A single bar rectangle
#[derive(Debug, Clone, Copy)]
pub struct BarRect {
    /// X position (left edge for vertical, bottom edge for horizontal)
    pub x: f64,
    /// Y position (bottom edge for vertical, left edge for horizontal)
    pub y: f64,
    /// Width (horizontal extent for vertical bars)
    pub width: f64,
    /// Height (vertical extent for vertical bars)
    pub height: f64,
    /// Series index this bar belongs to
    pub series: usize,
    /// Category index this bar belongs to
    pub category: usize,
}

/// Compute stacked bar rectangles
///
/// Positive contributions stack upwards from the baseline and negative ones
/// stack downwards, so a series that dips below zero does not eat into the
/// stack above it. That is the split [`stacked_bar_range`] already used to size
/// the value axis; running one cumulative total for both signs made the
/// geometry and the axis range disagree about where the stack ended.
///
/// # Arguments
/// * `values` - 2D array of values \[series\]\[category\]
/// * `categories` - Number of categories
/// * `config` - Stacked bar configuration
///
/// # Returns
/// Vec of BarRect for each bar
pub fn compute_stacked_bars(
    values: &[Vec<f64>],
    categories: usize,
    config: &StackedBarConfig,
) -> Vec<BarRect> {
    if values.is_empty() || categories == 0 {
        return vec![];
    }

    let bar_width = config.width;
    let half_width = bar_width / 2.0;

    let mut bars = Vec::new();
    let mut positive = vec![0.0; categories];
    let mut negative = vec![0.0; categories];

    for (series_idx, series_values) in values.iter().enumerate() {
        for cat_idx in 0..categories.min(series_values.len()) {
            let value = series_values[cat_idx];
            // A non-finite sample has no length, and adding it to a running
            // total would poison every bar stacked above it.
            if !value.is_finite() {
                continue;
            }
            let running = if value < 0.0 {
                &mut negative[cat_idx]
            } else {
                &mut positive[cat_idx]
            };
            let base = *running;
            *running += value;

            match config.orientation {
                BarOrientation::Vertical => {
                    bars.push(BarRect {
                        x: cat_idx as f64 - half_width,
                        y: base,
                        width: bar_width,
                        height: value,
                        series: series_idx,
                        category: cat_idx,
                    });
                }
                BarOrientation::Horizontal => {
                    bars.push(BarRect {
                        x: base,
                        y: cat_idx as f64 - half_width,
                        width: value,
                        height: bar_width,
                        series: series_idx,
                        category: cat_idx,
                    });
                }
            }
        }
    }

    bars
}

/// Compute grouped bar rectangles
///
/// # Arguments
/// * `values` - 2D array of values \[series\]\[category\]
/// * `categories` - Number of categories
/// * `config` - Grouped bar configuration
///
/// # Returns
/// Vec of BarRect for each bar
pub fn compute_grouped_bars(
    values: &[Vec<f64>],
    categories: usize,
    config: &GroupedBarConfig,
) -> Vec<BarRect> {
    if values.is_empty() || categories == 0 {
        return vec![];
    }

    let num_series = values.len();
    let group_width = config.group_width;
    let bar_gap = config.bar_gap;

    // Calculate individual bar width
    let total_gap = bar_gap * (num_series - 1) as f64;
    let bar_width = (group_width - total_gap) / num_series as f64;
    let bar_spacing = bar_width + bar_gap;

    let mut bars = Vec::new();

    for (series_idx, series_values) in values.iter().enumerate() {
        for (cat_idx, &value) in series_values.iter().enumerate().take(categories) {
            // Calculate bar position within group
            let group_start = cat_idx as f64 - group_width / 2.0;
            let bar_offset = series_idx as f64 * bar_spacing;

            match config.orientation {
                BarOrientation::Vertical => {
                    bars.push(BarRect {
                        x: group_start + bar_offset,
                        y: 0.0,
                        width: bar_width,
                        height: value,
                        series: series_idx,
                        category: cat_idx,
                    });
                }
                BarOrientation::Horizontal => {
                    bars.push(BarRect {
                        x: 0.0,
                        y: group_start + bar_offset,
                        width: value,
                        height: bar_width,
                        series: series_idx,
                        category: cat_idx,
                    });
                }
            }
        }
    }

    bars
}

/// Compute data range for stacked bars
///
/// # Returns
/// (min, max) for the value axis
pub fn stacked_bar_range(values: &[Vec<f64>]) -> (f64, f64) {
    if values.is_empty() {
        return (0.0, 1.0);
    }

    let num_categories = values.iter().map(|v| v.len()).max().unwrap_or(0);
    let mut max_sum: f64 = 0.0;
    let mut min_sum: f64 = 0.0;

    for cat_idx in 0..num_categories {
        let mut positive_sum = 0.0;
        let mut negative_sum = 0.0;

        for series in values {
            if cat_idx < series.len() {
                let value = series[cat_idx];
                if value >= 0.0 {
                    positive_sum += value;
                } else {
                    negative_sum += value;
                }
            }
        }

        max_sum = max_sum.max(positive_sum);
        min_sum = min_sum.min(negative_sum);
    }

    (min_sum, max_sum)
}

/// Compute data range for grouped bars
///
/// # Returns
/// (min, max) for the value axis
pub fn grouped_bar_range(values: &[Vec<f64>]) -> (f64, f64) {
    if values.is_empty() {
        return (0.0, 1.0);
    }

    let mut min_val: f64 = 0.0;
    let mut max_val: f64 = 0.0;

    for series in values {
        for &value in series {
            min_val = min_val.min(value);
            max_val = max_val.max(value);
        }
    }

    (min_val, max_val)
}

// ============================================================================
// Trait-Based API
// ============================================================================

/// Computed stacked bar data
#[derive(Debug, Clone)]
pub struct StackedBarData {
    /// All bar rectangles
    pub bars: Vec<BarRect>,
    /// Number of categories
    pub num_categories: usize,
    /// Number of series
    pub num_series: usize,
    /// Value range (min, max)
    pub value_range: (f64, f64),
    /// Configuration used
    pub(crate) config: StackedBarConfig,
}

/// Computed grouped bar data
#[derive(Debug, Clone)]
pub struct GroupedBarData {
    /// All bar rectangles
    pub bars: Vec<BarRect>,
    /// Number of categories
    pub num_categories: usize,
    /// Number of series
    pub num_series: usize,
    /// Value range (min, max)
    pub value_range: (f64, f64),
    /// Configuration used
    pub(crate) config: GroupedBarConfig,
}

/// Input for bar chart computation
pub struct BarInput<'a> {
    /// 2D values: \[series\]\[category\]
    pub values: &'a [Vec<f64>],
    /// Number of categories
    pub num_categories: usize,
}

impl<'a> BarInput<'a> {
    /// Create new bar input
    pub fn new(values: &'a [Vec<f64>], num_categories: usize) -> Self {
        Self {
            values,
            num_categories,
        }
    }
}

impl PlotCompute for StackedBar {
    type Input<'a> = BarInput<'a>;
    type Config = StackedBarConfig;
    type Output = StackedBarData;

    fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output> {
        if input.values.is_empty() || input.num_categories == 0 {
            return Err(crate::core::PlottingError::EmptyDataSet);
        }

        let bars = compute_stacked_bars(input.values, input.num_categories, config);
        let value_range = stacked_bar_range(input.values);

        Ok(StackedBarData {
            bars,
            num_categories: input.num_categories,
            num_series: input.values.len(),
            value_range,
            config: config.clone(),
        })
    }
}

impl PlotCompute for GroupedBar {
    type Input<'a> = BarInput<'a>;
    type Config = GroupedBarConfig;
    type Output = GroupedBarData;

    fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output> {
        if input.values.is_empty() || input.num_categories == 0 {
            return Err(crate::core::PlottingError::EmptyDataSet);
        }

        let bars = compute_grouped_bars(input.values, input.num_categories, config);
        let value_range = grouped_bar_range(input.values);

        Ok(GroupedBarData {
            bars,
            num_categories: input.num_categories,
            num_series: input.values.len(),
            value_range,
            config: config.clone(),
        })
    }
}

impl PlotData for StackedBarData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        let cat_range = (-0.5, self.num_categories as f64 - 0.5);
        match self.config.orientation {
            BarOrientation::Vertical => (cat_range, self.value_range),
            BarOrientation::Horizontal => (self.value_range, cat_range),
        }
    }

    fn is_empty(&self) -> bool {
        self.bars.is_empty()
    }
}

impl PlotData for GroupedBarData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        let cat_range = (-0.5, self.num_categories as f64 - 0.5);
        match self.config.orientation {
            BarOrientation::Vertical => (cat_range, self.value_range),
            BarOrientation::Horizontal => (self.value_range, cat_range),
        }
    }

    fn is_empty(&self) -> bool {
        self.bars.is_empty()
    }
}

impl PlotRender for StackedBarData {
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        theme: &Theme,
        _color: Color,
    ) -> Result<()> {
        if self.bars.is_empty() {
            return Ok(());
        }

        let config = &self.config;

        for bar in &self.bars {
            // Get color for this series
            let bar_color = config
                .colors
                .as_ref()
                .and_then(|c| c.get(bar.series).copied())
                .unwrap_or_else(|| theme.get_color(bar.series))
                .with_alpha(config.alpha);

            // Convert to screen coordinates
            let (x1, y1) = area.data_to_screen(bar.x, bar.y + bar.height);
            let (x2, y2) = area.data_to_screen(bar.x + bar.width, bar.y);

            let x = x1.min(x2);
            let y = y1.min(y2);
            let w = (x2 - x1).abs();
            let h = (y2 - y1).abs();

            renderer.draw_rectangle(x, y, w, h, bar_color, true)?;

            // Draw edge if specified
            if config.edge_width > 0.0
                && let Some(edge_color) = config.edge_color
            {
                let outline = vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h), (x, y)];
                renderer.draw_polyline(
                    &outline,
                    edge_color,
                    config.edge_width,
                    LineStyle::Solid,
                )?;
            }
        }

        Ok(())
    }
}

impl PlotRender for GroupedBarData {
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        theme: &Theme,
        _color: Color,
    ) -> Result<()> {
        if self.bars.is_empty() {
            return Ok(());
        }

        let config = &self.config;

        for bar in &self.bars {
            // Get color for this series
            let bar_color = config
                .colors
                .as_ref()
                .and_then(|c| c.get(bar.series).copied())
                .unwrap_or_else(|| theme.get_color(bar.series))
                .with_alpha(config.alpha);

            // Convert to screen coordinates
            let (x1, y1) = area.data_to_screen(bar.x, bar.y + bar.height);
            let (x2, y2) = area.data_to_screen(bar.x + bar.width, bar.y);

            let x = x1.min(x2);
            let y = y1.min(y2);
            let w = (x2 - x1).abs();
            let h = (y2 - y1).abs();

            renderer.draw_rectangle(x, y, w, h, bar_color, true)?;

            // Draw edge if specified
            if config.edge_width > 0.0
                && let Some(edge_color) = config.edge_color
            {
                let outline = vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h), (x, y)];
                renderer.draw_polyline(
                    &outline,
                    edge_color,
                    config.edge_width,
                    LineStyle::Solid,
                )?;
            }
        }

        Ok(())
    }
}

// ============================================================================
// One sub-series at a time: what the `Plot` builder actually adds
// ============================================================================

/// Which multi-series layout a [`BarSeriesData`] came out of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarLayout {
    /// Bars sit side by side, subdividing one category slot.
    Grouped,
    /// Bars sit on top of one another, centred in the slot.
    Stacked,
}

impl BarLayout {
    /// The builder method that draws this layout — the name a reader typed, so
    /// it is the name diagnostics quote.
    pub fn kind(self) -> &'static str {
        match self {
            Self::Grouped => "grouped_bar",
            Self::Stacked => "stacked_bar",
        }
    }
}

/// The bars one *named* value column of a grouped or stacked chart contributes.
///
/// A grouped or stacked chart is N of these, added to the plot as N ordinary
/// series. That is the whole design: the palette slot, the legend entry, the
/// `.color()` override and the `.label()` are all per-series things, and a
/// chart whose columns cannot be told apart in the legend is not a grouped
/// chart. Nothing about the render path is multi-series — it draws N single
/// series, exactly as it does for N `.line()` calls.
///
/// Positions are already resolved into data coordinates by
/// [`compute_grouped_bars`] / [`compute_stacked_bars`], so a group occupies the
/// same one-unit-wide category slot a single bar or a box plot does — see
/// [`category_slot_span`].
#[derive(Debug, Clone)]
pub struct BarSeriesData {
    /// This column's bars, in data coordinates.
    pub bars: Vec<BarRect>,
    /// Every category name of the chart, in slot order. Each sub-series carries
    /// the full list so the axis is labelled the same whichever of them the
    /// renderer asks; `CategoryAxis::harvest` deduplicates by slot position.
    pub categories: Vec<String>,
    /// Which layout produced `bars`.
    pub layout: BarLayout,
    /// Which axis the categories run along.
    pub orientation: BarOrientation,
    /// Fill opacity from the chart config; composes with the series alpha.
    pub(crate) alpha: f32,
    /// Explicit edge colour, or `None` for no edge.
    pub(crate) edge_color: Option<Color>,
    /// Edge width in **points**, so the outline is DPI-invariant.
    pub(crate) edge_width: f32,
}

/// Split a whole chart's rectangles into one [`BarSeriesData`] per value column.
///
/// Grouped and stacked charts differ only in where `compute_*_bars` put the
/// rectangles, so the split is written once and both go through it.
fn split_bars(
    bars: Vec<BarRect>,
    categories: &[String],
    names: &[String],
    layout: BarLayout,
    orientation: BarOrientation,
    alpha: f32,
    edge_color: Option<Color>,
    edge_width: f32,
) -> Vec<(String, BarSeriesData)> {
    let mut split: Vec<(String, BarSeriesData)> = names
        .iter()
        .map(|name| {
            (
                name.clone(),
                BarSeriesData {
                    bars: Vec::new(),
                    categories: categories.to_vec(),
                    layout,
                    orientation,
                    alpha,
                    edge_color,
                    edge_width,
                },
            )
        })
        .collect();

    for bar in bars {
        if let Some((_, data)) = split.get_mut(bar.series) {
            data.bars.push(bar);
        }
    }

    split
}

/// One [`BarSeriesData`] per named value column of a grouped bar chart.
///
/// `names` and `values` are parallel: `values[i]` is the column named
/// `names[i]`, holding one value per category.
pub fn grouped_bar_series(
    categories: &[String],
    names: &[String],
    values: &[Vec<f64>],
    config: &GroupedBarConfig,
) -> Vec<(String, BarSeriesData)> {
    split_bars(
        compute_grouped_bars(values, categories.len(), config),
        categories,
        names,
        BarLayout::Grouped,
        config.orientation,
        config.alpha,
        config.edge_color,
        config.edge_width,
    )
}

/// One [`BarSeriesData`] per named value column of a stacked bar chart.
///
/// Each column's rectangles start at the running total of the columns before
/// it, so the union of the sub-series' [`PlotData::data_bounds`] is the
/// cumulative extent of the stack — no separate whole-chart bounds arm exists,
/// or could disagree with the geometry.
pub fn stacked_bar_series(
    categories: &[String],
    names: &[String],
    values: &[Vec<f64>],
    config: &StackedBarConfig,
) -> Vec<(String, BarSeriesData)> {
    split_bars(
        compute_stacked_bars(values, categories.len(), config),
        categories,
        names,
        BarLayout::Stacked,
        config.orientation,
        config.alpha,
        config.edge_color,
        config.edge_width,
    )
}

impl BarSeriesData {
    /// The `(low, high)` span this column covers on the value axis, with the
    /// zero baseline folded in.
    ///
    /// Bars are drawn from their base to their tip, so both ends are data — and
    /// for a stacked column the base is the running total underneath it, which
    /// is what makes the union over the columns the cumulative extent.
    fn value_span(&self) -> (f64, f64) {
        let mut low = 0.0f64;
        let mut high = 0.0f64;
        for bar in &self.bars {
            let (start, end) = match self.orientation {
                BarOrientation::Vertical => (bar.y, bar.y + bar.height),
                BarOrientation::Horizontal => (bar.x, bar.x + bar.width),
            };
            if !start.is_finite() || !end.is_finite() {
                continue;
            }
            low = low.min(start.min(end));
            high = high.max(start.max(end));
        }
        (low, high)
    }

    /// The `(low, high)` span the category slots cover, in data units.
    fn category_span(&self) -> (f64, f64) {
        let last = self.categories.len().saturating_sub(1) as f64;
        (category_slot_span(0.0).0, category_slot_span(last).1)
    }
}

impl PlotData for BarSeriesData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        let categories = self.category_span();
        let values = self.value_span();
        match self.orientation {
            BarOrientation::Vertical => (categories, values),
            BarOrientation::Horizontal => (values, categories),
        }
    }

    fn is_empty(&self) -> bool {
        self.bars.is_empty()
    }
}

impl ComputedSeries for BarSeriesData {
    fn kind(&self) -> &'static str {
        self.layout.kind()
    }

    fn point_count(&self) -> usize {
        self.bars.len()
    }

    /// One slot per category, centred on its index — the same unit-wide slots
    /// [`Plot::bar`](crate::core::Plot::bar), box plots and violins take, so a
    /// group and a lone bar cannot be positioned by one rule and labelled by
    /// another.
    fn category_slots(&self) -> Vec<(String, f64)> {
        category_slots(&self.categories, self.categories.len())
    }

    fn category_orientation(&self) -> crate::core::Orientation {
        match self.orientation {
            BarOrientation::Vertical => crate::core::Orientation::Vertical,
            BarOrientation::Horizontal => crate::core::Orientation::Horizontal,
        }
    }

    /// A filled patch, like every other bar key.
    fn legend_key(&self) -> LegendKey {
        LegendKey::Patch
    }

    /// Bars are drawn from the zero baseline and must keep sitting exactly on
    /// it, the same thing `SeriesType::Bar` pins.
    fn pins_zero_baseline(&self) -> bool {
        true
    }

    /// The category axis carries ordinal slots, so it has no quantitative
    /// spacing to take a logarithm of; the value axis is projected and scales
    /// freely. Same answer, same wording, as `SeriesType::Bar`.
    fn axis_scale_support(&self) -> (AxisScaleSupport, AxisScaleSupport) {
        match self.orientation {
            BarOrientation::Vertical => (AxisScaleSupport::ORDINAL, AxisScaleSupport::Scaled),
            BarOrientation::Horizontal => (AxisScaleSupport::Scaled, AxisScaleSupport::ORDINAL),
        }
    }

    /// One filled rectangle per bar.
    fn primitives(&self, area: &PlotArea, style: &ComputedStyle) -> Vec<PlotPrimitive> {
        let fill = style.tinted(style.color.with_alpha(self.alpha));
        // The edge is authored in points like every other stroke in the crate,
        // and an edge colour is explicit or absent — the same rule hexbin
        // follows for a filled patch reached through `PlotPrimitive`.
        let edge_width_px = style.scale.points_to_pixels(self.edge_width.max(0.0));
        let edge = self
            .edge_color
            .filter(|_| self.edge_width > 0.0)
            .map(|color| (style.tinted(color), edge_width_px));

        self.bars
            .iter()
            .filter_map(|bar| {
                // Both corners are *edges* of a filled patch, not samples: a
                // bar whose base is zero on a log axis is not absent, it runs
                // off the bottom of the axis. `edge_data_to_screen` pins such
                // an edge to the axis floor, which is the rule
                // `PlotArea::fill_baseline_y` gives an ordinary bar.
                let (x0, y0) = area.edge_data_to_screen(bar.x, bar.y);
                let (x1, y1) = area.edge_data_to_screen(bar.x + bar.width, bar.y + bar.height);
                if !(x0.is_finite() && y0.is_finite() && x1.is_finite() && y1.is_finite()) {
                    return None;
                }
                Some(PlotPrimitive::Polygon {
                    points: vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)],
                    fill: Some(fill),
                    edge,
                })
            })
            .collect()
    }
}

impl PlotRender for BarSeriesData {
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

    #[test]
    fn test_stacked_bars() {
        let values = vec![vec![10.0, 20.0, 15.0], vec![5.0, 10.0, 8.0]];
        let config = StackedBarConfig::default();
        let bars = compute_stacked_bars(&values, 3, &config);

        assert_eq!(bars.len(), 6); // 2 series × 3 categories

        // First category, first series should start at 0
        assert!((bars[0].y - 0.0).abs() < 1e-10);
        assert!((bars[0].height - 10.0).abs() < 1e-10);

        // First category, second series should start at 10
        assert!((bars[3].y - 10.0).abs() < 1e-10);
        assert!((bars[3].height - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_grouped_bars() {
        let values = vec![vec![10.0, 20.0], vec![15.0, 25.0]];
        let config = GroupedBarConfig::default();
        let bars = compute_grouped_bars(&values, 2, &config);

        assert_eq!(bars.len(), 4); // 2 series × 2 categories

        // All bars should start at y=0 (not stacked)
        for bar in &bars {
            assert!((bar.y - 0.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_horizontal_stacked() {
        let values = vec![vec![10.0, 20.0]];
        let config = StackedBarConfig::default().horizontal();
        let bars = compute_stacked_bars(&values, 2, &config);

        // For horizontal, x is the value, width is the bar length
        assert!((bars[0].x - 0.0).abs() < 1e-10);
        assert!((bars[0].width - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_stacked_range() {
        let values = vec![vec![10.0, 20.0], vec![5.0, 15.0]];
        let (min, max) = stacked_bar_range(&values);
        assert!((min - 0.0).abs() < 1e-10);
        assert!((max - 35.0).abs() < 1e-10); // 20 + 15
    }

    #[test]
    fn test_grouped_range() {
        let values = vec![vec![10.0, -5.0], vec![20.0, 15.0]];
        let (min, max) = grouped_bar_range(&values);
        assert!((min - (-5.0)).abs() < 1e-10);
        assert!((max - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_stacked_bar_config_implements_plot_config() {
        fn assert_plot_config<T: PlotConfig>() {}
        assert_plot_config::<StackedBarConfig>();
    }

    #[test]
    fn test_grouped_bar_config_implements_plot_config() {
        fn assert_plot_config<T: PlotConfig>() {}
        assert_plot_config::<GroupedBarConfig>();
    }

    #[test]
    fn test_stacked_bar_plot_compute_trait() {
        use crate::plots::traits::PlotCompute;

        let values = vec![vec![10.0, 20.0, 15.0], vec![5.0, 10.0, 8.0]];
        let config = StackedBarConfig::default();
        let input = BarInput::new(&values, 3);
        let result = StackedBar::compute(input, &config);

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.bars.len(), 6);
        assert_eq!(data.num_categories, 3);
        assert_eq!(data.num_series, 2);
    }

    #[test]
    fn test_grouped_bar_plot_compute_trait() {
        use crate::plots::traits::PlotCompute;

        let values = vec![vec![10.0, 20.0], vec![15.0, 25.0]];
        let config = GroupedBarConfig::default();
        let input = BarInput::new(&values, 2);
        let result = GroupedBar::compute(input, &config);

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.bars.len(), 4);
        assert_eq!(data.num_categories, 2);
        assert_eq!(data.num_series, 2);
    }

    #[test]
    fn test_stacked_bar_plot_compute_empty() {
        use crate::plots::traits::PlotCompute;

        let values: Vec<Vec<f64>> = vec![];
        let config = StackedBarConfig::default();
        let input = BarInput::new(&values, 0);
        let result = StackedBar::compute(input, &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_stacked_bar_plot_data_trait() {
        use crate::plots::traits::{PlotCompute, PlotData};

        let values = vec![vec![10.0, 20.0], vec![5.0, 15.0]];
        let config = StackedBarConfig::default();
        let input = BarInput::new(&values, 2);
        let data = StackedBar::compute(input, &config).unwrap();

        // Test data_bounds
        let ((x_min, x_max), (y_min, y_max)) = data.data_bounds();
        assert!(x_min <= x_max);
        assert!(y_min <= y_max);

        // Test is_empty
        assert!(!data.is_empty());
    }

    #[test]
    fn test_grouped_bar_plot_data_trait() {
        use crate::plots::traits::{PlotCompute, PlotData};

        let values = vec![vec![10.0, 20.0], vec![15.0, 25.0]];
        let config = GroupedBarConfig::default();
        let input = BarInput::new(&values, 2);
        let data = GroupedBar::compute(input, &config).unwrap();

        // Test data_bounds
        let ((x_min, x_max), (y_min, y_max)) = data.data_bounds();
        assert!(x_min <= x_max);
        assert!(y_min <= y_max);

        // Test is_empty
        assert!(!data.is_empty());
    }

    // ===== The per-column split the `Plot` builder adds =====

    fn names() -> Vec<String> {
        vec!["2023".to_string(), "2024".to_string()]
    }

    fn categories3() -> Vec<String> {
        vec!["Q1".to_string(), "Q2".to_string(), "Q3".to_string()]
    }

    #[test]
    fn a_grouped_chart_splits_into_one_series_per_named_column() {
        let values = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
        let split = grouped_bar_series(
            &categories3(),
            &names(),
            &values,
            &GroupedBarConfig::default(),
        );

        assert_eq!(split.len(), 2, "one series per named value column");
        assert_eq!(split[0].0, "2023");
        assert_eq!(split[1].0, "2024");
        for (_, data) in &split {
            assert_eq!(data.bars.len(), 3, "one bar per category");
            assert_eq!(data.categories, categories3());
        }
    }

    #[test]
    fn a_group_occupies_exactly_one_category_slot() {
        // The whole point of sharing `category_slot_span`: a group is a slot,
        // subdivided — not a second positioning story with its own spacing.
        let values = vec![vec![1.0, 1.0], vec![1.0, 1.0], vec![1.0, 1.0]];
        let split = grouped_bar_series(
            &["a".to_string(), "b".to_string()],
            &["x".to_string(), "y".to_string(), "z".to_string()],
            &values,
            &GroupedBarConfig::default(),
        );

        let bars: Vec<&BarRect> = split.iter().flat_map(|(_, d)| d.bars.iter()).collect();
        for slot in 0..2usize {
            let (lo, hi) = category_slot_span(slot as f64);
            for bar in bars.iter().filter(|b| b.category == slot) {
                assert!(
                    bar.x >= lo && bar.x + bar.width <= hi,
                    "bar {bar:?} left its slot {lo}..{hi}"
                );
            }
        }
    }

    #[test]
    fn stacked_columns_report_cumulative_bounds() {
        // Each column's bounds cover its own segment; the union is the total.
        // Nothing computes a whole-chart extent separately, so nothing can
        // disagree with the geometry.
        let values = vec![vec![2.0, 1.0], vec![3.0, 4.0]];
        let split = stacked_bar_series(
            &["a".to_string(), "b".to_string()],
            &names(),
            &values,
            &StackedBarConfig::default(),
        );

        let (_, first) = &split[0];
        let (_, second) = &split[1];
        assert_eq!(first.data_bounds().1, (0.0, 2.0));
        assert_eq!(second.data_bounds().1, (0.0, 5.0));

        let top = split
            .iter()
            .map(|(_, d)| d.data_bounds().1.1)
            .fold(f64::NEG_INFINITY, f64::max);
        assert_eq!(top, stacked_bar_range(&values).1);
    }

    #[test]
    fn negative_contributions_stack_downwards() {
        // A negative value used to be added to the same running total as the
        // positives, so it ate into the stack above it and the geometry ended
        // somewhere `stacked_bar_range` did not.
        let values = vec![vec![3.0], vec![-2.0], vec![4.0]];
        let bars = compute_stacked_bars(&values, 1, &StackedBarConfig::default());

        assert_eq!(bars[0].y, 0.0);
        assert_eq!(bars[1].y, 0.0, "the negative bar hangs off the baseline");
        assert_eq!(bars[1].height, -2.0);
        assert_eq!(bars[2].y, 3.0, "the next positive resumes above the stack");

        let (low, high) = stacked_bar_range(&values);
        assert_eq!((low, high), (-2.0, 7.0));
    }

    #[test]
    fn a_non_finite_sample_does_not_poison_the_stack() {
        let values = vec![vec![1.0], vec![f64::NAN], vec![2.0]];
        let bars = compute_stacked_bars(&values, 1, &StackedBarConfig::default());
        assert_eq!(bars.len(), 2, "the NaN bar has no length, so it is dropped");
        assert_eq!(bars[1].y, 1.0);
    }

    #[test]
    fn every_column_labels_the_whole_category_axis() {
        let values = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
        for (_, data) in grouped_bar_series(
            &categories3(),
            &names(),
            &values,
            &GroupedBarConfig::default(),
        ) {
            assert_eq!(
                data.category_slots(),
                vec![
                    ("Q1".to_string(), 0.0),
                    ("Q2".to_string(), 1.0),
                    ("Q3".to_string(), 2.0),
                ]
            );
        }
    }

    #[test]
    fn a_bar_column_refuses_a_log_category_axis_in_the_words_a_bar_chart_uses() {
        let values = vec![vec![1.0, 2.0]];
        let split = stacked_bar_series(
            &["a".to_string(), "b".to_string()],
            &["only".to_string()],
            &values,
            &StackedBarConfig::default(),
        );
        assert_eq!(
            split[0].1.axis_scale_support(),
            (AxisScaleSupport::ORDINAL, AxisScaleSupport::Scaled)
        );
    }

    #[test]
    fn a_column_draws_one_filled_rectangle_per_bar() {
        let values = vec![vec![1.0, 2.0, 3.0]];
        let split = grouped_bar_series(
            &categories3(),
            &["only".to_string()],
            &values,
            &GroupedBarConfig::default(),
        );
        let data = &split[0].1;
        let ((x_min, x_max), (y_min, y_max)) = data.data_bounds();
        let area = PlotArea::new(0.0, 0.0, 200.0, 100.0, x_min, x_max, y_min, y_max);
        let style = ComputedStyle::opaque(
            crate::core::units::RenderScale::new(96.0),
            Color::from_rgb(10, 20, 30),
        );

        let primitives = data.primitives(&area, &style);
        assert_eq!(primitives.len(), 3);
        assert!(primitives.iter().all(|p| matches!(
            p,
            PlotPrimitive::Polygon {
                points,
                fill: Some(_),
                edge: None,
            } if points.len() == 4
        )));
    }
}
