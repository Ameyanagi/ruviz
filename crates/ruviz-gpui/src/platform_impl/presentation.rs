use super::*;

impl RuvizPlot {
    pub(super) fn retire_cached_frame(&mut self) {
        if let Some(frame) = self.cached_frame.take() {
            retire_primary_frame(&mut self.retired_images, frame.primary);
            if let Some(overlay_image) = frame.overlay_image {
                self.retired_images.push(overlay_image);
            }
        }
    }

    fn replace_cached_frame(&mut self, request: RenderRequest, mut frame: RenderedFrame) {
        let previous = self.cached_frame.take();
        let primary = self
            .resolve_primary_frame(previous.as_ref(), &mut frame)
            .expect("rendered frame must include a primary layer on first render");
        let overlay_image = if frame.target == RenderTargetKind::Image {
            None
        } else {
            frame.overlay_image.or_else(|| {
                previous
                    .as_ref()
                    .and_then(|cached| cached.overlay_image.as_ref().map(Arc::clone))
            })
        };

        if let Some(previous) = previous {
            maybe_retire_replaced_primary(&mut self.retired_images, &previous.primary, &primary);
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
            primary,
            overlay_image,
            stats: frame.stats,
            target: frame.target,
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
            Some(RenderedPrimary::Surface(base_image)) => {
                let previous_surface = previous.and_then(|cached| match &cached.primary {
                    PrimaryFrame::Surface(surface) => Some(surface),
                    PrimaryFrame::Image(_) => None,
                });

                match self
                    .surface_upload
                    .update(previous_surface, base_image.as_ref())
                {
                    Ok(surface) => Some(PrimaryFrame::Surface(surface)),
                    Err(_) => Some(PrimaryFrame::Image(render_image_from_ruviz(
                        base_image.as_ref().clone(),
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
                    entity_for_notify.update(cx, |_, cx| {
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
        let size_px = match self.options.sizing_policy {
            SizingPolicy::Fill => {
                let width = u32::from(bounds.size.width.ceil());
                let height = u32::from(bounds.size.height.ceil());
                (width, height)
            }
            SizingPolicy::FixedPixels { width, height } => (width, height),
        };

        if size_px.0 == 0 || size_px.1 == 0 {
            return None;
        }

        Some(RenderRequest::new(
            size_px,
            window.scale_factor(),
            self.options.interaction.time_seconds,
            self.effective_presentation_mode(),
        ))
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
            self.update_layout(bounds, request.size_px);
            self.start_render_if_needed(entity, request, window, cx);
        } else {
            self.last_layout = None;
        }

        self.cached_frame.as_ref().map(|frame| PaintFrame {
            primary: frame.primary.clone(),
            overlay_image: frame.overlay_image.as_ref().map(Arc::clone),
        })
    }

    fn update_layout(&mut self, bounds: Bounds<Pixels>, frame_size_px: (u32, u32)) {
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
        let cache_is_current = self
            .cached_frame
            .as_ref()
            .is_some_and(|frame| frame.request == request && !request.is_dirty(&self.session));
        if cache_is_current {
            return;
        }

        let Some(scheduled) = self.scheduler.schedule(request) else {
            return;
        };

        self.start_render(entity, scheduled, window, cx);
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
    }

    fn finish_render(
        &mut self,
        scheduled: ScheduledRender,
        result: std::result::Result<RenderedFrame, String>,
        cx: &mut Context<Self>,
    ) {
        if !self.scheduler.finish(&scheduled) {
            return;
        }

        self.in_flight_render = None;

        if let Ok(frame) = result {
            self.replace_cached_frame(scheduled.request.clone(), frame);
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
        if !layout.content_bounds.contains(&window_position) {
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
        let cursor_data_position = cursor_data_position(
            snapshot.visible_bounds,
            snapshot.plot_area,
            cursor_position_px,
        );
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

pub(super) fn retire_primary_frame(
    retired_images: &mut Vec<Arc<RenderImage>>,
    primary: PrimaryFrame,
) {
    match primary {
        PrimaryFrame::Image(image) => retired_images.push(image),
        #[cfg(all(feature = "gpu", target_os = "macos"))]
        PrimaryFrame::Surface(_) => {}
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
        image: &RuvizImage,
    ) -> std::result::Result<CVPixelBuffer, String> {
        let width = image.width as usize;
        let height = image.height as usize;
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

        write_surface_pixels(&pixel_buffer, width, height, &image.pixels)?;
        Ok(pixel_buffer)
    }
}

#[cfg(all(feature = "gpu", target_os = "macos"))]
fn write_surface_pixels(
    pixel_buffer: &CVPixelBuffer,
    width: usize,
    height: usize,
    rgba_pixels: &[u8],
) -> std::result::Result<(), String> {
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

        let y_width = pixel_buffer.get_width_of_plane(0);
        let y_height = pixel_buffer.get_height_of_plane(0);
        let y_stride = pixel_buffer.get_bytes_per_row_of_plane(0);
        let y_plane = unsafe { pixel_buffer.get_base_address_of_plane(0) } as *mut u8;
        if y_plane.is_null() {
            return Err("CVPixelBuffer luma plane base address was null".to_string());
        }

        for row in 0..height.min(y_height) {
            for col in 0..width.min(y_width) {
                let pixel = rgba_at(rgba_pixels, width, row, col);
                let y = rgb_to_ycbcr_full_range(pixel.0, pixel.1, pixel.2).0;
                unsafe {
                    *y_plane.add(row * y_stride + col) = y;
                }
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
            for uv_col in 0..uv_width {
                let x0 = uv_col * 2;
                let y0 = uv_row * 2;
                if x0 >= width || y0 >= height {
                    continue;
                }

                let mut r_sum: u32 = 0;
                let mut g_sum: u32 = 0;
                let mut b_sum: u32 = 0;
                let mut sample_count: u32 = 0;

                for sample_y in y0..(y0 + 2).min(height) {
                    for sample_x in x0..(x0 + 2).min(width) {
                        let (r, g, b, _) = rgba_at(rgba_pixels, width, sample_y, sample_x);
                        r_sum += r as u32;
                        g_sum += g as u32;
                        b_sum += b as u32;
                        sample_count += 1;
                    }
                }

                if sample_count == 0 {
                    continue;
                }

                let r = (r_sum / sample_count) as u8;
                let g = (g_sum / sample_count) as u8;
                let b = (b_sum / sample_count) as u8;
                let (_, cb, cr) = rgb_to_ycbcr_full_range(r, g, b);
                let uv_offset = uv_row * uv_stride + uv_col * 2;
                unsafe {
                    *uv_plane.add(uv_offset) = cb;
                    *uv_plane.add(uv_offset + 1) = cr;
                }
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

#[cfg(all(feature = "gpu", target_os = "macos"))]
fn rgba_at(rgba_pixels: &[u8], width: usize, row: usize, col: usize) -> (u8, u8, u8, u8) {
    let offset = (row * width + col) * 4;
    let end = offset.saturating_add(4);
    debug_assert!(
        rgba_pixels.len() >= end,
        "pixel buffer too small for ({row}, {col}) in {width}-wide image: len={} need>={end}",
        rgba_pixels.len()
    );
    (
        rgba_pixels[offset],
        rgba_pixels[offset + 1],
        rgba_pixels[offset + 2],
        rgba_pixels[offset + 3],
    )
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

fn render_frame_from_session(
    session: InteractivePlotSession,
    request: RenderRequest,
) -> std::result::Result<RenderedFrame, String> {
    let frame = match request.presentation_mode {
        PresentationMode::Image => session
            .render_to_image(ImageTarget {
                size_px: request.size_px,
                scale_factor: request.scale_factor(),
                time_seconds: request.time_seconds(),
            })
            .map_err(|err| err.to_string())?,
        PresentationMode::Hybrid => session
            .render_to_surface(SurfaceTarget {
                size_px: request.size_px,
                scale_factor: request.scale_factor(),
                time_seconds: request.time_seconds(),
            })
            .map_err(|err| err.to_string())?,
        #[allow(deprecated)]
        PresentationMode::SurfaceExperimental => session
            .render_to_surface(SurfaceTarget {
                size_px: request.size_px,
                scale_factor: request.scale_factor(),
                time_seconds: request.time_seconds(),
            })
            .map_err(|err| err.to_string())?,
    };

    let layer_state = frame.layer_state;
    let use_surface_primary = should_use_surface_primary(
        request.presentation_mode,
        frame.target,
        frame.surface_capability,
    );
    Ok(RenderedFrame {
        primary: if use_surface_primary {
            #[cfg(all(feature = "gpu", target_os = "macos"))]
            {
                layer_state
                    .base_dirty
                    .then(|| RenderedPrimary::Surface(Arc::clone(&frame.layers.base)))
            }
            #[cfg(not(all(feature = "gpu", target_os = "macos")))]
            {
                unreachable!("surface primary is only enabled on macOS with the gpu feature")
            }
        } else {
            match request.presentation_mode {
                PresentationMode::Image => Some(RenderedPrimary::Image(render_image_from_ruviz(
                    frame.image.as_ref().clone(),
                ))),
                PresentationMode::Hybrid => layer_state.base_dirty.then(|| {
                    RenderedPrimary::Image(render_image_from_ruviz(
                        frame.layers.base.as_ref().clone(),
                    ))
                }),
                #[allow(deprecated)]
                PresentationMode::SurfaceExperimental => layer_state.base_dirty.then(|| {
                    RenderedPrimary::Image(render_image_from_ruviz(
                        frame.layers.base.as_ref().clone(),
                    ))
                }),
            }
        },
        overlay_image: if matches!(request.presentation_mode, PresentationMode::Image) {
            None
        } else {
            frame.layers.overlay.as_ref().and_then(|overlay| {
                layer_state
                    .overlay_dirty
                    .then(|| render_image_from_ruviz(overlay.as_ref().clone()))
            })
        },
        stats: frame.stats,
        target: frame.target,
    })
}

pub(super) fn render_image_from_ruviz(image: RuvizImage) -> Arc<RenderImage> {
    let width = image.width;
    let height = image.height;
    let mut pixels = image.pixels;
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
        let alpha = src[3];
        if alpha == 0 {
            continue;
        }
        if alpha == u8::MAX {
            dst.copy_from_slice(src);
            continue;
        }

        let alpha = alpha as f32 / 255.0;
        dst[0] = blend_channel(dst[0], src[0], alpha);
        dst[1] = blend_channel(dst[1], src[1], alpha);
        dst[2] = blend_channel(dst[2], src[2], alpha);
        dst[3] = alpha_blend_alpha(dst[3], src[3]);
    }
}

fn blend_channel(background: u8, foreground: u8, alpha: f32) -> u8 {
    let bg = background as f32 / 255.0;
    let fg = foreground as f32 / 255.0;
    ((bg * (1.0 - alpha) + fg * alpha) * 255.0) as u8
}

fn alpha_blend_alpha(background: u8, foreground: u8) -> u8 {
    let bg = background as f32 / 255.0;
    let fg = foreground as f32 / 255.0;
    ((fg + bg * (1.0 - fg)) * 255.0) as u8
}
