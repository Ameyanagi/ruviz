//! Contour extraction using marching squares
//!
//! Provides both halves of a contour plot from the same 2D scalar field:
//!
//! - [`contour_lines`] / [`marching_squares`] — the iso*lines* at a set of levels.
//! - [`contour_bands`] — the iso*bands* between consecutive levels, as polygons.
//!
//! Both walk the same cells, classify corners with the same `value >= level`
//! test, resolve saddles with the same cell-centre rule and interpolate
//! crossings with the same `interpolate_crossing` helper, so a filled band
//! boundary lands exactly on the line drawn at that level.

use crate::core::{PlottingError, Result};

/// A single contour level with its lines
#[derive(Debug, Clone)]
pub struct ContourLevel {
    /// The z-value of this contour level
    pub level: f64,
    /// Line segments as (x1, y1, x2, y2)
    pub segments: Vec<(f64, f64, f64, f64)>,
}

/// A single filled contour band: everything whose value is in `[lower, upper)`.
///
/// Bands are half-open so that consecutive bands tile the value axis without
/// overlapping, which is what makes every point of the grid belong to exactly
/// one band.
#[derive(Debug, Clone, PartialEq)]
pub struct ContourBand {
    /// Inclusive lower bound, [`f64::NEG_INFINITY`] for the open-ended bottom band.
    pub lower: f64,
    /// Exclusive upper bound, [`f64::INFINITY`] for the open-ended top band.
    pub upper: f64,
    /// The polygons covering this band, each a closed ring of `(x, y)` vertices
    /// in data space, wound counter-clockwise. One polygon never spans more
    /// than one grid cell.
    pub polygons: Vec<Vec<(f64, f64)>>,
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

/// Linearly interpolate the point where an edge crosses `level`.
///
/// `va`/`vb` are the field values at the two ends of the edge and `a`/`b` the
/// coordinate being interpolated (an `x`, a `y`, or either along a diagonal).
/// A degenerate edge — both ends carrying the same value — has no single
/// crossing, so its midpoint is used.
///
/// Both the line tracer and the isoband tracer go through this one function, so
/// a band boundary cannot land anywhere other than on the line drawn at the
/// same level.
fn interpolate_crossing(va: f64, vb: f64, a: f64, b: f64, level: f64) -> f64 {
    if (vb - va).abs() < 1e-12 {
        (a + b) / 2.0
    } else {
        a + (level - va) / (vb - va) * (b - a)
    }
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
                interpolate_crossing(va, vb, a, b, level)
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

// ============================================================================
// Isobands (filled contours)
// ============================================================================
//
// Marching squares for *filled* regions is a different case table from marching
// squares for lines: each corner is classified below / inside / above the band
// instead of below / above a single level, so the table has 3^4 = 81 entries
// rather than 16. The table published as "marching squares with isobands" (the
// one MarchingSquares.js and d3-contour implement, after Nielson & Hamann's
// treatment of the ambiguous cases) is not an independent set of facts: every
// entry of it is the cell square clipped by `z >= lower` and then by
// `z < upper`, with the crossings interpolated linearly along the cell edges.
//
// This module computes that clip directly (Sutherland–Hodgman, carrying the
// field value on each vertex) instead of transcribing the 81 entries. That is
// the same choice the rest of the crate makes wherever a table and a rule
// disagree: a rule cannot drift from itself, whereas one mistyped row of a
// table is a wrong polygon nobody notices. It also collapses the "which corner
// is which" bookkeeping — rotations and reflections of a case are the same code
// path — and generalises for free to the open-ended bands, where one of the two
// clips simply does not run.
//
// The 81-entry table's only genuinely extra content is the ambiguous
// (saddle) cases, where the region kept by a clip is two disjoint corner blobs
// rather than one connected piece. Sutherland–Hodgman always emits a single
// ring, so those cells are split into two triangles first — see
// `saddle_split`, which resolves the ambiguity with the same cell-centre
// average `marching_squares` uses, so a band boundary never contradicts the
// line drawn over it.

/// A cell-corner vertex carrying the field value the clipper classifies it by.
#[derive(Debug, Clone, Copy)]
struct BandVertex {
    x: f64,
    y: f64,
    z: f64,
}

/// Which diagonal an ambiguous cell is split along.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Diagonal {
    /// Between corners 0 (bottom-left) and 2 (top-right).
    BottomLeftTopRight,
    /// Between corners 1 (bottom-right) and 3 (top-left).
    BottomRightTopLeft,
}

/// The area of a closed ring, by the shoelace formula. Always non-negative, so
/// it does not depend on the winding.
///
/// Same formula as [`Polygon::area`], on a borrowed ring: this runs once per
/// clipped cell, and `Polygon` owns its vertices, so going through it would mean
/// an allocation per cell purely to measure one. `Polygon::signed_area` should
/// delegate here rather than keep its own copy of the formula.
///
/// [`Polygon::area`]: crate::render::primitives::polygon::Polygon::area
pub(crate) fn polygon_area(ring: &[(f64, f64)]) -> f64 {
    if ring.len() < 3 {
        return 0.0;
    }

    let mut twice_area = 0.0;
    for index in 0..ring.len() {
        let (x0, y0) = ring[index];
        let (x1, y1) = ring[(index + 1) % ring.len()];
        twice_area += x0 * y1 - x1 * y0;
    }

    (twice_area / 2.0).abs()
}

/// Clip a polygon to one side of `level` (Sutherland–Hodgman).
///
/// `inside` decides which side is kept; it must be consistent with `level`, as
/// vertices introduced where the two sides meet are placed by
/// `interpolate_crossing` at `level`.
fn clip_to_level(
    polygon: &[BandVertex],
    level: f64,
    inside: impl Fn(f64) -> bool,
) -> Vec<BandVertex> {
    if polygon.len() < 3 {
        return Vec::new();
    }

    let mut clipped = Vec::with_capacity(polygon.len() + 2);
    for (index, &current) in polygon.iter().enumerate() {
        let previous = polygon[(index + polygon.len() - 1) % polygon.len()];
        let current_inside = inside(current.z);

        if current_inside != inside(previous.z) {
            clipped.push(BandVertex {
                x: interpolate_crossing(previous.z, current.z, previous.x, current.x, level),
                y: interpolate_crossing(previous.z, current.z, previous.y, current.y, level),
                z: level,
            });
        }
        if current_inside {
            clipped.push(current);
        }
    }

    clipped
}

/// The part of one convex piece of a cell that belongs to `[lower, upper)`.
///
/// An open-ended bound skips its clip entirely, which is why the `-inf` and
/// `+inf` bands paint the peaks and pits whole instead of leaving them white.
fn clip_piece_to_band(
    piece: &[BandVertex],
    lower: f64,
    upper: f64,
    area_epsilon: f64,
) -> Option<Vec<(f64, f64)>> {
    let mut polygon = if lower.is_finite() {
        clip_to_level(piece, lower, |z| z >= lower)
    } else {
        piece.to_vec()
    };

    if upper.is_finite() {
        // Strictly below, matching the half-open `[lower, upper)` band: a cell
        // sitting exactly on a level belongs to the band above it and to that
        // band only.
        polygon = clip_to_level(&polygon, upper, |z| z < upper);
    }

    let ring: Vec<(f64, f64)> = polygon.iter().map(|vertex| (vertex.x, vertex.y)).collect();
    (polygon_area(&ring) > area_epsilon).then_some(ring)
}

/// Do the four corner classifications alternate around the cell?
///
/// That is the marching-squares ambiguous configuration: one diagonal pair sits
/// above the level and the other below it, and the level's isoline can either
/// separate the two above corners or join them.
fn is_alternating(above: [bool; 4]) -> bool {
    above[0] == above[2] && above[1] == above[3] && above[0] != above[1]
}

/// Which corners of the cell are at or above `level`, in corner order.
fn corners_above(corners: &[BandVertex; 4], level: f64) -> [bool; 4] {
    [
        corners[0].z >= level,
        corners[1].z >= level,
        corners[2].z >= level,
        corners[3].z >= level,
    ]
}

/// The diagonal an ambiguous cell must be split along before clipping, if any.
///
/// A Sutherland–Hodgman clip emits one ring, so it can only ever produce the
/// *connected* reading of an ambiguous cell. That reading is correct exactly
/// when the cell centre is on the side being kept — the bilinear surface's
/// saddle sits at the centre, so whichever side the centre is on is the
/// connected one. When it is not, the cell is cut along the diagonal joining
/// the corner pair on the centre's side, which leaves one kept corner in each
/// triangle and so yields the two disjoint blobs.
///
/// This is the same rule, and the same `center >= level` comparison, that
/// `marching_squares_unchecked` resolves its saddle cases with.
///
/// At most one of the two bounds can ever ask for a split: the lower bound only
/// asks when `center < lower` and the upper bound only when `center >= upper`,
/// and `lower <= upper`.
fn saddle_split(
    corners: &[BandVertex; 4],
    center: f64,
    lower: f64,
    upper: f64,
) -> Option<Diagonal> {
    if lower.is_finite() {
        let above = corners_above(corners, lower);
        if is_alternating(above) && center < lower {
            // Keeping `z >= lower`, and the centre is below: the kept side is
            // the two corners above, split along the pair below.
            return Some(if above[0] {
                Diagonal::BottomRightTopLeft
            } else {
                Diagonal::BottomLeftTopRight
            });
        }
    }

    if upper.is_finite() {
        let above = corners_above(corners, upper);
        if is_alternating(above) && center >= upper {
            // Keeping `z < upper`, and the centre is above: the kept side is
            // the two corners below, split along the pair above.
            return Some(if above[0] {
                Diagonal::BottomLeftTopRight
            } else {
                Diagonal::BottomRightTopLeft
            });
        }
    }

    None
}

/// Append the polygons one cell contributes to one band.
fn push_cell_band_polygons(
    corners: &[BandVertex; 4],
    center: f64,
    lower: f64,
    upper: f64,
    area_epsilon: f64,
    polygons: &mut Vec<Vec<(f64, f64)>>,
) {
    let mut push = |piece: &[BandVertex]| {
        if let Some(polygon) = clip_piece_to_band(piece, lower, upper, area_epsilon) {
            polygons.push(polygon);
        }
    };

    // Both triangles of a split are wound counter-clockwise, like the cell.
    match saddle_split(corners, center, lower, upper) {
        None => push(corners.as_slice()),
        Some(Diagonal::BottomLeftTopRight) => {
            push([corners[0], corners[1], corners[2]].as_slice());
            push([corners[0], corners[2], corners[3]].as_slice());
        }
        Some(Diagonal::BottomRightTopLeft) => {
            push([corners[0], corners[1], corners[3]].as_slice());
            push([corners[1], corners[2], corners[3]].as_slice());
        }
    }
}

/// The half-open bands a set of levels defines.
///
/// Besides the bands between consecutive levels, the first and last are
/// open-ended — `(-inf, levels[0])` and `[levels.last(), +inf)` — mirroring
/// matplotlib's `extend="both"`. Without them every value below the lowest
/// level or above the highest one, i.e. every pit and every peak, would belong
/// to no band and be left unpainted. Together the bands tile the whole value
/// axis, so a finite value belongs to exactly one of them.
///
/// Levels are sorted and de-duplicated, and non-finite levels dropped, so an
/// unordered level list still yields a tiling instead of a set of empty bands.
fn band_bounds(levels: &[f64]) -> Vec<(f64, f64)> {
    let mut sorted: Vec<f64> = levels
        .iter()
        .copied()
        .filter(|level| level.is_finite())
        .collect();
    sorted.sort_by(f64::total_cmp);
    sorted.dedup();

    if sorted.is_empty() {
        return Vec::new();
    }

    let mut bounds = Vec::with_capacity(sorted.len() + 1);
    bounds.push((f64::NEG_INFINITY, sorted[0]));
    for pair in sorted.windows(2) {
        bounds.push((pair[0], pair[1]));
    }
    bounds.push((sorted[sorted.len() - 1], f64::INFINITY));

    bounds
}

/// Extract filled contour bands (isobands) from a 2D scalar field.
///
/// Where [`contour_lines`] returns the isoline at each level, this returns the
/// polygons *between* consecutive levels: the shape a filled contour is drawn
/// from. Cells crossed by a level are cut along the crossing, so a filled
/// contour is a set of smooth bands rather than a mosaic of whole cells.
///
/// # Arguments
/// * `x` - X coordinates of grid (length nx)
/// * `y` - Y coordinates of grid (length ny)
/// * `z` - Z values as row-major 2D array (ny rows × nx cols)
/// * `levels` - Z values separating the bands
///
/// # Returns
/// `levels.len() + 1` bands in ascending order, the first and last open-ended:
/// `(-inf, levels[0])` and `[levels.last(), +inf)`, so that peaks and pits stay
/// painted. Together the bands cover every cell of the grid exactly
/// once, except cells with a non-finite corner, which are left unpainted the
/// way [`marching_squares`] leaves them untraced.
///
/// An empty `levels` returns no bands, since there is nothing to separate.
///
/// # Complexity
/// `O(cells × bands)` comparisons and `O(cells)` polygons for a field that is
/// smooth relative to its levels: each cell rejects a band it cannot touch with
/// two comparisons against the cell's corner range, and a cell that lies wholly
/// inside a band is emitted as itself without clipping. Only the cells a level
/// actually crosses run the clipper, at constant cost each.
///
/// # Errors
/// Same shape validation as [`contour_lines`].
pub fn contour_bands(
    x: &[f64],
    y: &[f64],
    z: &[Vec<f64>],
    levels: &[f64],
) -> Result<Vec<ContourBand>> {
    let (ny, nx) = validate_grid("contour_bands", x, y, z)?;
    let bounds = band_bounds(levels);
    if ny < 2 || nx < 2 || bounds.is_empty() {
        return Ok(vec![]);
    }

    let mut polygons: Vec<Vec<Vec<(f64, f64)>>> = vec![Vec::new(); bounds.len()];

    for j in 0..(ny - 1) {
        for i in 0..(nx - 1) {
            // Counter-clockwise from bottom-left, the corner order the line
            // tracer uses.
            let corners = [
                BandVertex {
                    x: x[i],
                    y: y[j],
                    z: z[j][i],
                },
                BandVertex {
                    x: x[i + 1],
                    y: y[j],
                    z: z[j][i + 1],
                },
                BandVertex {
                    x: x[i + 1],
                    y: y[j + 1],
                    z: z[j + 1][i + 1],
                },
                BandVertex {
                    x: x[i],
                    y: y[j + 1],
                    z: z[j + 1][i],
                },
            ];

            // Same rule as the line tracer: a cell with a NaN (or infinite)
            // corner has no meaningful crossing, so it stays unpainted.
            if !corners.iter().all(|corner| corner.z.is_finite()) {
                continue;
            }

            // A bilinear cell has no interior extremum, so the corners bound it.
            let cell_min = corners.iter().fold(f64::INFINITY, |acc, c| acc.min(c.z));
            let cell_max = corners
                .iter()
                .fold(f64::NEG_INFINITY, |acc, c| acc.max(c.z));
            let center = corners.iter().map(|corner| corner.z).sum::<f64>() / 4.0;
            let cell_area = ((corners[1].x - corners[0].x) * (corners[3].y - corners[0].y)).abs();
            // Slivers below this are rounding noise from the clip, not geometry.
            let area_epsilon = cell_area * 1e-12;

            for (band_index, &(lower, upper)) in bounds.iter().enumerate() {
                if cell_max < lower || cell_min >= upper {
                    continue;
                }

                if cell_min >= lower && cell_max < upper {
                    // Wholly inside: emit the cell itself. This is the common
                    // case in the interior of a band, and keeping it an exact
                    // axis-aligned rectangle is what lets the renderers tile it
                    // without anti-aliased seams.
                    polygons[band_index].push(vec![
                        (corners[0].x, corners[0].y),
                        (corners[1].x, corners[1].y),
                        (corners[2].x, corners[2].y),
                        (corners[3].x, corners[3].y),
                    ]);
                    continue;
                }

                push_cell_band_polygons(
                    &corners,
                    center,
                    lower,
                    upper,
                    area_epsilon,
                    &mut polygons[band_index],
                );
            }
        }
    }

    Ok(bounds
        .into_iter()
        .zip(polygons)
        .map(|((lower, upper), polygons)| ContourBand {
            lower,
            upper,
            polygons,
        })
        .collect())
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

    // ------------------------------------------------------------------
    // Isobands
    // ------------------------------------------------------------------

    /// A unit-spaced `n × n` grid with a smooth peak at its centre.
    fn peak_grid(n: usize) -> (Vec<f64>, Vec<f64>, Vec<Vec<f64>>) {
        let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let center = (n - 1) as f64 / 2.0;
        let z = (0..n)
            .map(|iy| {
                (0..n)
                    .map(|ix| {
                        let dx = ix as f64 - center;
                        let dy = iy as f64 - center;
                        (-(dx * dx + dy * dy) / 4.0).exp()
                    })
                    .collect()
            })
            .collect();

        (x.clone(), x, z)
    }

    fn total_band_area(bands: &[ContourBand]) -> f64 {
        bands
            .iter()
            .flat_map(|band| band.polygons.iter())
            .map(|polygon| polygon_area(polygon.as_slice()))
            .sum()
    }

    /// The point of isobands: a cell a level runs through is *cut*, not painted
    /// whole. Here the level sits halfway up a vertical ramp, so each band must
    /// come out as half the cell.
    #[test]
    fn test_isobands_cut_cells_along_the_level() {
        let x = vec![0.0, 1.0];
        let y = vec![0.0, 1.0];
        let z = vec![vec![0.0, 0.0], vec![2.0, 2.0]];

        let bands = contour_bands(&x, &y, &z, &[1.0]).expect("square grid is valid");
        assert_eq!(bands.len(), 2);

        for band in &bands {
            assert_eq!(band.polygons.len(), 1);
            assert!(
                (polygon_area(&band.polygons[0]) - 0.5).abs() < 1e-12,
                "a level through the middle of a cell must halve it, got {:?}",
                band.polygons[0]
            );
        }

        // The cut lands on the interpolated crossing, not on a cell edge.
        assert!(
            bands[0].polygons[0]
                .iter()
                .any(|&(_, py)| (py - 0.5).abs() < 1e-12),
            "the band boundary must sit at the interpolated crossing"
        );
    }

    /// Every band boundary must land exactly where the contour line at that
    /// level is drawn, or the fill and the line would disagree on screen.
    #[test]
    fn test_isoband_edges_land_on_the_contour_line() {
        let (x, y, z) = peak_grid(5);
        let level = 0.5;

        let segments = marching_squares(&x, &y, &z, level).expect("square grid is valid");
        assert!(!segments.is_empty());

        let vertices: Vec<(f64, f64)> = contour_bands(&x, &y, &z, &[level])
            .expect("square grid is valid")
            .iter()
            .flat_map(|band| band.polygons.iter())
            .flatten()
            .copied()
            .collect();

        for (x1, y1, x2, y2) in segments {
            for point in [(x1, y1), (x2, y2)] {
                assert!(
                    vertices
                        .iter()
                        .any(|v| (v.0 - point.0).abs() < 1e-12 && (v.1 - point.1).abs() < 1e-12),
                    "contour line endpoint {point:?} is not a band vertex"
                );
            }
        }
    }

    /// The bands tile the grid: their total area is the grid's area, so nothing
    /// is painted twice and nothing is left white.
    #[test]
    fn test_isobands_tile_the_grid() {
        let (x, y, z) = peak_grid(9);
        let levels = auto_levels(&z, 5);

        let bands = contour_bands(&x, &y, &z, &levels).expect("square grid is valid");
        assert_eq!(bands.len(), levels.len() + 1);
        assert!((total_band_area(&bands) - 64.0).abs() < 1e-9);
    }

    /// A saddle cell is genuinely ambiguous, and the fill has to resolve it the
    /// way the line tracer does: with the cell-centre average. Here the centre
    /// is below the level, so the top band is two corner triangles rather than
    /// one blob spanning the cell — and the cell still tiles exactly.
    #[test]
    fn test_isobands_split_ambiguous_saddle_cells() {
        let x = vec![0.0, 1.0];
        let y = vec![0.0, 1.0];
        let z = vec![vec![12.0, 0.0], vec![0.0, 12.0]];

        let bands = contour_bands(&x, &y, &z, &[1.0, 10.0]).expect("square grid is valid");
        assert_eq!(bands.len(), 3);
        assert_eq!(
            bands[0].polygons.len(),
            2,
            "two corners below the low level"
        );
        assert_eq!(bands[1].polygons.len(), 1, "the middle band is connected");
        assert_eq!(
            bands[2].polygons.len(),
            2,
            "two corners above the high level"
        );
        assert!((total_band_area(&bands) - 1.0).abs() < 1e-12);
    }

    /// The open-ended bands are what keep peaks and pits painted: with levels
    /// that stop short of the data range, the extremes still belong to a band.
    #[test]
    fn test_isobands_are_open_ended() {
        let (x, y, z) = peak_grid(7);

        let bands = contour_bands(&x, &y, &z, &[0.2, 0.4]).expect("square grid is valid");
        assert_eq!(bands.len(), 3);
        assert_eq!(bands[0].lower, f64::NEG_INFINITY);
        assert_eq!(bands[2].upper, f64::INFINITY);
        assert!(!bands[0].polygons.is_empty(), "the pit must be painted");
        assert!(!bands[2].polygons.is_empty(), "the peak must be painted");
        assert!((total_band_area(&bands) - 36.0).abs() < 1e-9);
    }

    /// A single level is not a degenerate case: it still yields the two
    /// open-ended bands, and they still tile the grid.
    #[test]
    fn test_isobands_with_a_single_level() {
        let (x, y, z) = peak_grid(5);

        let bands = contour_bands(&x, &y, &z, &[0.5]).expect("square grid is valid");
        assert_eq!(bands.len(), 2);
        assert!((total_band_area(&bands) - 16.0).abs() < 1e-9);
    }

    /// Bands are half-open, so a constant field — where every level coincides
    /// with the data — is painted exactly once, not once per band.
    #[test]
    fn test_isobands_on_a_constant_field_paint_once() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![0.0, 1.0, 2.0];
        let z = vec![vec![5.0; 3]; 3];

        let bands = contour_bands(&x, &y, &z, &[5.0]).expect("square grid is valid");
        assert_eq!(bands.len(), 2);
        assert!(bands[0].polygons.is_empty(), "nothing is below the level");
        assert_eq!(bands[1].polygons.len(), 4, "every cell, exactly once");
        assert!((total_band_area(&bands) - 4.0).abs() < 1e-12);
    }

    /// Unordered or repeated levels still describe the same set of bands — a
    /// level list is a set of boundaries, not a sequence.
    #[test]
    fn test_isoband_levels_are_sorted_and_deduplicated() {
        let (x, y, z) = peak_grid(5);

        let ordered = contour_bands(&x, &y, &z, &[0.2, 0.5, 0.8]).expect("square grid is valid");
        let jumbled =
            contour_bands(&x, &y, &z, &[0.8, 0.2, 0.5, 0.2]).expect("square grid is valid");

        assert_eq!(ordered, jumbled);
        assert!((total_band_area(&ordered) - 16.0).abs() < 1e-9);
    }

    /// A NaN corner has no meaningful crossing, so the cells touching it stay
    /// unpainted — the same cells the line tracer skips.
    #[test]
    fn test_isobands_skip_non_finite_cells() {
        let (x, y, mut z) = peak_grid(5);
        z[2][2] = f64::NAN;

        let bands = contour_bands(&x, &y, &z, &[0.3, 0.6]).expect("square grid is valid");
        // Four of the sixteen cells touch the NaN vertex.
        assert!((total_band_area(&bands) - 12.0).abs() < 1e-9);
    }

    /// Degenerate inputs report, they do not panic — and they report through the
    /// same validation the line tracer uses.
    #[test]
    fn test_contour_bands_degenerate_inputs() {
        assert!(
            contour_bands(&[], &[], &[], &[0.5])
                .expect("empty grid is a valid shape")
                .is_empty()
        );
        assert!(
            contour_bands(&[0.0], &[0.0], &[vec![1.0]], &[0.5])
                .expect("1x1 grid is a valid shape")
                .is_empty()
        );

        let (x, y, z) = peak_grid(3);
        assert!(
            contour_bands(&x, &y, &z, &[])
                .expect("no levels is a valid request")
                .is_empty(),
            "no levels means no bands to separate"
        );
        assert!(
            contour_bands(&x, &y, &z, &[f64::NAN])
                .expect("a NaN level is not a shape error")
                .is_empty()
        );

        let ragged = vec![vec![0.0, 1.0, 2.0], vec![3.0, 4.0]];
        assert!(matches!(
            contour_bands(&[0.0, 1.0, 2.0], &[0.0, 1.0], &ragged, &[0.5]),
            Err(PlottingError::RaggedData2D { row: 1, .. })
        ));

        let wide = vec![vec![0.0, 1.0, 2.0], vec![3.0, 4.0, 5.0]];
        assert!(matches!(
            contour_bands(&[0.0, 1.0], &[0.0, 1.0], &wide, &[0.5]),
            Err(PlottingError::GridShapeMismatch {
                expected_columns: 2,
                actual_columns: 3,
                ..
            })
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
