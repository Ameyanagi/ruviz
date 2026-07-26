//! Contour plot implementations
//!
//! Provides contour and filled contour visualization for 2D scalar fields.
//!
//! # Trait-Based API
//!
//! Contour plots implement the core plot traits:
//! - [`PlotConfig`] for `ContourConfig`
//! - [`PlotCompute`] for `Contour` marker struct
//! - [`PlotData`] for `ContourPlotData`
//! - [`PlotRender`] for `ContourPlotData`

use crate::core::Result;
use crate::core::style_utils::StyleResolver;
use crate::plots::traits::{PlotArea, PlotCompute, PlotConfig, PlotData, PlotRender};
use crate::render::skia::SkiaRenderer;
use crate::render::{Color, ColorMap, ColorMapSpec, LineStyle, Theme};
use crate::stats::contour::{ContourLevel, auto_levels, contour_lines};

/// Interpolation method for smoothing contour data
///
/// `ContourInterpolation::default()` is [`ContourInterpolation::Linear`], the
/// same value [`ContourConfig`]`::default()` uses — every config enum in this
/// crate defaults to whatever its owning config defaults to.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum ContourInterpolation {
    /// No interpolation - use raw grid values
    Nearest,
    /// Bilinear interpolation for smoother transitions (default)
    #[default]
    Linear,
    /// Bicubic spline interpolation for smoothest appearance
    Cubic,
}

/// Configuration for contour plot
#[derive(Debug, Clone)]
pub struct ContourConfig {
    /// Contour levels (None for auto)
    pub levels: Option<Vec<f64>>,
    /// Number of auto levels
    pub n_levels: usize,
    /// Fill between contours
    pub filled: bool,
    /// Show contour lines
    pub show_lines: bool,
    /// Line width
    pub line_width: f32,
    /// Line color for single color mode
    pub line_color: Option<Color>,
    /// Stroke each contour line with the colormap colour of its level instead of
    /// the theme foreground. Off by default: a colormap-sampled stroke is only
    /// legible when the background happens to contrast with it.
    pub color_lines_by_level: bool,
    /// Colormap name
    pub cmap: String,
    /// Show labels on contours
    pub show_labels: bool,
    /// Label font size
    pub label_fontsize: f32,
    /// Alpha for filled contours
    pub alpha: f32,
    /// Interpolation method for smoothing
    pub interpolation: ContourInterpolation,
    /// Interpolation factor (grid upsampling multiplier, e.g., 4 = 4x resolution)
    pub interpolation_factor: usize,
    /// Whether to show a colorbar
    pub colorbar: bool,
    /// Label for the colorbar
    pub colorbar_label: Option<String>,
    /// Font size for colorbar tick labels (in points)
    pub colorbar_tick_font_size: f32,
    /// Font size for colorbar label (in points)
    pub colorbar_label_font_size: f32,
}

impl Default for ContourConfig {
    fn default() -> Self {
        Self {
            levels: None,
            n_levels: 10,
            filled: true,
            show_lines: true,
            line_width: 1.0,
            line_color: None,
            color_lines_by_level: false,
            cmap: "viridis".to_string(),
            show_labels: false,
            label_fontsize: 10.0,
            alpha: 1.0,
            // Apply 2x bilinear interpolation by default for smoother contours
            // This eliminates blocky appearance without significant performance cost
            interpolation: ContourInterpolation::Linear,
            interpolation_factor: 2,
            // Anything carrying a colour scale gets a colorbar by default, so
            // filled contours read the same way heatmaps do.
            colorbar: true,
            colorbar_label: None,
            colorbar_tick_font_size: 10.0,
            colorbar_label_font_size: 11.0,
        }
    }
}

impl ContourConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set explicit levels
    pub fn levels(mut self, levels: Vec<f64>) -> Self {
        self.levels = Some(levels);
        self
    }

    /// Set number of auto levels
    pub fn n_levels(mut self, n: usize) -> Self {
        self.n_levels = n.max(2);
        self
    }

    /// Enable filled contours
    pub fn filled(mut self, filled: bool) -> Self {
        self.filled = filled;
        self
    }

    /// Show contour lines
    pub fn show_lines(mut self, show: bool) -> Self {
        self.show_lines = show;
        self
    }

    /// Set line width
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width.max(0.1);
        self
    }

    /// Set line color
    pub fn line_color(mut self, color: Color) -> Self {
        self.line_color = Some(color);
        self
    }

    /// Stroke each contour line with its level's colormap colour.
    ///
    /// Off by default, because a colormap-sampled stroke is only legible when the
    /// background happens to contrast with it: on a filled contour each line
    /// matches the band beneath it, and on a dark theme the dark end of a
    /// sequential map vanishes into the background. Enable it when the level
    /// encoding matters more than guaranteed contrast.
    pub fn color_lines_by_level(mut self, enabled: bool) -> Self {
        self.color_lines_by_level = enabled;
        self
    }

    /// Set colormap.
    ///
    /// Accepts a name such as `"viridis"` or a [`ColorMap`] value.
    pub fn cmap(mut self, cmap: impl Into<ColorMapSpec>) -> Self {
        self.cmap = cmap.into().into_name();
        self
    }

    /// Show contour labels
    pub fn show_labels(mut self, show: bool) -> Self {
        self.show_labels = show;
        self
    }

    /// Set alpha
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Set interpolation method for smoothing
    ///
    /// # Arguments
    /// * `method` - Interpolation method (Nearest, Linear, or Cubic)
    ///
    /// # Example
    /// ```rust,ignore
    /// use ruviz::plots::ContourConfig;
    /// use ruviz::plots::ContourInterpolation;
    ///
    /// let config = ContourConfig::new()
    ///     .interpolation(ContourInterpolation::Linear)
    ///     .interpolation_factor(4);
    /// ```
    pub fn interpolation(mut self, method: ContourInterpolation) -> Self {
        self.interpolation = method;
        self
    }

    /// Set interpolation factor (grid upsampling multiplier)
    ///
    /// Higher values produce smoother contours but increase computation.
    /// Recommended values: 2-8.
    ///
    /// # Arguments
    /// * `factor` - Upsampling factor (1 = no upsampling, 4 = 4x resolution)
    pub fn interpolation_factor(mut self, factor: usize) -> Self {
        self.interpolation_factor = factor.max(1);
        self
    }

    /// Enable or disable colorbar
    ///
    /// When enabled, a colorbar showing the value-to-color mapping is displayed
    /// to the right of the contour plot.
    pub fn colorbar(mut self, show: bool) -> Self {
        self.colorbar = show;
        self
    }

    /// Set the colorbar label
    ///
    /// The label is displayed rotated 90° next to the colorbar.
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
}

// Implement PlotConfig marker trait
impl PlotConfig for ContourConfig {}

/// Marker struct for Contour plot type
pub struct Contour;

/// Computed contour data for plotting
#[derive(Debug, Clone)]
pub struct ContourPlotData {
    /// Contour levels used
    pub levels: Vec<f64>,
    /// Contour lines for each level
    pub lines: Vec<ContourLevel>,
    /// X grid coordinates
    pub x: Vec<f64>,
    /// Y grid coordinates
    pub y: Vec<f64>,
    /// Z values (row-major, same shape as grid)
    pub z: Vec<f64>,
    /// Grid dimensions (nx, ny)
    pub shape: (usize, usize),
    /// Configuration used
    pub(crate) config: ContourConfig,
}

/// Input for contour plot computation
pub struct ContourInput<'a> {
    /// X grid coordinates
    pub x: &'a [f64],
    /// Y grid coordinates
    pub y: &'a [f64],
    /// Z values (row-major, len = x.len() * y.len())
    pub z: &'a [f64],
}

impl<'a> ContourInput<'a> {
    /// Create new contour input
    pub fn new(x: &'a [f64], y: &'a [f64], z: &'a [f64]) -> Self {
        Self { x, y, z }
    }
}

/// Compute contour data from a 2D grid
///
/// # Arguments
/// * `x` - X grid coordinates
/// * `y` - Y grid coordinates
/// * `z` - Z values (row-major, len = x.len() * y.len())
/// * `config` - Contour configuration
///
/// # Returns
/// ContourPlotData for rendering
pub fn compute_contour_plot(
    x: &[f64],
    y: &[f64],
    z: &[f64],
    config: &ContourConfig,
) -> ContourPlotData {
    let nx = x.len();
    let ny = y.len();

    if nx == 0 || ny == 0 || z.len() != nx * ny {
        return ContourPlotData {
            levels: vec![],
            lines: vec![],
            x: vec![],
            y: vec![],
            z: vec![],
            shape: (0, 0),
            config: config.clone(),
        };
    }

    // Apply interpolation if configured
    let (interp_x, interp_y, interp_z, interp_nx, interp_ny) =
        if config.interpolation != ContourInterpolation::Nearest && config.interpolation_factor > 1
        {
            interpolate_grid(x, y, z, config.interpolation, config.interpolation_factor)
        } else {
            (x.to_vec(), y.to_vec(), z.to_vec(), nx, ny)
        };

    // Convert flat array to 2D for contour functions
    let z_2d: Vec<Vec<f64>> = (0..interp_ny)
        .map(|iy| interp_z[iy * interp_nx..(iy + 1) * interp_nx].to_vec())
        .collect();

    // Determine levels
    let levels = config
        .levels
        .clone()
        .unwrap_or_else(|| auto_levels(&z_2d, config.n_levels));

    // Compute contour lines for all levels at once
    let lines = contour_lines(&interp_x, &interp_y, &z_2d, &levels);

    ContourPlotData {
        levels,
        lines,
        x: interp_x,
        y: interp_y,
        z: interp_z,
        shape: (interp_nx, interp_ny),
        config: config.clone(),
    }
}

/// Does a cell average belong to the half-open band `[level_low, level_high)`?
///
/// The topmost band is open-ended (`level_high` is `+inf`) and is treated as
/// closed on the right so that even an infinite cell average is claimed. NaN
/// averages belong to no band and stay unpainted, like matplotlib's masked
/// values.
fn cell_in_band(z_avg: f64, level_low: f64, level_high: f64) -> bool {
    if z_avg.is_nan() {
        return false;
    }

    z_avg >= level_low && (z_avg < level_high || level_high.is_infinite())
}

/// Colormap position for a filled band, in `[0, 1]`
///
/// Interior bands use their midpoint normalized over the level span. The two
/// open-ended bands produced by [`contour_fill_regions`] map to the colormap
/// floor and ceiling, so the fill keeps reading as one continuous ramp instead
/// of introducing extra colors at the extremes.
fn band_color_position(level_low: f64, level_high: f64, z_min: f64, z_max: f64) -> f64 {
    if !level_low.is_finite() {
        return 0.0;
    }
    if !level_high.is_finite() {
        return 1.0;
    }

    let z_range = z_max - z_min;
    if z_range > 0.0 {
        (((level_low + level_high) / 2.0 - z_min) / z_range).clamp(0.0, 1.0)
    } else {
        0.5
    }
}

/// Get filled contour regions between levels
///
/// Returns polygons for each region as `(level_low, level_high, polygons)`.
///
/// Besides the bands between consecutive levels, the first and last entries are
/// open-ended (`level_low` is `-inf`, `level_high` is `+inf`), mirroring
/// matplotlib's `extend="both"`. Without them every cell below the lowest level
/// or above the highest one - i.e. every local minimum and maximum - would match
/// no band and be left unpainted, punching holes into the fill. Together the
/// bands cover the whole value axis, so each cell with a non-NaN average is
/// claimed by exactly one band.
#[allow(clippy::type_complexity)]
pub fn contour_fill_regions(data: &ContourPlotData) -> Vec<(f64, f64, Vec<Vec<(f64, f64)>>)> {
    // For filled contours, we need regions between each pair of levels
    // This is complex - typically done by tracing filled regions
    // For now, return a simplified version using grid cells

    let mut regions = Vec::new();

    if data.levels.is_empty() || data.x.len() < 2 || data.y.len() < 2 {
        return regions;
    }

    let nx = data.x.len();
    let ny = data.y.len();

    // Band bounds: (-inf, l0), [l0, l1), ... , [l_last, +inf)
    // A single level still yields the two open-ended bands.
    let mut bands: Vec<(f64, f64)> = Vec::with_capacity(data.levels.len() + 1);
    bands.push((f64::NEG_INFINITY, data.levels[0]));
    for pair in data.levels.windows(2) {
        bands.push((pair[0], pair[1]));
    }
    bands.push((data.levels[data.levels.len() - 1], f64::INFINITY));

    for (level_low, level_high) in bands {
        let mut polygons = Vec::new();

        // Create filled cells for this level range
        for iy in 0..ny - 1 {
            for ix in 0..nx - 1 {
                let z00 = data.z[iy * nx + ix];
                let z10 = data.z[iy * nx + ix + 1];
                let z01 = data.z[(iy + 1) * nx + ix];
                let z11 = data.z[(iy + 1) * nx + ix + 1];

                let z_avg = (z00 + z10 + z01 + z11) / 4.0;

                if cell_in_band(z_avg, level_low, level_high) {
                    // This cell is in this level range
                    let x0 = data.x[ix];
                    let x1 = data.x[ix + 1];
                    let y0 = data.y[iy];
                    let y1 = data.y[iy + 1];

                    polygons.push(vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]);
                }
            }
        }

        if !polygons.is_empty() {
            regions.push((level_low, level_high, polygons));
        }
    }

    regions
}

fn axis_aligned_rectangle_bounds(polygon: &[(f64, f64)]) -> Option<(f64, f64, f64, f64)> {
    if polygon.len() != 4 {
        return None;
    }

    let min_x = polygon
        .iter()
        .map(|(x, _)| *x)
        .fold(f64::INFINITY, f64::min);
    let max_x = polygon
        .iter()
        .map(|(x, _)| *x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = polygon
        .iter()
        .map(|(_, y)| *y)
        .fold(f64::INFINITY, f64::min);
    let max_y = polygon
        .iter()
        .map(|(_, y)| *y)
        .fold(f64::NEG_INFINITY, f64::max);
    let corners = [
        (min_x, min_y),
        (max_x, min_y),
        (max_x, max_y),
        (min_x, max_y),
    ];

    if corners
        .iter()
        .all(|corner| polygon.iter().any(|point| point == corner))
    {
        Some((min_x, min_y, max_x, max_y))
    } else {
        None
    }
}

fn draw_filled_contour_region(
    renderer: &mut SkiaRenderer,
    area: &PlotArea,
    polygon: &[(f64, f64)],
    fill_color: Color,
) -> Result<()> {
    if let Some((min_x, min_y, max_x, max_y)) = axis_aligned_rectangle_bounds(polygon) {
        let (sx1, sy1) = area.data_to_screen(min_x, max_y);
        let (sx2, sy2) = area.data_to_screen(max_x, min_y);
        renderer.draw_pixel_aligned_solid_rectangle(
            sx1.min(sx2),
            sy1.min(sy2),
            (sx2 - sx1).abs(),
            (sy2 - sy1).abs(),
            fill_color,
        )?;
    } else {
        let screen_polygon: Vec<(f32, f32)> = polygon
            .iter()
            .map(|(x, y)| area.data_to_screen(*x, *y))
            .collect();
        renderer.draw_filled_polygon(&screen_polygon, fill_color)?;
    }

    Ok(())
}

/// Resolve the stroke colour of one contour line.
///
/// Precedence:
///
/// 1. An explicit [`ContourConfig::line_color`] always wins.
/// 2. [`ContourConfig::color_lines_by_level`] opts into colormap-by-level
///    strokes; with a single level there is no gradient to sample, so it falls
///    back to the series colour.
/// 3. Otherwise the line strokes in `theme.foreground`.
///
/// `theme.foreground` is the default because it is the only choice that stays
/// legible on every theme. Colormap-sampled strokes fail twice over: on a filled
/// contour each line lands in almost exactly the colour of the band underneath
/// it, and on an unfilled contour the dark end of a sequential map (viridis'
/// purple, say) is indistinguishable from a dark theme's background.
pub(crate) fn contour_line_color(
    config: &ContourConfig,
    theme: &Theme,
    cmap: &ColorMap,
    series_color: Color,
    level_index: usize,
    n_levels: usize,
) -> Color {
    if let Some(color) = config.line_color {
        return color;
    }
    if config.color_lines_by_level {
        return if n_levels > 1 {
            cmap.sample(level_index as f64 / (n_levels - 1) as f64)
        } else {
            series_color
        };
    }
    theme.foreground
}

/// Compute data range for contour plot
pub fn contour_range(x: &[f64], y: &[f64]) -> ((f64, f64), (f64, f64)) {
    if x.is_empty() || y.is_empty() {
        return ((0.0, 1.0), (0.0, 1.0));
    }

    let x_min = x.iter().copied().fold(f64::INFINITY, f64::min);
    let x_max = x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let y_min = y.iter().copied().fold(f64::INFINITY, f64::min);
    let y_max = y.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    ((x_min, x_max), (y_min, y_max))
}

// ============================================================================
// Trait-Based API
// ============================================================================

impl PlotCompute for Contour {
    type Input<'a> = ContourInput<'a>;
    type Config = ContourConfig;
    type Output = ContourPlotData;

    fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output> {
        let nx = input.x.len();
        let ny = input.y.len();

        if nx == 0 || ny == 0 {
            return Err(crate::core::PlottingError::EmptyDataSet);
        }

        if input.z.len() != nx * ny {
            return Err(crate::core::PlottingError::InvalidInput(format!(
                "Z array length {} does not match grid size {} x {} = {}",
                input.z.len(),
                nx,
                ny,
                nx * ny
            )));
        }

        Ok(compute_contour_plot(input.x, input.y, input.z, config))
    }
}

impl PlotData for ContourPlotData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        contour_range(&self.x, &self.y)
    }

    fn is_empty(&self) -> bool {
        self.levels.is_empty() || self.x.is_empty() || self.y.is_empty()
    }
}

impl PlotRender for ContourPlotData {
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        theme: &Theme,
        color: Color,
    ) -> Result<()> {
        if self.is_empty() {
            return Ok(());
        }

        let config = &self.config;
        let n_levels = self.levels.len();
        let line_width_px = renderer.render_scale().points_to_pixels(config.line_width);

        // Get colormap for level coloring
        let cmap = ColorMap::by_name(&config.cmap).unwrap_or_else(ColorMap::viridis);

        // Draw filled regions if enabled
        if config.filled {
            let regions = contour_fill_regions(self);
            for (level_low, level_high, polygons) in regions {
                // Calculate color based on level position
                let z_min = self.levels.first().copied().unwrap_or(0.0);
                let z_max = self.levels.last().copied().unwrap_or(1.0);
                let t = band_color_position(level_low, level_high, z_min, z_max);

                let fill_color = cmap.sample(t).with_alpha(config.alpha);

                for polygon in &polygons {
                    draw_filled_contour_region(renderer, area, polygon, fill_color)?;
                }
            }
        }

        // Draw contour lines if enabled
        if config.show_lines {
            for (i, level) in self.lines.iter().enumerate() {
                let line_color = contour_line_color(config, theme, &cmap, color, i, n_levels);

                // Draw each contour segment (each segment is (x1, y1, x2, y2))
                for &(x1, y1, x2, y2) in &level.segments {
                    let (sx1, sy1) = area.data_to_screen(x1, y1);
                    let (sx2, sy2) = area.data_to_screen(x2, y2);
                    renderer.draw_line(
                        sx1,
                        sy1,
                        sx2,
                        sy2,
                        line_color,
                        line_width_px,
                        LineStyle::Solid,
                    )?;
                }
            }
        }

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

        let config = &self.config;
        let resolver = StyleResolver::new(theme);
        let n_levels = self.levels.len();

        // Get colormap for level coloring
        let cmap = ColorMap::by_name(&config.cmap).unwrap_or_else(ColorMap::viridis);

        let effective_alpha = config.alpha * alpha.clamp(0.0, 1.0);
        let effective_line_width = renderer.render_scale().points_to_pixels(
            line_width.unwrap_or_else(|| resolver.line_width(Some(config.line_width))),
        );

        // Draw filled regions if enabled
        if config.filled {
            let regions = contour_fill_regions(self);
            for (level_low, level_high, polygons) in regions {
                // Calculate color based on level position
                let z_min = self.levels.first().copied().unwrap_or(0.0);
                let z_max = self.levels.last().copied().unwrap_or(1.0);
                let t = band_color_position(level_low, level_high, z_min, z_max);

                let fill_color = cmap.sample(t).with_alpha(effective_alpha);

                for polygon in &polygons {
                    draw_filled_contour_region(renderer, area, polygon, fill_color)?;
                }
            }
        }

        // Draw contour lines if enabled
        if config.show_lines {
            for (i, level) in self.lines.iter().enumerate() {
                let line_color = contour_line_color(config, theme, &cmap, color, i, n_levels);
                let line_color =
                    line_color.with_alpha((f32::from(line_color.a) / 255.0) * effective_alpha);

                // Draw each contour segment (each segment is (x1, y1, x2, y2))
                for &(x1, y1, x2, y2) in &level.segments {
                    let (sx1, sy1) = area.data_to_screen(x1, y1);
                    let (sx2, sy2) = area.data_to_screen(x2, y2);
                    renderer.draw_line(
                        sx1,
                        sy1,
                        sx2,
                        sy2,
                        line_color,
                        effective_line_width,
                        LineStyle::Solid,
                    )?;
                }
            }
        }

        Ok(())
    }
}

// =============================================================================
// Grid Interpolation Functions
// =============================================================================

/// Interpolate a 2D grid to higher resolution
///
/// # Arguments
/// * `x` - Original X coordinates
/// * `y` - Original Y coordinates
/// * `z` - Original Z values (row-major, len = x.len() * y.len())
/// * `method` - Interpolation method
/// * `factor` - Upsampling factor (2 = 2x resolution, 4 = 4x resolution)
///
/// # Returns
/// (new_x, new_y, new_z, new_nx, new_ny)
fn interpolate_grid(
    x: &[f64],
    y: &[f64],
    z: &[f64],
    method: ContourInterpolation,
    factor: usize,
) -> (Vec<f64>, Vec<f64>, Vec<f64>, usize, usize) {
    let nx = x.len();
    let ny = y.len();

    if nx < 2 || ny < 2 || factor < 2 {
        return (x.to_vec(), y.to_vec(), z.to_vec(), nx, ny);
    }

    // Create new coordinate arrays with higher resolution
    let new_nx = (nx - 1) * factor + 1;
    let new_ny = (ny - 1) * factor + 1;

    let x_min = x[0];
    let x_max = x[nx - 1];
    let y_min = y[0];
    let y_max = y[ny - 1];

    let new_x: Vec<f64> = (0..new_nx)
        .map(|i| x_min + (x_max - x_min) * (i as f64) / ((new_nx - 1) as f64))
        .collect();

    let new_y: Vec<f64> = (0..new_ny)
        .map(|i| y_min + (y_max - y_min) * (i as f64) / ((new_ny - 1) as f64))
        .collect();

    // Interpolate Z values
    let mut new_z = vec![0.0; new_nx * new_ny];

    for iy in 0..new_ny {
        for ix in 0..new_nx {
            let fx = (new_x[ix] - x_min) / (x_max - x_min) * ((nx - 1) as f64);
            let fy = (new_y[iy] - y_min) / (y_max - y_min) * ((ny - 1) as f64);

            new_z[iy * new_nx + ix] = match method {
                ContourInterpolation::Nearest => {
                    // Nearest neighbor - shouldn't reach here due to check above
                    let ix_src = fx.round() as usize;
                    let iy_src = fy.round() as usize;
                    z[iy_src.min(ny - 1) * nx + ix_src.min(nx - 1)]
                }
                ContourInterpolation::Linear => bilinear_interpolate(z, nx, ny, fx, fy),
                ContourInterpolation::Cubic => bicubic_interpolate(z, nx, ny, fx, fy),
            };
        }
    }

    (new_x, new_y, new_z, new_nx, new_ny)
}

/// Bilinear interpolation at fractional coordinates
fn bilinear_interpolate(z: &[f64], nx: usize, ny: usize, fx: f64, fy: f64) -> f64 {
    let x0 = (fx.floor() as usize).min(nx - 2);
    let y0 = (fy.floor() as usize).min(ny - 2);
    let x1 = x0 + 1;
    let y1 = y0 + 1;

    let dx = fx - x0 as f64;
    let dy = fy - y0 as f64;

    let z00 = z[y0 * nx + x0];
    let z10 = z[y0 * nx + x1];
    let z01 = z[y1 * nx + x0];
    let z11 = z[y1 * nx + x1];

    // Bilinear interpolation formula
    z00 * (1.0 - dx) * (1.0 - dy) + z10 * dx * (1.0 - dy) + z01 * (1.0 - dx) * dy + z11 * dx * dy
}

/// Bicubic interpolation at fractional coordinates
///
/// Uses Catmull-Rom spline for smooth interpolation
fn bicubic_interpolate(z: &[f64], nx: usize, ny: usize, fx: f64, fy: f64) -> f64 {
    let x1 = (fx.floor() as isize).clamp(0, nx as isize - 1) as usize;
    let y1 = (fy.floor() as isize).clamp(0, ny as isize - 1) as usize;

    let dx = fx - x1 as f64;
    let dy = fy - y1 as f64;

    // Get 4x4 neighborhood with clamped boundary handling
    let get_z = |ix: isize, iy: isize| -> f64 {
        let cix = ix.clamp(0, nx as isize - 1) as usize;
        let ciy = iy.clamp(0, ny as isize - 1) as usize;
        z[ciy * nx + cix]
    };

    let x1i = x1 as isize;
    let y1i = y1 as isize;

    // Interpolate in Y direction first for each of 4 X columns
    let mut col_values = [0.0; 4];
    for (i, col_val) in col_values.iter_mut().enumerate() {
        let xi = x1i - 1 + i as isize;
        *col_val = cubic_interp(
            get_z(xi, y1i - 1),
            get_z(xi, y1i),
            get_z(xi, y1i + 1),
            get_z(xi, y1i + 2),
            dy,
        );
    }

    // Then interpolate in X direction
    cubic_interp(
        col_values[0],
        col_values[1],
        col_values[2],
        col_values[3],
        dx,
    )
}

/// Catmull-Rom cubic interpolation
fn cubic_interp(p0: f64, p1: f64, p2: f64, p3: f64, t: f64) -> f64 {
    let t2 = t * t;
    let t3 = t2 * t;

    // Catmull-Rom spline coefficients
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A config enum and the config that owns it must agree on `default()`,
    /// otherwise `ContourInterpolation::default()` silently means something
    /// different from "the interpolation you get if you never call
    /// `.interpolation(..)`".
    #[test]
    fn interpolation_default_matches_config_default() {
        assert_eq!(
            ContourInterpolation::default(),
            ContourConfig::default().interpolation
        );
        assert_eq!(
            ContourInterpolation::default(),
            ContourInterpolation::Linear
        );
    }

    fn make_test_grid() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let y: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let mut z = Vec::with_capacity(100);

        for iy in 0..10 {
            for ix in 0..10 {
                let xi = ix as f64 - 4.5;
                let yi = iy as f64 - 4.5;
                z.push((-(xi * xi + yi * yi) / 10.0).exp());
            }
        }

        (x, y, z)
    }

    #[test]
    fn test_contour_plot_basic() {
        let (x, y, z) = make_test_grid();
        // Disable interpolation for this test to get exact grid dimensions
        let config = ContourConfig::default().n_levels(5).interpolation_factor(1);
        let data = compute_contour_plot(&x, &y, &z, &config);

        assert_eq!(data.shape, (10, 10));
        assert!(!data.levels.is_empty());
    }

    #[test]
    fn test_contour_explicit_levels() {
        let (x, y, z) = make_test_grid();
        let levels = vec![0.1, 0.3, 0.5, 0.7, 0.9];
        let config = ContourConfig::default().levels(levels.clone());
        let data = compute_contour_plot(&x, &y, &z, &config);

        assert_eq!(data.levels, levels);
    }

    #[test]
    fn test_contour_range() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![0.0, 1.0, 2.0, 3.0];
        let ((x_min, x_max), (y_min, y_max)) = contour_range(&x, &y);

        assert!((x_min - 0.0).abs() < 1e-10);
        assert!((x_max - 2.0).abs() < 1e-10);
        assert!((y_min - 0.0).abs() < 1e-10);
        assert!((y_max - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_contour_fill_regions() {
        let (x, y, z) = make_test_grid();
        let config = ContourConfig::default().n_levels(3).filled(true);
        let data = compute_contour_plot(&x, &y, &z, &config);
        let regions = contour_fill_regions(&data);

        // Should have regions between levels
        assert!(!regions.is_empty() || data.levels.len() < 2);
    }

    #[test]
    fn test_contour_empty() {
        let x: Vec<f64> = vec![];
        let y: Vec<f64> = vec![];
        let z: Vec<f64> = vec![];
        let config = ContourConfig::default();
        let data = compute_contour_plot(&x, &y, &z, &config);

        assert!(data.levels.is_empty());
        assert_eq!(data.shape, (0, 0));
    }

    /// Map each fill polygon back to its grid cell index, assuming an integer
    /// grid where `data.x[ix] == ix as f64`.
    fn cell_claim_counts(data: &ContourPlotData) -> Vec<usize> {
        let nx = data.x.len();
        let ny = data.y.len();
        let mut claims = vec![0usize; (nx - 1) * (ny - 1)];

        for (_, _, polygons) in contour_fill_regions(data) {
            for polygon in &polygons {
                let x0 = polygon
                    .iter()
                    .map(|(px, _)| *px)
                    .fold(f64::INFINITY, f64::min);
                let y0 = polygon
                    .iter()
                    .map(|(_, py)| *py)
                    .fold(f64::INFINITY, f64::min);
                let ix = data
                    .x
                    .iter()
                    .position(|v| (v - x0).abs() < 1e-12)
                    .expect("polygon x corner on grid");
                let iy = data
                    .y
                    .iter()
                    .position(|v| (v - y0).abs() < 1e-12)
                    .expect("polygon y corner on grid");
                claims[iy * (nx - 1) + ix] += 1;
            }
        }

        claims
    }

    #[test]
    fn test_contour_fill_regions_cover_extremes() {
        // z ranges 0..16, levels only span 2..6, so cells below the lowest
        // level and above the highest one must still be filled.
        let x: Vec<f64> = (0..5).map(|i| i as f64).collect();
        let y: Vec<f64> = (0..5).map(|i| i as f64).collect();
        let mut z = Vec::with_capacity(25);
        for iy in 0..5 {
            for ix in 0..5 {
                z.push((ix * iy) as f64);
            }
        }

        let config = ContourConfig::default()
            .levels(vec![2.0, 4.0, 6.0])
            .interpolation_factor(1);
        let data = compute_contour_plot(&x, &y, &z, &config);
        let regions = contour_fill_regions(&data);

        let under = regions
            .iter()
            .find(|(low, _, _)| low.is_infinite() && low.is_sign_negative())
            .expect("open-ended band below the lowest level");
        assert!(!under.1.is_infinite());
        assert!(
            !under.2.is_empty(),
            "cells below the lowest level must fill"
        );

        let over = regions
            .iter()
            .find(|(_, high, _)| high.is_infinite() && high.is_sign_positive())
            .expect("open-ended band above the highest level");
        assert!(!over.0.is_infinite());
        assert!(
            !over.2.is_empty(),
            "cells above the highest level must fill"
        );

        assert!(cell_claim_counts(&data).iter().all(|c| *c == 1));
    }

    #[test]
    fn test_contour_fill_regions_cover_every_cell_once() {
        let (x, y, z) = make_test_grid();
        let config = ContourConfig::default().n_levels(4).interpolation_factor(1);
        let data = compute_contour_plot(&x, &y, &z, &config);

        let claims = cell_claim_counts(&data);
        assert!(
            claims.iter().all(|c| *c == 1),
            "every cell must be claimed by exactly one band, got {claims:?}"
        );
    }

    #[test]
    fn test_contour_fill_regions_single_level() {
        let (x, y, z) = make_test_grid();
        let config = ContourConfig::default()
            .levels(vec![0.5])
            .interpolation_factor(1);
        let data = compute_contour_plot(&x, &y, &z, &config);
        let regions = contour_fill_regions(&data);

        // Only the two open-ended bands exist, and they still cover the grid.
        assert_eq!(regions.len(), 2);
        assert!(cell_claim_counts(&data).iter().all(|c| *c == 1));
    }

    #[test]
    fn test_contour_fill_regions_skips_nan_cells() {
        let (x, y, mut z) = make_test_grid();
        z[5 * 10 + 5] = f64::NAN;

        let config = ContourConfig::default().n_levels(4).interpolation_factor(1);
        let data = compute_contour_plot(&x, &y, &z, &config);

        // The four cells touching the NaN vertex stay unpainted; every other
        // cell is still claimed exactly once.
        let claims = cell_claim_counts(&data);
        let unpainted = claims.iter().filter(|c| **c == 0).count();
        assert_eq!(unpainted, 4);
        assert!(claims.iter().all(|c| *c <= 1));
    }

    #[test]
    fn test_band_color_position() {
        // Open-ended bands clamp to the colormap floor and ceiling.
        assert_eq!(band_color_position(f64::NEG_INFINITY, 0.0, 0.0, 1.0), 0.0);
        assert_eq!(band_color_position(1.0, f64::INFINITY, 0.0, 1.0), 1.0);

        // Interior bands keep using the normalized midpoint.
        let t = band_color_position(0.0, 0.5, 0.0, 1.0);
        assert!((t - 0.25).abs() < 1e-12);

        // Degenerate level span falls back to the colormap midpoint.
        assert_eq!(band_color_position(1.0, 1.0, 1.0, 1.0), 0.5);
    }

    #[test]
    fn test_cell_in_band_membership() {
        assert!(cell_in_band(-5.0, f64::NEG_INFINITY, 0.0));
        assert!(!cell_in_band(0.0, f64::NEG_INFINITY, 0.0));
        assert!(cell_in_band(0.0, 0.0, 1.0));
        assert!(!cell_in_band(1.0, 0.0, 1.0));
        assert!(cell_in_band(1.0, 1.0, f64::INFINITY));
        assert!(cell_in_band(f64::INFINITY, 1.0, f64::INFINITY));
        assert!(!cell_in_band(f64::NAN, f64::NEG_INFINITY, f64::INFINITY));
    }

    #[test]
    fn test_contour_config_implements_plot_config() {
        fn assert_plot_config<T: PlotConfig>() {}
        assert_plot_config::<ContourConfig>();
    }

    #[test]
    fn test_contour_plot_compute_trait() {
        use crate::plots::traits::PlotCompute;

        let (x, y, z) = make_test_grid();
        // Disable interpolation for this test to get exact grid dimensions
        let config = ContourConfig::default().n_levels(5).interpolation_factor(1);
        let input = ContourInput::new(&x, &y, &z);
        let result = Contour::compute(input, &config);

        assert!(result.is_ok());
        let contour_data = result.unwrap();
        assert_eq!(contour_data.shape, (10, 10));
        assert!(!contour_data.levels.is_empty());
    }

    #[test]
    fn test_contour_plot_compute_empty() {
        use crate::plots::traits::PlotCompute;

        let x: Vec<f64> = vec![];
        let y: Vec<f64> = vec![];
        let z: Vec<f64> = vec![];
        let config = ContourConfig::default();
        let input = ContourInput::new(&x, &y, &z);
        let result = Contour::compute(input, &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_contour_plot_compute_mismatched_z() {
        use crate::plots::traits::PlotCompute;

        let x = vec![0.0, 1.0, 2.0];
        let y = vec![0.0, 1.0];
        let z = vec![1.0, 2.0, 3.0]; // Should be 6 elements (3 * 2)
        let config = ContourConfig::default();
        let input = ContourInput::new(&x, &y, &z);
        let result = Contour::compute(input, &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_contour_plot_data_trait() {
        use crate::plots::traits::{PlotCompute, PlotData};

        let (x, y, z) = make_test_grid();
        let config = ContourConfig::default().n_levels(5);
        let input = ContourInput::new(&x, &y, &z);
        let contour_data = Contour::compute(input, &config).unwrap();

        // Test data_bounds
        let ((x_min, x_max), (y_min, y_max)) = contour_data.data_bounds();
        assert!((x_min - 0.0).abs() < 1e-10);
        assert!((x_max - 9.0).abs() < 1e-10);
        assert!((y_min - 0.0).abs() < 1e-10);
        assert!((y_max - 9.0).abs() < 1e-10);

        // Test is_empty
        assert!(!contour_data.is_empty());
    }

    #[test]
    fn test_filled_contour_lines_use_theme_foreground() {
        let theme = Theme::dark();
        let cmap = ColorMap::viridis();
        let config = ContourConfig::default().filled(true);

        for level in 0..5 {
            assert_eq!(
                contour_line_color(&config, &theme, &cmap, Color::RED, level, 5),
                theme.foreground,
                "filled contour line {level} must contrast with the band it sits on"
            );
        }
    }

    #[test]
    fn test_unfilled_contour_lines_use_theme_foreground() {
        // The dark end of viridis is ~#440154, which is invisible on a dark theme
        // background — so an unfilled contour must follow the theme too.
        let cmap = ColorMap::viridis();
        let config = ContourConfig::default().filled(false);

        for theme in [Theme::default(), Theme::dark()] {
            for level in 0..5 {
                assert_eq!(
                    contour_line_color(&config, &theme, &cmap, Color::RED, level, 5),
                    theme.foreground,
                    "unfilled contour line {level} must contrast with the background"
                );
            }
        }
    }

    #[test]
    fn test_colormapped_contour_lines_are_opt_in() {
        let theme = Theme::default();
        let cmap = ColorMap::viridis();
        let config = ContourConfig::default()
            .filled(false)
            .color_lines_by_level(true);

        assert_eq!(
            contour_line_color(&config, &theme, &cmap, Color::RED, 0, 5),
            cmap.sample(0.0)
        );
        assert_eq!(
            contour_line_color(&config, &theme, &cmap, Color::RED, 4, 5),
            cmap.sample(1.0)
        );
    }

    #[test]
    fn test_single_level_colormapped_contour_uses_series_color() {
        let theme = Theme::default();
        let cmap = ColorMap::viridis();
        let config = ContourConfig::default()
            .filled(false)
            .color_lines_by_level(true);

        assert_eq!(
            contour_line_color(&config, &theme, &cmap, Color::RED, 0, 1),
            Color::RED
        );
    }

    #[test]
    fn test_explicit_contour_line_color_wins_over_theme() {
        let theme = Theme::dark();
        let cmap = ColorMap::viridis();

        for filled in [true, false] {
            let config = ContourConfig::default()
                .filled(filled)
                .line_color(Color::GREEN);
            assert_eq!(
                contour_line_color(&config, &theme, &cmap, Color::RED, 2, 5),
                Color::GREEN
            );
        }
    }
}
