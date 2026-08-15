//! Scatter plot configuration
//!
//! Provides [`ScatterConfig`] for configuring scatter plot appearance.

use crate::core::style_utils::defaults;
use crate::plots::traits::PlotConfig;
use crate::render::{Color, MarkerStyle};

/// Configuration for scatter plots
///
/// Controls the appearance of scatter series including marker style,
/// size, color, and optional edge styling.
///
/// # Example
///
/// ```rust
/// use ruviz::plots::basic::ScatterConfig;
/// use ruviz::render::MarkerStyle;
///
/// let config = ScatterConfig::new()
///     .marker(MarkerStyle::Diamond)
///     .size(10.0)
///     .edge_width(1.5);
/// ```
#[derive(Debug, Clone)]
pub struct ScatterConfig {
    /// Marker style (default: Circle)
    pub marker: MarkerStyle,
    /// Marker size in points (default: 6.0)
    pub size: f32,
    /// Marker fill color (None = auto from palette)
    pub color: Option<Color>,
    /// Marker alpha/transparency (0.0-1.0)
    pub alpha: f32,
    /// Edge color (None = auto-darken from fill)
    pub edge_color: Option<Color>,
    /// Edge width in points (default: `defaults::PATCH_LINE_WIDTH`, 0.8)
    pub edge_width: f32,
    /// Whether to show an edge around markers (default: **false**)
    ///
    /// Off by default, matching matplotlib's `scatter(..., edgecolors='face')`:
    /// a marker renders in exactly its series colour and nothing else.
    ///
    /// A *contrasting* rim cannot be a default. It is drawn over the marker's
    /// own boundary, so on overlapping data every marker darkens its
    /// neighbours' fill — a saturated band of a dense scatter reads ~30% darker
    /// than the palette colour and no longer matches its own legend key. On
    /// small markers the rim is most of the marker: at `size(2.0)` a circle is
    /// 5 rim pixels to 1 fill pixel, so the point reads as the rim colour
    /// rather than the series colour.
    ///
    /// Turn it on with `.show_edge(true)` (fill darkened by 30%), or by naming
    /// an [`edge_color`](ScatterConfig::edge_color) /
    /// [`edge_width`](ScatterConfig::edge_width). It pays off on sparse,
    /// large-marker plots, which is where matplotlib users reach for
    /// `edgecolors` too.
    ///
    /// Only the closed filled styles have an interior for a rim to bound, so
    /// this has no effect on the open styles or the line-drawn ones (plus,
    /// cross, star) — see [`MarkerStyle::takes_edge`].
    pub show_edge: bool,
    /// Whether to aggregate points into a plot-area pixel density grid.
    ///
    /// This is an opt-in approximation for very large scatters. Each occupied
    /// pixel is drawn in the series color with the alpha produced by repeatedly
    /// compositing the configured point alpha. Marker shape, size, edges, and
    /// antialiasing are intentionally not reproduced.
    pub density: bool,
}

impl Default for ScatterConfig {
    fn default() -> Self {
        Self {
            marker: MarkerStyle::Circle,
            size: 6.0,
            color: None,
            alpha: 1.0,
            edge_color: None,
            // A marker is a patch, so it gets the same edge width bars,
            // histograms and boxes get. Thinner than this and the rim is
            // sub-pixel at screen DPI, i.e. an edge you cannot see.
            edge_width: defaults::PATCH_LINE_WIDTH,
            // No rim unless asked for. A contrasting rim over a marker's own
            // boundary darkens whatever it overlaps, so on dense scatter it
            // shifts the whole series away from its palette colour, and on
            // markers of a few points it *is* the marker. See `show_edge`.
            show_edge: false,
            density: false,
        }
    }
}

impl PlotConfig for ScatterConfig {}

impl ScatterConfig {
    /// Create a new scatter configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set marker style
    ///
    /// # Arguments
    /// * `marker` - Marker style (Circle, Square, Triangle, etc.)
    pub fn marker(mut self, marker: MarkerStyle) -> Self {
        self.marker = marker;
        self
    }

    /// Set marker size in points
    ///
    /// # Arguments
    /// * `size` - Marker size (minimum 0.1)
    pub fn size(mut self, size: f32) -> Self {
        self.size = size.max(0.1);
        self
    }

    /// Set marker fill color
    ///
    /// # Arguments
    /// * `color` - Color for the marker fill
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set transparency
    ///
    /// # Arguments
    /// * `alpha` - Transparency value (0.0 = transparent, 1.0 = opaque)
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Set edge color explicitly
    ///
    /// If not set, edge color is auto-derived by darkening the fill color.
    ///
    /// Turns [`show_edge`](ScatterConfig::show_edge) on: naming an edge colour
    /// is asking for an edge. Call `.show_edge(false)` *afterwards* to override.
    ///
    /// Ignored by marker styles with no interior; see
    /// [`show_edge`](ScatterConfig::show_edge).
    ///
    /// # Arguments
    /// * `color` - Color for the marker edge
    pub fn edge_color(mut self, color: Color) -> Self {
        self.edge_color = Some(color);
        self.show_edge = true;
        self
    }

    /// Set edge width in points
    ///
    /// Turns [`show_edge`](ScatterConfig::show_edge) on for a positive width
    /// and off for a zero one, so a width alone is enough to get an edge.
    ///
    /// # Arguments
    /// * `width` - Edge line width in points (minimum 0.0)
    pub fn edge_width(mut self, width: f32) -> Self {
        self.edge_width = width.max(0.0);
        self.show_edge = self.edge_width > 0.0;
        self
    }

    /// Set whether to show edges around markers
    pub fn show_edge(mut self, show: bool) -> Self {
        self.show_edge = show;
        self
    }

    /// Enable or disable plot-area density aggregation.
    ///
    /// Density rendering makes work scale with plot pixels rather than
    /// points: counts are aggregated per pixel, spread over the marker's
    /// footprint, and colored at the scatter-equivalent alpha, keeping the
    /// exact render's silhouette. Disabled by default; most useful for
    /// scatters containing hundreds of thousands or millions of points. See
    /// `PlotSeriesBuilder::density` for which marker shapes the footprint
    /// models exactly.
    pub fn density(mut self, density: bool) -> Self {
        self.density = density;
        self
    }

    /// The configured marker edge as `(colour override, width in points)`.
    ///
    /// Returns `None` when no edge should be drawn — either
    /// [`show_edge(false)`](ScatterConfig::show_edge) or a zero
    /// [`edge_width`](ScatterConfig::edge_width), since an invisible rim is no
    /// rim. A `None` colour means "derive from the fill", which cannot be done
    /// here: auto-palette series only learn their fill at render time.
    ///
    /// The width stays in points; the renderer scales it to device pixels so
    /// the rim is the same physical thickness at every DPI.
    pub fn resolved_edge_spec(&self) -> Option<(Option<Color>, f32)> {
        super::marker_edge_spec(self.show_edge, self.edge_color, self.edge_width)
    }

    /// Convenience method to set size (alias for `size()`)
    ///
    /// Matches matplotlib's `s` parameter naming.
    pub fn s(self, size: f32) -> Self {
        self.size(size)
    }

    /// Convenience method to set color (alias for `color()`)
    ///
    /// Matches matplotlib's `c` parameter naming.
    pub fn c(self, color: Color) -> Self {
        self.color(color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ScatterConfig::default();
        assert!(matches!(config.marker, MarkerStyle::Circle));
        assert!((config.size - 6.0).abs() < f32::EPSILON);
        assert!(config.color.is_none());
        assert!((config.alpha - 1.0).abs() < f32::EPSILON);
        assert!(
            !config.show_edge,
            "a default marker is exactly its series colour: a contrasting rim \
             darkens whatever it overlaps and swallows small markers"
        );
        assert!(!config.density, "density rendering must be opt-in");
    }

    #[test]
    fn test_builder_methods() {
        let config = ScatterConfig::new()
            .marker(MarkerStyle::Square)
            .size(12.0)
            .color(Color::BLUE)
            .alpha(0.8)
            .edge_color(Color::BLACK)
            .edge_width(1.5)
            .density(true);

        assert!(matches!(config.marker, MarkerStyle::Square));
        assert!((config.size - 12.0).abs() < f32::EPSILON);
        assert_eq!(config.color, Some(Color::BLUE));
        assert!((config.alpha - 0.8).abs() < f32::EPSILON);
        assert_eq!(config.edge_color, Some(Color::BLACK));
        assert!((config.edge_width - 1.5).abs() < f32::EPSILON);
        assert!(config.density);
    }

    #[test]
    fn test_size_clamping() {
        let config = ScatterConfig::new().size(-5.0);
        assert!((config.size - 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn test_resolved_edge_spec_defaults_to_no_rim() {
        assert_eq!(
            ScatterConfig::default().resolved_edge_spec(),
            None,
            "a plain .scatter() must ask for no rim at all, so overlapping \
             markers cannot darken each other and small markers survive"
        );
    }

    #[test]
    fn test_resolved_edge_spec_is_enabled_by_show_edge() {
        assert_eq!(
            ScatterConfig::new().show_edge(true).resolved_edge_spec(),
            Some((None, defaults::PATCH_LINE_WIDTH)),
            "show_edge(true) must ask for a rim derived from the fill"
        );
    }

    #[test]
    fn test_resolved_edge_spec_is_enabled_by_asking_for_a_width() {
        assert_eq!(
            ScatterConfig::new().edge_width(1.5).resolved_edge_spec(),
            Some((None, 1.5)),
            "a width alone must produce an edge derived from the fill"
        );
    }

    #[test]
    fn test_resolved_edge_spec_is_enabled_by_asking_for_a_colour() {
        assert_eq!(
            ScatterConfig::new()
                .edge_color(Color::BLACK)
                .resolved_edge_spec(),
            Some((Some(Color::BLACK), defaults::PATCH_LINE_WIDTH)),
            "an edge colour alone must produce an edge at the default width"
        );
    }

    #[test]
    fn test_resolved_edge_spec_honours_show_edge() {
        assert_eq!(
            ScatterConfig::new()
                .edge_width(1.5)
                .show_edge(false)
                .resolved_edge_spec(),
            None,
            "show_edge(false) must reach the renderer as 'no edge'"
        );
    }

    #[test]
    fn test_resolved_edge_spec_is_disabled_by_zero_width() {
        assert_eq!(
            ScatterConfig::new()
                .edge_color(Color::BLACK)
                .edge_width(0.0)
                .resolved_edge_spec(),
            None,
            "edge_width(0.0) must switch the edge off, not floor it to a hairline"
        );
    }

    #[test]
    fn test_resolved_edge_spec_keeps_an_explicit_colour() {
        assert_eq!(
            ScatterConfig::new()
                .edge_color(Color::BLACK)
                .edge_width(1.5)
                .resolved_edge_spec(),
            Some((Some(Color::BLACK), 1.5)),
            "an explicit edge colour and width must survive to the renderer"
        );
    }

    #[test]
    fn test_matplotlib_aliases() {
        let config = ScatterConfig::new().s(10.0).c(Color::GREEN);

        assert!((config.size - 10.0).abs() < f32::EPSILON);
        assert_eq!(config.color, Some(Color::GREEN));
    }
}
