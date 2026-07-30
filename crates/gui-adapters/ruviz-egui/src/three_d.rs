use std::sync::{
    Arc,
    mpsc::{self, Receiver, Sender},
};

use egui::{Id, PointerButton, Response, Sense, TextureHandle, Ui};
use ruviz::core::{
    BackgroundRenderBackend3D, BackgroundRenderJob3D, BackgroundRenderOutcome3D,
    BackgroundRenderer3D, CameraSnapshot3D, CameraView3D, Image, ImageFit, InputEvent3D,
    InteractivePlot3DSession, LatestRequestScheduler, PickHit3D, PlotContextMenuAction,
    PointerButton3D, RenderedImage3D, RenderedLayer, ScheduledRequestId, TryIntoPlot3DSession,
    ViewStamp3D, physical_backing_size,
};

use crate::shared::{
    AdapterError, AdapterErrorKind, PlotSize, RenderWorker, ViewMode, catch_render_panic,
    claim_scroll_y, copy_image_to_clipboard, fitted_rect, map_point, map_point_clamped,
    next_widget_id, paint_texture, press_starts_in, spawn_png_save, upload_texture,
    visible_content_rect,
};

/// Start a feature-gated egui 3D adapter builder.
pub fn plot3d_builder<P>(plot: P) -> RuvizPlot3DBuilder<P> {
    RuvizPlot3DBuilder {
        plot,
        mode: ViewMode::Interactive,
        size: PlotSize::Fill,
        fit: ImageFit::Contain,
        id: None,
    }
}

/// Configuration builder for [`RuvizPlot3D`].
pub struct RuvizPlot3DBuilder<P> {
    plot: P,
    mode: ViewMode,
    size: PlotSize,
    fit: ImageFit,
    id: Option<Id>,
}

impl<P> RuvizPlot3DBuilder<P> {
    pub fn static_view(mut self) -> Self {
        self.mode = ViewMode::Static;
        self
    }

    pub fn interactive(mut self) -> Self {
        self.mode = ViewMode::Interactive;
        self
    }

    pub fn fill(mut self) -> Self {
        self.size = PlotSize::Fill;
        self
    }

    pub fn fixed_pixels(mut self, width: f32, height: f32) -> Self {
        self.size = PlotSize::FixedPixels { width, height };
        self
    }

    pub fn image_fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }

    pub fn id_source(mut self, source: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id = Some(Id::new(source));
        self
    }
}

impl<P> RuvizPlot3DBuilder<P>
where
    P: TryIntoPlot3DSession,
{
    pub fn build(self) -> ruviz::core::Result<RuvizPlot3D> {
        RuvizPlot3D::new(
            self.plot.try_into_plot3d_session()?,
            self.mode,
            self.size,
            self.fit,
            self.id.unwrap_or_else(|| next_widget_id("3d")),
        )
    }
}

/// Observable event emitted while showing a 3D plot.
///
/// New variants are added in minor releases, so match with a `_` arm.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Plot3DEvent {
    Hovered(Option<PickHit3D>),
    Picked(PickHit3D),
    CameraChanged(CameraSnapshot3D),
    DragCancelled,
    Reset,
    ContextMenuAction(PlotContextMenuAction),
    Error(AdapterError),
}

fn plot3d_event_marks_changed(event: &Plot3DEvent) -> bool {
    !matches!(
        event,
        Plot3DEvent::ContextMenuAction(
            PlotContextMenuAction::ResetView
                | PlotContextMenuAction::FitToContent
                | PlotContextMenuAction::SaveImage
                | PlotContextMenuAction::CopyImage
                | PlotContextMenuAction::CameraView(_)
        )
    )
}

/// Framework response plus ruviz-specific 3D interaction results.
#[derive(Clone, Debug)]
pub struct Plot3DResponse {
    pub response: Response,
    pub picked: Option<PickHit3D>,
    pub hovered: Option<PickHit3D>,
    pub camera_changed: bool,
    pub events: Vec<Plot3DEvent>,
    pub error: Option<AdapterError>,
}

impl Plot3DResponse {
    pub fn changed(&self) -> bool {
        self.camera_changed || self.events.iter().any(plot3d_event_marks_changed)
    }
}

#[derive(Clone)]
struct RenderRequest3D {
    job: BackgroundRenderJob3D,
    scene_epoch: u64,
    repaint: egui::Context,
}

enum RenderResult3D {
    Frame(RenderedImage3D),
    Error(AdapterError),
}

struct RenderCompletion3D {
    id: ScheduledRequestId,
    scene_epoch: u64,
    result: RenderResult3D,
}

struct WorkerRequest3D {
    id: ScheduledRequestId,
    request: RenderRequest3D,
}

/// App-owned retained egui widget for a 3D ruviz plot.
pub struct RuvizPlot3D {
    session: InteractivePlot3DSession,
    mode: ViewMode,
    size: PlotSize,
    fit: ImageFit,
    id: Id,
    texture: Option<TextureHandle>,
    installed_image: Option<Arc<Image>>,
    image_size: Option<(u32, u32)>,
    displayed_view: Option<ViewStamp3D>,
    scene_epoch: u64,
    scheduler: LatestRequestScheduler<RenderRequest3D>,
    worker: RenderWorker<WorkerRequest3D>,
    completion_tx: Sender<RenderCompletion3D>,
    completion_rx: Receiver<RenderCompletion3D>,
    save_completion_tx: Sender<Result<(), AdapterError>>,
    save_completion_rx: Receiver<Result<(), AdapterError>>,
    last_requested_view: Option<ViewStamp3D>,
    repaint_context: Option<egui::Context>,
    active_button: Option<PointerButton3D>,
    last_pointer: Option<egui::Pos2>,
    last_hover_position: Option<(u32, u32)>,
    hovered: Option<PickHit3D>,
    last_error: Option<AdapterError>,
}

impl RuvizPlot3D {
    fn new(
        session: InteractivePlot3DSession,
        mode: ViewMode,
        size: PlotSize,
        fit: ImageFit,
        id: Id,
    ) -> ruviz::core::Result<Self> {
        let (completion_tx, completion_rx) = mpsc::channel();
        let (save_completion_tx, save_completion_rx) = mpsc::channel();
        let worker = spawn_render_worker(completion_tx.clone()).map_err(|error| {
            ruviz::core::PlottingError::RenderError(format!(
                "failed to start ruviz-egui 3D render worker: {error}"
            ))
        })?;
        Ok(Self {
            session,
            mode,
            size,
            fit,
            id,
            texture: None,
            installed_image: None,
            image_size: None,
            displayed_view: None,
            scene_epoch: 0,
            scheduler: LatestRequestScheduler::default(),
            worker,
            completion_tx,
            completion_rx,
            save_completion_tx,
            save_completion_rx,
            last_requested_view: None,
            repaint_context: None,
            active_button: None,
            last_pointer: None,
            last_hover_position: None,
            hovered: None,
            last_error: None,
        })
    }

    pub fn session(&self) -> &InteractivePlot3DSession {
        &self.session
    }

    pub fn mode(&self) -> ViewMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: ViewMode) {
        self.mode = mode;
        if mode == ViewMode::Static {
            self.session.cancel_drag();
            self.active_button = None;
            self.last_pointer = None;
        }
    }

    pub fn last_error(&self) -> Option<&AdapterError> {
        self.last_error.as_ref()
    }

    pub fn clear_error(&mut self) {
        self.last_error = None;
    }

    /// Clear the last error and explicitly retry the unchanged current view.
    pub fn retry_render(&mut self) {
        self.last_error = None;
        self.last_requested_view = None;
        self.request_repaint();
    }

    /// Replace the scene and reset to the replacement camera.
    pub fn set_plot(&mut self, plot: impl TryIntoPlot3DSession) -> ruviz::core::Result<()> {
        let replacement = plot.try_into_plot3d_session()?;
        let result = self.session.try_replace(replacement);
        self.complete_replacement(result)
    }

    /// Replace the scene while preserving the current camera.
    pub fn set_plot_keep_view(
        &mut self,
        plot: impl TryIntoPlot3DSession,
    ) -> ruviz::core::Result<()> {
        let replacement = plot.try_into_plot3d_session()?;
        let result = self.session.replace_keep_camera(replacement);
        self.complete_replacement(result)
    }

    fn complete_replacement(&mut self, result: ruviz::core::Result<()>) -> ruviz::core::Result<()> {
        result?;
        self.after_replacement();
        Ok(())
    }

    /// Present the plot. Image rendering is always performed by a worker.
    pub fn show(&mut self, ui: &mut Ui) -> Plot3DResponse {
        self.repaint_context = Some(ui.ctx().clone());
        let size = self.size.desired(ui);
        let sense = plot_sense(self.mode);
        let (_, outer) = ui.allocate_space(size);
        let mut response = ui.interact(outer, self.id, sense);
        let mut events = Vec::new();
        self.drain_completions(ui.ctx(), &mut events);
        self.drain_save_completions(&mut events);

        let scale_factor = ui.ctx().pixels_per_point();
        let target_size = physical_backing_size(
            f64::from(outer.width()),
            f64::from(outer.height()),
            scale_factor,
        );
        if let Err(error) = self
            .session
            .resize(target_size.0, target_size.1, scale_factor)
        {
            self.record_error(
                AdapterError::new(AdapterErrorKind::Interaction, error),
                &mut events,
            );
        }

        let frame_size = self.image_size.unwrap_or(target_size);
        let content = fitted_rect(outer, frame_size, self.fit, scale_factor);
        let visible_content = visible_content_rect(content, outer);
        if let Some(texture) = &self.texture {
            paint_texture(ui, texture, content, outer);
        }

        let mut picked = None;
        let mut hovered = self.hovered;
        let mut camera_changed = false;
        let frame_is_current = self
            .displayed_view
            .is_some_and(|stamp| self.session.is_view_current(stamp));
        if !frame_is_current && self.hovered.take().is_some() {
            self.last_hover_position = None;
            events.push(Plot3DEvent::Hovered(None));
        }
        if self.mode == ViewMode::Interactive && self.image_size.is_some() && frame_is_current {
            let interaction =
                self.process_input(ui, &response, content, visible_content, frame_size);
            picked = interaction.picked;
            hovered = interaction.hovered;
            camera_changed = interaction.camera_changed;
            events.extend(interaction.events);
        } else if self.mode == ViewMode::Static && self.session.cancel_drag() {
            self.active_button = None;
            events.push(Plot3DEvent::DragCancelled);
        } else if let Some(button) = self.active_button {
            let (down, focused) = ui.input(|input| {
                (
                    input.pointer.button_down(match button {
                        PointerButton3D::Left => PointerButton::Primary,
                        PointerButton3D::Middle => PointerButton::Middle,
                        PointerButton3D::Right => PointerButton::Secondary,
                    }),
                    input.focused,
                )
            });
            if !down || !focused {
                self.session.cancel_drag();
                self.active_button = None;
                self.last_pointer = None;
                events.push(Plot3DEvent::DragCancelled);
            }
        }

        if let Some(action) =
            plot_context_menu_action(&response, self.mode, self.installed_image.is_some())
        {
            let mut outcome = InputOutcome3D::default();
            self.apply_context_menu_action(action, ui.ctx(), &mut outcome);
            picked = outcome.picked.or(picked);
            hovered = self.hovered;
            camera_changed |= outcome.camera_changed;
            events.extend(outcome.events);
        }

        let view = self.session.view_stamp();
        if self.last_requested_view != Some(view) {
            self.last_requested_view = Some(view);
            match self.session.background_render_job() {
                Ok(job) => self.queue_render(job, ui.ctx().clone()),
                Err(error) => self.record_error(
                    AdapterError::new(AdapterErrorKind::Render, error),
                    &mut events,
                ),
            }
        }

        if camera_changed || events.iter().any(plot3d_event_marks_changed) {
            response.mark_changed();
        }
        let error = events.iter().rev().find_map(|event| match event {
            Plot3DEvent::Error(error) => Some(error.clone()),
            _ => None,
        });
        Plot3DResponse {
            response,
            picked,
            hovered,
            camera_changed,
            events,
            error,
        }
    }

    fn after_replacement(&mut self) {
        self.last_requested_view = None;
        self.scene_epoch = self.scene_epoch.wrapping_add(1);
        self.displayed_view = None;
        self.active_button = None;
        self.last_pointer = None;
        self.last_hover_position = None;
        self.hovered = None;
        self.last_error = None;
        self.request_repaint();
    }

    fn request_repaint(&self) {
        if let Some(context) = &self.repaint_context {
            context.request_repaint();
        }
    }

    fn queue_render(&mut self, job: BackgroundRenderJob3D, repaint: egui::Context) {
        let request = RenderRequest3D {
            job,
            scene_epoch: self.scene_epoch,
            repaint,
        };
        if let Some(scheduled) = self.scheduler.request(request) {
            self.spawn_render(scheduled.id(), scheduled.into_request());
        }
    }

    fn spawn_render(&self, id: ScheduledRequestId, request: RenderRequest3D) {
        if let Err(error) = self.worker.send(WorkerRequest3D { id, request }) {
            let work = error.0;
            let _ = self.completion_tx.send(RenderCompletion3D {
                id: work.id,
                scene_epoch: work.request.scene_epoch,
                result: RenderResult3D::Error(AdapterError::new(
                    AdapterErrorKind::Render,
                    "ruviz-egui 3D render worker is unavailable",
                )),
            });
            work.request.repaint.request_repaint();
        }
    }

    fn drain_completions(&mut self, context: &egui::Context, events: &mut Vec<Plot3DEvent>) {
        while let Ok(completed) = self.completion_rx.try_recv() {
            self.handle_completion(context, events, completed);
        }
    }

    fn drain_save_completions(&mut self, events: &mut Vec<Plot3DEvent>) {
        while let Ok(result) = self.save_completion_rx.try_recv() {
            if let Err(error) = result {
                self.last_error = Some(error.clone());
                events.push(Plot3DEvent::Error(error));
            }
        }
    }

    fn handle_completion(
        &mut self,
        context: &egui::Context,
        events: &mut Vec<Plot3DEvent>,
        completed: RenderCompletion3D,
    ) {
        let Some(state) = self.scheduler.complete(completed.id) else {
            return;
        };
        if state.install && completed.scene_epoch == self.scene_epoch {
            match completed.result {
                RenderResult3D::Frame(frame) => match self.session.classify_render(frame) {
                    BackgroundRenderOutcome3D::Current(frame) => {
                        self.displayed_view = Some(frame.stamp.view());
                        self.install_image(context, frame.image);
                        self.last_error = None;
                    }
                    BackgroundRenderOutcome3D::Superseded { .. } => {}
                },
                RenderResult3D::Error(error) => self.record_error(error, events),
            }
        }
        if let Some(next) = state.next {
            self.spawn_render(next.id(), next.into_request());
        }
    }

    fn install_image(&mut self, context: &egui::Context, image: Image) {
        self.image_size = Some((image.width, image.height));
        let image = Arc::new(image);
        // The 3D renderer already emits straight alpha, so this wraps without
        // converting; `upload_texture` picks the matching egui constructor.
        upload_texture(
            context,
            &mut self.texture,
            || format!("ruviz-egui-3d-{:?}", self.id),
            &RenderedLayer::from_straight_image(Arc::clone(&image)),
        );
        self.installed_image = Some(image);
    }

    fn process_input(
        &mut self,
        ui: &Ui,
        response: &Response,
        content: egui::Rect,
        visible_content: egui::Rect,
        image_size: (u32, u32),
    ) -> InputOutcome3D {
        let mut outcome = InputOutcome3D {
            hovered: self.hovered,
            ..InputOutcome3D::default()
        };
        let pointer = response.interact_pointer_pos();

        if response.clicked() {
            response.request_focus();
        }
        if response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Escape)) {
            if self.active_button.is_some() {
                self.cancel_active_drag(&mut outcome);
            } else {
                self.session.cancel_drag();
            }
            self.apply_input(InputEvent3D::Escape, &mut outcome);
            outcome.events.push(Plot3DEvent::Reset);
        }

        if let Some(position) = response
            .hover_pos()
            .filter(|position| visible_content.contains(*position))
        {
            if let Some((x, y)) = map_point(content, position, image_size) {
                let key = ((x as f32).to_bits(), (y as f32).to_bits());
                if self.last_hover_position != Some(key) && self.active_button.is_none() {
                    match self.session.pick(x as f32, y as f32) {
                        Ok(hit) => {
                            self.hovered = hit;
                            outcome.hovered = hit;
                            outcome.events.push(Plot3DEvent::Hovered(hit));
                        }
                        Err(error) => self.record_input_error(error, &mut outcome),
                    }
                    self.last_hover_position = Some(key);
                }
                let scroll_y = claim_scroll_y(ui);
                if scroll_y != 0.0 {
                    self.apply_input(InputEvent3D::Wheel { delta_y: scroll_y }, &mut outcome);
                }
            }
        } else if self.last_hover_position.take().is_some() {
            self.hovered = None;
            outcome.hovered = None;
            outcome.events.push(Plot3DEvent::Hovered(None));
        }

        if response.double_clicked_by(PointerButton::Primary) {
            if self.active_button.is_some() {
                self.cancel_active_drag(&mut outcome);
            }
            if let Some((x, y)) = pointer.map(|position| {
                let mapped = map_point_clamped(content, position, image_size);
                (mapped.0 as f32, mapped.1 as f32)
            }) {
                self.apply_input(
                    InputEvent3D::DoubleClick {
                        x,
                        y,
                        button: PointerButton3D::Left,
                    },
                    &mut outcome,
                );
                outcome.events.push(Plot3DEvent::Reset);
            }
        }

        for (egui_button, ruviz_button) in [
            (PointerButton::Primary, PointerButton3D::Left),
            (PointerButton::Middle, PointerButton3D::Middle),
            (PointerButton::Secondary, PointerButton3D::Right),
        ] {
            let (pressed, press_origin) = ui.input(|input| {
                (
                    input.pointer.button_pressed(egui_button),
                    input.pointer.press_origin(),
                )
            });
            if self.active_button.is_none()
                && pressed
                && response.is_pointer_button_down_on()
                && press_starts_in(visible_content, press_origin)
            {
                let Some(origin) = press_origin else {
                    continue;
                };
                response.request_focus();
                self.begin_pointer_interaction(
                    origin,
                    content,
                    image_size,
                    ruviz_button,
                    &mut outcome,
                );
            }
            if response.dragged_by(egui_button)
                && self.active_button == Some(ruviz_button)
                && let Some(position) = pointer
            {
                let (x, y) = map_point_clamped(content, position, image_size);
                self.apply_input(
                    InputEvent3D::PointerMove {
                        x: x as f32,
                        y: y as f32,
                    },
                    &mut outcome,
                );
                self.last_pointer = Some(position);
            }
        }

        if let Some(button) = self.active_button {
            let (down, focused) = ui.input(|input| {
                (
                    input.pointer.button_down(match button {
                        PointerButton3D::Left => PointerButton::Primary,
                        PointerButton3D::Middle => PointerButton::Middle,
                        PointerButton3D::Right => PointerButton::Secondary,
                    }),
                    input.focused,
                )
            });
            if !down || !focused {
                if let Some(position) =
                    pointer.filter(|position| focused && visible_content.contains(*position))
                {
                    self.finish_drag(position, content, image_size, button, &mut outcome);
                } else {
                    self.cancel_active_drag(&mut outcome);
                }
            }
        }

        outcome
    }

    fn begin_pointer_interaction(
        &mut self,
        origin: egui::Pos2,
        content: egui::Rect,
        image_size: (u32, u32),
        button: PointerButton3D,
        outcome: &mut InputOutcome3D,
    ) {
        let (x, y) = map_point_clamped(content, origin, image_size);
        self.active_button = Some(button);
        self.last_pointer = Some(origin);
        self.apply_input(
            InputEvent3D::PointerDown {
                x: x as f32,
                y: y as f32,
                button,
            },
            outcome,
        );
    }

    fn finish_drag(
        &mut self,
        pointer: egui::Pos2,
        content: egui::Rect,
        image_size: (u32, u32),
        button: PointerButton3D,
        outcome: &mut InputOutcome3D,
    ) {
        let (x, y) = map_point_clamped(content, pointer, image_size);
        self.apply_input(
            InputEvent3D::PointerUp {
                x: x as f32,
                y: y as f32,
                button,
            },
            outcome,
        );
        self.active_button = None;
        self.last_pointer = None;
    }

    fn cancel_active_drag(&mut self, outcome: &mut InputOutcome3D) {
        self.session.cancel_drag();
        self.active_button = None;
        self.last_pointer = None;
        outcome.events.push(Plot3DEvent::DragCancelled);
    }

    fn apply_input(&mut self, event: InputEvent3D, outcome: &mut InputOutcome3D) {
        match self.session.handle_input(event) {
            Ok(result) => {
                if result.camera_changed {
                    if self.hovered.take().is_some() {
                        self.last_hover_position = None;
                        outcome.hovered = None;
                        outcome.events.push(Plot3DEvent::Hovered(None));
                    }
                    outcome.camera_changed = true;
                    outcome
                        .events
                        .push(Plot3DEvent::CameraChanged(self.session.camera_snapshot()));
                }
                if let Some(hit) = result.picked {
                    outcome.picked = Some(hit);
                    outcome.events.push(Plot3DEvent::Picked(hit));
                }
            }
            Err(error) => self.record_input_error(error, outcome),
        }
    }

    fn apply_context_menu_action(
        &mut self,
        action: PlotContextMenuAction,
        context: &egui::Context,
        outcome: &mut InputOutcome3D,
    ) {
        match action {
            PlotContextMenuAction::ResetView => {
                if self.active_button.is_some() {
                    self.cancel_active_drag(outcome);
                }
                if self.apply_camera_mutation(outcome, |session| session.reset_view()) {
                    outcome.events.push(Plot3DEvent::Reset);
                }
            }
            PlotContextMenuAction::FitToContent => {
                self.apply_camera_mutation(outcome, InteractivePlot3DSession::fit_to_content);
            }
            PlotContextMenuAction::SaveImage => {
                if let Some(image) = &self.installed_image
                    && let Err(error) = spawn_png_save(
                        Arc::clone(image),
                        "ruviz-plot-3d.png",
                        self.save_completion_tx.clone(),
                        context.clone(),
                    )
                {
                    self.last_error = Some(error.clone());
                    outcome.events.push(Plot3DEvent::Error(error));
                }
            }
            PlotContextMenuAction::CopyImage => {
                if let Some(image) = &self.installed_image {
                    copy_image_to_clipboard(context, image);
                }
            }
            PlotContextMenuAction::ToggleInteraction => {
                let mode = match self.mode {
                    ViewMode::Static => ViewMode::Interactive,
                    ViewMode::Interactive => ViewMode::Static,
                };
                self.set_mode(mode);
            }
            PlotContextMenuAction::CameraView(view) => {
                self.apply_camera_mutation(outcome, |session| session.apply_camera_view(view));
            }
            _ => {}
        }
        outcome.events.push(Plot3DEvent::ContextMenuAction(action));
    }

    fn record_camera_change(&mut self, outcome: &mut InputOutcome3D) {
        self.hovered = None;
        self.last_hover_position = None;
        outcome.hovered = None;
        outcome.camera_changed = true;
        outcome
            .events
            .push(Plot3DEvent::CameraChanged(self.session.camera_snapshot()));
    }

    fn apply_camera_mutation(
        &mut self,
        outcome: &mut InputOutcome3D,
        apply: impl FnOnce(&mut InteractivePlot3DSession) -> ruviz::core::Result<()>,
    ) -> bool {
        let before_camera = self.session.camera_snapshot();
        let before_view = self.session.view_stamp();
        if let Err(error) = apply(&mut self.session) {
            self.record_input_error(error, outcome);
            return false;
        }
        let after_camera = self.session.camera_snapshot();
        let after_view = self.session.view_stamp();
        let camera_changed = before_camera.camera != after_camera.camera;
        let view_changed = !before_view.same_camera(after_view);
        debug_assert_eq!(camera_changed, view_changed);
        if camera_changed && view_changed {
            self.record_camera_change(outcome);
            true
        } else {
            false
        }
    }

    fn record_input_error(
        &mut self,
        error: ruviz::core::PlottingError,
        outcome: &mut InputOutcome3D,
    ) {
        let error = AdapterError::new(AdapterErrorKind::Interaction, error);
        self.last_error = Some(error.clone());
        outcome.events.push(Plot3DEvent::Error(error));
    }

    fn record_error(&mut self, error: AdapterError, events: &mut Vec<Plot3DEvent>) {
        self.last_error = Some(error.clone());
        events.push(Plot3DEvent::Error(error));
    }
}

fn spawn_render_worker(
    sender: Sender<RenderCompletion3D>,
) -> std::io::Result<RenderWorker<WorkerRequest3D>> {
    RenderWorker::spawn(
        "ruviz-egui-3d-render",
        move |receiver: Receiver<WorkerRequest3D>| {
            let mut renderer = BackgroundRenderer3D::new(worker_backend());
            while let Ok(work) = receiver.recv() {
                // This worker is spawned once per widget, so a panicking render
                // must be contained: unwinding here would wedge the scheduler's
                // in-flight slot forever with no way to respawn the lane.
                let result = match catch_render_panic(|| renderer.render(work.request.job)) {
                    Ok(Ok(frame)) => RenderResult3D::Frame(frame),
                    Ok(Err(error)) => {
                        RenderResult3D::Error(AdapterError::new(AdapterErrorKind::Render, error))
                    }
                    Err(message) => {
                        // The renderer's state is unknown after a panic.
                        renderer = BackgroundRenderer3D::new(worker_backend());
                        RenderResult3D::Error(AdapterError::new(
                            AdapterErrorKind::Render,
                            format!("ruviz-egui 3D render panicked: {message}"),
                        ))
                    }
                };
                if sender
                    .send(RenderCompletion3D {
                        id: work.id,
                        scene_epoch: work.request.scene_epoch,
                        result,
                    })
                    .is_err()
                {
                    break;
                }
                work.request.repaint.request_repaint();
            }
        },
    )
}

const fn worker_backend() -> BackgroundRenderBackend3D {
    #[cfg(all(feature = "3d-gpu", not(target_arch = "wasm32")))]
    {
        BackgroundRenderBackend3D::GpuReadback
    }
    #[cfg(not(all(feature = "3d-gpu", not(target_arch = "wasm32"))))]
    {
        BackgroundRenderBackend3D::Cpu
    }
}

fn plot_sense(mode: ViewMode) -> Sense {
    match mode {
        ViewMode::Static => Sense::click(),
        ViewMode::Interactive => Sense::click_and_drag(),
    }
}

fn plot_context_menu_action(
    response: &Response,
    mode: ViewMode,
    has_image: bool,
) -> Option<PlotContextMenuAction> {
    let mut selected = None;
    response.context_menu(|ui| {
        select_context_action(
            ui,
            &mut selected,
            true,
            "Reset View",
            PlotContextMenuAction::ResetView,
        );
        select_context_action(
            ui,
            &mut selected,
            true,
            "Fit to Content",
            PlotContextMenuAction::FitToContent,
        );
        ui.separator();
        ui.menu_button("Camera View", |ui| {
            for &(label, view) in CAMERA_VIEW_ACTIONS {
                select_context_action(
                    ui,
                    &mut selected,
                    true,
                    label,
                    PlotContextMenuAction::CameraView(view),
                );
            }
        });
        ui.separator();
        select_context_action(
            ui,
            &mut selected,
            has_image,
            "Save PNG…",
            PlotContextMenuAction::SaveImage,
        );
        select_context_action(
            ui,
            &mut selected,
            has_image,
            "Copy Image",
            PlotContextMenuAction::CopyImage,
        );
        ui.separator();
        let toggle_label = match mode {
            ViewMode::Static => "Enable Interaction",
            ViewMode::Interactive => "Disable Interaction",
        };
        select_context_action(
            ui,
            &mut selected,
            true,
            toggle_label,
            PlotContextMenuAction::ToggleInteraction,
        );
    });
    selected
}

const CAMERA_VIEW_ACTIONS: &[(&str, CameraView3D)] = &[
    ("Isometric", CameraView3D::Isometric),
    ("Front", CameraView3D::Front),
    ("Back", CameraView3D::Back),
    ("Left", CameraView3D::Left),
    ("Right", CameraView3D::Right),
    ("Top", CameraView3D::Top),
    ("Bottom", CameraView3D::Bottom),
];

fn select_context_action(
    ui: &mut Ui,
    selected: &mut Option<PlotContextMenuAction>,
    enabled: bool,
    label: &str,
    action: PlotContextMenuAction,
) {
    if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
        *selected = Some(action);
        ui.close();
    }
}

#[derive(Default)]
struct InputOutcome3D {
    picked: Option<PickHit3D>,
    hovered: Option<PickHit3D>,
    camera_changed: bool,
    events: Vec<Plot3DEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plot() -> impl TryIntoPlot3DSession {
        ruviz::scatter3d(
            &[0.0_f64, 1.0, 2.0],
            &[0.0_f64, 1.0, 0.0],
            &[0.0_f64, 0.5, 1.0],
        )
    }

    #[test]
    fn builder_supports_static_interactive_and_fixed_size() {
        let static_plot = plot3d_builder(plot())
            .static_view()
            .fixed_pixels(640.0, 360.0)
            .build()
            .unwrap();
        assert_eq!(static_plot.mode(), ViewMode::Static);
        assert_eq!(
            plot3d_builder(plot())
                .interactive()
                .fill()
                .build()
                .unwrap()
                .mode(),
            ViewMode::Interactive
        );
    }

    #[test]
    fn keep_view_preserves_camera() {
        let mut widget = plot3d_builder(plot()).build().unwrap();
        widget.session.orbit(20.0, 10.0).unwrap();
        let camera = widget.session.camera();
        widget.set_plot_keep_view(plot()).unwrap();
        assert_eq!(widget.session.camera(), camera);
    }

    #[test]
    fn replacement_supersedes_the_requested_view_without_dropping_last_image() {
        let mut widget = plot3d_builder(plot()).build().unwrap();
        widget.image_size = Some((320, 200));
        widget.last_requested_view = Some(widget.session.view_stamp());
        widget.scene_epoch = u64::MAX;
        widget.set_plot(plot()).unwrap();
        assert!(widget.last_requested_view.is_none());
        assert_eq!(widget.image_size, Some((320, 200)));
        assert_eq!(widget.scene_epoch, 0);
    }

    #[test]
    fn replacement_keeps_the_serial_worker_slot_and_coalesces_new_work() {
        let mut widget = plot3d_builder(plot()).build().unwrap();
        let repaint = egui::Context::default();
        assert!(
            widget
                .scheduler
                .request(RenderRequest3D {
                    job: widget.session.background_render_job().unwrap(),
                    scene_epoch: widget.scene_epoch,
                    repaint: repaint.clone(),
                })
                .is_some()
        );

        widget.set_plot(plot()).unwrap();

        assert!(!widget.scheduler.is_idle());
        assert!(
            widget
                .scheduler
                .request(RenderRequest3D {
                    job: widget.session.background_render_job().unwrap(),
                    scene_epoch: widget.scene_epoch,
                    repaint,
                })
                .is_none()
        );
    }

    #[test]
    fn request_generation_exhaustion_does_not_clear_adapter_state() {
        let mut widget = plot3d_builder(plot()).build().unwrap();
        let view = widget.session.view_stamp();
        let pointer = egui::pos2(12.0, 34.0);
        widget.scene_epoch = 41;
        widget.last_requested_view = Some(view);
        widget.displayed_view = Some(view);
        widget.active_button = Some(PointerButton3D::Left);
        widget.last_pointer = Some(pointer);
        widget.last_hover_position = Some((12, 34));
        widget.last_error = Some(AdapterError::new(AdapterErrorKind::Render, "old error"));

        let error = widget
            .complete_replacement(Err(ruviz::core::PlottingError::RenderError(
                "3D render request space was exhausted during replacement".to_string(),
            )))
            .expect_err("request-generation exhaustion must be propagated");

        assert!(error.to_string().contains("request space was exhausted"));
        assert_eq!(widget.session.view_stamp(), view);
        assert_eq!(widget.scene_epoch, 41);
        assert_eq!(widget.last_requested_view, Some(view));
        assert_eq!(widget.displayed_view, Some(view));
        assert_eq!(widget.active_button, Some(PointerButton3D::Left));
        assert_eq!(widget.last_pointer, Some(pointer));
        assert_eq!(widget.last_hover_position, Some((12, 34)));
        assert_eq!(
            widget.last_error.as_ref().map(AdapterError::message),
            Some("old error")
        );
    }

    #[test]
    fn retry_render_clears_unchanged_view_sentinel_and_error() {
        let mut widget = plot3d_builder(plot()).build().unwrap();
        widget.last_requested_view = Some(widget.session.view_stamp());
        widget.last_error = Some(AdapterError::new(AdapterErrorKind::Render, "failed"));
        widget.retry_render();
        assert!(widget.last_requested_view.is_none());
        assert!(widget.last_error.is_none());
    }

    #[test]
    fn background_job_is_send_and_does_not_borrow_the_widget() {
        fn assert_send<T: Send>(_: &T) {}
        let mut widget = plot3d_builder(plot()).build().unwrap();
        let job = widget.session.background_render_job().unwrap();
        assert_send(&job);
    }

    #[cfg(all(feature = "3d-gpu", not(target_arch = "wasm32")))]
    #[test]
    fn gpu_readback_backend_installs_a_deterministic_worker_result() {
        assert_eq!(worker_backend(), BackgroundRenderBackend3D::GpuReadback);
        let mut widget = plot3d_builder(plot()).build().unwrap();
        let context = egui::Context::default();
        let job = widget.session.background_render_job().unwrap();
        let stamp = job.stamp();
        let scheduled = widget
            .scheduler
            .request(RenderRequest3D {
                job,
                scene_epoch: widget.scene_epoch,
                repaint: context.clone(),
            })
            .unwrap();
        let id = scheduled.id();
        drop(scheduled);
        let image = Image::new(
            4,
            3,
            (0..12)
                .flat_map(|index| [index as u8, 80, 160, 255])
                .collect(),
        );

        widget.handle_completion(
            &context,
            &mut Vec::new(),
            RenderCompletion3D {
                id,
                scene_epoch: widget.scene_epoch,
                result: RenderResult3D::Frame(RenderedImage3D { image, stamp }),
            },
        );

        assert_eq!(widget.displayed_view, Some(stamp.view()));
        assert_eq!(widget.image_size, Some((4, 3)));
        assert_eq!(
            widget.texture.as_ref().map(TextureHandle::size),
            Some([4, 3])
        );
    }

    #[test]
    fn pointer_down_is_forwarded_at_the_original_press_before_drag_thresholds() {
        let mut widget = plot3d_builder(plot()).build().unwrap();
        let content = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(200.0, 100.0));
        let origin = egui::pos2(30.0, 40.0);
        let mut outcome = InputOutcome3D::default();
        widget.begin_pointer_interaction(
            origin,
            content,
            (400, 200),
            PointerButton3D::Left,
            &mut outcome,
        );
        assert!(widget.session.is_drag_active());
        assert_eq!(widget.active_button, Some(PointerButton3D::Left));
        assert_eq!(widget.last_pointer, Some(origin));
    }

    #[test]
    fn escape_path_cancels_drag_before_reset() {
        let mut widget = plot3d_builder(plot()).build().unwrap();
        let content = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(200.0, 100.0));
        let mut outcome = InputOutcome3D::default();
        widget.begin_pointer_interaction(
            content.center(),
            content,
            (400, 200),
            PointerButton3D::Left,
            &mut outcome,
        );
        widget.cancel_active_drag(&mut outcome);
        widget.apply_input(InputEvent3D::Escape, &mut outcome);
        assert!(!widget.session.is_drag_active());
        assert!(widget.active_button.is_none());
        assert!(outcome.events.contains(&Plot3DEvent::DragCancelled));
    }

    #[test]
    fn static_mode_still_senses_context_clicks_without_dragging() {
        let sense = plot_sense(ViewMode::Static);
        assert!(sense.senses_click());
        assert!(!sense.senses_drag());
        assert!(plot_sense(ViewMode::Interactive).senses_drag());
    }

    #[test]
    fn context_menu_exposes_every_named_camera_view() {
        assert_eq!(
            CAMERA_VIEW_ACTIONS,
            &[
                ("Isometric", CameraView3D::Isometric),
                ("Front", CameraView3D::Front),
                ("Back", CameraView3D::Back),
                ("Left", CameraView3D::Left),
                ("Right", CameraView3D::Right),
                ("Top", CameraView3D::Top),
                ("Bottom", CameraView3D::Bottom),
            ]
        );
    }

    #[test]
    fn context_actions_toggle_static_mode_and_apply_named_camera_views() {
        let mut widget = plot3d_builder(plot()).static_view().build().unwrap();
        let context = egui::Context::default();
        let mut outcome = InputOutcome3D::default();
        let initial_camera = widget.session.camera_snapshot();

        widget.apply_context_menu_action(
            PlotContextMenuAction::ToggleInteraction,
            &context,
            &mut outcome,
        );
        assert_eq!(widget.mode(), ViewMode::Interactive);
        assert_eq!(
            outcome.events,
            vec![Plot3DEvent::ContextMenuAction(
                PlotContextMenuAction::ToggleInteraction
            )]
        );

        outcome = InputOutcome3D::default();
        widget.apply_context_menu_action(
            PlotContextMenuAction::CameraView(CameraView3D::Top),
            &context,
            &mut outcome,
        );
        assert!(outcome.camera_changed);
        assert!(matches!(
            outcome.events.as_slice(),
            [
                Plot3DEvent::CameraChanged(_),
                Plot3DEvent::ContextMenuAction(PlotContextMenuAction::CameraView(
                    CameraView3D::Top
                ))
            ]
        ));

        outcome = InputOutcome3D::default();
        widget.apply_context_menu_action(PlotContextMenuAction::ResetView, &context, &mut outcome);
        assert_eq!(
            widget.session.camera_snapshot().camera,
            initial_camera.camera
        );
        assert!(matches!(
            outcome.events.as_slice(),
            [
                Plot3DEvent::CameraChanged(_),
                Plot3DEvent::Reset,
                Plot3DEvent::ContextMenuAction(PlotContextMenuAction::ResetView)
            ]
        ));
    }

    #[test]
    fn repeated_context_camera_actions_do_not_report_a_change() {
        for action in [
            PlotContextMenuAction::ResetView,
            PlotContextMenuAction::FitToContent,
            PlotContextMenuAction::CameraView(CameraView3D::Top),
        ] {
            let mut widget = plot3d_builder(plot()).build().unwrap();
            let context = egui::Context::default();
            widget.apply_context_menu_action(action, &context, &mut InputOutcome3D::default());
            let before_repeat = widget.session.view_stamp();
            let mut repeated = InputOutcome3D::default();

            widget.apply_context_menu_action(action, &context, &mut repeated);

            assert!(!repeated.camera_changed, "{action:?} should be idempotent");
            assert_eq!(widget.session.view_stamp(), before_repeat);
            assert!(
                !repeated
                    .events
                    .iter()
                    .any(|event| matches!(event, Plot3DEvent::CameraChanged(_)))
            );
            assert!(
                !repeated.events.iter().any(plot3d_event_marks_changed),
                "{action:?} should not mark the egui response changed"
            );
        }
    }

    #[test]
    fn installed_frame_is_retained_for_menu_export_without_a_render() {
        let mut widget = plot3d_builder(plot()).build().unwrap();
        let context = egui::Context::default();
        let image = Image::new(2, 1, vec![1, 2, 3, 255, 4, 5, 6, 128]);

        widget.install_image(&context, image.clone());

        let installed = widget.installed_image.as_ref().unwrap();
        let export_handle = Arc::clone(installed);
        assert_eq!((installed.width, installed.height), (2, 1));
        assert_eq!(installed.pixels, image.pixels);
        assert!(Arc::ptr_eq(installed, &export_handle));
        assert!(widget.scheduler.is_idle());
    }

    #[test]
    fn asynchronous_save_failures_are_reported_on_the_ui_thread() {
        let mut widget = plot3d_builder(plot()).build().unwrap();
        widget
            .save_completion_tx
            .send(Err(AdapterError::new(
                AdapterErrorKind::Interaction,
                "save failed",
            )))
            .unwrap();
        let mut events = Vec::new();

        widget.drain_save_completions(&mut events);

        assert_eq!(
            widget.last_error().map(AdapterError::message),
            Some("save failed")
        );
        assert!(matches!(events.as_slice(), [Plot3DEvent::Error(_)]));
    }
}
