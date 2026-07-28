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
    collections::HashMap,
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use ruviz::core::{
    AlphaMode, HitResult, ImageFit, ImageTarget, InteractiveChangeSubscription,
    InteractivePlotSession, InteractiveRenderStamp, IntoPlotSession, LatestRequestScheduler,
    LogicalPoint, LogicalRect, PlotInputEvent, ScheduledRequest, ScheduledRequestId, ViewportPoint,
    fitted_content_rect, logical_to_physical, physical_backing_size, sanitize_scale_factor,
};
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

pub use slint_generated::{RuvizImageFit, RuvizPlotGrid, RuvizRuntime, RuvizSlotState};

static NEXT_SLOT_INCARNATION: AtomicU64 = AtomicU64::new(1);

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
type Dispatcher = Arc<dyn Fn(UiTask) -> Result<(), String> + Send + Sync + 'static>;
type PointerCallback = Arc<dyn Fn(PointerReport) + Send + Sync + 'static>;
type ErrorCallback = Arc<dyn Fn(AdapterError) + Send + Sync + 'static>;
type RuntimeConfigSink =
    Arc<dyn Fn(SlotId, InteractionMode, ImageFit, f32) + Send + Sync + 'static>;

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
/// Rendering uses a latest-request scheduler per slot. At most one worker is
/// active for a slot; intermediate resize/reactive requests are coalesced.
/// Workers send a [`SharedPixelBuffer`] to the Slint event loop, where the
/// [`slint::Image`] is constructed and installed. The last successful frame is
/// retained when a newer render fails.
#[derive(Clone)]
pub struct RuvizController {
    inner: Arc<ControllerInner>,
}

struct ControllerInner {
    slots: Mutex<HashMap<SlotId, SlotState>>,
    frame_sink: FrameSink,
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
    _subscription: Option<InteractiveChangeSubscription>,
    last_frame: Option<InstalledFrame>,
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

#[derive(Clone)]
enum RenderRequest {
    TwoD {
        incarnation: u64,
        session: InteractivePlotSession,
        target: ImageTarget,
    },
    #[cfg(feature = "3d")]
    ThreeD {
        incarnation: u64,
        session: Arc<Mutex<ruviz::core::InteractivePlot3DSession>>,
        renderer: Arc<Mutex<ruviz::core::BackgroundRenderer3D>>,
        job: ruviz::core::BackgroundRenderJob3D,
    },
}

impl RenderRequest {
    fn incarnation(&self) -> u64 {
        match self {
            Self::TwoD { incarnation, .. } => *incarnation,
            #[cfg(feature = "3d")]
            Self::ThreeD { incarnation, .. } => *incarnation,
        }
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
            Self::ThreeD { session, stamp } => session
                .lock()
                .expect("Slint 3D session lock poisoned")
                .is_render_current(*stamp),
        }
    }
}

struct RenderedBuffer {
    buffer: SharedPixelBuffer<Rgba8Pixel>,
    alpha_mode: AlphaMode,
    size_px: (u32, u32),
    validity: RenderValidity,
}

enum WorkerFailure {
    Superseded,
    Error(String),
}

impl RuvizController {
    /// Create a controller that installs frames through `frame_sink`.
    ///
    /// `frame_sink` always runs on the Slint UI event loop.
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
                frame_sink: Arc::new(frame_sink),
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
    pub fn attach<C>(component: &C) -> Self
    where
        C: slint::ComponentHandle + 'static,
        for<'a> RuvizRuntime<'a>: slint::Global<'a, C, StaticSelf = RuvizRuntime<'static>>,
    {
        use slint::Global as _;

        let runtime: RuvizRuntime<'_> = component.global();
        runtime.set_slots(slint::ModelRc::default());
        let frame_runtime = runtime.as_weak();
        let config_runtime = frame_runtime.clone();
        let config_sink: RuntimeConfigSink = Arc::new(move |slot, mode, fit, scale| {
            if let Some(runtime) = config_runtime.upgrade() {
                update_runtime_config(&runtime, slot, mode, fit, scale);
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
        controller.bind_runtime(component);
        controller
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
                );
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

    /// Register or replace a retained 2D slot while preserving its visible bounds.
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

        let (layout, previous_bounds) = {
            let mut slots = self.inner.slots.lock().expect("Slint slot lock poisoned");
            let old = slots.remove(&slot);
            let layout = old.as_ref().map_or(
                SlotLayout::with_scale(self.inner.default_scale_factor),
                |old| old.layout,
            );
            let bounds = if keep_view {
                #[cfg(feature = "3d")]
                {
                    old.as_ref().and_then(|old| match &old.plot {
                        PlotSlot::TwoD(session) => {
                            Some(session.view_bounds_snapshot().visible_bounds)
                        }
                        PlotSlot::ThreeD { .. } => None,
                    })
                }
                #[cfg(not(feature = "3d"))]
                {
                    old.as_ref().map(|old| {
                        let PlotSlot::TwoD(session) = &old.plot;
                        session.view_bounds_snapshot().visible_bounds
                    })
                }
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
                    incarnation: next_slot_incarnation(),
                    options,
                    layout,
                    scheduler,
                    drag: None,
                    _subscription: Some(subscription),
                    last_frame,
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
        let (layout, old_session, renderer, scheduler, last_frame) = {
            let mut slots = self.inner.slots.lock().expect("Slint slot lock poisoned");
            let old = slots.remove(&slot);
            let layout = old.as_ref().map_or(
                SlotLayout::with_scale(self.inner.default_scale_factor),
                |old| old.layout,
            );
            let old_session = old.as_ref().and_then(|old| match &old.plot {
                PlotSlot::ThreeD { session, .. } => Some(Arc::clone(session)),
                PlotSlot::TwoD(_) => None,
            });
            let desired_backend = background_backend(options.prefer_gpu);
            let renderer = old.as_ref().and_then(|old| match &old.plot {
                PlotSlot::ThreeD {
                    renderer, backend, ..
                } if *backend == desired_backend => Some(Arc::clone(renderer)),
                PlotSlot::TwoD(_) => None,
                PlotSlot::ThreeD { .. } => None,
            });
            let (scheduler, last_frame) = old.map_or_else(
                || (LatestRequestScheduler::default(), None),
                |old| (old.scheduler, old.last_frame),
            );
            (layout, old_session, renderer, scheduler, last_frame)
        };

        if keep_view && let Some(old_session) = old_session {
            replacement.restore_camera(
                old_session
                    .lock()
                    .expect("Slint 3D session lock poisoned")
                    .camera_snapshot(),
            )?;
        }
        let target = target_size(layout, options.sizing);
        replacement.resize(target.0, target.1, layout.scale_factor)?;
        let session = Arc::new(Mutex::new(replacement));
        let backend = background_backend(options.prefer_gpu);
        let renderer = renderer.unwrap_or_else(|| new_background_renderer(options.prefer_gpu));
        self.inner
            .slots
            .lock()
            .expect("Slint slot lock poisoned")
            .insert(
                slot,
                SlotState {
                    plot: PlotSlot::ThreeD {
                        session,
                        renderer,
                        backend,
                    },
                    incarnation: next_slot_incarnation(),
                    options,
                    layout,
                    scheduler,
                    drag: None,
                    _subscription: None,
                    last_frame,
                },
            );
        self.inner.sync_runtime(slot);
        self.inner.request_render(slot);
        Ok(())
    }

    /// Remove a slot. In-flight frames become non-installable.
    pub fn remove_plot(&self, slot: SlotId) -> bool {
        let removed = self
            .inner
            .slots
            .lock()
            .expect("Slint slot lock poisoned")
            .remove(&slot)
            .is_some();
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
            let mut slots = self.inner.slots.lock().expect("Slint slot lock poisoned");
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
            let mut slots = self.inner.slots.lock().expect("Slint slot lock poisoned");
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
                let result = session
                    .lock()
                    .expect("Slint 3D session lock poisoned")
                    .resize(target.0, target.1, scale_factor);
                if let Err(error) = result {
                    self.inner.report_error(slot, error.to_string());
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
    pub fn pointer_input(&self, slot: SlotId, input: PointerInput) {
        let action = {
            let mut slots = self.inner.slots.lock().expect("Slint slot lock poisoned");
            let Some(state) = slots.get_mut(&slot) else {
                return;
            };
            let handle = state.plot.clone_handle();
            if !state.pointer_input_enabled(input.kind) {
                if matches!(input.kind, PointerKind::Up | PointerKind::Cancel) {
                    state.drag = None;
                    PointerAction::Cancel { handle }
                } else {
                    return;
                }
            } else {
                let mapped = state.map_point(input.position);
                match input.kind {
                    PointerKind::Down => {
                        state.drag = Some(ActiveDrag {
                            button: input.button,
                            anchor: input.position,
                            last: input.position,
                            moved: false,
                        });
                        PointerAction::Down {
                            handle,
                            mapped,
                            input,
                        }
                    }
                    PointerKind::Move => {
                        if mapped.is_none() && state.drag.is_some() {
                            state.drag = None;
                            PointerAction::Cancel { handle }
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
                                drag.last = input.position;
                                state.drag = Some(drag);
                                (
                                    drag,
                                    if drag.moved {
                                        state.logical_delta_to_physical(delta)
                                    } else {
                                        LogicalPoint::default()
                                    },
                                )
                            });
                            PointerAction::Move {
                                handle,
                                mapped,
                                input,
                                drag,
                            }
                        }
                    }
                    PointerKind::Up => {
                        let drag = state.drag.take();
                        if mapped.is_none() || drag.is_some_and(|drag| drag.button != input.button)
                        {
                            PointerAction::Cancel { handle }
                        } else {
                            PointerAction::Up {
                                handle,
                                mapped,
                                input,
                                drag,
                            }
                        }
                    }
                    PointerKind::Cancel => {
                        state.drag = None;
                        PointerAction::Cancel { handle }
                    }
                }
            }
        };
        self.apply_pointer_action(slot, action);
        self.inner.sync_runtime(slot);
    }

    /// Apply a wheel zoom centered on the actual fitted image content.
    pub fn wheel(&self, slot: SlotId, delta_y: f64, position: LogicalPoint) -> bool {
        if !delta_y.is_finite() {
            return false;
        }
        let (handle, mapped) = {
            let slots = self.inner.slots.lock().expect("Slint slot lock poisoned");
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
                self.apply_3d_input(slot, &session, ruviz::core::InputEvent3D::Escape);
            }
            None => {}
        }
        self.inner.sync_runtime(slot);
    }

    /// Cancel a drag after release-outside, pointer capture loss, or focus loss.
    pub fn cancel_drag(&self, slot: SlotId) {
        let handle = {
            let mut slots = self.inner.slots.lock().expect("Slint slot lock poisoned");
            let Some(state) = slots.get_mut(&slot) else {
                return;
            };
            state.drag = None;
            state.plot.clone_handle()
        };
        match handle {
            PlotHandle::TwoD(session) => {
                session.cancel_interaction();
            }
            #[cfg(feature = "3d")]
            PlotHandle::ThreeD(session) => {
                session
                    .lock()
                    .expect("Slint 3D session lock poisoned")
                    .cancel_drag();
            }
        }
        self.inner.sync_runtime(slot);
    }

    /// Handle a double-click reset using fitted physical coordinates.
    pub fn double_click(&self, slot: SlotId, position: LogicalPoint) {
        let (handle, mapped) = {
            let slots = self.inner.slots.lock().expect("Slint slot lock poisoned");
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

    /// Install or clear the pointer report callback.
    pub fn on_pointer(&self, callback: impl Fn(PointerReport) + Send + Sync + 'static) {
        self.inner
            .callbacks
            .lock()
            .expect("Slint callback lock poisoned")
            .pointer = Some(Arc::new(callback));
    }

    /// Install or clear the error callback.
    pub fn on_error(&self, callback: impl Fn(AdapterError) + Send + Sync + 'static) {
        self.inner
            .callbacks
            .lock()
            .expect("Slint callback lock poisoned")
            .error = Some(Arc::new(callback));
    }

    /// Install the 3D pick callback.
    #[cfg(feature = "3d")]
    pub fn on_pick(
        &self,
        callback: impl Fn(SlotId, ruviz::core::PickHit3D) + Send + Sync + 'static,
    ) {
        self.inner
            .callbacks
            .lock()
            .expect("Slint callback lock poisoned")
            .pick = Some(Arc::new(callback));
    }

    /// Install the callback invoked after an authoritative 3D camera change.
    #[cfg(feature = "3d")]
    pub fn on_camera_change(
        &self,
        callback: impl Fn(SlotId, ruviz::core::CameraSnapshot3D) + Send + Sync + 'static,
    ) {
        self.inner
            .callbacks
            .lock()
            .expect("Slint callback lock poisoned")
            .camera = Some(Arc::new(callback));
    }

    /// Last frame dimensions installed for a slot.
    pub fn installed_size(&self, slot: SlotId) -> Option<(u32, u32)> {
        self.inner
            .slots
            .lock()
            .expect("Slint slot lock poisoned")
            .get(&slot)
            .and_then(|state| state.last_frame.as_ref())
            .map(|frame| frame.size_px)
    }

    /// Latest controller render generation installed for a slot.
    pub fn installed_generation(&self, slot: SlotId) -> Option<u64> {
        self.inner
            .slots
            .lock()
            .expect("Slint slot lock poisoned")
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
            } => match handle {
                PlotHandle::TwoD(session) => {
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
                        session
                            .lock()
                            .expect("Slint 3D session lock poisoned")
                            .cancel_drag();
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
                PlotHandle::ThreeD(session) => {
                    session
                        .lock()
                        .expect("Slint 3D session lock poisoned")
                        .cancel_drag();
                }
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
        let callback = self
            .inner
            .callbacks
            .lock()
            .expect("Slint callback lock poisoned")
            .pointer
            .clone();
        if let Some(callback) = callback {
            let hit = session
                .zip(physical_position)
                .map(|(session, position)| session.hit_test(position));
            callback(PointerReport {
                slot,
                kind,
                button,
                logical_position,
                physical_position,
                hit,
            });
        }
    }

    #[cfg(feature = "3d")]
    fn apply_3d_input(
        &self,
        slot: SlotId,
        session: &Arc<Mutex<ruviz::core::InteractivePlot3DSession>>,
        event: ruviz::core::InputEvent3D,
    ) {
        let result = session
            .lock()
            .expect("Slint 3D session lock poisoned")
            .handle_input(event);
        match result {
            Ok(result) => {
                let (pick_callback, camera_callback) = {
                    let callbacks = self
                        .inner
                        .callbacks
                        .lock()
                        .expect("Slint callback lock poisoned");
                    (callbacks.pick.clone(), callbacks.camera.clone())
                };
                if let (Some(hit), Some(callback)) = (result.picked, pick_callback) {
                    callback(slot, hit);
                }
                if result.camera_changed
                    && let Some(callback) = camera_callback
                {
                    callback(
                        slot,
                        session
                            .lock()
                            .expect("Slint 3D session lock poisoned")
                            .camera_snapshot(),
                    );
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
        let Some((incarnation, interaction, fit, scale)) = self
            .slots
            .lock()
            .expect("Slint slot lock poisoned")
            .get(&slot)
            .map(|state| {
                (
                    state.incarnation,
                    state.options.interaction,
                    state.options.fit,
                    state.layout.scale_factor,
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
            let current = inner
                .slots
                .lock()
                .expect("Slint slot lock poisoned")
                .get(&slot)
                .is_some_and(|state| state.incarnation == incarnation);
            if current {
                config_sink(slot, interaction, fit, scale);
            }
        });
        if let Err(error) = (self.dispatcher)(task) {
            self.report_error_direct(
                slot,
                format!("could not schedule runtime configuration: {error}"),
            );
        }
    }

    fn clear_runtime(self: &Arc<Self>, slot: SlotId) {
        let sink = Arc::clone(&self.frame_sink);
        let config_sink = self.runtime_config_sink.clone();
        let weak = Arc::downgrade(self);
        let task = Box::new(move || {
            let Some(inner) = weak.upgrade() else {
                return;
            };
            if inner
                .slots
                .lock()
                .expect("Slint slot lock poisoned")
                .contains_key(&slot)
            {
                return;
            }
            sink(slot, slint::Image::default());
            if let Some(config_sink) = config_sink {
                config_sink(slot, InteractionMode::Static, ImageFit::Contain, 1.0);
            }
        });
        if let Err(error) = (self.dispatcher)(task) {
            self.report_error_direct(slot, format!("could not clear runtime slot: {error}"));
        }
    }

    fn request_render(self: &Arc<Self>, slot: SlotId) {
        let scheduled = {
            let mut slots = self.slots.lock().expect("Slint slot lock poisoned");
            let Some(state) = slots.get_mut(&slot) else {
                return;
            };
            let target = target_size(state.layout, state.options.sizing);
            let request: Result<RenderRequest, String> = match &state.plot {
                PlotSlot::TwoD(session) => Ok(RenderRequest::TwoD {
                    incarnation: state.incarnation,
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
                } => session
                    .lock()
                    .expect("Slint 3D session lock poisoned")
                    .background_render_job()
                    .map(|job| RenderRequest::ThreeD {
                        incarnation: state.incarnation,
                        session: Arc::clone(session),
                        renderer: Arc::clone(renderer),
                        job,
                    })
                    .map_err(|error| error.to_string()),
            };
            request.map(|request| state.scheduler.request(request))
        };
        match scheduled {
            Ok(Some(scheduled)) => self.spawn_render(slot, scheduled),
            Ok(None) => {}
            Err(error) => self.report_error(slot, error),
        }
    }

    fn spawn_render(self: &Arc<Self>, slot: SlotId, scheduled: ScheduledRequest<RenderRequest>) {
        let weak = Arc::downgrade(self);
        let id = scheduled.id();
        let request = scheduled.into_request();
        let incarnation = request.incarnation();
        #[cfg(test)]
        let render_barrier = self
            .render_barrier
            .lock()
            .expect("Slint render barrier lock poisoned")
            .clone();
        let spawn = std::thread::Builder::new()
            .name(format!("ruviz-slint-{slot}"))
            .spawn(move || {
                #[cfg(test)]
                if let Some((entered, release)) = render_barrier {
                    entered.wait();
                    release.wait();
                }
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    render_request(request)
                }))
                .unwrap_or_else(|_| {
                    Err(WorkerFailure::Error(
                        "plot renderer panicked while producing a background frame".to_string(),
                    ))
                });
                if let Some(inner) = weak.upgrade() {
                    inner.finish_render(slot, id, incarnation, result);
                }
            });
        if let Err(error) = spawn {
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

    fn finish_render(
        self: &Arc<Self>,
        slot: SlotId,
        id: ScheduledRequestId,
        incarnation: u64,
        result: Result<RenderedBuffer, WorkerFailure>,
    ) {
        let (install, next) = {
            let mut slots = self.slots.lock().expect("Slint slot lock poisoned");
            let Some(state) = slots.get_mut(&slot) else {
                return;
            };
            let Some(completion) = state.scheduler.complete(id) else {
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
                let task = Box::new(move || {
                    let Some(inner) = weak.upgrade() else {
                        return;
                    };
                    let current_generation = {
                        let mut slots = inner.slots.lock().expect("Slint slot lock poisoned");
                        let Some(state) = slots.get_mut(&slot) else {
                            return;
                        };
                        if state.incarnation != incarnation
                            || state.scheduler.latest_generation() != id.generation()
                            || !rendered.is_current()
                        {
                            return;
                        }
                        let validity = rendered.validity.clone();
                        state.last_frame = Some(InstalledFrame {
                            size_px: rendered.size_px,
                            generation: id.generation(),
                            incarnation,
                            validity,
                        });
                        id.generation()
                    };
                    let image = match rendered.alpha_mode {
                        AlphaMode::Straight => slint::Image::from_rgba8(rendered.buffer),
                        AlphaMode::Premultiplied => {
                            slint::Image::from_rgba8_premultiplied(rendered.buffer)
                        }
                    };
                    if current_generation == id.generation() {
                        sink(slot, image);
                        inner.sync_runtime(slot);
                    }
                });
                if let Err(error) = (self.dispatcher)(task) {
                    self.report_error(slot, format!("could not schedule frame install: {error}"));
                }
            }
            Ok(_) => {}
            Err(WorkerFailure::Superseded) => {}
            Err(WorkerFailure::Error(message)) => self.report_error(slot, message),
        }

        if let Some(next) = next {
            self.spawn_render(slot, next);
        }
    }

    fn plot_handle(&self, slot: SlotId) -> Option<PlotHandle> {
        self.slots
            .lock()
            .expect("Slint slot lock poisoned")
            .get(&slot)
            .map(|state| state.plot.clone_handle())
    }

    fn report_error(&self, slot: SlotId, message: String) {
        let callback = self
            .callbacks
            .lock()
            .expect("Slint callback lock poisoned")
            .error
            .clone();
        if let Some(callback) = callback {
            let task = Box::new(move || callback(AdapterError { slot, message }));
            if let Err(error) = (self.dispatcher)(task) {
                self.report_error_direct(
                    slot,
                    format!("could not schedule error callback: {error}"),
                );
            }
        }
    }

    fn report_error_direct(&self, slot: SlotId, message: String) {
        let callback = self
            .callbacks
            .lock()
            .expect("Slint callback lock poisoned")
            .error
            .clone();
        if let Some(callback) = callback {
            callback(AdapterError { slot, message });
        }
    }
}

impl RenderedBuffer {
    fn is_current(&self) -> bool {
        match &self.validity {
            RenderValidity::TwoD { session, stamp } => session.is_render_stamp_current(*stamp),
            #[cfg(feature = "3d")]
            RenderValidity::ThreeD { session, stamp } => session
                .lock()
                .expect("Slint 3D session lock poisoned")
                .is_render_current(*stamp),
        }
    }
}

fn render_request(request: RenderRequest) -> Result<RenderedBuffer, WorkerFailure> {
    let (image, validity) = match request {
        RenderRequest::TwoD {
            incarnation: _,
            session,
            target,
        } => {
            let frame = session.render_to_image_stamped(target).map_err(|error| {
                if error.is_render_superseded() {
                    WorkerFailure::Superseded
                } else {
                    WorkerFailure::Error(error.to_string())
                }
            })?;
            let stamp = frame.render_stamp();
            (
                (*frame.frame.image).clone(),
                RenderValidity::TwoD { session, stamp },
            )
        }
        #[cfg(feature = "3d")]
        RenderRequest::ThreeD {
            incarnation: _,
            session,
            renderer,
            job,
        } => {
            let rendered = renderer
                .lock()
                .expect("Slint 3D renderer lock poisoned")
                .render(job)
                .map_err(|error| WorkerFailure::Error(error.to_string()))?;
            let stamp = rendered.stamp;
            (rendered.image, RenderValidity::ThreeD { session, stamp })
        }
    };
    let expected = usize::try_from(image.width)
        .ok()
        .and_then(|width| {
            usize::try_from(image.height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| {
            WorkerFailure::Error("rendered image dimensions overflow address space".to_string())
        })?;
    if image.pixels.len() != expected {
        return Err(WorkerFailure::Error(format!(
            "rendered RGBA buffer has {} bytes, expected {expected}",
            image.pixels.len()
        )));
    }
    let buffer = SharedPixelBuffer::clone_from_slice(&image.pixels, image.width, image.height);
    Ok(RenderedBuffer {
        buffer,
        alpha_mode: image.alpha_mode(),
        size_px: (image.width, image.height),
        validity,
    })
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

fn next_slot_incarnation() -> u64 {
    NEXT_SLOT_INCARNATION
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .expect("Slint slot incarnation space exhausted")
}

fn update_runtime_image(runtime: &RuvizRuntime<'_>, slot: SlotId, image: slint::Image) {
    update_runtime_row(runtime, slot, |row| row.source = image);
}

fn update_runtime_config(
    runtime: &RuvizRuntime<'_>,
    slot: SlotId,
    interaction: InteractionMode,
    fit: ImageFit,
    scale_factor: f32,
) {
    update_runtime_row(runtime, slot, |row| {
        row.interactive = interaction == InteractionMode::Interactive;
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

    fn wait_for(condition: impl Fn() -> bool) {
        let start = Instant::now();
        while !condition() {
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "timed out waiting for render worker"
            );
            std::thread::yield_now();
        }
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
            _subscription: None,
            last_frame: None,
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
        wait_for(|| controller.installed_generation(12).is_some());
        let new_incarnation = controller
            .inner
            .slots
            .lock()
            .expect("Slint slot lock poisoned")[&12]
            .incarnation;
        let installed = controller.installed_generation(12);
        assert_ne!(old_incarnation, new_incarnation);
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
        assert_eq!(controller.installed_generation(12), installed);
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

    #[test]
    fn images_use_matching_slint_alpha_constructors() {
        let straight = ruviz::core::Image::from_straight_rgba(1, 1, vec![20, 40, 60, 128]);
        let premultiplied =
            ruviz::core::Image::from_premultiplied_rgba(1, 1, vec![10, 20, 30, 128]);
        for image in [straight, premultiplied] {
            let buffer =
                SharedPixelBuffer::clone_from_slice(&image.pixels, image.width, image.height);
            let slint_image = match image.alpha_mode() {
                AlphaMode::Straight => slint::Image::from_rgba8(buffer),
                AlphaMode::Premultiplied => slint::Image::from_rgba8_premultiplied(buffer),
            };
            assert_eq!(slint_image.size().width, 1);
        }
    }
}
