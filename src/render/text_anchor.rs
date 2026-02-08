/// Shared text-anchor geometry helpers used by raster and vector renderers.
///
/// Static layout in ruviz uses:
/// - horizontal text anchors at top origin,
/// - rotated axis-label anchors at geometric center.
///
/// These helpers convert those layout anchors into draw coordinates.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextPlacementMetrics {
    pub width: f32,
    pub height: f32,
    pub baseline_from_top: f32,
}

impl TextPlacementMetrics {
    pub fn new(width: f32, height: f32, baseline_from_top: f32) -> Self {
        let width = width.max(0.0);
        let height = height.max(0.0);
        let baseline_from_top = baseline_from_top.clamp(0.0, height.max(0.0));
        Self {
            width,
            height,
            baseline_from_top,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAnchorKind {
    TopLeft,
    TopCenter,
    Center,
}

/// Convert an anchor coordinate and rendered bounds into top-left draw coordinates.
pub fn anchor_to_top_left(
    anchor_x: f32,
    anchor_y: f32,
    rendered_width: f32,
    rendered_height: f32,
    anchor: TextAnchorKind,
) -> (f32, f32) {
    match anchor {
        TextAnchorKind::TopLeft => (anchor_x, anchor_y),
        TextAnchorKind::TopCenter => (anchor_x - rendered_width / 2.0, anchor_y),
        TextAnchorKind::Center => (
            anchor_x - rendered_width / 2.0,
            anchor_y - rendered_height / 2.0,
        ),
    }
}

/// Convert a top-origin anchor y to baseline y.
pub fn top_anchor_to_baseline(anchor_top_y: f32, metrics: TextPlacementMetrics) -> f32 {
    anchor_top_y + metrics.baseline_from_top
}

/// Convert a center-origin anchor y to baseline y.
pub fn center_anchor_to_baseline(anchor_center_y: f32, metrics: TextPlacementMetrics) -> f32 {
    anchor_center_y - metrics.height / 2.0 + metrics.baseline_from_top
}
