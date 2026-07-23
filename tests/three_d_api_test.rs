#![cfg(feature = "3d")]

use ruviz::core::{Camera3D, PlottingError, Projection3D};
use ruviz::prelude::*;

// These functions are compile contracts for the one documented construction
// path. They are intentionally not executed until the depth renderer lands.
#[allow(dead_code)]
fn canonical_scatter_save(x: &[f64], y: &[f64], z: &[f64]) -> ruviz::core::Result<()> {
    scatter3d(x, y, z).save("scatter3d.png")
}

#[allow(dead_code)]
fn canonical_surface_save(x: &[f64], y: &[f64], z: &Vec<Vec<f64>>) -> ruviz::core::Result<()> {
    surface(x, y, z)
        .title("Surface")
        .xlabel("x")
        .ylabel("y")
        .zlabel("z")
        .save("surface.png")
}

#[test]
fn canonical_functions_accept_common_numeric_inputs() {
    scatter3d(&[0_i32, 1, 2], &[1_f32, 2.0, 3.0], &[2.0, 3.0, 4.0])
        .validate()
        .expect("mixed primitive arrays");

    let x = vec![0_f32, 1.0, 2.0];
    let y = vec![0_f64, 1.0];
    let z = vec![vec![0_f32, 1.0, 2.0], vec![1.0, 2.0, 3.0]];
    surface(&x, &y, &z).validate().expect("nested f32 grid");
    wireframe(&x, &y, &z).validate().expect("wireframe grid");

    line3d(&[0, 1], &[1, 2], &[2, 3])
        .validate()
        .expect("integer line");
}

#[test]
fn builders_chain_multiple_3d_series_without_an_end_call() {
    scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
        .marker_size(8.0)
        .line3d(&[0.0, 1.0], &[1.0, 0.0], &[0.5, 0.5])
        .line_width(2.0)
        .surface(&[0.0, 1.0], &[0.0, 1.0], &[[0.0, 1.0], [1.0, 0.0]])
        .validate()
        .expect("multi-series 3D plot");
}

#[test]
fn named_degree_camera_api_is_unambiguous() {
    let camera = Camera3D::default()
        .azimuth_deg(45.0)
        .elevation_deg(25.0)
        .roll_deg(5.0)
        .perspective_deg(40.0);
    assert_eq!(
        camera.projection(),
        Projection3D::Perspective {
            vertical_fov_deg: 40.0
        }
    );
    scatter3d(&[0.0], &[0.0], &[0.0])
        .camera(camera)
        .validate()
        .expect("valid named-degree camera");
}

#[test]
fn grid_diagnostics_name_expected_and_actual_shapes() {
    let error = surface(&[0.0, 1.0, 2.0], &[0.0, 1.0], &[[0.0, 1.0], [1.0, 2.0]])
        .validate()
        .expect_err("transposed dimensions");
    assert!(matches!(
        &error,
        PlottingError::GridShapeMismatch {
            operation: "surface",
            expected_rows: 2,
            expected_columns: 3,
            actual_rows: 2,
            actual_columns: 2,
        }
    ));
    assert_eq!(
        error.to_string(),
        "surface: z shape must be (y.len(), x.len()) = (2, 3), got (2, 2)"
    );
}

#[test]
fn ragged_grid_diagnostic_names_surface_and_row() {
    let error = surface(
        &[0.0, 1.0, 2.0],
        &[0.0, 1.0],
        &vec![vec![0.0, 1.0, 2.0], vec![1.0, 2.0]],
    )
    .validate()
    .expect_err("ragged grid");
    assert!(matches!(
        &error,
        PlottingError::RaggedData2D {
            context: "surface",
            row: 1,
            expected_columns: 3,
            actual_columns: 2,
        }
    ));
    assert_eq!(error.to_string(), "surface: row 1 has 2 values, expected 3");
}
