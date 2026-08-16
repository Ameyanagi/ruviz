//! Native [`slint`] component integration for [`ruviz`].
//!
//! The crate contains two deliberately separate layers:
//!
//! - the packaged `@Ruviz` Slint component library, containing `RuvizPlot`;
//! - [`RuvizController`], which retains plot sessions and renders on workers.
//!
//! The normal dependency does not select a Slint window backend or renderer.
//! Host applications remain responsible for that choice.

use std::{
    borrow::Cow,
    collections::HashMap,
    fmt,
    sync::{
        Arc, Mutex, MutexGuard, Weak,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
};

use ruviz::core::{
    AlphaMode, HitResult, Image as RuvizImage, ImageFit, ImageTarget,
    InteractiveChangeSubscription, InteractivePlotSession, InteractiveRenderStamp, IntoPlotSession,
    LatestRequestScheduler, LogicalPoint, LogicalRect, PlotContextMenuAction, PlotInputEvent,
    RenderedLayer, ScheduledRequest, ScheduledRequestId, ViewportPoint, ViewportRect,
    fitted_content_rect, logical_to_physical, physical_backing_size, sanitize_scale_factor,
    source_over_straight_rgba,
};
use ruviz::prelude::AxisScale;
use slint::{Model as _, Rgba8Pixel, SharedPixelBuffer};

pub use ruviz;
pub use slint;

/// Generated Rust bindings for the packaged `@Ruviz` Slint module.
///
/// This module is public because Slint's experimental external-module build
/// links consumer-generated code to this implementation.
pub mod slint_generated {
    slint::include_modules!();
}

pub use slint_generated::{
    RuvizContextAction, RuvizImageFit, RuvizPlotGrid, RuvizRuntime, RuvizSlotState,
};

static NEXT_SLOT_INCARNATION: AtomicU64 = AtomicU64::new(1);

fn lock_scalar_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn catch_callback<T>(callback: impl FnOnce() -> T) -> Result<T, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(callback))
        .map_err(|payload| panic_payload_message(payload.as_ref()))
}

fn write_image_png(image: &RuvizImage, path: impl AsRef<std::path::Path>) -> Result<(), String> {
    ruviz::export::write_rgba_png_atomic(path, image)
        .map_err(|error| format!("failed to save PNG: {error}"))
}

fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "panic payload did not contain a message".to_string()
    }
}

fn write_callback_diagnostic(message: &str) {
    use std::io::Write as _;

    let _ = writeln!(std::io::stderr().lock(), "ruviz-slint: {message}");
}

/// Stable identifier shared by a Slint `RuvizPlot` and its retained slot.
pub type SlotId = i32;

/// Whether a plot responds to pointer and keyboard input.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InteractionMode {
    /// Render an image but ignore interaction input.
    Static,
    /// Enable pan, zoom, hover, selection, brush, reset, orbit, and picking.
    #[default]
    Interactive,
}

/// Render-target sizing policy.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SizingMode {
    /// Follow the component's logical size and device scale.
    #[default]
    Fill,
    /// Keep a fixed physical backing size while still fitting into the widget.
    Fixed { width_px: u32, height_px: u32 },
}

/// Configuration retained independently for every controller slot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlotOptions {
    pub interaction: InteractionMode,
    pub sizing: SizingMode,
    pub fit: ImageFit,
    /// Prefer ruviz's GPU path when the corresponding crate feature is built.
    ///
    /// Slint presentation still uses GPU readback into a CPU pixel buffer.
    pub prefer_gpu: bool,
}

impl Default for SlotOptions {
    fn default() -> Self {
        Self {
            interaction: InteractionMode::Interactive,
            sizing: SizingMode::Fill,
            fit: ImageFit::Contain,
            prefer_gpu: cfg!(feature = "3d-gpu"),
        }
    }
}

/// Framework-neutral pointer event accepted by [`RuvizController`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerInput {
    pub kind: PointerKind,
    pub button: PointerButton,
    pub position: LogicalPoint,
}

/// Pointer transition emitted by the packaged component.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerKind {
    Down,
    Up,
    Move,
    Cancel,
}

/// Pointer button emitted by the packaged component.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PointerButton {
    #[default]
    None,
    Left,
    Right,
    Middle,
}

/// Coordinates and hit-test information reported after pointer input.
#[derive(Clone, Debug, PartialEq)]
pub struct PointerReport {
    pub slot: SlotId,
    pub kind: PointerKind,
    pub button: PointerButton,
    pub logical_position: LogicalPoint,
    pub physical_position: Option<ViewportPoint>,
    pub hit: Option<HitResult>,
}

/// Error reported by a retained adapter slot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterError {
    pub slot: SlotId,
    pub message: String,
}

impl fmt::Display for AdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Slint plot slot {}: {}", self.slot, self.message)
    }
}

impl std::error::Error for AdapterError {}

type UiTask = Box<dyn FnOnce() + Send + 'static>;
type FrameSink = Arc<dyn Fn(SlotId, slint::Image) + Send + Sync + 'static>;
type OverlaySink = Arc<dyn Fn(SlotId, Option<slint::Image>) + Send + Sync + 'static>;
type Dispatcher = Arc<dyn Fn(UiTask) -> Result<(), String> + Send + Sync + 'static>;
type PointerCallback = Arc<dyn Fn(PointerReport) + Send + Sync + 'static>;
type ErrorCallback = Arc<dyn Fn(AdapterError) + Send + Sync + 'static>;
type RuntimeConfigSink =
    Arc<dyn Fn(SlotId, InteractionMode, ImageFit, f32, bool, bool) + Send + Sync + 'static>;

#[cfg(feature = "3d")]
type PickCallback = Arc<dyn Fn(SlotId, ruviz::core::PickHit3D) + Send + Sync + 'static>;
#[cfg(feature = "3d")]
type CameraCallback = Arc<dyn Fn(SlotId, ruviz::core::CameraSnapshot3D) + Send + Sync + 'static>;

#[derive(Default)]
struct ControllerCallbacks {
    pointer: Option<PointerCallback>,
    error: Option<ErrorCallback>,
    #[cfg(feature = "3d")]
    pick: Option<PickCallback>,
    #[cfg(feature = "3d")]
    camera: Option<CameraCallback>,
}

/// Retained, cloneable controller for any number of Slint plot slots.
///
/// Rendering uses a latest-request scheduler per slot, drained by one
/// persistent worker thread per slot. At most one render is active for a slot;
/// intermediate resize/reactive requests are coalesced. Workers send a
/// [`SharedPixelBuffer`] to the Slint event loop, where the [`slint::Image`] is
/// constructed and installed. The last successful frame is retained when a
/// newer render fails.
///
/// A controller with an overlay sink (see [`RuvizController::on_overlay`],
/// installed automatically by [`RuvizController::attach`]) presents the plot
/// base and its interaction overlay as two stacked Slint images, so a hover,
/// tooltip, brush, or annotation change only re-uploads the small overlay and
/// the renderer composites the layers. Without an overlay sink the controller
/// blends the two layers itself and installs one flat image.
#[derive(Clone)]
pub struct RuvizController {
    inner: Arc<ControllerInner>,
}

struct ControllerInner {
    slots: Mutex<HashMap<SlotId, SlotState>>,
    render_workers: Mutex<HashMap<SlotId, RenderWorker>>,
    frame_sink: FrameSink,
    overlay_sink: Mutex<Option<OverlaySink>>,
    dispatcher: Dispatcher,
    runtime_config_sink: Option<RuntimeConfigSink>,
    default_scale_factor: f32,
    callbacks: Mutex<ControllerCallbacks>,
    #[cfg(test)]
    render_barrier: Mutex<Option<(Arc<std::sync::Barrier>, Arc<std::sync::Barrier>)>>,
}

struct SlotState {
    plot: PlotSlot,
    incarnation: u64,
    options: SlotOptions,
    layout: SlotLayout,
    scheduler: LatestRequestScheduler<RenderRequest>,
    drag: Option<ActiveDrag>,
    context_press: Option<ContextPress>,
    _subscription: Option<InteractiveChangeSubscription>,
    last_frame: Option<InstalledFrame>,
    /// Layers the slot's sinks actually hold right now.
    ///
    /// This is deliberately not `last_frame`: layers are handed to the sinks
    /// before the frame can be committed, so a frame that is presented but then
    /// rejected still changed what is on screen. Diffing against anything else
    /// would let a shown overlay survive a redraw that means to clear it.
    presented: Option<PresentedLayers>,
}

fn lock_slots(
    slots: &Mutex<HashMap<SlotId, SlotState>>,
) -> MutexGuard<'_, HashMap<SlotId, SlotState>> {
    match slots.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            let mut guard = poisoned.into_inner();
            guard.clear();
            slots.clear_poison();
            write_callback_diagnostic(
                "slot state was poisoned; all retained slots were cleared to fail closed",
            );
            guard
        }
    }
}

enum PlotSlot {
    TwoD(InteractivePlotSession),
    #[cfg(feature = "3d")]
    ThreeD {
        session: Arc<Mutex<ruviz::core::InteractivePlot3DSession>>,
        renderer: Arc<Mutex<ruviz::core::BackgroundRenderer3D>>,
        backend: ruviz::core::BackgroundRenderBackend3D,
    },
}

impl PlotSlot {
    fn is_3d(&self) -> bool {
        match self {
            Self::TwoD(_) => false,
            #[cfg(feature = "3d")]
            Self::ThreeD { .. } => true,
        }
    }

    fn customized_2d_visible_bounds(&self) -> Option<ViewportRect> {
        match self {
            Self::TwoD(session) => {
                let view = session.view_bounds_snapshot();
                viewport_bounds_materially_differ(
                    view.visible_bounds,
                    view.base_bounds,
                    &view.x_scale,
                    &view.y_scale,
                )
                .then_some(view.visible_bounds)
            }
            #[cfg(feature = "3d")]
            Self::ThreeD { .. } => None,
        }
    }
}

#[cfg(feature = "3d")]
fn lock_3d_session(
    session: &Mutex<ruviz::core::InteractivePlot3DSession>,
) -> Result<MutexGuard<'_, ruviz::core::InteractivePlot3DSession>, &'static str> {
    session
        .lock()
        .map_err(|_| "3D session state is poisoned and cannot be used safely")
}

#[cfg(feature = "3d")]
fn lock_3d_renderer(
    renderer: &Mutex<ruviz::core::BackgroundRenderer3D>,
) -> Result<MutexGuard<'_, ruviz::core::BackgroundRenderer3D>, &'static str> {
    renderer
        .lock()
        .map_err(|_| "3D renderer state is poisoned and cannot be used safely")
}

#[derive(Clone)]
struct RenderRequest {
    incarnation: u64,
    /// Layers the slot currently presents, used to skip unchanged uploads.
    ///
    /// `None` means nothing presentable is known to be on screen, so both
    /// layers are published even if that only clears the overlay.
    published: Option<PublishedLayers>,
    /// Whether base and overlay are presented as two stacked Slint images.
    layered: bool,
    kind: RenderRequestKind,
}

#[derive(Clone)]
enum RenderRequestKind {
    TwoD {
        session: InteractivePlotSession,
        target: ImageTarget,
    },
    #[cfg(feature = "3d")]
    ThreeD {
        session: Arc<Mutex<ruviz::core::InteractivePlot3DSession>>,
        renderer: Arc<Mutex<ruviz::core::BackgroundRenderer3D>>,
        job: ruviz::core::BackgroundRenderJob3D,
    },
}

/// Layer identities a slot has already handed to Slint.
///
/// A layer whose `Arc` is unchanged is still on screen, so the worker skips
/// both its pixel copy and its install.
#[derive(Clone)]
struct PublishedLayers {
    base: RenderedLayer,
    overlay: Option<RenderedLayer>,
}

/// What a slot's sinks currently hold, and under which presentation.
///
/// Layers only stay reusable while the slot keeps the same plot and the same
/// layered/flat presentation; anything else must be republished.
#[derive(Clone)]
struct PresentedLayers {
    incarnation: u64,
    layered: bool,
    layers: PublishedLayers,
}

struct RenderJob {
    id: ScheduledRequestId,
    incarnation: u64,
    request: RenderRequest,
    #[cfg(test)]
    barrier: Option<(Arc<std::sync::Barrier>, Arc<std::sync::Barrier>)>,
}

/// One persistent render lane per slot.
///
/// Dropping the controller drops the sender, which ends the worker's `recv`
/// loop after its current render; nothing is joined, so a slow render can never
/// block the UI thread that dropped the controller.
struct RenderWorker {
    sender: mpsc::Sender<RenderJob>,
    #[cfg(test)]
    busy: Arc<std::sync::atomic::AtomicBool>,
}

impl RenderWorker {
    fn start(slot: SlotId, weak: Weak<ControllerInner>) -> Result<Self, String> {
        let (sender, receiver) = mpsc::channel::<RenderJob>();
        #[cfg(test)]
        let busy = Arc::new(std::sync::atomic::AtomicBool::new(false));
        #[cfg(test)]
        let lane_busy = Arc::clone(&busy);
        std::thread::Builder::new()
            .name(format!("ruviz-slint-{slot}"))
            .spawn(move || {
                while let Ok(job) = receiver.recv() {
                    #[cfg(test)]
                    lane_busy.store(true, Ordering::SeqCst);
                    #[cfg(test)]
                    if let Some((entered, release)) = job.barrier {
                        entered.wait();
                        release.wait();
                    }
                    let result = match catch_callback(|| render_request(job.request)) {
                        Ok(result) => result,
                        Err(message) => Err(WorkerFailure::Error(format!(
                            "plot renderer panicked while producing a background frame: {message}"
                        ))),
                    };
                    // The controller may have been dropped mid-render; the
                    // frame is then simply discarded.
                    if let Some(inner) = weak.upgrade() {
                        inner.finish_render(slot, job.id, job.incarnation, result);
                    }
                    #[cfg(test)]
                    lane_busy.store(false, Ordering::SeqCst);
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            sender,
            #[cfg(test)]
            busy,
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct SlotLayout {
    outer: LogicalRect,
    scale_factor: f32,
}

impl SlotLayout {
    fn with_scale(scale_factor: f32) -> Self {
        Self {
            scale_factor: sanitize_scale_factor(scale_factor),
            ..Self::default()
        }
    }
}

impl Default for SlotLayout {
    fn default() -> Self {
        Self {
            outer: LogicalRect::new(0.0, 0.0, 640.0, 480.0),
            scale_factor: 1.0,
        }
    }
}

#[derive(Clone)]
struct InstalledFrame {
    base: RenderedLayer,
    overlay: Option<RenderedLayer>,
    size_px: (u32, u32),
    generation: u64,
    incarnation: u64,
    validity: RenderValidity,
}

impl InstalledFrame {
    fn is_current(&self) -> bool {
        self.validity.is_current()
    }
}

#[derive(Clone, Copy, Debug)]
struct ActiveDrag {
    button: PointerButton,
    anchor: LogicalPoint,
    last: LogicalPoint,
    moved: bool,
    forwarded: bool,
}

#[derive(Clone, Copy, Debug)]
struct ContextPress {
    anchor: LogicalPoint,
    moved: bool,
}

#[derive(Clone)]
enum RenderValidity {
    TwoD {
        session: InteractivePlotSession,
        stamp: InteractiveRenderStamp,
    },
    #[cfg(feature = "3d")]
    ThreeD {
        session: Arc<Mutex<ruviz::core::InteractivePlot3DSession>>,
        stamp: ruviz::core::RenderStamp3D,
    },
}

impl RenderValidity {
    fn is_current(&self) -> bool {
        match self {
            Self::TwoD { session, stamp } => session.is_render_stamp_current(*stamp),
            #[cfg(feature = "3d")]
            Self::ThreeD { session, stamp } => {
                lock_3d_session(session).is_ok_and(|session| session.is_render_current(*stamp))
            }
        }
    }
}

/// One presentable layer, already copied into Slint's pixel storage.
struct LayerBuffer {
    buffer: SharedPixelBuffer<Rgba8Pixel>,
    alpha_mode: AlphaMode,
}

impl LayerBuffer {
    fn new(image: &RuvizImage) -> Self {
        Self {
            buffer: SharedPixelBuffer::clone_from_slice(&image.pixels, image.width, image.height),
            alpha_mode: image.alpha_mode(),
        }
    }

    /// Copy a layer's native bytes, converting nothing.
    ///
    /// `into_slint_image` then picks `from_rgba8_premultiplied` for a
    /// premultiplied layer, so tiny-skia's native output reaches Slint without
    /// a demultiply/re-premultiply round trip.
    fn from_layer(layer: &RenderedLayer) -> Self {
        Self {
            buffer: SharedPixelBuffer::clone_from_slice(
                layer.pixels(),
                layer.width(),
                layer.height(),
            ),
            alpha_mode: layer.alpha_mode(),
        }
    }

    fn into_slint_image(self) -> slint::Image {
        match self.alpha_mode {
            AlphaMode::Straight => slint::Image::from_rgba8(self.buffer),
            AlphaMode::Premultiplied => slint::Image::from_rgba8_premultiplied(self.buffer),
        }
    }
}

/// What the overlay layer of an installed slot must become.
enum OverlayUpdate {
    /// The presented overlay is still correct; leave it untouched.
    Reuse,
    /// Replace it, or clear it when the frame carries no overlay.
    Replace(Option<LayerBuffer>),
}

struct RenderedFrame {
    /// `None` when the presented base layer is still the rendered one.
    base: Option<LayerBuffer>,
    overlay: OverlayUpdate,
    base_layer: RenderedLayer,
    overlay_layer: Option<RenderedLayer>,
    size_px: (u32, u32),
    /// Whether this frame was produced for layered presentation.
    layered: bool,
    validity: RenderValidity,
}

enum WorkerFailure {
    Superseded,
    Error(String),
}

impl RuvizController {
    /// Create a controller that installs frames through `frame_sink`.
    ///
    /// `frame_sink` always runs on the Slint UI event loop. It receives one
    /// flat image unless [`RuvizController::on_overlay`] is also installed.
    pub fn new(frame_sink: impl Fn(SlotId, slint::Image) + Send + Sync + 'static) -> Self {
        Self::with_dispatcher(frame_sink, |task| {
            slint::invoke_from_event_loop(task).map_err(|error| error.to_string())
        })
    }

    /// Create a controller with a custom UI dispatcher.
    ///
    /// This is useful for an application-owned event-loop proxy and for
    /// deterministic headless tests. The dispatcher must execute the task on
    /// the UI thread in production.
    pub fn with_dispatcher(
        frame_sink: impl Fn(SlotId, slint::Image) + Send + Sync + 'static,
        dispatcher: impl Fn(UiTask) -> Result<(), String> + Send + Sync + 'static,
    ) -> Self {
        Self::with_parts(frame_sink, dispatcher, None, 1.0)
    }

    fn with_parts(
        frame_sink: impl Fn(SlotId, slint::Image) + Send + Sync + 'static,
        dispatcher: impl Fn(UiTask) -> Result<(), String> + Send + Sync + 'static,
        runtime_config_sink: Option<RuntimeConfigSink>,
        default_scale_factor: f32,
    ) -> Self {
        Self {
            inner: Arc::new(ControllerInner {
                slots: Mutex::new(HashMap::new()),
                render_workers: Mutex::new(HashMap::new()),
                frame_sink: Arc::new(frame_sink),
                overlay_sink: Mutex::new(None),
                dispatcher: Arc::new(dispatcher),
                runtime_config_sink,
                default_scale_factor: sanitize_scale_factor(default_scale_factor),
                callbacks: Mutex::new(ControllerCallbacks::default()),
                #[cfg(test)]
                render_barrier: Mutex::new(None),
            }),
        }
    }

    /// Attach the controller-owned multi-slot runtime to a Slint component tree.
    ///
    /// `C` may be the standalone [`RuvizPlotGrid`] or an application component that
    /// imports `RuvizPlot` from `@Ruviz`.
    ///
    /// Layered presentation is enabled per component tree, not unconditionally:
    /// the overlay sink is installed the first time a component announces
    /// itself through `RuvizRuntime.overlay-supported`, which `RuvizPlot` does
    /// for itself. A custom component that binds `RuvizRuntime.slots[i].source`
    /// alone therefore keeps receiving one flat, pre-composed image and never
    /// silently loses its crosshair, tooltip, selection, or brush overlay.
    pub fn attach<C>(component: &C) -> Self
    where
        C: slint::ComponentHandle + 'static,
        for<'a> RuvizRuntime<'a>: slint::Global<'a, C, StaticSelf = RuvizRuntime<'static>>,
    {
        use slint::Global as _;

        let runtime: RuvizRuntime<'_> = component.global();
        runtime.set_slots(slint::ModelRc::default());
        let frame_runtime = runtime.as_weak();
        let overlay_runtime = frame_runtime.clone();
        let config_runtime = frame_runtime.clone();
        let config_sink: RuntimeConfigSink =
            Arc::new(move |slot, mode, fit, scale, is_3d, has_frame| {
                if let Some(runtime) = config_runtime.upgrade() {
                    update_runtime_config(&runtime, slot, mode, fit, scale, is_3d, has_frame);
                }
            });
        let controller = Self::with_parts(
            move |slot, image| {
                if let Some(runtime) = frame_runtime.upgrade() {
                    update_runtime_image(&runtime, slot, image);
                }
            },
            |task| slint::invoke_from_event_loop(task).map_err(|error| error.to_string()),
            Some(config_sink),
            component.window().scale_factor(),
        );
        let overlay_sink: OverlaySink = Arc::new(move |slot, overlay| {
            if let Some(runtime) = overlay_runtime.upgrade() {
                update_runtime_overlay(&runtime, slot, overlay.unwrap_or_default());
            }
        });
        let announced = controller.clone();
        runtime.on_overlay_supported(move |_slot| {
            announced.enable_overlay_layer(Arc::clone(&overlay_sink));
        });
        controller.bind_runtime(component);
        controller
    }

    /// Switch to layered presentation once a component can stack two images.
    ///
    /// Every `RuvizPlot` announces itself, so this runs repeatedly and only the
    /// first call installs the sink.
    fn enable_overlay_layer(&self, sink: OverlaySink) {
        {
            let mut installed = lock_scalar_recover(&self.inner.overlay_sink);
            if installed.is_some() {
                return;
            }
            *installed = Some(sink);
        }
        self.inner.request_render_all();
    }

    /// Install the sink that presents the interaction overlay layer.
    ///
    /// Installing it switches the slot to layered presentation: `frame_sink`
    /// receives the plot base and this sink receives the overlay drawn over it
    /// in the same fitted geometry, using normal source-over blending. `None`
    /// means the frame has no overlay, so any previously shown overlay must be
    /// cleared. Each layer is only handed over when it actually changed, which
    /// is what keeps a hover redraw off the base layer.
    ///
    /// [`RuvizController::attach`] installs this for a component that announces
    /// `RuvizRuntime.overlay-supported`. A controller without an overlay sink
    /// keeps receiving one flat, pre-composed image.
    pub fn on_overlay(&self, sink: impl Fn(SlotId, Option<slint::Image>) + Send + Sync + 'static) {
        *lock_scalar_recover(&self.inner.overlay_sink) = Some(Arc::new(sink));
        self.inner.request_render_all();
    }

    /// Connect the `RuvizRuntime` global callbacks to this controller.
    pub fn bind_runtime<C>(&self, component: &C)
    where
        C: slint::ComponentHandle,
        for<'a> RuvizRuntime<'a>: slint::Global<'a, C, StaticSelf = RuvizRuntime<'static>>,
    {
        let runtime: RuvizRuntime<'_> = component.global();
        let controller = self.clone();
        runtime.on_resized(move |slot, width, height, scale| {
            controller.resize(slot, f64::from(width), f64::from(height), scale);
        });

        let controller = self.clone();
        runtime.on_pointer_event(move |slot, kind, button, x, y| {
            if let (Some(kind), Some(button)) = (decode_pointer_kind(kind), decode_button(button)) {
                controller.pointer_input(
                    slot,
                    PointerInput {
                        kind,
                        button,
                        position: LogicalPoint::new(f64::from(x), f64::from(y)),
                    },
                )
            } else {
                false
            }
        });

        let controller = self.clone();
        runtime.on_context_action(move |slot, action| {
            if let Some(action) = decode_context_action(action) {
                controller.context_action(slot, action);
            }
        });

        let controller = self.clone();
        runtime.on_wheel_event(move |slot, _delta_x, delta_y, x, y| {
            controller.wheel(
                slot,
                f64::from(delta_y),
                LogicalPoint::new(f64::from(x), f64::from(y)),
            )
        });

        let controller = self.clone();
        runtime.on_double_clicked(move |slot, x, y| {
            controller.double_click(slot, LogicalPoint::new(f64::from(x), f64::from(y)));
        });

        let controller = self.clone();
        runtime.on_escape(move |slot| controller.reset_view(slot));

        let controller = self.clone();
        runtime.on_focus_lost(move |slot| controller.cancel_drag(slot));
    }

    /// Register or replace a retained 2D slot.
    pub fn set_plot(&self, slot: SlotId, plot: impl IntoPlotSession, options: SlotOptions) {
        self.set_plot_with_view_policy(slot, plot, options, false);
    }

    /// Replace a retained 2D slot while preserving a customized visible view.
    ///
    /// If the old view is still at its natural base bounds, the replacement
    /// uses its own natural bounds.
    pub fn set_plot_keep_view(
        &self,
        slot: SlotId,
        plot: impl IntoPlotSession,
        options: SlotOptions,
    ) {
        self.set_plot_with_view_policy(slot, plot, options, true);
    }

    fn set_plot_with_view_policy(
        &self,
        slot: SlotId,
        plot: impl IntoPlotSession,
        options: SlotOptions,
        keep_view: bool,
    ) {
        if slot < 0 {
            self.inner
                .report_error(slot, "slot identifiers must be non-negative".to_string());
            return;
        }
        let session = plot.into_plot_session();
        session.set_prefer_gpu(options.prefer_gpu);
        let weak = Arc::downgrade(&self.inner);
        let subscription = session.subscribe_changes(move |_| {
            if let Some(inner) = weak.upgrade() {
                inner.sync_runtime(slot);
                inner.request_render(slot);
            }
        });
        let Some(incarnation) = next_slot_incarnation() else {
            self.inner
                .report_error(slot, "Slint slot incarnation space exhausted".to_string());
            return;
        };

        let (layout, previous_bounds) = {
            let mut slots = lock_slots(&self.inner.slots);
            let old = slots.remove(&slot);
            let layout = old.as_ref().map_or(
                SlotLayout::with_scale(self.inner.default_scale_factor),
                |old| old.layout,
            );
            let bounds = if keep_view {
                old.as_ref()
                    .and_then(|old| old.plot.customized_2d_visible_bounds())
            } else {
                None
            };
            let (scheduler, last_frame) = old.map_or_else(
                || (LatestRequestScheduler::default(), None),
                |old| (old.scheduler, old.last_frame),
            );
            slots.insert(
                slot,
                SlotState {
                    plot: PlotSlot::TwoD(session.clone()),
                    incarnation,
                    options,
                    layout,
                    scheduler,
                    drag: None,
                    context_press: None,
                    _subscription: Some(subscription),
                    last_frame,
                    presented: None,
                },
            );
            (layout, bounds)
        };

        if let Some(bounds) = previous_bounds {
            session.defer_visible_bounds_restore(bounds);
        }
        let target = target_size(layout, options.sizing);
        session.resize(target, layout.scale_factor);
        self.inner.sync_runtime(slot);
        self.inner.request_render(slot);
    }

    /// Register or replace a retained 3D slot.
    #[cfg(feature = "3d")]
    pub fn set_plot3d(
        &self,
        slot: SlotId,
        plot: impl ruviz::core::TryIntoPlot3DSession,
        options: SlotOptions,
    ) -> ruviz::core::Result<()> {
        self.set_plot3d_with_view_policy(slot, plot, options, false)
    }

    /// Register or replace a retained 3D slot while preserving its camera.
    #[cfg(feature = "3d")]
    pub fn set_plot3d_keep_view(
        &self,
        slot: SlotId,
        plot: impl ruviz::core::TryIntoPlot3DSession,
        options: SlotOptions,
    ) -> ruviz::core::Result<()> {
        self.set_plot3d_with_view_policy(slot, plot, options, true)
    }

    #[cfg(feature = "3d")]
    fn set_plot3d_with_view_policy(
        &self,
        slot: SlotId,
        plot: impl ruviz::core::TryIntoPlot3DSession,
        options: SlotOptions,
        keep_view: bool,
    ) -> ruviz::core::Result<()> {
        if slot < 0 {
            return Err(ruviz::core::PlottingError::InvalidInput(
                "Slint slot identifiers must be non-negative".to_string(),
            ));
        }
        let mut replacement = plot.try_into_plot3d_session()?;
        let incarnation = next_slot_incarnation().ok_or_else(|| {
            ruviz::core::PlottingError::InvalidInput(
                "Slint slot incarnation space exhausted".to_string(),
            )
        })?;
        let mut slots = lock_slots(&self.inner.slots);
        let old = slots.get(&slot);
        let layout = old.map_or(
            SlotLayout::with_scale(self.inner.default_scale_factor),
            |old| old.layout,
        );
        if keep_view && let Some(PlotSlot::ThreeD { session, .. }) = old.map(|old| &old.plot) {
            let snapshot = lock_3d_session(session)
                .map_err(|message| ruviz::core::PlottingError::InvalidInput(message.to_string()))?
                .camera_snapshot();
            replacement.restore_camera(snapshot)?;
        }
        let target = target_size(layout, options.sizing);
        replacement.resize(target.0, target.1, layout.scale_factor)?;

        let backend = background_backend(options.prefer_gpu);
        let renderer = old
            .and_then(|old| match &old.plot {
                PlotSlot::ThreeD {
                    renderer,
                    backend: old_backend,
                    ..
                } if *old_backend == backend && !renderer.is_poisoned() => {
                    Some(Arc::clone(renderer))
                }
                PlotSlot::TwoD(_) | PlotSlot::ThreeD { .. } => None,
            })
            .unwrap_or_else(|| new_background_renderer(options.prefer_gpu));
        let (scheduler, last_frame) = slots.remove(&slot).map_or_else(
            || (LatestRequestScheduler::default(), None),
            |old| (old.scheduler, old.last_frame),
        );
        let session = Arc::new(Mutex::new(replacement));
        slots.insert(
            slot,
            SlotState {
                plot: PlotSlot::ThreeD {
                    session,
                    renderer,
                    backend,
                },
                incarnation,
                options,
                layout,
                scheduler,
                drag: None,
                context_press: None,
                _subscription: None,
                last_frame,
                presented: None,
            },
        );
        drop(slots);
        self.inner.sync_runtime(slot);
        self.inner.request_render(slot);
        Ok(())
    }

    /// Remove a slot. In-flight frames become non-installable.
    pub fn remove_plot(&self, slot: SlotId) -> bool {
        let removed = lock_slots(&self.inner.slots).remove(&slot).is_some();
        if removed {
            self.inner.clear_runtime(slot);
        }
        removed
    }

    /// Replace a slot's presentation and interaction options.
    ///
    /// Switching to static mode cancels transient interaction state. Sizing or
    /// backend preference changes schedule a new background frame.
    pub fn set_options(&self, slot: SlotId, options: SlotOptions) -> bool {
        let (handle, layout, cancel) = {
            let mut slots = lock_slots(&self.inner.slots);
            let Some(state) = slots.get_mut(&slot) else {
                return false;
            };
            let cancel = state.options.interaction == InteractionMode::Interactive
                && options.interaction == InteractionMode::Static;
            state.options = options;
            #[cfg(feature = "3d")]
            if let PlotSlot::ThreeD {
                renderer, backend, ..
            } = &mut state.plot
                && *backend != background_backend(options.prefer_gpu)
            {
                *renderer = new_background_renderer(options.prefer_gpu);
                *backend = background_backend(options.prefer_gpu);
            }
            (state.plot.clone_handle(), state.layout, cancel)
        };
        match &handle {
            PlotHandle::TwoD(session) => session.set_prefer_gpu(options.prefer_gpu),
            #[cfg(feature = "3d")]
            PlotHandle::ThreeD(_) => {}
        }
        if cancel {
            self.cancel_drag(slot);
            match &handle {
                PlotHandle::TwoD(session) => {
                    session.apply_input(PlotInputEvent::ClearHover);
                }
                #[cfg(feature = "3d")]
                PlotHandle::ThreeD(_) => {}
            }
            self.inner.request_render(slot);
        }
        self.resize(
            slot,
            layout.outer.width,
            layout.outer.height,
            layout.scale_factor,
        );
        self.inner.sync_runtime(slot);
        true
    }

    /// Update logical size and device scale for a component.
    pub fn resize(&self, slot: SlotId, logical_width: f64, logical_height: f64, scale_factor: f32) {
        let scale_factor = sanitize_scale_factor(scale_factor);
        let (plot, target) = {
            let mut slots = lock_slots(&self.inner.slots);
            let Some(state) = slots.get_mut(&slot) else {
                return;
            };
            state.layout = SlotLayout {
                outer: LogicalRect::new(0.0, 0.0, logical_width.max(1.0), logical_height.max(1.0)),
                scale_factor,
            };
            (
                state.plot.clone_handle(),
                target_size(state.layout, state.options.sizing),
            )
        };

        match plot {
            PlotHandle::TwoD(session) => session.resize(target, scale_factor),
            #[cfg(feature = "3d")]
            PlotHandle::ThreeD(session) => {
                let result = match lock_3d_session(&session) {
                    Ok(mut session) => session
                        .resize(target.0, target.1, scale_factor)
                        .map_err(|error| error.to_string()),
                    Err(message) => Err(message.to_string()),
                };
                if let Err(message) = result {
                    self.inner.report_error(slot, message);
                } else {
                    self.inner.request_render(slot);
                }
            }
        }
        self.inner.sync_runtime(slot);
    }

    /// Request a redraw without changing plot state.
    pub fn request_redraw(&self, slot: SlotId) {
        self.inner.request_render(slot);
    }

    /// Apply one pointer transition.
    ///
    /// Returns `true` when an unmoved secondary click should open the packaged
    /// context menu. Secondary presses are retained until the pointer crosses
    /// the drag threshold, so a menu click never starts a 2D brush or 3D pan.
    pub fn pointer_input(&self, slot: SlotId, input: PointerInput) -> bool {
        let (action, open_context_menu) = {
            let mut slots = lock_slots(&self.inner.slots);
            let Some(state) = slots.get_mut(&slot) else {
                return false;
            };
            let handle = state.plot.clone_handle();
            let mapped = state.map_point(input.position);
            let open_context_menu = match input.kind {
                PointerKind::Down if input.button == PointerButton::Right => {
                    state.context_press = Some(ContextPress {
                        anchor: input.position,
                        moved: false,
                    });
                    false
                }
                PointerKind::Down => {
                    state.context_press = None;
                    false
                }
                PointerKind::Move => {
                    if let Some(mut press) = state.context_press {
                        let total = LogicalPoint::new(
                            input.position.x - press.anchor.x,
                            input.position.y - press.anchor.y,
                        );
                        press.moved |= total.x.hypot(total.y) >= 3.0;
                        state.context_press = Some(press);
                    }
                    false
                }
                PointerKind::Up if input.button == PointerButton::Right => {
                    state.context_press.take().is_some_and(|press| {
                        let total = LogicalPoint::new(
                            input.position.x - press.anchor.x,
                            input.position.y - press.anchor.y,
                        );
                        !press.moved && total.x.hypot(total.y) < 3.0
                    })
                }
                PointerKind::Up => false,
                PointerKind::Cancel => {
                    state.context_press = None;
                    false
                }
            };
            if input.kind == PointerKind::Up
                && input.button == PointerButton::Right
                && let Some(drag) = state.drag.as_mut()
            {
                let total = LogicalPoint::new(
                    input.position.x - drag.anchor.x,
                    input.position.y - drag.anchor.y,
                );
                drag.moved |= total.x.hypot(total.y) >= 3.0;
            }
            if open_context_menu {
                state.drag = None;
                (None, true)
            } else if !state.pointer_input_enabled(input.kind) {
                if matches!(input.kind, PointerKind::Up | PointerKind::Cancel) {
                    state.drag = None;
                    (Some(PointerAction::Cancel { handle }), false)
                } else if state.stale_hover_clear_enabled(input.kind, mapped) {
                    (
                        Some(PointerAction::Move {
                            handle,
                            mapped: None,
                            input,
                            drag: None,
                            start: None,
                        }),
                        false,
                    )
                } else {
                    return false;
                }
            } else {
                match input.kind {
                    PointerKind::Down => {
                        if mapped.is_none() {
                            state.drag = None;
                            return false;
                        }
                        let forwarded = input.button != PointerButton::Right;
                        state.drag = Some(ActiveDrag {
                            button: input.button,
                            anchor: input.position,
                            last: input.position,
                            moved: false,
                            forwarded,
                        });
                        if forwarded {
                            (
                                Some(PointerAction::Down {
                                    handle,
                                    mapped,
                                    input,
                                }),
                                false,
                            )
                        } else {
                            (None, false)
                        }
                    }
                    PointerKind::Move => {
                        if mapped.is_none() && state.drag.is_some() {
                            state.drag = None;
                            (Some(PointerAction::Cancel { handle }), false)
                        } else {
                            let drag = state.drag.map(|mut drag| {
                                let delta = LogicalPoint::new(
                                    input.position.x - drag.last.x,
                                    input.position.y - drag.last.y,
                                );
                                let total = LogicalPoint::new(
                                    input.position.x - drag.anchor.x,
                                    input.position.y - drag.anchor.y,
                                );
                                drag.moved |= total.x.hypot(total.y) >= 3.0;
                                let start = if drag.button == PointerButton::Right
                                    && drag.moved
                                    && !drag.forwarded
                                {
                                    drag.forwarded = true;
                                    state
                                        .map_point(drag.anchor)
                                        .map(|mapped| (drag.anchor, mapped))
                                } else {
                                    None
                                };
                                drag.last = input.position;
                                state.drag = Some(drag);
                                (
                                    drag,
                                    if drag.moved {
                                        state.logical_delta_to_physical(delta)
                                    } else {
                                        LogicalPoint::default()
                                    },
                                    start,
                                )
                            });
                            if drag.is_some_and(|(drag, _, _)| {
                                drag.button == PointerButton::Right && !drag.moved
                            }) {
                                (None, false)
                            } else {
                                let (drag, start) = drag
                                    .map_or((None, None), |(drag, delta, start)| {
                                        (Some((drag, delta)), start)
                                    });
                                (
                                    Some(PointerAction::Move {
                                        handle,
                                        mapped,
                                        input,
                                        drag,
                                        start,
                                    }),
                                    false,
                                )
                            }
                        }
                    }
                    PointerKind::Up => {
                        let drag = state.drag.take();
                        if mapped.is_none()
                            || drag.is_some_and(|drag| {
                                drag.button != input.button
                                    || (drag.button == PointerButton::Right && !drag.forwarded)
                            })
                        {
                            (Some(PointerAction::Cancel { handle }), false)
                        } else {
                            (
                                Some(PointerAction::Up {
                                    handle,
                                    mapped,
                                    input,
                                    drag,
                                }),
                                false,
                            )
                        }
                    }
                    PointerKind::Cancel => {
                        state.drag = None;
                        (Some(PointerAction::Cancel { handle }), false)
                    }
                }
            }
        };
        if let Some(action) = action {
            self.apply_pointer_action(slot, action);
        }
        self.inner.sync_runtime(slot);
        open_context_menu
    }

    /// Apply a wheel zoom centered on the actual fitted image content.
    pub fn wheel(&self, slot: SlotId, delta_y: f64, position: LogicalPoint) -> bool {
        if !delta_y.is_finite() {
            return false;
        }
        let (handle, mapped) = {
            let slots = lock_slots(&self.inner.slots);
            let Some(state) = slots.get(&slot) else {
                return false;
            };
            if !state.interaction_enabled() {
                return false;
            }
            (state.plot.clone_handle(), state.map_point(position))
        };
        let Some(mapped) = mapped else {
            return false;
        };
        match handle {
            PlotHandle::TwoD(session) => {
                session.apply_input(PlotInputEvent::Zoom {
                    factor: (-delta_y * 0.0015).exp().clamp(0.1, 10.0),
                    center_px: mapped,
                });
            }
            #[cfg(feature = "3d")]
            PlotHandle::ThreeD(session) => {
                self.apply_3d_input(
                    slot,
                    &session,
                    ruviz::core::InputEvent3D::Wheel {
                        delta_y: (-delta_y) as f32,
                    },
                );
            }
        }
        self.inner.sync_runtime(slot);
        true
    }

    /// Reset a 2D view or 3D camera.
    pub fn reset_view(&self, slot: SlotId) {
        let handle = self.inner.plot_handle(slot);
        if handle.is_some() {
            // Escape/reset terminates any pointer sequence retained by either
            // the adapter or the core before changing the view.
            self.cancel_drag(slot);
        }
        match handle {
            Some(PlotHandle::TwoD(session)) => session.apply_input(PlotInputEvent::ResetView),
            #[cfg(feature = "3d")]
            Some(PlotHandle::ThreeD(session)) => {
                self.mutate_3d_camera(slot, &session, |session| session.reset_view());
            }
            None => {}
        }
        self.inner.sync_runtime(slot);
    }

    /// Execute one shared plot context-menu action.
    pub fn context_action(&self, slot: SlotId, action: PlotContextMenuAction) {
        match action {
            PlotContextMenuAction::ResetView => self.reset_view(slot),
            PlotContextMenuAction::FitToContent => self.fit_to_content(slot),
            PlotContextMenuAction::SaveImage => self.save_png_dialog(slot),
            PlotContextMenuAction::CopyImage => self.copy_image(slot),
            PlotContextMenuAction::ToggleInteraction => self.toggle_interaction(slot),
            #[cfg(feature = "3d")]
            PlotContextMenuAction::CameraView(view) => self.apply_camera_view(slot, view),
            _ => {}
        }
    }

    /// Fit the data while preserving the current 3D orientation.
    ///
    /// For 2D plots this is the natural data-bounds reset.
    pub fn fit_to_content(&self, slot: SlotId) {
        match self.inner.plot_handle(slot) {
            Some(PlotHandle::TwoD(session)) => {
                session.apply_input(PlotInputEvent::ResetView);
            }
            #[cfg(feature = "3d")]
            Some(PlotHandle::ThreeD(session)) => {
                self.mutate_3d_camera(slot, &session, |session| session.fit_to_content());
            }
            None => {}
        }
        self.inner.sync_runtime(slot);
    }

    /// Toggle whether a retained slot accepts plot interaction.
    pub fn toggle_interaction(&self, slot: SlotId) {
        let options = lock_slots(&self.inner.slots).get(&slot).map(|state| {
            let mut options = state.options;
            options.interaction = match options.interaction {
                InteractionMode::Static => InteractionMode::Interactive,
                InteractionMode::Interactive => InteractionMode::Static,
            };
            options
        });
        if let Some(options) = options {
            self.set_options(slot, options);
        }
    }

    #[cfg(feature = "3d")]
    fn apply_camera_view(&self, slot: SlotId, view: ruviz::core::CameraView3D) {
        let Some(PlotHandle::ThreeD(session)) = self.inner.plot_handle(slot) else {
            return;
        };
        self.mutate_3d_camera(slot, &session, |session| session.apply_camera_view(view));
        self.inner.sync_runtime(slot);
    }

    #[cfg(feature = "3d")]
    fn mutate_3d_camera(
        &self,
        slot: SlotId,
        session: &Arc<Mutex<ruviz::core::InteractivePlot3DSession>>,
        mutation: impl FnOnce(&mut ruviz::core::InteractivePlot3DSession) -> ruviz::core::Result<()>,
    ) {
        let snapshot = match lock_3d_session(session) {
            Ok(mut session) => {
                let previous = session.view_stamp();
                match mutation(&mut session) {
                    Ok(()) if !previous.same_camera(session.view_stamp()) => {
                        Some(session.camera_snapshot())
                    }
                    Ok(()) => None,
                    Err(error) => {
                        self.inner.report_error(slot, error.to_string());
                        None
                    }
                }
            }
            Err(message) => {
                self.inner.report_error(slot, message.to_string());
                None
            }
        };
        let Some(snapshot) = snapshot else {
            return;
        };
        let callback = lock_scalar_recover(&self.inner.callbacks).camera.clone();
        if let Some(callback) = callback
            && let Err(message) = catch_callback(|| callback(slot, snapshot))
        {
            self.inner
                .report_error(slot, format!("camera callback panicked: {message}"));
        }
        self.inner.request_render(slot);
    }

    /// Installed layers, kept uncomposed so presentation never blends them.
    ///
    /// Export composes on demand on its own worker; it is not a hot path.
    fn installed_layers(&self, slot: SlotId) -> Option<(Arc<RuvizImage>, Option<Arc<RuvizImage>>)> {
        lock_slots(&self.inner.slots)
            .get(&slot)
            .and_then(|state| state.last_frame.as_ref())
            .map(|frame| {
                (
                    Arc::clone(frame.base.image()),
                    frame
                        .overlay
                        .as_ref()
                        .map(|overlay| Arc::clone(overlay.image())),
                )
            })
    }

    fn save_png_dialog(&self, slot: SlotId) {
        let Some(layers) = self.installed_layers(slot) else {
            self.inner
                .report_error(slot, "no installed frame is available to save".to_string());
            return;
        };
        let worker_inner = Arc::clone(&self.inner);
        let spawn_error_inner = Arc::clone(&self.inner);
        let result = std::thread::Builder::new()
            .name(format!("ruviz-slint-save-png-{slot}"))
            .spawn(move || {
                let Some(path) = rfd::FileDialog::new()
                    .add_filter("PNG image", &["png"])
                    .set_file_name(format!("ruviz-plot-{slot}.png"))
                    .save_file()
                else {
                    return;
                };
                if let Err(message) =
                    write_image_png(&compose_layers(&layers.0, layers.1.as_deref()), path)
                {
                    worker_inner.report_error(slot, message);
                }
            });
        if let Err(error) = result {
            spawn_error_inner.report_error(slot, format!("failed to start PNG save: {error}"));
        }
    }

    fn copy_image(&self, slot: SlotId) {
        let Some(layers) = self.installed_layers(slot) else {
            self.inner
                .report_error(slot, "no installed frame is available to copy".to_string());
            return;
        };
        let worker_inner = Arc::clone(&self.inner);
        let spawn_error_inner = Arc::clone(&self.inner);
        let result = std::thread::Builder::new()
            .name(format!("ruviz-slint-copy-image-{slot}"))
            .spawn(move || {
                let image = compose_layers(&layers.0, layers.1.as_deref());
                let result = arboard::Clipboard::new()
                    .map_err(|error| format!("clipboard unavailable: {error}"))
                    .and_then(|mut clipboard| {
                        clipboard
                            .set_image(arboard::ImageData {
                                width: image.width as usize,
                                height: image.height as usize,
                                bytes: Cow::Borrowed(&image.pixels),
                            })
                            .map_err(|error| format!("failed to copy image: {error}"))
                    });
                if let Err(message) = result {
                    worker_inner.report_error(slot, message);
                }
            });
        if let Err(error) = result {
            spawn_error_inner.report_error(slot, format!("failed to start image copy: {error}"));
        }
    }

    /// Cancel a drag after release-outside, pointer capture loss, or focus loss.
    pub fn cancel_drag(&self, slot: SlotId) {
        let handle = {
            let mut slots = lock_slots(&self.inner.slots);
            let Some(state) = slots.get_mut(&slot) else {
                return;
            };
            state.drag = None;
            state.context_press = None;
            state.plot.clone_handle()
        };
        match handle {
            PlotHandle::TwoD(session) => {
                session.cancel_interaction();
            }
            #[cfg(feature = "3d")]
            PlotHandle::ThreeD(session) => match lock_3d_session(&session) {
                Ok(mut session) => {
                    session.cancel_drag();
                }
                Err(message) => self.inner.report_error(slot, message.to_string()),
            },
        }
        self.inner.sync_runtime(slot);
    }

    /// Handle a double-click reset using fitted physical coordinates.
    pub fn double_click(&self, slot: SlotId, position: LogicalPoint) {
        let (handle, mapped) = {
            let slots = lock_slots(&self.inner.slots);
            let Some(state) = slots.get(&slot) else {
                return;
            };
            if !state.interaction_enabled() {
                return;
            }
            (state.plot.clone_handle(), state.map_point(position))
        };
        #[cfg(not(feature = "3d"))]
        let _ = mapped;
        // Slint can still deliver the release for the press that formed this
        // double click. Clear it first so that release cannot select or pick
        // after the reset.
        self.cancel_drag(slot);
        match handle {
            PlotHandle::TwoD(session) => session.apply_input(PlotInputEvent::ResetView),
            #[cfg(feature = "3d")]
            PlotHandle::ThreeD(session) => {
                if let Some(mapped) = mapped {
                    self.apply_3d_input(
                        slot,
                        &session,
                        ruviz::core::InputEvent3D::DoubleClick {
                            x: mapped.x as f32,
                            y: mapped.y as f32,
                            button: ruviz::core::PointerButton3D::Left,
                        },
                    );
                }
            }
        }
        self.inner.sync_runtime(slot);
    }

    /// Install the pointer report callback.
    pub fn on_pointer(&self, callback: impl Fn(PointerReport) + Send + Sync + 'static) {
        lock_scalar_recover(&self.inner.callbacks).pointer = Some(Arc::new(callback));
    }

    /// Install the error callback.
    pub fn on_error(&self, callback: impl Fn(AdapterError) + Send + Sync + 'static) {
        lock_scalar_recover(&self.inner.callbacks).error = Some(Arc::new(callback));
    }

    /// Install the 3D pick callback.
    #[cfg(feature = "3d")]
    pub fn on_pick(
        &self,
        callback: impl Fn(SlotId, ruviz::core::PickHit3D) + Send + Sync + 'static,
    ) {
        lock_scalar_recover(&self.inner.callbacks).pick = Some(Arc::new(callback));
    }

    /// Install the callback invoked after an authoritative 3D camera change.
    #[cfg(feature = "3d")]
    pub fn on_camera_change(
        &self,
        callback: impl Fn(SlotId, ruviz::core::CameraSnapshot3D) + Send + Sync + 'static,
    ) {
        lock_scalar_recover(&self.inner.callbacks).camera = Some(Arc::new(callback));
    }

    /// Last frame dimensions installed for a slot.
    pub fn installed_size(&self, slot: SlotId) -> Option<(u32, u32)> {
        lock_slots(&self.inner.slots)
            .get(&slot)
            .and_then(|state| state.last_frame.as_ref())
            .map(|frame| frame.size_px)
    }

    /// Latest controller render generation installed for a slot.
    pub fn installed_generation(&self, slot: SlotId) -> Option<u64> {
        lock_slots(&self.inner.slots)
            .get(&slot)
            .and_then(|state| state.last_frame.as_ref())
            .map(|frame| frame.generation)
    }

    fn apply_pointer_action(&self, slot: SlotId, action: PointerAction) {
        match action {
            PointerAction::Down {
                handle,
                mapped,
                input,
            } => match handle {
                PlotHandle::TwoD(session) => {
                    if input.button == PointerButton::Right
                        && let Some(mapped) = mapped
                    {
                        session.apply_input(PlotInputEvent::BrushStart {
                            position_px: mapped,
                        });
                    }
                    self.report_pointer(
                        slot,
                        input.kind,
                        input.button,
                        input.position,
                        mapped,
                        Some(&session),
                    );
                }
                #[cfg(feature = "3d")]
                PlotHandle::ThreeD(session) => {
                    if let (Some(mapped), Some(button)) = (mapped, button_3d(input.button)) {
                        self.apply_3d_input(
                            slot,
                            &session,
                            ruviz::core::InputEvent3D::PointerDown {
                                x: mapped.x as f32,
                                y: mapped.y as f32,
                                button,
                            },
                        );
                    }
                    self.report_pointer(
                        slot,
                        input.kind,
                        input.button,
                        input.position,
                        mapped,
                        None,
                    );
                }
            },
            PointerAction::Move {
                handle,
                mapped,
                input,
                drag,
                start,
            } => match handle {
                PlotHandle::TwoD(session) => {
                    if let Some((logical_position, position_px)) = start {
                        session.apply_input(PlotInputEvent::BrushStart { position_px });
                        self.report_pointer(
                            slot,
                            PointerKind::Down,
                            PointerButton::Right,
                            logical_position,
                            Some(position_px),
                            Some(&session),
                        );
                    }
                    if let Some((drag, delta)) = drag {
                        if !drag.moved {
                            self.report_pointer(
                                slot,
                                input.kind,
                                input.button,
                                input.position,
                                mapped,
                                Some(&session),
                            );
                            return;
                        }
                        match drag.button {
                            PointerButton::Left => {
                                session.apply_input(PlotInputEvent::Pan {
                                    delta_px: ViewportPoint::new(delta.x, delta.y),
                                });
                            }
                            PointerButton::Right => {
                                session.apply_input(PlotInputEvent::BrushMove {
                                    position_px: mapped.unwrap_or_default(),
                                });
                            }
                            _ => {}
                        }
                    } else if let Some(position) = mapped {
                        session.apply_input(PlotInputEvent::Hover {
                            position_px: position,
                        });
                    } else {
                        session.apply_input(PlotInputEvent::ClearHover);
                    }
                    self.report_pointer(
                        slot,
                        input.kind,
                        input.button,
                        input.position,
                        mapped,
                        Some(&session),
                    );
                }
                #[cfg(feature = "3d")]
                PlotHandle::ThreeD(session) => {
                    if let Some((logical_position, start)) = start {
                        self.apply_3d_input(
                            slot,
                            &session,
                            ruviz::core::InputEvent3D::PointerDown {
                                x: start.x as f32,
                                y: start.y as f32,
                                button: ruviz::core::PointerButton3D::Right,
                            },
                        );
                        self.report_pointer(
                            slot,
                            PointerKind::Down,
                            PointerButton::Right,
                            logical_position,
                            Some(start),
                            None,
                        );
                    }
                    if let Some(mapped) = mapped {
                        self.apply_3d_input(
                            slot,
                            &session,
                            ruviz::core::InputEvent3D::PointerMove {
                                x: mapped.x as f32,
                                y: mapped.y as f32,
                            },
                        );
                    }
                    self.report_pointer(
                        slot,
                        input.kind,
                        input.button,
                        input.position,
                        mapped,
                        None,
                    );
                }
            },
            PointerAction::Up {
                handle,
                mapped,
                input,
                drag,
            } => match handle {
                PlotHandle::TwoD(session) => {
                    if let Some(mapped) = mapped {
                        match drag {
                            Some(ActiveDrag {
                                button: PointerButton::Right,
                                ..
                            }) => session.apply_input(PlotInputEvent::BrushEnd {
                                position_px: mapped,
                            }),
                            Some(ActiveDrag {
                                button: PointerButton::Left,
                                moved: false,
                                ..
                            }) => session.apply_input(PlotInputEvent::SelectAt {
                                position_px: mapped,
                            }),
                            _ => {}
                        }
                    }
                    self.report_pointer(
                        slot,
                        input.kind,
                        input.button,
                        input.position,
                        mapped,
                        Some(&session),
                    );
                }
                #[cfg(feature = "3d")]
                PlotHandle::ThreeD(session) => {
                    if let (Some(mapped), Some(button)) = (mapped, button_3d(input.button)) {
                        self.apply_3d_input(
                            slot,
                            &session,
                            ruviz::core::InputEvent3D::PointerUp {
                                x: mapped.x as f32,
                                y: mapped.y as f32,
                                button,
                            },
                        );
                    } else {
                        match lock_3d_session(&session) {
                            Ok(mut session) => {
                                session.cancel_drag();
                            }
                            Err(message) => self.inner.report_error(slot, message.to_string()),
                        }
                    }
                    self.report_pointer(
                        slot,
                        input.kind,
                        input.button,
                        input.position,
                        mapped,
                        None,
                    );
                }
            },
            PointerAction::Cancel { handle } => match handle {
                PlotHandle::TwoD(session) => {
                    session.cancel_interaction();
                }
                #[cfg(feature = "3d")]
                PlotHandle::ThreeD(session) => match lock_3d_session(&session) {
                    Ok(mut session) => {
                        session.cancel_drag();
                    }
                    Err(message) => self.inner.report_error(slot, message.to_string()),
                },
            },
        }
    }

    fn report_pointer(
        &self,
        slot: SlotId,
        kind: PointerKind,
        button: PointerButton,
        logical_position: LogicalPoint,
        physical_position: Option<ViewportPoint>,
        session: Option<&InteractivePlotSession>,
    ) {
        let callback = lock_scalar_recover(&self.inner.callbacks).pointer.clone();
        if let Some(callback) = callback {
            let hit = session
                .zip(physical_position)
                .map(|(session, position)| session.hit_test(position));
            if let Err(message) = catch_callback(|| {
                callback(PointerReport {
                    slot,
                    kind,
                    button,
                    logical_position,
                    physical_position,
                    hit,
                });
            }) {
                self.inner
                    .report_error(slot, format!("pointer callback panicked: {message}"));
            }
        }
    }

    #[cfg(feature = "3d")]
    fn apply_3d_input(
        &self,
        slot: SlotId,
        session: &Arc<Mutex<ruviz::core::InteractivePlot3DSession>>,
        event: ruviz::core::InputEvent3D,
    ) {
        let result = match lock_3d_session(session) {
            Ok(mut session) => session.handle_input(event),
            Err(message) => {
                self.inner.report_error(slot, message.to_string());
                return;
            }
        };
        match result {
            Ok(result) => {
                let (pick_callback, camera_callback) = {
                    let callbacks = lock_scalar_recover(&self.inner.callbacks);
                    (callbacks.pick.clone(), callbacks.camera.clone())
                };
                if let (Some(hit), Some(callback)) = (result.picked, pick_callback)
                    && let Err(message) = catch_callback(|| callback(slot, hit))
                {
                    self.inner
                        .report_error(slot, format!("pick callback panicked: {message}"));
                }
                if result.camera_changed
                    && let Some(callback) = camera_callback
                {
                    match lock_3d_session(session) {
                        Ok(session) => {
                            let snapshot = session.camera_snapshot();
                            if let Err(message) = catch_callback(|| callback(slot, snapshot)) {
                                self.inner.report_error(
                                    slot,
                                    format!("camera callback panicked: {message}"),
                                );
                            }
                        }
                        Err(message) => self.inner.report_error(slot, message.to_string()),
                    }
                }
                if result.request_redraw {
                    self.inner.request_render(slot);
                }
            }
            Err(error) => self.inner.report_error(slot, error.to_string()),
        }
    }
}

impl ControllerInner {
    fn sync_runtime(self: &Arc<Self>, slot: SlotId) {
        let Some(config_sink) = self.runtime_config_sink.clone() else {
            return;
        };
        let Some((incarnation, interaction, fit, scale, is_3d, has_frame)) =
            lock_slots(&self.slots).get(&slot).map(|state| {
                (
                    state.incarnation,
                    state.options.interaction,
                    state.options.fit,
                    state.layout.scale_factor,
                    state.plot.is_3d(),
                    state.last_frame.is_some(),
                )
            })
        else {
            return;
        };
        let weak = Arc::downgrade(self);
        let task = Box::new(move || {
            let Some(inner) = weak.upgrade() else {
                return;
            };
            let current = lock_slots(&inner.slots)
                .get(&slot)
                .is_some_and(|state| state.incarnation == incarnation);
            if current
                && let Err(message) = catch_callback(|| {
                    config_sink(slot, interaction, fit, scale, is_3d, has_frame);
                })
            {
                inner.report_error_direct(
                    slot,
                    format!("runtime configuration callback panicked: {message}"),
                );
            }
        });
        if let Err(error) = self.dispatch(task) {
            self.report_error_direct(
                slot,
                format!("could not schedule runtime configuration: {error}"),
            );
        }
    }

    fn clear_runtime(self: &Arc<Self>, slot: SlotId) {
        let sink = Arc::clone(&self.frame_sink);
        let overlay_sink = lock_scalar_recover(&self.overlay_sink).clone();
        let config_sink = self.runtime_config_sink.clone();
        let weak = Arc::downgrade(self);
        let task = Box::new(move || {
            let Some(inner) = weak.upgrade() else {
                return;
            };
            if lock_slots(&inner.slots).contains_key(&slot) {
                return;
            }
            if let Err(message) = catch_callback(|| sink(slot, slint::Image::default())) {
                inner.report_error_direct(
                    slot,
                    format!("runtime image callback panicked while clearing a slot: {message}"),
                );
            }
            if let Some(overlay_sink) = overlay_sink
                && let Err(message) = catch_callback(|| overlay_sink(slot, None))
            {
                inner.report_error_direct(
                    slot,
                    format!("runtime overlay callback panicked while clearing a slot: {message}"),
                );
            }
            if let Some(config_sink) = config_sink
                && let Err(message) = catch_callback(|| {
                    config_sink(
                        slot,
                        InteractionMode::Static,
                        ImageFit::Contain,
                        1.0,
                        false,
                        false,
                    );
                })
            {
                inner.report_error_direct(
                    slot,
                    format!("runtime configuration callback panicked: {message}"),
                );
            }
        });
        if let Err(error) = self.dispatch(task) {
            self.report_error_direct(slot, format!("could not clear runtime slot: {error}"));
        }
    }

    /// Redraw every retained slot, used when the presentation itself changes.
    fn request_render_all(self: &Arc<Self>) {
        let slots: Vec<SlotId> = lock_slots(&self.slots).keys().copied().collect();
        for slot in slots {
            self.request_render(slot);
        }
    }

    fn request_render(self: &Arc<Self>, slot: SlotId) {
        let layered = lock_scalar_recover(&self.overlay_sink).is_some();
        let scheduled = {
            let mut slots = lock_slots(&self.slots);
            let Some(state) = slots.get_mut(&slot) else {
                return;
            };
            let target = target_size(state.layout, state.options.sizing);
            let incarnation = state.incarnation;
            // Only layers presented for the live incarnation under the current
            // presentation are still on screen; anything else must be
            // republished so a stale overlay cannot survive.
            let published = state
                .presented
                .as_ref()
                .filter(|presented| {
                    presented.incarnation == incarnation && presented.layered == layered
                })
                .map(|presented| presented.layers.clone());
            let kind: Result<RenderRequestKind, String> = match &state.plot {
                PlotSlot::TwoD(session) => Ok(RenderRequestKind::TwoD {
                    session: session.clone(),
                    target: ImageTarget {
                        size_px: target,
                        scale_factor: state.layout.scale_factor,
                        time_seconds: 0.0,
                    },
                }),
                #[cfg(feature = "3d")]
                PlotSlot::ThreeD {
                    session, renderer, ..
                } => match lock_3d_session(session) {
                    Ok(mut session_guard) => session_guard
                        .background_render_job()
                        .map(|job| RenderRequestKind::ThreeD {
                            session: Arc::clone(session),
                            renderer: Arc::clone(renderer),
                            job,
                        })
                        .map_err(|error| error.to_string()),
                    Err(message) => Err(message.to_string()),
                },
            };
            kind.map(|kind| {
                state.scheduler.request(RenderRequest {
                    incarnation,
                    published,
                    layered,
                    kind,
                })
            })
        };
        match scheduled {
            Ok(Some(scheduled)) => self.spawn_render(slot, scheduled),
            Ok(None) => {}
            Err(error) => self.report_error(slot, error),
        }
    }

    /// Hand a scheduled render to the slot's persistent worker.
    ///
    /// The worker owns the slot's render lane, so the depth-1 latest-wins
    /// scheduler stays the only queue and requests never pile up as threads.
    fn spawn_render(self: &Arc<Self>, slot: SlotId, scheduled: ScheduledRequest<RenderRequest>) {
        let id = scheduled.id();
        let request = scheduled.into_request();
        let incarnation = request.incarnation;
        let job = RenderJob {
            id: id.clone(),
            incarnation,
            request,
            #[cfg(test)]
            barrier: lock_scalar_recover(&self.render_barrier).clone(),
        };
        if let Err(error) = self.render_worker(slot).and_then(|worker| {
            worker
                .send(job)
                .map_err(|_| "plot render worker stopped".to_string())
        }) {
            self.finish_render(
                slot,
                id,
                incarnation,
                Err(WorkerFailure::Error(format!(
                    "could not start plot render worker: {error}"
                ))),
            );
        }
    }

    /// The slot's render lane, started on first use and retained afterwards.
    ///
    /// Lanes outlive `remove_plot` so a detached in-flight render still owns
    /// the lane when the same slot identifier is registered again.
    fn render_worker(self: &Arc<Self>, slot: SlotId) -> Result<mpsc::Sender<RenderJob>, String> {
        let mut workers = lock_scalar_recover(&self.render_workers);
        if let Some(worker) = workers.get(&slot) {
            return Ok(worker.sender.clone());
        }
        let worker = RenderWorker::start(slot, Arc::downgrade(self))?;
        let sender = worker.sender.clone();
        workers.insert(slot, worker);
        Ok(sender)
    }

    #[cfg(test)]
    fn render_lane_busy(&self, slot: SlotId) -> bool {
        lock_scalar_recover(&self.render_workers)
            .get(&slot)
            .is_some_and(|worker| worker.busy.load(Ordering::SeqCst))
    }

    fn finish_render(
        self: &Arc<Self>,
        slot: SlotId,
        id: ScheduledRequestId,
        incarnation: u64,
        result: Result<RenderedFrame, WorkerFailure>,
    ) {
        let (install, next) = {
            let mut slots = lock_slots(&self.slots);
            let Some(state) = slots.get_mut(&slot) else {
                return;
            };
            let Some(completion) = state.scheduler.complete(id.clone()) else {
                return;
            };
            (
                completion.install && state.incarnation == incarnation,
                completion.next,
            )
        };

        match result {
            Ok(rendered) if install && rendered.is_current() => {
                let weak = Arc::downgrade(self);
                let sink = Arc::clone(&self.frame_sink);
                let overlay_sink = lock_scalar_recover(&self.overlay_sink).clone();
                let generation = id.generation();
                let task = Box::new(move || {
                    let Some(inner) = weak.upgrade() else {
                        return;
                    };
                    let still_current = {
                        let slots = lock_slots(&inner.slots);
                        matches!(
                            slots.get(&slot),
                            Some(state)
                                if state.incarnation == incarnation
                                    && state.scheduler.latest_generation() == generation
                                    && rendered.is_current()
                        )
                    };
                    if !still_current {
                        return;
                    }
                    let RenderedFrame {
                        base,
                        overlay,
                        base_layer,
                        overlay_layer,
                        size_px,
                        layered,
                        validity,
                    } = rendered;
                    // The worker skipped a layer against the state presented
                    // when the request was scheduled, which an earlier install
                    // may have moved on from since. Only the UI thread installs
                    // layers, so re-check the hint here and copy after all when
                    // it no longer holds.
                    let (base_reusable, overlay_reusable) = {
                        let slots = lock_slots(&inner.slots);
                        match slots.get(&slot).and_then(|state| state.presented.as_ref()) {
                            Some(presented)
                                if presented.incarnation == incarnation
                                    && presented.layered == layered =>
                            {
                                (
                                    presented.layers.base.same_buffer_as(&base_layer),
                                    match (&presented.layers.overlay, &overlay_layer) {
                                        (None, None) => true,
                                        (Some(shown), Some(next)) => shown.same_buffer_as(next),
                                        _ => false,
                                    },
                                )
                            }
                            _ => (false, false),
                        }
                    };
                    let base = match base {
                        None if !base_reusable => Some(LayerBuffer::from_layer(&base_layer)),
                        base => base,
                    };
                    let overlay = match overlay {
                        OverlayUpdate::Reuse if layered && !overlay_reusable => {
                            OverlayUpdate::Replace(
                                overlay_layer.as_ref().map(LayerBuffer::from_layer),
                            )
                        }
                        overlay => overlay,
                    };
                    if let Some(base) = base
                        && let Err(message) = catch_callback(|| sink(slot, base.into_slint_image()))
                    {
                        // What the sinks hold is now unknown, so the next frame
                        // must republish both layers instead of diffing.
                        inner.forget_presented(slot);
                        inner.report_error_direct(
                            slot,
                            format!("runtime image callback panicked: {message}"),
                        );
                        return;
                    }
                    if let OverlayUpdate::Replace(layer) = overlay
                        && let Some(overlay_sink) = overlay_sink
                        && let Err(message) = catch_callback(|| {
                            overlay_sink(slot, layer.map(LayerBuffer::into_slint_image));
                        })
                    {
                        inner.forget_presented(slot);
                        inner.report_error_direct(
                            slot,
                            format!("runtime overlay callback panicked: {message}"),
                        );
                        return;
                    }

                    let presented = PresentedLayers {
                        incarnation,
                        layered,
                        layers: PublishedLayers {
                            base: base_layer.clone(),
                            overlay: overlay_layer.clone(),
                        },
                    };
                    let committed = {
                        let mut slots = lock_slots(&inner.slots);
                        match slots.get_mut(&slot) {
                            // Both layers are on screen now, so record them
                            // whether or not the frame is still committable: a
                            // rejected frame changed the presentation anyway.
                            Some(state) => {
                                state.presented = Some(presented);
                                if state.incarnation == incarnation
                                    && state.scheduler.latest_generation() == generation
                                    && validity.is_current()
                                {
                                    state.last_frame = Some(InstalledFrame {
                                        base: base_layer,
                                        overlay: overlay_layer,
                                        size_px,
                                        generation,
                                        incarnation,
                                        validity,
                                    });
                                    true
                                } else {
                                    false
                                }
                            }
                            None => false,
                        }
                    };
                    if committed {
                        inner.sync_runtime(slot);
                    }
                });
                if let Err(error) = self.dispatch(task) {
                    self.report_error(slot, format!("could not schedule frame install: {error}"));
                }
            }
            Ok(_) => {}
            Err(WorkerFailure::Superseded) => {}
            Err(WorkerFailure::Error(message)) if install => self.report_error(slot, message),
            Err(WorkerFailure::Error(_)) => {}
        }

        if let Some(next) = next {
            self.spawn_render(slot, next);
        }
    }

    /// Forget which layers a slot presents.
    ///
    /// The next frame then republishes both layers instead of reusing what it
    /// believes is on screen.
    fn forget_presented(&self, slot: SlotId) {
        if let Some(state) = lock_slots(&self.slots).get_mut(&slot) {
            state.presented = None;
        }
    }

    fn plot_handle(&self, slot: SlotId) -> Option<PlotHandle> {
        lock_slots(&self.slots)
            .get(&slot)
            .map(|state| state.plot.clone_handle())
    }

    fn dispatch(&self, task: UiTask) -> Result<(), String> {
        catch_callback(|| (self.dispatcher)(task))
            .map_err(|message| format!("UI dispatcher panicked: {message}"))?
    }

    fn report_error(&self, slot: SlotId, message: String) {
        let callback = lock_scalar_recover(&self.callbacks).error.clone();
        if let Some(callback) = callback {
            let task = Box::new(move || {
                if let Err(message) = catch_callback(|| callback(AdapterError { slot, message })) {
                    write_callback_diagnostic(&format!("error callback panicked: {message}"));
                }
            });
            if let Err(error) = self.dispatch(task) {
                self.report_error_direct(
                    slot,
                    format!("could not schedule error callback: {error}"),
                );
            }
        }
    }

    fn report_error_direct(&self, slot: SlotId, message: String) {
        let callback = lock_scalar_recover(&self.callbacks).error.clone();
        if let Some(callback) = callback
            && let Err(message) = catch_callback(|| callback(AdapterError { slot, message }))
        {
            write_callback_diagnostic(&format!("error callback panicked: {message}"));
        }
    }
}

impl RenderedFrame {
    fn is_current(&self) -> bool {
        match &self.validity {
            RenderValidity::TwoD { session, stamp } => session.is_render_stamp_current(*stamp),
            #[cfg(feature = "3d")]
            RenderValidity::ThreeD { session, stamp } => {
                lock_3d_session(session).is_ok_and(|session| session.is_render_current(*stamp))
            }
        }
    }
}

fn render_request(request: RenderRequest) -> Result<RenderedFrame, WorkerFailure> {
    let RenderRequest {
        published,
        layered,
        kind,
        ..
    } = request;
    let (base_layer, overlay_layer, validity) = match kind {
        RenderRequestKind::TwoD { session, target } => {
            // Layered: the base is only re-rendered when it actually changed,
            // so an overlay-only redraw costs neither a composite nor a copy.
            let frame = session.render_layers_stamped(target).map_err(|error| {
                if error.is_render_superseded() {
                    WorkerFailure::Superseded
                } else {
                    WorkerFailure::Error(error.to_string())
                }
            })?;
            let stamp = frame.render_stamp();
            (
                frame.base,
                frame.overlay,
                RenderValidity::TwoD { session, stamp },
            )
        }
        #[cfg(feature = "3d")]
        RenderRequestKind::ThreeD {
            session,
            renderer,
            job,
        } => {
            let rendered = lock_3d_renderer(&renderer)
                .map_err(|message| WorkerFailure::Error(message.to_string()))?
                .render(job)
                .map_err(|error| WorkerFailure::Error(error.to_string()))?;
            let stamp = rendered.stamp;
            (
                RenderedLayer::from_straight_image(Arc::new(rendered.image)),
                None,
                RenderValidity::ThreeD { session, stamp },
            )
        }
    };
    validate_layer(&base_layer)?;
    let size_px = (base_layer.width(), base_layer.height());
    if let Some(overlay) = overlay_layer.as_ref() {
        validate_layer(overlay)?;
        if (overlay.width(), overlay.height()) != size_px {
            return Err(WorkerFailure::Error(
                "overlay layer does not match the base layer size".to_string(),
            ));
        }
    }

    let (base, overlay) = if layered {
        let base = (!published
            .as_ref()
            .is_some_and(|published| published.base.same_buffer_as(&base_layer)))
        .then(|| LayerBuffer::from_layer(&base_layer));
        // An unknown presented state must clear the overlay rather than reuse
        // it, so a replaced plot can never keep the old plot's overlay.
        let overlay = match (published.map(|published| published.overlay), &overlay_layer) {
            (Some(None), None) => OverlayUpdate::Reuse,
            (Some(Some(shown)), Some(next)) if shown.same_buffer_as(next) => OverlayUpdate::Reuse,
            (_, Some(next)) => OverlayUpdate::Replace(Some(LayerBuffer::from_layer(next))),
            (_, None) => OverlayUpdate::Replace(None),
        };
        (base, overlay)
    } else {
        // One flat image for a controller without an overlay sink.
        let composed = compose_layers(
            base_layer.image(),
            overlay_layer
                .as_ref()
                .map(|overlay| overlay.image().as_ref()),
        );
        (Some(LayerBuffer::new(&composed)), OverlayUpdate::Reuse)
    };

    Ok(RenderedFrame {
        base,
        overlay,
        base_layer,
        overlay_layer,
        size_px,
        layered,
        validity,
    })
}

/// Validate a layer's native buffer without materializing a straight view.
fn validate_layer(layer: &RenderedLayer) -> Result<(), WorkerFailure> {
    validate_rgba_dimensions(layer.width(), layer.height(), layer.pixels().len())
}

fn validate_rgba_dimensions(width: u32, height: u32, len: usize) -> Result<(), WorkerFailure> {
    let expected = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| {
            WorkerFailure::Error("rendered image dimensions overflow address space".to_string())
        })?;
    if len != expected {
        return Err(WorkerFailure::Error(format!(
            "rendered RGBA buffer has {len} bytes, expected {expected}"
        )));
    }
    Ok(())
}

/// Blend an overlay layer over its base layer.
///
/// Presentation never calls this: Slint stacks the two images and the renderer
/// composites them. It is only used for PNG export, clipboard copy, and the
/// single-sink controller that cannot present two layers.
fn compose_layers(base: &Arc<RuvizImage>, overlay: Option<&RuvizImage>) -> Arc<RuvizImage> {
    let Some(overlay) = overlay.filter(|overlay| {
        (overlay.width, overlay.height) == (base.width, base.height)
            && overlay.pixels.len() == base.pixels.len()
    }) else {
        return Arc::clone(base);
    };
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
    Arc::new(RuvizImage::from_straight_rgba(
        base.width,
        base.height,
        pixels,
    ))
}

enum PlotHandle {
    TwoD(InteractivePlotSession),
    #[cfg(feature = "3d")]
    ThreeD(Arc<Mutex<ruviz::core::InteractivePlot3DSession>>),
}

impl PlotSlot {
    fn clone_handle(&self) -> PlotHandle {
        match self {
            Self::TwoD(session) => PlotHandle::TwoD(session.clone()),
            #[cfg(feature = "3d")]
            Self::ThreeD { session, .. } => PlotHandle::ThreeD(Arc::clone(session)),
        }
    }
}

enum PointerAction {
    Down {
        handle: PlotHandle,
        mapped: Option<ViewportPoint>,
        input: PointerInput,
    },
    Move {
        handle: PlotHandle,
        mapped: Option<ViewportPoint>,
        input: PointerInput,
        drag: Option<(ActiveDrag, LogicalPoint)>,
        start: Option<(LogicalPoint, ViewportPoint)>,
    },
    Up {
        handle: PlotHandle,
        mapped: Option<ViewportPoint>,
        input: PointerInput,
        drag: Option<ActiveDrag>,
    },
    Cancel {
        handle: PlotHandle,
    },
}

impl SlotState {
    fn interaction_enabled(&self) -> bool {
        self.options.interaction == InteractionMode::Interactive
            && self
                .last_frame
                .as_ref()
                .is_some_and(|frame| frame.incarnation == self.incarnation && frame.is_current())
    }

    fn pointer_input_enabled(&self, kind: PointerKind) -> bool {
        self.options.interaction == InteractionMode::Interactive
            && (self.interaction_enabled()
                || (self.drag.is_some()
                    && matches!(
                        kind,
                        PointerKind::Move | PointerKind::Up | PointerKind::Cancel
                    )))
    }

    fn stale_hover_clear_enabled(&self, kind: PointerKind, mapped: Option<ViewportPoint>) -> bool {
        self.options.interaction == InteractionMode::Interactive
            && matches!(&self.plot, PlotSlot::TwoD(_))
            && kind == PointerKind::Move
            && mapped.is_none()
            && self.drag.is_none()
            && self
                .last_frame
                .as_ref()
                .is_some_and(|frame| frame.incarnation == self.incarnation && !frame.is_current())
    }

    fn image_size(&self) -> (u32, u32) {
        self.last_frame.as_ref().map_or_else(
            || target_size(self.layout, self.options.sizing),
            |frame| frame.size_px,
        )
    }

    fn content_rect(&self) -> LogicalRect {
        fitted_content_rect(self.layout.outer, self.image_size(), self.options.fit)
    }

    fn map_point(&self, point: LogicalPoint) -> Option<ViewportPoint> {
        if !self.layout.outer.contains(point) {
            return None;
        }
        logical_to_physical(self.content_rect(), point, self.image_size())
            .map(|(x, y)| ViewportPoint::new(x, y))
    }

    fn logical_delta_to_physical(&self, delta: LogicalPoint) -> LogicalPoint {
        let content = self.content_rect();
        let image = self.image_size();
        LogicalPoint::new(
            delta.x / content.width.max(f64::EPSILON) * f64::from(image.0),
            delta.y / content.height.max(f64::EPSILON) * f64::from(image.1),
        )
    }
}

const VIEW_BOUNDS_NORMALIZED_TOLERANCE: f64 = 1e-12;

fn viewport_bounds_materially_differ(
    visible: ViewportRect,
    base: ViewportRect,
    x_scale: &AxisScale,
    y_scale: &AxisScale,
) -> bool {
    !viewport_axis_bounds_close(
        (visible.min.x, visible.max.x),
        (base.min.x, base.max.x),
        x_scale,
    ) || !viewport_axis_bounds_close(
        (visible.min.y, visible.max.y),
        (base.min.y, base.max.y),
        y_scale,
    )
}

fn viewport_axis_bounds_close(visible: (f64, f64), base: (f64, f64), scale: &AxisScale) -> bool {
    if (visible.1 - visible.0).is_sign_negative() != (base.1 - base.0).is_sign_negative() {
        return false;
    }

    let start = scale.normalized_position(visible.0, base.0, base.1);
    let end = scale.normalized_position(visible.1, base.0, base.1);
    start.is_finite()
        && end.is_finite()
        && start.abs() <= VIEW_BOUNDS_NORMALIZED_TOLERANCE
        && (end - 1.0).abs() <= VIEW_BOUNDS_NORMALIZED_TOLERANCE
}

fn target_size(layout: SlotLayout, sizing: SizingMode) -> (u32, u32) {
    match sizing {
        SizingMode::Fill => {
            physical_backing_size(layout.outer.width, layout.outer.height, layout.scale_factor)
        }
        SizingMode::Fixed {
            width_px,
            height_px,
        } => (width_px.max(1), height_px.max(1)),
    }
}

fn decode_pointer_kind(value: i32) -> Option<PointerKind> {
    match value {
        0 => Some(PointerKind::Down),
        1 => Some(PointerKind::Up),
        2 => Some(PointerKind::Move),
        3 => Some(PointerKind::Cancel),
        _ => None,
    }
}

fn decode_button(value: i32) -> Option<PointerButton> {
    match value {
        0 => Some(PointerButton::None),
        1 => Some(PointerButton::Left),
        2 => Some(PointerButton::Right),
        3 => Some(PointerButton::Middle),
        _ => None,
    }
}

fn decode_context_action(action: RuvizContextAction) -> Option<PlotContextMenuAction> {
    Some(match action {
        RuvizContextAction::ResetView => PlotContextMenuAction::ResetView,
        RuvizContextAction::FitToContent => PlotContextMenuAction::FitToContent,
        RuvizContextAction::SavePng => PlotContextMenuAction::SaveImage,
        RuvizContextAction::CopyImage => PlotContextMenuAction::CopyImage,
        RuvizContextAction::ToggleInteraction => PlotContextMenuAction::ToggleInteraction,
        #[cfg(feature = "3d")]
        RuvizContextAction::ViewIsometric => {
            PlotContextMenuAction::CameraView(ruviz::core::CameraView3D::Isometric)
        }
        #[cfg(feature = "3d")]
        RuvizContextAction::ViewFront => {
            PlotContextMenuAction::CameraView(ruviz::core::CameraView3D::Front)
        }
        #[cfg(feature = "3d")]
        RuvizContextAction::ViewBack => {
            PlotContextMenuAction::CameraView(ruviz::core::CameraView3D::Back)
        }
        #[cfg(feature = "3d")]
        RuvizContextAction::ViewLeft => {
            PlotContextMenuAction::CameraView(ruviz::core::CameraView3D::Left)
        }
        #[cfg(feature = "3d")]
        RuvizContextAction::ViewRight => {
            PlotContextMenuAction::CameraView(ruviz::core::CameraView3D::Right)
        }
        #[cfg(feature = "3d")]
        RuvizContextAction::ViewTop => {
            PlotContextMenuAction::CameraView(ruviz::core::CameraView3D::Top)
        }
        #[cfg(feature = "3d")]
        RuvizContextAction::ViewBottom => {
            PlotContextMenuAction::CameraView(ruviz::core::CameraView3D::Bottom)
        }
        #[cfg(not(feature = "3d"))]
        RuvizContextAction::ViewIsometric
        | RuvizContextAction::ViewFront
        | RuvizContextAction::ViewBack
        | RuvizContextAction::ViewLeft
        | RuvizContextAction::ViewRight
        | RuvizContextAction::ViewTop
        | RuvizContextAction::ViewBottom => return None,
    })
}

fn next_slot_incarnation() -> Option<u64> {
    reserve_slot_incarnation(&NEXT_SLOT_INCARNATION)
}

fn reserve_slot_incarnation(counter: &AtomicU64) -> Option<u64> {
    counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .ok()
}

fn update_runtime_image(runtime: &RuvizRuntime<'_>, slot: SlotId, image: slint::Image) {
    update_runtime_row(runtime, slot, |row| row.source = image);
}

fn update_runtime_overlay(runtime: &RuvizRuntime<'_>, slot: SlotId, overlay: slint::Image) {
    update_runtime_row(runtime, slot, |row| row.overlay = overlay);
}

fn update_runtime_config(
    runtime: &RuvizRuntime<'_>,
    slot: SlotId,
    interaction: InteractionMode,
    fit: ImageFit,
    scale_factor: f32,
    is_3d: bool,
    has_frame: bool,
) {
    update_runtime_row(runtime, slot, |row| {
        row.interactive = interaction == InteractionMode::Interactive;
        row.is_3d = is_3d;
        row.has_frame = has_frame;
        row.image_fit = match fit {
            ImageFit::Contain => RuvizImageFit::Contain,
            ImageFit::Cover => RuvizImageFit::Cover,
            ImageFit::Fill => RuvizImageFit::Fill,
        };
        row.device_scale = sanitize_scale_factor(scale_factor);
    });
}

fn update_runtime_row(
    runtime: &RuvizRuntime<'_>,
    slot: SlotId,
    update: impl FnOnce(&mut RuvizSlotState),
) {
    let Ok(index) = usize::try_from(slot) else {
        return;
    };
    let model = runtime.get_slots();
    // Update in place when the row already exists so a frame install does not
    // rebuild the model and re-instantiate every slot in the repeater.
    if index < model.row_count()
        && let Some(rows) = model
            .as_any()
            .downcast_ref::<slint::VecModel<RuvizSlotState>>()
        && let Some(mut row) = model.row_data(index)
    {
        update(&mut row);
        rows.set_row_data(index, row);
        return;
    }
    let mut rows = (0..model.row_count())
        .map(|row| model.row_data(row).unwrap_or_default())
        .collect::<Vec<_>>();
    rows.resize_with(index.saturating_add(1), || RuvizSlotState {
        device_scale: 1.0,
        ..RuvizSlotState::default()
    });
    update(&mut rows[index]);
    runtime.set_slots(slint::ModelRc::new(slint::VecModel::from(rows)));
}

#[cfg(feature = "3d")]
fn new_background_renderer(prefer_gpu: bool) -> Arc<Mutex<ruviz::core::BackgroundRenderer3D>> {
    let backend = background_backend(prefer_gpu);
    Arc::new(Mutex::new(ruviz::core::BackgroundRenderer3D::new(backend)))
}

#[cfg(feature = "3d")]
fn background_backend(prefer_gpu: bool) -> ruviz::core::BackgroundRenderBackend3D {
    #[cfg(all(feature = "3d-gpu", not(target_arch = "wasm32")))]
    if prefer_gpu {
        return ruviz::core::BackgroundRenderBackend3D::GpuReadback;
    }
    let _ = prefer_gpu;
    ruviz::core::BackgroundRenderBackend3D::Cpu
}

#[cfg(feature = "3d")]
fn button_3d(button: PointerButton) -> Option<ruviz::core::PointerButton3D> {
    match button {
        PointerButton::Left => Some(ruviz::core::PointerButton3D::Left),
        PointerButton::Right => Some(ruviz::core::PointerButton3D::Right),
        PointerButton::Middle => Some(ruviz::core::PointerButton3D::Middle),
        PointerButton::None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        time::{Duration, Instant},
    };

    fn test_controller() -> (RuvizController, Arc<AtomicUsize>) {
        let frames = Arc::new(AtomicUsize::new(0));
        let installed = Arc::clone(&frames);
        let controller = RuvizController::with_dispatcher(
            move |_, _| {
                installed.fetch_add(1, Ordering::SeqCst);
            },
            |task| {
                task();
                Ok(())
            },
        );
        (controller, frames)
    }

    /// Block until `condition` holds, or fail with enough detail to diagnose why.
    ///
    /// This times out intermittently on CI (issue #152) — first on Linux and,
    /// as of 2026-08, on a 4-core Windows runner, where four 3D tests failed
    /// together at ~18M polls each: the waiters had CPU, the render workers
    /// did not. Four concurrent 3D renders (each with a rayon tile pool) plus
    /// four yield-spinning waiters oversubscribe a 4-core runner heavily,
    /// which is why CI now runs these suites with --test-threads=1. The bare
    /// message it used to fail with could not distinguish a worker that never
    /// ran from one that ran and never finished, so every theory about it
    /// stayed a theory.
    ///
    /// The poll count is the discriminator. A waiter that spun millions of
    /// times was not itself starved of CPU, which is the reading the spin
    /// invites; one that barely polled was descheduled and the machine, not
    /// this loop, is the story. `available_parallelism` records how many cores
    /// the failing runner actually had, which is otherwise guesswork.
    ///
    /// Do not change the waiting primitive to "fix" the flake without a
    /// reproduction and a control run: swapping `yield_now` for a 1ms sleep
    /// was tried and made these tests fail 11 runs out of 12 under load.
    ///
    /// The deadline is 60s because 10s was measured too tight for shared CI
    /// runners: with the suites already serialized, a 4-core Linux runner
    /// still timed out at 9.2M polls — the waiter had CPU the whole time and
    /// the render work was genuinely in flight, just slow next to noisy
    /// neighbors. A passing wait exits the moment the condition holds, so
    /// headroom costs nothing except on a real hang, which still fails.
    fn wait_for(condition: impl Fn() -> bool) {
        let start = Instant::now();
        let mut polls: u64 = 0;
        while !condition() {
            let elapsed = start.elapsed();
            assert!(
                elapsed < Duration::from_secs(60),
                "timed out waiting for render worker after {polls} polls in {elapsed:?} \
                 (available_parallelism: {:?})",
                std::thread::available_parallelism(),
            );
            polls += 1;
            std::thread::yield_now();
        }
    }

    #[test]
    fn packaged_component_focuses_on_pointer_down() {
        let component = include_str!("../ui/ruviz.slint").replace("\r\n", "\n");
        assert!(
            component.contains("if (event.kind == PointerEventKind.down)")
                && component.contains("input.focus();"),
            "RuvizPlot must acquire focus on press so Escape and focus loss are routed"
        );
    }

    #[test]
    fn packaged_component_exposes_the_full_context_menu_contract() {
        let component = include_str!("../ui/ruviz.slint");
        for required in [
            "ContextMenuArea",
            "Reset View",
            "Fit to Content",
            "Save PNG",
            "Copy Image",
            "toggle-interaction",
            "3D View",
            "view-isometric",
            "view-bottom",
        ] {
            assert!(
                component.contains(required),
                "packaged context menu must contain {required}"
            );
        }
        assert!(
            component.matches("enabled: root.slot-valid;").count() >= 3,
            "context area, focus scope, and touch area must remain available for static slots"
        );
        assert!(
            !component.contains("enabled: root.interactive;"),
            "Rust gates plot gestures; Slint must keep the keyboard/menu path enabled"
        );
    }

    #[test]
    fn packaged_grid_avoids_dynamic_grid_layout_metadata() {
        let component = include_str!("../ui/ruviz.slint");
        assert!(
            !component.contains("GridLayout"),
            "runtime slot repeaters must not participate in Slint's grid-layout metadata"
        );
        assert!(
            component.contains("width: root.cell-width;")
                && component.contains("height: root.cell-height;"),
            "runtime slots must retain explicit responsive cell geometry"
        );
    }

    #[cfg(feature = "3d")]
    fn two_d_session(handle: PlotHandle) -> InteractivePlotSession {
        match handle {
            PlotHandle::TwoD(session) => session,
            PlotHandle::ThreeD(_) => panic!("expected a 2D slot"),
        }
    }

    #[cfg(not(feature = "3d"))]
    fn two_d_session(handle: PlotHandle) -> InteractivePlotSession {
        let PlotHandle::TwoD(session) = handle;
        session
    }

    #[test]
    fn fill_sizing_uses_fractional_hidpi() {
        let (controller, frames) = test_controller();
        controller.set_plot(
            1,
            ruviz::prelude::Plot::new().line(&[0.0, 1.0], &[1.0, 2.0]),
            SlotOptions::default(),
        );
        controller.resize(1, 100.25, 50.1, 1.5);
        wait_for(|| controller.installed_size(1) == Some((151, 76)));
        assert!(frames.load(Ordering::SeqCst) >= 1);
    }

    #[test]
    fn fixed_sizing_is_independent_of_widget_resize() {
        let (controller, _) = test_controller();
        controller.set_plot(
            7,
            ruviz::prelude::Plot::new().scatter(&[0.0, 1.0], &[1.0, 2.0]),
            SlotOptions {
                sizing: SizingMode::Fixed {
                    width_px: 96,
                    height_px: 64,
                },
                ..SlotOptions::default()
            },
        );
        controller.resize(7, 600.0, 400.0, 2.0);
        wait_for(|| controller.installed_size(7) == Some((96, 64)));
    }

    #[test]
    fn keep_view_uses_replacement_bounds_when_old_view_is_untouched() {
        let (controller, _) = test_controller();
        controller.set_plot(
            27,
            ruviz::prelude::Plot::new()
                .line(&[0.0, 10.0], &[0.0, 10.0])
                .xlim(0.0, 10.0)
                .ylim(0.0, 10.0),
            SlotOptions::default(),
        );
        controller.set_plot_keep_view(
            27,
            ruviz::prelude::Plot::new()
                .line(&[100.0, 200.0], &[-5.0, 5.0])
                .xlim(100.0, 200.0)
                .ylim(-5.0, 5.0),
            SlotOptions::default(),
        );

        let session = two_d_session(controller.inner.plot_handle(27).unwrap());
        let view = session.view_bounds_snapshot();
        assert_eq!(view.visible_bounds, view.base_bounds);
        assert_eq!(view.base_bounds.min.x, 100.0);
        assert_eq!(view.base_bounds.max.x, 200.0);
    }

    #[test]
    fn keep_view_preserves_a_customized_old_view() {
        let (controller, _) = test_controller();
        controller.set_plot(
            28,
            ruviz::prelude::Plot::new()
                .line(&[0.0, 10.0], &[0.0, 10.0])
                .xlim(0.0, 10.0)
                .ylim(0.0, 10.0),
            SlotOptions::default(),
        );
        controller.resize(28, 640.0, 480.0, 1.0);
        wait_for(|| controller.installed_size(28) == Some((640, 480)));
        let old = two_d_session(controller.inner.plot_handle(28).unwrap());
        assert!(controller.wheel(28, -100.0, LogicalPoint::new(320.0, 240.0)));
        let customized = old.view_bounds_snapshot();
        assert_ne!(customized.visible_bounds, customized.base_bounds);

        controller.set_plot_keep_view(
            28,
            ruviz::prelude::Plot::new()
                .line(&[0.0, 20.0], &[0.0, 20.0])
                .xlim(0.0, 20.0)
                .ylim(0.0, 20.0),
            SlotOptions::default(),
        );

        wait_for(|| lock_slots(&controller.inner.slots)[&28].interaction_enabled());
        let replacement = two_d_session(controller.inner.plot_handle(28).unwrap());
        let view = replacement.view_bounds_snapshot();
        assert_eq!(view.visible_bounds, customized.visible_bounds);
        assert_ne!(view.visible_bounds, view.base_bounds);
    }

    #[test]
    fn poisoned_mutexes_recover_the_guarded_state() {
        let value = Arc::new(Mutex::new(42));
        let poison = Arc::clone(&value);
        let result = std::thread::spawn(move || {
            let _guard = poison.lock().expect("test lock should be available");
            panic!("poison the test mutex");
        })
        .join();
        assert!(result.is_err());
        assert_eq!(*lock_scalar_recover(&value), 42);
    }

    #[test]
    fn poisoned_slot_state_is_cleared_before_it_can_be_observed() {
        let (controller, _) = test_controller();
        controller.set_plot(
            20,
            ruviz::prelude::Plot::new().line(&[0.0, 1.0], &[1.0, 2.0]),
            SlotOptions::default(),
        );
        wait_for(|| controller.installed_size(20).is_some());

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut slots = controller
                .inner
                .slots
                .lock()
                .expect("test slot lock should be available");
            slots.get_mut(&20).expect("test slot should exist").drag = Some(ActiveDrag {
                button: PointerButton::Left,
                anchor: LogicalPoint::new(1.0, 1.0),
                last: LogicalPoint::new(2.0, 2.0),
                moved: true,
                forwarded: true,
            });
            panic!("poison the slot state after a partial mutation");
        }));
        assert!(result.is_err());

        assert_eq!(controller.installed_size(20), None);
        assert!(controller.inner.plot_handle(20).is_none());
    }

    #[test]
    fn incarnation_exhaustion_is_reported_without_a_panic() {
        let counter = AtomicU64::new(u64::MAX);
        assert_eq!(reserve_slot_incarnation(&counter), None);
        assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);
    }

    #[test]
    fn frame_sink_panics_are_reported_as_adapter_errors() {
        let errors = Arc::new(AtomicUsize::new(0));
        let reported = Arc::clone(&errors);
        let controller = RuvizController::with_dispatcher(
            |_, _| panic!("frame sink failed"),
            |task| {
                task();
                Ok(())
            },
        );
        controller.on_error(move |_| {
            reported.fetch_add(1, Ordering::SeqCst);
        });
        controller.set_plot(
            19,
            ruviz::prelude::Plot::new().line(&[0.0, 1.0], &[1.0, 2.0]),
            SlotOptions::default(),
        );
        wait_for(|| errors.load(Ordering::SeqCst) >= 1);
        assert_eq!(controller.installed_size(19), None);
        assert_eq!(controller.installed_generation(19), None);
    }

    #[test]
    fn reentrant_sink_cannot_commit_the_superseded_frame() {
        let controller_cell = Arc::new(Mutex::new(None::<RuvizController>));
        let callback_controller = Arc::clone(&controller_cell);
        let calls = Arc::new(AtomicUsize::new(0));
        let callback_calls = Arc::clone(&calls);
        let saw_early_commit = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let callback_saw_early_commit = Arc::clone(&saw_early_commit);
        let controller = RuvizController::with_dispatcher(
            move |slot, _| {
                if callback_calls.fetch_add(1, Ordering::SeqCst) == 0 {
                    let controller = callback_controller
                        .lock()
                        .expect("test controller lock should be available")
                        .clone()
                        .expect("test controller should be installed");
                    callback_saw_early_commit.store(
                        controller.installed_generation(slot).is_some(),
                        Ordering::SeqCst,
                    );
                    controller.resize(slot, 320.0, 240.0, 1.0);
                }
            },
            |task| {
                task();
                Ok(())
            },
        );
        *controller_cell
            .lock()
            .expect("test controller lock should be available") = Some(controller.clone());
        controller.set_plot(
            23,
            ruviz::prelude::Plot::new().line(&[0.0, 1.0], &[1.0, 2.0]),
            SlotOptions::default(),
        );

        wait_for(|| controller.installed_size(23) == Some((320, 240)));
        assert!(!saw_early_commit.load(Ordering::SeqCst));
        assert!(calls.load(Ordering::SeqCst) >= 2);
        *controller_cell
            .lock()
            .expect("test controller lock should be available") = None;
    }

    #[cfg(feature = "3d")]
    #[test]
    fn poisoned_3d_session_fails_closed_and_reports_an_error() {
        let (controller, _) = test_controller();
        let errors = Arc::new(AtomicUsize::new(0));
        let reported = Arc::clone(&errors);
        controller.on_error(move |_| {
            reported.fetch_add(1, Ordering::SeqCst);
        });
        controller
            .set_plot3d(
                21,
                ruviz::scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0]),
                SlotOptions::default(),
            )
            .unwrap();
        wait_for(|| controller.installed_size(21).is_some());
        let session = match controller.inner.plot_handle(21).unwrap() {
            PlotHandle::ThreeD(session) => session,
            PlotHandle::TwoD(_) => unreachable!(),
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _session = session
                .lock()
                .expect("test 3D session lock should be available");
            panic!("poison the 3D session");
        }));
        assert!(result.is_err());

        controller.request_redraw(21);
        wait_for(|| errors.load(Ordering::SeqCst) >= 1);
        assert!(
            !lock_slots(&controller.inner.slots)[&21].interaction_enabled(),
            "a poisoned 3D session must invalidate the installed frame"
        );
    }

    #[cfg(feature = "3d")]
    #[test]
    fn poisoned_3d_renderer_rejects_the_frame_and_preserves_last_good_generation() {
        let (controller, _) = test_controller();
        let errors = Arc::new(AtomicUsize::new(0));
        let reported = Arc::clone(&errors);
        controller.on_error(move |_| {
            reported.fetch_add(1, Ordering::SeqCst);
        });
        controller
            .set_plot3d(
                22,
                ruviz::scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0]),
                SlotOptions::default(),
            )
            .unwrap();
        wait_for(|| controller.installed_generation(22).is_some());
        let generation = controller.installed_generation(22);
        let renderer = {
            let slots = lock_slots(&controller.inner.slots);
            match &slots[&22].plot {
                PlotSlot::ThreeD { renderer, .. } => Arc::clone(renderer),
                PlotSlot::TwoD(_) => unreachable!(),
            }
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _renderer = renderer
                .lock()
                .expect("test 3D renderer lock should be available");
            panic!("poison the 3D renderer");
        }));
        assert!(result.is_err());

        controller.request_redraw(22);
        wait_for(|| errors.load(Ordering::SeqCst) >= 1);
        assert_eq!(controller.installed_generation(22), generation);
    }

    #[cfg(feature = "3d")]
    #[test]
    fn static_3d_slot_renders_but_rejects_interaction() {
        let (controller, _) = test_controller();
        controller
            .set_plot3d(
                26,
                ruviz::scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0]),
                SlotOptions {
                    interaction: InteractionMode::Static,
                    ..SlotOptions::default()
                },
            )
            .unwrap();
        controller.resize(26, 160.0, 120.0, 1.0);
        wait_for(|| controller.installed_size(26) == Some((160, 120)));
        let session = match controller.inner.plot_handle(26).unwrap() {
            PlotHandle::ThreeD(session) => session,
            PlotHandle::TwoD(_) => unreachable!(),
        };
        let initial = session
            .lock()
            .expect("test 3D session lock should be available")
            .camera();

        assert!(!controller.wheel(26, -20.0, LogicalPoint::new(80.0, 60.0)));
        controller.pointer_input(
            26,
            PointerInput {
                kind: PointerKind::Down,
                button: PointerButton::Left,
                position: LogicalPoint::new(80.0, 60.0),
            },
        );
        controller.pointer_input(
            26,
            PointerInput {
                kind: PointerKind::Move,
                button: PointerButton::Left,
                position: LogicalPoint::new(110.0, 70.0),
            },
        );
        assert_eq!(
            session
                .lock()
                .expect("test 3D session lock should be available")
                .camera(),
            initial
        );
    }

    #[cfg(all(feature = "3d-gpu", not(target_arch = "wasm32")))]
    #[test]
    fn gpu_readback_backend_is_selected_and_installs_the_returned_image() {
        let images = Arc::new(Mutex::new(Vec::<(u32, u32)>::new()));
        let installed = Arc::clone(&images);
        let controller = RuvizController::with_dispatcher(
            move |_, image| {
                let size = image.size();
                lock_scalar_recover(&installed).push((size.width, size.height));
            },
            |task| {
                task();
                Ok(())
            },
        );
        controller
            .set_plot3d(
                32,
                ruviz::scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0]),
                SlotOptions {
                    interaction: InteractionMode::Static,
                    sizing: SizingMode::Fixed {
                        width_px: 64,
                        height_px: 48,
                    },
                    prefer_gpu: true,
                    ..SlotOptions::default()
                },
            )
            .unwrap();
        let (backend, renderer) = {
            let slots = lock_slots(&controller.inner.slots);
            match &slots[&32].plot {
                PlotSlot::ThreeD {
                    renderer, backend, ..
                } => (*backend, Arc::clone(renderer)),
                PlotSlot::TwoD(_) => unreachable!(),
            }
        };
        assert_eq!(backend, ruviz::core::BackgroundRenderBackend3D::GpuReadback);
        assert_eq!(
            lock_3d_renderer(&renderer)
                .expect("test renderer should not be poisoned")
                .backend(),
            ruviz::core::BackgroundRenderBackend3D::GpuReadback
        );

        wait_for(|| controller.installed_size(32) == Some((64, 48)));
        assert!(
            lock_scalar_recover(&images).contains(&(64, 48)),
            "the GPU-readback image must be converted and installed in Slint"
        );
    }

    #[test]
    fn wrapper_scale_and_fit_mapping_matrix_is_exact() {
        let layout = SlotLayout {
            outer: LogicalRect::new(10.0, 20.0, 400.0, 400.0),
            scale_factor: 1.0,
        };
        for (scale, expected) in [
            (1.0, (101, 51)),
            (1.25, (126, 63)),
            (1.5, (151, 76)),
            (2.0, (201, 101)),
        ] {
            assert_eq!(
                target_size(
                    SlotLayout {
                        outer: LogicalRect::new(0.0, 0.0, 100.25, 50.1),
                        scale_factor: scale,
                    },
                    SizingMode::Fill,
                ),
                expected
            );
        }

        let state_for = |fit| SlotState {
            plot: PlotSlot::TwoD(
                ruviz::prelude::Plot::new()
                    .line(&[0.0, 1.0], &[0.0, 1.0])
                    .into_plot_session(),
            ),
            incarnation: 1,
            options: SlotOptions {
                sizing: SizingMode::Fixed {
                    width_px: 800,
                    height_px: 400,
                },
                fit,
                ..SlotOptions::default()
            },
            layout,
            scheduler: LatestRequestScheduler::default(),
            drag: None,
            context_press: None,
            _subscription: None,
            last_frame: None,
            presented: None,
        };

        let contain = state_for(ImageFit::Contain);
        assert_eq!(
            contain.map_point(LogicalPoint::new(210.0, 220.0)),
            Some(ViewportPoint::new(400.0, 200.0))
        );
        assert_eq!(
            contain.map_point(LogicalPoint::new(10.0, 120.0)),
            Some(ViewportPoint::new(0.0, 0.0))
        );
        assert_eq!(
            contain.map_point(LogicalPoint::new(410.0, 320.0)),
            Some(ViewportPoint::new(800.0, 400.0))
        );
        assert_eq!(contain.map_point(LogicalPoint::new(10.0, 20.0)), None);

        let cover = state_for(ImageFit::Cover);
        assert_eq!(
            cover.map_point(LogicalPoint::new(210.0, 220.0)),
            Some(ViewportPoint::new(400.0, 200.0))
        );
        assert_eq!(
            cover.map_point(LogicalPoint::new(10.0, 20.0)),
            Some(ViewportPoint::new(200.0, 0.0))
        );
        assert_eq!(
            cover.map_point(LogicalPoint::new(410.0, 420.0)),
            Some(ViewportPoint::new(600.0, 400.0))
        );
        assert_eq!(cover.map_point(LogicalPoint::new(-1.0, 220.0)), None);

        let fill = state_for(ImageFit::Fill);
        assert_eq!(
            fill.map_point(LogicalPoint::new(10.0, 20.0)),
            Some(ViewportPoint::new(0.0, 0.0))
        );
        assert_eq!(
            fill.map_point(LogicalPoint::new(210.0, 220.0)),
            Some(ViewportPoint::new(400.0, 200.0))
        );
        assert_eq!(
            fill.map_point(LogicalPoint::new(410.0, 420.0)),
            Some(ViewportPoint::new(800.0, 400.0))
        );
    }

    #[test]
    fn replacement_and_multiple_slots_install_independently() {
        let (controller, _) = test_controller();
        for slot in [3, 8] {
            controller.set_plot(
                slot,
                ruviz::prelude::Plot::new().line(&[0.0, 1.0], &[1.0, 2.0]),
                SlotOptions::default(),
            );
            controller.resize(slot, 80.0 + f64::from(slot), 60.0, 1.0);
        }
        controller.set_plot_keep_view(
            3,
            ruviz::prelude::Plot::new().bar(&["a", "b"], &[2.0, 1.0]),
            SlotOptions::default(),
        );
        wait_for(|| {
            controller.installed_size(3) == Some((83, 60))
                && controller.installed_size(8) == Some((88, 60))
        });
    }

    #[test]
    fn reactive_observable_change_schedules_and_installs_a_new_frame() {
        let (controller, frames) = test_controller();
        let x = ruviz::data::Observable::new(vec![0.0, 1.0, 2.0]);
        let y = ruviz::data::Observable::new(vec![0.0, 1.0, 4.0]);
        controller.set_plot(
            31,
            ruviz::prelude::Plot::new().line_source(x.clone(), y),
            SlotOptions::default(),
        );
        wait_for(|| controller.installed_generation(31).is_some());
        let initial_generation = controller
            .installed_generation(31)
            .expect("initial reactive frame should be installed");
        let initial_frames = frames.load(Ordering::SeqCst);

        x.set(vec![0.0, 1.0, 3.0]);

        wait_for(|| {
            controller
                .installed_generation(31)
                .is_some_and(|generation| generation > initial_generation)
                && frames.load(Ordering::SeqCst) > initial_frames
        });
    }

    #[test]
    fn cancellation_clears_release_outside_drag() {
        let (controller, _) = test_controller();
        controller.set_plot(
            4,
            ruviz::prelude::Plot::new().line(&[0.0, 1.0], &[1.0, 2.0]),
            SlotOptions::default(),
        );
        wait_for(|| controller.installed_size(4).is_some());
        controller.pointer_input(
            4,
            PointerInput {
                kind: PointerKind::Down,
                button: PointerButton::Left,
                position: LogicalPoint::new(0.5, 0.5),
            },
        );
        controller.cancel_drag(4);
        let slots = controller
            .inner
            .slots
            .lock()
            .expect("Slint slot lock poisoned");
        assert!(slots[&4].drag.is_none());
    }

    #[test]
    fn reset_cancels_active_drag_before_changing_the_view() {
        let (controller, _) = test_controller();
        controller.set_plot(
            17,
            ruviz::prelude::Plot::new().line(&[0.0, 1.0], &[1.0, 2.0]),
            SlotOptions::default(),
        );
        controller.resize(17, 200.0, 120.0, 1.0);
        wait_for(|| controller.installed_size(17) == Some((200, 120)));
        controller.pointer_input(
            17,
            PointerInput {
                kind: PointerKind::Down,
                button: PointerButton::Right,
                position: LogicalPoint::new(50.0, 50.0),
            },
        );
        controller.pointer_input(
            17,
            PointerInput {
                kind: PointerKind::Move,
                button: PointerButton::Right,
                position: LogicalPoint::new(80.0, 70.0),
            },
        );
        assert!(
            controller
                .inner
                .slots
                .lock()
                .expect("Slint slot lock poisoned")[&17]
                .drag
                .is_some()
        );

        controller.reset_view(17);

        assert!(
            controller
                .inner
                .slots
                .lock()
                .expect("Slint slot lock poisoned")[&17]
                .drag
                .is_none()
        );
    }

    #[test]
    fn double_click_cancels_the_retained_pointer_sequence() {
        let (controller, _) = test_controller();
        controller.set_plot(
            18,
            ruviz::prelude::Plot::new().scatter(&[0.0, 1.0], &[1.0, 2.0]),
            SlotOptions::default(),
        );
        controller.resize(18, 200.0, 120.0, 1.0);
        wait_for(|| controller.installed_size(18) == Some((200, 120)));
        controller.pointer_input(
            18,
            PointerInput {
                kind: PointerKind::Down,
                button: PointerButton::Left,
                position: LogicalPoint::new(100.0, 60.0),
            },
        );

        controller.double_click(18, LogicalPoint::new(100.0, 60.0));

        assert!(
            controller
                .inner
                .slots
                .lock()
                .expect("Slint slot lock poisoned")[&18]
                .drag
                .is_none()
        );
        controller.pointer_input(
            18,
            PointerInput {
                kind: PointerKind::Up,
                button: PointerButton::Left,
                position: LogicalPoint::new(100.0, 60.0),
            },
        );
        assert!(
            controller
                .inner
                .slots
                .lock()
                .expect("Slint slot lock poisoned")[&18]
                .drag
                .is_none()
        );
    }

    #[test]
    fn replacement_rejects_an_in_flight_old_incarnation() {
        let (controller, _) = test_controller();
        let entered = Arc::new(std::sync::Barrier::new(2));
        let release = Arc::new(std::sync::Barrier::new(2));
        *controller
            .inner
            .render_barrier
            .lock()
            .expect("Slint render barrier lock poisoned") =
            Some((Arc::clone(&entered), Arc::clone(&release)));

        controller.set_plot(
            11,
            ruviz::prelude::Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]),
            SlotOptions::default(),
        );
        entered.wait();
        *controller
            .inner
            .render_barrier
            .lock()
            .expect("Slint render barrier lock poisoned") = None;
        let old_incarnation = controller
            .inner
            .slots
            .lock()
            .expect("Slint slot lock poisoned")[&11]
            .incarnation;
        controller.set_plot(
            11,
            ruviz::prelude::Plot::new().bar(&["new"], &[7.0]),
            SlotOptions::default(),
        );
        let new_incarnation = controller
            .inner
            .slots
            .lock()
            .expect("Slint slot lock poisoned")[&11]
            .incarnation;
        assert_ne!(old_incarnation, new_incarnation);
        assert_eq!(controller.installed_generation(11), None);
        release.wait();
        wait_for(|| controller.installed_generation(11).is_some());
        let slots = controller
            .inner
            .slots
            .lock()
            .expect("Slint slot lock poisoned");
        assert_eq!(slots[&11].incarnation, new_incarnation);
        assert!(slots[&11].last_frame.as_ref().unwrap().is_current());
    }

    #[test]
    fn delayed_ui_install_does_not_present_a_replaced_incarnation() {
        let frames = Arc::new(AtomicUsize::new(0));
        let installed = Arc::clone(&frames);
        let tasks = Arc::new(Mutex::new(Vec::<UiTask>::new()));
        let dispatched = Arc::clone(&tasks);
        let controller = RuvizController::with_dispatcher(
            move |_, _| {
                installed.fetch_add(1, Ordering::SeqCst);
            },
            move |task| {
                dispatched
                    .lock()
                    .expect("test task queue lock should be available")
                    .push(task);
                Ok(())
            },
        );

        controller.set_plot(
            24,
            ruviz::prelude::Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]),
            SlotOptions::default(),
        );
        wait_for(|| {
            !tasks
                .lock()
                .expect("test task queue lock should be available")
                .is_empty()
        });
        let stale_install = tasks
            .lock()
            .expect("test task queue lock should be available")
            .remove(0);

        controller.set_plot(
            24,
            ruviz::prelude::Plot::new().bar(&["new"], &[7.0]),
            SlotOptions::default(),
        );
        stale_install();

        assert_eq!(frames.load(Ordering::SeqCst), 0);
        assert_eq!(controller.installed_generation(24), None);
    }

    #[test]
    fn remove_and_readd_is_safe_from_slot_aba_completion() {
        let (controller, _) = test_controller();
        let entered = Arc::new(std::sync::Barrier::new(2));
        let release = Arc::new(std::sync::Barrier::new(2));
        *controller
            .inner
            .render_barrier
            .lock()
            .expect("Slint render barrier lock poisoned") =
            Some((Arc::clone(&entered), Arc::clone(&release)));
        controller.set_plot(
            12,
            ruviz::prelude::Plot::new().line(&[0.0, 1.0], &[1.0, 2.0]),
            SlotOptions::default(),
        );
        entered.wait();
        let old_incarnation = controller
            .inner
            .slots
            .lock()
            .expect("Slint slot lock poisoned")[&12]
            .incarnation;
        *controller
            .inner
            .render_barrier
            .lock()
            .expect("Slint render barrier lock poisoned") = None;
        assert!(controller.remove_plot(12));
        controller.set_plot(
            12,
            ruviz::prelude::Plot::new().scatter(&[0.0, 1.0], &[4.0, 3.0]),
            SlotOptions::default(),
        );
        let new_incarnation = controller
            .inner
            .slots
            .lock()
            .expect("Slint slot lock poisoned")[&12]
            .incarnation;
        assert_ne!(old_incarnation, new_incarnation);
        assert_eq!(controller.installed_generation(12), None);
        assert!(
            controller.inner.render_lane_busy(12),
            "the detached old render must retain the physical render lane"
        );
        release.wait();
        wait_for(|| {
            controller
                .inner
                .slots
                .lock()
                .expect("Slint slot lock poisoned")[&12]
                .last_frame
                .as_ref()
                .is_some_and(InstalledFrame::is_current)
        });
        assert!(controller.installed_generation(12).is_some());
    }

    #[test]
    fn small_drag_does_not_pan_or_cross_click_threshold() {
        let (controller, _) = test_controller();
        controller.set_plot(
            13,
            ruviz::prelude::Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]),
            SlotOptions::default(),
        );
        controller.resize(13, 200.0, 120.0, 1.0);
        wait_for(|| controller.installed_size(13) == Some((200, 120)));
        let session = two_d_session(controller.inner.plot_handle(13).unwrap());
        let before = session.view_bounds_snapshot().visible_bounds;
        controller.pointer_input(
            13,
            PointerInput {
                kind: PointerKind::Down,
                button: PointerButton::Left,
                position: LogicalPoint::new(100.0, 60.0),
            },
        );
        controller.pointer_input(
            13,
            PointerInput {
                kind: PointerKind::Move,
                button: PointerButton::Left,
                position: LogicalPoint::new(101.0, 61.0),
            },
        );
        assert_eq!(session.view_bounds_snapshot().visible_bounds, before);
        assert!(
            !controller
                .inner
                .slots
                .lock()
                .expect("Slint slot lock poisoned")[&13]
                .drag
                .unwrap()
                .moved
        );
    }

    #[test]
    fn secondary_click_requests_menu_without_starting_a_brush() {
        let (controller, _) = test_controller();
        controller.set_plot(
            34,
            ruviz::prelude::Plot::new().line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0]),
            SlotOptions::default(),
        );
        controller.resize(34, 200.0, 120.0, 1.0);
        wait_for(|| controller.installed_size(34) == Some((200, 120)));
        let session = two_d_session(controller.inner.plot_handle(34).unwrap());
        let before = session.view_bounds_snapshot().visible_bounds;

        assert!(!controller.pointer_input(
            34,
            PointerInput {
                kind: PointerKind::Down,
                button: PointerButton::Right,
                position: LogicalPoint::new(80.0, 50.0),
            },
        ));
        assert!(controller.pointer_input(
            34,
            PointerInput {
                kind: PointerKind::Up,
                button: PointerButton::Right,
                position: LogicalPoint::new(80.0, 50.0),
            },
        ));

        assert_eq!(session.view_bounds_snapshot().visible_bounds, before);
        assert!(lock_slots(&controller.inner.slots)[&34].drag.is_none());
    }

    #[test]
    fn secondary_release_recomputes_menu_threshold_without_a_move_event() {
        let (controller, _) = test_controller();
        controller.set_plot(
            39,
            ruviz::prelude::Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]),
            SlotOptions::default(),
        );
        controller.resize(39, 200.0, 120.0, 1.0);
        wait_for(|| controller.installed_size(39) == Some((200, 120)));

        assert!(!controller.pointer_input(
            39,
            PointerInput {
                kind: PointerKind::Down,
                button: PointerButton::Right,
                position: LogicalPoint::new(40.0, 30.0),
            },
        ));
        assert!(
            !controller.pointer_input(
                39,
                PointerInput {
                    kind: PointerKind::Up,
                    button: PointerButton::Right,
                    position: LogicalPoint::new(44.0, 30.0),
                },
            ),
            "a release beyond the threshold must not open a menu when no move was delivered"
        );
        let state = &lock_slots(&controller.inner.slots)[&39];
        assert!(state.drag.is_none());
        assert!(state.context_press.is_none());
    }

    #[test]
    fn context_click_is_available_for_static_slots_without_an_installed_frame() {
        let (controller, _) = test_controller();
        controller.set_plot(
            40,
            ruviz::prelude::Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]),
            SlotOptions {
                interaction: InteractionMode::Static,
                ..SlotOptions::default()
            },
        );
        wait_for(|| controller.installed_size(40).is_some());
        lock_slots(&controller.inner.slots)
            .get_mut(&40)
            .expect("slot should exist")
            .last_frame = None;

        assert!(!controller.pointer_input(
            40,
            PointerInput {
                kind: PointerKind::Down,
                button: PointerButton::Right,
                position: LogicalPoint::new(80.0, 50.0),
            },
        ));
        assert!(controller.pointer_input(
            40,
            PointerInput {
                kind: PointerKind::Up,
                button: PointerButton::Right,
                position: LogicalPoint::new(80.0, 50.0),
            },
        ));
    }

    #[test]
    fn secondary_drag_still_brushes_after_crossing_the_menu_threshold() {
        let (controller, _) = test_controller();
        let reports = Arc::new(Mutex::new(Vec::<PointerReport>::new()));
        let reported = Arc::clone(&reports);
        controller.on_pointer(move |report| {
            lock_scalar_recover(&reported).push(report);
        });
        controller.set_plot(
            35,
            ruviz::prelude::Plot::new().line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0]),
            SlotOptions::default(),
        );
        controller.resize(35, 200.0, 120.0, 1.0);
        wait_for(|| controller.installed_size(35) == Some((200, 120)));
        assert!(!controller.pointer_input(
            35,
            PointerInput {
                kind: PointerKind::Down,
                button: PointerButton::Right,
                position: LogicalPoint::new(40.0, 30.0),
            },
        ));
        assert!(!controller.pointer_input(
            35,
            PointerInput {
                kind: PointerKind::Move,
                button: PointerButton::None,
                position: LogicalPoint::new(41.0, 31.0),
            },
        ));
        assert!(
            lock_scalar_recover(&reports).is_empty(),
            "sub-threshold movement must remain a possible context click"
        );
        assert!(!controller.pointer_input(
            35,
            PointerInput {
                kind: PointerKind::Move,
                button: PointerButton::None,
                position: LogicalPoint::new(160.0, 90.0),
            },
        ));
        assert_eq!(
            lock_scalar_recover(&reports)
                .iter()
                .map(|report| (report.kind, report.button))
                .collect::<Vec<_>>(),
            vec![
                (PointerKind::Down, PointerButton::Right),
                (PointerKind::Move, PointerButton::None),
            ]
        );
        assert!(!controller.pointer_input(
            35,
            PointerInput {
                kind: PointerKind::Up,
                button: PointerButton::Right,
                position: LogicalPoint::new(160.0, 90.0),
            },
        ));

        assert_eq!(
            lock_scalar_recover(&reports)
                .last()
                .map(|report| (report.kind, report.button)),
            Some((PointerKind::Up, PointerButton::Right))
        );
        assert!(lock_slots(&controller.inner.slots)[&35].drag.is_none());
    }

    #[test]
    fn context_toggle_can_reenable_a_static_slot() {
        let (controller, _) = test_controller();
        controller.set_plot(
            36,
            ruviz::prelude::Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]),
            SlotOptions {
                interaction: InteractionMode::Static,
                ..SlotOptions::default()
            },
        );
        wait_for(|| controller.installed_size(36).is_some());

        controller.context_action(36, PlotContextMenuAction::ToggleInteraction);

        assert_eq!(
            lock_slots(&controller.inner.slots)[&36].options.interaction,
            InteractionMode::Interactive
        );
    }

    #[test]
    fn installed_frame_can_be_exported_as_png() {
        let (controller, _) = test_controller();
        controller.set_plot(
            37,
            ruviz::prelude::Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]),
            SlotOptions::default(),
        );
        controller.resize(37, 73.0, 41.0, 1.0);
        wait_for(|| controller.installed_size(37) == Some((73, 41)));
        let path = std::env::temp_dir().join(format!(
            "ruviz-slint-context-menu-{}-{}.png",
            std::process::id(),
            controller.installed_generation(37).unwrap_or_default()
        ));

        let (base, overlay) = controller
            .installed_layers(37)
            .expect("installed layers should be retained");
        let image = compose_layers(&base, overlay.as_deref());
        write_image_png(&image, &path).expect("installed image should export");
        let bytes = std::fs::read(&path).expect("exported PNG should be readable");
        let _ = std::fs::remove_file(&path);

        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(u32::from_be_bytes(bytes[16..20].try_into().unwrap()), 73);
        assert_eq!(u32::from_be_bytes(bytes[20..24].try_into().unwrap()), 41);
    }

    #[test]
    fn letterbox_press_is_rejected_before_drag_state_is_created() {
        let (controller, _) = test_controller();
        let reports = Arc::new(AtomicUsize::new(0));
        let reported = Arc::clone(&reports);
        controller.on_pointer(move |_| {
            reported.fetch_add(1, Ordering::SeqCst);
        });
        controller.set_plot(
            29,
            ruviz::prelude::Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]),
            SlotOptions {
                sizing: SizingMode::Fixed {
                    width_px: 800,
                    height_px: 400,
                },
                fit: ImageFit::Contain,
                ..SlotOptions::default()
            },
        );
        controller.resize(29, 400.0, 400.0, 1.0);
        wait_for(|| controller.installed_size(29) == Some((800, 400)));
        let session = two_d_session(controller.inner.plot_handle(29).unwrap());
        let before = session.view_bounds_snapshot().visible_bounds;

        controller.pointer_input(
            29,
            PointerInput {
                kind: PointerKind::Down,
                button: PointerButton::Left,
                position: LogicalPoint::new(200.0, 20.0),
            },
        );

        assert_eq!(reports.load(Ordering::SeqCst), 0);
        assert!(lock_slots(&controller.inner.slots)[&29].drag.is_none());
        controller.pointer_input(
            29,
            PointerInput {
                kind: PointerKind::Move,
                button: PointerButton::Left,
                position: LogicalPoint::new(240.0, 200.0),
            },
        );
        assert_eq!(session.view_bounds_snapshot().visible_bounds, before);
        assert!(lock_slots(&controller.inner.slots)[&29].drag.is_none());
    }

    #[test]
    fn stale_presented_frame_disables_followup_input() {
        let (controller, _) = test_controller();
        controller.set_plot(
            14,
            ruviz::prelude::Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]),
            SlotOptions::default(),
        );
        controller.resize(14, 200.0, 120.0, 1.0);
        wait_for(|| controller.installed_size(14) == Some((200, 120)));
        let entered = Arc::new(std::sync::Barrier::new(2));
        let release = Arc::new(std::sync::Barrier::new(2));
        *controller
            .inner
            .render_barrier
            .lock()
            .expect("Slint render barrier lock poisoned") =
            Some((Arc::clone(&entered), Arc::clone(&release)));
        controller.wheel(14, -20.0, LogicalPoint::new(100.0, 60.0));
        entered.wait();
        let session = two_d_session(controller.inner.plot_handle(14).unwrap());
        let after_first = session.view_bounds_snapshot().visible_bounds;
        controller.wheel(14, -20.0, LogicalPoint::new(100.0, 60.0));
        assert_eq!(session.view_bounds_snapshot().visible_bounds, after_first);
        *controller
            .inner
            .render_barrier
            .lock()
            .expect("Slint render barrier lock poisoned") = None;
        release.wait();
    }

    #[test]
    fn stale_2d_frame_allows_only_outside_hover_clear_move() {
        let (controller, _) = test_controller();
        let reports = Arc::new(Mutex::new(Vec::<PointerReport>::new()));
        let reported = Arc::clone(&reports);
        controller.on_pointer(move |report| {
            lock_scalar_recover(&reported).push(report);
        });
        controller.set_plot(
            30,
            ruviz::prelude::Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]),
            SlotOptions::default(),
        );
        controller.resize(30, 200.0, 120.0, 1.0);
        wait_for(|| controller.installed_size(30) == Some((200, 120)));
        let entered = Arc::new(std::sync::Barrier::new(2));
        let release = Arc::new(std::sync::Barrier::new(2));
        *controller
            .inner
            .render_barrier
            .lock()
            .expect("Slint render barrier lock poisoned") =
            Some((Arc::clone(&entered), Arc::clone(&release)));
        assert!(controller.wheel(30, -20.0, LogicalPoint::new(100.0, 60.0)));
        entered.wait();
        assert!(!lock_slots(&controller.inner.slots)[&30].interaction_enabled());

        controller.pointer_input(
            30,
            PointerInput {
                kind: PointerKind::Move,
                button: PointerButton::None,
                position: LogicalPoint::new(-10.0, -10.0),
            },
        );
        {
            let reports = lock_scalar_recover(&reports);
            assert_eq!(reports.len(), 1);
            assert_eq!(reports[0].kind, PointerKind::Move);
            assert_eq!(reports[0].physical_position, None);
        }

        controller.pointer_input(
            30,
            PointerInput {
                kind: PointerKind::Move,
                button: PointerButton::None,
                position: LogicalPoint::new(100.0, 60.0),
            },
        );
        assert_eq!(lock_scalar_recover(&reports).len(), 1);
        *controller
            .inner
            .render_barrier
            .lock()
            .expect("Slint render barrier lock poisoned") = None;
        release.wait();
    }

    #[test]
    fn switching_to_static_clears_hover_and_installs_cleanup_frame() {
        let (controller, _) = test_controller();
        controller.set_plot(
            33,
            ruviz::prelude::Plot::new().scatter(&[0.0, 1.0, 2.0], &[0.0, 1.0, 0.0]),
            SlotOptions::default(),
        );
        controller.resize(33, 640.0, 480.0, 1.0);
        wait_for(|| controller.installed_generation(33).is_some());
        let session = two_d_session(controller.inner.plot_handle(33).unwrap());
        let mut hit_position = None;
        'search: for y in (0..480).step_by(4) {
            for x in (0..640).step_by(4) {
                let position = ViewportPoint::new(f64::from(x), f64::from(y));
                if !matches!(session.hit_test(position), HitResult::None) {
                    hit_position = Some(position);
                    break 'search;
                }
            }
        }
        let hit_position = hit_position.expect("rendered scatter must expose a hit target");
        let before_hover = controller
            .installed_generation(33)
            .expect("initial frame should be installed");
        controller.pointer_input(
            33,
            PointerInput {
                kind: PointerKind::Move,
                button: PointerButton::None,
                position: LogicalPoint::new(hit_position.x, hit_position.y),
            },
        );
        wait_for(|| {
            controller
                .installed_generation(33)
                .is_some_and(|generation| generation > before_hover)
        });
        let hover_generation = controller
            .installed_generation(33)
            .expect("hover frame should be installed");

        assert!(controller.set_options(
            33,
            SlotOptions {
                interaction: InteractionMode::Static,
                ..SlotOptions::default()
            }
        ));
        wait_for(|| {
            controller
                .installed_generation(33)
                .is_some_and(|generation| generation > hover_generation)
        });
        let notifications = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&notifications);
        let _subscription = session.subscribe_changes(move |_| {
            observed.fetch_add(1, Ordering::SeqCst);
        });
        session.apply_input(PlotInputEvent::ClearHover);
        assert_eq!(
            notifications.load(Ordering::SeqCst),
            0,
            "static-mode transition must already clear hover and its tooltip"
        );
    }

    #[test]
    fn active_drag_continues_while_new_frame_is_delayed() {
        let (controller, _) = test_controller();
        controller.set_plot(
            16,
            ruviz::prelude::Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]),
            SlotOptions::default(),
        );
        controller.resize(16, 200.0, 120.0, 1.0);
        wait_for(|| controller.installed_size(16) == Some((200, 120)));
        let entered = Arc::new(std::sync::Barrier::new(2));
        let release = Arc::new(std::sync::Barrier::new(2));
        *controller
            .inner
            .render_barrier
            .lock()
            .expect("Slint render barrier lock poisoned") =
            Some((Arc::clone(&entered), Arc::clone(&release)));
        controller.pointer_input(
            16,
            PointerInput {
                kind: PointerKind::Down,
                button: PointerButton::Left,
                position: LogicalPoint::new(100.0, 60.0),
            },
        );
        controller.pointer_input(
            16,
            PointerInput {
                kind: PointerKind::Move,
                button: PointerButton::Left,
                position: LogicalPoint::new(110.0, 60.0),
            },
        );
        entered.wait();
        let session = two_d_session(controller.inner.plot_handle(16).unwrap());
        let after_first = session.view_bounds_snapshot().visible_bounds;
        controller.pointer_input(
            16,
            PointerInput {
                kind: PointerKind::Move,
                button: PointerButton::Left,
                position: LogicalPoint::new(120.0, 60.0),
            },
        );
        assert_ne!(session.view_bounds_snapshot().visible_bounds, after_first);
        controller.pointer_input(
            16,
            PointerInput {
                kind: PointerKind::Cancel,
                button: PointerButton::None,
                position: LogicalPoint::new(120.0, 60.0),
            },
        );
        assert!(
            controller
                .inner
                .slots
                .lock()
                .expect("Slint slot lock poisoned")[&16]
                .drag
                .is_none()
        );
        *controller
            .inner
            .render_barrier
            .lock()
            .expect("Slint render barrier lock poisoned") = None;
        release.wait();
    }

    #[cfg(feature = "3d")]
    #[test]
    fn three_d_slot_renders_interacts_and_keeps_camera() {
        let (controller, _) = test_controller();
        controller
            .set_plot3d(
                15,
                ruviz::scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0]),
                SlotOptions::default(),
            )
            .unwrap();
        controller.resize(15, 160.0, 120.0, 1.0);
        wait_for(|| controller.installed_size(15) == Some((160, 120)));
        let session = match controller.inner.plot_handle(15).unwrap() {
            PlotHandle::ThreeD(session) => session,
            PlotHandle::TwoD(_) => unreachable!(),
        };
        let initial = session
            .lock()
            .expect("Slint 3D session lock poisoned")
            .camera();
        controller.pointer_input(
            15,
            PointerInput {
                kind: PointerKind::Down,
                button: PointerButton::Left,
                position: LogicalPoint::new(80.0, 60.0),
            },
        );
        controller.pointer_input(
            15,
            PointerInput {
                kind: PointerKind::Move,
                button: PointerButton::Left,
                position: LogicalPoint::new(100.0, 70.0),
            },
        );
        let changed = session
            .lock()
            .expect("Slint 3D session lock poisoned")
            .camera();
        assert_ne!(changed, initial);
        wait_for(|| {
            controller
                .inner
                .slots
                .lock()
                .expect("Slint slot lock poisoned")[&15]
                .interaction_enabled()
        });
        controller
            .set_plot3d_keep_view(
                15,
                ruviz::scatter3d(&[2.0, 3.0], &[1.0, 2.0], &[4.0, 5.0]),
                SlotOptions::default(),
            )
            .unwrap();
        let kept = match controller.inner.plot_handle(15).unwrap() {
            PlotHandle::ThreeD(session) => session
                .lock()
                .expect("Slint 3D session lock poisoned")
                .camera(),
            PlotHandle::TwoD(_) => unreachable!(),
        };
        assert_eq!(kept, changed);
    }

    #[cfg(feature = "3d")]
    #[test]
    fn three_d_context_click_does_not_pan_and_named_view_is_applied() {
        let (controller, _) = test_controller();
        controller
            .set_plot3d(
                38,
                ruviz::scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0]),
                SlotOptions::default(),
            )
            .unwrap();
        controller.resize(38, 160.0, 120.0, 1.0);
        wait_for(|| controller.installed_size(38) == Some((160, 120)));
        let session = match controller.inner.plot_handle(38).unwrap() {
            PlotHandle::ThreeD(session) => session,
            PlotHandle::TwoD(_) => unreachable!(),
        };
        let initial = lock_3d_session(&session).unwrap().camera();
        let camera_events = Arc::new(AtomicUsize::new(0));
        let observed_camera_events = Arc::clone(&camera_events);
        controller.on_camera_change(move |_, _| {
            observed_camera_events.fetch_add(1, Ordering::SeqCst);
        });

        assert!(!controller.pointer_input(
            38,
            PointerInput {
                kind: PointerKind::Down,
                button: PointerButton::Right,
                position: LogicalPoint::new(80.0, 60.0),
            },
        ));
        assert!(controller.pointer_input(
            38,
            PointerInput {
                kind: PointerKind::Up,
                button: PointerButton::Right,
                position: LogicalPoint::new(80.0, 60.0),
            },
        ));
        assert_eq!(lock_3d_session(&session).unwrap().camera(), initial);

        controller.context_action(
            38,
            PlotContextMenuAction::CameraView(ruviz::core::CameraView3D::Top),
        );
        let top = lock_3d_session(&session).unwrap().camera();
        assert_eq!(top.get_elevation_deg(), 89.9);
        assert_eq!(top.get_roll_deg(), 0.0);
        assert_eq!(camera_events.load(Ordering::SeqCst), 1);

        controller.context_action(
            38,
            PlotContextMenuAction::CameraView(ruviz::core::CameraView3D::Top),
        );
        assert_eq!(
            camera_events.load(Ordering::SeqCst),
            1,
            "selecting the active named view must not emit a camera event"
        );

        controller.context_action(38, PlotContextMenuAction::ResetView);
        assert_eq!(lock_3d_session(&session).unwrap().camera(), initial);
        assert_eq!(camera_events.load(Ordering::SeqCst), 2);
    }

    #[cfg(feature = "3d")]
    #[test]
    fn failed_three_d_keep_view_replacement_retains_the_old_slot() {
        let (controller, _) = test_controller();
        controller
            .set_plot3d(
                25,
                ruviz::scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0]),
                SlotOptions::default(),
            )
            .unwrap();
        wait_for(|| controller.installed_size(25).is_some());
        let old_session = match controller.inner.plot_handle(25).unwrap() {
            PlotHandle::ThreeD(session) => session,
            PlotHandle::TwoD(_) => unreachable!(),
        };
        let poison = Arc::clone(&old_session);
        assert!(
            std::thread::spawn(move || {
                let _guard = poison
                    .lock()
                    .expect("test 3D session lock should be available");
                panic!("poison the old 3D session");
            })
            .join()
            .is_err()
        );

        let result = controller.set_plot3d_keep_view(
            25,
            ruviz::scatter3d(&[2.0, 3.0], &[1.0, 2.0], &[4.0, 5.0]),
            SlotOptions::default(),
        );

        assert!(result.is_err());
        let retained = match controller.inner.plot_handle(25).unwrap() {
            PlotHandle::ThreeD(session) => session,
            PlotHandle::TwoD(_) => unreachable!(),
        };
        assert!(Arc::ptr_eq(&retained, &old_session));
    }

    #[test]
    fn images_use_matching_slint_alpha_constructors() {
        let straight = ruviz::core::Image::from_straight_rgba(1, 1, vec![20, 40, 60, 128]);
        let premultiplied =
            ruviz::core::Image::from_premultiplied_rgba(1, 1, vec![10, 20, 30, 128]);
        for image in [straight, premultiplied] {
            let slint_image = LayerBuffer::new(&image).into_slint_image();
            assert_eq!(slint_image.size().width, 1);
        }
    }

    #[test]
    fn packaged_component_stacks_the_overlay_layer() {
        let component = include_str!("../ui/ruviz.slint");
        assert!(
            component.contains("overlay: image,"),
            "the slot state must expose the overlay as its own layer"
        );
        assert_eq!(
            component.matches("image-fit: root.fitting;").count(),
            2,
            "base and overlay must be stacked in one shared fitted geometry"
        );
        assert_eq!(
            component
                .matches("RuvizRuntime.overlay-supported(root.slot-id);")
                .count(),
            2,
            "RuvizPlot must announce layered presentation from both init and \
             the slot handshake, since only one of them fires"
        );
    }

    #[test]
    fn compose_layers_blends_the_overlay_over_the_base() {
        let base = Arc::new(ruviz::core::Image::from_straight_rgba(
            1,
            1,
            vec![0, 0, 255, 255],
        ));
        let overlay = ruviz::core::Image::from_straight_rgba(1, 1, vec![255, 0, 0, 255]);
        assert_eq!(
            compose_layers(&base, Some(&overlay)).pixels,
            overlay.pixels,
            "an opaque overlay must replace the base"
        );
        assert_eq!(compose_layers(&base, None).pixels, base.pixels);
        let mismatched = ruviz::core::Image::from_straight_rgba(2, 1, vec![0; 8]);
        assert_eq!(
            compose_layers(&base, Some(&mismatched)).pixels,
            base.pixels,
            "a mismatched overlay must never corrupt the exported frame"
        );
    }

    /// Wait until a newer frame is installed and the slot's render lane is idle.
    fn wait_for_settled_frame_after(controller: &RuvizController, slot: SlotId, generation: u64) {
        wait_for(|| {
            controller
                .installed_generation(slot)
                .is_some_and(|installed| installed > generation)
                && !controller.inner.render_lane_busy(slot)
        });
    }

    fn first_hit_position(session: &InteractivePlotSession, size: (u32, u32)) -> ViewportPoint {
        for y in (0..size.1).step_by(4) {
            for x in (0..size.0).step_by(4) {
                let position = ViewportPoint::new(f64::from(x), f64::from(y));
                if !matches!(session.hit_test(position), HitResult::None) {
                    return position;
                }
            }
        }
        panic!("rendered scatter must expose a hit target");
    }

    fn layered_controller() -> (RuvizController, Arc<AtomicUsize>, Arc<Mutex<Vec<bool>>>) {
        let bases = Arc::new(AtomicUsize::new(0));
        let overlays = Arc::new(Mutex::new(Vec::<bool>::new()));
        let installed = Arc::clone(&bases);
        let controller = RuvizController::with_dispatcher(
            move |_, _| {
                installed.fetch_add(1, Ordering::SeqCst);
            },
            |task| {
                task();
                Ok(())
            },
        );
        let recorded = Arc::clone(&overlays);
        controller.on_overlay(move |_, overlay| {
            lock_scalar_recover(&recorded).push(overlay.is_some());
        });
        (controller, bases, overlays)
    }

    #[test]
    fn overlay_only_redraw_reuses_the_installed_base_layer() {
        let (controller, bases, overlays) = layered_controller();
        controller.set_plot(
            40,
            ruviz::prelude::Plot::new().scatter(&[0.0, 1.0, 2.0], &[0.0, 1.0, 0.0]),
            SlotOptions::default(),
        );
        controller.resize(40, 640.0, 480.0, 1.0);
        wait_for(|| controller.installed_size(40) == Some((640, 480)));
        let session = two_d_session(controller.inner.plot_handle(40).unwrap());
        let hit_position = first_hit_position(&session, (640, 480));
        let base_installs = bases.load(Ordering::SeqCst);
        assert!(base_installs >= 1, "the base layer must be installed once");
        assert!(
            lock_scalar_recover(&overlays).iter().all(|shown| !shown),
            "a plot without interaction state must not install an overlay"
        );
        let before_hover = controller
            .installed_generation(40)
            .expect("initial frame should be installed");

        controller.pointer_input(
            40,
            PointerInput {
                kind: PointerKind::Move,
                button: PointerButton::None,
                position: LogicalPoint::new(hit_position.x, hit_position.y),
            },
        );
        wait_for_settled_frame_after(&controller, 40, before_hover);
        assert_eq!(
            lock_scalar_recover(&overlays).last().copied(),
            Some(true),
            "hovering must install an overlay layer"
        );
        assert_eq!(
            bases.load(Ordering::SeqCst),
            base_installs,
            "an overlay-only redraw must not re-upload the base layer"
        );

        let hovered = controller
            .installed_generation(40)
            .expect("hover frame should be installed");
        session.apply_input(PlotInputEvent::ClearHover);
        wait_for_settled_frame_after(&controller, 40, hovered);
        assert_eq!(
            lock_scalar_recover(&overlays).last().copied(),
            Some(false),
            "clearing the hover must clear the presented overlay layer"
        );
        assert_eq!(
            bases.load(Ordering::SeqCst),
            base_installs,
            "clearing an overlay must not re-upload the base layer either"
        );
    }

    #[test]
    fn an_overlay_presented_by_an_uncommitted_frame_is_still_cleared() {
        let overlays = Arc::new(Mutex::new(Vec::<bool>::new()));
        let recorded = Arc::clone(&overlays);
        let hovered_session = Arc::new(Mutex::new(None::<InteractivePlotSession>));
        let sink_session = Arc::clone(&hovered_session);
        let controller = RuvizController::with_dispatcher(
            |_, _| {},
            |task| {
                task();
                Ok(())
            },
        );
        controller.on_overlay(move |_, overlay| {
            lock_scalar_recover(&recorded).push(overlay.is_some());
            // The crosshair is on screen now. Mutating the session here makes
            // its frame uncommittable, so `last_frame` never learns that the
            // slot presents an overlay.
            if overlay.is_some()
                && let Some(session) = lock_scalar_recover(&sink_session).take()
            {
                session.apply_input(PlotInputEvent::ClearHover);
            }
        });
        controller.set_plot(
            42,
            ruviz::prelude::Plot::new().scatter(&[0.0, 1.0, 2.0], &[0.0, 1.0, 0.0]),
            SlotOptions::default(),
        );
        controller.resize(42, 640.0, 480.0, 1.0);
        wait_for(|| controller.installed_size(42) == Some((640, 480)));
        let session = two_d_session(controller.inner.plot_handle(42).unwrap());
        let hit_position = first_hit_position(&session, (640, 480));
        *lock_scalar_recover(&hovered_session) = Some(session.clone());
        let before_hover = controller
            .installed_generation(42)
            .expect("initial frame should be installed");

        controller.pointer_input(
            42,
            PointerInput {
                kind: PointerKind::Move,
                button: PointerButton::None,
                position: LogicalPoint::new(hit_position.x, hit_position.y),
            },
        );
        wait_for_settled_frame_after(&controller, 42, before_hover);
        assert!(
            lock_scalar_recover(&overlays).contains(&true),
            "the hover frame must have presented an overlay"
        );
        assert_eq!(
            lock_scalar_recover(&overlays).last().copied(),
            Some(false),
            "an overlay presented by a frame that was never committed must \
             still be cleared by the next overlay-less frame"
        );
    }

    #[test]
    fn replacing_a_hovered_layered_plot_clears_the_stale_overlay() {
        let (controller, bases, overlays) = layered_controller();
        controller.set_plot(
            41,
            ruviz::prelude::Plot::new().scatter(&[0.0, 1.0, 2.0], &[0.0, 1.0, 0.0]),
            SlotOptions::default(),
        );
        controller.resize(41, 640.0, 480.0, 1.0);
        wait_for(|| controller.installed_size(41) == Some((640, 480)));
        let session = two_d_session(controller.inner.plot_handle(41).unwrap());
        let hit_position = first_hit_position(&session, (640, 480));
        let before_hover = controller
            .installed_generation(41)
            .expect("initial frame should be installed");
        controller.pointer_input(
            41,
            PointerInput {
                kind: PointerKind::Move,
                button: PointerButton::None,
                position: LogicalPoint::new(hit_position.x, hit_position.y),
            },
        );
        wait_for_settled_frame_after(&controller, 41, before_hover);
        assert_eq!(
            lock_scalar_recover(&overlays).last().copied(),
            Some(true),
            "the replaced plot must actually be showing an overlay"
        );
        let installs = bases.load(Ordering::SeqCst);
        let hovered = controller
            .installed_generation(41)
            .expect("hover frame should be installed");

        controller.set_plot(
            41,
            ruviz::prelude::Plot::new().bar(&["new"], &[7.0]),
            SlotOptions::default(),
        );
        wait_for_settled_frame_after(&controller, 41, hovered);
        assert!(
            bases.load(Ordering::SeqCst) > installs,
            "the replacement must publish its own base layer"
        );
        assert_eq!(
            lock_scalar_recover(&overlays).last().copied(),
            Some(false),
            "a replacement must never leave the old plot's overlay on screen"
        );
    }
}
