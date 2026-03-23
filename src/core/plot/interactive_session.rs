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
    time::{Duration, Instant},
};

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

    fn center(&self) -> ViewportPoint {
        ViewportPoint::new(
            (self.x_min + self.x_max) * 0.5,
            (self.y_min + self.y_max) * 0.5,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
struct TooltipState {
    content: String,
    position_px: ViewportPoint,
}

#[derive(Clone, Debug)]
struct SessionState {
    size_px: (u32, u32),
    scale_factor: f32,
    time_seconds: f64,
    data_bounds: DataBounds,
    base_bounds: DataBounds,
    zoom_level: f64,
    pan_offset: ViewportPoint,
    hovered: Option<HitResult>,
    selected: Vec<HitResult>,
    brush_anchor: Option<ViewportPoint>,
    brushed_region: Option<ViewportRect>,
    tooltip: Option<TooltipState>,
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
            zoom_level: 1.0,
            pan_offset: ViewportPoint::default(),
            hovered: None,
            selected: Vec::new(),
            brush_anchor: None,
            brushed_region: None,
            tooltip: None,
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
    image: Arc<Image>,
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
    image: Arc<Image>,
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
        let initial_data_bounds = compute_data_bounds(prepared.plot(), 0.0)
            .unwrap_or_else(|_| DataBounds::from_limits(0.0, 1.0, 0.0, 1.0));
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

        Self {
            inner: Arc::new(InteractivePlotSessionInner {
                prepared,
                frame_pacing: Mutex::new(FramePacing::Display),
                quality_policy: Mutex::new(QualityPolicy::Balanced),
                prefer_gpu: Mutex::new(false),
                reactive_subscription: Mutex::new(reactive_subscription),
                state: Mutex::new(SessionState {
                    data_bounds: initial_data_bounds,
                    base_bounds: initial_bounds,
                    ..SessionState::default()
                }),
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
                    return;
                }
            }
            PlotInputEvent::SetTime { time_seconds } => {
                if state.time_seconds.to_bits() != time_seconds.to_bits() {
                    state.time_seconds = time_seconds;
                    drop(state);
                    self.mark_dirty(DirtyDomain::Temporal);
                    return;
                }
            }
            PlotInputEvent::Zoom { factor, center_px } => {
                let pre_zoom_visible = visible_bounds(&state);
                let anchor_before = screen_to_data(&state, pre_zoom_visible, center_px);
                let old_zoom = state.zoom_level;
                let new_zoom = (old_zoom * factor).clamp(0.1, 100.0);
                if (new_zoom - old_zoom).abs() > f64::EPSILON {
                    state.zoom_level = new_zoom;
                    let post_zoom_visible = visible_bounds(&state);
                    let anchor_after = screen_to_data(&state, post_zoom_visible, center_px);
                    state.pan_offset.x += anchor_before.x - anchor_after.x;
                    state.pan_offset.y += anchor_before.y - anchor_after.y;
                    drop(state);
                    self.mark_dirty(DirtyDomain::Data);
                    self.mark_dirty(DirtyDomain::Overlay);
                    return;
                }
            }
            PlotInputEvent::Pan { delta_px } => {
                let visible = visible_bounds(&state);
                let width = visible.width().max(f64::EPSILON);
                let height = visible.height().max(f64::EPSILON);
                let size_x = state.size_px.0.max(1) as f64;
                let size_y = state.size_px.1.max(1) as f64;
                state.pan_offset.x -= (delta_px.x / size_x) * width;
                state.pan_offset.y += (delta_px.y / size_y) * height;
                drop(state);
                self.mark_dirty(DirtyDomain::Data);
                self.mark_dirty(DirtyDomain::Overlay);
                return;
            }
            PlotInputEvent::Hover { position_px } => {
                drop(state);
                let hit = self.hit_test(position_px);
                let mut state = self
                    .inner
                    .state
                    .lock()
                    .expect("InteractivePlotSession state lock poisoned");
                let changed = state.hovered != Some(hit.clone());
                state.hovered = match hit {
                    HitResult::None => None,
                    other => Some(other),
                };
                state.tooltip = state.hovered.as_ref().map(tooltip_from_hit);
                if changed {
                    drop(state);
                    self.mark_dirty(DirtyDomain::Overlay);
                }
                return;
            }
            PlotInputEvent::ClearHover => {
                if state.hovered.take().is_some() || state.tooltip.take().is_some() {
                    drop(state);
                    self.mark_dirty(DirtyDomain::Overlay);
                    return;
                }
            }
            PlotInputEvent::ResetView => {
                state.zoom_level = 1.0;
                state.pan_offset = ViewportPoint::default();
                drop(state);
                self.mark_dirty(DirtyDomain::Data);
                self.mark_dirty(DirtyDomain::Overlay);
                return;
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
                return;
            }
            PlotInputEvent::ClearSelection => {
                if !state.selected.is_empty() {
                    state.selected.clear();
                    drop(state);
                    self.mark_dirty(DirtyDomain::Overlay);
                    return;
                }
            }
            PlotInputEvent::BrushStart { position_px } => {
                state.brush_anchor = Some(position_px);
                state.brushed_region = Some(ViewportRect::from_points(position_px, position_px));
                drop(state);
                self.mark_dirty(DirtyDomain::Overlay);
                return;
            }
            PlotInputEvent::BrushMove { position_px } => {
                if let Some(anchor) = state.brush_anchor {
                    state.brushed_region = Some(ViewportRect::from_points(anchor, position_px));
                    drop(state);
                    self.mark_dirty(DirtyDomain::Overlay);
                    return;
                }
            }
            PlotInputEvent::BrushEnd { position_px } => {
                if let Some(anchor) = state.brush_anchor.take() {
                    state.brushed_region = Some(ViewportRect::from_points(anchor, position_px));
                    drop(state);
                    self.mark_dirty(DirtyDomain::Overlay);
                    return;
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
                drop(state);
                self.mark_dirty(DirtyDomain::Overlay);
                return;
            }
            PlotInputEvent::HideTooltip => {
                if state.tooltip.take().is_some() {
                    drop(state);
                    self.mark_dirty(DirtyDomain::Overlay);
                    return;
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
                    let cell_width = rect.width() as f64 / data.n_cols.max(1) as f64;
                    let cell_height = rect.height() as f64 / data.n_rows.max(1) as f64;
                    let col = ((position_px.x - rect.left() as f64) / cell_width)
                        .floor()
                        .clamp(0.0, data.n_cols.saturating_sub(1) as f64)
                        as usize;
                    let row_from_top = ((position_px.y - rect.top() as f64) / cell_height)
                        .floor()
                        .clamp(0.0, data.n_rows.saturating_sub(1) as f64)
                        as usize;
                    let row = data.n_rows.saturating_sub(row_from_top + 1);
                    let value = data.values[row][col];
                    best_hit = HitResult::HeatmapCell {
                        series_index,
                        row,
                        col,
                        value,
                        screen_rect: ViewportRect {
                            min: ViewportPoint::new(
                                rect.left() as f64 + cell_width * col as f64,
                                rect.top() as f64 + cell_height * row_from_top as f64,
                            ),
                            max: ViewportPoint::new(
                                rect.left() as f64 + cell_width * (col as f64 + 1.0),
                                rect.top() as f64 + cell_height * (row_from_top as f64 + 1.0),
                            ),
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

    fn render_to_target(
        &self,
        target: RenderTargetKind,
        size_px: (u32, u32),
        scale_factor: f32,
        time_seconds: f64,
    ) -> Result<InteractiveFrame> {
        let frame_start = Instant::now();
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
            let next_data_bounds =
                compute_data_bounds(plot, time_seconds).unwrap_or(state.data_bounds);
            state.data_bounds = next_data_bounds;
            state.base_bounds = constraints.apply(next_data_bounds);
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
        let base_result = self.ensure_base_image(&base_key, &geometry, dirty_before_render)?;
        let overlay_result = self.ensure_overlay_image(size_px, dirty_before_render)?;
        let composed = if target == RenderTargetKind::Image {
            Arc::new(compose_images(&base_result.image, &overlay_result.image))
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
        let stats = self.record_frame_stats(frame_start.elapsed(), target, surface_capability);

        Ok(InteractiveFrame {
            image: composed,
            layers: LayerImages {
                base: base_result.image,
                overlay: Some(overlay_result.image),
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
        let visible = visible_bounds(&state);
        let layout = compute_plot_layout(
            self.inner.prepared.plot(),
            state.size_px,
            state.scale_factor,
            state.time_seconds,
            visible,
        )?;

        let geometry = GeometrySnapshot {
            key: key.clone(),
            plot_area: layout.plot_area_rect,
            x_bounds: (visible.x_min, visible.x_max),
            y_bounds: (visible.y_min, visible.y_max),
        };

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
                            image: Arc::clone(&cached.image),
                            updated: false,
                        });
                    }
                }
            }
        }

        let mut pixels = vec![0u8; (size_px.0 * size_px.1 * 4) as usize];
        if let Some(hit) = state.hovered.as_ref() {
            draw_hit(&mut pixels, size_px, hit, Color::new_rgba(255, 165, 0, 180));
        }
        for hit in &state.selected {
            draw_hit(&mut pixels, size_px, hit, Color::new_rgba(255, 0, 0, 180));
        }
        if let Some(region) = state.brushed_region {
            draw_rect(
                &mut pixels,
                size_px,
                region,
                Color::new_rgba(0, 100, 255, 72),
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
            image: Arc::clone(&image),
        });
        Ok(OverlayLayerResult {
            image,
            updated: true,
        })
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
        let zoom_level = zoom_level.clamp(0.1, 100.0);
        if (state.zoom_level - zoom_level).abs() < f64::EPSILON
            && (state.pan_offset.x - pan_x).abs() < f64::EPSILON
            && (state.pan_offset.y - pan_y).abs() < f64::EPSILON
        {
            return;
        }
        state.zoom_level = zoom_level;
        state.pan_offset = ViewportPoint::new(pan_x, pan_y);
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
        drop(state);
        self.mark_dirty(DirtyDomain::Overlay);
    }
}

fn compute_data_bounds(plot: &Plot, time: f64) -> Result<DataBounds> {
    let snapshot = plot.snapshot_series(time);
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
    let base = state.base_bounds;
    let zoom = state.zoom_level.max(0.1);
    let width = base.width() / zoom;
    let height = base.height() / zoom;
    let center = ViewportPoint::new(
        base.center().x + state.pan_offset.x,
        base.center().y + state.pan_offset.y,
    );
    DataBounds::from_limits(
        center.x - width * 0.5,
        center.x + width * 0.5,
        center.y - height * 0.5,
        center.y + height * 0.5,
    )
}

fn screen_to_data(
    state: &SessionState,
    bounds: DataBounds,
    position_px: ViewportPoint,
) -> ViewportPoint {
    let width = state.size_px.0.max(1) as f64;
    let height = state.size_px.1.max(1) as f64;
    let normalized_x = position_px.x / width;
    let normalized_y = position_px.y / height;
    ViewportPoint::new(
        bounds.x_min + bounds.width() * normalized_x,
        bounds.y_max - bounds.height() * normalized_y,
    )
}

fn build_frame_key(plot: &Plot, state: &SessionState) -> InteractiveFrameKey {
    let visible = visible_bounds(state);
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
    let measured_dimensions = layout_plot.measure_layout_text(&renderer, &content, dpi)?;
    let measurements = measured_dimensions.as_ref();

    let layout = match &layout_plot.display.config.margins {
        MarginConfig::ContentDriven {
            edge_buffer,
            center_plot,
        } => {
            let layout_config = LayoutConfig {
                edge_buffer_pt: *edge_buffer,
                center_plot: *center_plot,
                ..Default::default()
            };
            LayoutCalculator::new(layout_config).compute(
                size_px,
                &content,
                &layout_plot.display.config.typography,
                &layout_plot.display.config.spacing,
                dpi,
                measurements,
            )
        }
        _ => LayoutCalculator::new(LayoutConfig::default()).compute(
            size_px,
            &content,
            &layout_plot.display.config.typography,
            &layout_plot.display.config.spacing,
            dpi,
            measurements,
        ),
    };

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

fn compose_images(base: &Image, overlay: &Image) -> Image {
    let mut pixels = base.pixels.clone();
    for (dst, src) in pixels
        .chunks_exact_mut(4)
        .zip(overlay.pixels.chunks_exact(4))
    {
        let alpha = src[3] as f32 / 255.0;
        if alpha <= 0.0 {
            continue;
        }
        dst[0] = blend_channel(dst[0], src[0], alpha);
        dst[1] = blend_channel(dst[1], src[1], alpha);
        dst[2] = blend_channel(dst[2], src[2], alpha);
        dst[3] = 255;
    }
    Image::new(base.width, base.height, pixels)
}

fn collect_streaming_draw_ops(
    plot: &Plot,
    size_px: (u32, u32),
    scale_factor: f32,
    time_seconds: f64,
) -> Result<Option<Vec<StreamingDrawOp>>> {
    if plot
        .display
        .title
        .as_ref()
        .is_some_and(|title| title.is_reactive())
        || plot
            .display
            .xlabel
            .as_ref()
            .is_some_and(|label| label.is_reactive())
        || plot
            .display
            .ylabel
            .as_ref()
            .is_some_and(|label| label.is_reactive())
        || plot
            .series_mgr
            .series
            .iter()
            .any(PlotSeries::has_reactive_style_sources)
    {
        return Ok(None);
    }

    let prepared_plot = plot.prepared_frame_plot(size_px, scale_factor, time_seconds);
    let mut draw_ops = Vec::new();
    let mut saw_streaming_update = false;

    for (series_index, series) in plot.series_mgr.series.iter().enumerate() {
        let color = series
            .color
            .unwrap_or_else(|| prepared_plot.display.theme.get_color(series_index));
        let line_width_pt = series
            .line_width
            .unwrap_or(prepared_plot.display.config.lines.data_width);
        let line_width_px = prepared_plot.line_width_px(line_width_pt);
        let dash_pattern = series.line_style.clone().unwrap_or(LineStyle::Solid);
        let marker_style = series.marker_style.unwrap_or(MarkerStyle::Circle);
        let marker_size_px = prepared_plot.line_width_px(series.marker_size.unwrap_or(8.0));

        match &series.series_type {
            SeriesType::Line { x_data, y_data } => {
                let Some(op) = streaming_draw_op(
                    series,
                    x_data,
                    y_data,
                    StreamingDrawKind::Line,
                    color,
                    line_width_px,
                    dash_pattern,
                    marker_style,
                    marker_size_px,
                    time_seconds,
                )?
                else {
                    if series.series_type.is_reactive() {
                        return Ok(None);
                    }
                    continue;
                };
                saw_streaming_update = true;
                draw_ops.push(op);
            }
            SeriesType::Scatter { x_data, y_data } => {
                let Some(op) = streaming_draw_op(
                    series,
                    x_data,
                    y_data,
                    StreamingDrawKind::Scatter,
                    color,
                    line_width_px,
                    dash_pattern,
                    marker_style,
                    marker_size_px,
                    time_seconds,
                )?
                else {
                    if series.series_type.is_reactive() {
                        return Ok(None);
                    }
                    continue;
                };
                saw_streaming_update = true;
                draw_ops.push(op);
            }
            _ if series.series_type.is_reactive() => return Ok(None),
            _ => {}
        }
    }

    Ok(saw_streaming_update.then_some(draw_ops))
}

fn streaming_draw_op(
    series: &PlotSeries,
    x_data: &PlotData,
    y_data: &PlotData,
    kind: StreamingDrawKind,
    color: Color,
    line_width_px: f32,
    line_style: LineStyle,
    marker_style: MarkerStyle,
    marker_size_px: f32,
    time_seconds: f64,
) -> Result<Option<StreamingDrawOp>> {
    if !matches!(x_data, PlotData::Streaming(_)) || !matches!(y_data, PlotData::Streaming(_)) {
        return Ok(None);
    }
    let appended_count = match (
        x_data.streaming_render_state(),
        y_data.streaming_render_state(),
    ) {
        (
            Some(crate::data::StreamingRenderState::AppendOnly {
                visible_appended: x_count,
            }),
            Some(crate::data::StreamingRenderState::AppendOnly {
                visible_appended: y_count,
            }),
        ) => x_count.min(y_count),
        _ => return Ok(None),
    };
    if appended_count == 0 {
        return Ok(None);
    }

    let x_values = x_data.resolve(time_seconds);
    let y_values = y_data.resolve(time_seconds);
    let len = x_values.len().min(y_values.len());
    if len == 0 || appended_count > len {
        return Ok(None);
    }

    let split_index = len - appended_count;
    let previous_point = if split_index > 0 {
        Some((x_values[split_index - 1], y_values[split_index - 1]))
    } else {
        None
    };
    let mut points = Vec::with_capacity(appended_count);
    for (&x, &y) in x_values[split_index..len]
        .iter()
        .zip(&y_values[split_index..len])
    {
        if x.is_finite() && y.is_finite() {
            points.push((x, y));
        }
    }

    if points.is_empty() {
        return Ok(None);
    }

    let _ = series;
    Ok(Some(StreamingDrawOp {
        kind,
        points,
        previous_point,
        color,
        line_width_px,
        line_style,
        marker_style,
        marker_size_px,
        draw_markers: kind == StreamingDrawKind::Scatter || series.marker_style.is_some(),
    }))
}

fn apply_streaming_draw_ops(
    base: &Image,
    geometry: &GeometrySnapshot,
    draw_ops: &[StreamingDrawOp],
) -> Result<Image> {
    let size =
        tiny_skia::IntSize::from_wh(base.width, base.height).ok_or(PlottingError::InvalidData {
            message: "Invalid frame size for incremental streaming render".to_string(),
            position: None,
        })?;
    let mut pixmap = tiny_skia::Pixmap::from_vec(base.pixels.clone(), size).ok_or(
        PlottingError::RenderError("Failed to create incremental streaming pixmap".to_string()),
    )?;

    for op in draw_ops {
        let mut mapped_points: Vec<(f32, f32)> = Vec::with_capacity(op.points.len());
        for &(x, y) in &op.points {
            let (px, py) = map_data_to_pixels(
                x,
                y,
                geometry.x_bounds.0,
                geometry.x_bounds.1,
                geometry.y_bounds.0,
                geometry.y_bounds.1,
                geometry.plot_area,
            );
            mapped_points.push((px, py));
        }

        if op.kind == StreamingDrawKind::Line {
            draw_incremental_polyline(
                &mut pixmap,
                geometry,
                op.previous_point,
                &mapped_points,
                op.color,
                op.line_width_px,
                &op.line_style,
            )?;
        }

        if op.draw_markers {
            for &(px, py) in &mapped_points {
                draw_incremental_marker(
                    &mut pixmap,
                    px,
                    py,
                    op.marker_size_px,
                    op.marker_style,
                    op.color,
                )?;
            }
        }
    }

    Ok(Image::new(base.width, base.height, pixmap.take()))
}

fn draw_incremental_polyline(
    pixmap: &mut tiny_skia::Pixmap,
    geometry: &GeometrySnapshot,
    previous_point: Option<(f64, f64)>,
    points: &[(f32, f32)],
    color: Color,
    line_width_px: f32,
    line_style: &LineStyle,
) -> Result<()> {
    let mut path = tiny_skia::PathBuilder::new();

    if let Some((x, y)) = previous_point {
        let (px, py) = map_data_to_pixels(
            x,
            y,
            geometry.x_bounds.0,
            geometry.x_bounds.1,
            geometry.y_bounds.0,
            geometry.y_bounds.1,
            geometry.plot_area,
        );
        path.move_to(px, py);
    } else if let Some(&(px, py)) = points.first() {
        path.move_to(px, py);
    } else {
        return Ok(());
    }

    for &(px, py) in points {
        path.line_to(px, py);
    }

    let Some(path) = path.finish() else {
        return Ok(());
    };

    let mut paint = tiny_skia::Paint::default();
    paint.set_color(color.to_tiny_skia_color());
    paint.anti_alias = true;

    let mut stroke = tiny_skia::Stroke {
        width: line_width_px.max(1.0),
        ..tiny_skia::Stroke::default()
    };
    if let Some(pattern) = line_style.to_dash_array() {
        stroke.dash = tiny_skia::StrokeDash::new(pattern, 0.0);
    }

    pixmap.stroke_path(
        &path,
        &paint,
        &stroke,
        tiny_skia::Transform::identity(),
        None,
    );

    Ok(())
}

fn draw_incremental_marker(
    pixmap: &mut tiny_skia::Pixmap,
    x: f32,
    y: f32,
    size: f32,
    style: MarkerStyle,
    color: Color,
) -> Result<()> {
    let radius = size * 0.5;
    let mut paint = tiny_skia::Paint::default();
    paint.set_color(color.to_tiny_skia_color());
    paint.anti_alias = true;

    match style {
        MarkerStyle::Circle | MarkerStyle::CircleOpen => {
            let circle = tiny_skia::PathBuilder::from_circle(x, y, radius).ok_or(
                PlottingError::RenderError("Failed to create circle marker path".to_string()),
            )?;
            if style.is_filled() {
                pixmap.fill_path(
                    &circle,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    tiny_skia::Transform::identity(),
                    None,
                );
            } else {
                let stroke = tiny_skia::Stroke {
                    width: (size * 0.15).max(1.0),
                    ..tiny_skia::Stroke::default()
                };
                pixmap.stroke_path(
                    &circle,
                    &paint,
                    &stroke,
                    tiny_skia::Transform::identity(),
                    None,
                );
            }
        }
        MarkerStyle::Square | MarkerStyle::SquareOpen => {
            let rect = tiny_skia::Rect::from_ltrb(x - radius, y - radius, x + radius, y + radius)
                .ok_or(PlottingError::RenderError(
                "Failed to create square marker path".to_string(),
            ))?;
            let path = tiny_skia::PathBuilder::from_rect(rect);
            if style.is_filled() {
                pixmap.fill_path(
                    &path,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    tiny_skia::Transform::identity(),
                    None,
                );
            } else {
                let stroke = tiny_skia::Stroke {
                    width: (size * 0.15).max(1.0),
                    ..tiny_skia::Stroke::default()
                };
                pixmap.stroke_path(
                    &path,
                    &paint,
                    &stroke,
                    tiny_skia::Transform::identity(),
                    None,
                );
            }
        }
        MarkerStyle::Triangle | MarkerStyle::TriangleOpen | MarkerStyle::TriangleDown => {
            let mut path = tiny_skia::PathBuilder::new();
            if style == MarkerStyle::TriangleDown {
                path.move_to(x, y + radius);
                path.line_to(x - radius * 0.866, y - radius * 0.5);
                path.line_to(x + radius * 0.866, y - radius * 0.5);
            } else {
                path.move_to(x, y - radius);
                path.line_to(x - radius * 0.866, y + radius * 0.5);
                path.line_to(x + radius * 0.866, y + radius * 0.5);
            }
            path.close();
            let path = path.finish().ok_or(PlottingError::RenderError(
                "Failed to create triangle marker path".to_string(),
            ))?;
            if style.is_filled() {
                pixmap.fill_path(
                    &path,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    tiny_skia::Transform::identity(),
                    None,
                );
            } else {
                let stroke = tiny_skia::Stroke {
                    width: (size * 0.15).max(1.0),
                    ..tiny_skia::Stroke::default()
                };
                pixmap.stroke_path(
                    &path,
                    &paint,
                    &stroke,
                    tiny_skia::Transform::identity(),
                    None,
                );
            }
        }
        MarkerStyle::Diamond | MarkerStyle::DiamondOpen => {
            let mut path = tiny_skia::PathBuilder::new();
            path.move_to(x, y - radius);
            path.line_to(x + radius, y);
            path.line_to(x, y + radius);
            path.line_to(x - radius, y);
            path.close();
            let path = path.finish().ok_or(PlottingError::RenderError(
                "Failed to create diamond marker path".to_string(),
            ))?;
            if style.is_filled() {
                pixmap.fill_path(
                    &path,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    tiny_skia::Transform::identity(),
                    None,
                );
            } else {
                let stroke = tiny_skia::Stroke {
                    width: (size * 0.15).max(1.0),
                    ..tiny_skia::Stroke::default()
                };
                pixmap.stroke_path(
                    &path,
                    &paint,
                    &stroke,
                    tiny_skia::Transform::identity(),
                    None,
                );
            }
        }
        MarkerStyle::Plus | MarkerStyle::Cross | MarkerStyle::Star => {
            let stroke = tiny_skia::Stroke {
                width: (size * 0.25).max(1.0),
                ..tiny_skia::Stroke::default()
            };
            let mut path = tiny_skia::PathBuilder::new();
            if matches!(style, MarkerStyle::Plus | MarkerStyle::Star) {
                path.move_to(x - radius, y);
                path.line_to(x + radius, y);
                path.move_to(x, y - radius);
                path.line_to(x, y + radius);
            }
            if matches!(style, MarkerStyle::Cross | MarkerStyle::Star) {
                let offset = radius * 0.707;
                path.move_to(x - offset, y - offset);
                path.line_to(x + offset, y + offset);
                path.move_to(x - offset, y + offset);
                path.line_to(x + offset, y - offset);
            }
            let path = path.finish().ok_or(PlottingError::RenderError(
                "Failed to create cross marker path".to_string(),
            ))?;
            pixmap.stroke_path(
                &path,
                &paint,
                &stroke,
                tiny_skia::Transform::identity(),
                None,
            );
        }
    }

    Ok(())
}

fn blend_channel(background: u8, foreground: u8, alpha: f32) -> u8 {
    let bg = background as f32 / 255.0;
    let fg = foreground as f32 / 255.0;
    ((bg * (1.0 - alpha) + fg * alpha) * 255.0) as u8
}

fn draw_hit(pixels: &mut [u8], size_px: (u32, u32), hit: &HitResult, color: Color) {
    match hit {
        HitResult::SeriesPoint {
            screen_position, ..
        } => {
            draw_circle(pixels, size_px, *screen_position, 6.0, color);
        }
        HitResult::HeatmapCell { screen_rect, .. } => {
            draw_rect(pixels, size_px, *screen_rect, color)
        }
        HitResult::None => {}
    }
}

fn draw_circle(
    pixels: &mut [u8],
    size_px: (u32, u32),
    center: ViewportPoint,
    radius: f64,
    color: Color,
) {
    let width = size_px.0 as i32;
    let height = size_px.1 as i32;
    let radius_sq = (radius * radius) as i32;
    let cx = center.x as i32;
    let cy = center.y as i32;

    for dy in -(radius as i32)..=(radius as i32) {
        for dx in -(radius as i32)..=(radius as i32) {
            if dx * dx + dy * dy > radius_sq {
                continue;
            }
            let x = cx + dx;
            let y = cy + dy;
            if x < 0 || y < 0 || x >= width || y >= height {
                continue;
            }
            let index = ((y * width + x) * 4) as usize;
            let alpha = color.a as f32 / 255.0;
            pixels[index] = blend_channel(pixels[index], color.r, alpha);
            pixels[index + 1] = blend_channel(pixels[index + 1], color.g, alpha);
            pixels[index + 2] = blend_channel(pixels[index + 2], color.b, alpha);
            pixels[index + 3] = color.a;
        }
    }
}

fn draw_rect(pixels: &mut [u8], size_px: (u32, u32), rect: ViewportRect, color: Color) {
    let width = size_px.0 as i32;
    let height = size_px.1 as i32;
    let x1 = rect.min.x.round() as i32;
    let y1 = rect.min.y.round() as i32;
    let x2 = rect.max.x.round() as i32;
    let y2 = rect.max.y.round() as i32;
    let alpha = color.a as f32 / 255.0;

    for y in y1.max(0)..y2.min(height) {
        for x in x1.max(0)..x2.min(width) {
            let index = ((y * width + x) * 4) as usize;
            pixels[index] = blend_channel(pixels[index], color.r, alpha);
            pixels[index + 1] = blend_channel(pixels[index + 1], color.g, alpha);
            pixels[index + 2] = blend_channel(pixels[index + 2], color.b, alpha);
            pixels[index + 3] = color.a;
        }
    }
}

fn draw_tooltip_overlay(pixels: &mut [u8], size_px: (u32, u32), tooltip: &TooltipState) {
    const TOOLTIP_FONT_SIZE: f32 = 13.0;
    const TOOLTIP_PADDING_X: f64 = 8.0;
    const TOOLTIP_PADDING_Y: f64 = 6.0;
    const TOOLTIP_CURSOR_GAP: f64 = 12.0;

    let text_renderer = TextRenderer::new();
    let font = FontConfig::new(FontFamily::SansSerif, TOOLTIP_FONT_SIZE);
    let (text_width, text_height) = text_renderer
        .measure_text(&tooltip.content, &font)
        .unwrap_or_else(|_| {
            (
                tooltip.content.chars().count() as f32 * TOOLTIP_FONT_SIZE * 0.6,
                TOOLTIP_FONT_SIZE * 1.2,
            )
        });

    let tooltip_width = f64::from(text_width) + TOOLTIP_PADDING_X * 2.0;
    let tooltip_height = f64::from(text_height) + TOOLTIP_PADDING_Y * 2.0;
    let view_width = size_px.0 as f64;
    let view_height = size_px.1 as f64;
    let max_left = (view_width - tooltip_width).max(0.0);
    let max_top = (view_height - tooltip_height).max(0.0);

    let mut left = tooltip.position_px.x + TOOLTIP_CURSOR_GAP;
    if left + tooltip_width > view_width {
        left = tooltip.position_px.x - tooltip_width - TOOLTIP_CURSOR_GAP;
    }
    let mut top = tooltip.position_px.y - tooltip_height - TOOLTIP_CURSOR_GAP;
    if top < 0.0 {
        top = tooltip.position_px.y + TOOLTIP_CURSOR_GAP;
    }

    left = left.clamp(0.0, max_left);
    top = top.clamp(0.0, max_top);

    let rect = ViewportRect {
        min: ViewportPoint::new(left, top),
        max: ViewportPoint::new(left + tooltip_width, top + tooltip_height),
    };
    draw_rect(pixels, size_px, rect, Color::new_rgba(255, 255, 220, 220));

    let Some(size) = tiny_skia::IntSize::from_wh(size_px.0, size_px.1) else {
        log::debug!("Skipping tooltip text render because overlay size is invalid");
        return;
    };
    let Some(mut pixmap) = tiny_skia::Pixmap::from_vec(pixels.to_vec(), size) else {
        log::debug!("Skipping tooltip text render because tooltip pixmap creation failed");
        return;
    };

    if let Err(err) = text_renderer.render_text(
        &mut pixmap,
        &tooltip.content,
        (left + TOOLTIP_PADDING_X) as f32,
        (top + TOOLTIP_PADDING_Y) as f32,
        &font,
        Color::new_rgba(24, 24, 24, 255),
    ) {
        log::debug!("Skipping tooltip text render after text rasterization failed: {err}");
        return;
    }

    let rendered = pixmap.take();
    pixels.copy_from_slice(&rendered);
}

fn tooltip_from_hit(hit: &HitResult) -> TooltipState {
    match hit {
        HitResult::SeriesPoint {
            screen_position,
            data_position,
            ..
        } => TooltipState {
            content: format!("x={:.3}, y={:.3}", data_position.x, data_position.y),
            position_px: *screen_position,
        },
        HitResult::HeatmapCell {
            screen_rect,
            row,
            col,
            value,
            ..
        } => TooltipState {
            content: format!("row={}, col={}, value={:.3}", row, col, value),
            position_px: screen_rect.max,
        },
        HitResult::None => TooltipState {
            content: String::new(),
            position_px: ViewportPoint::default(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Observable, StreamingXY, signal};
    use crate::prelude::Plot;
    use crate::render::{Color, MarkerStyle};
    use std::sync::{Arc, atomic::Ordering};

    fn render_target() -> SurfaceTarget {
        SurfaceTarget {
            size_px: (320, 240),
            scale_factor: 1.0,
            time_seconds: 0.0,
        }
    }

    fn derived_y_ticks(session: &InteractivePlotSession) -> Vec<f64> {
        let geometry = session
            .geometry_snapshot()
            .expect("geometry should be available after render");
        let plot = session.prepared_plot().plot();
        crate::axes::generate_ticks_for_scale(
            geometry.y_bounds.0,
            geometry.y_bounds.1,
            plot.layout.tick_config.major_ticks_y,
            &plot.layout.y_scale,
        )
    }

    fn color_centroid<F>(image: &Image, predicate: F) -> Option<ViewportPoint>
    where
        F: Fn(&[u8]) -> bool,
    {
        let mut x_sum = 0.0;
        let mut y_sum = 0.0;
        let mut count = 0.0;

        for (index, pixel) in image.pixels.chunks_exact(4).enumerate() {
            if !predicate(pixel) {
                continue;
            }

            let x = (index as u32 % image.width) as f64;
            let y = (index as u32 / image.width) as f64;
            x_sum += x;
            y_sum += y;
            count += 1.0;
        }

        (count > 0.0).then(|| ViewportPoint::new(x_sum / count, y_sum / count))
    }

    fn count_matching_pixels_near<F>(
        image: &Image,
        center: ViewportPoint,
        radius: u32,
        predicate: F,
    ) -> usize
    where
        F: Fn(&[u8]) -> bool,
    {
        let min_x = center.x.round().max(0.0) as i32 - radius as i32;
        let max_x = center.x.round().min(image.width as f64) as i32 + radius as i32;
        let min_y = center.y.round().max(0.0) as i32 - radius as i32;
        let max_y = center.y.round().min(image.height as f64) as i32 + radius as i32;

        let mut count = 0usize;
        for y in min_y.max(0)..max_y.min(image.height as i32) {
            for x in min_x.max(0)..max_x.min(image.width as i32) {
                let index = ((y as u32 * image.width + x as u32) * 4) as usize;
                if predicate(&image.pixels[index..index + 4]) {
                    count += 1;
                }
            }
        }

        count
    }

    #[test]
    fn test_dirty_domains_mark_and_clear() {
        let mut dirty = DirtyDomains::default();
        dirty.mark(DirtyDomain::Layout);
        dirty.mark(DirtyDomain::Overlay);
        dirty.mark(DirtyDomain::Temporal);

        assert!(dirty.layout);
        assert!(dirty.overlay);
        assert!(dirty.temporal);
        assert!(dirty.needs_base_render());
        assert!(dirty.needs_overlay_render());

        dirty.clear_base();
        assert!(!dirty.layout);
        assert!(!dirty.data);
        assert!(!dirty.temporal);
        assert!(!dirty.interaction);
        assert!(dirty.overlay);

        dirty.clear_overlay();
        assert!(!dirty.overlay);
    }

    #[test]
    fn test_resize_updates_size_and_scale_factor_together() {
        let plot: Plot = Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]).into();
        let session = plot.prepare_interactive();

        session.resize((640, 480), 2.0);

        let state = session
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned")
            .clone();
        assert_eq!(state.size_px, (640, 480));
        assert_eq!(state.scale_factor, 2.0);
        assert!(session.dirty_domains().layout);
    }

    #[test]
    fn test_resize_event_updates_size_and_scale_factor_together() {
        let plot: Plot = Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]).into();
        let session = plot.prepare_interactive();

        session.apply_input(PlotInputEvent::Resize {
            size_px: (640, 480),
            scale_factor: 2.0,
        });

        let state = session
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned")
            .clone();
        assert_eq!(state.size_px, (640, 480));
        assert_eq!(state.scale_factor, 2.0);
        assert!(session.dirty_domains().layout);
    }

    #[test]
    fn test_inflight_dirty_marks_survive_render_clear() {
        let plot: Plot = Plot::new().line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0]).into();
        let session = plot.prepare_interactive();

        session.mark_dirty(DirtyDomain::Data);
        let render_epoch = session.inner.dirty_epoch.load(Ordering::Acquire);
        session.mark_dirty(DirtyDomain::Overlay);
        session.clear_dirty_after_render(render_epoch);

        let dirty = session.dirty_domains();
        assert!(dirty.data);
        assert!(dirty.overlay);
    }

    #[test]
    fn test_session_invalidate_forces_base_rerender() {
        let plot: Plot = Plot::new()
            .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0])
            .title("Invalidate")
            .into();
        let session = plot.prepare_interactive();

        session
            .render_to_surface(render_target())
            .expect("initial surface frame should render");
        assert!(!session.dirty_domains().needs_base_render());

        session.invalidate();
        assert!(session.dirty_domains().needs_base_render());

        let rerendered = session
            .render_to_surface(render_target())
            .expect("invalidated surface frame should rerender");
        assert!(rerendered.layer_state.base_dirty);
    }

    #[test]
    fn test_compose_images_alpha_blends_overlay() {
        let base = Image::new(1, 1, vec![0, 0, 255, 255]);
        let overlay = Image::new(1, 1, vec![255, 0, 0, 128]);
        let composed = compose_images(&base, &overlay);

        assert!(composed.pixels[0] > 0);
        assert!(composed.pixels[2] > 0);
        assert_eq!(composed.pixels[3], 255);
    }

    #[test]
    fn test_overlay_only_updates_reuse_cached_base_layer() {
        let plot: Plot = Plot::new()
            .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0])
            .title("Layer Reuse")
            .into();
        let session = plot.prepare_interactive();

        let first = session
            .render_to_surface(SurfaceTarget {
                size_px: (320, 240),
                scale_factor: 1.0,
                time_seconds: 0.0,
            })
            .expect("surface frame should render");
        assert!(first.layer_state.base_dirty);

        session.apply_input(PlotInputEvent::Hover {
            position_px: ViewportPoint::new(160.0, 120.0),
        });
        let second = session
            .render_to_surface(SurfaceTarget {
                size_px: (320, 240),
                scale_factor: 1.0,
                time_seconds: 0.0,
            })
            .expect("surface frame should render after hover");

        assert!(!second.layer_state.base_dirty);
        assert!(second.layer_state.overlay_dirty);
        assert!(Arc::ptr_eq(&first.layers.base, &second.layers.base));
    }

    #[test]
    fn test_tooltip_overlay_renders_text_pixels() {
        let plot: Plot = Plot::new()
            .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0])
            .title("Tooltip")
            .into();
        let session = plot.prepare_interactive();

        session.apply_input(PlotInputEvent::ShowTooltip {
            content: "x=1.234, y=5.678".to_string(),
            position_px: ViewportPoint::new(180.0, 120.0),
        });

        let frame = session
            .render_to_surface(SurfaceTarget {
                size_px: (320, 240),
                scale_factor: 1.0,
                time_seconds: 0.0,
            })
            .expect("surface frame should render with tooltip");

        let overlay = frame
            .layers
            .overlay
            .expect("surface frame should include overlay pixels");
        let dark_text_pixels = overlay
            .pixels
            .chunks_exact(4)
            .filter(|pixel| pixel[3] > 0 && (pixel[0] < 220 || pixel[1] < 220 || pixel[2] < 180))
            .count();

        assert!(
            dark_text_pixels > 0,
            "tooltip overlay should contain dark text pixels in addition to the background box"
        );
    }

    #[test]
    fn test_supported_surface_series_use_fast_path_on_full_rerender() {
        let plot: Plot = Plot::new()
            .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0])
            .title("Fast Path")
            .into();
        let session = plot.prepare_interactive();

        let frame = session
            .render_to_surface(SurfaceTarget {
                size_px: (320, 240),
                scale_factor: 1.0,
                time_seconds: 0.0,
            })
            .expect("supported surface frame should render");

        assert_eq!(frame.surface_capability, SurfaceCapability::FastPath);
        assert!(!frame.layer_state.used_incremental_data);
    }

    #[test]
    fn test_unsupported_surface_series_fall_back_to_image_capability() {
        let plot: Plot = Plot::new()
            .histogram(&[0.0, 1.0, 1.5, 2.0, 2.5], None)
            .into();
        let session = plot.prepare_interactive();

        let frame = session
            .render_to_surface(SurfaceTarget {
                size_px: (320, 240),
                scale_factor: 1.0,
                time_seconds: 0.0,
            })
            .expect("fallback surface frame should render");

        assert_eq!(frame.surface_capability, SurfaceCapability::FallbackImage);
    }

    #[test]
    fn test_streaming_surface_render_uses_incremental_fast_path() {
        let stream = StreamingXY::new(256);
        stream.push_many(vec![(0.0, 0.0), (1.0, 0.5), (2.0, 1.0)]);

        let plot: Plot = Plot::new()
            .line_streaming(&stream)
            .xlim(0.0, 10.0)
            .ylim(-2.0, 2.0)
            .into();
        let session = plot.prepare_interactive();

        session
            .render_to_surface(SurfaceTarget {
                size_px: (320, 240),
                scale_factor: 1.0,
                time_seconds: 0.0,
            })
            .expect("initial surface frame should render");

        stream.push(3.0, 0.75);
        let incremental = session
            .render_to_surface(SurfaceTarget {
                size_px: (320, 240),
                scale_factor: 1.0,
                time_seconds: 0.0,
            })
            .expect("incremental surface frame should render");

        assert!(incremental.layer_state.used_incremental_data);
        assert_eq!(incremental.surface_capability, SurfaceCapability::FastPath);
        assert_eq!(stream.appended_count(), 0);
    }

    #[test]
    fn test_streaming_surface_render_falls_back_after_wraparound() {
        let stream = StreamingXY::new(3);
        stream.push_many(vec![(0.0, 0.0), (1.0, 0.5), (2.0, 1.0)]);

        let plot: Plot = Plot::new()
            .line_streaming(&stream)
            .xlim(0.0, 3.0)
            .ylim(-1.0, 2.0)
            .into();
        let session = plot.prepare_interactive();

        session
            .render_to_surface(render_target())
            .expect("initial wrapped-stream frame should render");

        stream.push(3.0, 1.25);
        let rerendered = session
            .render_to_surface(render_target())
            .expect("wrapped-stream surface frame should render");

        assert!(!rerendered.layer_state.used_incremental_data);
        assert_eq!(stream.appended_count(), 0);
    }

    #[test]
    fn test_zoom_keeps_cursor_anchor_stable() {
        let plot: Plot = Plot::new()
            .line(&[0.0, 10.0], &[0.0, 10.0])
            .xlim(0.0, 10.0)
            .ylim(0.0, 10.0)
            .into();
        let session = plot.prepare_interactive();

        session
            .render_to_surface(render_target())
            .expect("initial surface frame should render");

        let anchor_px = ViewportPoint::new(0.0, 120.0);
        let before_state = session
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned")
            .clone();
        let before_visible = visible_bounds(&before_state);
        let anchor_before = screen_to_data(&before_state, before_visible, anchor_px);

        session.apply_input(PlotInputEvent::Zoom {
            factor: 2.0,
            center_px: anchor_px,
        });

        let after_state = session
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned")
            .clone();
        let after_visible = visible_bounds(&after_state);
        let anchor_after = screen_to_data(&after_state, after_visible, anchor_px);

        assert!((anchor_before.x - anchor_after.x).abs() < 1e-9);
        assert!((anchor_before.y - anchor_after.y).abs() < 1e-9);
    }

    #[test]
    fn test_temporal_layout_content_uses_current_frame_labels() {
        let xlabel = signal::of(|time| {
            if time < 1.0 {
                "baseline".to_string()
            } else {
                "updated temporal label".to_string()
            }
        });
        let plot: Plot = Plot::new()
            .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0])
            .xlabel(xlabel)
            .into();
        let visible = DataBounds::from_limits(0.0, 2.0, 0.0, 4.0);

        let baseline_content = plot.create_plot_content(visible.y_min, visible.y_max);
        let current_content = plot.create_plot_content_at_time(visible.y_min, visible.y_max, 1.0);
        let prepared_content = plot
            .prepared_frame_plot(render_target().size_px, render_target().scale_factor, 1.0)
            .create_plot_content(visible.y_min, visible.y_max);

        assert_eq!(baseline_content.xlabel.as_deref(), Some("baseline"));
        assert_eq!(
            current_content.xlabel.as_deref(),
            Some("updated temporal label")
        );
        assert_eq!(prepared_content.xlabel, current_content.xlabel);

        compute_plot_layout(
            &plot,
            render_target().size_px,
            render_target().scale_factor,
            1.0,
            visible,
        )
        .expect("interactive layout should compute using current-frame labels");
    }

    #[test]
    fn test_incremental_line_render_preserves_markers() {
        let stream = StreamingXY::new(32);
        stream.push_many(vec![(0.5, 0.5), (1.0, 1.2)]);

        let plot: Plot = Plot::new()
            .line_streaming(&stream)
            .color(Color::new(220, 20, 20))
            .width(1.0)
            .marker(MarkerStyle::Square)
            .marker_size(18.0)
            .into();
        let plot = plot.ticks(false).grid(false).xlim(0.0, 3.0).ylim(0.0, 3.0);
        let session = plot.prepare_interactive();

        session
            .render_to_surface(render_target())
            .expect("initial line+marker surface frame should render");

        stream.push(2.0, 2.3);
        let incremental = session
            .render_to_surface(render_target())
            .expect("incremental line+marker surface frame should render");
        assert!(incremental.layer_state.used_incremental_data);

        let geometry = session
            .geometry_snapshot()
            .expect("geometry should be available after incremental render");
        let (px, py) = map_data_to_pixels(
            2.0,
            2.3,
            geometry.x_bounds.0,
            geometry.x_bounds.1,
            geometry.y_bounds.0,
            geometry.y_bounds.1,
            geometry.plot_area,
        );
        let point = ViewportPoint::new(px as f64, py as f64);
        let incremental_pixels =
            count_matching_pixels_near(incremental.layers.base.as_ref(), point, 12, |pixel| {
                pixel[3] > 0 && pixel[0] > 150 && pixel[1] < 100 && pixel[2] < 100
            });

        let full = session
            .prepared_plot()
            .plot()
            .prepared_frame_plot(render_target().size_px, render_target().scale_factor, 0.0)
            .xlim(0.0, 3.0)
            .ylim(0.0, 3.0)
            .render()
            .expect("full line+marker render should succeed");
        let full_pixels = count_matching_pixels_near(&full, point, 12, |pixel| {
            pixel[3] > 0 && pixel[0] > 150 && pixel[1] < 100 && pixel[2] < 100
        });

        assert!(incremental_pixels > 0);
        assert!((incremental_pixels as i32 - full_pixels as i32).abs() <= 12);
    }

    #[test]
    fn test_reactive_manual_ylim_stays_pinned_across_updates() {
        let y = Observable::new(vec![0.0, 0.5, 1.0, -0.25]);
        let plot: Plot = Plot::new()
            .line_source(vec![0.0, 1.0, 2.0, 3.0], y.clone())
            .ylim(-2.0, 2.0)
            .into();
        let session = plot.prepare_interactive();

        session
            .render_to_surface(render_target())
            .expect("initial surface frame should render");
        let first_geometry = session
            .geometry_snapshot()
            .expect("geometry should be available after initial render");
        let first_ticks = derived_y_ticks(&session);

        y.set(vec![12.0, -15.0, 8.0, -11.0]);
        session
            .render_to_surface(render_target())
            .expect("surface frame after first reactive update should render");
        let second_geometry = session
            .geometry_snapshot()
            .expect("geometry should be available after first reactive update");
        let second_ticks = derived_y_ticks(&session);

        y.set(vec![30.0, 5.0, -22.0, 18.0]);
        session
            .render_to_surface(render_target())
            .expect("surface frame after second reactive update should render");
        let third_geometry = session
            .geometry_snapshot()
            .expect("geometry should be available after second reactive update");
        let third_ticks = derived_y_ticks(&session);

        assert_eq!(first_geometry.y_bounds, (-2.0, 2.0));
        assert_eq!(second_geometry.y_bounds, (-2.0, 2.0));
        assert_eq!(third_geometry.y_bounds, (-2.0, 2.0));
        assert_eq!(first_ticks, second_ticks);
        assert_eq!(second_ticks, third_ticks);
    }

    #[test]
    fn test_dashboard_like_reactive_updates_do_not_drift_manual_ylim() {
        let x: Vec<f64> = (0..120).map(|index| index as f64 * 12.0 / 119.0).collect();
        let primary = Observable::new(
            x.iter()
                .map(|value| 0.85 * value.sin() + 0.2 * (value * 3.0).cos())
                .collect::<Vec<_>>(),
        );
        let baseline = Observable::new(
            x.iter()
                .map(|value| 0.4 * (value * 0.75).sin())
                .collect::<Vec<_>>(),
        );
        let event_x = Observable::new(vec![2.0, 6.0, 9.5]);
        let event_y = Observable::new(vec![1.1, -1.3, 1.4]);
        let accent = Observable::new(Color::new(42, 157, 143));

        let plot: Plot = Plot::new()
            .line(&x, &vec![1.2; x.len()])
            .color(Color::LIGHT_GRAY)
            .into();
        let plot: Plot = plot
            .line(&x, &vec![-1.2; x.len()])
            .color(Color::LIGHT_GRAY)
            .into();
        let plot: Plot = plot
            .line_source(x.clone(), primary.clone())
            .color_source(accent.clone())
            .line_width(2.4)
            .into();
        let plot: Plot = plot
            .line_source(x.clone(), baseline.clone())
            .color(Color::new(38, 70, 83))
            .line_width(1.6)
            .into();
        let plot: Plot = plot
            .scatter_source(event_x.clone(), event_y.clone())
            .color(Color::new(231, 111, 81))
            .marker(MarkerStyle::Diamond)
            .marker_size(9.0)
            .xlim(0.0, 12.0)
            .ylim(-2.0, 2.0)
            .into();
        let session = plot.prepare_interactive();

        session
            .render_to_surface(render_target())
            .expect("initial dashboard-like surface frame should render");
        let first_geometry = session
            .geometry_snapshot()
            .expect("geometry should be available after initial dashboard render");
        let first_ticks = derived_y_ticks(&session);

        primary.set(
            x.iter()
                .map(|value| 1.6 * (value * 1.3).sin() + 0.5 * (value * 2.8).cos())
                .collect::<Vec<_>>(),
        );
        baseline.set(
            x.iter()
                .map(|value| 0.55 * (value * 0.65).sin() - 0.2 * (value * 2.1).cos())
                .collect::<Vec<_>>(),
        );
        event_x.set(vec![1.0, 4.5, 10.5]);
        event_y.set(vec![1.5, -1.4, 1.7]);
        accent.set(Color::new(231, 111, 81));

        session
            .render_to_surface(render_target())
            .expect("dashboard-like surface frame after reactive updates should render");
        let second_geometry = session
            .geometry_snapshot()
            .expect("geometry should be available after dashboard reactive updates");
        let second_ticks = derived_y_ticks(&session);

        assert_eq!(first_geometry.y_bounds, (-2.0, 2.0));
        assert_eq!(second_geometry.y_bounds, (-2.0, 2.0));
        assert_eq!(first_ticks, second_ticks);
    }

    #[test]
    fn test_surface_frame_pixels_honor_manual_ylim() {
        let plot: Plot = Plot::new()
            .scatter(&[0.5], &[1.0])
            .color(Color::new(220, 20, 20))
            .marker(MarkerStyle::Square)
            .marker_size(18.0)
            .ticks(false)
            .grid(false)
            .into();
        let plot: Plot = plot
            .scatter(&[0.5], &[-1.0])
            .color(Color::new(20, 20, 220))
            .marker(MarkerStyle::Square)
            .marker_size(18.0)
            .ticks(false)
            .grid(false)
            .xlim(0.0, 1.0)
            .ylim(-2.0, 2.0)
            .into();
        let session = plot.prepare_interactive();
        let plain_plot = plot.clone().set_output_pixels(320, 240);
        let render_plot = session
            .prepared_plot()
            .plot()
            .prepared_frame_plot(render_target().size_px, render_target().scale_factor, 0.0)
            .xlim(0.0, 1.0)
            .ylim(-2.0, 2.0);

        assert_eq!(plain_plot.layout.y_limits, Some((-2.0, 2.0)));
        assert_eq!(render_plot.layout.y_limits, Some((-2.0, 2.0)));
        assert_eq!(render_plot.layout.x_limits, Some((0.0, 1.0)));

        let plain = plain_plot.render().expect("plain plot should render");
        let plain_red_center = color_centroid(&plain, |pixel| {
            pixel[3] > 0 && pixel[0] > 160 && pixel[1] < 80 && pixel[2] < 80
        })
        .expect("plain red marker pixels should be present");
        let direct = render_plot
            .render()
            .expect("direct prepared frame should render");
        let direct_red_center = color_centroid(&direct, |pixel| {
            pixel[3] > 0 && pixel[0] > 160 && pixel[1] < 80 && pixel[2] < 80
        })
        .expect("direct red marker pixels should be present");

        let visible = DataBounds::from_limits(0.0, 1.0, -2.0, 2.0);
        let layout = compute_plot_layout(
            &plain_plot,
            render_target().size_px,
            render_target().scale_factor,
            0.0,
            visible,
        )
        .expect("plot layout should compute for manual bounds");

        let frame = session
            .render_to_surface(render_target())
            .expect("surface frame should render");
        let base = frame.layers.base.as_ref();

        let red_center = color_centroid(base, |pixel| {
            pixel[3] > 0 && pixel[0] > 160 && pixel[1] < 80 && pixel[2] < 80
        })
        .expect("red marker pixels should be present");
        let blue_center = color_centroid(base, |pixel| {
            pixel[3] > 0 && pixel[0] < 80 && pixel[1] < 80 && pixel[2] > 160
        })
        .expect("blue marker pixels should be present");

        let expected_red_y = map_data_to_pixels(
            0.5,
            1.0,
            visible.x_min,
            visible.x_max,
            visible.y_min,
            visible.y_max,
            layout.plot_area_rect,
        )
        .1 as f64;

        assert!(
            (plain_red_center.y - expected_red_y).abs() <= 12.0,
            "plain render red marker y={} should be close to expected {}",
            plain_red_center.y,
            expected_red_y
        );
        assert!(
            (direct_red_center.y - expected_red_y).abs() <= 12.0,
            "direct render red marker y={} should be close to expected {}",
            direct_red_center.y,
            expected_red_y
        );
        let expected_blue_y = map_data_to_pixels(
            0.5,
            -1.0,
            visible.x_min,
            visible.x_max,
            visible.y_min,
            visible.y_max,
            layout.plot_area_rect,
        )
        .1 as f64;

        assert!(
            (red_center.y - expected_red_y).abs() <= 12.0,
            "red marker y={} should be close to expected {}",
            red_center.y,
            expected_red_y
        );
        assert!(
            (blue_center.y - expected_blue_y).abs() <= 12.0,
            "blue marker y={} should be close to expected {}",
            blue_center.y,
            expected_blue_y
        );
    }
}
