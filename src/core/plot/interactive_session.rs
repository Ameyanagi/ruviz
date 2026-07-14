use super::{
    BackendOperation, Image, Plot, PlotData, PlotSeries, PreparedPlot, ReactiveSubscription,
    ResolvedData, ResolvedFrame, ResolvedSeries, SeriesType,
};
use crate::{
    axes::{AxisScale, expand_degenerate_range},
    core::{
        CoordinateTransform, LayoutCalculator, LayoutConfig, MarginConfig, PlotLayout,
        PlottingError, REFERENCE_DPI, Result,
    },
    render::{
        Color, FontConfig, FontFamily, LineStyle, MarkerStyle, TextRenderer, skia::SkiaRenderer,
    },
};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

thread_local! {
    static ACTIVE_RENDER_SESSIONS: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
}

struct ActiveRenderGuard {
    session_id: usize,
}

impl ActiveRenderGuard {
    fn enter(session_id: usize) -> Result<Self> {
        let inserted = ACTIVE_RENDER_SESSIONS.with(|sessions| {
            sessions
                .try_borrow_mut()
                .map(|mut sessions| sessions.insert(session_id))
                .unwrap_or(false)
        });
        if !inserted {
            return Err(PlottingError::RenderError(
                "reentrant interactive render request".to_string(),
            ));
        }
        Ok(Self { session_id })
    }
}

impl Drop for ActiveRenderGuard {
    fn drop(&mut self) {
        ACTIVE_RENDER_SESSIONS.with(|sessions| {
            if let Ok(mut sessions) = sessions.try_borrow_mut() {
                sessions.remove(&self.session_id);
            }
        });
    }
}

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
const HIT_TEST_TOLERANCE_LOGICAL_PX: f64 = 8.0;
const MIN_INDEXED_POINT_COUNT: usize = 256;
const MAX_INDEX_QUERY_CELLS: u64 = 4096;

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
    render_gate: Mutex<()>,
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

    fn from_screen_corners(top_left: ViewportPoint, bottom_right: ViewportPoint) -> Self {
        Self::from_limits(top_left.x, bottom_right.x, bottom_right.y, top_left.y)
    }

    fn from_viewport_rect(bounds: ViewportRect) -> Self {
        Self::from_limits(bounds.min.x, bounds.max.x, bounds.min.y, bounds.max.y)
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
enum DisplayedData {
    Static,
    Shared(Arc<[f64]>),
}

impl DisplayedData {
    fn capture(source: &PlotData, resolved: &ResolvedData<'_>) -> Self {
        if source.is_static() {
            Self::Static
        } else {
            Self::Shared(
                resolved
                    .shared_arc()
                    .expect("dynamic frame data must use owned shared storage"),
            )
        }
    }

    fn values<'a>(&'a self, source: &'a PlotData) -> Option<&'a [f64]> {
        match self {
            Self::Static => source.as_static().map(Vec::as_slice),
            Self::Shared(values) => Some(values),
        }
    }
}

#[derive(Clone, Debug)]
struct DisplayedSeriesData {
    x: DisplayedData,
    y: DisplayedData,
}

#[derive(Clone, Debug)]
struct DisplayedFrameData {
    series: Vec<Option<DisplayedSeriesData>>,
}

impl DisplayedFrameData {
    fn capture(plot: &Plot, frame: &ResolvedFrame<'_>) -> Self {
        let series = plot
            .series_mgr
            .series
            .iter()
            .zip(&frame.series)
            .map(|(series, resolved)| match (&series.series_type, resolved) {
                (
                    SeriesType::Line { x_data, y_data }
                    | SeriesType::Scatter { x_data, y_data }
                    | SeriesType::ErrorBars { x_data, y_data, .. }
                    | SeriesType::ErrorBarsXY { x_data, y_data, .. },
                    ResolvedSeries::Line { x, y }
                    | ResolvedSeries::Scatter { x, y }
                    | ResolvedSeries::ErrorBars { x, y, .. }
                    | ResolvedSeries::ErrorBarsXY { x, y, .. },
                ) => Some(DisplayedSeriesData {
                    x: DisplayedData::capture(x_data, x),
                    y: DisplayedData::capture(y_data, y),
                }),
                _ => None,
            })
            .collect();
        Self { series }
    }

    fn xy<'a>(&'a self, plot: &'a Plot, series_index: usize) -> Option<(&'a [f64], &'a [f64])> {
        let displayed = self.series.get(series_index)?.as_ref()?;
        let series = plot.series_mgr.series.get(series_index)?;
        let (x_data, y_data) = match &series.series_type {
            SeriesType::Line { x_data, y_data }
            | SeriesType::Scatter { x_data, y_data }
            | SeriesType::ErrorBars { x_data, y_data, .. }
            | SeriesType::ErrorBarsXY { x_data, y_data, .. } => (x_data, y_data),
            _ => return None,
        };
        Some((displayed.x.values(x_data)?, displayed.y.values(y_data)?))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum AxisScaleIdentity {
    Linear,
    Log,
    SymLog { linthresh_bits: u64 },
}

impl From<&AxisScale> for AxisScaleIdentity {
    fn from(scale: &AxisScale) -> Self {
        match scale {
            AxisScale::Linear => Self::Linear,
            AxisScale::Log => Self::Log,
            AxisScale::SymLog { linthresh } => Self::SymLog {
                linthresh_bits: linthresh.to_bits(),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct PointHitIndexKey {
    frame_key: InteractiveFrameKey,
    plot_area_bits: [u32; 4],
    x_scale: AxisScaleIdentity,
    y_scale: AxisScaleIdentity,
    cell_size_bits: u64,
}

impl PointHitIndexKey {
    fn from_geometry(geometry: &GeometrySnapshot, cell_size_px: f64) -> Self {
        Self {
            frame_key: geometry.key.clone(),
            plot_area_bits: [
                geometry.plot_area.left().to_bits(),
                geometry.plot_area.top().to_bits(),
                geometry.plot_area.right().to_bits(),
                geometry.plot_area.bottom().to_bits(),
            ],
            x_scale: AxisScaleIdentity::from(&geometry.x_scale),
            y_scale: AxisScaleIdentity::from(&geometry.y_scale),
            cell_size_bits: cell_size_px.to_bits(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct IndexedPoint {
    point_index: usize,
    screen_position: ViewportPoint,
    data_position: ViewportPoint,
}

#[derive(Clone, Debug)]
struct ScreenSpacePointGrid {
    origin: ViewportPoint,
    cell_size_px: f64,
    cells: HashMap<(i64, i64), Vec<usize>>,
    points: Vec<IndexedPoint>,
}

impl ScreenSpacePointGrid {
    fn build(x: &[f64], y: &[f64], geometry: &GeometrySnapshot, cell_size_px: f64) -> Option<Self> {
        if x.len().min(y.len()) < MIN_INDEXED_POINT_COUNT
            || !cell_size_px.is_finite()
            || cell_size_px <= 0.0
            || !geometry.plot_area.width().is_finite()
            || !geometry.plot_area.height().is_finite()
            || geometry.plot_area.width() <= 0.0
            || geometry.plot_area.height() <= 0.0
        {
            return None;
        }

        let origin = ViewportPoint::new(
            geometry.plot_area.left() as f64,
            geometry.plot_area.top() as f64,
        );
        let mut cells = HashMap::<(i64, i64), Vec<usize>>::new();
        let mut points = Vec::with_capacity(x.len().min(y.len()));
        for (point_index, (&x_val, &y_val)) in x.iter().zip(y.iter()).enumerate() {
            let data_position = ViewportPoint::new(x_val, y_val);
            if !geometry.contains_transformable_data(data_position) {
                continue;
            }
            let screen_position = geometry.data_to_screen(data_position);
            if !screen_position.x.is_finite() || !screen_position.y.is_finite() {
                continue;
            }
            let Some(cell) = grid_cell(screen_position, origin, cell_size_px) else {
                continue;
            };
            let indexed_position = points.len();
            points.push(IndexedPoint {
                point_index,
                screen_position,
                data_position,
            });
            cells.entry(cell).or_default().push(indexed_position);
        }

        (points.len() >= MIN_INDEXED_POINT_COUNT).then_some(Self {
            origin,
            cell_size_px,
            cells,
            points,
        })
    }

    fn nearest(&self, position_px: ViewportPoint, tolerance_px: f64) -> GridQueryResult {
        let Some((min_col, min_row)) = grid_cell(
            ViewportPoint::new(position_px.x - tolerance_px, position_px.y - tolerance_px),
            self.origin,
            self.cell_size_px,
        ) else {
            return GridQueryResult::Fallback;
        };
        let Some((max_col, max_row)) = grid_cell(
            ViewportPoint::new(position_px.x + tolerance_px, position_px.y + tolerance_px),
            self.origin,
            self.cell_size_px,
        ) else {
            return GridQueryResult::Fallback;
        };
        let Some(min_col) = min_col.checked_sub(1) else {
            return GridQueryResult::Fallback;
        };
        let Some(min_row) = min_row.checked_sub(1) else {
            return GridQueryResult::Fallback;
        };
        let Some(max_col) = max_col.checked_add(1) else {
            return GridQueryResult::Fallback;
        };
        let Some(max_row) = max_row.checked_add(1) else {
            return GridQueryResult::Fallback;
        };
        let Some(column_count) = inclusive_cell_count(min_col, max_col) else {
            return GridQueryResult::Fallback;
        };
        let Some(row_count) = inclusive_cell_count(min_row, max_row) else {
            return GridQueryResult::Fallback;
        };
        if column_count.saturating_mul(row_count) > MAX_INDEX_QUERY_CELLS {
            return GridQueryResult::Fallback;
        }

        let mut best = None::<PointHitCandidate>;
        for row in min_row..=max_row {
            for col in min_col..=max_col {
                let Some(indexed_positions) = self.cells.get(&(col, row)) else {
                    continue;
                };
                for &indexed_position in indexed_positions {
                    let point = self.points[indexed_position];
                    let distance = screen_distance(position_px, point.screen_position);
                    if distance > tolerance_px {
                        continue;
                    }
                    let should_replace = best.as_ref().is_none_or(|current| {
                        distance < current.distance_px
                            || (distance == current.distance_px
                                && point.point_index < current.point_index)
                    });
                    if should_replace {
                        best = Some(PointHitCandidate {
                            point_index: point.point_index,
                            screen_position: point.screen_position,
                            data_position: point.data_position,
                            distance_px: distance,
                        });
                    }
                }
            }
        }
        GridQueryResult::Indexed(best)
    }
}

#[derive(Clone, Debug)]
struct PointHitIndex {
    key: PointHitIndexKey,
    series: Vec<Option<ScreenSpacePointGrid>>,
}

type LazyPointHitIndex = Arc<OnceLock<Arc<PointHitIndex>>>;

impl PointHitIndex {
    fn build(
        plot: &Plot,
        displayed_data: &DisplayedFrameData,
        geometry: &GeometrySnapshot,
    ) -> Self {
        let cell_size_px = geometry.logical_pixels_to_pixels(HIT_TEST_TOLERANCE_LOGICAL_PX);
        let series = plot
            .series_mgr
            .series
            .iter()
            .enumerate()
            .map(|(series_index, series)| match &series.series_type {
                SeriesType::Line { .. }
                | SeriesType::Scatter { .. }
                | SeriesType::ErrorBars { .. }
                | SeriesType::ErrorBarsXY { .. } => displayed_data
                    .xy(plot, series_index)
                    .and_then(|(x, y)| ScreenSpacePointGrid::build(x, y, geometry, cell_size_px)),
                _ => None,
            })
            .collect();
        Self {
            key: PointHitIndexKey::from_geometry(geometry, cell_size_px),
            series,
        }
    }

    fn matches_geometry(&self, geometry: &GeometrySnapshot) -> bool {
        self.key
            == PointHitIndexKey::from_geometry(geometry, f64::from_bits(self.key.cell_size_bits))
    }

    fn series_grid(&self, series_index: usize) -> Option<&ScreenSpacePointGrid> {
        self.series.get(series_index)?.as_ref()
    }
}

#[derive(Clone, Copy, Debug)]
struct PointHitCandidate {
    point_index: usize,
    screen_position: ViewportPoint,
    data_position: ViewportPoint,
    distance_px: f64,
}

#[derive(Clone, Copy, Debug)]
enum GridQueryResult {
    Indexed(Option<PointHitCandidate>),
    Fallback,
}

fn grid_cell(point: ViewportPoint, origin: ViewportPoint, cell_size_px: f64) -> Option<(i64, i64)> {
    let col = ((point.x - origin.x) / cell_size_px).floor();
    let row = ((point.y - origin.y) / cell_size_px).floor();
    if !col.is_finite()
        || !row.is_finite()
        || col < i64::MIN as f64
        || col > i64::MAX as f64
        || row < i64::MIN as f64
        || row > i64::MAX as f64
    {
        return None;
    }
    Some((col as i64, row as i64))
}

fn inclusive_cell_count(min: i64, max: i64) -> Option<u64> {
    if min > max {
        return None;
    }
    u64::try_from(i128::from(max) - i128::from(min) + 1).ok()
}

#[derive(Clone, Debug)]
struct InteractiveFrameCache {
    key: InteractiveFrameKey,
    image: Arc<Image>,
    geometry: GeometrySnapshot,
    displayed_data: DisplayedFrameData,
    point_hit_index: LazyPointHitIndex,
}

#[derive(Clone, Debug, PartialEq)]
struct OverlayFrameKey {
    size_px: (u32, u32),
    plot_area: Option<ViewportRect>,
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
    x_scale: AxisScale,
    y_scale: AxisScale,
    transform: CoordinateTransform,
}

impl GeometrySnapshot {
    fn with_bounds(&self, bounds: DataBounds) -> Self {
        let mut geometry = self.clone();
        geometry.x_bounds = (bounds.x_min, bounds.x_max);
        geometry.y_bounds = (bounds.y_min, bounds.y_max);
        geometry.transform.data_x = bounds.x_min..bounds.x_max;
        geometry.transform.data_y = bounds.y_min..bounds.y_max;
        geometry
    }

    fn logical_pixels_to_pixels(&self, logical_pixels: f64) -> f64 {
        logical_pixels * f64::from(f32::from_bits(self.key.scale_bits))
    }

    fn contains_screen(&self, point: ViewportPoint) -> bool {
        point.x.is_finite()
            && point.y.is_finite()
            && point.x >= self.plot_area.left() as f64
            && point.x <= self.plot_area.right() as f64
            && point.y >= self.plot_area.top() as f64
            && point.y <= self.plot_area.bottom() as f64
    }

    fn contains_data(&self, point: ViewportPoint) -> bool {
        point.x.is_finite()
            && point.y.is_finite()
            && value_in_bounds(point.x, self.x_bounds)
            && value_in_bounds(point.y, self.y_bounds)
    }

    fn contains_transformable_data(&self, point: ViewportPoint) -> bool {
        self.contains_data(point)
            && axis_accepts_value(&self.x_scale, point.x)
            && axis_accepts_value(&self.y_scale, point.y)
    }

    fn screen_to_data(&self, point: ViewportPoint) -> ViewportPoint {
        let (x, y) = self.transform.screen_to_data_scaled(
            point.x as f32,
            point.y as f32,
            &self.x_scale,
            &self.y_scale,
        );
        ViewportPoint::new(x, y)
    }

    fn data_to_screen(&self, point: ViewportPoint) -> ViewportPoint {
        let (x, y) =
            self.transform
                .data_to_screen_scaled(point.x, point.y, &self.x_scale, &self.y_scale);
        ViewportPoint::new(x as f64, y as f64)
    }

    fn clamp_screen(&self, point: ViewportPoint) -> ViewportPoint {
        ViewportPoint::new(
            point
                .x
                .clamp(self.plot_area.left() as f64, self.plot_area.right() as f64),
            point
                .y
                .clamp(self.plot_area.top() as f64, self.plot_area.bottom() as f64),
        )
    }

    fn clamp_data(&self, point: ViewportPoint) -> ViewportPoint {
        ViewportPoint::new(
            clamp_to_bounds(point.x, self.x_bounds),
            clamp_to_bounds(point.y, self.y_bounds),
        )
    }

    fn screen_normalized(&self, point: ViewportPoint) -> (f64, f64) {
        let data = self.screen_to_data(self.clamp_screen(point));
        (
            self.x_scale
                .normalized_position(data.x, self.x_bounds.0, self.x_bounds.1)
                .clamp(0.0, 1.0),
            self.y_scale
                .normalized_position(data.y, self.y_bounds.0, self.y_bounds.1)
                .clamp(0.0, 1.0),
        )
    }

    fn zoomed_bounds(
        &self,
        factor: f64,
        anchor_px: ViewportPoint,
        base_bounds: DataBounds,
    ) -> DataBounds {
        let (anchor_x, anchor_y) = self.screen_normalized(anchor_px);
        let x = zoom_axis_bounds(
            self.x_bounds,
            (base_bounds.x_min, base_bounds.x_max),
            &self.x_scale,
            factor,
            anchor_x,
        );
        let y = zoom_axis_bounds(
            self.y_bounds,
            (base_bounds.y_min, base_bounds.y_max),
            &self.y_scale,
            factor,
            anchor_y,
        );
        DataBounds::from_limits(x.0, x.1, y.0, y.1)
    }

    fn panned_bounds(&self, delta_px: ViewportPoint) -> DataBounds {
        let delta_x = delta_px.x / f64::from(self.plot_area.width()).max(1.0);
        let delta_y = delta_px.y / f64::from(self.plot_area.height()).max(1.0);
        DataBounds::from_limits(
            self.x_scale
                .inverse_normalized_position(-delta_x, self.x_bounds.0, self.x_bounds.1),
            self.x_scale.inverse_normalized_position(
                1.0 - delta_x,
                self.x_bounds.0,
                self.x_bounds.1,
            ),
            self.y_scale
                .inverse_normalized_position(delta_y, self.y_bounds.0, self.y_bounds.1),
            self.y_scale.inverse_normalized_position(
                1.0 + delta_y,
                self.y_bounds.0,
                self.y_bounds.1,
            ),
        )
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct AxisConstraints {
    x_limits: Option<(f64, f64)>,
    y_limits: Option<(f64, f64)>,
    x_scale: AxisScale,
    y_scale: AxisScale,
}

impl AxisConstraints {
    fn from_plot(plot: &Plot) -> Self {
        Self {
            x_limits: plot.layout.x_limits,
            y_limits: plot.layout.y_limits,
            x_scale: plot.layout.x_scale.clone(),
            y_scale: plot.layout.y_scale.clone(),
        }
    }

    fn apply(self, data_bounds: DataBounds) -> DataBounds {
        let (mut x_min, mut x_max) = self
            .x_limits
            .unwrap_or((data_bounds.x_min, data_bounds.x_max));
        let (mut y_min, mut y_max) = self
            .y_limits
            .unwrap_or((data_bounds.y_min, data_bounds.y_max));

        (x_min, x_max) = expand_degenerate_range(x_min, x_max, &self.x_scale);
        (y_min, y_max) = expand_degenerate_range(y_min, y_max, &self.y_scale);

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
        sync_legacy_viewport_fields(
            &mut state,
            &prepared.plot().layout.x_scale,
            &prepared.plot().layout.y_scale,
        );

        Self {
            inner: Arc::new(InteractivePlotSessionInner {
                prepared,
                frame_pacing: Mutex::new(FramePacing::Display),
                quality_policy: Mutex::new(QualityPolicy::Balanced),
                prefer_gpu: Mutex::new(false),
                reactive_subscription: Mutex::new(reactive_subscription),
                render_gate: Mutex::new(()),
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

    pub fn fitted_frame_size_px(&self, max_size_px: (u32, u32)) -> (u32, u32) {
        self.inner
            .prepared
            .plot()
            .fitted_output_size_for_max_pixels(max_size_px)
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
                let current_geometry = match self.interaction_geometry() {
                    Ok(geometry) => geometry,
                    Err(_) => return,
                };

                if !factor.is_finite() || factor <= 0.0 {
                    return;
                }

                let next_visible =
                    current_geometry.zoomed_bounds(factor, center_px, state_snapshot.base_bounds);
                if !bounds_have_extent(next_visible) {
                    return;
                }

                if bounds_close(state_snapshot.visible_bounds, next_visible) {
                    return;
                }

                let mut state = self
                    .inner
                    .state
                    .lock()
                    .expect("InteractivePlotSession state lock poisoned");
                set_visible_bounds(
                    &mut state,
                    next_visible,
                    &current_geometry.x_scale,
                    &current_geometry.y_scale,
                );
                drop(state);
                self.mark_dirty(DirtyDomain::Data);
                self.mark_dirty(DirtyDomain::Overlay);
            }
            PlotInputEvent::ZoomRect { region_px } => {
                let had_brush =
                    state.brush_anchor.take().is_some() || state.brushed_region.take().is_some();
                drop(state);

                if region_px.width() <= 1.0 || region_px.height() <= 1.0 {
                    if had_brush {
                        self.mark_dirty(DirtyDomain::Overlay);
                    }
                    return;
                }

                let current_geometry = match self.interaction_geometry() {
                    Ok(geometry) => geometry,
                    Err(_) => return,
                };
                let data_min =
                    current_geometry.screen_to_data(current_geometry.clamp_screen(region_px.min));
                let data_max =
                    current_geometry.screen_to_data(current_geometry.clamp_screen(region_px.max));
                let next_visible = DataBounds::from_screen_corners(data_min, data_max);
                if !bounds_have_extent(next_visible) {
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
                set_visible_bounds(
                    &mut state,
                    next_visible,
                    &current_geometry.x_scale,
                    &current_geometry.y_scale,
                );
                drop(state);
                if viewport_changed {
                    self.mark_dirty(DirtyDomain::Data);
                }
                self.mark_dirty(DirtyDomain::Overlay);
            }
            PlotInputEvent::Pan { delta_px } => {
                drop(state);
                let current_geometry = match self.interaction_geometry() {
                    Ok(geometry) => geometry,
                    Err(_) => return,
                };
                if !delta_px.x.is_finite() || !delta_px.y.is_finite() {
                    return;
                }
                let next_visible = current_geometry.panned_bounds(delta_px);
                if !bounds_have_extent(next_visible) {
                    return;
                }
                let mut state = self
                    .inner
                    .state
                    .lock()
                    .expect("InteractivePlotSession state lock poisoned");
                set_visible_bounds(
                    &mut state,
                    next_visible,
                    &current_geometry.x_scale,
                    &current_geometry.y_scale,
                );
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
                sync_legacy_viewport_fields(
                    &mut state,
                    &self.inner.prepared.plot().layout.x_scale,
                    &self.inner.prepared.plot().layout.y_scale,
                );
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
        let Some((geometry, displayed_data, point_hit_index)) = self.displayed_hit_test_data()
        else {
            return HitResult::None;
        };
        let tolerance_px = geometry.logical_pixels_to_pixels(HIT_TEST_TOLERANCE_LOGICAL_PX);
        hit_test_displayed_frame(
            self.inner.prepared.plot(),
            &displayed_data,
            &geometry,
            Some(&point_hit_index),
            position_px,
            tolerance_px,
        )
    }

    fn displayed_frame_data(
        &self,
    ) -> Option<(GeometrySnapshot, DisplayedFrameData, LazyPointHitIndex)> {
        self.inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned")
            .base_cache
            .as_ref()
            .map(|cache| {
                (
                    cache.geometry.clone(),
                    cache.displayed_data.clone(),
                    Arc::clone(&cache.point_hit_index),
                )
            })
    }

    fn displayed_hit_test_data(
        &self,
    ) -> Option<(GeometrySnapshot, DisplayedFrameData, Arc<PointHitIndex>)> {
        let (geometry, displayed_data, point_hit_index) = self.displayed_frame_data()?;
        let point_hit_index = Arc::clone(point_hit_index.get_or_init(|| {
            Arc::new(PointHitIndex::build(
                self.inner.prepared.plot(),
                &displayed_data,
                &geometry,
            ))
        }));
        Some((geometry, displayed_data, point_hit_index))
    }

    #[cfg(test)]
    fn hit_test_with_tolerance_px(
        &self,
        position_px: ViewportPoint,
        tolerance_px: f64,
    ) -> HitResult {
        let Some((geometry, displayed_data, point_hit_index)) = self.displayed_hit_test_data()
        else {
            return HitResult::None;
        };
        hit_test_displayed_frame(
            self.inner.prepared.plot(),
            &displayed_data,
            &geometry,
            Some(&point_hit_index),
            position_px,
            tolerance_px,
        )
    }

    #[cfg(test)]
    fn hit_test_brute_force_with_tolerance_px(
        &self,
        position_px: ViewportPoint,
        tolerance_px: f64,
    ) -> HitResult {
        let Some((geometry, displayed_data, _)) = self.displayed_frame_data() else {
            return HitResult::None;
        };
        hit_test_displayed_frame_brute_force(
            self.inner.prepared.plot(),
            &displayed_data,
            &geometry,
            position_px,
            tolerance_px,
        )
    }

    #[cfg(test)]
    fn point_hit_index_initialized(&self) -> bool {
        self.displayed_frame_data()
            .is_some_and(|(_, _, index)| index.get().is_some())
    }

    #[cfg(test)]
    fn indexed_point_series_count(&self) -> usize {
        self.displayed_hit_test_data()
            .map(|(_, _, index)| index.series.iter().flatten().count())
            .unwrap_or(0)
    }

    /// Converts a displayed-frame screen position to data coordinates.
    ///
    /// Returns `Ok(None)` for non-finite positions or positions outside the
    /// displayed plot area. An error is returned until a base frame has been
    /// rendered and its geometry is available.
    pub fn screen_to_data(&self, position_px: ViewportPoint) -> Result<Option<ViewportPoint>> {
        let geometry = self.displayed_geometry()?;
        if !geometry.contains_screen(position_px) {
            return Ok(None);
        }
        let data = geometry.screen_to_data(position_px);
        Ok((data.x.is_finite() && data.y.is_finite()).then_some(data))
    }

    /// Converts a screen position to data coordinates after clamping it to the
    /// displayed plot area.
    ///
    /// An error is returned for non-finite input or until displayed geometry is
    /// available.
    pub fn screen_to_data_clamped(&self, position_px: ViewportPoint) -> Result<ViewportPoint> {
        require_finite_point(position_px, "screen position")?;
        let geometry = self.displayed_geometry()?;
        let data = geometry.screen_to_data(geometry.clamp_screen(position_px));
        require_finite_point(data, "converted data position")?;
        Ok(data)
    }

    /// Converts a data position to screen coordinates for the displayed frame.
    ///
    /// Returns `Ok(None)` for non-finite positions or positions outside the
    /// displayed data bounds. An error is returned until displayed geometry is
    /// available.
    pub fn data_to_screen(&self, data_position: ViewportPoint) -> Result<Option<ViewportPoint>> {
        let geometry = self.displayed_geometry()?;
        if !geometry.contains_data(data_position) {
            return Ok(None);
        }
        let screen = geometry.data_to_screen(data_position);
        Ok((screen.x.is_finite() && screen.y.is_finite()).then_some(screen))
    }

    /// Converts a data position to screen coordinates after clamping it to the
    /// displayed data bounds.
    ///
    /// An error is returned for non-finite input or until displayed geometry is
    /// available.
    pub fn data_to_screen_clamped(&self, data_position: ViewportPoint) -> Result<ViewportPoint> {
        require_finite_point(data_position, "data position")?;
        let geometry = self.displayed_geometry()?;
        let screen = geometry.data_to_screen(geometry.clamp_data(data_position));
        require_finite_point(screen, "converted screen position")?;
        Ok(screen)
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
        let geometry = self
            .displayed_geometry()
            .or_else(|_| self.geometry_snapshot())?;
        let state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned")
            .clone();
        let displayed_bounds = DataBounds::from_limits(
            geometry.x_bounds.0,
            geometry.x_bounds.1,
            geometry.y_bounds.0,
            geometry.y_bounds.1,
        );
        let (zoom_level, pan_offset) = legacy_viewport_metrics(
            state.base_bounds,
            displayed_bounds,
            &geometry.x_scale,
            &geometry.y_scale,
        );

        Ok(InteractiveViewportSnapshot {
            zoom_level,
            pan_offset,
            base_bounds: data_bounds_to_viewport_rect(state.base_bounds),
            visible_bounds: data_bounds_to_viewport_rect(displayed_bounds),
            plot_area: plot_area_to_viewport_rect(geometry.plot_area),
            selected_count: state.selected.len(),
        })
    }

    /// Restores the visible bounds for the interactive viewport.
    pub fn restore_visible_bounds(&self, bounds: ViewportRect) -> bool {
        let next_visible = DataBounds::from_viewport_rect(bounds);
        if !bounds_have_extent(next_visible) {
            return false;
        }

        let plot = self.inner.prepared.plot();
        if plot
            .layout
            .x_scale
            .validate_range(next_visible.x_min, next_visible.x_max)
            .is_err()
            || plot
                .layout
                .y_scale
                .validate_range(next_visible.y_min, next_visible.y_max)
                .is_err()
        {
            return false;
        }

        let mut state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned");
        let next_visible = normalize_visible_bounds(
            next_visible,
            state.base_bounds,
            &plot.layout.x_scale,
            &plot.layout.y_scale,
        );
        if bounds_close(state.visible_bounds, next_visible) {
            return false;
        }

        set_visible_bounds(
            &mut state,
            next_visible,
            &plot.layout.x_scale,
            &plot.layout.y_scale,
        );
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
        let _active_render = ActiveRenderGuard::enter(Arc::as_ptr(&self.inner) as usize)?;
        let _render_guard = self
            .inner
            .render_gate
            .lock()
            .expect("InteractivePlotSession render gate poisoned");
        let frame_start = start_frame_timer();
        self.resize(size_px, scale_factor);
        self.apply_input(PlotInputEvent::SetTime { time_seconds });

        let state_before_render = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned")
            .clone();
        let render_result = (|| -> Result<InteractiveFrame> {
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
            let source_plot = self.inner.prepared.plot();
            let resolved_frame = dirty_before_render
                .needs_base_render()
                .then(|| source_plot.resolve_frame(time_seconds))
                .transpose()?;

            if dirty_before_render.layout
                || dirty_before_render.data
                || dirty_before_render.temporal
            {
                let constraints = AxisConstraints::from_plot(source_plot);
                let mut state = self
                    .inner
                    .state
                    .lock()
                    .expect("InteractivePlotSession state lock poisoned");
                let previous_base = state.base_bounds;
                let previous_visible = state.visible_bounds;
                let next_data_bounds = resolved_frame
                    .as_ref()
                    .and_then(|frame| compute_data_bounds_from_frame(source_plot, frame).ok())
                    .unwrap_or(state.data_bounds);
                state.data_bounds = next_data_bounds;
                state.base_bounds = constraints.apply(next_data_bounds);
                if bounds_close(previous_visible, previous_base) {
                    state.visible_bounds = state.base_bounds;
                } else {
                    state.visible_bounds = normalize_visible_bounds(
                        previous_visible,
                        state.base_bounds,
                        &source_plot.layout.x_scale,
                        &source_plot.layout.y_scale,
                    );
                }
                sync_legacy_viewport_fields(
                    &mut state,
                    &source_plot.layout.x_scale,
                    &source_plot.layout.y_scale,
                );
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

            let geometry = self.ensure_geometry(&base_key, resolved_frame.as_ref())?;
            let frame_size_px = {
                self.inner
                    .state
                    .lock()
                    .expect("InteractivePlotSession state lock poisoned")
                    .size_px
            };
            let base_result = self.ensure_base_image(
                &base_key,
                &geometry,
                dirty_before_render,
                resolved_frame.as_ref(),
            )?;
            self.refresh_overlay_state(dirty_before_render)?;
            let overlay_result = self.ensure_overlay_image(frame_size_px, dirty_before_render)?;
            let composed = if target == RenderTargetKind::Image {
                if let Some(overlay_image) = overlay_result.image.as_ref() {
                    Arc::new(compose_images(&base_result.image, overlay_image))
                } else {
                    Arc::clone(&base_result.image)
                }
            } else {
                Arc::clone(&base_result.image)
            };

            if base_result.updated {
                if let Some(frame) = resolved_frame.as_ref() {
                    frame.acknowledge_rendered(source_plot);
                }
            }
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
            let stats = self.record_frame_stats(
                elapsed_frame_time(frame_start),
                target,
                surface_capability,
            );

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
        })();

        if render_result.is_err() {
            *self
                .inner
                .state
                .lock()
                .expect("InteractivePlotSession state lock poisoned") = state_before_render;
        }
        render_result
    }

    fn ensure_geometry(
        &self,
        key: &InteractiveFrameKey,
        frame: Option<&ResolvedFrame<'_>>,
    ) -> Result<GeometrySnapshot> {
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
        let source_plot = self.inner.prepared.plot();
        let owned_frame = frame
            .is_none()
            .then(|| source_plot.resolve_frame(state.time_seconds))
            .transpose()?;
        let frame = frame.or(owned_frame.as_ref()).expect("geometry frame");
        let geometry = geometry_snapshot_for_state(source_plot, &state, key.clone(), frame)?;

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
        frame: Option<&ResolvedFrame<'_>>,
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
            if let Some(incremental) =
                self.try_incremental_stream_render(key, geometry, frame.expect("dirty base frame"))?
            {
                return Ok(incremental);
            }
        }

        let state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned")
            .clone();
        let source_plot = self.inner.prepared.plot();
        let frame = frame.expect("dirty base render must resolve a frame");
        let mut plot = source_plot
            .prepared_frame_shell_with_style(state.size_px, state.scale_factor, &frame.style)
            .xlim(geometry.x_bounds.0, geometry.x_bounds.1)
            .ylim(geometry.y_bounds.0, geometry.y_bounds.1);
        if self.prefer_gpu() {
            #[cfg(feature = "gpu")]
            {
                plot = plot.gpu(true);
            }
        }
        let mode = plot.render_execution_mode(BackendOperation::Interactive);
        let (renderer, _) = plot.render_renderer_with_frame_and_diagnostics(mode, frame)?;
        let image = Arc::new(renderer.into_image());
        let displayed_data = DisplayedFrameData::capture(source_plot, frame);
        let point_hit_index = Arc::new(OnceLock::new());

        let mut state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned");
        state.base_cache = Some(InteractiveFrameCache {
            key: key.clone(),
            image: Arc::clone(&image),
            geometry: geometry.clone(),
            displayed_data,
            point_hit_index,
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
            plot_area: state
                .base_cache
                .as_ref()
                .map(|cache| plot_area_to_viewport_rect(cache.geometry.plot_area)),
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
        let hit_clip = state
            .base_cache
            .as_ref()
            .map(|cache| plot_area_to_viewport_rect(cache.geometry.plot_area));
        if let Some(hit) = state.hovered.as_ref() {
            draw_hit(
                &mut pixels,
                size_px,
                hit,
                Color::new_rgba(255, 165, 0, 180),
                hit_clip,
            );
        }
        for hit in &state.selected {
            draw_hit(
                &mut pixels,
                size_px,
                hit,
                Color::new_rgba(255, 0, 0, 180),
                hit_clip,
            );
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

    fn refresh_overlay_state(&self, dirty_before_render: DirtyDomains) -> Result<()> {
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

        let (geometry, displayed_data) = state_snapshot
            .base_cache
            .as_ref()
            .map(|cache| (cache.geometry.clone(), cache.displayed_data.clone()))
            .ok_or_else(displayed_geometry_unavailable)?;
        let source_plot = self.inner.prepared.plot();
        let refreshed_hovered = state_snapshot
            .hovered
            .as_ref()
            .and_then(|hit| refresh_hit_result(hit, source_plot, &displayed_data, &geometry));
        let refreshed_selected = state_snapshot
            .selected
            .iter()
            .filter_map(|hit| refresh_hit_result(hit, source_plot, &displayed_data, &geometry))
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
        self.ensure_geometry(&key, None)
    }

    fn displayed_geometry(&self) -> Result<GeometrySnapshot> {
        self.inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned")
            .base_cache
            .as_ref()
            .map(|cache| cache.geometry.clone())
            .ok_or_else(displayed_geometry_unavailable)
    }

    fn interaction_geometry(&self) -> Result<GeometrySnapshot> {
        let state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned");
        state
            .base_cache
            .as_ref()
            .map(|cache| cache.geometry.with_bounds(state.visible_bounds))
            .ok_or_else(displayed_geometry_unavailable)
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
        frame: &ResolvedFrame<'_>,
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

        let source_plot = self.inner.prepared.plot();
        let Some(draw_ops) = collect_streaming_draw_ops(
            source_plot,
            frame,
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
        let displayed_data = DisplayedFrameData::capture(source_plot, frame);
        let point_hit_index = Arc::new(OnceLock::new());

        let mut state = self
            .inner
            .state
            .lock()
            .expect("InteractivePlotSession state lock poisoned");
        state.base_cache = Some(InteractiveFrameCache {
            key: key.clone(),
            image: Arc::clone(&image),
            geometry: geometry.clone(),
            displayed_data,
            point_hit_index,
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
        let plot = self.inner.prepared.plot();
        let normalized_half_span = 0.5 / zoom_level;
        let next_visible = DataBounds::from_limits(
            plot.layout.x_scale.inverse_normalized_position(
                0.5 - normalized_half_span,
                state.base_bounds.x_min,
                state.base_bounds.x_max,
            ) + pan_x,
            plot.layout.x_scale.inverse_normalized_position(
                0.5 + normalized_half_span,
                state.base_bounds.x_min,
                state.base_bounds.x_max,
            ) + pan_x,
            plot.layout.y_scale.inverse_normalized_position(
                0.5 - normalized_half_span,
                state.base_bounds.y_min,
                state.base_bounds.y_max,
            ) + pan_y,
            plot.layout.y_scale.inverse_normalized_position(
                0.5 + normalized_half_span,
                state.base_bounds.y_min,
                state.base_bounds.y_max,
            ) + pan_y,
        );
        if (state.zoom_level - zoom_level).abs() < f64::EPSILON
            && (state.pan_offset.x - pan_x).abs() < f64::EPSILON
            && (state.pan_offset.y - pan_y).abs() < f64::EPSILON
        {
            return;
        }
        set_visible_bounds(
            &mut state,
            next_visible,
            &plot.layout.x_scale,
            &plot.layout.y_scale,
        );
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
    let series = plot.snapshot_series(time);
    if series.is_empty() {
        let bounds = plot.empty_cartesian_bounds();
        return Ok(DataBounds::from_limits(
            bounds.0, bounds.1, bounds.2, bounds.3,
        ));
    }

    let (x_min, x_max, y_min, y_max) = plot.calculate_data_bounds_for_series(&series)?;
    Ok(DataBounds::from_limits(x_min, x_max, y_min, y_max))
}

fn compute_data_bounds_from_frame(plot: &Plot, frame: &ResolvedFrame<'_>) -> Result<DataBounds> {
    if frame.series.is_empty() {
        let bounds = plot.empty_cartesian_bounds();
        return Ok(DataBounds::from_limits(
            bounds.0, bounds.1, bounds.2, bounds.3,
        ));
    }

    let (mut x_min, mut x_max, mut y_min, mut y_max) =
        plot.calculate_data_bounds_from_resolved(&frame.series)?;

    (x_min, x_max) = expand_degenerate_range(x_min, x_max, &plot.layout.x_scale);
    (y_min, y_max) = expand_degenerate_range(y_min, y_max, &plot.layout.y_scale);

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
    axis_bounds_close((a.x_min, a.x_max), (b.x_min, b.x_max))
        && axis_bounds_close((a.y_min, a.y_max), (b.y_min, b.y_max))
}

fn axis_bounds_close(a: (f64, f64), b: (f64, f64)) -> bool {
    let tolerance = (a.1 - a.0).abs().max((b.1 - b.0).abs()) * 1e-12;
    (a.0 == b.0 || (a.0 - b.0).abs() <= tolerance) && (a.1 == b.1 || (a.1 - b.1).abs() <= tolerance)
}

fn bounds_have_extent(bounds: DataBounds) -> bool {
    bounds.x_min.is_finite()
        && bounds.x_max.is_finite()
        && bounds.y_min.is_finite()
        && bounds.y_max.is_finite()
        && bounds.x_min != bounds.x_max
        && bounds.y_min != bounds.y_max
}

fn normalize_axis_bounds(
    bounds: (f64, f64),
    base_bounds: (f64, f64),
    scale: &AxisScale,
) -> (f64, f64) {
    let start = scale.normalized_position(bounds.0, base_bounds.0, base_bounds.1);
    let end = scale.normalized_position(bounds.1, base_bounds.0, base_bounds.1);
    let center = (start + end) * 0.5;
    let direction = direction_for_span(end - start, 1.0);
    let span = (end - start)
        .abs()
        .clamp(1.0 / MAX_ZOOM_LEVEL, 1.0 / MIN_ZOOM_LEVEL)
        * direction;
    (
        scale.inverse_normalized_position(center - span * 0.5, base_bounds.0, base_bounds.1),
        scale.inverse_normalized_position(center + span * 0.5, base_bounds.0, base_bounds.1),
    )
}

fn zoom_axis_bounds(
    bounds: (f64, f64),
    base_bounds: (f64, f64),
    scale: &AxisScale,
    factor: f64,
    anchor: f64,
) -> (f64, f64) {
    let start = scale.normalized_position(bounds.0, base_bounds.0, base_bounds.1);
    let end = scale.normalized_position(bounds.1, base_bounds.0, base_bounds.1);
    let current_span = end - start;
    let direction = direction_for_span(current_span, 1.0);
    let next_span =
        (current_span.abs() / factor).clamp(1.0 / MAX_ZOOM_LEVEL, 1.0 / MIN_ZOOM_LEVEL) * direction;
    let anchor_position = start + anchor * current_span;
    (
        scale.inverse_normalized_position(
            anchor_position - anchor * next_span,
            base_bounds.0,
            base_bounds.1,
        ),
        scale.inverse_normalized_position(
            anchor_position + (1.0 - anchor) * next_span,
            base_bounds.0,
            base_bounds.1,
        ),
    )
}

fn normalize_visible_bounds(
    bounds: DataBounds,
    base_bounds: DataBounds,
    x_scale: &AxisScale,
    y_scale: &AxisScale,
) -> DataBounds {
    let x = normalize_axis_bounds(
        (bounds.x_min, bounds.x_max),
        (base_bounds.x_min, base_bounds.x_max),
        x_scale,
    );
    let y = normalize_axis_bounds(
        (bounds.y_min, bounds.y_max),
        (base_bounds.y_min, base_bounds.y_max),
        y_scale,
    );
    DataBounds::from_limits(x.0, x.1, y.0, y.1)
}

fn legacy_viewport_metrics(
    base_bounds: DataBounds,
    visible_bounds: DataBounds,
    x_scale: &AxisScale,
    y_scale: &AxisScale,
) -> (f64, ViewportPoint) {
    let x_span =
        x_scale.normalized_position(visible_bounds.x_max, base_bounds.x_min, base_bounds.x_max)
            - x_scale.normalized_position(
                visible_bounds.x_min,
                base_bounds.x_min,
                base_bounds.x_max,
            );
    let y_span =
        y_scale.normalized_position(visible_bounds.y_max, base_bounds.y_min, base_bounds.y_max)
            - y_scale.normalized_position(
                visible_bounds.y_min,
                base_bounds.y_min,
                base_bounds.y_max,
            );
    let zoom_x = x_span.abs().max(VIEWPORT_EPSILON).recip();
    let zoom_y = y_span.abs().max(VIEWPORT_EPSILON).recip();
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

fn sync_legacy_viewport_fields(state: &mut SessionState, x_scale: &AxisScale, y_scale: &AxisScale) {
    let (zoom_level, pan_offset) =
        legacy_viewport_metrics(state.base_bounds, state.visible_bounds, x_scale, y_scale);
    state.zoom_level = zoom_level;
    state.pan_offset = pan_offset;
}

fn set_visible_bounds(
    state: &mut SessionState,
    bounds: DataBounds,
    x_scale: &AxisScale,
    y_scale: &AxisScale,
) {
    state.visible_bounds = normalize_visible_bounds(bounds, state.base_bounds, x_scale, y_scale);
    sync_legacy_viewport_fields(state, x_scale, y_scale);
}

fn direction_for_span(span: f64, fallback: f64) -> f64 {
    if span < 0.0 || (span == 0.0 && fallback < 0.0) {
        -1.0
    } else {
        1.0
    }
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

fn value_in_bounds(value: f64, bounds: (f64, f64)) -> bool {
    value >= bounds.0.min(bounds.1) && value <= bounds.0.max(bounds.1)
}

fn clamp_to_bounds(value: f64, bounds: (f64, f64)) -> f64 {
    value.clamp(bounds.0.min(bounds.1), bounds.0.max(bounds.1))
}

fn require_finite_point(point: ViewportPoint, label: &str) -> Result<()> {
    if point.x.is_finite() && point.y.is_finite() {
        Ok(())
    } else {
        Err(PlottingError::InvalidInput(format!(
            "{label} must contain finite coordinates"
        )))
    }
}

fn axis_accepts_value(scale: &AxisScale, value: f64) -> bool {
    match scale {
        AxisScale::Linear | AxisScale::SymLog { .. } => true,
        AxisScale::Log => value > 0.0,
    }
}

fn screen_distance(first: ViewportPoint, second: ViewportPoint) -> f64 {
    let dx = first.x - second.x;
    let dy = first.y - second.y;
    (dx * dx + dy * dy).sqrt()
}

fn brute_force_point_candidate(
    x: &[f64],
    y: &[f64],
    geometry: &GeometrySnapshot,
    position_px: ViewportPoint,
    tolerance_px: f64,
) -> Option<PointHitCandidate> {
    let mut best = None::<PointHitCandidate>;
    for (point_index, (&x_val, &y_val)) in x.iter().zip(y.iter()).enumerate() {
        let data_position = ViewportPoint::new(x_val, y_val);
        if !geometry.contains_transformable_data(data_position) {
            continue;
        }
        let screen_position = geometry.data_to_screen(data_position);
        if !screen_position.x.is_finite() || !screen_position.y.is_finite() {
            continue;
        }
        let distance = screen_distance(position_px, screen_position);
        if distance <= tolerance_px
            && best
                .as_ref()
                .is_none_or(|current| distance < current.distance_px)
        {
            best = Some(PointHitCandidate {
                point_index,
                screen_position,
                data_position,
                distance_px: distance,
            });
        }
    }
    best
}

fn hit_test_displayed_frame_brute_force(
    plot: &Plot,
    displayed_data: &DisplayedFrameData,
    geometry: &GeometrySnapshot,
    position_px: ViewportPoint,
    tolerance_px: f64,
) -> HitResult {
    hit_test_displayed_frame(
        plot,
        displayed_data,
        geometry,
        None,
        position_px,
        tolerance_px,
    )
}

fn hit_test_displayed_frame(
    plot: &Plot,
    displayed_data: &DisplayedFrameData,
    geometry: &GeometrySnapshot,
    point_hit_index: Option<&PointHitIndex>,
    position_px: ViewportPoint,
    tolerance_px: f64,
) -> HitResult {
    if !geometry.contains_screen(position_px) || !tolerance_px.is_finite() || tolerance_px < 0.0 {
        return HitResult::None;
    }
    let point_hit_index = point_hit_index.filter(|index| index.matches_geometry(geometry));
    let mut best_hit = HitResult::None;
    let mut best_distance = f64::INFINITY;

    for (series_index, series) in plot.series_mgr.series.iter().enumerate() {
        match &series.series_type {
            SeriesType::Line { .. }
            | SeriesType::Scatter { .. }
            | SeriesType::ErrorBars { .. }
            | SeriesType::ErrorBarsXY { .. } => {
                let Some((x, y)) = displayed_data.xy(plot, series_index) else {
                    continue;
                };
                let candidate = match point_hit_index
                    .and_then(|index| index.series_grid(series_index))
                    .map(|grid| grid.nearest(position_px, tolerance_px))
                {
                    Some(GridQueryResult::Indexed(candidate)) => candidate,
                    Some(GridQueryResult::Fallback) | None => {
                        brute_force_point_candidate(x, y, geometry, position_px, tolerance_px)
                    }
                };
                let Some(candidate) = candidate else {
                    continue;
                };
                if candidate.distance_px < best_distance {
                    best_distance = candidate.distance_px;
                    best_hit = HitResult::SeriesPoint {
                        series_index,
                        point_index: candidate.point_index,
                        screen_position: candidate.screen_position,
                        data_position: candidate.data_position,
                        distance_px: candidate.distance_px,
                    };
                }
            }
            SeriesType::Heatmap { data } => {
                if data.n_rows == 0 || data.n_cols == 0 {
                    continue;
                }
                let data_position = geometry.screen_to_data(position_px);
                if !data_position.x.is_finite()
                    || !data_position.y.is_finite()
                    || data_position.x < data.x_extent.0
                    || data_position.x > data.x_extent.1
                    || data_position.y < data.y_extent.0
                    || data_position.y > data.y_extent.1
                {
                    continue;
                }

                let cell_width = (data.x_extent.1 - data.x_extent.0) / data.n_cols as f64;
                let cell_height = (data.y_extent.1 - data.y_extent.0) / data.n_rows as f64;
                if !cell_width.is_finite()
                    || !cell_height.is_finite()
                    || cell_width <= 0.0
                    || cell_height <= 0.0
                {
                    continue;
                }
                let col = ((data_position.x - data.x_extent.0) / cell_width)
                    .floor()
                    .clamp(0.0, data.n_cols.saturating_sub(1) as f64)
                    as usize;
                let row = ((data.y_extent.1 - data_position.y) / cell_height)
                    .floor()
                    .clamp(0.0, data.n_rows.saturating_sub(1) as f64)
                    as usize;
                let Some(value) = data
                    .values
                    .get(row)
                    .and_then(|values| values.get(col))
                    .copied()
                else {
                    continue;
                };
                if data.should_mask_value(value) {
                    continue;
                }
                let ((x1, x2), (y1, y2)) = data.cell_data_bounds(row, col);
                let first = geometry.data_to_screen(ViewportPoint::new(x1, y2));
                let second = geometry.data_to_screen(ViewportPoint::new(x2, y1));
                if !first.x.is_finite()
                    || !first.y.is_finite()
                    || !second.x.is_finite()
                    || !second.y.is_finite()
                {
                    continue;
                }
                best_hit = HitResult::HeatmapCell {
                    series_index,
                    row,
                    col,
                    value,
                    screen_rect: ViewportRect {
                        min: ViewportPoint::new(first.x.min(second.x), first.y.min(second.y)),
                        max: ViewportPoint::new(first.x.max(second.x), first.y.max(second.y)),
                    },
                };
            }
            _ => {}
        }
    }

    best_hit
}

fn displayed_geometry_unavailable() -> PlottingError {
    PlottingError::InvalidInput(
        "interactive coordinate conversion is unavailable before a base frame is displayed"
            .to_string(),
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
    frame: &ResolvedFrame<'_>,
) -> Result<GeometrySnapshot> {
    let visible = state.visible_bounds;
    let layout = compute_plot_layout_from_frame(
        plot,
        state.size_px,
        state.scale_factor,
        state.time_seconds,
        visible,
        frame,
    )?;

    Ok(GeometrySnapshot {
        key,
        plot_area: layout.plot_area_rect,
        x_bounds: (visible.x_min, visible.x_max),
        y_bounds: (visible.y_min, visible.y_max),
        x_scale: plot.layout.x_scale.clone(),
        y_scale: plot.layout.y_scale.clone(),
        transform: CoordinateTransform::new(
            visible.x_min..visible.x_max,
            visible.y_min..visible.y_max,
            layout.plot_area_rect.left()..layout.plot_area_rect.right(),
            layout.plot_area_rect.top()..layout.plot_area_rect.bottom(),
        ),
    })
}

fn refresh_hit_result(
    hit: &HitResult,
    plot: &Plot,
    displayed_data: &DisplayedFrameData,
    geometry: &GeometrySnapshot,
) -> Option<HitResult> {
    match hit {
        HitResult::SeriesPoint {
            series_index,
            point_index,
            distance_px,
            ..
        } => {
            let (x, y) = displayed_data.xy(plot, *series_index)?;
            let (x_val, y_val) = (*x.get(*point_index)?, *y.get(*point_index)?);
            if !x_val.is_finite() || !y_val.is_finite() {
                return None;
            }
            let screen_position = geometry.data_to_screen(ViewportPoint::new(x_val, y_val));

            Some(HitResult::SeriesPoint {
                series_index: *series_index,
                point_index: *point_index,
                screen_position,
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
            let series = plot.series_mgr.series.get(*series_index)?;
            let SeriesType::Heatmap { data } = &series.series_type else {
                return None;
            };
            if *row >= data.n_rows || *col >= data.n_cols {
                return None;
            }
            let value = data.values[*row][*col];
            if data.should_mask_value(value) {
                return None;
            }

            let ((x1, x2), (y1, y2)) = data.cell_data_bounds(*row, *col);
            let first = geometry.data_to_screen(ViewportPoint::new(x1, y2));
            let second = geometry.data_to_screen(ViewportPoint::new(x2, y1));
            Some(HitResult::HeatmapCell {
                series_index: *series_index,
                row: *row,
                col: *col,
                value,
                screen_rect: ViewportRect {
                    min: ViewportPoint::new(first.x.min(second.x), first.y.min(second.y)),
                    max: ViewportPoint::new(first.x.max(second.x), first.y.max(second.y)),
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
    let frame = plot.resolve_frame(time_seconds)?;
    compute_plot_layout_from_frame(plot, size_px, scale_factor, time_seconds, visible, &frame)
}

fn compute_plot_layout_from_frame(
    plot: &Plot,
    size_px: (u32, u32),
    scale_factor: f32,
    _time_seconds: f64,
    visible: DataBounds,
    frame: &ResolvedFrame<'_>,
) -> Result<ComputedSessionLayout> {
    let layout_plot = plot.prepared_frame_shell_with_style(size_px, scale_factor, &frame.style);
    layout_plot.validate_runtime_environment()?;
    let dpi = layout_plot.display.config.figure.dpi;

    let mut renderer = SkiaRenderer::with_font_family(
        size_px.0,
        size_px.1,
        layout_plot.display.theme.clone(),
        layout_plot.display.config.typography.family.clone(),
    )?;
    renderer.set_text_engine_mode(layout_plot.display.text_engine);
    renderer.set_render_scale(layout_plot.render_scale());

    let content =
        layout_plot.create_plot_content_from_resolved_text(visible.y_min, visible.y_max, frame);
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

#[cfg(test)]
use crate::render::skia::map_data_to_pixels;

#[cfg(test)]
fn screen_to_data(
    bounds: DataBounds,
    plot_area: tiny_skia::Rect,
    position_px: ViewportPoint,
) -> ViewportPoint {
    let position_px = ViewportPoint::new(
        position_px
            .x
            .clamp(plot_area.left() as f64, plot_area.right() as f64),
        position_px
            .y
            .clamp(plot_area.top() as f64, plot_area.bottom() as f64),
    );
    let transform = CoordinateTransform::new(
        bounds.x_min..bounds.x_max,
        bounds.y_min..bounds.y_max,
        plot_area.left()..plot_area.right(),
        plot_area.top()..plot_area.bottom(),
    );
    let (x, y) = transform.screen_to_data(position_px.x as f32, position_px.y as f32);
    ViewportPoint::new(x, y)
}
