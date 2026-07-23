use std::sync::Arc;

use glam::{Vec2, Vec3, Vec4};
#[cfg(feature = "parallel")]
use rayon::prelude::*;

use crate::core::plot::Image;
use crate::core::plot3d::layout::{Axis3Layout, Viewport3D};
use crate::core::{PlottingError, Result};
use crate::plots::SurfaceShading;
use crate::render::three_d::scene::{MeshColor3D, Scene3D};
use crate::render::{Color, ColorMap, LineStyle, MarkerStyle};

use super::clip::{ClipVertex3D, clip_segment, clip_triangle, is_inside_clip_volume};
use super::shading::shade;

const TILE_SIZE: u32 = 32;
const MAX_DEPTH_24: f32 = 16_777_215.0;
const EXPORT_SAMPLE_OFFSETS: [Vec2; 4] = [
    Vec2::new(0.375, 0.125),
    Vec2::new(0.875, 0.375),
    Vec2::new(0.125, 0.625),
    Vec2::new(0.625, 0.875),
];
const INTERACTIVE_SAMPLE_OFFSETS: [Vec2; 1] = [Vec2::new(0.5, 0.5)];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SoftwareQuality3D {
    Interactive,
    Export,
}

impl SoftwareQuality3D {
    fn sample_offsets(self) -> &'static [Vec2] {
        match self {
            Self::Interactive => &INTERACTIVE_SAMPLE_OFFSETS,
            Self::Export => &EXPORT_SAMPLE_OFFSETS,
        }
    }

    pub(crate) fn sample_count(self) -> u32 {
        self.sample_offsets().len() as u32
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SoftwareRenderOptions3D {
    pub(crate) quality: SoftwareQuality3D,
    pub(crate) parallel: bool,
}

impl SoftwareRenderOptions3D {
    pub(crate) fn interactive() -> Self {
        Self {
            quality: SoftwareQuality3D::Interactive,
            parallel: cfg!(feature = "parallel"),
        }
    }

    pub(crate) fn export() -> Self {
        Self {
            quality: SoftwareQuality3D::Export,
            parallel: cfg!(feature = "parallel"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct SoftwareRenderOutput3D {
    pub(crate) layer: Image,
    pub(crate) draw_calls: u64,
    pub(crate) primitives_culled: u64,
}

#[derive(Clone)]
struct MeshMaterial3D {
    color: MeshMaterialColor3D,
    shading: SurfaceShading,
    two_sided: bool,
}

#[derive(Clone)]
enum MeshMaterialColor3D {
    Solid(Color),
    Scalar(ColorMap),
}

impl MeshMaterial3D {
    fn color(&self, scalar: f32, normal: Vec3) -> Color {
        let color = match &self.color {
            MeshMaterialColor3D::Solid(color) => *color,
            MeshMaterialColor3D::Scalar(colormap) => colormap.sample(f64::from(scalar)),
        };
        shade(color, normal, self.shading, self.two_sided)
    }
}

#[derive(Clone)]
struct LineMaterial3D {
    color: Color,
    width: f32,
    dash_pattern: Option<Arc<[f32]>>,
}

#[derive(Clone, Copy)]
struct PointMaterial3D {
    color: Color,
    radius: f32,
    marker: MarkerStyle,
}

#[derive(Clone, Copy, Debug)]
struct RasterVertex3D {
    screen: Vec2,
    depth: f32,
    inverse_w: f32,
    normal: Vec3,
    scalar: f32,
}

#[derive(Clone)]
struct RasterTriangle3D {
    vertices: [RasterVertex3D; 3],
    material: Arc<MeshMaterial3D>,
    primitive_id: u64,
    bounds: PixelBounds3D,
}

#[derive(Clone)]
struct RasterLine3D {
    start: RasterVertex3D,
    end: RasterVertex3D,
    material: Arc<LineMaterial3D>,
    primitive_id: u64,
    bounds: PixelBounds3D,
}

#[derive(Clone)]
struct RasterPoint3D {
    center: RasterVertex3D,
    material: PointMaterial3D,
    primitive_id: u64,
    bounds: PixelBounds3D,
}

#[derive(Clone)]
enum RasterPrimitive3D {
    Triangle(RasterTriangle3D),
    Line(RasterLine3D),
    Point(RasterPoint3D),
}

impl RasterPrimitive3D {
    fn bounds(&self) -> PixelBounds3D {
        match self {
            Self::Triangle(triangle) => triangle.bounds,
            Self::Line(line) => line.bounds,
            Self::Point(point) => point.bounds,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct PixelBounds3D {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

#[derive(Debug)]
struct TileResult3D {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

struct TileSamples3D {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    sample_count: usize,
    colors: Vec<[u8; 4]>,
    depths: Vec<u32>,
    owners: Vec<u64>,
}

impl TileSamples3D {
    fn new(x: u32, y: u32, width: u32, height: u32, sample_count: usize) -> Self {
        let len = width as usize * height as usize * sample_count;
        Self {
            x,
            y,
            width,
            height,
            sample_count,
            colors: vec![[0; 4]; len],
            depths: vec![u32::MAX; len],
            owners: vec![u64::MAX; len],
        }
    }

    fn write(
        &mut self,
        pixel_x: u32,
        pixel_y: u32,
        sample: usize,
        depth: f32,
        primitive_id: u64,
        color: Color,
    ) {
        if !(0.0..=1.0).contains(&depth) || !depth.is_finite() {
            return;
        }
        let local_x = (pixel_x - self.x) as usize;
        let local_y = (pixel_y - self.y) as usize;
        let index = (local_y * self.width as usize + local_x) * self.sample_count + sample;
        let quantized = quantize_depth(depth);
        if quantized < self.depths[index]
            || (quantized == self.depths[index] && primitive_id < self.owners[index])
        {
            self.depths[index] = quantized;
            self.owners[index] = primitive_id;
            self.colors[index] = [color.r, color.g, color.b, color.a];
        }
    }

    fn resolve(self) -> TileResult3D {
        let pixel_count = self.width as usize * self.height as usize;
        let mut pixels = vec![0; pixel_count * 4];
        for pixel in 0..pixel_count {
            let mut accumulated = [0_u32; 4];
            for sample in 0..self.sample_count {
                let color = self.colors[pixel * self.sample_count + sample];
                for channel in 0..4 {
                    accumulated[channel] += u32::from(color[channel]);
                }
            }
            for channel in 0..4 {
                pixels[pixel * 4 + channel] =
                    (accumulated[channel] / self.sample_count as u32) as u8;
            }
        }
        TileResult3D {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
            pixels,
        }
    }
}

pub(crate) fn render_scene(
    scene: &Scene3D,
    layout: &Axis3Layout,
    dpi: f32,
    options: SoftwareRenderOptions3D,
) -> Result<SoftwareRenderOutput3D> {
    let (primitives, primitives_culled, draw_calls) = prepare_primitives(scene, layout, dpi)?;
    let viewport = layout.viewport;
    let tile_columns = viewport.width.div_ceil(TILE_SIZE);
    let tile_rows = viewport.height.div_ceil(TILE_SIZE);
    let tile_count = checked_area(tile_columns, tile_rows, "3D tile grid")?;
    let mut bins = vec![Vec::<usize>::new(); tile_count];
    for (primitive_index, primitive) in primitives.iter().enumerate() {
        let bounds = primitive.bounds();
        let first_column = (bounds.min_x - viewport.x) / TILE_SIZE;
        let last_column = (bounds.max_x - viewport.x) / TILE_SIZE;
        let first_row = (bounds.min_y - viewport.y) / TILE_SIZE;
        let last_row = (bounds.max_y - viewport.y) / TILE_SIZE;
        for row in first_row..=last_row {
            for column in first_column..=last_column {
                bins[(row * tile_columns + column) as usize].push(primitive_index);
            }
        }
    }

    let render_tile = |tile_index: usize| {
        render_tile(
            tile_index,
            tile_columns,
            viewport,
            &bins[tile_index],
            &primitives,
            options.quality.sample_offsets(),
        )
    };

    #[cfg(feature = "parallel")]
    let tiles: Vec<TileResult3D> = if options.parallel {
        (0..tile_count).into_par_iter().map(render_tile).collect()
    } else {
        (0..tile_count).map(render_tile).collect()
    };
    #[cfg(not(feature = "parallel"))]
    let tiles: Vec<TileResult3D> = {
        let _ = options.parallel;
        (0..tile_count).map(render_tile).collect()
    };

    let image_len = checked_rgba_len(layout.canvas_width, layout.canvas_height)?;
    let mut pixels = vec![0; image_len];
    for tile in tiles {
        for row in 0..tile.height {
            let source_start = row as usize * tile.width as usize * 4;
            let source_end = source_start + tile.width as usize * 4;
            let destination_start =
                ((tile.y + row) as usize * layout.canvas_width as usize + tile.x as usize) * 4;
            let destination_end = destination_start + tile.width as usize * 4;
            pixels[destination_start..destination_end]
                .copy_from_slice(&tile.pixels[source_start..source_end]);
        }
    }

    Ok(SoftwareRenderOutput3D {
        layer: Image::new(layout.canvas_width, layout.canvas_height, pixels),
        draw_calls,
        primitives_culled,
    })
}

fn prepare_primitives(
    scene: &Scene3D,
    layout: &Axis3Layout,
    dpi: f32,
) -> Result<(Vec<RasterPrimitive3D>, u64, u64)> {
    let estimated = scene
        .triangle_count()
        .saturating_add(scene.segment_count())
        .saturating_add(scene.point_count());
    let mut primitives = Vec::with_capacity(estimated);
    let mut culled = 0_u64;
    let mut primitive_id = 0_u64;
    let mut draw_calls = 0_u64;

    // Assign the lowest stable IDs to billboards and strokes so equal-depth
    // surface overlays remain visible without a camera-dependent depth bias.
    for batch in &scene.points {
        if !batch.geometry.positions.is_empty() {
            draw_calls += 1;
        }
        let radius = (batch.style.marker_size * dpi / 72.0 * 0.5).max(0.5);
        let material = PointMaterial3D {
            color: batch.style.color,
            radius,
            marker: batch.style.marker,
        };
        for &position in batch.geometry.positions.iter() {
            let clip = clip_vertex(layout, position, [0.0, 0.0, 1.0], 0.0);
            if !is_inside_clip_volume(clip.clip_position) {
                culled += 1;
                primitive_id = primitive_id.saturating_add(1);
                continue;
            }
            let center = raster_vertex(clip, layout.viewport)?;
            if let Some(bounds) = point_bounds(center.screen, radius, layout.viewport) {
                primitives.push(RasterPrimitive3D::Point(RasterPoint3D {
                    center,
                    material,
                    primitive_id,
                    bounds,
                }));
            } else {
                culled += 1;
            }
            primitive_id = primitive_id.saturating_add(1);
        }
    }

    for batch in &scene.lines {
        if !batch.geometry.segments.is_empty() {
            draw_calls += 1;
        }
        let dash_pattern = batch.style.line_style.to_dash_array().map(|pattern| {
            pattern
                .into_iter()
                .map(|length| (length * dpi / 100.0).max(0.25))
                .collect::<Vec<_>>()
                .into()
        });
        let material = Arc::new(LineMaterial3D {
            color: batch.style.color,
            width: (batch.style.line_width * dpi / 72.0).max(1.0),
            dash_pattern,
        });
        for &[start, end] in batch.geometry.segments.iter() {
            let start_index = start as usize;
            let end_index = end as usize;
            let Some((&start_position, &end_position)) = batch
                .geometry
                .positions
                .get(start_index)
                .zip(batch.geometry.positions.get(end_index))
            else {
                return Err(PlottingError::InvalidTopology3D {
                    reason: "3D line segment references an out-of-range vertex".to_string(),
                });
            };
            let Some([start, end]) = clip_segment(
                clip_vertex(layout, start_position, [0.0, 0.0, 1.0], 0.0),
                clip_vertex(layout, end_position, [0.0, 0.0, 1.0], 0.0),
            ) else {
                culled += 1;
                primitive_id = primitive_id.saturating_add(1);
                continue;
            };
            let start = raster_vertex(start, layout.viewport)?;
            let end = raster_vertex(end, layout.viewport)?;
            let radius = material.width * 0.5;
            if let Some(bounds) = line_bounds(start.screen, end.screen, radius, layout.viewport) {
                primitives.push(RasterPrimitive3D::Line(RasterLine3D {
                    start,
                    end,
                    material: Arc::clone(&material),
                    primitive_id,
                    bounds,
                }));
            } else {
                culled += 1;
            }
            primitive_id = primitive_id.saturating_add(1);
        }
    }

    for batch in &scene.meshes {
        if !batch.geometry.indices.is_empty() {
            draw_calls += 1;
        }
        let material = Arc::new(MeshMaterial3D {
            color: match &batch.style.color {
                MeshColor3D::Solid(color) => MeshMaterialColor3D::Solid(*color),
                MeshColor3D::Scalar {
                    colormap,
                    data_range,
                } => {
                    let _ = data_range;
                    MeshMaterialColor3D::Scalar(colormap.clone())
                }
            },
            shading: batch.style.shading,
            two_sided: batch.style.two_sided,
        });
        for triangle in batch.geometry.indices.chunks_exact(3) {
            let mut clip_vertices = [ClipVertex3D {
                clip_position: Vec4::ZERO,
                local_position: Vec3::ZERO,
                normal: Vec3::Z,
                color: Vec4::ONE,
                scalar: 0.0,
            }; 3];
            for (destination, &index) in clip_vertices.iter_mut().zip(triangle) {
                let Some(vertex) = batch.geometry.vertices.get(index as usize) else {
                    return Err(PlottingError::InvalidTopology3D {
                        reason: "3D mesh index references an out-of-range vertex".to_string(),
                    });
                };
                *destination = clip_vertex(layout, vertex.position, vertex.normal, vertex.scalar);
            }
            let clipped = clip_triangle(clip_vertices);
            if clipped.is_empty() {
                culled += 1;
                primitive_id = primitive_id.saturating_add(1);
                continue;
            }
            for clipped_triangle in clipped {
                let vertices = [
                    raster_vertex(clipped_triangle[0], layout.viewport)?,
                    raster_vertex(clipped_triangle[1], layout.viewport)?,
                    raster_vertex(clipped_triangle[2], layout.viewport)?,
                ];
                if let Some(bounds) = triangle_bounds(vertices, layout.viewport) {
                    primitives.push(RasterPrimitive3D::Triangle(RasterTriangle3D {
                        vertices,
                        material: Arc::clone(&material),
                        primitive_id,
                        bounds,
                    }));
                } else {
                    culled += 1;
                }
                primitive_id = primitive_id.saturating_add(1);
            }
        }
    }

    Ok((primitives, culled, draw_calls))
}

fn clip_vertex(
    layout: &Axis3Layout,
    position: [f32; 3],
    normal: [f32; 3],
    scalar: f32,
) -> ClipVertex3D {
    let local = Vec3::from_array(position);
    ClipVertex3D {
        clip_position: layout.camera.view_projection
            * (local * layout.camera.axis_aspect).extend(1.0),
        local_position: local,
        normal: Vec3::from_array(normal),
        color: Vec4::ONE,
        scalar,
    }
}

fn raster_vertex(vertex: ClipVertex3D, viewport: Viewport3D) -> Result<RasterVertex3D> {
    if !vertex.clip_position.is_finite() || vertex.clip_position.w <= f32::EPSILON {
        return Err(PlottingError::InvalidTopology3D {
            reason: "clipped 3D vertex has an invalid homogeneous divisor".to_string(),
        });
    }
    let inverse_w = vertex.clip_position.w.recip();
    let ndc = vertex.clip_position.truncate() * inverse_w;
    let screen = Vec2::new(
        viewport.x as f32 + (ndc.x * 0.5 + 0.5) * viewport.width as f32,
        viewport.y as f32 + (0.5 - ndc.y * 0.5) * viewport.height as f32,
    );
    if !screen.is_finite() || !ndc.z.is_finite() {
        return Err(PlottingError::InvalidTopology3D {
            reason: "clipped 3D vertex produced non-finite screen coordinates".to_string(),
        });
    }
    Ok(RasterVertex3D {
        screen,
        depth: ndc.z,
        inverse_w,
        normal: vertex.normal,
        scalar: vertex.scalar,
    })
}

fn render_tile(
    tile_index: usize,
    tile_columns: u32,
    viewport: Viewport3D,
    bin: &[usize],
    primitives: &[RasterPrimitive3D],
    sample_offsets: &[Vec2],
) -> TileResult3D {
    let tile_column = tile_index as u32 % tile_columns;
    let tile_row = tile_index as u32 / tile_columns;
    let x = viewport.x + tile_column * TILE_SIZE;
    let y = viewport.y + tile_row * TILE_SIZE;
    let width = TILE_SIZE.min(viewport.x + viewport.width - x);
    let height = TILE_SIZE.min(viewport.y + viewport.height - y);
    let mut samples = TileSamples3D::new(x, y, width, height, sample_offsets.len());
    for &primitive_index in bin {
        match &primitives[primitive_index] {
            RasterPrimitive3D::Triangle(triangle) => {
                rasterize_triangle(triangle, &mut samples, sample_offsets);
            }
            RasterPrimitive3D::Line(line) => {
                rasterize_line(line, &mut samples, sample_offsets);
            }
            RasterPrimitive3D::Point(point) => {
                rasterize_point(point, &mut samples, sample_offsets);
            }
        }
    }
    samples.resolve()
}

fn rasterize_triangle(triangle: &RasterTriangle3D, tile: &mut TileSamples3D, offsets: &[Vec2]) {
    let mut vertices = triangle.vertices;
    let mut area = edge(vertices[0].screen, vertices[1].screen, vertices[2].screen);
    if !area.is_finite() || area.abs() <= f32::EPSILON {
        return;
    }
    if area < 0.0 {
        vertices.swap(1, 2);
        area = -area;
    }
    let top_left = [
        is_top_left(vertices[1].screen, vertices[2].screen),
        is_top_left(vertices[2].screen, vertices[0].screen),
        is_top_left(vertices[0].screen, vertices[1].screen),
    ];
    let bounds = intersection(triangle.bounds, tile_bounds(tile));
    for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
            for (sample_index, offset) in offsets.iter().enumerate() {
                let sample = Vec2::new(x as f32 + offset.x, y as f32 + offset.y);
                let edges = [
                    edge(vertices[1].screen, vertices[2].screen, sample),
                    edge(vertices[2].screen, vertices[0].screen, sample),
                    edge(vertices[0].screen, vertices[1].screen, sample),
                ];
                if !(edge_inside(edges[0], top_left[0])
                    && edge_inside(edges[1], top_left[1])
                    && edge_inside(edges[2], top_left[2]))
                {
                    continue;
                }
                let barycentric = [edges[0] / area, edges[1] / area, edges[2] / area];
                let depth = barycentric[0] * vertices[0].depth
                    + barycentric[1] * vertices[1].depth
                    + barycentric[2] * vertices[2].depth;
                let denominator = barycentric[0] * vertices[0].inverse_w
                    + barycentric[1] * vertices[1].inverse_w
                    + barycentric[2] * vertices[2].inverse_w;
                if !denominator.is_finite() || denominator.abs() <= f32::EPSILON {
                    continue;
                }
                let scalar = (barycentric[0] * vertices[0].scalar * vertices[0].inverse_w
                    + barycentric[1] * vertices[1].scalar * vertices[1].inverse_w
                    + barycentric[2] * vertices[2].scalar * vertices[2].inverse_w)
                    / denominator;
                let normal = (vertices[0].normal * (barycentric[0] * vertices[0].inverse_w)
                    + vertices[1].normal * (barycentric[1] * vertices[1].inverse_w)
                    + vertices[2].normal * (barycentric[2] * vertices[2].inverse_w))
                    / denominator;
                tile.write(
                    x,
                    y,
                    sample_index,
                    depth,
                    triangle.primitive_id,
                    triangle.material.color(scalar, normal),
                );
            }
        }
    }
}

fn rasterize_line(line: &RasterLine3D, tile: &mut TileSamples3D, offsets: &[Vec2]) {
    let bounds = intersection(line.bounds, tile_bounds(tile));
    let delta = line.end.screen - line.start.screen;
    let length_squared = delta.length_squared();
    if !length_squared.is_finite() {
        return;
    }
    let radius_squared = (line.material.width * 0.5).powi(2);
    let length = length_squared.sqrt();
    for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
            for (sample_index, offset) in offsets.iter().enumerate() {
                let sample = Vec2::new(x as f32 + offset.x, y as f32 + offset.y);
                let parameter = if length_squared <= f32::EPSILON {
                    0.0
                } else {
                    ((sample - line.start.screen).dot(delta) / length_squared).clamp(0.0, 1.0)
                };
                let nearest = line.start.screen + delta * parameter;
                if sample.distance_squared(nearest) > radius_squared
                    || !dash_is_on(line.material.dash_pattern.as_deref(), parameter * length)
                {
                    continue;
                }
                let depth = line.start.depth + (line.end.depth - line.start.depth) * parameter;
                tile.write(
                    x,
                    y,
                    sample_index,
                    depth,
                    line.primitive_id,
                    line.material.color,
                );
            }
        }
    }
}

fn rasterize_point(point: &RasterPoint3D, tile: &mut TileSamples3D, offsets: &[Vec2]) {
    let bounds = intersection(point.bounds, tile_bounds(tile));
    for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
            for (sample_index, offset) in offsets.iter().enumerate() {
                let delta =
                    Vec2::new(x as f32 + offset.x, y as f32 + offset.y) - point.center.screen;
                if marker_contains(point.material.marker, delta, point.material.radius) {
                    tile.write(
                        x,
                        y,
                        sample_index,
                        point.center.depth,
                        point.primitive_id,
                        point.material.color,
                    );
                }
            }
        }
    }
}

fn marker_contains(marker: MarkerStyle, delta: Vec2, radius: f32) -> bool {
    if radius <= 0.0 {
        return false;
    }
    let normalized = delta / radius;
    let x = normalized.x.abs();
    let y = normalized.y.abs();
    let radial = normalized.length();
    match marker {
        MarkerStyle::Circle => radial <= 1.0,
        MarkerStyle::CircleOpen => (0.62..=1.0).contains(&radial),
        MarkerStyle::Square => x <= 1.0 && y <= 1.0,
        MarkerStyle::SquareOpen => x <= 1.0 && y <= 1.0 && (x >= 0.68 || y >= 0.68),
        MarkerStyle::Diamond => x + y <= 1.0,
        MarkerStyle::DiamondOpen => {
            let distance = x + y;
            (0.62..=1.0).contains(&distance)
        }
        MarkerStyle::Triangle => {
            normalized.y >= -1.0 && normalized.y <= 1.0 && x <= (1.0 - normalized.y) * 0.58
        }
        MarkerStyle::TriangleDown => {
            normalized.y >= -1.0 && normalized.y <= 1.0 && x <= (1.0 + normalized.y) * 0.58
        }
        MarkerStyle::TriangleOpen => {
            let outer =
                normalized.y >= -1.0 && normalized.y <= 1.0 && x <= (1.0 - normalized.y) * 0.58;
            let inner =
                normalized.y >= -0.48 && normalized.y <= 0.62 && x <= (0.62 - normalized.y) * 0.44;
            outer && !inner
        }
        MarkerStyle::Plus => (x <= 0.22 && y <= 1.0) || (y <= 0.22 && x <= 1.0),
        MarkerStyle::Cross => x <= 1.0 && y <= 1.0 && (x - y).abs() <= 0.28,
        MarkerStyle::Star => {
            if radial > 1.0 {
                return false;
            }
            let angle = normalized.y.atan2(normalized.x);
            let boundary = 0.58 + 0.42 * (angle * 5.0).cos().abs();
            radial <= boundary
        }
    }
}

fn dash_is_on(pattern: Option<&[f32]>, distance: f32) -> bool {
    let Some(pattern) = pattern.filter(|pattern| !pattern.is_empty()) else {
        return true;
    };
    let total: f32 = pattern.iter().sum();
    if !total.is_finite() || total <= f32::EPSILON {
        return true;
    }
    let mut position = distance.rem_euclid(total);
    for (index, &length) in pattern.iter().enumerate() {
        if position <= length {
            return index % 2 == 0;
        }
        position -= length;
    }
    true
}

fn edge(start: Vec2, end: Vec2, point: Vec2) -> f32 {
    (end.x - start.x) * (point.y - start.y) - (end.y - start.y) * (point.x - start.x)
}

fn is_top_left(start: Vec2, end: Vec2) -> bool {
    let delta = end - start;
    delta.y < 0.0 || (delta.y == 0.0 && delta.x > 0.0)
}

fn edge_inside(value: f32, top_left: bool) -> bool {
    value > 0.0 || (value == 0.0 && top_left)
}

fn quantize_depth(depth: f32) -> u32 {
    (depth.clamp(0.0, 1.0) * MAX_DEPTH_24).round() as u32
}

fn triangle_bounds(vertices: [RasterVertex3D; 3], viewport: Viewport3D) -> Option<PixelBounds3D> {
    clipped_bounds(
        vertices
            .iter()
            .map(|vertex| vertex.screen.x)
            .fold(f32::INFINITY, f32::min),
        vertices
            .iter()
            .map(|vertex| vertex.screen.y)
            .fold(f32::INFINITY, f32::min),
        vertices
            .iter()
            .map(|vertex| vertex.screen.x)
            .fold(f32::NEG_INFINITY, f32::max),
        vertices
            .iter()
            .map(|vertex| vertex.screen.y)
            .fold(f32::NEG_INFINITY, f32::max),
        viewport,
    )
}

fn line_bounds(start: Vec2, end: Vec2, radius: f32, viewport: Viewport3D) -> Option<PixelBounds3D> {
    clipped_bounds(
        start.x.min(end.x) - radius,
        start.y.min(end.y) - radius,
        start.x.max(end.x) + radius,
        start.y.max(end.y) + radius,
        viewport,
    )
}

fn point_bounds(center: Vec2, radius: f32, viewport: Viewport3D) -> Option<PixelBounds3D> {
    clipped_bounds(
        center.x - radius,
        center.y - radius,
        center.x + radius,
        center.y + radius,
        viewport,
    )
}

fn clipped_bounds(
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    viewport: Viewport3D,
) -> Option<PixelBounds3D> {
    if ![min_x, min_y, max_x, max_y].into_iter().all(f32::is_finite) {
        return None;
    }
    let viewport_max_x = viewport.x + viewport.width - 1;
    let viewport_max_y = viewport.y + viewport.height - 1;
    let min_x = min_x.floor().max(viewport.x as f32) as u32;
    let min_y = min_y.floor().max(viewport.y as f32) as u32;
    let max_x = max_x.ceil().min(viewport_max_x as f32) as u32;
    let max_y = max_y.ceil().min(viewport_max_y as f32) as u32;
    (min_x <= max_x && min_y <= max_y).then_some(PixelBounds3D {
        min_x,
        min_y,
        max_x,
        max_y,
    })
}

fn tile_bounds(tile: &TileSamples3D) -> PixelBounds3D {
    PixelBounds3D {
        min_x: tile.x,
        min_y: tile.y,
        max_x: tile.x + tile.width - 1,
        max_y: tile.y + tile.height - 1,
    }
}

fn intersection(left: PixelBounds3D, right: PixelBounds3D) -> PixelBounds3D {
    PixelBounds3D {
        min_x: left.min_x.max(right.min_x),
        min_y: left.min_y.max(right.min_y),
        max_x: left.max_x.min(right.max_x),
        max_y: left.max_y.min(right.max_y),
    }
}

fn checked_area(width: u32, height: u32, context: &str) -> Result<usize> {
    (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| PlottingError::InvalidTopology3D {
            reason: format!("{context} exceeds addressable memory"),
        })
}

fn checked_rgba_len(width: u32, height: u32) -> Result<usize> {
    checked_area(width, height, "3D image")?
        .checked_mul(4)
        .ok_or_else(|| PlottingError::InvalidTopology3D {
            reason: "3D RGBA image exceeds addressable memory".to_string(),
        })
}

#[cfg(test)]
#[path = "raster_correctness_tests.rs"]
mod correctness_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn vertex(x: f32, y: f32, depth: f32) -> RasterVertex3D {
        RasterVertex3D {
            screen: Vec2::new(x, y),
            depth,
            inverse_w: 1.0,
            normal: Vec3::Z,
            scalar: 0.0,
        }
    }

    fn material(color: Color) -> Arc<MeshMaterial3D> {
        Arc::new(MeshMaterial3D {
            color: MeshMaterialColor3D::Solid(color),
            shading: SurfaceShading::Unlit,
            two_sided: true,
        })
    }

    fn triangle(
        points: [(f32, f32); 3],
        depth: f32,
        color: Color,
        primitive_id: u64,
    ) -> RasterPrimitive3D {
        let vertices = points.map(|(x, y)| vertex(x, y, depth));
        RasterPrimitive3D::Triangle(RasterTriangle3D {
            vertices,
            material: material(color),
            primitive_id,
            bounds: triangle_bounds(
                vertices,
                Viewport3D {
                    x: 0,
                    y: 0,
                    width: 8,
                    height: 8,
                },
            )
            .expect("bounds"),
        })
    }

    fn raster_test_primitives(
        primitives: Vec<RasterPrimitive3D>,
        sample_offsets: &[Vec2],
    ) -> Vec<u8> {
        let viewport = Viewport3D {
            x: 0,
            y: 0,
            width: 8,
            height: 8,
        };
        let bin: Vec<_> = (0..primitives.len()).collect();
        render_tile(0, 1, viewport, &bin, &primitives, sample_offsets).pixels
    }

    #[test]
    fn nearer_triangle_wins_independent_of_submission_order() {
        let far = triangle([(1.0, 1.0), (7.0, 1.0), (1.0, 7.0)], 0.8, Color::BLUE, 0);
        let near = triangle([(1.0, 1.0), (7.0, 1.0), (1.0, 7.0)], 0.2, Color::RED, 1);
        for primitives in [vec![far.clone(), near.clone()], vec![near, far]] {
            let pixels = raster_test_primitives(primitives, &INTERACTIVE_SAMPLE_OFFSETS);
            let center = (2 * 8 + 2) * 4;
            assert_eq!(&pixels[center..center + 4], &[255, 0, 0, 255]);
        }
    }

    #[test]
    fn depth_ties_use_stable_primitive_id() {
        let first = triangle([(1.0, 1.0), (7.0, 1.0), (1.0, 7.0)], 0.5, Color::RED, 3);
        let second = triangle([(1.0, 1.0), (7.0, 1.0), (1.0, 7.0)], 0.5, Color::BLUE, 2);
        let pixels = raster_test_primitives(vec![first, second], &INTERACTIVE_SAMPLE_OFFSETS);
        let center = (2 * 8 + 2) * 4;
        assert_eq!(&pixels[center..center + 4], &[0, 0, 255, 255]);
    }

    #[test]
    fn top_left_rule_fills_a_two_triangle_quad_without_cracks() {
        let first = triangle([(1.0, 1.0), (7.0, 1.0), (7.0, 7.0)], 0.5, Color::RED, 0);
        let second = triangle([(1.0, 1.0), (7.0, 7.0), (1.0, 7.0)], 0.5, Color::RED, 1);
        let pixels = raster_test_primitives(vec![first, second], &INTERACTIVE_SAMPLE_OFFSETS);
        for y in 1..7 {
            for x in 1..7 {
                assert_eq!(pixels[(y * 8 + x) * 4 + 3], 255, "pixel ({x}, {y})");
            }
        }
    }

    #[test]
    fn four_sample_export_resolves_partial_edge_coverage() {
        let primitive = triangle([(0.0, 0.0), (4.0, 0.0), (0.0, 4.0)], 0.5, Color::RED, 0);
        let pixels = raster_test_primitives(vec![primitive], &EXPORT_SAMPLE_OFFSETS);
        assert!(pixels[3] > 0);
        assert!(pixels[(3 * 8 + 3) * 4 + 3] < 255);
    }

    #[test]
    fn marker_shapes_have_expected_center_behavior() {
        assert!(marker_contains(MarkerStyle::Circle, Vec2::ZERO, 5.0));
        assert!(!marker_contains(MarkerStyle::CircleOpen, Vec2::ZERO, 5.0));
        assert!(marker_contains(MarkerStyle::Plus, Vec2::ZERO, 5.0));
        assert!(!marker_contains(
            MarkerStyle::Diamond,
            Vec2::new(5.0, 5.0),
            5.0
        ));
    }

    #[test]
    fn isolated_depth_layer_has_a_stable_export_hash() {
        let viewport = Viewport3D {
            x: 0,
            y: 0,
            width: 8,
            height: 8,
        };
        let triangle = triangle([(0.5, 0.5), (7.5, 0.5), (0.5, 7.5)], 0.6, Color::BLUE, 8);
        let line_material = Arc::new(LineMaterial3D {
            color: Color::GREEN,
            width: 1.5,
            dash_pattern: None,
        });
        let line = RasterPrimitive3D::Line(RasterLine3D {
            start: vertex(0.5, 7.0, 0.4),
            end: vertex(7.0, 0.5, 0.4),
            material: line_material,
            primitive_id: 4,
            bounds: line_bounds(Vec2::new(0.5, 7.0), Vec2::new(7.0, 0.5), 0.75, viewport)
                .expect("line bounds"),
        });
        let point = RasterPrimitive3D::Point(RasterPoint3D {
            center: vertex(4.0, 4.0, 0.2),
            material: PointMaterial3D {
                color: Color::RED,
                radius: 1.5,
                marker: MarkerStyle::Circle,
            },
            primitive_id: 1,
            bounds: point_bounds(Vec2::new(4.0, 4.0), 1.5, viewport).expect("point bounds"),
        });
        let pixels = raster_test_primitives(vec![triangle, line, point], &EXPORT_SAMPLE_OFFSETS);
        assert_eq!(fnv1a64(&pixels), 0x3591_6942_3a5e_c98f);
    }

    fn fnv1a64(bytes: &[u8]) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for &byte in bytes {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash
    }
}
