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
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

pub(crate) fn aligned_text_anchor(
    align: crate::core::TextAlign,
    valign: crate::core::TextVAlign,
) -> TextAnchorKind {
    use crate::core::{TextAlign, TextVAlign};

    match (align, valign) {
        (TextAlign::Left, TextVAlign::Top) => TextAnchorKind::TopLeft,
        (TextAlign::Center, TextVAlign::Top) => TextAnchorKind::TopCenter,
        (TextAlign::Right, TextVAlign::Top) => TextAnchorKind::TopRight,
        (TextAlign::Left, TextVAlign::Middle) => TextAnchorKind::CenterLeft,
        (TextAlign::Center, TextVAlign::Middle) => TextAnchorKind::Center,
        (TextAlign::Right, TextVAlign::Middle) => TextAnchorKind::CenterRight,
        (TextAlign::Left, TextVAlign::Bottom) => TextAnchorKind::BottomLeft,
        (TextAlign::Center, TextVAlign::Bottom) => TextAnchorKind::BottomCenter,
        (TextAlign::Right, TextVAlign::Bottom) => TextAnchorKind::BottomRight,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct AnnotationTextLayout {
    pub text_x: f32,
    pub text_y: f32,
    pub box_x: f32,
    pub box_y: f32,
    pub box_width: f32,
    pub box_height: f32,
    /// Screen-coordinate rotation in degrees. Negative is visual counter-clockwise.
    pub rotation: f32,
}

pub(crate) fn annotation_text_layout(
    metrics: TextPlacementMetrics,
    align: crate::core::TextAlign,
    valign: crate::core::TextVAlign,
    padding: f32,
    counter_clockwise_rotation: f32,
) -> AnnotationTextLayout {
    let padding = padding.max(0.0);
    let (text_x, text_y) = anchor_to_top_left(
        0.0,
        0.0,
        metrics.width,
        metrics.height,
        aligned_text_anchor(align, valign),
    );
    AnnotationTextLayout {
        text_x,
        text_y,
        box_x: text_x - padding,
        box_y: text_y - padding,
        box_width: metrics.width + 2.0 * padding,
        box_height: metrics.height + 2.0 * padding,
        rotation: if counter_clockwise_rotation.is_finite() {
            -counter_clockwise_rotation
        } else {
            0.0
        },
    }
}

/// Convert an anchor coordinate and rendered bounds into top-left draw coordinates.
pub fn anchor_to_top_left(
    anchor_x: f32,
    anchor_y: f32,
    rendered_width: f32,
    rendered_height: f32,
    anchor: TextAnchorKind,
) -> (f32, f32) {
    let x = match anchor {
        TextAnchorKind::TopLeft | TextAnchorKind::CenterLeft | TextAnchorKind::BottomLeft => {
            anchor_x
        }
        TextAnchorKind::TopCenter | TextAnchorKind::Center | TextAnchorKind::BottomCenter => {
            anchor_x - rendered_width / 2.0
        }
        TextAnchorKind::TopRight | TextAnchorKind::CenterRight | TextAnchorKind::BottomRight => {
            anchor_x - rendered_width
        }
    };
    let y = match anchor {
        TextAnchorKind::TopLeft | TextAnchorKind::TopCenter | TextAnchorKind::TopRight => anchor_y,
        TextAnchorKind::CenterLeft | TextAnchorKind::Center | TextAnchorKind::CenterRight => {
            anchor_y - rendered_height / 2.0
        }
        TextAnchorKind::BottomLeft | TextAnchorKind::BottomCenter | TextAnchorKind::BottomRight => {
            anchor_y - rendered_height
        }
    };
    (x, y)
}

/// Convert a top-origin anchor y to baseline y.
pub fn top_anchor_to_baseline(anchor_top_y: f32, metrics: TextPlacementMetrics) -> f32 {
    anchor_top_y + metrics.baseline_from_top
}

/// Convert a center-origin anchor y to baseline y.
pub fn center_anchor_to_baseline(anchor_center_y: f32, metrics: TextPlacementMetrics) -> f32 {
    anchor_center_y - metrics.height / 2.0 + metrics.baseline_from_top
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_anchor_kinds_map_to_expected_top_left() {
        let cases = [
            (TextAnchorKind::TopLeft, (20.0, 30.0)),
            (TextAnchorKind::TopCenter, (15.0, 30.0)),
            (TextAnchorKind::TopRight, (10.0, 30.0)),
            (TextAnchorKind::CenterLeft, (20.0, 28.0)),
            (TextAnchorKind::Center, (15.0, 28.0)),
            (TextAnchorKind::CenterRight, (10.0, 28.0)),
            (TextAnchorKind::BottomLeft, (20.0, 26.0)),
            (TextAnchorKind::BottomCenter, (15.0, 26.0)),
            (TextAnchorKind::BottomRight, (10.0, 26.0)),
        ];

        for (anchor, expected) in cases {
            assert_eq!(anchor_to_top_left(20.0, 30.0, 10.0, 4.0, anchor), expected);
        }
    }

    #[test]
    fn annotation_layout_expands_padding_without_moving_text_anchor() {
        let layout = annotation_text_layout(
            TextPlacementMetrics::new(10.0, 4.0, 3.0),
            crate::core::TextAlign::Center,
            crate::core::TextVAlign::Middle,
            2.0,
            30.0,
        );

        assert_eq!((layout.text_x, layout.text_y), (-5.0, -2.0));
        assert_eq!((layout.box_x, layout.box_y), (-7.0, -4.0));
        assert_eq!((layout.box_width, layout.box_height), (14.0, 8.0));
        assert_eq!(layout.rotation, -30.0);
    }
}
