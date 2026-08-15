use super::*;
use crate::core::types::Point2f;
use crate::plots::{PlotArea, heatmap::HeatmapData};
use crate::render::{Color, LineStyle, MarkerStyle, skia::SkiaRenderer};
use std::sync::Arc;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

pub(super) type ClipRect = (f32, f32, f32, f32);

#[derive(Debug, Clone)]
pub(super) struct PolylineBatch {
    points: Arc<[Point2f]>,
    color: Color,
    line_width: f32,
    line_style: LineStyle,
    clip_rect: ClipRect,
}

impl PolylineBatch {
    pub(super) fn new(
        points: Arc<[Point2f]>,
        color: Color,
        line_width: f32,
        line_style: LineStyle,
        clip_rect: ClipRect,
    ) -> Self {
        Self {
            points,
            color,
            line_width,
            line_style,
            clip_rect,
        }
    }

    fn execute(&self, renderer: &mut SkiaRenderer) -> Result<()> {
        renderer.draw_polyline_points_clipped(
            self.points.as_ref(),
            self.color,
            self.line_width,
            self.line_style.clone(),
            self.clip_rect,
        )
    }
}

#[derive(Debug, Clone)]
pub(super) struct MarkerBatch {
    points: Arc<[Point2f]>,
    size: f32,
    style: MarkerStyle,
    color: Color,
    /// Marker rim as `(colour, width_in_points)`, or `None` for bare markers.
    ///
    /// The width stays in points here; the renderer scales it, so the rim is
    /// DPI-invariant. An edged batch cannot use the marker sprite compositor
    /// (its cache key has nowhere to record an edge), so keep this `None`
    /// unless the series actually asked for a rim.
    edge: Option<(Color, f32)>,
    clip_rect: ClipRect,
}

impl MarkerBatch {
    pub(super) fn new(
        points: Arc<[Point2f]>,
        size: f32,
        style: MarkerStyle,
        color: Color,
        edge: Option<(Color, f32)>,
        clip_rect: ClipRect,
    ) -> Self {
        Self {
            points,
            size,
            style,
            color,
            edge,
            clip_rect,
        }
    }

    fn execute(&self, renderer: &mut SkiaRenderer) -> Result<()> {
        renderer.draw_markers_styled_clipped(
            self.points.as_ref(),
            self.size,
            self.style,
            self.color,
            self.edge,
            self.clip_rect,
        )
    }
}

/// A plot-area density grid for one opt-in scatter series.
///
/// Counts are retained rather than expanded into projected points. Executing
/// the batch converts at most one plot pixel per bin and composites the result
/// as a single image, keeping both aggregation and drawing independent of the
/// number of input samples after the direct pass below.
#[derive(Debug, Clone)]
pub(super) struct DensityBatch {
    counts: Arc<[u32]>,
    width: u32,
    height: u32,
    color: Color,
    plot_area: tiny_skia::Rect,
}

#[derive(Clone, Copy)]
struct DensityGridSpec {
    width: usize,
    height: usize,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    x_scale: crate::axes::AxisScale,
    y_scale: crate::axes::AxisScale,
}

impl DensityGridSpec {
    fn len(self) -> usize {
        self.width.saturating_mul(self.height)
    }

    /// Project one raw coordinate pair directly to its density-grid cell.
    ///
    /// The normalized positions are the same ones used by the regular marker
    /// projection. The upper/right endpoints belong to the final cell, while
    /// samples outside manual axis limits are clipped before aggregation.
    #[inline]
    fn bin_index(self, x: f64, y: f64) -> Option<usize> {
        if !sample_is_representable(x, y, &self.x_scale, &self.y_scale) {
            return None;
        }

        let normalized_x = self.x_scale.normalized_position(x, self.x_min, self.x_max);
        let normalized_y = self.y_scale.normalized_position(y, self.y_min, self.y_max);
        let col = density_axis_bin(normalized_x, self.width)?;
        let row = density_axis_bin(1.0 - normalized_y, self.height)?;
        Some(row * self.width + col)
    }
}

#[inline]
fn density_axis_bin(normalized: f64, bins: usize) -> Option<usize> {
    if !normalized.is_finite() || !(0.0..=1.0).contains(&normalized) || bins == 0 {
        return None;
    }
    Some(((normalized * bins as f64) as usize).min(bins - 1))
}

fn zeroed_density_grid(len: usize) -> Vec<u32> {
    vec![0; len]
}

#[cfg(any(not(feature = "parallel"), test))]
fn aggregate_density_counts_serial(
    x_data: &[f64],
    y_data: &[f64],
    spec: DensityGridSpec,
) -> Vec<u32> {
    let mut counts = zeroed_density_grid(spec.len());
    for (&x, &y) in x_data.iter().zip(y_data) {
        if let Some(index) = spec.bin_index(x, y) {
            counts[index] = counts[index].saturating_add(1);
        }
    }
    counts
}

#[cfg(feature = "parallel")]
fn aggregate_density_counts_parallel(
    x_data: &[f64],
    y_data: &[f64],
    spec: DensityGridSpec,
) -> Vec<u32> {
    let grid_len = spec.len();
    let sample_count = x_data.len().min(y_data.len());
    let min_samples_per_grid = sample_count.div_ceil(rayon::current_num_threads()).max(1);
    x_data
        .par_iter()
        .zip(y_data.par_iter())
        .with_min_len(min_samples_per_grid)
        .fold(
            || zeroed_density_grid(grid_len),
            |mut counts, (&x, &y)| {
                if let Some(index) = spec.bin_index(x, y) {
                    counts[index] = counts[index].saturating_add(1);
                }
                counts
            },
        )
        .reduce(
            || zeroed_density_grid(grid_len),
            |mut left, right| {
                for (left_count, right_count) in left.iter_mut().zip(right) {
                    *left_count = left_count.saturating_add(right_count);
                }
                left
            },
        )
}

fn aggregate_density_counts(x_data: &[f64], y_data: &[f64], spec: DensityGridSpec) -> Vec<u32> {
    #[cfg(feature = "parallel")]
    {
        aggregate_density_counts_parallel(x_data, y_data, spec)
    }
    #[cfg(not(feature = "parallel"))]
    {
        aggregate_density_counts_serial(x_data, y_data, spec)
    }
}

impl DensityBatch {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn from_xy(
        x_data: &[f64],
        y_data: &[f64],
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        plot_area: tiny_skia::Rect,
        x_scale: &crate::axes::AxisScale,
        y_scale: &crate::axes::AxisScale,
        color: Color,
    ) -> Self {
        let width = plot_area.width().ceil().max(1.0) as usize;
        let height = plot_area.height().ceil().max(1.0) as usize;
        let spec = DensityGridSpec {
            width,
            height,
            x_min,
            x_max,
            y_min,
            y_max,
            x_scale: *x_scale,
            y_scale: *y_scale,
        };

        Self {
            counts: aggregate_density_counts(x_data, y_data, spec).into(),
            width: width as u32,
            height: height as u32,
            color,
            plot_area,
        }
    }

    fn execute(&self, renderer: &mut SkiaRenderer) -> Result<()> {
        renderer.draw_density_grid(
            self.counts.as_ref(),
            self.width,
            self.height,
            self.color,
            self.plot_area,
        )
    }
}

#[derive(Debug, Clone)]
pub(super) struct RectGridBatch {
    x_edges: Arc<[i32]>,
    y_edges: Arc<[i32]>,
    colors: Arc<[Option<Color>]>,
    n_rows: usize,
    n_cols: usize,
    cell_borders: bool,
}

impl RectGridBatch {
    pub(super) fn from_heatmap_data(
        data: &HeatmapData,
        area: PlotArea,
        alpha: f32,
    ) -> Option<Self> {
        if !data.can_use_pixel_aligned_grid_fast_path(alpha) {
            return None;
        }

        let (x_edges, y_edges) = data.pixel_aligned_screen_edges(&area);
        let colors = data
            .values
            .iter()
            .flat_map(|row| row.iter())
            .map(|&value| {
                (!data.should_mask_value(value)).then(|| data.get_color(value).with_alpha(alpha))
            })
            .collect::<Vec<_>>();

        Some(Self {
            x_edges: x_edges.into(),
            y_edges: y_edges.into(),
            colors: colors.into(),
            n_rows: data.n_rows,
            n_cols: data.n_cols,
            cell_borders: data.config.cell_borders,
        })
    }

    fn execute(&self, renderer: &mut SkiaRenderer) -> Result<()> {
        for row in 0..self.n_rows {
            let top = self.y_edges[row].min(self.y_edges[row + 1]);
            let bottom = self.y_edges[row].max(self.y_edges[row + 1]);
            if bottom <= top {
                continue;
            }

            for col in 0..self.n_cols {
                let Some(cell_color) = self.colors[row * self.n_cols + col] else {
                    continue;
                };
                let left = self.x_edges[col].min(self.x_edges[col + 1]);
                let right = self.x_edges[col].max(self.x_edges[col + 1]);
                if right <= left {
                    continue;
                }

                let x = left as f32;
                let y = top as f32;
                let width = (right - left) as f32;
                let height = (bottom - top) as f32;
                renderer.draw_pixel_aligned_solid_rectangle(x, y, width, height, cell_color)?;

                if self.cell_borders {
                    renderer.draw_pixel_aligned_rectangle_outline(
                        x,
                        y,
                        width,
                        height,
                        cell_color.darken(0.2),
                    )?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(super) enum StaticRasterBatch {
    Polyline(PolylineBatch),
    Markers(MarkerBatch),
    Density(DensityBatch),
    RectGrid(RectGridBatch),
}

impl StaticRasterBatch {
    fn execute(&self, renderer: &mut SkiaRenderer) -> Result<()> {
        match self {
            Self::Polyline(batch) => batch.execute(renderer),
            Self::Markers(batch) => batch.execute(renderer),
            Self::Density(batch) => batch.execute(renderer),
            Self::RectGrid(batch) => batch.execute(renderer),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct SeriesRasterPlan {
    batches: Vec<StaticRasterBatch>,
    used_exact_line_canonicalization: bool,
    used_raster_line_reduction: bool,
}

impl SeriesRasterPlan {
    pub(super) fn push_polyline(
        &mut self,
        points: Arc<[Point2f]>,
        color: Color,
        line_width: f32,
        line_style: LineStyle,
        clip_rect: ClipRect,
    ) {
        self.batches
            .push(StaticRasterBatch::Polyline(PolylineBatch::new(
                points, color, line_width, line_style, clip_rect,
            )));
    }

    /// Queue a marker batch. `edge` is `(colour, width_in_points)`, normally
    /// `Plot::resolved_marker_edge`, and `None` for bare markers.
    pub(super) fn push_markers(
        &mut self,
        points: Arc<[Point2f]>,
        size: f32,
        style: MarkerStyle,
        color: Color,
        edge: Option<(Color, f32)>,
        clip_rect: ClipRect,
    ) {
        self.batches
            .push(StaticRasterBatch::Markers(MarkerBatch::new(
                points, size, style, color, edge, clip_rect,
            )));
    }

    pub(super) fn push_density(&mut self, batch: DensityBatch) {
        self.batches.push(StaticRasterBatch::Density(batch));
    }

    pub(super) fn push_rect_grid(&mut self, batch: RectGridBatch) {
        self.batches.push(StaticRasterBatch::RectGrid(batch));
    }

    pub(super) fn note_exact_line_canonicalization(&mut self) {
        self.used_exact_line_canonicalization = true;
    }

    pub(super) fn note_raster_line_reduction(&mut self) {
        self.used_raster_line_reduction = true;
    }

    pub(super) fn execute(&self, renderer: &mut SkiaRenderer) -> Result<()> {
        if self.used_exact_line_canonicalization {
            renderer.note_exact_line_canonicalization();
        }
        if self.used_raster_line_reduction {
            renderer.note_raster_line_reduction();
        }

        for batch in &self.batches {
            batch.execute(renderer)?;
        }
        Ok(())
    }
}

pub(super) fn clip_rect_from_plot_area(plot_area: tiny_skia::Rect) -> ClipRect {
    (
        plot_area.x(),
        plot_area.y(),
        plot_area.width(),
        plot_area.height(),
    )
}

/// Can this sample be placed on these axes at all?
///
/// Defers to [`crate::axes::AxisScale::is_valid_value`] — the very predicate
/// the scales use — so a renderer can never disagree with its own axis about
/// which samples exist. Rejects a non-finite value on any scale and a
/// zero/negative value on a log scale.
pub(super) fn sample_is_representable(
    x: f64,
    y: f64,
    x_scale: &crate::axes::AxisScale,
    y_scale: &crate::axes::AxisScale,
) -> bool {
    x_scale.is_valid_value(x) && y_scale.is_valid_value(y)
}

/// The contiguous runs of representable samples, as index ranges.
///
/// **This is the one place a line's breaks are decided.** The raster, SVG and
/// parallel backends all split on these ranges, so they cannot disagree about
/// where a polyline stops and restarts. Ranges rather than points because each
/// backend owns its projected buffer differently.
///
/// An all-representable input yields a single `0..len` range, so the common
/// case costs one pass and no allocation beyond the `Vec`.
pub(super) fn representable_sample_runs(
    x_data: &[f64],
    y_data: &[f64],
    x_scale: &crate::axes::AxisScale,
    y_scale: &crate::axes::AxisScale,
) -> Vec<std::ops::Range<usize>> {
    let len = x_data.len().min(y_data.len());
    let mut runs = Vec::new();
    let mut start = None;

    for index in 0..len {
        if sample_is_representable(x_data[index], y_data[index], x_scale, y_scale) {
            start.get_or_insert(index);
        } else if let Some(run_start) = start.take() {
            runs.push(run_start..index);
        }
    }
    if let Some(run_start) = start {
        runs.push(run_start..len);
    }

    runs
}

/// Is every sample representable on these axes?
fn all_samples_representable(
    x_data: &[f64],
    y_data: &[f64],
    x_scale: &crate::axes::AxisScale,
    y_scale: &crate::axes::AxisScale,
) -> bool {
    x_data
        .iter()
        .zip(y_data.iter())
        .all(|(&x, &y)| sample_is_representable(x, y, x_scale, y_scale))
}

/// Project x/y samples into pixel space, **dropping** samples the axis scales
/// cannot represent.
///
/// This is the marker/scatter projection: a sample with no position on the axis
/// simply is not drawn. Line series must use [`project_xy_subpaths`] instead —
/// dropping a sample from a polyline silently joins the line across the gap,
/// inventing a segment the user never supplied.
pub(super) fn project_xy_points(
    x_data: &[f64],
    y_data: &[f64],
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    plot_area: tiny_skia::Rect,
    x_scale: &crate::axes::AxisScale,
    y_scale: &crate::axes::AxisScale,
) -> Arc<[Point2f]> {
    let projected = project_xy_points_unchecked(
        x_data, y_data, x_min, x_max, y_min, y_max, plot_area, x_scale, y_scale,
    );

    // Every render path validates both arrays as finite before projection, and
    // finiteness is the complete representability rule for linear axes. Log
    // and mixed-scale paths still need the scan for finite zero/negative data.
    if matches!(x_scale, crate::axes::AxisScale::Linear)
        && matches!(y_scale, crate::axes::AxisScale::Linear)
    {
        return projected;
    }

    if all_samples_representable(x_data, y_data, x_scale, y_scale) {
        return projected;
    }

    x_data
        .iter()
        .zip(y_data.iter())
        .zip(projected.iter())
        .filter(|&((&x, &y), _)| sample_is_representable(x, y, x_scale, y_scale))
        .map(|(_, point)| *point)
        .collect::<Vec<_>>()
        .into()
}

/// Project x/y samples into pixel space, **splitting** at every sample the axis
/// scales cannot represent.
///
/// Each returned run is one contiguous sub-path; drawing them as separate
/// polylines is what makes a line *break* at the gap instead of jumping across
/// it. The common case — every sample representable — returns a single run and
/// copies nothing.
///
/// This is the one place the split is decided, so the raster, SVG and parallel
/// backends cannot disagree about where a line breaks.
///
/// # A sample isolated between two rejected ones draws nothing
///
/// A run of length one is a zero-length polyline, and a line has no ink of its
/// own at a single point — matplotlib draws nothing there either. So a valid
/// sample whose *both* neighbours are off the axis is not visible as a line.
/// It is not lost: the marker path ([`project_xy_points`]) keeps every
/// representable sample, so `.line(x, y).marker(..)` still shows it, and so
/// does `.scatter(x, y)`. A marker-less line on data with isolated survivors
/// should ask for markers.
pub(super) fn project_xy_subpaths(
    x_data: &[f64],
    y_data: &[f64],
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    plot_area: tiny_skia::Rect,
    x_scale: &crate::axes::AxisScale,
    y_scale: &crate::axes::AxisScale,
) -> Vec<Arc<[Point2f]>> {
    let projected = project_xy_points_unchecked(
        x_data, y_data, x_min, x_max, y_min, y_max, plot_area, x_scale, y_scale,
    );

    // See `project_xy_points`: validation already establishes the only
    // representability condition linear axes impose.
    if matches!(x_scale, crate::axes::AxisScale::Linear)
        && matches!(y_scale, crate::axes::AxisScale::Linear)
    {
        return if projected.is_empty() {
            Vec::new()
        } else {
            vec![projected]
        };
    }

    if all_samples_representable(x_data, y_data, x_scale, y_scale) {
        return if projected.is_empty() {
            Vec::new()
        } else {
            vec![projected]
        };
    }

    representable_sample_runs(x_data, y_data, x_scale, y_scale)
        .into_iter()
        .map(|run| Arc::from(&projected[run]))
        .collect()
}

/// Project every sample, representable or not.
///
/// A rejected sample comes back as a `NaN` pixel pair. Only
/// [`project_xy_points`] and [`project_xy_subpaths`] may call this; every other
/// caller must go through one of them so that rejected samples are dropped or
/// split on rather than rasterised.
fn project_xy_points_unchecked(
    x_data: &[f64],
    y_data: &[f64],
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    plot_area: tiny_skia::Rect,
    x_scale: &crate::axes::AxisScale,
    y_scale: &crate::axes::AxisScale,
) -> Arc<[Point2f]> {
    if matches!(x_scale, crate::axes::AxisScale::Linear)
        && matches!(y_scale, crate::axes::AxisScale::Linear)
    {
        return project_linear_xy_points(x_data, y_data, x_min, x_max, y_min, y_max, plot_area);
    }

    project_scaled_xy_points(
        x_data, y_data, x_min, x_max, y_min, y_max, plot_area, x_scale, y_scale,
    )
}

fn project_linear_xy_points(
    x_data: &[f64],
    y_data: &[f64],
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    plot_area: tiny_skia::Rect,
) -> Arc<[Point2f]> {
    let x_range = x_max - x_min;
    let y_range = y_max - y_min;
    let x_is_degenerate = crate::axes::scale::linear_range_is_degenerate(x_range);
    let y_is_degenerate = crate::axes::scale::linear_range_is_degenerate(y_range);
    let left = plot_area.left();
    let bottom = plot_area.bottom();
    let width = plot_area.width();
    let height = plot_area.height();

    x_data
        .iter()
        .zip(y_data.iter())
        .map(|(&x, &y)| {
            let normalized_x = if x_is_degenerate {
                0.5
            } else {
                crate::axes::scale::linear_normalized_position_with_range(x, x_min, x_max, x_range)
            };
            let normalized_y = if y_is_degenerate {
                0.5
            } else {
                crate::axes::scale::linear_normalized_position_with_range(y, y_min, y_max, y_range)
            };
            Point2f::new(
                left + normalized_x as f32 * width,
                bottom - normalized_y as f32 * height,
            )
        })
        .collect::<Vec<_>>()
        .into()
}

fn project_scaled_xy_points(
    x_data: &[f64],
    y_data: &[f64],
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    plot_area: tiny_skia::Rect,
    x_scale: &crate::axes::AxisScale,
    y_scale: &crate::axes::AxisScale,
) -> Arc<[Point2f]> {
    let transform = crate::core::CoordinateTransform::from_plot_area(
        plot_area.left(),
        plot_area.top(),
        plot_area.width(),
        plot_area.height(),
        x_min,
        x_max,
        y_min,
        y_max,
    );

    x_data
        .iter()
        .zip(y_data.iter())
        .map(|(&x, &y)| {
            let (px, py) = transform.data_to_screen_scaled(x, y, x_scale, y_scale);
            Point2f::new(px, py)
        })
        .collect::<Vec<_>>()
        .into()
}

/// Build the [`PlotArea`] a `PlotRender` implementation draws through.
///
/// This is the only place a `PlotArea` is derived from a pixel rect plus data
/// bounds on the raster path, so every trait-rendered plot type is projected
/// through the figure's axis scales rather than through a linear default.
pub(super) fn plot_area_from_rect(
    plot_area: tiny_skia::Rect,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    x_scale: &crate::axes::AxisScale,
    y_scale: &crate::axes::AxisScale,
) -> PlotArea {
    PlotArea::new(
        plot_area.x(),
        plot_area.y(),
        plot_area.width(),
        plot_area.height(),
        x_min,
        x_max,
        y_min,
        y_max,
    )
    .with_scales(*x_scale, *y_scale)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::AxisScale;

    fn density_spec(width: usize, height: usize) -> DensityGridSpec {
        DensityGridSpec {
            width,
            height,
            x_min: 0.0,
            x_max: width as f64,
            y_min: 0.0,
            y_max: height as f64,
            x_scale: AxisScale::Linear,
            y_scale: AxisScale::Linear,
        }
    }

    #[test]
    fn test_density_aggregator_counts_hand_checked_grid() {
        let x = [0.0, 1.0, 3.999, 4.0, 2.0, -1.0];
        let y = [3.0, 2.0, 0.0, 3.0, 1.5, 1.0];

        let counts = aggregate_density_counts_serial(&x, &y, density_spec(4, 3));

        assert_eq!(
            counts,
            vec![
                1, 0, 0, 1, // top row: (0, 3) and inclusive (4, 3)
                0, 1, 1, 0, // middle row: (1, 2) and (2, 1.5)
                0, 0, 0, 1, // bottom row: (3.999, 0)
            ]
        );
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn test_density_parallel_aggregation_matches_serial() {
        let x = (0..50_000)
            .map(|index| ((index * 37) % 1_009) as f64 / 1_008.0 * 64.0)
            .collect::<Vec<_>>();
        let y = (0..50_000)
            .map(|index| ((index * 91) % 1_013) as f64 / 1_012.0 * 48.0)
            .collect::<Vec<_>>();
        let spec = density_spec(64, 48);

        let serial = aggregate_density_counts_serial(&x, &y, spec);
        let parallel = aggregate_density_counts_parallel(&x, &y, spec);

        assert_eq!(parallel, serial);
    }

    #[test]
    fn test_linear_projection_fast_path_matches_scaled_mapper() {
        let plot_area = tiny_skia::Rect::from_xywh(73.5, 41.25, 503.75, 318.5);
        assert!(plot_area.is_some(), "test rectangle should be valid");
        let Some(plot_area) = plot_area else {
            return;
        };
        let x_data = [-2.0, -0.5, 0.0, 1.25, 4.0, 7.5];
        let y_data = [8.0, 1.5, -2.0, 0.25, 3.0, 5.5];

        let points = project_xy_points(
            &x_data,
            &y_data,
            -2.0,
            7.5,
            -2.0,
            8.0,
            plot_area,
            &AxisScale::Linear,
            &AxisScale::Linear,
        );

        let expected = x_data
            .iter()
            .zip(y_data.iter())
            .map(|(&x, &y)| {
                let (px, py) = crate::render::skia::map_data_to_pixels_scaled(
                    x,
                    y,
                    -2.0,
                    7.5,
                    -2.0,
                    8.0,
                    plot_area,
                    &AxisScale::Linear,
                    &AxisScale::Linear,
                );
                Point2f::new(px, py)
            })
            .collect::<Vec<_>>();

        assert_eq!(points.as_ref(), expected.as_slice());
    }

    #[test]
    fn test_linear_projection_fast_path_matches_degenerate_axis_mapper() {
        let plot_area = tiny_skia::Rect::from_xywh(12.0, 18.0, 320.0, 240.0);
        assert!(plot_area.is_some(), "test rectangle should be valid");
        let Some(plot_area) = plot_area else {
            return;
        };
        let x_data = [1.0, 1.0, 1.0];
        let y_data = [-1.0, 0.0, 1.0];

        let points = project_xy_points(
            &x_data,
            &y_data,
            1.0,
            1.0,
            -1.0,
            1.0,
            plot_area,
            &AxisScale::Linear,
            &AxisScale::Linear,
        );

        let expected = x_data
            .iter()
            .zip(y_data.iter())
            .map(|(&x, &y)| {
                let (px, py) = crate::render::skia::map_data_to_pixels_scaled(
                    x,
                    y,
                    1.0,
                    1.0,
                    -1.0,
                    1.0,
                    plot_area,
                    &AxisScale::Linear,
                    &AxisScale::Linear,
                );
                Point2f::new(px, py)
            })
            .collect::<Vec<_>>();

        assert_eq!(points.as_ref(), expected.as_slice());
    }

    #[test]
    fn test_scaled_projection_uses_core_transform_with_reversed_ranges() {
        let plot_area = tiny_skia::Rect::from_xywh(20.0, 30.0, 600.0, 400.0);
        assert!(plot_area.is_some(), "test rectangle should be valid");
        let Some(plot_area) = plot_area else {
            return;
        };
        let x_data = [100.0, 10.0, 1.0];
        let y_data = [100.0, 0.0, -100.0];

        let points = project_xy_points(
            &x_data,
            &y_data,
            100.0,
            1.0,
            100.0,
            -100.0,
            plot_area,
            &AxisScale::Log,
            &AxisScale::symlog(1.0),
        );

        let expected = [
            Point2f::new(20.0, 430.0),
            Point2f::new(320.0, 230.0),
            Point2f::new(620.0, 30.0),
        ];
        assert_eq!(points.as_ref(), expected.as_slice());
    }

    #[test]
    fn test_linear_projection_fast_path_uses_shared_epsilon_and_extreme_range_rules() {
        let plot_area = tiny_skia::Rect::from_xywh(10.0, 20.0, 200.0, 100.0);
        assert!(plot_area.is_some(), "test rectangle should be valid");
        let Some(plot_area) = plot_area else {
            return;
        };
        let x_data = [0.0, f64::EPSILON / 2.0, f64::EPSILON];
        let y_data = [-f64::MAX, 0.0, f64::MAX];

        let points = project_xy_points(
            &x_data,
            &y_data,
            0.0,
            f64::EPSILON,
            -f64::MAX,
            f64::MAX,
            plot_area,
            &AxisScale::Linear,
            &AxisScale::Linear,
        );

        let expected = [
            Point2f::new(10.0, 120.0),
            Point2f::new(110.0, 70.0),
            Point2f::new(210.0, 20.0),
        ];
        assert_eq!(points.as_ref(), expected.as_slice());
    }

    /// A sample the axes cannot place must break the line, not be dropped
    /// silently — dropping it joins the two sides together and draws a segment
    /// the user never supplied.
    #[test]
    fn test_log_axis_gaps_split_the_polyline_instead_of_joining_across() {
        let plot_area = tiny_skia::Rect::from_xywh(0.0, 0.0, 100.0, 100.0);
        let Some(plot_area) = plot_area else {
            unreachable!("test rectangle should be valid");
        };
        // The 0.0 and -5.0 samples have no position on a log y axis.
        let x_data = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y_data = [1.0, 0.0, 10.0, -5.0, 100.0];

        let subpaths = project_xy_subpaths(
            &x_data,
            &y_data,
            1.0,
            5.0,
            1.0,
            100.0,
            plot_area,
            &AxisScale::Linear,
            &AxisScale::Log,
        );

        assert_eq!(
            subpaths.len(),
            3,
            "each rejected sample must break the line"
        );
        assert_eq!(subpaths[0].len(), 1);
        assert_eq!(subpaths[1].len(), 1);
        assert_eq!(subpaths[2].len(), 1);
        for subpath in &subpaths {
            for point in subpath.iter() {
                assert!(
                    point.x.is_finite() && point.y.is_finite(),
                    "no sub-path may contain a NaN pixel"
                );
            }
        }
    }

    #[test]
    fn test_representable_runs_cover_leading_trailing_and_interior_gaps() {
        let log = AxisScale::Log;
        let linear = AxisScale::Linear;

        assert_eq!(
            representable_sample_runs(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0], &linear, &log),
            vec![0..3],
            "an all-valid series is one unbroken run"
        );
        assert_eq!(
            representable_sample_runs(&[1.0, 2.0, 3.0], &[0.0, 2.0, 3.0], &linear, &log),
            vec![1..3],
            "a leading gap must not produce an empty run"
        );
        assert_eq!(
            representable_sample_runs(&[1.0, 2.0, 3.0], &[1.0, 2.0, 0.0], &linear, &log),
            vec![0..2],
            "a trailing gap must not produce an empty run"
        );
        assert_eq!(
            representable_sample_runs(&[1.0, 2.0, 3.0, 4.0], &[1.0, 0.0, 0.0, 4.0], &linear, &log),
            vec![0..1, 3..4],
            "adjacent gaps collapse into one break"
        );
        assert!(
            representable_sample_runs(&[1.0, 2.0], &[0.0, -1.0], &linear, &log).is_empty(),
            "a wholly unrepresentable series draws nothing"
        );
    }

    /// NaN is unrepresentable on *every* scale, so a linear plot must break at
    /// a NaN too rather than relying on the rasteriser to swallow it.
    #[test]
    fn test_non_finite_samples_break_a_linear_polyline() {
        assert_eq!(
            representable_sample_runs(
                &[1.0, 2.0, 3.0],
                &[1.0, f64::NAN, 3.0],
                &AxisScale::Linear,
                &AxisScale::Linear,
            ),
            vec![0..1, 2..3]
        );
    }

    /// Markers get the drop semantics, not the split semantics: a rejected
    /// sample simply is not drawn, and the surviving markers keep their pixels.
    #[test]
    fn test_marker_projection_drops_unrepresentable_samples() {
        let plot_area = tiny_skia::Rect::from_xywh(0.0, 0.0, 100.0, 100.0);
        let Some(plot_area) = plot_area else {
            unreachable!("test rectangle should be valid");
        };
        let x_data = [1.0, 2.0, 3.0];
        let y_data = [1.0, 0.0, 100.0];

        let points = project_xy_points(
            &x_data,
            &y_data,
            1.0,
            3.0,
            1.0,
            100.0,
            plot_area,
            &AxisScale::Linear,
            &AxisScale::Log,
        );

        assert_eq!(points.len(), 2, "the log-invalid sample must be dropped");
        assert!(points.iter().all(|p| p.x.is_finite() && p.y.is_finite()));
    }
}
