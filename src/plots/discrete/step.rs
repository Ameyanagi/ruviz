//! Step plot implementations
//!
//! Provides step function visualization for discrete/piecewise-constant data.
//!
//! # Trait-Based API
//!
//! Step plots implement the core plot traits:
//! - [`PlotConfig`] for `StepConfig`
//! - [`PlotCompute`] for `Step` marker struct
//! - [`PlotData`] for `StepData`
//! - [`PlotRender`] for `StepData`

use crate::core::Result;
use crate::plots::traits::{PlotArea, PlotCompute, PlotConfig, PlotData, PlotRender};
use crate::render::skia::SkiaRenderer;
use crate::render::{Color, LineStyle, Theme};

/// Where to place the step
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepWhere {
    /// Step at the left of each point (pre)
    Pre,
    /// Step at the right of each point (post)
    Post,
    /// Step at the middle between points (mid)
    Mid,
}

/// Configuration for step plot
#[derive(Debug, Clone)]
pub struct StepConfig {
    /// Where to place the step transition
    pub where_step: StepWhere,
    /// Line color (None for auto)
    pub color: Option<Color>,
    /// Line width
    pub line_width: f32,
    /// Fill under the step
    pub fill: bool,
    /// Fill alpha
    pub fill_alpha: f32,
    /// Baseline for fill
    pub baseline: f64,
}

impl Default for StepConfig {
    fn default() -> Self {
        Self {
            where_step: StepWhere::Pre,
            color: None,
            line_width: 1.5,
            fill: false,
            fill_alpha: 0.3,
            baseline: 0.0,
        }
    }
}

impl StepConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set step position
    pub fn where_step(mut self, where_step: StepWhere) -> Self {
        self.where_step = where_step;
        self
    }

    /// Set line color
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set line width
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width.max(0.1);
        self
    }

    /// Enable fill
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// Set fill alpha
    pub fn fill_alpha(mut self, alpha: f32) -> Self {
        self.fill_alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Set baseline
    pub fn baseline(mut self, baseline: f64) -> Self {
        self.baseline = baseline;
        self
    }
}

// Implement PlotConfig marker trait
impl PlotConfig for StepConfig {}

/// Marker struct for Step plot type
pub struct Step;

/// Compute step line vertices
///
/// # Arguments
/// * `x` - X coordinates
/// * `y` - Y coordinates
/// * `where_step` - Where to place the step
///
/// # Returns
/// Step line vertices as (x, y) pairs
pub fn step_line(x: &[f64], y: &[f64], where_step: StepWhere) -> Vec<(f64, f64)> {
    if x.is_empty() || y.is_empty() {
        return vec![];
    }

    let n = x.len().min(y.len());
    let mut vertices = Vec::with_capacity(n * 2);

    match where_step {
        StepWhere::Pre => {
            // Vertical step before horizontal
            for i in 0..n {
                if i > 0 {
                    vertices.push((x[i], y[i - 1]));
                }
                vertices.push((x[i], y[i]));
            }
        }
        StepWhere::Post => {
            // Horizontal then vertical step
            for i in 0..n {
                vertices.push((x[i], y[i]));
                if i < n - 1 {
                    vertices.push((x[i + 1], y[i]));
                }
            }
        }
        StepWhere::Mid => {
            // Step at midpoint
            for i in 0..n {
                if i > 0 {
                    let mid_x = (x[i - 1] + x[i]) / 2.0;
                    vertices.push((mid_x, y[i - 1]));
                    vertices.push((mid_x, y[i]));
                }
                vertices.push((x[i], y[i]));
            }
        }
    }

    vertices
}

/// Generate step polygon for filled step plot
///
/// # Arguments
/// * `x` - X coordinates
/// * `y` - Y coordinates
/// * `where_step` - Where to place the step
/// * `baseline` - Y value for baseline
///
/// # Returns
/// Polygon vertices as (x, y) pairs
pub fn step_polygon(x: &[f64], y: &[f64], where_step: StepWhere, baseline: f64) -> Vec<(f64, f64)> {
    let line_vertices = step_line(x, y, where_step);
    if line_vertices.is_empty() {
        return vec![];
    }

    let mut polygon = line_vertices;

    // Add baseline to close the polygon
    if let Some(&(last_x, _)) = polygon.last() {
        polygon.push((last_x, baseline));
    }
    if let Some(&(first_x, _)) = polygon.first() {
        polygon.push((first_x, baseline));
    }

    polygon
}

/// Compute step data range
pub fn step_range(x: &[f64], y: &[f64], baseline: f64) -> ((f64, f64), (f64, f64)) {
    if x.is_empty() || y.is_empty() {
        return ((0.0, 1.0), (0.0, 1.0));
    }

    let x_min = x.iter().copied().fold(f64::INFINITY, f64::min);
    let x_max = x.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    let y_min = y
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min)
        .min(baseline);
    let y_max = y
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max)
        .max(baseline);

    ((x_min, x_max), (y_min, y_max))
}

// ============================================================================
// Trait-Based API
// ============================================================================

/// Computed step plot data
#[derive(Debug, Clone)]
pub struct StepData {
    /// Line vertices
    pub line: Vec<(f64, f64)>,
    /// Polygon vertices (if fill enabled)
    pub polygon: Vec<(f64, f64)>,
    /// Original X coordinates
    pub x: Vec<f64>,
    /// Original Y coordinates
    pub y: Vec<f64>,
    /// Data bounds
    pub bounds: ((f64, f64), (f64, f64)),
    /// Configuration used
    pub(crate) config: StepConfig,
}

/// Input for step plot computation
pub struct StepInput<'a> {
    /// X coordinates
    pub x: &'a [f64],
    /// Y coordinates
    pub y: &'a [f64],
}

impl<'a> StepInput<'a> {
    /// Create new step input
    pub fn new(x: &'a [f64], y: &'a [f64]) -> Self {
        Self { x, y }
    }
}

impl PlotCompute for Step {
    type Input<'a> = StepInput<'a>;
    type Config = StepConfig;
    type Output = StepData;

    fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output> {
        if input.x.is_empty() || input.y.is_empty() {
            return Err(crate::core::PlottingError::EmptyDataSet);
        }

        let line = step_line(input.x, input.y, config.where_step);
        let polygon = if config.fill {
            step_polygon(input.x, input.y, config.where_step, config.baseline)
        } else {
            vec![]
        };
        let bounds = step_range(input.x, input.y, config.baseline);

        Ok(StepData {
            line,
            polygon,
            x: input.x.to_vec(),
            y: input.y.to_vec(),
            bounds,
            config: config.clone(),
        })
    }
}

impl PlotData for StepData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        self.bounds
    }

    fn is_empty(&self) -> bool {
        self.line.is_empty()
    }
}

impl PlotRender for StepData {
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        _theme: &Theme,
        color: Color,
    ) -> Result<()> {
        if self.line.is_empty() {
            return Ok(());
        }

        let config = &self.config;
        let line_color = config.color.unwrap_or(color);

        // Draw fill if enabled
        if config.fill && !self.polygon.is_empty() {
            let fill_color = line_color.with_alpha(config.fill_alpha);
            let screen_polygon: Vec<(f32, f32)> = self
                .polygon
                .iter()
                .map(|(x, y)| area.data_to_screen(*x, *y))
                .collect();
            renderer.draw_filled_polygon(&screen_polygon, fill_color)?;
        }

        // Draw step line
        let screen_line: Vec<(f32, f32)> = self
            .line
            .iter()
            .map(|(x, y)| area.data_to_screen(*x, *y))
            .collect();
        renderer.draw_polyline(
            &screen_line,
            line_color,
            config.line_width,
            LineStyle::Solid,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_pre() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![1.0, 2.0, 1.5];
        let vertices = step_line(&x, &y, StepWhere::Pre);

        // Pre: first point, then (x1, y0), (x1, y1), (x2, y1), (x2, y2)
        assert!(!vertices.is_empty());
        assert_eq!(vertices[0], (0.0, 1.0));
    }

    #[test]
    fn test_step_post() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![1.0, 2.0, 1.5];
        let vertices = step_line(&x, &y, StepWhere::Post);

        // Post: (x0, y0), (x1, y0), (x1, y1), (x2, y1), (x2, y2)
        assert!(!vertices.is_empty());
        assert_eq!(vertices[0], (0.0, 1.0));
        assert_eq!(vertices[1], (1.0, 1.0));
    }

    #[test]
    fn test_step_mid() {
        let x = vec![0.0, 2.0, 4.0];
        let y = vec![1.0, 2.0, 1.5];
        let vertices = step_line(&x, &y, StepWhere::Mid);

        // Mid: steps at midpoints
        assert!(!vertices.is_empty());
        // Should contain midpoint x=1.0
        assert!(vertices.iter().any(|(xi, _)| (*xi - 1.0).abs() < 1e-10));
    }

    #[test]
    fn test_step_polygon() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![1.0, 2.0, 1.5];
        let polygon = step_polygon(&x, &y, StepWhere::Post, 0.0);

        // Should be closed polygon including baseline
        assert!(!polygon.is_empty());
        // Last points should be on baseline
        let n = polygon.len();
        assert!((polygon[n - 1].1 - 0.0).abs() < 1e-10);
        assert!((polygon[n - 2].1 - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_step_range() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![1.0, 3.0, 2.0];
        let ((x_min, x_max), (y_min, y_max)) = step_range(&x, &y, 0.0);

        assert!((x_min - 0.0).abs() < 1e-10);
        assert!((x_max - 2.0).abs() < 1e-10);
        assert!((y_min - 0.0).abs() < 1e-10); // baseline
        assert!((y_max - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_step_empty() {
        let x: Vec<f64> = vec![];
        let y: Vec<f64> = vec![];
        let vertices = step_line(&x, &y, StepWhere::Pre);
        assert!(vertices.is_empty());
    }

    #[test]
    fn test_step_config_implements_plot_config() {
        fn assert_plot_config<T: PlotConfig>() {}
        assert_plot_config::<StepConfig>();
    }

    #[test]
    fn test_step_plot_compute_trait() {
        use crate::plots::traits::PlotCompute;

        let x = vec![0.0, 1.0, 2.0];
        let y = vec![1.0, 2.0, 1.5];
        let config = StepConfig::default();
        let input = StepInput::new(&x, &y);
        let result = Step::compute(input, &config);

        assert!(result.is_ok());
        let step_data = result.unwrap();
        assert!(!step_data.line.is_empty());
    }

    #[test]
    fn test_step_plot_compute_with_fill() {
        use crate::plots::traits::PlotCompute;

        let x = vec![0.0, 1.0, 2.0];
        let y = vec![1.0, 2.0, 1.5];
        let config = StepConfig::default().fill(true);
        let input = StepInput::new(&x, &y);
        let result = Step::compute(input, &config);

        assert!(result.is_ok());
        let step_data = result.unwrap();
        assert!(!step_data.polygon.is_empty());
    }

    #[test]
    fn test_step_plot_compute_empty() {
        use crate::plots::traits::PlotCompute;

        let x: Vec<f64> = vec![];
        let y: Vec<f64> = vec![];
        let config = StepConfig::default();
        let input = StepInput::new(&x, &y);
        let result = Step::compute(input, &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_step_plot_data_trait() {
        use crate::plots::traits::{PlotCompute, PlotData};

        let x = vec![0.0, 1.0, 2.0];
        let y = vec![1.0, 3.0, 2.0];
        let config = StepConfig::default();
        let input = StepInput::new(&x, &y);
        let step_data = Step::compute(input, &config).unwrap();

        // Test data_bounds
        let ((x_min, x_max), (y_min, y_max)) = step_data.data_bounds();
        assert!((x_min - 0.0).abs() < 1e-10);
        assert!((x_max - 2.0).abs() < 1e-10);
        assert!(y_min <= y_max);

        // Test is_empty
        assert!(!step_data.is_empty());
    }
}
