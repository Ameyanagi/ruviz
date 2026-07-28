use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use egui::{ColorImage, Id, Pos2, Rect, Vec2};
use ruviz::core::{
    AlphaMode, Image, ImageFit, LogicalPoint, LogicalRect, fitted_content_rect, logical_to_physical,
};

static NEXT_WIDGET_ID: AtomicU64 = AtomicU64::new(1);

/// Whether a plot accepts pointer and keyboard input.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ViewMode {
    /// Render and resize the plot, but ignore interaction.
    Static,
    /// Render, resize, and translate egui input into ruviz interaction.
    #[default]
    Interactive,
}

/// How an adapter reserves space in an egui layout.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum PlotSize {
    /// Consume the finite space currently available to the [`egui::Ui`].
    #[default]
    Fill,
    /// Reserve a fixed size in egui logical points.
    FixedPixels { width: f32, height: f32 },
}

impl PlotSize {
    pub(crate) fn desired(self, ui: &egui::Ui) -> Vec2 {
        const DEFAULT_WIDTH: f32 = 640.0;
        const DEFAULT_HEIGHT: f32 = 360.0;
        let available = ui.available_size_before_wrap();
        match self {
            Self::Fill => Vec2::new(
                finite_positive(available.x, DEFAULT_WIDTH),
                finite_positive(available.y, DEFAULT_HEIGHT),
            ),
            Self::FixedPixels { width, height } => Vec2::new(
                finite_positive(width, DEFAULT_WIDTH),
                finite_positive(height, DEFAULT_HEIGHT),
            ),
        }
    }
}

fn finite_positive(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

/// Origin of an adapter error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdapterErrorKind {
    Render,
    Interaction,
}

/// A clonable, UI-friendly error retained by a widget.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterError {
    kind: AdapterErrorKind,
    message: String,
}

impl AdapterError {
    pub(crate) fn new(kind: AdapterErrorKind, error: impl fmt::Display) -> Self {
        Self {
            kind,
            message: error.to_string(),
        }
    }

    pub fn kind(&self) -> AdapterErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for AdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AdapterError {}

pub(crate) fn next_widget_id(kind: &'static str) -> Id {
    Id::new((
        "ruviz-egui",
        kind,
        NEXT_WIDGET_ID.fetch_add(1, Ordering::Relaxed),
    ))
}

pub(crate) fn fitted_rect(outer: Rect, image_size: (u32, u32), fit: ImageFit) -> Rect {
    let logical = fitted_content_rect(
        LogicalRect::new(
            f64::from(outer.min.x),
            f64::from(outer.min.y),
            f64::from(outer.width()),
            f64::from(outer.height()),
        ),
        image_size,
        fit,
    );
    Rect::from_min_size(
        Pos2::new(logical.x as f32, logical.y as f32),
        Vec2::new(logical.width as f32, logical.height as f32),
    )
}

pub(crate) fn visible_content_rect(content: Rect, outer: Rect) -> Rect {
    content.intersect(outer)
}

pub(crate) fn press_starts_in(visible_content: Rect, press_origin: Option<Pos2>) -> bool {
    press_origin.is_some_and(|origin| visible_content.contains(origin))
}

pub(crate) fn map_point(content: Rect, point: Pos2, image_size: (u32, u32)) -> Option<(f64, f64)> {
    logical_to_physical(
        LogicalRect::new(
            f64::from(content.min.x),
            f64::from(content.min.y),
            f64::from(content.width()),
            f64::from(content.height()),
        ),
        LogicalPoint::new(f64::from(point.x), f64::from(point.y)),
        image_size,
    )
}

pub(crate) fn map_point_clamped(content: Rect, point: Pos2, image_size: (u32, u32)) -> (f64, f64) {
    let point = Pos2::new(
        point.x.clamp(content.min.x, content.max.x),
        point.y.clamp(content.min.y, content.max.y),
    );
    map_point(content, point, image_size).unwrap_or((0.0, 0.0))
}

pub(crate) fn map_delta(content: Rect, delta: Vec2, image_size: (u32, u32)) -> (f64, f64) {
    if content.width() <= 0.0 || content.height() <= 0.0 {
        return (0.0, 0.0);
    }
    (
        f64::from(delta.x / content.width()) * f64::from(image_size.0),
        f64::from(delta.y / content.height()) * f64::from(image_size.1),
    )
}

pub(crate) fn release_is_cancelled(
    content: Rect,
    pointer: Option<Pos2>,
    window_focused: bool,
) -> bool {
    !window_focused || pointer.is_none_or(|position| !content.contains(position))
}

pub(crate) fn claim_scroll_y(ui: &egui::Ui) -> f32 {
    ui.input_mut(|input| {
        let scroll_y = input.smooth_scroll_delta.y;
        if scroll_y != 0.0 {
            input.smooth_scroll_delta = Vec2::ZERO;
        }
        scroll_y
    })
}

pub(crate) fn color_image(image: &Image) -> ColorImage {
    let pixels = image.pixels_in_alpha_mode(AlphaMode::Premultiplied);
    ColorImage::from_rgba_premultiplied(
        [image.width as usize, image.height as usize],
        pixels.as_ref(),
    )
}

pub(crate) fn paint_texture(
    ui: &egui::Ui,
    texture: &egui::TextureHandle,
    content: Rect,
    outer: Rect,
) {
    ui.painter().with_clip_rect(outer).image(
        texture.id(),
        content,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviz::core::physical_backing_size;

    #[test]
    fn fitted_mapping_accounts_for_letterboxing_and_fractional_coordinates() {
        let outer = Rect::from_min_size(Pos2::new(10.25, 20.5), Vec2::new(400.0, 400.0));
        let content = fitted_rect(outer, (800, 400), ImageFit::Contain);
        assert_eq!(content.min, Pos2::new(10.25, 120.5));
        assert_eq!(
            map_point(content, Pos2::new(210.25, 220.5), (800, 400)),
            Some((400.0, 200.0))
        );
        assert_eq!(
            map_point(content, Pos2::new(210.25, 100.0), (800, 400)),
            None
        );
    }

    #[test]
    fn cover_interactions_are_limited_to_the_visible_intersection() {
        let outer = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(200.0, 100.0));
        let content = fitted_rect(outer, (100, 200), ImageFit::Cover);
        assert!(content.height() > outer.height());
        let visible = visible_content_rect(content, outer);
        assert_eq!(visible, outer);
        assert!(press_starts_in(visible, Some(outer.center())));
        assert!(!press_starts_in(
            visible,
            Some(Pos2::new(outer.center().x, outer.max.y + 1.0))
        ));
    }

    #[test]
    fn hidpi_backing_sizes_cover_supported_fractional_scales() {
        let cases = [
            (1.0, (101, 51)),
            (1.25, (126, 63)),
            (1.5, (151, 76)),
            (2.0, (201, 101)),
        ];
        for (scale, expected) in cases {
            assert_eq!(physical_backing_size(100.25, 50.1, scale), expected);
        }
    }

    #[test]
    fn contain_cover_and_fill_map_visible_corners_and_centers() {
        let outer = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(400.0, 300.0));
        let image_size = (800, 400);

        let contain = fitted_rect(outer, image_size, ImageFit::Contain);
        assert_eq!(
            map_point(contain, contain.min, image_size),
            Some((0.0, 0.0))
        );
        assert_eq!(
            map_point(contain, contain.center(), image_size),
            Some((400.0, 200.0))
        );
        assert_eq!(
            map_point(contain, contain.max, image_size),
            Some((800.0, 400.0))
        );
        assert_eq!(
            map_point(
                contain,
                Pos2::new(contain.center().x, contain.min.y - 1.0),
                image_size
            ),
            None
        );

        let cover = fitted_rect(outer, image_size, ImageFit::Cover);
        let visible_cover = visible_content_rect(cover, outer);
        assert_eq!(visible_cover, outer);
        assert_close(
            map_point(cover, visible_cover.min, image_size).unwrap(),
            (400.0 / 3.0, 0.0),
        );
        assert_close(
            map_point(cover, visible_cover.center(), image_size).unwrap(),
            (400.0, 200.0),
        );
        assert_close(
            map_point(cover, visible_cover.max, image_size).unwrap(),
            (2000.0 / 3.0, 400.0),
        );

        let fill = fitted_rect(outer, image_size, ImageFit::Fill);
        assert_eq!(fill, outer);
        assert_eq!(map_point(fill, fill.min, image_size), Some((0.0, 0.0)));
        assert_eq!(
            map_point(fill, fill.center(), image_size),
            Some((400.0, 200.0))
        );
        assert_eq!(map_point(fill, fill.max, image_size), Some((800.0, 400.0)));
        assert_eq!(
            map_point(
                fill,
                Pos2::new(fill.max.x + 1.0, fill.center().y),
                image_size
            ),
            None
        );
    }

    fn assert_close(actual: (f64, f64), expected: (f64, f64)) {
        assert!(
            (actual.0 - expected.0).abs() < 1e-4,
            "{actual:?} != {expected:?}"
        );
        assert!(
            (actual.1 - expected.1).abs() < 1e-4,
            "{actual:?} != {expected:?}"
        );
    }

    #[test]
    fn alpha_is_converted_before_egui_upload() {
        let image = Image::new(1, 1, vec![255, 0, 0, 128]);
        let converted = color_image(&image);
        assert_eq!(
            converted.pixels[0],
            egui::Color32::from_rgba_premultiplied(128, 0, 0, 128)
        );
    }

    #[test]
    fn generated_widget_ids_do_not_alias() {
        assert_ne!(next_widget_id("2d"), next_widget_id("2d"));
        assert_ne!(next_widget_id("2d"), next_widget_id("3d"));
    }

    #[test]
    fn release_outside_or_focus_loss_is_a_cancellation() {
        let content = Rect::from_min_max(Pos2::ZERO, Pos2::new(100.0, 100.0));
        assert!(!release_is_cancelled(
            content,
            Some(Pos2::new(50.0, 50.0)),
            true
        ));
        assert!(release_is_cancelled(
            content,
            Some(Pos2::new(101.0, 50.0)),
            true
        ));
        assert!(release_is_cancelled(content, None, true));
        assert!(release_is_cancelled(
            content,
            Some(Pos2::new(50.0, 50.0)),
            false
        ));
    }

    #[test]
    fn claimed_zoom_scroll_is_not_left_for_a_parent_scroll_area() {
        egui::__run_test_ui(|ui| {
            ui.input_mut(|input| input.smooth_scroll_delta = Vec2::new(3.0, 7.0));
            assert_eq!(claim_scroll_y(ui), 7.0);
            assert_eq!(ui.input(|input| input.smooth_scroll_delta), Vec2::ZERO);
        });
    }
}
