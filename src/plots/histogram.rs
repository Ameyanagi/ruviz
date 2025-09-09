/// Histogram plot implementation with automatic binning and statistical analysis
use crate::core::{PlottingError, Result};
use crate::data::Data1D;

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
}

impl Default for HistogramConfig {
    fn default() -> Self {
        Self {
            bins: None,
            range: None, 
            density: false,
            cumulative: false,
            bin_method: BinMethod::Sturges,
        }
    }
}

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
}

/// Calculate histogram from data
pub fn calculate_histogram<T, D: Data1D<T>>(data: &D, config: &HistogramConfig) -> Result<HistogramData> 
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

    // Determine range
    let (data_min, data_max) = match config.range {
        Some((min, max)) => (min, max),
        None => (*values.first().unwrap(), *values.last().unwrap()),
    };

    if data_max <= data_min {
        return Err(PlottingError::InvalidInput(
            "Histogram range max must be greater than min".to_string()
        ));
    }

    // Determine number of bins
    let n_bins = match config.bins {
        Some(bins) => {
            if bins == 0 {
                return Err(PlottingError::InvalidInput(
                    "Number of bins must be greater than 0".to_string()
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

    Ok(HistogramData {
        bin_edges,
        counts,
        n_samples,
        is_density,
    })
}

fn calculate_optimal_bins(values: &[f64], method: BinMethod) -> usize {
    let n = values.len() as f64;
    
    match method {
        BinMethod::Uniform => 10, // Default fallback
        BinMethod::Sturges => (n.log2() + 1.0).ceil() as usize,
        BinMethod::Scott => {
            let std_dev = calculate_std_dev(values);
            let bin_width = 3.5 * std_dev / n.powf(1.0/3.0);
            let range = values.last().unwrap() - values.first().unwrap();
            (range / bin_width).ceil().max(1.0) as usize
        },
        BinMethod::FreedmanDiaconis => {
            let iqr = calculate_iqr(values);
            if iqr == 0.0 {
                return 1; // All values are the same
            }
            let bin_width = 2.0 * iqr / n.powf(1.0/3.0);
            let range = values.last().unwrap() - values.first().unwrap();
            (range / bin_width).ceil().max(1.0) as usize
        },
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

fn calculate_std_dev(values: &[f64]) -> f64 {
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>() / values.len() as f64;
    variance.sqrt()
}

fn calculate_iqr(sorted_values: &[f64]) -> f64 {
    let n = sorted_values.len();
    let q1_idx = n / 4;
    let q3_idx = 3 * n / 4;
    sorted_values[q3_idx] - sorted_values[q1_idx]
}

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
            PlottingError::EmptyDataSet => {},
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
        let bin_width = (result.bin_edges[1] - result.bin_edges[0]);
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
            assert!(result.counts[i] >= result.counts[i-1]);
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
}