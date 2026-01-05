//! Joint plot implementations
//!
//! Provides joint distribution plots (scatter with marginal distributions).

use crate::render::Color;

/// Configuration for joint plot
#[derive(Debug, Clone)]
pub struct JointPlotConfig {
    /// Type of central plot
    pub kind: JointKind,
    /// Show marginal histograms
    pub marginal_hist: bool,
    /// Show marginal KDE
    pub marginal_kde: bool,
    /// Show rugplot on margins
    pub rugplot: bool,
    /// Scatter point size
    pub scatter_size: f32,
    /// Scatter alpha
    pub scatter_alpha: f32,
    /// Scatter color
    pub color: Option<Color>,
    /// Number of histogram bins
    pub bins: usize,
    /// Ratio of marginal plot size to main plot
    pub marginal_ratio: f64,
}

/// Type of central plot in joint plot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointKind {
    /// Scatter plot
    Scatter,
    /// Regression plot
    Reg,
    /// Hexbin density
    Hex,
    /// KDE density
    Kde,
    /// Residual plot
    Resid,
}

impl Default for JointPlotConfig {
    fn default() -> Self {
        Self {
            kind: JointKind::Scatter,
            marginal_hist: true,
            marginal_kde: true,
            rugplot: false,
            scatter_size: 5.0,
            scatter_alpha: 0.7,
            color: None,
            bins: 30,
            marginal_ratio: 0.2,
        }
    }
}

impl JointPlotConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set joint plot kind
    pub fn kind(mut self, kind: JointKind) -> Self {
        self.kind = kind;
        self
    }

    /// Enable marginal histograms
    pub fn marginal_hist(mut self, show: bool) -> Self {
        self.marginal_hist = show;
        self
    }

    /// Enable marginal KDE
    pub fn marginal_kde(mut self, show: bool) -> Self {
        self.marginal_kde = show;
        self
    }

    /// Enable rugplot
    pub fn rugplot(mut self, show: bool) -> Self {
        self.rugplot = show;
        self
    }

    /// Set color
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set number of bins
    pub fn bins(mut self, bins: usize) -> Self {
        self.bins = bins.max(2);
        self
    }
}

/// Layout for joint plot
#[derive(Debug, Clone)]
pub struct JointPlotLayout {
    /// Main plot bounds (x, y, width, height) as fractions
    pub main_bounds: (f64, f64, f64, f64),
    /// X marginal bounds (top)
    pub x_marginal_bounds: (f64, f64, f64, f64),
    /// Y marginal bounds (right)
    pub y_marginal_bounds: (f64, f64, f64, f64),
}

/// Compute joint plot layout
pub fn joint_plot_layout(marginal_ratio: f64) -> JointPlotLayout {
    let ratio = marginal_ratio.clamp(0.1, 0.4);
    let gap = 0.02;

    JointPlotLayout {
        main_bounds: (0.0, 0.0, 1.0 - ratio - gap, 1.0 - ratio - gap),
        x_marginal_bounds: (0.0, 1.0 - ratio, 1.0 - ratio - gap, ratio),
        y_marginal_bounds: (1.0 - ratio, 0.0, ratio, 1.0 - ratio - gap),
    }
}

/// Computed marginal histogram data
#[derive(Debug, Clone)]
pub struct MarginalHistogram {
    /// Bin edges
    pub edges: Vec<f64>,
    /// Bin counts
    pub counts: Vec<usize>,
    /// Bin centers
    pub centers: Vec<f64>,
}

/// Compute marginal histogram
pub fn compute_marginal_histogram(data: &[f64], bins: usize) -> MarginalHistogram {
    if data.is_empty() {
        return MarginalHistogram {
            edges: vec![],
            counts: vec![],
            centers: vec![],
        };
    }

    let min_val = data.iter().copied().fold(f64::INFINITY, f64::min);
    let max_val = data.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let range = if (max_val - min_val).abs() < 1e-10 {
        1.0
    } else {
        max_val - min_val
    };
    let bin_width = range / bins as f64;

    // Create edges
    let edges: Vec<f64> = (0..=bins).map(|i| min_val + i as f64 * bin_width).collect();

    // Count points in each bin
    let mut counts = vec![0_usize; bins];
    for &val in data {
        let bin = ((val - min_val) / bin_width).floor() as usize;
        let bin = bin.min(bins - 1); // Handle edge case
        counts[bin] += 1;
    }

    // Compute centers
    let centers: Vec<f64> = (0..bins)
        .map(|i| min_val + (i as f64 + 0.5) * bin_width)
        .collect();

    MarginalHistogram {
        edges,
        counts,
        centers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_joint_plot_layout() {
        let layout = joint_plot_layout(0.2);

        // Main plot should take most space
        assert!(layout.main_bounds.2 > 0.5);
        assert!(layout.main_bounds.3 > 0.5);

        // Marginals should be smaller
        assert!(layout.x_marginal_bounds.3 < 0.3);
        assert!(layout.y_marginal_bounds.2 < 0.3);
    }

    #[test]
    fn test_marginal_histogram() {
        let data = vec![1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0];
        let hist = compute_marginal_histogram(&data, 3);

        assert_eq!(hist.counts.len(), 3);
        assert_eq!(hist.edges.len(), 4);
        assert_eq!(hist.centers.len(), 3);

        // Total count should equal data length
        let total: usize = hist.counts.iter().sum();
        assert_eq!(total, 7);
    }

    #[test]
    fn test_marginal_histogram_empty() {
        let data: Vec<f64> = vec![];
        let hist = compute_marginal_histogram(&data, 10);

        assert!(hist.edges.is_empty());
        assert!(hist.counts.is_empty());
    }
}
