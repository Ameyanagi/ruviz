use proptest::prelude::*;

use super::*;

fn vertex(position: [f32; 4], scalar: f32) -> ClipVertex3D {
    ClipVertex3D {
        clip_position: Vec4::from_array(position),
        local_position: Vec3::new(scalar, scalar * 2.0, -scalar),
        normal: Vec3::new(0.0, scalar, 1.0),
        color: Vec4::new(scalar, 1.0 - scalar, 0.5, 1.0),
        scalar,
    }
}

fn assert_vertex_near(left: ClipVertex3D, right: ClipVertex3D) {
    // Boundary interpolation error grows with coordinate magnitude, and the
    // forward and reversed parameterizations (t versus 1-t) round
    // differently — by up to ~1e-4 at the ±8 coordinates this suite
    // generates, and differently again across SIMD arches. Scale the
    // allowance with the compared magnitudes; a genuine logic bug (wrong
    // attribute, wrong plane, wrong endpoint) misses by O(1).
    let scale = left
        .clip_position
        .abs()
        .max(right.clip_position.abs())
        .max_element()
        .max(1.0);
    let tolerance = 2.0e-4 * scale;
    assert!(
        left.clip_position
            .abs_diff_eq(right.clip_position, tolerance),
        "clip positions differ: {:?} versus {:?}",
        left.clip_position,
        right.clip_position
    );
    assert!(
        left.local_position
            .abs_diff_eq(right.local_position, tolerance),
        "local positions differ: {:?} versus {:?}",
        left.local_position,
        right.local_position
    );
    assert!(
        left.normal.abs_diff_eq(right.normal, tolerance),
        "normals differ: {:?} versus {:?}",
        left.normal,
        right.normal
    );
    assert!(
        left.color.abs_diff_eq(right.color, tolerance),
        "colors differ: {:?} versus {:?}",
        left.color,
        right.color
    );
    assert!((left.scalar - right.scalar).abs() <= tolerance);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn finite_segment_clipping_is_inside_idempotent_and_reversal_symmetric(
        start in prop::array::uniform4(-8.0f32..8.0),
        end in prop::array::uniform4(-8.0f32..8.0),
        start_scalar in 0.0f32..1.0,
        end_scalar in 0.0f32..1.0,
    ) {
        let start = vertex(
            [start[0], start[1], start[2], start[3].abs() + 0.1],
            start_scalar,
        );
        let end = vertex(
            [end[0], end[1], end[2], end[3].abs() + 0.1],
            end_scalar,
        );
        let forward = clip_segment(start, end);
        let reverse = clip_segment(end, start);

        prop_assert_eq!(forward.is_some(), reverse.is_some());
        if let (Some([forward_start, forward_end]), Some([reverse_start, reverse_end])) =
            (forward, reverse)
        {
            prop_assert!(
                is_inside_clip_volume(forward_start.clip_position),
                "forward start escaped the clip volume: {:?}",
                forward_start.clip_position
            );
            prop_assert!(
                is_inside_clip_volume(forward_end.clip_position),
                "forward end escaped the clip volume: {:?}",
                forward_end.clip_position
            );
            assert_vertex_near(forward_start, reverse_end);
            assert_vertex_near(forward_end, reverse_start);

            let repeated = clip_segment(forward_start, forward_end)
                .expect("an already-clipped segment remains visible");
            assert_vertex_near(repeated[0], forward_start);
            assert_vertex_near(repeated[1], forward_end);
        }
    }
}

#[test]
fn segment_clipping_interpolates_every_attribute_at_the_same_boundary() {
    let start = vertex([-2.0, 0.0, 0.5, 1.0], 0.0);
    let end = vertex([0.0, 0.0, 0.5, 1.0], 1.0);
    let [clipped_start, clipped_end] = clip_segment(start, end).expect("visible half segment");

    assert_vertex_near(clipped_start, start.lerp(end, 0.5));
    assert_vertex_near(clipped_end, end);
    assert!(is_inside_clip_volume(clipped_start.clip_position));
    assert!(is_inside_clip_volume(clipped_end.clip_position));
}
