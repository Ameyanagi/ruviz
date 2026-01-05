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

use crate::core::Result;
use crate::plots::traits::{PlotArea, PlotCompute, PlotConfig, PlotData, PlotRender};
use crate::render::skia::SkiaRenderer;
use crate::render::{Color, LineStyle, Theme};

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

/// Orientation for bar charts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarOrientation {
    Vertical,
    Horizontal,
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

    let num_series = values.len();
    let bar_width = config.width;
    let half_width = bar_width / 2.0;

    let mut bars = Vec::new();
    let mut cumulative = vec![0.0; categories];

    for (series_idx, series_values) in values.iter().enumerate() {
        for cat_idx in 0..categories.min(series_values.len()) {
            let value = series_values[cat_idx];
            let base = cumulative[cat_idx];

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

            cumulative[cat_idx] += value;
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
            if config.edge_width > 0.0 {
                if let Some(edge_color) = config.edge_color {
                    let outline = vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h), (x, y)];
                    renderer.draw_polyline(
                        &outline,
                        edge_color,
                        config.edge_width,
                        LineStyle::Solid,
                    )?;
                }
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
            if config.edge_width > 0.0 {
                if let Some(edge_color) = config.edge_color {
                    let outline = vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h), (x, y)];
                    renderer.draw_polyline(
                        &outline,
                        edge_color,
                        config.edge_width,
                        LineStyle::Solid,
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
}
