//! Swarm plot implementations
//!
//! Provides non-overlapping categorical scatter plots using beeswarm algorithm.

use crate::render::Color;
use crate::stats::beeswarm::beeswarm_positions;

/// Configuration for swarm plot
#[derive(Debug, Clone)]
pub struct SwarmConfig {
    /// Marker size
    pub size: f32,
    /// Marker color (None for auto)
    pub color: Option<Color>,
    /// Marker alpha
    pub alpha: f32,
    /// Orientation
    pub orientation: SwarmOrientation,
    /// Maximum width for spread
    pub width: f64,
    /// Dodge groups
    pub dodge: bool,
    /// Gap between dodged groups
    pub dodge_gap: f64,
}

/// Orientation for swarm plots
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwarmOrientation {
    Vertical,
    Horizontal,
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            size: 5.0,
            color: None,
            alpha: 0.8,
            orientation: SwarmOrientation::Vertical,
            width: 0.8,
            dodge: false,
            dodge_gap: 0.05,
        }
    }
}

impl SwarmConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
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
        self.orientation = SwarmOrientation::Horizontal;
        self
    }

    /// Set maximum spread width
    pub fn width(mut self, width: f64) -> Self {
        self.width = width.clamp(0.1, 1.0);
        self
    }

    /// Enable dodging
    pub fn dodge(mut self, dodge: bool) -> Self {
        self.dodge = dodge;
        self
    }
}

/// A single point in a swarm plot
#[derive(Debug, Clone, Copy)]
pub struct SwarmPoint {
    /// Category index
    pub category: usize,
    /// Original value
    pub value: f64,
    /// Final x position
    pub x: f64,
    /// Final y position
    pub y: f64,
    /// Optional group index
    pub group: Option<usize>,
}

/// Compute swarm plot points
///
/// # Arguments
/// * `categories` - Category indices for each point
/// * `values` - Values for each point
/// * `groups` - Optional group indices
/// * `config` - Swarm plot configuration
///
/// # Returns
/// Vec of SwarmPoint
pub fn compute_swarm_points(
    categories: &[usize],
    values: &[f64],
    groups: Option<&[usize]>,
    config: &SwarmConfig,
) -> Vec<SwarmPoint> {
    let n = categories.len().min(values.len());
    if n == 0 {
        return vec![];
    }

    // Group points by category (and optionally by group)
    let num_categories = categories.iter().max().map_or(0, |&m| m + 1);
    let num_groups = groups.map_or(1, |g| g.iter().max().map_or(1, |&m| m + 1));

    let mut all_points = Vec::with_capacity(n);

    // Process each category
    for cat in 0..num_categories {
        // Get points in this category
        let cat_indices: Vec<usize> = (0..n).filter(|&i| categories[i] == cat).collect();

        if cat_indices.is_empty() {
            continue;
        }

        if config.dodge && num_groups > 1 {
            // Process each group separately
            for grp in 0..num_groups {
                let grp_indices: Vec<usize> = cat_indices
                    .iter()
                    .filter(|&&i| groups.is_none_or(|g| g.get(i).copied().unwrap_or(0) == grp))
                    .copied()
                    .collect();

                if grp_indices.is_empty() {
                    continue;
                }

                let grp_values: Vec<f64> = grp_indices.iter().map(|&i| values[i]).collect();

                // Compute beeswarm positions
                let point_size = config.size as f64;
                let jitter_width = config.width / num_groups as f64;
                let positions = beeswarm_positions(&grp_values, point_size, jitter_width);

                // Calculate dodge offset
                let dodge_width = 0.8 / num_groups as f64;
                let dodge_offset = (grp as f64 - (num_groups - 1) as f64 / 2.0) * dodge_width;

                // Create swarm points
                for (idx, (i, &pos)) in grp_indices.iter().zip(positions.iter()).enumerate() {
                    let base_x = cat as f64 + dodge_offset;
                    let (x, y) = match config.orientation {
                        SwarmOrientation::Vertical => (base_x + pos, grp_values[idx]),
                        SwarmOrientation::Horizontal => (grp_values[idx], base_x + pos),
                    };

                    all_points.push(SwarmPoint {
                        category: cat,
                        value: grp_values[idx],
                        x,
                        y,
                        group: Some(grp),
                    });
                }
            }
        } else {
            // No dodging - process all points in category together
            let cat_values: Vec<f64> = cat_indices.iter().map(|&i| values[i]).collect();

            let point_size = config.size as f64;
            let jitter_width = config.width;
            let positions = beeswarm_positions(&cat_values, point_size, jitter_width);

            for (idx, (&i, &pos)) in cat_indices.iter().zip(positions.iter()).enumerate() {
                let base_x = cat as f64;
                let grp = groups.map(|g| g.get(i).copied().unwrap_or(0));

                let (x, y) = match config.orientation {
                    SwarmOrientation::Vertical => (base_x + pos, cat_values[idx]),
                    SwarmOrientation::Horizontal => (cat_values[idx], base_x + pos),
                };

                all_points.push(SwarmPoint {
                    category: cat,
                    value: cat_values[idx],
                    x,
                    y,
                    group: grp,
                });
            }
        }
    }

    all_points
}

/// Compute data range for swarm plot
pub fn swarm_range(
    points: &[SwarmPoint],
    num_categories: usize,
    orientation: SwarmOrientation,
) -> ((f64, f64), (f64, f64)) {
    if points.is_empty() {
        return ((0.0, 1.0), (0.0, 1.0));
    }

    let val_min = points.iter().map(|p| p.value).fold(f64::INFINITY, f64::min);
    let val_max = points
        .iter()
        .map(|p| p.value)
        .fold(f64::NEG_INFINITY, f64::max);

    // Account for spread width
    let x_spread = points
        .iter()
        .map(|p| p.x)
        .fold(0.0_f64, |a, b| a.max(b.abs()));
    let cat_range = (-x_spread - 0.5, num_categories as f64 + x_spread - 0.5);

    match orientation {
        SwarmOrientation::Vertical => (cat_range, (val_min, val_max)),
        SwarmOrientation::Horizontal => ((val_min, val_max), cat_range),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swarm_basic() {
        let categories = vec![0, 0, 0, 1, 1, 1];
        let values = vec![1.0, 1.0, 1.0, 2.0, 2.0, 2.0];
        let config = SwarmConfig::default();
        let points = compute_swarm_points(&categories, &values, None, &config);

        assert_eq!(points.len(), 6);
        // Points with same value should be spread out
        let cat0_points: Vec<_> = points.iter().filter(|p| p.category == 0).collect();
        // Check that not all x positions are identical
        let x_positions: Vec<f64> = cat0_points.iter().map(|p| p.x).collect();
        let all_same = x_positions.windows(2).all(|w| (w[0] - w[1]).abs() < 1e-10);
        assert!(!all_same || cat0_points.len() == 1);
    }

    #[test]
    fn test_swarm_horizontal() {
        let categories = vec![0, 1];
        let values = vec![1.0, 2.0];
        let config = SwarmConfig::default().horizontal();
        let points = compute_swarm_points(&categories, &values, None, &config);

        // For horizontal, y should be around category, x should be value
        for point in &points {
            assert!((point.x - point.value).abs() < 1e-10);
        }
    }

    #[test]
    fn test_swarm_with_groups() {
        let categories = vec![0, 0, 0, 0];
        let values = vec![1.0, 1.0, 2.0, 2.0];
        let groups = vec![0, 1, 0, 1];
        let config = SwarmConfig::default().dodge(true);
        let points = compute_swarm_points(&categories, &values, Some(&groups), &config);

        assert_eq!(points.len(), 4);
        for point in &points {
            assert!(point.group.is_some());
        }
    }

    #[test]
    fn test_swarm_empty() {
        let categories: Vec<usize> = vec![];
        let values: Vec<f64> = vec![];
        let config = SwarmConfig::default();
        let points = compute_swarm_points(&categories, &values, None, &config);

        assert!(points.is_empty());
    }
}
