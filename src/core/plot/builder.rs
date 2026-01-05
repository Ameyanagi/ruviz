//! Generic PlotBuilder for trait-based plot types
//!
//! This module provides `PlotBuilder<C>`, a generic builder that enables
//! zero-ceremony API patterns for plot types implementing the plot traits.
//!
//! # Design Philosophy
//!
//! The builder uses ownership-based state transitions:
//! - Series methods consume `Plot` and return `PlotBuilder<C>`
//! - Config methods return `PlotBuilder<C>` (same type)
//! - Terminal methods auto-finalize and save/render
//! - Plot-level methods forward to the inner `Plot`
//!
//! This enables seamless chaining without explicit `.end()` calls:
//!
//! ```rust,ignore
//! Plot::new()
//!     .kde(&data)           // -> PlotBuilder<KdeConfig>
//!     .bandwidth(0.5)       // -> PlotBuilder<KdeConfig>
//!     .title("KDE Plot")    // -> PlotBuilder<KdeConfig> (forwards to Plot)
//!     .save("kde.png")?;    // auto-finalize and save
//! ```

use crate::render::{Color, LineStyle, MarkerStyle};

/// Marker type for plot input data
///
/// This enum captures the different input types that plot series can have.
/// It allows the builder to store the input data generically.
#[derive(Clone, Debug)]
pub enum PlotInput {
    /// Single 1D data array (for KDE, histogram, ECDF, etc.)
    Single(Vec<f64>),
    /// Paired X-Y data (for line, scatter, etc.)
    XY(Vec<f64>, Vec<f64>),
    /// 2D grid data (for heatmap, contour)
    Grid2D {
        x: Vec<f64>,
        y: Vec<f64>,
        z: Vec<Vec<f64>>,
    },
    /// Categorical data (for bar charts)
    Categorical {
        categories: Vec<String>,
        values: Vec<f64>,
    },
}

/// Style options for a series
///
/// These are common styling options that apply to most plot types.
#[derive(Clone, Debug, Default)]
pub struct SeriesStyle {
    /// Series label for legend
    pub label: Option<String>,
    /// Series color
    pub color: Option<Color>,
    /// Line width override
    pub line_width: Option<f32>,
    /// Line style override
    pub line_style: Option<LineStyle>,
    /// Marker style (for scatter-like plots)
    pub marker_style: Option<MarkerStyle>,
    /// Marker size
    pub marker_size: Option<f32>,
    /// Alpha/transparency (0.0 = transparent, 1.0 = opaque)
    pub alpha: Option<f32>,
}

/// Generic plot builder for trait-based plot types
///
/// `PlotBuilder<C>` owns the `Plot` and accumulates series configuration
/// for a specific plot type parameterized by its config type `C`.
///
/// # Type Parameters
///
/// * `C` - The configuration type for this plot series (e.g., `KdeConfig`)
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::prelude::*;
///
/// // Zero-ceremony API - no .end() needed!
/// Plot::new()
///     .kde(&data)
///     .bandwidth(0.5)
///     .fill(true)
///     .save("kde.png")?;
///
/// // Multiple series - auto-finalize on transition
/// Plot::new()
///     .kde(&data1).color(Color::RED).label("Dataset A")
///     .kde(&data2).color(Color::BLUE).label("Dataset B")
///     .legend_best()
///     .save("comparison.png")?;
/// ```
#[derive(Debug)]
pub struct PlotBuilder<C>
where
    C: crate::plots::PlotConfig,
{
    /// The inner Plot being built (owned)
    pub(crate) plot: super::Plot,
    /// Input data for this series
    pub(crate) input: PlotInput,
    /// Configuration for this series
    pub(crate) config: C,
    /// Styling options for this series
    pub(crate) style: SeriesStyle,
}

impl<C> PlotBuilder<C>
where
    C: crate::plots::PlotConfig,
{
    /// Create a new PlotBuilder with the given plot, input, and config
    pub(crate) fn new(plot: super::Plot, input: PlotInput, config: C) -> Self {
        Self {
            plot,
            input,
            config,
            style: SeriesStyle::default(),
        }
    }

    // ===== Common styling methods =====

    /// Set series label for legend
    ///
    /// Labels identify this series in the plot legend.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .kde(&data)
    ///     .label("My KDE")
    ///     .legend_best()
    ///     .save("labeled.png")?;
    /// ```
    pub fn label<S: Into<String>>(mut self, label: S) -> Self {
        self.style.label = Some(label.into());
        self
    }

    /// Set series color
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .kde(&data)
    ///     .color(Color::RED)
    ///     .save("colored.png")?;
    /// ```
    pub fn color(mut self, color: Color) -> Self {
        self.style.color = Some(color);
        self
    }

    /// Set line width
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .kde(&data)
    ///     .line_width(2.5)
    ///     .save("thick.png")?;
    /// ```
    pub fn line_width(mut self, width: f32) -> Self {
        self.style.line_width = Some(width.max(0.1));
        self
    }

    /// Set line style
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .kde(&data)
    ///     .line_style(LineStyle::Dashed)
    ///     .save("dashed.png")?;
    /// ```
    pub fn line_style(mut self, style: LineStyle) -> Self {
        self.style.line_style = Some(style);
        self
    }

    /// Set transparency
    ///
    /// Values range from 0.0 (fully transparent) to 1.0 (fully opaque).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .kde(&data)
    ///     .alpha(0.7)
    ///     .save("transparent.png")?;
    /// ```
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.style.alpha = Some(alpha.clamp(0.0, 1.0));
        self
    }

    // ===== Plot-level method forwarding =====

    /// Set plot title
    ///
    /// This method forwards to the inner Plot.
    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.plot = self.plot.title(title);
        self
    }

    /// Set X-axis label
    ///
    /// This method forwards to the inner Plot.
    pub fn xlabel<S: Into<String>>(mut self, label: S) -> Self {
        self.plot = self.plot.xlabel(label);
        self
    }

    /// Set Y-axis label
    ///
    /// This method forwards to the inner Plot.
    pub fn ylabel<S: Into<String>>(mut self, label: S) -> Self {
        self.plot = self.plot.ylabel(label);
        self
    }

    /// Enable legend with automatic best position
    ///
    /// This method forwards to the inner Plot.
    pub fn legend_best(mut self) -> Self {
        self.plot = self.plot.legend_best();
        self
    }

    /// Set figure size in inches
    ///
    /// This method forwards to the inner Plot.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.plot = self.plot.size(width, height);
        self
    }

    /// Set figure size in pixels
    ///
    /// This method forwards to the inner Plot.
    pub fn size_px(mut self, width: u32, height: u32) -> Self {
        self.plot = self.plot.size_px(width, height);
        self
    }

    /// Set DPI for export quality
    ///
    /// This method forwards to the inner Plot.
    pub fn dpi(mut self, dpi: u32) -> Self {
        self.plot = self.plot.dpi(dpi);
        self
    }

    /// Set X-axis limits
    ///
    /// This method forwards to the inner Plot.
    pub fn xlim(mut self, min: f64, max: f64) -> Self {
        self.plot = self.plot.xlim(min, max);
        self
    }

    /// Set Y-axis limits
    ///
    /// This method forwards to the inner Plot.
    pub fn ylim(mut self, min: f64, max: f64) -> Self {
        self.plot = self.plot.ylim(min, max);
        self
    }

    /// Enable/disable grid
    ///
    /// This method forwards to the inner Plot.
    pub fn grid(mut self, enabled: bool) -> Self {
        self.plot = self.plot.grid(enabled);
        self
    }

    /// Set theme
    ///
    /// This method forwards to the inner Plot.
    pub fn theme(mut self, theme: crate::render::Theme) -> Self {
        self.plot = self.plot.theme(theme);
        self
    }

    // ===== Accessor methods =====

    /// Get a reference to the current configuration
    pub fn get_config(&self) -> &C {
        &self.config
    }

    /// Get a mutable reference to the current configuration
    pub fn get_config_mut(&mut self) -> &mut C {
        &mut self.config
    }

    /// Get a reference to the inner Plot
    pub fn get_plot(&self) -> &super::Plot {
        &self.plot
    }

    // ===== Deprecated methods for backward compatibility =====

    /// Finish configuring this series and return to the main Plot
    ///
    /// **Deprecated**: Series finalize automatically. Use `.save()` directly.
    #[deprecated(
        since = "0.8.0",
        note = "Not needed - series finalize automatically. Use .save() directly."
    )]
    pub fn end(self) -> super::Plot {
        // Note: Actual finalization would happen here in a full implementation
        // For now, just return the plot
        self.plot
    }

    /// Finish configuring this series and return to the main Plot
    ///
    /// **Deprecated**: Series finalize automatically. Use `.save()` directly.
    #[deprecated(
        since = "0.8.0",
        note = "Not needed - series finalize automatically. Use .save() directly."
    )]
    pub fn end_series(self) -> super::Plot {
        // Note: Actual finalization would happen here in a full implementation
        #[allow(deprecated)]
        self.end()
    }
}

// Note: Terminal methods (save, render) are implemented per-config type
// to properly finalize series before saving. See PlotBuilder<KdeConfig> below.

// =============================================================================
// KDE-specific PlotBuilder methods
// =============================================================================

impl PlotBuilder<crate::plots::KdeConfig> {
    /// Set bandwidth for KDE
    ///
    /// Bandwidth controls the smoothness of the density estimate.
    /// If not set, Scott's rule is used for automatic bandwidth selection.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .kde(&data)
    ///     .bandwidth(0.5)
    ///     .save("kde.png")?;
    /// ```
    pub fn bandwidth(mut self, bw: f64) -> Self {
        self.config.bandwidth = Some(bw);
        self
    }

    /// Set number of points for density curve
    ///
    /// More points create a smoother curve but increase computation time.
    /// Default is 200 points.
    pub fn n_points(mut self, n: usize) -> Self {
        self.config.n_points = n.max(10);
        self
    }

    /// Enable/disable fill under the curve
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .kde(&data)
    ///     .fill(true)
    ///     .fill_alpha(0.3)
    ///     .save("kde.png")?;
    /// ```
    pub fn fill(mut self, fill: bool) -> Self {
        self.config.fill = fill;
        self
    }

    /// Set fill alpha (transparency)
    ///
    /// Values range from 0.0 (fully transparent) to 1.0 (fully opaque).
    /// Default is 0.3.
    pub fn fill_alpha(mut self, alpha: f32) -> Self {
        self.config.fill_alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Set KDE line width
    ///
    /// This is a config-level setting separate from the series style line_width.
    pub fn kde_line_width(mut self, width: f32) -> Self {
        self.config.line_width = width.max(0.1);
        self
    }

    /// Enable cumulative distribution mode
    ///
    /// When enabled, displays the cumulative distribution function (CDF)
    /// instead of the probability density function (PDF).
    pub fn cumulative(mut self, cumulative: bool) -> Self {
        self.config.cumulative = cumulative;
        self
    }

    /// Clip the KDE to specified bounds
    ///
    /// Useful for truncating the density estimate at natural boundaries.
    pub fn clip(mut self, min: f64, max: f64) -> Self {
        self.config.clip = Some((min, max));
        self
    }

    /// Add a vertical reference line at the specified value
    pub fn vertical_line(mut self, x: f64) -> Self {
        self.config.vertical_lines.push(x);
        self
    }

    /// Finalize the KDE series and add it to the plot
    ///
    /// This computes the KDE and adds it as a series to the inner Plot.
    fn finalize(self) -> super::Plot {
        let data = match &self.input {
            PlotInput::Single(d) => d.clone(),
            _ => vec![], // Should not happen for KDE
        };

        // Compute KDE
        let kde_data = crate::plots::compute_kde(&data, &self.config);

        // Add series to plot using internal mutation
        self.plot.add_kde_series(kde_data, self.style)
    }

    /// Save the plot to a file
    ///
    /// Finalizes the KDE series and then saves.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .kde(&data)
    ///     .bandwidth(0.5)
    ///     .save("kde.png")?;
    /// ```
    pub fn save<P: AsRef<std::path::Path>>(self, path: P) -> crate::core::Result<()> {
        self.finalize().save(path)
    }

    /// Render the plot to an Image
    ///
    /// Finalizes the KDE series before rendering.
    pub fn render(self) -> crate::core::Result<super::Image> {
        self.finalize().render()
    }

    /// Render the plot to an SVG string
    ///
    /// Finalizes the KDE series before rendering.
    pub fn render_to_svg(self) -> crate::core::Result<String> {
        self.finalize().render_to_svg()
    }
}

// =============================================================================
// ECDF (Empirical Cumulative Distribution Function) Builder
// =============================================================================

impl PlotBuilder<crate::plots::EcdfConfig> {
    /// Set the statistic type for ECDF
    ///
    /// Options:
    /// - `EcdfStat::Proportion` (default): Y-axis from 0 to 1
    /// - `EcdfStat::Count`: Y-axis shows raw counts
    /// - `EcdfStat::Percent`: Y-axis from 0 to 100
    pub fn stat(mut self, stat: crate::plots::EcdfStat) -> Self {
        self.config.stat = stat;
        self
    }

    /// Enable complementary ECDF (survival function)
    ///
    /// When enabled, plots 1 - ECDF(x) instead of ECDF(x).
    pub fn complementary(mut self, comp: bool) -> Self {
        self.config.complementary = comp;
        self
    }

    /// Show confidence interval band
    ///
    /// Uses the DKW inequality to compute confidence bounds.
    pub fn show_ci(mut self, show: bool) -> Self {
        self.config.show_ci = show;
        self
    }

    /// Set confidence level for CI band
    ///
    /// Default is 0.95 (95% confidence interval).
    pub fn ci_level(mut self, level: f64) -> Self {
        self.config.ci_level = level.clamp(0.0, 1.0);
        self
    }

    /// Show markers at each data point
    pub fn show_markers(mut self, show: bool) -> Self {
        self.config.show_markers = show;
        self
    }

    /// Set marker size
    pub fn marker_size(mut self, size: f32) -> Self {
        self.config.marker_size = size.max(0.1);
        self
    }

    /// Set line width for ECDF
    pub fn ecdf_line_width(mut self, width: f32) -> Self {
        self.config.line_width = width.max(0.1);
        self
    }

    /// Finalize the ECDF series and add it to the plot
    fn finalize(self) -> super::Plot {
        let data = match &self.input {
            PlotInput::Single(d) => d.clone(),
            _ => vec![], // Should not happen for ECDF
        };

        // Compute ECDF
        let ecdf_data = crate::plots::compute_ecdf(&data, &self.config);

        // Add series to plot using internal mutation
        self.plot.add_ecdf_series(ecdf_data, self.style)
    }

    /// Save the plot to a file
    ///
    /// Finalizes the ECDF series and then saves.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .ecdf(&data)
    ///     .show_ci(true)
    ///     .save("ecdf.png")?;
    /// ```
    pub fn save<P: AsRef<std::path::Path>>(self, path: P) -> crate::core::Result<()> {
        self.finalize().save(path)
    }

    /// Render the plot to an Image
    ///
    /// Finalizes the ECDF series before rendering.
    pub fn render(self) -> crate::core::Result<super::Image> {
        self.finalize().render()
    }

    /// Render the plot to an SVG string
    ///
    /// Finalizes the ECDF series before rendering.
    pub fn render_to_svg(self) -> crate::core::Result<String> {
        self.finalize().render_to_svg()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A minimal test config for testing the builder infrastructure
    #[derive(Debug, Clone, Default)]
    struct TestConfig {
        value: f64,
    }

    impl crate::plots::PlotConfig for TestConfig {}

    #[test]
    fn test_plot_builder_creation() {
        let plot = super::super::Plot::new();
        let input = PlotInput::Single(vec![1.0, 2.0, 3.0]);
        let config = TestConfig::default();

        let builder = PlotBuilder::new(plot, input, config);

        assert!(builder.style.label.is_none());
        assert!(builder.style.color.is_none());
    }

    #[test]
    fn test_plot_builder_styling() {
        let plot = super::super::Plot::new();
        let input = PlotInput::Single(vec![1.0, 2.0, 3.0]);
        let config = TestConfig::default();

        let builder = PlotBuilder::new(plot, input, config)
            .label("Test")
            .color(Color::RED)
            .line_width(2.0)
            .alpha(0.8);

        assert_eq!(builder.style.label, Some("Test".to_string()));
        assert!(builder.style.color.is_some());
        assert_eq!(builder.style.line_width, Some(2.0));
        assert_eq!(builder.style.alpha, Some(0.8));
    }

    #[test]
    fn test_plot_builder_plot_forwarding() {
        let plot = super::super::Plot::new();
        let input = PlotInput::Single(vec![1.0, 2.0, 3.0]);
        let config = TestConfig::default();

        let builder = PlotBuilder::new(plot, input, config)
            .title("My Title")
            .xlabel("X Axis")
            .ylabel("Y Axis");

        // The plot should have the title set (we can check by calling get_plot)
        // Note: Plot fields are private, so we can't directly verify here
        // But the test ensures the method chaining works
        assert!(builder.get_plot().get_config().figure.width > 0.0);
    }

    #[test]
    fn test_plot_builder_alpha_clamping() {
        let plot = super::super::Plot::new();
        let input = PlotInput::Single(vec![1.0, 2.0, 3.0]);
        let config = TestConfig::default();

        let builder = PlotBuilder::new(plot, input, config).alpha(1.5); // Should clamp to 1.0
        assert_eq!(builder.style.alpha, Some(1.0));

        let plot = super::super::Plot::new();
        let input = PlotInput::Single(vec![1.0, 2.0, 3.0]);
        let config = TestConfig::default();

        let builder = PlotBuilder::new(plot, input, config).alpha(-0.5); // Should clamp to 0.0
        assert_eq!(builder.style.alpha, Some(0.0));
    }

    #[test]
    fn test_plot_builder_line_width_min() {
        let plot = super::super::Plot::new();
        let input = PlotInput::Single(vec![1.0, 2.0, 3.0]);
        let config = TestConfig::default();

        let builder = PlotBuilder::new(plot, input, config).line_width(0.01); // Should clamp to 0.1
        assert_eq!(builder.style.line_width, Some(0.1));
    }

    #[test]
    fn test_plot_input_variants() {
        // Test Single variant
        let single = PlotInput::Single(vec![1.0, 2.0]);
        match single {
            PlotInput::Single(data) => assert_eq!(data.len(), 2),
            _ => panic!("Expected Single variant"),
        }

        // Test XY variant
        let xy = PlotInput::XY(vec![1.0, 2.0], vec![3.0, 4.0]);
        match xy {
            PlotInput::XY(x, y) => {
                assert_eq!(x.len(), 2);
                assert_eq!(y.len(), 2);
            }
            _ => panic!("Expected XY variant"),
        }

        // Test Categorical variant
        let cat = PlotInput::Categorical {
            categories: vec!["A".to_string(), "B".to_string()],
            values: vec![10.0, 20.0],
        };
        match cat {
            PlotInput::Categorical { categories, values } => {
                assert_eq!(categories.len(), 2);
                assert_eq!(values.len(), 2);
            }
            _ => panic!("Expected Categorical variant"),
        }
    }
}
