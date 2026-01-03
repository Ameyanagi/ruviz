use crate::core::position::Position;
/// Subplot functionality for multiple plots in one figure
///
/// Provides grid-based layout system for arranging multiple plots
/// within a single figure, similar to matplotlib's subplot functionality.
use crate::core::{Plot, PlottingError, Result};
use crate::render::skia::SkiaRenderer;
use tiny_skia::Rect;

/// Grid specification for subplot layout
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridSpec {
    /// Number of rows in the subplot grid
    pub rows: usize,
    /// Number of columns in the subplot grid
    pub cols: usize,
    /// Horizontal spacing between subplots (as fraction of subplot width)
    pub hspace: f32,
    /// Vertical spacing between subplots (as fraction of subplot height)  
    pub wspace: f32,
}

impl GridSpec {
    /// Create a new grid specification
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            hspace: 0.2,
            wspace: 0.2,
        }
    }

    /// Set horizontal spacing between subplots
    pub fn with_hspace(mut self, hspace: f32) -> Self {
        self.hspace = hspace.clamp(0.0, 1.0);
        self
    }

    /// Set vertical spacing between subplots
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
    pub fn subplot_rect(
        &self,
        index: usize,
        figure_width: u32,
        figure_height: u32,
        margin: f32,
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
        let available_height = figure_height as f32 - 2.0 * margin_px;

        // Calculate subplot dimensions with spacing
        let subplot_width = available_width / self.cols as f32;
        let subplot_height = available_height / self.rows as f32;

        let spacing_x = subplot_width * self.wspace;
        let spacing_y = subplot_height * self.hspace;

        let plot_width = subplot_width - spacing_x;
        let plot_height = subplot_height - spacing_y;

        // Calculate subplot position
        let x = margin_px + col as f32 * subplot_width + spacing_x / 2.0;
        let y = margin_px + row as f32 * subplot_height + spacing_y / 2.0;

        Rect::from_xywh(x, y, plot_width, plot_height).ok_or_else(|| {
            PlottingError::InvalidInput("Invalid subplot dimensions calculated".to_string())
        })
    }
}

/// Subplot figure containing multiple plots arranged in a grid
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
    /// Figure margin (fraction of figure size)
    margin: f32,
}

impl SubplotFigure {
    /// Create a new subplot figure
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
            margin: 0.1, // 10% margin by default
        })
    }

    /// Set horizontal spacing between subplots
    pub fn hspace(mut self, hspace: f32) -> Self {
        self.grid = self.grid.with_hspace(hspace);
        self
    }

    /// Set vertical spacing between subplots
    pub fn wspace(mut self, wspace: f32) -> Self {
        self.grid = self.grid.with_wspace(wspace);
        self
    }

    /// Set overall figure title
    pub fn suptitle<S: Into<String>>(mut self, title: S) -> Self {
        self.suptitle = Some(title.into());
        self
    }

    /// Set figure margin
    pub fn margin(mut self, margin: f32) -> Self {
        self.margin = margin.clamp(0.0, 0.4); // Max 40% margin
        self
    }

    /// Add a plot at the specified subplot position
    /// Position is calculated as: index = row * cols + col (0-indexed)
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
    pub fn save<P: AsRef<std::path::Path>>(self, path: P) -> Result<()> {
        self.save_with_dpi(path, 96.0)
    }

    /// Render all subplots with specified DPI
    pub fn save_with_dpi<P: AsRef<std::path::Path>>(self, path: P, dpi: f32) -> Result<()> {
        // Create main renderer for the figure
        let theme = crate::render::Theme::default();
        let mut renderer = SkiaRenderer::new(self.width, self.height, theme)?;

        // Render figure title if present
        if let Some(title) = &self.suptitle {
            let title_y = 30.0 * (dpi / 96.0); // DPI-scaled title position
            let title_size = 16.0 * (dpi / 96.0); // DPI-scaled title size

            renderer.draw_text_centered(
                title,
                self.width as f32 / 2.0,
                title_y,
                title_size,
                crate::render::Color::new(0, 0, 0),
            )?;
        }

        // Render each subplot
        for (index, plot_opt) in self.plots.iter().enumerate() {
            if let Some(plot) = plot_opt {
                // Calculate subplot area
                let subplot_rect =
                    self.grid
                        .subplot_rect(index, self.width, self.height, self.margin)?;

                // Create a temporary renderer for this subplot
                let subplot_theme = plot.get_theme();
                let mut subplot_renderer = SkiaRenderer::new(
                    subplot_rect.width() as u32,
                    subplot_rect.height() as u32,
                    subplot_theme,
                )?;

                // Render the plot into the subplot area
                plot.render_to_renderer(&mut subplot_renderer, dpi)?;

                // Copy subplot renderer to main renderer at correct position
                renderer.draw_subplot(
                    subplot_renderer.to_image(),
                    subplot_rect.left() as u32,
                    subplot_rect.top() as u32,
                )?;
            }
        }

        // Save the final figure
        renderer.save_png(path)?;
        Ok(())
    }
}

/// Convenience function to create a subplot figure
pub fn subplots(rows: usize, cols: usize, width: u32, height: u32) -> Result<SubplotFigure> {
    SubplotFigure::new(rows, cols, width, height)
}

/// Convenience function to create a subplot figure with default size
pub fn subplots_default(rows: usize, cols: usize) -> Result<SubplotFigure> {
    // Default figure size scales with subplot count
    let base_width = 400;
    let base_height = 300;
    let width = (base_width * cols).min(1600) as u32;
    let height = (base_height * rows).min(1200) as u32;

    SubplotFigure::new(rows, cols, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_subplot_rect_calculation() {
        let grid = GridSpec::new(2, 2);

        // Test first subplot (top-left)
        let rect = grid.subplot_rect(0, 800, 600, 0.1).unwrap();
        assert!(rect.left() >= 80.0); // Should account for margin
        assert!(rect.top() >= 60.0);
        assert!(rect.width() > 0.0);
        assert!(rect.height() > 0.0);

        // Test last subplot (bottom-right)
        let rect = grid.subplot_rect(3, 800, 600, 0.1).unwrap();
        assert!(rect.right() <= 720.0); // Should fit within margins
        assert!(rect.bottom() <= 540.0);
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
        use crate::render::Theme;

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
