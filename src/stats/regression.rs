//! Regression analysis
//!
//! Provides linear and polynomial regression for regplot and residplot.

/// Result of regression analysis
#[derive(Debug, Clone)]
pub struct RegressionResult {
    /// Coefficients (intercept, slope for linear; c0, c1, c2... for polynomial)
    pub coefficients: Vec<f64>,
    /// R-squared value
    pub r_squared: f64,
    /// Standard error of the regression
    pub std_error: f64,
    /// Predicted values for input x
    pub predictions: Vec<f64>,
    /// Residuals (y - predicted)
    pub residuals: Vec<f64>,
}

/// Perform linear regression (y = a + b*x)
///
/// # Arguments
/// * `x` - Independent variable values
/// * `y` - Dependent variable values
///
/// # Returns
/// RegressionResult with coefficients [intercept, slope]
pub fn linear_regression(x: &[f64], y: &[f64]) -> RegressionResult {
    let n = x.len().min(y.len());

    if n < 2 {
        return RegressionResult {
            coefficients: vec![0.0, 0.0],
            r_squared: 0.0,
            std_error: 0.0,
            predictions: vec![],
            residuals: vec![],
        };
    }

    let n_f = n as f64;

    // Calculate means
    let x_mean = x.iter().take(n).sum::<f64>() / n_f;
    let y_mean = y.iter().take(n).sum::<f64>() / n_f;

    // Calculate slope and intercept
    let mut ss_xy = 0.0;
    let mut ss_xx = 0.0;
    let mut ss_yy = 0.0;

    for i in 0..n {
        let dx = x[i] - x_mean;
        let dy = y[i] - y_mean;
        ss_xy += dx * dy;
        ss_xx += dx * dx;
        ss_yy += dy * dy;
    }

    let slope = if ss_xx > 0.0 { ss_xy / ss_xx } else { 0.0 };
    let intercept = y_mean - slope * x_mean;

    // Calculate predictions and residuals
    let predictions: Vec<f64> = x.iter().take(n).map(|&xi| intercept + slope * xi).collect();
    let residuals: Vec<f64> = (0..n).map(|i| y[i] - predictions[i]).collect();

    // Calculate R-squared
    let ss_res: f64 = residuals.iter().map(|&r| r * r).sum();
    let r_squared = if ss_yy > 0.0 {
        1.0 - ss_res / ss_yy
    } else {
        0.0
    };

    // Calculate standard error
    let std_error = if n > 2 {
        (ss_res / (n - 2) as f64).sqrt()
    } else {
        0.0
    };

    RegressionResult {
        coefficients: vec![intercept, slope],
        r_squared,
        std_error,
        predictions,
        residuals,
    }
}

/// Perform polynomial regression (y = c0 + c1*x + c2*x^2 + ...)
///
/// # Arguments
/// * `x` - Independent variable values
/// * `y` - Dependent variable values
/// * `degree` - Polynomial degree (1 = linear, 2 = quadratic, etc.)
///
/// # Returns
/// RegressionResult with coefficients [c0, c1, c2, ...]
pub fn polynomial_regression(x: &[f64], y: &[f64], degree: usize) -> RegressionResult {
    let n = x.len().min(y.len());
    let degree = degree.min(n.saturating_sub(1)).max(1);

    if n < 2 {
        return RegressionResult {
            coefficients: vec![0.0; degree + 1],
            r_squared: 0.0,
            std_error: 0.0,
            predictions: vec![],
            residuals: vec![],
        };
    }

    // For degree 1, use simpler linear regression
    if degree == 1 {
        return linear_regression(x, y);
    }

    // Build Vandermonde matrix and solve using normal equations
    // This is a simplified implementation - for production, use proper linear algebra
    let coefficients = solve_polynomial(x, y, degree, n);

    // Calculate predictions
    let predictions: Vec<f64> = x
        .iter()
        .take(n)
        .map(|&xi| eval_polynomial(&coefficients, xi))
        .collect();

    // Calculate residuals
    let residuals: Vec<f64> = (0..n).map(|i| y[i] - predictions[i]).collect();

    // Calculate R-squared
    let y_mean = y.iter().take(n).sum::<f64>() / n as f64;
    let ss_tot: f64 = y.iter().take(n).map(|&yi| (yi - y_mean).powi(2)).sum();
    let ss_res: f64 = residuals.iter().map(|&r| r * r).sum();
    let r_squared = if ss_tot > 0.0 {
        1.0 - ss_res / ss_tot
    } else {
        0.0
    };

    // Standard error
    let df = n.saturating_sub(degree + 1);
    let std_error = if df > 0 {
        (ss_res / df as f64).sqrt()
    } else {
        0.0
    };

    RegressionResult {
        coefficients,
        r_squared,
        std_error,
        predictions,
        residuals,
    }
}

/// Solve polynomial coefficients using normal equations
fn solve_polynomial(x: &[f64], y: &[f64], degree: usize, n: usize) -> Vec<f64> {
    let m = degree + 1;

    // Build X^T * X matrix
    let mut xtx = vec![vec![0.0; m]; m];
    let mut xty = vec![0.0; m];

    for k in 0..n {
        let mut xi_powers = vec![1.0; m];
        for j in 1..m {
            xi_powers[j] = xi_powers[j - 1] * x[k];
        }

        for i in 0..m {
            xty[i] += xi_powers[i] * y[k];
            for j in 0..m {
                xtx[i][j] += xi_powers[i] * xi_powers[j];
            }
        }
    }

    // Solve using Gaussian elimination with partial pivoting
    gauss_solve(&mut xtx, &mut xty)
}

/// Gaussian elimination with partial pivoting
#[allow(clippy::needless_range_loop)]
fn gauss_solve(a: &mut [Vec<f64>], b: &mut [f64]) -> Vec<f64> {
    let n = b.len();

    // Forward elimination
    for i in 0..n {
        // Find pivot
        let mut max_row = i;
        for k in (i + 1)..n {
            if a[k][i].abs() > a[max_row][i].abs() {
                max_row = k;
            }
        }

        // Swap rows
        a.swap(i, max_row);
        b.swap(i, max_row);

        // Eliminate
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

    // Back substitution
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        if a[i][i].abs() < 1e-12 {
            continue;
        }
        x[i] = b[i];
        for j in (i + 1)..n {
            x[i] -= a[i][j] * x[j];
        }
        x[i] /= a[i][i];
    }

    x
}

/// Evaluate polynomial at point x
fn eval_polynomial(coefficients: &[f64], x: f64) -> f64 {
    let mut result = 0.0;
    let mut x_power = 1.0;
    for &c in coefficients {
        result += c * x_power;
        x_power *= x;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_regression() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0]; // y = 2x

        let result = linear_regression(&x, &y);

        assert!((result.coefficients[0] - 0.0).abs() < 1e-10); // intercept ≈ 0
        assert!((result.coefficients[1] - 2.0).abs() < 1e-10); // slope ≈ 2
        assert!((result.r_squared - 1.0).abs() < 1e-10); // Perfect fit
    }

    #[test]
    fn test_linear_regression_with_intercept() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![3.0, 5.0, 7.0, 9.0, 11.0]; // y = 2x + 1

        let result = linear_regression(&x, &y);

        assert!((result.coefficients[0] - 1.0).abs() < 1e-10); // intercept ≈ 1
        assert!((result.coefficients[1] - 2.0).abs() < 1e-10); // slope ≈ 2
    }

    #[test]
    fn test_polynomial_regression_quadratic() {
        let x: Vec<f64> = (-5..=5).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|&xi| xi * xi).collect(); // y = x^2

        let result = polynomial_regression(&x, &y, 2);

        assert!((result.coefficients[0] - 0.0).abs() < 1e-6); // c0 ≈ 0
        assert!((result.coefficients[1] - 0.0).abs() < 1e-6); // c1 ≈ 0
        assert!((result.coefficients[2] - 1.0).abs() < 1e-6); // c2 ≈ 1
        assert!(result.r_squared > 0.999);
    }

    #[test]
    fn test_residuals() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.1, 3.9, 6.2, 7.8, 10.1]; // Noisy y ≈ 2x

        let result = linear_regression(&x, &y);

        // Sum of residuals should be close to 0
        let residual_sum: f64 = result.residuals.iter().sum();
        assert!(residual_sum.abs() < 1e-10);
    }
}
