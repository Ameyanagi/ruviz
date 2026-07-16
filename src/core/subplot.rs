/// Subplot functionality for multiple plots in one figure
///
/// Provides grid-based layout system for arranging multiple plots
/// within a single figure, similar to matplotlib's subplot functionality.
use crate::core::{Plot, PlottingError, REFERENCE_DPI, RenderScale, Result};
use crate::render::{Theme, skia::SkiaRenderer};
use tiny_skia::Rect;

const DEFAULT_SUPTITLE_SCALE: f32 = 1.2;
const SUPTITLE_TOP_INSET_POINTS: f32 = 6.0;
const SUPTITLE_GRID_GAP_POINTS: f32 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct SuptitleLayout {
    font_size_px: f32,
    text_width: f32,
    text_height: f32,
    text_top: f32,
    reserved_height: f32,
}

impl SuptitleLayout {
    #[cfg(test)]
    fn text_bounds(self, figure_width: u32) -> Rect {
        Rect::from_xywh(
            (figure_width as f32 - self.text_width) / 2.0,
            self.text_top,
            self.text_width,
            self.text_height,
        )
        .expect("measured suptitle dimensions must form a valid rectangle")
    }
}

/// Grid specification for subplot layout
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridSpec {
    /// Number of rows in the subplot grid
    pub rows: usize,
    /// Number of columns in the subplot grid
    pub cols: usize,
    /// Vertical spacing between rows (as a fraction of subplot height)
    pub hspace: f32,
    /// Horizontal spacing between columns (as a fraction of subplot width)
    pub wspace: f32,
}

impl GridSpec {
    /// Create a new grid specification
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::prelude::GridSpec;
    ///
    /// let grid = GridSpec::new(2, 3);  // 2 rows, 3 columns
    /// assert_eq!(grid.total_subplots(), 6);
    /// ```
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            hspace: 0.0, // No spacing - subplots fill available area
            wspace: 0.0,
        }
    }

    /// Set vertical spacing between rows
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::prelude::GridSpec;
    ///
    /// let grid = GridSpec::new(2, 2).with_hspace(0.3);
    /// assert_eq!(grid.hspace, 0.3);
    /// ```
    pub fn with_hspace(mut self, hspace: f32) -> Self {
        self.hspace = hspace.clamp(0.0, 1.0);
        self
    }

    /// Set horizontal spacing between columns
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::prelude::GridSpec;
    ///
    /// let grid = GridSpec::new(2, 2).with_wspace(0.4);
    /// assert_eq!(grid.wspace, 0.4);
    /// ```
    pub fn with_wspace(mut self, wspace: f32) -> Self {
        self.wspace = wspace.clamp(0.0, 1.0);
        self
    }

    /// Total number of subplots in the grid
    pub fn total_subplots(&self) -> usize {
        self.rows * self.cols
    }

    /// Validate grid specification
    pub fn validate(&self) -> Result<()> {
        if self.rows == 0 || self.cols == 0 {
            return Err(PlottingError::InvalidInput(
                "Grid must have at least 1 row and 1 column".to_string(),
            ));
        }
        if self.rows > 10 || self.cols > 10 {
            return Err(PlottingError::InvalidInput(
                "Grid size limited to 10x10 for performance".to_string(),
            ));
        }
        Ok(())
    }

    /// Calculate subplot rectangle for given index
    ///
    /// # Arguments
    /// * `index` - Subplot index (row * cols + col)
    /// * `figure_width` - Total figure width in pixels
    /// * `figure_height` - Total figure height in pixels
    /// * `margin` - Margin as fraction of figure size
    /// * `top_offset` - Additional top offset for suptitle (in pixels)
    pub fn subplot_rect(
        &self,
        index: usize,
        figure_width: u32,
        figure_height: u32,
        margin: f32,
        top_offset: f32,
    ) -> Result<Rect> {
        if index >= self.total_subplots() {
            return Err(PlottingError::InvalidInput(format!(
                "Subplot index {} exceeds grid size {}",
                index,
                self.total_subplots()
            )));
        }

        let row = index / self.cols;
        let col = index % self.cols;

        // Calculate available space after margins
        let margin_px = margin * figure_width.min(figure_height) as f32;
        let available_width = figure_width as f32 - 2.0 * margin_px;
        // Subtract top_offset from available height to reserve space for suptitle
        let available_height = figure_height as f32 - 2.0 * margin_px - top_offset;

        // Calculate subplot dimensions with spacing
        let subplot_width = available_width / self.cols as f32;
        let subplot_height = available_height / self.rows as f32;

        let spacing_x = subplot_width * self.wspace;
        let spacing_y = subplot_height * self.hspace;

        let plot_width = subplot_width - spacing_x;
        let plot_height = subplot_height - spacing_y;

        // Calculate subplot position (add top_offset to y position)
        let x = margin_px + col as f32 * subplot_width + spacing_x / 2.0;
        let y = margin_px + top_offset + row as f32 * subplot_height + spacing_y / 2.0;

        Rect::from_xywh(x, y, plot_width, plot_height).ok_or_else(|| {
            PlottingError::InvalidInput("Invalid subplot dimensions calculated".to_string())
        })
    }
}

/// Subplot figure containing multiple plots arranged in a grid
///
/// Create subplot figures using [`subplots()`] or [`subplots_default()`].
///
/// # Example
///
/// ```rust,no_run
/// use ruviz::prelude::*;
///
/// let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
/// let y_sin: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
/// let y_cos: Vec<f64> = x.iter().map(|&v| v.cos()).collect();
///
/// let plot1 = Plot::new().line(&x, &y_sin).end_series();
/// let plot2 = Plot::new().line(&x, &y_cos).end_series();
///
/// subplots(1, 2, 800, 400)?
///     .subplot_at(0, plot1)?
///     .subplot_at(1, plot2)?
///     .save("side_by_side.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// ![Subplot example](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/subplots.png)
#[derive(Debug, Clone)]
pub struct SubplotFigure {
    /// Grid specification for layout
    grid: GridSpec,
    /// Individual plots in the figure
    plots: Vec<Option<Plot>>,
    /// Figure dimensions
    width: u32,
    height: u32,
    /// Overall figure title
    suptitle: Option<String>,
    /// Optional figure title font size override in points
    suptitle_font_size: Option<f32>,
    /// Figure-level styling used for the canvas and suptitle
    theme: Theme,
    /// Figure margin (fraction of figure size)
    margin: f32,
}

impl SubplotFigure {
    fn scaled_dimension(value: u32, dpi: f32, name: &str) -> Result<u32> {
        let scaled = f64::from(value) * f64::from(dpi) / f64::from(REFERENCE_DPI);
        if !scaled.is_finite() || scaled < 1.0 || scaled > f64::from(u32::MAX) {
            return Err(PlottingError::InvalidInput(format!(
                "Scaled subplot figure {name} is out of range ({name}={value}, dpi={dpi})"
            )));
        }
        Ok(scaled.round() as u32)
    }

    fn rect_pixel(value: f32, name: &str) -> Result<u32> {
        if !value.is_finite() || value < 0.0 || value > u32::MAX as f32 {
            return Err(PlottingError::InvalidInput(format!(
                "Subplot {name} is out of pixel range ({name}={value})"
            )));
        }
        Ok(value.floor() as u32)
    }

    fn margin_pixels(&self, width: u32, height: u32) -> f32 {
        self.margin * width.min(height) as f32
    }

    fn resolved_suptitle_font_size(&self) -> f32 {
        self.suptitle_font_size
            .unwrap_or(self.theme.title_font_size * DEFAULT_SUPTITLE_SCALE)
            .max(6.0)
    }

    fn suptitle_layout(
        &self,
        renderer: &SkiaRenderer,
        width: u32,
        height: u32,
    ) -> Result<Option<SuptitleLayout>> {
        let Some(title) = &self.suptitle else {
            return Ok(None);
        };

        let render_scale = renderer.render_scale();
        let font_size_px = render_scale.points_to_pixels(self.resolved_suptitle_font_size());
        let (text_width, text_height) = renderer.measure_text(title, font_size_px)?;
        let top_inset = render_scale.points_to_pixels(SUPTITLE_TOP_INSET_POINTS);
        let grid_gap = render_scale.points_to_pixels(SUPTITLE_GRID_GAP_POINTS);

        Ok(Some(SuptitleLayout {
            font_size_px,
            text_width,
            text_height,
            text_top: self.margin_pixels(width, height) + top_inset,
            reserved_height: top_inset + text_height + grid_gap,
        }))
    }

    /// Create a new subplot figure
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::prelude::*;
    ///
    /// // Create a 2x2 grid of subplots, 800x600 pixels
    /// let figure = SubplotFigure::new(2, 2, 800, 600)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(rows: usize, cols: usize, width: u32, height: u32) -> Result<Self> {
        let grid = GridSpec::new(rows, cols);
        grid.validate()?;

        let total_plots = grid.total_subplots();
        let plots = vec![None; total_plots];

        Ok(Self {
            grid,
            plots,
            width,
            height,
            suptitle: None,
            suptitle_font_size: None,
            theme: Theme::default(),
            margin: 0.05, // 5% margin by default - tighter layout
        })
    }

    /// Set vertical spacing between rows
    pub fn hspace(mut self, hspace: f32) -> Self {
        self.grid = self.grid.with_hspace(hspace);
        self
    }

    /// Set horizontal spacing between columns
    pub fn wspace(mut self, wspace: f32) -> Self {
        self.grid = self.grid.with_wspace(wspace);
        self
    }

    /// Set overall figure title
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// subplots(2, 2, 800, 600)?
    ///     .suptitle("My Figure Title")
    ///     .save("figure_with_title.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn suptitle<S: Into<String>>(mut self, title: S) -> Self {
        self.suptitle = Some(title.into());
        self
    }

    /// Set the overall figure title font size in typographic points.
    ///
    /// When unset, the size derives from the figure theme and is larger than
    /// the theme's panel title size.
    pub fn suptitle_font_size(mut self, size: f32) -> Self {
        self.suptitle_font_size = Some(size.max(6.0));
        self
    }

    /// Set figure-level styling for the canvas and suptitle.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Set figure margin
    pub fn margin(mut self, margin: f32) -> Self {
        self.margin = margin.clamp(0.0, 0.4); // Max 40% margin
        self
    }

    /// Add a plot at the specified subplot position
    ///
    /// Position is calculated as: index = row * cols + col (0-indexed)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let plot1 = Plot::new().line(&[1.0, 2.0], &[1.0, 4.0]).end_series();
    /// let plot2 = Plot::new().line(&[1.0, 2.0], &[2.0, 3.0]).end_series();
    ///
    /// subplots(2, 2, 800, 600)?
    ///     .subplot(0, 0, plot1)?  // Top-left
    ///     .subplot(1, 1, plot2)?  // Bottom-right
    ///     .save("grid.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn subplot(mut self, row: usize, col: usize, plot: Plot) -> Result<Self> {
        if row >= self.grid.rows || col >= self.grid.cols {
            return Err(PlottingError::InvalidInput(format!(
                "Subplot position ({}, {}) exceeds grid size {}x{}",
                row, col, self.grid.rows, self.grid.cols
            )));
        }

        let index = row * self.grid.cols + col;
        self.plots[index] = Some(plot);
        Ok(self)
    }

    /// Add a plot at the specified linear index (0-based)
    ///
    /// Linear index maps left-to-right, top-to-bottom: index = row * cols + col
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let plot = Plot::new().line(&[1.0, 2.0], &[1.0, 4.0]).end_series();
    ///
    /// // In a 2x3 grid: index 0 = (0,0), index 3 = (1,0), index 5 = (1,2)
    /// subplots(2, 3, 900, 600)?
    ///     .subplot_at(0, plot)?  // First position
    ///     .save("indexed.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn subplot_at(mut self, index: usize, plot: Plot) -> Result<Self> {
        if index >= self.plots.len() {
            return Err(PlottingError::InvalidInput(format!(
                "Subplot index {} exceeds total subplots {}",
                index,
                self.plots.len()
            )));
        }

        self.plots[index] = Some(plot);
        Ok(self)
    }

    /// Get grid specification
    pub fn grid_spec(&self) -> GridSpec {
        self.grid
    }

    /// Get subplot count
    pub fn subplot_count(&self) -> usize {
        self.plots.iter().filter(|p| p.is_some()).count()
    }

    /// Render all subplots to a single image and save
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let x = vec![1.0, 2.0, 3.0];
    /// let y = vec![1.0, 4.0, 9.0];
    ///
    /// let plot = Plot::new().line(&x, &y).end_series();
    ///
    /// subplots(1, 2, 800, 400)?
    ///     .subplot_at(0, plot.clone())?
    ///     .subplot_at(1, plot)?
    ///     .save("subplots.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn save<P: AsRef<std::path::Path>>(self, path: P) -> Result<()> {
        self.save_with_dpi(path, REFERENCE_DPI)
    }

    /// Render all subplots with specified DPI
    pub fn save_with_dpi<P: AsRef<std::path::Path>>(self, path: P, dpi: f32) -> Result<()> {
        if !dpi.is_finite() || dpi <= 0.0 {
            return Err(PlottingError::InvalidInput(format!(
                "Subplot figure DPI must be a finite, positive value (dpi={dpi})"
            )));
        }
        if dpi < crate::core::constants::dpi::MIN as f32 {
            return Err(PlottingError::InvalidInput(format!(
                "Subplot figure DPI must be at least {} (dpi={dpi})",
                crate::core::constants::dpi::MIN
            )));
        }
        if dpi > crate::core::constants::dpi::MAX as f32 {
            return Err(PlottingError::PerformanceLimit {
                limit_type: "DPI".to_string(),
                actual: dpi.ceil() as usize,
                maximum: crate::core::constants::dpi::MAX as usize,
            });
        }

        let width = Self::scaled_dimension(self.width, dpi, "width")?;
        let height = Self::scaled_dimension(self.height, dpi, "height")?;
        PlottingError::validate_dimensions(width, height)?;
        let dpi_scale = dpi / REFERENCE_DPI;

        // Create main renderer for the figure
        let mut renderer = SkiaRenderer::new(width, height, self.theme.clone())?;
        renderer.set_render_scale(RenderScale::from_canvas_size(width, height, dpi));

        let suptitle_layout = self.suptitle_layout(&renderer, width, height)?;
        let suptitle_height = suptitle_layout
            .map(|layout| layout.reserved_height)
            .unwrap_or(0.0);

        // Render figure title if present
        if let (Some(title), Some(layout)) = (&self.suptitle, suptitle_layout) {
            renderer.draw_text_centered(
                title,
                width as f32 / 2.0,
                layout.text_top,
                layout.font_size_px,
                self.theme.foreground,
            )?;
        }

        // Render each subplot
        for (index, plot_opt) in self.plots.iter().enumerate() {
            if let Some(plot) = plot_opt {
                // Calculate subplot area with suptitle offset
                let subplot_rect =
                    self.grid
                        .subplot_rect(index, width, height, self.margin, suptitle_height)?;

                // Calculate typography scale factor based on subplot size and DPI
                // Use reference-DPI dimensions so small subplots get the same
                // typography adjustment at every requested output DPI.
                let reference_dim = 300.0_f32;
                let subplot_min_dim = subplot_rect.width().min(subplot_rect.height()) / dpi_scale;
                let size_scale = (subplot_min_dim / reference_dim).clamp(0.35, 1.0);

                // Clone plot and scale typography for small subplots
                let scaled_plot = plot.clone().scale_typography(size_scale);

                // Create a temporary renderer for this subplot
                let subplot_theme = scaled_plot.get_theme();
                let subplot_width = Self::rect_pixel(subplot_rect.width(), "width")?;
                let subplot_height = Self::rect_pixel(subplot_rect.height(), "height")?;
                PlottingError::validate_subplot_dimensions(subplot_width, subplot_height)?;
                let mut subplot_renderer =
                    SkiaRenderer::new(subplot_width, subplot_height, subplot_theme)?;

                scaled_plot.render_to_renderer(&mut subplot_renderer, dpi)?;

                // Copy subplot renderer to main renderer at correct position
                renderer.draw_subplot(
                    subplot_renderer.into_image(),
                    Self::rect_pixel(subplot_rect.left(), "x position")?,
                    Self::rect_pixel(subplot_rect.top(), "y position")?,
                )?;
            }
        }

        // Save the final figure
        renderer.save_png(path)?;
        Ok(())
    }
}

/// Convenience function to create a subplot figure
///
/// # Example
///
/// ```rust,no_run
/// use ruviz::prelude::*;
///
/// let x: Vec<f64> = vec![1.0, 2.0, 3.0];
/// let sin_plot: Plot = Plot::new()
///     .line(&x, &x.iter().map(|&v| v.sin()).collect::<Vec<_>>())
///     .title("Sin")
///     .into();
/// let cos_plot: Plot = Plot::new()
///     .line(&x, &x.iter().map(|&v| v.cos()).collect::<Vec<_>>())
///     .title("Cos")
///     .into();
///
/// subplots(1, 2, 800, 400)?
///     .subplot_at(0, sin_plot)?
///     .subplot_at(1, cos_plot)?
///     .suptitle("Trigonometric Functions")
///     .save("trig.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn subplots(rows: usize, cols: usize, width: u32, height: u32) -> Result<SubplotFigure> {
    SubplotFigure::new(rows, cols, width, height)
}

/// Convenience function to create a subplot figure with default size
///
/// Default size scales based on number of subplots:
/// - Width: 400 * cols (max 1600)
/// - Height: 300 * rows (max 1200)
///
/// # Example
///
/// ```rust,no_run
/// use ruviz::prelude::*;
///
/// // Creates an 800x600 figure (400*2 x 300*2)
/// let plot: Plot = Plot::new().line(&[1.0, 2.0], &[1.0, 4.0]).into();
///
/// subplots_default(2, 2)?
///     .subplot_at(0, plot)?
///     .save("default_size.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn subplots_default(rows: usize, cols: usize) -> Result<SubplotFigure> {
    // Default figure size scales with subplot count
    let width = 400_usize
        .checked_mul(cols)
        .ok_or_else(|| PlottingError::InvalidInput("Default subplot width overflow".to_string()))?
        .min(1600);
    let height = 300_usize
        .checked_mul(rows)
        .ok_or_else(|| PlottingError::InvalidInput("Default subplot height overflow".to_string()))?
        .min(1200);
    let width = u32::try_from(width)
        .map_err(|_| PlottingError::InvalidInput("Default subplot width overflow".to_string()))?;
    let height = u32::try_from(height)
        .map_err(|_| PlottingError::InvalidInput("Default subplot height overflow".to_string()))?;

    SubplotFigure::new(rows, cols, width, height)
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;

    fn assert_rect(rect: Rect, expected: (f32, f32, f32, f32)) {
        assert_eq!(
            (rect.left(), rect.top(), rect.width(), rect.height()),
            expected
        );
    }

    fn assert_proportional(at_100: f32, at_200: f32, name: &str) {
        assert!(
            (at_200 - 2.0 * at_100).abs() <= 0.001,
            "{name} should scale exactly with DPI: at 100 DPI={at_100}, at 200 DPI={at_200}"
        );
    }

    fn assert_close(actual: f32, expected: f32, name: &str) {
        assert!(
            (actual - expected).abs() <= 0.001,
            "{name} should match: actual={actual}, expected={expected}"
        );
    }

    fn renderer_for(figure: &SubplotFigure, dpi: f32) -> (SkiaRenderer, u32, u32) {
        let width = SubplotFigure::scaled_dimension(figure.width, dpi, "width").unwrap();
        let height = SubplotFigure::scaled_dimension(figure.height, dpi, "height").unwrap();
        let mut renderer = SkiaRenderer::new(width, height, figure.theme.clone()).unwrap();
        renderer.set_render_scale(RenderScale::from_canvas_size(width, height, dpi));
        (renderer, width, height)
    }

    #[test]
    fn test_grid_spec_creation() {
        let grid = GridSpec::new(2, 3);
        assert_eq!(grid.rows, 2);
        assert_eq!(grid.cols, 3);
        assert_eq!(grid.total_subplots(), 6);
    }

    #[test]
    fn test_grid_spec_spacing() {
        let grid = GridSpec::new(2, 2).with_hspace(0.3).with_wspace(0.4);
        assert_eq!(grid.hspace, 0.3);
        assert_eq!(grid.wspace, 0.4);
    }

    #[test]
    fn test_grid_validation() {
        assert!(GridSpec::new(0, 1).validate().is_err());
        assert!(GridSpec::new(1, 0).validate().is_err());
        assert!(GridSpec::new(11, 1).validate().is_err());
        assert!(GridSpec::new(2, 3).validate().is_ok());
    }

    #[test]
    fn test_1x2_wspace_controls_horizontal_geometry() {
        let grid = GridSpec::new(1, 2).with_wspace(0.25);

        assert_rect(
            grid.subplot_rect(0, 800, 400, 0.0, 0.0).unwrap(),
            (50.0, 0.0, 300.0, 400.0),
        );
        assert_rect(
            grid.subplot_rect(1, 800, 400, 0.0, 0.0).unwrap(),
            (450.0, 0.0, 300.0, 400.0),
        );
    }

    #[test]
    fn test_2x1_hspace_controls_vertical_geometry() {
        let grid = GridSpec::new(2, 1).with_hspace(0.5);

        assert_rect(
            grid.subplot_rect(0, 800, 600, 0.0, 0.0).unwrap(),
            (0.0, 75.0, 800.0, 150.0),
        );
        assert_rect(
            grid.subplot_rect(1, 800, 600, 0.0, 0.0).unwrap(),
            (0.0, 375.0, 800.0, 150.0),
        );
    }

    #[test]
    fn test_2x2_spacing_directions_are_independent() {
        let grid = GridSpec::new(2, 2).with_hspace(0.5).with_wspace(0.25);

        for (index, expected) in [
            (0, (50.0, 75.0, 300.0, 150.0)),
            (1, (450.0, 75.0, 300.0, 150.0)),
            (2, (50.0, 375.0, 300.0, 150.0)),
            (3, (450.0, 375.0, 300.0, 150.0)),
        ] {
            assert_rect(
                grid.subplot_rect(index, 800, 600, 0.0, 0.0).unwrap(),
                expected,
            );
        }
    }

    #[test]
    fn test_suptitle_measurement_drives_title_and_grid_top_geometry() {
        let without_title = SubplotFigure::new(1, 1, 800, 600).unwrap().margin(0.1);
        let (renderer, width, height) = renderer_for(&without_title, REFERENCE_DPI);
        assert_eq!(
            without_title
                .suptitle_layout(&renderer, width, height)
                .unwrap(),
            None
        );
        assert_rect(
            without_title
                .grid
                .subplot_rect(0, width, height, without_title.margin, 0.0)
                .unwrap(),
            (60.0, 60.0, 680.0, 480.0),
        );

        let with_title = without_title.suptitle("Figure Title");
        let (renderer, width, height) = renderer_for(&with_title, REFERENCE_DPI);
        let layout = with_title
            .suptitle_layout(&renderer, width, height)
            .unwrap()
            .unwrap();
        let render_scale = renderer.render_scale();
        let inset = render_scale.points_to_pixels(SUPTITLE_TOP_INSET_POINTS);
        let gap = render_scale.points_to_pixels(SUPTITLE_GRID_GAP_POINTS);
        let expected_size = render_scale
            .points_to_pixels(with_title.theme.title_font_size * DEFAULT_SUPTITLE_SCALE);
        let (expected_width, expected_height) = renderer
            .measure_text("Figure Title", expected_size)
            .unwrap();

        assert_eq!(layout.font_size_px, expected_size);
        assert_eq!(layout.text_width, expected_width);
        assert_eq!(layout.text_height, expected_height);
        assert_eq!(layout.text_top, 60.0 + inset);
        assert_eq!(layout.reserved_height, inset + expected_height + gap);
        assert!(
            with_title.resolved_suptitle_font_size() > with_title.theme.title_font_size,
            "the default figure title must outrank the theme's panel title"
        );

        let title_bounds = layout.text_bounds(width);
        let subplot = with_title
            .grid
            .subplot_rect(0, width, height, with_title.margin, layout.reserved_height)
            .unwrap();
        assert_eq!(title_bounds.top(), layout.text_top);
        assert_close(
            title_bounds.height(),
            layout.text_height,
            "title bounds height",
        );
        assert_close(
            title_bounds.bottom() + gap,
            subplot.top(),
            "title-to-grid gap",
        );
    }

    #[test]
    fn test_custom_suptitle_font_size_is_point_based() {
        let figure = SubplotFigure::new(1, 1, 800, 600)
            .unwrap()
            .margin(0.1)
            .suptitle("Custom Title")
            .suptitle_font_size(24.0);
        let (renderer, width, height) = renderer_for(&figure, REFERENCE_DPI);
        let layout = figure
            .suptitle_layout(&renderer, width, height)
            .unwrap()
            .unwrap();

        assert_eq!(figure.suptitle_font_size, Some(24.0));
        let render_scale = renderer.render_scale();
        let expected_size = render_scale.points_to_pixels(24.0);
        let expected_height = renderer
            .measure_text("Custom Title", expected_size)
            .unwrap()
            .1;
        let expected_reserved = render_scale.points_to_pixels(SUPTITLE_TOP_INSET_POINTS)
            + expected_height
            + render_scale.points_to_pixels(SUPTITLE_GRID_GAP_POINTS);
        let subplot = figure
            .grid
            .subplot_rect(0, width, height, figure.margin, layout.reserved_height)
            .unwrap();

        assert_eq!(layout.font_size_px, expected_size);
        assert_eq!(layout.reserved_height, expected_reserved);
        assert_eq!(subplot.top(), 60.0 + expected_reserved);
    }

    #[test]
    fn test_suptitle_and_subplot_bounds_scale_proportionally_with_dpi() {
        let figure = SubplotFigure::new(2, 2, 800, 600)
            .unwrap()
            .margin(0.1)
            .hspace(0.5)
            .wspace(0.25)
            .suptitle("Scaling Figure");
        let (renderer_100, width_100, height_100) = renderer_for(&figure, 100.0);
        let (renderer_200, width_200, height_200) = renderer_for(&figure, 200.0);
        let title_100 = figure
            .suptitle_layout(&renderer_100, width_100, height_100)
            .unwrap()
            .unwrap();
        let title_200 = figure
            .suptitle_layout(&renderer_200, width_200, height_200)
            .unwrap()
            .unwrap();
        let bounds_100 = title_100.text_bounds(width_100);
        let bounds_200 = title_200.text_bounds(width_200);
        let subplot_100 = figure
            .grid
            .subplot_rect(
                3,
                width_100,
                height_100,
                figure.margin,
                title_100.reserved_height,
            )
            .unwrap();
        let subplot_200 = figure
            .grid
            .subplot_rect(
                3,
                width_200,
                height_200,
                figure.margin,
                title_200.reserved_height,
            )
            .unwrap();

        for (name, at_100, at_200) in [
            ("title left", bounds_100.left(), bounds_200.left()),
            ("title top", bounds_100.top(), bounds_200.top()),
            ("title width", bounds_100.width(), bounds_200.width()),
            ("title height", bounds_100.height(), bounds_200.height()),
            ("subplot left", subplot_100.left(), subplot_200.left()),
            ("subplot top", subplot_100.top(), subplot_200.top()),
            ("subplot width", subplot_100.width(), subplot_200.width()),
            ("subplot height", subplot_100.height(), subplot_200.height()),
        ] {
            assert_proportional(at_100, at_200, name);
        }
    }

    #[test]
    fn test_subplot_figure_creation() {
        let figure = SubplotFigure::new(2, 3, 800, 600).unwrap();
        assert_eq!(figure.subplot_count(), 0); // No plots added yet
        assert_eq!(figure.grid_spec().total_subplots(), 6);
    }

    #[test]
    fn test_subplot_positioning() {
        let mut figure = SubplotFigure::new(2, 2, 800, 600).unwrap();
        let plot = Plot::new();

        // Test adding subplot by row/col
        figure = figure.subplot(0, 1, plot.clone()).unwrap();
        assert_eq!(figure.subplot_count(), 1);

        // Test adding subplot by index
        figure = figure.subplot_at(3, plot).unwrap();
        assert_eq!(figure.subplot_count(), 2);
    }

    #[test]
    fn test_subplot_bounds_checking() {
        let figure = SubplotFigure::new(2, 2, 800, 600).unwrap();
        let plot = Plot::new();

        // Should fail - row out of bounds
        assert!(figure.clone().subplot(2, 0, plot.clone()).is_err());

        // Should fail - col out of bounds
        assert!(figure.clone().subplot(0, 2, plot.clone()).is_err());

        // Should fail - index out of bounds
        assert!(figure.clone().subplot_at(4, plot).is_err());
    }

    #[test]
    fn test_convenience_functions() {
        let figure = subplots(2, 3, 800, 600).unwrap();
        assert_eq!(figure.grid_spec().rows, 2);
        assert_eq!(figure.grid_spec().cols, 3);

        let figure = subplots_default(2, 2).unwrap();
        assert_eq!(figure.width, 800); // 400 * 2
        assert_eq!(figure.height, 600); // 300 * 2
    }

    #[test]
    fn test_subplot_rendering_integration() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![2.0, 4.0, 3.0];

        let plot = Plot::new().line(&x, &y).end_series().title("Test Plot");

        let figure = SubplotFigure::new(1, 1, 400, 300)
            .unwrap()
            .subplot(0, 0, plot)
            .unwrap();

        assert_eq!(figure.subplot_count(), 1);

        // The rendering itself is tested by the working example,
        // this tests the structure is correctly set up
        assert_eq!(figure.grid_spec().total_subplots(), 1);
        assert_eq!(figure.width, 400);
        assert_eq!(figure.height, 300);
    }

    #[test]
    fn test_subplot_renders_child_plot_legend() {
        let dir = tempfile::tempdir().unwrap();
        let with_legend_path = dir.path().join("with_legend.png");
        let without_legend_path = dir.path().join("without_legend.png");

        let x = vec![0.0, 1.0, 2.0, 3.0];
        let y1 = vec![0.1, 0.2, 0.3, 0.4];
        let y2 = vec![0.9, 0.8, 0.7, 0.6];

        let make_plot = |legend: bool| -> Plot {
            let builder = Plot::new()
                .line(&x, &y1)
                .label("Alpha")
                .line(&x, &y2)
                .label("Beta")
                .xlim(0.0, 3.0)
                .ylim(0.0, 1.0);

            if legend {
                builder.legend_best().into()
            } else {
                builder.into()
            }
        };

        subplots(1, 1, 500, 350)
            .unwrap()
            .subplot(0, 0, make_plot(true))
            .unwrap()
            .save(&with_legend_path)
            .unwrap();
        subplots(1, 1, 500, 350)
            .unwrap()
            .subplot(0, 0, make_plot(false))
            .unwrap()
            .save(&without_legend_path)
            .unwrap();

        let with_legend = image::open(with_legend_path).unwrap().to_rgba8();
        let without_legend = image::open(without_legend_path).unwrap().to_rgba8();
        assert_eq!(with_legend.dimensions(), without_legend.dimensions());

        let differing_pixels = with_legend
            .pixels()
            .zip(without_legend.pixels())
            .filter(|(left, right)| {
                left.0
                    .iter()
                    .zip(right.0.iter())
                    .any(|(lhs, rhs)| (*lhs as i16 - *rhs as i16).abs() > 8)
            })
            .count();

        assert!(
            differing_pixels > 250,
            "legend should visibly change subplot output; differing pixels={differing_pixels}"
        );
    }

    #[test]
    fn test_subplot_save_with_dpi_scales_canvas_and_composition() {
        fn ink_bounds(image: &image::RgbaImage) -> (u32, u32, u32, u32) {
            let mut bounds = (image.width(), image.height(), 0, 0);
            for (x, y, pixel) in image.enumerate_pixels() {
                if pixel.0[..3].iter().all(|channel| *channel > 245) {
                    continue;
                }
                bounds.0 = bounds.0.min(x);
                bounds.1 = bounds.1.min(y);
                bounds.2 = bounds.2.max(x);
                bounds.3 = bounds.3.max(y);
            }
            bounds
        }

        let dir = tempfile::tempdir().unwrap();
        let path_100 = dir.path().join("subplot-100.png");
        let path_200 = dir.path().join("subplot-200.png");
        let plot: Plot = Plot::new()
            .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 0.25])
            .title("Child")
            .into();
        let figure = SubplotFigure::new(1, 1, 400, 300)
            .unwrap()
            .suptitle("Figure")
            .subplot_at(0, plot)
            .unwrap();

        figure.clone().save(&path_100).unwrap();
        figure.save_with_dpi(&path_200, 200.0).unwrap();

        let image_100 = image::open(path_100).unwrap().to_rgba8();
        let image_200 = image::open(path_200).unwrap().to_rgba8();
        assert_eq!(image_100.dimensions(), (400, 300));
        assert_eq!(image_200.dimensions(), (800, 600));

        let bounds_100 = ink_bounds(&image_100);
        let bounds_200 = ink_bounds(&image_200);
        for (at_100, at_200) in [
            (bounds_100.0, bounds_200.0),
            (bounds_100.1, bounds_200.1),
            (bounds_100.2, bounds_200.2),
            (bounds_100.3, bounds_200.3),
        ] {
            assert!(
                (i64::from(at_200) - 2 * i64::from(at_100)).abs() <= 5,
                "subplot composition should scale with DPI: 100-DPI bounds={bounds_100:?}, 200-DPI bounds={bounds_200:?}"
            );
        }
    }

    #[test]
    fn test_subplot_children_may_be_smaller_than_top_level_minimum() {
        let dir = tempfile::tempdir().unwrap();
        let low_dpi_path = dir.path().join("low-dpi-rows.png");
        let small_columns_path = dir.path().join("small-columns.png");
        let plot = Plot::new();

        SubplotFigure::new(2, 1, 400, 300)
            .unwrap()
            .subplot_at(0, plot.clone())
            .unwrap()
            .subplot_at(1, plot.clone())
            .unwrap()
            .save_with_dpi(&low_dpi_path, 72.0)
            .unwrap();
        SubplotFigure::new(1, 2, 200, 100)
            .unwrap()
            .subplot_at(0, plot.clone())
            .unwrap()
            .subplot_at(1, plot)
            .unwrap()
            .save(&small_columns_path)
            .unwrap();

        assert_eq!(image::image_dimensions(low_dpi_path).unwrap(), (288, 216));
        assert_eq!(
            image::image_dimensions(small_columns_path).unwrap(),
            (200, 100)
        );
    }

    #[test]
    fn test_subplot_save_with_dpi_rejects_invalid_or_oversized_output() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid.png");

        for dpi in [f32::NAN, 0.0, 50.0, 3000.0] {
            assert!(
                SubplotFigure::new(1, 1, 400, 300)
                    .unwrap()
                    .save_with_dpi(&path, dpi)
                    .is_err(),
                "dpi={dpi} should fail"
            );
        }

        assert!(
            SubplotFigure::new(1, 1, u32::MAX, 300)
                .unwrap()
                .save(&path)
                .is_err()
        );
        assert!(subplots_default(1, usize::MAX).is_err());
    }

    #[test]
    fn test_subplot_save_rejects_malformed_child_margins_without_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid-child-margins.png");
        let plot = Plot::with_config(crate::core::PlotConfig {
            margins: crate::core::MarginConfig::content_driven_custom(f32::MAX, true),
            ..crate::core::PlotConfig::default()
        });

        let err = SubplotFigure::new(1, 1, 400, 300)
            .unwrap()
            .subplot_at(0, plot)
            .unwrap()
            .save(path)
            .expect_err("malformed child margins should return an error");

        assert!(matches!(err, PlottingError::InvalidInput(_)));
    }

    #[test]
    fn test_subplot_with_different_themes() {
        use crate::render::Theme;

        let x = vec![1.0, 2.0, 3.0];
        let y1 = vec![2.0, 4.0, 3.0];
        let y2 = vec![1.0, 3.0, 2.0];

        let plot1 = Plot::new()
            .line(&x, &y1)
            .end_series()
            .theme(Theme::default())
            .title("Default Theme");

        let plot2 = Plot::new()
            .line(&x, &y2)
            .end_series()
            .theme(Theme::dark())
            .title("Dark Theme");

        let figure = SubplotFigure::new(1, 2, 800, 400)
            .unwrap()
            .subplot(0, 0, plot1)
            .unwrap()
            .subplot(0, 1, plot2)
            .unwrap();

        assert_eq!(figure.subplot_count(), 2);

        // Verify themes are preserved
        let spec = figure.grid_spec();
        assert_eq!(spec.rows, 1);
        assert_eq!(spec.cols, 2);
    }

    #[test]
    fn test_subplot_suptitle_and_spacing() {
        let plot = Plot::new();

        let figure = SubplotFigure::new(2, 2, 800, 600)
            .unwrap()
            .suptitle("Overall Title")
            .hspace(0.4)
            .wspace(0.5)
            .subplot_at(0, plot)
            .unwrap();

        assert_eq!(figure.subplot_count(), 1);
        assert_eq!(figure.grid_spec().hspace, 0.4);
        assert_eq!(figure.grid_spec().wspace, 0.5);
        assert!(figure.suptitle.is_some());
        assert_eq!(figure.suptitle.as_ref().unwrap(), "Overall Title");
    }

    #[test]
    fn test_empty_subplot_figure() {
        let figure = SubplotFigure::new(2, 2, 800, 600).unwrap();

        assert_eq!(figure.subplot_count(), 0);
        assert_eq!(figure.grid_spec().total_subplots(), 4);

        // Empty figure should still be valid for adding plots later
        let plot = Plot::new();
        let updated_figure = figure.subplot(1, 1, plot).unwrap();
        assert_eq!(updated_figure.subplot_count(), 1);
    }

    #[test]
    fn test_large_subplot_grid() {
        // Test performance with larger grids
        let result = SubplotFigure::new(5, 4, 1200, 900);
        assert!(result.is_ok());

        let figure = result.unwrap();
        assert_eq!(figure.grid_spec().total_subplots(), 20);

        // Test bounds - should be within the 10x10 limit
        let large_result = SubplotFigure::new(10, 10, 2000, 2000);
        assert!(large_result.is_ok());

        // Should fail - exceeds 10x10 limit
        let too_large_result = SubplotFigure::new(11, 10, 2000, 2000);
        assert!(too_large_result.is_err());
    }
}
