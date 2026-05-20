use super::*;
use crate::core::types::Point2f;
use crate::plots::{PlotArea, heatmap::HeatmapData};
use crate::render::{Color, LineStyle, MarkerStyle, skia::SkiaRenderer};
use std::sync::Arc;

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
    clip_rect: ClipRect,
}

impl MarkerBatch {
    pub(super) fn new(
        points: Arc<[Point2f]>,
        size: f32,
        style: MarkerStyle,
        color: Color,
        clip_rect: ClipRect,
    ) -> Self {
        Self {
            points,
            size,
            style,
            color,
            clip_rect,
        }
    }

    fn execute(&self, renderer: &mut SkiaRenderer) -> Result<()> {
        renderer.draw_markers_clipped(
            self.points.as_ref(),
            self.size,
            self.style,
            self.color,
            self.clip_rect,
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
    RectGrid(RectGridBatch),
}

impl StaticRasterBatch {
    fn execute(&self, renderer: &mut SkiaRenderer) -> Result<()> {
        match self {
            Self::Polyline(batch) => batch.execute(renderer),
            Self::Markers(batch) => batch.execute(renderer),
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

    pub(super) fn push_markers(
        &mut self,
        points: Arc<[Point2f]>,
        size: f32,
        style: MarkerStyle,
        color: Color,
        clip_rect: ClipRect,
    ) {
        self.batches
            .push(StaticRasterBatch::Markers(MarkerBatch::new(
                points, size, style, color, clip_rect,
            )));
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
    let x_is_degenerate = x_range.abs() < f64::EPSILON;
    let y_is_degenerate = y_range.abs() < f64::EPSILON;
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
                (x - x_min) / x_range
            };
            let normalized_y = if y_is_degenerate {
                0.5
            } else {
                (y - y_min) / y_range
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
    x_data
        .iter()
        .zip(y_data.iter())
        .map(|(&x, &y)| {
            let (px, py) = crate::render::skia::map_data_to_pixels_scaled(
                x, y, x_min, x_max, y_min, y_max, plot_area, x_scale, y_scale,
            );
            Point2f::new(px, py)
        })
        .collect::<Vec<_>>()
        .into()
}

pub(super) fn plot_area_from_rect(
    plot_area: tiny_skia::Rect,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::AxisScale;

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
}
