use super::{
    Image, Plot, PlotData, PlotSeries, PreparedPlot, ReactiveSubscription, ResolvedSeries,
    SeriesType,
};
use crate::{
    core::{
        LayoutCalculator, LayoutConfig, MarginConfig, PlotLayout, PlottingError, REFERENCE_DPI,
        Result,
    },
    render::{
        Color, FontConfig, FontFamily, LineStyle, MarkerStyle, TextRenderer,
        skia::{SkiaRenderer, map_data_to_pixels},
    },
};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ViewportPoint {
    pub x: f64,
    pub y: f64,
}

impl ViewportPoint {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ViewportRect {
    pub min: ViewportPoint,
    pub max: ViewportPoint,
}

impl ViewportRect {
    pub fn from_points(a: ViewportPoint, b: ViewportPoint) -> Self {
        Self {
            min: ViewportPoint::new(a.x.min(b.x), a.y.min(b.y)),
            max: ViewportPoint::new(a.x.max(b.x), a.y.max(b.y)),
        }
    }

    pub fn contains(&self, point: ViewportPoint) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    pub fn width(&self) -> f64 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f64 {
        self.max.y - self.min.y
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InteractiveViewportSnapshot {
    pub zoom_level: f64,
    pub pan_offset: ViewportPoint,
    pub base_bounds: ViewportRect,
    pub visible_bounds: ViewportRect,
    pub plot_area: ViewportRect,
    pub selected_count: usize,
}

const MIN_ZOOM_LEVEL: f64 = 0.1;
const MAX_ZOOM_LEVEL: f64 = 100.0;
const VIEWPORT_EPSILON: f64 = 1e-9;

#[cfg(not(target_arch = "wasm32"))]
type FrameTimer = Instant;

#[cfg(target_arch = "wasm32")]
type FrameTimer = ();

#[cfg(not(target_arch = "wasm32"))]
fn start_frame_timer() -> FrameTimer {
    Instant::now()
}

#[cfg(target_arch = "wasm32")]
fn start_frame_timer() -> FrameTimer {}

#[cfg(not(target_arch = "wasm32"))]
fn elapsed_frame_time(start: FrameTimer) -> Duration {
    start.elapsed()
}

#[cfg(target_arch = "wasm32")]
fn elapsed_frame_time(_start: FrameTimer) -> Duration {
    Duration::ZERO
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FramePacing {
    #[default]
    Display,
    FixedHz(u16),
    Manual,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum QualityPolicy {
    Interactive,
    #[default]
    Balanced,
    Publication,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RenderTargetKind {
    #[default]
    Image,
    Surface,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SurfaceCapability {
    #[default]
    Unsupported,
    FallbackImage,
    FastPath,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DirtyDomains {
    pub layout: bool,
    pub data: bool,
    pub overlay: bool,
    pub temporal: bool,
    pub interaction: bool,
}

impl DirtyDomains {
    fn with_all() -> Self {
        Self {
            layout: true,
            data: true,
            overlay: true,
            temporal: true,
            interaction: true,
        }
    }

    fn mark(&mut self, domain: DirtyDomain) {
        match domain {
            DirtyDomain::Layout => self.layout = true,
            DirtyDomain::Data => self.data = true,
            DirtyDomain::Overlay => self.overlay = true,
            DirtyDomain::Temporal => self.temporal = true,
            DirtyDomain::Interaction => self.interaction = true,
        }
    }

    fn clear_base(&mut self) {
        self.layout = false;
        self.data = false;
        self.temporal = false;
        self.interaction = false;
    }

    fn clear_overlay(&mut self) {
        self.overlay = false;
    }

    fn needs_base_render(&self) -> bool {
        self.layout || self.data || self.temporal || self.interaction
    }

    fn needs_overlay_render(&self) -> bool {
        self.overlay || self.interaction
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirtyDomain {
    Layout,
    Data,
    Overlay,
    Temporal,
    Interaction,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrameStats {
    pub frame_count: u64,
    pub last_frame_time: Duration,
    pub average_frame_time: Duration,
    pub current_fps: f64,
    pub target_fps: Option<f64>,
    pub last_target: RenderTargetKind,
    pub last_surface_capability: SurfaceCapability,
}

impl Default for FrameStats {
    fn default() -> Self {
        Self {
            frame_count: 0,
            last_frame_time: Duration::ZERO,
            average_frame_time: Duration::ZERO,
            current_fps: 0.0,
            target_fps: Some(60.0),
            last_target: RenderTargetKind::Image,
            last_surface_capability: SurfaceCapability::Unsupported,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LayerRenderState {
    pub base_dirty: bool,
    pub overlay_dirty: bool,
    pub used_incremental_data: bool,
}

#[derive(Clone, Debug)]
pub struct LayerImages {
    pub base: Arc<Image>,
    pub overlay: Option<Arc<Image>>,
}

#[derive(Clone, Debug)]
pub struct InteractiveFrame {
    pub image: Arc<Image>,
    pub layers: LayerImages,
    pub layer_state: LayerRenderState,
    pub stats: FrameStats,
    pub target: RenderTargetKind,
    pub surface_capability: SurfaceCapability,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ImageTarget {
    pub size_px: (u32, u32),
    pub scale_factor: f32,
    pub time_seconds: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SurfaceTarget {
    pub size_px: (u32, u32),
    pub scale_factor: f32,
    pub time_seconds: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlotInputEvent {
    Resize {
        size_px: (u32, u32),
        scale_factor: f32,
    },
    SetTime {
        time_seconds: f64,
    },
    Zoom {
        factor: f64,
        center_px: ViewportPoint,
    },
    ZoomRect {
        region_px: ViewportRect,
    },
    Pan {
        delta_px: ViewportPoint,
    },
    Hover {
        position_px: ViewportPoint,
    },
    ClearHover,
    ResetView,
    SelectAt {
        position_px: ViewportPoint,
    },
    ClearSelection,
    BrushStart {
        position_px: ViewportPoint,
    },
    BrushMove {
        position_px: ViewportPoint,
    },
    BrushEnd {
        position_px: ViewportPoint,
    },
    ShowTooltip {
        content: String,
        position_px: ViewportPoint,
    },
    HideTooltip,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HitResult {
    None,
    SeriesPoint {
        series_index: usize,
        point_index: usize,
        screen_position: ViewportPoint,
        data_position: ViewportPoint,
        distance_px: f64,
    },
    HeatmapCell {
        series_index: usize,
        row: usize,
        col: usize,
        value: f64,
        screen_rect: ViewportRect,
    },
}

#[derive(Clone, Debug)]
pub struct InteractivePlotSession {
    inner: Arc<InteractivePlotSessionInner>,
}

#[derive(Debug)]
struct InteractivePlotSessionInner {
    prepared: PreparedPlot,
    frame_pacing: Mutex<FramePacing>,
    quality_policy: Mutex<QualityPolicy>,
    prefer_gpu: Mutex<bool>,
    reactive_subscription: Mutex<ReactiveSubscription>,
    state: Mutex<SessionState>,
    dirty: Arc<Mutex<DirtyDomains>>,
    dirty_epoch: Arc<AtomicU64>,
    stats: Mutex<FrameStats>,
    reactive_epoch: Arc<AtomicU64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct DataBounds {
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
}

impl DataBounds {
    fn from_limits(x_min: f64, x_max: f64, y_min: f64, y_max: f64) -> Self {
        Self {
            x_min,
            x_max,
            y_min,
            y_max,
        }
    }

    fn width(&self) -> f64 {
        self.x_max - self.x_min
    }

    fn height(&self) -> f64 {
        self.y_max - self.y_min
    }

    fn width_abs(&self) -> f64 {
        self.width().abs()
    }

    fn height_abs(&self) -> f64 {
        self.height().abs()
    }

    fn center(&self) -> ViewportPoint {
        ViewportPoint::new(
            (self.x_min + self.x_max) * 0.5,
            (self.y_min + self.y_max) * 0.5,
        )
    }

    fn from_points(a: ViewportPoint, b: ViewportPoint) -> Self {
        Self::from_limits(a.x.min(b.x), a.x.max(b.x), a.y.min(b.y), a.y.max(b.y))
    }

    fn from_screen_corners(top_left: ViewportPoint, bottom_right: ViewportPoint) -> Self {
        Self::from_limits(top_left.x, bottom_right.x, bottom_right.y, top_left.y)
    }

    fn from_viewport_rect(bounds: ViewportRect) -> Self {
        Self::from_limits(bounds.min.x, bounds.max.x, bounds.min.y, bounds.max.y)
    }

    fn with_center_size(center: ViewportPoint, width: f64, height: f64) -> Self {
        Self::from_limits(
            center.x - width * 0.5,
            center.x + width * 0.5,
            center.y - height * 0.5,
            center.y + height * 0.5,
        )
    }

    fn translated(self, dx: f64, dy: f64) -> Self {
        Self::from_limits(
            self.x_min + dx,
            self.x_max + dx,
            self.y_min + dy,
            self.y_max + dy,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
struct TooltipState {
    content: String,
    position_px: ViewportPoint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TooltipSource {
    Hover,
    Manual,
}

#[derive(Clone, Debug)]
struct SessionState {
    size_px: (u32, u32),
    scale_factor: f32,
    time_seconds: f64,
    data_bounds: DataBounds,
    base_bounds: DataBounds,
    visible_bounds: DataBounds,
    zoom_level: f64,
    pan_offset: ViewportPoint,
    hovered: Option<HitResult>,
    selected: Vec<HitResult>,
    brush_anchor: Option<ViewportPoint>,
    brushed_region: Option<ViewportRect>,
    tooltip: Option<TooltipState>,
    tooltip_source: Option<TooltipSource>,
    base_cache: Option<InteractiveFrameCache>,
    overlay_cache: Option<OverlayFrameCache>,
    geometry: Option<GeometrySnapshot>,
    last_reactive_epoch: u64,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            size_px: (0, 0),
            scale_factor: 1.0,
            time_seconds: 0.0,
            data_bounds: DataBounds::default(),
            base_bounds: DataBounds::default(),
            visible_bounds: DataBounds::default(),
            zoom_level: 1.0,
            pan_offset: ViewportPoint::default(),
            hovered: None,
            selected: Vec::new(),
            brush_anchor: None,
            brushed_region: None,
            tooltip: None,
            tooltip_source: None,
            base_cache: None,
            overlay_cache: None,
            geometry: None,
            last_reactive_epoch: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct InteractiveFrameKey {
    size_px: (u32, u32),
    scale_bits: u32,
    time_bits: Option<u64>,
    x_min_bits: u64,
    x_max_bits: u64,
    y_min_bits: u64,
    y_max_bits: u64,
    versions: Vec<u64>,
}

#[derive(Clone, Debug)]
struct InteractiveFrameCache {
    key: InteractiveFrameKey,
    image: Arc<Image>,
}

#[derive(Clone, Debug, PartialEq)]
struct OverlayFrameKey {
    size_px: (u32, u32),
    hovered: Option<HitResult>,
    selected: Vec<HitResult>,
    brushed_region: Option<ViewportRect>,
    tooltip: Option<(String, ViewportPoint)>,
}

#[derive(Clone, Debug)]
struct OverlayFrameCache {
    key: OverlayFrameKey,
    image: Option<Arc<Image>>,
}

#[derive(Clone, Debug)]
struct GeometrySnapshot {
    key: InteractiveFrameKey,
    plot_area: tiny_skia::Rect,
    x_bounds: (f64, f64),
    y_bounds: (f64, f64),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct AxisConstraints {
    x_limits: Option<(f64, f64)>,
    y_limits: Option<(f64, f64)>,
}

impl AxisConstraints {
    fn from_plot(plot: &Plot) -> Self {
        Self {
            x_limits: plot.layout.x_limits,
            y_limits: plot.layout.y_limits,
        }
    }

    fn apply(self, data_bounds: DataBounds) -> DataBounds {
        let (mut x_min, mut x_max) = self
            .x_limits
            .unwrap_or((data_bounds.x_min, data_bounds.x_max));
        let (mut y_min, mut y_max) = self
            .y_limits
            .unwrap_or((data_bounds.y_min, data_bounds.y_max));

        if (x_max - x_min).abs() < f64::EPSILON {
            x_min -= 1.0;
            x_max += 1.0;
        }
        if (y_max - y_min).abs() < f64::EPSILON {
            y_min -= 1.0;
            y_max += 1.0;
        }

        DataBounds::from_limits(x_min, x_max, y_min, y_max)
    }
}

#[derive(Clone, Debug)]
struct BaseLayerResult {
    image: Arc<Image>,
    updated: bool,
    used_incremental_data: bool,
}

#[derive(Clone, Debug)]
struct OverlayLayerResult {
    image: Option<Arc<Image>>,
    updated: bool,
}

#[derive(Clone, Debug)]
struct StreamingDrawOp {
    kind: StreamingDrawKind,
    points: Vec<(f64, f64)>,
    previous_point: Option<(f64, f64)>,
    color: Color,
    line_width_px: f32,
    line_style: LineStyle,
    marker_style: MarkerStyle,
    marker_size_px: f32,
    draw_markers: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StreamingDrawKind {
    Line,
    Scatter,
}

impl InteractiveFrameKey {
    fn same_viewport(&self, other: &Self) -> bool {
        self.size_px == other.size_px
            && self.scale_bits == other.scale_bits
            && self.time_bits == other.time_bits
            && self.x_min_bits == other.x_min_bits
            && self.x_max_bits == other.x_max_bits
            && self.y_min_bits == other.y_min_bits
            && self.y_max_bits == other.y_max_bits
    }
}

impl InteractivePlotSession {
    pub(crate) fn new(prepared: PreparedPlot) -> Self {
        let empty_bounds = prepared.plot().empty_cartesian_bounds();
        let initial_data_bounds = compute_data_bounds(prepared.plot(), 0.0).unwrap_or_else(|_| {
            DataBounds::from_limits(
                empty_bounds.0,
                empty_bounds.1,
                empty_bounds.2,
                empty_bounds.3,
            )
        });
        let initial_bounds = AxisConstraints::from_plot(prepared.plot()).apply(initial_data_bounds);
        let dirty = Arc::new(Mutex::new(DirtyDomains::with_all()));
        let dirty_epoch = Arc::new(AtomicU64::new(0));
        let reactive_epoch = Arc::new(AtomicU64::new(0));
        let dirty_for_callback = Arc::clone(&dirty);
        let dirty_epoch_for_callback = Arc::clone(&dirty_epoch);
        let epoch_for_callback = Arc::clone(&reactive_epoch);
        let reactive_subscription = prepared.subscribe_reactive(move || {
            if let Ok(mut domains) = dirty_for_callback.lock() {
                domains.mark(DirtyDomain::Data);
                domains.mark(DirtyDomain::Overlay);
            }
            dirty_epoch_for_callback.fetch_add(1, Ordering::AcqRel);
            epoch_for_callback.fetch_add(1, Ordering::AcqRel);
        });

        let mut state = SessionState {
            data_bounds: initial_data_bounds,
            base_bounds: initial_bounds,
            visible_bounds: initial_bounds,
            ..SessionState::default()
        };
        sync_legacy_viewport_fields(&mut state);

        Self {
            inner: Arc::new(InteractivePlotSessionInner {
                prepared,
                frame_pacing: Mutex::new(FramePacing::Display),
                quality_policy: Mutex::new(QualityPolicy::Balanced),
                prefer_gpu: Mutex::new(false),
                reactive_subscription: Mutex::new(reactive_subscription),
                state: Mutex::new(state),
                dirty,
                dirty_epoch,
                stats: Mutex::new(FrameStats::default()),
                reactive_epoch,
            }),
        }
    }

    pub fn prepared_plot(&self) -> &PreparedPlot {
        &self.inner.prepared
    }

    pub fn subscribe_reactive<F>(&self, callback: F) -> ReactiveSubscription
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.inner.prepared.subscribe_reactive(callback)
    }

    pub fn stats(&self) -> FrameStats {
        self.inner
            .stats
            .lock()
            .expect("InteractivePlotSession stats lock poisoned")
            .clone()
    }

    /// Drop cached geometry and layer images so the next frame rebuilds them.
    pub fn invalidate(&self) {
        self.inner.prepared.invalidate();
        {
            let mut state = self
                .inner
                .state
                .lock()
                .expect("InteractivePlotSession state lock poisoned");
            state.base_cache = None;
            state.overlay_cache = None;
            state.geometry = None;
        }
        self.inner.dirty_epoch.fetch_add(1, Ordering::AcqRel);
        *self
            .inner
            .dirty
            .lock()
            .expect("InteractivePlotSession dirty lock poisoned") = DirtyDomains::with_all();
    }

    pub fn frame_pacing(&self) -> FramePacing {
        *self
            .inner
            .frame_pacing
            .lock()
            .expect("InteractivePlotSession frame pacing lock poisoned")
    }

    pub fn set_frame_pacing(&self, pacing: FramePacing) {
        *self
            .inner
            .frame_pacing
            .lock()
            .expect("InteractivePlotSession frame pacing lock poisoned") = pacing;
        self.mark_dirty(DirtyDomain::Interaction);
    }

    pub fn quality_policy(&self) -> QualityPolicy {
        *self
            .inner
            .quality_policy
            .lock()
            .expect("InteractivePlotSession quality policy lock poisoned")
    }

    pub fn set_quality_policy(&self, quality: QualityPolicy) {
        *self
            .inner
            .quality_policy
            .lock()
            .expect("InteractivePlotSession quality policy lock poisoned") = quality;
        self.mark_dirty(DirtyDomain::Interaction);
    }

    pub fn prefer_gpu(&self) -> bool {
        *self
            .inner
            .prefer_gpu
            .lock()
            .expect("InteractivePlotSession prefer_gpu lock poisoned")
    }

    pub fn set_prefer_gpu(&self, prefer_gpu: bool) {
        *self
            .inner
            .prefer_gpu
            .lock()
            .expect("InteractivePlotSession prefer_gpu lock poisoned") = prefer_gpu;
        self.mark_dirty(DirtyDomain::Interaction);
    }

    pub fn resize(&self, size_px: (u32, u32), scale_factor: f32) {
        let mut state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned");
        let changed = Self::update_resize_state(&mut state, size_px, scale_factor);
        drop(state);
        if changed {
            self.mark_dirty(DirtyDomain::Layout);
        }
    }

    pub fn apply_input(&self, event: PlotInputEvent) {
        let mut state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned");

        match event {
            PlotInputEvent::Resize {
                size_px,
                scale_factor,
            } => {
                let changed = Self::update_resize_state(&mut state, size_px, scale_factor);
                drop(state);
                if changed {
                    self.mark_dirty(DirtyDomain::Layout);
                }
            }
            PlotInputEvent::SetTime { time_seconds } => {
                if state.time_seconds.to_bits() != time_seconds.to_bits() {
                    state.time_seconds = time_seconds;
                    drop(state);
                    self.mark_dirty(DirtyDomain::Temporal);
                }
            }
            PlotInputEvent::Zoom { factor, center_px } => {
                let state_snapshot = state.clone();
                drop(state);
                let current_geometry = match self.geometry_snapshot() {
                    Ok(geometry) => geometry,
                    Err(_) => return,
                };

                if !factor.is_finite() || factor <= 0.0 {
                    return;
                }

                let anchor_before = screen_to_data(
                    state_snapshot.visible_bounds,
                    current_geometry.plot_area,
                    center_px,
                );
                let (normalized_x, normalized_y) =
                    screen_to_normalized(current_geometry.plot_area, center_px);
                let width = clamp_visible_width(
                    state_snapshot.visible_bounds.width() / factor,
                    state_snapshot.base_bounds,
                );
                let height = clamp_visible_height(
                    state_snapshot.visible_bounds.height() / factor,
                    state_snapshot.base_bounds,
                );
                let next_visible = DataBounds::from_limits(
                    anchor_before.x - width * normalized_x,
                    anchor_before.x + width * (1.0 - normalized_x),
                    anchor_before.y - height * (1.0 - normalized_y),
                    anchor_before.y + height * normalized_y,
                );

                if bounds_close(state_snapshot.visible_bounds, next_visible) {
                    return;
                }

                let mut state = self
                    .inner
                    .state
                    .lock()
                    .expect("InteractivePlotSession state lock poisoned");
                set_visible_bounds(&mut state, next_visible);
                drop(state);
                self.mark_dirty(DirtyDomain::Data);
                self.mark_dirty(DirtyDomain::Overlay);
            }
            PlotInputEvent::ZoomRect { region_px } => {
                let state_snapshot = state.clone();
                let had_brush =
                    state.brush_anchor.take().is_some() || state.brushed_region.take().is_some();
                drop(state);

                if region_px.width() <= 1.0 || region_px.height() <= 1.0 {
                    if had_brush {
                        self.mark_dirty(DirtyDomain::Overlay);
                    }
                    return;
                }

                let current_geometry = match self.geometry_snapshot() {
                    Ok(geometry) => geometry,
                    Err(_) => return,
                };
                let data_min = screen_to_data(
                    state_snapshot.visible_bounds,
                    current_geometry.plot_area,
                    region_px.min,
                );
                let data_max = screen_to_data(
                    state_snapshot.visible_bounds,
                    current_geometry.plot_area,
                    region_px.max,
                );
                let next_visible = DataBounds::from_screen_corners(data_min, data_max);
                if next_visible.width_abs() <= VIEWPORT_EPSILON
                    || next_visible.height_abs() <= VIEWPORT_EPSILON
                {
                    if had_brush {
                        self.mark_dirty(DirtyDomain::Overlay);
                    }
                    return;
                }

                let mut state = self
                    .inner
                    .state
                    .lock()
                    .expect("InteractivePlotSession state lock poisoned");
                let viewport_changed = !bounds_close(state.visible_bounds, next_visible);
                state.brush_anchor = None;
                state.brushed_region = None;
                set_visible_bounds(&mut state, next_visible);
                drop(state);
                if viewport_changed {
                    self.mark_dirty(DirtyDomain::Data);
                }
                self.mark_dirty(DirtyDomain::Overlay);
            }
            PlotInputEvent::Pan { delta_px } => {
                let state_snapshot = state.clone();
                drop(state);
                let current_geometry = match self.geometry_snapshot() {
                    Ok(geometry) => geometry,
                    Err(_) => return,
                };
                let width = signed_span_with_min_magnitude(
                    state_snapshot.visible_bounds.width(),
                    f64::EPSILON,
                );
                let height = signed_span_with_min_magnitude(
                    state_snapshot.visible_bounds.height(),
                    f64::EPSILON,
                );
                let size_x = f64::from(current_geometry.plot_area.width()).max(1.0);
                let size_y = f64::from(current_geometry.plot_area.height()).max(1.0);
                let next_visible = state_snapshot.visible_bounds.translated(
                    -(delta_px.x / size_x) * width,
                    (delta_px.y / size_y) * height,
                );
                let mut state = self
                    .inner
                    .state
                    .lock()
                    .expect("InteractivePlotSession state lock poisoned");
                set_visible_bounds(&mut state, next_visible);
                drop(state);
                self.mark_dirty(DirtyDomain::Data);
                self.mark_dirty(DirtyDomain::Overlay);
            }
            PlotInputEvent::Hover { position_px } => {
                drop(state);
                let hit = self.hit_test(position_px);
                let mut state = self
                    .inner
                    .state
                    .lock()
                    .expect("InteractivePlotSession state lock poisoned");
                let next_hovered = match hit {
                    HitResult::None => None,
                    other => Some(other),
                };
                let next_tooltip = next_hovered.as_ref().map(tooltip_from_hit);
                let changed = state.hovered != next_hovered
                    || state.tooltip != next_tooltip
                    || state.tooltip_source != next_hovered.as_ref().map(|_| TooltipSource::Hover);
                state.hovered = next_hovered;
                state.tooltip = next_tooltip;
                state.tooltip_source = state.hovered.as_ref().map(|_| TooltipSource::Hover);
                if changed {
                    drop(state);
                    self.mark_dirty(DirtyDomain::Overlay);
                }
            }
            PlotInputEvent::ClearHover => {
                let hover_changed = state.hovered.take().is_some();
                let tooltip_changed = if state.tooltip_source == Some(TooltipSource::Hover) {
                    state.tooltip_source = None;
                    state.tooltip.take().is_some()
                } else {
                    false
                };
                if hover_changed || tooltip_changed {
                    drop(state);
                    self.mark_dirty(DirtyDomain::Overlay);
                }
            }
            PlotInputEvent::ResetView => {
                state.brush_anchor = None;
                state.brushed_region = None;
                state.visible_bounds = state.base_bounds;
                sync_legacy_viewport_fields(&mut state);
                drop(state);
                self.mark_dirty(DirtyDomain::Data);
                self.mark_dirty(DirtyDomain::Overlay);
            }
            PlotInputEvent::SelectAt { position_px } => {
                drop(state);
                let hit = self.hit_test(position_px);
                let mut state = self
                    .inner
                    .state
                    .lock()
                    .expect("InteractivePlotSession state lock poisoned");
                state.selected.clear();
                if !matches!(hit, HitResult::None) {
                    state.selected.push(hit);
                }
                drop(state);
                self.mark_dirty(DirtyDomain::Overlay);
            }
            PlotInputEvent::ClearSelection => {
                if !state.selected.is_empty() {
                    state.selected.clear();
                    drop(state);
                    self.mark_dirty(DirtyDomain::Overlay);
                }
            }
            PlotInputEvent::BrushStart { position_px } => {
                state.brush_anchor = Some(position_px);
                state.brushed_region = Some(ViewportRect::from_points(position_px, position_px));
                drop(state);
                self.mark_dirty(DirtyDomain::Overlay);
            }
            PlotInputEvent::BrushMove { position_px } => {
                if let Some(anchor) = state.brush_anchor {
                    state.brushed_region = Some(ViewportRect::from_points(anchor, position_px));
                    drop(state);
                    self.mark_dirty(DirtyDomain::Overlay);
                }
            }
            PlotInputEvent::BrushEnd { position_px } => {
                if let Some(anchor) = state.brush_anchor.take() {
                    state.brushed_region = Some(ViewportRect::from_points(anchor, position_px));
                    drop(state);
                    self.mark_dirty(DirtyDomain::Overlay);
                }
            }
            PlotInputEvent::ShowTooltip {
                content,
                position_px,
            } => {
                state.tooltip = Some(TooltipState {
                    content,
                    position_px,
                });
                state.tooltip_source = Some(TooltipSource::Manual);
                drop(state);
                self.mark_dirty(DirtyDomain::Overlay);
            }
            PlotInputEvent::HideTooltip => {
                if state.tooltip.take().is_some() {
                    state.tooltip_source = None;
                    drop(state);
                    self.mark_dirty(DirtyDomain::Overlay);
                }
            }
        }
    }

    pub fn hit_test(&self, position_px: ViewportPoint) -> HitResult {
        let geometry = match self.geometry_snapshot() {
            Ok(geometry) => geometry,
            Err(_) => return HitResult::None,
        };
        let state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned")
            .clone();
        let plot = self
            .inner
            .prepared
            .plot()
            .prepared_frame_plot(state.size_px, state.scale_factor, state.time_seconds)
            .xlim(geometry.x_bounds.0, geometry.x_bounds.1)
            .ylim(geometry.y_bounds.0, geometry.y_bounds.1);
        let snapshot = plot.snapshot_series(state.time_seconds);
        let mut best_hit = HitResult::None;
        let mut best_distance = f64::INFINITY;

        for (series_index, series) in snapshot.iter().enumerate() {
            match &series.series_type {
                SeriesType::Line { x_data, y_data }
                | SeriesType::Scatter { x_data, y_data }
                | SeriesType::ErrorBars { x_data, y_data, .. }
                | SeriesType::ErrorBarsXY { x_data, y_data, .. } => {
                    let x = x_data.resolve(0.0);
                    let y = y_data.resolve(0.0);
                    for (point_index, (&x_val, &y_val)) in x.iter().zip(y.iter()).enumerate() {
                        if !x_val.is_finite() || !y_val.is_finite() {
                            continue;
                        }
                        let (screen_x, screen_y) = map_data_to_pixels(
                            x_val,
                            y_val,
                            geometry.x_bounds.0,
                            geometry.x_bounds.1,
                            geometry.y_bounds.0,
                            geometry.y_bounds.1,
                            geometry.plot_area,
                        );
                        let dx = position_px.x - screen_x as f64;
                        let dy = position_px.y - screen_y as f64;
                        let distance = (dx * dx + dy * dy).sqrt();
                        if distance <= 8.0 && distance < best_distance {
                            best_distance = distance;
                            best_hit = HitResult::SeriesPoint {
                                series_index,
                                point_index,
                                screen_position: ViewportPoint::new(
                                    screen_x as f64,
                                    screen_y as f64,
                                ),
                                data_position: ViewportPoint::new(x_val, y_val),
                                distance_px: distance,
                            };
                        }
                    }
                }
                SeriesType::Heatmap { data } => {
                    let rect = geometry.plot_area;
                    if position_px.x < rect.left() as f64
                        || position_px.x > rect.right() as f64
                        || position_px.y < rect.top() as f64
                        || position_px.y > rect.bottom() as f64
                    {
                        continue;
                    }
                    let data_position = screen_to_data(
                        DataBounds::from_limits(
                            geometry.x_bounds.0,
                            geometry.x_bounds.1,
                            geometry.y_bounds.0,
                            geometry.y_bounds.1,
                        ),
                        rect,
                        position_px,
                    );
                    if data_position.x < data.x_extent.0
                        || data_position.x > data.x_extent.1
                        || data_position.y < data.y_extent.0
                        || data_position.y > data.y_extent.1
                    {
                        continue;
                    }

                    let cell_width =
                        (data.x_extent.1 - data.x_extent.0) / data.n_cols.max(1) as f64;
                    let cell_height =
                        (data.y_extent.1 - data.y_extent.0) / data.n_rows.max(1) as f64;
                    let col = ((data_position.x - data.x_extent.0) / cell_width)
                        .floor()
                        .clamp(0.0, data.n_cols.saturating_sub(1) as f64)
                        as usize;
                    let row = ((data.y_extent.1 - data_position.y) / cell_height)
                        .floor()
                        .clamp(0.0, data.n_rows.saturating_sub(1) as f64)
                        as usize;
                    let value = data.values[row][col];
                    let ((x1, x2), (y1, y2)) = data.cell_data_bounds(row, col);
                    let (sx1, sy1) = map_data_to_pixels(
                        x1,
                        y2,
                        geometry.x_bounds.0,
                        geometry.x_bounds.1,
                        geometry.y_bounds.0,
                        geometry.y_bounds.1,
                        rect,
                    );
                    let (sx2, sy2) = map_data_to_pixels(
                        x2,
                        y1,
                        geometry.x_bounds.0,
                        geometry.x_bounds.1,
                        geometry.y_bounds.0,
                        geometry.y_bounds.1,
                        rect,
                    );
                    best_hit = HitResult::HeatmapCell {
                        series_index,
                        row,
                        col,
                        value,
                        screen_rect: ViewportRect {
                            min: ViewportPoint::new(sx1.min(sx2) as f64, sy1.min(sy2) as f64),
                            max: ViewportPoint::new(sx1.max(sx2) as f64, sy1.max(sy2) as f64),
                        },
                    };
                }
                _ => {}
            }
        }

        best_hit
    }

    pub fn render_to_image(&self, target: ImageTarget) -> Result<InteractiveFrame> {
        self.render_to_target(
            RenderTargetKind::Image,
            target.size_px,
            target.scale_factor,
            target.time_seconds,
        )
    }

    pub fn render_to_surface(&self, target: SurfaceTarget) -> Result<InteractiveFrame> {
        self.render_to_target(
            RenderTargetKind::Surface,
            target.size_px,
            target.scale_factor,
            target.time_seconds,
        )
    }

    pub fn dirty_domains(&self) -> DirtyDomains {
        *self
            .inner
            .dirty
            .lock()
            .expect("InteractivePlotSession dirty lock poisoned")
    }

    /// Returns the current interactive viewport state.
    pub fn viewport_snapshot(&self) -> Result<InteractiveViewportSnapshot> {
        let geometry = self.geometry_snapshot()?;
        let state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned")
            .clone();

        Ok(InteractiveViewportSnapshot {
            zoom_level: state.zoom_level,
            pan_offset: state.pan_offset,
            base_bounds: data_bounds_to_viewport_rect(state.base_bounds),
            visible_bounds: data_bounds_to_viewport_rect(state.visible_bounds),
            plot_area: plot_area_to_viewport_rect(geometry.plot_area),
            selected_count: state.selected.len(),
        })
    }

    /// Restores the visible bounds for the interactive viewport.
    pub fn restore_visible_bounds(&self, bounds: ViewportRect) -> bool {
        let next_visible = DataBounds::from_viewport_rect(bounds);
        if next_visible.width_abs() <= VIEWPORT_EPSILON
            || next_visible.height_abs() <= VIEWPORT_EPSILON
        {
            return false;
        }

        let mut state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned");
        if bounds_close(state.visible_bounds, next_visible) {
            return false;
        }

        set_visible_bounds(&mut state, next_visible);
        drop(state);
        self.mark_dirty(DirtyDomain::Data);
        self.mark_dirty(DirtyDomain::Overlay);
        true
    }

    fn render_to_target(
        &self,
        target: RenderTargetKind,
        size_px: (u32, u32),
        scale_factor: f32,
        time_seconds: f64,
    ) -> Result<InteractiveFrame> {
        let frame_start = start_frame_timer();
        self.resize(size_px, scale_factor);
        self.apply_input(PlotInputEvent::SetTime { time_seconds });

        let reactive_epoch = self.inner.reactive_epoch.load(Ordering::Acquire);
        let mut mark_data_dirty = false;
        {
            let mut state = self
                .inner
                .state
                .lock()
                .expect("InteractivePlotSession state lock poisoned");
            if state.last_reactive_epoch != reactive_epoch {
                state.last_reactive_epoch = reactive_epoch;
                mark_data_dirty = true;
            }
        }
        if mark_data_dirty {
            self.mark_dirty(DirtyDomain::Data);
            self.mark_dirty(DirtyDomain::Overlay);
        }

        let dirty_before_render = self.dirty_domains();
        let dirty_epoch_before_render = self.inner.dirty_epoch.load(Ordering::Acquire);

        if dirty_before_render.layout || dirty_before_render.data || dirty_before_render.temporal {
            let plot = self.inner.prepared.plot();
            let constraints = AxisConstraints::from_plot(plot);
            let mut state = self
                .inner
                .state
                .lock()
                .expect("InteractivePlotSession state lock poisoned");
            let previous_base = state.base_bounds;
            let previous_visible = state.visible_bounds;
            let next_data_bounds =
                compute_data_bounds(plot, time_seconds).unwrap_or(state.data_bounds);
            state.data_bounds = next_data_bounds;
            state.base_bounds = constraints.apply(next_data_bounds);
            if bounds_close(previous_visible, previous_base) {
                state.visible_bounds = state.base_bounds;
            } else {
                state.visible_bounds =
                    normalize_visible_bounds(previous_visible, state.base_bounds);
            }
            sync_legacy_viewport_fields(&mut state);
        }

        let base_key = {
            let state = self
                .inner
                .state
                .lock()
                .expect("InteractivePlotSession state lock poisoned")
                .clone();
            build_frame_key(self.inner.prepared.plot(), &state)
        };

        let geometry = self.ensure_geometry(&base_key)?;
        self.refresh_overlay_state(&geometry, dirty_before_render)?;
        let base_result = self.ensure_base_image(&base_key, &geometry, dirty_before_render)?;
        let overlay_result = self.ensure_overlay_image(size_px, dirty_before_render)?;
        let composed = if target == RenderTargetKind::Image {
            if let Some(overlay_image) = overlay_result.image.as_ref() {
                Arc::new(compose_images(&base_result.image, overlay_image))
            } else {
                Arc::clone(&base_result.image)
            }
        } else {
            Arc::clone(&base_result.image)
        };

        self.clear_dirty_after_render(dirty_epoch_before_render);

        let surface_capability = if target == RenderTargetKind::Surface {
            if base_result.used_incremental_data
                || plot_supports_surface_fast_path(self.inner.prepared.plot())
            {
                SurfaceCapability::FastPath
            } else {
                SurfaceCapability::FallbackImage
            }
        } else {
            SurfaceCapability::Unsupported
        };
        let stats =
            self.record_frame_stats(elapsed_frame_time(frame_start), target, surface_capability);

        Ok(InteractiveFrame {
            image: composed,
            layers: LayerImages {
                base: base_result.image,
                overlay: overlay_result.image,
            },
            layer_state: LayerRenderState {
                base_dirty: base_result.updated,
                overlay_dirty: overlay_result.updated,
                used_incremental_data: base_result.used_incremental_data,
            },
            stats,
            target,
            surface_capability,
        })
    }

    fn ensure_geometry(&self, key: &InteractiveFrameKey) -> Result<GeometrySnapshot> {
        {
            let state = self
                .inner
                .state
                .lock()
                .expect("InteractivePlotSession state lock poisoned");
            if let Some(geometry) = &state.geometry {
                if geometry.key == *key {
                    return Ok(geometry.clone());
                }
            }
        }

        let state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned")
            .clone();
        let geometry =
            geometry_snapshot_for_state(self.inner.prepared.plot(), &state, key.clone())?;

        let mut state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned");
        state.geometry = Some(geometry.clone());
        Ok(geometry)
    }

    fn ensure_base_image(
        &self,
        key: &InteractiveFrameKey,
        geometry: &GeometrySnapshot,
        dirty_before_render: DirtyDomains,
    ) -> Result<BaseLayerResult> {
        {
            let state = self
                .inner
                .state
                .lock()
                .expect("InteractivePlotSession state lock poisoned");
            let dirty = self
                .inner
                .dirty
                .lock()
                .expect("InteractivePlotSession dirty lock poisoned");
            if !dirty.needs_base_render() {
                if let Some(cached) = &state.base_cache {
                    if cached.key == *key {
                        return Ok(BaseLayerResult {
                            image: Arc::clone(&cached.image),
                            updated: false,
                            used_incremental_data: false,
                        });
                    }
                }
            }
        }

        if dirty_before_render.data
            && !dirty_before_render.layout
            && !dirty_before_render.temporal
            && !dirty_before_render.interaction
        {
            if let Some(incremental) = self.try_incremental_stream_render(key, geometry)? {
                return Ok(incremental);
            }
        }

        let state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned")
            .clone();
        let mut plot = self
            .inner
            .prepared
            .plot()
            .prepared_frame_plot(state.size_px, state.scale_factor, state.time_seconds)
            .xlim(geometry.x_bounds.0, geometry.x_bounds.1)
            .ylim(geometry.y_bounds.0, geometry.y_bounds.1);
        if self.prefer_gpu() {
            #[cfg(feature = "gpu")]
            {
                plot = plot.gpu(true);
            }
        }
        let image = plot.render_at(state.time_seconds)?;
        self.inner.prepared.plot().mark_reactive_sources_rendered();
        let image = Arc::new(image);

        let mut state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned");
        state.base_cache = Some(InteractiveFrameCache {
            key: key.clone(),
            image: Arc::clone(&image),
        });
        Ok(BaseLayerResult {
            image,
            updated: true,
            used_incremental_data: false,
        })
    }

    fn ensure_overlay_image(
        &self,
        size_px: (u32, u32),
        dirty_before_render: DirtyDomains,
    ) -> Result<OverlayLayerResult> {
        let state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned")
            .clone();
        let overlay_key = OverlayFrameKey {
            size_px,
            hovered: state.hovered.clone(),
            selected: state.selected.clone(),
            brushed_region: state.brushed_region,
            tooltip: state
                .tooltip
                .as_ref()
                .map(|tooltip| (tooltip.content.clone(), tooltip.position_px)),
        };
        let overlay_is_empty = state.hovered.is_none()
            && state.selected.is_empty()
            && state.brushed_region.is_none()
            && state.tooltip.is_none();

        {
            let state = self
                .inner
                .state
                .lock()
                .expect("InteractivePlotSession state lock poisoned");
            let dirty = self
                .inner
                .dirty
                .lock()
                .expect("InteractivePlotSession dirty lock poisoned");
            if !dirty_before_render.needs_overlay_render() && !dirty.needs_overlay_render() {
                if let Some(cached) = &state.overlay_cache {
                    if cached.key == overlay_key {
                        return Ok(OverlayLayerResult {
                            image: cached.image.clone(),
                            updated: false,
                        });
                    }
                }
            }
        }

        if overlay_is_empty {
            let mut state = self
                .inner
                .state
                .lock()
                .expect("InteractivePlotSession state lock poisoned");
            state.overlay_cache = Some(OverlayFrameCache {
                key: overlay_key,
                image: None,
            });
            return Ok(OverlayLayerResult {
                image: None,
                updated: true,
            });
        }

        let mut pixels = vec![0u8; (size_px.0 * size_px.1 * 4) as usize];
        if let Some(hit) = state.hovered.as_ref() {
            draw_hit(&mut pixels, size_px, hit, Color::new_rgba(255, 165, 0, 180));
        }
        for hit in &state.selected {
            draw_hit(&mut pixels, size_px, hit, Color::new_rgba(255, 0, 0, 180));
        }
        if let Some(region) = state.brushed_region {
            draw_brush_rect(
                &mut pixels,
                size_px,
                region,
                Color::new_rgba(0, 100, 255, 72),
                Color::new_rgba(96, 208, 255, 220),
            );
        }
        if let Some(tooltip) = &state.tooltip {
            draw_tooltip_overlay(&mut pixels, size_px, tooltip);
        }

        let image = Arc::new(Image::new(size_px.0, size_px.1, pixels));
        let mut state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned");
        state.overlay_cache = Some(OverlayFrameCache {
            key: overlay_key,
            image: Some(Arc::clone(&image)),
        });
        Ok(OverlayLayerResult {
            image: Some(image),
            updated: true,
        })
    }

    fn refresh_overlay_state(
        &self,
        geometry: &GeometrySnapshot,
        dirty_before_render: DirtyDomains,
    ) -> Result<()> {
        if !dirty_before_render.layout && !dirty_before_render.data && !dirty_before_render.temporal
        {
            return Ok(());
        }

        let state_snapshot = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned")
            .clone();

        if state_snapshot.hovered.is_none()
            && state_snapshot.selected.is_empty()
            && state_snapshot.tooltip_source != Some(TooltipSource::Hover)
        {
            return Ok(());
        }

        let snapshot = self
            .inner
            .prepared
            .plot()
            .snapshot_series(state_snapshot.time_seconds);
        let refreshed_hovered = state_snapshot
            .hovered
            .as_ref()
            .and_then(|hit| refresh_hit_result(hit, &snapshot, geometry));
        let refreshed_selected = state_snapshot
            .selected
            .iter()
            .filter_map(|hit| refresh_hit_result(hit, &snapshot, geometry))
            .collect::<Vec<_>>();
        let (refreshed_tooltip, refreshed_tooltip_source) =
            if state_snapshot.tooltip_source == Some(TooltipSource::Hover) {
                (
                    refreshed_hovered.as_ref().map(tooltip_from_hit),
                    refreshed_hovered.as_ref().map(|_| TooltipSource::Hover),
                )
            } else {
                (
                    state_snapshot.tooltip.clone(),
                    state_snapshot.tooltip_source,
                )
            };

        let mut state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned");
        state.hovered = refreshed_hovered;
        state.selected = refreshed_selected;
        state.tooltip = refreshed_tooltip;
        state.tooltip_source = refreshed_tooltip_source;
        Ok(())
    }

    fn geometry_snapshot(&self) -> Result<GeometrySnapshot> {
        let state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned")
            .clone();
        let key = build_frame_key(self.inner.prepared.plot(), &state);
        self.ensure_geometry(&key)
    }

    fn mark_dirty(&self, domain: DirtyDomain) {
        self.inner.dirty_epoch.fetch_add(1, Ordering::AcqRel);
        self.inner
            .dirty
            .lock()
            .expect("InteractivePlotSession dirty lock poisoned")
            .mark(domain);
    }

    fn update_resize_state(
        state: &mut SessionState,
        size_px: (u32, u32),
        scale_factor: f32,
    ) -> bool {
        let normalized_size = (size_px.0.max(1), size_px.1.max(1));
        let normalized_scale = sanitize_scale_factor(scale_factor);
        let size_changed = state.size_px != normalized_size;
        let scale_changed = state.scale_factor.to_bits() != normalized_scale.to_bits();

        if size_changed {
            state.size_px = normalized_size;
        }
        if scale_changed {
            state.scale_factor = normalized_scale;
        }

        size_changed || scale_changed
    }

    fn clear_dirty_after_render(&self, dirty_epoch_before_render: u64) {
        if self.inner.dirty_epoch.load(Ordering::Acquire) != dirty_epoch_before_render {
            return;
        }

        let mut dirty = self
            .inner
            .dirty
            .lock()
            .expect("InteractivePlotSession dirty lock poisoned");
        if self.inner.dirty_epoch.load(Ordering::Acquire) != dirty_epoch_before_render {
            return;
        }
        dirty.clear_base();
        dirty.clear_overlay();
    }

    fn try_incremental_stream_render(
        &self,
        key: &InteractiveFrameKey,
        geometry: &GeometrySnapshot,
    ) -> Result<Option<BaseLayerResult>> {
        let (cached, state) = {
            let state = self
                .inner
                .state
                .lock()
                .expect("InteractivePlotSession state lock poisoned");
            let Some(cached) = state.base_cache.clone() else {
                return Ok(None);
            };
            if !cached.key.same_viewport(key) {
                return Ok(None);
            }
            (cached, state.clone())
        };

        let Some(draw_ops) = collect_streaming_draw_ops(
            self.inner.prepared.plot(),
            state.size_px,
            state.scale_factor,
            state.time_seconds,
        )?
        else {
            return Ok(None);
        };

        if draw_ops.is_empty() {
            return Ok(None);
        }

        let image = Arc::new(apply_streaming_draw_ops(
            cached.image.as_ref(),
            geometry,
            &draw_ops,
        )?);
        self.inner.prepared.plot().mark_reactive_sources_rendered();

        let mut state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned");
        state.base_cache = Some(InteractiveFrameCache {
            key: key.clone(),
            image: Arc::clone(&image),
        });

        Ok(Some(BaseLayerResult {
            image,
            updated: true,
            used_incremental_data: true,
        }))
    }

    fn record_frame_stats(
        &self,
        frame_time: Duration,
        target: RenderTargetKind,
        surface_capability: SurfaceCapability,
    ) -> FrameStats {
        let mut stats = self
            .inner
            .stats
            .lock()
            .expect("InteractivePlotSession stats lock poisoned");
        stats.frame_count = stats.frame_count.saturating_add(1);
        stats.last_frame_time = frame_time;
        stats.average_frame_time = if stats.frame_count == 1 {
            frame_time
        } else {
            let total_nanos = stats.average_frame_time.as_nanos() * (stats.frame_count - 1) as u128
                + frame_time.as_nanos();
            Duration::from_nanos((total_nanos / stats.frame_count as u128) as u64)
        };
        stats.current_fps = if frame_time.is_zero() {
            0.0
        } else {
            1.0 / frame_time.as_secs_f64()
        };
        stats.target_fps = match self.frame_pacing() {
            FramePacing::Display => Some(120.0),
            FramePacing::FixedHz(hz) if hz > 0 => Some(hz as f64),
            _ => None,
        };
        stats.last_target = target;
        stats.last_surface_capability = surface_capability;
        stats.clone()
    }

    pub(crate) fn sync_legacy_viewport(&self, zoom_level: f64, pan_x: f64, pan_y: f64) {
        let mut state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned");
        let zoom_level = zoom_level.clamp(MIN_ZOOM_LEVEL, MAX_ZOOM_LEVEL);
        let width = clamp_visible_width(state.base_bounds.width() / zoom_level, state.base_bounds);
        let height =
            clamp_visible_height(state.base_bounds.height() / zoom_level, state.base_bounds);
        let center = ViewportPoint::new(
            state.base_bounds.center().x + pan_x,
            state.base_bounds.center().y + pan_y,
        );
        let next_visible = DataBounds::with_center_size(center, width, height);
        if (state.zoom_level - zoom_level).abs() < f64::EPSILON
            && (state.pan_offset.x - pan_x).abs() < f64::EPSILON
            && (state.pan_offset.y - pan_y).abs() < f64::EPSILON
        {
            return;
        }
        state.visible_bounds = next_visible;
        sync_legacy_viewport_fields(&mut state);
        drop(state);
        self.mark_dirty(DirtyDomain::Data);
        self.mark_dirty(DirtyDomain::Overlay);
    }

    pub(crate) fn sync_legacy_hover(&self, data_position: Option<ViewportPoint>) {
        let mut state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned");
        state.hovered = data_position.map(|point| HitResult::SeriesPoint {
            series_index: 0,
            point_index: 0,
            screen_position: point,
            data_position: point,
            distance_px: 0.0,
        });
        state.tooltip = state.hovered.as_ref().map(tooltip_from_hit);
        state.tooltip_source = state.hovered.as_ref().map(|_| TooltipSource::Hover);
        drop(state);
        self.mark_dirty(DirtyDomain::Overlay);
    }
}

fn compute_data_bounds(plot: &Plot, time: f64) -> Result<DataBounds> {
    let snapshot = plot.snapshot_series(time);
    if snapshot.is_empty() {
        let bounds = plot.empty_cartesian_bounds();
        return Ok(DataBounds::from_limits(
            bounds.0, bounds.1, bounds.2, bounds.3,
        ));
    }

    let (mut x_min, mut x_max, mut y_min, mut y_max) =
        plot.calculate_data_bounds_for_series(&snapshot)?;

    if (x_max - x_min).abs() < f64::EPSILON {
        x_min -= 1.0;
        x_max += 1.0;
    }
    if (y_max - y_min).abs() < f64::EPSILON {
        y_min -= 1.0;
        y_max += 1.0;
    }

    Ok(DataBounds::from_limits(x_min, x_max, y_min, y_max))
}

fn plot_supports_surface_fast_path(plot: &Plot) -> bool {
    !plot.series_mgr.series.is_empty()
        && plot
            .series_mgr
            .series
            .iter()
            .all(|series| series.series_type.supports_interactive_surface_fast_path())
}

fn visible_bounds(state: &SessionState) -> DataBounds {
    state.visible_bounds
}

fn bounds_close(a: DataBounds, b: DataBounds) -> bool {
    (a.x_min - b.x_min).abs() <= VIEWPORT_EPSILON
        && (a.x_max - b.x_max).abs() <= VIEWPORT_EPSILON
        && (a.y_min - b.y_min).abs() <= VIEWPORT_EPSILON
        && (a.y_max - b.y_max).abs() <= VIEWPORT_EPSILON
}

fn clamp_visible_width(width: f64, base_bounds: DataBounds) -> f64 {
    let direction = direction_for_span(width, base_bounds.width());
    let base_width = base_bounds.width_abs().max(VIEWPORT_EPSILON);
    width
        .abs()
        .clamp(base_width / MAX_ZOOM_LEVEL, base_width / MIN_ZOOM_LEVEL)
        * direction
}

fn clamp_visible_height(height: f64, base_bounds: DataBounds) -> f64 {
    let direction = direction_for_span(height, base_bounds.height());
    let base_height = base_bounds.height_abs().max(VIEWPORT_EPSILON);
    height
        .abs()
        .clamp(base_height / MAX_ZOOM_LEVEL, base_height / MIN_ZOOM_LEVEL)
        * direction
}

fn normalize_visible_bounds(bounds: DataBounds, base_bounds: DataBounds) -> DataBounds {
    let center = bounds.center();
    let width = clamp_visible_width(
        signed_span_with_min_magnitude(bounds.width(), VIEWPORT_EPSILON),
        base_bounds,
    );
    let height = clamp_visible_height(
        signed_span_with_min_magnitude(bounds.height(), VIEWPORT_EPSILON),
        base_bounds,
    );
    DataBounds::with_center_size(center, width, height)
}

fn legacy_viewport_metrics(
    base_bounds: DataBounds,
    visible_bounds: DataBounds,
) -> (f64, ViewportPoint) {
    let zoom_x = base_bounds.width_abs() / visible_bounds.width_abs().max(VIEWPORT_EPSILON);
    let zoom_y = base_bounds.height_abs() / visible_bounds.height_abs().max(VIEWPORT_EPSILON);
    let zoom_level = (zoom_x * zoom_y)
        .abs()
        .sqrt()
        .clamp(MIN_ZOOM_LEVEL, MAX_ZOOM_LEVEL);
    (
        zoom_level,
        ViewportPoint::new(
            visible_bounds.center().x - base_bounds.center().x,
            visible_bounds.center().y - base_bounds.center().y,
        ),
    )
}

fn sync_legacy_viewport_fields(state: &mut SessionState) {
    let (zoom_level, pan_offset) = legacy_viewport_metrics(state.base_bounds, state.visible_bounds);
    state.zoom_level = zoom_level;
    state.pan_offset = pan_offset;
}

fn set_visible_bounds(state: &mut SessionState, bounds: DataBounds) {
    state.visible_bounds = normalize_visible_bounds(bounds, state.base_bounds);
    sync_legacy_viewport_fields(state);
}

fn direction_for_span(span: f64, fallback: f64) -> f64 {
    if span < 0.0 || (span == 0.0 && fallback < 0.0) {
        -1.0
    } else {
        1.0
    }
}

fn signed_span_with_min_magnitude(span: f64, min_magnitude: f64) -> f64 {
    let direction = if span < 0.0 { -1.0 } else { 1.0 };
    span.abs().max(min_magnitude) * direction
}

fn data_bounds_to_viewport_rect(bounds: DataBounds) -> ViewportRect {
    ViewportRect {
        min: ViewportPoint::new(bounds.x_min, bounds.y_min),
        max: ViewportPoint::new(bounds.x_max, bounds.y_max),
    }
}

fn plot_area_to_viewport_rect(plot_area: tiny_skia::Rect) -> ViewportRect {
    ViewportRect {
        min: ViewportPoint::new(plot_area.left() as f64, plot_area.top() as f64),
        max: ViewportPoint::new(plot_area.right() as f64, plot_area.bottom() as f64),
    }
}

fn screen_to_data(
    bounds: DataBounds,
    plot_area: tiny_skia::Rect,
    position_px: ViewportPoint,
) -> ViewportPoint {
    let (normalized_x, normalized_y) = screen_to_normalized(plot_area, position_px);
    ViewportPoint::new(
        bounds.x_min + bounds.width() * normalized_x,
        bounds.y_max - bounds.height() * normalized_y,
    )
}

fn screen_to_normalized(plot_area: tiny_skia::Rect, position_px: ViewportPoint) -> (f64, f64) {
    let left = plot_area.left() as f64;
    let right = plot_area.right() as f64;
    let top = plot_area.top() as f64;
    let bottom = plot_area.bottom() as f64;
    let clamped_x = position_px.x.clamp(left, right);
    let clamped_y = position_px.y.clamp(top, bottom);
    let width = f64::from(plot_area.width()).max(1.0);
    let height = f64::from(plot_area.height()).max(1.0);
    (
        ((clamped_x - left) / width).clamp(0.0, 1.0),
        ((clamped_y - top) / height).clamp(0.0, 1.0),
    )
}

fn build_frame_key(plot: &Plot, state: &SessionState) -> InteractiveFrameKey {
    let visible = state.visible_bounds;
    InteractiveFrameKey {
        size_px: state.size_px,
        scale_bits: sanitize_scale_factor(state.scale_factor).to_bits(),
        time_bits: plot
            .has_temporal_sources()
            .then_some(state.time_seconds.to_bits()),
        x_min_bits: visible.x_min.to_bits(),
        x_max_bits: visible.x_max.to_bits(),
        y_min_bits: visible.y_min.to_bits(),
        y_max_bits: visible.y_max.to_bits(),
        versions: plot.collect_reactive_versions(),
    }
}

fn sanitize_scale_factor(scale_factor: f32) -> f32 {
    if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    }
}

struct ComputedSessionLayout {
    plot_area_rect: tiny_skia::Rect,
}

fn geometry_snapshot_for_state(
    plot: &Plot,
    state: &SessionState,
    key: InteractiveFrameKey,
) -> Result<GeometrySnapshot> {
    let visible = state.visible_bounds;
    let layout = compute_plot_layout(
        plot,
        state.size_px,
        state.scale_factor,
        state.time_seconds,
        visible,
    )?;

    Ok(GeometrySnapshot {
        key,
        plot_area: layout.plot_area_rect,
        x_bounds: (visible.x_min, visible.x_max),
        y_bounds: (visible.y_min, visible.y_max),
    })
}

fn refresh_hit_result(
    hit: &HitResult,
    snapshot: &[PlotSeries],
    geometry: &GeometrySnapshot,
) -> Option<HitResult> {
    match hit {
        HitResult::SeriesPoint {
            series_index,
            point_index,
            distance_px,
            ..
        } => {
            let series = snapshot.get(*series_index)?;
            let (x_val, y_val) = match &series.series_type {
                SeriesType::Line { x_data, y_data }
                | SeriesType::Scatter { x_data, y_data }
                | SeriesType::ErrorBars { x_data, y_data, .. }
                | SeriesType::ErrorBarsXY { x_data, y_data, .. } => {
                    let x = x_data.resolve(0.0);
                    let y = y_data.resolve(0.0);
                    (*x.get(*point_index)?, *y.get(*point_index)?)
                }
                _ => return None,
            };
            if !x_val.is_finite() || !y_val.is_finite() {
                return None;
            }
            let (screen_x, screen_y) = map_data_to_pixels(
                x_val,
                y_val,
                geometry.x_bounds.0,
                geometry.x_bounds.1,
                geometry.y_bounds.0,
                geometry.y_bounds.1,
                geometry.plot_area,
            );

            Some(HitResult::SeriesPoint {
                series_index: *series_index,
                point_index: *point_index,
                screen_position: ViewportPoint::new(screen_x as f64, screen_y as f64),
                data_position: ViewportPoint::new(x_val, y_val),
                distance_px: *distance_px,
            })
        }
        HitResult::HeatmapCell {
            series_index,
            row,
            col,
            ..
        } => {
            let series = snapshot.get(*series_index)?;
            let SeriesType::Heatmap { data } = &series.series_type else {
                return None;
            };
            if *row >= data.n_rows || *col >= data.n_cols {
                return None;
            }

            let ((x1, x2), (y1, y2)) = data.cell_data_bounds(*row, *col);
            let (sx1, sy1) = map_data_to_pixels(
                x1,
                y2,
                geometry.x_bounds.0,
                geometry.x_bounds.1,
                geometry.y_bounds.0,
                geometry.y_bounds.1,
                geometry.plot_area,
            );
            let (sx2, sy2) = map_data_to_pixels(
                x2,
                y1,
                geometry.x_bounds.0,
                geometry.x_bounds.1,
                geometry.y_bounds.0,
                geometry.y_bounds.1,
                geometry.plot_area,
            );
            Some(HitResult::HeatmapCell {
                series_index: *series_index,
                row: *row,
                col: *col,
                value: data.values[*row][*col],
                screen_rect: ViewportRect {
                    min: ViewportPoint::new(sx1.min(sx2) as f64, sy1.min(sy2) as f64),
                    max: ViewportPoint::new(sx1.max(sx2) as f64, sy1.max(sy2) as f64),
                },
            })
        }
        HitResult::None => None,
    }
}

fn compute_plot_layout(
    plot: &Plot,
    size_px: (u32, u32),
    scale_factor: f32,
    time_seconds: f64,
    visible: DataBounds,
) -> Result<ComputedSessionLayout> {
    let layout_plot = plot.prepared_frame_plot(size_px, scale_factor, time_seconds);
    let dpi = layout_plot.display.config.figure.dpi;

    let mut renderer = SkiaRenderer::new(size_px.0, size_px.1, layout_plot.display.theme.clone())?;
    renderer.set_text_engine_mode(layout_plot.display.text_engine);
    renderer.set_render_scale(layout_plot.render_scale());

    let content = layout_plot.create_plot_content(visible.y_min, visible.y_max);
    let (x_ticks, y_ticks) = layout_plot.configured_major_ticks(
        visible.x_min,
        visible.x_max,
        visible.y_min,
        visible.y_max,
    );
    let measured_dimensions = layout_plot.measure_layout_text_with_ticks(
        &renderer,
        &content,
        dpi,
        &crate::render::skia::format_tick_labels(&x_ticks),
        &crate::render::skia::format_tick_labels(&y_ticks),
    )?;
    let layout = layout_plot.compute_layout_from_measurements(
        size_px,
        &content,
        dpi,
        measured_dimensions.as_ref(),
    );

    let plot_area_rect = tiny_skia::Rect::from_ltrb(
        layout.plot_area.left,
        layout.plot_area.top,
        layout.plot_area.right,
        layout.plot_area.bottom,
    )
    .ok_or(PlottingError::InvalidData {
        message: "Invalid plot area from layout".to_string(),
        position: None,
    })?;

    Ok(ComputedSessionLayout { plot_area_rect })
}

mod helpers;
#[cfg(test)]
mod tests;
use self::helpers::*;
