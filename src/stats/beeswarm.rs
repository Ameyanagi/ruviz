//! Beeswarm (non-overlapping) point placement
//!
//! Implements algorithms for placing points without overlap,
//! used by swarm plots.

/// A placed point with position
struct PlacedPoint {
    y: f64,
    x: f64,
}

/// Calculate non-overlapping x-positions for points in a swarm plot
///
/// Uses a simple sweepline algorithm to avoid point overlap.
///
/// # Arguments
/// * `y_values` - Y coordinates of points (determines vertical position)
/// * `point_size` - Diameter of each point
/// * `jitter_width` - Maximum horizontal spread
///
/// # Returns
/// Vec of x-offsets for each point
pub fn beeswarm_positions(y_values: &[f64], point_size: f64, jitter_width: f64) -> Vec<f64> {
    if y_values.is_empty() {
        return vec![];
    }

    let n = y_values.len();
    let mut positions = vec![0.0; n];

    // Sort points by y-value with indices
    let mut sorted_indices: Vec<usize> = (0..n).collect();
    sorted_indices.sort_by(|&a, &b| {
        y_values[a]
            .partial_cmp(&y_values[b])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Minimum separation between point centers
    let min_sep = point_size;
    let half_width = jitter_width / 2.0;

    // Track placed points for collision detection
    let mut placed: Vec<PlacedPoint> = Vec::with_capacity(n);

    for &idx in &sorted_indices {
        let y = y_values[idx];

        if !y.is_finite() {
            positions[idx] = 0.0;
            continue;
        }

        // Find best x position (closest to center) without collision
        let x = find_position(y, &placed, min_sep, half_width);

        positions[idx] = x;
        placed.push(PlacedPoint { y, x });
    }

    positions
}

/// Find a valid x position for a point
fn find_position(y: f64, placed: &[PlacedPoint], min_sep: f64, half_width: f64) -> f64 {
    // Filter to nearby points that could collide
    let nearby: Vec<_> = placed
        .iter()
        .filter(|p| (p.y - y).abs() < min_sep)
        .collect();

    if nearby.is_empty() {
        return 0.0;
    }

    // Try positions starting from center, expanding outward
    let step = min_sep * 0.5;
    let max_steps = (half_width / step).ceil() as usize;

    for i in 0..=max_steps {
        let offset = i as f64 * step;

        // Try positive offset
        if !has_collision(offset, y, &nearby, min_sep) {
            return offset.min(half_width);
        }

        // Try negative offset
        if offset > 0.0 && !has_collision(-offset, y, &nearby, min_sep) {
            return (-offset).max(-half_width);
        }
    }

    // If all positions collide, place at edge
    if nearby.iter().map(|p| p.x).sum::<f64>() >= 0.0 {
        -half_width
    } else {
        half_width
    }
}

/// Check if position collides with any placed point
fn has_collision(x: f64, y: f64, nearby: &[&PlacedPoint], min_sep: f64) -> bool {
    for p in nearby {
        let dx = x - p.x;
        let dy = y - p.y;
        let dist_sq = dx * dx + dy * dy;
        if dist_sq < min_sep * min_sep {
            return true;
        }
    }
    false
}

/// Calculate positions using quasirandom jitter
///
/// Faster than true beeswarm but may have some overlap.
/// Uses low-discrepancy sequence for better distribution.
///
/// # Arguments
/// * `n` - Number of points
/// * `jitter_width` - Maximum horizontal spread
///
/// # Returns
/// Vec of x-offsets
pub fn quasirandom_jitter(n: usize, jitter_width: f64) -> Vec<f64> {
    let half_width = jitter_width / 2.0;

    // Use golden ratio-based low-discrepancy sequence
    let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
    let alpha = 1.0 / phi;

    (0..n)
        .map(|i| {
            let t = (0.5 + alpha * i as f64) % 1.0;
            (t - 0.5) * 2.0 * half_width
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beeswarm_empty() {
        let positions = beeswarm_positions(&[], 10.0, 50.0);
        assert!(positions.is_empty());
    }

    #[test]
    fn test_beeswarm_single() {
        let positions = beeswarm_positions(&[5.0], 10.0, 50.0);
        assert_eq!(positions.len(), 1);
        assert!((positions[0] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_beeswarm_no_overlap() {
        // Points at same y should be spread out
        let y_values = vec![5.0, 5.0, 5.0, 5.0, 5.0];
        let point_size = 10.0;
        let positions = beeswarm_positions(&y_values, point_size, 100.0);

        // Check no overlapping pairs
        for i in 0..positions.len() {
            for j in (i + 1)..positions.len() {
                let dx = positions[i] - positions[j];
                let dy = y_values[i] - y_values[j];
                let dist = (dx * dx + dy * dy).sqrt();
                // Allow small tolerance
                assert!(dist >= point_size * 0.9, "Points {} and {} overlap", i, j);
            }
        }
    }

    #[test]
    fn test_beeswarm_spread_points() {
        // Points spread in y should have smaller x spread
        let y_values: Vec<f64> = (0..10).map(|i| i as f64 * 20.0).collect();
        let positions = beeswarm_positions(&y_values, 10.0, 50.0);

        // Most should be near center
        let center_count = positions.iter().filter(|&&x| x.abs() < 5.0).count();
        assert!(center_count > 5);
    }

    #[test]
    fn test_quasirandom_jitter() {
        let positions = quasirandom_jitter(100, 50.0);
        assert_eq!(positions.len(), 100);

        // All should be within bounds
        assert!(positions.iter().all(|&x| x.abs() <= 25.0));

        // Should have good spread (not all on one side)
        let positive_count = positions.iter().filter(|&&x| x > 0.0).count();
        assert!(positive_count > 30 && positive_count < 70);
    }
}
