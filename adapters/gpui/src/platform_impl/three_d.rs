use super::*;

use ruviz::core::{
    BackgroundRenderBackend3D, BackgroundRenderJob3D, BackgroundRenderOutcome3D,
    BackgroundRenderer3D, Camera3D, CameraSnapshot3D, InputEvent3D, InteractivePlot3DSession,
    PickHit3D, PlottingError, PointerButton3D, RenderStamp3D, StampedPick3D, TryIntoPlot3DSession,
    ViewStamp3D,
    adapter::{
        ImageFit as CoreImageFit, LatestRequestScheduler, LogicalPoint, LogicalRect,
        ScheduledRequest, ScheduledRequestId, fitted_content_rect, logical_to_physical,
        physical_backing_size,
    },
};

type Plot3DPickHandler = Arc<dyn Fn(PickHit3D) + Send + Sync>;
type Plot3DErrorHandler = Arc<dyn Fn(PlottingError) + Send + Sync>;

/// Events emitted by [`RuvizPlot3D`].
///
/// Builder callbacks are convenient thread-safe observers. GPUI views that
/// update host UI state should normally subscribe to the entity so they receive
/// a normal GPUI context.
#[derive(Clone, Debug)]
pub enum Plot3DEvent {
    Pick(PickHit3D),
    CameraChanged(CameraSnapshot3D),
    Error(PlottingError),
}

/// Options shared by the GPUI 3D builder and retained view.
#[derive(Clone, Debug, PartialEq)]
pub struct RuvizPlot3DOptions {
    pub sizing_policy: SizingPolicy,
    pub image_fit: ImageFit,
    pub interactive: bool,
    /// Worker-owned image rendering backend.
    ///
    /// GPU selection still reads the rendered frame back for GPUI image upload.
    pub render_backend: BackgroundRenderBackend3D,
}

impl Default for RuvizPlot3DOptions {
    fn default() -> Self {
        Self {
            sizing_policy: SizingPolicy::Fill,
            image_fit: ImageFit::Contain,
            interactive: true,
            render_backend: default_background_backend(),
        }
    }
}

/// Fluent builder for a GPUI 3D plot entity.
pub struct RuvizPlot3DBuilder<P> {
    plot: P,
    options: RuvizPlot3DOptions,
    on_pick: Option<Plot3DPickHandler>,
    on_error: Option<Plot3DErrorHandler>,
}

impl<P> RuvizPlot3DBuilder<P>
where
    P: TryIntoPlot3DSession + 'static,
{
    fn new(plot: P) -> Self {
        Self {
            plot,
            options: RuvizPlot3DOptions::default(),
            on_pick: None,
            on_error: None,
        }
    }

    /// Enable orbit, pan, zoom, reset, and picking.
    pub fn interactive(mut self) -> Self {
        self.options.interactive = true;
        self
    }

    /// Render responsively while ignoring user input.
    pub fn static_view(mut self) -> Self {
        self.options.interactive = false;
        self
    }

    /// Fill the parent-provided logical bounds.
    pub fn fill(mut self) -> Self {
        self.options.sizing_policy = SizingPolicy::Fill;
        self
    }

    /// Use fixed logical dimensions. The backing image is still scaled for
    /// fractional HiDPI.
    pub fn fixed_pixels(mut self, width: u32, height: u32) -> Self {
        self.options.sizing_policy = SizingPolicy::FixedPixels { width, height };
        self
    }

    /// Select how the rendered image is fitted inside widget bounds.
    pub fn image_fit(mut self, image_fit: ImageFit) -> Self {
        self.options.image_fit = image_fit;
        self
    }

    /// Select the worker-owned rendering backend.
    ///
    /// [`BackgroundRenderBackend3D::GpuReadback`] lazily creates and then
    /// retains one GPU renderer on the worker. Presentation remains an
    /// image upload, not zero-copy texture interop.
    pub fn render_backend(mut self, render_backend: BackgroundRenderBackend3D) -> Self {
        self.options.render_backend = render_backend;
        self
    }

    /// Observe successful primary-button picks.
    pub fn on_pick<F>(mut self, handler: F) -> Self
    where
        F: Fn(PickHit3D) + Send + Sync + 'static,
    {
        self.on_pick = Some(Arc::new(handler));
        self
    }

    /// Observe background render, resize, interaction, and construction errors.
    ///
    /// [`Self::try_build`] returns construction errors directly. The
    /// compatibility [`Self::build`] method creates a non-rendering entity and
    /// reports its construction error through this callback and
    /// [`Plot3DEvent::Error`].
    pub fn on_error<F>(mut self, handler: F) -> Self
    where
        F: Fn(PlottingError) + Send + Sync + 'static,
    {
        self.on_error = Some(Arc::new(handler));
        self
    }

    /// Fallibly create the retained GPUI entity.
    pub fn try_build<V>(self, cx: &mut Context<V>) -> Result<Entity<RuvizPlot3D>>
    where
        V: 'static,
    {
        let session = self.plot.try_into_plot3d_session()?;
        let options = self.options;
        let on_pick = self.on_pick;
        let on_error = self.on_error;
        Ok(cx.new(move |cx| RuvizPlot3D::new(session, options, on_pick, on_error, cx)))
    }

    /// Create the retained GPUI entity.
    ///
    /// Invalid plots produce a non-rendering entity that reports the
    /// construction error. Prefer [`Self::try_build`] when the caller can
    /// handle the error synchronously.
    pub fn build<V>(self, cx: &mut Context<V>) -> Entity<RuvizPlot3D>
    where
        V: 'static,
    {
        let Self {
            plot,
            options,
            on_pick,
            on_error,
        } = self;
        match plot.try_into_plot3d_session() {
            Ok(session) => {
                cx.new(move |cx| RuvizPlot3D::new(session, options, on_pick, on_error, cx))
            }
            Err(error) => cx.new(move |cx| {
                RuvizPlot3D::new_with_initial_error(
                    InteractivePlot3DSession::error_placeholder(),
                    options,
                    on_pick,
                    on_error,
                    error,
                    cx,
                )
            }),
        }
    }
}

#[derive(Clone)]
struct RenderRequest3D {
    job: BackgroundRenderJob3D,
    size_px: (u32, u32),
}

#[derive(Clone)]
struct CachedFrame3D {
    image: Arc<RenderImage>,
    stamp: RenderStamp3D,
    size_px: (u32, u32),
}

#[derive(Clone)]
struct PaintFrame3D {
    image: Arc<RenderImage>,
    content_bounds: Bounds<Pixels>,
}

#[derive(Clone, Copy)]
struct InteractionLayout3D {
    content: LogicalRect,
    frame_size_px: (u32, u32),
}

/// GPUI image-backed adapter for a retained ruviz 3D session.
///
/// Rendering is captured on the UI thread as an immutable
/// [`BackgroundRenderJob3D`] and executed on GPUI's background executor. Image
/// construction and presentation happen back on the UI thread. With `3d-gpu`,
/// a worker-owned GPU renderer is retained across jobs, then each frame is read
/// back for GPUI upload. This does not claim direct texture interop or zero-copy
/// presentation.
pub struct RuvizPlot3D {
    session: InteractivePlot3DSession,
    options: RuvizPlot3DOptions,
    cached_frame: Option<CachedFrame3D>,
    scheduler: LatestRequestScheduler<RenderRequest3D>,
    requested_view: Option<ViewStamp3D>,
    in_flight_render: Option<Task<()>>,
    background_renderer: Arc<Mutex<BackgroundRenderer3D>>,
    component_bounds: Option<Bounds<Pixels>>,
    interaction_layout: Option<InteractionLayout3D>,
    focus_handle: FocusHandle,
    focus_subscription: Option<gpui::Subscription>,
    active_pointer_button: Option<PointerButton3D>,
    selected: Option<StampedPick3D>,
    on_pick: Option<Plot3DPickHandler>,
    on_error: Option<Plot3DErrorHandler>,
    construction_error: Option<PlottingError>,
    construction_error_reported: bool,
}

impl RuvizPlot3D {
    fn new(
        session: InteractivePlot3DSession,
        options: RuvizPlot3DOptions,
        on_pick: Option<Plot3DPickHandler>,
        on_error: Option<Plot3DErrorHandler>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::new_inner(session, options, on_pick, on_error, None, cx)
    }

    fn new_with_initial_error(
        session: InteractivePlot3DSession,
        options: RuvizPlot3DOptions,
        on_pick: Option<Plot3DPickHandler>,
        on_error: Option<Plot3DErrorHandler>,
        error: PlottingError,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::new_inner(session, options, on_pick, on_error, Some(error), cx)
    }

    fn new_inner(
        session: InteractivePlot3DSession,
        options: RuvizPlot3DOptions,
        on_pick: Option<Plot3DPickHandler>,
        on_error: Option<Plot3DErrorHandler>,
        construction_error: Option<PlottingError>,
        cx: &mut Context<Self>,
    ) -> Self {
        let background_renderer = Arc::new(Mutex::new(BackgroundRenderer3D::new(
            options.render_backend,
        )));
        Self {
            session,
            options,
            cached_frame: None,
            scheduler: LatestRequestScheduler::default(),
            requested_view: None,
            in_flight_render: None,
            background_renderer,
            component_bounds: None,
            interaction_layout: None,
            focus_handle: cx.focus_handle(),
            focus_subscription: None,
            active_pointer_button: None,
            selected: None,
            on_pick,
            on_error,
            construction_error,
            construction_error_reported: false,
        }
    }

    /// Current retained core session.
    pub fn session(&self) -> &InteractivePlot3DSession {
        &self.session
    }

    /// Mutable retained core session escape hatch.
    ///
    /// Direct mutation cannot notify GPUI automatically. The caller must
    /// invalidate the host entity after the borrow ends. Prefer
    /// [`Self::set_camera`], [`Self::set_plot`], and
    /// [`Self::set_plot_keep_view`], which invalidate and notify correctly.
    #[deprecated(
        note = "Use set_camera, set_plot, or set_plot_keep_view so GPUI invalidation is automatic."
    )]
    pub fn session_mut(&mut self) -> &mut InteractivePlot3DSession {
        self.requested_view = None;
        self.selected = None;
        &mut self.session
    }

    pub fn options(&self) -> &RuvizPlot3DOptions {
        &self.options
    }

    /// Most recent pick, only while its scene, camera, and target are current.
    pub const fn selected(&self) -> Option<PickHit3D> {
        match self.selected {
            Some(pick) => Some(pick.hit),
            None => None,
        }
    }

    /// Current stamped pick for hosts that need to retain and validate it.
    pub fn stamped_pick(&self) -> Option<StampedPick3D> {
        self.selected
            .filter(|pick| self.session.is_stamped_pick_current(pick))
    }

    /// Pointer button currently retained by the adapter drag lifecycle.
    pub const fn active_pointer_button(&self) -> Option<PointerButton3D> {
        self.active_pointer_button
    }

    /// Toggle sphere lighting while retaining camera, selection, and active drag.
    pub fn set_sphere_shading(&mut self, enabled: bool, cx: &mut Context<Self>) -> Result<()> {
        if self.session.set_sphere_shading(enabled)? {
            self.selected = self.session.current_pick();
            self.requested_view = None;
            cx.notify();
        }
        Ok(())
    }

    /// Replace the camera and schedule a fresh frame.
    pub fn set_camera(&mut self, camera: Camera3D, cx: &mut Context<Self>) -> Result<()> {
        let before = self.session.camera_snapshot();
        self.session.set_camera(camera)?;
        self.invalidate_view();
        if self.session.camera_snapshot() != before {
            cx.emit(Plot3DEvent::CameraChanged(self.session.camera_snapshot()));
        }
        cx.notify();
        Ok(())
    }

    /// Reset to the camera supplied by the current plot.
    pub fn reset_view(&mut self, cx: &mut Context<Self>) -> Result<()> {
        let before = self.session.camera_snapshot();
        self.session.reset_view()?;
        self.session.cancel_drag();
        self.invalidate_view();
        if self.session.camera_snapshot() != before {
            cx.emit(Plot3DEvent::CameraChanged(self.session.camera_snapshot()));
        }
        cx.notify();
        Ok(())
    }

    /// Replace the plot and use the replacement's camera.
    pub fn set_plot<P>(&mut self, plot: P, cx: &mut Context<Self>) -> Result<()>
    where
        P: TryIntoPlot3DSession,
    {
        self.session.try_replace(plot.try_into_plot3d_session()?)?;
        self.construction_error = None;
        self.construction_error_reported = false;
        self.invalidate_view();
        cx.notify();
        Ok(())
    }

    /// Replace the plot while retaining the current 3D view (camera).
    pub fn set_plot_keep_view<P>(&mut self, plot: P, cx: &mut Context<Self>) -> Result<()>
    where
        P: TryIntoPlot3DSession,
    {
        self.session
            .replace_keep_camera(plot.try_into_plot3d_session()?)?;
        self.construction_error = None;
        self.construction_error_reported = false;
        self.invalidate_view();
        cx.notify();
        Ok(())
    }

    /// Compatibility spelling for [`Self::set_plot_keep_view`].
    #[deprecated(note = "Use set_plot_keep_view for naming parity with the 2D GPUI adapter.")]
    pub fn set_plot_keep_camera<P>(&mut self, plot: P, cx: &mut Context<Self>) -> Result<()>
    where
        P: TryIntoPlot3DSession,
    {
        self.set_plot_keep_view(plot, cx)
    }

    /// Change between interactive and static behavior.
    pub fn set_interactive(&mut self, interactive: bool, cx: &mut Context<Self>) {
        self.options.interactive = interactive;
        if !interactive {
            self.cancel_drag();
        }
        cx.notify();
    }

    /// Retry the current view after a reported background render error.
    pub fn retry_render(&mut self, cx: &mut Context<Self>) {
        self.requested_view = None;
        cx.notify();
    }

    fn invalidate_view(&mut self) {
        self.requested_view = None;
        self.selected = None;
        self.session.clear_pick();
        self.cancel_drag();
        // Keep the current worker and scheduler alive. Any already-running
        // synchronous backend call must remain the sole in-flight render;
        // prepaint queues one coalesced latest request, while the core render
        // stamp rejects a completion from the previous view or scene.
    }

    fn report_error(&self, error: PlottingError, cx: &mut Context<Self>) {
        if let Some(handler) = &self.on_error {
            handler(error.clone());
        }
        cx.emit(Plot3DEvent::Error(error));
    }

    fn report_pick(&self, hit: PickHit3D, cx: &mut Context<Self>) {
        if let Some(handler) = &self.on_pick {
            handler(hit);
        }
        cx.emit(Plot3DEvent::Pick(hit));
    }

    fn desired_size(bounds: Bounds<Pixels>, scale_factor: f32) -> (u32, u32) {
        physical_backing_size(
            f64::from(bounds.size.width),
            f64::from(bounds.size.height),
            scale_factor,
        )
    }

    fn prepaint(
        &mut self,
        entity: Entity<Self>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<PaintFrame3D> {
        self.component_bounds = Some(bounds);
        if let Some(error) = self.construction_error.clone() {
            if !self.construction_error_reported {
                self.construction_error_reported = true;
                self.report_error(error, cx);
            }
            return self.paint_frame(bounds);
        }
        let scale_factor = window.scale_factor();
        let desired_size = Self::desired_size(bounds, scale_factor);
        let before_resize = self.session.view_stamp();
        if let Err(error) = self
            .session
            .resize(desired_size.0, desired_size.1, scale_factor)
        {
            self.report_error(error, cx);
            return self.paint_frame(bounds);
        }
        if !before_resize.same_target(self.session.view_stamp()) {
            self.cancel_drag();
        }

        if self
            .selected
            .is_some_and(|pick| !self.session.is_stamped_pick_current(&pick))
        {
            self.selected = None;
        }

        let desired_view = self.session.view_stamp();
        let cached_is_current = self
            .cached_frame
            .as_ref()
            .is_some_and(|frame| frame.stamp.view() == desired_view);
        if !cached_is_current && self.requested_view != Some(desired_view) {
            match self.session.background_render_job() {
                Ok(job) => {
                    let request = RenderRequest3D {
                        job,
                        size_px: desired_size,
                    };
                    self.requested_view = Some(desired_view);
                    if let Some(scheduled) = self.scheduler.request(request) {
                        self.start_render(entity, scheduled, cx);
                    }
                }
                Err(error) => self.report_error(error, cx),
            }
        }

        self.paint_frame(bounds)
    }

    fn paint_frame(&mut self, bounds: Bounds<Pixels>) -> Option<PaintFrame3D> {
        let frame = self.cached_frame.as_ref()?;
        let content = fit_bounds(bounds, frame.size_px, self.options.image_fit);
        self.interaction_layout = Some(InteractionLayout3D {
            content: logical_rect(content),
            frame_size_px: frame.size_px,
        });
        Some(PaintFrame3D {
            image: Arc::clone(&frame.image),
            content_bounds: content,
        })
    }

    fn start_render(
        &mut self,
        entity: Entity<Self>,
        scheduled: ScheduledRequest<RenderRequest3D>,
        cx: &mut Context<Self>,
    ) {
        let request_id = scheduled.id();
        let request = scheduled.into_request();
        let size_px = request.size_px;
        let job = request.job;
        let render_stamp = job.stamp();
        let background_renderer = Arc::clone(&self.background_renderer);
        let background = cx.background_executor().spawn(async move {
            background_renderer
                .lock()
                .map_err(|_| {
                    PlottingError::RenderError(
                        "GPUI 3D background renderer lock was poisoned".to_string(),
                    )
                })?
                .render(job)
        });

        let task = cx.spawn(async move |weak, cx| {
            let result = background.await;
            let _ = weak.update(cx, |view, cx| {
                view.finish_render(entity, request_id, render_stamp, size_px, result, cx);
            });
        });
        self.in_flight_render = Some(task);
    }

    fn finish_render(
        &mut self,
        entity: Entity<Self>,
        request_id: ScheduledRequestId,
        render_stamp: RenderStamp3D,
        size_px: (u32, u32),
        result: Result<ruviz::core::RenderedImage3D>,
        cx: &mut Context<Self>,
    ) {
        let Some(completion) = self.scheduler.complete(request_id) else {
            return;
        };
        self.in_flight_render = None;

        match result {
            Ok(rendered) if completion.install => match self.session.classify_render(rendered) {
                BackgroundRenderOutcome3D::Current(rendered) => {
                    self.cached_frame = Some(CachedFrame3D {
                        image: render_image_from_ruviz(rendered.image),
                        stamp: rendered.stamp,
                        size_px,
                    });
                }
                BackgroundRenderOutcome3D::Superseded { .. } => {}
            },
            Ok(_) => {}
            Err(error) if completion.install && self.session.is_render_current(render_stamp) => {
                self.report_error(error, cx);
            }
            Err(_) => {}
        }

        if let Some(next) = completion.next {
            self.start_render(entity, next, cx);
        }
        cx.notify();
    }

    fn local_position(&self, position: Point<Pixels>) -> Option<(f32, f32)> {
        let component_bounds = self.component_bounds?;
        let layout = self.interaction_layout?;
        map_position_to_frame(component_bounds, layout, position)
    }

    fn installed_view_is_current(&self) -> bool {
        self.cached_frame
            .as_ref()
            .is_some_and(|frame| frame.stamp.view() == self.session.view_stamp())
    }

    fn apply(&mut self, event: InputEvent3D, cx: &mut Context<Self>) -> Result<()> {
        if matches!(event, InputEvent3D::Escape) {
            self.cancel_drag();
        }
        if !self.options.interactive {
            return Ok(());
        }
        let updates_pick = matches!(
            event,
            InputEvent3D::PointerUp {
                button: PointerButton3D::Left,
                ..
            }
        );
        let result = self.session.handle_input(event)?;
        if result.camera_changed {
            self.selected = None;
            cx.emit(Plot3DEvent::CameraChanged(self.session.camera_snapshot()));
        }
        if let Some(hit) = result.picked {
            self.selected = self.session.current_pick();
            self.report_pick(hit, cx);
        } else if updates_pick {
            self.selected = self.session.current_pick();
        } else if self
            .selected
            .is_some_and(|pick| !self.session.is_stamped_pick_current(&pick))
        {
            self.selected = None;
        }
        if result.request_redraw {
            self.requested_view = None;
            cx.notify();
        }
        Ok(())
    }

    fn pointer_button(button: MouseButton) -> Option<PointerButton3D> {
        match button {
            MouseButton::Left => Some(PointerButton3D::Left),
            MouseButton::Middle => Some(PointerButton3D::Middle),
            MouseButton::Right => Some(PointerButton3D::Right),
            MouseButton::Navigate(_) => None,
        }
    }

    fn pointer_down(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) -> Result<()> {
        if !self.installed_view_is_current() {
            return Ok(());
        }
        let Some((x, y)) = self.local_position(event.position) else {
            return Ok(());
        };
        let Some(button) = Self::pointer_button(event.button) else {
            return Ok(());
        };
        self.apply(InputEvent3D::PointerDown { x, y, button }, cx)?;
        if self.session.is_drag_active() {
            self.active_pointer_button = Some(button);
        }
        Ok(())
    }

    fn pointer_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) -> Result<()> {
        if !self.installed_view_is_current() {
            return Ok(());
        }
        let Some((x, y)) = self.local_position(event.position) else {
            return Ok(());
        };
        self.apply(InputEvent3D::PointerMove { x, y }, cx)
    }

    fn pointer_up(&mut self, event: &MouseUpEvent, cx: &mut Context<Self>) -> Result<()> {
        let Some(button) = Self::pointer_button(event.button) else {
            return Ok(());
        };
        if self.active_pointer_button != Some(button) {
            return Ok(());
        }
        if !self.installed_view_is_current() {
            self.cancel_drag();
            return Ok(());
        }
        let Some((x, y)) = self.local_position(event.position) else {
            self.cancel_drag();
            return Ok(());
        };
        if button == PointerButton3D::Left && event.click_count >= 2 {
            self.cancel_drag();
            return self.apply(InputEvent3D::DoubleClick { x, y, button }, cx);
        }
        self.apply(InputEvent3D::PointerUp { x, y, button }, cx)?;
        self.active_pointer_button = None;
        Ok(())
    }

    fn cancel_drag(&mut self) {
        self.active_pointer_button = None;
        self.session.cancel_drag();
    }

    fn cancel_drag_for_release(&mut self, button: MouseButton) {
        let Some(button) = Self::pointer_button(button) else {
            return;
        };
        if self.active_pointer_button == Some(button) {
            self.cancel_drag();
        }
    }

    fn scroll(&mut self, event: &ScrollWheelEvent, cx: &mut Context<Self>) -> Result<bool> {
        if !self.options.interactive
            || !self.installed_view_is_current()
            || self.local_position(event.position).is_none()
        {
            return Ok(false);
        }
        let delta_y = match event.delta {
            ScrollDelta::Pixels(point) => -f32::from(point.y),
            ScrollDelta::Lines(point) => point.y * LINE_SCROLL_DELTA_PX,
        };
        self.apply(InputEvent3D::Wheel { delta_y }, cx)?;
        Ok(true)
    }
}

impl Focusable for RuvizPlot3D {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl gpui::EventEmitter<Plot3DEvent> for RuvizPlot3D {}

impl Render for RuvizPlot3D {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.focus_subscription.is_none() {
            self.focus_subscription =
                Some(cx.on_blur(&self.focus_handle, window, |view, _, _| view.cancel_drag()));
        }
        let entity = cx.entity();
        let plot_canvas = canvas::<Option<PaintFrame3D>>(
            {
                let entity = entity.clone();
                move |bounds, window, cx| {
                    let entity_for_render = entity.clone();
                    entity.update(cx, move |view, cx| {
                        view.prepaint(entity_for_render, bounds, window, cx)
                    })
                }
            },
            move |_bounds, frame: Option<PaintFrame3D>, window, _cx| {
                if let Some(frame) = frame {
                    let _ = window.paint_image(
                        frame.content_bounds,
                        Corners::default(),
                        frame.image,
                        0,
                        false,
                    );
                }
            },
        )
        .size_full();

        let mut root = div()
            .overflow_hidden()
            .track_focus(&self.focus_handle)
            .child(plot_canvas)
            .on_mouse_down(MouseButton::Left, pointer_down_handler(entity.clone()))
            .on_mouse_down(MouseButton::Middle, pointer_down_handler(entity.clone()))
            .on_mouse_down(MouseButton::Right, pointer_down_handler(entity.clone()))
            .on_mouse_move({
                let entity = entity.clone();
                move |event, _, cx| {
                    entity.update(cx, |view, cx| {
                        if let Err(error) = view.pointer_move(event, cx) {
                            view.report_error(error, cx);
                        }
                    });
                }
            })
            .on_mouse_up(MouseButton::Left, pointer_up_handler(entity.clone()))
            .on_mouse_up(MouseButton::Middle, pointer_up_handler(entity.clone()))
            .on_mouse_up(MouseButton::Right, pointer_up_handler(entity.clone()))
            .on_mouse_up_out(MouseButton::Left, cancel_drag_handler(entity.clone()))
            .on_mouse_up_out(MouseButton::Middle, cancel_drag_handler(entity.clone()))
            .on_mouse_up_out(MouseButton::Right, cancel_drag_handler(entity.clone()))
            .on_scroll_wheel({
                let entity = entity.clone();
                move |event, _, cx| {
                    entity.update(cx, |view, cx| match view.scroll(event, cx) {
                        Ok(true) => cx.stop_propagation(),
                        Ok(false) => {}
                        Err(error) => view.report_error(error, cx),
                    });
                }
            })
            .on_key_down({
                let entity = entity.clone();
                move |event, _, cx| {
                    if event.keystroke.key.as_str() == "escape" {
                        entity.update(cx, |view, cx| {
                            if let Err(error) = view.apply(InputEvent3D::Escape, cx) {
                                view.report_error(error, cx);
                            }
                        });
                    }
                }
            });

        root.interactivity().on_hover({
            let entity = entity.clone();
            move |hovered, _, cx| {
                if !hovered {
                    entity.update(cx, |view, _| view.cancel_drag());
                }
            }
        });

        match self.options.sizing_policy {
            SizingPolicy::Fill => {
                root = root.size_full();
            }
            SizingPolicy::FixedPixels { width, height } => {
                root = root.w(px(width as f32)).h(px(height as f32));
            }
        }
        root
    }
}

fn pointer_down_handler(
    entity: Entity<RuvizPlot3D>,
) -> impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static {
    move |event, window, cx| {
        entity.update(cx, |view, cx| {
            cx.focus_self(window);
            if let Err(error) = view.pointer_down(event, cx) {
                view.report_error(error, cx);
            }
        });
    }
}

fn pointer_up_handler(
    entity: Entity<RuvizPlot3D>,
) -> impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static {
    move |event, _, cx| {
        entity.update(cx, |view, cx| {
            if let Err(error) = view.pointer_up(event, cx) {
                view.report_error(error, cx);
            }
        });
    }
}

fn cancel_drag_handler(
    entity: Entity<RuvizPlot3D>,
) -> impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static {
    move |event, _, cx| {
        entity.update(cx, |view, _| {
            view.cancel_drag_for_release(event.button);
        });
    }
}

fn logical_rect(bounds: Bounds<Pixels>) -> LogicalRect {
    LogicalRect::new(
        f64::from(bounds.origin.x),
        f64::from(bounds.origin.y),
        f64::from(bounds.size.width),
        f64::from(bounds.size.height),
    )
}

fn core_image_fit(image_fit: ImageFit) -> CoreImageFit {
    match image_fit {
        ImageFit::Contain => CoreImageFit::Contain,
        ImageFit::Cover => CoreImageFit::Cover,
        ImageFit::Fill => CoreImageFit::Fill,
    }
}

const fn default_background_backend() -> BackgroundRenderBackend3D {
    #[cfg(feature = "gpu")]
    {
        BackgroundRenderBackend3D::GpuReadback
    }
    #[cfg(not(feature = "gpu"))]
    {
        BackgroundRenderBackend3D::Cpu
    }
}

fn fit_bounds(
    outer: Bounds<Pixels>,
    image_size_px: (u32, u32),
    image_fit: ImageFit,
) -> Bounds<Pixels> {
    let fitted = fitted_content_rect(
        logical_rect(outer),
        image_size_px,
        core_image_fit(image_fit),
    );
    Bounds {
        origin: point(px(fitted.x as f32), px(fitted.y as f32)),
        size: size(px(fitted.width as f32), px(fitted.height as f32)),
    }
}

fn map_position_to_frame(
    component_bounds: Bounds<Pixels>,
    layout: InteractionLayout3D,
    position: Point<Pixels>,
) -> Option<(f32, f32)> {
    if !component_bounds.contains(&position) {
        return None;
    }
    logical_to_physical(
        layout.content,
        LogicalPoint::new(f64::from(position.x), f64::from(position.y)),
        layout.frame_size_px,
    )
    .map(|(x, y)| (x as f32, y as f32))
}

/// Start a fluent GPUI 3D entity builder.
pub fn plot3d_builder<P>(plot: P) -> RuvizPlot3DBuilder<P>
where
    P: TryIntoPlot3DSession + 'static,
{
    RuvizPlot3DBuilder::new(plot)
}

/// Compatibility shortcut for building from an existing retained 3D session.
pub fn plot3d<V>(session: InteractivePlot3DSession, cx: &mut Context<V>) -> Entity<RuvizPlot3D>
where
    V: 'static,
{
    plot3d_builder(session).build(cx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;
    use ruviz::scatter3d;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn fitted_mapping_rejects_contain_letterbox() {
        let outer = Bounds {
            origin: point(px(10.0), px(20.0)),
            size: size(px(400.0), px(400.0)),
        };
        let content = fit_bounds(outer, (400, 200), ImageFit::Contain);
        assert_eq!(content.origin, point(px(10.0), px(120.0)));
        assert_eq!(content.size, size(px(400.0), px(200.0)));

        let layout = InteractionLayout3D {
            content: logical_rect(content),
            frame_size_px: (800, 400),
        };
        assert_eq!(
            logical_to_physical(
                layout.content,
                LogicalPoint::new(210.0, 220.0),
                layout.frame_size_px
            ),
            Some((400.0, 200.0))
        );
        assert_eq!(
            logical_to_physical(
                layout.content,
                LogicalPoint::new(210.0, 100.0),
                layout.frame_size_px
            ),
            None
        );
    }

    #[test]
    fn wrapper_mapping_covers_fractional_hidpi_and_all_fit_modes() {
        fn assert_point_close(actual: (f32, f32), expected: (f64, f64)) {
            assert!(
                (f64::from(actual.0) - expected.0).abs() < 1e-3,
                "x mismatch: {actual:?} versus {expected:?}"
            );
            assert!(
                (f64::from(actual.1) - expected.1).abs() < 1e-3,
                "y mismatch: {actual:?} versus {expected:?}"
            );
        }

        let outer = Bounds {
            origin: point(px(10.0), px(20.0)),
            size: size(px(300.0), px(240.0)),
        };
        let source_logical = Bounds {
            origin: point(px(0.0), px(0.0)),
            size: size(px(320.25), px(180.25)),
        };
        let outer_logical = logical_rect(outer);
        let outer_center = point(
            px((outer_logical.x + outer_logical.width * 0.5) as f32),
            px((outer_logical.y + outer_logical.height * 0.5) as f32),
        );
        let outside = point(px(outer_logical.x as f32 - 1.0), px(50.0));

        for scale_factor in [1.0, 1.25, 1.5, 2.0] {
            let frame_size_px = RuvizPlot3D::desired_size(source_logical, scale_factor);
            assert_eq!(
                frame_size_px,
                physical_backing_size(320.25, 180.25, scale_factor)
            );

            for image_fit in [ImageFit::Contain, ImageFit::Cover, ImageFit::Fill] {
                let content_bounds = fit_bounds(outer, frame_size_px, image_fit);
                let layout = InteractionLayout3D {
                    content: logical_rect(content_bounds),
                    frame_size_px,
                };

                assert_point_close(
                    map_position_to_frame(outer, layout, outer_center)
                        .expect("all centered fits contain the outer center"),
                    (
                        f64::from(frame_size_px.0) * 0.5,
                        f64::from(frame_size_px.1) * 0.5,
                    ),
                );
                assert_eq!(
                    map_position_to_frame(outer, layout, outside),
                    None,
                    "component bounds must reject input even when Cover content extends past them"
                );

                match image_fit {
                    ImageFit::Contain => {
                        let content = logical_rect(content_bounds);
                        let top_left = point(px(content.x as f32), px(content.y as f32));
                        let bottom_right = point(
                            px((content.x + content.width - 1e-3) as f32),
                            px((content.y + content.height - 1e-3) as f32),
                        );
                        assert_point_close(
                            map_position_to_frame(outer, layout, top_left)
                                .expect("contain top-left"),
                            (0.0, 0.0),
                        );
                        assert_point_close(
                            map_position_to_frame(outer, layout, bottom_right)
                                .expect("contain bottom-right"),
                            logical_to_physical(
                                layout.content,
                                LogicalPoint::new(
                                    f64::from(bottom_right.x),
                                    f64::from(bottom_right.y),
                                ),
                                frame_size_px,
                            )
                            .expect("inset contain bottom-right"),
                        );
                        assert_eq!(
                            map_position_to_frame(
                                outer,
                                layout,
                                point(px(outer_logical.x as f32), px(outer_logical.y as f32))
                            ),
                            None,
                            "contain letterbox must reject outer-corner input"
                        );
                    }
                    ImageFit::Cover | ImageFit::Fill => {
                        for corner in [
                            point(
                                px((outer_logical.x + 1e-3) as f32),
                                px((outer_logical.y + 1e-3) as f32),
                            ),
                            point(
                                px((outer_logical.x + outer_logical.width - 1e-3) as f32),
                                px((outer_logical.y + outer_logical.height - 1e-3) as f32),
                            ),
                        ] {
                            let expected = logical_to_physical(
                                layout.content,
                                LogicalPoint::new(f64::from(corner.x), f64::from(corner.y)),
                                frame_size_px,
                            )
                            .expect("cover/fill outer corner is fitted content");
                            assert_point_close(
                                map_position_to_frame(outer, layout, corner)
                                    .expect("cover/fill corner mapping"),
                                expected,
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn gpui_backing_size_ceil_preserves_fractional_hidpi_edges() {
        let bounds = Bounds {
            origin: point(px(0.0), px(0.0)),
            size: size(px(100.25), px(50.1)),
        };
        assert_eq!(RuvizPlot3D::desired_size(bounds, 1.25), (126, 63));
        assert_eq!(RuvizPlot3D::desired_size(bounds, 1.5), (151, 76));
    }

    #[test]
    fn retained_view_accepts_a_native_3d_builder_and_static_options() {
        let cx = TestAppContext::single();
        let options = RuvizPlot3DOptions {
            sizing_policy: SizingPolicy::FixedPixels {
                width: 320,
                height: 240,
            },
            image_fit: ImageFit::Fill,
            interactive: false,
            render_backend: default_background_backend(),
        };
        let session = scatter3d(&[0.0], &[1.0], &[2.0])
            .try_into_plot3d_session()
            .expect("valid 3D builder");
        let entity =
            cx.update(|cx| cx.new(move |cx| RuvizPlot3D::new(session, options, None, None, cx)));
        cx.read(|app| {
            app.read_entity(&entity, |view, _| {
                assert!(!view.options().interactive);
                assert_eq!(
                    view.options().sizing_policy,
                    SizingPolicy::FixedPixels {
                        width: 320,
                        height: 240
                    }
                );
                assert_eq!(view.options().image_fit, ImageFit::Fill);
                assert_eq!(
                    view.background_renderer
                        .lock()
                        .expect("background renderer")
                        .backend(),
                    default_background_backend()
                );
            });
        });
    }

    #[test]
    fn infallible_builder_retains_and_reports_invalid_plot_errors() {
        struct TestHost {
            plot: Entity<RuvizPlot3D>,
        }

        let cx = TestAppContext::single();
        let host = cx.update(|cx| {
            cx.new(|cx| TestHost {
                plot: plot3d_builder(scatter3d(&[0.0], &[1.0, 2.0], &[3.0])).build(cx),
            })
        });
        let entity = cx.read(|app| app.read_entity(&host, |host, _| host.plot.clone()));

        cx.read(|app| {
            app.read_entity(&entity, |view, _| {
                let error = view
                    .construction_error
                    .as_ref()
                    .expect("construction error");
                assert!(error.to_string().contains("length"));
                assert!(!view.construction_error_reported);
                assert!(view.cached_frame.is_none());
                assert!(view.requested_view.is_none());
            });
        });
    }

    #[test]
    fn default_backend_matches_the_compiled_gpui_feature() {
        #[cfg(feature = "gpu")]
        assert_eq!(
            RuvizPlot3DOptions::default().render_backend,
            BackgroundRenderBackend3D::GpuReadback
        );
        #[cfg(not(feature = "gpu"))]
        assert_eq!(
            RuvizPlot3DOptions::default().render_backend,
            BackgroundRenderBackend3D::Cpu
        );
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn gpu_readback_selection_installs_a_deterministic_completed_image() {
        let cx = TestAppContext::single();
        let session = scatter3d(&[0.0], &[1.0], &[2.0])
            .interactive_session()
            .expect("initial session");
        let entity = cx.update(|cx| {
            cx.new(move |cx| {
                RuvizPlot3D::new(session, RuvizPlot3DOptions::default(), None, None, cx)
            })
        });

        cx.update(|cx| {
            entity.update(cx, |view, cx| {
                assert_eq!(
                    view.options.render_backend,
                    BackgroundRenderBackend3D::GpuReadback,
                );
                assert_eq!(
                    view.background_renderer
                        .lock()
                        .expect("background renderer")
                        .backend(),
                    BackgroundRenderBackend3D::GpuReadback,
                );

                let job = view.session.background_render_job().expect("render job");
                let stamp = job.stamp();
                let scheduled = view
                    .scheduler
                    .request(RenderRequest3D {
                        job,
                        size_px: (2, 1),
                    })
                    .expect("first request should start");
                view.finish_render(
                    entity.clone(),
                    scheduled.id(),
                    stamp,
                    (2, 1),
                    Ok(ruviz::core::RenderedImage3D {
                        image: ruviz::core::Image::new(
                            2,
                            1,
                            vec![10, 20, 30, 255, 40, 50, 60, 255],
                        ),
                        stamp,
                    }),
                    cx,
                );

                let installed = view.cached_frame.as_ref().expect("installed frame");
                assert_eq!(installed.size_px, (2, 1));
                assert_eq!(installed.stamp, stamp);
                assert_eq!(
                    installed.image.as_bytes(0).expect("installed pixels"),
                    &[30, 20, 10, 255, 60, 50, 40, 255],
                );
            });
        });
    }

    #[test]
    fn installed_frame_gate_rejects_camera_and_replacement_stamps() {
        let cx = TestAppContext::single();
        let session = scatter3d(&[0.0], &[1.0], &[2.0])
            .interactive_session()
            .expect("initial session");
        let entity = cx.update(|cx| {
            cx.new(move |cx| {
                RuvizPlot3D::new(session, RuvizPlot3DOptions::default(), None, None, cx)
            })
        });

        cx.update(|cx| {
            entity.update(cx, |view, _| {
                let stamp = view
                    .session
                    .background_render_job()
                    .expect("render job")
                    .stamp();
                view.cached_frame = Some(CachedFrame3D {
                    image: render_image_from_ruviz(ruviz::core::Image::new(
                        1,
                        1,
                        vec![0, 0, 0, 255],
                    )),
                    stamp,
                    size_px: (1, 1),
                });
                assert!(view.installed_view_is_current());

                view.session.orbit(1.0, 0.0).expect("camera change");
                assert!(!view.installed_view_is_current());

                let pre_replacement = view
                    .session
                    .background_render_job()
                    .expect("pre-replacement job")
                    .stamp();
                view.session.replace(
                    scatter3d(&[3.0], &[4.0], &[5.0])
                        .interactive_session()
                        .expect("replacement"),
                );
                assert!(!view.installed_view_is_current());
                assert!(!view.session.is_render_current(pre_replacement));
            });
        });
    }

    #[test]
    fn sphere_shading_retains_selection_and_invalidates_the_displayed_frame() {
        use ruviz::prelude::{Color, Point3D, Sphere3D, spheres3d};
        let atoms = [Sphere3D::new(
            42,
            Point3D::new(0.0, 0.0, 0.0),
            1.0,
            Color::RED,
        )];
        let plot = spheres3d(&atoms);
        let position = plot.clone().project(atoms[0].center).unwrap().unwrap();
        let mut session = plot.interactive_session().unwrap();
        session.pick(position.0, position.1).unwrap();
        let camera = session.camera();
        let cx = TestAppContext::single();
        let entity = cx.update(|cx| {
            cx.new(move |cx| {
                let mut view =
                    RuvizPlot3D::new(session, RuvizPlot3DOptions::default(), None, None, cx);
                view.selected = view.session.current_pick();
                view
            })
        });
        cx.update(|cx| {
            entity.update(cx, |view, cx| {
                let old = view.session.background_render_job().unwrap().stamp();
                view.requested_view = Some(old.view());
                view.set_sphere_shading(false, cx).unwrap();
                assert_eq!(view.session.camera(), camera);
                assert_eq!(view.selected().unwrap().sources(), &[42]);
                assert!(view.stamped_pick().is_some());
                assert!(view.requested_view.is_none());
                assert!(!view.session.is_render_current(old));
            })
        });
    }

    #[test]
    fn selected_remains_const_and_is_eagerly_cleared_on_view_invalidation() {
        const fn selected_from(view: &RuvizPlot3D) -> Option<PickHit3D> {
            view.selected()
        }

        let cx = TestAppContext::single();
        let mut session = scatter3d(&[0.0], &[1.0], &[2.0])
            .interactive_session()
            .expect("initial session");
        let (width, height) = session.size_px();
        let mut picked = false;
        for y in (0..height).step_by(4) {
            for x in (0..width).step_by(4) {
                if session
                    .pick(x as f32, y as f32)
                    .expect("pickable-position search")
                    .is_some()
                {
                    picked = true;
                    break;
                }
            }
            if picked {
                break;
            }
        }
        assert!(picked, "test plot must contain a pickable position");

        let entity = cx.update(|cx| {
            cx.new(move |cx| {
                let mut view =
                    RuvizPlot3D::new(session, RuvizPlot3DOptions::default(), None, None, cx);
                view.selected = view.session.current_pick();
                view
            })
        });

        cx.update(|cx| {
            entity.update(cx, |view, _| {
                assert!(selected_from(view).is_some());
                view.invalidate_view();
                assert!(selected_from(view).is_none());
                assert!(view.stamped_pick().is_none());
            });
        });
    }

    #[test]
    fn escape_cancels_an_active_drag_before_resetting() {
        let cx = TestAppContext::single();
        let session = scatter3d(&[0.0], &[1.0], &[2.0])
            .interactive_session()
            .expect("initial session");
        let entity = cx.update(|cx| {
            cx.new(move |cx| {
                RuvizPlot3D::new(session, RuvizPlot3DOptions::default(), None, None, cx)
            })
        });
        cx.update(|cx| {
            entity.update(cx, |view, cx| {
                view.session
                    .handle_input(InputEvent3D::PointerDown {
                        x: 20.0,
                        y: 20.0,
                        button: PointerButton3D::Left,
                    })
                    .expect("pointer down");
                view.active_pointer_button = Some(PointerButton3D::Left);
                assert!(view.session.is_drag_active());
                view.apply(InputEvent3D::Escape, cx).expect("escape");
                assert!(!view.session.is_drag_active());
                assert_eq!(view.active_pointer_button(), None);
            });
        });
    }

    #[test]
    fn different_button_release_does_not_end_the_active_drag() {
        let cx = TestAppContext::single();
        let session = scatter3d(&[0.0], &[1.0], &[2.0])
            .interactive_session()
            .expect("initial session");
        let entity = cx.update(|cx| {
            cx.new(move |cx| {
                RuvizPlot3D::new(session, RuvizPlot3DOptions::default(), None, None, cx)
            })
        });

        cx.update(|cx| {
            entity.update(cx, |view, cx| {
                view.session
                    .handle_input(InputEvent3D::PointerDown {
                        x: 20.0,
                        y: 20.0,
                        button: PointerButton3D::Left,
                    })
                    .expect("pointer down");
                view.active_pointer_button = Some(PointerButton3D::Left);

                view.pointer_up(
                    &MouseUpEvent {
                        button: MouseButton::Right,
                        ..MouseUpEvent::default()
                    },
                    cx,
                )
                .expect("unrelated release");

                assert!(view.session.is_drag_active());
                assert_eq!(view.active_pointer_button(), Some(PointerButton3D::Left));

                view.cancel_drag_for_release(MouseButton::Right);
                assert!(view.session.is_drag_active());
                assert_eq!(view.active_pointer_button(), Some(PointerButton3D::Left));
            });
        });
    }

    #[test]
    fn matching_release_cancels_a_drag_when_the_installed_view_is_stale() {
        let cx = TestAppContext::single();
        let session = scatter3d(&[0.0], &[1.0], &[2.0])
            .interactive_session()
            .expect("initial session");
        let entity = cx.update(|cx| {
            cx.new(move |cx| {
                RuvizPlot3D::new(session, RuvizPlot3DOptions::default(), None, None, cx)
            })
        });

        cx.update(|cx| {
            entity.update(cx, |view, cx| {
                view.session
                    .handle_input(InputEvent3D::PointerDown {
                        x: 20.0,
                        y: 20.0,
                        button: PointerButton3D::Left,
                    })
                    .expect("pointer down");
                view.active_pointer_button = Some(PointerButton3D::Left);

                view.pointer_up(
                    &MouseUpEvent {
                        button: MouseButton::Left,
                        ..MouseUpEvent::default()
                    },
                    cx,
                )
                .expect("matching release");

                assert!(!view.session.is_drag_active());
                assert_eq!(view.active_pointer_button(), None);
            });
        });
    }

    #[test]
    fn double_click_release_resets_without_emitting_a_second_pick() {
        fn first_pickable_position(session: &mut InteractivePlot3DSession) -> (f32, f32) {
            let (width, height) = session.size_px();
            let center = (width as f32 * 0.5, height as f32 * 0.5);
            if session
                .pick(center.0, center.1)
                .expect("center pick")
                .is_some()
            {
                session.clear_pick();
                return center;
            }
            for y in (0..height).step_by(4) {
                for x in (0..width).step_by(4) {
                    if session
                        .pick(x as f32, y as f32)
                        .expect("pickable-position search")
                        .is_some()
                    {
                        session.clear_pick();
                        return (x as f32, y as f32);
                    }
                }
            }
            panic!("test plot must contain a pickable position");
        }

        let picks = Arc::new(AtomicUsize::new(0));
        let picks_for_handler = Arc::clone(&picks);
        let cx = TestAppContext::single();
        let mut session = scatter3d(&[0.0], &[1.0], &[2.0])
            .interactive_session()
            .expect("initial session");
        let reset_camera = session.camera();
        session.orbit(12.0, 0.0).expect("move camera");
        let pick_position = first_pickable_position(&mut session);
        let entity = cx.update(|cx| {
            cx.new(move |cx| {
                RuvizPlot3D::new(
                    session,
                    RuvizPlot3DOptions::default(),
                    Some(Arc::new(move |_| {
                        picks_for_handler.fetch_add(1, Ordering::Relaxed);
                    })),
                    None,
                    cx,
                )
            })
        });

        cx.update(|cx| {
            entity.update(cx, |view, cx| {
                let size_px = view.session.size_px();
                let stamp = view
                    .session
                    .background_render_job()
                    .expect("render job")
                    .stamp();
                view.cached_frame = Some(CachedFrame3D {
                    image: render_image_from_ruviz(ruviz::core::Image::new(
                        1,
                        1,
                        vec![0, 0, 0, 255],
                    )),
                    stamp,
                    size_px,
                });
                view.interaction_layout = Some(InteractionLayout3D {
                    content: LogicalRect::new(0.0, 0.0, f64::from(size_px.0), f64::from(size_px.1)),
                    frame_size_px: size_px,
                });
                view.component_bounds = Some(Bounds {
                    origin: point(px(0.0), px(0.0)),
                    size: size(px(size_px.0 as f32), px(size_px.1 as f32)),
                });
                let position = point(px(pick_position.0), px(pick_position.1));

                view.pointer_down(
                    &MouseDownEvent {
                        button: MouseButton::Left,
                        position,
                        click_count: 1,
                        ..MouseDownEvent::default()
                    },
                    cx,
                )
                .expect("first pointer down");
                view.pointer_up(
                    &MouseUpEvent {
                        button: MouseButton::Left,
                        position,
                        click_count: 1,
                        ..MouseUpEvent::default()
                    },
                    cx,
                )
                .expect("first pointer up");
                assert_eq!(picks.load(Ordering::Relaxed), 1);

                view.pointer_down(
                    &MouseDownEvent {
                        button: MouseButton::Left,
                        position,
                        click_count: 2,
                        ..MouseDownEvent::default()
                    },
                    cx,
                )
                .expect("second pointer down");
                view.pointer_up(
                    &MouseUpEvent {
                        button: MouseButton::Left,
                        position,
                        click_count: 2,
                        ..MouseUpEvent::default()
                    },
                    cx,
                )
                .expect("double-click pointer up");

                assert_eq!(
                    picks.load(Ordering::Relaxed),
                    1,
                    "the reset click must not emit a second pick"
                );
                assert_eq!(view.active_pointer_button(), None);
                assert!(!view.session.is_drag_active());
                assert_eq!(view.session.camera(), reset_camera);
            });
        });
    }

    #[test]
    fn invalidation_preserves_one_in_flight_and_coalesces_latest() {
        let cx = TestAppContext::single();
        let session = scatter3d(&[0.0], &[1.0], &[2.0])
            .interactive_session()
            .expect("initial session");
        let entity = cx.update(|cx| {
            cx.new(move |cx| {
                RuvizPlot3D::new(session, RuvizPlot3DOptions::default(), None, None, cx)
            })
        });

        cx.update(|cx| {
            entity.update(cx, |view, _| {
                let worker = Arc::clone(&view.background_renderer);
                let first_job = view.session.background_render_job().expect("first job");
                let first_size = view.session.size_px();
                let first = view
                    .scheduler
                    .request(RenderRequest3D {
                        job: first_job,
                        size_px: first_size,
                    })
                    .expect("first request starts");

                view.session.orbit(5.0, 0.0).expect("newer camera");
                view.invalidate_view();
                assert!(!view.scheduler.is_idle());
                assert!(Arc::ptr_eq(&worker, &view.background_renderer));

                let newest_job = view.session.background_render_job().expect("newest job");
                let newest_size = view.session.size_px();
                assert!(
                    view.scheduler
                        .request(RenderRequest3D {
                            job: newest_job,
                            size_px: newest_size,
                        })
                        .is_none(),
                    "newest work must queue behind the synchronous in-flight worker"
                );

                let completion = view
                    .scheduler
                    .complete(first.id())
                    .expect("first identity remains the in-flight request");
                assert!(!completion.install);
                let newest = completion.next.expect("latest request is retained");
                assert!(
                    view.scheduler
                        .complete(newest.id())
                        .expect("newest completion")
                        .install
                );
            });
        });
    }

    #[test]
    fn fitted_scroll_reports_when_gpui_must_stop_propagation() {
        let cx = TestAppContext::single();
        let session = scatter3d(&[0.0], &[1.0], &[2.0])
            .interactive_session()
            .expect("initial session");
        let entity = cx.update(|cx| {
            cx.new(move |cx| {
                RuvizPlot3D::new(session, RuvizPlot3DOptions::default(), None, None, cx)
            })
        });

        cx.update(|cx| {
            entity.update(cx, |view, cx| {
                let size_px = view.session.size_px();
                let stamp = view
                    .session
                    .background_render_job()
                    .expect("render job")
                    .stamp();
                view.cached_frame = Some(CachedFrame3D {
                    image: render_image_from_ruviz(ruviz::core::Image::new(
                        1,
                        1,
                        vec![0, 0, 0, 255],
                    )),
                    stamp,
                    size_px,
                });
                view.interaction_layout = Some(InteractionLayout3D {
                    content: LogicalRect::new(10.0, 20.0, 100.0, 80.0),
                    frame_size_px: size_px,
                });
                view.component_bounds = Some(Bounds {
                    origin: point(px(0.0), px(0.0)),
                    size: size(px(120.0), px(120.0)),
                });

                let outside = ScrollWheelEvent {
                    position: point(px(5.0), px(5.0)),
                    delta: ScrollDelta::Lines(point(0.0, 1.0)),
                    ..ScrollWheelEvent::default()
                };
                assert!(!view.scroll(&outside, cx).expect("outside scroll"));

                let before = view.session.camera();
                let inside = ScrollWheelEvent {
                    position: point(px(50.0), px(50.0)),
                    delta: ScrollDelta::Lines(point(0.0, 1.0)),
                    ..ScrollWheelEvent::default()
                };
                assert!(view.scroll(&inside, cx).expect("inside scroll"));
                assert_ne!(view.session.camera(), before);
            });
        });
    }

    #[test]
    fn stale_worker_errors_are_suppressed_after_camera_and_replacement_changes() {
        fn schedule_error_target(
            view: &mut RuvizPlot3D,
        ) -> (ScheduledRequestId, RenderStamp3D, (u32, u32)) {
            let job = view.session.background_render_job().expect("render job");
            let stamp = job.stamp();
            let size_px = view.session.size_px();
            let scheduled = view
                .scheduler
                .request(RenderRequest3D { job, size_px })
                .expect("idle scheduler starts request");
            (scheduled.id(), stamp, size_px)
        }

        let reported = Arc::new(AtomicUsize::new(0));
        let reported_for_handler = Arc::clone(&reported);
        let cx = TestAppContext::single();
        let session = scatter3d(&[0.0], &[1.0], &[2.0])
            .interactive_session()
            .expect("initial session");
        let entity = cx.update(|cx| {
            cx.new(move |cx| {
                RuvizPlot3D::new(
                    session,
                    RuvizPlot3DOptions::default(),
                    None,
                    Some(Arc::new(move |_| {
                        reported_for_handler.fetch_add(1, Ordering::Relaxed);
                    })),
                    cx,
                )
            })
        });

        let entity_for_finish = entity.clone();
        cx.update(|cx| {
            entity.update(cx, |view, cx| {
                let (camera_id, camera_stamp, camera_size) = schedule_error_target(view);
                view.session.orbit(4.0, 0.0).expect("camera mutation");
                view.finish_render(
                    entity_for_finish.clone(),
                    camera_id,
                    camera_stamp,
                    camera_size,
                    Err(PlottingError::RenderError("stale camera".to_string())),
                    cx,
                );
                assert_eq!(reported.load(Ordering::Relaxed), 0);

                let (replacement_id, replacement_stamp, replacement_size) =
                    schedule_error_target(view);
                view.session.replace(
                    scatter3d(&[3.0], &[4.0], &[5.0])
                        .interactive_session()
                        .expect("replacement"),
                );
                view.finish_render(
                    entity_for_finish.clone(),
                    replacement_id,
                    replacement_stamp,
                    replacement_size,
                    Err(PlottingError::RenderError("stale replacement".to_string())),
                    cx,
                );
                assert_eq!(reported.load(Ordering::Relaxed), 0);

                let (current_id, current_stamp, current_size) = schedule_error_target(view);
                view.finish_render(
                    entity_for_finish.clone(),
                    current_id,
                    current_stamp,
                    current_size,
                    Err(PlottingError::RenderError("current".to_string())),
                    cx,
                );
                assert_eq!(reported.load(Ordering::Relaxed), 1);
            });
        });
    }

    #[test]
    fn replacement_can_reset_or_keep_camera() {
        let cx = TestAppContext::single();
        let session = scatter3d(&[0.0], &[1.0], &[2.0])
            .interactive_session()
            .expect("initial session");
        let entity = cx.update(|cx| {
            cx.new(move |cx| {
                RuvizPlot3D::new(session, RuvizPlot3DOptions::default(), None, None, cx)
            })
        });

        cx.update(|cx| {
            entity.update(cx, |view, cx| {
                let camera = view.session().camera().azimuth_deg(77.0);
                view.set_camera(camera, cx).expect("camera");
                view.set_plot_keep_view(scatter3d(&[3.0], &[4.0], &[5.0]), cx)
                    .expect("keep-camera replacement");
                assert_eq!(view.session().camera(), camera);
                view.set_plot(scatter3d(&[6.0], &[7.0], &[8.0]), cx)
                    .expect("reset-camera replacement");
                assert_ne!(view.session().camera(), camera);
            });
        });
    }
}
