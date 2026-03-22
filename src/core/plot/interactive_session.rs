use super::{Image, Plot, PreparedPlot, ReactiveSubscription, ResolvedSeries, SeriesType};
use crate::{
    core::{
        LayoutCalculator, LayoutConfig, MarginConfig, PlotLayout, PlottingError, REFERENCE_DPI,
        Result,
    },
    render::{
        Color,
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

#[derive(Clone, Debug, Default)]
pub struct LayerImages {
    pub chrome: Option<Image>,
    pub data: Option<Image>,
    pub overlay: Option<Image>,
}

#[derive(Clone, Debug)]
pub struct InteractiveFrame {
    pub image: Image,
    pub layers: LayerImages,
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
        ViewportPoint::new((self.x_min + self.x_max) * 0.5, (self.y_min + self.y_max) * 0.5)
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
    image: Image,
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
    image: Image,
}

#[derive(Clone, Debug)]
struct GeometrySnapshot {
    key: InteractiveFrameKey,
    plot_area: tiny_skia::Rect,
    x_bounds: (f64, f64),
    y_bounds: (f64, f64),
}

impl InteractivePlotSession {
    pub(crate) fn new(prepared: PreparedPlot) -> Self {
        let initial_bounds = compute_base_bounds(prepared.plot(), 0.0).unwrap_or_else(|_| {
            DataBounds::from_limits(0.0, 1.0, 0.0, 1.0)
        });
        let dirty = Arc::new(Mutex::new(DirtyDomains::with_all()));
        let reactive_epoch = Arc::new(AtomicU64::new(0));
        let dirty_for_callback = Arc::clone(&dirty);
        let epoch_for_callback = Arc::clone(&reactive_epoch);
        let reactive_subscription = prepared.subscribe_reactive(move || {
            if let Ok(mut domains) = dirty_for_callback.lock() {
                domains.mark(DirtyDomain::Data);
                domains.mark(DirtyDomain::Overlay);
            }
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
                    base_bounds: initial_bounds,
                    ..SessionState::default()
                }),
                dirty,
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
        let normalized = (size_px.0.max(1), size_px.1.max(1));
        if state.size_px != normalized {
            state.size_px = normalized;
            drop(state);
            self.mark_dirty(DirtyDomain::Layout);
            return;
        }
        let scale_factor = sanitize_scale_factor(scale_factor);
        if state.scale_factor.to_bits() != scale_factor.to_bits() {
            state.scale_factor = scale_factor;
            drop(state);
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
                let normalized = (size_px.0.max(1), size_px.1.max(1));
                if state.size_px != normalized {
                    state.size_px = normalized;
                    drop(state);
                    self.mark_dirty(DirtyDomain::Layout);
                    return;
                }
                let scale_factor = sanitize_scale_factor(scale_factor);
                if state.scale_factor.to_bits() != scale_factor.to_bits() {
                    state.scale_factor = scale_factor;
                    drop(state);
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
                let old_zoom = state.zoom_level;
                state.zoom_level = (state.zoom_level * factor).clamp(0.1, 100.0);
                if (state.zoom_level - old_zoom).abs() > f64::EPSILON {
                    let visible = visible_bounds(&state);
                    let center_data = screen_to_data(&state, visible, center_px);
                    let current_center = visible.center();
                    let adjust = 1.0 - old_zoom / state.zoom_level;
                    state.pan_offset.x += (center_data.x - current_center.x) * adjust;
                    state.pan_offset.y += (center_data.y - current_center.y) * adjust;
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
                                screen_position: ViewportPoint::new(screen_x as f64, screen_y as f64),
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
        self.render_to_target(RenderTargetKind::Image, target.size_px, target.scale_factor, target.time_seconds)
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
        let dirty_before_render = self.dirty_domains();
        let base_key = {
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

            if dirty_before_render.layout
                || dirty_before_render.data
                || dirty_before_render.temporal
                || mark_data_dirty
            {
                if let Ok(base_bounds) = compute_base_bounds(self.inner.prepared.plot(), time_seconds) {
                    let mut state = self
                        .inner
                        .state
                        .lock()
                        .expect("InteractivePlotSession state lock poisoned");
                    state.base_bounds = base_bounds;
                }
            }

            let state = self
                .inner
                .state
                .lock()
                .expect("InteractivePlotSession state lock poisoned")
                .clone();
            build_frame_key(self.inner.prepared.plot(), &state)
        };

        let geometry = self.ensure_geometry(&base_key)?;
        let base_image = self.ensure_base_image(&base_key, &geometry)?;
        let overlay_image = self.ensure_overlay_image(size_px)?;
        let composed = compose_images(&base_image, &overlay_image);

        let mut dirty = self
            .inner
            .dirty
            .lock()
            .expect("InteractivePlotSession dirty lock poisoned");
        dirty.clear_base();
        dirty.clear_overlay();
        drop(dirty);

        let surface_capability = if target == RenderTargetKind::Surface {
            SurfaceCapability::FallbackImage
        } else {
            SurfaceCapability::Unsupported
        };
        let stats = self.record_frame_stats(frame_start.elapsed(), target, surface_capability);

        Ok(InteractiveFrame {
            image: composed,
            layers: LayerImages {
                chrome: Some(base_image),
                data: None,
                overlay: Some(overlay_image),
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
        let snapshot = self.inner.prepared.plot().snapshot_series(state.time_seconds);
        let visible = visible_bounds(&state);
        let layout = compute_plot_layout(
            self.inner.prepared.plot(),
            &snapshot,
            state.size_px,
            state.scale_factor,
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
    ) -> Result<Image> {
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
                        return Ok(cached.image.clone());
                    }
                }
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

        let mut state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned");
        state.base_cache = Some(InteractiveFrameCache {
            key: key.clone(),
            image: image.clone(),
        });
        Ok(image)
    }

    fn ensure_overlay_image(&self, size_px: (u32, u32)) -> Result<Image> {
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
            if !dirty.needs_overlay_render() {
                if let Some(cached) = &state.overlay_cache {
                    if cached.key == overlay_key {
                        return Ok(cached.image.clone());
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
            draw_rect(&mut pixels, size_px, region, Color::new_rgba(0, 100, 255, 72));
        }
        if let Some(tooltip) = &state.tooltip {
            let width = tooltip.content.len() as f64 * 8.0 + 16.0;
            let rect = ViewportRect {
                min: ViewportPoint::new(tooltip.position_px.x, tooltip.position_px.y - 28.0),
                max: ViewportPoint::new(tooltip.position_px.x + width, tooltip.position_px.y),
            };
            draw_rect(&mut pixels, size_px, rect, Color::new_rgba(255, 255, 220, 200));
        }

        let image = Image::new(size_px.0, size_px.1, pixels);
        let mut state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned");
        state.overlay_cache = Some(OverlayFrameCache {
            key: overlay_key,
            image: image.clone(),
        });
        Ok(image)
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
        self.inner
            .dirty
            .lock()
            .expect("InteractivePlotSession dirty lock poisoned")
            .mark(domain);
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

fn compute_base_bounds(plot: &Plot, time: f64) -> Result<DataBounds> {
    let snapshot = plot.snapshot_series(time);
    let (mut x_min, mut x_max, mut y_min, mut y_max) = if let (
        Some((x_min_manual, x_max_manual)),
        Some((y_min_manual, y_max_manual)),
    ) = (plot.layout.x_limits, plot.layout.y_limits)
    {
        (x_min_manual, x_max_manual, y_min_manual, y_max_manual)
    } else if let Some((x_min_manual, x_max_manual)) = plot.layout.x_limits {
        let (_, _, y_min_calc, y_max_calc) = plot.calculate_data_bounds_for_series(&snapshot)?;
        (x_min_manual, x_max_manual, y_min_calc, y_max_calc)
    } else if let Some((y_min_manual, y_max_manual)) = plot.layout.y_limits {
        let (x_min_calc, x_max_calc, _, _) = plot.calculate_data_bounds_for_series(&snapshot)?;
        (x_min_calc, x_max_calc, y_min_manual, y_max_manual)
    } else {
        plot.calculate_data_bounds_for_series(&snapshot)?
    };

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

fn visible_bounds(state: &SessionState) -> DataBounds {
    let base = state.base_bounds;
    let zoom = state.zoom_level.max(0.1);
    let width = base.width() / zoom;
    let height = base.height() / zoom;
    let center = ViewportPoint::new(base.center().x + state.pan_offset.x, base.center().y + state.pan_offset.y);
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
        time_bits: plot.has_temporal_sources().then_some(state.time_seconds.to_bits()),
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
    _snapshot_series: &[super::PlotSeries],
    size_px: (u32, u32),
    scale_factor: f32,
    visible: DataBounds,
) -> Result<ComputedSessionLayout> {
    let min_device_scale = crate::core::constants::dpi::MIN as f32 / REFERENCE_DPI;
    let device_scale = if !scale_factor.is_finite() || scale_factor <= 0.0 {
        1.0
    } else {
        scale_factor.max(min_device_scale)
    };
    let dpi = (REFERENCE_DPI * device_scale).round();

    let mut renderer = SkiaRenderer::new(size_px.0, size_px.1, plot.display.theme.clone())?;
    renderer.set_text_engine_mode(plot.display.text_engine);
    renderer.set_render_scale(plot.render_scale());

    let content = plot.create_plot_content(visible.y_min, visible.y_max);
    let measured_dimensions = plot.measure_layout_text(&renderer, &content, dpi)?;
    let measurements = measured_dimensions.as_ref();

    let layout = match &plot.display.config.margins {
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
                &plot.display.config.typography,
                &plot.display.config.spacing,
                dpi,
                measurements,
            )
        }
        _ => LayoutCalculator::new(LayoutConfig::default()).compute(
            size_px,
            &content,
            &plot.display.config.typography,
            &plot.display.config.spacing,
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
    for (dst, src) in pixels.chunks_exact_mut(4).zip(overlay.pixels.chunks_exact(4)) {
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

fn blend_channel(background: u8, foreground: u8, alpha: f32) -> u8 {
    let bg = background as f32 / 255.0;
    let fg = foreground as f32 / 255.0;
    ((bg * (1.0 - alpha) + fg * alpha) * 255.0) as u8
}

fn draw_hit(pixels: &mut [u8], size_px: (u32, u32), hit: &HitResult, color: Color) {
    match hit {
        HitResult::SeriesPoint { screen_position, .. } => {
            draw_circle(pixels, size_px, *screen_position, 6.0, color);
        }
        HitResult::HeatmapCell { screen_rect, .. } => draw_rect(pixels, size_px, *screen_rect, color),
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

fn draw_rect(
    pixels: &mut [u8],
    size_px: (u32, u32),
    rect: ViewportRect,
    color: Color,
) {
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
    fn test_compose_images_alpha_blends_overlay() {
        let base = Image::new(1, 1, vec![0, 0, 255, 255]);
        let overlay = Image::new(1, 1, vec![255, 0, 0, 128]);
        let composed = compose_images(&base, &overlay);

        assert!(composed.pixels[0] > 0);
        assert!(composed.pixels[2] > 0);
        assert_eq!(composed.pixels[3], 255);
    }
}
