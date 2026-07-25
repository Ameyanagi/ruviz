//! Regression plot implementations
//!
//! Provides linear regression, polynomial regression, and residual plots.

use crate::render::Color;
use crate::stats::regression::{linear_regression, polynomial_regression};

/// Configuration for regression plot
#[derive(Debug, Clone)]
pub struct RegPlotConfig {
    /// Scatter point color
    pub scatter_color: Option<Color>,
    /// Line color
    pub line_color: Option<Color>,
    /// Scatter point size
    pub scatter_size: f32,
    /// Line width
    pub line_width: f32,
    /// Scatter alpha
    pub scatter_alpha: f32,
    /// Show confidence interval
    pub ci: Option<f64>,
    /// CI fill alpha
    pub ci_alpha: f32,
    /// Regression order (1 = linear, 2 = quadratic, etc.)
    pub order: usize,
    /// Number of points for regression line
    pub n_points: usize,
    /// Constrain the fit to pass through `(0, 0)`
    ///
    /// The constant term is dropped from the design matrix, so the fitted
    /// polynomial is `c1*x + c2*x^2 + ...` with `c0 == 0`.
    pub fit_through_origin: bool,
}

impl Default for RegPlotConfig {
    fn default() -> Self {
        Self {
            scatter_color: None,
            line_color: None,
            scatter_size: 5.0,
            line_width: 2.0,
            scatter_alpha: 0.6,
            ci: Some(95.0),
            ci_alpha: 0.15,
            order: 1,
            n_points: 100,
            fit_through_origin: false,
        }
    }
}

impl RegPlotConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set regression order
    pub fn order(mut self, order: usize) -> Self {
        self.order = order.max(1);
        self
    }

    /// Set scatter color
    pub fn scatter_color(mut self, color: Color) -> Self {
        self.scatter_color = Some(color);
        self
    }

    /// Set line color
    pub fn line_color(mut self, color: Color) -> Self {
        self.line_color = Some(color);
        self
    }

    /// Set scatter size
    pub fn scatter_size(mut self, size: f32) -> Self {
        self.scatter_size = size.max(0.0);
        self
    }

    /// Set line width
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width.max(0.1);
        self
    }

    /// Set confidence interval (None to disable)
    pub fn ci(mut self, ci: Option<f64>) -> Self {
        self.ci = ci.map(|c| c.clamp(0.0, 99.99));
        self
    }

    /// Constrain the fit to pass through the origin
    ///
    /// See [`RegPlotConfig::fit_through_origin`].
    pub fn through_origin(mut self, through: bool) -> Self {
        self.fit_through_origin = through;
        self
    }
}

/// Computed regression data
#[derive(Debug, Clone)]
pub struct RegPlotData {
    /// Original scatter points
    pub scatter_x: Vec<f64>,
    pub scatter_y: Vec<f64>,
    /// Regression line points
    pub line_x: Vec<f64>,
    pub line_y: Vec<f64>,
    /// Confidence interval (if enabled) - lower and upper bounds
    pub ci_lower: Option<Vec<f64>>,
    pub ci_upper: Option<Vec<f64>>,
    /// Regression coefficients
    pub coefficients: Vec<f64>,
    /// R-squared value
    pub r_squared: f64,
}

/// Compute regression plot data
///
/// # Arguments
/// * `x` - X values
/// * `y` - Y values
/// * `config` - Regression plot configuration
///
/// # Returns
/// RegPlotData for rendering
pub fn compute_regplot(x: &[f64], y: &[f64], config: &RegPlotConfig) -> RegPlotData {
    let n = x.len().min(y.len());
    if n < 2 {
        return RegPlotData {
            scatter_x: x.to_vec(),
            scatter_y: y.to_vec(),
            line_x: vec![],
            line_y: vec![],
            ci_lower: None,
            ci_upper: None,
            coefficients: vec![],
            r_squared: 0.0,
        };
    }

    // Fit regression
    let coefficients = if config.fit_through_origin {
        fit_through_origin(x, y, config.order, n)
    } else if config.order == 1 {
        linear_regression(x, y).coefficients
    } else {
        polynomial_regression(x, y, config.order).coefficients
    };

    // Generate line points
    let x_min = x.iter().copied().fold(f64::INFINITY, f64::min);
    let x_max = x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let x_step = (x_max - x_min) / (config.n_points - 1) as f64;

    let line_x: Vec<f64> = (0..config.n_points)
        .map(|i| x_min + i as f64 * x_step)
        .collect();

    let line_y: Vec<f64> = line_x
        .iter()
        .map(|&xi| evaluate_polynomial(&coefficients, xi))
        .collect();

    // Compute R-squared
    let y_mean = y.iter().sum::<f64>() / n as f64;
    let ss_tot: f64 = y.iter().map(|&yi| (yi - y_mean).powi(2)).sum();
    let ss_res: f64 = x
        .iter()
        .zip(y.iter())
        .map(|(&xi, &yi)| {
            let y_pred = evaluate_polynomial(&coefficients, xi);
            (yi - y_pred).powi(2)
        })
        .sum();
    let r_squared = if ss_tot > 0.0 {
        1.0 - ss_res / ss_tot
    } else {
        0.0
    };

    // Compute confidence interval (constant-width normal approximation).
    //
    // The band width tracks `config.ci`: it used to be hardcoded at the 95%
    // critical value, so `ci(Some(80.0))` silently drew a 95% band.
    let (ci_lower, ci_upper) = if let Some(level) = config.ci {
        // Through-origin fits estimate one fewer parameter.
        let n_params = if config.fit_through_origin {
            config.order
        } else {
            coefficients.len()
        };
        let dof = n.saturating_sub(n_params).max(1) as f64;
        let se = (ss_res / dof).sqrt();
        let z = normal_two_sided_critical_value(level);

        let lower: Vec<f64> = line_y.iter().map(|&y| y - z * se).collect();
        let upper: Vec<f64> = line_y.iter().map(|&y| y + z * se).collect();
        (Some(lower), Some(upper))
    } else {
        (None, None)
    };

    RegPlotData {
        scatter_x: x.to_vec(),
        scatter_y: y.to_vec(),
        line_x,
        line_y,
        ci_lower,
        ci_upper,
        coefficients,
        r_squared,
    }
}

/// Evaluate polynomial at x
fn evaluate_polynomial(coeffs: &[f64], x: f64) -> f64 {
    coeffs
        .iter()
        .enumerate()
        .map(|(i, &c)| c * x.powi(i as i32))
        .sum()
}

/// Least-squares fit of `c1*x + c2*x^2 + ... + cd*x^d` — no constant term.
///
/// Backs [`RegPlotConfig::fit_through_origin`]. Returns coefficients in the
/// same ascending-power layout as the unconstrained fits, with `c0 == 0.0`, so
/// [`evaluate_polynomial`] works unchanged.
///
/// `degree == 1` reduces to the closed form `b = Σxy / Σx²`; higher degrees go
/// through the normal equations for the reduced design matrix.
fn fit_through_origin(x: &[f64], y: &[f64], degree: usize, n: usize) -> Vec<f64> {
    let degree = degree.max(1);
    let mut coefficients = vec![0.0; degree + 1];

    if degree == 1 {
        let sum_xy: f64 = (0..n).map(|i| x[i] * y[i]).sum();
        let sum_xx: f64 = (0..n).map(|i| x[i] * x[i]).sum();
        coefficients[1] = if sum_xx > 0.0 { sum_xy / sum_xx } else { 0.0 };
        return coefficients;
    }

    // Normal equations for the design matrix whose columns are x^1..x^degree.
    let mut xtx = vec![vec![0.0; degree]; degree];
    let mut xty = vec![0.0; degree];

    for k in 0..n {
        let mut powers = vec![0.0; degree];
        let mut power = 1.0;
        for slot in powers.iter_mut() {
            power *= x[k];
            *slot = power;
        }
        for i in 0..degree {
            xty[i] += powers[i] * y[k];
            for j in 0..degree {
                xtx[i][j] += powers[i] * powers[j];
            }
        }
    }

    for (index, value) in gauss_solve(&mut xtx, &mut xty).into_iter().enumerate() {
        coefficients[index + 1] = value;
    }

    coefficients
}

/// Gaussian elimination with partial pivoting.
///
/// A singular pivot leaves that unknown at zero rather than producing an
/// infinity, so a degenerate fit collapses to a flatter curve instead of a NaN
/// plot.
#[allow(clippy::needless_range_loop)]
fn gauss_solve(a: &mut [Vec<f64>], b: &mut [f64]) -> Vec<f64> {
    let n = b.len();

    for i in 0..n {
        let mut max_row = i;
        for k in (i + 1)..n {
            if a[k][i].abs() > a[max_row][i].abs() {
                max_row = k;
            }
        }
        a.swap(i, max_row);
        b.swap(i, max_row);

        if a[i][i].abs() < 1e-12 {
            continue;
        }

        for k in (i + 1)..n {
            let factor = a[k][i] / a[i][i];
            for j in i..n {
                a[k][j] -= factor * a[i][j];
            }
            b[k] -= factor * b[i];
        }
    }

    let mut solution = vec![0.0; n];
    for i in (0..n).rev() {
        if a[i][i].abs() < 1e-12 {
            continue;
        }
        solution[i] = b[i];
        for j in (i + 1)..n {
            solution[i] -= a[i][j] * solution[j];
        }
        solution[i] /= a[i][i];
    }

    solution
}

/// Two-sided standard-normal critical value for a confidence `level` in percent.
///
/// `95.0` returns ≈ 1.9600, `99.0` ≈ 2.5758, `68.0` ≈ 0.9945.
///
/// Uses the Beasley-Springer-Moro / Acklam rational approximation to the
/// inverse normal CDF (absolute error < 1.15e-9 over the open unit interval),
/// which is far more accurate than the band ever needs and avoids a dependency.
fn normal_two_sided_critical_value(level: f64) -> f64 {
    let level = level.clamp(0.0, 99.999_9);
    // Two-sided: the upper tail holds (100 - level)/2 of the mass.
    let p = 1.0 - (100.0 - level) / 200.0;
    inverse_standard_normal_cdf(p)
}

/// Acklam's rational approximation to the inverse standard normal CDF.
fn inverse_standard_normal_cdf(p: f64) -> f64 {
    const A: [f64; 6] = [
        -3.969683028665376e+01,
        2.209460984245205e+02,
        -2.759285104469687e+02,
        1.383_577_518_672_69e2,
        -3.066479806614716e+01,
        2.506628277459239e+00,
    ];
    const B: [f64; 5] = [
        -5.447609879822406e+01,
        1.615858368580409e+02,
        -1.556989798598866e+02,
        6.680131188771972e+01,
        -1.328068155288572e+01,
    ];
    const C: [f64; 6] = [
        -7.784894002430293e-03,
        -3.223964580411365e-01,
        -2.400758277161838e+00,
        -2.549732539343734e+00,
        4.374664141464968e+00,
        2.938163982698783e+00,
    ];
    const D: [f64; 4] = [
        7.784695709041462e-03,
        3.224671290700398e-01,
        2.445134137142996e+00,
        3.754408661907416e+00,
    ];
    const P_LOW: f64 = 0.02425;

    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }

    if p < P_LOW {
        let q = (-2.0 * p.ln()).sqrt();
        return (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0);
    }
    if p > 1.0 - P_LOW {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        return -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0);
    }

    let q = p - 0.5;
    let r = q * q;
    (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
        / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
}

/// Configuration for residual plot
#[derive(Debug, Clone)]
pub struct ResidPlotConfig {
    /// Point color
    pub color: Option<Color>,
    /// Point size
    pub size: f32,
    /// Point alpha
    pub alpha: f32,
    /// Show horizontal line at y=0
    pub show_baseline: bool,
    /// Regression order for computing residuals
    pub order: usize,
    /// Lowess smoothing
    pub lowess: bool,
}

impl Default for ResidPlotConfig {
    fn default() -> Self {
        Self {
            color: None,
            size: 5.0,
            alpha: 0.7,
            show_baseline: true,
            order: 1,
            lowess: false,
        }
    }
}

impl ResidPlotConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set point color
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set regression order
    pub fn order(mut self, order: usize) -> Self {
        self.order = order.max(1);
        self
    }

    /// Enable lowess smoothing
    pub fn lowess(mut self, enable: bool) -> Self {
        self.lowess = enable;
        self
    }
}

/// Computed residual plot data
#[derive(Debug, Clone)]
pub struct ResidPlotData {
    /// X positions (fitted values or original x)
    pub x: Vec<f64>,
    /// Residual values
    pub residuals: Vec<f64>,
    /// Baseline for reference
    pub baseline: f64,
}

/// Compute residual plot data
pub fn compute_residplot(x: &[f64], y: &[f64], config: &ResidPlotConfig) -> ResidPlotData {
    let n = x.len().min(y.len());
    if n < 2 {
        return ResidPlotData {
            x: x.to_vec(),
            residuals: vec![0.0; n],
            baseline: 0.0,
        };
    }

    // Fit regression
    let reg_result = if config.order == 1 {
        linear_regression(x, y)
    } else {
        polynomial_regression(x, y, config.order)
    };
    let coefficients = reg_result.coefficients.clone();

    // Compute fitted values and residuals
    let fitted: Vec<f64> = x
        .iter()
        .map(|&xi| evaluate_polynomial(&coefficients, xi))
        .collect();

    let residuals: Vec<f64> = y
        .iter()
        .zip(fitted.iter())
        .map(|(&yi, &fi)| yi - fi)
        .collect();

    ResidPlotData {
        x: fitted,
        residuals,
        baseline: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regplot_linear() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 5.0, 4.0, 5.0];
        let config = RegPlotConfig::default();
        let data = compute_regplot(&x, &y, &config);

        assert_eq!(data.scatter_x.len(), 5);
        assert!(!data.line_x.is_empty());
        assert!(data.r_squared >= 0.0 && data.r_squared <= 1.0);
    }

    #[test]
    fn test_regplot_polynomial() {
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|&xi| xi * xi).collect();
        let config = RegPlotConfig::default().order(2);
        let data = compute_regplot(&x, &y, &config);

        // Quadratic fit should be nearly perfect
        assert!(data.r_squared > 0.99);
    }

    #[test]
    fn test_regplot_with_ci() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 5.0, 4.0, 5.0];
        let config = RegPlotConfig::default().ci(Some(95.0));
        let data = compute_regplot(&x, &y, &config);

        assert!(data.ci_lower.is_some());
        assert!(data.ci_upper.is_some());
    }

    // ------------------------------------------------------------------
    // fit_through_origin (plan item 2.4)
    // ------------------------------------------------------------------

    #[test]
    fn test_through_origin_forces_a_zero_intercept() {
        // y = 2x + 10: the unconstrained fit has a large intercept.
        let x: Vec<f64> = (1..=10).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|xi| 2.0 * xi + 10.0).collect();

        let free = compute_regplot(&x, &y, &RegPlotConfig::default());
        let forced = compute_regplot(&x, &y, &RegPlotConfig::default().through_origin(true));

        assert!(
            (free.coefficients[0] - 10.0).abs() < 1e-9,
            "unconstrained intercept: {}",
            free.coefficients[0]
        );
        assert_eq!(forced.coefficients[0], 0.0);
        assert!(
            (free.coefficients[1] - forced.coefficients[1]).abs() > 1e-6,
            "through-origin produced the same slope as the free fit"
        );
    }

    #[test]
    fn test_through_origin_recovers_an_exact_proportional_fit() {
        let x: Vec<f64> = (1..=20).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|xi| 3.5 * xi).collect();

        let data = compute_regplot(&x, &y, &RegPlotConfig::default().through_origin(true));

        assert_eq!(data.coefficients[0], 0.0);
        assert!((data.coefficients[1] - 3.5).abs() < 1e-9);
        assert!(data.r_squared > 0.999_999);
    }

    #[test]
    fn test_through_origin_matches_the_closed_form_least_squares_slope() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.2, 3.8, 6.1, 8.4, 9.7];
        let expected = x.iter().zip(&y).map(|(a, b)| a * b).sum::<f64>()
            / x.iter().map(|a| a * a).sum::<f64>();

        let data = compute_regplot(&x, &y, &RegPlotConfig::default().through_origin(true));
        assert!((data.coefficients[1] - expected).abs() < 1e-12);
    }

    #[test]
    fn test_through_origin_works_for_higher_orders() {
        // y = 4x^2 + x, exactly representable without a constant term.
        let x: Vec<f64> = (1..=15).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|xi| 4.0 * xi * xi + xi).collect();

        let data = compute_regplot(
            &x,
            &y,
            &RegPlotConfig::default().order(2).through_origin(true),
        );

        assert_eq!(data.coefficients.len(), 3);
        assert_eq!(data.coefficients[0], 0.0);
        assert!((data.coefficients[1] - 1.0).abs() < 1e-6);
        assert!((data.coefficients[2] - 4.0).abs() < 1e-6);
        assert!(data.r_squared > 0.999_999);
    }

    #[test]
    fn test_through_origin_line_passes_through_zero() {
        let x: Vec<f64> = (1..=10).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|xi| 2.0 * xi + 10.0).collect();
        let data = compute_regplot(&x, &y, &RegPlotConfig::default().through_origin(true));

        assert!(evaluate_polynomial(&data.coefficients, 0.0).abs() < 1e-12);
    }

    // ------------------------------------------------------------------
    // Confidence level actually reaching the band
    // ------------------------------------------------------------------

    #[test]
    fn test_confidence_level_changes_the_band_width() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 5.0, 4.0, 5.0];

        let width = |level: f64| {
            let data = compute_regplot(&x, &y, &RegPlotConfig::default().ci(Some(level)));
            data.ci_upper.unwrap()[0] - data.ci_lower.unwrap()[0]
        };

        let narrow = width(50.0);
        let standard = width(95.0);
        let wide = width(99.0);

        assert!(
            narrow < standard && standard < wide,
            "50%={narrow} 95%={standard} 99%={wide}"
        );
    }

    #[test]
    fn test_normal_critical_values_match_the_published_table() {
        assert!((normal_two_sided_critical_value(95.0) - 1.959_964).abs() < 1e-5);
        assert!((normal_two_sided_critical_value(99.0) - 2.575_829).abs() < 1e-5);
        assert!((normal_two_sided_critical_value(90.0) - 1.644_854).abs() < 1e-5);
        assert!((normal_two_sided_critical_value(50.0) - 0.674_490).abs() < 1e-5);
    }

    #[test]
    fn test_residplot() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0]; // Perfect linear
        let config = ResidPlotConfig::default();
        let data = compute_residplot(&x, &y, &config);

        assert_eq!(data.residuals.len(), 5);
        // Residuals should be very small for perfect linear fit
        for r in &data.residuals {
            assert!(r.abs() < 1e-10);
        }
    }
}
