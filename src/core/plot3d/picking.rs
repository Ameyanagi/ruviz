use std::cmp::Ordering;

use glam::{Vec2, Vec3, Vec4};

use crate::core::{PlottingError, Result};
use crate::render::three_d::scene::{Scene3D, SceneGeometry3D};
use crate::render::three_d::software::clip::{ClipVertex3D, clip_segment, is_inside_clip_volume};

use super::Point3D;
use super::builder::Plot3D;
use super::layout::Axis3Layout;
use super::prepared::PreparedSceneCache3D;

const LEAF_SIZE: usize = 8;

/// Kind of retained primitive selected by a 3d pick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickPrimitive3D {
    /// One scatter marker.
    Point,
    /// A data-space sphere. Source indices contain its application-owned ID.
    Sphere,
    /// One line or wireframe segment.
    LineSegment,
    /// One triangle from a surface series.
    SurfaceTriangle,
}

/// Nearest depth-correct result from a 3d viewport pick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickHit3D {
    /// High-level series index in the chained 3d plot.
    pub series_index: u32,
    /// Primitive kind.
    pub primitive: PickPrimitive3D,
    /// Primitive index within its retained batch.
    pub primitive_index: u32,
    /// Triangle index within the retained surface batch.
    ///
    /// This compatibility field matches [`Self::primitive_index`] for a
    /// surface and is zero for points and line segments.
    pub triangle_index: u32,
    /// Source-data indices (stable IDs for spheres). Read the first
    /// [`Self::source_count`] entries.
    pub source_indices: [u32; 3],
    /// Number of meaningful entries in [`Self::source_indices`].
    pub source_count: u8,
    /// Triangle barycentric coordinates in source-index order.
    ///
    /// Points use `[1, 0, 0]`. Lines use `[1-t, t, 0]`.
    pub barycentric: [f32; 3],
    /// Hit position in the original data coordinate system.
    pub point: Point3D,
    /// Distance from the near-plane ray origin in normalized scene units.
    pub ray_distance: f32,
    /// Retained scene generation that produced this result.
    pub scene_generation: u64,
    /// Camera generation that produced this result.
    pub camera_generation: u64,
}

impl PickHit3D {
    /// Meaningful source-data indices for this primitive.
    pub fn sources(&self) -> &[u32] {
        &self.source_indices[..usize::from(self.source_count).min(self.source_indices.len())]
    }
}

#[derive(Clone, Copy, Debug)]
struct Aabb3D {
    min: Vec3,
    max: Vec3,
}

impl Aabb3D {
    fn empty() -> Self {
        Self {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
        }
    }

    fn from_triangle(vertices: [Vec3; 3]) -> Self {
        let mut bounds = Self::empty();
        for vertex in vertices {
            bounds.include(vertex);
        }
        bounds
    }

    fn include(&mut self, point: Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }

    fn include_bounds(&mut self, other: Self) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }

    fn centroid(self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    fn intersects_ray(self, origin: Vec3, direction: Vec3, max_distance: f32) -> bool {
        let mut entry = 0.0_f32;
        let mut exit = max_distance;
        for (origin, direction, minimum, maximum) in [
            (origin.x, direction.x, self.min.x, self.max.x),
            (origin.y, direction.y, self.min.y, self.max.y),
            (origin.z, direction.z, self.min.z, self.max.z),
        ] {
            if direction.abs() <= f32::MIN_POSITIVE {
                if origin < minimum || origin > maximum {
                    return false;
                }
                continue;
            }
            let first = (minimum - origin) / direction;
            let second = (maximum - origin) / direction;
            entry = entry.max(first.min(second));
            exit = exit.min(first.max(second));
            if entry > exit {
                return false;
            }
        }
        true
    }
}

#[derive(Clone, Copy, Debug)]
struct TriangleRef3D {
    mesh_batch: u32,
    triangle_index: u32,
    bounds: Aabb3D,
}

#[derive(Clone, Copy, Debug)]
enum BvhNodeKind3D {
    Internal { left: u32, right: u32 },
    Leaf { start: u32, len: u32 },
}

#[derive(Clone, Copy, Debug)]
struct BvhNode3D {
    bounds: Aabb3D,
    kind: BvhNodeKind3D,
}

/// Camera-independent BVH over retained surface triangles.
#[derive(Clone, Debug, Default)]
pub(crate) struct Bvh3D {
    nodes: Vec<BvhNode3D>,
    triangles: Vec<TriangleRef3D>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BvhHit3D {
    pub(crate) mesh_batch: u32,
    pub(crate) triangle_index: u32,
    pub(crate) distance: f32,
    pub(crate) barycentric: [f32; 3],
    pub(crate) local_position: Vec3,
}

impl Bvh3D {
    pub(crate) fn build(geometry: &SceneGeometry3D) -> Result<Self> {
        let triangle_count = geometry
            .meshes
            .iter()
            .map(|mesh| mesh.indices.len() / 3)
            .sum();
        let mut triangles = Vec::with_capacity(triangle_count);
        for (mesh_batch, mesh) in geometry.meshes.iter().enumerate() {
            let mesh_batch =
                u32::try_from(mesh_batch).map_err(|_| PlottingError::InvalidTopology3D {
                    reason: "3D mesh batch count exceeds u32 BVH indexing".to_string(),
                })?;
            for (triangle_index, triangle) in mesh.indices.chunks_exact(3).enumerate() {
                let vertices = triangle_vertices(geometry, mesh_batch, triangle)?;
                triangles.push(TriangleRef3D {
                    mesh_batch,
                    triangle_index: u32::try_from(triangle_index).map_err(|_| {
                        PlottingError::InvalidTopology3D {
                            reason: "3D triangle count exceeds u32 BVH indexing".to_string(),
                        }
                    })?,
                    bounds: Aabb3D::from_triangle(vertices),
                });
            }
        }
        if triangles.is_empty() {
            return Ok(Self::default());
        }

        let estimated_nodes = triangles.len().saturating_mul(2);
        let mut nodes = Vec::with_capacity(estimated_nodes);
        build_node(&mut triangles, &mut nodes, 0, triangle_count)?;
        Ok(Self { nodes, triangles })
    }

    pub(crate) fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    pub(crate) fn intersect_ray(
        &self,
        geometry: &SceneGeometry3D,
        origin: Vec3,
        direction: Vec3,
    ) -> Result<Option<BvhHit3D>> {
        if self.nodes.is_empty() {
            return Ok(None);
        }
        if !origin.is_finite()
            || !direction.is_finite()
            || direction.length_squared() <= f32::EPSILON
        {
            return Err(PlottingError::InvalidInput(
                "3D picking ray must have a finite, non-zero direction".to_string(),
            ));
        }
        let direction = direction.normalize();
        let mut closest: Option<BvhHit3D> = None;
        let mut stack = vec![0_u32];
        while let Some(node_index) = stack.pop() {
            let node = self.nodes[node_index as usize];
            let max_distance = closest.map_or(f32::INFINITY, |hit| hit.distance);
            if !node.bounds.intersects_ray(origin, direction, max_distance) {
                continue;
            }
            match node.kind {
                BvhNodeKind3D::Internal { left, right } => {
                    // Push right first so the stable left partition is visited
                    // first when both children have the same entry distance.
                    stack.push(right);
                    stack.push(left);
                }
                BvhNodeKind3D::Leaf { start, len } => {
                    for primitive_index in start..start + len {
                        let triangle = self.triangles[primitive_index as usize];
                        let indices = triangle_indices(geometry, triangle)?;
                        let vertices = triangle_vertices(geometry, triangle.mesh_batch, indices)?;
                        let Some((distance, barycentric)) =
                            ray_triangle(origin, direction, vertices)
                        else {
                            continue;
                        };
                        let replace = closest.is_none_or(|current| {
                            distance < current.distance
                                || (distance.to_bits() == current.distance.to_bits()
                                    && (triangle.mesh_batch, triangle.triangle_index)
                                        < (current.mesh_batch, current.triangle_index))
                        });
                        if replace {
                            closest = Some(BvhHit3D {
                                mesh_batch: triangle.mesh_batch,
                                triangle_index: triangle.triangle_index,
                                distance,
                                barycentric,
                                local_position: origin + direction * distance,
                            });
                        }
                    }
                }
            }
        }
        Ok(closest)
    }
}

impl Plot3D {
    pub(super) fn pick_at(self, screen_x: f32, screen_y: f32) -> Result<Option<PickHit3D>> {
        let frame = self.resolve()?;
        let layout = Axis3Layout::resolve(&frame)?;
        let mut cache = PreparedSceneCache3D::default();
        let (scene, bvh, _) = cache.prepare_with_bvh(&frame)?;
        pick_scene(&frame, &layout, &scene, &bvh, screen_x, screen_y, 0, 0)
    }

    pub(super) fn project_at(self, point: Point3D) -> Result<Option<(f32, f32)>> {
        let frame = self.resolve()?;
        let layout = Axis3Layout::resolve(&frame)?;
        let local = frame.bounds.normalize(point, Vec3::ONE);
        Ok(project_visible_local(&layout, local).map(|projected| (projected.x, projected.y)))
    }
}

pub(crate) fn pick_scene(
    frame: &super::resolve::ResolvedFrame3D,
    layout: &Axis3Layout,
    scene: &Scene3D,
    bvh: &Bvh3D,
    screen_x: f32,
    screen_y: f32,
    scene_generation: u64,
    camera_generation: u64,
) -> Result<Option<PickHit3D>> {
    let Some((ray_origin, ray_direction)) = layout.screen_ray_local(screen_x, screen_y)? else {
        return Ok(None);
    };
    let cursor = Vec2::new(screen_x, screen_y);
    let mut closest: Option<(f32, u8, u32, u32, PickHit3D)> = None;

    for batch in &scene.points {
        let tolerance = batch.style.marker_size * frame.figure.dpi / 144.0 + 3.0;
        for (point_index, (&position, &source_index)) in batch
            .geometry
            .positions
            .iter()
            .zip(batch.geometry.source_indices.iter())
            .enumerate()
        {
            let local = Vec3::from_array(position);
            let Some(projected) = project_visible_local(layout, local) else {
                continue;
            };
            let screen_distance = cursor.distance(projected.truncate());
            if screen_distance > tolerance {
                continue;
            }
            let primitive_index = checked_u32(point_index, "3D point pick index")?;
            let hit = PickHit3D {
                series_index: batch.geometry.series_index,
                primitive: PickPrimitive3D::Point,
                primitive_index,
                triangle_index: 0,
                source_indices: [source_index; 3],
                source_count: 1,
                barycentric: [1.0, 0.0, 0.0],
                point: frame.bounds.denormalize(local, Vec3::ONE),
                ray_distance: (local - ray_origin).dot(ray_direction).max(0.0),
                scene_generation,
                camera_generation,
            };
            consider_pick(
                &mut closest,
                projected.z,
                0,
                batch.geometry.series_index,
                primitive_index,
                hit,
            );
        }
    }

    for batch in &scene.spheres {
        for (index, sphere) in batch.geometry.instances.iter().enumerate() {
            // Context with <=5% opacity is visible but does not intercept picks.
            if sphere.color.a <= 12 {
                continue;
            }
            let center = Vec3::from_array(sphere.center);
            let radii = Vec3::from_array(sphere.radii);
            let Some(distance) =
                crate::render::three_d::sphere::intersect(ray_origin, ray_direction, center, radii)
            else {
                continue;
            };
            let local = ray_origin + ray_direction * distance;
            let Some(projected) = project_visible_local(layout, local) else {
                continue;
            };
            let primitive_index = checked_u32(index, "3D sphere pick index")?;
            let hit = PickHit3D {
                series_index: batch.geometry.series_index,
                primitive: PickPrimitive3D::Sphere,
                primitive_index,
                triangle_index: 0,
                source_indices: [sphere.id; 3],
                source_count: 1,
                barycentric: [1.0, 0.0, 0.0],
                point: frame.bounds.denormalize(local, Vec3::ONE),
                ray_distance: distance,
                scene_generation,
                camera_generation,
            };
            consider_pick(
                &mut closest,
                projected.z,
                0,
                batch.geometry.series_index,
                primitive_index,
                hit,
            );
        }
    }

    for batch in &scene.lines {
        let tolerance = batch.style.line_width * frame.figure.dpi / 144.0 + 3.0;
        for (segment_index, &[start_index, end_index]) in batch.geometry.segments.iter().enumerate()
        {
            let start = indexed_position(&batch.geometry.positions, start_index, "line start")?;
            let end = indexed_position(&batch.geometry.positions, end_index, "line end")?;
            let Some((projected_start, projected_end, original_start_t, original_end_t)) =
                project_clipped_segment(layout, start, end)
            else {
                continue;
            };
            let (screen_distance, clipped_t) = point_segment_distance(
                cursor,
                projected_start.truncate(),
                projected_end.truncate(),
            );
            if screen_distance > tolerance {
                continue;
            }
            let original_t = original_start_t + (original_end_t - original_start_t) * clipped_t;
            let local = start.lerp(end, original_t);
            let depth = projected_start.z + (projected_end.z - projected_start.z) * clipped_t;
            let primitive_index = checked_u32(segment_index, "3D line pick index")?;
            let start_source = indexed_source(
                &batch.geometry.source_indices,
                start_index,
                "line start source",
            )?;
            let end_source =
                indexed_source(&batch.geometry.source_indices, end_index, "line end source")?;
            let hit = PickHit3D {
                series_index: batch.geometry.series_index,
                primitive: PickPrimitive3D::LineSegment,
                primitive_index,
                triangle_index: 0,
                source_indices: [start_source, end_source, end_source],
                source_count: 2,
                barycentric: [1.0 - original_t, original_t, 0.0],
                point: frame.bounds.denormalize(local, Vec3::ONE),
                ray_distance: (local - ray_origin).dot(ray_direction).max(0.0),
                scene_generation,
                camera_generation,
            };
            consider_pick(
                &mut closest,
                depth,
                1,
                batch.geometry.series_index,
                primitive_index,
                hit,
            );
        }
    }

    if let Some(hit) = bvh.intersect_ray(&scene.geometry, ray_origin, ray_direction)? {
        let mesh = scene
            .geometry
            .meshes
            .get(hit.mesh_batch as usize)
            .ok_or_else(|| PlottingError::InvalidTopology3D {
                reason: "3D pick references an out-of-range mesh batch".to_string(),
            })?;
        let triangle_start = hit.triangle_index as usize * 3;
        let indices = mesh
            .indices
            .get(triangle_start..triangle_start + 3)
            .ok_or_else(|| PlottingError::InvalidTopology3D {
                reason: "3D pick references an out-of-range triangle".to_string(),
            })?;
        let mut source_indices = [0_u32; 3];
        for (destination, &index) in source_indices.iter_mut().zip(indices) {
            *destination = mesh
                .vertices
                .get(index as usize)
                .ok_or_else(|| PlottingError::InvalidTopology3D {
                    reason: "3D pick references an out-of-range vertex".to_string(),
                })?
                .source_index;
        }
        if let Some(projected) = project_visible_local(layout, hit.local_position) {
            let surface_hit = PickHit3D {
                series_index: mesh.series_index,
                primitive: PickPrimitive3D::SurfaceTriangle,
                primitive_index: hit.triangle_index,
                triangle_index: hit.triangle_index,
                source_indices,
                source_count: 3,
                barycentric: hit.barycentric,
                point: frame.bounds.denormalize(hit.local_position, Vec3::ONE),
                ray_distance: hit.distance,
                scene_generation,
                camera_generation,
            };
            consider_pick(
                &mut closest,
                projected.z,
                2,
                mesh.series_index,
                hit.triangle_index,
                surface_hit,
            );
        }
    }

    Ok(closest.map(|(_, _, _, _, hit)| hit))
}

fn consider_pick(
    closest: &mut Option<(f32, u8, u32, u32, PickHit3D)>,
    depth: f32,
    priority: u8,
    series_index: u32,
    primitive_index: u32,
    hit: PickHit3D,
) {
    let key = (depth, priority, series_index, primitive_index);
    let replace = closest.as_ref().is_none_or(
        |(current_depth, current_priority, current_series, current_primitive, _)| {
            depth.total_cmp(current_depth).is_lt()
                || (depth.to_bits() == current_depth.to_bits()
                    && (priority, series_index, primitive_index)
                        < (*current_priority, *current_series, *current_primitive))
        },
    );
    if replace {
        *closest = Some((key.0, key.1, key.2, key.3, hit));
    }
}

fn project_visible_local(layout: &Axis3Layout, local: Vec3) -> Option<Vec3> {
    let clip = layout.camera.view_projection * (local * layout.camera.axis_aspect).extend(1.0);
    if !is_inside_clip_volume(clip) {
        return None;
    }
    Some(project_clip_position(layout, clip))
}

fn project_clipped_segment(
    layout: &Axis3Layout,
    start: Vec3,
    end: Vec3,
) -> Option<(Vec3, Vec3, f32, f32)> {
    let vertex = |local_position: Vec3, scalar: f32| ClipVertex3D {
        clip_position: layout.camera.view_projection
            * (local_position * layout.camera.axis_aspect).extend(1.0),
        local_position,
        normal: Vec3::ZERO,
        color: Vec4::ZERO,
        scalar,
    };
    let [start, end] = clip_segment(vertex(start, 0.0), vertex(end, 1.0))?;
    Some((
        project_clip_position(layout, start.clip_position),
        project_clip_position(layout, end.clip_position),
        start.scalar,
        end.scalar,
    ))
}

fn project_clip_position(layout: &Axis3Layout, clip: Vec4) -> Vec3 {
    let ndc = clip.truncate() / clip.w;
    Vec3::new(
        layout.viewport.x as f32 + (ndc.x * 0.5 + 0.5) * layout.viewport.width as f32,
        layout.viewport.y as f32 + (0.5 - ndc.y * 0.5) * layout.viewport.height as f32,
        ndc.z,
    )
}

fn point_segment_distance(point: Vec2, start: Vec2, end: Vec2) -> (f32, f32) {
    let segment = end - start;
    let length_squared = segment.length_squared();
    let t = if length_squared <= f32::EPSILON {
        0.0
    } else {
        ((point - start).dot(segment) / length_squared).clamp(0.0, 1.0)
    };
    (point.distance(start + segment * t), t)
}

fn indexed_position(positions: &[[f32; 3]], index: u32, context: &str) -> Result<Vec3> {
    positions
        .get(index as usize)
        .copied()
        .map(Vec3::from_array)
        .ok_or_else(|| PlottingError::InvalidTopology3D {
            reason: format!("3D pick references an out-of-range {context}"),
        })
}

fn indexed_source(sources: &[u32], index: u32, context: &str) -> Result<u32> {
    sources
        .get(index as usize)
        .copied()
        .ok_or_else(|| PlottingError::InvalidTopology3D {
            reason: format!("3D pick references an out-of-range {context}"),
        })
}

fn build_node(
    triangles: &mut [TriangleRef3D],
    nodes: &mut Vec<BvhNode3D>,
    start: usize,
    len: usize,
) -> Result<u32> {
    let node_index = checked_u32(nodes.len(), "3D BVH node index")?;
    nodes.push(BvhNode3D {
        bounds: Aabb3D::empty(),
        kind: BvhNodeKind3D::Leaf { start: 0, len: 0 },
    });

    let slice = &mut triangles[start..start + len];
    let mut bounds = Aabb3D::empty();
    let mut centroid_bounds = Aabb3D::empty();
    for triangle in slice.iter() {
        bounds.include_bounds(triangle.bounds);
        centroid_bounds.include(triangle.bounds.centroid());
    }
    let kind = if len <= LEAF_SIZE {
        BvhNodeKind3D::Leaf {
            start: checked_u32(start, "3D BVH leaf start")?,
            len: checked_u32(len, "3D BVH leaf length")?,
        }
    } else {
        let axis = largest_axis(centroid_bounds.max - centroid_bounds.min);
        let middle = len / 2;
        slice.select_nth_unstable_by(middle, |left, right| {
            component(left.bounds.centroid(), axis)
                .total_cmp(&component(right.bounds.centroid(), axis))
                .then_with(|| stable_triangle_order(*left, *right))
        });
        let left = build_node(triangles, nodes, start, middle)?;
        let right = build_node(triangles, nodes, start + middle, len - middle)?;
        BvhNodeKind3D::Internal { left, right }
    };
    nodes[node_index as usize] = BvhNode3D { bounds, kind };
    Ok(node_index)
}

fn stable_triangle_order(left: TriangleRef3D, right: TriangleRef3D) -> Ordering {
    (left.mesh_batch, left.triangle_index).cmp(&(right.mesh_batch, right.triangle_index))
}

fn largest_axis(extent: Vec3) -> usize {
    if extent.x >= extent.y && extent.x >= extent.z {
        0
    } else if extent.y >= extent.z {
        1
    } else {
        2
    }
}

fn component(value: Vec3, axis: usize) -> f32 {
    match axis {
        0 => value.x,
        1 => value.y,
        _ => value.z,
    }
}

fn triangle_indices(geometry: &SceneGeometry3D, triangle: TriangleRef3D) -> Result<&[u32]> {
    let mesh = geometry
        .meshes
        .get(triangle.mesh_batch as usize)
        .ok_or_else(|| PlottingError::InvalidTopology3D {
            reason: "3D BVH references an out-of-range mesh batch".to_string(),
        })?;
    let start = triangle.triangle_index as usize * 3;
    mesh.indices
        .get(start..start + 3)
        .ok_or_else(|| PlottingError::InvalidTopology3D {
            reason: "3D BVH references an out-of-range triangle".to_string(),
        })
}

fn triangle_vertices(
    geometry: &SceneGeometry3D,
    mesh_batch: u32,
    indices: &[u32],
) -> Result<[Vec3; 3]> {
    let mesh = geometry.meshes.get(mesh_batch as usize).ok_or_else(|| {
        PlottingError::InvalidTopology3D {
            reason: "3D triangle references an out-of-range mesh batch".to_string(),
        }
    })?;
    let mut vertices = [Vec3::ZERO; 3];
    for (destination, &index) in vertices.iter_mut().zip(indices) {
        let vertex =
            mesh.vertices
                .get(index as usize)
                .ok_or_else(|| PlottingError::InvalidTopology3D {
                    reason: "3D triangle references an out-of-range vertex".to_string(),
                })?;
        *destination = Vec3::from_array(vertex.position);
    }
    Ok(vertices)
}

fn ray_triangle(origin: Vec3, direction: Vec3, vertices: [Vec3; 3]) -> Option<(f32, [f32; 3])> {
    let edge1 = vertices[1] - vertices[0];
    let edge2 = vertices[2] - vertices[0];
    let perpendicular = direction.cross(edge2);
    let determinant = edge1.dot(perpendicular);
    if !determinant.is_finite() || determinant.abs() <= 1.0e-7 {
        return None;
    }
    let inverse_determinant = determinant.recip();
    let offset = origin - vertices[0];
    let u = offset.dot(perpendicular) * inverse_determinant;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let cross = offset.cross(edge1);
    let v = direction.dot(cross) * inverse_determinant;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let distance = edge2.dot(cross) * inverse_determinant;
    if !distance.is_finite() || distance < 0.0 {
        return None;
    }
    Some((distance, [1.0 - u - v, u, v]))
}

fn checked_u32(value: usize, context: &str) -> Result<u32> {
    u32::try_from(value).map_err(|_| PlottingError::InvalidTopology3D {
        reason: format!("{context} exceeds u32 indexing"),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::render::three_d::scene::{MeshGeometryBatch3D, MeshVertex3D, SceneGeometry3D};

    use super::*;

    fn vertex(position: [f32; 3]) -> MeshVertex3D {
        MeshVertex3D {
            position,
            normal: [0.0, 0.0, 1.0],
            scalar: 0.0,
            source_index: 0,
        }
    }

    fn two_layer_geometry() -> SceneGeometry3D {
        SceneGeometry3D {
            meshes: vec![
                Arc::new(MeshGeometryBatch3D {
                    series_index: 0,
                    vertices: vec![
                        vertex([-1.0, -1.0, 0.0]),
                        vertex([1.0, -1.0, 0.0]),
                        vertex([0.0, 1.0, 0.0]),
                    ]
                    .into(),
                    indices: vec![0, 1, 2].into(),
                }),
                Arc::new(MeshGeometryBatch3D {
                    series_index: 1,
                    vertices: vec![
                        vertex([-1.0, -1.0, 0.5]),
                        vertex([1.0, -1.0, 0.5]),
                        vertex([0.0, 1.0, 0.5]),
                    ]
                    .into(),
                    indices: vec![0, 1, 2].into(),
                }),
            ],
            ..SceneGeometry3D::default()
        }
    }

    #[test]
    fn nearest_triangle_is_returned_with_barycentrics() {
        let geometry = two_layer_geometry();
        let bvh = Bvh3D::build(&geometry).expect("BVH");
        assert_eq!(bvh.triangle_count(), 2);
        let hit = bvh
            .intersect_ray(&geometry, Vec3::new(0.0, 0.0, 2.0), -Vec3::Z)
            .expect("intersection")
            .expect("hit");
        assert_eq!(hit.mesh_batch, 1);
        assert!((hit.distance - 1.5).abs() <= 1.0e-6);
        assert!((hit.barycentric.iter().sum::<f32>() - 1.0).abs() <= 1.0e-6);
        assert!((hit.local_position.z - 0.5).abs() <= 1.0e-6);
    }

    #[test]
    fn misses_and_invalid_rays_are_distinct() {
        let geometry = two_layer_geometry();
        let bvh = Bvh3D::build(&geometry).expect("BVH");
        assert!(
            bvh.intersect_ray(&geometry, Vec3::new(4.0, 4.0, 2.0), -Vec3::Z)
                .expect("miss")
                .is_none()
        );
        assert!(
            bvh.intersect_ray(&geometry, Vec3::ZERO, Vec3::ZERO)
                .is_err()
        );
    }

    #[test]
    fn axis_aligned_ray_on_a_flat_bounds_edge_is_supported() {
        let geometry = two_layer_geometry();
        let bvh = Bvh3D::build(&geometry).expect("BVH");
        let hit = bvh
            .intersect_ray(&geometry, Vec3::new(-1.0, -1.0, 2.0), -Vec3::Z)
            .expect("intersection");
        assert!(hit.is_some());
    }

    #[test]
    fn surface_hit_beyond_explicit_clip_limits_is_a_miss_not_an_error() {
        let frame = crate::surface(&[-1.0, 1.0], &[-1.0, 1.0], &[[10.0, 10.0], [10.0, 10.0]])
            .zlim(0.0, 1.0)
            .finalize()
            .resolve()
            .expect("resolved frame");
        let layout = Axis3Layout::resolve(&frame).expect("axis layout");
        let mut cache = PreparedSceneCache3D::default();
        let (scene, bvh, _) = cache.prepare_with_bvh(&frame).expect("prepared scene");
        let screen_x = layout.viewport.x as f32 + layout.viewport.width as f32 * 0.5;
        let screen_y = layout.viewport.y as f32 + layout.viewport.height as f32 * 0.5;

        let hit = pick_scene(&frame, &layout, &scene, &bvh, screen_x, screen_y, 0, 0)
            .expect("clipped surface pick");

        assert!(hit.is_none());
    }
}
