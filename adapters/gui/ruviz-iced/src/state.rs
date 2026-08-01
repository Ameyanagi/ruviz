use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
#[cfg(feature = "3d")]
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use futures::StreamExt;
use futures::stream::BoxStream;
use iced::{Subscription, Task};
use ruviz::axes::AxisScale;
use ruviz::core::{
    ImageFit, ImageTarget, InteractiveChangeRevision, InteractiveChangeSubscription,
    InteractivePlotSession, IntoPlotSession, LatestRequestScheduler, PlotContextMenuAction,
    PlotInputEvent, PlottingError, ScheduledRequest, ScheduledRequestId, ViewportPoint,
    ViewportRect, physical_backing_size, sanitize_scale_factor,
};

#[cfg(feature = "3d")]
use ruviz::core::{
    BackgroundRenderBackend3D, BackgroundRenderJob3D, BackgroundRenderOutcome3D,
    BackgroundRenderer3D, InputEvent3D, InteractivePlot3DSession, PointerButton3D,
    TryIntoPlot3DSession,
};

use crate::{
    ContextActionCompletion, Event, ExportSource, LayerRole, LayerUpload, Message, MessageKind,
    PointerButton, PreparedLayers, Presentation, PresentedImage, PresentedLayerSources, Sizing,
    StateIncarnation, Update, WidgetEvent,
};
#[cfg(feature = "3d")]
use crate::{Rendered3DFrame, iced_handle_owned};

static NEXT_WAKE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq)]
struct RenderRequest2D {
    incarnation: StateIncarnation,
    change_revision: InteractiveChangeRevision,
    size_px: (u32, u32),
    scale_factor: f32,
    time_seconds: f64,
}

struct Drag2D {
    button: PointerButton,
    anchor: ViewportPoint,
    last: ViewportPoint,
    moved: bool,
}

#[derive(Clone)]
struct WakeData {
    id: u64,
    receiver: async_channel::Receiver<()>,
}

impl PartialEq for WakeData {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for WakeData {}

impl Hash for WakeData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

struct ChangeWake {
    data: WakeData,
    _subscription: InteractiveChangeSubscription,
}

impl ChangeWake {
    fn new(session: &InteractivePlotSession) -> Self {
        let (sender, receiver) = async_channel::bounded(1);
        let subscription = session.subscribe_changes(move |_| {
            let _ = sender.try_send(());
        });
        Self {
            data: WakeData {
                id: NEXT_WAKE_ID.fetch_add(1, Ordering::Relaxed),
                receiver,
            },
            _subscription: subscription,
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::run_with(self.data.clone(), wake_stream)
            .map(|()| Message(MessageKind::Changed2D))
    }
}

fn wake_stream(data: &WakeData) -> BoxStream<'static, ()> {
    let receiver = data.receiver.clone();
    futures::stream::unfold(receiver, |receiver| async move {
        receiver.recv().await.ok().map(|()| ((), receiver))
    })
    .boxed()
}

/// Iced-owned state for a static or interactive 2D plot.
pub struct PlotState {
    session: InteractivePlotSession,
    pub(crate) presentation: Presentation,
    pub(crate) interaction_enabled: bool,
    pub(crate) sizing: Sizing,
    pub(crate) fit: ImageFit,
    scale_factor: f32,
    logical_size: (f64, f64),
    time_seconds: f64,
    scheduler: LatestRequestScheduler<RenderRequest2D>,
    incarnation: StateIncarnation,
    pub(crate) presented: Option<PresentedImage>,
    pub(crate) presented_stamp: Option<ruviz::core::InteractiveRenderStamp>,
    presented_layers: PresentedLayerSources,
    wake: ChangeWake,
    last_seen_change: InteractiveChangeRevision,
    drag: Option<Drag2D>,
    cursor_px: Option<ViewportPoint>,
    /// Newest cursor position observed while a render was in flight.
    ///
    /// Hover moves are coalesced rather than dropped: no event is derived from
    /// stale geometry, but the position is replayed once the frame is current.
    pending_hover: Option<ViewportPoint>,
}

/// Construct a static 2D state.
pub fn static_view<P: IntoPlotSession>(plot: P) -> PlotState {
    PlotState::static_view(plot)
}

/// Construct an interactive 2D state.
pub fn interactive<P: IntoPlotSession>(plot: P) -> PlotState {
    PlotState::interactive(plot)
}

impl PlotState {
    /// Construct a static image-backed plot.
    pub fn static_view<P: IntoPlotSession>(plot: P) -> Self {
        Self::new(plot.into_plot_session(), Presentation::Static)
    }

    /// Construct an interactive plot with pan, zoom, hover, selection, and
    /// reset gestures.
    pub fn interactive<P: IntoPlotSession>(plot: P) -> Self {
        Self::new(plot.into_plot_session(), Presentation::Interactive)
    }

    fn new(session: InteractivePlotSession, presentation: Presentation) -> Self {
        let wake = ChangeWake::new(&session);
        let last_seen_change = session.change_revision();
        Self {
            session,
            presentation,
            interaction_enabled: presentation == Presentation::Interactive,
            sizing: Sizing::default(),
            fit: ImageFit::Contain,
            scale_factor: 1.0,
            logical_size: Sizing::default().logical_fallback(),
            time_seconds: 0.0,
            scheduler: LatestRequestScheduler::default(),
            incarnation: StateIncarnation::new(),
            presented: None,
            presented_stamp: None,
            presented_layers: PresentedLayerSources::default(),
            wake,
            last_seen_change,
            drag: None,
            cursor_px: None,
            pending_hover: None,
        }
    }

    /// Consume this state and use all available Iced layout space.
    pub fn fill(mut self) -> Self {
        self.sizing = Sizing::Fill;
        self
    }

    /// Consume this state and request a fixed logical size.
    pub fn fixed(mut self, width: f32, height: f32) -> Self {
        self = self.fixed_pixels(width, height);
        self
    }

    /// Consume this state and request a fixed size in logical Iced pixels.
    pub fn fixed_pixels(mut self, width: f32, height: f32) -> Self {
        self.sizing = valid_fixed(width, height);
        self.logical_size = self.sizing.logical_fallback();
        self
    }

    /// Select image fitting used for drawing and input mapping.
    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }

    /// Set the initial device scale. Later Iced `Rescaled` window events keep
    /// this value current automatically.
    pub fn scale_factor(mut self, scale_factor: f32) -> Self {
        self.scale_factor = sanitize_scale_factor(scale_factor);
        self
    }

    /// Prefer ruviz's GPU backend where it can render and read an image back.
    ///
    /// This does not enable direct or zero-copy Iced presentation.
    pub fn prefer_gpu(self, prefer_gpu: bool) -> Self {
        self.session.set_prefer_gpu(prefer_gpu);
        self
    }

    /// Current presentation mode.
    pub const fn presentation(&self) -> Presentation {
        self.presentation
    }

    /// Whether pan, zoom, hover, and selection gestures are currently enabled.
    ///
    /// The context menu remains available while interaction is disabled.
    pub const fn interaction_enabled(&self) -> bool {
        self.interaction_enabled
    }

    /// Enable or disable plot gestures while keeping the context menu available.
    pub fn set_interaction_enabled(&mut self, enabled: bool) -> Update {
        if self.interaction_enabled == enabled {
            return Update::none();
        }
        self.interaction_enabled = enabled;
        // Both sides must run: a widget-level drag and a session-level
        // interaction can be live at once, and leaving either behind would
        // strand it. Kept as separate statements rather than `|` so the
        // unconditional evaluation is not mistaken for a typo'd `||`.
        let dropped_drag = self.drag.take().is_some();
        let cancelled_interaction = self.session.cancel_interaction();
        let had_drag = dropped_drag || cancelled_interaction;
        let task = if had_drag {
            self.note_direct_change();
            self.queue_render()
        } else {
            Task::none()
        };
        Update::with_event(task, Event::InteractionToggled(enabled))
    }

    /// Current sizing policy.
    pub const fn sizing(&self) -> Sizing {
        self.sizing
    }

    /// Current fitted-image policy.
    pub const fn image_fit(&self) -> ImageFit {
        self.fit
    }

    /// Retained ruviz session.
    pub const fn session(&self) -> &InteractivePlotSession {
        &self.session
    }

    /// Source alpha mode of the retained frame. Iced receives straight RGBA.
    pub fn source_alpha_mode(&self) -> Option<ruviz::core::AlphaMode> {
        self.presented.as_ref().map(|frame| frame.source_alpha)
    }

    /// Request the initial or a forced redraw.
    pub fn request_render(&mut self) -> Update {
        self.last_seen_change = self.session.change_revision();
        Update::task(self.queue_render())
    }

    /// Set the animation/temporal time and redraw.
    pub fn set_time(&mut self, time_seconds: f64) -> Update {
        if !time_seconds.is_finite() {
            return Update::with_event(
                Task::none(),
                Event::Error(PlottingError::InvalidInput(
                    "Iced plot time must be finite".to_owned(),
                )),
            );
        }
        self.time_seconds = time_seconds;
        self.session
            .apply_input(PlotInputEvent::SetTime { time_seconds });
        self.note_direct_change();
        Update::task(self.queue_render())
    }

    /// Replace the plot and reset to the replacement's own viewport.
    pub fn set_plot<P: IntoPlotSession>(&mut self, plot: P) -> Update {
        self.replace_session(plot.into_plot_session(), None);
        Update::task(self.queue_render())
    }

    /// Replace the plot while restoring a user-customized visible view.
    ///
    /// If the old view still matches its base bounds, the replacement keeps
    /// its own natural bounds.
    pub fn set_plot_keep_view<P: IntoPlotSession>(&mut self, plot: P) -> Update {
        let old_view = self.session.view_bounds_snapshot();
        let visible = viewport_bounds_materially_differ(
            old_view.visible_bounds,
            old_view.base_bounds,
            &old_view.x_scale,
            &old_view.y_scale,
        )
        .then_some(old_view.visible_bounds);
        self.replace_session(plot.into_plot_session(), visible);
        Update::task(self.queue_render())
    }

    fn replace_session(&mut self, session: InteractivePlotSession, visible: Option<ViewportRect>) {
        self.session = session;
        if let Some(visible) = visible {
            self.session.defer_visible_bounds_restore(visible);
        }
        self.wake = ChangeWake::new(&self.session);
        self.last_seen_change = self.session.change_revision();
        self.incarnation = StateIncarnation::new();
        self.drag = None;
        self.cursor_px = None;
        self.pending_hover = None;
        self.presented_layers = PresentedLayerSources::default();
        // Keep the previous allocation visible until the replacement is ready.
    }

    /// Subscription that wakes Iced for reactive data and other out-of-band
    /// retained-session changes.
    pub fn subscription(&self) -> Subscription<Message> {
        self.wake.subscription()
    }

    /// Route one adapter message through the Elm-owned state.
    pub fn update(&mut self, message: Message) -> Update {
        match message.0 {
            MessageKind::Widget2D(event) => self.handle_widget_event(event),
            MessageKind::Changed2D => {
                let revision = self.session.change_revision();
                if revision == self.last_seen_change {
                    Update::none()
                } else {
                    self.last_seen_change = revision;
                    Update::task(self.queue_render())
                }
            }
            MessageKind::Rendered2D {
                incarnation,
                request_id,
                change_revision,
                result,
            } => self.complete_render(incarnation, request_id, change_revision, result),
            MessageKind::Allocated2D {
                incarnation,
                request_id,
                layers,
                allocations,
            } => self.complete_allocation(incarnation, request_id, layers, allocations),
            MessageKind::ContextActionCompleted(result) => complete_context_action(result),
            #[cfg(feature = "3d")]
            _ => Update::none(),
        }
    }

    fn handle_widget_event(&mut self, event: WidgetEvent) -> Update {
        match event {
            WidgetEvent::BoundsChanged { logical_size } => {
                let normalized = (
                    finite_dimension(logical_size.0),
                    finite_dimension(logical_size.1),
                );
                if self.logical_size == normalized {
                    return Update::none();
                }
                self.logical_size = normalized;
                Update::task(self.queue_render())
            }
            WidgetEvent::ScaleFactorChanged(scale_factor) => {
                let scale_factor = sanitize_scale_factor(scale_factor);
                if self.scale_factor.to_bits() == scale_factor.to_bits() {
                    return Update::none();
                }
                self.scale_factor = scale_factor;
                Update::task(self.queue_render())
            }
            WidgetEvent::ContextMenuAction(action) => self.context_menu_action(action),
            _ if !self.interaction_enabled => Update::none(),
            WidgetEvent::PointerMoved(position) => self.pointer_moved(position),
            WidgetEvent::HoverCoalesced { position_px } => {
                // Stale geometry: remember the newest position only. It is
                // replayed against fresh geometry once a frame is installed.
                self.pending_hover = Some(viewport_point(position_px));
                Update::none()
            }
            WidgetEvent::PointerPressed {
                position_px,
                button,
            } => {
                let point = viewport_point(position_px);
                self.cursor_px = Some(point);
                self.drag = Some(Drag2D {
                    button,
                    anchor: point,
                    last: point,
                    moved: false,
                });
                if button == PointerButton::Right {
                    self.session
                        .apply_input(PlotInputEvent::BrushStart { position_px: point });
                    self.note_direct_change();
                    Update::task(self.queue_render())
                } else {
                    Update::none()
                }
            }
            WidgetEvent::PointerReleased {
                position_px,
                button,
            } => self.pointer_released(viewport_point(position_px), button),
            WidgetEvent::DoubleClick { .. } => {
                self.drag = None;
                self.session.cancel_interaction();
                self.session.apply_input(PlotInputEvent::ResetView);
                self.note_direct_change();
                Update::with_event(self.queue_render(), Event::ViewReset)
            }
            WidgetEvent::Escape => {
                let adapter_drag = self.drag.take().is_some();
                let core_interaction = self.session.cancel_interaction();
                if adapter_drag || core_interaction {
                    self.note_direct_change();
                    Update::with_event(self.queue_render(), Event::DragCancelled)
                } else {
                    self.session.apply_input(PlotInputEvent::ResetView);
                    self.note_direct_change();
                    Update::with_event(self.queue_render(), Event::ViewReset)
                }
            }
            WidgetEvent::Wheel {
                position_px,
                delta_y,
            } => {
                let factor = (f64::from(delta_y) * 0.0025).exp();
                self.session.apply_input(PlotInputEvent::Zoom {
                    factor,
                    center_px: viewport_point(position_px),
                });
                self.note_direct_change();
                Update::with_event(self.queue_render(), Event::ViewChanged)
            }
            WidgetEvent::CancelDrag => self.cancel_drag(),
        }
    }

    fn context_menu_action(&mut self, action: PlotContextMenuAction) -> Update {
        match action {
            PlotContextMenuAction::ResetView => {
                self.drag = None;
                self.session.cancel_interaction();
                self.session.apply_input(PlotInputEvent::ResetView);
                self.note_direct_change();
                Update::with_event(self.queue_render(), Event::ViewReset)
            }
            PlotContextMenuAction::FitToContent => {
                self.drag = None;
                self.session.cancel_interaction();
                self.session.apply_input(PlotInputEvent::ResetView);
                self.note_direct_change();
                Update::with_event(self.queue_render(), Event::ViewChanged)
            }
            PlotContextMenuAction::SaveImage => {
                self.presented.as_ref().map_or_else(Update::none, |image| {
                    Update::task(save_presented_image_task(image, "ruviz-plot.png"))
                })
            }
            PlotContextMenuAction::CopyImage => {
                self.presented.as_ref().map_or_else(Update::none, |image| {
                    Update::task(copy_presented_image_task(image))
                })
            }
            PlotContextMenuAction::ToggleInteraction => {
                self.set_interaction_enabled(!self.interaction_enabled)
            }
            #[cfg(feature = "3d")]
            PlotContextMenuAction::CameraView(_) => Update::none(),
            _ => Update::none(),
        }
    }

    fn pointer_moved(&mut self, position: Option<(f64, f64)>) -> Update {
        self.pending_hover = None;
        let Some(position) = position.map(viewport_point) else {
            self.cursor_px = None;
            self.session.apply_input(PlotInputEvent::ClearHover);
            self.note_direct_change();
            return Update::with_event(self.queue_render(), Event::Hovered2D(None));
        };
        self.cursor_px = Some(position);
        if let Some(drag) = &mut self.drag {
            let delta = ViewportPoint::new(position.x - drag.last.x, position.y - drag.last.y);
            let total = (position.x - drag.anchor.x).hypot(position.y - drag.anchor.y);
            drag.moved |= total >= 3.0;
            drag.last = position;
            let view_changed = match drag.button {
                PointerButton::Left | PointerButton::Middle if drag.moved => {
                    self.session
                        .apply_input(PlotInputEvent::Pan { delta_px: delta });
                    true
                }
                PointerButton::Right => {
                    self.session.apply_input(PlotInputEvent::BrushMove {
                        position_px: position,
                    });
                    false
                }
                _ => return Update::none(),
            };
            self.note_direct_change();
            if view_changed {
                Update::with_event(self.queue_render(), Event::ViewChanged)
            } else {
                Update::task(self.queue_render())
            }
        } else {
            self.session.apply_input(PlotInputEvent::Hover {
                position_px: position,
            });
            let hit = self.session.hit_test(position);
            self.note_direct_change();
            let hovered = (!matches!(hit, ruviz::core::HitResult::None)).then_some(hit);
            Update::with_event(self.queue_render(), Event::Hovered2D(hovered))
        }
    }

    fn pointer_released(&mut self, position: ViewportPoint, button: PointerButton) -> Update {
        let Some(drag) = self.drag.take() else {
            return Update::none();
        };
        if drag.button != button {
            return self.cancel_captured_drag(drag);
        }
        match button {
            PointerButton::Right => {
                self.session.apply_input(PlotInputEvent::BrushEnd {
                    position_px: position,
                });
                self.session.apply_input(PlotInputEvent::ZoomRect {
                    region_px: ViewportRect::from_points(drag.anchor, position),
                });
                self.note_direct_change();
                Update::with_event(self.queue_render(), Event::ViewChanged)
            }
            PointerButton::Left if !drag.moved => {
                self.session.apply_input(PlotInputEvent::SelectAt {
                    position_px: position,
                });
                let hit = self.session.hit_test(position);
                self.note_direct_change();
                Update {
                    task: self.queue_render(),
                    events: vec![Event::Clicked2D(hit.clone()), Event::SelectionChanged(hit)],
                }
            }
            _ => Update::none(),
        }
    }

    fn cancel_drag(&mut self) -> Update {
        let Some(drag) = self.drag.take() else {
            return Update::none();
        };
        self.cancel_captured_drag(drag)
    }

    fn cancel_captured_drag(&mut self, drag: Drag2D) -> Update {
        if drag.button == PointerButton::Right && self.session.cancel_interaction() {
            self.note_direct_change();
        }
        Update::with_event(self.queue_render(), Event::DragCancelled)
    }

    fn note_direct_change(&mut self) {
        self.last_seen_change = self.session.change_revision();
    }

    fn queue_render(&mut self) -> Task<Message> {
        let size_px =
            physical_backing_size(self.logical_size.0, self.logical_size.1, self.scale_factor);
        let request = RenderRequest2D {
            incarnation: self.incarnation.clone(),
            change_revision: self.session.change_revision(),
            size_px,
            scale_factor: self.scale_factor,
            time_seconds: self.time_seconds,
        };
        match self.scheduler.request(request) {
            Some(request) => {
                render_2d_task(request, self.session.clone(), self.presented_layers.clone())
            }
            None => Task::none(),
        }
    }

    fn complete_render(
        &mut self,
        incarnation: StateIncarnation,
        request_id: ScheduledRequestId,
        change_revision: InteractiveChangeRevision,
        result: Result<PreparedLayers, PlottingError>,
    ) -> Update {
        let Some(completion) = self.scheduler.complete(request_id.clone()) else {
            return Update::none();
        };
        let next_task = completion
            .next
            .map(|next| render_2d_task(next, self.session.clone(), self.presented_layers.clone()))
            .unwrap_or_else(Task::none);

        if !completion.install || incarnation != self.incarnation {
            return Update::task(next_task);
        }

        match result {
            Ok(prepared)
                if self
                    .session
                    .is_render_stamp_current(prepared.layers.render_stamp()) =>
            {
                let allocate = image_allocation_2d_task(incarnation, request_id, prepared);
                Update::task(Task::batch([next_task, allocate]))
            }
            Ok(_) => Update::task(next_task),
            Err(error) if error.is_render_superseded() => Update::task(next_task),
            Err(_) if change_revision != self.session.change_revision() => Update::task(next_task),
            Err(error) => Update::with_event(next_task, Event::Error(error)),
        }
    }

    fn complete_allocation(
        &mut self,
        incarnation: StateIncarnation,
        _request_id: ScheduledRequestId,
        layers: Option<ruviz::core::StampedInteractiveLayers>,
        allocations: Vec<(LayerRole, Result<iced::widget::image::Allocation, String>)>,
    ) -> Update {
        let Some(layers) = layers else {
            return Update::none();
        };
        if incarnation != self.incarnation
            || !self.session.is_render_stamp_current(layers.render_stamp())
        {
            return Update::none();
        }

        let mut base = None;
        let mut overlay = None;
        for (role, allocation) in allocations {
            match allocation {
                Ok(allocation) => match role {
                    LayerRole::Base => base = Some(allocation),
                    LayerRole::Overlay => overlay = Some(allocation),
                },
                Err(error) => {
                    return Update::with_event(
                        Task::none(),
                        Event::Error(PlottingError::RenderError(format!(
                            "Iced image allocation failed: {error}"
                        ))),
                    );
                }
            }
        }
        let Some(base) = base else {
            return Update::none();
        };

        // Iced's `Handle::from_rgba` and iced_wgpu's image pipeline both want
        // straight alpha, so this adapter materializes the straight view. The
        // view is memoized on the shared cached layer, so an overlay-only
        // redraw still yields the same Arc and hits `LayerUpload::Reuse`.
        let image = layers.base.image();
        self.presented = Some(PresentedImage {
            allocation: base.clone(),
            overlay: overlay.clone(),
            size_px: (image.width, image.height),
            source_alpha: image.alpha_mode(),
        });
        self.presented_layers = PresentedLayerSources {
            base: Some((Arc::clone(image), base)),
            overlay: layers
                .overlay
                .as_ref()
                .map(|layer| layer.image())
                .zip(overlay)
                .map(|(image, allocation)| (Arc::clone(image), allocation)),
        };
        self.presented_stamp = Some(layers.render_stamp());
        self.apply_pending_hover()
    }

    /// Replay the newest coalesced hover position now that geometry is current.
    fn apply_pending_hover(&mut self) -> Update {
        let Some(position) = self.pending_hover.take() else {
            return Update::none();
        };
        if !self.interaction_enabled || self.drag.is_some() || self.cursor_px == Some(position) {
            return Update::none();
        }
        self.pointer_moved(Some((position.x, position.y)))
    }
}

fn render_2d_task(
    scheduled: ScheduledRequest<RenderRequest2D>,
    session: InteractivePlotSession,
    presented: PresentedLayerSources,
) -> Task<Message> {
    let request_id = scheduled.id();
    let request = scheduled.into_request();
    let incarnation = request.incarnation;
    let change_revision = request.change_revision;
    Task::perform(
        async move {
            // Layered rendering skips the core's full-frame CPU composite, and
            // building the Iced handles here keeps every per-pixel cost off the
            // UI thread.
            session
                .render_layers_stamped(ImageTarget {
                    size_px: request.size_px,
                    scale_factor: request.scale_factor,
                    time_seconds: request.time_seconds,
                })
                .map(|layers| presented.plan(layers))
        },
        move |result| {
            Message(MessageKind::Rendered2D {
                incarnation,
                request_id,
                change_revision,
                result,
            })
        },
    )
}

fn allocate_layer(
    role: LayerRole,
    upload: LayerUpload,
) -> Task<(LayerRole, Result<iced::widget::image::Allocation, String>)> {
    match upload {
        // Arc-identical to the presented layer: keep the existing allocation
        // instead of re-uploading the same pixels.
        LayerUpload::Reuse(allocation) => Task::done((role, Ok(allocation))),
        LayerUpload::New(handle) => iced::widget::image::allocate(handle)
            .map(move |allocation| (role, allocation.map_err(|error| error.to_string()))),
    }
}

fn image_allocation_2d_task(
    incarnation: StateIncarnation,
    request_id: ScheduledRequestId,
    prepared: PreparedLayers,
) -> Task<Message> {
    let PreparedLayers {
        layers,
        base,
        overlay,
    } = prepared;
    let mut tasks = vec![allocate_layer(LayerRole::Base, base)];
    if let Some(overlay) = overlay {
        tasks.push(allocate_layer(LayerRole::Overlay, overlay));
    }
    let mut layers = Some(layers);
    Task::batch(tasks).collect().map(move |allocations| {
        Message(MessageKind::Allocated2D {
            incarnation: incarnation.clone(),
            request_id: request_id.clone(),
            layers: layers.take(),
            allocations,
        })
    })
}

#[cfg(feature = "3d")]
#[derive(Clone)]
struct RenderRequest3D {
    incarnation: StateIncarnation,
    job: BackgroundRenderJob3D,
}

#[cfg(feature = "3d")]
/// Iced-owned retained state for a static or interactive 3D plot.
pub struct Plot3DState {
    session: InteractivePlot3DSession,
    pub(crate) presentation: Presentation,
    pub(crate) interaction_enabled: bool,
    pub(crate) sizing: Sizing,
    pub(crate) fit: ImageFit,
    scale_factor: f32,
    logical_size: (f64, f64),
    scheduler: LatestRequestScheduler<RenderRequest3D>,
    incarnation: StateIncarnation,
    renderer: Arc<Mutex<BackgroundRenderer3D>>,
    pub(crate) presented: Option<PresentedImage>,
    pub(crate) presented_view: Option<ruviz::core::ViewStamp3D>,
    cursor_px: Option<(f32, f32)>,
}

#[cfg(feature = "3d")]
/// Construct a static 3D state.
pub fn static_view_3d<P: TryIntoPlot3DSession>(plot: P) -> ruviz::core::Result<Plot3DState> {
    Plot3DState::static_view(plot)
}

#[cfg(feature = "3d")]
/// Construct an interactive 3D state.
pub fn interactive_3d<P: TryIntoPlot3DSession>(plot: P) -> ruviz::core::Result<Plot3DState> {
    Plot3DState::interactive(plot)
}

#[cfg(feature = "3d")]
impl Plot3DState {
    /// Construct a static image-backed 3D state.
    pub fn static_view<P: TryIntoPlot3DSession>(plot: P) -> ruviz::core::Result<Self> {
        Ok(Self::new(
            plot.try_into_plot3d_session()?,
            Presentation::Static,
        ))
    }

    /// Construct an interactive 3D state with orbit, pan, zoom, picking, and
    /// reset gestures.
    pub fn interactive<P: TryIntoPlot3DSession>(plot: P) -> ruviz::core::Result<Self> {
        Ok(Self::new(
            plot.try_into_plot3d_session()?,
            Presentation::Interactive,
        ))
    }

    fn new(session: InteractivePlot3DSession, presentation: Presentation) -> Self {
        Self {
            session,
            presentation,
            interaction_enabled: presentation == Presentation::Interactive,
            sizing: Sizing::default(),
            fit: ImageFit::Contain,
            scale_factor: 1.0,
            logical_size: Sizing::default().logical_fallback(),
            scheduler: LatestRequestScheduler::default(),
            incarnation: StateIncarnation::new(),
            renderer: Arc::new(Mutex::new(background_renderer())),
            presented: None,
            presented_view: None,
            cursor_px: None,
        }
    }

    /// Consume this state and use all available layout space.
    pub fn fill(mut self) -> Self {
        self.sizing = Sizing::Fill;
        self
    }

    /// Consume this state and request a fixed logical size.
    pub fn fixed(mut self, width: f32, height: f32) -> Self {
        self = self.fixed_pixels(width, height);
        self
    }

    /// Consume this state and request a fixed size in logical Iced pixels.
    pub fn fixed_pixels(mut self, width: f32, height: f32) -> Self {
        self.sizing = valid_fixed(width, height);
        self.logical_size = self.sizing.logical_fallback();
        self
    }

    /// Select image fitting used for drawing and input mapping.
    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }

    /// Set the initial device scale.
    pub fn scale_factor(mut self, scale_factor: f32) -> Self {
        self.scale_factor = sanitize_scale_factor(scale_factor);
        self
    }

    /// Retained 3D session.
    pub const fn session(&self) -> &InteractivePlot3DSession {
        &self.session
    }

    /// Presentation mode selected when this state was constructed.
    pub const fn presentation(&self) -> Presentation {
        self.presentation
    }

    /// Whether orbit, pan, zoom, and picking gestures are currently enabled.
    ///
    /// The context menu remains available while interaction is disabled.
    pub const fn interaction_enabled(&self) -> bool {
        self.interaction_enabled
    }

    /// Enable or disable plot gestures while keeping the context menu available.
    pub fn set_interaction_enabled(&mut self, enabled: bool) -> Update {
        if self.interaction_enabled == enabled {
            return Update::none();
        }
        self.interaction_enabled = enabled;
        let cancelled = self.session.cancel_drag();
        let mut update = Update::with_event(Task::none(), Event::InteractionToggled(enabled));
        if cancelled {
            update.events.push(Event::DragCancelled);
        }
        update
    }

    /// Source alpha mode of the retained frame. Iced receives straight RGBA.
    pub fn source_alpha_mode(&self) -> Option<ruviz::core::AlphaMode> {
        self.presented.as_ref().map(|frame| frame.source_alpha)
    }

    /// Request the initial or a forced redraw.
    pub fn request_render(&mut self) -> Update {
        match self.queue_render() {
            Ok(task) => Update::task(task),
            Err(error) => Update::with_event(Task::none(), Event::Error(error)),
        }
    }

    /// Replace the plot and reset to its camera.
    pub fn set_plot<P: TryIntoPlot3DSession>(&mut self, plot: P) -> Update {
        let result = plot
            .try_into_plot3d_session()
            .and_then(|replacement| self.session.try_replace(replacement));
        self.complete_replacement(result)
    }

    /// Replace the plot while preserving the authoritative camera.
    pub fn set_plot_keep_view<P: TryIntoPlot3DSession>(&mut self, plot: P) -> Update {
        let result = plot
            .try_into_plot3d_session()
            .and_then(|replacement| self.session.replace_keep_camera(replacement));
        self.complete_replacement(result)
    }

    fn complete_replacement(&mut self, result: ruviz::core::Result<()>) -> Update {
        match result {
            Ok(()) => {
                self.incarnation = StateIncarnation::new();
                self.cursor_px = None;
                self.request_render()
            }
            Err(error) => Update::with_event(Task::none(), Event::Error(error)),
        }
    }

    /// 3D sessions are changed by routed messages, so no extra subscription is
    /// required. This method enables a uniform host integration.
    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }

    /// Route one adapter message through the Elm-owned state.
    pub fn update(&mut self, message: Message) -> Update {
        match message.0 {
            MessageKind::Widget3D(event) => self.handle_widget_event(event),
            MessageKind::Rendered3D {
                incarnation,
                request_id,
                result,
            } => self.complete_render(incarnation, request_id, result),
            MessageKind::Allocated3D {
                incarnation,
                request_id,
                rendered,
                source_alpha,
                allocation,
            } => self.complete_allocation(
                incarnation,
                request_id,
                rendered,
                source_alpha,
                allocation,
            ),
            MessageKind::ContextActionCompleted(result) => complete_context_action(result),
            _ => Update::none(),
        }
    }

    fn handle_widget_event(&mut self, event: WidgetEvent) -> Update {
        let result = match event {
            WidgetEvent::BoundsChanged { logical_size } => {
                let normalized = (
                    finite_dimension(logical_size.0),
                    finite_dimension(logical_size.1),
                );
                if self.logical_size == normalized {
                    return Update::none();
                }
                self.logical_size = normalized;
                self.resize_and_render()
            }
            WidgetEvent::ScaleFactorChanged(scale_factor) => {
                let scale_factor = sanitize_scale_factor(scale_factor);
                if self.scale_factor.to_bits() == scale_factor.to_bits() {
                    return Update::none();
                }
                self.scale_factor = scale_factor;
                self.resize_and_render()
            }
            WidgetEvent::ContextMenuAction(action) => return self.context_menu_action(action),
            _ if !self.interaction_enabled => return Update::none(),
            WidgetEvent::PointerMoved(position) => {
                let Some((x, y)) = position else {
                    self.cursor_px = None;
                    return Update::none();
                };
                let (x, y) = (x as f32, y as f32);
                self.cursor_px = Some((x, y));
                self.apply_3d_input(InputEvent3D::PointerMove { x, y })
            }
            WidgetEvent::HoverCoalesced {
                position_px: (x, y),
            } => {
                // 3D has no hover overlay; only the cursor position is tracked.
                self.cursor_px = Some((x as f32, y as f32));
                return Update::none();
            }
            WidgetEvent::PointerPressed {
                position_px: (x, y),
                button,
            } => {
                let (x, y) = (x as f32, y as f32);
                self.cursor_px = Some((x, y));
                self.apply_3d_input(InputEvent3D::PointerDown {
                    x,
                    y,
                    button: pointer_button_3d(button),
                })
            }
            WidgetEvent::PointerReleased {
                position_px: (x, y),
                button,
            } => self.apply_3d_input(InputEvent3D::PointerUp {
                x: x as f32,
                y: y as f32,
                button: pointer_button_3d(button),
            }),
            WidgetEvent::DoubleClick {
                position_px: (x, y),
            } => {
                self.session.cancel_drag();
                let outcome = self.apply_3d_input(InputEvent3D::DoubleClick {
                    x: x as f32,
                    y: y as f32,
                    button: PointerButton3D::Left,
                });
                return match outcome {
                    Ok(update) => merge_event(update, Event::CameraReset),
                    Err(error) => Update::with_event(Task::none(), Event::Error(error)),
                };
            }
            WidgetEvent::Wheel { delta_y, .. } => {
                self.apply_3d_input(InputEvent3D::Wheel { delta_y })
            }
            WidgetEvent::Escape => {
                if self.session.cancel_drag() {
                    return Update::with_event(Task::none(), Event::DragCancelled);
                }
                let outcome = self.apply_3d_input(InputEvent3D::Escape);
                return match outcome {
                    Ok(update) => merge_event(update, Event::CameraReset),
                    Err(error) => Update::with_event(Task::none(), Event::Error(error)),
                };
            }
            WidgetEvent::CancelDrag => {
                if self.session.cancel_drag() {
                    return Update::with_event(Task::none(), Event::DragCancelled);
                }
                return Update::none();
            }
        };
        match result {
            Ok(update) => update,
            Err(error) => Update::with_event(Task::none(), Event::Error(error)),
        }
    }

    fn context_menu_action(&mut self, action: PlotContextMenuAction) -> Update {
        let before = self.session.camera();
        let cancelled_drag = self.session.cancel_drag();
        let result = match action {
            PlotContextMenuAction::ResetView => self.session.reset_view(),
            PlotContextMenuAction::FitToContent => self.session.fit_to_content(),
            PlotContextMenuAction::CameraView(view) => self.session.apply_camera_view(view),
            PlotContextMenuAction::SaveImage => {
                return self.presented.as_ref().map_or_else(Update::none, |image| {
                    let file_name = self
                        .session
                        .title()
                        .map(default_export_filename)
                        .unwrap_or_else(|| "ruviz-3d-plot.png".to_owned());
                    Update::task(save_presented_image_task(image, &file_name))
                });
            }
            PlotContextMenuAction::CopyImage => {
                return self.presented.as_ref().map_or_else(Update::none, |image| {
                    Update::task(copy_presented_image_task(image))
                });
            }
            PlotContextMenuAction::ToggleInteraction => {
                return self.set_interaction_enabled(!self.interaction_enabled);
            }
            _ => return Update::none(),
        };
        match result {
            Ok(()) if self.session.camera() != before => {
                let event = if action == PlotContextMenuAction::ResetView {
                    Event::CameraReset
                } else {
                    Event::CameraChanged(self.session.camera_snapshot())
                };
                match self.queue_render() {
                    Ok(task) => Update::with_event(task, event),
                    Err(error) => Update::with_event(Task::none(), Event::Error(error)),
                }
            }
            Ok(()) if cancelled_drag => Update::with_event(Task::none(), Event::DragCancelled),
            Ok(()) => Update::none(),
            Err(error) => Update::with_event(Task::none(), Event::Error(error)),
        }
    }

    fn resize_and_render(&mut self) -> Result<Update, PlottingError> {
        let (width, height) =
            physical_backing_size(self.logical_size.0, self.logical_size.1, self.scale_factor);
        self.session.resize(width, height, self.scale_factor)?;
        Ok(Update::task(self.queue_render()?))
    }

    fn apply_3d_input(&mut self, input: InputEvent3D) -> Result<Update, PlottingError> {
        let outcome = self.session.handle_input(input)?;
        let task = if outcome.request_redraw {
            self.queue_render()?
        } else {
            Task::none()
        };
        match outcome.picked {
            Some(hit) => Ok(Update::with_event(task, Event::Picked3D(hit))),
            None if outcome.camera_changed => Ok(Update::with_event(
                task,
                Event::CameraChanged(self.session.camera_snapshot()),
            )),
            None => Ok(Update::task(task)),
        }
    }

    fn queue_render(&mut self) -> Result<Task<Message>, PlottingError> {
        let (width, height) =
            physical_backing_size(self.logical_size.0, self.logical_size.1, self.scale_factor);
        self.session.resize(width, height, self.scale_factor)?;
        let job = self.session.background_render_job()?;
        let request = RenderRequest3D {
            incarnation: self.incarnation.clone(),
            job,
        };
        Ok(match self.scheduler.request(request) {
            Some(request) => render_3d_task(request, Arc::clone(&self.renderer)),
            None => Task::none(),
        })
    }

    fn complete_render(
        &mut self,
        incarnation: StateIncarnation,
        request_id: ScheduledRequestId,
        result: Result<Rendered3DFrame, PlottingError>,
    ) -> Update {
        let Some(completion) = self.scheduler.complete(request_id.clone()) else {
            return Update::none();
        };
        let next_task = match completion.next {
            Some(next) => render_3d_task(next, Arc::clone(&self.renderer)),
            None => Task::none(),
        };
        if !completion.install || incarnation != self.incarnation {
            return Update::task(next_task);
        }
        match result {
            Ok(frame) => match self.session.classify_render(frame.rendered) {
                BackgroundRenderOutcome3D::Current(rendered) => {
                    let source_alpha = rendered.image.alpha_mode();
                    let allocate = image_allocation_3d_task(
                        incarnation,
                        request_id,
                        rendered,
                        frame.handle,
                        source_alpha,
                    );
                    Update::task(Task::batch([next_task, allocate]))
                }
                BackgroundRenderOutcome3D::Superseded { .. } => Update::task(next_task),
            },
            Err(error) if error.is_render_superseded() => Update::task(next_task),
            Err(error) => Update::with_event(next_task, Event::Error(error)),
        }
    }

    fn complete_allocation(
        &mut self,
        incarnation: StateIncarnation,
        _request_id: ScheduledRequestId,
        rendered: Option<ruviz::core::RenderedImage3D>,
        source_alpha: ruviz::core::AlphaMode,
        allocation: Result<iced::widget::image::Allocation, String>,
    ) -> Update {
        let Some(rendered) = rendered else {
            return Update::none();
        };
        if incarnation != self.incarnation
            || !matches!(
                self.session.classify_render(rendered.clone()),
                BackgroundRenderOutcome3D::Current(_)
            )
        {
            return Update::none();
        }
        match allocation {
            Ok(allocation) => {
                self.presented = Some(PresentedImage {
                    allocation,
                    // The 3D pipeline produces a single composed frame.
                    overlay: None,
                    size_px: (rendered.image.width, rendered.image.height),
                    source_alpha,
                });
                self.presented_view = Some(rendered.stamp.view());
                Update::none()
            }
            Err(error) => Update::with_event(
                Task::none(),
                Event::Error(PlottingError::RenderError(format!(
                    "Iced image allocation failed: {error}"
                ))),
            ),
        }
    }
}

#[cfg(feature = "3d")]
fn render_3d_task(
    scheduled: ScheduledRequest<RenderRequest3D>,
    renderer: Arc<Mutex<BackgroundRenderer3D>>,
) -> Task<Message> {
    let request_id = scheduled.id();
    let request = scheduled.into_request();
    let incarnation = request.incarnation;
    Task::perform(
        async move {
            renderer
                .lock()
                .map_err(|_| {
                    PlottingError::RenderError(
                        "ruviz-iced 3D worker renderer lock was poisoned".to_owned(),
                    )
                })
                .and_then(|mut renderer| renderer.render(request.job))
                // Build the Iced handle here so its full-frame copy never runs
                // on the UI thread inside `update`.
                .map(|rendered| Rendered3DFrame {
                    handle: iced_handle_owned(&rendered.image).0,
                    rendered,
                })
        },
        move |result| {
            Message(MessageKind::Rendered3D {
                incarnation,
                request_id,
                result,
            })
        },
    )
}

#[cfg(feature = "3d")]
fn image_allocation_3d_task(
    incarnation: StateIncarnation,
    request_id: ScheduledRequestId,
    rendered: ruviz::core::RenderedImage3D,
    handle: iced::widget::image::Handle,
    source_alpha: ruviz::core::AlphaMode,
) -> Task<Message> {
    let mut rendered = Some(rendered);
    iced::widget::image::allocate(handle).map(move |allocation| {
        Message(MessageKind::Allocated3D {
            incarnation: incarnation.clone(),
            request_id: request_id.clone(),
            rendered: rendered.take(),
            source_alpha,
            allocation: allocation.map_err(|error| error.to_string()),
        })
    })
}

#[cfg(feature = "3d")]
fn pointer_button_3d(button: PointerButton) -> PointerButton3D {
    match button {
        PointerButton::Left => PointerButton3D::Left,
        PointerButton::Middle => PointerButton3D::Middle,
        PointerButton::Right => PointerButton3D::Right,
    }
}

#[cfg(feature = "3d")]
fn merge_event(mut update: Update, event: Event) -> Update {
    update.events.push(event);
    update
}

#[cfg(feature = "3d")]
fn background_renderer() -> BackgroundRenderer3D {
    #[cfg(feature = "3d-gpu")]
    {
        BackgroundRenderer3D::new(BackgroundRenderBackend3D::GpuReadback)
    }
    #[cfg(not(feature = "3d-gpu"))]
    {
        BackgroundRenderer3D::new(BackgroundRenderBackend3D::Cpu)
    }
}

fn complete_context_action(result: Result<ContextActionCompletion, PlottingError>) -> Update {
    match result {
        Ok(ContextActionCompletion::ImageCopied) => {
            Update::with_event(Task::none(), Event::ImageCopied)
        }
        Ok(ContextActionCompletion::ImageSaved) => {
            Update::with_event(Task::none(), Event::ImageSaved)
        }
        Ok(ContextActionCompletion::SaveCancelled) => Update::none(),
        Err(error) => Update::with_event(Task::none(), Event::Error(error)),
    }
}

fn copy_presented_image_task(image: &PresentedImage) -> Task<Message> {
    let source = image.export_source();
    Task::perform(
        async move {
            let (width, height, pixels) = compose_export(source)?;
            let mut clipboard = arboard::Clipboard::new().map_err(|error| {
                PlottingError::SystemError(format!("clipboard unavailable: {error}"))
            })?;
            clipboard
                .set_image(arboard::ImageData {
                    width: width as usize,
                    height: height as usize,
                    bytes: Cow::Owned(pixels),
                })
                .map_err(|error| {
                    PlottingError::SystemError(format!("failed to copy the plot image: {error}"))
                })?;
            Ok(ContextActionCompletion::ImageCopied)
        },
        |result| Message(MessageKind::ContextActionCompleted(result)),
    )
}

fn save_presented_image_task(image: &PresentedImage, file_name: &str) -> Task<Message> {
    let source = image.export_source();
    let file_name = file_name.to_owned();
    Task::perform(
        async move {
            let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter("PNG image", &["png"])
                .set_file_name(&file_name)
                .save_file()
                .await
            else {
                return Ok(ContextActionCompletion::SaveCancelled);
            };
            let (width, height, pixels) = compose_export(source)?;
            let image = ruviz::core::Image::from_straight_rgba(width, height, pixels);
            ruviz::export::write_rgba_png_atomic(file.path(), &image)?;
            Ok(ContextActionCompletion::ImageSaved)
        },
        |result| Message(MessageKind::ContextActionCompleted(result)),
    )
}

/// Flatten the presented base and overlay layers for export.
fn compose_export(
    source: Result<ExportSource, PlottingError>,
) -> Result<(u32, u32, Vec<u8>), PlottingError> {
    source?.compose()
}

#[cfg(feature = "3d")]
fn default_export_filename(title: &str) -> String {
    let mut slug = title
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    slug = slug.trim_matches('-').to_owned();
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    if slug.is_empty() {
        "ruviz-3d-plot.png".to_owned()
    } else {
        format!("{slug}.png")
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

fn valid_fixed(width: f32, height: f32) -> Sizing {
    Sizing::Fixed {
        width: if width.is_finite() && width > 0.0 {
            width
        } else {
            1.0
        },
        height: if height.is_finite() && height > 0.0 {
            height
        } else {
            1.0
        },
    }
}

fn finite_dimension(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        1.0
    }
}

fn viewport_point((x, y): (f64, f64)) -> ViewportPoint {
    ViewportPoint::new(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviz::data::observable::Observable;
    use ruviz::prelude::Plot;

    /// Deterministic software renderer for widget tests.
    ///
    /// The dev-dependency enables `wgpu`, so `iced::Renderer` is the fallback
    /// renderer. Requesting the `tiny-skia` backend by name makes the wgpu
    /// branch decline before it touches a GPU adapter.
    fn test_renderer() -> iced::Renderer {
        use iced::advanced::renderer::Headless;

        futures::executor::block_on(iced::Renderer::new(
            iced::Font::default(),
            iced::Pixels(16.0),
            Some("tiny-skia"),
        ))
        .expect("tiny-skia test renderer")
    }

    fn task_output(task: Task<Message>) -> Message {
        let mut actions =
            iced_runtime::task::into_stream(task).expect("test task should produce one message");
        let renderer = test_renderer();
        futures::executor::block_on(async move {
            loop {
                match actions
                    .next()
                    .await
                    .expect("test task ended without output")
                {
                    iced_runtime::Action::Output(message) => return message,
                    iced_runtime::Action::Image(iced_runtime::image::Action::Allocate(
                        handle,
                        sender,
                    )) => {
                        let allocation =
                            iced::advanced::image::Renderer::load_image(&renderer, &handle);
                        let _ = sender.send(allocation);
                    }
                    action => panic!("unexpected Iced test action: {action:?}"),
                }
            }
        })
    }

    fn install_render_update(state: &mut PlotState, update: Update) {
        let rendered = task_output(update.into_task());
        let allocation = state.update(rendered);
        assert!(allocation.events().is_empty());
        let allocated = task_output(allocation.into_task());
        let installed = state.update(allocated);
        assert!(installed.events().is_empty());
        assert!(state.presented.is_some());
        assert!(
            state
                .presented_stamp
                .is_some_and(|stamp| state.session.is_render_stamp_current(stamp))
        );
    }

    fn layered_state() -> PlotState {
        PlotState::interactive(Plot::new().line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0]))
            .fixed_pixels(320.0, 240.0)
    }

    fn layered_target() -> ImageTarget {
        ImageTarget {
            size_px: (320, 240),
            scale_factor: 1.0,
            time_seconds: 0.0,
        }
    }

    fn show_tooltip(state: &PlotState) {
        state.session.apply_input(PlotInputEvent::ShowTooltip {
            content: "layered".to_owned(),
            position_px: ViewportPoint::new(120.0, 80.0),
        });
    }

    #[test]
    fn overlay_only_redraw_reuses_the_base_allocation_and_stacks_the_overlay() {
        let mut state = layered_state();
        let initial = state.request_render();
        install_render_update(&mut state, initial);
        let base_id = state.presented.as_ref().unwrap().handle().id();
        assert!(
            state.presented.as_ref().unwrap().overlay.is_none(),
            "an idle plot must not present an overlay layer"
        );

        show_tooltip(&state);
        let update = state.update(Message(MessageKind::Changed2D));
        install_render_update(&mut state, update);

        let presented = state.presented.as_ref().unwrap();
        assert_eq!(
            presented.handle().id(),
            base_id,
            "an overlay-only redraw must reuse the base allocation instead of re-uploading it"
        );
        assert!(
            presented.overlay.is_some(),
            "the overlay must be presented as its own stacked layer"
        );

        // A base change must still replace the base allocation.
        state.session.apply_input(PlotInputEvent::Zoom {
            factor: 2.0,
            center_px: ViewportPoint::new(160.0, 120.0),
        });
        state.note_direct_change();
        let update = Update::task(state.queue_render());
        install_render_update(&mut state, update);
        assert_ne!(state.presented.as_ref().unwrap().handle().id(), base_id);
    }

    #[test]
    fn export_composes_the_presented_layers_like_the_composed_render_path() {
        let mut state = layered_state();
        let initial = state.request_render();
        install_render_update(&mut state, initial);
        show_tooltip(&state);
        let update = state.update(Message(MessageKind::Changed2D));
        install_render_update(&mut state, update);
        assert!(state.presented.as_ref().unwrap().overlay.is_some());

        let (width, height, pixels) = state
            .presented
            .as_ref()
            .unwrap()
            .export_source()
            .expect("presented layers must be exportable")
            .compose()
            .expect("presented layers must compose");
        let composed = state
            .session
            .render_to_image_stamped(layered_target())
            .expect("composed reference frame");
        assert_eq!(
            (width, height),
            (composed.frame.image.width, composed.frame.image.height)
        );
        assert_eq!(
            pixels, composed.frame.image.pixels,
            "Save PNG and Copy image must still see the flattened frame"
        );
    }

    #[test]
    fn coalesced_hover_replays_the_newest_position_on_the_next_current_frame() {
        let mut state = layered_state();
        let initial = state.request_render();
        install_render_update(&mut state, initial);

        // A render is in flight, so the widget only reports positions.
        let pending = Update::task(state.queue_render());
        let stale = state.handle_widget_event(WidgetEvent::HoverCoalesced {
            position_px: (100.0, 90.0),
        });
        assert!(
            stale.events().is_empty(),
            "a coalesced hover must not derive events from stale geometry"
        );
        let newest = state.handle_widget_event(WidgetEvent::HoverCoalesced {
            position_px: (150.0, 120.0),
        });
        assert!(newest.events().is_empty());
        assert!(state.cursor_px.is_none());

        let rendered = task_output(pending.into_task());
        let allocation = state.update(rendered);
        let allocated = task_output(allocation.into_task());
        let installed = state.update(allocated);

        let cursor = state
            .cursor_px
            .expect("the coalesced hover must be applied");
        assert_eq!(
            (cursor.x, cursor.y),
            (150.0, 120.0),
            "the newest coalesced position must win over the one that arrived first"
        );
        assert!(matches!(installed.event(), Some(Event::Hovered2D(_))));
        assert!(state.pending_hover.is_none());
    }

    #[test]
    fn fixed_size_is_sanitized() {
        let state = PlotState::interactive(Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]))
            .fixed(f32::NAN, -10.0);
        assert_eq!(
            state.sizing(),
            Sizing::Fixed {
                width: 1.0,
                height: 1.0
            }
        );
    }

    #[test]
    fn keep_view_restores_customized_visible_bounds_on_replacement() {
        let mut state =
            PlotState::interactive(Plot::new().line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0]));
        state
            .session
            .render_to_image(ImageTarget {
                size_px: (400, 300),
                scale_factor: 1.0,
                time_seconds: 0.0,
            })
            .unwrap();
        state.session.apply_input(PlotInputEvent::Zoom {
            factor: 2.0,
            center_px: ViewportPoint::new(200.0, 150.0),
        });
        let visible = state.session.view_bounds_snapshot().visible_bounds;
        let _ = state.set_plot_keep_view(Plot::new().line(&[0.0, 1.0, 2.0], &[2.0, 3.0, 4.0]));
        state
            .session
            .render_to_image(ImageTarget {
                size_px: (400, 300),
                scale_factor: 1.0,
                time_seconds: 0.0,
            })
            .unwrap();
        let restored = state.session.view_bounds_snapshot().visible_bounds;
        assert!((restored.min.x - visible.min.x).abs() < 1e-12);
        assert!((restored.min.y - visible.min.y).abs() < 1e-12);
        assert!((restored.max.x - visible.max.x).abs() < 1e-12);
        assert!((restored.max.y - visible.max.y).abs() < 1e-12);
    }

    #[test]
    fn keep_view_uses_replacement_bounds_when_old_view_is_untouched() {
        let mut state = PlotState::interactive(
            Plot::new()
                .line(&[0.0, 10.0], &[0.0, 10.0])
                .xlim(0.0, 10.0)
                .ylim(0.0, 10.0),
        );
        let old_base = state.session.view_bounds_snapshot().base_bounds;
        let replacement = Plot::new()
            .line(&[100.0, 200.0], &[-5.0, 5.0])
            .xlim(100.0, 200.0)
            .ylim(-5.0, 5.0);

        let _ = state.set_plot_keep_view(replacement);
        state
            .session
            .render_to_image(ImageTarget {
                size_px: (400, 300),
                scale_factor: 1.0,
                time_seconds: 0.0,
            })
            .unwrap();
        let replacement_view = state.session.view_bounds_snapshot();
        assert_eq!(
            replacement_view.visible_bounds,
            replacement_view.base_bounds
        );
        assert_ne!(replacement_view.base_bounds, old_base);
    }

    #[test]
    fn release_outside_cancels_retained_drag() {
        let mut state = PlotState::interactive(Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]));
        let _ = state.handle_widget_event(WidgetEvent::PointerPressed {
            position_px: (20.0, 20.0),
            button: PointerButton::Left,
        });
        assert!(state.drag.is_some());
        let update = state.handle_widget_event(WidgetEvent::CancelDrag);
        assert!(matches!(update.event(), Some(Event::DragCancelled)));
        assert!(state.drag.is_none());
    }

    #[test]
    fn two_d_captured_pan_continues_until_release_while_redraw_is_pending() {
        let mut state = PlotState::interactive(Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]));
        state
            .session
            .render_to_image(ImageTarget {
                size_px: (320, 240),
                scale_factor: 1.0,
                time_seconds: 0.0,
            })
            .unwrap();
        let _ = state.handle_widget_event(WidgetEvent::PointerPressed {
            position_px: (20.0, 20.0),
            button: PointerButton::Left,
        });
        let first = state.handle_widget_event(WidgetEvent::PointerMoved(Some((40.0, 30.0))));
        let after_first = state.session.view_bounds_snapshot().visible_bounds;
        assert!(matches!(first.event(), Some(Event::ViewChanged)));

        let second = state.handle_widget_event(WidgetEvent::PointerMoved(Some((55.0, 45.0))));
        assert!(matches!(second.event(), Some(Event::ViewChanged)));
        assert_ne!(
            state.session.view_bounds_snapshot().visible_bounds,
            after_first
        );

        let _ = state.handle_widget_event(WidgetEvent::PointerReleased {
            position_px: (55.0, 45.0),
            button: PointerButton::Left,
        });
        assert!(state.drag.is_none());
    }

    #[test]
    fn escape_and_mismatched_release_cancel_right_brush_state() {
        let mut state = PlotState::interactive(Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]));
        let _ = state.handle_widget_event(WidgetEvent::PointerPressed {
            position_px: (20.0, 20.0),
            button: PointerButton::Right,
        });
        assert!(state.session.cancel_interaction());

        let _ = state.handle_widget_event(WidgetEvent::PointerPressed {
            position_px: (20.0, 20.0),
            button: PointerButton::Right,
        });
        let update = state.handle_widget_event(WidgetEvent::Escape);
        assert!(matches!(update.event(), Some(Event::DragCancelled)));
        assert!(!state.session.cancel_interaction());

        let _ = state.handle_widget_event(WidgetEvent::PointerPressed {
            position_px: (20.0, 20.0),
            button: PointerButton::Right,
        });
        let update = state.handle_widget_event(WidgetEvent::PointerReleased {
            position_px: (20.0, 20.0),
            button: PointerButton::Left,
        });
        assert!(matches!(update.event(), Some(Event::DragCancelled)));
        assert!(!state.session.cancel_interaction());
    }

    #[test]
    fn static_state_ignores_interaction() {
        let mut state = PlotState::static_view(Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]));
        assert!(!state.interaction_enabled());
        let before = state.session.change_revision();
        let _ = state.handle_widget_event(WidgetEvent::Wheel {
            position_px: (100.0, 100.0),
            delta_y: 120.0,
        });
        assert_eq!(state.session.change_revision(), before);

        let toggled = state.context_menu_action(PlotContextMenuAction::ToggleInteraction);
        assert!(state.interaction_enabled());
        assert!(matches!(
            toggled.event(),
            Some(Event::InteractionToggled(true))
        ));
        let wheel = state.handle_widget_event(WidgetEvent::Wheel {
            position_px: (100.0, 100.0),
            delta_y: 120.0,
        });
        assert!(matches!(wheel.event(), Some(Event::ViewChanged)));
    }

    #[test]
    fn two_d_reset_and_fit_context_actions_restore_natural_bounds() {
        for action in [
            PlotContextMenuAction::ResetView,
            PlotContextMenuAction::FitToContent,
        ] {
            let mut state =
                PlotState::interactive(Plot::new().line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0]));
            state
                .session
                .render_to_image(ImageTarget {
                    size_px: (320, 240),
                    scale_factor: 1.0,
                    time_seconds: 0.0,
                })
                .unwrap();
            let natural = state.session.view_bounds_snapshot().base_bounds;
            state.session.apply_input(PlotInputEvent::Zoom {
                factor: 2.0,
                center_px: ViewportPoint::new(160.0, 120.0),
            });
            assert_ne!(state.session.view_bounds_snapshot().visible_bounds, natural);

            let update = state.context_menu_action(action);
            assert_eq!(state.session.view_bounds_snapshot().visible_bounds, natural);
            assert!(matches!(
                (action, update.event()),
                (PlotContextMenuAction::ResetView, Some(Event::ViewReset))
                    | (
                        PlotContextMenuAction::FitToContent,
                        Some(Event::ViewChanged)
                    )
            ));
        }
    }

    #[test]
    fn stale_pointer_leave_still_clears_hover_overlay() {
        let mut state = PlotState::interactive(Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]));
        state
            .session
            .render_to_image(ImageTarget {
                size_px: (320, 240),
                scale_factor: 1.0,
                time_seconds: 0.0,
            })
            .unwrap();
        let _ = state.handle_widget_event(WidgetEvent::PointerMoved(Some((160.0, 120.0))));
        state.session.invalidate();

        let update = state.handle_widget_event(WidgetEvent::PointerMoved(None));
        assert!(matches!(update.event(), Some(Event::Hovered2D(None))));
        assert!(state.drag.is_none());
    }

    #[test]
    fn observable_change_wakes_schedules_and_installs_reactive_redraw() {
        let values = Observable::new(vec![0.0, 1.0]);
        let plot: Plot = Plot::new()
            .line_source(vec![0.0, 1.0], values.clone())
            .into();
        let mut state = PlotState::interactive(plot);
        let initial = state.request_render();
        install_render_update(&mut state, initial);
        let initial_stamp = state.presented_stamp;
        while state.wake.data.receiver.try_recv().is_ok() {}
        let previous_generation = state.scheduler.latest_generation();

        values.set(vec![2.0, 3.0]);
        state
            .wake
            .data
            .receiver
            .try_recv()
            .expect("observable change should wake the Iced subscription");
        let update = state.update(Message(MessageKind::Changed2D));
        assert!(!state.scheduler.is_idle());
        assert!(state.scheduler.latest_generation() > previous_generation);
        install_render_update(&mut state, update);

        assert_ne!(state.presented_stamp, initial_stamp);
    }

    #[test]
    fn independent_states_reject_cross_routed_completion_and_input() {
        let mut first = PlotState::interactive(
            Plot::new()
                .line(&[0.0, 1.0], &[0.0, 1.0])
                .xlim(0.0, 1.0)
                .ylim(0.0, 1.0),
        );
        let mut second = PlotState::interactive(
            Plot::new()
                .line(&[10.0, 20.0], &[10.0, 20.0])
                .xlim(10.0, 20.0)
                .ylim(10.0, 20.0),
        );
        let first_initial = first.request_render();
        install_render_update(&mut first, first_initial);
        let second_initial = second.request_render();
        install_render_update(&mut second, second_initial);
        let second_stamp = second.presented_stamp;
        let second_bounds = second.session.view_bounds_snapshot().visible_bounds;

        let first_update = first.handle_widget_event(WidgetEvent::Wheel {
            position_px: (320.0, 240.0),
            delta_y: 80.0,
        });
        assert_eq!(
            second.session.view_bounds_snapshot().visible_bounds,
            second_bounds
        );
        let rendered_for_first = task_output(first_update.into_task());
        assert!(
            second
                .update(rendered_for_first.clone())
                .events()
                .is_empty()
        );
        assert_eq!(second.presented_stamp, second_stamp);

        let allocation_for_first = first.update(rendered_for_first);
        let allocated_for_first = task_output(allocation_for_first.into_task());
        assert!(
            second
                .update(allocated_for_first.clone())
                .events()
                .is_empty()
        );
        assert_eq!(second.presented_stamp, second_stamp);
        assert!(first.update(allocated_for_first).events().is_empty());
        assert_ne!(first.presented_stamp, second.presented_stamp);
    }

    #[test]
    fn iced_scheduler_coalesces_replacement_and_resize_to_newest_request() {
        let mut scheduler = LatestRequestScheduler::default();
        let revision = PlotState::static_view(Plot::new())
            .session
            .change_revision();
        let first_incarnation = StateIncarnation::new();
        let second_incarnation = StateIncarnation::new();
        let newest_incarnation = StateIncarnation::new();
        let first = scheduler
            .request(RenderRequest2D {
                incarnation: first_incarnation,
                change_revision: revision,
                size_px: (320, 240),
                scale_factor: 1.0,
                time_seconds: 0.0,
            })
            .unwrap();
        assert!(
            scheduler
                .request(RenderRequest2D {
                    incarnation: second_incarnation,
                    change_revision: revision,
                    size_px: (400, 300),
                    scale_factor: 1.25,
                    time_seconds: 0.0,
                })
                .is_none()
        );
        assert!(
            scheduler
                .request(RenderRequest2D {
                    incarnation: newest_incarnation.clone(),
                    change_revision: revision,
                    size_px: (800, 600),
                    scale_factor: 2.0,
                    time_seconds: 1.0,
                })
                .is_none()
        );

        let completion = scheduler.complete(first.id()).unwrap();
        assert!(!completion.install);
        let newest = completion.next.unwrap();
        assert_eq!(newest.request().incarnation, newest_incarnation);
        assert_eq!(newest.request().size_px, (800, 600));
        assert_eq!(newest.request().scale_factor, 2.0);
        assert_eq!(newest.request().time_seconds, 1.0);
        assert!(scheduler.complete(newest.id()).unwrap().install);
    }

    #[test]
    fn duplicate_allocation_emission_is_ignored_without_aliasing_incarnations() {
        let mut state = PlotState::interactive(Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]));
        let current = state.incarnation.clone();
        assert_eq!(current, state.incarnation);
        assert_ne!(current, StateIncarnation::new());

        let mut scheduler = LatestRequestScheduler::default();
        let request = scheduler
            .request(RenderRequest2D {
                incarnation: current.clone(),
                change_revision: state.session.change_revision(),
                size_px: (1, 1),
                scale_factor: 1.0,
                time_seconds: 0.0,
            })
            .unwrap();
        let update = state.complete_allocation(
            current,
            request.id(),
            None,
            vec![(
                LayerRole::Base,
                Err("duplicate completion must be ignored".to_owned()),
            )],
        );
        assert!(update.events().is_empty());
    }

    #[test]
    fn obsolete_reactive_render_error_is_suppressed_by_request_revision() {
        let mut state = PlotState::interactive(Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]));
        let request_revision = state.session.change_revision();
        let request = state
            .scheduler
            .request(RenderRequest2D {
                incarnation: state.incarnation.clone(),
                change_revision: request_revision,
                size_px: (320, 240),
                scale_factor: 1.0,
                time_seconds: 0.0,
            })
            .unwrap();

        state.session.invalidate();
        assert_ne!(state.session.change_revision(), request_revision);
        let update = state.complete_render(
            state.incarnation.clone(),
            request.id(),
            request_revision,
            Err(PlottingError::RenderError(
                "obsolete reactive render failure".to_owned(),
            )),
        );
        assert!(update.events().is_empty());
    }

    #[test]
    fn replacement_session_rejects_old_render_stamp_before_allocation() {
        let mut state = PlotState::interactive(Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]));
        let old_frame = state
            .session
            .render_to_image_stamped(ImageTarget {
                size_px: (320, 240),
                scale_factor: 1.0,
                time_seconds: 0.0,
            })
            .unwrap();
        assert!(
            state
                .session
                .is_render_stamp_current(old_frame.render_stamp())
        );
        let old_incarnation = state.incarnation.clone();
        let _ = state.set_plot(Plot::new().line(&[0.0, 1.0], &[2.0, 3.0]));
        assert_ne!(state.incarnation, old_incarnation);
        assert!(
            !state
                .session
                .is_render_stamp_current(old_frame.render_stamp())
        );
    }

    #[test]
    fn raw_iced_handle_receives_canonical_straight_alpha() {
        let image = Arc::new(ruviz::core::Image::from_premultiplied_rgba(
            1,
            1,
            vec![64, 32, 16, 128],
        ));
        assert_eq!(image.alpha_mode(), ruviz::core::AlphaMode::Straight);
        let handle = crate::iced_handle(&image);
        match handle {
            iced::widget::image::Handle::Rgba {
                width,
                height,
                pixels,
                ..
            } => {
                assert_eq!((width, height), (1, 1));
                assert_eq!(pixels.as_ref(), &[128, 64, 32, 128]);
            }
            _ => panic!("expected raw RGBA handle"),
        }
    }

    #[cfg(feature = "3d")]
    #[test]
    fn three_d_global_cancel_clears_core_drag() {
        let mut state =
            Plot3DState::interactive(ruviz::scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0]))
                .unwrap();
        let _ = state.handle_widget_event(WidgetEvent::PointerPressed {
            position_px: (20.0, 20.0),
            button: PointerButton::Left,
        });
        assert!(state.session.is_drag_active());
        let update = state.handle_widget_event(WidgetEvent::CancelDrag);
        assert!(matches!(update.event(), Some(Event::DragCancelled)));
        assert!(!state.session.is_drag_active());
    }

    #[cfg(feature = "3d")]
    #[test]
    fn three_d_orbit_reports_camera_change_and_escape_cancels_active_drag() {
        let mut state =
            Plot3DState::interactive(ruviz::scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0]))
                .unwrap();
        let _ = state.handle_widget_event(WidgetEvent::PointerPressed {
            position_px: (20.0, 20.0),
            button: PointerButton::Left,
        });
        let update = state.handle_widget_event(WidgetEvent::PointerMoved(Some((40.0, 35.0))));
        assert!(matches!(update.event(), Some(Event::CameraChanged(_))));
        let after_first = state.session.camera_snapshot();
        let update = state.handle_widget_event(WidgetEvent::PointerMoved(Some((55.0, 45.0))));
        assert!(matches!(update.event(), Some(Event::CameraChanged(_))));
        assert_ne!(state.session.camera_snapshot().camera, after_first.camera);
        let _ = state.handle_widget_event(WidgetEvent::PointerReleased {
            position_px: (55.0, 45.0),
            button: PointerButton::Left,
        });
        assert!(!state.session.is_drag_active());

        let _ = state.handle_widget_event(WidgetEvent::PointerPressed {
            position_px: (30.0, 30.0),
            button: PointerButton::Left,
        });
        let update = state.handle_widget_event(WidgetEvent::Escape);
        assert!(matches!(update.event(), Some(Event::DragCancelled)));
        assert!(!state.session.is_drag_active());
    }

    #[cfg(feature = "3d")]
    #[test]
    fn three_d_context_views_fit_and_reset_share_the_authoritative_camera() {
        let mut state =
            Plot3DState::interactive(ruviz::scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0]))
                .unwrap();
        let initial = state.session.camera();

        let view_update = state.context_menu_action(PlotContextMenuAction::CameraView(
            ruviz::core::CameraView3D::Front,
        ));
        let front = state.session.camera();
        assert_eq!(front.get_azimuth_deg(), -90.0);
        assert_eq!(front.get_elevation_deg(), 0.0);
        assert!(matches!(view_update.event(), Some(Event::CameraChanged(_))));
        assert!(
            state
                .context_menu_action(PlotContextMenuAction::CameraView(
                    ruviz::core::CameraView3D::Front,
                ))
                .events()
                .is_empty(),
            "an already-active camera view must be a no-op"
        );

        state
            .session
            .set_camera(
                front
                    .zoom(3.0)
                    .look_at(ruviz::core::Point3D::new(0.2, 0.3, 0.4)),
            )
            .unwrap();
        let fit_update = state.context_menu_action(PlotContextMenuAction::FitToContent);
        let fitted = state.session.camera();
        assert_eq!(fitted.get_azimuth_deg(), front.get_azimuth_deg());
        assert_eq!(fitted.get_elevation_deg(), front.get_elevation_deg());
        assert_eq!(fitted.get_zoom(), 1.0);
        assert!(fitted.target().is_some());
        assert!(matches!(fit_update.event(), Some(Event::CameraChanged(_))));
        assert!(
            state
                .context_menu_action(PlotContextMenuAction::FitToContent)
                .events()
                .is_empty(),
            "fitting an already-fitted camera must be a no-op"
        );

        let reset_update = state.context_menu_action(PlotContextMenuAction::ResetView);
        assert_eq!(state.session.camera(), initial);
        assert!(matches!(reset_update.event(), Some(Event::CameraReset)));
        assert!(
            state
                .context_menu_action(PlotContextMenuAction::ResetView)
                .events()
                .is_empty(),
            "resetting an already-reset camera must be a no-op"
        );
    }

    #[cfg(feature = "3d")]
    #[test]
    fn three_d_pick_is_forwarded_as_adapter_event() {
        let mut state = Plot3DState::interactive(ruviz::scatter3d(&[0.0], &[0.0], &[0.0])).unwrap();
        let (width, height) = state.session.size_px();
        let mut hit_position = None;
        'outer: for y in (0..height).step_by(8) {
            for x in (0..width).step_by(8) {
                if state.session.pick(x as f32, y as f32).unwrap().is_some() {
                    hit_position = Some((f64::from(x), f64::from(y)));
                    break 'outer;
                }
            }
        }
        let hit_position = hit_position.expect("single origin point should be pickable");
        state.session.clear_pick();
        let _ = state.handle_widget_event(WidgetEvent::PointerPressed {
            position_px: hit_position,
            button: PointerButton::Left,
        });
        let update = state.handle_widget_event(WidgetEvent::PointerReleased {
            position_px: hit_position,
            button: PointerButton::Left,
        });
        assert!(matches!(update.event(), Some(Event::Picked3D(_))));
    }

    #[cfg(feature = "3d")]
    #[test]
    fn three_d_keep_view_and_render_requests_retain_camera_and_worker() {
        let mut state =
            Plot3DState::interactive(ruviz::scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0]))
                .unwrap();
        state.session.orbit(12.0, -5.0).unwrap();
        let camera = state.session.camera_snapshot().camera;
        let worker = Arc::clone(&state.renderer);

        let _ = state.set_plot_keep_view(ruviz::line3d(
            &[-1.0, 0.0, 1.0],
            &[0.0, 1.0, 0.0],
            &[1.0, 0.0, -1.0],
        ));
        assert_eq!(state.session.camera_snapshot().camera, camera);
        assert!(Arc::ptr_eq(&state.renderer, &worker));

        let _ = state.request_render();
        assert!(Arc::ptr_eq(&state.renderer, &worker));
    }

    #[cfg(feature = "3d")]
    #[test]
    fn failed_three_d_replacement_reports_error_without_rotating_incarnation() {
        let mut state = Plot3DState::interactive(ruviz::scatter3d(&[0.0], &[0.0], &[0.0])).unwrap();
        let incarnation = state.incarnation.clone();
        let view = state.session.view_stamp();
        let update = state.complete_replacement(Err(PlottingError::RenderError(
            "3D render request space was exhausted during replacement".to_owned(),
        )));

        assert!(matches!(update.event(), Some(Event::Error(_))));
        assert_eq!(state.incarnation, incarnation);
        assert_eq!(state.session.view_stamp(), view);
    }

    #[cfg(all(feature = "3d-gpu", not(target_arch = "wasm32")))]
    #[test]
    fn three_d_gpu_readback_selection_installs_mocked_worker_image() {
        let mut state = Plot3DState::interactive(ruviz::scatter3d(&[0.0], &[0.0], &[0.0])).unwrap();
        assert_eq!(
            state.renderer.lock().expect("renderer lock").backend(),
            BackgroundRenderBackend3D::GpuReadback,
        );

        let job = state.session.background_render_job().unwrap();
        let scheduled = state
            .scheduler
            .request(RenderRequest3D {
                incarnation: state.incarnation.clone(),
                job: job.clone(),
            })
            .unwrap();
        // The adapter boundary receives the same RenderedImage3D regardless of
        // backend. Use deterministic CPU production as a mock GPU-worker result
        // so this installation contract does not require a physical adapter.
        let rendered = BackgroundRenderer3D::new(BackgroundRenderBackend3D::Cpu)
            .render(job)
            .unwrap();
        // Mirror `render_3d_task`: the worker builds the handle so its
        // full-frame copy never lands on the UI thread.
        let frame = Rendered3DFrame {
            handle: crate::iced_handle_owned(&rendered.image).0,
            rendered,
        };
        let allocation =
            state.complete_render(state.incarnation.clone(), scheduled.id(), Ok(frame));
        let allocated = task_output(allocation.into_task());
        let installed = state.update(allocated);

        assert!(installed.events().is_empty());
        assert!(state.presented.is_some());
        assert_eq!(state.presented_view, Some(state.session.view_stamp()));
    }
}
