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
use crate::render::{Color, SkiaRenderer, Theme};

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
