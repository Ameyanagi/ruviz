//! Kernel Density Estimation
//!
//! Provides 1D and 2D kernel density estimation using Gaussian kernels.
//! Used by violin plots, KDE plots, and 2D density visualizations.

use std::f64::consts::PI;

/// Result of kernel density estimation
#[derive(Debug, Clone)]
pub struct KdeResult {
    /// X coordinates of the density curve
    pub x: Vec<f64>,
    /// Density values at each x coordinate
    pub density: Vec<f64>,
    /// Bandwidth used for estimation
    pub bandwidth: f64,
}

/// Compute 1D kernel density estimation using Gaussian kernel
///
/// # Arguments
/// * `data` - Input data points
/// * `bandwidth` - Optional bandwidth (uses Scott's rule if None)
/// * `n_points` - Number of points in output grid (default 100)
///
/// # Returns
/// KdeResult containing x coordinates and density values
pub fn kde_1d(data: &[f64], bandwidth: Option<f64>, n_points: Option<usize>) -> KdeResult {
    let n_points = n_points.unwrap_or(100);

    if data.is_empty() {
        return KdeResult {
            x: vec![],
            density: vec![],
            bandwidth: 0.0,
        };
    }

    // Scott's rule unless the caller supplied a usable bandwidth.
    let bw = resolve_bandwidth(bandwidth, data);

    // Find data range
    let min_val = data.iter().copied().fold(f64::INFINITY, f64::min);
    let max_val = data.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    // Extend range by 2 bandwidths (balances smoothness with avoiding overflow)
    let x_min = min_val - 2.0 * bw;
    let x_max = max_val + 2.0 * bw;

    // Create evaluation grid
    let x = linspace(x_min, x_max, n_points);

    // Calculate density at each point
    let n = data.len() as f64;
    let density: Vec<f64> = x
        .iter()
        .map(|&xi| {
            let sum: f64 = data.iter().map(|&d| gaussian_kernel((xi - d) / bw)).sum();
            sum / (n * bw)
        })
        .collect();

    KdeResult {
        x,
        density,
        bandwidth: bw,
    }
}

/// Compute Gaussian KDE (alias for kde_1d)
pub fn gaussian_kde(data: &[f64], bandwidth: Option<f64>) -> KdeResult {
    kde_1d(data, bandwidth, None)
}

/// Compute 2D kernel density estimation
///
/// # Arguments
/// * `x` - X coordinates of data points
/// * `y` - Y coordinates of data points
/// * `bandwidth` - Optional bandwidth tuple (bw_x, bw_y)
/// * `grid_size` - Size of output grid (default 50x50)
///
/// # Returns
/// Tuple of (x_grid, y_grid, density_matrix)
pub fn kde_2d(
    x: &[f64],
    y: &[f64],
    bandwidth: Option<(f64, f64)>,
    grid_size: Option<usize>,
) -> (Vec<f64>, Vec<f64>, Vec<Vec<f64>>) {
    let grid_size = grid_size.unwrap_or(50);
    let n = x.len().min(y.len());

    if n == 0 {
        return (vec![], vec![], vec![]);
    }

    // Calculate bandwidths (each axis falls back independently)
    let (requested_x, requested_y) = bandwidth.unzip();
    let bw_x = resolve_bandwidth(requested_x, x);
    let bw_y = resolve_bandwidth(requested_y, y);

    // Find data ranges
    let x_min = x.iter().copied().fold(f64::INFINITY, f64::min) - 3.0 * bw_x;
    let x_max = x.iter().copied().fold(f64::NEG_INFINITY, f64::max) + 3.0 * bw_x;
    let y_min = y.iter().copied().fold(f64::INFINITY, f64::min) - 3.0 * bw_y;
    let y_max = y.iter().copied().fold(f64::NEG_INFINITY, f64::max) + 3.0 * bw_y;

    // Create grids
    let x_grid = linspace(x_min, x_max, grid_size);
    let y_grid = linspace(y_min, y_max, grid_size);

    // Calculate density on grid
    let n_f = n as f64;
    let density: Vec<Vec<f64>> = y_grid
        .iter()
        .map(|&yi| {
            x_grid
                .iter()
                .map(|&xi| {
                    let sum: f64 = (0..n)
                        .map(|i| {
                            let kx = gaussian_kernel((xi - x[i]) / bw_x);
                            let ky = gaussian_kernel((yi - y[i]) / bw_y);
                            kx * ky
                        })
                        .sum();
                    sum / (n_f * bw_x * bw_y)
                })
                .collect()
        })
        .collect();

    (x_grid, y_grid, density)
}

/// Gaussian kernel function
#[inline]
fn gaussian_kernel(x: f64) -> f64 {
    (-0.5 * x * x).exp() / (2.0 * PI).sqrt()
}

/// Bandwidth used when every scale estimate degenerates (constant data, a
/// single sample, or a non-finite sample).
const FALLBACK_BANDWIDTH: f64 = 1.0;

/// Is `bandwidth` usable as a Gaussian KDE bandwidth?
///
/// A zero, negative, or non-finite bandwidth makes every kernel evaluation
/// `0.0 / 0.0`, so the density comes back all-NaN and the plot silently renders
/// blank. scipy and matplotlib both refuse such a bandwidth; this predicate is
/// the single place that decides what "usable" means, so no caller can drift.
#[inline]
pub fn is_valid_bandwidth(bandwidth: f64) -> bool {
    bandwidth.is_finite() && bandwidth > 0.0
}

/// Clamp a computed bandwidth to a strictly positive, finite value.
#[inline]
fn positive_bandwidth_or_fallback(bandwidth: f64) -> f64 {
    if is_valid_bandwidth(bandwidth) {
        bandwidth
    } else {
        FALLBACK_BANDWIDTH
    }
}

/// Resolve the bandwidth for a KDE evaluation.
///
/// An explicitly supplied bandwidth wins only when [`is_valid_bandwidth`]
/// accepts it; otherwise the normal-reference rule (which is itself floored)
/// takes over.
#[inline]
fn resolve_bandwidth(explicit: Option<f64>, data: &[f64]) -> f64 {
    match explicit {
        Some(bandwidth) if is_valid_bandwidth(bandwidth) => bandwidth,
        _ => scotts_rule(data),
    }
}

/// `n` evenly spaced evaluation points spanning `[min, max]`.
///
/// Shared by the 1D and 2D estimators so the degenerate counts are handled
/// identically: `n == 0` used to underflow `n - 1` and panic, and `n == 1`
/// used to evaluate `0.0 * inf` and yield a NaN grid point.
fn linspace(min: f64, max: f64, n: usize) -> Vec<f64> {
    match n {
        0 => Vec::new(),
        1 => vec![(min + max) / 2.0],
        _ => {
            let step = (max - min) / (n - 1) as f64;
            (0..n).map(|i| min + i as f64 * step).collect()
        }
    }
}

/// Sample standard deviation (Bessel-corrected). Returns `None` for `n < 2`.
fn sample_std_dev(data: &[f64]) -> Option<f64> {
    let n = data.len() as f64;
    if n < 2.0 {
        return None;
    }
    let mean = data.iter().sum::<f64>() / n;
    let variance = data.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    Some(variance.sqrt())
}

/// Interquartile range using linear interpolation between order statistics.
///
/// Matches the percentile convention used elsewhere in the crate
/// (`crate::plots::statistics::percentile`).
fn interquartile_range(data: &[f64]) -> f64 {
    if data.len() < 2 {
        return 0.0;
    }
    let mut sorted: Vec<f64> = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let quantile = |p: f64| -> f64 {
        let idx = p * (sorted.len() - 1) as f64;
        let lower = idx.floor() as usize;
        let upper = idx.ceil() as usize;
        let frac = idx - lower as f64;
        sorted[lower] * (1.0 - frac) + sorted[upper] * frac
    };

    quantile(0.75) - quantile(0.25)
}

/// Scott's rule for bandwidth selection.
///
/// `h = 1.06 * sigma * n^(-1/5)`
///
/// Scott, D. W. (1992). *Multivariate Density Estimation*, eq. 6.42 — the
/// normal-reference bandwidth that minimises AMISE for Gaussian data.
///
/// Returns [`FALLBACK_BANDWIDTH`] for fewer than two points, and for
/// zero-variance or non-finite samples, so callers always receive a strictly
/// positive, finite bandwidth (see [`is_valid_bandwidth`]).
pub fn scotts_rule(data: &[f64]) -> f64 {
    let n = data.len() as f64;
    let Some(std_dev) = sample_std_dev(data) else {
        return FALLBACK_BANDWIDTH;
    };

    positive_bandwidth_or_fallback(1.06 * std_dev * n.powf(-0.2))
}

/// Silverman's rule of thumb for bandwidth selection.
///
/// `h = 0.9 * min(sigma, IQR / 1.34) * n^(-1/5)`
///
/// Silverman, B. W. (1986). *Density Estimation for Statistics and Data
/// Analysis*, eq. 3.31. This is the **robust** form: the `IQR / 1.34` term is
/// an outlier-resistant scale estimate (1.34 ≈ the IQR of a standard normal),
/// and the 0.9 factor trades a little efficiency at the normal for much better
/// behaviour on skewed or heavy-tailed samples.
///
/// It is deliberately *not* the `(4/3)^(1/5) * sigma * n^(-1/5)` ≈
/// `1.06 * sigma * n^(-1/5)` normal-reference form, because that is
/// numerically identical to [`scotts_rule`] and would make the two methods
/// indistinguishable.
///
/// Returns [`FALLBACK_BANDWIDTH`] for fewer than two points, and for degenerate
/// samples where both scale estimates collapse to zero — the same floor
/// [`scotts_rule`] uses, so the two estimators cannot diverge on degenerate
/// input.
pub fn silvermans_rule(data: &[f64]) -> f64 {
    let n = data.len() as f64;
    let Some(std_dev) = sample_std_dev(data) else {
        return FALLBACK_BANDWIDTH;
    };

    let iqr_scale = interquartile_range(data) / 1.34;
    // A zero IQR (>50% ties) must not zero out the bandwidth, so it only
    // participates when it is positive.
    let scale = if iqr_scale > 0.0 {
        std_dev.min(iqr_scale)
    } else {
        std_dev
    };

    positive_bandwidth_or_fallback(0.9 * scale * n.powf(-0.2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kde_1d_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = kde_1d(&data, None, Some(50));

        assert_eq!(result.x.len(), 50);
        assert_eq!(result.density.len(), 50);
        assert!(result.bandwidth > 0.0);

        // Density should be positive
        assert!(result.density.iter().all(|&d| d >= 0.0));
    }

    #[test]
    fn test_kde_1d_empty() {
        let result = kde_1d(&[], None, None);
        assert!(result.x.is_empty());
        assert!(result.density.is_empty());
    }

    #[test]
    fn test_kde_2d_basic() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![1.0, 4.0, 2.0, 5.0, 3.0];
        let (x_grid, y_grid, density) = kde_2d(&x, &y, None, Some(10));

        assert_eq!(x_grid.len(), 10);
        assert_eq!(y_grid.len(), 10);
        assert_eq!(density.len(), 10);
        assert_eq!(density[0].len(), 10);
    }

    #[test]
    fn test_scotts_rule() {
        // Normal distribution with std=1
        let data: Vec<f64> = (0..1000).map(|i| (i as f64 - 500.0) / 100.0).collect();
        let bw = scotts_rule(&data);
        assert!(bw > 0.0);
    }

    #[test]
    fn test_silvermans_rule_matches_the_published_formula() {
        let data: Vec<f64> = (0..1000).map(|i| (i as f64 - 500.0) / 100.0).collect();
        let n = data.len() as f64;

        let mean = data.iter().sum::<f64>() / n;
        let sigma = (data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
        let iqr = interquartile_range(&data);
        let expected = 0.9 * sigma.min(iqr / 1.34) * n.powf(-0.2);

        assert!((silvermans_rule(&data) - expected).abs() < 1e-12);
    }

    #[test]
    fn test_silvermans_rule_is_not_scotts_rule() {
        // The whole point of the two names: they must produce different numbers.
        let data: Vec<f64> = (0..500).map(|i| (i as f64 * 0.37).sin() * 10.0).collect();
        let scott = scotts_rule(&data);
        let silverman = silvermans_rule(&data);

        assert!(scott > 0.0 && silverman > 0.0);
        assert!(
            (scott - silverman).abs() > 1e-9,
            "Silverman ({silverman}) silently collapsed onto Scott ({scott})"
        );
    }

    #[test]
    fn test_silvermans_rule_is_robust_to_outliers() {
        // One extreme point inflates sigma but barely moves the IQR, so the
        // robust rule must react far less than the normal-reference rule.
        let mut data: Vec<f64> = (0..200).map(|i| i as f64 / 200.0).collect();
        let scott_clean = scotts_rule(&data);
        let silverman_clean = silvermans_rule(&data);

        data.push(1_000.0);
        let scott_dirty = scotts_rule(&data);
        let silverman_dirty = silvermans_rule(&data);

        let scott_growth = scott_dirty / scott_clean;
        let silverman_growth = silverman_dirty / silverman_clean;
        assert!(
            silverman_growth < scott_growth,
            "silverman_growth={silverman_growth} scott_growth={scott_growth}"
        );
    }

    #[test]
    fn test_bandwidth_rules_stay_positive_on_degenerate_input() {
        assert_eq!(scotts_rule(&[]), 1.0);
        assert_eq!(silvermans_rule(&[]), 1.0);
        assert_eq!(scotts_rule(&[3.0]), 1.0);
        assert_eq!(silvermans_rule(&[3.0]), 1.0);
        assert_eq!(scotts_rule(&[7.0; 50]), 1.0);
        assert_eq!(silvermans_rule(&[7.0; 50]), 1.0);
    }

    /// Regression: constant data gave a zero bandwidth, hence a zero-width grid
    /// and `0.0 / 0.0` densities — a blank plot with no error.
    #[test]
    fn test_kde_1d_of_constant_data_is_finite_and_normalised() {
        let data = vec![7.0; 40];
        let result = kde_1d(&data, None, Some(64));

        assert!(result.bandwidth > 0.0);
        assert!(result.x.iter().all(|value| value.is_finite()));
        assert!(
            result.density.iter().all(|d| d.is_finite() && *d >= 0.0),
            "constant-data KDE produced non-finite densities: {:?}",
            result.density
        );
        assert!(
            result.density.iter().cloned().fold(0.0, f64::max) > 0.0,
            "constant-data KDE produced an entirely flat (blank) curve"
        );
        // The grid must have real extent, otherwise nothing can be drawn.
        assert!(result.x.last().unwrap() > result.x.first().unwrap());
    }

    /// Regression: an explicit non-positive bandwidth reached the kernel and
    /// produced an all-NaN density.
    #[test]
    fn test_kde_1d_rejects_unusable_explicit_bandwidths() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let expected = scotts_rule(&data);

        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let result = kde_1d(&data, Some(bad), Some(32));
            assert_eq!(
                result.bandwidth, expected,
                "bandwidth {bad} should fall back to Scott's rule"
            );
            assert!(result.density.iter().all(|d| d.is_finite()));
        }

        // A usable bandwidth is still honoured verbatim.
        assert_eq!(kde_1d(&data, Some(0.25), Some(32)).bandwidth, 0.25);
    }

    #[test]
    fn test_kde_2d_rejects_unusable_explicit_bandwidths() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![1.0, 4.0, 2.0, 5.0, 3.0];
        let (x_grid, y_grid, density) = kde_2d(&x, &y, Some((0.0, -2.0)), Some(8));

        assert!(x_grid.iter().chain(y_grid.iter()).all(|v| v.is_finite()));
        assert!(density.iter().flatten().all(|d| d.is_finite() && *d >= 0.0));
        assert!(density.iter().flatten().cloned().fold(0.0, f64::max) > 0.0);
    }

    /// Regression: `n_points == 0` underflowed `n_points - 1` and `n_points == 1`
    /// evaluated `0.0 * inf`, so a public call could panic or return a NaN grid.
    #[test]
    fn test_kde_grids_handle_degenerate_point_counts() {
        let data = vec![1.0, 2.0, 3.0];

        let empty = kde_1d(&data, None, Some(0));
        assert!(empty.x.is_empty() && empty.density.is_empty());

        let single = kde_1d(&data, None, Some(1));
        assert_eq!(single.x.len(), 1);
        assert!(single.x[0].is_finite() && single.density[0].is_finite());

        let (x_grid, y_grid, density) = kde_2d(&data, &data, None, Some(1));
        assert_eq!((x_grid.len(), y_grid.len(), density.len()), (1, 1, 1));
        assert!(x_grid[0].is_finite() && y_grid[0].is_finite());
        assert!(density[0][0].is_finite());
    }

    #[test]
    fn test_is_valid_bandwidth_matches_the_floor_applied_by_both_rules() {
        assert!(is_valid_bandwidth(0.5));
        for bad in [0.0, -0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(!is_valid_bandwidth(bad), "{bad} should be rejected");
        }

        // Both estimators route through the same floor.
        for degenerate in [vec![], vec![3.0], vec![7.0; 50]] {
            assert!(is_valid_bandwidth(scotts_rule(&degenerate)));
            assert!(is_valid_bandwidth(silvermans_rule(&degenerate)));
            assert_eq!(scotts_rule(&degenerate), silvermans_rule(&degenerate));
        }
    }

    #[test]
    fn test_bandwidth_rules_reject_non_finite_samples() {
        let data = vec![1.0, 2.0, f64::INFINITY];
        assert_eq!(scotts_rule(&data), FALLBACK_BANDWIDTH);
        assert_eq!(silvermans_rule(&data), FALLBACK_BANDWIDTH);
    }

    #[test]
    fn test_silvermans_rule_survives_a_zero_iqr() {
        // >50% ties give IQR = 0; falling back to sigma keeps the KDE finite.
        let mut data = vec![5.0; 60];
        data.extend([0.0, 1.0, 9.0, 10.0]);
        let bw = silvermans_rule(&data);
        assert!(bw > 0.0 && bw.is_finite());
    }
}
