//! Area and fill between plot implementations
//!
//! Provides area fill, fill_between, and stackplot functionality.
//!
//! # Trait-Based API
//!
//! Area plots implement the core plot traits:
//! - [`PlotConfig`] for `AreaConfig` and `StackPlotConfig`
//! - [`PlotCompute`] for `Area` and `StackedArea` marker structs
//! - [`PlotData`] for `AreaData` and `StackedAreaData`
//! - [`PlotRender`] for `AreaData` and `StackedAreaData`

use crate::core::Result;
use crate::plots::traits::{PlotArea, PlotCompute, PlotConfig, PlotData, PlotRender};
use crate::render::skia::SkiaRenderer;
use crate::render::{Color, LineStyle, Theme};

/// Configuration for area plot
#[derive(Debug, Clone)]
pub struct AreaConfig {
    /// Fill color
    pub color: Option<Color>,
    /// Fill alpha
    pub alpha: f32,
    /// Line color (None for no line)
    pub line_color: Option<Color>,
    /// Line width
    pub line_width: f32,
    /// Baseline value (default 0.0)
    pub baseline: f64,
    /// Interpolation between points
    pub interpolation: AreaInterpolation,
}

/// Interpolation method for area fill
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AreaInterpolation {
    /// Linear interpolation between points
    Linear,
    /// Step function (constant until next point)
    Step,
    /// Smooth spline interpolation
    Smooth,
}

impl Default for AreaConfig {
    fn default() -> Self {
        Self {
            color: None,
            alpha: 0.5,
            line_color: None,
            line_width: 1.5,
            baseline: 0.0,
            interpolation: AreaInterpolation::Linear,
        }
    }
}

impl AreaConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set fill color
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set fill alpha
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
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

    /// Set baseline
    pub fn baseline(mut self, baseline: f64) -> Self {
        self.baseline = baseline;
        self
    }

    /// Set interpolation
    pub fn interpolation(mut self, interp: AreaInterpolation) -> Self {
        self.interpolation = interp;
        self
    }
}

/// Generate polygon vertices for area fill (fill to baseline)
///
/// # Arguments
/// * `x` - X coordinates
/// * `y` - Y coordinates
/// * `baseline` - Y value for baseline
///
/// # Returns
/// Polygon vertices as (x, y) pairs
pub fn area_polygon(x: &[f64], y: &[f64], baseline: f64) -> Vec<(f64, f64)> {
    if x.is_empty() || y.is_empty() {
        return vec![];
    }

    let n = x.len().min(y.len());
    let mut polygon = Vec::with_capacity(n * 2 + 2);

    // Top edge (data points)
    for i in 0..n {
        polygon.push((x[i], y[i]));
    }

    // Bottom edge (baseline, reversed)
    polygon.push((x[n - 1], baseline));
    polygon.push((x[0], baseline));

    polygon
}

/// Generate polygon vertices for fill_between
///
/// # Arguments
/// * `x` - X coordinates
/// * `y1` - Lower Y boundary
/// * `y2` - Upper Y boundary
///
/// # Returns
/// Polygon vertices as (x, y) pairs
pub fn fill_between_polygon(x: &[f64], y1: &[f64], y2: &[f64]) -> Vec<(f64, f64)> {
    if x.is_empty() || y1.is_empty() || y2.is_empty() {
        return vec![];
    }

    let n = x.len().min(y1.len()).min(y2.len());
    let mut polygon = Vec::with_capacity(n * 2);

    // Upper boundary (y2)
    for i in 0..n {
        polygon.push((x[i], y2[i]));
    }

    // Lower boundary (y1, reversed)
    for i in (0..n).rev() {
        polygon.push((x[i], y1[i]));
    }

    polygon
}

/// Generate polygon vertices for fill_between with condition
///
/// Only fills where condition is true
///
/// # Arguments
/// * `x` - X coordinates
/// * `y1` - Lower Y boundary
/// * `y2` - Upper Y boundary
/// * `where_mask` - Boolean mask for which points to include
///
/// # Returns
/// Vec of polygon segments (each segment is a separate filled region)
pub fn fill_between_where(
    x: &[f64],
    y1: &[f64],
    y2: &[f64],
    where_mask: &[bool],
) -> Vec<Vec<(f64, f64)>> {
    if x.is_empty() || y1.is_empty() || y2.is_empty() || where_mask.is_empty() {
        return vec![];
    }

    let n = x.len().min(y1.len()).min(y2.len()).min(where_mask.len());
    let mut segments = Vec::new();
    let mut current_segment: Option<(usize, usize)> = None;

    for (i, &mask_val) in where_mask.iter().enumerate().take(n) {
        if mask_val {
            match current_segment {
                None => current_segment = Some((i, i)),
                Some((start, _)) => current_segment = Some((start, i)),
            }
        } else if let Some((start, end)) = current_segment {
            // End of segment
            let segment_x: Vec<f64> = x[start..=end].to_vec();
            let segment_y1: Vec<f64> = y1[start..=end].to_vec();
            let segment_y2: Vec<f64> = y2[start..=end].to_vec();
            segments.push(fill_between_polygon(&segment_x, &segment_y1, &segment_y2));
            current_segment = None;
        }
    }

    // Handle final segment
    if let Some((start, end)) = current_segment {
        let segment_x: Vec<f64> = x[start..=end].to_vec();
        let segment_y1: Vec<f64> = y1[start..=end].to_vec();
        let segment_y2: Vec<f64> = y2[start..=end].to_vec();
        segments.push(fill_between_polygon(&segment_x, &segment_y1, &segment_y2));
    }

    segments
}

/// Configuration for stacked area plot
#[derive(Debug, Clone)]
pub struct StackPlotConfig {
    /// Colors for each series (None for auto-colors)
    pub colors: Option<Vec<Color>>,
    /// Alpha for fill
    pub alpha: f32,
    /// Labels for each series
    pub labels: Vec<String>,
    /// Baseline mode
    pub baseline: StackBaseline,
    /// Show lines between areas
    pub show_lines: bool,
    /// Line color
    pub line_color: Color,
    /// Line width
    pub line_width: f32,
}

/// Baseline mode for stack plot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackBaseline {
    /// Zero baseline (standard stacked area)
    Zero,
    /// Symmetric around zero (streamgraph style)
    Symmetric,
    /// Wiggle minimized (ThemeRiver style)
    Wiggle,
}

impl Default for StackPlotConfig {
    fn default() -> Self {
        Self {
            colors: None,
            alpha: 0.8,
            labels: vec![],
            baseline: StackBaseline::Zero,
            show_lines: false,
            line_color: Color::new(255, 255, 255),
            line_width: 0.5,
        }
    }
}

impl StackPlotConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
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

    /// Set baseline mode
    pub fn baseline(mut self, baseline: StackBaseline) -> Self {
        self.baseline = baseline;
        self
    }

    /// Show separator lines
    pub fn lines(mut self, show: bool) -> Self {
        self.show_lines = show;
        self
    }
}

// Implement PlotConfig marker trait
impl PlotConfig for AreaConfig {}
impl PlotConfig for StackPlotConfig {}

/// Marker struct for Area plot type
pub struct Area;

/// Marker struct for StackedArea plot type
pub struct StackedArea;

/// Compute stacked area data
///
/// # Arguments
/// * `x` - Shared X coordinates
/// * `ys` - Multiple Y series to stack
/// * `baseline` - Baseline mode
///
/// # Returns
/// Vec of (lower_bound, upper_bound) for each series
pub fn compute_stack(
    x: &[f64],
    ys: &[Vec<f64>],
    baseline: StackBaseline,
) -> Vec<(Vec<f64>, Vec<f64>)> {
    if x.is_empty() || ys.is_empty() {
        return vec![];
    }

    let n = x.len();
    let num_series = ys.len();

    // Compute cumulative sums
    let mut cumulative: Vec<Vec<f64>> = vec![vec![0.0; n]; num_series + 1];

    for (i, y) in ys.iter().enumerate() {
        for j in 0..n.min(y.len()) {
            cumulative[i + 1][j] = cumulative[i][j] + y[j];
        }
    }

    // Apply baseline transformation
    let offset: Vec<f64> = match baseline {
        StackBaseline::Zero => vec![0.0; n],
        StackBaseline::Symmetric => {
            // Center around zero
            let total = &cumulative[num_series];
            total.iter().map(|t| -t / 2.0).collect()
        }
        StackBaseline::Wiggle => {
            // Minimize wiggle (ThemeRiver algorithm)
            // For simplicity, use symmetric for now
            let total = &cumulative[num_series];
            total.iter().map(|t| -t / 2.0).collect()
        }
    };

    // Build result
    let mut result = Vec::with_capacity(num_series);
    for i in 0..num_series {
        let lower: Vec<f64> = cumulative[i]
            .iter()
            .zip(offset.iter())
            .map(|(c, o)| c + o)
            .collect();
        let upper: Vec<f64> = cumulative[i + 1]
            .iter()
            .zip(offset.iter())
            .map(|(c, o)| c + o)
            .collect();
        result.push((lower, upper));
    }

    result
}

// ============================================================================
// Trait-Based API
// ============================================================================

/// Computed area data
#[derive(Debug, Clone)]
pub struct AreaData {
    /// Polygon vertices for the filled area
    pub polygon: Vec<(f64, f64)>,
    /// X coordinates
    pub x: Vec<f64>,
    /// Y coordinates
    pub y: Vec<f64>,
    /// Data bounds
    pub bounds: ((f64, f64), (f64, f64)),
    /// Configuration used
    pub(crate) config: AreaConfig,
}

/// Input for area plot computation
pub struct AreaInput<'a> {
    /// X coordinates
    pub x: &'a [f64],
    /// Y coordinates
    pub y: &'a [f64],
}

impl<'a> AreaInput<'a> {
    /// Create new area input
    pub fn new(x: &'a [f64], y: &'a [f64]) -> Self {
        Self { x, y }
    }
}

impl PlotCompute for Area {
    type Input<'a> = AreaInput<'a>;
    type Config = AreaConfig;
    type Output = AreaData;

    fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output> {
        if input.x.is_empty() || input.y.is_empty() {
            return Err(crate::core::PlottingError::EmptyDataSet);
        }

        let polygon = area_polygon(input.x, input.y, config.baseline);

        let x_min = input.x.iter().cloned().fold(f64::INFINITY, f64::min);
        let x_max = input.x.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let y_min = input
            .y
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min)
            .min(config.baseline);
        let y_max = input
            .y
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max)
            .max(config.baseline);

        Ok(AreaData {
            polygon,
            x: input.x.to_vec(),
            y: input.y.to_vec(),
            bounds: ((x_min, x_max), (y_min, y_max)),
            config: config.clone(),
        })
    }
}

impl PlotData for AreaData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        self.bounds
    }

    fn is_empty(&self) -> bool {
        self.polygon.is_empty()
    }
}

impl PlotRender for AreaData {
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        _theme: &Theme,
        color: Color,
    ) -> Result<()> {
        if self.polygon.is_empty() {
            return Ok(());
        }

        let config = &self.config;
        let fill_color = config.color.unwrap_or(color).with_alpha(config.alpha);

        // Convert polygon to screen coordinates
        let screen_polygon: Vec<(f32, f32)> = self
            .polygon
            .iter()
            .map(|(x, y)| area.data_to_screen(*x, *y))
            .collect();

        // Draw filled area
        renderer.draw_filled_polygon(&screen_polygon, fill_color)?;

        // Draw top line if configured
        if let Some(line_color) = config.line_color {
            let n = self.x.len();
            let line_points: Vec<(f32, f32)> = (0..n)
                .map(|i| area.data_to_screen(self.x[i], self.y[i]))
                .collect();
            renderer.draw_polyline(
                &line_points,
                line_color,
                config.line_width,
                LineStyle::Solid,
            )?;
        }

        Ok(())
    }
}

/// Computed stacked area data
#[derive(Debug, Clone)]
pub struct StackedAreaData {
    /// Stack bounds for each series (lower, upper)
    pub stacks: Vec<(Vec<f64>, Vec<f64>)>,
    /// X coordinates
    pub x: Vec<f64>,
    /// Data bounds
    pub bounds: ((f64, f64), (f64, f64)),
    /// Configuration used
    pub(crate) config: StackPlotConfig,
}

/// Input for stacked area plot computation
pub struct StackedAreaInput<'a> {
    /// X coordinates
    pub x: &'a [f64],
    /// Y series to stack
    pub ys: &'a [Vec<f64>],
}

impl<'a> StackedAreaInput<'a> {
    /// Create new stacked area input
    pub fn new(x: &'a [f64], ys: &'a [Vec<f64>]) -> Self {
        Self { x, ys }
    }
}

impl PlotCompute for StackedArea {
    type Input<'a> = StackedAreaInput<'a>;
    type Config = StackPlotConfig;
    type Output = StackedAreaData;

    fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output> {
        if input.x.is_empty() || input.ys.is_empty() {
            return Err(crate::core::PlottingError::EmptyDataSet);
        }

        let stacks = compute_stack(input.x, input.ys, config.baseline);

        if stacks.is_empty() {
            return Err(crate::core::PlottingError::EmptyDataSet);
        }

        // Compute bounds
        let x_min = input.x.iter().cloned().fold(f64::INFINITY, f64::min);
        let x_max = input.x.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for (lower, upper) in &stacks {
            y_min = y_min.min(lower.iter().cloned().fold(f64::INFINITY, f64::min));
            y_max = y_max.max(upper.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
        }

        Ok(StackedAreaData {
            stacks,
            x: input.x.to_vec(),
            bounds: ((x_min, x_max), (y_min, y_max)),
            config: config.clone(),
        })
    }
}

impl PlotData for StackedAreaData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        self.bounds
    }

    fn is_empty(&self) -> bool {
        self.stacks.is_empty()
    }
}

impl PlotRender for StackedAreaData {
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        theme: &Theme,
        _color: Color,
    ) -> Result<()> {
        if self.stacks.is_empty() {
            return Ok(());
        }

        let config = &self.config;

        for (i, (lower, upper)) in self.stacks.iter().enumerate() {
            let polygon = fill_between_polygon(&self.x, lower, upper);

            let fill_color = config
                .colors
                .as_ref()
                .and_then(|c| c.get(i).copied())
                .unwrap_or_else(|| theme.get_color(i))
                .with_alpha(config.alpha);

            // Convert to screen coordinates
            let screen_polygon: Vec<(f32, f32)> = polygon
                .iter()
                .map(|(x, y)| area.data_to_screen(*x, *y))
                .collect();

            renderer.draw_filled_polygon(&screen_polygon, fill_color)?;

            // Draw separator line if configured
            if config.show_lines && i < self.stacks.len() - 1 {
                let upper_line: Vec<(f32, f32)> = self
                    .x
                    .iter()
                    .zip(upper.iter())
                    .map(|(x, y)| area.data_to_screen(*x, *y))
                    .collect();
                renderer.draw_polyline(
                    &upper_line,
                    config.line_color,
                    config.line_width,
                    LineStyle::Solid,
                )?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_area_polygon() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![1.0, 2.0, 1.0];
        let polygon = area_polygon(&x, &y, 0.0);

        // Should have 5 points: 3 data + 2 baseline
        assert_eq!(polygon.len(), 5);
        assert_eq!(polygon[0], (0.0, 1.0));
        assert_eq!(polygon[3], (2.0, 0.0));
        assert_eq!(polygon[4], (0.0, 0.0));
    }

    #[test]
    fn test_fill_between_polygon() {
        let x = vec![0.0, 1.0, 2.0];
        let y1 = vec![0.0, 0.0, 0.0];
        let y2 = vec![1.0, 2.0, 1.0];
        let polygon = fill_between_polygon(&x, &y1, &y2);

        // Should have 6 points: 3 upper + 3 lower
        assert_eq!(polygon.len(), 6);
    }

    #[test]
    fn test_fill_between_where() {
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let y1 = vec![0.0; 5];
        let y2 = vec![1.0; 5];
        let mask = vec![true, true, false, true, true];

        let segments = fill_between_where(&x, &y1, &y2, &mask);
        assert_eq!(segments.len(), 2); // Two segments due to false in middle
    }

    #[test]
    fn test_compute_stack_zero() {
        let x = vec![0.0, 1.0, 2.0];
        let ys = vec![vec![1.0, 2.0, 1.0], vec![2.0, 1.0, 2.0]];

        let stack = compute_stack(&x, &ys, StackBaseline::Zero);
        assert_eq!(stack.len(), 2);

        // First series: 0 to y[0]
        assert!((stack[0].0[0] - 0.0).abs() < 1e-10);
        assert!((stack[0].1[0] - 1.0).abs() < 1e-10);

        // Second series: y[0] to y[0]+y[1]
        assert!((stack[1].0[0] - 1.0).abs() < 1e-10);
        assert!((stack[1].1[0] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_stack_symmetric() {
        let x = vec![0.0, 1.0];
        let ys = vec![vec![2.0, 2.0]];

        let stack = compute_stack(&x, &ys, StackBaseline::Symmetric);
        assert_eq!(stack.len(), 1);

        // Should be centered around 0
        assert!((stack[0].0[0] - (-1.0)).abs() < 1e-10);
        assert!((stack[0].1[0] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_area_config_implements_plot_config() {
        fn assert_plot_config<T: PlotConfig>() {}
        assert_plot_config::<AreaConfig>();
    }

    #[test]
    fn test_stack_plot_config_implements_plot_config() {
        fn assert_plot_config<T: PlotConfig>() {}
        assert_plot_config::<StackPlotConfig>();
    }

    #[test]
    fn test_area_plot_compute_trait() {
        use crate::plots::traits::PlotCompute;

        let x = vec![0.0, 1.0, 2.0];
        let y = vec![1.0, 2.0, 1.0];
        let config = AreaConfig::default();
        let input = AreaInput::new(&x, &y);
        let result = Area::compute(input, &config);

        assert!(result.is_ok());
        let area_data = result.unwrap();
        assert!(!area_data.polygon.is_empty());
    }

    #[test]
    fn test_area_plot_compute_empty() {
        use crate::plots::traits::PlotCompute;

        let x: Vec<f64> = vec![];
        let y: Vec<f64> = vec![];
        let config = AreaConfig::default();
        let input = AreaInput::new(&x, &y);
        let result = Area::compute(input, &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_area_plot_data_trait() {
        use crate::plots::traits::{PlotCompute, PlotData};

        let x = vec![0.0, 1.0, 2.0];
        let y = vec![1.0, 2.0, 1.0];
        let config = AreaConfig::default();
        let input = AreaInput::new(&x, &y);
        let area_data = Area::compute(input, &config).unwrap();

        // Test data_bounds
        let ((x_min, x_max), (y_min, y_max)) = area_data.data_bounds();
        assert!((x_min - 0.0).abs() < 1e-10);
        assert!((x_max - 2.0).abs() < 1e-10);
        assert!(y_min <= y_max);

        // Test is_empty
        assert!(!area_data.is_empty());
    }

    #[test]
    fn test_stacked_area_plot_compute_trait() {
        use crate::plots::traits::PlotCompute;

        let x = vec![0.0, 1.0, 2.0];
        let ys = vec![vec![1.0, 2.0, 1.0], vec![2.0, 1.0, 2.0]];
        let config = StackPlotConfig::default();
        let input = StackedAreaInput::new(&x, &ys);
        let result = StackedArea::compute(input, &config);

        assert!(result.is_ok());
        let stack_data = result.unwrap();
        assert_eq!(stack_data.stacks.len(), 2);
    }

    #[test]
    fn test_stacked_area_plot_data_trait() {
        use crate::plots::traits::{PlotCompute, PlotData};

        let x = vec![0.0, 1.0, 2.0];
        let ys = vec![vec![1.0, 2.0, 1.0], vec![2.0, 1.0, 2.0]];
        let config = StackPlotConfig::default();
        let input = StackedAreaInput::new(&x, &ys);
        let stack_data = StackedArea::compute(input, &config).unwrap();

        // Test data_bounds
        let ((x_min, x_max), (y_min, y_max)) = stack_data.data_bounds();
        assert!((x_min - 0.0).abs() < 1e-10);
        assert!((x_max - 2.0).abs() < 1e-10);
        assert!(y_min <= y_max);

        // Test is_empty
        assert!(!stack_data.is_empty());
    }
}
