use glam::{Vec3, Vec4};

/// Vertex plus interpolants used by homogeneous clipping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ClipVertex3D {
    pub(crate) clip_position: Vec4,
    pub(crate) local_position: Vec3,
    pub(crate) normal: Vec3,
    pub(crate) color: Vec4,
    pub(crate) scalar: f32,
}

impl ClipVertex3D {
    pub(crate) fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            clip_position: self.clip_position.lerp(other.clip_position, t),
            local_position: self.local_position.lerp(other.local_position, t),
            normal: self.normal.lerp(other.normal, t),
            color: self.color.lerp(other.color, t),
            scalar: self.scalar + (other.scalar - self.scalar) * t,
        }
    }
}

#[derive(Clone, Copy)]
enum ClipPlane {
    Left,
    Right,
    Bottom,
    Top,
    Near,
    Far,
}

const CLIP_PLANES: [ClipPlane; 6] = [
    ClipPlane::Left,
    ClipPlane::Right,
    ClipPlane::Bottom,
    ClipPlane::Top,
    ClipPlane::Near,
    ClipPlane::Far,
];

impl ClipPlane {
    fn distance(self, position: Vec4) -> f32 {
        match self {
            Self::Left => position.w + position.x,
            Self::Right => position.w - position.x,
            Self::Bottom => position.w + position.y,
            Self::Top => position.w - position.y,
            Self::Near => position.z,
            Self::Far => position.w - position.z,
        }
    }
}

pub(crate) fn is_inside_clip_volume(position: Vec4) -> bool {
    position.is_finite()
        && position.w > 0.0
        && CLIP_PLANES
            .into_iter()
            .all(|plane| plane.distance(position) >= 0.0)
}

/// Clip a triangle against the wgpu homogeneous clip volume.
///
/// The returned fan contains zero or more triangles whose vertices satisfy
/// `-w <= x,y <= w` and `0 <= z <= w`.
pub(crate) fn clip_triangle(triangle: [ClipVertex3D; 3]) -> Vec<[ClipVertex3D; 3]> {
    let mut polygon = triangle.to_vec();
    let mut scratch = Vec::with_capacity(9);

    for plane in CLIP_PLANES {
        if polygon.is_empty() {
            break;
        }
        scratch.clear();
        let Some(&previous_vertex) = polygon.last() else {
            break;
        };
        let mut previous = previous_vertex;
        let mut previous_distance = plane.distance(previous.clip_position);
        let mut previous_inside = previous_distance >= 0.0;

        for &current in &polygon {
            let current_distance = plane.distance(current.clip_position);
            let current_inside = current_distance >= 0.0;
            if current_inside != previous_inside {
                let denominator = previous_distance - current_distance;
                if denominator != 0.0 {
                    let t = (previous_distance / denominator).clamp(0.0, 1.0);
                    scratch.push(previous.lerp(current, t));
                }
            }
            if current_inside {
                scratch.push(current);
            }
            previous = current;
            previous_distance = current_distance;
            previous_inside = current_inside;
        }
        std::mem::swap(&mut polygon, &mut scratch);
    }

    if polygon.len() < 3 {
        return Vec::new();
    }
    let first = polygon[0];
    (1..polygon.len() - 1)
        .map(|index| [first, polygon[index], polygon[index + 1]])
        .collect()
}

/// Clip a line segment against the wgpu homogeneous clip volume.
pub(crate) fn clip_segment(
    mut start: ClipVertex3D,
    mut end: ClipVertex3D,
) -> Option<[ClipVertex3D; 2]> {
    for plane in CLIP_PLANES {
        let start_distance = plane.distance(start.clip_position);
        let end_distance = plane.distance(end.clip_position);
        let start_inside = start_distance >= 0.0;
        let end_inside = end_distance >= 0.0;
        match (start_inside, end_inside) {
            (false, false) => return None,
            (true, true) => {}
            _ => {
                let denominator = start_distance - end_distance;
                if denominator == 0.0 {
                    return None;
                }
                let t = (start_distance / denominator).clamp(0.0, 1.0);
                let intersection = start.lerp(end, t);
                if start_inside {
                    end = intersection;
                } else {
                    start = intersection;
                }
            }
        }
    }
    Some([start, end])
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn vertex(position: [f32; 4]) -> ClipVertex3D {
        ClipVertex3D {
            clip_position: Vec4::from_array(position),
            local_position: Vec3::ZERO,
            normal: Vec3::Z,
            color: Vec4::ONE,
            scalar: 0.5,
        }
    }

    #[test]
    fn all_six_wgpu_clip_planes_are_enforced() {
        for outside in [
            [-1.1, 0.0, 0.5, 1.0],
            [1.1, 0.0, 0.5, 1.0],
            [0.0, -1.1, 0.5, 1.0],
            [0.0, 1.1, 0.5, 1.0],
            [0.0, 0.0, -0.1, 1.0],
            [0.0, 0.0, 1.1, 1.0],
        ] {
            assert!(!is_inside_clip_volume(Vec4::from_array(outside)));
        }
        assert!(is_inside_clip_volume(Vec4::new(0.0, 0.0, 0.5, 1.0)));
    }

    #[test]
    fn inside_triangle_is_preserved() {
        let input = [
            vertex([-0.5, -0.5, 0.5, 1.0]),
            vertex([0.5, -0.5, 0.5, 1.0]),
            vertex([0.0, 0.5, 0.5, 1.0]),
        ];
        assert_eq!(clip_triangle(input), vec![input]);
    }

    #[test]
    fn triangle_crossing_near_plane_is_clipped_and_triangulated() {
        let output = clip_triangle([
            vertex([-0.5, -0.5, -0.5, 1.0]),
            vertex([0.5, -0.5, 0.5, 1.0]),
            vertex([0.0, 0.5, 0.5, 1.0]),
        ]);
        assert_eq!(output.len(), 2);
        assert!(output.iter().flatten().all(|vertex| {
            is_inside_clip_volume(vertex.clip_position) && vertex.clip_position.z.abs() <= 1.0
        }));
    }

    #[test]
    fn fully_clipped_triangle_is_removed() {
        assert!(
            clip_triangle([
                vertex([-0.5, -0.5, -0.5, 1.0]),
                vertex([0.5, -0.5, -0.5, 1.0]),
                vertex([0.0, 0.5, -0.5, 1.0]),
            ])
            .is_empty()
        );
    }

    #[test]
    fn segment_crossing_left_plane_is_shortened() {
        let [start, end] =
            clip_segment(vertex([-2.0, 0.0, 0.5, 1.0]), vertex([0.5, 0.0, 0.5, 1.0]))
                .expect("visible segment");
        assert!((start.clip_position.x + 1.0).abs() <= 1.0e-6);
        assert_eq!(end.clip_position.x, 0.5);
        assert!(is_inside_clip_volume(start.clip_position));
        assert!(is_inside_clip_volume(end.clip_position));
    }

    proptest! {
        #[test]
        fn finite_homogeneous_triangles_never_panic_or_create_non_finite_vertices(
            coordinates in prop::collection::vec(-8.0f32..8.0, 12),
        ) {
            let triangle = [
                vertex([coordinates[0], coordinates[1], coordinates[2], coordinates[3].abs() + 0.1]),
                vertex([coordinates[4], coordinates[5], coordinates[6], coordinates[7].abs() + 0.1]),
                vertex([coordinates[8], coordinates[9], coordinates[10], coordinates[11].abs() + 0.1]),
            ];
            for vertex in clip_triangle(triangle).into_iter().flatten() {
                prop_assert!(vertex.clip_position.is_finite());
                prop_assert!(vertex.local_position.is_finite());
                prop_assert!(vertex.normal.is_finite());
                prop_assert!(vertex.color.is_finite());
                prop_assert!(vertex.scalar.is_finite());
            }
        }
    }
}
