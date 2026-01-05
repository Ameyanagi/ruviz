//! Contour extraction using marching squares
//!
//! Provides contour line extraction from 2D scalar fields.

/// A single contour level with its lines
#[derive(Debug, Clone)]
pub struct ContourLevel {
    /// The z-value of this contour level
    pub level: f64,
    /// Line segments as (x1, y1, x2, y2)
    pub segments: Vec<(f64, f64, f64, f64)>,
}

/// Extract contour lines at specified levels using marching squares
///
/// # Arguments
/// * `x` - X coordinates of grid (length nx)
/// * `y` - Y coordinates of grid (length ny)
/// * `z` - Z values as row-major 2D array (ny rows × nx cols)
/// * `levels` - Z values at which to extract contours
///
/// # Returns
/// Vec of ContourLevel, one per level
pub fn contour_lines(x: &[f64], y: &[f64], z: &[Vec<f64>], levels: &[f64]) -> Vec<ContourLevel> {
    let ny = z.len();
    if ny == 0 {
        return vec![];
    }
    let nx = z[0].len();
    if nx == 0 || x.len() < 2 || y.len() < 2 {
        return vec![];
    }

    levels
        .iter()
        .map(|&level| {
            let segments = marching_squares(x, y, z, level);
            ContourLevel { level, segments }
        })
        .collect()
}

/// Marching squares algorithm for a single contour level
///
/// # Arguments
/// * `x` - X grid coordinates
/// * `y` - Y grid coordinates
/// * `z` - Z values as 2D array
/// * `level` - Contour level to extract
///
/// # Returns
/// Vec of line segments (x1, y1, x2, y2)
pub fn marching_squares(
    x: &[f64],
    y: &[f64],
    z: &[Vec<f64>],
    level: f64,
) -> Vec<(f64, f64, f64, f64)> {
    let ny = z.len();
    let nx = z[0].len();

    let mut segments = Vec::new();

    // Process each cell (2x2 grid of values)
    for j in 0..(ny - 1) {
        for i in 0..(nx - 1) {
            // Get corner values (counter-clockwise from bottom-left)
            let v0 = z[j][i]; // bottom-left
            let v1 = z[j][i + 1]; // bottom-right
            let v2 = z[j + 1][i + 1]; // top-right
            let v3 = z[j + 1][i]; // top-left

            // Skip cells with NaN values
            if !v0.is_finite() || !v1.is_finite() || !v2.is_finite() || !v3.is_finite() {
                continue;
            }

            // Calculate case index (4-bit binary)
            let case = ((v0 >= level) as u8)
                | (((v1 >= level) as u8) << 1)
                | (((v2 >= level) as u8) << 2)
                | (((v3 >= level) as u8) << 3);

            // Get corner coordinates
            let x0 = x[i];
            let x1 = x[i + 1];
            let y0 = y[j];
            let y1 = y[j + 1];

            // Interpolate crossing points
            let cross = |va: f64, vb: f64, a: f64, b: f64| -> f64 {
                if (vb - va).abs() < 1e-12 {
                    (a + b) / 2.0
                } else {
                    a + (level - va) / (vb - va) * (b - a)
                }
            };

            // Edge crossing points
            let bottom = || (cross(v0, v1, x0, x1), y0);
            let right = || (x1, cross(v1, v2, y0, y1));
            let top = || (cross(v3, v2, x0, x1), y1);
            let left = || (x0, cross(v0, v3, y0, y1));

            // Lookup table for marching squares
            match case {
                0 | 15 => {} // No contour
                1 | 14 => {
                    let (bx, by) = bottom();
                    let (lx, ly) = left();
                    segments.push((bx, by, lx, ly));
                }
                2 | 13 => {
                    let (bx, by) = bottom();
                    let (rx, ry) = right();
                    segments.push((bx, by, rx, ry));
                }
                3 | 12 => {
                    let (lx, ly) = left();
                    let (rx, ry) = right();
                    segments.push((lx, ly, rx, ry));
                }
                4 | 11 => {
                    let (rx, ry) = right();
                    let (tx, ty) = top();
                    segments.push((rx, ry, tx, ty));
                }
                5 => {
                    // Saddle point - ambiguous case
                    let center = (v0 + v1 + v2 + v3) / 4.0;
                    let (bx, by) = bottom();
                    let (rx, ry) = right();
                    let (tx, ty) = top();
                    let (lx, ly) = left();
                    if center >= level {
                        segments.push((bx, by, rx, ry));
                        segments.push((tx, ty, lx, ly));
                    } else {
                        segments.push((bx, by, lx, ly));
                        segments.push((rx, ry, tx, ty));
                    }
                }
                6 | 9 => {
                    let (bx, by) = bottom();
                    let (tx, ty) = top();
                    segments.push((bx, by, tx, ty));
                }
                7 | 8 => {
                    let (tx, ty) = top();
                    let (lx, ly) = left();
                    segments.push((tx, ty, lx, ly));
                }
                10 => {
                    // Saddle point - ambiguous case
                    let center = (v0 + v1 + v2 + v3) / 4.0;
                    let (bx, by) = bottom();
                    let (rx, ry) = right();
                    let (tx, ty) = top();
                    let (lx, ly) = left();
                    if center >= level {
                        segments.push((bx, by, lx, ly));
                        segments.push((rx, ry, tx, ty));
                    } else {
                        segments.push((bx, by, rx, ry));
                        segments.push((tx, ty, lx, ly));
                    }
                }
                _ => {}
            }
        }
    }

    segments
}

/// Generate automatic contour levels
///
/// # Arguments
/// * `z` - Z values as 2D array
/// * `n_levels` - Number of contour levels to generate
///
/// # Returns
/// Vec of evenly-spaced contour levels
pub fn auto_levels(z: &[Vec<f64>], n_levels: usize) -> Vec<f64> {
    let mut z_min = f64::INFINITY;
    let mut z_max = f64::NEG_INFINITY;

    for row in z {
        for &val in row {
            if val.is_finite() {
                z_min = z_min.min(val);
                z_max = z_max.max(val);
            }
        }
    }

    if !z_min.is_finite() || !z_max.is_finite() || n_levels == 0 {
        return vec![];
    }

    let step = (z_max - z_min) / (n_levels + 1) as f64;
    (1..=n_levels).map(|i| z_min + i as f64 * step).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marching_squares_simple() {
        // Simple 2x2 grid with diagonal
        let x = vec![0.0, 1.0];
        let y = vec![0.0, 1.0];
        let z = vec![vec![0.0, 1.0], vec![1.0, 2.0]];

        let segments = marching_squares(&x, &y, &z, 0.5);
        assert!(!segments.is_empty());
    }

    #[test]
    fn test_contour_lines() {
        // 3x3 grid with a peak in the center
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![0.0, 1.0, 2.0];
        let z = vec![
            vec![0.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 0.0],
        ];

        let contours = contour_lines(&x, &y, &z, &[0.5]);
        assert_eq!(contours.len(), 1);
        assert!(!contours[0].segments.is_empty());
    }

    #[test]
    fn test_auto_levels() {
        let z = vec![
            vec![0.0, 1.0, 2.0],
            vec![1.0, 2.0, 3.0],
            vec![2.0, 3.0, 4.0],
        ];

        let levels = auto_levels(&z, 3);
        assert_eq!(levels.len(), 3);
        assert!(levels[0] > 0.0);
        assert!(levels[2] < 4.0);
    }

    #[test]
    fn test_empty_input() {
        let contours = contour_lines(&[], &[], &[], &[0.5]);
        assert!(contours.is_empty());
    }
}
