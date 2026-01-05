/// Box plot implementation with statistical analysis and outlier detection
use crate::core::style_utils::{StyleResolver, defaults};
use crate::core::{PlottingError, Result};
use crate::data::Data1D;
use crate::plots::traits::{PlotArea, PlotConfig, PlotData, PlotRender};
use crate::render::{Color, LineStyle, MarkerStyle, SkiaRenderer, Theme};

/// Configuration for box plots
#[derive(Debug, Clone)]
pub struct BoxPlotConfig {
    /// Method for calculating outliers
    pub outlier_method: OutlierMethod,
    /// Whether to show outliers as individual points
    pub show_outliers: bool,
    /// Whether to show means as additional markers
    pub show_mean: bool,
    /// Box plot orientation
    pub orientation: BoxOrientation,
    /// Whisker calculation method
    pub whisker_method: WhiskerMethod,
    /// Fill alpha (opacity), default from defaults::BOXPLOT_FILL_ALPHA
    pub fill_alpha: Option<f32>,
    /// Edge color (auto-derived from fill if None)
    pub edge_color: Option<Color>,
    /// Edge width in points (default from defaults::PATCH_LINE_WIDTH)
    pub edge_width: Option<f32>,
    /// Box width ratio (fraction of available space, default 0.5)
    pub width_ratio: Option<f32>,
    /// Whisker line width (None = use theme.line_width)
    pub whisker_width: Option<f32>,
    /// Median line width (default: theme.line_width * 1.5)
    pub median_width: Option<f32>,
    /// Cap width as fraction of box width (default 0.5)
    pub cap_width: Option<f32>,
    /// Outlier marker size (default 6.0)
    pub flier_size: Option<f32>,
}

/// Methods for detecting outliers
#[allow(clippy::upper_case_acronyms)] // IQR is the standard statistics acronym
#[derive(Debug, Clone, Copy)]
pub enum OutlierMethod {
    /// Standard IQR method: outliers beyond 1.5 * IQR from quartiles
    IQR,
    /// Modified IQR method: outliers beyond 2.5 * IQR from quartiles
    ModifiedIQR,
    /// Standard deviation method: outliers beyond n standard deviations
    StandardDeviation(f64),
    /// No outlier detection
    None,
}

/// Box plot orientation
#[derive(Debug, Clone, Copy)]
pub enum BoxOrientation {
    /// Vertical box plots
    Vertical,
    /// Horizontal box plots
    Horizontal,
}

/// Methods for calculating whiskers
#[derive(Debug, Clone, Copy)]
pub enum WhiskerMethod {
    /// Whiskers extend to min/max values within 1.5 * IQR from quartiles
    Tukey,
    /// Whiskers extend to actual min/max of data
    MinMax,
    /// Whiskers extend to 5th and 95th percentiles
    Percentile5_95,
    /// Whiskers extend to 10th and 90th percentiles
    Percentile10_90,
}

/// Computed box plot statistics
#[derive(Debug, Clone)]
pub struct BoxPlotData {
    /// Minimum value (or lower whisker)
    pub min: f64,
    /// First quartile (Q1, 25th percentile)
    pub q1: f64,
    /// Median (Q2, 50th percentile)
    pub median: f64,
    /// Third quartile (Q3, 75th percentile)
    pub q3: f64,
    /// Maximum value (or upper whisker)
    pub max: f64,
    /// Mean value (optional)
    pub mean: Option<f64>,
    /// Outlier values
    pub outliers: Vec<f64>,
    /// Total number of data points
    pub n_samples: usize,
    /// Interquartile range (Q3 - Q1)
    pub iqr: f64,
    /// Box orientation
    pub orientation: BoxOrientation,
    /// Fill alpha for box
    pub fill_alpha: f32,
    /// Edge color (None = auto-derive)
    pub edge_color: Option<Color>,
    /// Edge width in points
    pub edge_width: f32,
    /// Box width ratio
    pub width_ratio: f32,
    /// Whisker line width
    pub whisker_width: Option<f32>,
    /// Median line width
    pub median_width: Option<f32>,
    /// Cap width as fraction of box width
    pub cap_width: f32,
    /// Outlier marker size
    pub flier_size: f32,
    /// Whether to show outliers
    pub show_outliers: bool,
    /// Whether to show mean
    pub show_mean: bool,
}

// Implement PlotData trait for BoxPlotData
impl PlotData for BoxPlotData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        // For vertical box plots, x is typically categorical (0, 1, 2, etc.)
        // and y spans the data range
        let (y_min, y_max) = if self.outliers.is_empty() {
            (self.min, self.max)
        } else {
            let outlier_min = self.outliers.iter().copied().fold(f64::INFINITY, f64::min);
            let outlier_max = self
                .outliers
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
            (self.min.min(outlier_min), self.max.max(outlier_max))
        };

        match self.orientation {
            BoxOrientation::Vertical => ((-0.5, 0.5), (y_min, y_max)),
            BoxOrientation::Horizontal => ((y_min, y_max), (-0.5, 0.5)),
        }
    }

    fn is_empty(&self) -> bool {
        self.n_samples == 0
    }
}

// Implement PlotRender trait for BoxPlotData
impl PlotRender for BoxPlotData {
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        theme: &Theme,
        color: Color,
    ) -> Result<()> {
        self.render_styled(
            renderer,
            area,
            theme,
            color,
            self.fill_alpha,
            Some(self.edge_width),
        )
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

        let resolver = StyleResolver::new(theme);

        // Resolve styling
        let fill_alpha = alpha.clamp(0.0, 1.0);
        let fill_color = color.with_alpha(fill_alpha);
        let edge_color = resolver.edge_color(color, self.edge_color);
        let edge_width = resolver.patch_line_width(line_width);
        let whisker_width = self.whisker_width.unwrap_or(theme.line_width);
        let median_width = self.median_width.unwrap_or(theme.line_width * 1.5);

        // For rendering, use the center position (x=0 for single box)
        let center_x = 0.0;

        match self.orientation {
            BoxOrientation::Vertical => {
                self.render_vertical(
                    renderer,
                    area,
                    center_x,
                    fill_color,
                    edge_color,
                    edge_width,
                    whisker_width,
                    median_width,
                    color,
                )?;
            }
            BoxOrientation::Horizontal => {
                self.render_horizontal(
                    renderer,
                    area,
                    center_x,
                    fill_color,
                    edge_color,
                    edge_width,
                    whisker_width,
                    median_width,
                    color,
                )?;
            }
        }

        Ok(())
    }
}

impl BoxPlotData {
    /// Render a vertical box plot
    fn render_vertical(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        center_x: f64,
        fill_color: Color,
        edge_color: Color,
        edge_width: f32,
        whisker_width: f32,
        median_width: f32,
        marker_color: Color,
    ) -> Result<()> {
        // Calculate box dimensions
        let box_half_width = (self.width_ratio * 0.5) as f64;
        let cap_half_width = box_half_width * self.cap_width as f64;

        // Convert key y-values to screen coordinates
        let (left_x, _) = area.data_to_screen(center_x - box_half_width, self.q1);
        let (right_x, _) = area.data_to_screen(center_x + box_half_width, self.q1);
        let (_, q1_y) = area.data_to_screen(center_x, self.q1);
        let (_, q3_y) = area.data_to_screen(center_x, self.q3);
        let (_, median_y) = area.data_to_screen(center_x, self.median);
        let (_, min_y) = area.data_to_screen(center_x, self.min);
        let (_, max_y) = area.data_to_screen(center_x, self.max);
        let (center_screen_x, _) = area.data_to_screen(center_x, 0.0);

        // Draw box (filled rectangle from Q1 to Q3)
        let box_x = left_x;
        let box_y = q3_y.min(q1_y); // q3_y is higher on screen (smaller y)
        let box_width = right_x - left_x;
        let box_height = (q1_y - q3_y).abs();

        renderer.draw_rectangle(box_x, box_y, box_width, box_height, fill_color, true)?;

        // Draw box edge
        if edge_width > 0.0 {
            let vertices = [
                (box_x, box_y),
                (box_x + box_width, box_y),
                (box_x + box_width, box_y + box_height),
                (box_x, box_y + box_height),
            ];
            renderer.draw_polygon_outline(&vertices, edge_color, edge_width)?;
        }

        // Draw median line
        renderer.draw_line(
            left_x,
            median_y,
            right_x,
            median_y,
            edge_color,
            median_width,
            LineStyle::Solid,
        )?;

        // Draw whiskers (vertical lines from Q1 to min, and Q3 to max)
        renderer.draw_line(
            center_screen_x,
            q1_y,
            center_screen_x,
            min_y,
            edge_color,
            whisker_width,
            LineStyle::Solid,
        )?;
        renderer.draw_line(
            center_screen_x,
            q3_y,
            center_screen_x,
            max_y,
            edge_color,
            whisker_width,
            LineStyle::Solid,
        )?;

        // Draw caps (horizontal lines at min and max)
        let (cap_left, _) = area.data_to_screen(center_x - cap_half_width, self.min);
        let (cap_right, _) = area.data_to_screen(center_x + cap_half_width, self.min);
        renderer.draw_line(
            cap_left,
            min_y,
            cap_right,
            min_y,
            edge_color,
            whisker_width,
            LineStyle::Solid,
        )?;
        renderer.draw_line(
            cap_left,
            max_y,
            cap_right,
            max_y,
            edge_color,
            whisker_width,
            LineStyle::Solid,
        )?;

        // Draw outliers
        if self.show_outliers && !self.outliers.is_empty() {
            for &outlier in &self.outliers {
                let (ox, oy) = area.data_to_screen(center_x, outlier);
                renderer.draw_marker(ox, oy, self.flier_size, MarkerStyle::Circle, marker_color)?;
            }
        }

        // Draw mean if present
        if self.show_mean {
            if let Some(mean_val) = self.mean {
                let (mx, my) = area.data_to_screen(center_x, mean_val);
                // Diamond marker for mean
                renderer.draw_marker(
                    mx,
                    my,
                    self.flier_size,
                    MarkerStyle::Diamond,
                    marker_color,
                )?;
            }
        }

        Ok(())
    }

    /// Render a horizontal box plot
    fn render_horizontal(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        center_y: f64,
        fill_color: Color,
        edge_color: Color,
        edge_width: f32,
        whisker_width: f32,
        median_width: f32,
        marker_color: Color,
    ) -> Result<()> {
        // Calculate box dimensions
        let box_half_height = (self.width_ratio * 0.5) as f64;
        let cap_half_height = box_half_height * self.cap_width as f64;

        // Convert key x-values to screen coordinates
        let (q1_x, _) = area.data_to_screen(self.q1, center_y);
        let (q3_x, _) = area.data_to_screen(self.q3, center_y);
        let (median_x, _) = area.data_to_screen(self.median, center_y);
        let (min_x, _) = area.data_to_screen(self.min, center_y);
        let (max_x, _) = area.data_to_screen(self.max, center_y);
        let (_, top_y) = area.data_to_screen(self.q1, center_y - box_half_height);
        let (_, bottom_y) = area.data_to_screen(self.q1, center_y + box_half_height);
        let (_, center_screen_y) = area.data_to_screen(0.0, center_y);

        // Draw box (filled rectangle from Q1 to Q3)
        let box_x = q1_x.min(q3_x);
        let box_y = top_y.min(bottom_y);
        let box_width = (q3_x - q1_x).abs();
        let box_height = (bottom_y - top_y).abs();

        renderer.draw_rectangle(box_x, box_y, box_width, box_height, fill_color, true)?;

        // Draw box edge
        if edge_width > 0.0 {
            let vertices = [
                (box_x, box_y),
                (box_x + box_width, box_y),
                (box_x + box_width, box_y + box_height),
                (box_x, box_y + box_height),
            ];
            renderer.draw_polygon_outline(&vertices, edge_color, edge_width)?;
        }

        // Draw median line (vertical for horizontal box plot)
        renderer.draw_line(
            median_x,
            top_y,
            median_x,
            bottom_y,
            edge_color,
            median_width,
            LineStyle::Solid,
        )?;

        // Draw whiskers (horizontal lines from Q1 to min, and Q3 to max)
        renderer.draw_line(
            q1_x,
            center_screen_y,
            min_x,
            center_screen_y,
            edge_color,
            whisker_width,
            LineStyle::Solid,
        )?;
        renderer.draw_line(
            q3_x,
            center_screen_y,
            max_x,
            center_screen_y,
            edge_color,
            whisker_width,
            LineStyle::Solid,
        )?;

        // Draw caps (vertical lines at min and max)
        let (_, cap_top) = area.data_to_screen(self.min, center_y - cap_half_height);
        let (_, cap_bottom) = area.data_to_screen(self.min, center_y + cap_half_height);
        renderer.draw_line(
            min_x,
            cap_top,
            min_x,
            cap_bottom,
            edge_color,
            whisker_width,
            LineStyle::Solid,
        )?;
        renderer.draw_line(
            max_x,
            cap_top,
            max_x,
            cap_bottom,
            edge_color,
            whisker_width,
            LineStyle::Solid,
        )?;

        // Draw outliers
        if self.show_outliers && !self.outliers.is_empty() {
            for &outlier in &self.outliers {
                let (ox, oy) = area.data_to_screen(outlier, center_y);
                renderer.draw_marker(ox, oy, self.flier_size, MarkerStyle::Circle, marker_color)?;
            }
        }

        // Draw mean if present
        if self.show_mean {
            if let Some(mean_val) = self.mean {
                let (mx, my) = area.data_to_screen(mean_val, center_y);
                renderer.draw_marker(
                    mx,
                    my,
                    self.flier_size,
                    MarkerStyle::Diamond,
                    marker_color,
                )?;
            }
        }

        Ok(())
    }
}

impl Default for BoxPlotConfig {
    fn default() -> Self {
        Self {
            outlier_method: OutlierMethod::IQR,
            show_outliers: true,
            show_mean: false,
            orientation: BoxOrientation::Vertical,
            whisker_method: WhiskerMethod::Tukey,
            fill_alpha: None,
            edge_color: None,
            edge_width: None,
            width_ratio: None,
            whisker_width: None,
            median_width: None,
            cap_width: None,
            flier_size: None,
        }
    }
}

// Implement PlotConfig marker trait
impl PlotConfig for BoxPlotConfig {}

impl BoxPlotConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn outlier_method(mut self, method: OutlierMethod) -> Self {
        self.outlier_method = method;
        self
    }

    pub fn show_outliers(mut self, show: bool) -> Self {
        self.show_outliers = show;
        self
    }

    pub fn show_mean(mut self, show: bool) -> Self {
        self.show_mean = show;
        self
    }

    pub fn orientation(mut self, orientation: BoxOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn whisker_method(mut self, method: WhiskerMethod) -> Self {
        self.whisker_method = method;
        self
    }

    /// Set fill alpha (0.0-1.0)
    pub fn fill_alpha(mut self, alpha: f32) -> Self {
        self.fill_alpha = Some(alpha.clamp(0.0, 1.0));
        self
    }

    /// Set edge color explicitly
    pub fn edge_color(mut self, color: Color) -> Self {
        self.edge_color = Some(color);
        self
    }

    /// Set edge width in points
    pub fn edge_width(mut self, width: f32) -> Self {
        self.edge_width = Some(width);
        self
    }

    /// Set box width ratio (fraction of available space)
    pub fn width_ratio(mut self, ratio: f32) -> Self {
        self.width_ratio = Some(ratio.clamp(0.0, 1.0));
        self
    }

    /// Set whisker line width
    pub fn whisker_width(mut self, width: f32) -> Self {
        self.whisker_width = Some(width);
        self
    }

    /// Set median line width
    pub fn median_width(mut self, width: f32) -> Self {
        self.median_width = Some(width);
        self
    }

    /// Set cap width as fraction of box width
    pub fn cap_width(mut self, width: f32) -> Self {
        self.cap_width = Some(width.clamp(0.0, 1.0));
        self
    }

    /// Set outlier marker size
    pub fn flier_size(mut self, size: f32) -> Self {
        self.flier_size = Some(size);
        self
    }
}

/// Calculate box plot statistics from data
pub fn calculate_box_plot<T, D: Data1D<T>>(data: &D, config: &BoxPlotConfig) -> Result<BoxPlotData>
where
    T: Into<f64> + Copy,
{
    // Collect and validate data using shared utility
    let values = crate::data::collect_finite_values_sorted(data)?;
    let n_samples = values.len();

    // Calculate quartiles
    let q1 = calculate_percentile(&values, 25.0);
    let median = calculate_percentile(&values, 50.0);
    let q3 = calculate_percentile(&values, 75.0);
    let iqr = q3 - q1;

    // Calculate mean if requested
    let mean = if config.show_mean {
        Some(values.iter().sum::<f64>() / n_samples as f64)
    } else {
        None
    };

    // Calculate whiskers and outliers
    let (whisker_min, whisker_max, outliers) =
        calculate_whiskers_and_outliers(&values, q1, q3, iqr, config);

    // Extract styling from config, using defaults for None values
    let fill_alpha = config.fill_alpha.unwrap_or(defaults::BOXPLOT_FILL_ALPHA);
    let edge_color = config.edge_color;
    let edge_width = config.edge_width.unwrap_or(defaults::PATCH_LINE_WIDTH);
    let width_ratio = config.width_ratio.unwrap_or(defaults::BOXPLOT_WIDTH_RATIO);
    let whisker_width = config.whisker_width;
    let median_width = config.median_width;
    let cap_width = config.cap_width.unwrap_or(defaults::BOXPLOT_CAP_WIDTH);
    let flier_size = config.flier_size.unwrap_or(defaults::FLIER_SIZE);

    Ok(BoxPlotData {
        min: whisker_min,
        q1,
        median,
        q3,
        max: whisker_max,
        mean,
        outliers,
        n_samples,
        iqr,
        orientation: config.orientation,
        fill_alpha,
        edge_color,
        edge_width,
        width_ratio,
        whisker_width,
        median_width,
        cap_width,
        flier_size,
        show_outliers: config.show_outliers,
        show_mean: config.show_mean,
    })
}

// Use shared statistical utilities
use super::statistics::percentile as calculate_percentile;

fn calculate_whiskers_and_outliers(
    values: &[f64],
    q1: f64,
    q3: f64,
    iqr: f64,
    config: &BoxPlotConfig,
) -> (f64, f64, Vec<f64>) {
    let mut outliers = Vec::new();

    // When OutlierMethod::None, whiskers should extend to min/max regardless of whisker_method
    let (whisker_min, whisker_max) = if matches!(config.outlier_method, OutlierMethod::None) {
        (values[0], values[values.len() - 1])
    } else {
        match config.whisker_method {
            WhiskerMethod::Tukey => {
                let lower_bound = q1 - 1.5 * iqr;
                let upper_bound = q3 + 1.5 * iqr;

                // Find whiskers as furthest non-outlier points
                let whisker_min = values
                    .iter()
                    .find(|&&x| x >= lower_bound)
                    .copied()
                    .unwrap_or(values[0]);
                let whisker_max = values
                    .iter()
                    .rev()
                    .find(|&&x| x <= upper_bound)
                    .copied()
                    .unwrap_or(values[values.len() - 1]);

                (whisker_min, whisker_max)
            }
            WhiskerMethod::MinMax => (values[0], values[values.len() - 1]),
            WhiskerMethod::Percentile5_95 => (
                calculate_percentile(values, 5.0),
                calculate_percentile(values, 95.0),
            ),
            WhiskerMethod::Percentile10_90 => (
                calculate_percentile(values, 10.0),
                calculate_percentile(values, 90.0),
            ),
        }
    };

    // Detect outliers based on configuration
    if config.show_outliers {
        let (lower_threshold, upper_threshold) = match config.outlier_method {
            OutlierMethod::IQR => (q1 - 1.5 * iqr, q3 + 1.5 * iqr),
            OutlierMethod::ModifiedIQR => (q1 - 2.5 * iqr, q3 + 2.5 * iqr),
            OutlierMethod::StandardDeviation(n_std) => {
                let mean = values.iter().sum::<f64>() / values.len() as f64;
                let std_dev = (values.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
                    / values.len() as f64)
                    .sqrt();
                (mean - n_std * std_dev, mean + n_std * std_dev)
            }
            OutlierMethod::None => (f64::NEG_INFINITY, f64::INFINITY),
        };

        for &value in values {
            if value < lower_threshold || value > upper_threshold {
                outliers.push(value);
            }
        }
    }

    (whisker_min, whisker_max, outliers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_plot_basic_functionality() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let config = BoxPlotConfig::new();

        let result = calculate_box_plot(&data, &config).unwrap();

        assert_eq!(result.n_samples, 10);
        assert_eq!(result.median, 5.5); // Median of 1-10
        assert_eq!(result.q1, 3.25); // Q1
        assert_eq!(result.q3, 7.75); // Q3  
        assert_eq!(result.iqr, 4.5); // Q3 - Q1
        assert!(result.outliers.is_empty()); // No outliers in uniform data
    }

    #[test]
    fn test_box_plot_empty_data() {
        let data: Vec<f64> = vec![];
        let config = BoxPlotConfig::new();

        let result = calculate_box_plot(&data, &config);
        assert!(result.is_err());

        match result.unwrap_err() {
            PlottingError::EmptyDataSet => {}
            _ => panic!("Expected EmptyDataSet error"),
        }
    }

    #[test]
    fn test_box_plot_with_outliers() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 100.0]; // 100 is outlier
        let config = BoxPlotConfig::new();

        let result = calculate_box_plot(&data, &config).unwrap();

        assert_eq!(result.n_samples, 10);
        assert!(!result.outliers.is_empty());
        assert!(result.outliers.contains(&100.0));
        assert!(result.max < 100.0); // Whisker should not extend to outlier
    }

    #[test]
    fn test_box_plot_no_outliers_config() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 100.0]; // 100 would be outlier
        let config = BoxPlotConfig::new().outlier_method(OutlierMethod::None);

        let result = calculate_box_plot(&data, &config).unwrap();

        assert!(result.outliers.is_empty()); // No outliers detected
        assert_eq!(result.max, 100.0); // Max includes extreme value
    }

    #[test]
    fn test_box_plot_with_mean() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let config = BoxPlotConfig::new().show_mean(true);

        let result = calculate_box_plot(&data, &config).unwrap();

        assert!(result.mean.is_some());
        assert_eq!(result.mean.unwrap(), 3.0); // Mean of 1-5
    }

    #[test]
    fn test_box_plot_without_mean() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let config = BoxPlotConfig::new().show_mean(false);

        let result = calculate_box_plot(&data, &config).unwrap();

        assert!(result.mean.is_none());
    }

    #[test]
    fn test_box_plot_single_value() {
        let data = vec![5.0];
        let config = BoxPlotConfig::new();

        let result = calculate_box_plot(&data, &config).unwrap();

        assert_eq!(result.median, 5.0);
        assert_eq!(result.q1, 5.0);
        assert_eq!(result.q3, 5.0);
        assert_eq!(result.min, 5.0);
        assert_eq!(result.max, 5.0);
        assert_eq!(result.iqr, 0.0);
    }

    #[test]
    fn test_box_plot_identical_values() {
        let data = vec![5.0; 100]; // 100 identical values
        let config = BoxPlotConfig::new();

        let result = calculate_box_plot(&data, &config).unwrap();

        assert_eq!(result.median, 5.0);
        assert_eq!(result.q1, 5.0);
        assert_eq!(result.q3, 5.0);
        assert_eq!(result.iqr, 0.0);
        assert!(result.outliers.is_empty());
    }

    #[test]
    fn test_box_plot_whisker_methods() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];

        // Test Tukey method (default)
        let tukey_config = BoxPlotConfig::new().whisker_method(WhiskerMethod::Tukey);
        let tukey_result = calculate_box_plot(&data, &tukey_config).unwrap();

        // Test MinMax method
        let minmax_config = BoxPlotConfig::new().whisker_method(WhiskerMethod::MinMax);
        let minmax_result = calculate_box_plot(&data, &minmax_config).unwrap();

        assert_eq!(minmax_result.min, 1.0);
        assert_eq!(minmax_result.max, 10.0);

        // MinMax whiskers should extend further than or equal to Tukey
        assert!(minmax_result.min <= tukey_result.min);
        assert!(minmax_result.max >= tukey_result.max);
    }

    #[test]
    fn test_box_plot_percentile_whiskers() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();

        let config = BoxPlotConfig::new().whisker_method(WhiskerMethod::Percentile5_95);
        let result = calculate_box_plot(&data, &config).unwrap();

        // For data 1-100, 5th percentile should be around 5 and 95th around 95
        assert!((result.min - 5.0).abs() < 1.0);
        assert!((result.max - 95.0).abs() < 1.0);
    }

    #[test]
    fn test_box_plot_with_nan_values() {
        let data = vec![1.0, 2.0, f64::NAN, 4.0, 5.0];
        let config = BoxPlotConfig::new();

        let result = calculate_box_plot(&data, &config).unwrap();

        // NaN values should be filtered out
        assert_eq!(result.n_samples, 4); // Only finite values counted
        assert_eq!(result.median, 3.0); // Median of [1,2,4,5] = (2+4)/2 = 3.0
    }

    #[test]
    fn test_box_plot_modified_iqr_outliers() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 15.0]; // 15 might be outlier

        let standard_config = BoxPlotConfig::new().outlier_method(OutlierMethod::IQR);
        let modified_config = BoxPlotConfig::new().outlier_method(OutlierMethod::ModifiedIQR);

        let standard_result = calculate_box_plot(&data, &standard_config).unwrap();
        let modified_result = calculate_box_plot(&data, &modified_config).unwrap();

        // Modified IQR should be more lenient, potentially fewer outliers
        assert!(modified_result.outliers.len() <= standard_result.outliers.len());
    }

    #[test]
    fn test_box_plot_standard_deviation_outliers() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 20.0]; // 20 is outlier
        let config = BoxPlotConfig::new().outlier_method(OutlierMethod::StandardDeviation(2.0));

        let result = calculate_box_plot(&data, &config).unwrap();

        // Should detect outliers beyond 2 standard deviations
        assert!(!result.outliers.is_empty());
    }

    #[test]
    fn test_percentile_calculation() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        assert_eq!(calculate_percentile(&data, 0.0), 1.0); // Min
        assert_eq!(calculate_percentile(&data, 50.0), 3.0); // Median
        assert_eq!(calculate_percentile(&data, 100.0), 5.0); // Max
        assert_eq!(calculate_percentile(&data, 25.0), 2.0); // Q1
        assert_eq!(calculate_percentile(&data, 75.0), 4.0); // Q3
    }

    #[test]
    fn test_box_plot_config_builder() {
        let config = BoxPlotConfig::new()
            .outlier_method(OutlierMethod::ModifiedIQR)
            .show_outliers(false)
            .show_mean(true)
            .orientation(BoxOrientation::Horizontal)
            .whisker_method(WhiskerMethod::Percentile10_90);

        match config.outlier_method {
            OutlierMethod::ModifiedIQR => {}
            _ => panic!("Expected ModifiedIQR"),
        }
        assert!(!config.show_outliers);
        assert!(config.show_mean);
        match config.orientation {
            BoxOrientation::Horizontal => {}
            _ => panic!("Expected Horizontal"),
        }
        match config.whisker_method {
            WhiskerMethod::Percentile10_90 => {}
            _ => panic!("Expected Percentile10_90"),
        }
    }

    #[test]
    fn test_plot_data_trait() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let config = BoxPlotConfig::new();
        let boxplot = calculate_box_plot(&data, &config).unwrap();

        // Test data_bounds: returns ((x_min, x_max), (y_min, y_max))
        let ((_x_min, _x_max), (y_min, y_max)) = boxplot.data_bounds();
        // Bounds should include y range (min to max whisker values)
        assert!(y_min <= 1.0);
        assert!(y_max >= 10.0);

        // Test is_empty
        assert!(!boxplot.is_empty());
    }

    #[test]
    fn test_styling_fields() {
        let config = BoxPlotConfig::new()
            .fill_alpha(0.5)
            .edge_color(Color::new(255, 0, 0))
            .edge_width(2.0)
            .width_ratio(0.8)
            .whisker_width(1.5)
            .median_width(2.5)
            .cap_width(0.6)
            .flier_size(8.0);

        assert_eq!(config.fill_alpha, Some(0.5));
        assert!(config.edge_color.is_some());
        assert_eq!(config.edge_width, Some(2.0));
        assert_eq!(config.width_ratio, Some(0.8));
        assert_eq!(config.whisker_width, Some(1.5));
        assert_eq!(config.median_width, Some(2.5));
        assert_eq!(config.cap_width, Some(0.6));
        assert_eq!(config.flier_size, Some(8.0));

        // Test that styling propagates to BoxPlotData
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let boxplot = calculate_box_plot(&data, &config).unwrap();

        assert_eq!(boxplot.fill_alpha, 0.5);
        assert!(boxplot.edge_color.is_some());
        assert_eq!(boxplot.edge_width, 2.0);
        assert_eq!(boxplot.width_ratio, 0.8);
    }
}
