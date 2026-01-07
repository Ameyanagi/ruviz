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

use crate::core::error::Result;
use crate::core::transform::CoordinateTransform;
use crate::render::{Color, SkiaRenderer, Theme};

/// Defines the plot area for rendering
///
/// Represents the rectangular region where plot data should be rendered,
/// including the coordinate transformation parameters.
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
}

impl PlotArea {
    /// Create a new plot area with the given bounds
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
        }
    }

    /// Create a [`CoordinateTransform`] from this plot area.
    ///
    /// This allows using the unified coordinate transformation API
    /// while maintaining backward compatibility with `PlotArea`.
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

    /// Transform data coordinates to screen coordinates
    ///
    /// # Arguments
    /// * `data_x` - X coordinate in data space
    /// * `data_y` - Y coordinate in data space
    ///
    /// # Returns
    /// (screen_x, screen_y) in pixel coordinates
    #[inline]
    pub fn data_to_screen(&self, data_x: f64, data_y: f64) -> (f32, f32) {
        self.to_transform().data_to_screen(data_x, data_y)
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
        self.to_transform().screen_to_data(screen_x, screen_y)
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
            fill: Color::new(100, 150, 200),
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
    fn test_styled_shape_fill_with_alpha() {
        let shape = TestShape {
            fill: Color::new(100, 150, 200),
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
