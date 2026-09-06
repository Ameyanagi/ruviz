//! Shared paint composition for image and macOS surface pan previews.

use super::*;
use ruviz::render::Color;

#[derive(Clone, Copy)]
pub(super) enum PanLayer {
    Anchor,
    Background(Color),
    Content,
}

/// Emit placements and clips without reading or copying a frame's pixels.
/// Keeping this shared by both presentation backends also lets the pixel
/// regression exercise exactly the composition used by the window painter.
pub(super) fn paint_pan_layers(
    frame: Bounds<Pixels>,
    preview: PanPreview,
    mut paint: impl FnMut(PanLayer, Bounds<Pixels>, Bounds<Pixels>),
) {
    let [figure, panel] = preview.background;
    let [r, g, b, a] = source_over_straight_rgba(
        [figure.r, figure.g, figure.b, figure.a],
        [panel.r, panel.g, panel.b, panel.a],
    );
    let background = PanLayer::Background(Color::from_rgba(r, g, b, a));
    if a == 255 {
        // The usual opaque plot needs just one extra quad: erase the old
        // data across the ENTIRE interior before placing the shifted frame.
        paint(PanLayer::Anchor, frame, frame);
        paint(background, preview.plot_bounds, preview.plot_bounds);
    } else {
        // Blending a translucent background over the anchor cannot erase its
        // data. Exclude the anchor's interior, and fill only uncovered strips
        // so the shifted frame's own background is also composited just once.
        for_each_outside(frame, preview.plot_bounds, |mask| {
            paint(PanLayer::Anchor, frame, mask);
        });
        for_each_outside(preview.plot_bounds, preview.mask, |mask| {
            paint(background, mask, mask);
        });
    }
    if preview.mask.size.width > px(0.0) && preview.mask.size.height > px(0.0) {
        paint(
            PanLayer::Content,
            Bounds::new(frame.origin + preview.offset, frame.size),
            preview.mask,
        );
    }
}

/// Disjoint positive-area rectangles covering `outer` minus `inner`.
/// Intersect first: a fast pan can put the (empty) preview mask completely
/// outside the plot, in which case the whole plot needs its background.
fn for_each_outside(
    outer: Bounds<Pixels>,
    inner: Bounds<Pixels>,
    mut visit: impl FnMut(Bounds<Pixels>),
) {
    let left = outer.left().max(inner.left());
    let right = outer.right().min(inner.right());
    let top = outer.top().max(inner.top());
    let bottom = outer.bottom().min(inner.bottom());
    if outer.size.width <= px(0.0) || outer.size.height <= px(0.0) {
        return;
    }
    if right <= left || bottom <= top {
        visit(outer);
        return;
    }
    for (x0, y0, x1, y1) in [
        (outer.left(), outer.top(), outer.right(), top),
        (outer.left(), bottom, outer.right(), outer.bottom()),
        (outer.left(), top, left, bottom),
        (right, top, outer.right(), bottom),
    ] {
        if x1 > x0 && y1 > y0 {
            visit(Bounds::new(point(x0, y0), size(x1 - x0, y1 - y0)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDTH: usize = 120;
    const HOST: [u8; 4] = [30, 40, 50, 255];
    const TRACE: [u8; 4] = [230, 40, 70, 255];

    fn contains(bounds: Bounds<Pixels>, x: usize, y: usize) -> bool {
        let p = point(px(x as f32 + 0.5), px(y as f32 + 0.5));
        p.x >= bounds.left() && p.x < bounds.right() && p.y >= bounds.top() && p.y < bounds.bottom()
    }

    fn composite(preview: PanPreview, source: &[[u8; 4]]) -> Vec<[u8; 4]> {
        let mut output = vec![HOST; WIDTH * WIDTH];
        let frame = Bounds::new(point(px(0.0), px(0.0)), size(px(120.0), px(120.0)));
        paint_pan_layers(frame, preview, |layer, placement, mask| {
            for y in 0..WIDTH {
                for x in 0..WIDTH {
                    if !contains(mask, x, y) || !contains(placement, x, y) {
                        continue;
                    }
                    let pixel = match layer {
                        PanLayer::Background(c) => [c.r, c.g, c.b, c.a],
                        PanLayer::Anchor | PanLayer::Content => {
                            let sx = (x as f32 - f32::from(placement.origin.x)) as usize;
                            let sy = (y as f32 - f32::from(placement.origin.y)) as usize;
                            source[sy * WIDTH + sx]
                        }
                    };
                    output[y * WIDTH + x] = source_over_straight_rgba(output[y * WIDTH + x], pixel);
                }
            }
        });
        output
    }

    #[test]
    fn pan_pixels_never_retain_the_unshifted_trace() {
        // A cross reaches all four uncovered edges. Big deltas and reversals
        // model input arriving before the next raster, including zero overlap.
        for background in [
            [Color::WHITE, Color::TRANSPARENT],
            [Color::from_rgb(25, 25, 25), Color::TRANSPARENT],
            [Color::WHITE, Color::from_rgba(120, 160, 200, 128)],
            [Color::TRANSPARENT, Color::from_rgba(120, 160, 200, 128)],
        ] {
            let mut view = super::super::tests::unit_frame_view();
            view.background = background;
            view.axis_inset_px = 2.0;
            let base = source_over_straight_rgba(
                [
                    background[0].r,
                    background[0].g,
                    background[0].b,
                    background[0].a,
                ],
                [
                    background[1].r,
                    background[1].g,
                    background[1].b,
                    background[1].a,
                ],
            );
            let mut source = vec![base; WIDTH * WIDTH];
            for y in 10..110 {
                for x in 10..110 {
                    if x == 60 || y == 60 {
                        source[y * WIDTH + x] = TRACE;
                    }
                }
            }
            let frame = Bounds::new(point(px(0.0), px(0.0)), size(px(120.0), px(120.0)));
            for (dx, dy) in [
                (30, 0),
                (-30, 0),
                (0, 30),
                (0, -30),
                (70, 70),
                (-70, -70),
                (130, 0),
                (0, -130),
                (0, 0),
            ] {
                let pending = ViewportRect::from_points(
                    ViewportPoint::new(-f64::from(dx) / 100.0, f64::from(dy) / 100.0),
                    ViewportPoint::new(1.0 - f64::from(dx) / 100.0, 1.0 + f64::from(dy) / 100.0),
                );
                let preview = preview_translation_onto(
                    &view,
                    pending,
                    view.plot_area,
                    view.axis_inset_px,
                    frame,
                    (120, 120),
                    true,
                )
                .unwrap();
                let output = composite(preview, &source);
                for y in 0..WIDTH {
                    for x in 0..WIDTH {
                        let expected = if contains(preview.plot_bounds, x, y) {
                            if contains(preview.mask, x, y) {
                                let sx = (x as i32 - dx) as usize;
                                let sy = (y as i32 - dy) as usize;
                                source_over_straight_rgba(HOST, source[sy * WIDTH + sx])
                            } else {
                                source_over_straight_rgba(HOST, base)
                            }
                        } else {
                            source_over_straight_rgba(HOST, source[y * WIDTH + x])
                        };
                        assert_eq!(
                            output[y * WIDTH + x],
                            expected,
                            "pan ({dx}, {dy}) pixel ({x}, {y}) background {background:?}"
                        );
                    }
                }
            }
        }
    }
}
