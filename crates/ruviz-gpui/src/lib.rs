#[cfg(not(any(target_os = "macos", target_os = "linux")))]
compile_error!("ruviz-gpui currently supports macOS and Linux only.");

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod supported {
    use arboard::{Clipboard, ImageData};
    #[cfg(all(feature = "gpu", target_os = "macos"))]
    use core_foundation::{
        base::{CFType, TCFType},
        boolean::CFBoolean,
        dictionary::CFDictionary,
        string::CFString,
    };
    #[cfg(all(feature = "gpu", target_os = "macos"))]
    use core_video::pixel_buffer::{
        CVPixelBuffer, CVPixelBufferKeys, kCVPixelFormatType_420YpCbCr8BiPlanarFullRange,
    };
    use futures::{
        StreamExt,
        channel::mpsc::{UnboundedReceiver, unbounded},
        executor::block_on,
    };
    use gpui::{
        AnyElement, App, Bounds, Context, Corners, Entity, FocusHandle, InteractiveElement,
        IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
        ObjectFit, Pixels, Point, Render, RenderImage, ScrollDelta, ScrollWheelEvent, Task, Window,
        canvas, div, point, prelude::*, px, rgb, rgba, size,
    };
    use image::{Frame, ImageBuffer, Rgba};
    use ruviz::{
        core::plot::Image as RuvizImage,
        core::{
            FramePacing, FrameStats, ImageTarget, InteractivePlotSession, Plot, PlotInputEvent,
            PlottingError, PreparedPlot, QualityPolicy, ReactiveSubscription, RenderTargetKind,
            Result, SurfaceCapability, SurfaceTarget, ViewportPoint, ViewportRect,
        },
        export::write_rgba_png_atomic,
    };
    use smallvec::smallvec;
    use std::{
        borrow::Cow,
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
        time::{Duration, Instant},
    };

    pub use gpui;
    pub use ruviz;

    const DRAG_THRESHOLD_PX: f64 = 3.0;
    const LINE_SCROLL_DELTA_PX: f32 = 50.0;
    const MENU_MIN_WIDTH_PX: f32 = 220.0;
    const MENU_ITEM_HEIGHT_PX: f32 = 30.0;
    const MENU_SEPARATOR_HEIGHT_PX: f32 = 10.0;
    const MENU_PADDING_X_PX: f32 = 14.0;
    const MENU_PADDING_Y_PX: f32 = 8.0;
    const MENU_EDGE_MARGIN_PX: f32 = 8.0;

    type ContextMenuActionHandler =
        Arc<dyn Fn(GpuiContextMenuActionContext) -> Result<()> + Send + Sync>;

    pub trait IntoPlotSession {
        fn into_plot_session(self) -> InteractivePlotSession;
    }

    impl IntoPlotSession for InteractivePlotSession {
        fn into_plot_session(self) -> InteractivePlotSession {
            self
        }
    }

    impl IntoPlotSession for PreparedPlot {
        fn into_plot_session(self) -> InteractivePlotSession {
            self.into_interactive()
        }
    }

    impl IntoPlotSession for Plot {
        fn into_plot_session(self) -> InteractivePlotSession {
            self.prepare_interactive()
        }
    }

    #[deprecated(note = "Use IntoPlotSession instead.")]
    pub trait IntoRuvizSession {
        fn into_session(self) -> InteractivePlotSession;
    }

    #[allow(deprecated)]
    impl<T> IntoRuvizSession for T
    where
        T: IntoPlotSession,
    {
        fn into_session(self) -> InteractivePlotSession {
            self.into_plot_session()
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

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct GpuiContextMenuItem {
        pub id: String,
        pub label: String,
        pub enabled: bool,
    }

    impl GpuiContextMenuItem {
        pub fn new<I, L>(id: I, label: L) -> Self
        where
            I: Into<String>,
            L: Into<String>,
        {
            Self {
                id: id.into(),
                label: label.into(),
                enabled: true,
            }
        }

        pub fn enabled(mut self, enabled: bool) -> Self {
            self.enabled = enabled;
            self
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct GpuiContextMenuConfig {
        pub enabled: bool,
        pub show_reset_view: bool,
        pub show_set_home_view: bool,
        pub show_go_to_home_view: bool,
        pub show_save_png: bool,
        /// Enables the built-in `Cmd/Ctrl+C` shortcut to copy the current plot image.
        pub show_copy_image: bool,
        pub show_copy_cursor_coordinates: bool,
        pub show_copy_visible_bounds: bool,
        pub custom_items: Vec<GpuiContextMenuItem>,
    }

    impl Default for GpuiContextMenuConfig {
        fn default() -> Self {
            Self {
                enabled: true,
                show_reset_view: true,
                show_set_home_view: true,
                show_go_to_home_view: true,
                show_save_png: true,
                show_copy_image: true,
                show_copy_cursor_coordinates: true,
                show_copy_visible_bounds: true,
                custom_items: Vec::new(),
            }
        }
    }

    #[derive(Clone, Debug)]
    pub struct GpuiContextMenuActionContext {
        pub action_id: String,
        pub visible_bounds: ViewportRect,
        pub plot_area_px: ViewportRect,
        pub frame_size_px: (u32, u32),
        pub scale_factor: f32,
        pub cursor_position_px: ViewportPoint,
        pub cursor_data_position: Option<ViewportPoint>,
        pub image: RuvizImage,
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

    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub enum PerformancePreset {
        Interactive,
        #[default]
        Balanced,
        Publication,
    }

    impl PerformancePreset {
        fn into_options(self) -> PerformanceOptions {
            match self {
                Self::Interactive => PerformanceOptions {
                    frame_pacing: FramePacing::Display,
                    quality_policy: QualityPolicy::Interactive,
                    prefer_gpu: cfg!(feature = "gpu"),
                },
                Self::Balanced => PerformanceOptions::default(),
                Self::Publication => PerformanceOptions {
                    frame_pacing: FramePacing::Display,
                    quality_policy: QualityPolicy::Publication,
                    prefer_gpu: false,
                },
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
        pub context_menu: GpuiContextMenuConfig,
    }

    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub enum ActiveBackend {
        #[default]
        Idle,
        Image,
        HybridFallback,
        HybridFastPath,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct PlotStats {
        pub render: FrameStats,
        pub presentation: PresentationStats,
        pub dropped_frames: u64,
        pub active_backend: ActiveBackend,
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
        average_present_interval_secs: f64,
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
                let interval_secs = interval.as_secs_f64();
                self.stats.last_present_interval = interval;
                self.average_present_interval_secs = if self.stats.frame_count <= 2 {
                    interval_secs
                } else {
                    let sample_count = self.stats.frame_count - 1;
                    let previous_sample_count = (sample_count - 1) as f64;
                    (self.average_present_interval_secs * previous_sample_count + interval_secs)
                        / sample_count as f64
                };
                self.stats.average_present_interval =
                    Duration::from_secs_f64(self.average_present_interval_secs);
                self.stats.current_fps = if interval.is_zero() {
                    0.0
                } else {
                    1.0 / interval_secs
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
    enum PrimaryFrame {
        Image(Arc<RenderImage>),
        #[cfg(all(feature = "gpu", target_os = "macos"))]
        Surface(CVPixelBuffer),
    }

    #[derive(Clone)]
    enum RenderedPrimary {
        Image(Arc<RenderImage>),
        #[cfg(all(feature = "gpu", target_os = "macos"))]
        Surface(Arc<RuvizImage>),
    }

    #[derive(Clone)]
    struct CachedFrame {
        request: RenderRequest,
        primary: PrimaryFrame,
        overlay_image: Option<Arc<RenderImage>>,
        stats: FrameStats,
        target: RenderTargetKind,
    }

    struct RenderedFrame {
        primary: Option<RenderedPrimary>,
        overlay_image: Option<Arc<RenderImage>>,
        stats: FrameStats,
        target: RenderTargetKind,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct ScheduledRender {
        generation: u64,
        request: RenderRequest,
    }

    #[derive(Debug, Default)]
    struct RenderScheduler {
        latest_requested_generation: u64,
        in_flight: Option<ScheduledRender>,
        queued: Option<ScheduledRender>,
        dropped_frames: u64,
    }

    impl RenderScheduler {
        fn schedule(&mut self, request: RenderRequest) -> Option<ScheduledRender> {
            self.latest_requested_generation = self.latest_requested_generation.saturating_add(1);
            let scheduled = ScheduledRender {
                generation: self.latest_requested_generation,
                request,
            };

            if self.in_flight.is_some() {
                if self.queued.replace(scheduled).is_some() {
                    self.dropped_frames = self.dropped_frames.saturating_add(1);
                }
                None
            } else {
                Some(scheduled)
            }
        }

        fn start(&mut self, scheduled: ScheduledRender) {
            self.in_flight = Some(scheduled);
        }

        fn finish(&mut self, scheduled: &ScheduledRender) -> bool {
            if self.in_flight.as_ref() != Some(scheduled) {
                return false;
            }
            self.in_flight = None;
            true
        }

        fn take_queued(&mut self) -> Option<ScheduledRender> {
            self.queued.take()
        }

        fn reset(&mut self) {
            *self = Self::default();
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum BuiltinContextMenuAction {
        ResetView,
        SetCurrentViewAsHome,
        GoToHomeView,
        SavePng,
        CopyImage,
        CopyCursorCoordinates,
        CopyVisibleBounds,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum ContextMenuEntryKind {
        Builtin(BuiltinContextMenuAction),
        Custom { id: String },
        Separator,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct ContextMenuEntry {
        kind: ContextMenuEntryKind,
        label: String,
        enabled: bool,
        bounds: Option<Bounds<Pixels>>,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct ContextMenuState {
        panel_bounds: Bounds<Pixels>,
        entries: Vec<ContextMenuEntry>,
        hovered_index: Option<usize>,
        trigger_position: Point<Pixels>,
        trigger_position_px: ViewportPoint,
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    enum ActiveDrag {
        #[default]
        None,
        LeftPan {
            anchor_px: ViewportPoint,
            last_px: ViewportPoint,
            crossed_threshold: bool,
        },
        RightZoom {
            anchor_px: ViewportPoint,
            current_px: ViewportPoint,
            crossed_threshold: bool,
            zoom_enabled: bool,
        },
        Brush {
            anchor_px: ViewportPoint,
            current_px: ViewportPoint,
            crossed_threshold: bool,
        },
    }

    pub struct RuvizPlotBuilder<P> {
        plot: P,
        options: RuvizPlotOptions,
        context_menu_action_handler: Option<ContextMenuActionHandler>,
    }

    impl<P> RuvizPlotBuilder<P>
    where
        P: IntoPlotSession + 'static,
    {
        fn new(plot: P) -> Self {
            Self {
                plot,
                options: RuvizPlotOptions::default(),
                context_menu_action_handler: None,
            }
        }

        pub fn interactive(mut self) -> Self {
            self.options.interaction = InteractionOptions::default();
            self
        }

        pub fn static_view(mut self) -> Self {
            self.options.interaction = InteractionOptions {
                time_seconds: self.options.interaction.time_seconds,
                image_fit: self.options.interaction.image_fit,
                pan: false,
                zoom: false,
                hover: false,
                selection: false,
                tooltips: false,
            };
            self
        }

        pub fn performance_preset(mut self, preset: PerformancePreset) -> Self {
            self.options.performance = preset.into_options();
            self
        }

        pub fn presentation(mut self, presentation_mode: PresentationMode) -> Self {
            self.options.presentation_mode = presentation_mode;
            self
        }

        pub fn fill(mut self) -> Self {
            self.options.sizing_policy = SizingPolicy::Fill;
            self
        }

        pub fn fixed_pixels(mut self, width: u32, height: u32) -> Self {
            self.options.sizing_policy = SizingPolicy::FixedPixels { width, height };
            self
        }

        pub fn interaction_options(mut self, interaction: InteractionOptions) -> Self {
            self.options.interaction = interaction;
            self
        }

        pub fn performance_options(mut self, performance: PerformanceOptions) -> Self {
            self.options.performance = performance;
            self
        }

        pub fn context_menu(mut self, context_menu: GpuiContextMenuConfig) -> Self {
            self.options.context_menu = context_menu;
            self
        }

        pub fn on_context_menu_action<F>(mut self, handler: F) -> Self
        where
            F: Fn(GpuiContextMenuActionContext) -> Result<()> + Send + Sync + 'static,
        {
            self.context_menu_action_handler = Some(Arc::new(handler));
            self
        }

        pub fn build<V>(self, cx: &mut Context<V>) -> Entity<RuvizPlot>
        where
            V: 'static,
        {
            if self.options.context_menu.enabled
                && !self.options.context_menu.custom_items.is_empty()
                && self.context_menu_action_handler.is_none()
            {
                panic!(
                    "GPUI context menu custom items require on_context_menu_action(...) before build()"
                );
            }
            let options = self.options;
            let plot = self.plot;
            let context_menu_action_handler = self.context_menu_action_handler;
            cx.new(move |cx| {
                RuvizPlot::from_options_impl(plot, options, context_menu_action_handler, cx)
            })
        }
    }

    pub fn plot<P, V>(plot: P, cx: &mut Context<V>) -> Entity<RuvizPlot>
    where
        P: IntoPlotSession + 'static,
        V: 'static,
    {
        plot_builder(plot).build(cx)
    }

    pub fn plot_builder<P>(plot: P) -> RuvizPlotBuilder<P>
    where
        P: IntoPlotSession + 'static,
    {
        RuvizPlotBuilder::new(plot)
    }

    #[derive(Clone)]
    struct InteractionLayout {
        component_bounds: Bounds<Pixels>,
        content_bounds: Bounds<Pixels>,
        frame_size_px: (u32, u32),
    }

    #[derive(Clone)]
    struct PaintFrame {
        primary: PrimaryFrame,
        overlay_image: Option<Arc<RenderImage>>,
    }

    #[cfg(all(feature = "gpu", target_os = "macos"))]
    struct SurfaceUploadState {
        pixel_buffer_options: CFDictionary<CFString, CFType>,
    }

    #[cfg(all(feature = "gpu", target_os = "macos"))]
    impl Default for SurfaceUploadState {
        fn default() -> Self {
            Self {
                pixel_buffer_options: make_surface_pixel_buffer_options(),
            }
        }
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
        #[cfg(all(feature = "gpu", target_os = "macos"))]
        surface_upload: SurfaceUploadState,
        scheduler: RenderScheduler,
        in_flight_render: Option<Task<()>>,
        last_layout: Option<InteractionLayout>,
        focus_handle: FocusHandle,
        last_pointer_px: Option<ViewportPoint>,
        active_drag: ActiveDrag,
        context_menu_action_handler: Option<ContextMenuActionHandler>,
        context_menu: Option<ContextMenuState>,
        home_view_bounds: Option<ViewportRect>,
    }

    impl RuvizPlot {
        fn from_options_impl<P>(
            plot: P,
            options: RuvizPlotOptions,
            context_menu_action_handler: Option<ContextMenuActionHandler>,
            cx: &mut Context<Self>,
        ) -> Self
        where
            P: IntoPlotSession,
        {
            let session = plot.into_plot_session();
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
                #[cfg(all(feature = "gpu", target_os = "macos"))]
                surface_upload: SurfaceUploadState::default(),
                scheduler: RenderScheduler::default(),
                in_flight_render: None,
                last_layout: None,
                focus_handle: cx.focus_handle(),
                last_pointer_px: None,
                active_drag: ActiveDrag::None,
                context_menu_action_handler,
                context_menu: None,
                home_view_bounds: None,
            }
        }

        #[deprecated(note = "Use ruviz_gpui::plot(...) or ruviz_gpui::plot_builder(...).")]
        pub fn new<P>(plot: P, cx: &mut Context<Self>) -> Self
        where
            P: IntoPlotSession,
        {
            Self::from_options_impl(plot, RuvizPlotOptions::default(), None, cx)
        }

        #[deprecated(note = "Use ruviz_gpui::plot_builder(...).build(cx).")]
        pub fn with_options<P>(plot: P, options: RuvizPlotOptions, _cx: &mut Context<Self>) -> Self
        where
            P: IntoPlotSession,
        {
            Self::from_options_impl(plot, options, None, _cx)
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

        pub fn context_menu_config(&self) -> &GpuiContextMenuConfig {
            &self.options.context_menu
        }

        pub fn stats(&self) -> PlotStats {
            PlotStats {
                render: self.frame_stats(),
                presentation: self.presentation_stats(),
                dropped_frames: self.scheduler.dropped_frames,
                active_backend: self
                    .cached_frame
                    .as_ref()
                    .map(active_backend_for_frame)
                    .unwrap_or_default(),
            }
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
            P: IntoPlotSession,
        {
            self.replace_session(plot.into_plot_session());
            cx.notify();
        }

        pub fn set_time(&mut self, time_seconds: f64, cx: &mut Context<Self>) {
            self.options.interaction.time_seconds = time_seconds;
            self.session
                .apply_input(PlotInputEvent::SetTime { time_seconds });
            cx.notify();
        }

        pub fn reset_view(&mut self, cx: &mut Context<Self>) {
            self.session.apply_input(PlotInputEvent::ResetView);
            self.reset_pointer_state();
            self.invalidate_frame_state();
            cx.notify();
        }

        pub fn set_current_view_as_home(&mut self, cx: &mut Context<Self>) -> Result<()> {
            self.home_view_bounds = Some(self.session.viewport_snapshot()?.visible_bounds);
            cx.notify();
            Ok(())
        }

        pub fn go_to_home_view(&mut self, cx: &mut Context<Self>) -> Result<()> {
            let Some(home_view_bounds) = self.home_view_bounds else {
                return Ok(());
            };
            if self.session.restore_visible_bounds(home_view_bounds) {
                self.reset_pointer_state();
                self.invalidate_frame_state();
                cx.notify();
            }
            Ok(())
        }

        pub fn save_png(&mut self, window: &Window, _cx: &mut Context<Self>) -> Result<()> {
            let image = self.capture_visible_view_image(window)?;
            self.spawn_save_png_dialog(image)
        }

        pub fn copy_image(&mut self, window: &Window, _cx: &mut Context<Self>) -> Result<()> {
            let image = self.capture_visible_view_image(window)?;
            self.copy_image_to_clipboard(&image)
        }

        pub fn copy_cursor_coordinates(&self) -> Result<()> {
            let cursor_position_px = self.current_cursor_position_px().ok_or_else(|| {
                PlottingError::InvalidInput(
                    "cursor coordinates are unavailable until the pointer enters the plot area"
                        .to_string(),
                )
            })?;
            let snapshot = self.session.viewport_snapshot()?;
            let cursor_data_position = cursor_data_position(
                snapshot.visible_bounds,
                snapshot.plot_area,
                cursor_position_px,
            )
            .ok_or_else(|| {
                PlottingError::InvalidInput("cursor is outside the plotted data area".to_string())
            })?;
            self.copy_text_to_clipboard(&format!(
                "x={:.6}, y={:.6}",
                cursor_data_position.x, cursor_data_position.y
            ))
        }

        pub fn copy_visible_bounds(&self) -> Result<()> {
            let snapshot = self.session.viewport_snapshot()?;
            self.copy_text_to_clipboard(&format!(
                "x=[{:.6}, {:.6}], y=[{:.6}, {:.6}]",
                snapshot.visible_bounds.min.x,
                snapshot.visible_bounds.max.x,
                snapshot.visible_bounds.min.y,
                snapshot.visible_bounds.max.y
            ))
        }

        #[deprecated(note = "Use ruviz_gpui::plot_builder(...).presentation(...).build(cx).")]
        pub fn set_presentation_mode(
            &mut self,
            presentation_mode: PresentationMode,
            cx: &mut Context<Self>,
        ) {
            self.options.presentation_mode = presentation_mode;
            self.invalidate_frame_state();
            cx.notify();
        }

        #[deprecated(
            note = "Use ruviz_gpui::plot_builder(...).fill()/fixed_pixels(...).build(cx)."
        )]
        pub fn set_sizing_policy(&mut self, sizing_policy: SizingPolicy, cx: &mut Context<Self>) {
            self.options.sizing_policy = sizing_policy;
            self.invalidate_frame_state();
            cx.notify();
        }

        #[deprecated(
            note = "Use ruviz_gpui::plot_builder(...).performance_options(...).build(cx)."
        )]
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

        #[deprecated(
            note = "Use ruviz_gpui::plot_builder(...).interaction_options(...).build(cx)."
        )]
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
            self.session.invalidate();
            self.scheduler.reset();
            self.in_flight_render = None;
        }

        fn replace_session(&mut self, session: InteractivePlotSession) {
            self.session = session;
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
            self.retire_cached_frame();
            self.scheduler.reset();
            self.in_flight_render = None;
            self.last_layout = None;
            self.context_menu = None;
            self.home_view_bounds = None;
        }

        fn reset_pointer_state(&mut self) {
            self.last_pointer_px = None;
            self.active_drag = ActiveDrag::None;
        }

        fn retire_cached_frame(&mut self) {
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
                maybe_retire_replaced_primary(
                    &mut self.retired_images,
                    &previous.primary,
                    &primary,
                );
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

        fn current_request(
            &self,
            bounds: Bounds<Pixels>,
            window: &Window,
        ) -> Option<RenderRequest> {
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

        fn local_viewport_point(&self, window_position: Point<Pixels>) -> Option<ViewportPoint> {
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

        fn clamped_viewport_point(&self, window_position: Point<Pixels>) -> Option<ViewportPoint> {
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

        fn viewport_point_to_window_position(
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
                x: layout.content_bounds.origin.x + px((normalized_x * content_width) as f32),
                y: layout.content_bounds.origin.y + px((normalized_y * content_height) as f32),
            })
        }

        fn current_cursor_position_px(&self) -> Option<ViewportPoint> {
            self.last_pointer_px
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

        fn capture_visible_view_image(&self, window: &Window) -> Result<RuvizImage> {
            let target = self.current_capture_target(window).ok_or_else(|| {
                PlottingError::InvalidInput(
                    "plot image capture is unavailable before the GPUI view has been laid out"
                        .to_string(),
                )
            })?;
            let frame = self.session.render_to_image(target)?;
            Ok(frame.image.as_ref().clone())
        }

        fn build_action_context(
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

        fn copy_text_to_clipboard(&self, text: &str) -> Result<()> {
            let mut clipboard = Clipboard::new().map_err(|err| {
                PlottingError::SystemError(format!("clipboard unavailable: {err}"))
            })?;
            clipboard
                .set_text(text.to_string())
                .map_err(|err| PlottingError::SystemError(format!("failed to copy text: {err}")))
        }

        fn copy_image_to_clipboard(&self, image: &RuvizImage) -> Result<()> {
            let mut clipboard = Clipboard::new().map_err(|err| {
                PlottingError::SystemError(format!("clipboard unavailable: {err}"))
            })?;
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

        fn spawn_save_png_dialog(&self, image: RuvizImage) -> Result<()> {
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
                    let _ = write_rgba_png_atomic(file_handle.path(), &image);
                })
                .map(|_| ())
                .map_err(|err| {
                    PlottingError::SystemError(format!(
                        "failed to spawn GPUI PNG export worker: {err}"
                    ))
                })
        }

        fn close_context_menu(&mut self, cx: &mut Context<Self>) {
            if self.context_menu.take().is_some() {
                cx.notify();
            }
        }

        fn update_context_menu_hover(
            &mut self,
            hovered_index: Option<usize>,
            cx: &mut Context<Self>,
        ) {
            if let Some(menu) = self.context_menu.as_mut() {
                if menu.hovered_index != hovered_index {
                    menu.hovered_index = hovered_index;
                    cx.notify();
                }
            }
        }

        fn context_menu_entry_index_at(&self, position: Point<Pixels>) -> Option<usize> {
            let menu = self.context_menu.as_ref()?;
            menu.entries.iter().position(|entry| {
                !matches!(entry.kind, ContextMenuEntryKind::Separator)
                    && entry
                        .bounds
                        .is_some_and(|bounds| bounds.contains(&position))
            })
        }

        fn build_context_menu_entries(
            &self,
            cursor_position_px: ViewportPoint,
        ) -> Result<Vec<ContextMenuEntry>> {
            let snapshot = self.session.viewport_snapshot()?;
            let cursor_available = cursor_data_position(
                snapshot.visible_bounds,
                snapshot.plot_area,
                cursor_position_px,
            )
            .is_some();
            let mut entries = Vec::new();

            let push_entry = |entries: &mut Vec<ContextMenuEntry>,
                              kind: ContextMenuEntryKind,
                              label: &str,
                              enabled: bool| {
                entries.push(ContextMenuEntry {
                    kind,
                    label: label.to_string(),
                    enabled,
                    bounds: None,
                });
            };
            let push_separator = |entries: &mut Vec<ContextMenuEntry>| {
                if !entries.is_empty()
                    && !matches!(
                        entries.last().map(|entry| &entry.kind),
                        Some(ContextMenuEntryKind::Separator)
                    )
                {
                    entries.push(ContextMenuEntry {
                        kind: ContextMenuEntryKind::Separator,
                        label: String::new(),
                        enabled: false,
                        bounds: None,
                    });
                }
            };

            if self.options.context_menu.show_reset_view {
                push_entry(
                    &mut entries,
                    ContextMenuEntryKind::Builtin(BuiltinContextMenuAction::ResetView),
                    "Reset View",
                    true,
                );
            }
            if self.options.context_menu.show_set_home_view {
                push_entry(
                    &mut entries,
                    ContextMenuEntryKind::Builtin(BuiltinContextMenuAction::SetCurrentViewAsHome),
                    "Set Current View As Home",
                    true,
                );
            }
            if self.options.context_menu.show_go_to_home_view {
                push_entry(
                    &mut entries,
                    ContextMenuEntryKind::Builtin(BuiltinContextMenuAction::GoToHomeView),
                    "Go To Home View",
                    self.home_view_bounds.is_some(),
                );
            }

            let export_group_enabled = self.options.context_menu.show_save_png
                || self.options.context_menu.show_copy_image
                || self.options.context_menu.show_copy_cursor_coordinates
                || self.options.context_menu.show_copy_visible_bounds;
            if !entries.is_empty() && export_group_enabled {
                push_separator(&mut entries);
            }

            if self.options.context_menu.show_save_png {
                push_entry(
                    &mut entries,
                    ContextMenuEntryKind::Builtin(BuiltinContextMenuAction::SavePng),
                    "Save PNG...",
                    true,
                );
            }
            if self.options.context_menu.show_copy_image {
                push_entry(
                    &mut entries,
                    ContextMenuEntryKind::Builtin(BuiltinContextMenuAction::CopyImage),
                    "Copy Image",
                    true,
                );
            }
            if self.options.context_menu.show_copy_cursor_coordinates {
                push_entry(
                    &mut entries,
                    ContextMenuEntryKind::Builtin(BuiltinContextMenuAction::CopyCursorCoordinates),
                    "Copy Cursor Coordinates",
                    cursor_available,
                );
            }
            if self.options.context_menu.show_copy_visible_bounds {
                push_entry(
                    &mut entries,
                    ContextMenuEntryKind::Builtin(BuiltinContextMenuAction::CopyVisibleBounds),
                    "Copy Visible Bounds",
                    true,
                );
            }

            if !self.options.context_menu.custom_items.is_empty() {
                push_separator(&mut entries);
                for item in &self.options.context_menu.custom_items {
                    entries.push(ContextMenuEntry {
                        kind: ContextMenuEntryKind::Custom {
                            id: item.id.clone(),
                        },
                        label: item.label.clone(),
                        enabled: item.enabled,
                        bounds: None,
                    });
                }
            }

            while matches!(
                entries.last().map(|entry| &entry.kind),
                Some(ContextMenuEntryKind::Separator)
            ) {
                entries.pop();
            }

            Ok(entries)
        }

        fn build_context_menu(
            &self,
            position: Point<Pixels>,
            cursor_position_px: ViewportPoint,
        ) -> Result<Option<ContextMenuState>> {
            let layout = match self.last_layout.as_ref() {
                Some(layout) => layout,
                None => return Ok(None),
            };
            let mut entries = self.build_context_menu_entries(cursor_position_px)?;
            if entries.is_empty() {
                return Ok(None);
            }

            let max_chars = entries
                .iter()
                .filter(|entry| !matches!(entry.kind, ContextMenuEntryKind::Separator))
                .map(|entry| entry.label.chars().count())
                .max()
                .unwrap_or(0);
            let panel_width =
                MENU_MIN_WIDTH_PX.max(max_chars as f32 * 7.2 + MENU_PADDING_X_PX * 2.0);
            let content_height = entries.iter().fold(0.0, |height, entry| {
                height
                    + if matches!(entry.kind, ContextMenuEntryKind::Separator) {
                        MENU_SEPARATOR_HEIGHT_PX
                    } else {
                        MENU_ITEM_HEIGHT_PX
                    }
            });
            let panel_height = content_height + MENU_PADDING_Y_PX * 2.0;

            let min_left = layout.component_bounds.origin.x + px(MENU_EDGE_MARGIN_PX);
            let min_top = layout.component_bounds.origin.y + px(MENU_EDGE_MARGIN_PX);
            let max_left = (layout.component_bounds.origin.x + layout.component_bounds.size.width)
                - px(panel_width + MENU_EDGE_MARGIN_PX);
            let max_top = (layout.component_bounds.origin.y + layout.component_bounds.size.height)
                - px(panel_height + MENU_EDGE_MARGIN_PX);
            let left = position.x.max(min_left).min(max_left.max(min_left));
            let top = position.y.max(min_top).min(max_top.max(min_top));
            let panel_bounds =
                Bounds::new(point(left, top), size(px(panel_width), px(panel_height)));

            let mut cursor_y = top + px(MENU_PADDING_Y_PX);
            for entry in &mut entries {
                let entry_height = if matches!(entry.kind, ContextMenuEntryKind::Separator) {
                    MENU_SEPARATOR_HEIGHT_PX
                } else {
                    MENU_ITEM_HEIGHT_PX
                };
                entry.bounds = Some(Bounds::new(
                    point(left, cursor_y),
                    size(px(panel_width), px(entry_height)),
                ));
                cursor_y += px(entry_height);
            }

            let hovered_index = entries.iter().position(|entry| {
                !matches!(entry.kind, ContextMenuEntryKind::Separator)
                    && entry
                        .bounds
                        .is_some_and(|bounds| bounds.contains(&position))
            });
            Ok(Some(ContextMenuState {
                panel_bounds,
                entries,
                hovered_index,
                trigger_position: position,
                trigger_position_px: cursor_position_px,
            }))
        }

        fn open_context_menu(
            &mut self,
            position: Point<Pixels>,
            cursor_position_px: ViewportPoint,
            cx: &mut Context<Self>,
        ) -> Result<()> {
            if !self.options.context_menu.enabled {
                return Ok(());
            }

            self.session.apply_input(PlotInputEvent::ClearHover);
            self.session.apply_input(PlotInputEvent::HideTooltip);
            self.context_menu = self.build_context_menu(position, cursor_position_px)?;
            cx.notify();
            Ok(())
        }

        fn handle_context_menu_left_click(
            &mut self,
            position: Point<Pixels>,
            window: &Window,
            cx: &mut Context<Self>,
        ) -> Result<()> {
            let Some(index) = self.context_menu_entry_index_at(position) else {
                self.close_context_menu(cx);
                return Ok(());
            };

            let Some(menu) = self.context_menu.as_ref() else {
                return Ok(());
            };
            if !menu.entries.get(index).is_some_and(|entry| entry.enabled) {
                return Ok(());
            }

            self.activate_context_menu_entry(index, window, cx)
        }

        fn activate_context_menu_entry(
            &mut self,
            index: usize,
            window: &Window,
            cx: &mut Context<Self>,
        ) -> Result<()> {
            let Some(menu) = self.context_menu.as_ref() else {
                return Ok(());
            };
            let Some(entry) = menu.entries.get(index).cloned() else {
                return Ok(());
            };
            let trigger_position_px = menu.trigger_position_px;
            self.close_context_menu(cx);

            match entry.kind {
                ContextMenuEntryKind::Builtin(action) => {
                    self.execute_builtin_context_menu_action(action, window, cx)
                }
                ContextMenuEntryKind::Custom { id } => {
                    let Some(handler) = self.context_menu_action_handler.clone() else {
                        return Ok(());
                    };
                    let Some(context) =
                        self.build_action_context(id, window, trigger_position_px)?
                    else {
                        return Ok(());
                    };
                    handler(context)
                }
                ContextMenuEntryKind::Separator => Ok(()),
            }
        }

        fn execute_builtin_context_menu_action(
            &mut self,
            action: BuiltinContextMenuAction,
            window: &Window,
            cx: &mut Context<Self>,
        ) -> Result<()> {
            match action {
                BuiltinContextMenuAction::ResetView => {
                    self.reset_view(cx);
                    Ok(())
                }
                BuiltinContextMenuAction::SetCurrentViewAsHome => self.set_current_view_as_home(cx),
                BuiltinContextMenuAction::GoToHomeView => self.go_to_home_view(cx),
                BuiltinContextMenuAction::SavePng => self.save_png(window, cx),
                BuiltinContextMenuAction::CopyImage => self.copy_image(window, cx),
                BuiltinContextMenuAction::CopyCursorCoordinates => self.copy_cursor_coordinates(),
                BuiltinContextMenuAction::CopyVisibleBounds => self.copy_visible_bounds(),
            }
        }

        fn builtin_shortcut_action_for_keystroke(
            &self,
            event: &KeyDownEvent,
        ) -> Option<BuiltinContextMenuAction> {
            if !event.keystroke.modifiers.secondary() || event.keystroke.modifiers.alt {
                return None;
            }

            match event.keystroke.key.as_str() {
                "s" if self.options.context_menu.show_save_png => {
                    Some(BuiltinContextMenuAction::SavePng)
                }
                "c" if self.options.context_menu.show_copy_image => {
                    Some(BuiltinContextMenuAction::CopyImage)
                }
                _ => None,
            }
        }

        fn zoom_overlay_bounds(&self) -> Option<Bounds<Pixels>> {
            let ActiveDrag::RightZoom {
                anchor_px,
                current_px,
                crossed_threshold,
                zoom_enabled,
            } = self.active_drag
            else {
                return None;
            };
            if !crossed_threshold || !zoom_enabled {
                return None;
            }

            let start = self.viewport_point_to_window_position(anchor_px)?;
            let end = self.viewport_point_to_window_position(current_px)?;
            let left = start.x.min(end.x);
            let top = start.y.min(end.y);
            let width = (start.x.max(end.x) - left).max(px(1.0));
            let height = (start.y.max(end.y) - top).max(px(1.0));
            Some(Bounds::new(point(left, top), size(width, height)))
        }

        fn handle_left_mouse_down(
            &mut self,
            event: &MouseDownEvent,
            window: &mut Window,
            cx: &mut Context<Self>,
        ) -> Result<()> {
            window.focus(&self.focus_handle);

            if self.context_menu.is_some() {
                return self.handle_context_menu_left_click(event.position, window, cx);
            }

            let Some(position_px) = self.local_viewport_point(event.position) else {
                return Ok(());
            };

            self.last_pointer_px = Some(position_px);
            if event.click_count >= 2 {
                self.reset_view(cx);
                return Ok(());
            }

            if event.modifiers.shift && self.options.interaction.selection {
                self.active_drag = ActiveDrag::Brush {
                    anchor_px: position_px,
                    current_px: position_px,
                    crossed_threshold: false,
                };
                self.session
                    .apply_input(PlotInputEvent::BrushStart { position_px });
                cx.notify();
                return Ok(());
            }

            if self.options.interaction.pan {
                self.active_drag = ActiveDrag::LeftPan {
                    anchor_px: position_px,
                    last_px: position_px,
                    crossed_threshold: false,
                };
            } else {
                self.active_drag = ActiveDrag::None;
            }
            cx.notify();
            Ok(())
        }

        fn handle_right_mouse_down(
            &mut self,
            event: &MouseDownEvent,
            window: &mut Window,
            cx: &mut Context<Self>,
        ) -> Result<()> {
            window.focus(&self.focus_handle);
            if self.context_menu.is_some() {
                self.close_context_menu(cx);
            }

            let Some(position_px) = self.local_viewport_point(event.position) else {
                return Ok(());
            };
            self.last_pointer_px = Some(position_px);
            self.active_drag = ActiveDrag::RightZoom {
                anchor_px: position_px,
                current_px: position_px,
                crossed_threshold: false,
                zoom_enabled: self.options.interaction.zoom,
            };
            cx.notify();
            Ok(())
        }

        fn handle_mouse_move(
            &mut self,
            event: &MouseMoveEvent,
            cx: &mut Context<Self>,
        ) -> Result<()> {
            if self.context_menu.is_some() {
                let hovered_index = self.context_menu_entry_index_at(event.position);
                self.update_context_menu_hover(hovered_index, cx);
                return Ok(());
            }

            let position_px = self.local_viewport_point(event.position);
            if let Some(position_px) = position_px {
                self.last_pointer_px = Some(position_px);
            }

            match (self.active_drag, event.pressed_button) {
                (
                    ActiveDrag::LeftPan {
                        anchor_px,
                        last_px,
                        crossed_threshold,
                    },
                    Some(MouseButton::Left),
                ) if self.options.interaction.pan => {
                    let Some(position_px) = position_px else {
                        return Ok(());
                    };
                    let crossed_threshold_now = crossed_threshold
                        || distance_px(anchor_px, position_px) > DRAG_THRESHOLD_PX;
                    let delta_px =
                        ViewportPoint::new(position_px.x - last_px.x, position_px.y - last_px.y);
                    if delta_px.x.abs() > f64::EPSILON || delta_px.y.abs() > f64::EPSILON {
                        self.session.apply_input(PlotInputEvent::Pan { delta_px });
                        cx.notify();
                    }
                    self.active_drag = ActiveDrag::LeftPan {
                        anchor_px,
                        last_px: position_px,
                        crossed_threshold: crossed_threshold_now,
                    };
                    return Ok(());
                }
                (
                    ActiveDrag::Brush {
                        anchor_px,
                        current_px: _,
                        crossed_threshold,
                    },
                    Some(MouseButton::Left),
                ) if self.options.interaction.selection => {
                    let Some(position_px) = position_px else {
                        return Ok(());
                    };
                    let crossed_threshold_now = crossed_threshold
                        || distance_px(anchor_px, position_px) > DRAG_THRESHOLD_PX;
                    self.session
                        .apply_input(PlotInputEvent::BrushMove { position_px });
                    self.active_drag = ActiveDrag::Brush {
                        anchor_px,
                        current_px: position_px,
                        crossed_threshold: crossed_threshold_now,
                    };
                    cx.notify();
                    return Ok(());
                }
                (
                    ActiveDrag::RightZoom {
                        anchor_px,
                        current_px: _,
                        crossed_threshold,
                        zoom_enabled,
                    },
                    Some(MouseButton::Right),
                ) => {
                    let position_px = if zoom_enabled {
                        self.clamped_viewport_point(event.position)
                            .unwrap_or(anchor_px)
                    } else {
                        position_px.unwrap_or(anchor_px)
                    };
                    let crossed_threshold_now = crossed_threshold
                        || distance_px(anchor_px, position_px) > DRAG_THRESHOLD_PX;
                    if crossed_threshold_now {
                        self.session.apply_input(PlotInputEvent::ClearHover);
                        self.session.apply_input(PlotInputEvent::HideTooltip);
                    }
                    self.active_drag = ActiveDrag::RightZoom {
                        anchor_px,
                        current_px: position_px,
                        crossed_threshold: crossed_threshold_now,
                        zoom_enabled,
                    };
                    cx.notify();
                    return Ok(());
                }
                _ => {}
            }

            match position_px {
                Some(position_px) if self.options.interaction.hover => {
                    self.session
                        .apply_input(PlotInputEvent::Hover { position_px });
                    if !self.options.interaction.tooltips {
                        self.session.apply_input(PlotInputEvent::HideTooltip);
                    }
                    cx.notify();
                }
                None if self.options.interaction.hover => {
                    self.session.apply_input(PlotInputEvent::ClearHover);
                    self.session.apply_input(PlotInputEvent::HideTooltip);
                    cx.notify();
                }
                None | Some(_) => {}
            }

            Ok(())
        }

        fn handle_left_mouse_up(
            &mut self,
            event: &MouseUpEvent,
            cx: &mut Context<Self>,
        ) -> Result<()> {
            let position_px = self
                .local_viewport_point(event.position)
                .or(self.last_pointer_px);

            match (self.active_drag, position_px) {
                (
                    ActiveDrag::Brush {
                        anchor_px: _,
                        current_px: _,
                        crossed_threshold: _,
                    },
                    Some(position_px),
                ) if self.options.interaction.selection => {
                    self.session
                        .apply_input(PlotInputEvent::BrushEnd { position_px });
                    cx.notify();
                }
                (
                    ActiveDrag::LeftPan {
                        anchor_px,
                        last_px: _,
                        crossed_threshold,
                    },
                    Some(position_px),
                ) if self.options.interaction.selection && !crossed_threshold => {
                    if distance_px(anchor_px, position_px) <= DRAG_THRESHOLD_PX {
                        self.session
                            .apply_input(PlotInputEvent::SelectAt { position_px });
                        cx.notify();
                    }
                }
                _ => {}
            }

            self.reset_pointer_state();
            Ok(())
        }

        fn handle_right_mouse_up(
            &mut self,
            event: &MouseUpEvent,
            window: &Window,
            cx: &mut Context<Self>,
        ) -> Result<()> {
            let position_px = self
                .local_viewport_point(event.position)
                .or_else(|| self.clamped_viewport_point(event.position))
                .or(self.last_pointer_px);

            match (self.active_drag, position_px) {
                (
                    ActiveDrag::RightZoom {
                        anchor_px,
                        current_px: _,
                        crossed_threshold,
                        zoom_enabled,
                    },
                    Some(position_px),
                ) => {
                    if crossed_threshold {
                        if zoom_enabled {
                            let region_px = ViewportRect::from_points(anchor_px, position_px);
                            if region_px.width() > DRAG_THRESHOLD_PX
                                && region_px.height() > DRAG_THRESHOLD_PX
                            {
                                self.session
                                    .apply_input(PlotInputEvent::ZoomRect { region_px });
                                self.invalidate_frame_state();
                                cx.notify();
                            }
                        }
                    } else {
                        self.open_context_menu(event.position, position_px, cx)?;
                    }
                }
                (ActiveDrag::RightZoom { .. }, None) => {
                    self.close_context_menu(cx);
                }
                _ => {}
            }

            self.reset_pointer_state();
            let _ = window;
            Ok(())
        }

        fn handle_hover_change(&mut self, hovered: bool, cx: &mut Context<Self>) {
            if hovered {
                return;
            }

            if self.context_menu.is_some() {
                self.update_context_menu_hover(None, cx);
                return;
            }

            self.session.apply_input(PlotInputEvent::ClearHover);
            self.session.apply_input(PlotInputEvent::HideTooltip);
            self.reset_pointer_state();
            cx.notify();
        }

        fn normalize_scroll_delta(delta: ScrollDelta) -> f64 {
            match delta {
                ScrollDelta::Pixels(delta) => f64::from(delta.y),
                ScrollDelta::Lines(delta) => -(delta.y as f64) * f64::from(LINE_SCROLL_DELTA_PX),
            }
        }

        fn handle_scroll_wheel(
            &mut self,
            event: &ScrollWheelEvent,
            cx: &mut Context<Self>,
        ) -> Result<()> {
            if !self.options.interaction.zoom || self.context_menu.is_some() {
                return Ok(());
            }

            let Some(center_px) = self.local_viewport_point(event.position) else {
                return Ok(());
            };

            let delta_y = Self::normalize_scroll_delta(event.delta);
            if delta_y.abs() <= f64::EPSILON {
                return Ok(());
            }

            let factor = (1.0025f64).powf(-delta_y).clamp(0.25, 4.0);
            self.session
                .apply_input(PlotInputEvent::Zoom { factor, center_px });
            cx.notify();
            Ok(())
        }

        fn handle_key_down(
            &mut self,
            event: &KeyDownEvent,
            window: &Window,
            cx: &mut Context<Self>,
        ) -> Result<bool> {
            if let Some(action) = self.builtin_shortcut_action_for_keystroke(event) {
                self.close_context_menu(cx);
                self.execute_builtin_context_menu_action(action, window, cx)?;
                return Ok(true);
            }

            match event.keystroke.key.as_str() {
                "escape" => {
                    if self.context_menu.is_some() {
                        self.close_context_menu(cx);
                    } else {
                        self.reset_view(cx);
                    }
                    Ok(true)
                }
                "delete" => {
                    self.session.apply_input(PlotInputEvent::ClearSelection);
                    self.invalidate_frame_state();
                    cx.notify();
                    Ok(true)
                }
                _ => Ok(false),
            }
        }

        fn render_zoom_overlay(&self, bounds: Bounds<Pixels>) -> AnyElement {
            let layout = self
                .last_layout
                .as_ref()
                .expect("zoom overlay requires an active layout");
            let local_left = bounds.origin.x - layout.component_bounds.origin.x;
            let local_top = bounds.origin.y - layout.component_bounds.origin.y;
            div()
                .absolute()
                .left(local_left)
                .top(local_top)
                .w(bounds.size.width)
                .h(bounds.size.height)
                .bg(rgba(0x5aa9e633))
                .border_1()
                .border_color(rgba(0x7fc8f8ff))
                .rounded_sm()
                .into_any_element()
        }

        fn render_context_menu_overlay(&self) -> Option<AnyElement> {
            let layout = self.last_layout.as_ref()?;
            let menu = self.context_menu.as_ref()?;
            let panel_left = menu.panel_bounds.origin.x - layout.component_bounds.origin.x;
            let panel_top = menu.panel_bounds.origin.y - layout.component_bounds.origin.y;
            let mut panel = div()
                .absolute()
                .left(panel_left)
                .top(panel_top)
                .w(menu.panel_bounds.size.width)
                .bg(rgb(0x1b1e24))
                .border_1()
                .border_color(rgba(0x46515eff))
                .rounded_md()
                .shadow_lg()
                .overflow_hidden()
                .flex()
                .flex_col()
                .py(px(MENU_PADDING_Y_PX));

            let mut children = Vec::new();
            for (index, entry) in menu.entries.iter().enumerate() {
                if matches!(entry.kind, ContextMenuEntryKind::Separator) {
                    children.push(
                        div()
                            .h(px(1.0))
                            .mx(px(MENU_PADDING_X_PX))
                            .my(px((MENU_SEPARATOR_HEIGHT_PX - 1.0) * 0.5))
                            .bg(rgba(0x46515eff))
                            .into_any_element(),
                    );
                    continue;
                }

                let is_hovered = menu.hovered_index == Some(index) && entry.enabled;
                let text_color = if entry.enabled {
                    rgb(0xf5f7fb)
                } else {
                    rgb(0x707781)
                };

                let entry_bg = if is_hovered {
                    rgba(0x3b4d6333)
                } else {
                    rgba(0x00000000)
                };
                children.push(
                    div()
                        .h(px(MENU_ITEM_HEIGHT_PX))
                        .px(px(MENU_PADDING_X_PX))
                        .bg(entry_bg)
                        .text_color(text_color)
                        .text_sm()
                        .flex()
                        .items_center()
                        .child(entry.label.clone())
                        .into_any_element(),
                );
            }
            panel = panel.children(children);
            Some(panel.into_any_element())
        }
    }

    impl Render for RuvizPlot {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let entity = cx.entity();
            self.ensure_reactive_watcher(entity.clone(), window, cx);

            let plot_canvas = canvas::<Option<PaintFrame>>(
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
                    move |bounds, image: Option<PaintFrame>, window: &mut Window, _cx| {
                        presentation_clock
                            .lock()
                            .expect("RuvizPlot presentation clock lock poisoned")
                            .record_now();
                        if let Some(image) = image {
                            let fitted_bounds = match image.primary {
                                PrimaryFrame::Image(primary_image) => {
                                    let image_size = primary_image.size(0);
                                    let fitted_bounds =
                                        image_fit.into_gpui().get_bounds(bounds, image_size);
                                    let _ = window.paint_image(
                                        fitted_bounds,
                                        Corners::default(),
                                        primary_image,
                                        0,
                                        false,
                                    );
                                    fitted_bounds
                                }
                                #[cfg(all(feature = "gpu", target_os = "macos"))]
                                PrimaryFrame::Surface(surface) => {
                                    let image_size = size(
                                        surface.get_width().into(),
                                        surface.get_height().into(),
                                    );
                                    let fitted_bounds =
                                        image_fit.into_gpui().get_bounds(bounds, image_size);
                                    window.paint_surface(fitted_bounds, surface);
                                    fitted_bounds
                                }
                            };
                            if let Some(overlay_image) = image.overlay_image {
                                let _ = window.paint_image(
                                    fitted_bounds,
                                    Corners::default(),
                                    overlay_image,
                                    0,
                                    false,
                                );
                            }
                        }
                    }
                },
            )
            .size_full();

            let mut root = div()
                .relative()
                .track_focus(&self.focus_handle)
                .child(plot_canvas)
                .when_some(self.zoom_overlay_bounds(), |this, bounds| {
                    this.child(self.render_zoom_overlay(bounds))
                })
                .when_some(self.render_context_menu_overlay(), |this, menu_overlay| {
                    this.child(menu_overlay)
                })
                .on_mouse_down(MouseButton::Left, {
                    let entity = entity.clone();
                    move |event, window, cx| {
                        entity.update(cx, |view, cx| {
                            if let Err(err) = view.handle_left_mouse_down(event, window, cx) {
                                eprintln!("ruviz-gpui left mouse down failed: {err}");
                            }
                        });
                    }
                })
                .on_mouse_down(MouseButton::Right, {
                    let entity = entity.clone();
                    move |event, window, cx| {
                        entity.update(cx, |view, cx| {
                            if let Err(err) = view.handle_right_mouse_down(event, window, cx) {
                                eprintln!("ruviz-gpui right mouse down failed: {err}");
                            }
                        });
                    }
                })
                .on_mouse_move({
                    let entity = entity.clone();
                    move |event, _, cx| {
                        entity.update(cx, |view, cx| {
                            if let Err(err) = view.handle_mouse_move(event, cx) {
                                eprintln!("ruviz-gpui mouse move failed: {err}");
                            }
                        });
                    }
                })
                .on_mouse_up(MouseButton::Left, {
                    let entity = entity.clone();
                    move |event, _, cx| {
                        entity.update(cx, |view, cx| {
                            if let Err(err) = view.handle_left_mouse_up(event, cx) {
                                eprintln!("ruviz-gpui left mouse up failed: {err}");
                            }
                        });
                    }
                })
                .on_mouse_up_out(MouseButton::Left, {
                    let entity = entity.clone();
                    move |event, _, cx| {
                        entity.update(cx, |view, cx| {
                            if let Err(err) = view.handle_left_mouse_up(event, cx) {
                                eprintln!("ruviz-gpui left mouse up-out failed: {err}");
                            }
                        });
                    }
                })
                .on_mouse_up(MouseButton::Right, {
                    let entity = entity.clone();
                    move |event, window, cx| {
                        entity.update(cx, |view, cx| {
                            if let Err(err) = view.handle_right_mouse_up(event, window, cx) {
                                eprintln!("ruviz-gpui right mouse up failed: {err}");
                            }
                        });
                    }
                })
                .on_mouse_up_out(MouseButton::Right, {
                    let entity = entity.clone();
                    move |event, window, cx| {
                        entity.update(cx, |view, cx| {
                            if let Err(err) = view.handle_right_mouse_up(event, window, cx) {
                                eprintln!("ruviz-gpui right mouse up-out failed: {err}");
                            }
                        });
                    }
                })
                .on_scroll_wheel({
                    let entity = entity.clone();
                    move |event, _, cx| {
                        entity.update(cx, |view, cx| {
                            if let Err(err) = view.handle_scroll_wheel(event, cx) {
                                eprintln!("ruviz-gpui scroll handling failed: {err}");
                            }
                        });
                    }
                })
                .on_key_down({
                    let entity = entity.clone();
                    move |event, window, cx| {
                        entity.update(cx, |view, cx| {
                            match view.handle_key_down(event, window, cx) {
                                Ok(true) => cx.stop_propagation(),
                                Ok(false) => {}
                                Err(err) => eprintln!("ruviz-gpui key handling failed: {err}"),
                            }
                        });
                    }
                });

            root.interactivity().on_hover({
                let entity = entity.clone();
                move |hovered, _, cx| {
                    entity.update(cx, |view, cx| {
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

    fn apply_performance_options(session: &InteractivePlotSession, options: PerformanceOptions) {
        session.set_frame_pacing(options.frame_pacing);
        session.set_quality_policy(options.quality_policy);
        session.set_prefer_gpu(options.prefer_gpu);
    }

    fn active_backend_for_frame(frame: &CachedFrame) -> ActiveBackend {
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

    fn distance_px(a: ViewportPoint, b: ViewportPoint) -> f64 {
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        (dx * dx + dy * dy).sqrt()
    }

    fn cursor_data_position(
        visible_bounds: ViewportRect,
        plot_area: ViewportRect,
        cursor_position_px: ViewportPoint,
    ) -> Option<ViewportPoint> {
        let plot_width = plot_area.width();
        let plot_height = plot_area.height();
        if plot_width <= f64::EPSILON || plot_height <= f64::EPSILON {
            return None;
        }
        if !plot_area.contains(cursor_position_px) {
            return None;
        }

        let normalized_x = (cursor_position_px.x - plot_area.min.x) / plot_width;
        let normalized_y = (cursor_position_px.y - plot_area.min.y) / plot_height;
        Some(ViewportPoint::new(
            visible_bounds.min.x + visible_bounds.width() * normalized_x,
            visible_bounds.max.y - visible_bounds.height() * normalized_y,
        ))
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

    fn retire_primary_frame(retired_images: &mut Vec<Arc<RenderImage>>, primary: PrimaryFrame) {
        match primary {
            PrimaryFrame::Image(image) => retired_images.push(image),
            #[cfg(all(feature = "gpu", target_os = "macos"))]
            PrimaryFrame::Surface(_) => {}
        }
    }

    #[cfg(all(feature = "gpu", target_os = "macos"))]
    fn maybe_retire_replaced_primary(
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
    fn maybe_retire_replaced_primary(
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
    fn should_use_surface_primary(
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
    fn should_use_surface_primary(
        _presentation_mode: PresentationMode,
        _target: RenderTargetKind,
        _surface_capability: SurfaceCapability,
    ) -> bool {
        false
    }

    #[cfg(all(feature = "gpu", target_os = "macos"))]
    fn make_surface_pixel_buffer_options() -> CFDictionary<CFString, CFType> {
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
        fn update(
            &mut self,
            previous: Option<&CVPixelBuffer>,
            image: &RuvizImage,
        ) -> std::result::Result<CVPixelBuffer, String> {
            let width = image.width as usize;
            let height = image.height as usize;
            let pixel_buffer = match previous {
                Some(previous)
                    if previous.get_width() == width && previous.get_height() == height =>
                {
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
                    PresentationMode::Image => Some(RenderedPrimary::Image(
                        render_image_from_ruviz(frame.image.as_ref().clone()),
                    )),
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

    fn render_image_from_ruviz(image: RuvizImage) -> Arc<RenderImage> {
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

    fn rgba_to_bgra_in_place(pixels: &mut [u8]) {
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use gpui::{Modifiers, TestAppContext};
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
        fn test_render_request_becomes_dirty_after_session_invalidate() {
            let plot: Plot = Plot::new().line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0]).into();
            let session = plot.prepare_interactive();
            let request = RenderRequest::new((320, 240), 1.0, 0.0, PresentationMode::Hybrid);

            session
                .render_to_surface(SurfaceTarget {
                    size_px: request.size_px,
                    scale_factor: request.scale_factor(),
                    time_seconds: request.time_seconds(),
                })
                .expect("interactive session should render");
            assert!(!request.is_dirty(&session));

            session.invalidate();
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
        fn test_replace_session_retires_cached_frame() {
            let cx = TestAppContext::single();
            let focus_handle = cx.update(|cx| cx.focus_handle());
            let initial_plot: Plot = Plot::new().line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0]).into();
            let initial_session = initial_plot.prepare_interactive();
            let (pending, receiver, subscription) = bind_reactive_session(&initial_session);
            let cached_primary =
                render_image_from_ruviz(ruviz::core::plot::Image::new(1, 1, vec![0, 0, 0, 255]));
            let cached_frame = CachedFrame {
                request: RenderRequest::new((320, 240), 1.0, 0.0, PresentationMode::Hybrid),
                primary: PrimaryFrame::Image(cached_primary),
                overlay_image: None,
                stats: FrameStats::default(),
                target: RenderTargetKind::Surface,
            };
            let mut view = RuvizPlot {
                session: initial_session,
                subscription,
                reactive_notify_pending: pending,
                reactive_receiver: Some(receiver),
                reactive_watcher: None,
                presentation_clock: Arc::new(Mutex::new(PresentationClock::default())),
                options: RuvizPlotOptions::default(),
                cached_frame: Some(cached_frame),
                retired_images: Vec::new(),
                #[cfg(all(feature = "gpu", target_os = "macos"))]
                surface_upload: SurfaceUploadState::default(),
                scheduler: RenderScheduler::default(),
                in_flight_render: None,
                last_layout: Some(InteractionLayout {
                    component_bounds: Bounds::default(),
                    content_bounds: Bounds::default(),
                    frame_size_px: (320, 240),
                }),
                focus_handle,
                last_pointer_px: Some(ViewportPoint::new(2.0, 2.0)),
                active_drag: ActiveDrag::LeftPan {
                    anchor_px: ViewportPoint::new(1.0, 1.0),
                    last_px: ViewportPoint::new(2.0, 2.0),
                    crossed_threshold: true,
                },
                context_menu_action_handler: None,
                context_menu: None,
                home_view_bounds: None,
            };

            let replacement_plot: Plot = Plot::new().line(&[0.0, 1.0], &[1.0, 2.0]).into();
            let replacement_session = replacement_plot.prepare_interactive();
            view.replace_session(replacement_session);

            assert!(view.cached_frame.is_none());
            assert_eq!(view.retired_images.len(), 1);
            assert!(view.last_layout.is_none());
            assert_eq!(view.active_drag, ActiveDrag::None);
            assert!(view.last_pointer_px.is_none());
        }

        #[test]
        fn test_secondary_shortcut_maps_to_builtins() {
            let view = RuvizPlot {
                session: {
                    let plot: Plot = Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]).into();
                    plot.prepare_interactive()
                },
                subscription: ReactiveSubscription::default(),
                reactive_notify_pending: Arc::new(AtomicBool::new(false)),
                reactive_receiver: None,
                reactive_watcher: None,
                presentation_clock: Arc::new(Mutex::new(PresentationClock::default())),
                options: RuvizPlotOptions::default(),
                cached_frame: None,
                retired_images: Vec::new(),
                #[cfg(all(feature = "gpu", target_os = "macos"))]
                surface_upload: SurfaceUploadState::default(),
                scheduler: RenderScheduler::default(),
                in_flight_render: None,
                last_layout: None,
                focus_handle: TestAppContext::single().update(|cx| cx.focus_handle()),
                last_pointer_px: None,
                active_drag: ActiveDrag::None,
                context_menu_action_handler: None,
                context_menu: None,
                home_view_bounds: None,
            };

            let save = KeyDownEvent {
                keystroke: gpui::Keystroke {
                    modifiers: Modifiers::secondary_key(),
                    key: "s".to_string(),
                    key_char: None,
                },
                is_held: false,
            };
            let copy = KeyDownEvent {
                keystroke: gpui::Keystroke {
                    modifiers: Modifiers::secondary_key(),
                    key: "c".to_string(),
                    key_char: None,
                },
                is_held: false,
            };

            assert_eq!(
                view.builtin_shortcut_action_for_keystroke(&save),
                Some(BuiltinContextMenuAction::SavePng)
            );
            assert_eq!(
                view.builtin_shortcut_action_for_keystroke(&copy),
                Some(BuiltinContextMenuAction::CopyImage)
            );
        }

        #[test]
        fn test_cursor_data_position_maps_plot_area() {
            let visible = ViewportRect::from_points(
                ViewportPoint::new(0.0, 0.0),
                ViewportPoint::new(10.0, 20.0),
            );
            let plot_area = ViewportRect::from_points(
                ViewportPoint::new(100.0, 50.0),
                ViewportPoint::new(300.0, 250.0),
            );
            let cursor = ViewportPoint::new(200.0, 150.0);

            let data = cursor_data_position(visible, plot_area, cursor)
                .expect("cursor inside plot area should map to data coordinates");
            assert!((data.x - 5.0).abs() < 1e-6);
            assert!((data.y - 10.0).abs() < 1e-6);
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

        #[test]
        fn test_presentation_clock_preserves_average_precision() {
            let start = Instant::now();
            let mut clock = PresentationClock::default();

            clock.record_at(start);
            clock.record_at(start + Duration::from_nanos(1));
            clock.record_at(start + Duration::from_nanos(3));
            clock.record_at(start + Duration::from_nanos(6));

            let stats = clock.stats();
            assert_eq!(stats.average_present_interval, Duration::from_nanos(2));
            assert!((clock.average_present_interval_secs - 2e-9).abs() < 1e-18);
        }

        #[test]
        fn test_render_scheduler_coalesces_queued_requests() {
            let mut scheduler = RenderScheduler::default();
            let first = RenderRequest::new((320, 240), 1.0, 0.0, PresentationMode::Hybrid);
            let second = RenderRequest::new((320, 240), 1.0, 1.0, PresentationMode::Hybrid);
            let third = RenderRequest::new((640, 480), 1.0, 1.0, PresentationMode::Hybrid);

            let scheduled = scheduler
                .schedule(first.clone())
                .expect("first request should start");
            scheduler.start(scheduled.clone());
            assert!(scheduler.schedule(second).is_none());
            assert!(scheduler.schedule(third).is_none());
            assert_eq!(scheduler.dropped_frames, 1);

            assert!(scheduler.finish(&scheduled));
            let queued = scheduler
                .take_queued()
                .expect("queued request should remain");
            assert_eq!(queued.request.size_px, (640, 480));
        }

        #[test]
        fn test_surface_primary_only_enabled_for_fast_path_frames() {
            #[allow(deprecated)]
            let deprecated_hybrid = PresentationMode::SurfaceExperimental;

            #[cfg(all(feature = "gpu", target_os = "macos"))]
            {
                assert!(should_use_surface_primary(
                    PresentationMode::Hybrid,
                    RenderTargetKind::Surface,
                    SurfaceCapability::FastPath,
                ));
                assert!(should_use_surface_primary(
                    deprecated_hybrid,
                    RenderTargetKind::Surface,
                    SurfaceCapability::FastPath,
                ));
            }

            #[cfg(not(all(feature = "gpu", target_os = "macos")))]
            {
                assert!(!should_use_surface_primary(
                    PresentationMode::Hybrid,
                    RenderTargetKind::Surface,
                    SurfaceCapability::FastPath,
                ));
                assert!(!should_use_surface_primary(
                    deprecated_hybrid,
                    RenderTargetKind::Surface,
                    SurfaceCapability::FastPath,
                ));
            }

            assert!(!should_use_surface_primary(
                PresentationMode::Hybrid,
                RenderTargetKind::Surface,
                SurfaceCapability::FallbackImage,
            ));
            assert!(!should_use_surface_primary(
                PresentationMode::Image,
                RenderTargetKind::Image,
                SurfaceCapability::Unsupported,
            ));
        }

        #[test]
        fn test_active_backend_reports_fallback_for_image_backed_surface_frames() {
            let frame = CachedFrame {
                request: RenderRequest::new((320, 240), 1.0, 0.0, PresentationMode::Hybrid),
                primary: PrimaryFrame::Image(render_image_from_ruviz(
                    ruviz::core::plot::Image::new(1, 1, vec![0, 0, 0, 255]),
                )),
                overlay_image: None,
                stats: FrameStats::default(),
                target: RenderTargetKind::Surface,
            };

            assert_eq!(
                active_backend_for_frame(&frame),
                ActiveBackend::HybridFallback
            );
        }

        #[cfg(all(feature = "gpu", target_os = "macos"))]
        #[test]
        fn test_active_backend_reports_fast_path_for_surface_backed_frames() {
            let mut upload = SurfaceUploadState::default();
            let surface = upload
                .update(
                    None,
                    &ruviz::core::plot::Image::new(
                        2,
                        2,
                        vec![
                            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
                        ],
                    ),
                )
                .expect("surface upload should succeed");
            let frame = CachedFrame {
                request: RenderRequest::new((320, 240), 1.0, 0.0, PresentationMode::Hybrid),
                primary: PrimaryFrame::Surface(surface),
                overlay_image: None,
                stats: FrameStats::default(),
                target: RenderTargetKind::Surface,
            };

            assert_eq!(
                active_backend_for_frame(&frame),
                ActiveBackend::HybridFastPath
            );
        }

        #[cfg(all(feature = "gpu", target_os = "macos"))]
        #[test]
        fn test_surface_upload_reuses_pixel_buffer_when_size_is_stable() {
            let mut upload = SurfaceUploadState::default();
            let first = upload
                .update(
                    None,
                    &ruviz::core::plot::Image::new(2, 1, vec![1, 2, 3, 255, 4, 5, 6, 255]),
                )
                .expect("first surface upload should succeed");
            let reused = upload
                .update(
                    Some(&first),
                    &ruviz::core::plot::Image::new(2, 1, vec![10, 20, 30, 255, 40, 50, 60, 255]),
                )
                .expect("second surface upload should succeed");

            assert_eq!(first, reused);
            assert_eq!(reused.get_width(), 2);
            assert_eq!(reused.get_height(), 1);
            assert_eq!(
                reused.get_pixel_format(),
                kCVPixelFormatType_420YpCbCr8BiPlanarFullRange
            );
            assert!(reused.is_planar());
            assert_eq!(reused.get_plane_count(), 2);
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub use gpui;

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub use supported::*;
