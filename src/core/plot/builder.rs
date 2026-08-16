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
//!
//! Mixed series and styled annotations can also continue without `.end_series()`:
//!
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! let x = vec![0.0, 1.0, 2.0];
//! let y = vec![0.0, 1.0, 0.5];
//!
//! Plot::new()
//!     .line(&x, &y)
//!     .vline_styled(1.0, Color::RED, 2.0, LineStyle::Dashed)
//!     .scatter(&x, &y)
//!     .save("chained.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # IntoPlot Trait
//!
//! The [`IntoPlot`] trait provides a unified interface for all builder types,
//! enabling generic functions to accept any builder:
//!
//! ```rust,ignore
//! fn process_plot(p: impl IntoPlot) {
//!     let plot = p.into_plot();
//!     // ... process the plot
//! }
//! ```

use super::data::{PlotData, ReactiveValue};
use super::types::SeriesStyleProps;
use crate::render::{Color, LineStyle, MarkerStyle};

/// Extension trait providing a generic conditional combinator for fluent builders.
///
/// Implemented for ruviz's consuming builder families rather than as a blanket
/// impl over all types so that `use ruviz::prelude::*` can coexist with other
/// fluent-builder preludes (e.g. GPUI's `FluentBuilder::when`) without method
/// ambiguity.
pub trait BuilderWhen: Sized {
    /// Apply `f` to `self` when `condition` is true; otherwise return `self` unchanged.
    ///
    /// The closure is not invoked when `condition` is false.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::prelude::*;
    ///
    /// let x = vec![0.0, 1.0, 2.0];
    /// let y = vec![0.0, 1.0, 0.5];
    /// let show_label = true;
    ///
    /// let _plot = Plot::new()
    ///     .line(&x, &y)
    ///     .when(show_label, |builder| builder.label("series"))
    ///     .into_plot();
    /// ```
    fn when(self, condition: bool, f: impl FnOnce(Self) -> Self) -> Self {
        if condition { f(self) } else { self }
    }
}

impl BuilderWhen for super::Plot {}
impl<C> BuilderWhen for PlotBuilder<C> where C: crate::plots::PlotConfig + Clone {}
impl BuilderWhen for super::series_builders::SeriesGroupBuilder {}
impl BuilderWhen for crate::core::subplot::SubplotFigure {}
impl BuilderWhen for crate::core::config::PlotConfigBuilder {}
impl BuilderWhen for crate::render::theme::ThemeBuilder {}
#[cfg(all(feature = "interactive", not(target_arch = "wasm32")))]
impl BuilderWhen for crate::interactive::window::InteractiveWindowBuilder {}

/// Trait for types that can be converted to a finalized [`Plot`](super::Plot)
///
/// This trait enables uniform handling of both builder types (`Plot` and
/// `PlotBuilder<C>`) and allows functions to accept any builder generically.
///
/// # When to Use
///
/// Use `IntoPlot` when you want to write a function that accepts any plot builder type:
///
/// ```rust,ignore
/// use ruviz::prelude::*;
///
/// fn save_with_title(builder: impl IntoPlot, title: &str) -> Result<(), PlottingError> {
///     let plot = builder.into_plot().title(title);
///     plot.save("output.png")
/// }
///
/// // Works with Plot
/// save_with_title(Plot::new(), "Direct Plot")?;
///
/// // Works with PlotBuilder
/// save_with_title(Plot::new().kde(&data), "KDE Plot")?;
/// save_with_title(Plot::new().line(&x, &y), "Line Plot")?;
/// ```
///
/// # Relationship to `Into<Plot>`
///
/// All `IntoPlot` implementors also implement `Into<Plot>`. The `IntoPlot` trait
/// provides additional functionality:
/// - `into_plot()`: Explicit conversion method (more discoverable than `.into()`)
/// - `as_plot()`: Read-only access to the inner Plot without consuming the builder
pub trait IntoPlot: Sized {
    /// Consume this builder and return the finalized Plot
    ///
    /// Any pending series configuration is committed before returning.
    fn into_plot(self) -> super::Plot;

    /// Get a reference to the inner Plot
    ///
    /// This allows inspecting the plot configuration without consuming the builder.
    fn as_plot(&self) -> &super::Plot;
}

/// Implementation for Plot itself (identity conversion)
impl IntoPlot for super::Plot {
    fn into_plot(self) -> super::Plot {
        self
    }

    fn as_plot(&self) -> &super::Plot {
        self
    }
}

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
            #[cfg(not(target_arch = "wasm32"))]
            pub fn save<P: AsRef<std::path::Path>>(self, path: P) -> crate::core::Result<()> {
                self.finalize().save(path)
            }

            /// Render the plot to an Image
            ///
            /// Finalizes the series before rendering.
            pub fn render(self) -> crate::core::Result<super::Image> {
                self.finalize().render()
            }

            /// Render the plot to PNG bytes.
            ///
            /// Finalizes the series before rendering.
            pub fn render_png_bytes(self) -> crate::core::Result<Vec<u8>> {
                self.finalize().render_png_bytes()
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
            #[cfg(not(target_arch = "wasm32"))]
            pub fn export_svg<P: AsRef<std::path::Path>>(self, path: P) -> crate::core::Result<()> {
                self.finalize().export_svg(path)
            }

            /// Save to PDF file
            ///
            /// Finalizes the series before saving.
            #[cfg(all(feature = "pdf", not(target_arch = "wasm32")))]
            pub fn save_pdf<P: AsRef<std::path::Path>>(self, path: P) -> crate::core::Result<()> {
                self.finalize().save_pdf(path)
            }

            /// Save to PDF file with an explicit `(width_mm, height_mm)` page size.
            ///
            /// Finalizes the series before saving.
            #[cfg(all(feature = "pdf", not(target_arch = "wasm32")))]
            pub fn save_pdf_with_size<P: AsRef<std::path::Path>>(
                self,
                path: P,
                size: Option<(f64, f64)>,
            ) -> crate::core::Result<()> {
                self.finalize().save_pdf_with_size(path, size)
            }

            /// Save with specific dimensions
            ///
            /// Finalizes the series before saving.
            #[cfg(not(target_arch = "wasm32"))]
            pub fn save_with_size<P: AsRef<std::path::Path>>(
                self,
                path: P,
                width: u32,
                height: u32,
            ) -> crate::core::Result<()> {
                self.finalize().save_with_size(path, width, height)
            }

            impl_series_continuation_methods!(self.finalize());

            // NOTE: `legend_position` deliberately lives on the generic
            // `impl<C> PlotBuilder<C>` block (next to `legend` and `legend_best`)
            // rather than here. Defining it per config type is what let it drift
            // into returning `Plot` instead of `Self` and silently ending the
            // builder chain; a single generic definition makes that impossible.

            /// Finish configuring this series and return to the main Plot
            ///
            /// **Deprecated**: Series finalize automatically. Use `.save()` directly.
            /// Mixed Cartesian/non-Cartesian plots also render through normal
            /// fluent chaining, so this is not needed as a workaround.
            #[deprecated(
                since = "0.1.0",
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

        impl IntoPlot for PlotBuilder<$config> {
            fn into_plot(self) -> super::Plot {
                self.finalize()
            }

            fn as_plot(&self) -> &super::Plot {
                &self.plot
            }
        }
    };
}

macro_rules! impl_inset_builder_methods {
    ($(($config:ty, $series_name:literal)),+ $(,)?) => {
        $(
            impl PlotBuilder<$config> {
                /// Override inset placement for mixed Cartesian/non-Cartesian plots.
                pub fn inset_layout(mut self, layout: super::InsetLayout) -> Self {
                    self.style.inset_layout = Some(layout.normalized());
                    self
                }

                #[doc = concat!(
                    "Set the inset anchor used when this ",
                    $series_name,
                    " is rendered inside a mixed plot."
                )]
                pub fn inset_anchor(mut self, anchor: super::InsetAnchor) -> Self {
                    let mut layout = self.style.inset_layout.unwrap_or_default();
                    layout.anchor = anchor;
                    self.style.inset_layout = Some(layout.normalized());
                    self
                }

                /// Set inset width/height as fractions of the main plot area.
                pub fn inset_size_frac(mut self, width_frac: f32, height_frac: f32) -> Self {
                    let mut layout = self.style.inset_layout.unwrap_or_default();
                    layout.width_frac = width_frac;
                    layout.height_frac = height_frac;
                    self.style.inset_layout = Some(layout.normalized());
                    self
                }

                /// Set inset margin in points.
                pub fn inset_margin_pt(mut self, margin_pt: f32) -> Self {
                    let mut layout = self.style.inset_layout.unwrap_or_default();
                    layout.margin_pt = margin_pt;
                    self.style.inset_layout = Some(layout.normalized());
                    self
                }
            }
        )+
    };
}

/// Generate the four colour-key setters for every plot type that draws one.
///
/// A colour key is the whole reading of a heatmap, a contour, a hexbin or a
/// magnitude-coloured quiver, so all four spell it identically. Writing the
/// setters by hand is what let heatmap and quiver ship with a colorbar their
/// builders could not reach while contour and hexbin exposed two of the four —
/// generating them from one list makes that divergence impossible.
///
/// The four `*Config` types all carry `colorbar: bool`,
/// `colorbar_label: Option<String>`, `colorbar_tick_font_size: Option<f32>` and
/// `colorbar_label_font_size: Option<f32>`; the font sizes are normalised here
/// so one plot type cannot accept a 0 pt caption that another rejects.
macro_rules! impl_colorbar_builder_methods {
    ($(($config:ty, $series_name:literal, $reads:literal)),+ $(,)?) => {
        $(
            impl PlotBuilder<$config> {
                #[doc = concat!("Show or hide the colorbar. It is on by default, because ", $reads, ".")]
                pub fn colorbar(mut self, show: bool) -> Self {
                    self.config.colorbar = show;
                    self
                }

                #[doc = concat!(
                    "Caption for the colorbar — what this ",
                    $series_name,
                    "'s colours are measuring."
                )]
                ///
                /// The caption is drawn rotated a quarter turn beside the bar.
                pub fn colorbar_label<S: Into<String>>(mut self, label: S) -> Self {
                    self.config.colorbar_label = Some(label.into());
                    self
                }

                /// Set the colorbar tick font size, in points.
                ///
                /// Leave it unset to follow the figure's theme.
                pub fn colorbar_tick_font_size(mut self, size: f32) -> Self {
                    self.config.colorbar_tick_font_size = Some(size.max(1.0));
                    self
                }

                /// Set the colorbar caption font size, in points.
                ///
                /// Leave it unset to follow the figure's theme.
                pub fn colorbar_label_font_size(mut self, size: f32) -> Self {
                    self.config.colorbar_label_font_size = Some(size.max(1.0));
                    self
                }
            }
        )+
    };
}

impl_colorbar_builder_methods!(
    (
        crate::plots::heatmap::HeatmapConfig,
        "heatmap",
        "the colour of a cell is the whole reading and nothing else decodes it"
    ),
    (
        crate::plots::ContourConfig,
        "contour",
        "the colour of a band is the whole reading and nothing else decodes it"
    ),
    (
        crate::plots::continuous::hexbin::HexbinConfig,
        "hexbin",
        "the colour of a hexagon is the whole reading and nothing else decodes it"
    ),
    (
        crate::plots::QuiverConfig,
        "quiver",
        "an arrow's colour is the only thing that reports its magnitude"
    ),
);

/// Marker type for plot input data
///
/// This enum captures the different input types that plot series can have.
/// It allows the builder to store the input data generically.
#[derive(Clone, Debug)]
pub enum PlotInput {
    /// Single 1D data array (for KDE, histogram, ECDF, etc.)
    Single(Vec<f64>),
    /// Single 1D data array from a source-backed value (histogram, box plot).
    SingleSource(super::PlotData),
    /// Paired X-Y data (for line, scatter, etc.)
    XY(Vec<f64>, Vec<f64>),
    /// Paired X-Y data from source-backed plot values.
    XYSource(super::PlotData, super::PlotData),
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
    /// Categorical data with source-backed values.
    CategoricalSource {
        categories: Vec<String>,
        values: super::PlotData,
    },
    /// Vector field data for quiver plots.
    Quiver {
        x: Vec<f64>,
        y: Vec<f64>,
        u: Vec<f64>,
        v: Vec<f64>,
    },
    /// X/Y data with optional error magnitudes (error bar series).
    ErrorBars {
        x: super::PlotData,
        y: super::PlotData,
        x_errors: Option<super::PlotData>,
        y_errors: Option<super::PlotData>,
    },
    /// Live X/Y buffer read at render time (streaming line and scatter series).
    Streaming(crate::data::StreamingXY),
    /// A hierarchical clustering result (dendrogram series).
    Linkage(crate::stats::clustering::Linkage),
    /// N named value series over one shared axis (grouped bar, stacked bar,
    /// stacked area). The axis is category names for the bar charts and
    /// numeric x positions for stacked area.
    MultiSeries(super::types::MultiSeriesInput),
}

impl PlotInput {
    /// Count the number of data points in this input
    pub fn point_count(&self) -> usize {
        match self {
            PlotInput::Single(data) => data.len(),
            PlotInput::SingleSource(data) => data.len(),
            PlotInput::XY(x, _) => x.len(),
            PlotInput::XYSource(x, _) => x.len(),
            PlotInput::Grid2D { x, y, .. } => x.len() * y.len(),
            PlotInput::Categorical { values, .. } => values.len(),
            PlotInput::CategoricalSource { values, .. } => values.len(),
            PlotInput::Quiver { x, .. } => x.len(),
            PlotInput::ErrorBars { x, .. } => x.len(),
            PlotInput::Streaming(stream) => stream.len(),
            PlotInput::MultiSeries(input) => input.point_count(),
            // One "point" per merge in the tree, which is what the renderer draws.
            PlotInput::Linkage(linkage) => linkage.matrix.len(),
        }
    }
}

/// Resolve an X/Y-shaped [`PlotInput`] into the pair of [`PlotData`] lanes that
/// line and scatter series render from.
///
/// Streaming input is the same series with a live buffer attached, so the buffer
/// is recorded on `style` here rather than in a second series constructor — that
/// keeps `line_streaming` and `line` on exactly one code path.
fn resolve_xy_input(input: &PlotInput, style: &mut SeriesStyle) -> (PlotData, PlotData) {
    match input {
        PlotInput::XY(x, y) => (PlotData::Static(x.clone()), PlotData::Static(y.clone())),
        PlotInput::XYSource(x, y) => (x.clone(), y.clone()),
        PlotInput::Single(y) => {
            // Generate x values as indices
            let x: Vec<f64> = (0..y.len()).map(|i| i as f64).collect();
            (PlotData::Static(x), PlotData::Static(y.clone()))
        }
        PlotInput::Streaming(stream) => {
            style.streaming_source = Some(stream.clone());
            (
                PlotData::Streaming(stream.x().clone()),
                PlotData::Streaming(stream.y().clone()),
            )
        }
        _ => (PlotData::Static(vec![]), PlotData::Static(vec![])),
    }
}

fn quiver_length_mismatch(
    x: &[f64],
    y: &[f64],
    u: &[f64],
    v: &[f64],
) -> Option<crate::core::PlottingError> {
    [y.len(), u.len(), v.len()]
        .into_iter()
        .find(|&len| len != x.len())
        .map(|len| crate::core::PlottingError::DataLengthMismatch {
            x_len: x.len(),
            y_len: len,
            series_index: None,
        })
}

fn validate_quiver_input(x: &[f64], y: &[f64], u: &[f64], v: &[f64]) -> crate::core::Result<()> {
    if let Some(err) = quiver_length_mismatch(x, y, u, v) {
        return Err(err);
    }

    crate::core::PlottingError::validate_data(x)?;
    crate::core::PlottingError::validate_data(y)?;
    crate::core::PlottingError::validate_data(u)?;
    crate::core::PlottingError::validate_data(v)?;
    Ok(())
}

/// Style options for a series
///
/// These are common styling options that apply to most plot types.
#[derive(Clone, Debug, Default)]
pub struct SeriesStyle {
    /// Series label for legend
    pub label: Option<String>,
    /// The reactive style properties, declared once by
    /// [`series_style_properties!`](super::types::SeriesStyleProps).
    ///
    /// `PlotSeries` holds the same type, and building a series moves this
    /// straight across — so the two cannot come to disagree about what a
    /// property is or what setting it means.
    pub(crate) props: SeriesStyleProps,
    /// Y-axis error bar values
    pub y_errors: Option<crate::plots::error::ErrorValues>,
    /// X-axis error bar values
    pub x_errors: Option<crate::plots::error::ErrorValues>,
    /// Error bar styling configuration
    pub error_config: Option<crate::plots::error::ErrorBarConfig>,
    /// Inset placement for non-Cartesian series in mixed plots.
    pub inset_layout: Option<super::InsetLayout>,
    /// Live buffer this series re-reads at render time (streaming series only).
    pub streaming_source: Option<crate::data::StreamingXY>,
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
        self.style.props.color.set(color.into());
        self
    }

    /// Set a reactive series color source.
    pub fn color_source<S>(mut self, color: S) -> Self
    where
        S: Into<ReactiveValue<Color>>,
    {
        self.style.props.color.set(color.into());
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
        self.style.props.line_width.set(width.into());
        self
    }

    /// Set a reactive line width source.
    pub fn line_width_source<S>(mut self, width: S) -> Self
    where
        S: Into<ReactiveValue<f32>>,
    {
        self.style.props.line_width.set(width.into());
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
        self.style.props.line_style.set(style.into());
        self
    }

    /// Set a reactive line style source.
    pub fn line_style_source<S>(mut self, style: S) -> Self
    where
        S: Into<ReactiveValue<LineStyle>>,
    {
        self.style.props.line_style.set(style.into());
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
        self.style.props.alpha.set(alpha.into());
        self
    }

    /// Set a reactive alpha/transparency source.
    pub fn alpha_source<S>(mut self, alpha: S) -> Self
    where
        S: Into<ReactiveValue<f32>>,
    {
        self.style.props.alpha.set(alpha.into());
        self
    }

    // ===== Error bar methods =====

    /// Attach symmetric Y error bars to this series
    ///
    /// # Arguments
    /// * `errors` - Error values (same magnitude for +/-)
    pub fn with_yerr<E: crate::data::NumericData1D>(mut self, errors: &E) -> Self {
        match crate::data::collect_numeric_data_1d(errors, self.plot.null_policy) {
            Ok(values) => {
                self.style.y_errors = Some(crate::plots::error::ErrorValues::symmetric(values));
            }
            Err(err) => {
                self.plot.set_pending_ingestion_error(err);
            }
        }
        self
    }

    /// Attach symmetric X error bars to this series
    ///
    /// # Arguments
    /// * `errors` - Error values (same magnitude for +/-)
    pub fn with_xerr<E: crate::data::NumericData1D>(mut self, errors: &E) -> Self {
        match crate::data::collect_numeric_data_1d(errors, self.plot.null_policy) {
            Ok(values) => {
                self.style.x_errors = Some(crate::plots::error::ErrorValues::symmetric(values));
            }
            Err(err) => {
                self.plot.set_pending_ingestion_error(err);
            }
        }
        self
    }

    /// Attach asymmetric Y error bars to this series
    ///
    /// # Arguments
    /// * `lower` - Lower error values (extending downward)
    /// * `upper` - Upper error values (extending upward)
    pub fn with_yerr_asymmetric<E1, E2>(mut self, lower: &E1, upper: &E2) -> Self
    where
        E1: crate::data::NumericData1D,
        E2: crate::data::NumericData1D,
    {
        let lower_values = crate::data::collect_numeric_data_1d(lower, self.plot.null_policy);
        let upper_values = crate::data::collect_numeric_data_1d(upper, self.plot.null_policy);
        match (lower_values, upper_values) {
            (Ok(lower), Ok(upper)) => {
                self.style.y_errors =
                    Some(crate::plots::error::ErrorValues::asymmetric(lower, upper));
            }
            (Err(err), _) | (_, Err(err)) => {
                self.plot.set_pending_ingestion_error(err);
            }
        }
        self
    }

    /// Attach asymmetric X error bars to this series
    ///
    /// # Arguments
    /// * `lower` - Lower error values (extending left)
    /// * `upper` - Upper error values (extending right)
    pub fn with_xerr_asymmetric<E1, E2>(mut self, lower: &E1, upper: &E2) -> Self
    where
        E1: crate::data::NumericData1D,
        E2: crate::data::NumericData1D,
    {
        let lower_values = crate::data::collect_numeric_data_1d(lower, self.plot.null_policy);
        let upper_values = crate::data::collect_numeric_data_1d(upper, self.plot.null_policy);
        match (lower_values, upper_values) {
            (Ok(lower), Ok(upper)) => {
                self.style.x_errors =
                    Some(crate::plots::error::ErrorValues::asymmetric(lower, upper));
            }
            (Err(err), _) | (_, Err(err)) => {
                self.plot.set_pending_ingestion_error(err);
            }
        }
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
    pub fn title(mut self, title: impl Into<super::PlotText>) -> Self {
        self.plot = self.plot.title(title);
        self
    }

    /// Set X-axis label
    ///
    /// This method forwards to the inner Plot.
    pub fn xlabel(mut self, label: impl Into<super::PlotText>) -> Self {
        self.plot = self.plot.xlabel(label);
        self
    }

    /// Set Y-axis label
    ///
    /// This method forwards to the inner Plot.
    pub fn ylabel(mut self, label: impl Into<super::PlotText>) -> Self {
        self.plot = self.plot.ylabel(label);
        self
    }

    /// Set null handling policy for dataframe-backed numeric inputs.
    pub fn null_policy(mut self, policy: crate::data::NullPolicy) -> Self {
        self.plot = self.plot.null_policy(policy);
        self
    }

    /// Enable legend with automatic best position
    ///
    /// Equivalent to `legend(LegendPosition::Best)`. Available on every
    /// `PlotBuilder<C>`, exactly like [`legend`](Self::legend) and
    /// [`legend_position`](Self::legend_position).
    ///
    /// This method forwards to the inner Plot.
    pub fn legend_best(mut self) -> Self {
        self.plot = self.plot.legend_best();
        self
    }

    /// Enable legend at a specific position
    ///
    /// Accepts a [`LegendPosition`](crate::core::LegendPosition) (canonical) or
    /// the deprecated [`Position`](crate::core::Position), which converts
    /// losslessly. This method forwards to the inner Plot.
    pub fn legend(mut self, position: impl Into<crate::core::LegendPosition>) -> Self {
        self.plot = self.plot.legend(position);
        self
    }

    /// Enable legend at a specific position
    ///
    /// Long-form spelling of [`legend`](Self::legend), matching
    /// [`Plot::legend_position`](super::Plot::legend_position) so the same call
    /// works before and after a series method. Accepts a
    /// [`LegendPosition`](crate::core::LegendPosition) (canonical) or the
    /// deprecated [`Position`](crate::core::Position), which converts losslessly.
    ///
    /// Returns `Self`, so the chain continues into further series or config
    /// calls:
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let x = vec![0.0, 1.0, 2.0];
    /// let y = vec![0.0, 1.0, 0.5];
    ///
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .legend_position(LegendPosition::UpperRight)
    ///     .line_width(2.0) // still on the builder
    ///     .save("legend.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn legend_position(mut self, position: impl Into<crate::core::LegendPosition>) -> Self {
        self.plot = self.plot.legend_position(position.into());
        self
    }

    /// Set legend font size in typographic points.
    ///
    /// This method forwards to the inner Plot.
    pub fn legend_font_size(mut self, size: f32) -> Self {
        self.plot = self.plot.legend_font_size(size);
        self
    }

    /// Set legend corner radius for rounded corners
    ///
    /// This method forwards to the inner Plot.
    pub fn legend_corner_radius(mut self, radius: f32) -> Self {
        self.plot = self.plot.legend_corner_radius(radius);
        self
    }

    /// Set number of legend columns
    ///
    /// This method forwards to the inner Plot.
    pub fn legend_columns(mut self, columns: usize) -> Self {
        self.plot = self.plot.legend_columns(columns);
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

    /// Trade exactness for speed on large data.
    ///
    /// This method forwards to the inner Plot. See [`super::Plot::fast`] for
    /// what fast mode substitutes and its documented limitations.
    pub fn fast(mut self, enabled: bool) -> Self {
        self.plot = self.plot.fast(enabled);
        self
    }

    /// Set the font family used for plot text.
    ///
    /// This method forwards to the inner Plot.
    pub fn font_family<F>(mut self, family: F) -> Self
    where
        F: Into<crate::render::FontFamily>,
    {
        self.plot = self.plot.font_family(family);
        self
    }

    /// Set maximum output resolution while preserving figure aspect ratio
    ///
    /// This method forwards to the inner Plot. See [`super::Plot::max_resolution`] for details.
    pub fn max_resolution(mut self, max_width: u32, max_height: u32) -> Self {
        self.plot = self.plot.max_resolution(max_width, max_height);
        self
    }

    /// Set X-axis limits
    ///
    /// This method forwards to the inner Plot. Descending bounds preserve a
    /// reversed axis direction.
    pub fn xlim(mut self, min: f64, max: f64) -> Self {
        self.plot = self.plot.xlim(min, max);
        self
    }

    /// Set Y-axis limits
    ///
    /// This method forwards to the inner Plot. Descending bounds preserve a
    /// reversed axis direction.
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

    /// Enable or disable tick marks and tick labels.
    ///
    /// This method forwards to the inner Plot.
    pub fn ticks(mut self, enabled: bool) -> Self {
        self.plot = self.plot.ticks(enabled);
        self
    }

    /// Set tick direction to inside.
    ///
    /// This method forwards to the inner Plot.
    pub fn tick_direction_inside(mut self) -> Self {
        self.plot = self.plot.tick_direction_inside();
        self
    }

    /// Set tick direction to outside.
    ///
    /// This method forwards to the inner Plot.
    pub fn tick_direction_outside(mut self) -> Self {
        self.plot = self.plot.tick_direction_outside();
        self
    }

    /// Set tick direction to straddle the plot border.
    ///
    /// This method forwards to the inner Plot.
    pub fn tick_direction_inout(mut self) -> Self {
        self.plot = self.plot.tick_direction_inout();
        self
    }

    /// Set which plot borders render tick marks.
    ///
    /// This method forwards to the inner Plot.
    pub fn tick_sides(mut self, sides: crate::core::TickSides) -> Self {
        self.plot = self.plot.tick_sides(sides);
        self
    }

    /// How the x tick labels are oriented when they would collide.
    ///
    /// This method forwards to the inner Plot.
    pub fn xtick_rotation(mut self, rotation: crate::render::XTickRotation) -> Self {
        self.plot = self.plot.xtick_rotation(rotation);
        self
    }

    /// Show ticks on all four sides.
    ///
    /// This method forwards to the inner Plot.
    pub fn ticks_all_sides(mut self) -> Self {
        self.plot = self.plot.ticks_all_sides();
        self
    }

    /// Show ticks only on the bottom and left sides.
    ///
    /// This method forwards to the inner Plot.
    pub fn ticks_bottom_left(mut self) -> Self {
        self.plot = self.plot.ticks_bottom_left();
        self
    }

    /// Enable or disable top ticks.
    ///
    /// This method forwards to the inner Plot.
    pub fn show_top_ticks(mut self, enabled: bool) -> Self {
        self.plot = self.plot.show_top_ticks(enabled);
        self
    }

    /// Enable or disable bottom ticks.
    ///
    /// This method forwards to the inner Plot.
    pub fn show_bottom_ticks(mut self, enabled: bool) -> Self {
        self.plot = self.plot.show_bottom_ticks(enabled);
        self
    }

    /// Enable or disable left ticks.
    ///
    /// This method forwards to the inner Plot.
    pub fn show_left_ticks(mut self, enabled: bool) -> Self {
        self.plot = self.plot.show_left_ticks(enabled);
        self
    }

    /// Enable or disable right ticks.
    ///
    /// This method forwards to the inner Plot.
    pub fn show_right_ticks(mut self, enabled: bool) -> Self {
        self.plot = self.plot.show_right_ticks(enabled);
        self
    }

    /// Enable or disable Typst text rendering mode.
    ///
    /// This method forwards to the inner Plot.
    ///
    /// Requires the `typst-math` feature.
    /// If your crate makes Typst optional, guard this call with
    /// `#[cfg(feature = "typst-math")]`.
    #[cfg(feature = "typst-math")]
    #[cfg_attr(docsrs, doc(cfg(feature = "typst-math")))]
    pub fn typst(mut self, enabled: bool) -> Self {
        self.plot = self.plot.typst(enabled);
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

    /// Store a GPU backend preference.
    ///
    /// This method forwards to the inner Plot. Public raster operations currently
    /// resolve the preference to Skia with an inspectable fallback reason.
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

    /// Add an arrow annotation with custom styling
    ///
    /// This method forwards to the inner Plot.
    pub fn arrow_styled(
        mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        style: crate::core::ArrowStyle,
    ) -> Self {
        self.plot = self.plot.arrow_styled(x1, y1, x2, y2, style);
        self
    }

    /// Add a text annotation
    ///
    /// This method forwards to the inner Plot.
    pub fn text<S: Into<String>>(mut self, x: f64, y: f64, text: S) -> Self {
        self.plot = self.plot.text(x, y, text);
        self
    }

    /// Add a text annotation with custom styling
    ///
    /// This method forwards to the inner Plot.
    pub fn text_styled<S: Into<String>>(
        mut self,
        x: f64,
        y: f64,
        text: S,
        style: crate::core::TextStyle,
    ) -> Self {
        self.plot = self.plot.text_styled(x, y, text, style);
        self
    }

    /// Add a horizontal reference line
    ///
    /// This method forwards to the inner Plot.
    pub fn hline(mut self, y: f64) -> Self {
        self.plot = self.plot.hline(y);
        self
    }

    /// Add a horizontal reference line with custom styling
    ///
    /// This method forwards to the inner Plot.
    pub fn hline_styled(mut self, y: f64, color: Color, width: f32, style: LineStyle) -> Self {
        self.plot = self.plot.hline_styled(y, color, width, style);
        self
    }

    /// Add a vertical reference line
    ///
    /// This method forwards to the inner Plot.
    pub fn vline(mut self, x: f64) -> Self {
        self.plot = self.plot.vline(x);
        self
    }

    /// Add a vertical reference line with custom styling
    ///
    /// This method forwards to the inner Plot.
    pub fn vline_styled(mut self, x: f64, color: Color, width: f32, style: LineStyle) -> Self {
        self.plot = self.plot.vline_styled(x, color, width, style);
        self
    }

    /// Add a rectangle annotation
    ///
    /// This method forwards to the inner Plot.
    pub fn rect(mut self, x: f64, y: f64, width: f64, height: f64) -> Self {
        self.plot = self.plot.rect(x, y, width, height);
        self
    }

    /// Add a rectangle annotation with custom styling
    ///
    /// This method forwards to the inner Plot.
    pub fn rect_styled(
        mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        style: crate::core::ShapeStyle,
    ) -> Self {
        self.plot = self.plot.rect_styled(x, y, width, height, style);
        self
    }

    /// Add a fill between two curves
    ///
    /// This method forwards to the inner Plot.
    pub fn fill_between(mut self, x: &[f64], y1: &[f64], y2: &[f64]) -> Self {
        self.plot = self.plot.fill_between(x, y1, y2);
        self
    }

    /// Add a fill between a curve and a baseline
    ///
    /// This method forwards to the inner Plot.
    pub fn fill_to_baseline(mut self, x: &[f64], y: &[f64], baseline: f64) -> Self {
        self.plot = self.plot.fill_to_baseline(x, y, baseline);
        self
    }

    /// Add a styled fill between two curves
    ///
    /// This method forwards to the inner Plot.
    pub fn fill_between_styled(
        mut self,
        x: &[f64],
        y1: &[f64],
        y2: &[f64],
        style: crate::core::FillStyle,
        where_positive: bool,
    ) -> Self {
        self.plot = self
            .plot
            .fill_between_styled(x, y1, y2, style, where_positive);
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
    /// Set the bandwidth selection method.
    ///
    /// Bandwidth controls the smoothness of the density estimate. Defaults to
    /// Scott's rule. Takes the same argument as
    /// [`PlotBuilder::<ViolinConfig>::bandwidth`](PlotBuilder::bandwidth):
    /// a [`BandwidthMethod`](crate::plots::BandwidthMethod), or a bare number
    /// for a fixed bandwidth.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .kde(&data)
    ///     .bandwidth(0.5)                            // fixed
    ///     .save("kde.png")?;
    ///
    /// Plot::new()
    ///     .kde(&data)
    ///     .bandwidth(BandwidthMethod::Silverman)     // rule
    ///     .save("kde.png")?;
    /// ```
    pub fn bandwidth(mut self, bw: impl Into<crate::plots::BandwidthMethod>) -> Self {
        self.config.bandwidth = bw.into();
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
    #[deprecated(
        since = "0.6.0",
        note = "use `line_width`, the spelling shared by every builder; unlike `kde_line_width` it is honoured by every backend"
    )]
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
    #[deprecated(
        since = "0.6.0",
        note = "use `line_width`, the spelling shared by every builder; unlike `ecdf_line_width` it is honoured by every backend"
    )]
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

    /// Set the colormap.
    ///
    /// Accepts a name such as `"viridis"` or a [`ColorMap`](crate::render::ColorMap) value.
    pub fn cmap(mut self, cmap: impl Into<crate::render::ColorMapSpec>) -> Self {
        self.config.cmap = cmap.into().into_name();
        self
    }

    /// Set colormap by name (e.g., "viridis", "plasma", "magma")
    #[deprecated(
        since = "0.6.0",
        note = "renamed: use `cmap(name)`, which also accepts a `ColorMap` value"
    )]
    pub fn colormap_name(self, name: &str) -> Self {
        self.cmap(name)
    }

    /// Set contour line width
    #[deprecated(
        since = "0.6.0",
        note = "use `line_width`, the spelling shared by every builder; unlike `contour_line_width` it is honoured by every backend"
    )]
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

    // `colorbar`, `colorbar_label`, `colorbar_tick_font_size` and
    // `colorbar_label_font_size` come from `impl_colorbar_builder_methods!`,
    // which every plot type that draws a colour key shares.

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

impl_inset_builder_methods!(
    (crate::plots::PieConfig, "pie chart"),
    (crate::plots::RadarConfig, "radar chart"),
    (crate::plots::PolarPlotConfig, "polar plot"),
);

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

    /// Set slice label font size in typographic points
    ///
    /// Named for the thing it changes, matching the neighbouring
    /// [`label_distance`](Self::label_distance) and
    /// [`show_labels`](Self::show_labels), and distinct from the plot-wide
    /// `Plot::font_size` and [`legend_font_size`](Self::legend_font_size).
    pub fn label_font_size(mut self, size: f32) -> Self {
        self.config.label_font_size = size;
        self
    }

    /// Set label font size
    #[deprecated(
        since = "0.6.0",
        note = "ambiguous against the plot-wide `Plot::font_size`: use `label_font_size`, which sets the pie slice label size"
    )]
    pub fn font_size(self, size: f32) -> Self {
        self.label_font_size(size)
    }

    /// Set label distance from center (as fraction of radius)
    pub fn label_distance(mut self, distance: f64) -> Self {
        self.config.label_distance = distance;
        self
    }

    /// Sweep the wedges clockwise from the start angle (the default)
    pub fn clockwise(mut self) -> Self {
        self.config.counter_clockwise = false;
        self
    }

    /// Sweep the wedges counter-clockwise from the start angle, as matplotlib does
    pub fn counter_clockwise(mut self) -> Self {
        self.config.counter_clockwise = true;
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
        // Contiguous storage copies in one memcpy; anything else keeps the
        // per-element path. Same values either way.
        let values_vec: Vec<f64> = match values.as_slice() {
            Some(slice) => slice.to_vec(),
            None => (0..values.len())
                .filter_map(|i| values.get(i).copied())
                .collect(),
        };

        // Capture any pending label from the previous .label() call for the PREVIOUS series
        // Pattern: .series([...]).label("A").series([...]).label("B")
        // When the second .series() is called, we capture "A" for the first series
        if let Some(label) = self.style.label.take()
            && let Some(last) = self.config.series_labels.last_mut()
            && last.is_empty()
        {
            *last = label;
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
    pub fn series_label<S: Into<String>>(mut self, name: S) -> Self {
        let name = name.into();
        // Update the label for the most recently added series
        if let Some(last) = self.config.series_labels.last_mut() {
            last.clone_from(&name);
        }
        // Also update the style label for backward compatibility
        self.style.label = Some(name);
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
    ///         .series_color(Color::RED)
    ///         .series_fill_alpha(0.4)
    ///     .add_series("Series 2", &[3.0, 2.0, 1.0])
    ///         .series_color(Color::BLUE)
    ///     .save("styled.png")?;
    /// ```
    pub fn add_series<S, V>(mut self, name: S, values: &V) -> Self
    where
        S: Into<String>,
        V: crate::data::Data1D<f64>,
    {
        // Contiguous storage copies in one memcpy; anything else keeps the
        // per-element path. Same values either way.
        let values_vec: Vec<f64> = match values.as_slice() {
            Some(slice) => slice.to_vec(),
            None => (0..values.len())
                .filter_map(|i| values.get(i).copied())
                .collect(),
        };

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

    /// Set the colour of the current (most recently added) radar series
    ///
    /// Applies to the series added by the most recent `add_series()` call, and
    /// pairs with [`series_label`](Self::series_label). If no series has been
    /// added, this is a no-op.
    ///
    /// Not to be confused with the chart-wide [`color`](Self::color), which sets
    /// the whole radar series' base colour.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .radar(&["A", "B", "C"])
    ///     .add_series("Red Series", &[1.0, 2.0, 3.0])
    ///         .series_color(Color::RED)
    ///     .save("red.png")?;
    /// ```
    pub fn series_color(mut self, color: Color) -> Self {
        if let Some(idx) = self.config.current_series_idx
            && let Some(ref mut colors) = self.config.colors
            && let Some(c) = colors.get_mut(idx)
        {
            *c = color;
        }
        self
    }

    /// Set color for the current (most recently added) series
    #[deprecated(
        since = "0.6.0",
        note = "renamed for consistency with `series_label`: use `series_color`"
    )]
    pub fn with_color(self, color: Color) -> Self {
        self.series_color(color)
    }

    /// Set the fill alpha of the current (most recently added) radar series
    ///
    /// Applies to the series added by the most recent `add_series()` call.
    /// Values range from 0.0 (transparent) to 1.0 (opaque). If no series has
    /// been added, this is a no-op.
    ///
    /// Not to be confused with the chart-wide
    /// [`fill_alpha`](Self::fill_alpha).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .radar(&["A", "B", "C"])
    ///     .add_series("Transparent", &[1.0, 2.0, 3.0])
    ///         .series_fill_alpha(0.2)
    ///     .save("transparent.png")?;
    /// ```
    pub fn series_fill_alpha(mut self, alpha: f32) -> Self {
        if let Some(idx) = self.config.current_series_idx
            && let Some(a) = self.config.per_series_fill_alphas.get_mut(idx)
        {
            *a = Some(alpha.clamp(0.0, 1.0));
        }
        self
    }

    /// Set fill alpha for the current (most recently added) series
    #[deprecated(
        since = "0.6.0",
        note = "renamed for consistency with `series_label`: use `series_fill_alpha`"
    )]
    pub fn with_fill_alpha(self, alpha: f32) -> Self {
        self.series_fill_alpha(alpha)
    }

    /// Set the line width of the current (most recently added) radar series
    ///
    /// Applies to the series added by the most recent `add_series()` call, in
    /// typographic points. If no series has been added, this is a no-op.
    ///
    /// Not to be confused with the chart-wide
    /// [`radar_line_width`](Self::radar_line_width).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .radar(&["A", "B", "C"])
    ///     .add_series("Thick Lines", &[1.0, 2.0, 3.0])
    ///         .series_line_width(3.0)
    ///     .save("thick.png")?;
    /// ```
    pub fn series_line_width(mut self, width: f32) -> Self {
        if let Some(idx) = self.config.current_series_idx
            && let Some(w) = self.config.per_series_line_widths.get_mut(idx)
        {
            *w = Some(width.max(0.1));
        }
        self
    }

    /// Set line width for the current (most recently added) series
    #[deprecated(
        since = "0.6.0",
        note = "renamed for consistency with `series_label`: use `series_line_width`"
    )]
    pub fn with_line_width(self, width: f32) -> Self {
        self.series_line_width(width)
    }

    /// Set the chart-wide fill alpha
    ///
    /// Values range from 0.0 (transparent) to 1.0 (opaque). For a single
    /// series, use [`series_fill_alpha`](Self::series_fill_alpha).
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
        if let Some(label) = self.style.label.take()
            && let Some(last) = self.config.series_labels.last_mut()
            && last.is_empty()
        {
            *last = label;
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

    /// Show/hide the concentric rings of the polar grid.
    pub fn show_rgrid(mut self, show: bool) -> Self {
        self.config.show_rgrid = show;
        self
    }

    /// Show/hide the radial spokes of the polar grid.
    pub fn show_thetagrid(mut self, show: bool) -> Self {
        self.config.show_thetagrid = show;
        self
    }

    /// How many concentric rings — and radial labels — the grid has.
    pub fn rgrid_count(mut self, count: usize) -> Self {
        self.config.rgrid_count = count;
        self
    }

    /// How many spokes — and angular labels — the grid has.
    pub fn thetagrid_count(mut self, count: usize) -> Self {
        self.config.thetagrid_count = count;
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

    /// Set the violin body width, in data units
    ///
    /// This is the maximum width of the density silhouette measured on the
    /// category axis, *not* a fraction of the category slot — that is
    /// `box_width` on the boxen builder.
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .violin(&data)
    ///     .violin_width(0.8)
    ///     .save("violin.png")?;
    /// ```
    pub fn violin_width(mut self, width: f64) -> Self {
        self.config.width = width.max(0.1);
        self
    }

    /// Set violin width
    #[deprecated(
        since = "0.6.0",
        note = "ambiguous: `width` means something different on every plot type. Use `violin_width`, which sets the violin body width in data units"
    )]
    pub fn width(self, width: f64) -> Self {
        self.violin_width(width)
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

    /// Choose how the KDE bandwidth is selected.
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .violin(&data)
    ///     .bandwidth(BandwidthMethod::Silverman)
    ///     .save("violin.png")?;
    /// ```
    pub fn bandwidth(mut self, method: impl Into<crate::plots::BandwidthMethod>) -> Self {
        self.config.bandwidth = method.into();
        self
    }

    // `.category(..)` and `.x_position(..)` are not written here. They come from
    // `impl_category_axis!` in series_internal.rs, which emits them for every
    // categorical plot type at once — a hand-written copy here is exactly how
    // violin ended up with a `.category()` that box plots and boxen plots did
    // not have.

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

// =============================================================================
// Boxen Plot Builder
// =============================================================================

impl PlotBuilder<crate::plots::BoxenConfig> {
    /// Set the maximum number of letter-value levels to draw.
    pub fn k_depth(mut self, k: usize) -> Self {
        self.config.k_depth = Some(k.max(1));
        self
    }

    /// Set the box width as a fraction of category spacing (0.1 – 1.0)
    ///
    /// A *fraction*, not data units — the violin builder's `violin_width` is
    /// the data-unit knob.
    pub fn box_width(mut self, width: f64) -> Self {
        self.config.width = width.clamp(0.1, 1.0);
        self
    }

    /// Set the box width as a fraction of category spacing.
    #[deprecated(
        since = "0.6.0",
        note = "ambiguous: `width` means something different on every plot type. Use `box_width`, which sets the box width as a fraction of category spacing"
    )]
    pub fn width(self, width: f64) -> Self {
        self.box_width(width)
    }

    /// Set the saturation gradient used across nested boxes.
    pub fn saturation(mut self, saturation: f32) -> Self {
        self.config.saturation = saturation.clamp(0.0, 1.0);
        self
    }

    /// Show or hide outlier markers outside the outermost box.
    pub fn show_outliers(mut self, show: bool) -> Self {
        self.config.show_outliers = show;
        self
    }

    /// Set the outlier marker size in points.
    pub fn outlier_size(mut self, size: f32) -> Self {
        self.config.outlier_size = size.max(0.0);
        self
    }

    /// Set the box edge line width in points.
    pub fn edge_width(mut self, width: f32) -> Self {
        self.config.line_width = width.max(0.0);
        self
    }

    /// Set horizontal orientation.
    pub fn horizontal(mut self) -> Self {
        self.config.orient = crate::plots::distribution::BoxenOrientation::Horizontal;
        self
    }

    /// Set vertical orientation.
    pub fn vertical(mut self) -> Self {
        self.config.orient = crate::plots::distribution::BoxenOrientation::Vertical;
        self
    }

    /// Finalize the boxen series and add it to the plot.
    fn finalize(self) -> super::Plot {
        let data = match &self.input {
            PlotInput::Single(data) => data.clone(),
            _ => Vec::new(),
        };

        let boxen_data = crate::plots::compute_boxen(&data, &self.config);
        if boxen_data.boxes.is_empty() {
            let mut plot = self.plot;
            plot.set_pending_ingestion_error(crate::core::PlottingError::EmptyDataSet);
            plot
        } else {
            self.plot.add_boxen_series(boxen_data, self.style)
        }
    }
}

impl_terminal_methods!(crate::plots::BoxenConfig);

// =============================================================================
// Quiver Plot Builder
// =============================================================================

impl PlotBuilder<crate::plots::QuiverConfig> {
    /// Set the scale factor applied to arrow lengths.
    ///
    /// Part of the `arrow_*` family: [`arrow_scale`](Self::arrow_scale),
    /// [`arrow_width`](Self::arrow_width),
    /// [`arrow_head_length`](Self::arrow_head_length),
    /// [`arrow_head_width`](Self::arrow_head_width).
    pub fn arrow_scale(mut self, scale: f64) -> Self {
        self.config.scale = scale.max(0.0);
        self
    }

    /// Set the scale factor applied to arrow lengths.
    #[deprecated(
        since = "0.6.0",
        note = "ambiguous against the axis-scale setters `xscale`/`yscale`: use `arrow_scale`, which scales arrow lengths"
    )]
    pub fn scale(self, scale: f64) -> Self {
        self.arrow_scale(scale)
    }

    /// Set the arrow shaft stroke width, in typographic points.
    ///
    /// A stroke width — unlike the violin builder's data-unit `violin_width`
    /// or the boxen builder's fractional `box_width`.
    pub fn arrow_width(mut self, width: f32) -> Self {
        self.config.width = width.max(0.1);
        self
    }

    /// Set the arrow stroke width in points.
    #[deprecated(
        since = "0.6.0",
        note = "ambiguous: `width` means something different on every plot type. Use `arrow_width`, which sets the arrow shaft stroke width in points"
    )]
    pub fn width(self, width: f32) -> Self {
        self.arrow_width(width)
    }

    /// Set the arrow head length as a fraction of arrow length.
    pub fn arrow_head_length(mut self, headlength: f64) -> Self {
        self.config.headlength = headlength.max(0.0);
        self
    }

    /// Set the arrow head length as a fraction of arrow length.
    #[deprecated(
        since = "0.6.0",
        note = "renamed for consistency with the rest of the `arrow_*` family: use `arrow_head_length`"
    )]
    pub fn headlength(self, headlength: f64) -> Self {
        self.arrow_head_length(headlength)
    }

    /// Set the arrow head width as a fraction of arrow length.
    pub fn arrow_head_width(mut self, headwidth: f64) -> Self {
        self.config.headwidth = headwidth.max(0.0);
        self
    }

    /// Set the arrow head width as a fraction of arrow length.
    #[deprecated(
        since = "0.6.0",
        note = "renamed for consistency with the rest of the `arrow_*` family: use `arrow_head_width`"
    )]
    pub fn headwidth(self, headwidth: f64) -> Self {
        self.arrow_head_width(headwidth)
    }

    /// Interpret `u` as angles in radians and `v` as magnitudes.
    pub fn angles_mode(mut self, enabled: bool) -> Self {
        self.config.angles_mode = enabled;
        self
    }

    /// Set the point on each arrow anchored at `(x, y)`.
    pub fn pivot(mut self, pivot: crate::plots::QuiverPivot) -> Self {
        self.config.pivot = pivot;
        self
    }

    /// Color arrows by vector magnitude using the configured colormap.
    pub fn color_by_magnitude(mut self, enabled: bool) -> Self {
        self.config.color_by_magnitude = enabled;
        self
    }

    /// Set the colormap used when coloring arrows by magnitude.
    ///
    /// Accepts a name such as `"viridis"` or a [`ColorMap`](crate::render::ColorMap) value.
    pub fn cmap(mut self, cmap: impl Into<crate::render::ColorMapSpec>) -> Self {
        self.config.cmap = cmap.into().into_name();
        self
    }

    /// Finalize the quiver series and add it to the plot.
    fn finalize(self) -> super::Plot {
        if self.plot.pending_ingestion_error().is_some() {
            return self.plot;
        }

        let (x, y, u, v) = match &self.input {
            PlotInput::Quiver { x, y, u, v } => (x.clone(), y.clone(), u.clone(), v.clone()),
            _ => return self.plot,
        };

        if let Err(err) = validate_quiver_input(&x, &y, &u, &v) {
            let mut plot = self.plot;
            plot.set_pending_ingestion_error(err);
            return plot;
        }

        let quiver_data = crate::plots::compute_quiver(&x, &y, &u, &v, &self.config);
        self.plot.add_quiver_series(quiver_data, self.style)
    }
}

impl_terminal_methods!(crate::plots::QuiverConfig);

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
        self.style.props.marker_style.set(style.into());
        self
    }

    /// Set a reactive marker style.
    pub fn marker_source<S>(mut self, style: S) -> Self
    where
        S: Into<ReactiveValue<crate::render::MarkerStyle>>,
    {
        self.config.show_markers = true;
        self.style.props.marker_style.set(style.into());
        self
    }

    /// Set marker size
    ///
    /// # Arguments
    /// * `size` - Marker size in points (default: 6.0)
    pub fn marker_size(mut self, size: f32) -> Self {
        self.config.marker_size = size.max(0.1);
        self.style.props.marker_size.set(size.into());
        self
    }

    /// Set a reactive marker size.
    pub fn marker_size_source<S>(mut self, size: S) -> Self
    where
        S: Into<ReactiveValue<f32>>,
    {
        self.style.props.marker_size.set(size.into());
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
    /// Exactly equivalent to [`line_style`](Self::line_style), which is the
    /// spelling every builder shares.
    #[deprecated(
        since = "0.6.0",
        note = "ambiguous short alias: use `line_style`, the spelling shared by every builder"
    )]
    pub fn style(self, line_style: crate::render::LineStyle) -> Self {
        self.line_style(line_style)
    }

    /// Set a reactive line style.
    #[deprecated(
        since = "0.6.0",
        note = "ambiguous short alias: use `line_style_source`, the spelling shared by every builder"
    )]
    pub fn style_source<S>(self, line_style: S) -> Self
    where
        S: Into<ReactiveValue<crate::render::LineStyle>>,
    {
        self.line_style_source(line_style)
    }

    /// Set the marker edge colour
    ///
    /// Turns the edge on; see
    /// [`LineConfig::marker_edge_color`](crate::plots::basic::LineConfig::marker_edge_color).
    pub fn marker_edge_color(mut self, color: Color) -> Self {
        self.config = std::mem::take(&mut self.config).marker_edge_color(color);
        self
    }

    /// Set the marker edge width in points
    ///
    /// A positive width turns the edge on and `0.0` turns it off; see
    /// [`LineConfig::marker_edge_width`](crate::plots::basic::LineConfig::marker_edge_width).
    pub fn marker_edge_width(mut self, width: f32) -> Self {
        self.config = std::mem::take(&mut self.config).marker_edge_width(width);
        self
    }

    /// Show or hide the marker edge
    ///
    /// Line markers follow the same default scatter markers do — bare unless
    /// asked otherwise; see
    /// [`LineConfig::show_marker_edge`](crate::plots::basic::LineConfig::show_marker_edge).
    pub fn show_marker_edge(mut self, show: bool) -> Self {
        self.config = std::mem::take(&mut self.config).show_marker_edge(show);
        self
    }

    /// Finalize the line series and add it to the plot
    fn finalize(mut self) -> super::Plot {
        let (x_data, y_data) = resolve_xy_input(&self.input, &mut self.style);

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
    /// Enable or disable plot-area density aggregation for this scatter.
    ///
    /// The density path bins each point directly into a pixel-sized grid,
    /// spreads the counts over the series' marker footprint, and composites
    /// each pixel in the series color at the scatter-equivalent alpha
    /// `1 - (1 - alpha)^covering_markers` — so the result keeps the exact
    /// render's silhouette while its cost scales with plot pixels rather
    /// than points. Marker size and shape are honored for every marker whose
    /// rows are one centered span (circle, square, diamond, the triangles,
    /// plus); `Cross` and `Star` fall back to their bounding disk, open
    /// variants render filled, and edges and antialiasing are not
    /// reproduced. Density series cannot be exported to SVG. The default is
    /// `false`, preserving exact marker rendering unless explicitly
    /// requested; see the performance guide's capability table.
    pub fn density(mut self, density: bool) -> Self {
        self.config.density = Some(density);
        self
    }

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
        self.style.props.marker_style.set(style.into());
        self
    }

    /// Set a reactive marker style.
    pub fn marker_source<S>(mut self, style: S) -> Self
    where
        S: Into<ReactiveValue<crate::render::MarkerStyle>>,
    {
        self.style.props.marker_style.set(style.into());
        self
    }

    /// Set marker size
    ///
    /// # Arguments
    /// * `size` - Marker size in points (default: 6.0)
    pub fn marker_size(mut self, size: f32) -> Self {
        self.config.size = size.max(0.1);
        self.style.props.marker_size.set(size.into());
        self
    }

    /// Set a reactive marker size.
    pub fn marker_size_source<S>(mut self, size: S) -> Self
    where
        S: Into<ReactiveValue<f32>>,
    {
        self.style.props.marker_size.set(size.into());
        self
    }

    /// Set marker edge width
    ///
    /// A positive width turns the edge on and `0.0` turns it off. Delegates to
    /// [`ScatterConfig::edge_width`](crate::plots::basic::ScatterConfig::edge_width)
    /// so that rule lives in exactly one place.
    ///
    /// # Arguments
    /// * `width` - Edge width in points (default: 0.8)
    pub fn edge_width(mut self, width: f32) -> Self {
        self.config = std::mem::take(&mut self.config).edge_width(width);
        self
    }

    /// Set marker edge color
    ///
    /// Turns the edge on; see
    /// [`ScatterConfig::edge_color`](crate::plots::basic::ScatterConfig::edge_color).
    pub fn edge_color(mut self, color: Color) -> Self {
        self.config = std::mem::take(&mut self.config).edge_color(color);
        self
    }

    /// Show or hide the marker edge
    ///
    /// Markers are bare by default; `.show_edge(true)` rims a filled marker
    /// with its own fill darkened by 30%. A rim is drawn over the marker's
    /// boundary, so it darkens overlapping neighbours and dominates markers of
    /// a few points — see
    /// [`ScatterConfig::show_edge`](crate::plots::basic::ScatterConfig::show_edge).
    pub fn show_edge(mut self, show: bool) -> Self {
        self.config = std::mem::take(&mut self.config).show_edge(show);
        self
    }

    /// Finalize the scatter series and add it to the plot
    fn finalize(mut self) -> super::Plot {
        let (x_data, y_data) = resolve_xy_input(&self.input, &mut self.style);

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
            PlotInput::CategoricalSource { categories, values } => {
                (categories.clone(), values.clone())
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

// ============================================================================
// Terminal methods for the four config types whose `finalize()` lives in
// `series_api.rs`.
//
// `Plot`'s fields are `pub(super)`, so the histogram/box plot/heatmap/error bar
// `finalize()` bodies cannot be written from `src/plots/*.rs`; they live in
// `series_api.rs` instead. `macro_rules! impl_terminal_methods` is textually
// scoped to this file, so the invocations have to be here.
// ============================================================================
impl_terminal_methods!(crate::plots::HistogramConfig);
impl_terminal_methods!(crate::plots::BoxPlotConfig);
impl_terminal_methods!(crate::plots::heatmap::HeatmapConfig);
impl_terminal_methods!(crate::plots::error::ErrorBarConfig);

// The compute-only plot types, which enter through `SeriesType::Computed`.
// Their `finalize()`s are in `series_api.rs` for the same reason as the four
// above: they touch `Plot`'s `pub(super)` fields.
impl_terminal_methods!(crate::plots::distribution::RugConfig);
impl_terminal_methods!(crate::plots::categorical::StripConfig);
impl_terminal_methods!(crate::plots::categorical::SwarmConfig);
impl_terminal_methods!(crate::plots::continuous::hexbin::HexbinConfig);
impl_terminal_methods!(crate::plots::hierarchical::DendrogramConfig);

// The multi-series plot types (grouped bar, stacked bar, stacked area). Their
// `finalize()`s are in `series_api.rs` for the same reason as the ones above:
// they touch `Plot`'s `pub(super)` fields.
impl_terminal_methods!(crate::plots::categorical::GroupedBarConfig);
impl_terminal_methods!(crate::plots::categorical::StackedBarConfig);
impl_terminal_methods!(crate::plots::continuous::StackPlotConfig);

#[cfg(test)]
mod tests;

/// Tests for the API-consistency guarantees this module is responsible for:
/// every setter returns `Self`, the unambiguous names are the ones that work,
/// and each deprecated alias still does exactly what the new name does.
#[cfg(test)]
mod api_consistency_tests {
    use super::*;
    use crate::core::LegendPosition;

    fn sample() -> Vec<f64> {
        (0..64)
            .map(|i| (i as f64 * 0.37).sin() + i as f64 / 32.0)
            .collect()
    }

    // ===== 1. No setter may end the chain =====

    #[test]
    fn legend_position_returns_the_builder_so_the_chain_continues() {
        // Regression: `legend_position` used to return `Plot`, silently dropping
        // the caller out of the builder mid-chain.
        let builder = super::super::Plot::new()
            .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 0.5])
            .legend_position(LegendPosition::UpperRight)
            // These only resolve if `legend_position` returned `Self`.
            .line_width(2.5)
            .label("series");

        assert_eq!(builder.style.props.line_width.cloned(), Some(2.5));
        assert_eq!(builder.style.label.as_deref(), Some("series"));
        assert!(builder.get_plot().layout.legend.enabled);
        assert_eq!(
            builder.get_plot().layout.legend.position,
            LegendPosition::UpperRight
        );
    }

    #[test]
    fn legend_position_is_available_on_non_cartesian_builders_too() {
        let builder = super::super::Plot::new()
            .pie(&[1.0, 2.0, 3.0])
            .legend_position(LegendPosition::OutsideRight)
            .donut(0.4);

        assert!(builder.get_plot().layout.legend.enabled);
        assert_eq!(
            builder.get_plot().layout.legend.position,
            LegendPosition::OutsideRight
        );
        assert!((builder.get_config().inner_radius - 0.4).abs() < 1e-12);
    }

    // ===== 4. legend / legend_best / legend_position parity =====

    #[test]
    fn legend_legend_best_and_legend_position_agree_on_the_builder() {
        let via_legend = super::super::Plot::new()
            .kde(&sample())
            .legend(LegendPosition::Best);
        let via_best = super::super::Plot::new().kde(&sample()).legend_best();
        let via_position = super::super::Plot::new()
            .kde(&sample())
            .legend_position(LegendPosition::Best);

        for builder in [&via_legend, &via_best, &via_position] {
            assert!(builder.get_plot().layout.legend.enabled);
            assert_eq!(
                builder.get_plot().layout.legend.position,
                LegendPosition::Best
            );
        }
    }

    // ===== 2. `width` no longer means three different things =====

    #[test]
    fn violin_width_sets_the_body_width_and_the_deprecated_alias_matches() {
        let renamed = super::super::Plot::new()
            .violin(&sample())
            .violin_width(0.75);
        #[allow(deprecated)]
        let legacy = super::super::Plot::new().violin(&sample()).width(0.75);

        assert!((renamed.get_config().width - 0.75).abs() < 1e-12);
        assert!((legacy.get_config().width - renamed.get_config().width).abs() < 1e-12);
    }

    #[test]
    fn box_width_sets_the_category_fraction_and_the_deprecated_alias_matches() {
        let renamed = super::super::Plot::new().boxen(&sample()).box_width(0.7);
        #[allow(deprecated)]
        let legacy = super::super::Plot::new().boxen(&sample()).width(0.7);

        assert!((renamed.get_config().width - 0.7).abs() < 1e-12);
        assert!((legacy.get_config().width - renamed.get_config().width).abs() < 1e-12);
        // Still a fraction: clamped into 0.1..=1.0, unlike violin_width.
        let clamped = super::super::Plot::new().boxen(&sample()).box_width(9.0);
        assert!((clamped.get_config().width - 1.0).abs() < 1e-12);
    }

    #[test]
    fn arrow_family_sets_quiver_config_and_the_deprecated_aliases_match() {
        let x = vec![0.0, 1.0];
        let y = vec![0.0, 1.0];
        let u = vec![1.0, 0.5];
        let v = vec![0.25, 0.75];

        let renamed = super::super::Plot::new()
            .quiver(&x, &y, &u, &v)
            .arrow_scale(0.25)
            .arrow_width(1.25)
            .arrow_head_length(0.35)
            .arrow_head_width(0.2);

        #[allow(deprecated)]
        let legacy = super::super::Plot::new()
            .quiver(&x, &y, &u, &v)
            .scale(0.25)
            .width(1.25)
            .headlength(0.35)
            .headwidth(0.2);

        assert!((renamed.get_config().scale - 0.25).abs() < 1e-12);
        assert!((renamed.get_config().width - 1.25).abs() < 1e-6);
        assert!((renamed.get_config().headlength - 0.35).abs() < 1e-12);
        assert!((renamed.get_config().headwidth - 0.2).abs() < 1e-12);

        assert!((legacy.get_config().scale - renamed.get_config().scale).abs() < 1e-12);
        assert!((legacy.get_config().width - renamed.get_config().width).abs() < 1e-6);
        assert!((legacy.get_config().headlength - renamed.get_config().headlength).abs() < 1e-12);
        assert!((legacy.get_config().headwidth - renamed.get_config().headwidth).abs() < 1e-12);
    }

    // ===== 3. line_style / line_width are the one spelling =====

    #[test]
    fn line_style_and_the_deprecated_style_alias_are_equivalent() {
        let renamed = super::super::Plot::new()
            .line(&[0.0, 1.0], &[0.0, 1.0])
            .line_style(LineStyle::Dashed);
        #[allow(deprecated)]
        let legacy = super::super::Plot::new()
            .line(&[0.0, 1.0], &[0.0, 1.0])
            .style(LineStyle::Dashed);

        assert_eq!(
            renamed.style.props.line_style.cloned(),
            Some(LineStyle::Dashed)
        );
        assert_eq!(
            legacy.style.props.line_style.cloned(),
            renamed.style.props.line_style.cloned()
        );
        assert!(legacy.style.props.line_style.source().is_none());
    }

    #[test]
    fn line_style_source_and_the_deprecated_style_source_alias_are_equivalent() {
        let renamed = super::super::Plot::new()
            .line(&[0.0, 1.0], &[0.0, 1.0])
            .line_style_source(LineStyle::Dotted);
        #[allow(deprecated)]
        let legacy = super::super::Plot::new()
            .line(&[0.0, 1.0], &[0.0, 1.0])
            .style_source(LineStyle::Dotted);

        assert_eq!(
            renamed.style.props.line_style.cloned(),
            Some(LineStyle::Dotted)
        );
        assert_eq!(
            legacy.style.props.line_style.cloned(),
            renamed.style.props.line_style.cloned()
        );
    }

    #[test]
    fn generic_line_width_reaches_every_builder_that_had_a_bespoke_spelling() {
        // `line_width` is the replacement named by the deprecation notes on
        // `kde_line_width` / `ecdf_line_width` / `contour_line_width`; it must
        // record the override on all three.
        let kde = super::super::Plot::new().kde(&sample()).line_width(3.0);
        let ecdf = super::super::Plot::new().ecdf(&sample()).line_width(3.0);
        let contour_x = vec![0.0, 1.0, 2.0];
        let contour_y = vec![0.0, 1.0];
        let contour_z = vec![0.5; contour_x.len() * contour_y.len()];
        let contour = super::super::Plot::new()
            .contour(&contour_x, &contour_y, &contour_z)
            .line_width(3.0);

        assert_eq!(kde.style.props.line_width.cloned(), Some(3.0));
        assert_eq!(ecdf.style.props.line_width.cloned(), Some(3.0));
        assert_eq!(contour.style.props.line_width.cloned(), Some(3.0));
    }

    // ===== Renamed-for-clarity accessors =====

    #[test]
    fn pie_label_font_size_replaces_the_ambiguous_font_size() {
        let renamed = super::super::Plot::new()
            .pie(&[1.0, 2.0])
            .label_font_size(14.0);
        #[allow(deprecated)]
        let legacy = super::super::Plot::new().pie(&[1.0, 2.0]).font_size(14.0);

        assert!((renamed.get_config().label_font_size - 14.0).abs() < 1e-6);
        assert!(
            (legacy.get_config().label_font_size - renamed.get_config().label_font_size).abs()
                < 1e-6
        );
    }

    #[test]
    fn radar_series_stylers_replace_the_with_prefixed_spellings() {
        let renamed = super::super::Plot::new()
            .radar(&["A", "B", "C"])
            .add_series("One", &[1.0, 2.0, 3.0])
            .series_color(Color::RED)
            .series_fill_alpha(0.25)
            .series_line_width(3.0);

        #[allow(deprecated)]
        let legacy = super::super::Plot::new()
            .radar(&["A", "B", "C"])
            .add_series("One", &[1.0, 2.0, 3.0])
            .with_color(Color::RED)
            .with_fill_alpha(0.25)
            .with_line_width(3.0);

        assert_eq!(
            renamed.get_config().colors.as_deref(),
            Some([Color::RED].as_slice())
        );
        assert_eq!(
            renamed.get_config().per_series_fill_alphas,
            vec![Some(0.25)]
        );
        assert_eq!(renamed.get_config().per_series_line_widths, vec![Some(3.0)]);

        assert_eq!(legacy.get_config().colors, renamed.get_config().colors);
        assert_eq!(
            legacy.get_config().per_series_fill_alphas,
            renamed.get_config().per_series_fill_alphas
        );
        assert_eq!(
            legacy.get_config().per_series_line_widths,
            renamed.get_config().per_series_line_widths
        );
        // The chart-wide knob stays separate from the per-series one.
        let chart_wide = super::super::Plot::new()
            .radar(&["A", "B", "C"])
            .add_series("One", &[1.0, 2.0, 3.0])
            .fill_alpha(0.9);
        assert!((chart_wide.get_config().fill_alpha - 0.9).abs() < 1e-6);
        assert_eq!(chart_wide.get_config().per_series_fill_alphas, vec![None]);
    }
}
