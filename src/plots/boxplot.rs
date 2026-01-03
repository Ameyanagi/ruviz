/// Box plot implementation with statistical analysis and outlier detection
use crate::core::{PlottingError, Result};
use crate::data::Data1D;

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
}

/// Methods for detecting outliers
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
}

impl Default for BoxPlotConfig {
    fn default() -> Self {
        Self {
            outlier_method: OutlierMethod::IQR,
            show_outliers: true,
            show_mean: false,
            orientation: BoxOrientation::Vertical,
            whisker_method: WhiskerMethod::Tukey,
        }
    }
}

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
}

/// Calculate box plot statistics from data
pub fn calculate_box_plot<T, D: Data1D<T>>(data: &D, config: &BoxPlotConfig) -> Result<BoxPlotData>
where
    T: Into<f64> + Copy,
{
    if data.len() == 0 {
        return Err(PlottingError::EmptyDataSet);
    }

    // Collect and validate data
    let mut values: Vec<f64> = Vec::with_capacity(data.len());
    for i in 0..data.len() {
        if let Some(val) = data.get(i) {
            let val: f64 = (*val).into();
            if val.is_finite() {
                values.push(val);
            }
        }
    }

    if values.is_empty() {
        return Err(PlottingError::InvalidData {
            message: "No finite values in data".to_string(),
            position: None,
        });
    }

    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
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
}
