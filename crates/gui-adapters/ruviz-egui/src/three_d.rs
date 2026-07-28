use std::sync::mpsc::{self, Receiver, Sender};

use egui::{Id, PointerButton, Response, Sense, TextureHandle, TextureOptions, Ui};
use ruviz::core::{
    BackgroundRenderBackend3D, BackgroundRenderJob3D, BackgroundRenderOutcome3D,
    BackgroundRenderer3D, CameraSnapshot3D, Image, ImageFit, InputEvent3D,
    InteractivePlot3DSession, LatestRequestScheduler, PickHit3D, PointerButton3D, RenderedImage3D,
    ScheduledRequestId, TryIntoPlot3DSession, ViewStamp3D, physical_backing_size,
};

use crate::shared::{
    AdapterError, AdapterErrorKind, PlotSize, ViewMode, claim_scroll_y, color_image, fitted_rect,
    map_point, map_point_clamped, next_widget_id, paint_texture, press_starts_in,
    release_is_cancelled, visible_content_rect,
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
#[derive(Clone, Debug, PartialEq)]
pub enum Plot3DEvent {
    Hovered(Option<PickHit3D>),
    Picked(PickHit3D),
    CameraChanged(CameraSnapshot3D),
    DragCancelled,
    Reset,
    Error(AdapterError),
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
        self.camera_changed || !self.events.is_empty()
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
    image_size: Option<(u32, u32)>,
    displayed_view: Option<ViewStamp3D>,
    scene_epoch: u64,
    scheduler: LatestRequestScheduler<RenderRequest3D>,
    worker_tx: Sender<WorkerRequest3D>,
    completion_tx: Sender<RenderCompletion3D>,
    completion_rx: Receiver<RenderCompletion3D>,
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
        let (worker_tx, worker_rx) = mpsc::channel();
        spawn_render_worker(worker_rx, completion_tx.clone()).map_err(|error| {
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
            image_size: None,
            displayed_view: None,
            scene_epoch: 0,
            scheduler: LatestRequestScheduler::default(),
            worker_tx,
            completion_tx,
            completion_rx,
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
        self.session.replace(replacement);
        self.after_replacement();
        Ok(())
    }

    /// Replace the scene while preserving the current camera.
    pub fn set_plot_keep_view(
        &mut self,
        plot: impl TryIntoPlot3DSession,
    ) -> ruviz::core::Result<()> {
        let replacement = plot.try_into_plot3d_session()?;
        self.session.replace_keep_camera(replacement)?;
        self.after_replacement();
        Ok(())
    }

    /// Present the plot. Image rendering is always performed by a worker.
    pub fn show(&mut self, ui: &mut Ui) -> Plot3DResponse {
        self.repaint_context = Some(ui.ctx().clone());
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
        let content = fitted_rect(outer, frame_size, self.fit);
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

        if camera_changed || !events.is_empty() {
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
        self.scene_epoch = self
            .scene_epoch
            .checked_add(1)
            .expect("ruviz-egui 3D replacement epoch exhausted");
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
        if let Err(error) = self.worker_tx.send(WorkerRequest3D { id, request }) {
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
            let Some(state) = self.scheduler.complete(completed.id) else {
                continue;
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
    }

    fn install_image(&mut self, context: &egui::Context, image: Image) {
        self.image_size = Some((image.width, image.height));
        let color = color_image(&image);
        if let Some(texture) = &mut self.texture {
            texture.set(color, TextureOptions::LINEAR);
        } else {
            self.texture = Some(context.load_texture(
                format!("ruviz-egui-3d-{:?}", self.id),
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
                let origin = press_origin.expect("visible pointer origin was checked above");
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
                if !release_is_cancelled(visible_content, pointer, focused) {
                    self.finish_drag(
                        pointer.expect("known visible release was checked above"),
                        content,
                        image_size,
                        button,
                        &mut outcome,
                    );
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
    receiver: Receiver<WorkerRequest3D>,
    sender: Sender<RenderCompletion3D>,
) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("ruviz-egui-3d-render".to_string())
        .spawn(move || {
            #[cfg(all(feature = "3d-gpu", not(target_arch = "wasm32")))]
            let backend = BackgroundRenderBackend3D::GpuReadback;
            #[cfg(not(all(feature = "3d-gpu", not(target_arch = "wasm32"))))]
            let backend = BackgroundRenderBackend3D::Cpu;
            let mut renderer = BackgroundRenderer3D::new(backend);
            while let Ok(work) = receiver.recv() {
                let result = match renderer.render(work.request.job) {
                    Ok(frame) => RenderResult3D::Frame(frame),
                    Err(error) => {
                        RenderResult3D::Error(AdapterError::new(AdapterErrorKind::Render, error))
                    }
                };
                let _ = sender.send(RenderCompletion3D {
                    id: work.id,
                    scene_epoch: work.request.scene_epoch,
                    result,
                });
                work.request.repaint.request_repaint();
            }
        })
        .map(|_| ())
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
        widget.set_plot(plot()).unwrap();
        assert!(widget.last_requested_view.is_none());
        assert_eq!(widget.image_size, Some((320, 200)));
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
}
