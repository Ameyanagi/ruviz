use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SendError, Sender};
use std::thread::JoinHandle;

use egui::{ColorImage, Id, Pos2, Rect, TextureHandle, TextureOptions, Vec2};
use ruviz::core::{
    AlphaMode, Image, ImageFit, LogicalPoint, LogicalRect, RenderedLayer, fitted_content_rect,
    logical_to_physical, physical_backing_size, source_over_straight_rgba,
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

/// Place the last rendered frame inside the widget rectangle.
///
/// A frame that was rendered for exactly this rectangle is presented at its
/// natural size on whole physical pixels: the backing size is a ceiling of the
/// logical box, so the shared fitted geometry would otherwise scale it by a
/// fraction of a pixel and force the sampler to interpolate a plot that is
/// already pixel-exact. Every other frame size — most importantly a frame that
/// is still catching up with a resize — keeps the shared fitted geometry.
pub(crate) fn fitted_rect(
    outer: Rect,
    image_size: (u32, u32),
    fit: ImageFit,
    pixels_per_point: f32,
) -> Rect {
    let outer_logical = LogicalRect::new(
        f64::from(outer.min.x),
        f64::from(outer.min.y),
        f64::from(outer.width()),
        f64::from(outer.height()),
    );
    if pixels_per_point.is_finite()
        && pixels_per_point > 0.0
        && image_size
            == physical_backing_size(outer_logical.width, outer_logical.height, pixels_per_point)
    {
        return Rect::from_min_size(
            Pos2::new(
                snap_to_physical_pixel(outer.min.x, pixels_per_point),
                snap_to_physical_pixel(outer.min.y, pixels_per_point),
            ),
            Vec2::new(
                image_size.0 as f32 / pixels_per_point,
                image_size.1 as f32 / pixels_per_point,
            ),
        );
    }
    let logical = fitted_content_rect(outer_logical, image_size, fit);
    Rect::from_min_size(
        Pos2::new(logical.x as f32, logical.y as f32),
        Vec2::new(logical.width as f32, logical.height as f32),
    )
}

fn snap_to_physical_pixel(logical: f32, pixels_per_point: f32) -> f32 {
    (logical * pixels_per_point).round() / pixels_per_point
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

/// Texture filtering for every plot layer.
///
/// Layers are rendered at the exact physical backing size and presented on
/// whole physical pixels by [`fitted_rect`], so nearest sampling reproduces the
/// rendered pixels instead of blurring them.
pub(crate) const LAYER_TEXTURE_OPTIONS: TextureOptions = TextureOptions::NEAREST;

/// Convert a straight-alpha ruviz frame into egui's premultiplied texels.
///
/// This is the only full-frame pass an unchanged adapter still performs, so it
/// runs once per newly rendered layer and writes the final buffer directly
/// rather than materialising an intermediate premultiplied byte buffer.
pub(crate) fn color_image(image: &Image) -> ColorImage {
    let pixels = image.pixels_in_alpha_mode(AlphaMode::Straight);
    ColorImage::from_rgba_unmultiplied(
        [image.width as usize, image.height as usize],
        pixels.as_ref(),
    )
}

/// Build a `ColorImage` from a layer's native buffer with no alpha conversion.
///
/// `Color32::from_rgba_premultiplied` is a plain field copy while
/// `from_rgba_unmultiplied` multiplies every channel, so taking ruviz's native
/// premultiplied layer here removes a full-frame pass rather than adding one.
pub(crate) fn layer_color_image(layer: &RenderedLayer) -> ColorImage {
    let size = [layer.width() as usize, layer.height() as usize];
    match layer.alpha_mode() {
        AlphaMode::Premultiplied => ColorImage::from_rgba_premultiplied(size, layer.pixels()),
        AlphaMode::Straight => ColorImage::from_rgba_unmultiplied(size, layer.pixels()),
    }
}

/// Upload a layer, reusing the existing texture allocation when there is one.
pub(crate) fn upload_texture(
    context: &egui::Context,
    texture: &mut Option<TextureHandle>,
    name: impl FnOnce() -> String,
    layer: &RenderedLayer,
) {
    let color = layer_color_image(layer);
    match texture {
        Some(texture) => texture.set(color, LAYER_TEXTURE_OPTIONS),
        None => *texture = Some(context.load_texture(name(), color, LAYER_TEXTURE_OPTIONS)),
    }
}

/// Blend an overlay layer over its base layer for export actions.
///
/// Presentation stacks the two layers instead, so this full-frame composite is
/// only paid for when the user saves or copies the plot.
pub(crate) fn compose_over(base: &Image, overlay: &Image) -> Image {
    if base.width != overlay.width || base.height != overlay.height {
        return base.clone();
    }
    let mut pixels = base.pixels_in_alpha_mode(AlphaMode::Straight).into_owned();
    let overlay_pixels = overlay.pixels_in_alpha_mode(AlphaMode::Straight);
    for (destination, source) in pixels
        .chunks_exact_mut(4)
        .zip(overlay_pixels.chunks_exact(4))
    {
        let blended = source_over_straight_rgba(
            [
                destination[0],
                destination[1],
                destination[2],
                destination[3],
            ],
            [source[0], source[1], source[2], source[3]],
        );
        destination.copy_from_slice(&blended);
    }
    Image::new(base.width, base.height, pixels)
}

/// A persistent render thread owned by one widget.
///
/// The widget keeps a single thread for the lifetime of the plot instead of
/// spawning one per frame. Dropping the worker closes the request channel, so
/// the loop exits after the unit of work it is already running.
pub(crate) struct RenderWorker<T> {
    sender: Option<Sender<T>>,
    handle: Option<JoinHandle<()>>,
}

impl<T: Send + 'static> RenderWorker<T> {
    pub(crate) fn spawn(
        name: &'static str,
        run: impl FnOnce(Receiver<T>) + Send + 'static,
    ) -> std::io::Result<Self> {
        let (sender, receiver) = mpsc::channel();
        let handle = std::thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || run(receiver))?;
        Ok(Self {
            sender: Some(sender),
            handle: Some(handle),
        })
    }

    pub(crate) fn send(&self, work: T) -> Result<(), SendError<T>> {
        match self.sender.as_ref() {
            Some(sender) => sender.send(work),
            None => Err(SendError(work)),
        }
    }
}

/// Run one unit of render work, turning a panic into a reportable message.
///
/// A panicking render must never unwind out of a worker loop: the lane would
/// die with the scheduler's in-flight slot still occupied, so every later
/// request would be dropped and the widget would freeze without surfacing an
/// error.
pub(crate) fn catch_render_panic<R>(render: impl FnOnce() -> R) -> Result<R, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(render)).map_err(|payload| {
        payload
            .downcast_ref::<&str>()
            .map(|message| (*message).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "panic payload did not contain a message".to_owned())
    })
}

impl<T> Drop for RenderWorker<T> {
    fn drop(&mut self) {
        // Close the channel so the loop exits once its current unit of work
        // finishes, then detach rather than join.
        //
        // Joining here would block whichever thread drops the widget — the UI
        // thread — for the length of an in-flight render. Closing the channel
        // cannot interrupt a render already underway, so removing a plot or
        // shutting the application down would stall for a full frame, and for
        // a dashboard that cost is paid once per plot.
        //
        // Detaching is safe because the worker owns everything it touches: the
        // `InteractivePlotSession` and `egui::Context` it holds are Arc-backed
        // clones, and its final send simply fails once the receiver is gone.
        drop(self.sender.take());
        drop(self.handle.take());
    }
}

pub(crate) fn spawn_png_save(
    image: Arc<Image>,
    suggested_name: &str,
    completion: Sender<Result<(), AdapterError>>,
    repaint: egui::Context,
) -> Result<(), AdapterError> {
    let suggested_name = suggested_name.to_owned();
    std::thread::Builder::new()
        .name("ruviz-egui-png-save".to_owned())
        .spawn(move || {
            let result = pollster::block_on(async {
                let Some(file) = rfd::AsyncFileDialog::new()
                    .add_filter("PNG image", &["png"])
                    .set_file_name(&suggested_name)
                    .save_file()
                    .await
                else {
                    return Ok(());
                };
                let mut path = file.path().to_owned();
                if !path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
                {
                    path.set_extension("png");
                }
                write_png(image.as_ref(), &path)
            });
            let _ = completion.send(result);
            repaint.request_repaint();
        })
        .map(|_| ())
        .map_err(|error| AdapterError::new(AdapterErrorKind::Interaction, error))
}

fn write_png(image: &Image, path: &std::path::Path) -> Result<(), AdapterError> {
    ruviz::export::write_rgba_png_atomic(path, image)
        .map_err(|error| AdapterError::new(AdapterErrorKind::Interaction, error))
}

pub(crate) fn copy_image_to_clipboard(context: &egui::Context, image: &Image) {
    context.copy_image(color_image(image));
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

    #[test]
    fn a_panicking_render_is_reported_instead_of_unwinding_the_worker() {
        assert_eq!(catch_render_panic(|| 7), Ok(7));
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let caught = catch_render_panic(|| panic!("render exploded"));
        std::panic::set_hook(previous);
        assert_eq!(caught, Err("render exploded".to_owned()));
    }

    #[test]
    fn fitted_mapping_accounts_for_letterboxing_and_fractional_coordinates() {
        let outer = Rect::from_min_size(Pos2::new(10.25, 20.5), Vec2::new(400.0, 400.0));
        let content = fitted_rect(outer, (800, 400), ImageFit::Contain, 1.0);
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
        let content = fitted_rect(outer, (100, 200), ImageFit::Cover, 1.0);
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

        let contain = fitted_rect(outer, image_size, ImageFit::Contain, 1.0);
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

        let cover = fitted_rect(outer, image_size, ImageFit::Cover, 1.0);
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

        let fill = fitted_rect(outer, image_size, ImageFit::Fill, 1.0);
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
    fn a_frame_rendered_for_this_rect_is_presented_one_to_one_on_physical_pixels() {
        for (logical, scale) in [((100.25_f64, 50.1_f64), 2.0_f32), ((640.0, 360.0), 1.0)] {
            let outer = Rect::from_min_size(
                Pos2::new(10.3, 20.6),
                Vec2::new(logical.0 as f32, logical.1 as f32),
            );
            let image_size = physical_backing_size(logical.0, logical.1, scale);

            let content = fitted_rect(outer, image_size, ImageFit::Contain, scale);

            let physical_min = (content.min.x * scale, content.min.y * scale);
            assert_eq!(physical_min.0, physical_min.0.round());
            assert_eq!(physical_min.1, physical_min.1.round());
            assert_eq!(content.width() * scale, image_size.0 as f32);
            assert_eq!(content.height() * scale, image_size.1 as f32);
        }
    }

    #[test]
    fn a_frame_that_lags_behind_a_resize_keeps_the_shared_fitted_geometry() {
        let outer = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(400.0, 400.0));
        let stale = fitted_rect(outer, (800, 400), ImageFit::Contain, 1.0);
        assert_eq!(stale.width(), 400.0);
        assert_eq!(stale.height(), 200.0);
    }

    #[test]
    fn export_composition_blends_the_overlay_over_the_base() {
        let base = Image::new(2, 1, vec![255, 0, 0, 255, 10, 20, 30, 255]);
        let overlay = Image::new(2, 1, vec![0, 0, 255, 0, 0, 0, 255, 128]);

        let composed = compose_over(&base, &overlay);

        assert_eq!((composed.width, composed.height), (2, 1));
        assert_eq!(composed.pixels[..4], base.pixels[..4]);
        assert_eq!(
            composed.pixels[4..],
            source_over_straight_rgba([10, 20, 30, 255], [0, 0, 255, 128])
        );
    }

    #[test]
    fn export_composition_ignores_a_mismatched_overlay() {
        let base = Image::new(2, 1, vec![255, 0, 0, 255, 10, 20, 30, 255]);
        let overlay = Image::new(1, 1, vec![0, 0, 255, 255]);
        assert_eq!(compose_over(&base, &overlay).pixels, base.pixels);
    }

    /// Drop must close the channel so the loop terminates, but must not join:
    /// joining would block the dropping thread for an in-flight render. This
    /// waits on the worker's own post-loop send, so it proves termination
    /// without depending on drop having blocked.
    #[test]
    fn a_dropped_worker_closes_its_channel_and_lets_the_thread_finish() {
        let (observed, observer) = std::sync::mpsc::channel();
        let worker = RenderWorker::spawn("ruviz-egui-test-worker", move |receiver| {
            while let Ok(work) = receiver.recv() {
                let _ = observed.send(work);
            }
            let _ = observed.send(u32::MAX);
        })
        .unwrap();

        worker.send(7).unwrap();
        assert_eq!(observer.recv().unwrap(), 7);
        drop(worker);

        assert_eq!(observer.recv().unwrap(), u32::MAX);
        assert!(observer.recv().is_err());
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
    fn clipboard_export_uses_the_retained_image_pixels() {
        let context = egui::Context::default();
        let image = Image::new(2, 1, vec![255, 0, 0, 255, 0, 128, 255, 128]);
        let output = context.run_ui(egui::RawInput::default(), |ui| {
            copy_image_to_clipboard(ui.ctx(), &image);
        });

        let copied = output
            .platform_output
            .commands
            .iter()
            .find_map(|command| match command {
                egui::OutputCommand::CopyImage(image) => Some(image),
                _ => None,
            })
            .expect("copy action should emit an image clipboard command");
        assert_eq!(copied.size, [2, 1]);
        assert_eq!(
            copied.pixels[0],
            egui::Color32::from_rgba_premultiplied(255, 0, 0, 255)
        );
        assert_eq!(
            copied.pixels[1],
            egui::Color32::from_rgba_premultiplied(0, 64, 128, 128)
        );
    }

    #[test]
    fn png_export_writes_the_retained_frame_as_png() {
        let image = Image::new(1, 1, vec![10, 20, 30, 128]);
        let path = std::env::temp_dir().join(format!(
            "ruviz-egui-export-{}-{}.png",
            std::process::id(),
            NEXT_WIDGET_ID.fetch_add(1, Ordering::Relaxed)
        ));

        write_png(&image, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
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
