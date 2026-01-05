//! Strip plot implementations
//!
//! Provides strip plots (jittered scatter for categorical data).

use crate::render::Color;

/// Configuration for strip plot
#[derive(Debug, Clone)]
pub struct StripConfig {
    /// Jitter amount (0.0-1.0 as fraction of category spacing)
    pub jitter: f64,
    /// Marker size
    pub size: f32,
    /// Marker color (None for auto)
    pub color: Option<Color>,
    /// Marker alpha
    pub alpha: f32,
    /// Orientation
    pub orientation: StripOrientation,
    /// Dodge groups (for grouped strip plots)
    pub dodge: bool,
    /// Random seed for jitter
    pub seed: u64,
}

/// Orientation for strip plots
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StripOrientation {
    Vertical,
    Horizontal,
}

impl Default for StripConfig {
    fn default() -> Self {
        Self {
            jitter: 0.3,
            size: 5.0,
            color: None,
            alpha: 0.8,
            orientation: StripOrientation::Vertical,
            dodge: false,
            seed: 42,
        }
    }
}

impl StripConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set jitter amount
    pub fn jitter(mut self, jitter: f64) -> Self {
        self.jitter = jitter.clamp(0.0, 1.0);
        self
    }

    /// Set marker size
    pub fn size(mut self, size: f32) -> Self {
        self.size = size.max(0.1);
        self
    }

    /// Set color
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set alpha
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Set horizontal orientation
    pub fn horizontal(mut self) -> Self {
        self.orientation = StripOrientation::Horizontal;
        self
    }

    /// Enable dodging for groups
    pub fn dodge(mut self, dodge: bool) -> Self {
        self.dodge = dodge;
        self
    }

    /// Set random seed
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }
}

/// A single point in a strip plot
#[derive(Debug, Clone, Copy)]
pub struct StripPoint {
    /// Category index
    pub category: usize,
    /// Value
    pub value: f64,
    /// Jittered x position
    pub x: f64,
    /// Y position (value for vertical, category for horizontal)
    pub y: f64,
    /// Optional group index
    pub group: Option<usize>,
}

/// Simple pseudo-random number generator
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_f64(&mut self) -> f64 {
        // xorshift64
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        (self.state as f64) / (u64::MAX as f64)
    }
}

/// Compute strip plot points
///
/// # Arguments
/// * `categories` - Category indices for each point
/// * `values` - Values for each point
/// * `groups` - Optional group indices
/// * `config` - Strip plot configuration
///
/// # Returns
/// Vec of StripPoint
pub fn compute_strip_points(
    categories: &[usize],
    values: &[f64],
    groups: Option<&[usize]>,
    config: &StripConfig,
) -> Vec<StripPoint> {
    let n = categories.len().min(values.len());
    if n == 0 {
        return vec![];
    }

    let mut rng = SimpleRng::new(config.seed);
    let mut points = Vec::with_capacity(n);

    // Find number of groups for dodging
    let num_groups = groups.map_or(1, |g| g.iter().max().map_or(1, |&m| m + 1));

    for i in 0..n {
        let cat = categories[i];
        let val = values[i];
        let grp = groups.map(|g| g.get(i).copied().unwrap_or(0));

        // Compute jitter
        let jitter = (rng.next_f64() - 0.5) * config.jitter;

        // Compute position with optional dodging
        let base_x = cat as f64;
        let x = if config.dodge && num_groups > 1 {
            let grp_idx = grp.unwrap_or(0);
            let dodge_width = 0.8 / num_groups as f64;
            let dodge_offset = (grp_idx as f64 - (num_groups - 1) as f64 / 2.0) * dodge_width;
            base_x + dodge_offset + jitter * dodge_width
        } else {
            base_x + jitter
        };

        let (x, y) = match config.orientation {
            StripOrientation::Vertical => (x, val),
            StripOrientation::Horizontal => (val, x),
        };

        points.push(StripPoint {
            category: cat,
            value: val,
            x,
            y,
            group: grp,
        });
    }

    points
}

/// Compute data range for strip plot
pub fn strip_range(
    points: &[StripPoint],
    num_categories: usize,
    orientation: StripOrientation,
) -> ((f64, f64), (f64, f64)) {
    if points.is_empty() {
        return ((0.0, 1.0), (0.0, 1.0));
    }

    let val_min = points.iter().map(|p| p.value).fold(f64::INFINITY, f64::min);
    let val_max = points
        .iter()
        .map(|p| p.value)
        .fold(f64::NEG_INFINITY, f64::max);

    let cat_range = (-0.5, num_categories as f64 - 0.5);

    match orientation {
        StripOrientation::Vertical => (cat_range, (val_min, val_max)),
        StripOrientation::Horizontal => ((val_min, val_max), cat_range),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_basic() {
        let categories = vec![0, 0, 0, 1, 1, 1, 2, 2, 2];
        let values = vec![1.0, 1.5, 2.0, 3.0, 3.5, 4.0, 2.0, 2.5, 3.0];
        let config = StripConfig::default();
        let points = compute_strip_points(&categories, &values, None, &config);

        assert_eq!(points.len(), 9);
        // Points should be jittered around their categories
        for point in &points {
            let expected_x = point.category as f64;
            assert!((point.x - expected_x).abs() < 0.5);
        }
    }

    #[test]
    fn test_strip_horizontal() {
        let categories = vec![0, 1, 2];
        let values = vec![1.0, 2.0, 3.0];
        let config = StripConfig::default().horizontal();
        let points = compute_strip_points(&categories, &values, None, &config);

        // For horizontal, x should be value, y should be category
        for point in &points {
            assert!((point.x - point.value).abs() < 1e-10);
        }
    }

    #[test]
    fn test_strip_with_groups() {
        let categories = vec![0, 0, 1, 1];
        let values = vec![1.0, 2.0, 1.0, 2.0];
        let groups = vec![0, 1, 0, 1];
        let config = StripConfig::default().dodge(true);
        let points = compute_strip_points(&categories, &values, Some(&groups), &config);

        assert_eq!(points.len(), 4);
        // Each point should have a group
        for point in &points {
            assert!(point.group.is_some());
        }
    }

    #[test]
    fn test_strip_range() {
        let categories = vec![0, 1, 2];
        let values = vec![1.0, 5.0, 3.0];
        let config = StripConfig::default();
        let points = compute_strip_points(&categories, &values, None, &config);
        let ((x_min, x_max), (y_min, y_max)) = strip_range(&points, 3, StripOrientation::Vertical);

        assert!(x_min < 0.0);
        assert!(x_max > 2.0);
        assert!((y_min - 1.0).abs() < 1e-10);
        assert!((y_max - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_strip_empty() {
        let categories: Vec<usize> = vec![];
        let values: Vec<f64> = vec![];
        let config = StripConfig::default();
        let points = compute_strip_points(&categories, &values, None, &config);

        assert!(points.is_empty());
    }
}
