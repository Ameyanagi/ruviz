use std::sync::mpsc::{self, Receiver, Sender};

use egui::{Id, PointerButton, Response, Sense, TextureHandle, TextureOptions, Ui};
use ruviz::axes::AxisScale;
use ruviz::core::{
    HitResult, Image, ImageFit, ImageTarget, InteractiveChangeRevision,
    InteractiveChangeSubscription, InteractivePlotSession, InteractiveRenderStamp, IntoPlotSession,
    LatestRequestScheduler, Plot, PlotInputEvent, ScheduledRequestId, ViewportPoint, ViewportRect,
    physical_backing_size,
};

use crate::shared::{
    AdapterError, AdapterErrorKind, PlotSize, ViewMode, claim_scroll_y, color_image, fitted_rect,
    map_delta, map_point, map_point_clamped, next_widget_id, paint_texture, press_starts_in,
    release_is_cancelled, visible_content_rect,
};

/// Start an egui adapter builder from any ruviz plot, prepared plot, builder,
/// or retained 2D session.
pub fn plot_builder(plot: impl IntoPlotSession) -> RuvizPlotBuilder {
    RuvizPlotBuilder {
        session: plot.into_plot_session(),
        mode: ViewMode::Interactive,
        size: PlotSize::Fill,
        fit: ImageFit::Contain,
        prefer_gpu: cfg!(feature = "gpu"),
        id: None,
    }
}

/// Configuration builder for [`RuvizPlot`].
pub struct RuvizPlotBuilder {
    session: InteractivePlotSession,
    mode: ViewMode,
    size: PlotSize,
    fit: ImageFit,
    prefer_gpu: bool,
    id: Option<Id>,
}

impl RuvizPlotBuilder {
    /// Disable all user interaction while retaining resize and reactive redraws.
    pub fn static_view(mut self) -> Self {
        self.mode = ViewMode::Static;
        self
    }

    /// Enable pan, zoom, hover, click selection, brushing, and reset.
    pub fn interactive(mut self) -> Self {
        self.mode = ViewMode::Interactive;
        self
    }

    /// Consume the finite space available from the parent egui layout.
    pub fn fill(mut self) -> Self {
        self.size = PlotSize::Fill;
        self
    }

    /// Reserve a fixed size in egui logical points.
    pub fn fixed_pixels(mut self, width: f32, height: f32) -> Self {
        self.size = PlotSize::FixedPixels { width, height };
        self
    }

    /// Choose how the last rendered image is fitted into the widget rectangle.
    pub fn image_fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }

    /// Prefer ruviz's diagnosed 2D GPU path when the `gpu` feature is enabled.
    ///
    /// egui presentation still requires CPU readback and texture upload.
    pub fn prefer_gpu(mut self, prefer_gpu: bool) -> Self {
        self.prefer_gpu = prefer_gpu;
        self
    }

    /// Supply a stable egui identity. Generated IDs are unique by default.
    pub fn id_source(mut self, source: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id = Some(Id::new(source));
        self
    }

    pub fn build(self) -> RuvizPlot {
        RuvizPlot::new(
            self.session,
            self.mode,
            self.size,
            self.fit,
            self.prefer_gpu,
            self.id.unwrap_or_else(|| next_widget_id("2d")),
        )
    }
}

/// Observable event emitted while showing a 2D plot.
#[derive(Clone, Debug, PartialEq)]
pub enum PlotEvent {
    Hovered(Option<HitResult>),
    Clicked(HitResult),
    SelectionChanged,
    ViewChanged,
    BrushStarted,
    BrushFinished,
    DragCancelled,
    Reset,
    Error(AdapterError),
}

/// Framework response plus ruviz-specific interaction results.
#[derive(Clone, Debug)]
pub struct PlotResponse {
    pub response: Response,
    pub clicked: Option<HitResult>,
    pub hovered: Option<HitResult>,
    pub selection_changed: bool,
    pub view_changed: bool,
    pub events: Vec<PlotEvent>,
    pub error: Option<AdapterError>,
}

impl PlotResponse {
    pub fn changed(&self) -> bool {
        self.selection_changed || self.view_changed || !self.events.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RenderKey2D {
    size_px: (u32, u32),
    scale_bits: u32,
    time_bits: u64,
    revision: InteractiveChangeRevision,
}

#[derive(Clone)]
struct RenderRequest2D {
    session: InteractivePlotSession,
    session_epoch: u64,
    target: ImageTarget,
    repaint: egui::Context,
}

struct Rendered2D {
    image: Image,
    stamp: InteractiveRenderStamp,
}

enum RenderResult2D {
    Frame(Rendered2D),
    Superseded,
    Error(AdapterError),
}

struct RenderCompletion2D {
    id: ScheduledRequestId,
    session_epoch: u64,
    result: RenderResult2D,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Drag2D {
    Pan,
    Brush,
}

/// App-owned retained egui widget for a 2D ruviz plot.
pub struct RuvizPlot {
    session: InteractivePlotSession,
    mode: ViewMode,
    size: PlotSize,
    fit: ImageFit,
    prefer_gpu: bool,
    id: Id,
    texture: Option<TextureHandle>,
    image_size: Option<(u32, u32)>,
    displayed_stamp: Option<InteractiveRenderStamp>,
    session_epoch: u64,
    scheduler: LatestRequestScheduler<RenderRequest2D>,
    completion_tx: Sender<RenderCompletion2D>,
    completion_rx: Receiver<RenderCompletion2D>,
    last_requested: Option<RenderKey2D>,
    subscription: Option<InteractiveChangeSubscription>,
    subscribed_context: Option<egui::Context>,
    active_drag: Option<Drag2D>,
    last_drag_position: Option<egui::Pos2>,
    last_hover_position: Option<(u64, u64)>,
    last_hover: Option<HitResult>,
    session_hover_active: bool,
    last_error: Option<AdapterError>,
    time_seconds: f64,
}

impl RuvizPlot {
    fn new(
        session: InteractivePlotSession,
        mode: ViewMode,
        size: PlotSize,
        fit: ImageFit,
        prefer_gpu: bool,
        id: Id,
    ) -> Self {
        session.set_prefer_gpu(prefer_gpu);
        let (completion_tx, completion_rx) = mpsc::channel();
        Self {
            session,
            mode,
            size,
            fit,
            prefer_gpu,
            id,
            texture: None,
            image_size: None,
            displayed_stamp: None,
            session_epoch: 0,
            scheduler: LatestRequestScheduler::default(),
            completion_tx,
            completion_rx,
            last_requested: None,
            subscription: None,
            subscribed_context: None,
            active_drag: None,
            last_drag_position: None,
            last_hover_position: None,
            last_hover: None,
            session_hover_active: false,
            last_error: None,
            time_seconds: 0.0,
        }
    }

    pub fn session(&self) -> &InteractivePlotSession {
        &self.session
    }

    pub fn mode(&self) -> ViewMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: ViewMode) {
        self.mode = mode;
        if mode == ViewMode::Static {
            self.cancel_drag();
            if self.clear_hover_if_pointer_left(false) {
                self.last_requested = None;
                self.request_repaint();
            }
        }
    }

    pub fn set_time_seconds(&mut self, time_seconds: f64) {
        if self.time_seconds.to_bits() != time_seconds.to_bits() {
            self.time_seconds = time_seconds;
            self.session
                .apply_input(PlotInputEvent::SetTime { time_seconds });
            self.last_requested = None;
            self.request_repaint();
        }
    }

    /// Change the retained session's 2D backend preference.
    pub fn set_prefer_gpu(&mut self, prefer_gpu: bool) {
        if self.prefer_gpu != prefer_gpu {
            self.prefer_gpu = prefer_gpu;
            self.session.set_prefer_gpu(prefer_gpu);
            self.last_requested = None;
            self.request_repaint();
        }
    }

    pub fn last_error(&self) -> Option<&AdapterError> {
        self.last_error.as_ref()
    }

    pub fn clear_error(&mut self) {
        self.last_error = None;
    }

    /// Clear the last error and request the current frame again.
    pub fn retry_render(&mut self) {
        self.last_error = None;
        self.last_requested = None;
        self.request_repaint();
    }

    /// Replace the retained plot and reset to its view.
    pub fn set_plot(&mut self, plot: impl IntoPlotSession) {
        self.replace_session(plot.into_plot_session(), false);
    }

    /// Replace the retained plot while retaining a customized visible view.
    ///
    /// The previous visible bounds are restored only when they materially
    /// differ from the previous plot's base bounds. An untouched view uses the
    /// replacement plot's natural bounds.
    pub fn set_plot_keep_view(&mut self, plot: impl IntoPlotSession) {
        self.replace_session(plot.into_plot_session(), true);
    }

    /// Mark the current session dirty and request a fresh frame.
    pub fn invalidate(&mut self) {
        self.session.invalidate();
        self.last_requested = None;
        self.request_repaint();
    }

    /// Present the plot. This method never performs image rendering.
    pub fn show(&mut self, ui: &mut Ui) -> PlotResponse {
        self.ensure_subscription(ui.ctx());
        let size = self.size.desired(ui);
        let sense = if self.mode == ViewMode::Interactive {
            Sense::click_and_drag()
        } else {
            Sense::hover()
        };
        let (_, outer) = ui.allocate_space(size);
        let mut response = ui.interact(outer, self.id, sense);

        let mut events = Vec::new();
        self.drain_completions(ui.ctx(), &mut events);

        let scale_factor = ui.ctx().pixels_per_point();
        let target_size = physical_backing_size(
            f64::from(outer.width()),
            f64::from(outer.height()),
            scale_factor,
        );
        self.session.resize(target_size, scale_factor);
        let frame_size = self.image_size.unwrap_or(target_size);
        let content = fitted_rect(outer, frame_size, self.fit);
        let visible_content = visible_content_rect(content, outer);

        if let Some(texture) = &self.texture {
            paint_texture(ui, texture, content, outer);
        }

        let mut clicked = None;
        let mut hovered = self.last_hover.clone();
        let mut selection_changed = false;
        let mut view_changed = false;

        let frame_is_current = self
            .displayed_stamp
            .is_some_and(|stamp| self.session.is_render_stamp_current(stamp));
        let pointer_over_visible_content = response
            .hover_pos()
            .is_some_and(|position| visible_content.contains(position));
        if self.clear_hover_if_pointer_left(pointer_over_visible_content) {
            hovered = None;
            events.push(PlotEvent::Hovered(None));
        }
        if !frame_is_current && self.last_hover.take().is_some() {
            self.last_hover_position = None;
            hovered = None;
            events.push(PlotEvent::Hovered(None));
        }
        if self.mode == ViewMode::Interactive && self.image_size.is_some() && frame_is_current {
            let interaction =
                self.process_input(ui, &response, content, visible_content, frame_size);
            clicked = interaction.clicked;
            hovered = interaction.hovered;
            selection_changed = interaction.selection_changed;
            view_changed = interaction.view_changed;
            events.extend(interaction.events);
        } else if self.mode == ViewMode::Static && self.active_drag.is_some() {
            self.cancel_drag();
            events.push(PlotEvent::DragCancelled);
        } else if self.active_drag.is_some() {
            let (primary_down, focused) = ui.input(|input| {
                (
                    input.pointer.button_down(PointerButton::Primary),
                    input.focused,
                )
            });
            if !primary_down || !focused {
                let mut outcome = InputOutcome2D::default();
                self.cancel_active_drag(&mut outcome);
                events.extend(outcome.events);
            } else if let Some(position) = response.interact_pointer_pos() {
                self.last_drag_position = Some(position);
            }
        }

        let key = RenderKey2D {
            size_px: target_size,
            scale_bits: scale_factor.to_bits(),
            time_bits: self.time_seconds.to_bits(),
            revision: self.session.change_revision(),
        };
        self.request_render_if_needed(
            key,
            ImageTarget {
                size_px: target_size,
                scale_factor,
                time_seconds: self.time_seconds,
            },
            ui.ctx().clone(),
        );

        if selection_changed || view_changed || !events.is_empty() {
            response.mark_changed();
        }
        let error = events.iter().rev().find_map(|event| match event {
            PlotEvent::Error(error) => Some(error.clone()),
            _ => None,
        });
        PlotResponse {
            response,
            clicked,
            hovered,
            selection_changed,
            view_changed,
            events,
            error,
        }
    }

    fn replace_session(&mut self, replacement: InteractivePlotSession, keep_view: bool) {
        replacement.set_prefer_gpu(self.prefer_gpu);
        self.session.cancel_interaction();
        let old_viewport = self.session.view_bounds_snapshot();
        if keep_view
            && viewport_bounds_materially_differ(
                old_viewport.visible_bounds,
                old_viewport.base_bounds,
                &old_viewport.x_scale,
                &old_viewport.y_scale,
            )
        {
            replacement.defer_visible_bounds_restore(old_viewport.visible_bounds);
        }
        self.session = replacement;
        self.session_epoch = self.session_epoch.wrapping_add(1);
        self.subscription = None;
        self.last_requested = None;
        self.displayed_stamp = None;
        self.active_drag = None;
        self.last_drag_position = None;
        self.last_hover = None;
        self.last_hover_position = None;
        self.session_hover_active = false;
        self.last_error = None;
        self.request_repaint();
    }

    fn ensure_subscription(&mut self, context: &egui::Context) {
        let already_subscribed = self
            .subscribed_context
            .as_ref()
            .is_some_and(|current| current == context);
        if already_subscribed && self.subscription.is_some() {
            return;
        }
        let repaint = context.clone();
        self.subscription = Some(self.session.subscribe_changes(move |_| {
            repaint.request_repaint();
        }));
        self.subscribed_context = Some(context.clone());
    }

    fn request_repaint(&self) {
        if let Some(context) = &self.subscribed_context {
            context.request_repaint();
        }
    }

    fn queue_render(&mut self, target: ImageTarget, repaint: egui::Context) {
        let request = RenderRequest2D {
            session: self.session.clone(),
            session_epoch: self.session_epoch,
            target,
            repaint,
        };
        if let Some(scheduled) = self.scheduler.request(request) {
            self.spawn_render(scheduled.id(), scheduled.into_request());
        }
    }

    fn request_render_if_needed(
        &mut self,
        key: RenderKey2D,
        target: ImageTarget,
        repaint: egui::Context,
    ) -> bool {
        if self.last_requested == Some(key) {
            return false;
        }
        self.last_requested = Some(key);
        self.queue_render(target, repaint);
        true
    }

    fn spawn_render(&self, id: ScheduledRequestId, request: RenderRequest2D) {
        let sender = self.completion_tx.clone();
        let worker_sender = sender.clone();
        let worker_id = id.clone();
        let repaint = request.repaint.clone();
        if let Err(error) = std::thread::Builder::new()
            .name("ruviz-egui-2d-render".to_string())
            .spawn(move || {
                let result = match request.session.render_to_image_stamped(request.target) {
                    Ok(frame) => RenderResult2D::Frame(Rendered2D {
                        image: frame.frame.image.as_ref().clone(),
                        stamp: frame.render_stamp(),
                    }),
                    Err(error) if error.is_render_superseded() => RenderResult2D::Superseded,
                    Err(error) => {
                        RenderResult2D::Error(AdapterError::new(AdapterErrorKind::Render, error))
                    }
                };
                let _ = worker_sender.send(RenderCompletion2D {
                    id: worker_id,
                    session_epoch: request.session_epoch,
                    result,
                });
                request.repaint.request_repaint();
            })
        {
            let _ = sender.send(RenderCompletion2D {
                id,
                session_epoch: self.session_epoch,
                result: RenderResult2D::Error(AdapterError::new(AdapterErrorKind::Render, error)),
            });
            repaint.request_repaint();
        }
    }

    fn drain_completions(&mut self, context: &egui::Context, events: &mut Vec<PlotEvent>) {
        while let Ok(completed) = self.completion_rx.try_recv() {
            self.handle_completion(context, events, completed);
        }
    }

    fn handle_completion(
        &mut self,
        context: &egui::Context,
        events: &mut Vec<PlotEvent>,
        completed: RenderCompletion2D,
    ) {
        let Some(state) = self.scheduler.complete(completed.id) else {
            return;
        };
        if state.install && completed.session_epoch == self.session_epoch {
            match completed.result {
                RenderResult2D::Frame(frame)
                    if self.session.is_render_stamp_current(frame.stamp) =>
                {
                    self.install_image(context, frame.image);
                    self.displayed_stamp = Some(frame.stamp);
                    self.last_error = None;
                }
                RenderResult2D::Frame(_) | RenderResult2D::Superseded => {}
                RenderResult2D::Error(error) => {
                    self.last_error = Some(error.clone());
                    events.push(PlotEvent::Error(error));
                }
            }
        }
        if let Some(next) = state.next {
            self.spawn_render(next.id(), next.into_request());
        }
    }

    fn install_image(&mut self, context: &egui::Context, image: Image) {
        self.image_size = Some((image.width, image.height));
        let color = color_image(&image);
        if let Some(texture) = &mut self.texture {
            texture.set(color, TextureOptions::LINEAR);
        } else {
            self.texture = Some(context.load_texture(
                format!("ruviz-egui-2d-{:?}", self.id),
                color,
                TextureOptions::LINEAR,
            ));
        }
    }

    fn process_input(
        &mut self,
        ui: &Ui,
        response: &Response,
        content: egui::Rect,
        visible_content: egui::Rect,
        image_size: (u32, u32),
    ) -> InputOutcome2D {
        let mut outcome = InputOutcome2D {
            hovered: self.last_hover.clone(),
            ..InputOutcome2D::default()
        };
        let pointer = response.interact_pointer_pos();
        let primary_down = ui.input(|input| input.pointer.button_down(PointerButton::Primary));

        if response.clicked() {
            response.request_focus();
        }
        if response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Escape)) {
            if self.active_drag.is_some() {
                self.cancel_active_drag(&mut outcome);
            } else {
                self.session.cancel_interaction();
            }
            self.session.apply_input(PlotInputEvent::ResetView);
            outcome.view_changed = true;
            outcome.events.push(PlotEvent::Reset);
            outcome.events.push(PlotEvent::ViewChanged);
        }

        if let Some(position) = response
            .hover_pos()
            .filter(|position| visible_content.contains(*position))
        {
            if let Some((x, y)) = map_point(content, position, image_size) {
                let position_px = ViewportPoint::new(x, y);
                let hover_key = (x.to_bits(), y.to_bits());
                if self.last_hover_position != Some(hover_key) {
                    let hit = self.session.hit_test(position_px);
                    self.session
                        .apply_input(PlotInputEvent::Hover { position_px });
                    self.session_hover_active = true;
                    self.last_hover_position = Some(hover_key);
                    self.last_hover = (!matches!(hit, HitResult::None)).then_some(hit);
                    outcome.hovered = self.last_hover.clone();
                    outcome
                        .events
                        .push(PlotEvent::Hovered(self.last_hover.clone()));
                }

                let scroll_y = claim_scroll_y(ui);
                if scroll_y != 0.0 {
                    let factor = (f64::from(scroll_y) * 0.002).exp();
                    self.session.apply_input(PlotInputEvent::Zoom {
                        factor,
                        center_px: position_px,
                    });
                    outcome.view_changed = true;
                    outcome.events.push(PlotEvent::ViewChanged);
                }
            }
        } else if self.clear_hover_if_pointer_left(false) {
            outcome.hovered = None;
            outcome.events.push(PlotEvent::Hovered(None));
        }

        if response.double_clicked_by(PointerButton::Primary) {
            if self.active_drag.is_some() {
                self.cancel_active_drag(&mut outcome);
            }
            self.session.apply_input(PlotInputEvent::ResetView);
            outcome.view_changed = true;
            outcome.events.push(PlotEvent::Reset);
            outcome.events.push(PlotEvent::ViewChanged);
        } else if response.clicked_by(PointerButton::Primary)
            && let Some(position) = pointer.and_then(|point| map_point(content, point, image_size))
        {
            let position_px = ViewportPoint::new(position.0, position.1);
            let hit = self.session.hit_test(position_px);
            self.session
                .apply_input(PlotInputEvent::SelectAt { position_px });
            outcome.selection_changed = true;
            outcome.events.push(PlotEvent::SelectionChanged);
            if !matches!(hit, HitResult::None) {
                outcome.clicked = Some(hit.clone());
                outcome.events.push(PlotEvent::Clicked(hit));
            }
        }

        if response.drag_started_by(PointerButton::Primary) {
            let brush = ui.input(|input| input.modifiers.shift);
            let press_origin = ui.input(|input| input.pointer.press_origin());
            if !press_starts_in(visible_content, press_origin) {
                return outcome;
            }
            let Some(start) = press_origin else {
                return outcome;
            };
            response.request_focus();
            self.last_drag_position = Some(start);
            if brush {
                let (x, y) = map_point_clamped(content, start, image_size);
                self.session.apply_input(PlotInputEvent::BrushStart {
                    position_px: ViewportPoint::new(x, y),
                });
                self.active_drag = Some(Drag2D::Brush);
                outcome.events.push(PlotEvent::BrushStarted);
            } else {
                self.active_drag = Some(Drag2D::Pan);
            }
        }

        if response.dragged_by(PointerButton::Primary)
            && let Some(position) = pointer
        {
            match self.active_drag {
                Some(Drag2D::Pan) => {
                    let delta = incremental_drag_delta(self.last_drag_position, position);
                    let (x, y) = map_delta(content, delta, image_size);
                    if x != 0.0 || y != 0.0 {
                        self.session.apply_input(PlotInputEvent::Pan {
                            delta_px: ViewportPoint::new(x, y),
                        });
                        outcome.view_changed = true;
                        outcome.events.push(PlotEvent::ViewChanged);
                    }
                }
                Some(Drag2D::Brush) => {
                    let (x, y) = map_point_clamped(content, position, image_size);
                    self.session.apply_input(PlotInputEvent::BrushMove {
                        position_px: ViewportPoint::new(x, y),
                    });
                }
                None => {}
            }
            self.last_drag_position = Some(position);
        }

        let focused = ui.input(|input| input.focused);
        if response.drag_stopped_by(PointerButton::Primary) {
            if release_is_cancelled(visible_content, pointer, focused) {
                self.cancel_active_drag(&mut outcome);
            } else {
                self.finish_drag(pointer, content, image_size, &mut outcome);
            }
        } else if self.active_drag.is_some() && (!primary_down || !focused) {
            self.cancel_active_drag(&mut outcome);
        }

        outcome
    }

    fn clear_hover_if_pointer_left(&mut self, pointer_over_visible_content: bool) -> bool {
        if pointer_over_visible_content || !std::mem::take(&mut self.session_hover_active) {
            return false;
        }
        self.session.apply_input(PlotInputEvent::ClearHover);
        self.last_hover_position = None;
        self.last_hover = None;
        true
    }

    fn finish_drag(
        &mut self,
        position: Option<egui::Pos2>,
        content: egui::Rect,
        image_size: (u32, u32),
        outcome: &mut InputOutcome2D,
    ) {
        if self.active_drag.take() == Some(Drag2D::Brush) {
            let (x, y) =
                map_point_clamped(content, position.unwrap_or(content.center()), image_size);
            self.session.apply_input(PlotInputEvent::BrushEnd {
                position_px: ViewportPoint::new(x, y),
            });
            outcome.selection_changed = true;
            outcome.events.push(PlotEvent::SelectionChanged);
            outcome.events.push(PlotEvent::BrushFinished);
        }
        self.last_drag_position = None;
    }

    fn cancel_active_drag(&mut self, outcome: &mut InputOutcome2D) {
        self.active_drag = None;
        self.session.cancel_interaction();
        self.last_drag_position = None;
        outcome.events.push(PlotEvent::DragCancelled);
    }

    fn cancel_drag(&mut self) {
        self.active_drag = None;
        self.session.cancel_interaction();
        self.last_drag_position = None;
    }
}

fn incremental_drag_delta(previous: Option<egui::Pos2>, current: egui::Pos2) -> egui::Vec2 {
    previous.map_or(egui::Vec2::ZERO, |previous| current - previous)
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

#[derive(Default)]
struct InputOutcome2D {
    clicked: Option<HitResult>,
    hovered: Option<HitResult>,
    selection_changed: bool,
    view_changed: bool,
    events: Vec<PlotEvent>,
}

impl Default for RuvizPlot {
    fn default() -> Self {
        plot_builder(Plot::new().line(&[0.0, 1.0], &[0.0, 1.0])).build()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::Duration;

    use super::*;
    use ruviz::data::Observable;

    fn plot() -> impl IntoPlotSession {
        Plot::new().line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 0.0])
    }

    const BACKGROUND_RENDER_TIMEOUT: Duration = Duration::from_secs(30);

    fn scheduled_completion(
        widget: &mut RuvizPlot,
        target: ImageTarget,
        repaint: egui::Context,
    ) -> RenderCompletion2D {
        let scheduled = widget
            .scheduler
            .request(RenderRequest2D {
                session: widget.session.clone(),
                session_epoch: widget.session_epoch,
                target,
                repaint,
            })
            .unwrap();
        let id = scheduled.id();
        let request = scheduled.into_request();
        let frame = request
            .session
            .render_to_image_stamped(request.target)
            .unwrap();
        RenderCompletion2D {
            id,
            session_epoch: request.session_epoch,
            result: RenderResult2D::Frame(Rendered2D {
                image: frame.frame.image.as_ref().clone(),
                stamp: frame.render_stamp(),
            }),
        }
    }

    #[test]
    fn builder_exposes_static_interactive_and_sizing_modes() {
        let static_plot = plot_builder(plot())
            .static_view()
            .fixed_pixels(320.0, 180.0)
            .image_fit(ImageFit::Cover)
            .build();
        assert_eq!(static_plot.mode(), ViewMode::Static);

        let interactive = plot_builder(plot()).interactive().fill().build();
        assert_eq!(interactive.mode(), ViewMode::Interactive);
    }

    #[test]
    fn keep_view_restores_compatible_bounds() {
        let initial = Plot::new()
            .line(&[0.0, 10.0], &[0.0, 10.0])
            .xlim(0.0, 10.0)
            .ylim(0.0, 10.0);
        let mut widget = plot_builder(initial).build();
        let custom_view =
            ViewportRect::from_points(ViewportPoint::new(2.0, 1.0), ViewportPoint::new(8.0, 9.0));
        assert!(widget.session.restore_visible_bounds(custom_view));

        let replacement = Plot::new()
            .line(&[0.0, 20.0], &[0.0, 20.0])
            .xlim(0.0, 20.0)
            .ylim(0.0, 20.0);
        widget.set_plot_keep_view(replacement);
        widget
            .session
            .render_to_image_stamped(ImageTarget {
                size_px: (320, 200),
                scale_factor: 1.0,
                time_seconds: 0.0,
            })
            .unwrap();
        let after = widget.session.view_bounds_snapshot().visible_bounds;
        assert!((after.min.x - custom_view.min.x).abs() < 1e-12);
        assert!((after.min.y - custom_view.min.y).abs() < 1e-12);
        assert!((after.max.x - custom_view.max.x).abs() < 1e-12);
        assert!((after.max.y - custom_view.max.y).abs() < 1e-12);
    }

    #[test]
    fn keep_view_uses_replacement_bounds_when_old_view_was_untouched() {
        let initial = Plot::new()
            .line(&[0.0, 10.0], &[0.0, 10.0])
            .xlim(0.0, 10.0)
            .ylim(0.0, 10.0);
        let mut widget = plot_builder(initial).build();
        let replacement = Plot::new()
            .line(&[100.0, 200.0], &[-5.0, 5.0])
            .xlim(100.0, 200.0)
            .ylim(-5.0, 5.0);

        widget.set_plot_keep_view(replacement);

        let snapshot = widget.session.view_bounds_snapshot();
        assert_eq!(snapshot.visible_bounds, snapshot.base_bounds);
        assert_eq!(
            snapshot.base_bounds,
            ViewportRect::from_points(
                ViewportPoint::new(100.0, -5.0),
                ViewportPoint::new(200.0, 5.0),
            )
        );
    }

    #[test]
    fn pointer_leave_clears_hover_even_after_stale_bookkeeping_is_retired() {
        let mut widget = plot_builder(plot()).build();
        widget.session_hover_active = true;
        widget.last_hover_position = None;
        widget.last_hover = None;

        assert!(!widget.clear_hover_if_pointer_left(true));
        assert!(widget.session_hover_active);
        assert!(widget.clear_hover_if_pointer_left(false));
        assert!(!widget.session_hover_active);
        assert!(widget.last_hover_position.is_none());
        assert!(widget.last_hover.is_none());
    }

    #[test]
    fn switching_to_static_clears_drag_hover_and_requests_a_cleanup_frame() {
        let mut widget = plot_builder(plot()).build();
        widget.active_drag = Some(Drag2D::Brush);
        widget.last_drag_position = Some(egui::pos2(10.0, 20.0));
        widget.session_hover_active = true;
        widget.last_hover_position = Some((10, 20));
        widget.last_requested = Some(RenderKey2D {
            size_px: (80, 48),
            scale_bits: 1.0_f32.to_bits(),
            time_bits: 0.0_f64.to_bits(),
            revision: widget.session.change_revision(),
        });

        widget.set_mode(ViewMode::Static);

        assert_eq!(widget.mode(), ViewMode::Static);
        assert!(widget.active_drag.is_none());
        assert!(widget.last_drag_position.is_none());
        assert!(!widget.session_hover_active);
        assert!(widget.last_hover_position.is_none());
        assert!(widget.last_requested.is_none());
    }

    #[test]
    fn observable_change_wakes_schedules_and_installs_a_reactive_frame() {
        let y = Observable::new(vec![0.0, 1.0, 4.0]);
        let reactive_plot = Plot::new().line_source(vec![0.0, 1.0, 2.0], y.clone());
        let mut widget = plot_builder(reactive_plot).build();
        let context = egui::Context::default();
        let wake_count = Arc::new(AtomicUsize::new(0));
        context.set_request_repaint_callback({
            let wake_count = Arc::clone(&wake_count);
            move |_| {
                wake_count.fetch_add(1, Ordering::SeqCst);
            }
        });
        widget.ensure_subscription(&context);
        let target = ImageTarget {
            size_px: (96, 64),
            scale_factor: 1.0,
            time_seconds: 0.0,
        };
        let initial_key = RenderKey2D {
            size_px: target.size_px,
            scale_bits: target.scale_factor.to_bits(),
            time_bits: target.time_seconds.to_bits(),
            revision: widget.session.change_revision(),
        };
        assert!(widget.request_render_if_needed(initial_key, target, context.clone()));
        let initial_completion = widget
            .completion_rx
            .recv_timeout(BACKGROUND_RENDER_TIMEOUT)
            .expect("initial reactive frame should complete");
        widget.handle_completion(&context, &mut Vec::new(), initial_completion);
        let initial_stamp = widget.displayed_stamp.unwrap();
        assert!(widget.texture.is_some());

        for _ in 0..4 {
            let _ = context.run_ui(egui::RawInput::default(), |_| {});
            if !context.has_requested_repaint() {
                break;
            }
        }
        assert!(!context.has_requested_repaint());
        let wakes_before_update = wake_count.load(Ordering::SeqCst);
        let revision_before_update = widget.session.change_revision();

        y.set(vec![0.0, 1.0, 9.0]);

        let updated_revision = widget.session.change_revision();
        assert_ne!(updated_revision, revision_before_update);
        assert!(context.has_requested_repaint());
        assert!(wake_count.load(Ordering::SeqCst) > wakes_before_update);
        let updated_key = RenderKey2D {
            revision: updated_revision,
            ..initial_key
        };
        assert!(widget.request_render_if_needed(updated_key, target, context.clone()));
        let updated_completion = widget
            .completion_rx
            .recv_timeout(BACKGROUND_RENDER_TIMEOUT)
            .expect("updated reactive frame should complete");
        widget.handle_completion(&context, &mut Vec::new(), updated_completion);

        let updated_stamp = widget.displayed_stamp.unwrap();
        assert_ne!(updated_stamp, initial_stamp);
        assert!(widget.session.is_render_stamp_current(updated_stamp));
        assert_eq!(widget.image_size, Some(target.size_px));
        assert!(widget.texture.is_some());
    }

    #[test]
    fn multiple_widgets_keep_completion_and_input_routing_independent() {
        let mut first =
            plot_builder(Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]).title("first")).build();
        let mut second =
            plot_builder(Plot::new().line(&[0.0, 1.0], &[1.0, 0.0]).title("second")).build();
        let context = egui::Context::default();
        let first_target = ImageTarget {
            size_px: (80, 48),
            scale_factor: 1.0,
            time_seconds: 0.0,
        };
        let second_target = ImageTarget {
            size_px: (120, 72),
            scale_factor: 1.0,
            time_seconds: 0.0,
        };
        let first_completion = scheduled_completion(&mut first, first_target, context.clone());
        let second_completion = scheduled_completion(&mut second, second_target, context.clone());

        let foreign_completion = RenderCompletion2D {
            id: first_completion.id.clone(),
            session_epoch: first_completion.session_epoch,
            result: match &first_completion.result {
                RenderResult2D::Frame(frame) => RenderResult2D::Frame(Rendered2D {
                    image: frame.image.clone(),
                    stamp: frame.stamp,
                }),
                RenderResult2D::Superseded | RenderResult2D::Error(_) => unreachable!(),
            },
        };
        second.handle_completion(&context, &mut Vec::new(), foreign_completion);
        assert!(second.displayed_stamp.is_none());
        assert!(!second.scheduler.is_idle());

        first.handle_completion(&context, &mut Vec::new(), first_completion);
        assert_eq!(first.image_size, Some(first_target.size_px));
        assert!(first.texture.is_some());
        assert!(second.texture.is_none());

        second.handle_completion(&context, &mut Vec::new(), second_completion);
        assert_eq!(second.image_size, Some(second_target.size_px));
        assert!(second.texture.is_some());
        assert_ne!(
            first.texture.as_ref().map(TextureHandle::id),
            second.texture.as_ref().map(TextureHandle::id)
        );

        first.session_hover_active = true;
        second.session_hover_active = true;
        assert!(first.clear_hover_if_pointer_left(false));
        assert!(!first.session_hover_active);
        assert!(second.session_hover_active);

        first.active_drag = Some(Drag2D::Pan);
        second.active_drag = Some(Drag2D::Brush);
        let mut first_events = InputOutcome2D::default();
        first.cancel_active_drag(&mut first_events);
        assert!(first.active_drag.is_none());
        assert_eq!(second.active_drag, Some(Drag2D::Brush));
        assert_eq!(first_events.events, vec![PlotEvent::DragCancelled]);
    }

    #[test]
    fn replacement_does_not_discard_last_good_texture_state_eagerly() {
        let mut widget = plot_builder(plot()).build();
        widget.image_size = Some((640, 360));
        widget.session_epoch = u64::MAX;
        widget.set_plot(plot());
        assert_eq!(widget.image_size, Some((640, 360)));
        assert!(widget.last_requested.is_none());
        assert_eq!(widget.session_epoch, 0);
    }

    #[test]
    fn replacement_keeps_an_existing_render_in_flight_and_coalesces_new_work() {
        let mut widget = plot_builder(plot()).build();
        let repaint = egui::Context::default();
        assert!(
            widget
                .scheduler
                .request(RenderRequest2D {
                    session: widget.session.clone(),
                    session_epoch: widget.session_epoch,
                    target: ImageTarget::default(),
                    repaint: repaint.clone(),
                })
                .is_some()
        );

        widget.set_plot(plot());

        assert!(!widget.scheduler.is_idle());
        assert!(
            widget
                .scheduler
                .request(RenderRequest2D {
                    session: widget.session.clone(),
                    session_epoch: widget.session_epoch,
                    target: ImageTarget::default(),
                    repaint,
                })
                .is_none()
        );
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn gpu_feature_defaults_to_readback_preference_and_remains_configurable() {
        let mut widget = plot_builder(plot()).build();
        assert!(widget.session.prefer_gpu());
        widget.set_prefer_gpu(false);
        assert!(!widget.session.prefer_gpu());
    }

    #[test]
    fn render_keys_distinguish_fractional_scale_and_reactive_revision() {
        let widget = plot_builder(plot()).build();
        let first = RenderKey2D {
            size_px: (400, 300),
            scale_bits: 1.25_f32.to_bits(),
            time_bits: 0.0_f64.to_bits(),
            revision: widget.session.change_revision(),
        };
        let second = RenderKey2D {
            scale_bits: 1.5_f32.to_bits(),
            ..first
        };
        assert_ne!(first, second);
    }

    #[test]
    fn retry_render_clears_error_and_request_sentinel() {
        let mut widget = plot_builder(plot()).build();
        widget.last_requested = Some(RenderKey2D {
            size_px: (640, 360),
            scale_bits: 1.0_f32.to_bits(),
            time_bits: 0.0_f64.to_bits(),
            revision: widget.session.change_revision(),
        });
        widget.last_error = Some(AdapterError::new(AdapterErrorKind::Render, "failed"));
        widget.retry_render();
        assert!(widget.last_requested.is_none());
        assert!(widget.last_error.is_none());
    }

    #[test]
    fn pan_uses_only_the_delta_since_the_previous_frame() {
        let first = incremental_drag_delta(Some(egui::pos2(10.0, 20.0)), egui::pos2(13.0, 24.0));
        let second = incremental_drag_delta(Some(egui::pos2(13.0, 24.0)), egui::pos2(15.0, 25.0));
        assert_eq!(first, egui::vec2(3.0, 4.0));
        assert_eq!(second, egui::vec2(2.0, 1.0));
    }

    #[test]
    fn cancelled_brush_does_not_emit_selection_or_finish() {
        let mut widget = plot_builder(plot()).build();
        widget.active_drag = Some(Drag2D::Brush);
        let mut outcome = InputOutcome2D::default();
        widget.cancel_active_drag(&mut outcome);
        assert_eq!(outcome.events, vec![PlotEvent::DragCancelled]);
        assert!(!outcome.selection_changed);
    }
}
