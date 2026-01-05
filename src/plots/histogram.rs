/// Histogram plot implementation with automatic binning and statistical analysis
use crate::core::style_utils::{StyleResolver, defaults};
use crate::core::{PlottingError, Result};
use crate::data::Data1D;
use crate::plots::traits::{PlotArea, PlotConfig, PlotData, PlotRender};
use crate::render::{Color, SkiaRenderer, Theme};

/// Configuration for histogram plots
#[derive(Debug, Clone)]
pub struct HistogramConfig {
    /// Number of bins (auto-calculated if None)
    pub bins: Option<usize>,
    /// Range for histogram (auto-calculated if None)
    pub range: Option<(f64, f64)>,
    /// Whether to normalize to probability density
    pub density: bool,
    /// Whether to show cumulative distribution
    pub cumulative: bool,
    /// Bin edges calculation method
    pub bin_method: BinMethod,
    /// Fill alpha (opacity), default from defaults::HISTOGRAM_FILL_ALPHA
    pub fill_alpha: Option<f32>,
    /// Edge color (auto-derived from fill if None)
    pub edge_color: Option<Color>,
    /// Edge width in points (default from defaults::PATCH_LINE_WIDTH)
    pub edge_width: Option<f32>,
    /// Bar width as fraction of bin width (0.0-1.0, default 0.9)
    pub bar_width: Option<f32>,
}

/// Methods for calculating histogram bin edges
#[derive(Debug, Clone, Copy)]
pub enum BinMethod {
    /// Equal-width bins (default)
    Uniform,
    /// Sturges' rule: ceil(log2(n) + 1)
    Sturges,
    /// Scott's rule: 3.5 * std / n^(1/3)
    Scott,
    /// Freedman-Diaconis rule: 2 * IQR / n^(1/3)
    FreedmanDiaconis,
}

/// Computed histogram data
#[derive(Debug, Clone)]
pub struct HistogramData {
    /// Bin edges (length = counts.len() + 1)
    pub bin_edges: Vec<f64>,
    /// Count or density in each bin
    pub counts: Vec<f64>,
    /// Total number of data points
    pub n_samples: usize,
    /// Whether the data is normalized to density
    pub is_density: bool,
    /// Fill alpha (from config or default)
    pub fill_alpha: f32,
    /// Edge color (None = auto-derive from fill)
    pub edge_color: Option<Color>,
    /// Edge width in points
    pub edge_width: f32,
    /// Bar width as fraction of bin width
    pub bar_width: f32,
}

// Implement PlotData trait for HistogramData
impl PlotData for HistogramData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        let x_min = self.bin_edges.first().copied().unwrap_or(0.0);
        let x_max = self.bin_edges.last().copied().unwrap_or(1.0);
        let y_min = 0.0; // Histogram always starts at 0
        let y_max = self
            .counts
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);

        ((x_min, x_max), (y_min, y_max))
    }

    fn is_empty(&self) -> bool {
        self.counts.is_empty() || self.counts.iter().all(|&c| c == 0.0)
    }
}

// Implement PlotRender trait for HistogramData
impl PlotRender for HistogramData {
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

        // Draw bars
        for (i, &count) in self.counts.iter().enumerate() {
            if count <= 0.0 {
                continue;
            }

            let left = self.bin_edges[i];
            let right = self.bin_edges[i + 1];
            let bin_width = right - left;

            // Apply bar_width factor (center bars within bin)
            let bar_actual_width = bin_width * self.bar_width as f64;
            let bar_offset = (bin_width - bar_actual_width) / 2.0;
            let bar_left = left + bar_offset;
            let bar_right = right - bar_offset;

            // Convert to screen coordinates
            let (x1, y1) = area.data_to_screen(bar_left, 0.0);
            let (x2, y2) = area.data_to_screen(bar_right, count);

            // Calculate rectangle bounds
            let rect_x = x1.min(x2);
            let rect_y = y1.min(y2);
            let rect_width = (x2 - x1).abs();
            let rect_height = (y2 - y1).abs();

            // Draw filled rectangle
            renderer.draw_rectangle(
                rect_x,
                rect_y,
                rect_width,
                rect_height,
                fill_color,
                true, // filled
            )?;

            // Draw edge (if width > 0)
            if edge_width > 0.0 {
                // Use polygon outline for customizable stroke width
                let vertices = [
                    (rect_x, rect_y),
                    (rect_x + rect_width, rect_y),
                    (rect_x + rect_width, rect_y + rect_height),
                    (rect_x, rect_y + rect_height),
                ];
                renderer.draw_polygon_outline(&vertices, edge_color, edge_width)?;
            }
        }

        Ok(())
    }
}

impl Default for HistogramConfig {
    fn default() -> Self {
        Self {
            bins: None,
            range: None,
            density: false,
            cumulative: false,
            bin_method: BinMethod::Sturges,
            fill_alpha: None,
            edge_color: None,
            edge_width: None,
            bar_width: None,
        }
    }
}

// Implement PlotConfig marker trait
impl PlotConfig for HistogramConfig {}

impl HistogramConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bins(mut self, bins: usize) -> Self {
        self.bins = Some(bins);
        self
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.range = Some((min, max));
        self
    }

    pub fn density(mut self, density: bool) -> Self {
        self.density = density;
        self
    }

    pub fn cumulative(mut self, cumulative: bool) -> Self {
        self.cumulative = cumulative;
        self
    }

    pub fn bin_method(mut self, method: BinMethod) -> Self {
        self.bin_method = method;
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

    /// Set bar width as fraction of bin width (0.0-1.0)
    pub fn bar_width(mut self, width: f32) -> Self {
        self.bar_width = Some(width.clamp(0.0, 1.0));
        self
    }
}

/// Calculate histogram from data
pub fn calculate_histogram<T, D: Data1D<T>>(
    data: &D,
    config: &HistogramConfig,
) -> Result<HistogramData>
where
    T: Into<f64> + Copy,
{
    // Collect and validate data using shared utility
    let values = crate::data::collect_finite_values_sorted(data)?;
    let n_samples = values.len();

    // Determine range
    let (mut data_min, mut data_max) = match config.range {
        Some((min, max)) => (min, max),
        None => (*values.first().unwrap(), *values.last().unwrap()),
    };

    // Handle edge case where all values are identical
    if (data_max - data_min).abs() < f64::EPSILON {
        // Create a small range around the single value
        let epsilon = if data_min.abs() > f64::EPSILON {
            data_min.abs() * 0.1
        } else {
            1.0
        };
        data_min -= epsilon;
        data_max += epsilon;
    }

    if data_max <= data_min {
        return Err(PlottingError::InvalidInput(
            "Histogram range max must be greater than min".to_string(),
        ));
    }

    // Determine number of bins
    let n_bins = match config.bins {
        Some(bins) => {
            if bins == 0 {
                return Err(PlottingError::InvalidInput(
                    "Number of bins must be greater than 0".to_string(),
                ));
            }
            bins
        }
        None => calculate_optimal_bins(&values, config.bin_method),
    };

    // Create bin edges
    let bin_edges = create_bin_edges(data_min, data_max, n_bins);

    // Count values in each bin
    let mut counts = vec![0.0; n_bins];
    for &value in &values {
        if value < data_min || value > data_max {
            continue; // Skip out-of-range values
        }

        let bin_idx = if value == data_max {
            n_bins - 1 // Last value goes in last bin
        } else {
            ((value - data_min) / (data_max - data_min) * n_bins as f64).floor() as usize
        };

        if bin_idx < n_bins {
            counts[bin_idx] += 1.0;
        }
    }

    // Apply cumulative if requested
    if config.cumulative {
        for i in 1..counts.len() {
            counts[i] += counts[i - 1];
        }
    }

    // Apply density normalization if requested
    let is_density = config.density;
    if config.density {
        let bin_width = (data_max - data_min) / n_bins as f64;
        let total_area = counts.iter().sum::<f64>() * bin_width;
        if total_area > 0.0 {
            for count in &mut counts {
                *count /= total_area;
            }
        }
    }

    // Extract styling from config, using defaults for None values
    let fill_alpha = config.fill_alpha.unwrap_or(defaults::HISTOGRAM_FILL_ALPHA);
    let edge_color = config.edge_color;
    let edge_width = config.edge_width.unwrap_or(defaults::PATCH_LINE_WIDTH);
    let bar_width = config.bar_width.unwrap_or(defaults::HISTOGRAM_BAR_WIDTH);

    Ok(HistogramData {
        bin_edges,
        counts,
        n_samples,
        is_density,
        fill_alpha,
        edge_color,
        edge_width,
        bar_width,
    })
}

fn calculate_optimal_bins(values: &[f64], method: BinMethod) -> usize {
    let n = values.len() as f64;

    match method {
        BinMethod::Uniform => 10, // Default fallback
        BinMethod::Sturges => (n.log2() + 1.0).ceil() as usize,
        BinMethod::Scott => {
            let std_dev = calculate_std_dev(values);
            let bin_width = 3.5 * std_dev / n.powf(1.0 / 3.0);
            let range = values.last().unwrap() - values.first().unwrap();
            (range / bin_width).ceil().max(1.0) as usize
        }
        BinMethod::FreedmanDiaconis => {
            let iqr = calculate_iqr(values);
            if iqr == 0.0 {
                return 1; // All values are the same
            }
            let bin_width = 2.0 * iqr / n.powf(1.0 / 3.0);
            let range = values.last().unwrap() - values.first().unwrap();
            (range / bin_width).ceil().max(1.0) as usize
        }
    }
}

fn create_bin_edges(min: f64, max: f64, n_bins: usize) -> Vec<f64> {
    let mut edges = Vec::with_capacity(n_bins + 1);
    for i in 0..=n_bins {
        let edge = min + (max - min) * i as f64 / n_bins as f64;
        edges.push(edge);
    }
    edges
}

// Use shared statistical utilities
use super::statistics::{iqr as calculate_iqr, std_dev as calculate_std_dev};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_histogram_basic_functionality() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let config = HistogramConfig::new().bins(5);

        let result = calculate_histogram(&data, &config).unwrap();

        assert_eq!(result.n_samples, 5);
        assert_eq!(result.bin_edges.len(), 6); // n_bins + 1
        assert_eq!(result.counts.len(), 5);
        assert!(!result.is_density);

        // Each bin should contain exactly one value
        for count in &result.counts {
            assert_eq!(*count, 1.0);
        }
    }

    #[test]
    fn test_histogram_empty_data() {
        let data: Vec<f64> = vec![];
        let config = HistogramConfig::new();

        let result = calculate_histogram(&data, &config);
        assert!(result.is_err());

        match result.unwrap_err() {
            PlottingError::EmptyDataSet => {}
            _ => panic!("Expected EmptyDataSet error"),
        }
    }

    #[test]
    fn test_histogram_invalid_range() {
        let data = vec![1.0, 2.0, 3.0];
        let config = HistogramConfig::new().range(5.0, 3.0); // max < min

        let result = calculate_histogram(&data, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_histogram_zero_bins() {
        let data = vec![1.0, 2.0, 3.0];
        let config = HistogramConfig::new().bins(0);

        let result = calculate_histogram(&data, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_histogram_density_normalization() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let config = HistogramConfig::new().bins(5).density(true);

        let result = calculate_histogram(&data, &config).unwrap();

        assert!(result.is_density);

        // For density, the area under histogram should be 1
        let bin_width = result.bin_edges[1] - result.bin_edges[0];
        let total_area: f64 = result.counts.iter().map(|&c| c * bin_width).sum();
        assert!((total_area - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_histogram_cumulative() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let config = HistogramConfig::new().bins(5).cumulative(true);

        let result = calculate_histogram(&data, &config).unwrap();

        // Cumulative histogram should be monotonic increasing
        for i in 1..result.counts.len() {
            assert!(result.counts[i] >= result.counts[i - 1]);
        }

        // Last bin should contain total count
        assert_eq!(result.counts.last().unwrap(), &5.0);
    }

    #[test]
    fn test_histogram_sturges_rule() {
        let data = vec![1.0; 100]; // 100 identical values
        let config = HistogramConfig::new().bin_method(BinMethod::Sturges);

        let result = calculate_histogram(&data, &config).unwrap();

        // Sturges rule: ceil(log2(n) + 1) = ceil(log2(100) + 1) ≈ 8
        let expected_bins = (100.0_f64.log2() + 1.0).ceil() as usize;
        assert_eq!(result.bin_edges.len(), expected_bins + 1);
    }

    #[test]
    fn test_histogram_with_outliers() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 1000.0]; // 1000.0 is outlier
        let config = HistogramConfig::new().bins(5).range(0.0, 10.0);

        let result = calculate_histogram(&data, &config).unwrap();

        // Outlier should be excluded from range
        assert_eq!(result.n_samples, 6); // Still counts all samples
        let total_in_range: f64 = result.counts.iter().sum();
        assert_eq!(total_in_range, 5.0); // Only 5 values in range
    }

    #[test]
    fn test_histogram_bin_edges() {
        let data = vec![0.0, 10.0];
        let config = HistogramConfig::new().bins(10).range(0.0, 10.0);

        let result = calculate_histogram(&data, &config).unwrap();

        // Check bin edges are evenly spaced
        assert_eq!(result.bin_edges.len(), 11);
        assert_eq!(result.bin_edges[0], 0.0);
        assert_eq!(result.bin_edges[10], 10.0);

        for i in 0..10 {
            let expected = i as f64;
            assert!((result.bin_edges[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_histogram_identical_values() {
        let data = vec![5.0; 100]; // All identical values
        let config = HistogramConfig::new().bins(1);

        let result = calculate_histogram(&data, &config).unwrap();

        assert_eq!(result.counts.len(), 1);
        assert_eq!(result.counts[0], 100.0);
    }

    #[test]
    fn test_histogram_with_nan_values() {
        let data = vec![1.0, 2.0, f64::NAN, 4.0, 5.0];
        let config = HistogramConfig::new().bins(4);

        let result = calculate_histogram(&data, &config).unwrap();

        // NaN values should be filtered out
        let total_count: f64 = result.counts.iter().sum();
        assert_eq!(total_count, 4.0); // Only finite values counted
    }

    #[test]
    fn test_histogram_scott_method() {
        let data: Vec<f64> = (0..1000).map(|x| x as f64 * 0.01).collect(); // 0.0 to 9.99
        let config = HistogramConfig::new().bin_method(BinMethod::Scott);

        let result = calculate_histogram(&data, &config);
        assert!(result.is_ok());

        let hist = result.unwrap();
        assert!(hist.bin_edges.len() > 1);
        assert_eq!(hist.bin_edges.len(), hist.counts.len() + 1);
    }

    #[test]
    fn test_histogram_freedman_diaconis_method() {
        let data: Vec<f64> = (0..1000).map(|x| x as f64 * 0.01).collect();
        let config = HistogramConfig::new().bin_method(BinMethod::FreedmanDiaconis);

        let result = calculate_histogram(&data, &config);
        assert!(result.is_ok());

        let hist = result.unwrap();
        assert!(hist.bin_edges.len() > 1);
        assert_eq!(hist.bin_edges.len(), hist.counts.len() + 1);
    }

    #[test]
    fn test_histogram_plot_data_trait() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let config = HistogramConfig::new().bins(5);

        let hist = calculate_histogram(&data, &config).unwrap();

        // Test data_bounds
        let ((x_min, x_max), (y_min, y_max)) = hist.data_bounds();
        assert!((x_min - 1.0).abs() < 1e-10);
        assert!((x_max - 10.0).abs() < 1e-10);
        assert!((y_min - 0.0).abs() < 1e-10); // histogram y starts at 0
        assert!(y_max > 0.0);

        // Test is_empty
        assert!(!hist.is_empty());
    }

    #[test]
    fn test_histogram_styling_defaults() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let config = HistogramConfig::new().bins(5);

        let hist = calculate_histogram(&data, &config).unwrap();

        // Check default styling values from defaults module
        assert!((hist.fill_alpha - 1.0).abs() < 1e-10); // HISTOGRAM_FILL_ALPHA = 1.0
        assert!((hist.edge_width - 0.8).abs() < 1e-10); // PATCH_LINE_WIDTH = 0.8
        assert!((hist.bar_width - 0.9).abs() < 1e-10); // HISTOGRAM_BAR_WIDTH = 0.9
        assert!(hist.edge_color.is_none()); // Auto-derived
    }

    #[test]
    fn test_histogram_custom_styling() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let config = HistogramConfig::new()
            .bins(5)
            .fill_alpha(0.5)
            .edge_width(2.0)
            .bar_width(0.7)
            .edge_color(Color::RED);

        let hist = calculate_histogram(&data, &config).unwrap();

        assert!((hist.fill_alpha - 0.5).abs() < 1e-10);
        assert!((hist.edge_width - 2.0).abs() < 1e-10);
        assert!((hist.bar_width - 0.7).abs() < 1e-10);
        assert_eq!(hist.edge_color, Some(Color::RED));
    }

    #[test]
    fn test_histogram_config_is_plot_config() {
        // Verify HistogramConfig implements PlotConfig
        fn accepts_plot_config<T: PlotConfig>(_: &T) {}
        let config = HistogramConfig::default();
        accepts_plot_config(&config);
    }
}
