use std::sync::Arc;

use proptest::prelude::*;

#[cfg(feature = "parallel")]
use crate::core::{Bounds3D, Camera3D, Point3D};
#[cfg(feature = "parallel")]
use crate::render::three_d::scene::{
    LineBatch3D, LineGeometryBatch3D, MeshBatch3D, MeshColor3D, MeshGeometryBatch3D, MeshStyle3D,
    MeshVertex3D, PointBatch3D, PointGeometryBatch3D, PointStyle3D, Scene3D, SceneGeometry3D,
    StrokeStyle3D,
};

use super::*;

fn vertex_with_interpolants(
    screen: Vec2,
    depth: f32,
    inverse_w: f32,
    scalar: f32,
) -> RasterVertex3D {
    RasterVertex3D {
        screen,
        depth,
        inverse_w,
        normal: Vec3::Z,
        scalar,
    }
}

fn test_viewport(width: u32, height: u32) -> Viewport3D {
    Viewport3D {
        x: 0,
        y: 0,
        width,
        height,
    }
}

fn raster_triangle(depth: f32, color: Color, primitive_id: u64) -> RasterPrimitive3D {
    let vertices = [
        vertex_with_interpolants(Vec2::new(0.0, 0.0), depth, 1.0, 0.0),
        vertex_with_interpolants(Vec2::new(8.0, 0.0), depth, 1.0, 0.0),
        vertex_with_interpolants(Vec2::new(0.0, 8.0), depth, 1.0, 0.0),
    ];
    RasterPrimitive3D::Triangle(RasterTriangle3D {
        vertices,
        material: Arc::new(MeshMaterial3D {
            color: MeshMaterialColor3D::Solid(color),
            shading: SurfaceShading::Unlit,
            two_sided: true,
        }),
        primitive_id,
        bounds: triangle_bounds(vertices, test_viewport(8, 8)).expect("triangle bounds"),
    })
}

fn render_test_primitives(primitives: &[RasterPrimitive3D]) -> Vec<u8> {
    let bin: Vec<_> = (0..primitives.len()).collect();
    render_tile(
        0,
        1,
        test_viewport(8, 8),
        &bin,
        primitives,
        &INTERACTIVE_SAMPLE_OFFSETS,
    )
    .pixels
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn quantized_depth_is_bounded_and_monotonic(
        left in -4.0f32..4.0,
        right in -4.0f32..4.0,
    ) {
        let lower = left.min(right);
        let upper = left.max(right);
        prop_assert!(quantize_depth(lower) <= quantize_depth(upper));
        prop_assert!(quantize_depth(lower) <= MAX_DEPTH_24 as u32);
        prop_assert!(quantize_depth(upper) <= MAX_DEPTH_24 as u32);
    }

    #[test]
    fn nearer_unique_depth_wins_for_either_submission_order(
        near in 0.0f32..0.49,
        separation in 0.02f32..0.49,
    ) {
        let far = (near + separation).min(1.0);
        let near_triangle = raster_triangle(near, Color::RED, 91);
        let far_triangle = raster_triangle(far, Color::BLUE, 2);
        let forward = render_test_primitives(&[near_triangle.clone(), far_triangle.clone()]);
        let reverse = render_test_primitives(&[far_triangle, near_triangle]);
        prop_assert_eq!(&forward, &reverse);
        let sample = (2 * 8 + 2) * 4;
        prop_assert_eq!(&forward[sample..sample + 4], &[255, 0, 0, 255]);
    }
}

#[test]
fn perspective_correct_scalar_interpolation_differs_from_affine_interpolation() {
    let vertices = [
        vertex_with_interpolants(Vec2::new(0.0, 0.0), 0.5, 1.0, 0.0),
        vertex_with_interpolants(Vec2::new(8.0, 0.0), 0.5, 0.5, 1.0),
        vertex_with_interpolants(Vec2::new(0.0, 8.0), 0.5, 0.25, 1.0),
    ];
    let colormap = ColorMap::new("black-white".to_string(), vec![Color::BLACK, Color::WHITE]);
    let triangle = RasterPrimitive3D::Triangle(RasterTriangle3D {
        vertices,
        material: Arc::new(MeshMaterial3D {
            color: MeshMaterialColor3D::Scalar(colormap.clone()),
            shading: SurfaceShading::Unlit,
            two_sided: true,
        }),
        primitive_id: 0,
        bounds: triangle_bounds(vertices, test_viewport(8, 8)).expect("triangle bounds"),
    });

    let pixels = render_test_primitives(&[triangle]);
    let sample = 36; // RGBA offset for pixel (1, 1) in an 8-pixel-wide tile.
    let barycentric = [0.625_f32, 0.1875, 0.1875];
    let denominator = barycentric[0] + barycentric[1] * 0.5 + barycentric[2] * 0.25;
    let perspective_scalar = (barycentric[1] * 0.5 + barycentric[2] * 0.25) / denominator;
    let expected = colormap.sample(f64::from(perspective_scalar));
    let affine = colormap.sample(f64::from(barycentric[1] + barycentric[2]));

    assert_eq!(
        &pixels[sample..sample + 4],
        &[expected.r, expected.g, expected.b, expected.a]
    );
    assert_ne!(expected, affine);
}

#[cfg(feature = "parallel")]
#[test]
fn serial_and_parallel_mixed_scenes_are_byte_identical_across_tile_boundaries() {
    let point_geometry = Arc::new(PointGeometryBatch3D {
        series_index: 0,
        positions: Arc::from([[-0.8, -0.8, 0.7], [0.0, 0.0, 0.0], [3.0, 3.0, 3.0]]),
        source_indices: Arc::from([0, 1, 2]),
    });
    let line_geometry = Arc::new(LineGeometryBatch3D {
        series_index: 1,
        positions: Arc::from([[-3.0, 0.0, 0.4], [3.0, 0.0, -0.4]]),
        source_indices: Arc::from([0, 1]),
        segments: Arc::from([[0, 1]]),
    });
    let mesh_geometry = Arc::new(MeshGeometryBatch3D {
        series_index: 2,
        vertices: Arc::from([
            MeshVertex3D {
                position: [-1.2, -0.9, 0.2],
                normal: [0.0, 0.0, 1.0],
                scalar: 0.0,
                source_index: 0,
            },
            MeshVertex3D {
                position: [2.8, -0.9, 0.2],
                normal: [0.0, 0.0, 1.0],
                scalar: 0.5,
                source_index: 1,
            },
            MeshVertex3D {
                position: [0.0, 1.1, -0.2],
                normal: [0.0, 0.0, 1.0],
                scalar: 1.0,
                source_index: 2,
            },
        ]),
        indices: Arc::from([0, 1, 2]),
    });
    let geometry = Arc::new(SceneGeometry3D {
        spheres: Vec::new(),
        points: vec![Arc::clone(&point_geometry)],
        lines: vec![Arc::clone(&line_geometry)],
        meshes: vec![Arc::clone(&mesh_geometry)],
    });
    let scene = Scene3D {
        spheres: Vec::new(),
        geometry,
        points: vec![PointBatch3D {
            geometry: point_geometry,
            style: PointStyle3D {
                color: Color::RED,
                marker: MarkerStyle::Circle,
                marker_size: 8.0,
                label: None,
            },
        }],
        lines: vec![LineBatch3D {
            geometry: line_geometry,
            style: StrokeStyle3D {
                color: Color::GREEN,
                line_width: 2.0,
                line_style: LineStyle::Solid,
                label: None,
            },
        }],
        meshes: vec![MeshBatch3D {
            geometry: mesh_geometry,
            style: MeshStyle3D {
                color: MeshColor3D::Scalar {
                    colormap: ColorMap::viridis(),
                    data_range: (0.0, 1.0),
                },
                shading: SurfaceShading::Unlit,
                two_sided: true,
                label: None,
            },
        }],
    };
    let bounds =
        Bounds3D::new(Point3D::new(-1.0, -1.0, -1.0), Point3D::new(1.0, 1.0, 1.0)).expect("bounds");
    let viewport = test_viewport(70, 55);
    let layout = Axis3Layout {
        canvas_width: viewport.width,
        canvas_height: viewport.height,
        viewport,
        camera: Camera3D::default()
            .perspective_deg(47.0)
            .prepare(viewport.width as f32 / viewport.height as f32, bounds)
            .expect("camera"),
        panes: Vec::new(),
        grid_lines: Vec::new(),
        box_edges: Vec::new(),
        tick_marks: Vec::new(),
        tick_labels: Vec::new(),
        axis_labels: Vec::new(),
        title: None,
        legend: None,
        colorbars: Vec::new(),
    };

    let serial = render_scene(
        &scene,
        &layout,
        96.0,
        SoftwareRenderOptions3D {
            quality: SoftwareQuality3D::Export,
            parallel: false,
        },
    )
    .expect("serial render");
    let parallel = render_scene(
        &scene,
        &layout,
        96.0,
        SoftwareRenderOptions3D {
            quality: SoftwareQuality3D::Export,
            parallel: true,
        },
    )
    .expect("parallel render");

    assert_eq!(serial.layer.pixels, parallel.layer.pixels);
    assert_eq!(serial.draw_calls, parallel.draw_calls);
    assert_eq!(serial.primitives_culled, parallel.primitives_culled);
    assert!(serial.primitives_culled > 0);
}
