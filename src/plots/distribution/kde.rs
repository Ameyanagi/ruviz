//! KDE (Kernel Density Estimation) plot implementations
//!
//! Provides smooth distribution visualization through kernel density estimation.
//!
//! # Trait-Based API
//!
//! The KDE plot implements the plot traits for unified behavior:
//! - [`crate::plots::PlotCompute`]: Transforms input data into KDE curves
//! - [`crate::plots::PlotData`]: Provides data bounds and emptiness check
//! - [`crate::plots::PlotRender`]: Renders to a canvas
//!
//! # Example
//!
//! ```rust,ignore
//! use ruviz::prelude::*;
//!
//! // Zero-ceremony API (recommended)
//! Plot::new()
//!     .kde(&data)
//!     .bandwidth(0.5)
//!     .fill(true)
//!     .title("KDE Distribution")
//!     .save("kde.png")?;
//! ```

use crate::core::error::Result;
use crate::plots::traits::{PlotArea, PlotCompute, PlotConfig, PlotData, PlotRender};
use crate::render::{Color, LineStyle, SkiaRenderer, Theme};
use crate::stats::kde::{kde_1d, kde_2d};

// =============================================================================
// KDE Configuration
// =============================================================================

/// Configuration for KDE plot
///
/// Controls the appearance and computation of kernel density estimation plots.
///
/// # Example
///
/// ```rust
/// use ruviz::plots::distribution::KdeConfig;
///
/// let config = KdeConfig::new()
///     .bandwidth(0.5)
///     .n_points(200)
///     .fill(true)
///     .fill_alpha(0.3);
/// ```
#[derive(Debug, Clone)]
pub struct KdeConfig {
    /// Bandwidth method (None = Scott's rule)
    pub bandwidth: Option<f64>,
    /// Number of points for density curve
    pub n_points: usize,
    /// Fill under the curve
    pub fill: bool,
    /// Fill alpha
    pub fill_alpha: f32,
    /// Line color (None for auto)
    pub color: Option<Color>,
    /// Line width
    pub line_width: f32,
    /// Whether to shade based on density
    pub shade: bool,
    /// Vertical line at specific value
    pub vertical_lines: Vec<f64>,
    /// Cumulative distribution
    pub cumulative: bool,
    /// Whether to clip at data bounds
    pub clip: Option<(f64, f64)>,
}

impl Default for KdeConfig {
    fn default() -> Self {
        Self {
            bandwidth: None,
            n_points: 200,
            fill: true,
            fill_alpha: 0.3,
            color: None,
            line_width: 2.0,
            shade: false,
            vertical_lines: vec![],
            cumulative: false,
            clip: None,
        }
    }
}

// Implement PlotConfig marker trait
impl PlotConfig for KdeConfig {}

impl KdeConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set bandwidth
    pub fn bandwidth(mut self, bw: f64) -> Self {
        self.bandwidth = Some(bw);
        self
    }

    /// Set number of points
    pub fn n_points(mut self, n: usize) -> Self {
        self.n_points = n.max(10);
        self
    }

    /// Enable/disable fill
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// Set fill alpha
    pub fn fill_alpha(mut self, alpha: f32) -> Self {
        self.fill_alpha = alpha.clamp(0.0, 1.0);
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

    /// Enable cumulative distribution
    pub fn cumulative(mut self, cumulative: bool) -> Self {
        self.cumulative = cumulative;
        self
    }

    /// Set clip bounds
    pub fn clip(mut self, min: f64, max: f64) -> Self {
        self.clip = Some((min, max));
        self
    }

    /// Add vertical line
    pub fn vertical_line(mut self, x: f64) -> Self {
        self.vertical_lines.push(x);
        self
    }
}

/// Deprecated alias for backward compatibility
#[deprecated(since = "0.8.0", note = "Use KdeConfig instead")]
pub type KdePlotConfig = KdeConfig;

// =============================================================================
// KDE Data
// =============================================================================

/// Computed KDE data for plotting
///
/// Contains the density curve coordinates and metadata from KDE computation.
/// This struct implements [`PlotData`] and [`PlotRender`] traits.
///
/// # Example
///
/// ```rust
/// use ruviz::plots::distribution::{KdeConfig, compute_kde};
///
/// let data = vec![1.0, 2.0, 2.5, 3.0, 3.5, 4.0];
/// let kde_data = compute_kde(&data, &KdeConfig::default());
///
/// // Access computed values
/// println!("Bandwidth: {}", kde_data.bandwidth);
/// println!("Points: {}", kde_data.x.len());
/// ```
#[derive(Debug, Clone)]
pub struct KdeData {
    /// X coordinates
    pub x: Vec<f64>,
    /// Y coordinates (density or cumulative)
    pub y: Vec<f64>,
    /// Bandwidth used
    pub bandwidth: f64,
    /// Whether this is cumulative
    pub cumulative: bool,
    /// Configuration used for computation (for rendering)
    pub(crate) config: KdeConfig,
}

/// Deprecated alias for backward compatibility
#[deprecated(since = "0.8.0", note = "Use KdeData instead")]
pub type KdePlotData = KdeData;

// =============================================================================
// KDE Computation
// =============================================================================

/// Compute KDE for plotting
///
/// # Arguments
/// * `data` - Input data
/// * `config` - KDE configuration
///
/// # Returns
/// `KdeData` for rendering
///
/// # Example
///
/// ```rust
/// use ruviz::plots::distribution::{KdeConfig, compute_kde};
///
/// let data = vec![1.0, 2.0, 2.5, 3.0, 3.5, 4.0];
/// let kde_data = compute_kde(&data, &KdeConfig::default());
///
/// assert!(!kde_data.x.is_empty());
/// ```
pub fn compute_kde(data: &[f64], config: &KdeConfig) -> KdeData {
    if data.is_empty() {
        return KdeData {
            x: vec![],
            y: vec![],
            bandwidth: 0.0,
            cumulative: false,
            config: config.clone(),
        };
    }

    let kde = kde_1d(data, config.bandwidth, Some(config.n_points));

    let (x, y) = if config.cumulative {
        // Convert to cumulative distribution
        let mut cumulative = Vec::with_capacity(kde.density.len());
        let mut sum = 0.0;
        let dx = if kde.x.len() > 1 {
            kde.x[1] - kde.x[0]
        } else {
            1.0
        };

        for d in &kde.density {
            sum += d * dx;
            cumulative.push(sum);
        }

        // Normalize to [0, 1]
        if let Some(&max) = cumulative.last() {
            if max > 0.0 {
                for c in &mut cumulative {
                    *c /= max;
                }
            }
        }

        (kde.x, cumulative)
    } else {
        (kde.x, kde.density)
    };

    // Apply clipping if specified
    let (x, y) = if let Some((min, max)) = config.clip {
        let mut clipped_x = Vec::new();
        let mut clipped_y = Vec::new();

        for (xi, yi) in x.iter().zip(y.iter()) {
            if *xi >= min && *xi <= max {
                clipped_x.push(*xi);
                clipped_y.push(*yi);
            }
        }

        (clipped_x, clipped_y)
    } else {
        (x, y)
    };

    KdeData {
        x,
        y,
        bandwidth: kde.bandwidth,
        cumulative: config.cumulative,
        config: config.clone(),
    }
}

/// Deprecated alias for backward compatibility
#[deprecated(since = "0.8.0", note = "Use compute_kde instead")]
pub fn compute_kde_plot(data: &[f64], config: &KdeConfig) -> KdeData {
    compute_kde(data, config)
}

/// Generate polygon vertices for filled KDE plot
///
/// Returns vertices that close the polygon to baseline
pub fn kde_fill_polygon(kde_data: &KdeData, baseline: f64) -> Vec<(f64, f64)> {
    if kde_data.x.is_empty() {
        return vec![];
    }

    let n = kde_data.x.len();
    let mut polygon = Vec::with_capacity(n * 2 + 2);

    // Top edge (KDE curve)
    for i in 0..n {
        polygon.push((kde_data.x[i], kde_data.y[i]));
    }

    // Bottom edge (baseline)
    polygon.push((kde_data.x[n - 1], baseline));
    polygon.push((kde_data.x[0], baseline));

    polygon
}

// =============================================================================
// Trait Implementations
// =============================================================================

/// Marker type for KDE plot computation
///
/// This empty struct is used to implement [`PlotCompute`] for KDE plots.
pub struct Kde;

impl PlotCompute for Kde {
    type Input<'a> = &'a [f64];
    type Config = KdeConfig;
    type Output = KdeData;

    fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output> {
        Ok(compute_kde(input, config))
    }
}

impl PlotData for KdeData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        if self.x.is_empty() {
            return ((0.0, 1.0), (0.0, 1.0));
        }

        let x_min = self.x.iter().copied().fold(f64::INFINITY, f64::min);
        let x_max = self.x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        // Density always starts at 0
        let y_min = 0.0;
        let y_max = self.y.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        ((x_min, x_max), (y_min, y_max * 1.05)) // 5% padding on top
    }

    fn is_empty(&self) -> bool {
        self.x.is_empty()
    }
}

impl PlotRender for KdeData {
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        _theme: &Theme,
        color: Color,
    ) -> Result<()> {
        if self.is_empty() {
            return Ok(());
        }

        // Transform data to screen coordinates
        let points: Vec<(f32, f32)> = self
            .x
            .iter()
            .zip(self.y.iter())
            .map(|(&x, &y)| area.data_to_screen(x, y))
            .collect();

        // Draw fill if enabled
        if self.config.fill {
            let baseline_y = area.data_to_screen(0.0, 0.0).1;

            // Create polygon for fill
            let mut polygon: Vec<(f32, f32)> = Vec::with_capacity(points.len() + 2);
            polygon.push((points[0].0, baseline_y));
            polygon.extend_from_slice(&points);
            polygon.push((points[points.len() - 1].0, baseline_y));

            let fill_color = color.with_alpha(self.config.fill_alpha);
            renderer.draw_filled_polygon(&polygon, fill_color)?;
        }

        // Draw the line
        let line_width = self.config.line_width;
        renderer.draw_polyline(&points, color, line_width, LineStyle::Solid)?;

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
        if self.is_empty() {
            return Ok(());
        }

        let actual_color = color.with_alpha(alpha);
        let actual_line_width = line_width.unwrap_or(self.config.line_width);

        // Transform data to screen coordinates
        let points: Vec<(f32, f32)> = self
            .x
            .iter()
            .zip(self.y.iter())
            .map(|(&x, &y)| area.data_to_screen(x, y))
            .collect();

        // Draw fill if enabled
        if self.config.fill {
            let baseline_y = area.data_to_screen(0.0, 0.0).1;

            let mut polygon: Vec<(f32, f32)> = Vec::with_capacity(points.len() + 2);
            polygon.push((points[0].0, baseline_y));
            polygon.extend_from_slice(&points);
            polygon.push((points[points.len() - 1].0, baseline_y));

            let fill_alpha = self.config.fill_alpha * alpha;
            let fill_color = color.with_alpha(fill_alpha);
            renderer.draw_filled_polygon(&polygon, fill_color)?;
        }

        // Draw the line
        renderer.draw_polyline(&points, actual_color, actual_line_width, LineStyle::Solid)?;

        // Call the base render for any additional processing
        let _ = theme; // Acknowledge unused parameter

        Ok(())
    }
}

/// Configuration for 2D KDE (density heatmap)
#[derive(Debug, Clone)]
pub struct Kde2dPlotConfig {
    /// Bandwidth for x dimension
    pub bandwidth_x: Option<f64>,
    /// Bandwidth for y dimension
    pub bandwidth_y: Option<f64>,
    /// Grid resolution
    pub grid_size: usize,
    /// Number of contour levels
    pub levels: usize,
    /// Whether to fill contours
    pub fill: bool,
    /// Colormap name
    pub cmap: String,
    /// Show scatter points
    pub show_points: bool,
    /// Point size
    pub point_size: f32,
    /// Point alpha
    pub point_alpha: f32,
}

impl Default for Kde2dPlotConfig {
    fn default() -> Self {
        Self {
            bandwidth_x: None,
            bandwidth_y: None,
            grid_size: 100,
            levels: 10,
            fill: true,
            cmap: "viridis".to_string(),
            show_points: false,
            point_size: 3.0,
            point_alpha: 0.5,
        }
    }
}

impl Kde2dPlotConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set grid size
    pub fn grid_size(mut self, size: usize) -> Self {
        self.grid_size = size.max(10);
        self
    }

    /// Set number of contour levels
    pub fn levels(mut self, levels: usize) -> Self {
        self.levels = levels.max(2);
        self
    }

    /// Enable fill
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// Show scatter points
    pub fn show_points(mut self, show: bool) -> Self {
        self.show_points = show;
        self
    }

    /// Set colormap
    pub fn cmap(mut self, cmap: &str) -> Self {
        self.cmap = cmap.to_string();
        self
    }
}

/// Result of 2D kernel density estimation for plotting
#[derive(Debug, Clone)]
pub struct Kde2dPlotData {
    /// X grid coordinates
    pub x: Vec<f64>,
    /// Y grid coordinates
    pub y: Vec<f64>,
    /// Density values as 2D array (row-major)
    pub density: Vec<Vec<f64>>,
}

/// Compute 2D KDE for density plot
pub fn compute_kde_2d_plot(x: &[f64], y: &[f64], config: &Kde2dPlotConfig) -> Kde2dPlotData {
    let bandwidth = match (config.bandwidth_x, config.bandwidth_y) {
        (Some(bx), Some(by)) => Some((bx, by)),
        _ => None,
    };
    let (x_grid, y_grid, density) = kde_2d(x, y, bandwidth, Some(config.grid_size));

    Kde2dPlotData {
        x: x_grid,
        y: y_grid,
        density,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plots::traits::PlotCompute;

    #[test]
    fn test_kde_basic() {
        let data = vec![1.0, 2.0, 2.5, 3.0, 3.5, 4.0];
        let config = KdeConfig::default();
        let kde_data = compute_kde(&data, &config);

        assert!(!kde_data.x.is_empty());
        assert_eq!(kde_data.x.len(), kde_data.y.len());
        assert!(!kde_data.cumulative);
    }

    #[test]
    fn test_kde_cumulative() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let config = KdeConfig::default().cumulative(true);
        let kde_data = compute_kde(&data, &config);

        assert!(kde_data.cumulative);
        // Cumulative should be monotonically increasing
        for i in 1..kde_data.y.len() {
            assert!(kde_data.y[i] >= kde_data.y[i - 1] - 1e-10);
        }
        // Last value should be approximately 1.0
        if let Some(&last) = kde_data.y.last() {
            assert!((last - 1.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_kde_clipped() {
        let data = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let config = KdeConfig::default().clip(1.0, 4.0);
        let kde_data = compute_kde(&data, &config);

        for &xi in &kde_data.x {
            assert!(xi >= 1.0 && xi <= 4.0);
        }
    }

    #[test]
    fn test_kde_fill_polygon() {
        let kde_data = KdeData {
            x: vec![0.0, 1.0, 2.0],
            y: vec![0.1, 0.5, 0.2],
            bandwidth: 0.5,
            cumulative: false,
            config: KdeConfig::default(),
        };

        let polygon = kde_fill_polygon(&kde_data, 0.0);
        assert_eq!(polygon.len(), 5); // 3 points + 2 baseline points
    }

    #[test]
    fn test_kde_empty() {
        let data: Vec<f64> = vec![];
        let config = KdeConfig::default();
        let kde_data = compute_kde(&data, &config);

        assert!(kde_data.x.is_empty());
        assert!(kde_data.y.is_empty());
    }

    // ==========================================================================
    // Trait Implementation Tests
    // ==========================================================================

    #[test]
    fn test_kde_plot_compute_trait() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let config = KdeConfig::default();

        let result = Kde::compute(&data, &config);
        assert!(result.is_ok());

        let kde_data = result.unwrap();
        assert!(!kde_data.is_empty());
        assert_eq!(kde_data.x.len(), config.n_points);
    }

    #[test]
    fn test_kde_plot_data_trait() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let config = KdeConfig::default();
        let kde_data = compute_kde(&data, &config);

        // Test data_bounds
        let ((x_min, x_max), (y_min, y_max)) = kde_data.data_bounds();
        assert!(x_min < x_max);
        assert_eq!(y_min, 0.0); // Density starts at 0
        assert!(y_max > 0.0);

        // Test is_empty
        assert!(!kde_data.is_empty());

        // Test empty data
        let empty_data: Vec<f64> = vec![];
        let empty_kde = compute_kde(&empty_data, &config);
        assert!(empty_kde.is_empty());
    }

    #[test]
    fn test_kde_config_implements_plot_config() {
        // Verify that KdeConfig implements PlotConfig (compile-time check)
        fn accepts_plot_config<T: PlotConfig>(_: &T) {}
        let config = KdeConfig::default();
        accepts_plot_config(&config);
    }

    #[test]
    fn test_kde_config_builder_methods() {
        let config = KdeConfig::new()
            .bandwidth(0.5)
            .n_points(100)
            .fill(true)
            .fill_alpha(0.5)
            .cumulative(false)
            .clip(0.0, 10.0)
            .vertical_line(5.0);

        assert_eq!(config.bandwidth, Some(0.5));
        assert_eq!(config.n_points, 100);
        assert!(config.fill);
        assert_eq!(config.fill_alpha, 0.5);
        assert!(!config.cumulative);
        assert_eq!(config.clip, Some((0.0, 10.0)));
        assert_eq!(config.vertical_lines.len(), 1);
    }

    // Backward compatibility tests
    #[test]
    #[allow(deprecated)]
    fn test_deprecated_type_aliases() {
        // Test that deprecated aliases still work
        let _config: KdePlotConfig = KdeConfig::default();
        let data = vec![1.0, 2.0, 3.0];
        let _kde_data: KdePlotData = compute_kde_plot(&data, &_config);
    }
}
