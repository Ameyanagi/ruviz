//! Boxen (Letter-Value) plot implementations
//!
//! Provides enhanced box plots that show more quantile information.

use crate::render::Color;
use crate::stats::quantile::letter_values;

/// Configuration for boxen plot
#[derive(Debug, Clone)]
pub struct BoxenConfig {
    /// Maximum number of letter value levels to show
    pub k_depth: Option<usize>,
    /// Width of boxes (fraction of category spacing)
    pub width: f64,
    /// Colors for boxes (None for auto)
    pub color: Option<Color>,
    /// Saturation gradient (darken toward center)
    pub saturation: f32,
    /// Show outliers
    pub show_outliers: bool,
    /// Outlier marker size
    pub outlier_size: f32,
    /// Line width for box edges
    pub line_width: f32,
    /// Orientation
    pub orient: BoxenOrientation,
}

/// Orientation for boxen plots
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxenOrientation {
    Vertical,
    Horizontal,
}

impl Default for BoxenConfig {
    fn default() -> Self {
        Self {
            k_depth: None, // Auto-determine based on data size
            width: 0.8,
            color: None,
            saturation: 0.75,
            show_outliers: true,
            outlier_size: 4.0,
            line_width: 1.0,
            orient: BoxenOrientation::Vertical,
        }
    }
}

impl BoxenConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set k-depth
    pub fn k_depth(mut self, k: usize) -> Self {
        self.k_depth = Some(k.max(1));
        self
    }

    /// Set width
    pub fn width(mut self, width: f64) -> Self {
        self.width = width.clamp(0.1, 1.0);
        self
    }

    /// Set color
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Show outliers
    pub fn show_outliers(mut self, show: bool) -> Self {
        self.show_outliers = show;
        self
    }

    /// Set horizontal orientation
    pub fn horizontal(mut self) -> Self {
        self.orient = BoxenOrientation::Horizontal;
        self
    }
}

/// A single box in a boxen plot
#[derive(Debug, Clone)]
pub struct BoxenBox {
    /// Level (0 = outermost, increasing toward center)
    pub level: usize,
    /// Lower bound of box
    pub lower: f64,
    /// Upper bound of box
    pub upper: f64,
    /// Width (relative, decreases toward center)
    pub width: f64,
}

/// Computed boxen data for a single distribution
#[derive(Debug, Clone)]
pub struct BoxenData {
    /// All boxes from outer to inner
    pub boxes: Vec<BoxenBox>,
    /// Median value
    pub median: f64,
    /// Outlier values
    pub outliers: Vec<f64>,
    /// Original data range
    pub data_range: (f64, f64),
}

/// Compute boxen plot data
///
/// # Arguments
/// * `data` - Input data
/// * `config` - Boxen configuration
///
/// # Returns
/// BoxenData for rendering
pub fn compute_boxen(data: &[f64], config: &BoxenConfig) -> BoxenData {
    if data.is_empty() {
        return BoxenData {
            boxes: vec![],
            median: 0.0,
            outliers: vec![],
            data_range: (0.0, 1.0),
        };
    }

    let mut sorted: Vec<f64> = data.iter().copied().filter(|x| x.is_finite()).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    if sorted.is_empty() {
        return BoxenData {
            boxes: vec![],
            median: 0.0,
            outliers: vec![],
            data_range: (0.0, 1.0),
        };
    }

    let n = sorted.len();

    // Determine k depth
    let k = config.k_depth.unwrap_or_else(|| {
        // Tukey's criterion: use log2(n) levels
        ((n as f64).log2().floor() as usize).clamp(1, 10)
    });

    // Get letter values
    let lvs = letter_values(&sorted, Some(k));

    // Create boxes
    let mut boxes = Vec::new();
    let num_levels = lvs.len();

    for (level, (lower, upper)) in lvs.iter().enumerate() {
        // Width decreases toward center
        let width_factor = 1.0 - (level as f64 / (num_levels + 1) as f64) * 0.3;

        boxes.push(BoxenBox {
            level,
            lower: *lower,
            upper: *upper,
            width: config.width * width_factor,
        });
    }

    // Compute median
    let median = if n % 2 == 0 {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    };

    // Find outliers (outside outermost box)
    let outliers = if config.show_outliers && !boxes.is_empty() {
        let outer_lower = boxes[0].lower;
        let outer_upper = boxes[0].upper;
        sorted
            .iter()
            .copied()
            .filter(|&x| x < outer_lower || x > outer_upper)
            .collect()
    } else {
        vec![]
    };

    let data_range = (*sorted.first().unwrap(), *sorted.last().unwrap());

    BoxenData {
        boxes,
        median,
        outliers,
        data_range,
    }
}

/// Generate box rectangle vertices
pub fn boxen_rect(boxen: &BoxenBox, center: f64, orient: BoxenOrientation) -> Vec<(f64, f64)> {
    let half_width = boxen.width / 2.0;

    match orient {
        BoxenOrientation::Vertical => {
            vec![
                (center - half_width, boxen.lower),
                (center + half_width, boxen.lower),
                (center + half_width, boxen.upper),
                (center - half_width, boxen.upper),
            ]
        }
        BoxenOrientation::Horizontal => {
            vec![
                (boxen.lower, center - half_width),
                (boxen.upper, center - half_width),
                (boxen.upper, center + half_width),
                (boxen.lower, center + half_width),
            ]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boxen_basic() {
        let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let config = BoxenConfig::default();
        let boxen = compute_boxen(&data, &config);

        assert!(!boxen.boxes.is_empty());
        assert!((boxen.median - 49.5).abs() < 1e-10);
    }

    #[test]
    fn test_boxen_nested_boxes() {
        let data: Vec<f64> = (0..1000).map(|i| i as f64).collect();
        let config = BoxenConfig::default().k_depth(5);
        let boxen = compute_boxen(&data, &config);

        assert_eq!(boxen.boxes.len(), 5);

        // Each inner box should be narrower
        for i in 1..boxen.boxes.len() {
            assert!(boxen.boxes[i].width <= boxen.boxes[i - 1].width);
        }
    }

    #[test]
    fn test_boxen_rect_vertical() {
        let box_data = BoxenBox {
            level: 0,
            lower: 10.0,
            upper: 20.0,
            width: 0.8,
        };
        let rect = boxen_rect(&box_data, 0.0, BoxenOrientation::Vertical);

        assert_eq!(rect.len(), 4);
        // Check that rectangle covers the right range
        assert!((rect[0].1 - 10.0).abs() < 1e-10); // lower
        assert!((rect[2].1 - 20.0).abs() < 1e-10); // upper
    }

    #[test]
    fn test_boxen_empty() {
        let data: Vec<f64> = vec![];
        let config = BoxenConfig::default();
        let boxen = compute_boxen(&data, &config);

        assert!(boxen.boxes.is_empty());
    }
}
