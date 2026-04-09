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

#[derive(Debug, Clone, Copy)]
pub(super) struct RectGridBatch<'a> {
    data: &'a HeatmapData,
    area: PlotArea,
    alpha: f32,
}

impl<'a> RectGridBatch<'a> {
    pub(super) fn new(data: &'a HeatmapData, area: PlotArea, alpha: f32) -> Self {
        Self { data, area, alpha }
    }

    fn execute(&self, renderer: &mut SkiaRenderer) -> Result<()> {
        self.data.draw_cells_batch(renderer, &self.area, self.alpha)
    }
}

#[derive(Debug, Clone)]
pub(super) enum StaticRasterBatch<'a> {
    Polyline(PolylineBatch),
    Markers(MarkerBatch),
    RectGrid(RectGridBatch<'a>),
}

impl StaticRasterBatch<'_> {
    fn execute(&self, renderer: &mut SkiaRenderer) -> Result<()> {
        match self {
            Self::Polyline(batch) => batch.execute(renderer),
            Self::Markers(batch) => batch.execute(renderer),
            Self::RectGrid(batch) => batch.execute(renderer),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct SeriesRasterPlan<'a> {
    batches: Vec<StaticRasterBatch<'a>>,
}

impl<'a> SeriesRasterPlan<'a> {
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

    pub(super) fn push_rect_grid(&mut self, data: &'a HeatmapData, area: PlotArea, alpha: f32) {
        self.batches
            .push(StaticRasterBatch::RectGrid(RectGridBatch::new(
                data, area, alpha,
            )));
    }

    pub(super) fn execute(self, renderer: &mut SkiaRenderer) -> Result<()> {
        for batch in self.batches {
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
) -> Arc<[Point2f]> {
    x_data
        .iter()
        .zip(y_data.iter())
        .map(|(&x, &y)| {
            let (px, py) = crate::render::skia::map_data_to_pixels(
                x, y, x_min, x_max, y_min, y_max, plot_area,
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
