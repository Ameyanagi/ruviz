//! Violin plot implementation
//!
//! Provides violin plots for visualizing distribution shapes,
//! combining KDE with optional box/strip components.

use crate::render::{Color, SkiaRenderer, Theme};
use crate::stats::kde::{KdeResult, kde_1d};

/// Configuration for violin plot
#[derive(Debug, Clone)]
pub struct ViolinConfig {
    /// Number of points for KDE evaluation
    pub n_points: usize,
    /// Bandwidth selection method
    pub bandwidth: BandwidthMethod,
    /// Show inner boxplot
    pub show_box: bool,
    /// Show inner quartile lines
    pub show_quartiles: bool,
    /// Show median line
    pub show_median: bool,
    /// Show individual data points (like strip/swarm)
    pub show_points: bool,
    /// Split violin (show half on each side for comparison)
    pub split: bool,
    /// Scale method for width
    pub scale: ViolinScale,
    /// Maximum width of violin (in plot units)
    pub width: f64,
    /// Orientation
    pub orientation: Orientation,
    /// Color for violin fill
    pub fill_color: Option<Color>,
    /// Alpha for fill
    pub fill_alpha: f32,
    /// Color for outline
    pub line_color: Option<Color>,
    /// Line width
    pub line_width: f32,
    /// Inner color (for box/quartiles)
    pub inner_color: Color,
}

/// Bandwidth selection method
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BandwidthMethod {
    /// Scott's rule (default)
    Scott,
    /// Silverman's rule
    Silverman,
    /// Fixed bandwidth value
    Fixed(f64),
}

/// Scaling method for violin width
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolinScale {
    /// Same area for all violins
    Area,
    /// Same maximum width for all violins
    Width,
    /// Width proportional to sample count
    Count,
}

/// Orientation for violin
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

impl Default for ViolinConfig {
    fn default() -> Self {
        Self {
            n_points: 100,
            bandwidth: BandwidthMethod::Scott,
            show_box: true,
            show_quartiles: true,
            show_median: true,
            show_points: false,
            split: false,
            scale: ViolinScale::Width,
            width: 0.8,
            orientation: Orientation::Vertical,
            fill_color: None,
            fill_alpha: 0.7,
            line_color: None,
            line_width: 1.0,
            inner_color: Color::new(0, 0, 0),
        }
    }
}

impl ViolinConfig {
    /// Create new config with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set number of evaluation points
    pub fn n_points(mut self, n: usize) -> Self {
        self.n_points = n.max(10);
        self
    }

    /// Set bandwidth method
    pub fn bandwidth(mut self, method: BandwidthMethod) -> Self {
        self.bandwidth = method;
        self
    }

    /// Show/hide inner boxplot
    pub fn box_plot(mut self, show: bool) -> Self {
        self.show_box = show;
        self
    }

    /// Show/hide quartile lines
    pub fn quartiles(mut self, show: bool) -> Self {
        self.show_quartiles = show;
        self
    }

    /// Show/hide median
    pub fn median(mut self, show: bool) -> Self {
        self.show_median = show;
        self
    }

    /// Show/hide data points
    pub fn points(mut self, show: bool) -> Self {
        self.show_points = show;
        self
    }

    /// Enable split violin mode
    pub fn split(mut self, split: bool) -> Self {
        self.split = split;
        self
    }

    /// Set scale method
    pub fn scale(mut self, scale: ViolinScale) -> Self {
        self.scale = scale;
        self
    }

    /// Set maximum width
    pub fn width(mut self, width: f64) -> Self {
        self.width = width.max(0.1);
        self
    }

    /// Set horizontal orientation
    pub fn horizontal(mut self) -> Self {
        self.orientation = Orientation::Horizontal;
        self
    }

    /// Set vertical orientation
    pub fn vertical(mut self) -> Self {
        self.orientation = Orientation::Vertical;
        self
    }

    /// Set fill color
    pub fn fill_color(mut self, color: Color) -> Self {
        self.fill_color = Some(color);
        self
    }

    /// Set fill alpha
    pub fn fill_alpha(mut self, alpha: f32) -> Self {
        self.fill_alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Set line color
    pub fn line_color(mut self, color: Color) -> Self {
        self.line_color = Some(color);
        self
    }

    /// Set line width
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width.max(0.0);
        self
    }
}

/// Violin plot data with computed KDE
#[derive(Debug, Clone)]
pub struct ViolinData {
    /// Original data
    pub data: Vec<f64>,
    /// KDE result
    pub kde: KdeResult,
    /// Quartiles (q1, median, q3)
    pub quartiles: (f64, f64, f64),
    /// Data min and max
    pub range: (f64, f64),
}

impl ViolinData {
    /// Compute violin data from raw values
    pub fn from_values(data: &[f64], config: &ViolinConfig) -> Option<Self> {
        if data.is_empty() {
            return None;
        }

        // Filter valid values
        let mut sorted: Vec<f64> = data.iter().filter(|v| v.is_finite()).copied().collect();
        if sorted.is_empty() {
            return None;
        }
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let n = sorted.len();
        let min = sorted[0];
        let max = sorted[n - 1];

        // Compute KDE
        let bandwidth = match config.bandwidth {
            BandwidthMethod::Fixed(bw) => Some(bw),
            _ => None, // Use Scott's rule by default
        };
        let kde = kde_1d(&sorted, bandwidth, Some(config.n_points));

        // Compute quartiles
        let q1 = percentile(&sorted, 25.0);
        let median = percentile(&sorted, 50.0);
        let q3 = percentile(&sorted, 75.0);

        Some(Self {
            data: sorted,
            kde,
            quartiles: (q1, median, q3),
            range: (min, max),
        })
    }

    /// Get maximum density (for scaling)
    pub fn max_density(&self) -> f64 {
        self.kde.density.iter().copied().fold(0.0, f64::max)
    }
}

/// Calculate percentile from sorted data
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let n = sorted.len();
    let idx = (p / 100.0) * (n - 1) as f64;
    let lower = idx.floor() as usize;
    let upper = idx.ceil() as usize;
    let frac = idx - lower as f64;

    if lower >= n || upper >= n {
        sorted[n - 1]
    } else {
        sorted[lower] * (1.0 - frac) + sorted[upper] * frac
    }
}

/// Generate violin polygon vertices
///
/// # Arguments
/// * `violin` - Violin data with KDE
/// * `center_x` - X position for vertical violin
/// * `center_y` - Y position for horizontal violin
/// * `half_width` - Half the maximum width
/// * `config` - Violin configuration
///
/// # Returns
/// Left and right polygon vertices for the violin shape
#[allow(clippy::type_complexity)]
pub fn violin_polygon(
    violin: &ViolinData,
    center: f64,
    half_width: f64,
    config: &ViolinConfig,
) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
    let max_density = violin.max_density();
    if max_density <= 0.0 {
        return (vec![], vec![]);
    }

    let scale = half_width / max_density;

    let mut left_side = Vec::with_capacity(violin.kde.x.len());
    let mut right_side = Vec::with_capacity(violin.kde.x.len());

    for (i, (&x, &d)) in violin
        .kde
        .x
        .iter()
        .zip(violin.kde.density.iter())
        .enumerate()
    {
        let width = d * scale;

        match config.orientation {
            Orientation::Vertical => {
                // x is actually y (the data value), center is the x position
                if config.split {
                    left_side.push((center, x));
                    right_side.push((center + width, x));
                } else {
                    left_side.push((center - width, x));
                    right_side.push((center + width, x));
                }
            }
            Orientation::Horizontal => {
                // x is the data value (horizontal axis), center is y position
                if config.split {
                    left_side.push((x, center));
                    right_side.push((x, center + width));
                } else {
                    left_side.push((x, center - width));
                    right_side.push((x, center + width));
                }
            }
        }
    }

    (left_side, right_side)
}

/// Create closed polygon from left and right sides
pub fn close_violin_polygon(left: &[(f64, f64)], right: &[(f64, f64)]) -> Vec<(f64, f64)> {
    if left.is_empty() || right.is_empty() {
        return vec![];
    }

    let mut polygon = Vec::with_capacity(left.len() + right.len());

    // Add left side (bottom to top)
    polygon.extend_from_slice(left);

    // Add right side (top to bottom)
    for point in right.iter().rev() {
        polygon.push(*point);
    }

    polygon
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_violin_data_basic() {
        let data: Vec<f64> = (0..100).map(|i| (i as f64 * 0.1).sin()).collect();
        let config = ViolinConfig::default();
        let violin = ViolinData::from_values(&data, &config);

        assert!(violin.is_some());
        let violin = violin.unwrap();
        assert!(!violin.kde.x.is_empty());
        assert!(violin.max_density() > 0.0);
    }

    #[test]
    fn test_violin_data_empty() {
        let data: Vec<f64> = vec![];
        let config = ViolinConfig::default();
        let violin = ViolinData::from_values(&data, &config);
        assert!(violin.is_none());
    }

    #[test]
    fn test_violin_polygon() {
        let data: Vec<f64> = (0..50).map(|i| i as f64).collect();
        let config = ViolinConfig::default();
        let violin = ViolinData::from_values(&data, &config).unwrap();

        let (left, right) = violin_polygon(&violin, 0.5, 0.3, &config);
        assert!(!left.is_empty());
        assert!(!right.is_empty());
        assert_eq!(left.len(), right.len());
    }

    #[test]
    fn test_close_polygon() {
        let left = vec![(0.0, 0.0), (0.0, 1.0), (0.0, 2.0)];
        let right = vec![(1.0, 0.0), (1.0, 1.0), (1.0, 2.0)];

        let closed = close_violin_polygon(&left, &right);
        assert_eq!(closed.len(), 6);
    }

    #[test]
    fn test_percentile() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((percentile(&data, 50.0) - 3.0).abs() < 1e-10);
        assert!((percentile(&data, 0.0) - 1.0).abs() < 1e-10);
        assert!((percentile(&data, 100.0) - 5.0).abs() < 1e-10);
    }
}
