//! Rendering backend and styling

pub mod backend;
pub mod color;
pub mod cosmic_text_renderer;
#[cfg(feature = "gpu")]
pub mod gpu;
#[cfg(feature = "parallel")]
pub mod parallel;
pub mod pooled;
pub mod primitives;
#[cfg(feature = "simd")]
pub mod simd;
pub mod skia;
pub mod style;
pub mod text;
pub mod theme;

pub use backend::Renderer;
pub use color::{Color, ColorError, ColorMap};
pub use cosmic_text_renderer::CosmicTextRenderer;
#[cfg(feature = "gpu")]
pub use gpu::{GpuBackend, GpuRenderer, initialize_gpu_backend, is_gpu_available};
#[cfg(feature = "parallel")]
pub use parallel::{
    DetailedPerformanceInfo, ParallelConfig, ParallelRenderer, PerformanceStats, SeriesRenderData,
};
pub use pooled::{LineSegment, PooledRenderer, PooledRendererStats, get_pooled_renderer};
pub use primitives::{Arc, Arrow, Polygon, Wedge};
#[cfg(feature = "simd")]
pub use simd::{CoordinateBounds, PixelViewport, SIMDPerformanceInfo, SIMDTransformer};
pub use skia::SkiaRenderer;
pub use style::{LineStyle, MarkerStyle};
pub use text::{FontConfig, FontFamily, FontStyle, FontWeight};
pub use text::{TextRenderer, get_font_system, get_swash_cache, initialize_text_system};
pub use theme::{Theme, ThemeBuilder, ThemeVariant};

/// Viewport defining visible data bounds with optional margin
///
/// Used for culling data points outside the visible area during interactive
/// rendering (zoom/pan operations).
#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    /// Visible X range (min, max)
    pub x_range: (f64, f64),
    /// Visible Y range (min, max)
    pub y_range: (f64, f64),
    /// Margin factor (0.0 = no margin, 0.1 = 10% margin on each side)
    pub margin: f64,
}

impl Viewport {
    /// Create a new viewport with the specified ranges
    pub fn new(x_min: f64, x_max: f64, y_min: f64, y_max: f64) -> Self {
        Self {
            x_range: (x_min, x_max),
            y_range: (y_min, y_max),
            margin: 0.0,
        }
    }

    /// Create a viewport with margin
    ///
    /// Margin extends the effective viewport bounds to include nearby points.
    /// A margin of 0.1 means 10% extra on each side.
    pub fn with_margin(mut self, margin: f64) -> Self {
        self.margin = margin.max(0.0);
        self
    }

    /// Get effective X range including margin
    pub fn effective_x_range(&self) -> (f64, f64) {
        let width = self.x_range.1 - self.x_range.0;
        let margin_amount = width * self.margin;
        (
            self.x_range.0 - margin_amount,
            self.x_range.1 + margin_amount,
        )
    }

    /// Get effective Y range including margin
    pub fn effective_y_range(&self) -> (f64, f64) {
        let height = self.y_range.1 - self.y_range.0;
        let margin_amount = height * self.margin;
        (
            self.y_range.0 - margin_amount,
            self.y_range.1 + margin_amount,
        )
    }

    /// Check if a point is within the viewport (including margin)
    pub fn contains(&self, x: f64, y: f64) -> bool {
        let (x_min, x_max) = self.effective_x_range();
        let (y_min, y_max) = self.effective_y_range();
        x >= x_min && x <= x_max && y >= y_min && y <= y_max
    }
}

/// Cull points to only those visible within the viewport
///
/// Filters out points that fall outside the visible viewport bounds,
/// reducing the number of points that need to be transformed and rendered.
///
/// # Arguments
///
/// * `points` - Slice of (x, y) coordinate pairs
/// * `viewport` - The visible viewport bounds
///
/// # Returns
///
/// Vec of (x, y) pairs that are within the viewport bounds
///
/// # Example
///
/// ```rust,no_run
/// use ruviz::render::{Viewport, cull_to_viewport};
///
/// let points = vec![(0.0, 0.0), (5.0, 5.0), (100.0, 100.0)];
/// let viewport = Viewport::new(0.0, 10.0, 0.0, 10.0);
/// let visible = cull_to_viewport(&points, &viewport);
/// // visible contains only (0.0, 0.0) and (5.0, 5.0)
/// ```
pub fn cull_to_viewport(points: &[(f64, f64)], viewport: &Viewport) -> Vec<(f64, f64)> {
    points
        .iter()
        .copied()
        .filter(|(x, y)| viewport.contains(*x, *y))
        .collect()
}

/// Cull separate X and Y arrays to only points visible within the viewport
///
/// Similar to `cull_to_viewport` but works with separate X and Y arrays,
/// which is common in plotting APIs.
///
/// # Returns
///
/// Tuple of (culled_x, culled_y) Vecs containing only visible points
pub fn cull_xy_to_viewport(
    x_data: &[f64],
    y_data: &[f64],
    viewport: &Viewport,
) -> (Vec<f64>, Vec<f64>) {
    let len = x_data.len().min(y_data.len());
    let mut result_x = Vec::with_capacity(len);
    let mut result_y = Vec::with_capacity(len);

    for i in 0..len {
        if viewport.contains(x_data[i], y_data[i]) {
            result_x.push(x_data[i]);
            result_y.push(y_data[i]);
        }
    }

    (result_x, result_y)
}

#[cfg(test)]
mod viewport_tests {
    use super::*;

    #[test]
    fn test_viewport_contains() {
        let viewport = Viewport::new(0.0, 10.0, 0.0, 10.0);
        assert!(viewport.contains(5.0, 5.0));
        assert!(viewport.contains(0.0, 0.0));
        assert!(viewport.contains(10.0, 10.0));
        assert!(!viewport.contains(-1.0, 5.0));
        assert!(!viewport.contains(11.0, 5.0));
        assert!(!viewport.contains(5.0, -1.0));
        assert!(!viewport.contains(5.0, 11.0));
    }

    #[test]
    fn test_viewport_with_margin() {
        let viewport = Viewport::new(0.0, 10.0, 0.0, 10.0).with_margin(0.1);
        // Margin of 0.1 means 10% of width (1.0) extra on each side
        assert!(viewport.contains(-0.5, 5.0)); // Within margin
        assert!(viewport.contains(10.5, 5.0)); // Within margin
        assert!(!viewport.contains(-1.5, 5.0)); // Outside margin
        assert!(!viewport.contains(11.5, 5.0)); // Outside margin
    }

    #[test]
    fn test_cull_to_viewport() {
        let points = vec![
            (0.0, 0.0),
            (5.0, 5.0),
            (10.0, 10.0),
            (15.0, 15.0),
            (-5.0, 5.0),
        ];
        let viewport = Viewport::new(0.0, 10.0, 0.0, 10.0);
        let culled = cull_to_viewport(&points, &viewport);

        assert_eq!(culled.len(), 3);
        assert!(culled.contains(&(0.0, 0.0)));
        assert!(culled.contains(&(5.0, 5.0)));
        assert!(culled.contains(&(10.0, 10.0)));
        assert!(!culled.contains(&(15.0, 15.0)));
        assert!(!culled.contains(&(-5.0, 5.0)));
    }

    #[test]
    fn test_cull_xy_to_viewport() {
        let x = vec![0.0, 5.0, 10.0, 15.0, -5.0];
        let y = vec![0.0, 5.0, 10.0, 15.0, 5.0];
        let viewport = Viewport::new(0.0, 10.0, 0.0, 10.0);
        let (culled_x, culled_y) = cull_xy_to_viewport(&x, &y, &viewport);

        assert_eq!(culled_x.len(), 3);
        assert_eq!(culled_y.len(), 3);
        assert_eq!(culled_x[0], 0.0);
        assert_eq!(culled_x[1], 5.0);
        assert_eq!(culled_x[2], 10.0);
    }

    #[test]
    fn test_culled_is_subset() {
        let points: Vec<(f64, f64)> = (0..100).map(|i| (i as f64, (i * 2) as f64)).collect();
        let viewport = Viewport::new(20.0, 40.0, 40.0, 80.0);
        let culled = cull_to_viewport(&points, &viewport);

        // All culled points must be in original
        for point in &culled {
            assert!(points.contains(point));
        }

        // All culled points must be within viewport
        for (x, y) in &culled {
            assert!(viewport.contains(*x, *y));
        }
    }
}
