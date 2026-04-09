//! Heatmap visualization for 2D array data
//!
//! Provides color-mapped grid visualization with colorbars and annotations.
//!
//! # Colorbar Configuration
//!
//! Heatmaps support configurable colorbar labels and font sizes:
//!
//! ```rust,ignore
//! use ruviz::prelude::{AxisScale, HeatmapConfig};
//!
//! let config = HeatmapConfig::default()
//!     .value_scale(AxisScale::Log)
//!     .colorbar(true)
//!     .colorbar_log_subticks(true)
//!     .cell_borders(false)
//!     .colorbar_label("Temperature (°C)")
//!     .colorbar_tick_font_size(10.0)    // Tick label size
//!     .colorbar_label_font_size(11.0);  // Axis label size
//! ```
//!
//! When using `AxisScale::Log`, the effective `vmin`/`vmax` range must remain
//! strictly positive.
//!
//! # Trait-Based API
//!
//! Heatmap plots implement the core plot traits:
//! - [`PlotConfig`] for `HeatmapConfig`
//! - [`PlotData`] for `HeatmapData`
//! - [`PlotRender`] for `HeatmapData`

use crate::axes::AxisScale;
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
    /// Value scale for color mapping and colorbar ticks
    pub value_scale: AxisScale,
    /// Whether to show a colorbar
    pub colorbar: bool,
    /// Label for the colorbar
    pub colorbar_label: Option<String>,
    /// Font size for colorbar tick labels (in points)
    pub colorbar_tick_font_size: f32,
    /// Font size for colorbar label (in points)
    pub colorbar_label_font_size: f32,
    /// Whether logarithmic colorbars draw minor subticks
    pub colorbar_log_subticks: bool,
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
    /// Whether heatmap cells should draw visible borders
    pub cell_borders: bool,
    /// Whether SymLog heatmaps should derive linthresh from the smallest positive finite value
    pub symlog_auto_linthresh: bool,
    /// Physical extent for the heatmap grid as (xmin, xmax, ymin, ymax)
    pub extent: Option<(f64, f64, f64, f64)>,
}

impl Default for HeatmapConfig {
    fn default() -> Self {
        Self {
            colormap: ColorMap::viridis(),
            vmin: None,
            vmax: None,
            value_scale: AxisScale::Linear,
            colorbar: true,
            colorbar_label: None,
            colorbar_tick_font_size: 12.0, // Readable colorbar tick labels
            colorbar_label_font_size: 14.0, // Larger for visibility
            colorbar_log_subticks: true,
            xticklabels: None,
            yticklabels: None,
            interpolation: Interpolation::Nearest,
            annotate: false,
            annotation_format: "{:.2}".to_string(),
            aspect: None,
            alpha: 1.0,
            cell_borders: false,
            symlog_auto_linthresh: false,
            extent: None,
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

    /// Set the value scale used for color normalization and colorbar ticks.
    ///
    /// `AxisScale::Log` requires the effective `vmin` and `vmax` range to be
    /// strictly positive.
    pub fn value_scale(mut self, scale: AxisScale) -> Self {
        self.value_scale = scale;
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

    /// Set the colorbar tick font size (in points)
    ///
    /// Default is 10.0pt to match axis tick labels.
    pub fn colorbar_tick_font_size(mut self, size: f32) -> Self {
        self.colorbar_tick_font_size = size.max(1.0);
        self
    }

    /// Set the colorbar label font size (in points)
    ///
    /// Default is 11.0pt, slightly larger than tick labels.
    pub fn colorbar_label_font_size(mut self, size: f32) -> Self {
        self.colorbar_label_font_size = size.max(1.0);
        self
    }

    /// Enable or disable logarithmic colorbar subticks.
    ///
    /// This only affects `AxisScale::Log` colorbars.
    pub fn colorbar_log_subticks(mut self, show: bool) -> Self {
        self.colorbar_log_subticks = show;
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

    /// Enable or disable visible cell borders.
    ///
    /// Borders are disabled by default so heatmaps render as continuous tiles.
    pub fn cell_borders(mut self, enabled: bool) -> Self {
        self.cell_borders = enabled;
        self
    }

    /// Derive `SymLog` linthresh from the smallest positive finite heatmap value.
    ///
    /// When enabled, the configured `AxisScale::SymLog { .. }` linthresh is
    /// replaced during heatmap processing.
    pub fn symlog_auto_linthresh(mut self, enabled: bool) -> Self {
        self.symlog_auto_linthresh = enabled;
        self
    }

    /// Set the physical extent covered by the heatmap grid.
    ///
    /// The extent is interpreted as `(xmin, xmax, ymin, ymax)` and must be
    /// strictly increasing on both axes. Use `xlim`/`ylim` separately when you
    /// want to reverse the visible axis direction.
    pub fn extent(mut self, xmin: f64, xmax: f64, ymin: f64, ymax: f64) -> Self {
        self.extent = Some((xmin, xmax, ymin, ymax));
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
    /// Physical x extent covered by the heatmap grid
    pub x_extent: (f64, f64),
    /// Physical y extent covered by the heatmap grid
    pub y_extent: (f64, f64),
    /// Configuration
    pub config: HeatmapConfig,
}

impl HeatmapData {
    fn can_use_pixel_aligned_grid_fast_path(&self, alpha: f32) -> bool {
        matches!(self.config.interpolation, Interpolation::Nearest) && alpha >= 1.0
    }

    fn x_step(&self) -> f64 {
        (self.x_extent.1 - self.x_extent.0) / self.n_cols.max(1) as f64
    }

    fn y_step(&self) -> f64 {
        (self.y_extent.1 - self.y_extent.0) / self.n_rows.max(1) as f64
    }

    pub(crate) fn cell_data_bounds(&self, row: usize, col: usize) -> ((f64, f64), (f64, f64)) {
        let dx = self.x_step();
        let dy = self.y_step();
        let x1 = self.x_extent.0 + col as f64 * dx;
        let x2 = self.x_extent.0 + (col + 1) as f64 * dx;
        // Row 0 remains at the visual top of the heatmap, so it occupies the
        // highest y interval within the configured extent.
        let y_top = self.y_extent.1 - row as f64 * dy;
        let y_bottom = self.y_extent.1 - (row + 1) as f64 * dy;
        ((x1, x2), (y_bottom, y_top))
    }

    pub(crate) fn cell_screen_rect(
        &self,
        area: &PlotArea,
        row: usize,
        col: usize,
    ) -> (f32, f32, f32, f32) {
        let ((x1, x2), (y1, y2)) = self.cell_data_bounds(row, col);
        let (sx1, sy1) = area.data_to_screen(x1, y2);
        let (sx2, sy2) = area.data_to_screen(x2, y1);
        let x = sx1.min(sx2);
        let y = sy1.min(sy2);
        let width = (sx2 - sx1).abs();
        let height = (sy2 - sy1).abs();
        (x, y, width, height)
    }

    fn normalized_value(&self, value: f64) -> f64 {
        self.config
            .value_scale
            .normalized_position(value, self.vmin, self.vmax)
    }

    pub fn should_mask_value(&self, value: f64) -> bool {
        if !value.is_finite() {
            return true;
        }

        matches!(self.config.value_scale, AxisScale::Log) && value <= 0.0
    }

    /// Get color for a specific cell value
    pub fn get_color(&self, value: f64) -> Color {
        let normalized = self.normalized_value(value).clamp(0.0, 1.0);
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

    fn pixel_aligned_screen_edges(&self, area: &PlotArea) -> (Vec<i32>, Vec<i32>) {
        let x_min = area.x;
        let x_max = area.x + area.width;
        let y_min = area.y;
        let y_max = area.y + area.height;
        let x_step = self.x_step();
        let y_step = self.y_step();

        let x_edges = (0..=self.n_cols)
            .map(|index| {
                let x = self.x_extent.0 + index as f64 * x_step;
                area.data_to_screen(x, self.y_extent.0)
                    .0
                    .clamp(x_min, x_max)
                    .round() as i32
            })
            .collect();
        let y_edges = (0..=self.n_rows)
            .map(|index| {
                let y = self.y_extent.1 - index as f64 * y_step;
                area.data_to_screen(self.x_extent.0, y)
                    .1
                    .clamp(y_min, y_max)
                    .round() as i32
            })
            .collect();

        (x_edges, y_edges)
    }

    fn draw_cells_pixel_aligned_grid(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        alpha: f32,
    ) -> PlotResult<()> {
        let (x_edges, y_edges) = self.pixel_aligned_screen_edges(area);

        for row in 0..self.n_rows {
            let top = y_edges[row].min(y_edges[row + 1]);
            let bottom = y_edges[row].max(y_edges[row + 1]);
            if bottom <= top {
                continue;
            }

            for col in 0..self.n_cols {
                let value = self.values[row][col];
                if self.should_mask_value(value) {
                    continue;
                }

                let left = x_edges[col].min(x_edges[col + 1]);
                let right = x_edges[col].max(x_edges[col + 1]);
                if right <= left {
                    continue;
                }

                let cell_color = self.get_color(value).with_alpha(alpha);
                let x = left as f32;
                let y = top as f32;
                let width = (right - left) as f32;
                let height = (bottom - top) as f32;

                renderer.draw_pixel_aligned_solid_rectangle(x, y, width, height, cell_color)?;

                if self.config.cell_borders {
                    renderer.draw_pixel_aligned_rectangle_outline(
                        x,
                        y,
                        width,
                        height,
                        cell_color.darken(0.2),
                    )?;
                }
            }
        }

        Ok(())
    }

    fn draw_cells_legacy(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        alpha: f32,
    ) -> PlotResult<()> {
        for row in 0..self.n_rows {
            for col in 0..self.n_cols {
                let value = self.values[row][col];
                if self.should_mask_value(value) {
                    continue;
                }

                let cell_color = self.get_color(value).with_alpha(alpha);

                let (x, y, width, height) = self.cell_screen_rect(area, row, col);
                let left = x.max(area.x);
                let top = y.max(area.y);
                let right = (x + width).min(area.x + area.width);
                let bottom = (y + height).min(area.y + area.height);
                if right <= left || bottom <= top {
                    continue;
                }
                let width = right - left;
                let height = bottom - top;

                renderer
                    .draw_pixel_aligned_solid_rectangle(left, top, width, height, cell_color)?;

                if self.config.cell_borders {
                    renderer.draw_pixel_aligned_rectangle_outline(
                        left,
                        top,
                        width,
                        height,
                        cell_color.darken(0.2),
                    )?;
                }
            }
        }

        Ok(())
    }

    fn draw_cells(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        alpha: f32,
    ) -> PlotResult<()> {
        if self.can_use_pixel_aligned_grid_fast_path(alpha) {
            return self.draw_cells_pixel_aligned_grid(renderer, area, alpha);
        }

        self.draw_cells_legacy(renderer, area, alpha)
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

    let mut config = config;
    let (x_extent, y_extent) = match config.extent {
        Some((xmin, xmax, ymin, ymax)) => {
            if !xmin.is_finite() || !xmax.is_finite() || !ymin.is_finite() || !ymax.is_finite() {
                return Err("Heatmap extent must contain only finite values".to_string());
            }
            if xmax <= xmin || ymax <= ymin {
                return Err("Heatmap extent must satisfy xmin < xmax and ymin < ymax".to_string());
            }
            ((xmin, xmax), (ymin, ymax))
        }
        None => ((0.0, n_cols as f64), (0.0, n_rows as f64)),
    };

    // Calculate data range
    let mut data_min = f64::INFINITY;
    let mut data_max = f64::NEG_INFINITY;
    let mut positive_min = f64::INFINITY;
    let mut positive_max = f64::NEG_INFINITY;

    for row in data {
        for &value in row {
            if value.is_finite() {
                data_min = data_min.min(value);
                data_max = data_max.max(value);
                if value > 0.0 {
                    positive_min = positive_min.min(value);
                    positive_max = positive_max.max(value);
                }
            }
        }
    }

    if !data_min.is_finite() || !data_max.is_finite() {
        return Err("Heatmap data contains only non-finite values".to_string());
    }

    if config.symlog_auto_linthresh
        && let AxisScale::SymLog { .. } = config.value_scale
    {
        if !positive_min.is_finite() {
            return Err(
                "SymLog auto linthresh requires at least one positive finite value.".to_string(),
            );
        }
        config.value_scale = AxisScale::SymLog {
            linthresh: positive_min,
        };
    }

    // Use config overrides or data range
    let (vmin, vmax) = match config.value_scale {
        AxisScale::Log => {
            let vmin = if let Some(vmin) = config.vmin {
                vmin
            } else if positive_min.is_finite() {
                positive_min
            } else {
                return Err(
                    "Logarithmic heatmaps require at least one positive finite value.".to_string(),
                );
            };
            let vmax = if let Some(vmax) = config.vmax {
                vmax
            } else if positive_max.is_finite() {
                positive_max
            } else {
                return Err(
                    "Logarithmic heatmaps require at least one positive finite value.".to_string(),
                );
            };
            (vmin, vmax)
        }
        _ => (
            config.vmin.unwrap_or(data_min),
            config.vmax.unwrap_or(data_max),
        ),
    };
    config.value_scale.validate_range(vmin, vmax)?;

    Ok(HeatmapData {
        values: data.to_vec(),
        n_rows,
        n_cols,
        data_min,
        data_max,
        vmin,
        vmax,
        x_extent,
        y_extent,
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
        (
            (
                self.x_extent.0.min(self.x_extent.1),
                self.x_extent.0.max(self.x_extent.1),
            ),
            (
                self.y_extent.0.min(self.y_extent.1),
                self.y_extent.0.max(self.y_extent.1),
            ),
        )
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
        self.draw_cells(renderer, area, config.alpha)
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

        self.draw_cells(renderer, area, effective_alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::plot::Image;

    fn mean_normalized_rgba_diff(reference: &Image, candidate: &Image) -> f64 {
        assert_eq!(reference.width, candidate.width);
        assert_eq!(reference.height, candidate.height);

        reference
            .pixels
            .iter()
            .zip(candidate.pixels.iter())
            .map(|(lhs, rhs)| (*lhs as f64 - *rhs as f64).abs() / 255.0)
            .sum::<f64>()
            / reference.pixels.len() as f64
    }

    fn fraction_pixels_within_channel_delta(
        reference: &Image,
        candidate: &Image,
        max_delta: u8,
    ) -> f64 {
        assert_eq!(reference.width, candidate.width);
        assert_eq!(reference.height, candidate.height);

        let mut matching = 0usize;
        for (lhs, rhs) in reference
            .pixels
            .chunks_exact(4)
            .zip(candidate.pixels.chunks_exact(4))
        {
            if lhs
                .iter()
                .zip(rhs.iter())
                .all(|(left, right)| (*left as i16 - *right as i16).abs() <= max_delta as i16)
            {
                matching += 1;
            }
        }
        matching as f64 / (reference.width * reference.height) as f64
    }

    fn non_background_fraction(image: &Image) -> f64 {
        let total = image.pixels.chunks_exact(4).len() as f64;
        let ink = image
            .pixels
            .chunks_exact(4)
            .filter(|pixel| pixel[3] > 0 && (pixel[0] < 248 || pixel[1] < 248 || pixel[2] < 248))
            .count() as f64;
        ink / total.max(1.0)
    }

    fn assert_heatmap_parity(reference: &Image, candidate: &Image) {
        let mean_diff = mean_normalized_rgba_diff(reference, candidate);
        let within_delta = fraction_pixels_within_channel_delta(reference, candidate, 24);
        let reference_ink = non_background_fraction(reference);
        let candidate_ink = non_background_fraction(candidate);

        assert!(
            mean_diff <= 0.015,
            "heatmap drifted too far from legacy cells: mean_diff={mean_diff:.6}"
        );
        assert!(
            within_delta >= 0.99,
            "heatmap has too many per-pixel outliers relative to legacy cells: within_delta={within_delta:.4}"
        );
        assert!(
            (reference_ink - candidate_ink).abs() <= 0.10,
            "heatmap changed visible ink coverage too much: reference_ink={reference_ink:.4} candidate_ink={candidate_ink:.4}"
        );
    }

    fn render_heatmap_cells(
        data: &HeatmapData,
        area: &PlotArea,
        use_legacy: bool,
    ) -> crate::core::Result<Image> {
        let mut renderer = SkiaRenderer::new(120, 120, Theme::default())?;
        if use_legacy {
            data.draw_cells_legacy(&mut renderer, area, data.config.alpha)?;
        } else {
            data.draw_cells(&mut renderer, area, data.config.alpha)?;
        }
        Ok(renderer.into_image())
    }

    #[test]
    fn test_heatmap_config_defaults() {
        let config = HeatmapConfig::default();
        assert!(config.colorbar);
        assert!(!config.annotate);
        assert_eq!(config.interpolation, Interpolation::Nearest);
        assert!(config.vmin.is_none());
        assert!(config.vmax.is_none());
        assert_eq!(config.value_scale, AxisScale::Linear);
        assert!(config.colorbar_log_subticks);
        assert!(!config.cell_borders);
        assert!(!config.symlog_auto_linthresh);
        assert!(config.extent.is_none());
    }

    #[test]
    fn test_opaque_nearest_heatmap_fast_path_stays_in_parity_with_legacy_cells() {
        let rows = 48usize;
        let cols = 256usize;
        let stripe_start = cols / 2 - 4;
        let stripe_end = stripe_start + 8;
        let mut values = vec![vec![0.0; cols]; rows];
        for row in &mut values {
            for cell in &mut row[stripe_start..stripe_end] {
                *cell = 1.0;
            }
        }

        let data = process_heatmap(&values, HeatmapConfig::new().colorbar(false))
            .expect("heatmap data should process");
        let area = PlotArea::new(8.0, 10.0, 90.0, 92.0, 0.0, cols as f64, 0.0, rows as f64);

        let reference = render_heatmap_cells(&data, &area, true).expect("legacy heatmap render");
        let candidate = render_heatmap_cells(&data, &area, false).expect("fast heatmap render");

        assert_heatmap_parity(&reference, &candidate);
    }

    #[test]
    fn test_translucent_heatmap_keeps_legacy_cell_renderer() {
        let values = vec![
            vec![0.1, 0.4, 0.7],
            vec![0.2, 0.5, 0.8],
            vec![0.3, 0.6, 0.9],
        ];
        let data = process_heatmap(&values, HeatmapConfig::new().colorbar(false).alpha(0.5))
            .expect("heatmap data should process");
        let area = PlotArea::new(8.0, 10.0, 90.0, 92.0, 0.0, 3.0, 0.0, 3.0);

        let reference = render_heatmap_cells(&data, &area, true).expect("legacy heatmap render");
        let candidate =
            render_heatmap_cells(&data, &area, false).expect("translucent heatmap render");

        assert_eq!(reference.pixels, candidate.pixels);
    }

    #[test]
    fn test_heatmap_config_builder() {
        let config = HeatmapConfig::new()
            .colormap(ColorMap::plasma())
            .vmin(0.0)
            .vmax(100.0)
            .value_scale(AxisScale::Log)
            .colorbar(true)
            .colorbar_label("Temperature")
            .colorbar_log_subticks(false)
            .cell_borders(true)
            .symlog_auto_linthresh(true)
            .extent(0.0, 3.0, 0.0, 2.0)
            .annotate(true);

        assert_eq!(config.vmin, Some(0.0));
        assert_eq!(config.vmax, Some(100.0));
        assert_eq!(config.value_scale, AxisScale::Log);
        assert!(config.colorbar);
        assert_eq!(config.colorbar_label, Some("Temperature".to_string()));
        assert!(!config.colorbar_log_subticks);
        assert!(config.cell_borders);
        assert!(config.symlog_auto_linthresh);
        assert_eq!(config.extent, Some((0.0, 3.0, 0.0, 2.0)));
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
    fn test_process_heatmap_uses_custom_extent() {
        let data = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let config = HeatmapConfig::new().extent(10.0, 14.0, 20.0, 24.0);
        let result = process_heatmap(&data, config).unwrap();

        assert_eq!(result.x_extent, (10.0, 14.0));
        assert_eq!(result.y_extent, (20.0, 24.0));
    }

    #[test]
    fn test_process_heatmap_rejects_invalid_extent() {
        let data = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let config = HeatmapConfig::new().extent(1.0, 1.0, 0.0, 2.0);
        assert!(process_heatmap(&data, config).is_err());
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
    fn test_heatmap_get_color_uses_log_value_scale() {
        let data = vec![vec![1.0, 10.0, 100.0]];
        let config = HeatmapConfig::new()
            .vmin(1.0)
            .vmax(100.0)
            .value_scale(AxisScale::Log);
        let heatmap = process_heatmap(&data, config).unwrap();

        let log_mid = heatmap.get_color(10.0);
        let expected_mid = heatmap.config.colormap.sample(0.5);
        assert_eq!(log_mid, expected_mid);
    }

    #[test]
    fn test_process_heatmap_log_scale_ignores_nonpositive_cells_for_auto_range() {
        let data = vec![vec![0.0, 1.0], vec![10.0, 100.0]];
        let config = HeatmapConfig::new().value_scale(AxisScale::Log);
        let result = process_heatmap(&data, config).unwrap();

        assert_eq!(result.vmin, 1.0);
        assert_eq!(result.vmax, 100.0);
    }

    #[test]
    fn test_process_heatmap_rejects_invalid_explicit_log_bounds() {
        let data = vec![vec![0.0, 1.0], vec![10.0, 100.0]];
        let config = HeatmapConfig::new().value_scale(AxisScale::Log).vmin(0.0);
        assert!(process_heatmap(&data, config).is_err());
    }

    #[test]
    fn test_process_heatmap_log_scale_rejects_missing_positive_values() {
        let data = vec![vec![0.0, -1.0], vec![f64::NEG_INFINITY, f64::NAN]];
        let config = HeatmapConfig::new().value_scale(AxisScale::Log);
        assert!(process_heatmap(&data, config).is_err());
    }

    #[test]
    fn test_process_heatmap_symlog_auto_linthresh_uses_smallest_positive_value() {
        let data = vec![vec![0.0, 0.01], vec![1.0, 10.0]];
        let config = HeatmapConfig::new()
            .value_scale(AxisScale::symlog(1.0))
            .symlog_auto_linthresh(true);
        let result = process_heatmap(&data, config).unwrap();

        assert_eq!(
            result.config.value_scale,
            AxisScale::SymLog { linthresh: 0.01 }
        );
    }

    #[test]
    fn test_process_heatmap_symlog_auto_linthresh_rejects_missing_positive_values() {
        let data = vec![vec![0.0, -1.0], vec![-10.0, f64::NAN]];
        let config = HeatmapConfig::new()
            .value_scale(AxisScale::symlog(1.0))
            .symlog_auto_linthresh(true);

        assert!(process_heatmap(&data, config).is_err());
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

    #[test]
    fn test_heatmap_plot_data_trait_uses_extent_bounds() {
        let data = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let config = HeatmapConfig::new().extent(0.0, 8.0, 0.0, 4.0);
        let heatmap = process_heatmap(&data, config).unwrap();

        let ((x_min, x_max), (y_min, y_max)) = heatmap.data_bounds();
        assert_eq!((x_min, x_max), (0.0, 8.0));
        assert_eq!((y_min, y_max), (0.0, 4.0));
    }
}
