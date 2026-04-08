//! GPUI component adapter for [`ruviz`](https://docs.rs/ruviz).
//!
//! `ruviz-gpui` embeds a shared `ruviz` interactive plot session inside a GPUI
//! application. It is intended for native desktop apps that want the plot
//! interaction model from `ruviz` while keeping presentation and layout inside a
//! GPUI view tree.
//!
//! # What This Crate Provides
//!
//! - `RuvizPlot` for embedding static or interactive plots in GPUI views
//! - configurable presentation modes for image-backed and hybrid rendering
//! - built-in pan, zoom, hover, selection, and context-menu behavior
//! - clipboard and PNG save helpers routed through the host platform
//!
//! # Platform Support
//!
//! `ruviz-gpui` currently supports macOS, Linux, and Windows. Unsupported
//! targets fail at compile time.
//!
//! # Recommended Usage
//!
//! Add both crates to your application:
//!
//! ```toml
//! [dependencies]
//! ruviz = "0.4.3"
//! ruviz-gpui = "0.4.3"
//! ```
//!
//! Then build a normal `ruviz::Plot` or `PreparedPlot` and hand it to the GPUI
//! component. See the repository examples under
//! `crates/ruviz-gpui/examples/` for end-to-end setups.
//!
//! # Documentation
//!
//! - Repository README: <https://github.com/Ameyanagi/ruviz/blob/main/README.md>
//! - Adapter README: <https://github.com/Ameyanagi/ruviz/tree/main/crates/ruviz-gpui>

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
compile_error!("ruviz-gpui currently supports macOS, Linux, and Windows only.");

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
mod platform_impl {
    mod interaction;
    mod presentation;

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
        AnyElement, App, Bounds, Context, Corners, Entity, FocusHandle, Focusable,
        InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent,
        MouseUpEvent, ObjectFit, Pixels, Point, Render, RenderImage, ScrollDelta, ScrollWheelEvent,
        Task, Window, canvas, div, point, prelude::*, px, rgb, rgba, size,
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

    use self::interaction::*;
    use self::presentation::*;

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
        /// Controls whether the "Save PNG..." context menu item and built-in `Cmd/Ctrl+S`
        /// shortcut are available.
        pub show_save_png: bool,
        /// Controls whether the "Copy Image" context menu item and built-in `Cmd/Ctrl+C`
        /// shortcut are available.
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
        /// Snapshot of the current visible plot image, captured eagerly when the action context
        /// is built. Cached image-backed frames are reused when available; otherwise this may
        /// trigger a fresh image render.
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

        fn validate(&self) -> Result<()> {
            if self.options.context_menu.enabled
                && !self.options.context_menu.custom_items.is_empty()
                && self.context_menu_action_handler.is_none()
            {
                return Err(PlottingError::InvalidInput(
                    "GPUI context menu custom items require on_context_menu_action(...) before build()"
                        .to_string(),
                ));
            }
            Ok(())
        }

        pub fn try_build<V>(self, cx: &mut Context<V>) -> Result<Entity<RuvizPlot>>
        where
            V: 'static,
        {
            self.validate()?;
            let options = self.options;
            let plot = self.plot;
            let context_menu_action_handler = self.context_menu_action_handler;
            Ok(cx.new(move |cx| {
                RuvizPlot::from_options_impl(plot, options, context_menu_action_handler, cx)
            }))
        }

        pub fn build<V>(self, cx: &mut Context<V>) -> Entity<RuvizPlot>
        where
            V: 'static,
        {
            self.try_build(cx)
                .unwrap_or_else(|err| panic!("failed to build RuvizPlot: {err}"))
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
        interaction_state: InteractionState,
        context_menu_action_handler: Option<ContextMenuActionHandler>,
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
                interaction_state: InteractionState::default(),
                context_menu_action_handler,
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
            self.interaction_state.home_view_bounds =
                Some(self.session.viewport_snapshot()?.visible_bounds);
            cx.notify();
            Ok(())
        }

        pub fn go_to_home_view(&mut self, cx: &mut Context<Self>) -> Result<()> {
            let Some(home_view_bounds) = self.interaction_state.home_view_bounds else {
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
            self.copy_cursor_coordinates_at(cursor_position_px)
        }

        pub(crate) fn copy_cursor_coordinates_at(
            &self,
            cursor_position_px: ViewportPoint,
        ) -> Result<()> {
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
            self.interaction_state.reset();
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
                                log_interaction_error("left mouse down", &err);
                            }
                        });
                    }
                })
                .on_mouse_down(MouseButton::Right, {
                    let entity = entity.clone();
                    move |event, window, cx| {
                        entity.update(cx, |view, cx| {
                            if let Err(err) = view.handle_right_mouse_down(event, window, cx) {
                                log_interaction_error("right mouse down", &err);
                            }
                        });
                    }
                })
                .on_mouse_move({
                    let entity = entity.clone();
                    move |event, _, cx| {
                        entity.update(cx, |view, cx| {
                            if let Err(err) = view.handle_mouse_move(event, cx) {
                                log_interaction_error("mouse move", &err);
                            }
                        });
                    }
                })
                .on_mouse_up(MouseButton::Left, {
                    let entity = entity.clone();
                    move |event, _, cx| {
                        entity.update(cx, |view, cx| {
                            if let Err(err) = view.handle_left_mouse_up(event, cx) {
                                log_interaction_error("left mouse up", &err);
                            }
                        });
                    }
                })
                .on_mouse_up_out(MouseButton::Left, {
                    let entity = entity.clone();
                    move |event, _, cx| {
                        entity.update(cx, |view, cx| {
                            if let Err(err) = view.handle_left_mouse_up(event, cx) {
                                log_interaction_error("left mouse up-out", &err);
                            }
                        });
                    }
                })
                .on_mouse_up(MouseButton::Right, {
                    let entity = entity.clone();
                    move |event, _, cx| {
                        entity.update(cx, |view, cx| {
                            if let Err(err) = view.handle_right_mouse_up(event, cx) {
                                log_interaction_error("right mouse up", &err);
                            }
                        });
                    }
                })
                .on_mouse_up_out(MouseButton::Right, {
                    let entity = entity.clone();
                    move |event, _, cx| {
                        entity.update(cx, |view, cx| {
                            if let Err(err) = view.handle_right_mouse_up(event, cx) {
                                log_interaction_error("right mouse up-out", &err);
                            }
                        });
                    }
                })
                .on_scroll_wheel({
                    let entity = entity.clone();
                    move |event, _, cx| {
                        entity.update(cx, |view, cx| {
                            if let Err(err) = view.handle_scroll_wheel(event, cx) {
                                log_interaction_error("scroll handling", &err);
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
                                Err(err) => log_interaction_error("key handling", &err),
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

    impl Focusable for RuvizPlot {
        fn focus_handle(&self, _: &App) -> FocusHandle {
            self.focus_handle.clone()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use gpui::{Modifiers, MouseButton, TestAppContext};
        use ruviz::{data::Observable, prelude::Plot};

        #[test]
        fn test_rgba_to_bgra_conversion() {
            let mut pixels = vec![1, 2, 3, 255, 10, 20, 30, 128];
            rgba_to_bgra_in_place(&mut pixels);
            assert_eq!(pixels, vec![3, 2, 1, 255, 30, 20, 10, 128]);
        }

        #[test]
        fn test_render_image_to_ruviz_round_trips_pixels() {
            let original = ruviz::core::plot::Image::new(2, 1, vec![1, 2, 3, 255, 10, 20, 30, 128]);
            let render = render_image_from_ruviz(original.clone());
            let restored = render_image_to_ruviz(render.as_ref())
                .expect("render image should decode back into RGBA pixels");
            assert_eq!(restored.width, original.width);
            assert_eq!(restored.height, original.height);
            assert_eq!(restored.pixels, original.pixels);
        }

        #[test]
        fn test_cached_frame_capture_reuses_cached_image_and_overlay() {
            let cx = TestAppContext::single();
            let focus_handle = cx.update(|cx| cx.focus_handle());
            let plot: Plot = Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]).into();
            let session = plot.prepare_interactive();
            let (pending, receiver, subscription) = bind_reactive_session(&session);

            let base = ruviz::core::plot::Image::new(1, 1, vec![20, 40, 60, 255]);
            let overlay = ruviz::core::plot::Image::new(1, 1, vec![200, 100, 50, 128]);
            let mut expected = base.pixels.clone();
            blend_rgba_into_rgba(&overlay.pixels, &mut expected);

            let view = RuvizPlot {
                session,
                subscription,
                reactive_notify_pending: pending,
                reactive_receiver: Some(receiver),
                reactive_watcher: None,
                presentation_clock: Arc::new(Mutex::new(PresentationClock::default())),
                options: RuvizPlotOptions::default(),
                cached_frame: Some(CachedFrame {
                    request: RenderRequest::new((1, 1), 1.0, 0.0, PresentationMode::Hybrid),
                    primary: PrimaryFrame::Image(render_image_from_ruviz(base)),
                    overlay_image: Some(render_image_from_ruviz(overlay)),
                    stats: FrameStats::default(),
                    target: RenderTargetKind::Surface,
                }),
                retired_images: Vec::new(),
                #[cfg(all(feature = "gpu", target_os = "macos"))]
                surface_upload: SurfaceUploadState::default(),
                scheduler: RenderScheduler::default(),
                in_flight_render: None,
                last_layout: None,
                focus_handle,
                interaction_state: InteractionState::default(),
                context_menu_action_handler: None,
            };

            let captured = view
                .capture_visible_view_image_from_cache()
                .expect("cached frame should provide a visible image");
            assert_eq!(captured.width, 1);
            assert_eq!(captured.height, 1);
            assert_eq!(captured.pixels, expected);
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
                interaction_state: InteractionState {
                    last_pointer_px: Some(ViewportPoint::new(2.0, 2.0)),
                    active_drag: ActiveDrag::LeftPan {
                        anchor_px: ViewportPoint::new(1.0, 1.0),
                        last_px: ViewportPoint::new(2.0, 2.0),
                        crossed_threshold: true,
                    },
                    context_menu: None,
                    home_view_bounds: None,
                },
                context_menu_action_handler: None,
            };

            let replacement_plot: Plot = Plot::new().line(&[0.0, 1.0], &[1.0, 2.0]).into();
            let replacement_session = replacement_plot.prepare_interactive();
            view.replace_session(replacement_session);

            assert!(view.cached_frame.is_none());
            assert_eq!(view.retired_images.len(), 1);
            assert!(view.last_layout.is_none());
            assert_eq!(view.interaction_state.active_drag, ActiveDrag::None);
            assert!(view.interaction_state.last_pointer_px.is_none());
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
                interaction_state: InteractionState::default(),
                context_menu_action_handler: None,
            };

            let save = KeyDownEvent {
                keystroke: gpui::Keystroke {
                    modifiers: Modifiers::secondary_key(),
                    key: "s".to_string(),
                    key_char: None,
                },
                is_held: false,
                prefer_character_input: false,
            };
            let copy = KeyDownEvent {
                keystroke: gpui::Keystroke {
                    modifiers: Modifiers::secondary_key(),
                    key: "c".to_string(),
                    key_char: None,
                },
                is_held: false,
                prefer_character_input: false,
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
        fn test_right_mouse_up_far_from_anchor_zooms_without_move_events() {
            let mut app = TestAppContext::single();
            let (view, cx) = app.add_window_view(|_, cx| {
                let plot: Plot = Plot::new()
                    .line(&[0.0, 1.0, 2.0, 3.0], &[0.0, 1.0, 0.5, 1.5])
                    .into();
                RuvizPlot::from_options_impl(plot, RuvizPlotOptions::default(), None, cx)
            });

            cx.refresh().expect("window refresh should succeed");
            cx.run_until_parked();

            let (start, end, initial_bounds) = cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    let layout = view
                        .last_layout
                        .clone()
                        .expect("plot should have a resolved layout");
                    let start = point(
                        layout.content_bounds.origin.x + layout.content_bounds.size.width * 0.25,
                        layout.content_bounds.origin.y + layout.content_bounds.size.height * 0.25,
                    );
                    let end = point(
                        layout.content_bounds.origin.x + layout.content_bounds.size.width * 0.75,
                        layout.content_bounds.origin.y + layout.content_bounds.size.height * 0.75,
                    );
                    (
                        start,
                        end,
                        view.session
                            .viewport_snapshot()
                            .expect("viewport snapshot should succeed")
                            .visible_bounds,
                    )
                })
            });

            cx.simulate_mouse_down(start, MouseButton::Right, Modifiers::default());
            cx.simulate_mouse_up(end, MouseButton::Right, Modifiers::default());
            cx.run_until_parked();

            let (context_menu_open, final_bounds) = cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    (
                        view.interaction_state.context_menu.is_some(),
                        view.session
                            .viewport_snapshot()
                            .expect("viewport snapshot should succeed")
                            .visible_bounds,
                    )
                })
            });

            assert!(!context_menu_open);
            assert_ne!(final_bounds, initial_bounds);
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

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub use gpui;

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub use platform_impl::*;
