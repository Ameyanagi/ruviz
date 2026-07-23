use std::cmp::Ordering;

use glam::Vec3;

use crate::core::{PlottingError, Result};
use crate::render::three_d::scene::SceneGeometry3D;

use super::Point3D;
use super::builder::Plot3D;
use super::layout::Axis3Layout;
use super::prepared::PreparedSceneCache3D;

const LEAF_SIZE: usize = 8;

/// Kind of retained primitive selected by a 3d pick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickPrimitive3D {
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
    /// Triangle index within the retained surface batch.
    pub triangle_index: u32,
    /// Source-grid vertex indices for the triangle.
    pub source_indices: [u32; 3],
    /// Triangle barycentric coordinates in source-index order.
    pub barycentric: [f32; 3],
    /// Hit position in the original data coordinate system.
    pub point: Point3D,
    /// Distance from the near-plane ray origin in normalized scene units.
    pub ray_distance: f32,
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
        let Some((origin, direction)) = layout.screen_ray_local(screen_x, screen_y)? else {
            return Ok(None);
        };
        let mut cache = PreparedSceneCache3D::default();
        let (scene, bvh, _) = cache.prepare_with_bvh(&frame)?;
        let Some(hit) = bvh.intersect_ray(&scene.geometry, origin, direction)? else {
            return Ok(None);
        };
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
        Ok(Some(PickHit3D {
            series_index: mesh.series_index,
            primitive: PickPrimitive3D::SurfaceTriangle,
            triangle_index: hit.triangle_index,
            source_indices,
            barycentric: hit.barycentric,
            point: frame.bounds.denormalize(hit.local_position, Vec3::ONE),
            ray_distance: hit.distance,
        }))
    }
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
}
