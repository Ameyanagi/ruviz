//! Hexbin plot implementations
//!
//! Provides hexagonal binning for visualizing 2D point density.
//!
//! # Trait-Based API
//!
//! Hexbin plots implement the core plot traits:
//! - [`PlotConfig`] for `HexbinConfig`
//! - [`PlotCompute`] for `Hexbin` marker struct
//! - [`PlotData`] for `HexbinPlotData`
//! - [`PlotRender`] for `HexbinPlotData`

use crate::core::Result;
use crate::plots::traits::{PlotArea, PlotCompute, PlotConfig, PlotData, PlotRender};
use crate::render::skia::SkiaRenderer;
use crate::render::{Color, ColorMap, LineStyle, Theme};
use std::collections::HashMap;

/// Configuration for hexbin plot
#[derive(Debug, Clone)]
pub struct HexbinConfig {
    /// Grid size (number of hexagons across x-axis)
    pub gridsize: usize,
    /// Colormap name
    pub cmap: String,
    /// Aggregation function
    pub reduce_fn: ReduceFunction,
    /// Minimum count to show hexagon (None = show all)
    pub mincnt: Option<usize>,
    /// Maximum count for color scaling (None = auto)
    pub maxcnt: Option<usize>,
    /// Edge color for hexagons
    pub edge_color: Option<Color>,
    /// Edge width
    pub edge_width: f32,
    /// Alpha for fill
    pub alpha: f32,
    /// Logarithmic color scale
    pub log_scale: bool,
}

/// Aggregation function for hexbin values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReduceFunction {
    /// Count points in each bin
    Count,
    /// Mean of values
    Mean,
    /// Sum of values
    Sum,
    /// Minimum value
    Min,
    /// Maximum value
    Max,
    /// Standard deviation
    Std,
}

impl Default for HexbinConfig {
    fn default() -> Self {
        Self {
            gridsize: 30,
            cmap: "viridis".to_string(),
            reduce_fn: ReduceFunction::Count,
            mincnt: None,
            maxcnt: None,
            edge_color: None,
            edge_width: 0.0,
            alpha: 1.0,
            log_scale: false,
        }
    }
}

impl HexbinConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set grid size
    pub fn gridsize(mut self, size: usize) -> Self {
        self.gridsize = size.max(5);
        self
    }

    /// Set colormap
    pub fn cmap(mut self, cmap: &str) -> Self {
        self.cmap = cmap.to_string();
        self
    }

    /// Set reduce function
    pub fn reduce_fn(mut self, reduce: ReduceFunction) -> Self {
        self.reduce_fn = reduce;
        self
    }

    /// Set minimum count threshold
    pub fn mincnt(mut self, cnt: usize) -> Self {
        self.mincnt = Some(cnt);
        self
    }

    /// Set edge color
    pub fn edge_color(mut self, color: Color) -> Self {
        self.edge_color = Some(color);
        self
    }

    /// Set alpha
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Enable log scale
    pub fn log_scale(mut self, log: bool) -> Self {
        self.log_scale = log;
        self
    }
}

// Implement PlotConfig marker trait
impl PlotConfig for HexbinConfig {}

/// Marker struct for Hexbin plot type
pub struct Hexbin;

/// A single hexagonal bin
#[derive(Debug, Clone)]
pub struct HexBin {
    /// Center x coordinate
    pub cx: f64,
    /// Center y coordinate
    pub cy: f64,
    /// Aggregated value (count, mean, etc.)
    pub value: f64,
    /// Number of points in this bin
    pub count: usize,
    /// Hexagon vertices
    pub vertices: [(f64, f64); 6],
}

impl HexBin {
    /// Create hexagon vertices for flat-top orientation
    pub fn compute_vertices(cx: f64, cy: f64, size: f64) -> [(f64, f64); 6] {
        let mut vertices = [(0.0, 0.0); 6];
        for (i, vertex) in vertices.iter_mut().enumerate() {
            let angle = std::f64::consts::PI / 3.0 * i as f64;
            *vertex = (cx + size * angle.cos(), cy + size * angle.sin());
        }
        vertices
    }
}

/// Computed hexbin data for plotting
#[derive(Debug, Clone)]
pub struct HexbinPlotData {
    /// All hexagonal bins with values
    pub bins: Vec<HexBin>,
    /// Hexagon size (radius)
    pub hex_size: f64,
    /// Value range (min, max)
    pub value_range: (f64, f64),
    /// Data bounds
    pub bounds: ((f64, f64), (f64, f64)),
    /// Configuration used
    pub(crate) config: HexbinConfig,
}

/// Input for hexbin plot computation
pub struct HexbinInput<'a> {
    /// X coordinates
    pub x: &'a [f64],
    /// Y coordinates
    pub y: &'a [f64],
    /// Optional values for aggregation
    pub values: Option<&'a [f64]>,
}

impl<'a> HexbinInput<'a> {
    /// Create new hexbin input
    pub fn new(x: &'a [f64], y: &'a [f64]) -> Self {
        Self { x, y, values: None }
    }

    /// Create hexbin input with values
    pub fn with_values(x: &'a [f64], y: &'a [f64], values: &'a [f64]) -> Self {
        Self {
            x,
            y,
            values: Some(values),
        }
    }
}

/// Compute hexagonal index from point
fn hex_index(x: f64, y: f64, size: f64) -> (i64, i64) {
    // Axial coordinates for flat-top hexagons
    let q = (2.0 / 3.0 * x) / size;
    let r = (-1.0 / 3.0 * x + 3.0_f64.sqrt() / 3.0 * y) / size;

    // Round to nearest hex
    hex_round(q, r)
}

fn hex_round(q: f64, r: f64) -> (i64, i64) {
    let s = -q - r;

    let mut rq = q.round();
    let mut rr = r.round();
    let rs = s.round();

    let q_diff = (rq - q).abs();
    let r_diff = (rr - r).abs();
    let s_diff = (rs - s).abs();

    if q_diff > r_diff && q_diff > s_diff {
        rq = -rr - rs;
    } else if r_diff > s_diff {
        rr = -rq - rs;
    }

    (rq as i64, rr as i64)
}

/// Convert axial hex coordinates to center point
fn hex_to_center(q: i64, r: i64, size: f64) -> (f64, f64) {
    let x = size * (3.0 / 2.0 * q as f64);
    let y = size * (3.0_f64.sqrt() / 2.0 * q as f64 + 3.0_f64.sqrt() * r as f64);
    (x, y)
}

/// Compute hexbin data from points
///
/// # Arguments
/// * `x` - X coordinates
/// * `y` - Y coordinates
/// * `values` - Optional values for aggregation (None for count)
/// * `config` - Hexbin configuration
///
/// # Returns
/// HexbinPlotData for rendering
pub fn compute_hexbin(
    x: &[f64],
    y: &[f64],
    values: Option<&[f64]>,
    config: &HexbinConfig,
) -> HexbinPlotData {
    if x.is_empty() || y.is_empty() {
        return HexbinPlotData {
            bins: vec![],
            hex_size: 1.0,
            value_range: (0.0, 1.0),
            bounds: ((0.0, 1.0), (0.0, 1.0)),
            config: config.clone(),
        };
    }

    let n = x.len().min(y.len());

    // Find data bounds
    let x_min = x.iter().copied().fold(f64::INFINITY, f64::min);
    let x_max = x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let y_min = y.iter().copied().fold(f64::INFINITY, f64::min);
    let y_max = y.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    // Calculate hex size based on gridsize
    let x_range = x_max - x_min;
    let hex_size = x_range / (config.gridsize as f64 * 1.5);

    // Bin points
    let mut bin_data: HashMap<(i64, i64), Vec<f64>> = HashMap::new();

    for i in 0..n {
        let (q, r) = hex_index(x[i] - x_min, y[i] - y_min, hex_size);
        let val = values.map_or(1.0, |v| v.get(i).copied().unwrap_or(1.0));
        bin_data.entry((q, r)).or_default().push(val);
    }

    // Aggregate and create hexbins
    let mut bins = Vec::new();
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;

    for ((q, r), vals) in bin_data {
        let count = vals.len();

        // Apply mincnt filter
        if let Some(min) = config.mincnt {
            if count < min {
                continue;
            }
        }

        let value = match config.reduce_fn {
            ReduceFunction::Count => count as f64,
            ReduceFunction::Mean => vals.iter().sum::<f64>() / count as f64,
            ReduceFunction::Sum => vals.iter().sum(),
            ReduceFunction::Min => vals.iter().copied().fold(f64::INFINITY, f64::min),
            ReduceFunction::Max => vals.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            ReduceFunction::Std => {
                let mean = vals.iter().sum::<f64>() / count as f64;
                let variance = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / count as f64;
                variance.sqrt()
            }
        };

        min_value = min_value.min(value);
        max_value = max_value.max(value);

        let (cx, cy) = hex_to_center(q, r, hex_size);
        let vertices = HexBin::compute_vertices(cx + x_min, cy + y_min, hex_size);

        bins.push(HexBin {
            cx: cx + x_min,
            cy: cy + y_min,
            value,
            count,
            vertices,
        });
    }

    // Apply maxcnt
    if let Some(max) = config.maxcnt {
        max_value = max_value.min(max as f64);
    }

    // Apply log scale
    if config.log_scale && min_value > 0.0 {
        for bin in &mut bins {
            bin.value = bin.value.ln();
        }
        min_value = min_value.ln();
        max_value = max_value.ln();
    }

    HexbinPlotData {
        bins,
        hex_size,
        value_range: (min_value, max_value),
        bounds: ((x_min, x_max), (y_min, y_max)),
        config: config.clone(),
    }
}

/// Compute data range for hexbin plot
pub fn hexbin_range(data: &HexbinPlotData) -> ((f64, f64), (f64, f64)) {
    data.bounds
}

// ============================================================================
// Trait-Based API
// ============================================================================

impl PlotCompute for Hexbin {
    type Input<'a> = HexbinInput<'a>;
    type Config = HexbinConfig;
    type Output = HexbinPlotData;

    fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output> {
        if input.x.is_empty() || input.y.is_empty() {
            return Err(crate::core::PlottingError::EmptyDataSet);
        }

        Ok(compute_hexbin(input.x, input.y, input.values, config))
    }
}

impl PlotData for HexbinPlotData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        self.bounds
    }

    fn is_empty(&self) -> bool {
        self.bins.is_empty()
    }
}

impl PlotRender for HexbinPlotData {
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        _theme: &Theme,
        _color: Color,
    ) -> Result<()> {
        if self.bins.is_empty() {
            return Ok(());
        }

        let config = &self.config;
        let (min_value, max_value) = self.value_range;
        let value_range = max_value - min_value;

        // Get colormap
        let cmap = ColorMap::by_name(&config.cmap).unwrap_or_else(ColorMap::viridis);

        for bin in &self.bins {
            // Calculate color based on value
            let t = if value_range > 0.0 {
                (bin.value - min_value) / value_range
            } else {
                0.5
            };
            let fill_color = cmap.sample(t).with_alpha(config.alpha);

            // Convert vertices to screen coordinates
            let screen_vertices: Vec<(f32, f32)> = bin
                .vertices
                .iter()
                .map(|(x, y)| area.data_to_screen(*x, *y))
                .collect();

            // Draw filled hexagon
            renderer.draw_filled_polygon(&screen_vertices, fill_color)?;

            // Draw edge if configured
            if let Some(edge_color) = config.edge_color {
                if config.edge_width > 0.0 {
                    // Draw hexagon outline
                    for i in 0..6 {
                        let (x1, y1) = screen_vertices[i];
                        let (x2, y2) = screen_vertices[(i + 1) % 6];
                        renderer.draw_line(
                            x1,
                            y1,
                            x2,
                            y2,
                            edge_color,
                            config.edge_width,
                            LineStyle::Solid,
                        )?;
                    }
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
    fn test_hexbin_basic() {
        let x: Vec<f64> = (0..100).map(|i| (i as f64) / 10.0).collect();
        let y: Vec<f64> = (0..100).map(|i| ((i as f64) / 10.0).sin()).collect();
        let config = HexbinConfig::default().gridsize(10);
        let data = compute_hexbin(&x, &y, None, &config);

        assert!(!data.bins.is_empty());
        // All bins should have count >= 1
        for bin in &data.bins {
            assert!(bin.count >= 1);
        }
    }

    #[test]
    fn test_hexbin_with_values() {
        let x = vec![0.0, 0.1, 0.2, 1.0, 1.1, 1.2];
        let y = vec![0.0, 0.1, 0.2, 0.0, 0.1, 0.2];
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let config = HexbinConfig::default()
            .gridsize(5)
            .reduce_fn(ReduceFunction::Mean);
        let data = compute_hexbin(&x, &y, Some(&values), &config);

        assert!(!data.bins.is_empty());
    }

    #[test]
    fn test_hexbin_mincnt() {
        let x = vec![0.0, 0.0, 0.0, 10.0];
        let y = vec![0.0, 0.0, 0.0, 10.0];
        let config = HexbinConfig::default().gridsize(5).mincnt(2);
        let data = compute_hexbin(&x, &y, None, &config);

        // Single point at (10, 10) should be filtered out
        for bin in &data.bins {
            assert!(bin.count >= 2);
        }
    }

    #[test]
    fn test_hex_vertices() {
        let vertices = HexBin::compute_vertices(0.0, 0.0, 1.0);
        assert_eq!(vertices.len(), 6);

        // All vertices should be at distance 1 from center
        for (x, y) in vertices {
            let dist = (x * x + y * y).sqrt();
            assert!((dist - 1.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_hexbin_empty() {
        let x: Vec<f64> = vec![];
        let y: Vec<f64> = vec![];
        let config = HexbinConfig::default();
        let data = compute_hexbin(&x, &y, None, &config);

        assert!(data.bins.is_empty());
    }

    #[test]
    fn test_hexbin_config_implements_plot_config() {
        fn assert_plot_config<T: PlotConfig>() {}
        assert_plot_config::<HexbinConfig>();
    }

    #[test]
    fn test_hexbin_plot_compute_trait() {
        use crate::plots::traits::PlotCompute;

        let x: Vec<f64> = (0..100).map(|i| (i as f64) / 10.0).collect();
        let y: Vec<f64> = (0..100).map(|i| ((i as f64) / 10.0).sin()).collect();
        let config = HexbinConfig::default().gridsize(10);
        let input = HexbinInput::new(&x, &y);
        let result = Hexbin::compute(input, &config);

        assert!(result.is_ok());
        let hexbin_data = result.unwrap();
        assert!(!hexbin_data.bins.is_empty());
    }

    #[test]
    fn test_hexbin_plot_compute_empty() {
        use crate::plots::traits::PlotCompute;

        let x: Vec<f64> = vec![];
        let y: Vec<f64> = vec![];
        let config = HexbinConfig::default();
        let input = HexbinInput::new(&x, &y);
        let result = Hexbin::compute(input, &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_hexbin_plot_data_trait() {
        use crate::plots::traits::{PlotCompute, PlotData};

        let x: Vec<f64> = (0..50).map(|i| (i as f64) / 5.0).collect();
        let y: Vec<f64> = (0..50).map(|i| ((i as f64) / 5.0).cos()).collect();
        let config = HexbinConfig::default().gridsize(5);
        let input = HexbinInput::new(&x, &y);
        let hexbin_data = Hexbin::compute(input, &config).unwrap();

        // Test data_bounds
        let ((x_min, x_max), (y_min, y_max)) = hexbin_data.data_bounds();
        assert!(x_min <= x_max);
        assert!(y_min <= y_max);

        // Test is_empty
        assert!(!hexbin_data.is_empty());
    }

    #[test]
    fn test_hexbin_plot_compute_with_values() {
        use crate::plots::traits::PlotCompute;

        let x = vec![0.0, 0.1, 0.2, 1.0, 1.1, 1.2];
        let y = vec![0.0, 0.1, 0.2, 0.0, 0.1, 0.2];
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let config = HexbinConfig::default()
            .gridsize(5)
            .reduce_fn(ReduceFunction::Mean);
        let input = HexbinInput::with_values(&x, &y, &values);
        let result = Hexbin::compute(input, &config);

        assert!(result.is_ok());
        let hexbin_data = result.unwrap();
        assert!(!hexbin_data.bins.is_empty());
    }
}
