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

use super::data::PlotData;
use crate::render::{Color, LineStyle, MarkerStyle};

/// Macro to generate terminal methods for PlotBuilder implementations
///
/// This macro generates the `save()`, `render()`, and `render_to_svg()` methods
/// that are identical across all PlotBuilder config types. Each implementation
/// calls `self.finalize()` before delegating to the underlying Plot method.
///
/// # Usage
///
/// ```rust,ignore
/// impl PlotBuilder<MyConfig> {
///     fn finalize(self) -> Plot { /* ... */ }
/// }
/// impl_terminal_methods!(MyConfig);
/// ```
macro_rules! impl_terminal_methods {
    ($config:ty) => {
        impl PlotBuilder<$config> {
            /// Save the plot to a file
            ///
            /// Finalizes the series and then saves.
            pub fn save<P: AsRef<std::path::Path>>(self, path: P) -> crate::core::Result<()> {
                self.finalize().save(path)
            }

            /// Render the plot to an Image
            ///
            /// Finalizes the series before rendering.
            pub fn render(self) -> crate::core::Result<super::Image> {
                self.finalize().render()
            }

            /// Render the plot to an SVG string
            ///
            /// Finalizes the series before rendering.
            pub fn render_to_svg(self) -> crate::core::Result<String> {
                self.finalize().render_to_svg()
            }

            /// Export to SVG file
            ///
            /// Finalizes the series before exporting.
            pub fn export_svg<P: AsRef<std::path::Path>>(self, path: P) -> crate::core::Result<()> {
                self.finalize().export_svg(path)
            }

            /// Save to PDF file
            ///
            /// Finalizes the series before saving.
            #[cfg(feature = "pdf")]
            pub fn save_pdf<P: AsRef<std::path::Path>>(self, path: P) -> crate::core::Result<()> {
                self.finalize().save_pdf(path)
            }

            /// Save with specific dimensions
            ///
            /// Finalizes the series before saving.
            pub fn save_with_size<P: AsRef<std::path::Path>>(
                self,
                path: P,
                width: u32,
                height: u32,
            ) -> crate::core::Result<()> {
                self.finalize().size_px(width, height).save(path)
            }

            /// Add a line series after finalizing the current series
            ///
            /// Enables chaining multiple series: `.line(...).scatter(...).save()`
            pub fn line<X, Y>(self, x: &X, y: &Y) -> PlotBuilder<crate::plots::basic::LineConfig>
            where
                X: crate::data::Data1D<f64>,
                Y: crate::data::Data1D<f64>,
            {
                self.finalize().line(x, y)
            }

            /// Add a scatter series after finalizing the current series
            ///
            /// Enables chaining multiple series: `.line(...).scatter(...).save()`
            pub fn scatter<X, Y>(
                self,
                x: &X,
                y: &Y,
            ) -> PlotBuilder<crate::plots::basic::ScatterConfig>
            where
                X: crate::data::Data1D<f64>,
                Y: crate::data::Data1D<f64>,
            {
                self.finalize().scatter(x, y)
            }

            /// Set legend position
            ///
            /// Finalizes the series and sets legend position on the resulting Plot.
            pub fn legend_position(self, position: crate::core::LegendPosition) -> super::Plot {
                self.finalize().legend_position(position)
            }

            /// Add a bar series after finalizing the current series
            ///
            /// Enables chaining multiple series: `.line(...).bar(...).save()`
            pub fn bar<S, V>(
                self,
                categories: &[S],
                values: &V,
            ) -> PlotBuilder<crate::plots::basic::BarConfig>
            where
                S: ToString,
                V: crate::data::Data1D<f64>,
            {
                self.finalize().bar(categories, values)
            }

            /// Finish configuring this series and return to the main Plot
            ///
            /// **Deprecated**: Series finalize automatically. Use `.save()` directly.
            #[deprecated(
                since = "0.8.0",
                note = "Not needed - series finalize automatically. Use .save() directly."
            )]
            pub fn end_series(self) -> super::Plot {
                self.finalize()
            }
        }

        impl From<PlotBuilder<$config>> for super::Plot {
            fn from(builder: PlotBuilder<$config>) -> super::Plot {
                builder.finalize()
            }
        }
    };
}

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

impl PlotInput {
    /// Count the number of data points in this input
    pub fn point_count(&self) -> usize {
        match self {
            PlotInput::Single(data) => data.len(),
            PlotInput::XY(x, _) => x.len(),
            PlotInput::Grid2D { x, y, .. } => x.len() * y.len(),
            PlotInput::Categorical { values, .. } => values.len(),
        }
    }
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
    /// Y-axis error bar values
    pub y_errors: Option<crate::plots::error::ErrorValues>,
    /// X-axis error bar values
    pub x_errors: Option<crate::plots::error::ErrorValues>,
    /// Error bar styling configuration
    pub error_config: Option<crate::plots::error::ErrorBarConfig>,
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
#[derive(Debug, Clone)]
pub struct PlotBuilder<C>
where
    C: crate::plots::PlotConfig + Clone,
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

    // ===== Error bar methods =====

    /// Attach symmetric Y error bars to this series
    ///
    /// # Arguments
    /// * `errors` - Error values (same magnitude for +/-)
    pub fn with_yerr<E: crate::data::Data1D<f64>>(mut self, errors: &E) -> Self {
        self.style.y_errors = Some(crate::plots::error::ErrorValues::symmetric(
            errors.iter().copied().collect(),
        ));
        self
    }

    /// Attach symmetric X error bars to this series
    ///
    /// # Arguments
    /// * `errors` - Error values (same magnitude for +/-)
    pub fn with_xerr<E: crate::data::Data1D<f64>>(mut self, errors: &E) -> Self {
        self.style.x_errors = Some(crate::plots::error::ErrorValues::symmetric(
            errors.iter().copied().collect(),
        ));
        self
    }

    /// Attach asymmetric Y error bars to this series
    ///
    /// # Arguments
    /// * `lower` - Lower error values (extending downward)
    /// * `upper` - Upper error values (extending upward)
    pub fn with_yerr_asymmetric<E1, E2>(mut self, lower: &E1, upper: &E2) -> Self
    where
        E1: crate::data::Data1D<f64>,
        E2: crate::data::Data1D<f64>,
    {
        self.style.y_errors = Some(crate::plots::error::ErrorValues::asymmetric(
            lower.iter().copied().collect(),
            upper.iter().copied().collect(),
        ));
        self
    }

    /// Attach asymmetric X error bars to this series
    ///
    /// # Arguments
    /// * `lower` - Lower error values (extending left)
    /// * `upper` - Upper error values (extending right)
    pub fn with_xerr_asymmetric<E1, E2>(mut self, lower: &E1, upper: &E2) -> Self
    where
        E1: crate::data::Data1D<f64>,
        E2: crate::data::Data1D<f64>,
    {
        self.style.x_errors = Some(crate::plots::error::ErrorValues::asymmetric(
            lower.iter().copied().collect(),
            upper.iter().copied().collect(),
        ));
        self
    }

    /// Configure error bar styling
    ///
    /// # Arguments
    /// * `config` - Error bar configuration
    pub fn error_config(mut self, config: crate::plots::error::ErrorBarConfig) -> Self {
        self.style.error_config = Some(config);
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

    /// Enable legend at a specific position
    ///
    /// This method forwards to the inner Plot.
    pub fn legend(mut self, position: crate::core::Position) -> Self {
        self.plot = self.plot.legend(position);
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

    /// Enable auto-optimization for rendering backend selection
    ///
    /// This method forwards to the inner Plot, including the current
    /// builder's data points in the total count for optimization decisions.
    pub fn auto_optimize(mut self) -> Self {
        let current_points = self.input.point_count();
        self.plot = self.plot.auto_optimize_with_extra_points(current_points);
        self
    }

    /// Set X-axis scale (linear, log, symlog)
    ///
    /// This method forwards to the inner Plot.
    pub fn xscale(mut self, scale: crate::axes::AxisScale) -> Self {
        self.plot = self.plot.xscale(scale);
        self
    }

    /// Set Y-axis scale (linear, log, symlog)
    ///
    /// This method forwards to the inner Plot.
    pub fn yscale(mut self, scale: crate::axes::AxisScale) -> Self {
        self.plot = self.plot.yscale(scale);
        self
    }

    /// Set backend explicitly (overrides auto-optimization)
    ///
    /// This method forwards to the inner Plot.
    pub fn backend(mut self, backend: super::BackendType) -> Self {
        self.plot = self.plot.backend(backend);
        self
    }

    /// Enable GPU acceleration for coordinate transformations
    ///
    /// This method forwards to the inner Plot.
    #[cfg(feature = "gpu")]
    pub fn gpu(mut self, enabled: bool) -> Self {
        self.plot = self.plot.gpu(enabled);
        self
    }

    /// Get the name of the currently selected backend
    pub fn get_backend_name(&self) -> &'static str {
        self.plot.get_backend_name()
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

    // ===== Annotation forwarding methods =====

    /// Add an annotation to the plot
    ///
    /// This method forwards to the inner Plot.
    pub fn annotate(mut self, annotation: crate::core::Annotation) -> Self {
        self.plot = self.plot.annotate(annotation);
        self
    }

    /// Add an arrow annotation
    ///
    /// This method forwards to the inner Plot.
    pub fn arrow(mut self, x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        self.plot = self.plot.arrow(x1, y1, x2, y2);
        self
    }

    /// Add a horizontal reference line
    ///
    /// This method forwards to the inner Plot.
    pub fn hline(mut self, y: f64) -> Self {
        self.plot = self.plot.hline(y);
        self
    }

    /// Add a vertical reference line
    ///
    /// This method forwards to the inner Plot.
    pub fn vline(mut self, x: f64) -> Self {
        self.plot = self.plot.vline(x);
        self
    }

    /// Add a fill between two curves
    ///
    /// This method forwards to the inner Plot.
    pub fn fill_between(mut self, x: &[f64], y1: &[f64], y2: &[f64]) -> Self {
        self.plot = self.plot.fill_between(x, y1, y2);
        self
    }

    /// Add a vertical span (shaded region)
    ///
    /// This method forwards to the inner Plot.
    pub fn axvspan(mut self, x_min: f64, x_max: f64) -> Self {
        self.plot = self.plot.axvspan(x_min, x_max);
        self
    }

    /// Add a horizontal span (shaded region)
    ///
    /// This method forwards to the inner Plot.
    pub fn axhspan(mut self, y_min: f64, y_max: f64) -> Self {
        self.plot = self.plot.axhspan(y_min, y_max);
        self
    }

    // ===== Deprecated methods for backward compatibility =====

    // Note: `end_series()` is now generated by impl_terminal_methods! macro
    // to properly call finalize() before returning the Plot.
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
}

// Generate terminal methods (save, render, render_to_svg) for KdeConfig
impl_terminal_methods!(crate::plots::KdeConfig);

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
}

// Generate terminal methods (save, render, render_to_svg) for EcdfConfig
impl_terminal_methods!(crate::plots::EcdfConfig);

// =============================================================================
// Contour Plot Builder
// =============================================================================

impl PlotBuilder<crate::plots::ContourConfig> {
    /// Set number of contour levels
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .contour(&x, &y, &z)
    ///     .levels(15)
    ///     .save("contour.png")?;
    /// ```
    pub fn levels(mut self, n: usize) -> Self {
        self.config.n_levels = n.max(2);
        self
    }

    /// Set explicit contour level values
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .contour(&x, &y, &z)
    ///     .level_values(vec![0.1, 0.2, 0.5, 0.8, 0.9])
    ///     .save("contour.png")?;
    /// ```
    pub fn level_values(mut self, levels: Vec<f64>) -> Self {
        self.config.levels = Some(levels);
        self
    }

    /// Enable/disable filled contours
    ///
    /// When enabled, regions between contour lines are filled with color.
    pub fn filled(mut self, filled: bool) -> Self {
        self.config.filled = filled;
        self
    }

    /// Show/hide contour lines
    pub fn show_lines(mut self, show: bool) -> Self {
        self.config.show_lines = show;
        self
    }

    /// Show/hide contour labels
    pub fn show_labels(mut self, show: bool) -> Self {
        self.config.show_labels = show;
        self
    }

    /// Set colormap by name (e.g., "viridis", "plasma", "magma")
    pub fn colormap_name(mut self, name: &str) -> Self {
        self.config.cmap = name.to_string();
        self
    }

    /// Set contour line width
    pub fn contour_line_width(mut self, width: f32) -> Self {
        self.config.line_width = width.max(0.1);
        self
    }

    /// Enable contour smoothing with interpolation
    ///
    /// Smoothes the contour by upsampling the grid before computing contour lines.
    /// This produces smoother, more professional-looking contours.
    ///
    /// # Arguments
    /// * `method` - Interpolation method (Linear or Cubic)
    /// * `factor` - Upsampling factor (2-8 recommended). Higher = smoother but slower.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ruviz::plots::ContourInterpolation;
    ///
    /// Plot::new()
    ///     .contour(&x, &y, &z)
    ///     .smooth(ContourInterpolation::Cubic, 4)
    ///     .save("smooth_contour.png")?;
    /// ```
    pub fn smooth(mut self, method: crate::plots::ContourInterpolation, factor: usize) -> Self {
        self.config.interpolation = method;
        self.config.interpolation_factor = factor.max(1);
        self
    }

    /// Enable/disable colorbar for the contour plot
    ///
    /// When enabled, a colorbar showing the value-to-color mapping is displayed
    /// to the right of the contour plot.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .contour(&x, &y, &z)
    ///     .colorbar(true)
    ///     .colorbar_label("Temperature (°C)")
    ///     .save("contour_with_colorbar.png")?;
    /// ```
    pub fn colorbar(mut self, show: bool) -> Self {
        self.config.colorbar = show;
        self
    }

    /// Set the colorbar label
    ///
    /// The label is displayed rotated 90° next to the colorbar.
    pub fn colorbar_label(mut self, label: &str) -> Self {
        self.config.colorbar_label = Some(label.to_string());
        self
    }

    /// Finalize the contour series and add it to the plot
    fn finalize(self) -> super::Plot {
        let (x, y, z) = match &self.input {
            PlotInput::Grid2D { x, y, z } => (x.clone(), y.clone(), z.clone()),
            _ => (vec![], vec![], vec![]),
        };

        // Flatten z for compute_contour_plot
        let z_flat: Vec<f64> = z.iter().flat_map(|row| row.iter().copied()).collect();

        // Compute contour data
        let contour_data = crate::plots::compute_contour_plot(&x, &y, &z_flat, &self.config);

        // Add series to plot
        self.plot.add_contour_series(contour_data, self.style)
    }
}

// Generate terminal methods (save, render, render_to_svg) for ContourConfig
impl_terminal_methods!(crate::plots::ContourConfig);

// =============================================================================
// Pie Chart Builder
// =============================================================================

impl PlotBuilder<crate::plots::PieConfig> {
    /// Set slice labels
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .pie(&values)
    ///     .labels(&["A", "B", "C", "D"])
    ///     .save("pie.png")?;
    /// ```
    pub fn labels<S: AsRef<str>>(mut self, labels: &[S]) -> Self {
        self.config.labels = labels.iter().map(|s| s.as_ref().to_string()).collect();
        self
    }

    /// Set explode values for each slice
    ///
    /// Values represent the fraction of the radius to offset each slice.
    /// Higher values push the slice further from center.
    pub fn explode(mut self, explode: &[f64]) -> Self {
        self.config.explode = explode.to_vec();
        self
    }

    /// Create a donut chart with the specified inner radius ratio
    ///
    /// # Arguments
    ///
    /// * `ratio` - Inner radius as fraction of outer radius (0.0 to 0.95)
    pub fn donut(mut self, ratio: f64) -> Self {
        self.config.inner_radius = ratio.clamp(0.0, 0.95);
        self
    }

    /// Set the start angle in degrees (default: 90 = top/12 o'clock)
    pub fn start_angle(mut self, degrees: f64) -> Self {
        self.config.start_angle = degrees;
        self
    }

    /// Enable/disable percentage labels on slices
    ///
    /// When enabled, shows percentage values on each wedge.
    pub fn show_percentages(mut self, show: bool) -> Self {
        self.config.show_percentages = show;
        self
    }

    /// Enable/disable value labels on slices
    pub fn show_values(mut self, show: bool) -> Self {
        self.config.show_values = show;
        self
    }

    /// Enable/disable category labels on slices
    pub fn show_labels(mut self, show: bool) -> Self {
        self.config.show_labels = show;
        self
    }

    /// Set shadow offset (0 = no shadow, higher = more offset)
    pub fn shadow(mut self, offset: f64) -> Self {
        self.config.shadow = offset.max(0.0);
        self
    }

    /// Set label font size
    pub fn font_size(mut self, size: f32) -> Self {
        self.config.label_font_size = size;
        self
    }

    /// Set label distance from center (as fraction of radius)
    pub fn label_distance(mut self, distance: f64) -> Self {
        self.config.label_distance = distance;
        self
    }

    /// Go clockwise instead of counter-clockwise
    pub fn clockwise(mut self) -> Self {
        self.config.counter_clockwise = false;
        self
    }

    /// Finalize the pie series and add it to the plot
    fn finalize(self) -> super::Plot {
        let values = match &self.input {
            PlotInput::Single(v) => v.clone(),
            _ => vec![],
        };

        // Compute pie data using the compute method (normalized coordinates)
        let pie_data = crate::plots::composition::pie::PieData::compute(&values, &self.config);

        // Add series to plot
        self.plot.add_pie_series(pie_data, self.style)
    }
}

// Generate terminal methods (save, render, render_to_svg) for PieConfig
impl_terminal_methods!(crate::plots::PieConfig);

// =============================================================================
// Radar Chart Builder
// =============================================================================

// Note: Radar series metadata is now stored directly in RadarConfig:
// - series_labels: Vec<String> for series names
// - colors: Option<Vec<Color>> for per-series colors
// - per_series_fill_alphas: Vec<Option<f32>> for per-series fill alpha
// - per_series_line_widths: Vec<Option<f32>> for per-series line width
// - current_series_idx: Option<usize> for chained styling

impl PlotBuilder<crate::plots::RadarConfig> {
    /// Add a data series to the radar chart
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .radar(&["A", "B", "C", "D", "E"])
    ///     .series(&[1.0, 2.0, 3.0, 4.0, 5.0])
    ///     .label("Series 1")
    ///     .save("radar.png")?;
    /// ```
    pub fn series<V: crate::data::Data1D<f64>>(mut self, values: &V) -> Self {
        let values_vec: Vec<f64> = (0..values.len())
            .filter_map(|i| values.get(i).copied())
            .collect();

        // Capture any pending label from the previous .label() call for the PREVIOUS series
        // Pattern: .series([...]).label("A").series([...]).label("B")
        // When the second .series() is called, we capture "A" for the first series
        if let Some(label) = self.style.label.take() {
            if let Some(last) = self.config.series_labels.last_mut() {
                if last.is_empty() {
                    *last = label;
                }
            }
        }

        // Store series data in the input
        match &mut self.input {
            PlotInput::Single(data) => {
                // Append values with a separator (NaN) between series
                if !data.is_empty() {
                    data.push(f64::NAN); // Series separator
                }
                data.extend(values_vec);
            }
            _ => {
                self.input = PlotInput::Single(values_vec);
            }
        }

        // Push a placeholder for this new series - will be filled by subsequent .label() call
        self.config.series_labels.push(String::new());

        self
    }

    /// Set label for the current (most recently added) series
    ///
    /// This label appears in the legend for this specific series.
    pub fn series_label(mut self, name: &str) -> Self {
        // Update the label for the most recently added series
        if let Some(last) = self.config.series_labels.last_mut() {
            *last = name.to_string();
        }
        // Also update the style label for backward compatibility
        self.style.label = Some(name.to_string());
        self
    }

    /// Add a named series to the radar chart (recommended API)
    ///
    /// This is the preferred way to add series to a radar chart, as it explicitly
    /// binds the series name with its data. The name will appear in the legend.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ruviz::prelude::*;
    ///
    /// Plot::new()
    ///     .radar(&["Speed", "Power", "Defense", "Magic", "Luck"])
    ///     .add_series("Warrior", &[90.0, 85.0, 80.0, 20.0, 50.0])
    ///     .add_series("Mage", &[30.0, 40.0, 30.0, 95.0, 60.0])
    ///     .title("Character Comparison")
    ///     .save("characters.png")?;
    /// ```
    ///
    /// You can also chain styling methods after `add_series()`:
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .radar(&["A", "B", "C"])
    ///     .add_series("Series 1", &[1.0, 2.0, 3.0])
    ///         .with_color(Color::RED)
    ///         .with_fill_alpha(0.4)
    ///     .add_series("Series 2", &[3.0, 2.0, 1.0])
    ///         .with_color(Color::BLUE)
    ///     .save("styled.png")?;
    /// ```
    pub fn add_series<S, V>(mut self, name: S, values: &V) -> Self
    where
        S: Into<String>,
        V: crate::data::Data1D<f64>,
    {
        let values_vec: Vec<f64> = (0..values.len())
            .filter_map(|i| values.get(i).copied())
            .collect();

        let name_string = name.into();

        // Add to series_labels
        self.config.series_labels.push(name_string);

        // Initialize per-series styling with None (use defaults)
        // Ensure colors vec exists
        if self.config.colors.is_none() {
            self.config.colors = Some(vec![]);
        }
        if let Some(ref mut colors) = self.config.colors {
            colors.push(Color::TRANSPARENT); // Placeholder, will be replaced by theme color if not set
        }
        self.config.per_series_fill_alphas.push(None);
        self.config.per_series_line_widths.push(None);

        // Track current series index for chained styling
        let series_idx = self.config.series_labels.len() - 1;
        self.config.current_series_idx = Some(series_idx);

        // Store in input for finalize() compatibility
        match &mut self.input {
            PlotInput::Single(data) => {
                if !data.is_empty() {
                    data.push(f64::NAN); // Series separator
                }
                data.extend(values_vec);
            }
            _ => {
                self.input = PlotInput::Single(values_vec);
            }
        }

        self
    }

    /// Set color for the current (most recently added) series
    ///
    /// This method applies to the series added by the most recent `add_series()` call.
    /// If no series has been added, this is a no-op.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .radar(&["A", "B", "C"])
    ///     .add_series("Red Series", &[1.0, 2.0, 3.0])
    ///         .with_color(Color::RED)
    ///     .save("red.png")?;
    /// ```
    pub fn with_color(mut self, color: Color) -> Self {
        if let Some(idx) = self.config.current_series_idx {
            if let Some(ref mut colors) = self.config.colors {
                if let Some(c) = colors.get_mut(idx) {
                    *c = color;
                }
            }
        }
        self
    }

    /// Set fill alpha for the current (most recently added) series
    ///
    /// This method applies to the series added by the most recent `add_series()` call.
    /// Values range from 0.0 (transparent) to 1.0 (opaque).
    /// If no series has been added, this is a no-op.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .radar(&["A", "B", "C"])
    ///     .add_series("Transparent", &[1.0, 2.0, 3.0])
    ///         .with_fill_alpha(0.2)
    ///     .save("transparent.png")?;
    /// ```
    pub fn with_fill_alpha(mut self, alpha: f32) -> Self {
        if let Some(idx) = self.config.current_series_idx {
            if let Some(a) = self.config.per_series_fill_alphas.get_mut(idx) {
                *a = Some(alpha.clamp(0.0, 1.0));
            }
        }
        self
    }

    /// Set line width for the current (most recently added) series
    ///
    /// This method applies to the series added by the most recent `add_series()` call.
    /// If no series has been added, this is a no-op.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .radar(&["A", "B", "C"])
    ///     .add_series("Thick Lines", &[1.0, 2.0, 3.0])
    ///         .with_line_width(3.0)
    ///     .save("thick.png")?;
    /// ```
    pub fn with_line_width(mut self, width: f32) -> Self {
        if let Some(idx) = self.config.current_series_idx {
            if let Some(w) = self.config.per_series_line_widths.get_mut(idx) {
                *w = Some(width.max(0.1));
            }
        }
        self
    }

    /// Set fill alpha for the current series
    ///
    /// Values range from 0.0 (transparent) to 1.0 (opaque).
    pub fn fill_alpha(mut self, alpha: f32) -> Self {
        self.config.fill_alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Set number of grid rings
    pub fn rings(mut self, n: usize) -> Self {
        self.config.grid_rings = n.max(1);
        self
    }

    /// Enable/disable fill for the polygon
    pub fn fill(mut self, fill: bool) -> Self {
        self.config.fill = fill;
        self
    }

    /// Set line width
    pub fn radar_line_width(mut self, width: f32) -> Self {
        self.config.line_width = width.max(0.1);
        self
    }

    /// Show/hide axis labels
    pub fn show_axis_labels(mut self, show: bool) -> Self {
        self.config.show_axis_labels = show;
        self
    }

    /// Finalize the radar chart and add it to the plot
    fn finalize(mut self) -> super::Plot {
        // Capture any pending label from the last .label() call for the last series
        // (since there's no subsequent .series() call to capture it)
        if let Some(label) = self.style.label.take() {
            if let Some(last) = self.config.series_labels.last_mut() {
                if last.is_empty() {
                    *last = label;
                }
            }
        }

        // Parse series from the accumulated data
        let all_values = match &self.input {
            PlotInput::Single(v) => v.clone(),
            _ => vec![],
        };

        // Split by NaN separators
        let mut series_data: Vec<Vec<f64>> = vec![];
        let mut current_series: Vec<f64> = vec![];

        for &v in &all_values {
            if v.is_nan() {
                if !current_series.is_empty() {
                    series_data.push(current_series);
                    current_series = vec![];
                }
            } else {
                current_series.push(v);
            }
        }
        if !current_series.is_empty() {
            series_data.push(current_series);
        }

        // Compute radar data with series labels
        let series_labels = if self.config.series_labels.is_empty() {
            None
        } else {
            Some(self.config.series_labels.as_slice())
        };
        let radar_data = crate::plots::compute_radar_chart_with_labels(
            &series_data,
            &self.config,
            series_labels,
        );

        // Add series to plot
        self.plot.add_radar_series(radar_data, self.style)
    }
}

// Generate terminal methods (save, render, render_to_svg) for RadarConfig
impl_terminal_methods!(crate::plots::RadarConfig);

// =============================================================================
// Polar Plot Builder
// =============================================================================

impl PlotBuilder<crate::plots::PolarPlotConfig> {
    /// Enable fill under the polar curve
    pub fn fill(mut self, fill: bool) -> Self {
        self.config.fill = fill;
        self
    }

    /// Set fill alpha (transparency)
    pub fn fill_alpha(mut self, alpha: f32) -> Self {
        self.config.fill_alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Set marker size (0 = no markers)
    pub fn marker_size(mut self, size: f32) -> Self {
        self.config.marker_size = size.max(0.0);
        self
    }

    /// Show/hide angular labels (0°, 45°, 90°, etc.)
    pub fn show_theta_labels(mut self, show: bool) -> Self {
        self.config.show_theta_labels = show;
        self
    }

    /// Show/hide radial labels
    pub fn show_r_labels(mut self, show: bool) -> Self {
        self.config.show_r_labels = show;
        self
    }

    /// Set theta (angle) offset in radians
    pub fn theta_offset(mut self, offset: f64) -> Self {
        self.config.theta_offset = offset;
        self
    }

    /// Finalize the polar series and add it to the plot
    fn finalize(self) -> super::Plot {
        let (r, theta) = match &self.input {
            PlotInput::XY(r, theta) => (r.clone(), theta.clone()),
            _ => (vec![], vec![]),
        };

        // Compute polar data
        let polar_data = crate::plots::compute_polar_plot(&r, &theta, &self.config);

        // Add series to plot
        self.plot.add_polar_series(polar_data, self.style)
    }
}

// Generate terminal methods (save, render, render_to_svg) for PolarPlotConfig
impl_terminal_methods!(crate::plots::PolarPlotConfig);

// =============================================================================
// Violin Plot Builder
// =============================================================================

impl PlotBuilder<crate::plots::ViolinConfig> {
    /// Show/hide inner boxplot
    ///
    /// When enabled, shows a small box representing the IQR inside the violin.
    pub fn show_box(mut self, show: bool) -> Self {
        self.config.show_box = show;
        self
    }

    /// Show/hide quartile lines
    pub fn show_quartiles(mut self, show: bool) -> Self {
        self.config.show_quartiles = show;
        self
    }

    /// Show/hide median marker
    pub fn show_median(mut self, show: bool) -> Self {
        self.config.show_median = show;
        self
    }

    /// Show/hide data points inside the violin
    pub fn show_points(mut self, show: bool) -> Self {
        self.config.show_points = show;
        self
    }

    /// Enable split violin mode (half-violin)
    pub fn split(mut self, split: bool) -> Self {
        self.config.split = split;
        self
    }

    /// Set fill alpha (transparency)
    ///
    /// Values range from 0.0 (transparent) to 1.0 (opaque).
    pub fn fill_alpha(mut self, alpha: f32) -> Self {
        self.config.fill_alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Set violin width
    pub fn width(mut self, width: f64) -> Self {
        self.config.width = width.max(0.1);
        self
    }

    /// Set horizontal orientation
    pub fn horizontal(mut self) -> Self {
        self.config.orientation = crate::plots::distribution::violin::Orientation::Horizontal;
        self
    }

    /// Set vertical orientation (default)
    pub fn vertical(mut self) -> Self {
        self.config.orientation = crate::plots::distribution::violin::Orientation::Vertical;
        self
    }

    /// Set number of KDE evaluation points
    pub fn n_points(mut self, n: usize) -> Self {
        self.config.n_points = n.max(10);
        self
    }

    /// Set category name for this violin
    ///
    /// The category name is displayed on the X-axis instead of numeric values.
    /// This enables categorical axis mode for the plot.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .violin(&data)
    ///     .category("Group A")
    ///     .save("violin.png")?;
    /// ```
    pub fn category(mut self, name: &str) -> Self {
        self.config.category = Some(name.to_string());
        self
    }

    /// Finalize the violin series and add it to the plot
    fn finalize(self) -> super::Plot {
        let data = match &self.input {
            PlotInput::Single(d) => d.clone(),
            _ => vec![],
        };

        // Compute violin data
        let violin_data = crate::plots::ViolinData::from_values(&data, &self.config);

        match violin_data {
            Some(vdata) => self.plot.add_violin_series(vdata, self.style),
            None => self.plot, // Return plot unchanged if data is invalid
        }
    }
}

// Generate terminal methods (save, render, render_to_svg) for ViolinConfig
impl_terminal_methods!(crate::plots::ViolinConfig);

// ============================================================================
// LineConfig PlotBuilder Implementation
// ============================================================================

impl PlotBuilder<crate::plots::basic::LineConfig> {
    /// Set marker style for data points (enables markers)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .marker(MarkerStyle::Circle)
    ///     .save("line_markers.png")?;
    /// ```
    pub fn marker(mut self, style: crate::render::MarkerStyle) -> Self {
        self.config.marker = Some(style);
        self.config.show_markers = true;
        self
    }

    /// Set marker size
    ///
    /// # Arguments
    /// * `size` - Marker size in points (default: 6.0)
    pub fn marker_size(mut self, size: f32) -> Self {
        self.config.marker_size = size.max(0.1);
        self
    }

    /// Enable or disable markers on data points
    pub fn show_markers(mut self, show: bool) -> Self {
        self.config.show_markers = show;
        self
    }

    /// Set whether to draw the connecting line
    ///
    /// Set to `false` to show only markers without connecting lines.
    pub fn draw_line(mut self, draw: bool) -> Self {
        self.config.draw_line = draw;
        self
    }

    /// Set line style (solid, dashed, dotted, etc.)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .style(LineStyle::Dashed)
    ///     .save("dashed_line.png")?;
    /// ```
    pub fn style(mut self, line_style: crate::render::LineStyle) -> Self {
        self.style.line_style = Some(line_style);
        self
    }

    /// Finalize the line series and add it to the plot
    fn finalize(self) -> super::Plot {
        let (x_data, y_data) = match &self.input {
            PlotInput::XY(x, y) => (PlotData::Static(x.clone()), PlotData::Static(y.clone())),
            PlotInput::Single(y) => {
                // Generate x values as indices
                let x: Vec<f64> = (0..y.len()).map(|i| i as f64).collect();
                (PlotData::Static(x), PlotData::Static(y.clone()))
            }
            _ => (PlotData::Static(vec![]), PlotData::Static(vec![])),
        };

        self.plot
            .add_line_series(x_data, y_data, &self.config, self.style)
    }
}

// Generate terminal methods for LineConfig
impl_terminal_methods!(crate::plots::basic::LineConfig);

// ============================================================================
// ScatterConfig PlotBuilder Implementation
// ============================================================================

impl PlotBuilder<crate::plots::basic::ScatterConfig> {
    /// Set marker style
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .scatter(&x, &y)
    ///     .marker(MarkerStyle::Triangle)
    ///     .save("scatter.png")?;
    /// ```
    pub fn marker(mut self, style: crate::render::MarkerStyle) -> Self {
        self.config.marker = style;
        self
    }

    /// Set marker size
    ///
    /// # Arguments
    /// * `size` - Marker size in points (default: 6.0)
    pub fn marker_size(mut self, size: f32) -> Self {
        self.config.size = size.max(0.1);
        self
    }

    /// Set marker edge width
    ///
    /// # Arguments
    /// * `width` - Edge width in points (default: 0.5)
    pub fn edge_width(mut self, width: f32) -> Self {
        self.config.edge_width = width.max(0.0);
        self
    }

    /// Set marker edge color
    pub fn edge_color(mut self, color: Color) -> Self {
        self.config.edge_color = Some(color);
        self
    }

    /// Finalize the scatter series and add it to the plot
    fn finalize(self) -> super::Plot {
        let (x_data, y_data) = match &self.input {
            PlotInput::XY(x, y) => (PlotData::Static(x.clone()), PlotData::Static(y.clone())),
            PlotInput::Single(y) => {
                let x: Vec<f64> = (0..y.len()).map(|i| i as f64).collect();
                (PlotData::Static(x), PlotData::Static(y.clone()))
            }
            _ => (PlotData::Static(vec![]), PlotData::Static(vec![])),
        };

        self.plot
            .add_scatter_series(x_data, y_data, &self.config, self.style)
    }
}

// Generate terminal methods for ScatterConfig
impl_terminal_methods!(crate::plots::basic::ScatterConfig);

// ============================================================================
// BarConfig PlotBuilder Implementation
// ============================================================================

impl PlotBuilder<crate::plots::basic::BarConfig> {
    /// Set bar width as fraction of available space
    ///
    /// # Arguments
    /// * `width` - Width fraction (0.0-1.0, default: 0.8)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .bar(&["A", "B", "C"], &[1.0, 2.0, 3.0])
    ///     .bar_width(0.6)
    ///     .save("bar.png")?;
    /// ```
    pub fn bar_width(mut self, width: f32) -> Self {
        self.config.width = width.clamp(0.0, 1.0);
        self
    }

    /// Set bar edge width
    ///
    /// # Arguments
    /// * `width` - Edge width in points (default: 0.8)
    pub fn edge_width(mut self, width: f32) -> Self {
        self.config.edge_width = width.max(0.0);
        self
    }

    /// Set bar edge color
    pub fn edge_color(mut self, color: Color) -> Self {
        self.config.edge_color = Some(color);
        self
    }

    /// Set bar orientation (vertical or horizontal)
    pub fn orientation(mut self, orientation: crate::plots::basic::BarOrientation) -> Self {
        self.config.orientation = orientation;
        self
    }

    /// Set base value for bars
    ///
    /// # Arguments
    /// * `bottom` - Base value for bars (default: 0.0)
    pub fn bottom(mut self, bottom: f64) -> Self {
        self.config.bottom = bottom;
        self
    }

    /// Finalize the bar series and add it to the plot
    fn finalize(self) -> super::Plot {
        let (categories, values) = match &self.input {
            PlotInput::Categorical { categories, values } => {
                (categories.clone(), PlotData::Static(values.clone()))
            }
            PlotInput::Single(y) => {
                // Generate category labels as indices
                let cats: Vec<String> = (0..y.len()).map(|i| i.to_string()).collect();
                (cats, PlotData::Static(y.clone()))
            }
            _ => (vec![], PlotData::Static(vec![])),
        };

        self.plot
            .add_bar_series(categories, values, &self.config, self.style)
    }
}

// Generate terminal methods for BarConfig
impl_terminal_methods!(crate::plots::basic::BarConfig);

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
