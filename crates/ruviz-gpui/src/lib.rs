#![allow(dead_code)]

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
compile_error!("ruviz-gpui currently supports macOS and Linux only.");

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod supported {
    use futures::{
        StreamExt,
        channel::mpsc::{UnboundedReceiver, unbounded},
    };
    use gpui::{
        App, Bounds, Context, Corners, Entity, IntoElement, MouseButton, MouseDownEvent,
        InteractiveElement, MouseMoveEvent, MouseUpEvent, ObjectFit, Pixels, Render, RenderImage,
        ScrollWheelEvent, Task, Window, canvas, div, prelude::*, px, size,
    };
    use image::{Frame, ImageBuffer, Rgba};
    use ruviz::{
        core::{
            FramePacing, FrameStats, ImageTarget, InteractivePlotSession, Plot, PlotInputEvent,
            PreparedPlot, QualityPolicy, ReactiveSubscription, RenderTargetKind,
            SurfaceCapability, SurfaceTarget, ViewportPoint,
        },
        core::plot::Image as RuvizImage,
    };
    use smallvec::smallvec;
    use std::{
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
        time::{Duration, Instant},
    };

    pub use gpui;
    pub use ruviz;

    pub trait IntoRuvizSession {
        fn into_session(self) -> InteractivePlotSession;
    }

    impl IntoRuvizSession for InteractivePlotSession {
        fn into_session(self) -> InteractivePlotSession {
            self
        }
    }

    impl IntoRuvizSession for PreparedPlot {
        fn into_session(self) -> InteractivePlotSession {
            self.into_interactive()
        }
    }

    impl IntoRuvizSession for Plot {
        fn into_session(self) -> InteractivePlotSession {
            self.prepare_interactive()
        }
    }

    /// Presentation backend for the GPUI embed.
    ///
    /// `Hybrid` is the default presentation path. When GPU support is unavailable,
    /// it falls back to the stable image-backed renderer.
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub enum PresentationMode {
        Image,
        #[default]
        Hybrid,
        #[deprecated(note = "Use Hybrid; this alias falls back to Hybrid/Image.")]
        SurfaceExperimental,
    }

    /// Layout policy for the embedded plot.
    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    pub enum SizingPolicy {
        #[default]
        Fill,
        FixedPixels {
            width: u32,
            height: u32,
        },
    }

    /// Image-fit policy used by the image-backed presentation mode.
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub enum ImageFit {
        #[default]
        Contain,
        Cover,
        Fill,
    }

    impl ImageFit {
        fn into_gpui(self) -> ObjectFit {
            match self {
                Self::Contain => ObjectFit::Contain,
                Self::Cover => ObjectFit::Cover,
                Self::Fill => ObjectFit::Fill,
            }
        }
    }

    /// Time and interaction settings for the embedded plot.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct InteractionOptions {
        pub time_seconds: f64,
        pub image_fit: ImageFit,
        pub pan: bool,
        pub zoom: bool,
        pub hover: bool,
        pub selection: bool,
        pub tooltips: bool,
    }

    impl Default for InteractionOptions {
        fn default() -> Self {
            Self {
                time_seconds: 0.0,
                image_fit: ImageFit::Contain,
                pan: true,
                zoom: true,
                hover: true,
                selection: true,
                tooltips: true,
            }
        }
    }

    /// Performance tuning options for the shared interactive session.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct PerformanceOptions {
        pub frame_pacing: FramePacing,
        pub quality_policy: QualityPolicy,
        pub prefer_gpu: bool,
    }

    impl Default for PerformanceOptions {
        fn default() -> Self {
            Self {
                frame_pacing: FramePacing::Display,
                quality_policy: QualityPolicy::Balanced,
                prefer_gpu: cfg!(feature = "gpu"),
            }
        }
    }

    /// Top-level configuration for the GPUI plot component.
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct RuvizPlotOptions {
        pub presentation_mode: PresentationMode,
        pub sizing_policy: SizingPolicy,
        pub performance: PerformanceOptions,
        pub interaction: InteractionOptions,
    }

    /// Presentation cadence measured from GPUI's paint phase.
    ///
    /// This is distinct from [`FrameStats`], which tracks how quickly `ruviz`
    /// produces frames. `PresentationStats` approximates how often GPUI is
    /// actually painting this component.
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct PresentationStats {
        pub frame_count: u64,
        pub last_present_interval: Duration,
        pub average_present_interval: Duration,
        pub current_fps: f64,
    }

    #[derive(Debug, Default)]
    struct PresentationClock {
        stats: PresentationStats,
        last_presented_at: Option<Instant>,
    }

    impl PresentationClock {
        fn stats(&self) -> PresentationStats {
            self.stats.clone()
        }

        fn reset(&mut self) {
            *self = Self::default();
        }

        fn record_now(&mut self) {
            self.record_at(Instant::now());
        }

        fn record_at(&mut self, now: Instant) {
            self.stats.frame_count = self.stats.frame_count.saturating_add(1);

            if let Some(previous) = self.last_presented_at {
                let interval = now.saturating_duration_since(previous);
                self.stats.last_present_interval = interval;
                self.stats.average_present_interval = if self.stats.frame_count <= 2 {
                    interval
                } else {
                    let sample_count = self.stats.frame_count - 1;
                    let previous_total =
                        self.stats.average_present_interval.as_nanos() * (sample_count - 1) as u128;
                    let total = previous_total + interval.as_nanos();
                    Duration::from_nanos((total / sample_count as u128) as u64)
                };
                self.stats.current_fps = if interval.is_zero() {
                    0.0
                } else {
                    1.0 / interval.as_secs_f64()
                };
            }

            self.last_presented_at = Some(now);
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct RenderRequest {
        size_px: (u32, u32),
        scale_bits: u32,
        time_bits: u64,
        presentation_mode: PresentationMode,
    }

    impl RenderRequest {
        fn new(
            size_px: (u32, u32),
            scale_factor: f32,
            time_seconds: f64,
            presentation_mode: PresentationMode,
        ) -> Self {
            Self {
                size_px: (size_px.0.max(1), size_px.1.max(1)),
                scale_bits: sanitize_scale_factor(scale_factor).to_bits(),
                time_bits: time_seconds.to_bits(),
                presentation_mode,
            }
        }

        fn scale_factor(&self) -> f32 {
            f32::from_bits(self.scale_bits)
        }

        fn time_seconds(&self) -> f64 {
            f64::from_bits(self.time_bits)
        }

        fn is_dirty(&self, session: &InteractivePlotSession) -> bool {
            let dirty = session.dirty_domains();
            dirty.layout || dirty.data || dirty.overlay || dirty.temporal || dirty.interaction
        }
    }

    #[derive(Clone)]
    struct CachedFrame {
        request: RenderRequest,
        image: Arc<RenderImage>,
        stats: FrameStats,
        target: RenderTargetKind,
        surface_capability: SurfaceCapability,
    }

    struct RenderedFrame {
        image: Arc<RenderImage>,
        stats: FrameStats,
        target: RenderTargetKind,
        surface_capability: SurfaceCapability,
    }

    #[derive(Clone)]
    struct InteractionLayout {
        bounds: Bounds<Pixels>,
        content_bounds: Bounds<Pixels>,
        frame_size_px: (u32, u32),
    }

    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    enum DragMode {
        #[default]
        None,
        Pan,
        Brush,
    }

    /// GPUI view that renders a `ruviz` plot into a cached `RenderImage`.
    pub struct RuvizPlot {
        session: InteractivePlotSession,
        subscription: ReactiveSubscription,
        reactive_notify_pending: Arc<AtomicBool>,
        reactive_receiver: Option<UnboundedReceiver<()>>,
        reactive_watcher: Option<Task<()>>,
        presentation_clock: Arc<Mutex<PresentationClock>>,
        options: RuvizPlotOptions,
        cached_frame: Option<CachedFrame>,
        retired_images: Vec<Arc<RenderImage>>,
        in_flight_request: Option<RenderRequest>,
        queued_request: Option<RenderRequest>,
        in_flight_render: Option<Task<()>>,
        last_layout: Option<InteractionLayout>,
        pointer_down_px: Option<ViewportPoint>,
        last_pointer_px: Option<ViewportPoint>,
        drag_mode: DragMode,
    }

    impl RuvizPlot {
        pub fn new<P>(plot: P, cx: &mut Context<Self>) -> Self
        where
            P: IntoRuvizSession,
        {
            Self::with_options(plot, RuvizPlotOptions::default(), cx)
        }

        pub fn with_options<P>(plot: P, options: RuvizPlotOptions, _cx: &mut Context<Self>) -> Self
        where
            P: IntoRuvizSession,
        {
            let session = plot.into_session();
            apply_performance_options(&session, options.performance);
            let (reactive_notify_pending, reactive_receiver, subscription) =
                bind_reactive_session(&session);
            Self {
                session,
                subscription,
                reactive_notify_pending,
                reactive_receiver: Some(reactive_receiver),
                reactive_watcher: None,
                presentation_clock: Arc::new(Mutex::new(PresentationClock::default())),
                options,
                cached_frame: None,
                retired_images: Vec::new(),
                in_flight_request: None,
                queued_request: None,
                in_flight_render: None,
                last_layout: None,
                pointer_down_px: None,
                last_pointer_px: None,
                drag_mode: DragMode::None,
            }
        }

        pub fn interactive_session(&self) -> &InteractivePlotSession {
            &self.session
        }

        pub fn prepared_plot(&self) -> &PreparedPlot {
            self.session.prepared_plot()
        }

        pub fn presentation_mode(&self) -> PresentationMode {
            self.options.presentation_mode
        }

        pub fn sizing_policy(&self) -> &SizingPolicy {
            &self.options.sizing_policy
        }

        pub fn performance_options(&self) -> &PerformanceOptions {
            &self.options.performance
        }

        pub fn interaction_options(&self) -> &InteractionOptions {
            &self.options.interaction
        }

        pub fn frame_stats(&self) -> FrameStats {
            self.cached_frame
                .as_ref()
                .map(|frame| frame.stats.clone())
                .unwrap_or_else(|| self.session.stats())
        }

        pub fn presentation_stats(&self) -> PresentationStats {
            self.presentation_clock
                .lock()
                .expect("RuvizPlot presentation clock lock poisoned")
                .stats()
        }

        pub fn set_plot<P>(&mut self, plot: P, cx: &mut Context<Self>)
        where
            P: IntoRuvizSession,
        {
            self.session = plot.into_session();
            apply_performance_options(&self.session, self.options.performance);
            let (reactive_notify_pending, reactive_receiver, subscription) =
                bind_reactive_session(&self.session);
            self.subscription = subscription;
            self.reactive_notify_pending = reactive_notify_pending;
            self.reactive_receiver = Some(reactive_receiver);
            self.reactive_watcher = None;
            self.presentation_clock
                .lock()
                .expect("RuvizPlot presentation clock lock poisoned")
                .reset();
            self.reset_pointer_state();
            self.invalidate_frame_state();
            cx.notify();
        }

        pub fn set_time(&mut self, time_seconds: f64, cx: &mut Context<Self>) {
            self.options.interaction.time_seconds = time_seconds;
            self.session
                .apply_input(PlotInputEvent::SetTime { time_seconds });
            cx.notify();
        }

        pub fn set_presentation_mode(
            &mut self,
            presentation_mode: PresentationMode,
            cx: &mut Context<Self>,
        ) {
            self.options.presentation_mode = presentation_mode;
            self.invalidate_frame_state();
            cx.notify();
        }

        pub fn set_sizing_policy(&mut self, sizing_policy: SizingPolicy, cx: &mut Context<Self>) {
            self.options.sizing_policy = sizing_policy;
            self.invalidate_frame_state();
            cx.notify();
        }

        pub fn set_performance_options(
            &mut self,
            performance: PerformanceOptions,
            cx: &mut Context<Self>,
        ) {
            self.options.performance = performance;
            apply_performance_options(&self.session, performance);
            self.invalidate_frame_state();
            cx.notify();
        }

        pub fn set_interaction_options(
            &mut self,
            interaction: InteractionOptions,
            cx: &mut Context<Self>,
        ) {
            self.options.interaction = interaction;
            self.invalidate_frame_state();
            cx.notify();
        }

        fn invalidate_frame_state(&mut self) {
            self.retire_cached_frame();
            self.in_flight_request = None;
            self.queued_request = None;
            self.in_flight_render = None;
        }

        fn reset_pointer_state(&mut self) {
            self.pointer_down_px = None;
            self.last_pointer_px = None;
            self.drag_mode = DragMode::None;
        }

        fn retire_cached_frame(&mut self) {
            if let Some(frame) = self.cached_frame.take() {
                self.retired_images.push(frame.image);
            }
        }

        fn replace_cached_frame(&mut self, request: RenderRequest, frame: RenderedFrame) {
            self.retire_cached_frame();
            self.cached_frame = Some(CachedFrame {
                request,
                image: frame.image,
                stats: frame.stats,
                target: frame.target,
                surface_capability: frame.surface_capability,
            });
        }

        fn flush_retired_images(&mut self, mut window: Option<&mut Window>, cx: &mut App) {
            for image in self.retired_images.drain(..) {
                cx.drop_image(image, window.as_deref_mut());
            }
        }

        fn ensure_reactive_watcher(
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
                        let _ = entity_for_notify.update(cx, |_, cx| {
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

        fn prepaint(
            &mut self,
            entity: Entity<Self>,
            bounds: Bounds<Pixels>,
            window: &mut Window,
            cx: &mut App,
        ) -> Option<Arc<RenderImage>> {
            self.flush_retired_images(Some(window), cx);

            if let Some(request) = self.current_request(bounds, window) {
                self.update_layout(bounds, request.size_px);
                self.start_render_if_needed(entity, request, window, cx);
            } else {
                self.last_layout = None;
            }

            self.cached_frame
                .as_ref()
                .map(|frame| Arc::clone(&frame.image))
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
                bounds,
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

            if self.in_flight_request.is_some() {
                self.queued_request = Some(request);
                return;
            }

            self.start_render(entity, request, window, cx);
        }

        fn start_render(
            &mut self,
            entity: Entity<Self>,
            request: RenderRequest,
            window: &mut Window,
            cx: &mut App,
        ) {
            let session = self.session.clone();
            let request_for_task = request.clone();
            let render_job =
                cx.background_executor()
                    .spawn(async move { render_frame_from_session(session, request_for_task) });

            let entity_for_update = entity.clone();
            let request_for_update = request.clone();
            let task = window.spawn(cx, async move |cx| {
                let result = render_job.await;
                cx.on_next_frame(move |_, cx| {
                    let _ = entity_for_update.update(cx, |view, cx| {
                        view.finish_render(request_for_update, result, cx);
                        cx.notify();
                    });
                });
            });

            self.in_flight_request = Some(request);
            self.in_flight_render = Some(task);
        }

        fn finish_render(
            &mut self,
            request: RenderRequest,
            result: std::result::Result<RenderedFrame, String>,
            cx: &mut Context<Self>,
        ) {
            if self.in_flight_request.as_ref() != Some(&request) {
                return;
            }

            self.in_flight_request = None;
            self.in_flight_render = None;

            if let Ok(frame) = result {
                self.replace_cached_frame(request.clone(), frame);
            }

            if self.queued_request.take().is_some() {
                cx.notify();
            }
        }

        fn local_viewport_point(&self, window_position: gpui::Point<Pixels>) -> Option<ViewportPoint> {
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

        fn handle_mouse_down(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) {
            let Some(position_px) = self.local_viewport_point(event.position) else {
                return;
            };

            if event.click_count >= 2 {
                self.session.apply_input(PlotInputEvent::ResetView);
                self.reset_pointer_state();
                self.invalidate_frame_state();
                cx.notify();
                return;
            }

            self.pointer_down_px = Some(position_px);
            self.last_pointer_px = Some(position_px);

            if event.modifiers.shift && self.options.interaction.selection {
                self.drag_mode = DragMode::Brush;
                self.session
                    .apply_input(PlotInputEvent::BrushStart { position_px });
            } else if self.options.interaction.pan {
                self.drag_mode = DragMode::Pan;
            } else {
                self.drag_mode = DragMode::None;
            }

            cx.notify();
        }

        fn handle_mouse_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
            let position_px = match self.local_viewport_point(event.position) {
                Some(position_px) => position_px,
                None => {
                    if !event.dragging() && self.options.interaction.hover {
                        self.session.apply_input(PlotInputEvent::ClearHover);
                        self.session.apply_input(PlotInputEvent::HideTooltip);
                        cx.notify();
                    }
                    return;
                }
            };

            if event.dragging() {
                match self.drag_mode {
                    DragMode::Pan if self.options.interaction.pan => {
                        if let Some(previous) = self.last_pointer_px {
                            let delta_px = ViewportPoint::new(
                                position_px.x - previous.x,
                                position_px.y - previous.y,
                            );
                            if delta_px.x.abs() > f64::EPSILON || delta_px.y.abs() > f64::EPSILON {
                                self.session.apply_input(PlotInputEvent::Pan { delta_px });
                                cx.notify();
                            }
                        }
                    }
                    DragMode::Brush if self.options.interaction.selection => {
                        self.session
                            .apply_input(PlotInputEvent::BrushMove { position_px });
                        cx.notify();
                    }
                    DragMode::Pan | DragMode::Brush | DragMode::None => {}
                }
            } else if self.options.interaction.hover {
                self.session.apply_input(PlotInputEvent::Hover { position_px });
                if !self.options.interaction.tooltips {
                    self.session.apply_input(PlotInputEvent::HideTooltip);
                }
                cx.notify();
            }

            self.last_pointer_px = Some(position_px);
        }

        fn handle_mouse_up(&mut self, event: &MouseUpEvent, cx: &mut Context<Self>) {
            let position_px = self
                .local_viewport_point(event.position)
                .or(self.last_pointer_px)
                .or(self.pointer_down_px);

            match (self.drag_mode, position_px) {
                (DragMode::Brush, Some(position_px)) if self.options.interaction.selection => {
                    self.session
                        .apply_input(PlotInputEvent::BrushEnd { position_px });
                    cx.notify();
                }
                (_, Some(position_px)) if self.options.interaction.selection => {
                    if self
                        .pointer_down_px
                        .is_some_and(|origin| distance_px(origin, position_px) <= 3.0)
                    {
                        self.session
                            .apply_input(PlotInputEvent::SelectAt { position_px });
                        cx.notify();
                    }
                }
                _ => {}
            }

            self.reset_pointer_state();
        }

        fn handle_hover_change(&mut self, hovered: bool, cx: &mut Context<Self>) {
            if hovered {
                return;
            }

            self.session.apply_input(PlotInputEvent::ClearHover);
            self.session.apply_input(PlotInputEvent::HideTooltip);
            self.reset_pointer_state();
            cx.notify();
        }

        fn handle_scroll_wheel(&mut self, event: &ScrollWheelEvent, cx: &mut Context<Self>) {
            if !self.options.interaction.zoom {
                return;
            }

            let Some(center_px) = self.local_viewport_point(event.position) else {
                return;
            };

            let delta = event.delta.pixel_delta(px(16.0));
            let delta_y = f64::from(delta.y);
            if delta_y.abs() <= f64::EPSILON {
                return;
            }

            let factor = (1.0025f64).powf(-delta_y).clamp(0.25, 4.0);
            self.session
                .apply_input(PlotInputEvent::Zoom { factor, center_px });
            cx.notify();
        }
    }

    impl Render for RuvizPlot {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let entity = cx.entity();
            self.ensure_reactive_watcher(entity.clone(), window, cx);

            let plot_canvas = canvas::<Option<Arc<RenderImage>>>(
                {
                    let entity = entity.clone();
                    move |bounds, window, cx| {
                        let entity_for_prepaint = entity.clone();
                        entity.update(cx, move |view, cx| {
                            view.prepaint(entity_for_prepaint, bounds, window, cx)
                        })
                    }
                },
                {
                    let image_fit = self.options.interaction.image_fit;
                    let presentation_clock = Arc::clone(&self.presentation_clock);
                    move |bounds, image: Option<Arc<RenderImage>>, window: &mut Window, _cx| {
                        presentation_clock
                            .lock()
                            .expect("RuvizPlot presentation clock lock poisoned")
                            .record_now();
                        if let Some(image) = image {
                            let image_size = image.size(0);
                            let fitted_bounds = image_fit.into_gpui().get_bounds(bounds, image_size);
                            let _ = window.paint_image(
                                fitted_bounds,
                                Corners::default(),
                                image,
                                0,
                                false,
                            );
                        }
                    }
                },
            )
            .size_full();

            let mut root = div()
                .relative()
                .child(plot_canvas)
                .on_mouse_down(MouseButton::Left, {
                    let entity = entity.clone();
                    move |event, _, cx| {
                        let _ = entity.update(cx, |view, cx| {
                            view.handle_mouse_down(event, cx);
                        });
                    }
                })
                .on_mouse_move({
                    let entity = entity.clone();
                    move |event, _, cx| {
                        let _ = entity.update(cx, |view, cx| {
                            view.handle_mouse_move(event, cx);
                        });
                    }
                })
                .on_mouse_up(MouseButton::Left, {
                    let entity = entity.clone();
                    move |event, _, cx| {
                        let _ = entity.update(cx, |view, cx| {
                            view.handle_mouse_up(event, cx);
                        });
                    }
                })
                .on_mouse_up_out(MouseButton::Left, {
                    let entity = entity.clone();
                    move |event, _, cx| {
                        let _ = entity.update(cx, |view, cx| {
                            view.handle_mouse_up(event, cx);
                        });
                    }
                })
                .on_scroll_wheel({
                    let entity = entity.clone();
                    move |event, _, cx| {
                        let _ = entity.update(cx, |view, cx| {
                            view.handle_scroll_wheel(event, cx);
                        });
                    }
                });

            root.interactivity().on_hover({
                let entity = entity.clone();
                move |hovered, _, cx| {
                    let _ = entity.update(cx, |view, cx| {
                        view.handle_hover_change(*hovered, cx);
                    });
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

    fn bind_reactive_session(
        session: &InteractivePlotSession,
    ) -> (
        Arc<AtomicBool>,
        UnboundedReceiver<()>,
        ReactiveSubscription,
    ) {
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

    fn apply_performance_options(session: &InteractivePlotSession, options: PerformanceOptions) {
        session.set_frame_pacing(options.frame_pacing);
        session.set_quality_policy(options.quality_policy);
        session.set_prefer_gpu(options.prefer_gpu);
    }

    fn distance_px(a: ViewportPoint, b: ViewportPoint) -> f64 {
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        (dx * dx + dy * dy).sqrt()
    }

    fn sanitize_scale_factor(scale_factor: f32) -> f32 {
        if scale_factor.is_finite() && scale_factor > 0.0 {
            scale_factor
        } else {
            1.0
        }
    }

    #[allow(deprecated)]
    fn resolve_presentation_mode(requested: PresentationMode) -> PresentationMode {
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

        Ok(RenderedFrame {
            image: render_image_from_ruviz(frame.image),
            stats: frame.stats,
            target: frame.target,
            surface_capability: frame.surface_capability,
        })
    }

    fn render_image_from_ruviz(image: RuvizImage) -> Arc<RenderImage> {
        let mut pixels = image.pixels;
        rgba_to_bgra_in_place(&mut pixels);
        let buffer = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(image.width, image.height, pixels)
            .expect("rendered frame size must match RGBA pixel buffer");
        Arc::new(RenderImage::new(smallvec![Frame::new(buffer)]))
    }

    fn rgba_to_bgra_in_place(pixels: &mut [u8]) {
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ruviz::{data::Observable, prelude::Plot};

        #[test]
        fn test_rgba_to_bgra_conversion() {
            let mut pixels = vec![1, 2, 3, 255, 10, 20, 30, 128];
            rgba_to_bgra_in_place(&mut pixels);
            assert_eq!(pixels, vec![3, 2, 1, 255, 30, 20, 10, 128]);
        }

        #[test]
        fn test_hybrid_mode_falls_back_without_gpu_feature() {
            #[cfg(not(feature = "gpu"))]
            assert_eq!(
                resolve_presentation_mode(PresentationMode::Hybrid),
                PresentationMode::Image
            );
        }

        #[test]
        #[allow(deprecated)]
        fn test_surface_mode_alias_resolves() {
            #[cfg(feature = "gpu")]
            assert_eq!(
                resolve_presentation_mode(PresentationMode::SurfaceExperimental),
                PresentationMode::Hybrid
            );

            #[cfg(not(feature = "gpu"))]
            assert_eq!(
                resolve_presentation_mode(PresentationMode::SurfaceExperimental),
                PresentationMode::Image
            );
        }

        #[test]
        fn test_render_request_is_dirty_before_first_frame_and_clean_after_render() {
            let plot: Plot = Plot::new().line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0]).into();
            let session = plot.prepare_interactive();
            let request = RenderRequest::new((320, 240), 1.0, 0.0, PresentationMode::Image);

            assert!(request.is_dirty(&session));
            session
                .render_to_image(ImageTarget {
                    size_px: request.size_px,
                    scale_factor: request.scale_factor(),
                    time_seconds: request.time_seconds(),
                })
                .expect("interactive session should render");
            assert!(!request.is_dirty(&session));
        }

        #[test]
        fn test_render_request_becomes_dirty_after_observable_update() {
            let y = Observable::new(vec![0.0, 1.0, 4.0]);
            let plot: Plot = Plot::new()
                .line_source(vec![0.0, 1.0, 2.0], y.clone())
                .into();
            let session = plot.prepare_interactive();
            let request = RenderRequest::new((320, 240), 1.0, 0.0, PresentationMode::Image);

            session
                .render_to_image(ImageTarget {
                    size_px: request.size_px,
                    scale_factor: request.scale_factor(),
                    time_seconds: request.time_seconds(),
                })
                .expect("interactive session should render");
            assert!(!request.is_dirty(&session));

            y.set(vec![0.0, 1.0, 9.0]);
            assert!(request.is_dirty(&session));
        }

        #[test]
        fn test_bind_reactive_session_coalesces_notifications() {
            let y = Observable::new(vec![0.0, 1.0, 4.0]);
            let plot: Plot = Plot::new()
                .line_source(vec![0.0, 1.0, 2.0], y.clone())
                .into();
            let session = plot.prepare_interactive();
            let (pending, mut receiver, _subscription) = bind_reactive_session(&session);

            y.set(vec![1.0, 2.0, 3.0]);
            y.set(vec![2.0, 3.0, 4.0]);

            assert!(pending.load(Ordering::Acquire));
            assert!(receiver.try_recv().is_ok());
        }

        #[test]
        fn test_presentation_clock_tracks_paint_cadence() {
            let start = Instant::now();
            let mut clock = PresentationClock::default();

            clock.record_at(start);
            clock.record_at(start + Duration::from_millis(16));
            clock.record_at(start + Duration::from_millis(32));

            let stats = clock.stats();
            assert_eq!(stats.frame_count, 3);
            assert!(stats.last_present_interval >= Duration::from_millis(16));
            assert!(stats.average_present_interval >= Duration::from_millis(16));
            assert!(stats.current_fps > 50.0 && stats.current_fps < 70.0);
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub use gpui;

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub use supported::*;
