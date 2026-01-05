//! Heatmap visualization for 2D array data
//!
//! Provides color-mapped grid visualization with colorbars and annotations.
//!
//! # Trait-Based API
//!
//! Heatmap plots implement the core plot traits:
//! - [`PlotConfig`] for `HeatmapConfig`
//! - [`PlotData`] for `HeatmapData`
//! - [`PlotRender`] for `HeatmapData`

use crate::core::Result as PlotResult;
use crate::core::style_utils::StyleResolver;
use crate::plots::traits::{PlotArea, PlotConfig, PlotData, PlotRender};
use crate::render::skia::SkiaRenderer;
use crate::render::{Color, ColorMap, Theme};

/// Interpolation method for heatmap rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Interpolation {
    /// Each cell is a solid rectangle with no smoothing
    #[default]
    Nearest,
    /// Colors are smoothly interpolated between cell centers
    Bilinear,
}

/// Configuration for heatmap rendering
#[derive(Debug, Clone)]
pub struct HeatmapConfig {
    /// Colormap to use for value-to-color mapping
    pub colormap: ColorMap,
    /// Minimum value for color mapping (None = auto from data)
    pub vmin: Option<f64>,
    /// Maximum value for color mapping (None = auto from data)
    pub vmax: Option<f64>,
    /// Whether to show a colorbar
    pub colorbar: bool,
    /// Label for the colorbar
    pub colorbar_label: Option<String>,
    /// Custom labels for X axis ticks
    pub xticklabels: Option<Vec<String>>,
    /// Custom labels for Y axis ticks
    pub yticklabels: Option<Vec<String>>,
    /// Interpolation method
    pub interpolation: Interpolation,
    /// Whether to annotate cells with values
    pub annotate: bool,
    /// Format string for annotations (e.g., "{:.2}")
    pub annotation_format: String,
    /// Aspect ratio (None = auto, Some(1.0) = square cells)
    pub aspect: Option<f64>,
    /// Alpha transparency for the heatmap (0.0 - 1.0)
    pub alpha: f32,
}

impl Default for HeatmapConfig {
    fn default() -> Self {
        Self {
            colormap: ColorMap::viridis(),
            vmin: None,
            vmax: None,
            colorbar: true,
            colorbar_label: None,
            xticklabels: None,
            yticklabels: None,
            interpolation: Interpolation::Nearest,
            annotate: false,
            annotation_format: "{:.2}".to_string(),
            aspect: None,
            alpha: 1.0,
        }
    }
}

impl HeatmapConfig {
    /// Create a new HeatmapConfig with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the colormap
    pub fn colormap(mut self, colormap: ColorMap) -> Self {
        self.colormap = colormap;
        self
    }

    /// Set the minimum value for color mapping
    pub fn vmin(mut self, vmin: f64) -> Self {
        self.vmin = Some(vmin);
        self
    }

    /// Set the maximum value for color mapping
    pub fn vmax(mut self, vmax: f64) -> Self {
        self.vmax = Some(vmax);
        self
    }

    /// Enable or disable colorbar
    pub fn colorbar(mut self, show: bool) -> Self {
        self.colorbar = show;
        self
    }

    /// Set the colorbar label
    pub fn colorbar_label<S: Into<String>>(mut self, label: S) -> Self {
        self.colorbar_label = Some(label.into());
        self
    }

    /// Set custom X axis tick labels
    pub fn xticklabels(mut self, labels: Vec<String>) -> Self {
        self.xticklabels = Some(labels);
        self
    }

    /// Set custom Y axis tick labels
    pub fn yticklabels(mut self, labels: Vec<String>) -> Self {
        self.yticklabels = Some(labels);
        self
    }

    /// Set interpolation method
    pub fn interpolation(mut self, method: Interpolation) -> Self {
        self.interpolation = method;
        self
    }

    /// Enable or disable cell value annotations
    pub fn annotate(mut self, show: bool) -> Self {
        self.annotate = show;
        self
    }

    /// Set the annotation format string
    pub fn annotation_format<S: Into<String>>(mut self, format: S) -> Self {
        self.annotation_format = format.into();
        self
    }

    /// Set aspect ratio (1.0 = square cells)
    pub fn aspect(mut self, ratio: f64) -> Self {
        self.aspect = Some(ratio);
        self
    }

    /// Set alpha transparency
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }
}

// Implement PlotConfig marker trait
impl PlotConfig for HeatmapConfig {}

/// Processed heatmap data ready for rendering
#[derive(Debug, Clone)]
pub struct HeatmapData {
    /// 2D array of values (row-major order)
    pub values: Vec<Vec<f64>>,
    /// Number of rows
    pub n_rows: usize,
    /// Number of columns
    pub n_cols: usize,
    /// Minimum value in data
    pub data_min: f64,
    /// Maximum value in data
    pub data_max: f64,
    /// Effective minimum for color mapping
    pub vmin: f64,
    /// Effective maximum for color mapping
    pub vmax: f64,
    /// Configuration
    pub config: HeatmapConfig,
}

impl HeatmapData {
    /// Get color for a specific cell value
    pub fn get_color(&self, value: f64) -> Color {
        let range = self.vmax - self.vmin;
        if range.abs() < f64::EPSILON {
            return self.config.colormap.sample(0.5);
        }
        let normalized = ((value - self.vmin) / range).clamp(0.0, 1.0);
        self.config.colormap.sample(normalized)
    }

    /// Get a contrasting text color for annotations
    pub fn get_text_color(&self, background: Color) -> Color {
        // Calculate relative luminance
        let luminance = 0.299 * (background.r as f64)
            + 0.587 * (background.g as f64)
            + 0.114 * (background.b as f64);
        if luminance > 128.0 {
            Color::BLACK
        } else {
            Color::WHITE
        }
    }
}

/// Process a 2D array into HeatmapData
pub fn process_heatmap(data: &[Vec<f64>], config: HeatmapConfig) -> Result<HeatmapData, String> {
    if data.is_empty() {
        return Err("Heatmap data is empty".to_string());
    }

    let n_rows = data.len();
    let n_cols = data[0].len();

    // Verify all rows have the same length
    for (i, row) in data.iter().enumerate() {
        if row.len() != n_cols {
            return Err(format!(
                "Row {} has {} columns, expected {}",
                i,
                row.len(),
                n_cols
            ));
        }
    }

    // Calculate data range
    let mut data_min = f64::INFINITY;
    let mut data_max = f64::NEG_INFINITY;

    for row in data {
        for &value in row {
            if value.is_finite() {
                data_min = data_min.min(value);
                data_max = data_max.max(value);
            }
        }
    }

    if !data_min.is_finite() || !data_max.is_finite() {
        return Err("Heatmap data contains only non-finite values".to_string());
    }

    // Use config overrides or data range
    let vmin = config.vmin.unwrap_or(data_min);
    let vmax = config.vmax.unwrap_or(data_max);

    Ok(HeatmapData {
        values: data.to_vec(),
        n_rows,
        n_cols,
        data_min,
        data_max,
        vmin,
        vmax,
        config,
    })
}

/// Process a flat array with dimensions into HeatmapData
pub fn process_heatmap_flat(
    data: &[f64],
    n_rows: usize,
    n_cols: usize,
    config: HeatmapConfig,
) -> Result<HeatmapData, String> {
    if data.len() != n_rows * n_cols {
        return Err(format!(
            "Data length {} does not match dimensions {}x{}",
            data.len(),
            n_rows,
            n_cols
        ));
    }

    // Convert to 2D array
    let values: Vec<Vec<f64>> = (0..n_rows)
        .map(|r| data[r * n_cols..(r + 1) * n_cols].to_vec())
        .collect();

    process_heatmap(&values, config)
}

// =============================================================================
// PlotData and PlotRender implementations
// =============================================================================

impl PlotData for HeatmapData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        // X bounds: 0 to n_cols
        // Y bounds: 0 to n_rows
        ((0.0, self.n_cols as f64), (0.0, self.n_rows as f64))
    }

    fn is_empty(&self) -> bool {
        self.values.is_empty() || self.values[0].is_empty()
    }
}

impl PlotRender for HeatmapData {
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        _theme: &Theme,
        _color: Color, // Heatmaps use colormap, not single color
    ) -> PlotResult<()> {
        if self.is_empty() {
            return Ok(());
        }

        let config = &self.config;
        let alpha = config.alpha;

        // Draw each cell
        for row in 0..self.n_rows {
            for col in 0..self.n_cols {
                let value = self.values[row][col];
                if !value.is_finite() {
                    continue;
                }

                // Get color from colormap
                let cell_color = self.get_color(value).with_alpha(alpha);

                // Calculate cell bounds in data coordinates
                // Y is inverted (row 0 at top)
                let x1 = col as f64;
                let x2 = (col + 1) as f64;
                let y1 = (self.n_rows - row - 1) as f64;
                let y2 = (self.n_rows - row) as f64;

                // Convert to screen coordinates
                let (sx1, sy1) = area.data_to_screen(x1, y2);
                let (sx2, sy2) = area.data_to_screen(x2, y1);

                // Draw the cell
                renderer.draw_rectangle(sx1, sy1, sx2 - sx1, sy2 - sy1, cell_color, true)?;
            }
        }

        Ok(())
    }

    fn render_styled(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        theme: &Theme,
        _color: Color,
        alpha: f32,
        _line_width: Option<f32>,
    ) -> PlotResult<()> {
        if self.is_empty() {
            return Ok(());
        }

        let config = &self.config;
        let _resolver = StyleResolver::new(theme);

        // Use provided alpha or config alpha
        let effective_alpha = if alpha != 1.0 { alpha } else { config.alpha };

        // Draw each cell
        for row in 0..self.n_rows {
            for col in 0..self.n_cols {
                let value = self.values[row][col];
                if !value.is_finite() {
                    continue;
                }

                // Get color from colormap
                let cell_color = self.get_color(value).with_alpha(effective_alpha);

                // Calculate cell bounds in data coordinates
                // Y is inverted (row 0 at top)
                let x1 = col as f64;
                let x2 = (col + 1) as f64;
                let y1 = (self.n_rows - row - 1) as f64;
                let y2 = (self.n_rows - row) as f64;

                // Convert to screen coordinates
                let (sx1, sy1) = area.data_to_screen(x1, y2);
                let (sx2, sy2) = area.data_to_screen(x2, y1);

                // Draw the cell
                renderer.draw_rectangle(sx1, sy1, sx2 - sx1, sy2 - sy1, cell_color, true)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heatmap_config_defaults() {
        let config = HeatmapConfig::default();
        assert!(config.colorbar);
        assert!(!config.annotate);
        assert_eq!(config.interpolation, Interpolation::Nearest);
        assert!(config.vmin.is_none());
        assert!(config.vmax.is_none());
    }

    #[test]
    fn test_heatmap_config_builder() {
        let config = HeatmapConfig::new()
            .colormap(ColorMap::plasma())
            .vmin(0.0)
            .vmax(100.0)
            .colorbar(true)
            .colorbar_label("Temperature")
            .annotate(true);

        assert_eq!(config.vmin, Some(0.0));
        assert_eq!(config.vmax, Some(100.0));
        assert!(config.colorbar);
        assert_eq!(config.colorbar_label, Some("Temperature".to_string()));
        assert!(config.annotate);
    }

    #[test]
    fn test_process_heatmap() {
        let data = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
        let config = HeatmapConfig::default();
        let result = process_heatmap(&data, config).unwrap();

        assert_eq!(result.n_rows, 2);
        assert_eq!(result.n_cols, 3);
        assert!((result.data_min - 1.0).abs() < f64::EPSILON);
        assert!((result.data_max - 6.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_process_heatmap_with_vmin_vmax() {
        let data = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let config = HeatmapConfig::new().vmin(0.0).vmax(10.0);
        let result = process_heatmap(&data, config).unwrap();

        assert!((result.vmin - 0.0).abs() < f64::EPSILON);
        assert!((result.vmax - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_process_heatmap_empty() {
        let data: Vec<Vec<f64>> = vec![];
        let config = HeatmapConfig::default();
        assert!(process_heatmap(&data, config).is_err());
    }

    #[test]
    fn test_process_heatmap_jagged() {
        let data = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0]]; // Jagged array
        let config = HeatmapConfig::default();
        assert!(process_heatmap(&data, config).is_err());
    }

    #[test]
    fn test_heatmap_get_color() {
        let data = vec![vec![0.0, 1.0]];
        let config = HeatmapConfig::new().vmin(0.0).vmax(1.0);
        let heatmap = process_heatmap(&data, config).unwrap();

        // At vmin, should get first color of colormap
        let color_min = heatmap.get_color(0.0);
        // At vmax, should get last color of colormap
        let color_max = heatmap.get_color(1.0);

        // Colors should be different
        assert!(color_min != color_max);
    }

    #[test]
    fn test_get_text_color() {
        let data = vec![vec![0.0, 1.0]];
        let config = HeatmapConfig::default();
        let heatmap = process_heatmap(&data, config).unwrap();

        // Dark background should get white text
        let white_text = heatmap.get_text_color(Color::BLACK);
        assert_eq!(white_text, Color::WHITE);

        // Light background should get black text
        let black_text = heatmap.get_text_color(Color::WHITE);
        assert_eq!(black_text, Color::BLACK);
    }

    #[test]
    fn test_process_heatmap_flat() {
        let flat_data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let config = HeatmapConfig::default();
        let result = process_heatmap_flat(&flat_data, 2, 3, config).unwrap();

        assert_eq!(result.n_rows, 2);
        assert_eq!(result.n_cols, 3);
        assert_eq!(result.values[0], vec![1.0, 2.0, 3.0]);
        assert_eq!(result.values[1], vec![4.0, 5.0, 6.0]);
    }

    #[test]
    fn test_interpolation_enum() {
        assert_eq!(Interpolation::default(), Interpolation::Nearest);
    }

    #[test]
    fn test_heatmap_config_implements_plot_config() {
        fn assert_plot_config<T: PlotConfig>() {}
        assert_plot_config::<HeatmapConfig>();
    }

    #[test]
    fn test_heatmap_plot_data_trait() {
        let data = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
        let config = HeatmapConfig::default();
        let heatmap = process_heatmap(&data, config).unwrap();

        // Test data_bounds
        let ((x_min, x_max), (y_min, y_max)) = heatmap.data_bounds();
        assert!((x_min - 0.0).abs() < 0.001);
        assert!((x_max - 3.0).abs() < 0.001);
        assert!((y_min - 0.0).abs() < 0.001);
        assert!((y_max - 2.0).abs() < 0.001);

        // Test is_empty
        assert!(!heatmap.is_empty());
    }
}
