//! Core plot traits for data transformation and rendering
//!
//! This module defines the core traits that unify all plot type implementations:
//!
//! - [`PlotCompute`]: Transforms raw input data into computed plot data
//! - [`PlotData`]: Common interface for computed data (bounds, emptiness)
//! - [`PlotRender`]: Renders computed data to a renderer
//!
//! # Design Philosophy
//!
//! These traits follow a separation of concerns:
//!
//! 1. **Computation** (`PlotCompute`): Pure data transformation, no rendering knowledge
//! 2. **Data interface** (`PlotData`): Common queries on computed data
//! 3. **Rendering** (`PlotRender`): Visual output, depends on computed data
//!
//! # Example
//!
//! ```rust,ignore
//! use ruviz::plots::traits::{PlotCompute, PlotData, PlotRender};
//!
//! // Compute KDE data
//! let kde_data = Kde::compute(&data, &KdeConfig::default())?;
//!
//! // Query bounds for axis setup
//! let ((x_min, x_max), (y_min, y_max)) = kde_data.data_bounds();
//!
//! // Render to canvas
//! kde_data.render(&mut renderer, &area, &theme, color)?;
//! ```

use crate::axes::AxisScale;
use crate::core::error::Result;
use crate::core::transform::CoordinateTransform;
use crate::core::units::RenderScale;
use crate::render::{Color, LineStyle, MarkerStyle, SkiaRenderer, Theme};

/// Defines the plot area for rendering
///
/// Represents the rectangular region where plot data should be rendered,
/// including the coordinate transformation parameters.
///
/// # This mapping follows the axis scale
///
/// A `PlotArea` carries the [`AxisScale`] of each axis, and
/// [`PlotArea::data_to_screen`] projects through it. A plot type drawn through
/// `PlotArea` therefore honours a logarithmic axis for free, and declares
/// [`AxisScaleSupport::Scaled`] — see [`AxisScaleSupport`]. This is the same
/// projection `map_data_to_pixels_scaled` performs, reached through the same
/// [`CoordinateTransform`], so the two cannot drift.
///
/// [`PlotArea::new`] defaults both axes to [`AxisScale::Linear`]; render paths
/// call [`PlotArea::with_scales`] to state the figure's actual scales.
#[derive(Debug, Clone, Copy)]
pub struct PlotArea {
    /// Left edge of plot area in pixels
    pub x: f32,
    /// Top edge of plot area in pixels
    pub y: f32,
    /// Width of plot area in pixels
    pub width: f32,
    /// Height of plot area in pixels
    pub height: f32,
    /// Data x-axis minimum value
    pub x_min: f64,
    /// Data x-axis maximum value
    pub x_max: f64,
    /// Data y-axis minimum value
    pub y_min: f64,
    /// Data y-axis maximum value
    pub y_max: f64,
    /// Scale the x axis is projected through
    pub x_scale: AxisScale,
    /// Scale the y axis is projected through
    pub y_scale: AxisScale,
}

impl PlotArea {
    /// Create a new plot area with the given bounds and linear axes.
    ///
    /// Use [`Self::with_scales`] to attach the figure's axis scales.
    pub fn new(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> Self {
        Self {
            x,
            y,
            width,
            height,
            x_min,
            x_max,
            y_min,
            y_max,
            x_scale: AxisScale::Linear,
            y_scale: AxisScale::Linear,
        }
    }

    /// The same plot area, projected through the given axis scales.
    #[inline]
    #[must_use]
    pub fn with_scales(mut self, x_scale: AxisScale, y_scale: AxisScale) -> Self {
        self.x_scale = x_scale;
        self.y_scale = y_scale;
        self
    }

    /// Create a [`CoordinateTransform`] from this plot area.
    ///
    /// The transform carries the pixel and data ranges only; the axis scales
    /// are supplied per call by the `*_scaled` methods on it, which is how
    /// [`Self::data_to_screen`] applies them.
    #[inline]
    pub fn to_transform(self) -> CoordinateTransform {
        CoordinateTransform::from_plot_area(
            self.x,
            self.y,
            self.width,
            self.height,
            self.x_min,
            self.x_max,
            self.y_min,
            self.y_max,
        )
    }

    /// Transform data coordinates to screen coordinates, following this area's
    /// axis scales.
    ///
    /// Returns `NaN` pixels for a sample the scale cannot represent (zero or
    /// negative on a log axis, or any non-finite value). Callers that need to
    /// *know* a sample was rejected — to break a polyline, or to drop a marker
    /// — must use [`Self::try_data_to_screen`] instead.
    ///
    /// # Arguments
    /// * `data_x` - X coordinate in data space
    /// * `data_y` - Y coordinate in data space
    ///
    /// # Returns
    /// (screen_x, screen_y) in pixel coordinates
    #[inline]
    pub fn data_to_screen(&self, data_x: f64, data_y: f64) -> (f32, f32) {
        self.to_transform()
            .data_to_screen_scaled(data_x, data_y, &self.x_scale, &self.y_scale)
    }

    /// Transform data coordinates to screen coordinates, rejecting samples the
    /// axis scales cannot represent.
    ///
    /// Returns `None` exactly when [`AxisScale::is_valid_value`] rejects either
    /// coordinate, so a renderer can break its geometry at the gap rather than
    /// drawing across it.
    #[inline]
    pub fn try_data_to_screen(&self, data_x: f64, data_y: f64) -> Option<(f32, f32)> {
        self.to_transform()
            .try_data_to_screen_scaled(data_x, data_y, &self.x_scale, &self.y_scale)
    }

    /// Project the edge of axis-clipped geometry, pinning an unplaceable edge
    /// to the axis floor.
    ///
    /// Image-like geometry — heatmap cells, filled contour bands — is clipped by
    /// the axes rather than broken at gaps: a cell whose extent starts at zero
    /// on a log axis is not absent, it simply runs off the bottom of the axis.
    /// A log axis' limits are always positive, so a zero or negative edge is
    /// unambiguously below every value the axis can show and belongs at its low
    /// end. Without this it would project to `NaN` and land at pixel zero,
    /// outside the plot area entirely.
    ///
    /// Only edges may use this. A *sample* that the axis cannot place has no
    /// position at all and must be dropped or split on — see
    /// [`Self::try_data_to_screen`].
    #[inline]
    pub fn edge_data_to_screen(&self, data_x: f64, data_y: f64) -> (f32, f32) {
        fn pinned(scale: &AxisScale, value: f64, min: f64, max: f64) -> f64 {
            if scale.is_valid_value(value) {
                value
            } else {
                min.min(max)
            }
        }

        self.data_to_screen(
            pinned(&self.x_scale, data_x, self.x_min, self.x_max),
            pinned(&self.y_scale, data_y, self.y_min, self.y_max),
        )
    }

    /// Project a run of data points, **dropping** those the axis scales cannot
    /// place.
    ///
    /// This is the polygon/marker projection: a vertex with no position on the
    /// axis is not part of the shape, and the shape closes over the vertices
    /// that remain. A polyline must use [`Self::project_subpaths`] instead —
    /// dropping a sample from a line silently joins it across the gap.
    pub fn project_points<I>(&self, points: I) -> Vec<(f32, f32)>
    where
        I: IntoIterator<Item = (f64, f64)>,
    {
        points
            .into_iter()
            .filter_map(|(x, y)| self.try_data_to_screen(x, y))
            .collect()
    }

    /// Project a run of data points into contiguous sub-paths, **breaking** at
    /// every point the axis scales cannot place.
    ///
    /// Drawing each run as its own polyline is what makes a curve break at the
    /// gap instead of jumping across it.
    ///
    /// A run of length one draws nothing: a line has no ink at a single point.
    /// Use [`Self::project_points`] for anything that should still mark such a
    /// sample.
    pub fn project_subpaths<I>(&self, points: I) -> Vec<Vec<(f32, f32)>>
    where
        I: IntoIterator<Item = (f64, f64)>,
    {
        let mut runs: Vec<Vec<(f32, f32)>> = Vec::new();
        let mut current: Vec<(f32, f32)> = Vec::new();
        for (x, y) in points {
            match self.try_data_to_screen(x, y) {
                Some(point) => current.push(point),
                None => {
                    if !current.is_empty() {
                        runs.push(std::mem::take(&mut current));
                    }
                }
            }
        }
        if !current.is_empty() {
            runs.push(current);
        }
        runs
    }

    /// Screen y of the baseline a series fills down to.
    ///
    /// Area fills, density curves, step functions, bars and histograms all fill
    /// from their value to zero. A logarithmic y axis has no position for zero,
    /// so there the baseline is the bottom edge of the plot area — the axis
    /// floor, which is as far down as the axis goes. This is the single place
    /// that rule is stated, so no two fill renderers can disagree about where
    /// the bottom of a bar is.
    #[inline]
    pub fn fill_baseline_y(&self) -> f32 {
        if self.y_scale.is_valid_value(0.0) {
            self.to_transform()
                .data_to_screen_scaled(self.x_min, 0.0, &AxisScale::Linear, &self.y_scale)
                .1
        } else {
            self.y + self.height
        }
    }

    /// Transform screen coordinates to data coordinates
    ///
    /// # Arguments
    /// * `screen_x` - X coordinate in pixels
    /// * `screen_y` - Y coordinate in pixels
    ///
    /// # Returns
    /// (data_x, data_y) in data space
    #[inline]
    pub fn screen_to_data(&self, screen_x: f32, screen_y: f32) -> (f64, f64) {
        self.to_transform()
            .screen_to_data_scaled(screen_x, screen_y, &self.x_scale, &self.y_scale)
    }

    /// Check if a data point is within the plot area bounds
    #[inline]
    pub fn contains_data(&self, data_x: f64, data_y: f64) -> bool {
        self.to_transform().contains_data(data_x, data_y)
    }

    /// Get the center point of the plot area in screen coordinates
    pub fn center(&self) -> (f32, f32) {
        self.to_transform().screen_center()
    }

    /// Get the center point of the plot area in data coordinates
    pub fn data_center(&self) -> (f64, f64) {
        self.to_transform().data_center()
    }
}

/// Whether a plot type's geometry can honour a non-linear [`AxisScale`] on one
/// axis.
///
/// A figure draws its axis line, its ticks and its tick labels scale-aware. If
/// the series geometry on the same figure is laid out linearly, the result is a
/// log-labelled axis with linearly-positioned marks — a quantitatively wrong
/// plot, produced without any warning. This enum is how a plot type states
/// which of those it does, so the render path can refuse the combination it
/// cannot draw truthfully instead of drawing it wrong.
///
/// The rule the crate follows, per axis:
///
/// * geometry projected through the axis scale — with `map_data_to_pixels_scaled`
///   or through [`PlotArea::data_to_screen`], which are the same transform →
///   [`Self::Scaled`];
/// * geometry placed at ordinal slots or in a synthetic cell rather than at a
///   data value, so there is no quantity to take a logarithm of →
///   [`Self::Unsupported`];
/// * a plot type with its own coordinate system that never consults the figure
///   axes (pie, radar, polar) → [`Self::Independent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisScaleSupport {
    /// The renderer projects this axis through the configured [`AxisScale`],
    /// so every scale renders correctly.
    Scaled,
    /// The plot type draws in its own coordinate system and never consults the
    /// figure's axis, so the configured scale cannot misplace its geometry.
    Independent,
    /// The renderer cannot place this axis' geometry under a non-linear scale.
    /// The payload says why and is quoted verbatim in the error the render
    /// path returns.
    Unsupported(&'static str),
}

impl AxisScaleSupport {
    /// The refusal every categorical axis gives, worded once.
    ///
    /// Bars, box plots, violins, boxen plots, strip plots, swarm plots and the
    /// leaf axis of a dendrogram all place their geometry in the same
    /// unit-wide ordinal slots, so they all owe the reader the same
    /// explanation. Writing it here is what stops that from becoming several
    /// slightly different sentences about one situation.
    pub const ORDINAL: Self = Self::Unsupported(
        "its categories sit at ordinal positions, which carry no quantitative spacing",
    );

    /// Whether `scale` can be rendered faithfully on this axis.
    ///
    /// Only a non-linear scale can be refused: [`AxisScale::Linear`] is what
    /// every renderer already draws.
    pub fn accepts(&self, scale: &AxisScale) -> bool {
        match self {
            Self::Scaled | Self::Independent => true,
            Self::Unsupported(_) => matches!(scale, AxisScale::Linear),
        }
    }

    /// The reason a non-linear scale is refused, or `None` when it is accepted.
    pub fn rejection_reason(&self) -> Option<&'static str> {
        match self {
            Self::Scaled | Self::Independent => None,
            Self::Unsupported(reason) => Some(reason),
        }
    }
}

/// Marker trait for plot configuration types
///
/// All plot-specific configuration structs should implement this trait.
/// This enables generic handling of configurations in the `PlotBuilder`.
///
/// # Requirements
///
/// Implementations must provide:
/// - `Default`: Sensible defaults for all configuration options
/// - `Clone`: Allow cloning for storage in series
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Debug, Clone, Default)]
/// pub struct KdeConfig {
///     pub bandwidth: Option<f64>,
///     pub n_points: usize,
///     pub fill: bool,
/// }
///
/// impl PlotConfig for KdeConfig {}
/// ```
pub trait PlotConfig: Default + Clone {}

/// Trait for computing plot data from raw input
///
/// This trait defines the data transformation step for each plot type.
/// It takes raw input data and configuration, producing computed data
/// ready for rendering.
///
/// # Type Parameters
///
/// - `Input<'a>`: The input data type (can be borrowed)
/// - `Config`: Plot-specific configuration implementing [`PlotConfig`]
/// - `Output`: Computed data implementing [`PlotData`]
///
/// # Example
///
/// ```rust,ignore
/// impl PlotCompute for Kde {
///     type Input<'a> = &'a [f64];
///     type Config = KdeConfig;
///     type Output = KdeData;
///
///     fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output> {
///         // Perform KDE computation...
///         Ok(KdeData { x, y, bandwidth })
///     }
/// }
/// ```
pub trait PlotCompute {
    /// The input data type (typically a reference to slices or tuples of slices)
    type Input<'a>;

    /// The configuration type for this plot
    type Config: PlotConfig;

    /// The computed output data type
    type Output: PlotData;

    /// Compute plot data from input and configuration
    ///
    /// # Arguments
    /// * `input` - The raw input data
    /// * `config` - Plot-specific configuration
    ///
    /// # Returns
    /// Computed data ready for rendering, or an error
    fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output>;
}

/// Common interface for computed plot data
///
/// All computed plot data types implement this trait to provide
/// common queries needed for rendering setup and validation.
///
/// # Example
///
/// ```rust,ignore
/// impl PlotData for KdeData {
///     fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
///         let x_min = self.x.iter().copied().fold(f64::INFINITY, f64::min);
///         let x_max = self.x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
///         let y_min = 0.0; // Density starts at 0
///         let y_max = self.y.iter().copied().fold(f64::NEG_INFINITY, f64::max);
///         ((x_min, x_max), (y_min, y_max))
///     }
///
///     fn is_empty(&self) -> bool {
///         self.x.is_empty()
///     }
/// }
/// ```
pub trait PlotData {
    /// Get the data bounds for this plot data
    ///
    /// Returns `((x_min, x_max), (y_min, y_max))` representing the
    /// bounding box of the data in data coordinates.
    ///
    /// # Returns
    /// A tuple of tuples: `((x_min, x_max), (y_min, y_max))`
    fn data_bounds(&self) -> ((f64, f64), (f64, f64));

    /// Check if the plot data is empty
    ///
    /// Empty data should not be rendered and may require special handling.
    fn is_empty(&self) -> bool;
}

/// Trait for rendering computed plot data
///
/// This trait extends [`PlotData`] with rendering capability.
/// Implementations draw the computed data to a renderer within
/// the specified plot area.
///
/// # Example
///
/// ```rust,ignore
/// impl PlotRender for KdeData {
///     fn render(
///         &self,
///         renderer: &mut SkiaRenderer,
///         area: &PlotArea,
///         theme: &Theme,
///         color: Color,
///     ) -> Result<()> {
///         if self.is_empty() {
///             return Ok(());
///         }
///
///         // Draw the KDE curve
///         let points: Vec<(f32, f32)> = self.x.iter()
///             .zip(self.y.iter())
///             .map(|(&x, &y)| area.data_to_screen(x, y))
///             .collect();
///
///         renderer.draw_polyline(&points, color, theme.line_width, LineStyle::Solid)?;
///         Ok(())
///     }
/// }
/// ```
pub trait PlotRender: PlotData {
    /// Render the plot data to a renderer
    ///
    /// # Arguments
    /// * `renderer` - The Skia renderer to draw to
    /// * `area` - The plot area defining coordinate transformation
    /// * `theme` - Theme for styling (line widths, etc.)
    /// * `color` - The color to use for this series
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if rendering fails
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        theme: &Theme,
        color: Color,
    ) -> Result<()>;

    /// Render with additional styling options
    ///
    /// Default implementation calls `render()` with the base color.
    /// Override for plot types that support additional styling.
    fn render_styled(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        theme: &Theme,
        color: Color,
        _alpha: f32,
        _line_width: Option<f32>,
    ) -> Result<()> {
        self.render(renderer, area, theme, color)
    }

    /// Render with resolved plot-level grid styling when the plot type owns its axes.
    fn render_styled_with_grid(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        theme: &Theme,
        color: Color,
        alpha: f32,
        line_width: Option<f32>,
        _grid_style: Option<&crate::core::GridStyle>,
    ) -> Result<()> {
        self.render_styled(renderer, area, theme, color, alpha, line_width)
    }
}

/// One drawing instruction in device pixels, in the vocabulary both backends
/// share.
///
/// A [`ComputedSeries`] describes what it wants drawn as a list of these, and
/// each backend has exactly one loop that turns them into ink
/// ([`draw_primitives`] for raster, [`draw_primitives_svg`] for SVG). That is
/// what makes it impossible for a plot type wired this way to render in PNG and
/// not in SVG: there is no per-plot-type SVG code to forget to write.
///
/// Every coordinate and width here is already in device pixels — the
/// projection and the points-to-pixels conversion happen in
/// [`ComputedSeries::primitives`], where the plot type's own geometry lives.
#[derive(Debug, Clone, PartialEq)]
pub enum PlotPrimitive {
    /// A straight stroked segment.
    Line {
        /// Start point, in device pixels.
        from: (f32, f32),
        /// End point, in device pixels.
        to: (f32, f32),
        /// Stroke colour, alpha already applied.
        color: Color,
        /// Stroke width in device pixels.
        width_px: f32,
        /// Dash pattern.
        style: LineStyle,
    },
    /// A closed shape, optionally filled and optionally outlined.
    Polygon {
        /// Vertices in device pixels; the shape closes automatically.
        points: Vec<(f32, f32)>,
        /// Fill colour, or `None` for an outline-only shape.
        fill: Option<Color>,
        /// `(colour, width in device pixels)` of the outline, or `None`.
        edge: Option<(Color, f32)>,
    },
    /// A point marker.
    Marker {
        /// Centre, in device pixels.
        at: (f32, f32),
        /// Marker size in device pixels.
        size_px: f32,
        /// Marker shape.
        style: MarkerStyle,
        /// Marker colour, alpha already applied.
        color: Color,
    },
}

/// The styling a series was given, resolved, plus the scale that converts the
/// plot type's point-valued sizes into device pixels.
///
/// Passed to [`ComputedSeries::primitives`] so that a plot type never has to
/// reach for the renderer to find out how big a point is — which is how three
/// plot types ended up drawing the same physical size at every DPI.
#[derive(Debug, Clone, Copy)]
pub struct ComputedStyle {
    /// Points-to-pixels conversion for this render.
    pub scale: RenderScale,
    /// The series colour the palette resolved, before the series alpha.
    pub color: Color,
    /// Series alpha in `0.0..=1.0`.
    pub alpha: f32,
    /// Series line width in **points**, or `None` to use the plot type's own.
    pub line_width: Option<f32>,
    /// The theme's [`Theme::patch_edge_color`] override, carried here because a
    /// [`ComputedSeries::primitives`] call has no theme to reach for.
    ///
    /// Resolve it with [`Self::patch_edge`] rather than reading it directly, so
    /// a filled patch drawn through `primitives` follows the same edge rule as
    /// one drawn through [`PlotRender::render`].
    ///
    /// [`Theme::patch_edge_color`]: crate::render::Theme::patch_edge_color
    pub patch_edge_color: Option<Color>,
}

impl ComputedStyle {
    /// The style a bare [`PlotRender::render`] call implies: full opacity and
    /// the plot type's own line width.
    ///
    /// A `PlotRender::render` implementation is handed the theme, so pair this
    /// with [`Self::with_patch_edge_color`] when the plot type fills patches.
    pub fn opaque(scale: RenderScale, color: Color) -> Self {
        Self {
            scale,
            color,
            alpha: 1.0,
            line_width: None,
            patch_edge_color: None,
        }
    }

    /// `self` carrying the theme's filled-patch edge override.
    pub fn with_patch_edge_color(mut self, patch_edge_color: Option<Color>) -> Self {
        self.patch_edge_color = patch_edge_color;
        self
    }

    /// Resolve a filled patch's edge into the `(colour, width_in_pixels)` pair
    /// the primitives stroke with, or `None` when there is no edge.
    ///
    /// The colour rule is [`StyleResolver::patch_edge`]'s, so a bar reached
    /// through `primitives` and a bar reached through the renderer cannot
    /// darken their edges differently: an explicit colour wins, then the
    /// theme's override, then the fill darkened. `width_points` is the
    /// authored width in **points** and comes back scaled to device pixels,
    /// because that is what a [`PlotPrimitive`] carries.
    ///
    /// [`StyleResolver::patch_edge`]: crate::core::style_utils::StyleResolver::patch_edge
    pub fn patch_edge(
        &self,
        fill: Color,
        explicit: Option<Color>,
        width_points: f32,
    ) -> Option<(Color, f32)> {
        (width_points > 0.0).then(|| {
            let color = explicit
                .or(self.patch_edge_color)
                .unwrap_or_else(|| fill.darken(crate::core::style_utils::PATCH_EDGE_DARKEN));
            (color, self.scale.points_to_pixels(width_points))
        })
    }

    /// `base` with this series' alpha composed over its own.
    pub fn tinted(&self, base: Color) -> Color {
        base.with_alpha((f32::from(base.a) / 255.0) * self.alpha.clamp(0.0, 1.0))
    }

    /// The stroke width in device pixels, preferring the series override.
    pub fn stroke_px(&self, fallback_points: f32) -> f32 {
        self.scale
            .points_to_pixels(self.line_width.unwrap_or(fallback_points))
    }
}

/// Draw a [`ComputedSeries`]' primitives to the raster backend.
///
/// The twin of [`draw_primitives_svg`]; keep the two adjacent so any
/// divergence is five lines apart instead of two files apart.
pub fn draw_primitives(renderer: &mut SkiaRenderer, primitives: &[PlotPrimitive]) -> Result<()> {
    for primitive in primitives {
        match primitive {
            PlotPrimitive::Line {
                from,
                to,
                color,
                width_px,
                style,
            } => {
                renderer.draw_line(from.0, from.1, to.0, to.1, *color, *width_px, style.clone())?;
            }
            PlotPrimitive::Polygon { points, fill, edge } => {
                if let Some(fill) = fill {
                    renderer.draw_filled_polygon(points, *fill)?;
                }
                if let Some((color, width_px)) = edge {
                    renderer.draw_polygon_outline(points, *color, *width_px)?;
                }
            }
            PlotPrimitive::Marker {
                at,
                size_px,
                style,
                color,
            } => {
                renderer.draw_marker(at.0, at.1, *size_px, *style, *color)?;
            }
        }
    }
    Ok(())
}

/// Draw a [`ComputedSeries`]' primitives to the SVG backend.
///
/// The twin of [`draw_primitives`]. The SVG primitives return no error — they
/// drop unplaceable geometry rather than failing an export — so this cannot
/// fail either.
pub fn draw_primitives_svg(svg: &mut crate::export::SvgRenderer, primitives: &[PlotPrimitive]) {
    for primitive in primitives {
        match primitive {
            PlotPrimitive::Line {
                from,
                to,
                color,
                width_px,
                style,
            } => {
                svg.draw_line(from.0, from.1, to.0, to.1, *color, *width_px, style.clone());
            }
            PlotPrimitive::Polygon { points, fill, edge } => {
                if let Some(fill) = fill {
                    svg.draw_filled_polygon(points, *fill);
                }
                if let Some((color, width_px)) = edge {
                    svg.draw_polygon_outline(points, *color, *width_px);
                }
            }
            PlotPrimitive::Marker {
                at,
                size_px,
                style,
                color,
            } => {
                svg.draw_marker(at.0, at.1, *size_px, *style, *color);
            }
        }
    }
}

/// A precomputed plot payload the builder carries without a bespoke series
/// variant per plot type.
///
/// This is the crate's one extension point for "a plot type that ships finished
/// geometry". Rug, strip, swarm, hexbin and dendrogram all reach the render path
/// through it, and so does every future compute-only type.
///
/// # Why one trait instead of one variant per plot type
///
/// The internal `SeriesType` enum is matched exhaustively in eleven places —
/// bounds, validation, axis-scale support, point counting, and the raster and
/// SVG render paths. Five bespoke variants would have meant roughly forty new
/// match arms and five more things to keep in step; one variant means eight
/// arms, written once. Adding a plot type after this costs a `Plot::` method, a
/// `finalize()` and an `impl ComputedSeries` — no render arm, no bounds arm, no
/// entry in the axis-scale table, and therefore no way for a new type to be
/// wired into one backend and not the other.
///
/// [`PlotRender`] and [`PlotData`] are object-safe (every method takes `&self`,
/// none is generic and none returns `Self`), which is what makes the collapse
/// possible.
///
/// # Implementing
///
/// ```rust,ignore
/// impl ComputedSeries for MyPlotData {
///     fn kind(&self) -> &'static str {
///         "myplot"
///     }
///
///     fn point_count(&self) -> usize {
///         self.points.len()
///     }
///
///     fn primitives(&self, area: &PlotArea, style: &ComputedStyle) -> Vec<PlotPrimitive> {
///         // ...one description of the geometry, drawn by both backends
///     }
/// }
/// ```
pub trait ComputedSeries: PlotRender + std::fmt::Debug + Send + Sync {
    /// Plot-type name quoted in diagnostics and axis-scale refusals.
    ///
    /// Use the builder method's name (`"rug"`, `"hexbin"`), because that is what
    /// the reader of the error message typed.
    fn kind(&self) -> &'static str;

    /// Everything this series wants drawn, in device pixels.
    ///
    /// This is the *only* description of the geometry. Both backends consume it
    /// ([`draw_primitives`] and [`draw_primitives_svg`]), and the plot type's
    /// own [`PlotRender`] impl should be written over it too, so a caller
    /// driving the trait directly, a PNG and an SVG cannot show three different
    /// pictures.
    ///
    /// Drop anything the axes cannot place rather than emitting a `NaN`
    /// coordinate — see [`PlotArea::try_data_to_screen`].
    fn primitives(&self, area: &PlotArea, style: &ComputedStyle) -> Vec<PlotPrimitive>;

    /// Which axis scales this geometry can honour.
    ///
    /// Anything projected through [`PlotArea::data_to_screen`] is
    /// [`AxisScaleSupport::Scaled`] on both axes, which is the default and the
    /// case for every implementor in this crate today. Override only for
    /// geometry placed at ordinal slots or in its own coordinate system — see
    /// [`AxisScaleSupport`] for the rule.
    fn axis_scale_support(&self) -> (AxisScaleSupport, AxisScaleSupport) {
        (AxisScaleSupport::Scaled, AxisScaleSupport::Scaled)
    }

    /// Sample count, for the renderer's auto-optimisation heuristics.
    fn point_count(&self) -> usize;

    /// The category slots this series occupies, as `(tick label, slot centre)`.
    ///
    /// Override it and the plot gets a real category axis: the names the caller
    /// passed are printed under the data instead of the bare slot numbers
    /// `-0.5, 0, 0.5 …`, which mean nothing to a reader. This is the same
    /// mechanism bar charts, box plots, violins and boxen plots are on — see
    /// `series_category_slots` — so a categorical plot type joins it by
    /// answering this one question rather than by teaching the renderer a new
    /// special case.
    ///
    /// The default is "no slots", i.e. an ordinary quantitative x axis.
    fn category_slots(&self) -> Vec<(String, f64)> {
        Vec::new()
    }

    /// Which axis carries [`Self::category_slots`].
    ///
    /// Vertical plots place categories on x; horizontal plots place them on y.
    /// The default preserves the ordinary categorical-x behavior.
    fn category_orientation(&self) -> crate::core::Orientation {
        crate::core::Orientation::Vertical
    }

    /// The shape this plot type's legend swatch should take.
    ///
    /// A key that shows a line for a cloud of markers is worse than no key: it
    /// tells the reader something untrue about the picture. Answer with the mark
    /// the geometry is actually made of — the default, [`LegendKey::Line`], is
    /// right for anything stroked.
    fn legend_key(&self) -> LegendKey {
        LegendKey::Line
    }

    /// The colour scale this series wants explained, if it has one.
    ///
    /// A plot type that maps values to colours is unreadable without it — the
    /// reader can see that one hexagon is teal and another yellow and has no way
    /// to find out what either means. Return the same
    /// [`ColorbarRequest`](crate::render::colorbar::ColorbarRequest) a heatmap
    /// or contour returns and the margin reservation, the raster draw and the
    /// SVG draw all pick it up together.
    fn colorbar(
        &self,
        _theme: &crate::render::Theme,
    ) -> Option<crate::render::colorbar::ColorbarRequest> {
        None
    }

    /// Whether this geometry is drawn from the `y = 0` baseline, so the
    /// baseline must keep touching the axis edge (matplotlib `sticky_edges`).
    ///
    /// True for anything bar-shaped; false — the default — for everything else.
    /// It is answered here rather than in the bounds code so that a computed
    /// plot type pins its baseline by the same one-method mechanism it uses to
    /// answer every other "what does this geometry need?" question.
    fn pins_zero_baseline(&self) -> bool {
        false
    }
}

/// The swatch shape a [`ComputedSeries`] wants in the legend.
///
/// Deliberately a small vocabulary of *marks*, not a copy of
/// [`LegendItemType`](crate::core::legend::LegendItemType): the plot type says
/// what it draws with, and the legend keeps sole ownership of how a key for that
/// mark is styled and measured.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LegendKey {
    /// Stroked geometry — a rug's ticks, a dendrogram's arms.
    #[default]
    Line,
    /// Discrete markers — a strip or swarm cloud.
    Marker,
    /// A filled patch.
    Patch,
    /// No key at all. For geometry whose meaning is a colour scale rather than a
    /// series identity: a single swatch would have to pick one colour out of a
    /// colormap and imply the rest.
    None,
}

/// Trait for filled shapes with optional edge styling
///
/// This trait provides a consistent interface for plot elements that have
/// both a fill color and an edge (stroke). Examples include histogram bars,
/// box plot boxes, violin fills, and bar chart bars.
///
/// # Design Philosophy
///
/// The trait follows matplotlib/seaborn conventions:
/// - Fill color is the primary color (from theme palette or explicit)
/// - Edge color defaults to a darker version of the fill (30% darker)
/// - Edge width defaults to `patch.linewidth` (0.8pt in matplotlib)
/// - Alpha affects fill transparency
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::plots::traits::StyledShape;
/// use ruviz::render::Color;
///
/// struct Bar {
///     fill: Color,
///     edge: Option<Color>,
///     edge_width: f32,
///     alpha: f32,
/// }
///
/// impl StyledShape for Bar {
///     fn fill_color(&self) -> Color {
///         self.fill
///     }
///
///     fn edge_color(&self) -> Option<Color> {
///         self.edge
///     }
///
///     fn edge_width(&self) -> f32 {
///         self.edge_width
///     }
///
///     fn alpha(&self) -> f32 {
///         self.alpha
///     }
/// }
/// ```
pub trait StyledShape {
    /// Get the fill color for this shape
    ///
    /// This is the primary color used to fill the interior of the shape.
    fn fill_color(&self) -> Color;

    /// Get the explicit edge color, if set
    ///
    /// Returns `None` to use auto-derived edge color (typically 30% darker
    /// than fill color). Returns `Some(color)` for explicit edge color.
    fn edge_color(&self) -> Option<Color>;

    /// Get the edge (stroke) width in points
    ///
    /// Default is 0.8pt (matplotlib's `patch.linewidth`).
    fn edge_width(&self) -> f32;

    /// Get the fill alpha (opacity)
    ///
    /// Returns a value between 0.0 (fully transparent) and 1.0 (fully opaque).
    fn alpha(&self) -> f32;

    /// Get the resolved edge color
    ///
    /// If an explicit edge color is set, returns that. Otherwise,
    /// returns a 30% darker version of the fill color.
    ///
    /// This is a convenience method that can be overridden for custom behavior.
    fn resolved_edge_color(&self) -> Color {
        self.edge_color()
            .unwrap_or_else(|| self.fill_color().darken(0.3))
    }

    /// Get the fill color with alpha applied
    ///
    /// Returns the fill color with the shape's alpha value applied.
    fn fill_color_with_alpha(&self) -> Color {
        self.fill_color().with_alpha(self.alpha())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `PlotArea` and `map_data_to_pixels_scaled` are the two ways the crate
    /// projects a data point. They have to be the same projection: the SVG path
    /// uses one and the trait-rendered plot types use the other, and a heatmap
    /// whose cells disagreed with the ticks beside them is exactly what happens
    /// when they drift.
    #[test]
    fn plot_area_matches_the_scaled_pixel_mapper() {
        let rect = tiny_skia::Rect::from_xywh(24.0, 12.0, 480.0, 360.0).unwrap();
        let area = PlotArea::new(24.0, 12.0, 480.0, 360.0, 1.0, 1000.0, 0.5, 200.0)
            .with_scales(AxisScale::Log, AxisScale::Log);

        for (x, y) in [(1.0, 0.5), (10.0, 7.0), (250.0, 199.0), (1000.0, 200.0)] {
            let (ax, ay) = area.data_to_screen(x, y);
            let (mx, my) = crate::render::skia::map_data_to_pixels_scaled(
                x,
                y,
                1.0,
                1000.0,
                0.5,
                200.0,
                rect,
                &AxisScale::Log,
                &AxisScale::Log,
            );
            assert!(
                (ax - mx).abs() < 1e-4,
                "x differs at ({x}, {y}): {ax} vs {mx}"
            );
            assert!(
                (ay - my).abs() < 1e-4,
                "y differs at ({x}, {y}): {ay} vs {my}"
            );
        }
    }

    #[test]
    fn plot_area_defaults_to_a_linear_projection() {
        let area = PlotArea::new(0.0, 0.0, 100.0, 100.0, 0.0, 10.0, 0.0, 10.0);
        assert_eq!(area.x_scale, AxisScale::Linear);
        assert_eq!(area.y_scale, AxisScale::Linear);
        let (x, _) = area.data_to_screen(5.0, 5.0);
        assert!((x - 50.0).abs() < 1e-4);
    }

    #[test]
    fn plot_area_rejects_samples_a_log_axis_cannot_place() {
        let area = PlotArea::new(0.0, 0.0, 100.0, 100.0, 1.0, 100.0, 1.0, 100.0)
            .with_scales(AxisScale::Log, AxisScale::Log);
        assert!(area.try_data_to_screen(10.0, 10.0).is_some());
        assert!(area.try_data_to_screen(0.0, 10.0).is_none());
        assert!(area.try_data_to_screen(10.0, -1.0).is_none());
        assert!(area.try_data_to_screen(f64::NAN, 10.0).is_none());
    }

    #[test]
    fn plot_area_splits_a_run_at_every_unplaceable_point() {
        let area = PlotArea::new(0.0, 0.0, 100.0, 100.0, 1.0, 100.0, 1.0, 100.0)
            .with_scales(AxisScale::Log, AxisScale::Linear);
        let runs = area.project_subpaths([
            (1.0, 1.0),
            (2.0, 2.0),
            (0.0, 3.0),
            (4.0, 4.0),
            (-1.0, 5.0),
            (6.0, 6.0),
            (7.0, 7.0),
        ]);
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].len(), 2);
        assert_eq!(runs[1].len(), 1);
        assert_eq!(runs[2].len(), 2);

        // The polygon projection keeps the same survivors, without the split.
        let points = area.project_points([(1.0, 1.0), (0.0, 3.0), (4.0, 4.0)]);
        assert_eq!(points.len(), 2);
    }

    #[test]
    fn plot_area_fill_baseline_falls_back_to_the_axis_floor_on_a_log_axis() {
        let linear = PlotArea::new(0.0, 10.0, 100.0, 200.0, 0.0, 10.0, 0.0, 100.0);
        assert!(
            (linear.fill_baseline_y() - 210.0).abs() < 1e-4,
            "zero is on the axis"
        );

        let log = PlotArea::new(0.0, 10.0, 100.0, 200.0, 0.0, 10.0, 1.0, 100.0)
            .with_scales(AxisScale::Linear, AxisScale::Log);
        assert!(
            (log.fill_baseline_y() - 210.0).abs() < 1e-4,
            "a log axis has no zero, so the fill bottoms out on its floor"
        );
    }

    #[test]
    fn plot_area_pins_an_unplaceable_edge_to_the_axis_floor() {
        let area = PlotArea::new(20.0, 0.0, 100.0, 100.0, 1.0, 100.0, 1.0, 100.0)
            .with_scales(AxisScale::Log, AxisScale::Linear);
        // A cell extent that starts at zero is clipped by the axis, not lost.
        let (x, _) = area.edge_data_to_screen(0.0, 50.0);
        assert!((x - 20.0).abs() < 1e-4, "expected the axis floor, got {x}");
        assert!(area.data_to_screen(0.0, 50.0).0.is_nan());
    }

    #[test]
    fn test_plot_area_creation() {
        let area = PlotArea::new(100.0, 50.0, 600.0, 400.0, 0.0, 10.0, 0.0, 100.0);

        assert_eq!(area.x, 100.0);
        assert_eq!(area.y, 50.0);
        assert_eq!(area.width, 600.0);
        assert_eq!(area.height, 400.0);
        assert_eq!(area.x_min, 0.0);
        assert_eq!(area.x_max, 10.0);
        assert_eq!(area.y_min, 0.0);
        assert_eq!(area.y_max, 100.0);
    }

    #[test]
    fn test_plot_area_data_to_screen() {
        let area = PlotArea::new(100.0, 50.0, 600.0, 400.0, 0.0, 10.0, 0.0, 100.0);

        // Bottom-left corner in data = top-right in screen (y inverted)
        let (sx, sy) = area.data_to_screen(0.0, 0.0);
        assert!((sx - 100.0).abs() < 0.01);
        assert!((sy - 450.0).abs() < 0.01); // y=50+400=450 (bottom of plot area)

        // Top-right corner in data = bottom-left in screen (y inverted)
        let (sx, sy) = area.data_to_screen(10.0, 100.0);
        assert!((sx - 700.0).abs() < 0.01); // x=100+600=700
        assert!((sy - 50.0).abs() < 0.01); // y=50 (top of plot area)

        // Center
        let (sx, sy) = area.data_to_screen(5.0, 50.0);
        assert!((sx - 400.0).abs() < 0.01); // x=100+300=400
        assert!((sy - 250.0).abs() < 0.01); // y=50+200=250
    }

    #[test]
    fn test_plot_area_screen_to_data() {
        let area = PlotArea::new(100.0, 50.0, 600.0, 400.0, 0.0, 10.0, 0.0, 100.0);

        // Round-trip test
        let (data_x, data_y) = (5.0, 50.0);
        let (sx, sy) = area.data_to_screen(data_x, data_y);
        let (rx, ry) = area.screen_to_data(sx, sy);

        assert!((rx - data_x).abs() < 0.01);
        assert!((ry - data_y).abs() < 0.01);
    }

    #[test]
    fn test_plot_area_contains_data() {
        let area = PlotArea::new(100.0, 50.0, 600.0, 400.0, 0.0, 10.0, 0.0, 100.0);

        assert!(area.contains_data(5.0, 50.0)); // Inside
        assert!(area.contains_data(0.0, 0.0)); // Corner
        assert!(area.contains_data(10.0, 100.0)); // Corner
        assert!(!area.contains_data(-1.0, 50.0)); // Outside x
        assert!(!area.contains_data(5.0, 150.0)); // Outside y
    }

    #[test]
    fn test_plot_area_center() {
        let area = PlotArea::new(100.0, 50.0, 600.0, 400.0, 0.0, 10.0, 0.0, 100.0);

        let (cx, cy) = area.center();
        assert!((cx - 400.0).abs() < 0.01);
        assert!((cy - 250.0).abs() < 0.01);

        let (dx, dy) = area.data_center();
        assert!((dx - 5.0).abs() < 0.01);
        assert!((dy - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_plot_area_zero_range() {
        // Test handling of zero-range data (single point)
        let area = PlotArea::new(100.0, 50.0, 600.0, 400.0, 5.0, 5.0, 50.0, 50.0);

        // Should map to center when range is zero
        let (sx, sy) = area.data_to_screen(5.0, 50.0);
        assert!((sx - 400.0).abs() < 0.01); // Center x
        assert!((sy - 250.0).abs() < 0.01); // Center y
    }

    // Tests for StyledShape trait
    struct TestShape {
        fill: Color,
        edge: Option<Color>,
        edge_width: f32,
        alpha: f32,
    }

    impl StyledShape for TestShape {
        fn fill_color(&self) -> Color {
            self.fill
        }

        fn edge_color(&self) -> Option<Color> {
            self.edge
        }

        fn edge_width(&self) -> f32 {
            self.edge_width
        }

        fn alpha(&self) -> f32 {
            self.alpha
        }
    }

    #[test]
    fn test_styled_shape_explicit_edge() {
        let shape = TestShape {
            fill: Color::BLUE,
            edge: Some(Color::RED),
            edge_width: 1.5,
            alpha: 0.8,
        };

        assert_eq!(shape.fill_color(), Color::BLUE);
        assert_eq!(shape.edge_color(), Some(Color::RED));
        assert_eq!(shape.resolved_edge_color(), Color::RED);
        assert_eq!(shape.edge_width(), 1.5);
        assert_eq!(shape.alpha(), 0.8);
    }

    #[test]
    fn test_styled_shape_auto_edge() {
        let shape = TestShape {
            fill: Color::from_rgb(100, 150, 200),
            edge: None,
            edge_width: 0.8,
            alpha: 1.0,
        };

        // Auto-derived edge should be 30% darker
        let edge = shape.resolved_edge_color();
        assert_eq!(edge.r, 70); // 100 * 0.7
        assert_eq!(edge.g, 105); // 150 * 0.7
        assert_eq!(edge.b, 140); // 200 * 0.7
    }

    #[test]
    fn test_axis_scale_support_only_refuses_non_linear_scales() {
        let unsupported = AxisScaleSupport::Unsupported("positions bars by category index");

        // Linear is what every renderer already draws, so it is never refused.
        assert!(unsupported.accepts(&AxisScale::Linear));
        assert!(!unsupported.accepts(&AxisScale::Log));
        assert!(!unsupported.accepts(&AxisScale::SymLog { linthresh: 1.0 }));

        for support in [AxisScaleSupport::Scaled, AxisScaleSupport::Independent] {
            assert!(support.accepts(&AxisScale::Linear));
            assert!(support.accepts(&AxisScale::Log));
            assert!(support.accepts(&AxisScale::SymLog { linthresh: 1.0 }));
        }
    }

    #[test]
    fn test_axis_scale_support_carries_a_reason_only_when_it_refuses() {
        assert_eq!(
            AxisScaleSupport::Unsupported("because").rejection_reason(),
            Some("because")
        );
        assert_eq!(AxisScaleSupport::Scaled.rejection_reason(), None);
        assert_eq!(AxisScaleSupport::Independent.rejection_reason(), None);
    }

    #[test]
    fn test_styled_shape_fill_with_alpha() {
        let shape = TestShape {
            fill: Color::from_rgb(100, 150, 200),
            edge: None,
            edge_width: 0.8,
            alpha: 0.5,
        };

        let fill_with_alpha = shape.fill_color_with_alpha();
        assert_eq!(fill_with_alpha.r, 100);
        assert_eq!(fill_with_alpha.g, 150);
        assert_eq!(fill_with_alpha.b, 200);
        assert_eq!(fill_with_alpha.a, 127); // 255 * 0.5 ≈ 127
    }
}
