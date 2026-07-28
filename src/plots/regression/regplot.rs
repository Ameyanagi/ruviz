//! Regression plot implementations
//!
//! Provides linear regression, polynomial regression, and residual plots.
//!
//! # The confidence band
//!
//! [`compute_regplot`] returns a **confidence interval for the mean response**:
//!
//! ```text
//! ŷ(x₀) ± t(level, n − p) · σ̂ · √( x₀ᵀ (XᵀX)⁻¹ x₀ )
//! ```
//!
//! It is the interval that covers the true regression curve, not one that
//! covers future observations. Consequently it is narrowest near the centre of
//! mass of `x`, widens towards the ends of the data, and shrinks like `1/√n`.
//!
//! Before 2026-07-27 this function returned `ŷ ± z · σ̂` — a band of constant
//! width, with no leverage term and a normal rather than Student-t quantile.
//! That is a different interval entirely (roughly a *prediction* interval for
//! large `n`), and measured against the textbook formula it was 9.9× too wide
//! at the centre of a 100-point fit while being a third too *narrow* at `n = 5`,
//! where the two errors run in opposite directions. Nothing drew it, which is
//! the only reason it went unnoticed. Do not reintroduce a constant-width band.

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
    /// Confidence level in percent for the band around the fitted curve
    ///
    /// `Some(95.0)` is a 95% confidence interval for the mean response; see the
    /// module documentation for the formula and for what it is *not*. `None`
    /// disables the band.
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
    /// Confidence band for the mean response, aligned with `line_x`/`line_y`
    ///
    /// `None` when [`RegPlotConfig::ci`] is `None`, and also when the fit has no
    /// residual degrees of freedom (`n <= p`), where `σ̂` is undefined and no
    /// interval exists. See the module documentation.
    pub ci_lower: Option<Vec<f64>>,
    /// Upper edge of the band described by [`RegPlotData::ci_lower`]
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

    // Only the paired prefix is fitted; a longer `x` or `y` contributes nothing
    // to the fit, so it must not contribute to the mean, the residuals or the
    // design matrix either.
    let x_fit = &x[..n];
    let y_fit = &y[..n];

    // Fit regression
    let coefficients = if config.fit_through_origin {
        fit_through_origin(x_fit, y_fit, config.order, n)
    } else if config.order == 1 {
        linear_regression(x_fit, y_fit).coefficients
    } else {
        polynomial_regression(x_fit, y_fit, config.order).coefficients
    };

    // Generate line points. `n_points` is a public field, so 0 and 1 both reach
    // here; two is the fewest that describes a segment, and below that
    // `n_points - 1` underflows.
    let n_points = config.n_points.max(2);
    let x_min = x_fit.iter().copied().fold(f64::INFINITY, f64::min);
    let x_max = x_fit.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let x_step = (x_max - x_min) / (n_points - 1) as f64;

    let line_x: Vec<f64> = (0..n_points).map(|i| x_min + i as f64 * x_step).collect();

    let line_y: Vec<f64> = line_x
        .iter()
        .map(|&xi| evaluate_polynomial(&coefficients, xi))
        .collect();

    // Compute R-squared
    let y_mean = y_fit.iter().sum::<f64>() / n as f64;
    let ss_tot: f64 = y_fit.iter().map(|&yi| (yi - y_mean).powi(2)).sum();
    let ss_res: f64 = x_fit
        .iter()
        .zip(y_fit.iter())
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

    // Confidence band for the mean response:
    //
    //     ŷ(x₀) ± t(level, n − p) · σ̂ · √( x₀ᵀ (XᵀX)⁻¹ x₀ )
    //
    // The leverage factor under the square root is the whole point. Drop it and
    // the band becomes a constant vertical offset — a different interval, ~10×
    // too wide at the centre of a 100-point fit — which is what this function
    // used to return. See the module docs.
    let n_params = design_len(coefficients.len(), config.fit_through_origin);
    let (ci_lower, ci_upper) = match config.ci {
        // With no residual degrees of freedom the fit is exact, σ̂ is undefined,
        // and no interval exists. Saying so beats inventing one.
        Some(level) if n_params > 0 && n > n_params => {
            let dof = (n - n_params) as f64;
            let sigma = (ss_res / dof).sqrt();
            let t = student_t_two_sided_critical_value(level, dof);
            let gram = design_gram(x_fit, coefficients.len(), config.fit_through_origin);

            let half_widths: Vec<f64> = line_x
                .iter()
                .map(|&x0| {
                    let row = design_row(x0, coefficients.len(), config.fit_through_origin);
                    t * sigma * leverage(&gram, &row).sqrt()
                })
                .collect();

            let lower: Vec<f64> = line_y
                .iter()
                .zip(&half_widths)
                .map(|(&fitted, &half)| fitted - half)
                .collect();
            let upper: Vec<f64> = line_y
                .iter()
                .zip(&half_widths)
                .map(|(&fitted, &half)| fitted + half)
                .collect();
            (Some(lower), Some(upper))
        }
        _ => (None, None),
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

// ---------------------------------------------------------------------------
// Design matrix
//
// One description of the fitted model, shared by the fit and the band, so the
// two cannot disagree about how many parameters were estimated or which columns
// exist. `coefficient_count` is always `coefficients.len()` — read from the fit
// rather than from `config.order`, because `polynomial_regression` lowers the
// degree when the sample is too small to support it.
// ---------------------------------------------------------------------------

/// Number of parameters actually estimated.
///
/// A through-origin fit has no constant column, so it estimates one fewer than
/// it has coefficients (`c0` is pinned at zero, not fitted).
fn design_len(coefficient_count: usize, through_origin: bool) -> usize {
    if through_origin {
        coefficient_count.saturating_sub(1)
    } else {
        coefficient_count
    }
}

/// One row of the design matrix: `[1, x, x², …]`, or `[x, x², …]` through the origin.
fn design_row(x: f64, coefficient_count: usize, through_origin: bool) -> Vec<f64> {
    let first_power = usize::from(through_origin);
    (first_power..coefficient_count)
        .map(|power| x.powi(power as i32))
        .collect()
}

/// `XᵀX` for the design that [`design_row`] describes.
fn design_gram(x: &[f64], coefficient_count: usize, through_origin: bool) -> Vec<Vec<f64>> {
    let size = design_len(coefficient_count, through_origin);
    let mut gram = vec![vec![0.0; size]; size];

    for &xi in x {
        let row = design_row(xi, coefficient_count, through_origin);
        for (i, gram_row) in gram.iter_mut().enumerate() {
            for (cell, &rj) in gram_row.iter_mut().zip(row.iter()) {
                *cell += row[i] * rj;
            }
        }
    }

    gram
}

/// Leverage `rᵀ (XᵀX)⁻¹ r` — the variance of the fitted value at that row, in
/// units of `σ²`.
///
/// Solves `XᵀX v = r` instead of forming the inverse: the matrix is `p × p`
/// with `p ≤ 5` in practice, and a rank-deficient design leaves the affected
/// component at zero (see [`gauss_solve`]) rather than producing an infinity.
///
/// For a simple linear fit this reduces to the familiar
/// `1/n + (x₀ − x̄)² / Σ(xᵢ − x̄)²`, which
/// `test_leverage_matches_the_closed_form_for_a_linear_fit` pins.
fn leverage(gram: &[Vec<f64>], row: &[f64]) -> f64 {
    let mut matrix = gram.to_vec();
    let mut rhs = row.to_vec();
    let solution = gauss_solve(&mut matrix, &mut rhs);

    row.iter()
        .zip(&solution)
        .map(|(&r, &v)| r * v)
        .sum::<f64>()
        .max(0.0)
}

// ---------------------------------------------------------------------------
// Student-t quantile
//
// One critical-value function, not two: the band always wants `t`, and `z` is
// simply `t` at infinite degrees of freedom (this function returns 1.959964 for
// a 95% level at dof = 1e7). At n = 5 the difference is not academic — z gives
// 1.96 where the correct value is 3.18.
// ---------------------------------------------------------------------------

/// Two-sided Student-t critical value: the `t` with `P(|T| ≤ t) == level/100`.
///
/// Returns ≈ 12.7062 at `(95.0, 1)`, 2.2281 at `(95.0, 10)` and 2.7500 at
/// `(99.0, 30)`, matching published tables to their last printed digit.
fn student_t_two_sided_critical_value(level: f64, dof: f64) -> f64 {
    let level = level.clamp(0.0, 99.999_9);
    let upper_tail = (100.0 - level) / 200.0;
    if upper_tail >= 0.5 || dof <= 0.0 {
        return 0.0;
    }

    // The upper tail is strictly decreasing in `t`, so bracket then bisect.
    let mut low = 0.0_f64;
    let mut high = 1.0_f64;
    while high < 1e12 && student_t_upper_tail(high, dof) > upper_tail {
        high *= 2.0;
    }
    // 200 halvings take any bracket that survives the loop below f64 resolution.
    for _ in 0..200 {
        let mid = 0.5 * (low + high);
        if student_t_upper_tail(mid, dof) > upper_tail {
            low = mid;
        } else {
            high = mid;
        }
    }

    0.5 * (low + high)
}

/// `P(T > t)` for `t ≥ 0` with `dof` degrees of freedom.
fn student_t_upper_tail(t: f64, dof: f64) -> f64 {
    if t <= 0.0 {
        return 0.5;
    }
    0.5 * regularized_incomplete_beta(dof / (dof + t * t), 0.5 * dof, 0.5)
}

/// Regularized incomplete beta function `I_x(a, b)`.
fn regularized_incomplete_beta(x: f64, a: f64, b: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }

    let ln_front = ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (1.0 - x).ln();
    let front = ln_front.exp();

    // The continued fraction converges quickly only on its own side of the
    // mode; past it, use the symmetry `I_x(a, b) = 1 − I_{1−x}(b, a)`.
    if x < (a + 1.0) / (a + b + 2.0) {
        front * beta_continued_fraction(a, b, x) / a
    } else {
        1.0 - front * beta_continued_fraction(b, a, 1.0 - x) / b
    }
}

/// Continued-fraction expansion for [`regularized_incomplete_beta`], evaluated
/// by the modified Lentz method.
fn beta_continued_fraction(a: f64, b: f64, x: f64) -> f64 {
    const MAX_ITERATIONS: usize = 300;
    const EPSILON: f64 = 3.0e-16;
    const TINY: f64 = 1.0e-300;

    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;

    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < TINY {
        d = TINY;
    }
    d = 1.0 / d;
    let mut fraction = d;

    for iteration in 1..=MAX_ITERATIONS {
        let m = iteration as f64;
        let m2 = 2.0 * m;

        // Even step.
        let numerator = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + numerator * d;
        if d.abs() < TINY {
            d = TINY;
        }
        c = 1.0 + numerator / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        fraction *= d * c;

        // Odd step.
        let numerator = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + numerator * d;
        if d.abs() < TINY {
            d = TINY;
        }
        c = 1.0 + numerator / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        let delta = d * c;
        fraction *= delta;

        if (delta - 1.0).abs() < EPSILON {
            break;
        }
    }

    fraction
}

/// Natural log of the gamma function, by the Lanczos approximation at `g = 7`.
///
/// Agrees with a reference `lgamma` to ~4e-15 relative over the arguments this
/// module uses (all ≥ 0.5). Written out here rather than pulled in as a
/// dependency: it is fifteen lines and the only special function ruviz needs.
fn ln_gamma(x: f64) -> f64 {
    const COEFFICIENTS: [f64; 9] = [
        0.9999999999998099,
        676.5203681218851,
        -1259.1392167224028,
        771.3234287776531,
        -176.6150291621406,
        12.507343278686905,
        -0.13857109526572012,
        9.984369578019572e-6,
        1.5056327351493116e-7,
    ];
    const G: f64 = 7.0;

    let mut series = COEFFICIENTS[0];
    for (index, coefficient) in COEFFICIENTS.iter().enumerate().skip(1) {
        series += coefficient / (x - 1.0 + index as f64);
    }

    let t = x - 1.0 + G + 0.5;
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x - 0.5) * t.ln() - t + series.ln()
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
    // Confidence band: it is a CI for the mean response, and it is correct
    // ------------------------------------------------------------------

    /// A deterministic, mildly noisy `y = 2x + 3` sample.
    fn noisy_linear(n: usize) -> (Vec<f64>, Vec<f64>) {
        let x: Vec<f64> = (1..=n).map(|i| i as f64).collect();
        let y: Vec<f64> = x
            .iter()
            .map(|&xi| 2.0 * xi + 3.0 + (xi * 1.7).sin() * 2.0)
            .collect();
        (x, y)
    }

    fn band_half_width(data: &RegPlotData, index: usize) -> f64 {
        0.5 * (data.ci_upper.as_ref().unwrap()[index] - data.ci_lower.as_ref().unwrap()[index])
    }

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
    fn test_student_t_critical_values_match_the_published_table() {
        // Two-sided t table, verified independently by numerically integrating
        // the t density between ±t and confirming the mass is exactly the level.
        let cases = [
            (95.0, 1.0, 12.706_205),
            (95.0, 2.0, 4.302_653),
            (95.0, 3.0, 3.182_446),
            (95.0, 5.0, 2.570_582),
            (95.0, 10.0, 2.228_139),
            (95.0, 30.0, 2.042_272),
            (95.0, 100.0, 1.983_972),
            (99.0, 3.0, 5.840_909),
            (99.0, 10.0, 3.169_273),
            (99.0, 30.0, 2.749_996),
            (90.0, 10.0, 1.812_461),
            (90.0, 30.0, 1.697_261),
        ];

        for (level, dof, expected) in cases {
            let got = student_t_two_sided_critical_value(level, dof);
            assert!(
                (got - expected).abs() < 1e-5,
                "t({level}%, dof={dof}) = {got}, expected {expected}"
            );
        }
    }

    #[test]
    fn test_student_t_approaches_the_normal_value_at_large_dof() {
        let z = 1.959_963_985;
        let far = student_t_two_sided_critical_value(95.0, 1e7);
        assert!((far - z).abs() < 1e-5, "t(95%, 1e7) = {far}");

        // ...and is meaningfully larger at the sample sizes a plot actually has.
        let small = student_t_two_sided_critical_value(95.0, 3.0);
        assert!(small > z * 1.5, "t(95%, dof=3) = {small}");
    }

    #[test]
    fn test_band_is_narrowest_at_the_centre_of_mass_and_widens_outward() {
        // This is the defect the band used to have: it was a constant offset,
        // so this test would have failed with every half-width identical.
        let (x, y) = noisy_linear(40);
        let data = compute_regplot(&x, &y, &RegPlotConfig::default());

        let last = data.line_x.len() - 1;
        let middle = last / 2;
        let centre = band_half_width(&data, middle);
        let left = band_half_width(&data, 0);
        let right = band_half_width(&data, last);

        assert!(
            centre < left,
            "centre {centre} not narrower than left {left}"
        );
        assert!(
            centre < right,
            "centre {centre} not narrower than right {right}"
        );
        assert!(
            (left - right).abs() < 1e-9,
            "an evenly spaced sample must give a symmetric band: {left} vs {right}"
        );
    }

    #[test]
    fn test_band_shrinks_as_the_sample_grows() {
        // A constant-width band does not do this: σ̂ converges to the noise
        // level and the band stops narrowing. The leverage term is what makes
        // the interval a statement about the *fitted curve*.
        let width_at_centre = |n: usize| {
            let (x, y) = noisy_linear(n);
            let data = compute_regplot(&x, &y, &RegPlotConfig::default());
            band_half_width(&data, data.line_x.len() / 2)
        };

        let small = width_at_centre(20);
        let large = width_at_centre(200);
        assert!(
            large < small / 2.0,
            "10x the sample should more than halve the band: {small} -> {large}"
        );
    }

    #[test]
    fn test_band_matches_the_textbook_formula_for_a_simple_linear_fit() {
        let (x, y) = noisy_linear(25);
        let n = x.len();
        let data = compute_regplot(&x, &y, &RegPlotConfig::default());

        let x_mean = x.iter().sum::<f64>() / n as f64;
        let s_xx: f64 = x.iter().map(|xi| (xi - x_mean).powi(2)).sum();
        let ss_res: f64 = x
            .iter()
            .zip(&y)
            .map(|(&xi, &yi)| (yi - evaluate_polynomial(&data.coefficients, xi)).powi(2))
            .sum();
        let sigma = (ss_res / (n - 2) as f64).sqrt();
        let t = student_t_two_sided_critical_value(95.0, (n - 2) as f64);

        for (index, &x0) in data.line_x.iter().enumerate() {
            let expected = t * sigma * (1.0 / n as f64 + (x0 - x_mean).powi(2) / s_xx).sqrt();
            let got = band_half_width(&data, index);
            assert!(
                (got - expected).abs() < 1e-9,
                "at x0={x0}: {got} vs textbook {expected}"
            );
        }
    }

    #[test]
    fn test_leverage_matches_the_closed_form_for_a_linear_fit() {
        let x: Vec<f64> = (1..=12).map(|i| i as f64).collect();
        let n = x.len() as f64;
        let x_mean = x.iter().sum::<f64>() / n;
        let s_xx: f64 = x.iter().map(|xi| (xi - x_mean).powi(2)).sum();
        let gram = design_gram(&x, 2, false);

        for x0 in [1.0, 4.0, x_mean, 9.0, 12.0] {
            let got = leverage(&gram, &design_row(x0, 2, false));
            let expected = 1.0 / n + (x0 - x_mean).powi(2) / s_xx;
            assert!(
                (got - expected).abs() < 1e-12,
                "x0={x0}: {got} vs {expected}"
            );
        }
    }

    #[test]
    fn test_leverages_sum_to_the_parameter_count() {
        // trace(H) == p for any OLS design; a strong check that `design_gram`
        // and `leverage` agree about the model.
        let x: Vec<f64> = (1..=12).map(|i| i as f64).collect();
        for coefficient_count in [2, 3, 4] {
            let gram = design_gram(&x, coefficient_count, false);
            let total: f64 = x
                .iter()
                .map(|&xi| leverage(&gram, &design_row(xi, coefficient_count, false)))
                .sum();
            assert!(
                (total - coefficient_count as f64).abs() < 1e-9,
                "p={coefficient_count}: trace(H) = {total}"
            );
        }
    }

    #[test]
    fn test_through_origin_band_pinches_to_zero_at_the_origin() {
        // No constant column means no `1/n` floor, so the fitted value at x = 0
        // is known exactly: the band must close there.
        let x: Vec<f64> = (1..=15).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|xi| 2.0 * xi + (xi * 0.9).sin()).collect();
        let gram = design_gram(&x, 2, true);

        assert!(leverage(&gram, &design_row(0.0, 2, true)).abs() < 1e-12);
        assert!(leverage(&gram, &design_row(15.0, 2, true)) > 0.0);

        let data = compute_regplot(&x, &y, &RegPlotConfig::default().through_origin(true));
        assert!(data.ci_lower.is_some());
    }

    #[test]
    fn test_no_band_without_residual_degrees_of_freedom() {
        // Two points and two parameters: the fit is exact and σ̂ is undefined.
        // The old code substituted `dof.max(1)` and returned a zero-width band
        // as though it had measured something.
        let data = compute_regplot(&[1.0, 2.0], &[3.0, 5.0], &RegPlotConfig::default());
        assert!(data.ci_lower.is_none());
        assert!(data.ci_upper.is_none());
    }

    #[test]
    fn test_band_is_absent_when_disabled() {
        let (x, y) = noisy_linear(10);
        let data = compute_regplot(&x, &y, &RegPlotConfig::default().ci(None));
        assert!(data.ci_lower.is_none());
        assert!(data.ci_upper.is_none());
    }

    #[test]
    fn test_band_brackets_the_fitted_line() {
        let (x, y) = noisy_linear(30);
        let data = compute_regplot(&x, &y, &RegPlotConfig::default());
        let lower = data.ci_lower.as_ref().unwrap();
        let upper = data.ci_upper.as_ref().unwrap();

        assert_eq!(lower.len(), data.line_y.len());
        assert_eq!(upper.len(), data.line_y.len());
        for (index, &fitted) in data.line_y.iter().enumerate() {
            assert!(lower[index] < fitted && fitted < upper[index]);
            assert!(lower[index].is_finite() && upper[index].is_finite());
        }
    }

    // ------------------------------------------------------------------
    // Public fields that reach this function without going through a setter
    // ------------------------------------------------------------------

    #[test]
    fn test_degenerate_n_points_does_not_underflow() {
        // `n_points` is a plain public field, so it is not clamped on the way
        // in. `0` used to panic on `config.n_points - 1`.
        let (x, y) = noisy_linear(8);
        for n_points in [0, 1, 2] {
            let config = RegPlotConfig {
                n_points,
                ..Default::default()
            };
            let data = compute_regplot(&x, &y, &config);
            assert_eq!(data.line_x.len(), n_points.max(2));
            assert!(data.line_x.iter().all(|value| value.is_finite()));
        }
    }

    #[test]
    fn test_trailing_unpaired_samples_do_not_bias_the_fit() {
        // `n` is the paired prefix; the extra `y` used to be summed into the
        // mean while being divided by `n`.
        let x = vec![1.0, 2.0, 3.0, 4.0];
        let y = vec![2.0, 4.0, 6.0, 8.0];
        let mut y_long = y.clone();
        y_long.extend([1000.0, -1000.0]);

        let paired = compute_regplot(&x, &y, &RegPlotConfig::default());
        let padded = compute_regplot(&x, &y_long, &RegPlotConfig::default());

        assert!((paired.r_squared - padded.r_squared).abs() < 1e-12);
        assert_eq!(paired.coefficients, padded.coefficients);
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
