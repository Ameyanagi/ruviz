//! Radar (Spider/Star) chart implementations
//!
//! Provides radar/spider charts for multivariate data visualization.
//!
//! # Trait-Based API
//!
//! Radar charts implement the core plot traits:
//! - [`PlotConfig`] for `RadarConfig`
//! - [`PlotCompute`] for `Radar` marker struct
//! - [`PlotData`] for `RadarPlotData`
//! - [`PlotRender`] for `RadarPlotData`

use crate::core::Result;
use crate::plots::traits::{PlotArea, PlotCompute, PlotConfig, PlotData, PlotRender};
use crate::render::skia::SkiaRenderer;
use crate::render::{Color, LineStyle, MarkerStyle, Theme};
use std::f64::consts::PI;

/// Configuration for radar chart
#[derive(Debug, Clone)]
pub struct RadarConfig {
    /// Labels for each axis
    pub labels: Vec<String>,
    /// Colors for each series (None for auto)
    pub colors: Option<Vec<Color>>,
    /// Fill areas
    pub fill: bool,
    /// Fill alpha
    pub fill_alpha: f32,
    /// Line width
    pub line_width: f32,
    /// Marker size
    pub marker_size: f32,
    /// Show grid
    pub show_grid: bool,
    /// Number of grid rings
    pub grid_rings: usize,
    /// Start angle in radians
    pub start_angle: f64,
    /// Value range (min, max) - None for auto
    pub value_range: Option<(f64, f64)>,
}

impl Default for RadarConfig {
    fn default() -> Self {
        Self {
            labels: vec![],
            colors: None,
            fill: true,
            fill_alpha: 0.25,
            line_width: 2.0,
            marker_size: 4.0,
            show_grid: true,
            grid_rings: 5,
            start_angle: PI / 2.0, // Start from top
            value_range: None,
        }
    }
}

impl RadarConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set axis labels
    pub fn labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    /// Set colors
    pub fn colors(mut self, colors: Vec<Color>) -> Self {
        self.colors = Some(colors);
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

    /// Set line width
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width.max(0.1);
        self
    }

    /// Set value range
    pub fn value_range(mut self, min: f64, max: f64) -> Self {
        self.value_range = Some((min, max));
        self
    }

    /// Set start angle
    pub fn start_angle(mut self, angle: f64) -> Self {
        self.start_angle = angle;
        self
    }
}

// Implement PlotConfig marker trait
impl PlotConfig for RadarConfig {}

/// Marker struct for Radar plot type
pub struct Radar;

/// A single series in a radar chart
#[derive(Debug, Clone)]
pub struct RadarSeries {
    /// Normalized values (0.0 to 1.0)
    pub values: Vec<f64>,
    /// Polygon vertices in cartesian coordinates
    pub polygon: Vec<(f64, f64)>,
    /// Marker positions
    pub markers: Vec<(f64, f64)>,
    /// Series label
    pub label: String,
}

/// Input for radar chart computation
pub struct RadarInput<'a> {
    /// Multiple series of values \[series\]\[axis\]
    pub data: &'a [Vec<f64>],
}

impl<'a> RadarInput<'a> {
    /// Create new radar input
    pub fn new(data: &'a [Vec<f64>]) -> Self {
        Self { data }
    }
}

/// Computed radar chart data
#[derive(Debug, Clone)]
pub struct RadarPlotData {
    /// All series data
    pub series: Vec<RadarSeries>,
    /// Axis endpoints (from center outward)
    pub axes: Vec<((f64, f64), (f64, f64))>,
    /// Grid ring vertices
    pub grid_rings: Vec<Vec<(f64, f64)>>,
    /// Axis labels and positions
    pub axis_labels: Vec<(String, f64, f64)>,
    /// Value range used
    pub value_range: (f64, f64),
    /// Configuration used
    pub(crate) config: RadarConfig,
}

/// Compute radar chart data
///
/// # Arguments
/// * `data` - Multiple series of values \[series\]\[axis\]
/// * `config` - Radar chart configuration
///
/// # Returns
/// RadarPlotData for rendering
pub fn compute_radar_chart(data: &[Vec<f64>], config: &RadarConfig) -> RadarPlotData {
    if data.is_empty() {
        return RadarPlotData {
            series: vec![],
            axes: vec![],
            grid_rings: vec![],
            axis_labels: vec![],
            value_range: (0.0, 1.0),
            config: config.clone(),
        };
    }

    // Find number of axes
    let n_axes = data.iter().map(|s| s.len()).max().unwrap_or(0);
    if n_axes == 0 {
        return RadarPlotData {
            series: vec![],
            axes: vec![],
            grid_rings: vec![],
            axis_labels: vec![],
            value_range: (0.0, 1.0),
            config: config.clone(),
        };
    }

    // Determine value range
    let (v_min, v_max) = config.value_range.unwrap_or_else(|| {
        let mut min_val = f64::INFINITY;
        let mut max_val = f64::NEG_INFINITY;
        for series in data {
            for &v in series {
                min_val = min_val.min(v);
                max_val = max_val.max(v);
            }
        }
        let min_val = if min_val.is_finite() {
            min_val.min(0.0)
        } else {
            0.0
        };
        let max_val = if max_val.is_finite() {
            max_val.max(1.0)
        } else {
            1.0
        };
        (min_val, max_val)
    });

    let range = if (v_max - v_min).abs() < 1e-10 {
        1.0
    } else {
        v_max - v_min
    };

    // Calculate angles for each axis
    let angle_step = 2.0 * PI / n_axes as f64;
    let angles: Vec<f64> = (0..n_axes)
        .map(|i| config.start_angle - i as f64 * angle_step)
        .collect();

    // Generate axes
    let axes: Vec<((f64, f64), (f64, f64))> = angles
        .iter()
        .map(|&theta| ((0.0, 0.0), (theta.cos(), theta.sin())))
        .collect();

    // Generate grid rings
    let grid_rings: Vec<Vec<(f64, f64)>> = (1..=config.grid_rings)
        .map(|ring| {
            let r = ring as f64 / config.grid_rings as f64;
            angles
                .iter()
                .map(|&theta| (r * theta.cos(), r * theta.sin()))
                .collect()
        })
        .collect();

    // Generate axis labels
    let axis_labels: Vec<(String, f64, f64)> = angles
        .iter()
        .enumerate()
        .map(|(i, &theta)| {
            let label = config
                .labels
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("Axis {}", i + 1));
            let label_r = 1.1; // Slightly outside
            (label, label_r * theta.cos(), label_r * theta.sin())
        })
        .collect();

    // Generate series data
    let series: Vec<RadarSeries> = data
        .iter()
        .enumerate()
        .map(|(series_idx, values)| {
            let normalized: Vec<f64> = values.iter().map(|&v| (v - v_min) / range).collect();

            let polygon: Vec<(f64, f64)> = normalized
                .iter()
                .zip(angles.iter())
                .map(|(&r, &theta)| (r * theta.cos(), r * theta.sin()))
                .collect();

            let markers = polygon.clone();

            RadarSeries {
                values: normalized,
                polygon,
                markers,
                label: format!("Series {}", series_idx + 1),
            }
        })
        .collect();

    RadarPlotData {
        series,
        axes,
        grid_rings,
        axis_labels,
        value_range: (v_min, v_max),
        config: config.clone(),
    }
}

// ============================================================================
// Trait-Based API
// ============================================================================

impl PlotCompute for Radar {
    type Input<'a> = RadarInput<'a>;
    type Config = RadarConfig;
    type Output = RadarPlotData;

    fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output> {
        if input.data.is_empty() {
            return Err(crate::core::PlottingError::EmptyDataSet);
        }

        Ok(compute_radar_chart(input.data, config))
    }
}

impl PlotData for RadarPlotData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        // Radar plots use normalized -1 to 1 range (1.1 for labels)
        ((-1.1, 1.1), (-1.1, 1.1))
    }

    fn is_empty(&self) -> bool {
        self.series.is_empty()
    }
}

impl PlotRender for RadarPlotData {
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        theme: &Theme,
        color: Color,
    ) -> Result<()> {
        if self.series.is_empty() {
            return Ok(());
        }

        let config = &self.config;

        // Draw grid rings
        if config.show_grid {
            let grid_color = theme.grid_color;
            for ring in &self.grid_rings {
                if ring.len() < 2 {
                    continue;
                }
                // Draw closed polygon for the ring
                for i in 0..ring.len() {
                    let (x1, y1) = ring[i];
                    let (x2, y2) = ring[(i + 1) % ring.len()];
                    let (sx1, sy1) = area.data_to_screen(x1, y1);
                    let (sx2, sy2) = area.data_to_screen(x2, y2);
                    renderer.draw_line(sx1, sy1, sx2, sy2, grid_color, 0.5, LineStyle::Solid)?;
                }
            }

            // Draw axes
            for &((x1, y1), (x2, y2)) in &self.axes {
                let (sx1, sy1) = area.data_to_screen(x1, y1);
                let (sx2, sy2) = area.data_to_screen(x2, y2);
                renderer.draw_line(sx1, sy1, sx2, sy2, grid_color, 0.5, LineStyle::Solid)?;
            }
        }

        // Draw each series
        let colors = config.colors.clone().unwrap_or_else(|| {
            // Use color cycle
            vec![color]
        });

        for (series_idx, series) in self.series.iter().enumerate() {
            let series_color = colors
                .get(series_idx % colors.len())
                .copied()
                .unwrap_or(color);

            // Draw fill if enabled
            if config.fill && !series.polygon.is_empty() {
                let fill_color = series_color.with_alpha(config.fill_alpha);
                let screen_polygon: Vec<(f32, f32)> = series
                    .polygon
                    .iter()
                    .map(|(x, y)| area.data_to_screen(*x, *y))
                    .collect();
                renderer.draw_filled_polygon(&screen_polygon, fill_color)?;
            }

            // Draw lines connecting points
            if series.polygon.len() > 1 {
                let n = series.polygon.len();
                for i in 0..n {
                    let (x1, y1) = series.polygon[i];
                    let (x2, y2) = series.polygon[(i + 1) % n];
                    let (sx1, sy1) = area.data_to_screen(x1, y1);
                    let (sx2, sy2) = area.data_to_screen(x2, y2);
                    renderer.draw_line(
                        sx1,
                        sy1,
                        sx2,
                        sy2,
                        series_color,
                        config.line_width,
                        LineStyle::Solid,
                    )?;
                }
            }

            // Draw markers if configured
            if config.marker_size > 0.0 {
                for (x, y) in &series.markers {
                    let (sx, sy) = area.data_to_screen(*x, *y);
                    renderer.draw_marker(
                        sx,
                        sy,
                        config.marker_size,
                        MarkerStyle::Circle,
                        series_color,
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
    fn test_radar_basic() {
        let data = vec![vec![1.0, 2.0, 3.0, 4.0, 5.0], vec![5.0, 4.0, 3.0, 2.0, 1.0]];
        let config = RadarConfig::default();
        let plot = compute_radar_chart(&data, &config);

        assert_eq!(plot.series.len(), 2);
        assert_eq!(plot.axes.len(), 5);
        assert_eq!(plot.grid_rings.len(), 5);
    }

    #[test]
    fn test_radar_with_labels() {
        let data = vec![vec![1.0, 2.0, 3.0]];
        let config =
            RadarConfig::default().labels(vec!["A".to_string(), "B".to_string(), "C".to_string()]);
        let plot = compute_radar_chart(&data, &config);

        assert_eq!(plot.axis_labels[0].0, "A");
        assert_eq!(plot.axis_labels[1].0, "B");
        assert_eq!(plot.axis_labels[2].0, "C");
    }

    #[test]
    fn test_radar_normalization() {
        let data = vec![vec![0.0, 50.0, 100.0]];
        let config = RadarConfig::default().value_range(0.0, 100.0);
        let plot = compute_radar_chart(&data, &config);

        assert!((plot.series[0].values[0] - 0.0).abs() < 1e-10);
        assert!((plot.series[0].values[1] - 0.5).abs() < 1e-10);
        assert!((plot.series[0].values[2] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_radar_empty() {
        let data: Vec<Vec<f64>> = vec![];
        let config = RadarConfig::default();
        let plot = compute_radar_chart(&data, &config);

        assert!(plot.series.is_empty());
    }

    #[test]
    fn test_radar_config_implements_plot_config() {
        fn assert_plot_config<T: PlotConfig>() {}
        assert_plot_config::<RadarConfig>();
    }

    #[test]
    fn test_radar_compute_trait() {
        use crate::plots::traits::PlotCompute;

        let data = vec![vec![1.0, 2.0, 3.0, 4.0, 5.0]];
        let config = RadarConfig::default();
        let input = RadarInput::new(&data);
        let result = Radar::compute(input, &config);

        assert!(result.is_ok());
        let radar_data = result.unwrap();
        assert_eq!(radar_data.series.len(), 1);
        assert_eq!(radar_data.axes.len(), 5);
    }

    #[test]
    fn test_radar_compute_empty() {
        use crate::plots::traits::PlotCompute;

        let data: Vec<Vec<f64>> = vec![];
        let config = RadarConfig::default();
        let input = RadarInput::new(&data);
        let result = Radar::compute(input, &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_radar_data_trait() {
        use crate::plots::traits::{PlotCompute, PlotData};

        let data = vec![vec![1.0, 2.0, 3.0]];
        let config = RadarConfig::default();
        let input = RadarInput::new(&data);
        let radar_data = Radar::compute(input, &config).unwrap();

        // Test data_bounds
        let ((x_min, x_max), (y_min, y_max)) = radar_data.data_bounds();
        assert!((x_min - (-1.1)).abs() < 1e-10);
        assert!((x_max - 1.1).abs() < 1e-10);
        assert!((y_min - (-1.1)).abs() < 1e-10);
        assert!((y_max - 1.1).abs() < 1e-10);

        // Test is_empty
        assert!(!radar_data.is_empty());
    }
}
