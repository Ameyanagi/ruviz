use super::*;

impl RuvizPlot {
    /// Maps an absolute GPUI window position to displayed plot data coordinates.
    ///
    /// Returns `Ok(None)` before GPUI layout is available, outside the displayed
    /// frame, or in a frame margin outside the core plot area. Core displayed-frame
    /// conversion errors are returned unchanged.
    pub fn data_at(&self, window_position: Point<Pixels>) -> Result<Option<ViewportPoint>> {
        let Some(base_generation) = self.presented_base_generation() else {
            return Ok(None);
        };
        let Some(viewport_position) = self.local_viewport_point(window_position) else {
            return Ok(None);
        };
        let data_position = self.session.screen_to_data(viewport_position)?;
        if !self.is_base_generation_presented(base_generation) {
            return Ok(None);
        }
        Ok(data_position)
    }

    /// Maps displayed plot data coordinates to an absolute GPUI window position.
    ///
    /// Returns `Ok(None)` before GPUI layout is available or when the data point is
    /// outside the displayed visible bounds. Core displayed-frame conversion errors
    /// are returned unchanged.
    pub fn screen_at(&self, data_position: ViewportPoint) -> Result<Option<Point<Pixels>>> {
        let Some(base_generation) = self.presented_base_generation() else {
            return Ok(None);
        };
        let viewport_position = self.session.data_to_screen(data_position)?;
        if !self.is_base_generation_presented(base_generation) {
            return Ok(None);
        }
        Ok(viewport_position.and_then(|position| self.viewport_point_to_window_position(position)))
    }

    pub(super) fn build_plot_pointer_event(
        &self,
        kind: PlotPointerEventKind,
        mouse_button: Option<MouseButton>,
        window_position: Point<Pixels>,
    ) -> Result<Option<PlotPointerEvent>> {
        let Some(base_generation) = self.presented_base_generation() else {
            return Ok(None);
        };
        let Some(viewport_position) = self.local_viewport_point(window_position) else {
            return Ok(None);
        };
        let data_position = self.session.screen_to_data(viewport_position)?;
        let viewport = self.session.viewport_snapshot()?;
        let hit = self.session.hit_test(viewport_position);
        if !self.is_base_generation_presented(base_generation) {
            return Ok(None);
        }
        Ok(Some(PlotPointerEvent {
            kind,
            mouse_button,
            window_position,
            viewport_position,
            data_position,
            viewport,
            hit,
        }))
    }

    fn presented_base_generation(&self) -> Option<u64> {
        let generation = self.cached_frame.as_ref()?.base_generation;
        self.is_base_generation_presented(generation)
            .then_some(generation)
    }

    fn is_base_generation_presented(&self, generation: u64) -> bool {
        self.cached_frame
            .as_ref()
            .is_some_and(|frame| frame.base_generation == generation)
            && self.session.displayed_frame_generation() == Some(generation)
    }

    pub(super) fn replace_cached_frame(
        &mut self,
        mut request: RenderRequest,
        mut frame: RenderedFrame,
    ) {
        request.presented_base_generation = Some(frame.base_generation);
        let previous = self.cached_frame.take();
        let primary = self
            .resolve_primary_frame(previous.as_ref(), &mut frame)
            .expect("rendered frame must include a primary layer on first render");
        let overlay_image = if frame.target == RenderTargetKind::Image {
            None
        } else {
            match frame.overlay {
                RenderedOverlay::Reuse => previous
                    .as_ref()
                    .and_then(|cached| cached.overlay_image.as_ref().map(Arc::clone)),
                RenderedOverlay::Replace(overlay) => overlay,
            }
        };

        if let Some(previous) = previous {
            let anchored = match (&previous.primary, self.pan_anchor.as_ref()) {
                (PrimaryFrame::Image(image), Some(anchor)) => {
                    matches!(&anchor.primary, PrimaryFrame::Image(held) if Arc::ptr_eq(held, image))
                }
                _ => false,
            };
            if !anchored {
                maybe_retire_replaced_primary(
                    &mut self.retired_images,
                    &previous.primary,
                    &primary,
                );
            }
            if let Some(previous_overlay) = previous.overlay_image {
                let overlay_reused = overlay_image
                    .as_ref()
                    .is_some_and(|current| Arc::ptr_eq(current, &previous_overlay));
                if !overlay_reused {
                    self.retired_images.push(previous_overlay);
                }
            }
        }

        self.cached_frame = Some(CachedFrame {
            request,
            base_generation: frame.base_generation,
            primary,
            overlay_image,
            stats: frame.stats,
            target: frame.target,
            view: frame.view,
        });
    }

    fn resolve_primary_frame(
        &mut self,
        previous: Option<&CachedFrame>,
        frame: &mut RenderedFrame,
    ) -> Option<PrimaryFrame> {
        match frame.primary.take() {
            Some(RenderedPrimary::Image(image)) => Some(PrimaryFrame::Image(image)),
            #[cfg(all(feature = "gpu", target_os = "macos"))]
            Some(RenderedPrimary::Surface(base_layer)) => {
                // The anchor keeps its pixels for the whole drag, so a frame
                // it shares its surface with must render into a new one.
                let anchor_surface =
                    self.pan_anchor
                        .as_ref()
                        .and_then(|anchor| match &anchor.primary {
                            PrimaryFrame::Surface(surface) => Some(surface.as_CFTypeRef()),
                            PrimaryFrame::Image(_) => None,
                        });
                let previous_surface = previous
                    .and_then(|cached| match &cached.primary {
                        PrimaryFrame::Surface(surface) => Some(surface),
                        PrimaryFrame::Image(_) => None,
                    })
                    .filter(|surface| anchor_surface != Some(surface.as_CFTypeRef()));

                match self.surface_upload.update(previous_surface, &base_layer) {
                    Ok(surface) => Some(PrimaryFrame::Surface(surface)),
                    Err(_) => Some(PrimaryFrame::Image(render_image_from_ruviz(
                        base_layer.image().as_ref().clone(),
                    ))),
                }
            }
            None => previous.map(|cached| cached.primary.clone()),
        }
    }

    fn flush_retired_images(&mut self, mut window: Option<&mut Window>, cx: &mut App) {
        for image in self.retired_images.drain(..) {
            cx.drop_image(image, window.as_deref_mut());
        }
    }

    pub(super) fn ensure_reactive_watcher(
        &mut self,
        entity: Entity<Self>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.reactive_watcher.is_some() || self.subscription.is_empty() {
            return;
        }

        let mut receiver = match self.reactive_receiver.take() {
            Some(receiver) => receiver,
            None => return,
        };

        let pending = Arc::clone(&self.reactive_notify_pending);
        let task = window.spawn(cx, async move |cx| {
            while receiver.next().await.is_some() {
                let entity_for_notify = entity.clone();
                let pending_for_notify = Arc::clone(&pending);
                cx.on_next_frame(move |_, cx| {
                    entity_for_notify.update(cx, |view, cx| {
                        view.failed_request = None;
                        view.last_error = None;
                        pending_for_notify.store(false, Ordering::Release);
                        cx.notify();
                    });
                });
            }
        });

        self.reactive_watcher = Some(task);
    }

    fn effective_presentation_mode(&self) -> PresentationMode {
        resolve_presentation_mode(self.options.presentation_mode)
    }

    fn current_request(&self, bounds: Bounds<Pixels>, window: &Window) -> Option<RenderRequest> {
        let scale_factor = window.scale_factor();
        let logical_size_px = (f64::from(bounds.size.width), f64::from(bounds.size.height));
        let frame_size_px = frame_size_px_for_policy(
            &self.session,
            &self.options.sizing_policy,
            logical_size_px,
            scale_factor,
        )?;

        Some(
            RenderRequest::new(
                frame_size_px,
                scale_factor,
                self.options.interaction.time_seconds,
                self.effective_presentation_mode(),
            )
            .with_presented_base_generation(
                self.cached_frame
                    .as_ref()
                    .map(|frame| frame.base_generation),
            ),
        )
    }

    pub(super) fn prepaint(
        &mut self,
        entity: Entity<Self>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<PaintFrame> {
        self.flush_retired_images(Some(window), cx);

        if let Some(request) = self.current_request(bounds, window) {
            let requested_size_px = request.size_px;
            self.start_render_if_needed(entity, request, window, cx);
            let frame_size_px = self
                .cached_frame
                .as_ref()
                .map_or(requested_size_px, |frame| frame.request.size_px);
            self.update_layout_state(bounds, frame_size_px);
        } else {
            self.last_layout = None;
        }

        self.update_pan_anchor(cx);
        self.pan_paint()
    }

    /// Keep the drag anchor in step with the drag: capture the frame on
    /// screen when a pan crosses its threshold, drop it once the drag has
    /// ended and the final raster shows the pending view, and drop it early
    /// when the frame geometry changes (resize, scale, presentation).
    pub(super) fn update_pan_anchor(&mut self, cx: &mut App) {
        let contract_changed = match (self.pan_anchor.as_ref(), self.cached_frame.as_ref()) {
            (Some(anchor), Some(frame)) => !anchor.request.same_frame_contract(&frame.request),
            _ => false,
        };
        if contract_changed {
            self.clear_pan_anchor(cx);
        }
        if self.pan_drag_active() {
            if self.pan_anchor.is_none()
                && let Some(frame) = self.cached_frame.as_ref()
                && let Some(view) = frame.view.as_ref()
            {
                self.pan_anchor = Some(PanAnchor {
                    request: frame.request.clone(),
                    primary: frame.primary.clone(),
                    view: view.clone(),
                });
            }
            return;
        }
        if self.pan_anchor.is_some() && self.pan_preview().is_none() {
            // The drag is over and the frame on screen shows the pending view:
            // its own axes are correct again.
            self.clear_pan_anchor(cx);
        }
    }

    fn clear_pan_anchor(&mut self, cx: &mut App) {
        let Some(anchor) = self.pan_anchor.take() else {
            return;
        };
        match anchor.primary {
            PrimaryFrame::Image(image) => {
                let still_shown = self.cached_frame.as_ref().is_some_and(|frame| {
                    matches!(&frame.primary, PrimaryFrame::Image(current) if Arc::ptr_eq(current, &image))
                });
                if !still_shown {
                    cx.drop_image(image, None);
                }
            }
            // A surface is released with the anchor; nothing else holds it.
            #[cfg(all(feature = "gpu", target_os = "macos"))]
            PrimaryFrame::Surface(_) => {}
        }
    }

    /// What to paint: the plot-area content (the newest frame whose plot
    /// area matches the anchor's, else the anchor itself) shifted under the
    /// anchor's axes while a pan drag is in progress, or the cached frame
    /// with a plain pan preview otherwise.
    pub(super) fn pan_paint(&self) -> Option<PaintFrame> {
        let frame = self.cached_frame.as_ref()?;
        let overlay_image = frame.overlay_image.as_ref().map(Arc::clone);
        let Some(anchor) = self.pan_anchor.as_ref() else {
            return Some(PaintFrame {
                primary: frame.primary.clone(),
                overlay_image,
                preview: self.pan_preview(),
                axes: None,
            });
        };
        let layout = self.last_layout.as_ref()?;
        let pending = self.session.view_bounds_snapshot().visible_bounds;
        // A raster whose plot area has the anchor's size slides into the
        // anchor's frame exactly; one whose margins moved cannot, so the
        // anchor's own content stays until the drag ends.
        let (content, view) = match frame.view.as_ref() {
            Some(view) if same_plot_area_size(view, &anchor.view) => (&frame.primary, view),
            _ => (&anchor.primary, &anchor.view),
        };
        let preview = preview_translation_onto(
            view,
            pending,
            anchor.view.plot_area,
            anchor.view.axis_inset_px,
            layout.content_bounds,
            layout.frame_size_px,
            true,
        );
        match preview {
            Some(preview) => Some(PaintFrame {
                primary: content.clone(),
                overlay_image,
                preview: Some(preview),
                axes: Some(anchor.primary.clone()),
            }),
            // Not a pure pan any more (a zoom landed): fall back to the frame.
            None => Some(PaintFrame {
                primary: frame.primary.clone(),
                overlay_image,
                preview: self.pan_preview(),
                axes: None,
            }),
        }
    }

    /// The GPU translation that shows the session's pending view with the
    /// cached frame, or `None` when the frame already shows it (or the
    /// change is not a pure pan).
    pub(super) fn pan_preview(&self) -> Option<PanPreview> {
        let frame = self.cached_frame.as_ref()?;
        let view = frame.view.as_ref()?;
        let layout = self.last_layout.as_ref()?;
        let pending = self.session.view_bounds_snapshot().visible_bounds;
        preview_translation(view, pending, layout.content_bounds, layout.frame_size_px)
    }

    /// Whether a left-button pan drag has crossed its threshold.
    pub(super) fn pan_drag_active(&self) -> bool {
        matches!(
            self.interaction_state.active_drag,
            ActiveDrag::LeftPan {
                crossed_threshold: true,
                ..
            }
        )
    }

    /// Wake the view up once `delay` has passed so a throttled raster is
    /// scheduled even if no further input arrives.
    fn arm_raster_wakeup(
        &mut self,
        entity: Entity<Self>,
        delay: Duration,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.raster_wakeup.is_some() {
            return;
        }
        let timer = cx.background_executor().timer(delay);
        let task = window.spawn(cx, async move |cx| {
            timer.await;
            cx.on_next_frame(move |_, cx| {
                entity.update(cx, |view, cx| {
                    view.raster_wakeup = None;
                    cx.notify();
                });
            });
        });
        self.raster_wakeup = Some(task);
    }

    #[cfg(test)]
    pub(super) fn update_layout(&mut self, bounds: Bounds<Pixels>, frame_size_px: (u32, u32)) {
        self.update_layout_state(bounds, frame_size_px);
    }

    fn update_layout_state(&mut self, bounds: Bounds<Pixels>, frame_size_px: (u32, u32)) {
        let image_size = size(frame_size_px.0.into(), frame_size_px.1.into());
        let content_bounds = self
            .options
            .interaction
            .image_fit
            .into_gpui()
            .get_bounds(bounds, image_size);

        self.last_layout = Some(InteractionLayout {
            component_bounds: bounds,
            content_bounds,
            frame_size_px,
        });
    }

    fn start_render_if_needed(
        &mut self,
        entity: Entity<Self>,
        request: RenderRequest,
        window: &mut Window,
        cx: &mut App,
    ) {
        if !self.should_start_render(&request) {
            return;
        }

        // During a pan the cached frame is translated on the GPU, so rasters
        // only need to keep the axes fresh: space them out instead of
        // rendering after every pointer move.
        if self.pan_drag_active()
            && self.scheduler.in_flight.is_none()
            && let Some(started) = self.last_render_started
            && started.elapsed() < PAN_RASTER_INTERVAL
        {
            self.arm_raster_wakeup(entity, PAN_RASTER_INTERVAL - started.elapsed(), window, cx);
            return;
        }

        let Some(scheduled) = self.scheduler.schedule(request) else {
            return;
        };

        self.start_render(entity, scheduled, window, cx);
    }

    pub(super) fn should_start_render(&self, request: &RenderRequest) -> bool {
        !self.cache_is_current(request) && self.failed_request.as_ref() != Some(request)
    }

    pub(super) fn cache_is_current(&self, request: &RenderRequest) -> bool {
        self.cached_frame.as_ref().is_some_and(|frame| {
            frame.request == *request
                && self.session.displayed_frame_generation() == Some(frame.base_generation)
                && !request.is_dirty(&self.session)
        })
    }

    fn start_render(
        &mut self,
        entity: Entity<Self>,
        scheduled: ScheduledRender,
        window: &mut Window,
        cx: &mut App,
    ) {
        let session = self.session.clone();
        let request_for_task = scheduled.request.clone();
        let render_job = cx
            .background_executor()
            .spawn(async move { render_frame_from_session(session, request_for_task) });

        let entity_for_update = entity.clone();
        let scheduled_for_update = scheduled.clone();
        let task = window.spawn(cx, async move |cx| {
            let result = render_job.await;
            cx.on_next_frame(move |_, cx| {
                entity_for_update.update(cx, |view, cx| {
                    view.finish_render(scheduled_for_update, result, cx);
                    cx.notify();
                });
            });
        });

        self.scheduler.start(scheduled);
        self.in_flight_render = Some(task);
        self.last_render_started = Some(Instant::now());
    }

    pub(super) fn finish_render(
        &mut self,
        scheduled: ScheduledRender,
        result: Result<RenderedFrame>,
        cx: &mut Context<Self>,
    ) {
        let Some(install) = self.scheduler.finish(&scheduled) else {
            return;
        };

        self.in_flight_render = None;

        if install {
            match result {
                Ok(frame) => {
                    self.failed_request = None;
                    self.last_error = None;
                    self.replace_cached_frame(scheduled.request.clone(), frame);
                }
                Err(PlottingError::RenderSuperseded) => {
                    self.failed_request = None;
                }
                Err(error) => {
                    self.failed_request = Some(scheduled.request.clone());
                    self.report_error(error, cx);
                }
            }
        }

        if self.scheduler.take_queued().is_some() {
            cx.notify();
        }
    }

    pub(super) fn local_viewport_point(
        &self,
        window_position: Point<Pixels>,
    ) -> Option<ViewportPoint> {
        let layout = self.last_layout.as_ref()?;
        if !layout.component_bounds.contains(&window_position)
            || !layout.content_bounds.contains(&window_position)
        {
            return None;
        }

        let local_x = f64::from(window_position.x - layout.content_bounds.origin.x);
        let local_y = f64::from(window_position.y - layout.content_bounds.origin.y);
        let content_width = f64::from(layout.content_bounds.size.width).max(1.0);
        let content_height = f64::from(layout.content_bounds.size.height).max(1.0);

        Some(ViewportPoint::new(
            ((local_x / content_width) * layout.frame_size_px.0 as f64)
                .clamp(0.0, layout.frame_size_px.0 as f64),
            ((local_y / content_height) * layout.frame_size_px.1 as f64)
                .clamp(0.0, layout.frame_size_px.1 as f64),
        ))
    }

    pub(super) fn clamped_viewport_point(
        &self,
        window_position: Point<Pixels>,
    ) -> Option<ViewportPoint> {
        let layout = self.last_layout.as_ref()?;
        let min_x = layout.content_bounds.origin.x;
        let min_y = layout.content_bounds.origin.y;
        let max_x = min_x + layout.content_bounds.size.width;
        let max_y = min_y + layout.content_bounds.size.height;
        let clamped = Point {
            x: window_position.x.max(min_x).min(max_x),
            y: window_position.y.max(min_y).min(max_y),
        };
        self.local_viewport_point(clamped)
    }

    pub(super) fn viewport_point_to_window_position(
        &self,
        viewport_point: ViewportPoint,
    ) -> Option<Point<Pixels>> {
        let layout = self.last_layout.as_ref()?;
        let content_width = f64::from(layout.content_bounds.size.width).max(1.0);
        let content_height = f64::from(layout.content_bounds.size.height).max(1.0);
        let normalized_x =
            (viewport_point.x / layout.frame_size_px.0.max(1) as f64).clamp(0.0, 1.0);
        let normalized_y =
            (viewport_point.y / layout.frame_size_px.1.max(1) as f64).clamp(0.0, 1.0);
        Some(Point {
            x: layout.component_bounds.origin.x
                + (layout.content_bounds.origin.x - layout.component_bounds.origin.x)
                + px((normalized_x * content_width) as f32),
            y: layout.component_bounds.origin.y
                + (layout.content_bounds.origin.y - layout.component_bounds.origin.y)
                + px((normalized_y * content_height) as f32),
        })
    }

    fn current_capture_target(&self, window: &Window) -> Option<ImageTarget> {
        if let Some(frame) = self.cached_frame.as_ref() {
            return Some(ImageTarget {
                size_px: frame.request.size_px,
                scale_factor: frame.request.scale_factor(),
                time_seconds: frame.request.time_seconds(),
            });
        }

        self.last_layout.as_ref().map(|layout| ImageTarget {
            size_px: layout.frame_size_px,
            scale_factor: window.scale_factor(),
            time_seconds: self.options.interaction.time_seconds,
        })
    }

    pub(super) fn capture_visible_view_image(&self, window: &Window) -> Result<RuvizImage> {
        if let Some(image) = self.capture_visible_view_image_from_cache() {
            return Ok(image);
        }

        let target = self.current_capture_target(window).ok_or_else(|| {
            PlottingError::InvalidInput(
                "plot image capture is unavailable before the GPUI view has been laid out"
                    .to_string(),
            )
        })?;
        let frame = self.session.render_to_image(target)?;
        Ok(frame.image.as_ref().clone())
    }

    pub(super) fn capture_visible_view_image_from_cache(&self) -> Option<RuvizImage> {
        let frame = self.cached_frame.as_ref()?;
        let mut image = match &frame.primary {
            PrimaryFrame::Image(primary) => render_image_to_ruviz(primary)?,
            #[cfg(all(feature = "gpu", target_os = "macos"))]
            PrimaryFrame::Surface(_) => return None,
        };

        if let Some(overlay) = frame.overlay_image.as_ref() {
            let overlay = render_image_to_ruviz(overlay)?;
            if (overlay.width, overlay.height) != (image.width, image.height) {
                // Blending row-major buffers of different widths shears the
                // overlay; fall back to a fresh session render instead.
                return None;
            }
            blend_rgba_into_rgba(&overlay.pixels, &mut image.pixels);
        }

        Some(image)
    }

    pub(super) fn build_action_context(
        &self,
        action_id: String,
        window: &Window,
        cursor_position_px: ViewportPoint,
    ) -> Result<Option<GpuiContextMenuActionContext>> {
        let snapshot = self.session.viewport_snapshot()?;
        let cursor_data_position = self.session.screen_to_data(cursor_position_px)?;
        let image = self.capture_visible_view_image(window)?;
        Ok(Some(GpuiContextMenuActionContext {
            action_id,
            visible_bounds: snapshot.visible_bounds,
            plot_area_px: snapshot.plot_area,
            frame_size_px: (image.width, image.height),
            scale_factor: self
                .current_capture_target(window)
                .map_or(1.0, |t| t.scale_factor),
            cursor_position_px,
            cursor_data_position,
            image,
        }))
    }

    pub(super) fn copy_text_to_clipboard(&self, text: &str) -> Result<()> {
        let mut clipboard = Clipboard::new()
            .map_err(|err| PlottingError::SystemError(format!("clipboard unavailable: {err}")))?;
        clipboard
            .set_text(text.to_string())
            .map_err(|err| PlottingError::SystemError(format!("failed to copy text: {err}")))
    }

    pub(super) fn copy_image_to_clipboard(&self, image: &RuvizImage) -> Result<()> {
        let mut clipboard = Clipboard::new()
            .map_err(|err| PlottingError::SystemError(format!("clipboard unavailable: {err}")))?;
        clipboard
            .set_image(ImageData {
                width: image.width as usize,
                height: image.height as usize,
                bytes: Cow::Owned(image.pixels.clone()),
            })
            .map_err(|err| PlottingError::SystemError(format!("failed to copy image: {err}")))
    }

    fn default_export_filename(&self) -> String {
        "gpui-plot.png".to_string()
    }

    pub(super) fn spawn_save_png_dialog(&self, image: RuvizImage) -> Result<()> {
        let file_name = self.default_export_filename();
        let dialog = rfd::AsyncFileDialog::new()
            .add_filter("PNG image", &["png"])
            .set_file_name(&file_name);

        std::thread::Builder::new()
            .name("ruviz-gpui-save-png".to_string())
            .spawn(move || {
                let Some(file_handle) = block_on(dialog.save_file()) else {
                    return;
                };
                if let Err(err) = write_rgba_png_atomic(file_handle.path(), &image) {
                    eprintln!(
                        "ruviz-gpui: failed to export PNG to {}: {err}",
                        file_handle.path().display()
                    );
                }
            })
            .map(|_| ())
            .map_err(|err| {
                PlottingError::SystemError(format!("failed to spawn GPUI PNG export worker: {err}"))
            })
    }
}

pub(super) fn frame_size_px_for_policy(
    session: &InteractivePlotSession,
    sizing_policy: &SizingPolicy,
    logical_size_px: (f64, f64),
    scale_factor: f32,
) -> Option<(u32, u32)> {
    let size_px = match sizing_policy {
        SizingPolicy::Fill => (
            fill_backing_dimension_px(logical_size_px.0, scale_factor),
            fill_backing_dimension_px(logical_size_px.1, scale_factor),
        ),
        SizingPolicy::FixedPixels { width, height } => (*width, *height),
    };

    if size_px.0 == 0 || size_px.1 == 0 {
        return None;
    }

    match sizing_policy {
        SizingPolicy::Fill => Some(session.fitted_frame_size_px(size_px)),
        SizingPolicy::FixedPixels { .. } => Some(size_px),
    }
}

pub(super) fn bind_reactive_session(
    session: &InteractivePlotSession,
) -> (Arc<AtomicBool>, UnboundedReceiver<()>, ReactiveSubscription) {
    let reactive_notify_pending = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = unbounded();
    let pending_for_callback = Arc::clone(&reactive_notify_pending);
    let subscription = session.subscribe_reactive(move || {
        if pending_for_callback
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            let _ = sender.unbounded_send(());
        }
    });

    (reactive_notify_pending, receiver, subscription)
}

pub(super) fn apply_performance_options(
    session: &InteractivePlotSession,
    options: PerformanceOptions,
) {
    session.set_frame_pacing(options.frame_pacing);
    session.set_quality_policy(options.quality_policy);
    session.set_prefer_gpu(options.prefer_gpu);
}

pub(super) fn active_backend_for_frame(frame: &CachedFrame) -> ActiveBackend {
    match frame.target {
        RenderTargetKind::Image => ActiveBackend::Image,
        #[cfg(all(feature = "gpu", target_os = "macos"))]
        RenderTargetKind::Surface => match frame.primary {
            PrimaryFrame::Surface(_) => ActiveBackend::HybridFastPath,
            PrimaryFrame::Image(_) => ActiveBackend::HybridFallback,
        },
        #[cfg(not(all(feature = "gpu", target_os = "macos")))]
        RenderTargetKind::Surface => ActiveBackend::HybridFallback,
    }
}

pub(super) fn sanitize_scale_factor(scale_factor: f32) -> f32 {
    if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    }
}

#[allow(deprecated)]
pub(super) fn resolve_presentation_mode(requested: PresentationMode) -> PresentationMode {
    match requested {
        PresentationMode::Image => PresentationMode::Image,
        PresentationMode::Hybrid => {
            #[cfg(feature = "gpu")]
            {
                PresentationMode::Hybrid
            }
            #[cfg(not(feature = "gpu"))]
            {
                PresentationMode::Image
            }
        }
        PresentationMode::SurfaceExperimental => {
            #[cfg(feature = "gpu")]
            {
                PresentationMode::Hybrid
            }
            #[cfg(not(feature = "gpu"))]
            {
                PresentationMode::Image
            }
        }
    }
}

#[cfg(all(feature = "gpu", target_os = "macos"))]
pub(super) fn maybe_retire_replaced_primary(
    retired_images: &mut Vec<Arc<RenderImage>>,
    previous: &PrimaryFrame,
    current: &PrimaryFrame,
) {
    match (previous, current) {
        (PrimaryFrame::Image(previous), PrimaryFrame::Image(current))
            if !Arc::ptr_eq(previous, current) =>
        {
            retired_images.push(Arc::clone(previous));
        }
        (PrimaryFrame::Image(previous), _) => retired_images.push(Arc::clone(previous)),
        _ => {}
    }
}

#[cfg(not(all(feature = "gpu", target_os = "macos")))]
pub(super) fn maybe_retire_replaced_primary(
    retired_images: &mut Vec<Arc<RenderImage>>,
    previous: &PrimaryFrame,
    current: &PrimaryFrame,
) {
    let PrimaryFrame::Image(previous) = previous;
    let PrimaryFrame::Image(current) = current;
    if !Arc::ptr_eq(previous, current) {
        retired_images.push(Arc::clone(previous));
    }
}

#[cfg(all(feature = "gpu", target_os = "macos"))]
#[allow(deprecated)]
pub(super) fn should_use_surface_primary(
    presentation_mode: PresentationMode,
    target: RenderTargetKind,
    surface_capability: SurfaceCapability,
) -> bool {
    matches!(
        presentation_mode,
        PresentationMode::Hybrid | PresentationMode::SurfaceExperimental
    ) && target == RenderTargetKind::Surface
        && surface_capability == SurfaceCapability::FastPath
}

#[cfg(not(all(feature = "gpu", target_os = "macos")))]
pub(super) fn should_use_surface_primary(
    _presentation_mode: PresentationMode,
    _target: RenderTargetKind,
    _surface_capability: SurfaceCapability,
) -> bool {
    false
}

#[cfg(all(feature = "gpu", target_os = "macos"))]
pub(super) fn make_surface_pixel_buffer_options() -> CFDictionary<CFString, CFType> {
    let iosurface_key: CFString = CVPixelBufferKeys::IOSurfaceProperties.into();
    let metal_key: CFString = CVPixelBufferKeys::MetalCompatibility.into();
    let cg_image_key: CFString = CVPixelBufferKeys::CGImageCompatibility.into();
    let bitmap_context_key: CFString = CVPixelBufferKeys::CGBitmapContextCompatibility.into();
    let iosurface_value = CFDictionary::<CFString, CFType>::from_CFType_pairs(&[]);

    CFDictionary::from_CFType_pairs(&[
        (iosurface_key, iosurface_value.as_CFType()),
        (metal_key, CFBoolean::true_value().as_CFType()),
        (cg_image_key, CFBoolean::true_value().as_CFType()),
        (bitmap_context_key, CFBoolean::true_value().as_CFType()),
    ])
}

#[cfg(all(feature = "gpu", target_os = "macos"))]
impl SurfaceUploadState {
    pub(super) fn update(
        &mut self,
        previous: Option<&CVPixelBuffer>,
        layer: &RenderedLayer,
    ) -> std::result::Result<CVPixelBuffer, String> {
        let width = layer.width() as usize;
        let height = layer.height() as usize;
        let pixel_buffer = match previous {
            Some(previous) if previous.get_width() == width && previous.get_height() == height => {
                previous.clone()
            }
            _ => CVPixelBuffer::new(
                kCVPixelFormatType_420YpCbCr8BiPlanarFullRange,
                width,
                height,
                Some(&self.pixel_buffer_options),
            )
            .map_err(|status| format!("Failed to create CVPixelBuffer: {status}"))?,
        };

        write_surface_pixels(
            &pixel_buffer,
            width,
            height,
            layer.pixels(),
            layer.alpha_mode(),
        )?;
        Ok(pixel_buffer)
    }
}

/// Whether two frames lay out their plot area with the same size (within
/// half a frame pixel), so one's content can be shown inside the other's axes.
pub(super) fn same_plot_area_size(a: &FrameView, b: &FrameView) -> bool {
    let (aw, ah) = (
        a.plot_area.max.x - a.plot_area.min.x,
        a.plot_area.max.y - a.plot_area.min.y,
    );
    let (bw, bh) = (
        b.plot_area.max.x - b.plot_area.min.x,
        b.plot_area.max.y - b.plot_area.min.y,
    );
    (aw - bw).abs() <= 0.5 && (ah - bh).abs() <= 0.5
}

#[cfg(all(feature = "gpu", target_os = "macos"))]
fn write_surface_pixels(
    pixel_buffer: &CVPixelBuffer,
    width: usize,
    height: usize,
    rgba_pixels: &[u8],
    alpha_mode: AlphaMode,
) -> std::result::Result<(), String> {
    if rgba_pixels.len() < width * height * 4 {
        return Err(format!(
            "surface source has {} bytes, expected at least {} for {width}x{height}",
            rgba_pixels.len(),
            width * height * 4
        ));
    }

    let lock_status = pixel_buffer.lock_base_address(0);
    if lock_status != 0 {
        return Err(format!(
            "Failed to lock CVPixelBuffer base address: {lock_status}"
        ));
    }

    let copy_result = (|| {
        if !pixel_buffer.is_planar() || pixel_buffer.get_plane_count() < 2 {
            return Err("Expected a bi-planar 420f CVPixelBuffer".to_string());
        }

        let y_width = pixel_buffer.get_width_of_plane(0).min(width);
        let y_height = pixel_buffer.get_height_of_plane(0).min(height);
        let y_stride = pixel_buffer.get_bytes_per_row_of_plane(0);
        let y_plane = unsafe { pixel_buffer.get_base_address_of_plane(0) } as *mut u8;
        if y_plane.is_null() {
            return Err("CVPixelBuffer luma plane base address was null".to_string());
        }

        let row_bytes = width * 4;
        for row in 0..y_height {
            let src = &rgba_pixels[row * row_bytes..row * row_bytes + y_width * 4];
            // SAFETY: the plane is locked for the duration of this closure and
            // `y_width` is clamped to the plane width, so every write stays
            // inside row `row` of the plane.
            let dst =
                unsafe { std::slice::from_raw_parts_mut(y_plane.add(row * y_stride), y_width) };
            for (dst, px) in dst.iter_mut().zip(src.chunks_exact(4)) {
                let (r, g, b) = straight_rgb(px, alpha_mode);
                *dst = luma_full_range(r, g, b);
            }
        }

        let uv_width = pixel_buffer.get_width_of_plane(1);
        let uv_height = pixel_buffer.get_height_of_plane(1);
        let uv_stride = pixel_buffer.get_bytes_per_row_of_plane(1);
        let uv_plane = unsafe { pixel_buffer.get_base_address_of_plane(1) } as *mut u8;
        if uv_plane.is_null() {
            return Err("CVPixelBuffer chroma plane base address was null".to_string());
        }

        for uv_row in 0..uv_height {
            let y0 = uv_row * 2;
            if y0 >= height {
                break;
            }
            let rows = if y0 + 1 < height { 2 } else { 1 };
            let row0 = &rgba_pixels[y0 * row_bytes..y0 * row_bytes + row_bytes];
            let row1 =
                &rgba_pixels[(y0 + rows - 1) * row_bytes..(y0 + rows - 1) * row_bytes + row_bytes];
            let uv_cols = uv_width.min(width.div_ceil(2));
            // SAFETY: as above, for row `uv_row` of the chroma plane; `uv_cols`
            // is clamped to the plane width so `uv_cols * 2` bytes fit.
            let dst = unsafe {
                std::slice::from_raw_parts_mut(uv_plane.add(uv_row * uv_stride), uv_cols * 2)
            };
            for (uv_col, dst) in dst.chunks_exact_mut(2).enumerate() {
                let x0 = uv_col * 2;
                let cols = if x0 + 1 < width { 2 } else { 1 };
                let mut r_sum: u32 = 0;
                let mut g_sum: u32 = 0;
                let mut b_sum: u32 = 0;
                for source_row in [row0, row1].iter().take(rows) {
                    for x in x0..x0 + cols {
                        let (r, g, b) = straight_rgb(&source_row[x * 4..x * 4 + 4], alpha_mode);
                        r_sum += u32::from(r);
                        g_sum += u32::from(g);
                        b_sum += u32::from(b);
                    }
                }
                let sample_count = (rows * cols) as u32;
                let r = (r_sum / sample_count) as u8;
                let g = (g_sum / sample_count) as u8;
                let b = (b_sum / sample_count) as u8;
                let (_, cb, cr) = rgb_to_ycbcr_full_range(r, g, b);
                dst[0] = cb;
                dst[1] = cr;
            }
        }

        Ok(())
    })();

    let unlock_status = pixel_buffer.unlock_base_address(0);
    if unlock_status != 0 {
        return match copy_result {
            Ok(()) => Err(format!(
                "Failed to unlock CVPixelBuffer base address: {unlock_status}"
            )),
            Err(copy_err) => Err(format!(
                "Failed to unlock CVPixelBuffer base address: {unlock_status}; copy error: {copy_err}"
            )),
        };
    }

    copy_result
}

/// Straight RGB of one pixel, undoing premultiplication only where the alpha
/// says it changed something. A plot's base layer is opaque, so this is a
/// branch per pixel rather than a full-frame divide.
#[cfg(all(feature = "gpu", target_os = "macos"))]
#[inline]
fn straight_rgb(px: &[u8], alpha_mode: AlphaMode) -> (u8, u8, u8) {
    let (r, g, b, a) = (px[0], px[1], px[2], px[3]);
    match alpha_mode {
        AlphaMode::Straight => (r, g, b),
        AlphaMode::Premultiplied => match a {
            255 => (r, g, b),
            0 => (0, 0, 0),
            a => {
                let a = u32::from(a);
                let un = |c: u8| ((u32::from(c) * 255 + a / 2) / a).min(255) as u8;
                (un(r), un(g), un(b))
            }
        },
    }
}

#[cfg(all(feature = "gpu", target_os = "macos"))]
#[inline]
fn luma_full_range(r: u8, g: u8, b: u8) -> u8 {
    ((77 * i32::from(r) + 150 * i32::from(g) + 29 * i32::from(b) + 128) >> 8).clamp(0, 255) as u8
}

#[cfg(all(feature = "gpu", target_os = "macos"))]
fn rgb_to_ycbcr_full_range(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let r = r as i32;
    let g = g as i32;
    let b = b as i32;

    let y = ((77 * r + 150 * g + 29 * b + 128) >> 8).clamp(0, 255) as u8;
    let cb = (((-43 * r - 85 * g + 128 * b + 128) >> 8) + 128).clamp(0, 255) as u8;
    let cr = (((128 * r - 107 * g - 21 * b + 128) >> 8) + 128).clamp(0, 255) as u8;

    (y, cb, cr)
}

pub(super) fn render_frame_from_session(
    session: InteractivePlotSession,
    request: RenderRequest,
) -> Result<RenderedFrame> {
    let frame = match request.presentation_mode {
        PresentationMode::Image => {
            let result = session.render_to_image_with_generation(ImageTarget {
                size_px: request.size_px,
                scale_factor: request.scale_factor(),
                time_seconds: request.time_seconds(),
            })?;
            RenderedFrame {
                base_generation: result.base_generation,
                primary: Some(RenderedPrimary::Image(render_image_from_ruviz(
                    result.frame.image.as_ref().clone(),
                ))),
                overlay: RenderedOverlay::Replace(None),
                stats: result.frame.stats,
                target: result.frame.target,
                view: None,
            }
        }
        #[allow(deprecated)]
        PresentationMode::Hybrid | PresentationMode::SurfaceExperimental => {
            // Layers stay in the renderer's native alpha representation: the
            // surface upload converts straight from them, so no frame pays
            // the full-size premultiplied -> straight pass.
            let layers = session.render_surface_layers_stamped(SurfaceTarget {
                size_px: request.size_px,
                scale_factor: request.scale_factor(),
                time_seconds: request.time_seconds(),
            })?;
            let base_generation = layers.base_generation;
            let layer_state = layers.layer_state;
            let generation_changed = request.presented_base_generation != Some(base_generation);
            let include_base = layer_state.base_dirty || generation_changed;
            let use_surface_primary = should_use_surface_primary(
                request.presentation_mode,
                layers.target,
                layers.surface_capability,
            );
            let primary = if use_surface_primary {
                #[cfg(all(feature = "gpu", target_os = "macos"))]
                {
                    include_base.then(|| RenderedPrimary::Surface(layers.base.clone()))
                }
                #[cfg(not(all(feature = "gpu", target_os = "macos")))]
                {
                    unreachable!("surface primary is only enabled on macOS with the gpu feature")
                }
            } else {
                include_base.then(|| {
                    RenderedPrimary::Image(render_image_from_ruviz(
                        layers.base.image().as_ref().clone(),
                    ))
                })
            };
            let overlay = if layer_state.overlay_dirty || generation_changed {
                RenderedOverlay::Replace(
                    layers
                        .overlay
                        .as_ref()
                        .map(|overlay| render_image_from_ruviz(overlay.image().as_ref().clone())),
                )
            } else {
                RenderedOverlay::Reuse
            };
            RenderedFrame {
                base_generation,
                primary,
                overlay,
                stats: layers.stats,
                target: layers.target,
                view: None,
            }
        }
    };
    // The session's displayed geometry is this frame's: one render is in
    // flight per view, and the worker just committed it.
    let view = frame_view_from_session(&session);
    Ok(RenderedFrame { view, ..frame })
}

pub(super) fn render_image_from_ruviz(image: RuvizImage) -> Arc<RenderImage> {
    let width = image.width;
    let height = image.height;
    // GPUI's `RenderImage` byte contract is straight-alpha BGRA. Normalize
    // explicitly from ruviz's recorded alpha mode before swapping channels;
    // translucent renderer output must never rely on a guessed convention.
    let mut pixels = image.pixels_in_alpha_mode(AlphaMode::Straight).into_owned();
    rgba_to_bgra_in_place(&mut pixels);
    let actual_len = pixels.len();
    let expected_len = width as usize * height as usize * 4;
    let buffer = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(width, height, pixels)
        .unwrap_or_else(|| {
            panic!(
                "rendered frame size must match RGBA pixel buffer ({}x{}, expected {} bytes, got {})",
                width, height, expected_len, actual_len
            )
        });
    Arc::new(RenderImage::new(smallvec![Frame::new(buffer)]))
}

pub(super) fn render_image_to_ruviz(image: &RenderImage) -> Option<RuvizImage> {
    let size = image.size(0);
    let width = u32::from(size.width);
    let height = u32::from(size.height);
    let mut pixels = image.as_bytes(0)?.to_vec();
    rgba_to_bgra_in_place(&mut pixels);
    Some(RuvizImage::new(width, height, pixels))
}

pub(super) fn rgba_to_bgra_in_place(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
}

pub(super) fn blend_rgba_into_rgba(src_rgba: &[u8], dst_rgba: &mut [u8]) {
    for (src, dst) in src_rgba.chunks_exact(4).zip(dst_rgba.chunks_exact_mut(4)) {
        let destination = [dst[0], dst[1], dst[2], dst[3]];
        let source = [src[0], src[1], src[2], src[3]];
        dst.copy_from_slice(&source_over_straight_rgba(destination, source));
    }
}

#[cfg(all(test, feature = "gpu", target_os = "macos"))]
mod surface_tests {
    use super::*;

    #[test]
    fn straight_rgb_undoes_premultiplication_only_where_alpha_asks() {
        assert_eq!(
            straight_rgb(&[10, 20, 30, 255], AlphaMode::Premultiplied),
            (10, 20, 30)
        );
        assert_eq!(
            straight_rgb(&[10, 20, 30, 0], AlphaMode::Premultiplied),
            (0, 0, 0)
        );
        assert_eq!(
            straight_rgb(&[64, 32, 16, 128], AlphaMode::Premultiplied),
            (128, 64, 32)
        );
        assert_eq!(
            straight_rgb(&[64, 32, 16, 128], AlphaMode::Straight),
            (64, 32, 16)
        );
    }

    #[test]
    fn surface_upload_from_premultiplied_layer_matches_straight_layer() {
        // A half-transparent premultiplied pixel and its straight twin must
        // land as the same luma/chroma: the upload un-premultiplies inline
        // instead of asking the layer for a materialized straight view.
        let straight = RenderedLayer::from_straight_image(Arc::new(RuvizImage::new(
            2,
            2,
            vec![
                200, 100, 50, 255, 128, 64, 32, 128, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        )));
        let premultiplied = RenderedLayer::from_premultiplied_pixels(
            2,
            2,
            vec![
                200, 100, 50, 255, 64, 32, 16, 128, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        );
        let mut upload = SurfaceUploadState::default();
        let a = upload.update(None, &straight).expect("straight upload");
        let mut upload = SurfaceUploadState::default();
        let b = upload
            .update(None, &premultiplied)
            .expect("premultiplied upload");
        let read = |buffer: &CVPixelBuffer| {
            buffer.lock_base_address(1);
            let mut bytes = Vec::new();
            for plane in 0..2 {
                let stride = buffer.get_bytes_per_row_of_plane(plane);
                let rows = buffer.get_height_of_plane(plane);
                let cols = buffer.get_width_of_plane(plane) * if plane == 1 { 2 } else { 1 };
                let base = unsafe { buffer.get_base_address_of_plane(plane) } as *const u8;
                for row in 0..rows {
                    bytes.extend_from_slice(unsafe {
                        std::slice::from_raw_parts(base.add(row * stride), cols)
                    });
                }
            }
            buffer.unlock_base_address(1);
            bytes
        };
        assert_eq!(read(&a), read(&b));
        assert!(
            !premultiplied.has_straight_view(),
            "no straight view was materialized"
        );
    }
}
