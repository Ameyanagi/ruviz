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
//! - absolute-window coordinate mapping and frame-aware click/hover callbacks
//! - clipboard and PNG save helpers routed through the host platform
//!
//! # Coordinates And Pointer Events
//!
//! [`RuvizPlot::data_at`] accepts an absolute GPUI window [`Point<Pixels>`], while
//! [`RuvizPlot::screen_at`] returns one. Both methods use the currently displayed
//! backing frame and return `Ok(None)` when layout or a valid in-bounds mapping is
//! unavailable. Click and hover callbacks share [`PlotPointerEvent`]:
//!
//! ```rust,ignore
//! let plot = ruviz_gpui::plot_builder(plot)
//!     .on_plot_click(|event| {
//!         println!("clicked data={:?}, hit={:?}", event.data_position, event.hit);
//!     })
//!     .on_plot_hover(|event| {
//!         update_status(event.window_position, event.data_position);
//!     })
//!     .build(cx);
//! ```
//!
//! Builder callbacks are convenient thread-safe observers. GPUI views that update host UI state
//! should normally subscribe to the plot entity with `cx.subscribe(&plot, ...)`; [`RuvizPlot`]
//! emits every click and hover event through [`gpui::EventEmitter`] as well as invoking the
//! corresponding builder callback.
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
//! ruviz = "0.4.20"
//! ruviz-gpui = "0.4.20"
//! ```
//!
//! Then build a normal `ruviz::Plot` or `PreparedPlot` and hand it to the GPUI
//! component. See the repository examples under
//! `crates/ruviz-gpui/examples/` for end-to-end setups.
//!
//! For data-only updates, build the plot once with `Plot::line_source` or
//! `Plot::scatter_source` and `Observable<Vec<f64>>`, then call
//! `Observable::set` to replace a complete coordinate vector. Reactive renders
//! retain a customized visible view; a view still at its base bounds can
//! autoscale to the new data. `BatchUpdate` defers each observable's
//! notifications until guard drop and coalesces repeated changes within that
//! observable. Separate observables still flush independently; the guard is not
//! a shared data lock.
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
        axes::AxisScale,
        core::plot::Image as RuvizImage,
        core::{
            Annotation, AnnotationId, FramePacing, FrameStats, HitResult, ImageTarget,
            InteractivePlotSession, InteractiveViewportSnapshot, Plot, PlotInputEvent,
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
    type PlotPointerEventHandler = Arc<dyn Fn(PlotPointerEvent) + Send + Sync>;

    #[derive(Clone, Default)]
    struct PlotPointerEventHandlers {
        click: Option<PlotPointerEventHandler>,
        hover: Option<PlotPointerEventHandler>,
    }

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

    /// The kind of pointer event delivered by a [`PlotPointerEvent`] callback.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum PlotPointerEventKind {
        Click,
        Hover,
    }

    /// Frame-aware coordinates and hit information for a plot pointer event.
    ///
    /// [`Self::window_position`] is an absolute GPUI window coordinate. The
    /// viewport coordinate, data coordinate, snapshot, and hit result all use
    /// the interactive session's currently displayed backing frame.
    /// [`RuvizPlot`] emits this type through [`gpui::EventEmitter`], so host views can use
    /// `cx.subscribe(&plot, ...)` and update UI state with their normal GPUI context.
    #[derive(Clone, Debug, PartialEq)]
    pub struct PlotPointerEvent {
        pub kind: PlotPointerEventKind,
        /// The mouse button for a click, or the currently pressed button for a hover.
        pub mouse_button: Option<MouseButton>,
        /// Absolute position in the GPUI window coordinate system.
        pub window_position: Point<Pixels>,
        /// Position in the plot's displayed backing frame, in viewport pixels.
        pub viewport_position: ViewportPoint,
        /// Displayed data coordinates, or `None` in frame margins outside the plot area.
        pub data_position: Option<ViewportPoint>,
        /// Current displayed viewport state, including plot area and visible bounds.
        pub viewport: InteractiveViewportSnapshot,
        /// Hit result from the same displayed-frame coordinate contract.
        pub hit: HitResult,
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
        presented_base_generation: Option<u64>,
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
                presented_base_generation: None,
            }
        }

        fn with_presented_base_generation(mut self, generation: Option<u64>) -> Self {
            self.presented_base_generation = generation;
            self
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

    fn fill_backing_dimension_px(logical_px: u32, scale_factor: f32) -> u32 {
        if logical_px == 0 {
            return 0;
        }
        let backing_scale = sanitize_scale_factor(scale_factor).max(1.0);
        ((logical_px as f32) * backing_scale).ceil().max(1.0) as u32
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
        base_generation: u64,
        primary: PrimaryFrame,
        overlay_image: Option<Arc<RenderImage>>,
        stats: FrameStats,
        target: RenderTargetKind,
    }

    struct RenderedFrame {
        base_generation: u64,
        primary: Option<RenderedPrimary>,
        overlay: RenderedOverlay,
        stats: FrameStats,
        target: RenderTargetKind,
    }

    enum RenderedOverlay {
        Reuse,
        Replace(Option<Arc<RenderImage>>),
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
        pointer_event_handlers: PlotPointerEventHandlers,
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
                pointer_event_handlers: PlotPointerEventHandlers::default(),
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

        /// Registers a thread-safe observer for primary-button clicks on the displayed plot frame.
        ///
        /// The callback runs on release and is suppressed for drags. Under standard platform
        /// double-click semantics, click-count 1 may emit before click-count 2 is known; the
        /// click-count 2 release emits no additional click. Host UI state should normally use
        /// `cx.subscribe(&plot, ...)` instead so the subscriber receives a GPUI context.
        pub fn on_plot_click<F>(mut self, handler: F) -> Self
        where
            F: Fn(PlotPointerEvent) + Send + Sync + 'static,
        {
            self.pointer_event_handlers.click = Some(Arc::new(handler));
            self
        }

        /// Registers a thread-safe observer for pointer movement over the displayed plot frame.
        ///
        /// This callback is independent of the built-in hover/tooltip option, which continues
        /// to run when enabled. Host UI state should normally use `cx.subscribe(&plot, ...)`.
        pub fn on_plot_hover<F>(mut self, handler: F) -> Self
        where
            F: Fn(PlotPointerEvent) + Send + Sync + 'static,
        {
            self.pointer_event_handlers.hover = Some(Arc::new(handler));
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
            let pointer_event_handlers = self.pointer_event_handlers;
            Ok(cx.new(move |cx| {
                RuvizPlot::from_options_impl(
                    plot,
                    options,
                    context_menu_action_handler,
                    pointer_event_handlers,
                    cx,
                )
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
        pointer_event_handlers: PlotPointerEventHandlers,
    }

    impl RuvizPlot {
        fn from_options_impl<P>(
            plot: P,
            options: RuvizPlotOptions,
            context_menu_action_handler: Option<ContextMenuActionHandler>,
            pointer_event_handlers: PlotPointerEventHandlers,
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
                pointer_event_handlers,
            }
        }

        #[deprecated(note = "Use ruviz_gpui::plot(...) or ruviz_gpui::plot_builder(...).")]
        pub fn new<P>(plot: P, cx: &mut Context<Self>) -> Self
        where
            P: IntoPlotSession,
        {
            Self::from_options_impl(
                plot,
                RuvizPlotOptions::default(),
                None,
                PlotPointerEventHandlers::default(),
                cx,
            )
        }

        #[deprecated(note = "Use ruviz_gpui::plot_builder(...).build(cx).")]
        pub fn with_options<P>(plot: P, options: RuvizPlotOptions, _cx: &mut Context<Self>) -> Self
        where
            P: IntoPlotSession,
        {
            Self::from_options_impl(
                plot,
                options,
                None,
                PlotPointerEventHandlers::default(),
                _cx,
            )
        }

        pub fn interactive_session(&self) -> &InteractivePlotSession {
            &self.session
        }

        pub fn prepared_plot(&self) -> &PreparedPlot {
            self.session.prepared_plot()
        }

        /// Adds a dynamic annotation to this plot's interactive overlay.
        pub fn add_annotation(
            &mut self,
            annotation: Annotation,
            cx: &mut Context<Self>,
        ) -> Result<AnnotationId> {
            let id = self.session.add_annotation(annotation)?;
            cx.notify();
            Ok(id)
        }

        /// Returns a copy of a dynamic annotation owned by this plot session.
        pub fn annotation(&self, id: AnnotationId) -> Result<Annotation> {
            self.session.annotation(id)
        }

        /// Replaces a dynamic annotation's complete value.
        pub fn update_annotation(
            &mut self,
            id: AnnotationId,
            annotation: Annotation,
            cx: &mut Context<Self>,
        ) -> Result<()> {
            self.session.update_annotation(id, annotation)?;
            cx.notify();
            Ok(())
        }

        /// Removes a dynamic annotation from this plot session.
        pub fn remove_annotation(
            &mut self,
            id: AnnotationId,
            cx: &mut Context<Self>,
        ) -> Result<bool> {
            let removed = self.session.remove_annotation(id)?;
            cx.notify();
            Ok(removed)
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

        /// Converts `plot` into an interactive session and replaces the current one.
        ///
        /// This is a destructive replacement. The previous visible view is
        /// discarded (a session created from a plot starts at its natural/base
        /// bounds), any custom home view is cleared, and pointer, drag, hover,
        /// selection, cached frames, scheduler and in-flight work, and reactive
        /// subscriptions from the previous session are discarded.
        ///
        /// For a replacement that retains only a user-customized visible view,
        /// use [`Self::set_plot_keep_view`]. For data-only changes, prefer
        /// source-backed series and replace their `Observable<Vec<f64>>` values.
        pub fn set_plot<P>(&mut self, plot: P, cx: &mut Context<Self>)
        where
            P: IntoPlotSession,
        {
            self.replace_session(plot.into_plot_session());
            cx.notify();
        }

        /// Replaces the plot while attempting to retain a customized visible view.
        ///
        /// The old visible data bounds are retained only when they materially
        /// differ from the old base bounds. An untouched view therefore keeps the
        /// replacement session's own view, normally the replacement plot's
        /// natural bounds. Restoration goes through the new session's normal
        /// scale validation and normalization, including log-domain checks,
        /// reversed axes, and zoom limits.
        ///
        /// Like [`Self::set_plot`], this still replaces the entire interactive
        /// session. It does not preserve the home view, pointer, drag, hover,
        /// selection, cached frames, scheduler or in-flight work, or reactive
        /// subscriptions.
        ///
        /// Restoration is applied during the replacement session's next render,
        /// after its data bounds have been resolved for the configured time. If the
        /// old bounds are incompatible with the replacement scales, the replacement
        /// session keeps its natural bounds.
        pub fn set_plot_keep_view<P>(&mut self, plot: P, cx: &mut Context<Self>)
        where
            P: IntoPlotSession,
        {
            let old_viewport = self.session.view_bounds_snapshot();
            let should_restore = viewport_bounds_materially_differ(
                old_viewport.visible_bounds,
                old_viewport.base_bounds,
                &old_viewport.x_scale,
                &old_viewport.y_scale,
            );

            self.replace_session(plot.into_plot_session());
            if should_restore {
                self.session
                    .defer_visible_bounds_restore(old_viewport.visible_bounds);
            }
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
            let cursor_data_position = self
                .session
                .screen_to_data(cursor_position_px)?
                .ok_or_else(|| {
                    PlottingError::InvalidInput(
                        "cursor is outside the plotted data area".to_string(),
                    )
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

    fn viewport_axis_bounds_close(
        visible: (f64, f64),
        base: (f64, f64),
        scale: &AxisScale,
    ) -> bool {
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

    impl gpui::EventEmitter<PlotPointerEvent> for RuvizPlot {}

    #[cfg(test)]
    mod tests {
        use super::*;
        use gpui::{Modifiers, MouseButton, TestAppContext};
        use ruviz::plots::heatmap::HeatmapConfig;
        use ruviz::{
            axes::AxisScale,
            data::{BatchUpdate, Observable, signal},
            prelude::Plot,
        };

        fn viewport_rect(x_min: f64, x_max: f64, y_min: f64, y_max: f64) -> ViewportRect {
            ViewportRect {
                min: ViewportPoint::new(x_min, y_min),
                max: ViewportPoint::new(x_max, y_max),
            }
        }

        fn assert_viewport_bounds_close(actual: ViewportRect, expected: ViewportRect) {
            assert!(
                !viewport_bounds_materially_differ(
                    actual,
                    expected,
                    &AxisScale::Linear,
                    &AxisScale::Linear,
                ),
                "actual bounds {actual:?} differed from expected bounds {expected:?}"
            );
        }

        fn render_test_surface(session: &InteractivePlotSession) {
            session
                .render_to_surface(SurfaceTarget {
                    size_px: (320, 240),
                    scale_factor: 1.0,
                    time_seconds: 0.0,
                })
                .expect("test surface should render");
        }

        struct PointerEventSubscriber {
            events: Arc<Mutex<Vec<PlotPointerEvent>>>,
            _subscription: gpui::Subscription,
        }

        fn mapped_test_view(
            plot: Plot,
            options: RuvizPlotOptions,
            frame_size_px: (u32, u32),
            scale_factor: f32,
            component_bounds: Bounds<Pixels>,
        ) -> (TestAppContext, Entity<RuvizPlot>) {
            let session = plot.prepare_interactive();
            let result = session
                .render_to_surface_with_generation(SurfaceTarget {
                    size_px: frame_size_px,
                    scale_factor,
                    time_seconds: options.interaction.time_seconds,
                })
                .expect("mapped test frame should render");
            let base_generation = result.base_generation;
            let frame = result.frame;
            let time_seconds = options.interaction.time_seconds;
            let presentation_mode = options.presentation_mode;
            let cx = TestAppContext::single();
            let view = cx.update(|cx| {
                cx.new(move |cx| {
                    let mut view = RuvizPlot::from_options_impl(
                        session,
                        options,
                        None,
                        PlotPointerEventHandlers::default(),
                        cx,
                    );
                    view.cached_frame = Some(CachedFrame {
                        request: RenderRequest::new(
                            frame_size_px,
                            scale_factor,
                            time_seconds,
                            presentation_mode,
                        ),
                        base_generation,
                        primary: PrimaryFrame::Image(render_image_from_ruviz(
                            frame.layers.base.as_ref().clone(),
                        )),
                        overlay_image: None,
                        stats: frame.stats,
                        target: frame.target,
                    });
                    view.update_layout(component_bounds, frame_size_px);
                    view
                })
            });
            (cx, view)
        }

        fn assert_window_points_close(actual: Point<Pixels>, expected: Point<Pixels>) {
            assert!((f64::from(actual.x) - f64::from(expected.x)).abs() < 1e-4);
            assert!((f64::from(actual.y) - f64::from(expected.y)).abs() < 1e-4);
        }

        #[test]
        fn test_dynamic_annotation_wrappers_delegate_to_session() {
            let cx = TestAppContext::single();
            let plot: Plot = Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]).into();
            let view = cx.update(|cx| {
                cx.new(move |cx| {
                    RuvizPlot::from_options_impl(
                        plot,
                        RuvizPlotOptions::default(),
                        None,
                        PlotPointerEventHandlers::default(),
                        cx,
                    )
                })
            });

            let id = cx.update(|cx| {
                view.update(cx, |view, cx| {
                    view.add_annotation(Annotation::vline(0.25), cx)
                        .expect("wrapper add should succeed")
                })
            });
            cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    assert!(matches!(
                        view.annotation(id).unwrap(),
                        Annotation::VLine { x, .. } if x == 0.25
                    ));
                })
            });
            cx.update(|cx| {
                view.update(cx, |view, cx| {
                    view.update_annotation(id, Annotation::vline(0.75), cx)
                        .expect("wrapper update should succeed");
                    assert!(view.remove_annotation(id, cx).unwrap());
                    assert!(matches!(
                        view.annotation(id),
                        Err(PlottingError::UnknownAnnotationId)
                    ));
                });
            });
        }

        fn synchronize_test_frame(view: &mut RuvizPlot, request: RenderRequest) {
            let frame = render_frame_from_session(view.session.clone(), request.clone())
                .expect("test frame should render");
            view.replace_cached_frame(request, frame);
        }

        #[test]
        fn test_builder_when_coexists_with_gpui_fluent_builder() {
            // Both preludes export a `.when(...)` extension; ruviz's BuilderWhen
            // is implemented only for ruviz builder families so neither call is
            // ambiguous when both preludes are glob-imported.
            use gpui::prelude::*;
            use ruviz::prelude::*;

            let _plot: Plot = Plot::new()
                .when(true, |plot| plot.title("conditional"))
                .line(&[0.0, 1.0], &[0.0, 1.0])
                .when(true, |builder| builder.label("series"))
                .into();
            let _element = gpui::div().when(true, |div| div.flex());
        }

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
                    base_generation: 0,
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
                pointer_event_handlers: PlotPointerEventHandlers::default(),
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
        fn test_fill_backing_dimension_uses_display_scale_without_shrinking() {
            assert_eq!(fill_backing_dimension_px(320, 1.0), 320);
            assert_eq!(fill_backing_dimension_px(320, 2.0), 640);
            assert_eq!(fill_backing_dimension_px(320, 1.5), 480);
            assert_eq!(fill_backing_dimension_px(320, 0.5), 320);
            assert_eq!(fill_backing_dimension_px(0, 2.0), 0);
        }

        #[test]
        fn test_window_data_round_trip_respects_scale_and_presentation_geometry() {
            let component_bounds =
                Bounds::new(point(px(100.0), px(50.0)), size(px(500.0), px(300.0)));
            let plot: Plot = Plot::new()
                .size(4.0, 3.0)
                .scatter(&[5.0], &[10.0])
                .xlim(0.0, 10.0)
                .ylim(0.0, 20.0)
                .into();
            let (cx, view) = mapped_test_view(
                plot,
                RuvizPlotOptions::default(),
                (800, 600),
                2.0,
                component_bounds,
            );

            cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    let layout = view.last_layout.as_ref().expect("layout should exist");
                    assert_eq!(layout.content_bounds.origin, point(px(150.0), px(50.0)));
                    assert_eq!(layout.content_bounds.size, size(px(400.0), px(300.0)));

                    let window_position = view
                        .screen_at(ViewportPoint::new(5.0, 10.0))
                        .expect("data mapping should succeed")
                        .expect("center data should be visible");
                    let data_position = view
                        .data_at(window_position)
                        .expect("window mapping should succeed")
                        .expect("mapped point should be in the plot area");
                    assert!((data_position.x - 5.0).abs() < 1e-6);
                    assert!((data_position.y - 10.0).abs() < 1e-6);
                    assert_window_points_close(
                        view.screen_at(data_position)
                            .expect("inverse mapping should succeed")
                            .expect("round-tripped data should remain visible"),
                        window_position,
                    );
                })
            });

            let plot: Plot = Plot::new()
                .size(4.0, 3.0)
                .line(&[0.0, 1.0], &[0.0, 1.0])
                .into();
            let mut fill_options = RuvizPlotOptions::default();
            fill_options.interaction.image_fit = ImageFit::Fill;
            let (cx, view) =
                mapped_test_view(plot, fill_options, (800, 600), 2.0, component_bounds);
            cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    let layout = view.last_layout.as_ref().expect("layout should exist");
                    assert_eq!(layout.content_bounds, component_bounds);
                    let center = ViewportPoint::new(0.5, 0.5);
                    let window_position = view
                        .screen_at(center)
                        .expect("fill inverse mapping should succeed")
                        .expect("center should be visible");
                    let round_trip = view
                        .data_at(window_position)
                        .expect("fill forward mapping should succeed")
                        .expect("fill center should remain inside the plot area");
                    assert!((round_trip.x - center.x).abs() < 1e-6);
                    assert!((round_trip.y - center.y).abs() < 1e-6);
                })
            });
        }

        #[test]
        fn test_coordinate_mapping_returns_none_before_layout_and_outside_plot() {
            let plot: Plot = Plot::new()
                .line(&[0.0, 1.0], &[0.0, 1.0])
                .xlim(0.0, 1.0)
                .ylim(0.0, 1.0)
                .into();
            let session = plot.prepare_interactive();
            session
                .render_to_surface(SurfaceTarget {
                    size_px: (400, 300),
                    scale_factor: 1.0,
                    time_seconds: 0.0,
                })
                .expect("coordinate test frame should render");
            let cx = TestAppContext::single();
            let view = cx.update(|cx| {
                cx.new(move |cx| {
                    RuvizPlot::from_options_impl(
                        session,
                        RuvizPlotOptions::default(),
                        None,
                        PlotPointerEventHandlers::default(),
                        cx,
                    )
                })
            });

            cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    assert_eq!(view.data_at(point(px(10.0), px(10.0))).unwrap(), None);
                    assert_eq!(view.screen_at(ViewportPoint::new(0.5, 0.5)).unwrap(), None);
                })
            });

            let bounds = Bounds::new(point(px(100.0), px(100.0)), size(px(400.0), px(300.0)));
            cx.update(|app| {
                view.update(app, |view, _| view.update_layout(bounds, (400, 300)));
            });
            cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    assert_eq!(view.data_at(point(px(99.0), px(100.0))).unwrap(), None);
                    let frame_corner = view
                        .viewport_point_to_window_position(ViewportPoint::new(0.0, 0.0))
                        .expect("layout should map frame coordinates");
                    assert_eq!(view.data_at(frame_corner).unwrap(), None);
                    assert_eq!(view.screen_at(ViewportPoint::new(2.0, 2.0)).unwrap(), None);
                })
            });
        }

        #[test]
        fn test_generation_recovery_replaces_base_and_clears_stale_overlay() {
            let component_bounds =
                Bounds::new(point(px(40.0), px(30.0)), size(px(400.0), px(300.0)));
            let plot: Plot = Plot::new()
                .scatter(&[0.5], &[0.5])
                .xlim(0.0, 1.0)
                .ylim(0.0, 1.0)
                .into();
            let (cx, view) = mapped_test_view(
                plot,
                RuvizPlotOptions::default(),
                (400, 300),
                1.0,
                component_bounds,
            );

            let (window_position, installed_generation) = cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    (
                        view.screen_at(ViewportPoint::new(0.5, 0.5))
                            .expect("initial mapping should succeed")
                            .expect("center should be visible"),
                        view.cached_frame
                            .as_ref()
                            .expect("test frame should be installed")
                            .base_generation,
                    )
                })
            });

            cx.update(|app| {
                view.update(app, |view, _| {
                    view.session.apply_input(PlotInputEvent::Hover {
                        position_px: ViewportPoint::new(200.0, 150.0),
                    });
                    synchronize_test_frame(
                        view,
                        RenderRequest::new((400, 300), 1.0, 0.0, PresentationMode::Hybrid)
                            .with_presented_base_generation(Some(installed_generation)),
                    );
                    assert!(
                        view.cached_frame
                            .as_ref()
                            .expect("generation A should remain installed")
                            .overlay_image
                            .is_some(),
                        "generation A must start with a visible overlay"
                    );
                    let installed_generation = view
                        .cached_frame
                        .as_ref()
                        .expect("generation A should remain installed")
                        .base_generation;

                    view.session.apply_input(PlotInputEvent::ClearHover);
                    view.session.apply_input(PlotInputEvent::Pan {
                        delta_px: ViewportPoint::new(30.0, 0.0),
                    });
                    let newer = view
                        .session
                        .render_to_surface_with_generation(SurfaceTarget {
                            size_px: (400, 300),
                            scale_factor: 1.0,
                            time_seconds: 0.0,
                        })
                        .expect("newer core frame should render");
                    assert!(newer.base_generation > installed_generation);
                    assert_eq!(
                        view.cached_frame
                            .as_ref()
                            .expect("old GPUI frame remains installed")
                            .base_generation,
                        installed_generation
                    );
                    assert!(
                        newer.frame.layers.overlay.is_none(),
                        "generation B should authoritatively have no overlay"
                    );
                    assert!(
                        !view.session.dirty_domains().overlay,
                        "the external B render must leave no later overlay dirtiness"
                    );
                    assert!(
                        view.cached_frame
                            .as_ref()
                            .expect("old GPUI frame remains installed")
                            .overlay_image
                            .is_some(),
                        "GPUI should still hold A's overlay before recovery"
                    );

                    assert_eq!(view.data_at(window_position).unwrap(), None);
                    assert_eq!(view.screen_at(ViewportPoint::new(0.5, 0.5)).unwrap(), None);
                    assert_eq!(
                        view.build_plot_pointer_event(
                            PlotPointerEventKind::Hover,
                            None,
                            window_position,
                        )
                        .unwrap(),
                        None
                    );

                    let recovery_request =
                        RenderRequest::new((400, 300), 1.0, 0.0, PresentationMode::Hybrid)
                            .with_presented_base_generation(Some(installed_generation));
                    assert!(!view.cache_is_current(&recovery_request));

                    let recovered =
                        render_frame_from_session(view.session.clone(), recovery_request.clone())
                            .expect("the next GPUI render should recover the newer generation");
                    assert_eq!(recovered.base_generation, newer.base_generation);
                    assert!(
                        recovered.primary.is_some(),
                        "a generation mismatch must carry the matching base image"
                    );
                    assert!(
                        matches!(recovered.overlay, RenderedOverlay::Replace(None)),
                        "generation recovery must authoritatively clear A's overlay"
                    );
                    match recovered
                        .primary
                        .as_ref()
                        .expect("recovery frame must include a primary")
                    {
                        RenderedPrimary::Image(image) => {
                            let recovered_base = render_image_to_ruviz(image)
                                .expect("recovered base image should be readable");
                            assert_eq!(
                                recovered_base.pixels.as_slice(),
                                newer.frame.layers.base.pixels.as_slice(),
                                "generation B must be associated with generation B's base pixels"
                            );
                        }
                        #[cfg(all(feature = "gpu", target_os = "macos"))]
                        RenderedPrimary::Surface(_) => {}
                    }
                    view.replace_cached_frame(recovery_request, recovered);

                    assert_eq!(
                        view.cached_frame
                            .as_ref()
                            .expect("recovered GPUI frame should be installed")
                            .base_generation,
                        newer.base_generation
                    );
                    assert!(
                        view.cached_frame
                            .as_ref()
                            .expect("recovered GPUI frame should be installed")
                            .overlay_image
                            .is_none(),
                        "A's overlay must not be reused with generation B"
                    );
                    assert!(view.data_at(window_position).unwrap().is_some());
                    assert!(
                        view.screen_at(ViewportPoint::new(0.5, 0.5))
                            .unwrap()
                            .is_some()
                    );
                    let installed_request = &view
                        .cached_frame
                        .as_ref()
                        .expect("recovered GPUI frame should be installed")
                        .request;
                    assert!(view.cache_is_current(installed_request));
                });
            });
        }

        #[test]
        fn test_pointer_handlers_ignore_generation_mismatch_and_recover_after_install() {
            let component_bounds =
                Bounds::new(point(px(40.0), px(30.0)), size(px(400.0), px(300.0)));
            let plot: Plot = Plot::new()
                .scatter(&[0.5], &[0.5])
                .xlim(0.0, 1.0)
                .ylim(0.0, 1.0)
                .into();
            let (cx, view) = mapped_test_view(
                plot,
                RuvizPlotOptions::default(),
                (400, 300),
                1.0,
                component_bounds,
            );
            let events = Arc::new(Mutex::new(Vec::<PlotPointerEvent>::new()));

            cx.update(|app| {
                view.update(app, |view, cx| {
                    let events_for_click = Arc::clone(&events);
                    let events_for_hover = Arc::clone(&events);
                    view.pointer_event_handlers = PlotPointerEventHandlers {
                        click: Some(Arc::new(move |event| {
                            events_for_click
                                .lock()
                                .expect("pointer event lock poisoned")
                                .push(event);
                        })),
                        hover: Some(Arc::new(move |event| {
                            events_for_hover
                                .lock()
                                .expect("pointer event lock poisoned")
                                .push(event);
                        })),
                    };

                    let stale_window_position = view
                        .screen_at(ViewportPoint::new(0.5, 0.5))
                        .expect("initial mapping should succeed")
                        .expect("scatter point should be visible");
                    let installed_generation = view
                        .cached_frame
                        .as_ref()
                        .expect("generation A should be installed")
                        .base_generation;
                    view.session.apply_input(PlotInputEvent::Pan {
                        delta_px: ViewportPoint::new(30.0, 0.0),
                    });
                    let newer = view
                        .session
                        .render_to_surface_with_generation(SurfaceTarget {
                            size_px: (400, 300),
                            scale_factor: 1.0,
                            time_seconds: 0.0,
                        })
                        .expect("generation B should render externally");
                    assert!(newer.base_generation > installed_generation);
                    assert!(!view.session.dirty_domains().overlay);

                    view.handle_mouse_move(
                        &MouseMoveEvent {
                            position: stale_window_position,
                            pressed_button: None,
                            modifiers: Modifiers::default(),
                        },
                        cx,
                    )
                    .expect("mismatched hover should be ignored normally");
                    let stale_px = view
                        .local_viewport_point(stale_window_position)
                        .expect("stale point should still map through the installed layout");
                    view.interaction_state.active_drag = ActiveDrag::LeftPan {
                        anchor_px: stale_px,
                        anchor_window: stale_window_position,
                        last_px: stale_px,
                        crossed_threshold: false,
                        click_eligible: true,
                    };
                    view.handle_left_mouse_up(
                        &MouseUpEvent {
                            button: MouseButton::Left,
                            position: stale_window_position,
                            modifiers: Modifiers::default(),
                            click_count: 1,
                        },
                        cx,
                    )
                    .expect("mismatched click should be ignored normally");

                    assert!(
                        !view.session.dirty_domains().overlay,
                        "mismatched hover/click must not alter core hover or selection"
                    );
                    assert_eq!(
                        view.session
                            .viewport_snapshot()
                            .expect("generation B viewport should exist")
                            .selected_count,
                        0
                    );
                    assert!(
                        events
                            .lock()
                            .expect("pointer event lock poisoned")
                            .is_empty()
                    );

                    let recovery_request =
                        RenderRequest::new((400, 300), 1.0, 0.0, PresentationMode::Hybrid)
                            .with_presented_base_generation(Some(installed_generation));
                    let recovered =
                        render_frame_from_session(view.session.clone(), recovery_request.clone())
                            .expect("GPUI recovery should succeed");
                    view.replace_cached_frame(recovery_request, recovered);

                    let current_window_position = view
                        .screen_at(ViewportPoint::new(0.5, 0.5))
                        .expect("recovered mapping should succeed")
                        .expect("scatter point should remain visible");
                    view.handle_mouse_move(
                        &MouseMoveEvent {
                            position: current_window_position,
                            pressed_button: None,
                            modifiers: Modifiers::default(),
                        },
                        cx,
                    )
                    .expect("hover should recover after B installation");
                    let current_px = view
                        .local_viewport_point(current_window_position)
                        .expect("current point should map through the layout");
                    view.interaction_state.active_drag = ActiveDrag::LeftPan {
                        anchor_px: current_px,
                        anchor_window: current_window_position,
                        last_px: current_px,
                        crossed_threshold: false,
                        click_eligible: true,
                    };
                    view.handle_left_mouse_up(
                        &MouseUpEvent {
                            button: MouseButton::Left,
                            position: current_window_position,
                            modifiers: Modifiers::default(),
                            click_count: 1,
                        },
                        cx,
                    )
                    .expect("click should recover after B installation");

                    assert!(view.session.dirty_domains().overlay);
                    assert_eq!(
                        view.session
                            .viewport_snapshot()
                            .expect("recovered viewport should exist")
                            .selected_count,
                        1
                    );
                    let events = events.lock().expect("pointer event lock poisoned");
                    assert_eq!(events.len(), 2);
                    assert_eq!(events[0].kind, PlotPointerEventKind::Hover);
                    assert_eq!(events[1].kind, PlotPointerEventKind::Click);
                });
            });
        }

        #[test]
        fn test_heatmap_hover_payload_coexists_with_builtin_hover_and_coalesces_duplicates() {
            let events = Arc::new(Mutex::new(Vec::<PlotPointerEvent>::new()));
            let events_for_callback = Arc::clone(&events);
            let plot: Plot = Plot::new()
                .heatmap(
                    &vec![vec![1.0, 2.0], vec![3.0, 4.0]],
                    Some(HeatmapConfig::new().colorbar(false)),
                )
                .xlim(0.0, 2.0)
                .ylim(0.0, 2.0)
                .into();
            let session = plot.prepare_interactive();
            let cx = TestAppContext::single();
            let view = cx.update(|cx| {
                cx.new(move |cx| {
                    let mut view = RuvizPlot::from_options_impl(
                        session,
                        RuvizPlotOptions::default(),
                        None,
                        PlotPointerEventHandlers {
                            click: None,
                            hover: Some(Arc::new(move |event| {
                                events_for_callback
                                    .lock()
                                    .expect("hover event lock poisoned")
                                    .push(event);
                            })),
                        },
                        cx,
                    );
                    synchronize_test_frame(
                        &mut view,
                        RenderRequest::new((400, 300), 1.0, 0.0, PresentationMode::Hybrid),
                    );
                    view.update_layout(
                        Bounds::new(Point::default(), size(px(400.0), px(300.0))),
                        (400, 300),
                    );
                    view
                })
            });
            let window_position = cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    view.screen_at(ViewportPoint::new(1.5, 1.5))
                        .expect("heatmap inverse mapping should succeed")
                        .expect("heatmap cell center should be visible")
                })
            });

            cx.update(|app| {
                view.update(app, |view, cx| {
                    view.handle_mouse_move(
                        &MouseMoveEvent {
                            position: window_position,
                            pressed_button: None,
                            modifiers: Modifiers::default(),
                        },
                        cx,
                    )
                    .expect("heatmap hover should succeed");
                    assert!(view.session.dirty_domains().overlay);
                    view.handle_mouse_move(
                        &MouseMoveEvent {
                            position: window_position,
                            pressed_button: None,
                            modifiers: Modifiers::default(),
                        },
                        cx,
                    )
                    .expect("duplicate heatmap hover should succeed");
                });
            });

            let events = events.lock().expect("hover event lock poisoned");
            assert_eq!(events.len(), 1);
            let event = &events[0];
            assert_eq!(event.kind, PlotPointerEventKind::Hover);
            assert_eq!(event.mouse_button, None);
            assert_eq!(event.window_position, window_position);
            assert!(event.viewport.plot_area.contains(event.viewport_position));
            assert!(matches!(
                event.hit,
                HitResult::HeatmapCell {
                    series_index: 0,
                    row: 0,
                    col: 1,
                    value: 2.0,
                    ..
                }
            ));
        }

        #[test]
        fn test_fill_sizing_fits_backing_size_to_figure_aspect() {
            let plot = Plot::new().size(4.0, 3.0);
            let session = plot.prepare_interactive();

            let frame_size =
                frame_size_px_for_policy(&session, &SizingPolicy::Fill, (400, 250), 2.0);

            assert_eq!(frame_size, Some((666, 500)));
        }

        #[test]
        fn test_fixed_pixels_sizing_preserves_exact_requested_size() {
            let plot = Plot::new().size(4.0, 3.0);
            let session = plot.prepare_interactive();

            let frame_size = frame_size_px_for_policy(
                &session,
                &SizingPolicy::FixedPixels {
                    width: 800,
                    height: 500,
                },
                (400, 250),
                2.0,
            );

            assert_eq!(frame_size, Some((800, 500)));
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
        fn test_set_plot_resets_customized_view_and_replacement_state() {
            let cx = TestAppContext::single();
            let old_y = Observable::new(vec![5.0]);
            let replacement_y = Observable::new(vec![1.0, 2.0]);
            let initial_plot: Plot = Plot::new()
                .scatter_source(vec![5.0], old_y.clone())
                .xlim(0.0, 10.0)
                .ylim(0.0, 10.0)
                .into();
            let view = cx.update(|cx| {
                cx.new(|cx| {
                    RuvizPlot::from_options_impl(
                        initial_plot,
                        RuvizPlotOptions::default(),
                        None,
                        PlotPointerEventHandlers::default(),
                        cx,
                    )
                })
            });
            let custom_view = viewport_rect(2.0, 8.0, 1.0, 9.0);
            let replacement_plot: Plot = Plot::new()
                .line_source(vec![100.0, 200.0], replacement_y.clone())
                .xlim(100.0, 200.0)
                .ylim(-5.0, 5.0)
                .into();

            cx.update(|cx| {
                view.update(cx, |view, cx| {
                    view.session
                        .render_to_surface(SurfaceTarget {
                            size_px: (320, 240),
                            scale_factor: 1.0,
                            time_seconds: 0.0,
                        })
                        .expect("initial selection frame should render");
                    let point = view
                        .session
                        .data_to_screen(ViewportPoint::new(5.0, 5.0))
                        .expect("selection conversion should succeed")
                        .expect("selection point should be visible");
                    view.session
                        .apply_input(PlotInputEvent::SelectAt { position_px: point });
                    assert!(view.session.restore_visible_bounds(custom_view));

                    view.interaction_state.last_pointer_px = Some(ViewportPoint::new(4.0, 5.0));
                    view.interaction_state.active_drag = ActiveDrag::LeftPan {
                        anchor_px: ViewportPoint::new(1.0, 1.0),
                        anchor_window: Point::new(px(1.0), px(1.0)),
                        last_px: ViewportPoint::new(2.0, 2.0),
                        crossed_threshold: true,
                        click_eligible: false,
                    };
                    view.interaction_state.home_view_bounds = Some(custom_view);
                    view.last_layout = Some(InteractionLayout {
                        component_bounds: Bounds::default(),
                        content_bounds: Bounds::default(),
                        frame_size_px: (320, 240),
                    });
                    view.cached_frame = Some(CachedFrame {
                        request: RenderRequest::new((320, 240), 1.0, 0.0, PresentationMode::Hybrid),
                        base_generation: 0,
                        primary: PrimaryFrame::Image(render_image_from_ruviz(
                            ruviz::core::plot::Image::new(1, 1, vec![0, 0, 0, 255]),
                        )),
                        overlay_image: None,
                        stats: FrameStats::default(),
                        target: RenderTargetKind::Surface,
                    });
                    view.scheduler.latest_requested_generation = 7;
                    view.scheduler.dropped_frames = 3;

                    view.set_plot(replacement_plot, cx);
                });
            });

            cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    render_test_surface(&view.session);
                    let snapshot = view
                        .session
                        .viewport_snapshot()
                        .expect("replacement viewport should be available");
                    assert_eq!(snapshot.visible_bounds, snapshot.base_bounds);
                    assert_ne!(snapshot.visible_bounds, custom_view);
                    assert_eq!(snapshot.selected_count, 0);
                    assert!(view.cached_frame.is_none());
                    assert_eq!(view.retired_images.len(), 1);
                    assert!(view.last_layout.is_none());
                    assert_eq!(view.interaction_state, InteractionState::default());
                    assert_eq!(view.scheduler.latest_requested_generation, 0);
                    assert!(view.scheduler.in_flight.is_none());
                    assert!(view.scheduler.queued.is_none());
                    assert_eq!(view.scheduler.dropped_frames, 0);
                    assert!(view.in_flight_render.is_none());
                })
            });
            assert_eq!(old_y.subscriber_count(), 0);
            assert_eq!(replacement_y.subscriber_count(), 2);
        }

        #[test]
        fn test_set_plot_keep_view_restores_customized_view_and_resets_non_view_state() {
            let cx = TestAppContext::single();
            let old_y = Observable::new(vec![5.0]);
            let replacement_y = Observable::new(vec![0.0, 100.0]);
            let initial_plot: Plot = Plot::new()
                .scatter_source(vec![5.0], old_y.clone())
                .xlim(0.0, 10.0)
                .ylim(0.0, 10.0)
                .into();
            let view = cx.update(|cx| {
                cx.new(|cx| {
                    RuvizPlot::from_options_impl(
                        initial_plot,
                        RuvizPlotOptions::default(),
                        None,
                        PlotPointerEventHandlers::default(),
                        cx,
                    )
                })
            });
            let custom_view = viewport_rect(2.0, 8.0, 1.0, 9.0);
            let replacement_plot: Plot = Plot::new()
                .line_source(vec![0.0, 100.0], replacement_y.clone())
                .xlim(0.0, 100.0)
                .ylim(0.0, 100.0)
                .into();

            cx.update(|cx| {
                view.update(cx, |view, cx| {
                    render_test_surface(&view.session);
                    let point = view
                        .session
                        .data_to_screen(ViewportPoint::new(5.0, 5.0))
                        .expect("selection conversion should succeed")
                        .expect("selection point should be visible");
                    view.session
                        .apply_input(PlotInputEvent::SelectAt { position_px: point });
                    assert_eq!(view.session.viewport_snapshot().unwrap().selected_count, 1);
                    assert!(view.session.restore_visible_bounds(custom_view));
                    view.interaction_state.last_pointer_px = Some(ViewportPoint::new(4.0, 5.0));
                    view.interaction_state.active_drag = ActiveDrag::RightZoom {
                        anchor_px: ViewportPoint::new(1.0, 1.0),
                        anchor_window: Point::new(px(1.0), px(1.0)),
                        current_px: ViewportPoint::new(2.0, 2.0),
                        crossed_threshold: true,
                        zoom_enabled: true,
                    };
                    view.interaction_state.home_view_bounds = Some(custom_view);
                    view.scheduler.latest_requested_generation = 4;

                    view.set_plot_keep_view(replacement_plot, cx)
                })
            });

            cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    render_test_surface(&view.session);
                    let snapshot = view
                        .session
                        .viewport_snapshot()
                        .expect("replacement viewport should be available");
                    assert_viewport_bounds_close(snapshot.visible_bounds, custom_view);
                    assert_ne!(snapshot.visible_bounds, snapshot.base_bounds);
                    assert_eq!(snapshot.selected_count, 0);
                    assert_eq!(view.interaction_state, InteractionState::default());
                    assert_eq!(view.scheduler.latest_requested_generation, 0);
                    assert!(view.cached_frame.is_none());
                    assert!(view.in_flight_render.is_none());
                })
            });
            assert_eq!(old_y.subscriber_count(), 0);
            assert_eq!(replacement_y.subscriber_count(), 2);
        }

        #[test]
        fn test_set_plot_keep_view_restores_after_temporal_bounds_resolve() {
            let cx = TestAppContext::single();
            let initial_plot: Plot = Plot::new()
                .line(&[0.0, 1.0], &[0.0, 100.0])
                .xlim(0.0, 1.0)
                .ylim(0.0, 100.0)
                .into();
            let view = cx.update(|cx| {
                cx.new(|cx| {
                    RuvizPlot::from_options_impl(
                        initial_plot,
                        RuvizPlotOptions::default(),
                        None,
                        PlotPointerEventHandlers::default(),
                        cx,
                    )
                })
            });
            let custom_view = viewport_rect(0.0, 1.0, 20.0, 80.0);
            let temporal_y = signal::of(|time| {
                if time < 50.0 {
                    vec![-1.0, 1.0]
                } else {
                    vec![0.0, 100.0]
                }
            });
            let replacement_plot: Plot = Plot::new()
                .line_source(vec![0.0, 1.0], temporal_y)
                .xlim(0.0, 1.0)
                .into();

            cx.update(|cx| {
                view.update(cx, |view, cx| {
                    render_test_surface(&view.session);
                    assert!(view.session.restore_visible_bounds(custom_view));
                    view.set_plot_keep_view(replacement_plot, cx);
                })
            });

            cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    view.session
                        .render_to_surface(SurfaceTarget {
                            size_px: (320, 240),
                            scale_factor: 1.0,
                            time_seconds: 100.0,
                        })
                        .expect("temporal replacement frame should render");
                    let snapshot = view.session.viewport_snapshot().unwrap();
                    assert_viewport_bounds_close(snapshot.visible_bounds, custom_view);
                    assert_eq!(snapshot.base_bounds, viewport_rect(0.0, 1.0, 0.0, 100.0));
                })
            });
        }

        #[test]
        fn test_set_plot_keep_view_uses_replacement_bounds_for_untouched_view() {
            let cx = TestAppContext::single();
            let initial_plot: Plot = Plot::new()
                .line(&[0.0, 10.0], &[0.0, 10.0])
                .xlim(0.0, 10.0)
                .ylim(0.0, 10.0)
                .into();
            let view = cx.update(|cx| {
                cx.new(|cx| {
                    RuvizPlot::from_options_impl(
                        initial_plot,
                        RuvizPlotOptions::default(),
                        None,
                        PlotPointerEventHandlers::default(),
                        cx,
                    )
                })
            });
            let replacement_plot: Plot = Plot::new()
                .line(&[100.0, 200.0], &[-5.0, 5.0])
                .xlim(100.0, 200.0)
                .ylim(-5.0, 5.0)
                .into();

            cx.update(|cx| {
                view.update(cx, |view, cx| view.set_plot_keep_view(replacement_plot, cx))
            });

            cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    render_test_surface(&view.session);
                    let snapshot = view.session.viewport_snapshot().unwrap();
                    assert_eq!(snapshot.visible_bounds, snapshot.base_bounds);
                    assert_eq!(snapshot.base_bounds, viewport_rect(100.0, 200.0, -5.0, 5.0));
                })
            });
        }

        #[test]
        fn test_set_plot_keep_view_keeps_already_equal_replacement_bounds() {
            let cx = TestAppContext::single();
            let initial_plot: Plot = Plot::new()
                .line(&[0.0, 10.0], &[0.0, 10.0])
                .xlim(0.0, 10.0)
                .ylim(0.0, 10.0)
                .into();
            let view = cx.update(|cx| {
                cx.new(|cx| {
                    RuvizPlot::from_options_impl(
                        initial_plot,
                        RuvizPlotOptions::default(),
                        None,
                        PlotPointerEventHandlers::default(),
                        cx,
                    )
                })
            });
            let custom_view = viewport_rect(2.0, 8.0, 1.0, 9.0);
            let replacement_plot: Plot = Plot::new()
                .line(&[2.0, 8.0], &[1.0, 9.0])
                .xlim(2.0, 8.0)
                .ylim(1.0, 9.0)
                .into();

            cx.update(|cx| {
                view.update(cx, |view, cx| {
                    assert!(view.session.restore_visible_bounds(custom_view));
                    view.set_plot_keep_view(replacement_plot, cx)
                })
            });

            cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    render_test_surface(&view.session);
                    let snapshot = view.session.viewport_snapshot().unwrap();
                    assert_viewport_bounds_close(snapshot.visible_bounds, custom_view);
                    assert_viewport_bounds_close(snapshot.visible_bounds, snapshot.base_bounds);
                })
            });
        }

        #[test]
        fn test_set_plot_keep_view_detects_customization_on_large_log_range() {
            let cx = TestAppContext::single();
            let initial_plot: Plot = Plot::new()
                .line(&[1.0, 1e15], &[0.0, 1.0])
                .xscale(AxisScale::Log)
                .xlim(1.0, 1e15)
                .ylim(0.0, 1.0)
                .into();
            let view = cx.update(|cx| {
                cx.new(|cx| {
                    RuvizPlot::from_options_impl(
                        initial_plot,
                        RuvizPlotOptions::default(),
                        None,
                        PlotPointerEventHandlers::default(),
                        cx,
                    )
                })
            });
            let custom_view = viewport_rect(10.0, 1e15, 0.0, 1.0);
            let replacement_plot: Plot = Plot::new()
                .line(&[1.0, 1e16], &[0.0, 1.0])
                .xscale(AxisScale::Log)
                .xlim(1.0, 1e16)
                .ylim(0.0, 1.0)
                .into();

            cx.update(|cx| {
                view.update(cx, |view, cx| {
                    assert!(view.session.restore_visible_bounds(custom_view));
                    view.set_plot_keep_view(replacement_plot, cx)
                })
            });

            cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    render_test_surface(&view.session);
                    let snapshot = view.session.view_bounds_snapshot();
                    assert!(!viewport_bounds_materially_differ(
                        snapshot.visible_bounds,
                        custom_view,
                        &AxisScale::Log,
                        &AxisScale::Linear,
                    ));
                })
            });
        }

        #[test]
        fn test_set_plot_keep_view_rejects_old_bounds_outside_new_log_domain() {
            let cx = TestAppContext::single();
            let initial_plot: Plot = Plot::new()
                .line(&[-100.0, 100.0], &[0.0, 1.0])
                .xlim(-100.0, 100.0)
                .ylim(0.0, 1.0)
                .into();
            let view = cx.update(|cx| {
                cx.new(|cx| {
                    RuvizPlot::from_options_impl(
                        initial_plot,
                        RuvizPlotOptions::default(),
                        None,
                        PlotPointerEventHandlers::default(),
                        cx,
                    )
                })
            });
            let invalid_for_log = viewport_rect(-10.0, -1.0, 0.1, 0.9);
            let replacement_plot: Plot = Plot::new()
                .line(&[1.0, 100.0], &[0.0, 1.0])
                .xscale(AxisScale::Log)
                .xlim(1.0, 100.0)
                .ylim(0.0, 1.0)
                .into();

            cx.update(|cx| {
                view.update(cx, |view, cx| {
                    render_test_surface(&view.session);
                    assert!(view.session.restore_visible_bounds(invalid_for_log));
                    view.set_plot_keep_view(replacement_plot, cx)
                })
            });

            cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    render_test_surface(&view.session);
                    let snapshot = view.session.viewport_snapshot().unwrap();
                    assert_eq!(snapshot.visible_bounds, snapshot.base_bounds);
                    assert!(snapshot.visible_bounds.min.x > 0.0);
                })
            });
        }

        #[test]
        fn test_set_plot_keep_view_normalizes_linear_bounds_for_new_log_scale() {
            let cx = TestAppContext::single();
            let initial_plot: Plot = Plot::new()
                .line(&[1.0, 100.0], &[0.0, 1.0])
                .xlim(1.0, 100.0)
                .ylim(0.0, 1.0)
                .into();
            let view = cx.update(|cx| {
                cx.new(|cx| {
                    RuvizPlot::from_options_impl(
                        initial_plot,
                        RuvizPlotOptions::default(),
                        None,
                        PlotPointerEventHandlers::default(),
                        cx,
                    )
                })
            });
            let far_linear_view = viewport_rect(1.0, 1e15, 0.0, 1.0);
            let replacement_plot: Plot = Plot::new()
                .line(&[1.0, 100.0], &[0.0, 1.0])
                .xscale(AxisScale::Log)
                .xlim(1.0, 100.0)
                .ylim(0.0, 1.0)
                .into();

            cx.update(|cx| {
                view.update(cx, |view, cx| {
                    assert!(view.session.restore_visible_bounds(far_linear_view));
                    view.set_plot_keep_view(replacement_plot, cx)
                })
            });

            cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    render_test_surface(&view.session);
                    let snapshot = view.session.view_bounds_snapshot();
                    let normalized_span = AxisScale::Log.normalized_position(
                        snapshot.visible_bounds.max.x,
                        snapshot.base_bounds.min.x,
                        snapshot.base_bounds.max.x,
                    ) - AxisScale::Log.normalized_position(
                        snapshot.visible_bounds.min.x,
                        snapshot.base_bounds.min.x,
                        snapshot.base_bounds.max.x,
                    );
                    assert!(snapshot.visible_bounds.min.x > 0.0);
                    assert!((normalized_span.abs() - 0.01).abs() < 1e-12);
                    assert_ne!(snapshot.visible_bounds, snapshot.base_bounds);
                })
            });
        }

        #[test]
        fn test_set_plot_keep_view_normalizes_outlying_bounds_to_new_zoom_limits() {
            let cx = TestAppContext::single();
            let initial_plot: Plot = Plot::new()
                .line(&[0.0, 100.0], &[0.0, 100.0])
                .xlim(0.0, 100.0)
                .ylim(0.0, 100.0)
                .into();
            let view = cx.update(|cx| {
                cx.new(|cx| {
                    RuvizPlot::from_options_impl(
                        initial_plot,
                        RuvizPlotOptions::default(),
                        None,
                        PlotPointerEventHandlers::default(),
                        cx,
                    )
                })
            });
            let outlying_view = viewport_rect(-450.0, 550.0, -450.0, 550.0);
            let replacement_plot: Plot = Plot::new()
                .line(&[0.0, 1.0], &[0.0, 1.0])
                .xlim(0.0, 1.0)
                .ylim(0.0, 1.0)
                .into();

            cx.update(|cx| {
                view.update(cx, |view, cx| {
                    render_test_surface(&view.session);
                    assert!(view.session.restore_visible_bounds(outlying_view));
                    render_test_surface(&view.session);
                    view.set_plot_keep_view(replacement_plot, cx)
                })
            });

            cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    render_test_surface(&view.session);
                    let snapshot = view.session.viewport_snapshot().unwrap();
                    assert!((snapshot.visible_bounds.width().abs() - 10.0).abs() < 1e-9);
                    assert!((snapshot.visible_bounds.height().abs() - 10.0).abs() < 1e-9);
                    assert_ne!(snapshot.visible_bounds, outlying_view);
                })
            });
        }

        #[test]
        fn test_set_plot_keep_view_restores_bounds_across_opposite_axis_orientation() {
            let cx = TestAppContext::single();
            let initial_plot: Plot = Plot::new()
                .line(&[0.0, 10.0], &[0.0, 20.0])
                .xlim(10.0, 0.0)
                .ylim(20.0, 0.0)
                .into();
            let view = cx.update(|cx| {
                cx.new(|cx| {
                    RuvizPlot::from_options_impl(
                        initial_plot,
                        RuvizPlotOptions::default(),
                        None,
                        PlotPointerEventHandlers::default(),
                        cx,
                    )
                })
            });
            let reversed_view = viewport_rect(8.0, 2.0, 15.0, 5.0);
            let replacement_plot: Plot = Plot::new()
                .line(&[0.0, 100.0], &[0.0, 200.0])
                .xlim(0.0, 100.0)
                .ylim(0.0, 200.0)
                .into();

            cx.update(|cx| {
                view.update(cx, |view, cx| {
                    render_test_surface(&view.session);
                    assert!(view.session.restore_visible_bounds(reversed_view));
                    view.set_plot_keep_view(replacement_plot, cx)
                })
            });

            cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    render_test_surface(&view.session);
                    let snapshot = view.session.viewport_snapshot().unwrap();
                    assert_viewport_bounds_close(snapshot.visible_bounds, reversed_view);
                    assert!(snapshot.visible_bounds.width() < 0.0);
                    assert!(snapshot.visible_bounds.height() < 0.0);
                })
            });
        }

        #[test]
        fn test_observable_vector_replacement_keeps_session_and_customized_view() {
            let cx = TestAppContext::single();
            let x = Observable::new(vec![0.0, 5.0, 10.0]);
            let y = Observable::new(vec![0.0, 5.0, 10.0]);
            let plot: Plot = Plot::new().line_source(x.clone(), y.clone()).into();
            let view = cx.update(|cx| {
                cx.new(|cx| {
                    RuvizPlot::from_options_impl(
                        plot,
                        RuvizPlotOptions::default(),
                        None,
                        PlotPointerEventHandlers::default(),
                        cx,
                    )
                })
            });
            let custom_view = viewport_rect(2.0, 8.0, 1.0, 9.0);
            let old_base = cx.update(|cx| {
                view.update(cx, |view, _| {
                    view.session
                        .render_to_surface(SurfaceTarget {
                            size_px: (320, 240),
                            scale_factor: 1.0,
                            time_seconds: 0.0,
                        })
                        .expect("initial observable frame should render");
                    let snapshot = view.session.viewport_snapshot().unwrap();
                    assert!(view.session.restore_visible_bounds(custom_view));
                    view.interaction_state.last_pointer_px = Some(ViewportPoint::new(3.0, 4.0));
                    snapshot.base_bounds
                })
            });

            {
                let mut batch = BatchUpdate::new();
                batch.add(&x);
                batch.add(&y);
                x.set(vec![0.0, 10.0, 20.0]);
                y.set(vec![-10.0, 0.0, 10.0]);
            }

            cx.update(|cx| {
                view.update(cx, |view, _| {
                    view.session
                        .render_to_surface(SurfaceTarget {
                            size_px: (320, 240),
                            scale_factor: 1.0,
                            time_seconds: 0.0,
                        })
                        .expect("updated observable frame should render");
                })
            });

            cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    let snapshot = view.session.viewport_snapshot().unwrap();
                    assert_ne!(snapshot.base_bounds, old_base);
                    assert_viewport_bounds_close(snapshot.visible_bounds, custom_view);
                    assert_eq!(
                        view.interaction_state.last_pointer_px,
                        Some(ViewportPoint::new(3.0, 4.0))
                    );
                    assert!(!view.subscription.is_empty());
                })
            });
            assert_eq!(x.subscriber_count(), 2);
            assert_eq!(y.subscriber_count(), 2);
        }

        #[test]
        fn test_observable_replacement_keeps_customized_wide_log_view() {
            let cx = TestAppContext::single();
            let x = Observable::new(vec![1.0, 1e15]);
            let y = Observable::new(vec![0.0, 1.0]);
            let plot: Plot = Plot::new()
                .line_source(x.clone(), y.clone())
                .xscale(AxisScale::Log)
                .xlim(1.0, 1e15)
                .ylim(0.0, 1.0)
                .into();
            let view = cx.update(|cx| {
                cx.new(|cx| {
                    RuvizPlot::from_options_impl(
                        plot,
                        RuvizPlotOptions::default(),
                        None,
                        PlotPointerEventHandlers::default(),
                        cx,
                    )
                })
            });
            let custom_view = viewport_rect(10.0, 1e15, 0.0, 1.0);

            cx.update(|cx| {
                view.update(cx, |view, _| {
                    render_test_surface(&view.session);
                    assert!(view.session.restore_visible_bounds(custom_view));
                })
            });

            {
                let mut batch = BatchUpdate::new();
                batch.add(&x);
                batch.add(&y);
                x.set(vec![1.0, 1e16]);
                y.set(vec![0.0, 1.0]);
            }

            cx.update(|cx| {
                view.update(cx, |view, _| render_test_surface(&view.session));
            });

            cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    let snapshot = view.session.viewport_snapshot().unwrap();
                    assert!(!viewport_bounds_materially_differ(
                        snapshot.visible_bounds,
                        custom_view,
                        &AxisScale::Log,
                        &AxisScale::Linear,
                    ));
                    assert!(viewport_bounds_materially_differ(
                        snapshot.visible_bounds,
                        snapshot.base_bounds,
                        &AxisScale::Log,
                        &AxisScale::Linear,
                    ));
                })
            });
        }

        #[test]
        fn test_viewport_customization_comparison_ignores_roundoff() {
            let base = viewport_rect(0.0, 100.0, -50.0, 50.0);
            let roundoff = viewport_rect(5e-11, 100.0 - 5e-11, -50.0, 50.0);
            let customized = viewport_rect(0.01, 99.99, -50.0, 50.0);
            let reversed = viewport_rect(100.0, 0.0, -50.0, 50.0);

            assert!(!viewport_bounds_materially_differ(
                base,
                roundoff,
                &AxisScale::Linear,
                &AxisScale::Linear,
            ));
            assert!(viewport_bounds_materially_differ(
                base,
                customized,
                &AxisScale::Linear,
                &AxisScale::Linear,
            ));
            assert!(viewport_bounds_materially_differ(
                reversed,
                base,
                &AxisScale::Linear,
                &AxisScale::Linear,
            ));
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
                base_generation: 0,
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
                        anchor_window: Point::new(px(1.0), px(1.0)),
                        last_px: ViewportPoint::new(2.0, 2.0),
                        crossed_threshold: true,
                        click_eligible: false,
                    },
                    context_menu: None,
                    home_view_bounds: None,
                    last_hover_event: None,
                },
                context_menu_action_handler: None,
                pointer_event_handlers: PlotPointerEventHandlers::default(),
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
                pointer_event_handlers: PlotPointerEventHandlers::default(),
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
        fn test_click_fires_once_while_drag_and_builtin_interactions_do_not_click() {
            let clicks = Arc::new(Mutex::new(Vec::<PlotPointerEvent>::new()));
            let clicks_for_callback = Arc::clone(&clicks);
            let mut app = TestAppContext::single();
            let (view, cx) = app.add_window_view(|_, cx| {
                let plot: Plot = Plot::new()
                    .scatter(&[0.5], &[0.5])
                    .xlim(0.0, 1.0)
                    .ylim(0.0, 1.0)
                    .into();
                RuvizPlot::from_options_impl(
                    plot,
                    RuvizPlotOptions::default(),
                    None,
                    PlotPointerEventHandlers {
                        click: Some(Arc::new(move |event| {
                            clicks_for_callback
                                .lock()
                                .expect("click event lock poisoned")
                                .push(event);
                        })),
                        hover: None,
                    },
                    cx,
                )
            });

            for _ in 0..4 {
                cx.refresh().expect("rendered frame refresh should succeed");
                cx.run_until_parked();
            }
            cx.update(|_, app| {
                view.update(app, |view, _| {
                    let layout = view
                        .last_layout
                        .clone()
                        .expect("prepaint should resolve test layout");
                    synchronize_test_frame(
                        view,
                        RenderRequest::new(
                            layout.frame_size_px,
                            1.0,
                            0.0,
                            PresentationMode::Hybrid,
                        ),
                    );
                    view.update_layout(layout.component_bounds, layout.frame_size_px);
                });
            });

            let (click_position, drag_start, drag_end, initial_bounds) = cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    let screen_at = |data_position| {
                        view.screen_at(data_position)
                            .expect("test inverse mapping should succeed")
                            .expect("test point should be visible")
                    };
                    (
                        screen_at(ViewportPoint::new(0.5, 0.5)),
                        screen_at(ViewportPoint::new(0.25, 0.25)),
                        screen_at(ViewportPoint::new(0.75, 0.75)),
                        view.session
                            .viewport_snapshot()
                            .expect("initial viewport should exist")
                            .visible_bounds,
                    )
                })
            });

            cx.simulate_mouse_down(click_position, MouseButton::Left, Modifiers::default());
            assert!(clicks.lock().expect("click event lock poisoned").is_empty());
            cx.simulate_mouse_up(click_position, MouseButton::Left, Modifiers::default());
            {
                let clicks = clicks.lock().expect("click event lock poisoned");
                assert_eq!(clicks.len(), 1);
                assert_eq!(clicks[0].kind, PlotPointerEventKind::Click);
                assert_eq!(clicks[0].mouse_button, Some(MouseButton::Left));
                assert!(matches!(clicks[0].hit, HitResult::SeriesPoint { .. }));
            }
            let selected_count = cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    view.session
                        .viewport_snapshot()
                        .expect("selected viewport should exist")
                        .selected_count
                })
            });
            assert_eq!(selected_count, 1);

            cx.simulate_mouse_down(drag_start, MouseButton::Right, Modifiers::default());
            cx.simulate_mouse_move(drag_end, MouseButton::Right, Modifiers::default());
            cx.simulate_mouse_up(drag_end, MouseButton::Right, Modifiers::default());
            assert_eq!(clicks.lock().expect("click event lock poisoned").len(), 1);
            let zoomed_bounds = cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    view.session
                        .viewport_snapshot()
                        .expect("zoomed viewport should exist")
                        .visible_bounds
                })
            });
            assert_ne!(zoomed_bounds, initial_bounds);
            cx.refresh().expect("zoomed frame refresh should succeed");
            cx.run_until_parked();
            cx.update(|_, app| {
                view.update(app, |view, _| {
                    let layout = view
                        .last_layout
                        .clone()
                        .expect("zoomed layout should remain available");
                    synchronize_test_frame(
                        view,
                        RenderRequest::new(
                            layout.frame_size_px,
                            1.0,
                            0.0,
                            PresentationMode::Hybrid,
                        ),
                    );
                });
            });

            cx.simulate_mouse_down(drag_start, MouseButton::Left, Modifiers::default());
            cx.simulate_mouse_move(drag_end, MouseButton::Left, Modifiers::default());
            let pan_crossed_threshold = cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    matches!(
                        view.interaction_state.active_drag,
                        ActiveDrag::LeftPan {
                            crossed_threshold: true,
                            ..
                        }
                    )
                })
            });
            assert!(pan_crossed_threshold);
            cx.simulate_mouse_up(drag_end, MouseButton::Left, Modifiers::default());
            assert_eq!(clicks.lock().expect("click event lock poisoned").len(), 1);

            cx.simulate_mouse_down(drag_start, MouseButton::Left, Modifiers::shift());
            cx.simulate_mouse_move(drag_end, MouseButton::Left, Modifiers::shift());
            let brush_crossed_threshold = cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    matches!(
                        view.interaction_state.active_drag,
                        ActiveDrag::Brush {
                            crossed_threshold: true,
                            ..
                        }
                    )
                })
            });
            assert!(brush_crossed_threshold);
            cx.simulate_mouse_up(drag_end, MouseButton::Left, Modifiers::shift());
            assert_eq!(clicks.lock().expect("click event lock poisoned").len(), 1);

            cx.simulate_mouse_down(click_position, MouseButton::Right, Modifiers::default());
            cx.simulate_mouse_up(click_position, MouseButton::Right, Modifiers::default());
            let context_menu_open = cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    view.interaction_state.context_menu.is_some()
                })
            });
            assert!(context_menu_open);
            assert_eq!(clicks.lock().expect("click event lock poisoned").len(), 1);

            cx.simulate_event(MouseDownEvent {
                button: MouseButton::Left,
                position: click_position,
                modifiers: Modifiers::default(),
                click_count: 2,
                first_mouse: false,
            });
            cx.simulate_event(MouseUpEvent {
                button: MouseButton::Left,
                position: click_position,
                modifiers: Modifiers::default(),
                click_count: 2,
            });
            assert_eq!(clicks.lock().expect("click event lock poisoned").len(), 1);
        }

        #[test]
        fn test_full_double_click_sequence_emits_at_most_one_click_to_gpui_subscriber() {
            let mut app = TestAppContext::single();
            let (view, cx) = app.add_window_view(|_, cx| {
                let plot: Plot = Plot::new()
                    .scatter(&[0.5], &[0.5])
                    .xlim(0.0, 1.0)
                    .ylim(0.0, 1.0)
                    .into();
                RuvizPlot::from_options_impl(
                    plot,
                    RuvizPlotOptions::default(),
                    None,
                    PlotPointerEventHandlers::default(),
                    cx,
                )
            });
            for _ in 0..4 {
                cx.refresh().expect("rendered frame refresh should succeed");
                cx.run_until_parked();
            }
            cx.update(|_, app| {
                view.update(app, |view, _| {
                    let request = view.cached_frame.as_ref().map_or_else(
                        || {
                            let layout = view
                                .last_layout
                                .as_ref()
                                .expect("prepaint should resolve test layout");
                            RenderRequest::new(
                                layout.frame_size_px,
                                1.0,
                                0.0,
                                PresentationMode::Hybrid,
                            )
                        },
                        |frame| frame.request.clone(),
                    );
                    synchronize_test_frame(view, request);
                });
            });

            let events = Arc::new(Mutex::new(Vec::new()));
            let subscriber = cx.update(|_, app| {
                let events = Arc::clone(&events);
                app.new(|cx: &mut Context<PointerEventSubscriber>| {
                    let subscription = cx.subscribe(
                        &view,
                        move |subscriber: &mut PointerEventSubscriber,
                              _,
                              event: &PlotPointerEvent,
                              cx| {
                            subscriber
                                .events
                                .lock()
                                .expect("subscriber event lock poisoned")
                                .push(event.clone());
                            cx.notify();
                        },
                    );
                    PointerEventSubscriber {
                        events,
                        _subscription: subscription,
                    }
                })
            });
            cx.run_until_parked();

            let click_position = cx.read(|app| {
                app.read_entity(&view, |view, _| {
                    view.screen_at(ViewportPoint::new(0.5, 0.5))
                        .expect("click mapping should succeed")
                        .expect("center should be visible")
                })
            });
            for click_count in [1, 2] {
                cx.simulate_event(MouseDownEvent {
                    button: MouseButton::Left,
                    position: click_position,
                    modifiers: Modifiers::default(),
                    click_count,
                    first_mouse: false,
                });
                cx.simulate_event(MouseUpEvent {
                    button: MouseButton::Left,
                    position: click_position,
                    modifiers: Modifiers::default(),
                    click_count,
                });
            }
            cx.run_until_parked();

            cx.read(|app| {
                app.read_entity(&subscriber, |subscriber, _| {
                    let events = subscriber
                        .events
                        .lock()
                        .expect("subscriber event lock poisoned");
                    assert_eq!(events.len(), 1);
                    assert_eq!(events[0].kind, PlotPointerEventKind::Click);
                })
            });
        }

        #[test]
        fn test_pan_threshold_uses_window_pixels_across_presentation_scales() {
            for (frame_size_px, component_size, scale_factor) in [
                ((100, 100), 400.0, 2.0),
                ((800, 800), 200.0, 2.0),
                ((300, 300), 300.0, 1.5),
            ] {
                let component_bounds = Bounds::new(
                    point(px(20.0), px(30.0)),
                    size(px(component_size), px(component_size)),
                );
                let plot: Plot = Plot::new()
                    .line(&[0.0, 1.0], &[0.0, 1.0])
                    .xlim(0.0, 1.0)
                    .ylim(0.0, 1.0)
                    .into();
                let (cx, view) = mapped_test_view(
                    plot,
                    RuvizPlotOptions::default(),
                    frame_size_px,
                    scale_factor,
                    component_bounds,
                );

                let anchor_window = point(
                    component_bounds.origin.x + component_bounds.size.width * 0.5,
                    component_bounds.origin.y + component_bounds.size.height * 0.5,
                );
                cx.update(|app| {
                    view.update(app, |view, cx| {
                        let anchor_px = view
                            .local_viewport_point(anchor_window)
                            .expect("anchor should map into the frame");
                        assert!(!view.session.dirty_domains().data);
                        view.interaction_state.active_drag = ActiveDrag::LeftPan {
                            anchor_px,
                            anchor_window,
                            last_px: anchor_px,
                            crossed_threshold: false,
                            click_eligible: true,
                        };

                        view.handle_mouse_move(
                            &MouseMoveEvent {
                                position: point(anchor_window.x + px(2.0), anchor_window.y),
                                pressed_button: Some(MouseButton::Left),
                                modifiers: Modifiers::default(),
                            },
                            cx,
                        )
                        .expect("sub-threshold move should succeed");
                        assert!(
                            !view.session.dirty_domains().data,
                            "sub-threshold jitter must not pan for frame {frame_size_px:?}"
                        );
                        assert!(matches!(
                            view.interaction_state.active_drag,
                            ActiveDrag::LeftPan {
                                last_px,
                                crossed_threshold: false,
                                ..
                            } if last_px == anchor_px
                        ));

                        view.handle_mouse_move(
                            &MouseMoveEvent {
                                position: point(anchor_window.x + px(4.0), anchor_window.y),
                                pressed_button: Some(MouseButton::Left),
                                modifiers: Modifiers::default(),
                            },
                            cx,
                        )
                        .expect("threshold-crossing move should succeed");
                        assert!(matches!(
                            view.interaction_state.active_drag,
                            ActiveDrag::LeftPan {
                                crossed_threshold: true,
                                last_px,
                                ..
                            } if last_px != anchor_px
                        ));
                        assert!(
                            view.session.dirty_domains().data,
                            "crossing must apply the accumulated pan for frame {frame_size_px:?}"
                        );
                    });
                });
            }
        }

        #[test]
        fn test_scroll_during_primary_press_cancels_click_eligibility() {
            let component_bounds =
                Bounds::new(point(px(10.0), px(10.0)), size(px(400.0), px(300.0)));
            let plot: Plot = Plot::new()
                .scatter(&[0.5], &[0.5])
                .xlim(0.0, 1.0)
                .ylim(0.0, 1.0)
                .into();
            let (cx, view) = mapped_test_view(
                plot,
                RuvizPlotOptions::default(),
                (400, 300),
                1.0,
                component_bounds,
            );
            let anchor_window = point(px(210.0), px(160.0));

            cx.update(|app| {
                view.update(app, |view, cx| {
                    let anchor_px = view
                        .local_viewport_point(anchor_window)
                        .expect("anchor should map into the frame");
                    view.interaction_state.active_drag = ActiveDrag::LeftPan {
                        anchor_px,
                        anchor_window,
                        last_px: anchor_px,
                        crossed_threshold: false,
                        click_eligible: true,
                    };
                    view.handle_scroll_wheel(
                        &ScrollWheelEvent {
                            position: anchor_window,
                            delta: ScrollDelta::Pixels(point(px(0.0), px(-20.0))),
                            ..Default::default()
                        },
                        cx,
                    )
                    .expect("scroll zoom should succeed");
                    assert!(matches!(
                        view.interaction_state.active_drag,
                        ActiveDrag::LeftPan {
                            click_eligible: false,
                            ..
                        }
                    ));

                    view.handle_left_mouse_up(
                        &MouseUpEvent {
                            button: MouseButton::Left,
                            position: anchor_window,
                            modifiers: Modifiers::default(),
                            click_count: 1,
                        },
                        cx,
                    )
                    .expect("release after scroll should succeed");
                    assert_eq!(
                        view.session
                            .viewport_snapshot()
                            .expect("viewport should exist")
                            .selected_count,
                        0
                    );
                });
            });
        }

        #[test]
        fn test_right_mouse_up_far_from_anchor_zooms_without_move_events() {
            let mut app = TestAppContext::single();
            let (view, cx) = app.add_window_view(|_, cx| {
                let plot: Plot = Plot::new()
                    .line(&[0.0, 1.0, 2.0, 3.0], &[0.0, 1.0, 0.5, 1.5])
                    .into();
                RuvizPlot::from_options_impl(
                    plot,
                    RuvizPlotOptions::default(),
                    None,
                    PlotPointerEventHandlers::default(),
                    cx,
                )
            });

            cx.refresh().expect("window refresh should succeed");
            cx.run_until_parked();
            cx.update(|_, app| {
                view.update(app, |view, _| {
                    let layout = view
                        .last_layout
                        .clone()
                        .expect("plot should have a resolved layout");
                    synchronize_test_frame(
                        view,
                        RenderRequest::new(
                            layout.frame_size_px,
                            1.0,
                            0.0,
                            PresentationMode::Hybrid,
                        ),
                    );
                });
            });

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
        fn test_cursor_data_position_uses_core_scaled_conversion() {
            let plot: Plot = Plot::new()
                .line(&[1.0, 100.0], &[-100.0, 100.0])
                .xscale(AxisScale::Log)
                .yscale(AxisScale::symlog(1.0))
                .xlim(1.0, 100.0)
                .ylim(-100.0, 100.0)
                .into();
            let session = plot.prepare_interactive();
            session
                .render_to_surface(SurfaceTarget {
                    size_px: (320, 240),
                    scale_factor: 1.0,
                    time_seconds: 0.0,
                })
                .expect("cursor conversion frame should render");
            let plot_area = session
                .viewport_snapshot()
                .expect("displayed viewport should be available")
                .plot_area;
            let cursor = ViewportPoint::new(
                (plot_area.min.x + plot_area.max.x) * 0.5,
                (plot_area.min.y + plot_area.max.y) * 0.5,
            );

            let data = session
                .screen_to_data(cursor)
                .expect("cursor conversion should succeed")
                .expect("cursor inside plot area should map to data coordinates");
            assert!((data.x - 10.0).abs() < 1e-6);
            assert!(data.y.abs() < 1e-6);
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
                base_generation: 0,
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
                base_generation: 0,
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
