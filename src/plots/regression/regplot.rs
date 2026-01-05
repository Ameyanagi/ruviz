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
    /// Fit through origin
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

    /// Fit through origin
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
    let reg_result = if config.order == 1 {
        linear_regression(x, y)
    } else {
        polynomial_regression(x, y, config.order)
    };
    let coefficients = reg_result.coefficients.clone();

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

    // Compute confidence interval (simplified approximation)
    let (ci_lower, ci_upper) = if config.ci.is_some() {
        // Standard error approximation
        let mse = ss_res / (n - coefficients.len()) as f64;
        let se = mse.sqrt();
        let t_crit = 1.96; // Approximate for 95% CI

        let lower: Vec<f64> = line_y.iter().map(|&y| y - t_crit * se).collect();
        let upper: Vec<f64> = line_y.iter().map(|&y| y + t_crit * se).collect();
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
