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

    // Calculate bandwidth using Scott's rule if not provided
    let bw = bandwidth.unwrap_or_else(|| scotts_rule(data));

    // Find data range
    let min_val = data.iter().copied().fold(f64::INFINITY, f64::min);
    let max_val = data.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    // Extend range by 3 bandwidths
    let x_min = min_val - 3.0 * bw;
    let x_max = max_val + 3.0 * bw;

    // Create evaluation grid
    let step = (x_max - x_min) / (n_points - 1) as f64;
    let x: Vec<f64> = (0..n_points).map(|i| x_min + i as f64 * step).collect();

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

    // Calculate bandwidths
    let (bw_x, bw_y) = bandwidth.unwrap_or_else(|| (scotts_rule(x), scotts_rule(y)));

    // Find data ranges
    let x_min = x.iter().copied().fold(f64::INFINITY, f64::min) - 3.0 * bw_x;
    let x_max = x.iter().copied().fold(f64::NEG_INFINITY, f64::max) + 3.0 * bw_x;
    let y_min = y.iter().copied().fold(f64::INFINITY, f64::min) - 3.0 * bw_y;
    let y_max = y.iter().copied().fold(f64::NEG_INFINITY, f64::max) + 3.0 * bw_y;

    // Create grids
    let x_step = (x_max - x_min) / (grid_size - 1) as f64;
    let y_step = (y_max - y_min) / (grid_size - 1) as f64;

    let x_grid: Vec<f64> = (0..grid_size).map(|i| x_min + i as f64 * x_step).collect();
    let y_grid: Vec<f64> = (0..grid_size).map(|i| y_min + i as f64 * y_step).collect();

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

/// Scott's rule for bandwidth selection
fn scotts_rule(data: &[f64]) -> f64 {
    let n = data.len() as f64;
    if n < 2.0 {
        return 1.0;
    }

    let mean = data.iter().sum::<f64>() / n;
    let variance = data.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let std_dev = variance.sqrt();

    // Scott's rule: h = 1.06 * sigma * n^(-1/5)
    1.06 * std_dev * n.powf(-0.2)
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
}
