use std::sync::Arc;

use glam::Vec3;

use crate::core::{Bounds3D, PlottingError, Result};
use crate::plots::three_d::{Grid3DData, Points3DData};
use crate::plots::{SurfaceSampling, SurfaceShading};
use crate::render::three_d::scene::{
    LineBatch3D, LineGeometryBatch3D, MeshBatch3D, MeshColor3D, MeshGeometryBatch3D, MeshStyle3D,
    MeshVertex3D, PointBatch3D, PointGeometryBatch3D, PointStyle3D, Scene3D, SceneGeometry3D,
    StrokeStyle3D,
};
#[cfg(feature = "parallel")]
use crate::render::three_d::software::raster::SoftwareQuality3D;
use crate::render::three_d::software::raster::{
    SoftwareRenderOptions3D, SoftwareRenderOutput3D, render_scene,
};

use super::RenderDiagnostics3D;
use super::builder::{Plot3D, Series3D};
use super::layout::Axis3Layout;
use super::resolve::{CacheKey3D, ResolvedFrame3D};

#[derive(Default)]
pub(crate) struct PreparedSceneCache3D {
    geometry_key: Option<CacheKey3D>,
    appearance_key: Option<CacheKey3D>,
    view_key: Option<CacheKey3D>,
    geometry: Option<Arc<SceneGeometry3D>>,
    scene: Option<Arc<Scene3D>>,
}

impl PreparedSceneCache3D {
    pub(crate) fn prepare(
        &mut self,
        frame: &ResolvedFrame3D,
    ) -> Result<(Arc<Scene3D>, RenderDiagnostics3D)> {
        let mut diagnostics = RenderDiagnostics3D::default();
        let geometry_changed = self.geometry_key != Some(frame.keys.geometry);
        let geometry = if geometry_changed {
            let geometry = Arc::new(lower_geometry(frame, &mut diagnostics)?);
            self.geometry = Some(Arc::clone(&geometry));
            self.geometry_key = Some(frame.keys.geometry);
            geometry
        } else {
            Arc::clone(
                self.geometry
                    .as_ref()
                    .expect("matching geometry key must retain geometry"),
            )
        };

        let appearance_changed = self.appearance_key != Some(frame.keys.appearance);
        let scene = if geometry_changed || appearance_changed {
            let scene = Arc::new(assemble_scene(frame, Arc::clone(&geometry))?);
            self.scene = Some(Arc::clone(&scene));
            self.appearance_key = Some(frame.keys.appearance);
            scene
        } else {
            Arc::clone(
                self.scene
                    .as_ref()
                    .expect("matching scene keys must retain a scene"),
            )
        };

        if self.view_key != Some(frame.keys.view) {
            diagnostics.camera_uniform_writes = 1;
            self.view_key = Some(frame.keys.view);
        }
        diagnostics.points_submitted = scene.point_count() as u64;
        diagnostics.triangles_submitted = scene.triangle_count() as u64;
        diagnostics.sampling_mode = sampling_mode(&frame.series).to_string();
        Ok((scene, diagnostics))
    }
}

impl Plot3D {
    pub(super) fn prepare_once(self) -> Result<(Arc<Scene3D>, RenderDiagnostics3D)> {
        let frame = self.resolve()?;
        PreparedSceneCache3D::default().prepare(&frame)
    }

    pub(super) fn render_software_layer(
        self,
        options: SoftwareRenderOptions3D,
    ) -> Result<PreparedSoftwareFrame3D> {
        let frame = self.resolve()?;
        let layout = Axis3Layout::resolve(&frame)?;
        let (scene, mut diagnostics) = PreparedSceneCache3D::default().prepare(&frame)?;
        let output = render_scene(&scene, &layout, frame.figure.dpi, options)?;
        diagnostics.actual_backend = "cpu3d".to_string();
        diagnostics.draw_calls = output.draw_calls;
        diagnostics.primitives_culled = output.primitives_culled;
        diagnostics.readback_bytes = 0;
        Ok(PreparedSoftwareFrame3D {
            frame,
            layout,
            output,
            diagnostics,
        })
    }
}

pub(super) struct PreparedSoftwareFrame3D {
    pub(super) frame: ResolvedFrame3D,
    pub(super) layout: Axis3Layout,
    pub(super) output: SoftwareRenderOutput3D,
    pub(super) diagnostics: RenderDiagnostics3D,
}

fn lower_geometry(
    frame: &ResolvedFrame3D,
    diagnostics: &mut RenderDiagnostics3D,
) -> Result<SceneGeometry3D> {
    diagnostics.scene_compiles = 1;
    let mut geometry = SceneGeometry3D::default();
    for (series_index, series) in frame.series.iter().enumerate() {
        let series_index =
            u32::try_from(series_index).map_err(|_| PlottingError::InvalidTopology3D {
                reason: "3D series count exceeds u32 indexing".to_string(),
            })?;
        match series {
            Series3D::Scatter { data, .. } => {
                geometry
                    .points
                    .push(Arc::new(lower_points(series_index, data, frame.bounds)?));
            }
            Series3D::Line { data, .. } => {
                geometry
                    .lines
                    .push(Arc::new(lower_polyline(series_index, data, frame.bounds)?));
            }
            Series3D::Surface { data, config, .. } => {
                diagnostics.triangulations += 1;
                if config.shading != SurfaceShading::Unlit {
                    diagnostics.normal_recomputations += 1;
                }
                geometry.meshes.push(Arc::new(lower_surface(
                    series_index,
                    data,
                    frame.bounds,
                    config.sampling,
                    config.shading,
                )?));
            }
            Series3D::Wireframe { data, config, .. } => {
                geometry.lines.push(Arc::new(lower_wireframe(
                    series_index,
                    data,
                    frame.bounds,
                    config.sampling,
                )?));
            }
        }
    }
    Ok(geometry)
}

fn assemble_scene(frame: &ResolvedFrame3D, geometry: Arc<SceneGeometry3D>) -> Result<Scene3D> {
    let mut points = Vec::with_capacity(geometry.points.len());
    let mut lines = Vec::with_capacity(geometry.lines.len());
    let mut meshes = Vec::with_capacity(geometry.meshes.len());

    for batch in &geometry.points {
        let Series3D::Scatter { config, label, .. } = &frame.series[batch.series_index as usize]
        else {
            return Err(series_geometry_mismatch(batch.series_index));
        };
        points.push(PointBatch3D {
            geometry: Arc::clone(batch),
            style: PointStyle3D {
                color: config
                    .color
                    .unwrap_or_else(|| palette_color(&frame.theme, batch.series_index as usize)),
                marker: config.marker,
                marker_size: config.marker_size,
                label: label.clone(),
            },
        });
    }

    for batch in &geometry.lines {
        let style = match &frame.series[batch.series_index as usize] {
            Series3D::Line { config, label, .. } => StrokeStyle3D {
                color: config
                    .color
                    .unwrap_or_else(|| palette_color(&frame.theme, batch.series_index as usize)),
                line_width: config.line_width,
                line_style: config.line_style.clone(),
                label: label.clone(),
            },
            Series3D::Wireframe { config, label, .. } => StrokeStyle3D {
                color: config.color.unwrap_or(frame.theme.foreground),
                line_width: config.line_width,
                line_style: config.line_style.clone(),
                label: label.clone(),
            },
            _ => return Err(series_geometry_mismatch(batch.series_index)),
        };
        lines.push(LineBatch3D {
            geometry: Arc::clone(batch),
            style,
        });
    }

    for batch in &geometry.meshes {
        let Series3D::Surface {
            data,
            config,
            label,
        } = &frame.series[batch.series_index as usize]
        else {
            return Err(series_geometry_mismatch(batch.series_index));
        };
        let color = match config.color {
            Some(color) => MeshColor3D::Solid(color),
            None => MeshColor3D::Scalar {
                colormap: config.colormap.clone(),
                data_range: finite_range(&data.z).ok_or(PlottingError::EmptyDataSet)?,
            },
        };
        meshes.push(MeshBatch3D {
            geometry: Arc::clone(batch),
            style: MeshStyle3D {
                color,
                shading: config.shading,
                two_sided: true,
                label: label.clone(),
            },
        });
    }

    Ok(Scene3D {
        geometry,
        points,
        lines,
        meshes,
    })
}

fn series_geometry_mismatch(series_index: u32) -> PlottingError {
    PlottingError::InvalidTopology3D {
        reason: format!("retained geometry does not match 3D series {series_index}"),
    }
}

fn palette_color(theme: &crate::render::Theme, series_index: usize) -> crate::render::Color {
    if theme.color_palette.is_empty() {
        theme.foreground
    } else {
        theme.color_palette[series_index % theme.color_palette.len()]
    }
}

fn lower_points(
    series_index: u32,
    data: &Points3DData,
    bounds: Bounds3D,
) -> Result<PointGeometryBatch3D> {
    let mut positions = Vec::with_capacity(data.x.len());
    let mut source_indices = Vec::with_capacity(data.x.len());
    for index in 0..data.x.len() {
        let point = super::Point3D::new(data.x[index], data.y[index], data.z[index]);
        if !point.is_finite() {
            continue;
        }
        positions.push(bounds.normalize(point, Vec3::ONE).to_array());
        source_indices.push(checked_u32(index, "scatter3d source index")?);
    }
    Ok(PointGeometryBatch3D {
        series_index,
        positions: positions.into(),
        source_indices: source_indices.into(),
    })
}

fn lower_polyline(
    series_index: u32,
    data: &Points3DData,
    bounds: Bounds3D,
) -> Result<LineGeometryBatch3D> {
    let mut positions = Vec::with_capacity(data.x.len());
    let mut source_indices = Vec::with_capacity(data.x.len());
    let mut segments = Vec::with_capacity(data.x.len().saturating_sub(1));
    let mut previous = None;
    for index in 0..data.x.len() {
        let point = super::Point3D::new(data.x[index], data.y[index], data.z[index]);
        if !point.is_finite() {
            previous = None;
            continue;
        }
        let vertex_index = checked_u32(positions.len(), "line3d vertex index")?;
        positions.push(bounds.normalize(point, Vec3::ONE).to_array());
        source_indices.push(checked_u32(index, "line3d source index")?);
        if let Some(previous) = previous {
            segments.push([previous, vertex_index]);
        }
        previous = Some(vertex_index);
    }
    Ok(LineGeometryBatch3D {
        series_index,
        positions: positions.into(),
        source_indices: source_indices.into(),
        segments: segments.into(),
    })
}

fn lower_surface(
    series_index: u32,
    data: &Grid3DData,
    bounds: Bounds3D,
    sampling: SurfaceSampling,
    shading: SurfaceShading,
) -> Result<MeshGeometryBatch3D> {
    let rows = sampled_indices(data.rows, sampling_limit(sampling).map(|value| value.0));
    let columns = sampled_indices(data.columns, sampling_limit(sampling).map(|value| value.1));
    let sampled_len =
        rows.len()
            .checked_mul(columns.len())
            .ok_or_else(|| PlottingError::InvalidTopology3D {
                reason: "sampled surface shape overflows usize".to_string(),
            })?;
    let scalar_range = finite_range(&data.z).ok_or(PlottingError::EmptyDataSet)?;
    let mut vertices = Vec::with_capacity(sampled_len);
    let mut vertex_map = vec![None; sampled_len];

    for (sample_row, &source_row) in rows.iter().enumerate() {
        for (sample_column, &source_column) in columns.iter().enumerate() {
            let source_index = checked_grid_index(data, source_row, source_column)?;
            let point = super::Point3D::new(
                data.x[source_column],
                data.y[source_row],
                data.z[source_index],
            );
            if !point.is_finite() {
                continue;
            }
            let vertex_index = checked_u32(vertices.len(), "surface vertex index")?;
            vertex_map[sample_row * columns.len() + sample_column] = Some(vertex_index);
            vertices.push(MeshVertex3D {
                position: bounds.normalize(point, Vec3::ONE).to_array(),
                normal: Vec3::Z.to_array(),
                scalar: normalize_scalar(data.z[source_index], scalar_range),
                source_index: checked_u32(source_index, "surface source index")?,
            });
        }
    }

    let cell_count = rows
        .len()
        .saturating_sub(1)
        .checked_mul(columns.len().saturating_sub(1))
        .ok_or_else(|| PlottingError::InvalidTopology3D {
            reason: "sampled surface cell count overflows usize".to_string(),
        })?;
    let mut indices = Vec::with_capacity(cell_count.saturating_mul(6));
    for row in 0..rows.len().saturating_sub(1) {
        for column in 0..columns.len().saturating_sub(1) {
            let width = columns.len();
            let p00 = vertex_map[row * width + column];
            let p01 = vertex_map[row * width + column + 1];
            let p11 = vertex_map[(row + 1) * width + column + 1];
            let p10 = vertex_map[(row + 1) * width + column];
            push_triangle_if_finite(&mut indices, p00, p01, p11);
            push_triangle_if_finite(&mut indices, p00, p11, p10);
        }
    }

    match shading {
        SurfaceShading::Unlit => {}
        SurfaceShading::Smooth => recompute_smooth_normals(&mut vertices, &indices),
        SurfaceShading::Flat => {
            let (flat_vertices, flat_indices) = expand_flat_vertices(&vertices, &indices)?;
            vertices = flat_vertices;
            indices = flat_indices;
        }
    }

    Ok(MeshGeometryBatch3D {
        series_index,
        vertices: vertices.into(),
        indices: indices.into(),
    })
}

fn lower_wireframe(
    series_index: u32,
    data: &Grid3DData,
    bounds: Bounds3D,
    sampling: SurfaceSampling,
) -> Result<LineGeometryBatch3D> {
    let rows = sampled_indices(data.rows, sampling_limit(sampling).map(|value| value.0));
    let columns = sampled_indices(data.columns, sampling_limit(sampling).map(|value| value.1));
    let sampled_len =
        rows.len()
            .checked_mul(columns.len())
            .ok_or_else(|| PlottingError::InvalidTopology3D {
                reason: "sampled wireframe shape overflows usize".to_string(),
            })?;
    let mut positions = Vec::with_capacity(sampled_len);
    let mut source_indices = Vec::with_capacity(sampled_len);
    let mut vertex_map = vec![None; sampled_len];

    for (sample_row, &source_row) in rows.iter().enumerate() {
        for (sample_column, &source_column) in columns.iter().enumerate() {
            let source_index = checked_grid_index(data, source_row, source_column)?;
            let point = super::Point3D::new(
                data.x[source_column],
                data.y[source_row],
                data.z[source_index],
            );
            if !point.is_finite() {
                continue;
            }
            let vertex_index = checked_u32(positions.len(), "wireframe vertex index")?;
            vertex_map[sample_row * columns.len() + sample_column] = Some(vertex_index);
            positions.push(bounds.normalize(point, Vec3::ONE).to_array());
            source_indices.push(checked_u32(source_index, "wireframe source index")?);
        }
    }

    let horizontal = rows.len().saturating_mul(columns.len().saturating_sub(1));
    let vertical = rows.len().saturating_sub(1).saturating_mul(columns.len());
    let mut segments = Vec::with_capacity(horizontal.saturating_add(vertical));
    let width = columns.len();
    for row in 0..rows.len() {
        for column in 0..columns.len().saturating_sub(1) {
            push_segment_if_finite(
                &mut segments,
                vertex_map[row * width + column],
                vertex_map[row * width + column + 1],
            );
        }
    }
    for row in 0..rows.len().saturating_sub(1) {
        for column in 0..columns.len() {
            push_segment_if_finite(
                &mut segments,
                vertex_map[row * width + column],
                vertex_map[(row + 1) * width + column],
            );
        }
    }

    Ok(LineGeometryBatch3D {
        series_index,
        positions: positions.into(),
        source_indices: source_indices.into(),
        segments: segments.into(),
    })
}

fn sampled_indices(length: usize, limit: Option<usize>) -> Vec<usize> {
    let Some(limit) = limit.filter(|&limit| limit < length) else {
        return (0..length).collect();
    };
    (0..limit)
        .map(|index| index * (length - 1) / (limit - 1))
        .collect()
}

fn sampling_limit(sampling: SurfaceSampling) -> Option<(usize, usize)> {
    match sampling {
        SurfaceSampling::Auto | SurfaceSampling::Full => None,
        SurfaceSampling::MaxGrid { rows, columns } => Some((rows, columns)),
    }
}

fn sampling_mode(series: &[Series3D]) -> &'static str {
    let mut full = false;
    let mut sampled = false;
    for series in series {
        let sampling = match series {
            Series3D::Surface { config, .. } => Some(config.sampling),
            Series3D::Wireframe { config, .. } => Some(config.sampling),
            _ => None,
        };
        match sampling {
            Some(SurfaceSampling::MaxGrid { .. }) => sampled = true,
            Some(SurfaceSampling::Auto | SurfaceSampling::Full) => full = true,
            None => {}
        }
    }
    match (full, sampled) {
        (true, true) => "mixed",
        (false, true) => "max-grid",
        _ => "full",
    }
}

fn checked_grid_index(data: &Grid3DData, row: usize, column: usize) -> Result<usize> {
    row.checked_mul(data.columns)
        .and_then(|offset| offset.checked_add(column))
        .filter(|&index| index < data.z.len())
        .ok_or_else(|| PlottingError::InvalidTopology3D {
            reason: format!(
                "{} grid index ({row}, {column}) is outside {}x{}",
                data.operation, data.rows, data.columns
            ),
        })
}

fn checked_u32(value: usize, context: &str) -> Result<u32> {
    u32::try_from(value).map_err(|_| PlottingError::InvalidTopology3D {
        reason: format!("{context} exceeds u32 indexing"),
    })
}

fn push_triangle_if_finite(
    indices: &mut Vec<u32>,
    first: Option<u32>,
    second: Option<u32>,
    third: Option<u32>,
) {
    if let (Some(first), Some(second), Some(third)) = (first, second, third) {
        indices.extend_from_slice(&[first, second, third]);
    }
}

fn push_segment_if_finite(segments: &mut Vec<[u32; 2]>, start: Option<u32>, end: Option<u32>) {
    if let (Some(start), Some(end)) = (start, end) {
        segments.push([start, end]);
    }
}

fn recompute_smooth_normals(vertices: &mut [MeshVertex3D], indices: &[u32]) {
    let mut accumulated = vec![Vec3::ZERO; vertices.len()];
    for triangle in indices.chunks_exact(3) {
        let [first, second, third] = [
            triangle[0] as usize,
            triangle[1] as usize,
            triangle[2] as usize,
        ];
        let p0 = Vec3::from_array(vertices[first].position);
        let p1 = Vec3::from_array(vertices[second].position);
        let p2 = Vec3::from_array(vertices[third].position);
        let face = (p1 - p0).cross(p2 - p0);
        accumulated[first] += face;
        accumulated[second] += face;
        accumulated[third] += face;
    }
    for (vertex, normal) in vertices.iter_mut().zip(accumulated) {
        vertex.normal = normalized_or_up(normal).to_array();
    }
}

fn expand_flat_vertices(
    vertices: &[MeshVertex3D],
    indices: &[u32],
) -> Result<(Vec<MeshVertex3D>, Vec<u32>)> {
    let mut flat_vertices = Vec::with_capacity(indices.len());
    let mut flat_indices = Vec::with_capacity(indices.len());
    for triangle in indices.chunks_exact(3) {
        let source = [
            vertices[triangle[0] as usize],
            vertices[triangle[1] as usize],
            vertices[triangle[2] as usize],
        ];
        let normal = normalized_or_up(
            (Vec3::from_array(source[1].position) - Vec3::from_array(source[0].position))
                .cross(Vec3::from_array(source[2].position) - Vec3::from_array(source[0].position)),
        )
        .to_array();
        for mut vertex in source {
            vertex.normal = normal;
            flat_indices.push(checked_u32(
                flat_vertices.len(),
                "flat surface vertex index",
            )?);
            flat_vertices.push(vertex);
        }
    }
    Ok((flat_vertices, flat_indices))
}

fn normalized_or_up(normal: Vec3) -> Vec3 {
    if normal.is_finite() && normal.length_squared() > f32::EPSILON {
        normal.normalize()
    } else {
        Vec3::Z
    }
}

fn finite_range(values: &[f64]) -> Option<(f64, f64)> {
    let mut range: Option<(f64, f64)> = None;
    for &value in values {
        if !value.is_finite() {
            continue;
        }
        match &mut range {
            Some((minimum, maximum)) => {
                *minimum = minimum.min(value);
                *maximum = maximum.max(value);
            }
            None => range = Some((value, value)),
        }
    }
    range
}

fn normalize_scalar(value: f64, (minimum, maximum): (f64, f64)) -> f32 {
    if minimum == maximum {
        return 0.5;
    }
    let half_span = maximum * 0.5 - minimum * 0.5;
    let center = minimum * 0.5 + maximum * 0.5;
    (((value - center) / half_span) * 0.5 + 0.5) as f32
}

#[cfg(test)]
mod tests {
    use crate::{line3d, scatter3d, surface, wireframe};

    use super::*;

    fn prepare(frame: ResolvedFrame3D) -> (Arc<Scene3D>, RenderDiagnostics3D) {
        PreparedSceneCache3D::default()
            .prepare(&frame)
            .expect("prepared scene")
    }

    #[test]
    fn surface_uses_fixed_row_major_split_and_upward_normals() {
        let frame = surface(
            &[0.0, 1.0, 2.0],
            &[0.0, 1.0],
            &[[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        )
        .finalize()
        .resolve()
        .expect("frame");
        let (scene, diagnostics) = prepare(frame);
        assert_eq!(scene.triangle_count(), 4);
        assert_eq!(diagnostics.triangulations, 1);
        let mesh = &scene.meshes[0].geometry;
        assert_eq!(&*mesh.indices, &[0, 1, 4, 0, 4, 3, 1, 2, 5, 1, 5, 4]);
        assert!(mesh.vertices.iter().all(|vertex| vertex.normal[2] > 0.999));
    }

    #[test]
    fn nan_surface_vertex_removes_only_touching_triangles() {
        let frame = surface(&[0.0, 1.0], &[0.0, 1.0], &[[0.0, f64::NAN], [0.0, 0.0]])
            .finalize()
            .resolve()
            .expect("frame");
        let (scene, _) = prepare(frame);
        assert_eq!(scene.triangle_count(), 1);
    }

    #[test]
    fn nan_splits_lines_without_joining_across_the_gap() {
        let frame = line3d(
            &[0.0, 1.0, 2.0, 3.0, 4.0],
            &[0.0, 1.0, f64::NAN, 3.0, 4.0],
            &[0.0, 1.0, 2.0, 3.0, 4.0],
        )
        .finalize()
        .resolve()
        .expect("frame");
        let (scene, _) = prepare(frame);
        assert_eq!(scene.segment_count(), 2);
        assert_eq!(&*scene.lines[0].geometry.segments, &[[0, 1], [2, 3]]);
    }

    #[test]
    fn wireframe_contains_unique_grid_edges_without_diagonals() {
        let frame = wireframe(
            &[0.0, 1.0, 2.0],
            &[0.0, 1.0],
            &[[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        )
        .finalize()
        .resolve()
        .expect("frame");
        let (scene, _) = prepare(frame);
        assert_eq!(scene.segment_count(), 7);
    }

    #[test]
    fn max_grid_sampling_includes_both_endpoints() {
        let values = [0.0, 1.0, 2.0, 3.0, 4.0];
        let z = [
            [0.0, 0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 0.0],
        ];
        let frame = surface(&values, &values, &z)
            .sampling(SurfaceSampling::MaxGrid {
                rows: 3,
                columns: 3,
            })
            .finalize()
            .resolve()
            .expect("frame");
        let (scene, diagnostics) = prepare(frame);
        assert_eq!(scene.meshes[0].geometry.vertices.len(), 9);
        assert_eq!(scene.triangle_count(), 8);
        assert_eq!(diagnostics.sampling_mode, "max-grid");
        let sources: Vec<_> = scene.meshes[0]
            .geometry
            .vertices
            .iter()
            .map(|vertex| vertex.source_index)
            .collect();
        assert_eq!(sources, vec![0, 2, 4, 10, 12, 14, 20, 22, 24]);
    }

    #[test]
    fn camera_only_prepare_reuses_scene_and_rebuilds_nothing() {
        let base = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .finalize()
            .resolve()
            .expect("base");
        let camera = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .azimuth_deg(10.0)
            .finalize()
            .resolve()
            .expect("camera");
        let mut cache = PreparedSceneCache3D::default();
        let (first, _) = cache.prepare(&base).expect("first");
        let (second, diagnostics) = cache.prepare(&camera).expect("second");
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(diagnostics.scene_compiles, 0);
        assert_eq!(diagnostics.triangulations, 0);
        assert_eq!(diagnostics.normal_recomputations, 0);
        assert_eq!(diagnostics.bvh_rebuilds, 0);
        assert_eq!(diagnostics.vertex_upload_bytes, 0);
        assert_eq!(diagnostics.index_upload_bytes, 0);
        assert_eq!(diagnostics.buffer_creations, 0);
        assert_eq!(diagnostics.camera_uniform_writes, 1);
    }

    #[test]
    fn style_only_prepare_reuses_geometry() {
        let base = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .finalize()
            .resolve()
            .expect("base");
        let styled = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .marker_size(11.0)
            .finalize()
            .resolve()
            .expect("styled");
        let mut cache = PreparedSceneCache3D::default();
        let (first, _) = cache.prepare(&base).expect("first");
        let (second, diagnostics) = cache.prepare(&styled).expect("second");
        assert!(Arc::ptr_eq(&first.geometry, &second.geometry));
        assert!(!Arc::ptr_eq(&first, &second));
        assert_eq!(diagnostics.scene_compiles, 0);
        assert_eq!(diagnostics.triangulations, 0);
        assert_eq!(diagnostics.normal_recomputations, 0);
    }

    #[test]
    fn data_change_rebuilds_geometry_and_scene() {
        let base = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .finalize()
            .resolve()
            .expect("base");
        let changed = scatter3d(&[0.0, 2.0], &[0.0, 1.0], &[0.0, 1.0])
            .finalize()
            .resolve()
            .expect("changed");
        let mut cache = PreparedSceneCache3D::default();
        let (first, _) = cache.prepare(&base).expect("first");
        let (second, diagnostics) = cache.prepare(&changed).expect("second");
        assert!(!Arc::ptr_eq(&first.geometry, &second.geometry));
        assert!(!Arc::ptr_eq(&first, &second));
        assert_eq!(diagnostics.scene_compiles, 1);
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn serial_and_parallel_software_tiles_are_byte_identical() {
        let frame = surface(
            &[0.0, 1.0, 2.0],
            &[0.0, 1.0, 2.0],
            &[[0.0, 0.5, 0.0], [0.5, 1.0, 0.5], [0.0, 0.5, 0.0]],
        )
        .figure_size(2.0, 1.5)
        .dpi(72)
        .finalize()
        .resolve()
        .expect("frame");
        let layout = Axis3Layout::resolve(&frame).expect("layout");
        let (scene, _) = prepare(frame.clone());
        let serial = render_scene(
            &scene,
            &layout,
            frame.figure.dpi,
            SoftwareRenderOptions3D {
                quality: SoftwareQuality3D::Export,
                parallel: false,
            },
        )
        .expect("serial");
        let parallel = render_scene(
            &scene,
            &layout,
            frame.figure.dpi,
            SoftwareRenderOptions3D {
                quality: SoftwareQuality3D::Export,
                parallel: true,
            },
        )
        .expect("parallel");
        assert_eq!(serial.layer.pixels, parallel.layer.pixels);
    }
}
