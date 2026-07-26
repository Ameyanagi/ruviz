//! Contour extraction using marching squares
//!
//! Provides contour line extraction from 2D scalar fields.

use crate::core::{PlottingError, Result};

/// A single contour level with its lines
#[derive(Debug, Clone)]
pub struct ContourLevel {
    /// The z-value of this contour level
    pub level: f64,
    /// Line segments as (x1, y1, x2, y2)
    pub segments: Vec<(f64, f64, f64, f64)>,
}

/// Validate a regular contour grid and return its `(ny, nx)` shape.
///
/// Every public entry point in this module funnels through this one function,
/// so the indexing contract (`z` is `y.len()` rows of `x.len()` columns) cannot
/// drift between them.
///
/// Grids with fewer than two rows or columns contain no marching-squares cells
/// and are reported as an empty, not an invalid, shape.
fn validate_grid(
    operation: &'static str,
    x: &[f64],
    y: &[f64],
    z: &[Vec<f64>],
) -> Result<(usize, usize)> {
    let ny = z.len();
    let nx = z.first().map_or(0, Vec::len);

    for (row_index, row) in z.iter().enumerate() {
        if row.len() != nx {
            return Err(PlottingError::RaggedData2D {
                context: operation,
                row: row_index,
                expected_columns: nx,
                actual_columns: row.len(),
            });
        }
    }

    if y.len() != ny || x.len() != nx {
        return Err(PlottingError::GridShapeMismatch {
            operation,
            expected_rows: y.len(),
            expected_columns: x.len(),
            actual_rows: ny,
            actual_columns: nx,
        });
    }

    Ok((ny, nx))
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
/// One [`ContourLevel`] per requested level.
///
/// # Errors
/// Returns [`PlottingError::RaggedData2D`] if the rows of `z` have differing
/// lengths, and [`PlottingError::GridShapeMismatch`] if `z` is not exactly
/// `y.len()` rows of `x.len()` columns.
pub fn contour_lines(
    x: &[f64],
    y: &[f64],
    z: &[Vec<f64>],
    levels: &[f64],
) -> Result<Vec<ContourLevel>> {
    let (ny, nx) = validate_grid("contour_lines", x, y, z)?;
    if ny < 2 || nx < 2 {
        return Ok(vec![]);
    }

    Ok(levels
        .iter()
        .map(|&level| ContourLevel {
            level,
            segments: marching_squares_unchecked(x, y, z, ny, nx, level),
        })
        .collect())
}

/// Marching squares algorithm for a single contour level
///
/// # Arguments
/// * `x` - X grid coordinates (length nx)
/// * `y` - Y grid coordinates (length ny)
/// * `z` - Z values as row-major 2D array (ny rows × nx cols)
/// * `level` - Contour level to extract
///
/// # Returns
/// Line segments as `(x1, y1, x2, y2)`.
///
/// # Errors
/// Same shape validation as [`contour_lines`].
pub fn marching_squares(
    x: &[f64],
    y: &[f64],
    z: &[Vec<f64>],
    level: f64,
) -> Result<Vec<(f64, f64, f64, f64)>> {
    let (ny, nx) = validate_grid("marching_squares", x, y, z)?;
    if ny < 2 || nx < 2 {
        return Ok(vec![]);
    }

    Ok(marching_squares_unchecked(x, y, z, ny, nx, level))
}

/// Marching squares over an already-validated grid.
///
/// `ny >= 2`, `nx >= 2`, `z` is `ny` rows of `nx` columns, `y.len() == ny` and
/// `x.len() == nx` — all guaranteed by [`validate_grid`].
fn marching_squares_unchecked(
    x: &[f64],
    y: &[f64],
    z: &[Vec<f64>],
    ny: usize,
    nx: usize,
    level: f64,
) -> Vec<(f64, f64, f64, f64)> {
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

        let segments = marching_squares(&x, &y, &z, 0.5).expect("square grid is valid");
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

        let contours = contour_lines(&x, &y, &z, &[0.5]).expect("square grid is valid");
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
        let contours = contour_lines(&[], &[], &[], &[0.5]).expect("empty grid is a valid shape");
        assert!(contours.is_empty());
    }

    /// Regression: `marching_squares` used to do `z[0].len()` with no guard and
    /// aborted the process on an empty grid instead of returning a `Result`.
    #[test]
    fn test_marching_squares_reports_empty_grid_instead_of_panicking() {
        assert!(
            marching_squares(&[], &[], &[], 0.5)
                .expect("empty grid is a valid shape")
                .is_empty()
        );
        // A single row/column has no marching-squares cells, but is not invalid.
        assert!(
            marching_squares(&[0.0], &[0.0], &[vec![1.0]], 0.5)
                .expect("1x1 grid is a valid shape")
                .is_empty()
        );
    }

    /// Regression: ragged rows used to index out of bounds via `z[0].len()`.
    #[test]
    fn test_ragged_grid_is_rejected() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![0.0, 1.0];
        let z = vec![vec![0.0, 1.0, 2.0], vec![3.0, 4.0]];

        assert!(matches!(
            marching_squares(&x, &y, &z, 0.5),
            Err(PlottingError::RaggedData2D { row: 1, .. })
        ));
        assert!(matches!(
            contour_lines(&x, &y, &z, &[0.5]),
            Err(PlottingError::RaggedData2D { row: 1, .. })
        ));
    }

    /// Regression: grid dimensions were taken from `z` while `x`/`y` were
    /// indexed at `i + 1`, so a short `x` panicked.
    #[test]
    fn test_coordinate_length_mismatch_is_rejected() {
        let x = vec![0.0, 1.0];
        let y = vec![0.0, 1.0];
        let z = vec![vec![0.0, 1.0, 2.0], vec![3.0, 4.0, 5.0]];

        assert!(matches!(
            marching_squares(&x, &y, &z, 0.5),
            Err(PlottingError::GridShapeMismatch {
                expected_columns: 2,
                actual_columns: 3,
                ..
            })
        ));
        assert!(matches!(
            contour_lines(&x, &y, &z, &[0.5]),
            Err(PlottingError::GridShapeMismatch {
                expected_rows: 2,
                actual_rows: 2,
                ..
            })
        ));
    }
}
