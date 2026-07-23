#![cfg(feature = "3d")]

use ruviz::core::{Camera3D, PlottingError, Projection3D};
use ruviz::prelude::*;

// These functions are compile contracts for the one documented construction
// path.
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

#[cfg(feature = "ndarray_support")]
#[test]
fn ndarray_vectors_views_and_matrices_use_the_same_3d_api() {
    use ndarray::{Array1, array};

    let x = Array1::from_vec(vec![0_f32, 1.0, 2.0]);
    let y = Array1::from_vec(vec![0_f32, 1.0, 2.0]);
    let z = Array1::from_vec(vec![0_f32, 1.0, 0.0]);
    scatter3d(&x.view(), &y.view(), &z.view())
        .validate()
        .expect("ndarray point views");

    let grid = array![[0_f32, 1.0, 0.0], [1.0, 2.0, 1.0]];
    surface(&x, &array![0_f32, 1.0], &grid)
        .validate()
        .expect("ndarray surface");
}

#[cfg(any(feature = "nalgebra_support", feature = "nalgebra"))]
#[test]
fn nalgebra_vectors_and_matrices_use_the_same_3d_api() {
    let x = nalgebra::DVector::from_vec(vec![0.0, 1.0, 2.0]);
    let y = nalgebra::DVector::from_vec(vec![0.0, 1.0, 2.0]);
    let z = nalgebra::DVector::from_vec(vec![0.0, 1.0, 0.0]);
    line3d(&x, &y, &z)
        .validate()
        .expect("nalgebra point vectors");

    let grid_y = nalgebra::DVector::from_vec(vec![0.0, 1.0]);
    let grid = nalgebra::DMatrix::from_row_slice(2, 3, &[0.0, 1.0, 0.0, 1.0, 2.0, 1.0]);
    wireframe(&x, &grid_y, &grid)
        .validate()
        .expect("nalgebra surface matrix");
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
fn retained_interaction_api_is_small_and_generation_safe() {
    let mut session = scatter3d(&[0.0], &[0.0], &[0.0])
        .interactive_session()
        .expect("session");
    let initial = session.camera_snapshot();
    session.orbit(12.0, -4.0).expect("orbit");
    session.pan(3.0, 2.0).expect("pan");
    session.zoom_by(1.1).expect("zoom");
    assert_ne!(session.camera(), initial.camera);
    session.restore_camera(initial).expect("restore");
    assert_eq!(session.camera(), initial.camera);
    let replacement = scatter3d(&[1.0], &[2.0], &[3.0])
        .interactive_session_with_view(initial)
        .expect("keep-view replacement");
    assert_eq!(replacement.camera(), initial.camera);
    assert_ne!(
        replacement.camera_snapshot().scene_generation,
        initial.scene_generation
    );
    let image = session.render().expect("interactive CPU frame");
    assert_eq!(
        image.pixels.len(),
        image.width as usize * image.height as usize * 4
    );
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

#[test]
fn compile_diagnostics_are_structured_and_truthful() {
    let diagnostics = surface(&[0.0, 1.0], &[0.0, 1.0], &[[0.0, 1.0], [1.0, 2.0]])
        .benchmark_compile_scene_with_diagnostics()
        .expect("scene diagnostics");
    assert_eq!(diagnostics.scene_compiles, 1);
    assert_eq!(diagnostics.triangulations, 1);
    assert_eq!(diagnostics.triangles_submitted, 2);
    assert_eq!(diagnostics.actual_backend, "unresolved");
    assert_eq!(diagnostics.readback_bytes, 0);
}

#[test]
fn cpu_render_is_deterministic_and_reports_the_actual_backend() {
    fn render() -> (ruviz::core::Image, ruviz::core::RenderDiagnostics3D) {
        surface(&[0.0, 1.0], &[0.0, 1.0], &[[0.0, 0.75], [0.25, 1.0]])
            .title("CPU 3d")
            .xlabel("x")
            .ylabel("y")
            .zlabel("z")
            .figure_size(2.4, 1.8)
            .dpi(72)
            .benchmark_render_with_diagnostics()
            .expect("software render")
    }

    let (first, diagnostics) = render();
    let (second, _) = render();
    assert_eq!((first.width, first.height), (172, 129));
    assert_eq!(
        first.pixels.len(),
        first.width as usize * first.height as usize * 4
    );
    assert_eq!(first.pixels, second.pixels);
    assert_eq!(diagnostics.actual_backend, "cpu3d");
    assert_eq!(diagnostics.draw_calls, 1);
    assert_eq!(diagnostics.triangles_submitted, 2);
    assert_eq!(diagnostics.readback_bytes, 0);
}

#[cfg(not(feature = "gpu"))]
#[test]
fn auto_render_truthfully_reports_cpu_when_gpu_feature_is_disabled() {
    let (image, diagnostics) = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
        .render_auto_with_diagnostics()
        .expect("auto CPU fallback");

    assert!(image.pixels.iter().any(|&channel| channel != 0));
    assert_eq!(diagnostics.actual_backend, "cpu3d");
    assert_eq!(diagnostics.adapter_name, None);
    assert_eq!(diagnostics.readback_bytes, 0);
    assert_eq!(
        diagnostics.fallback_reason.as_deref(),
        Some("direct native 3d GPU rendering is unavailable because the `gpu` feature is disabled")
    );
}

#[test]
fn png_and_hybrid_svg_terminals_are_live() {
    let png = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[1.0, 0.0])
        .figure_size(2.0, 1.5)
        .dpi(72)
        .render_png_bytes()
        .expect("PNG");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));

    let svg = line3d(&[0.0, 1.0], &[0.0, 1.0], &[1.0, 0.0])
        .title("Hybrid 3d")
        .figure_size(2.0, 1.5)
        .dpi(72)
        .render_to_svg()
        .expect("SVG");
    assert!(svg.contains("data:image/png;base64,"));
    assert!(svg.contains("Hybrid 3d"));
    assert_eq!(svg.matches("<image ").count(), 1);
}

#[test]
fn one_call_surface_pick_returns_source_indices_and_data_coordinates() {
    let hit = surface(&[-1.0, 1.0], &[-1.0, 1.0], &[[0.0, 0.0], [0.0, 0.0]])
        .figure_size(2.4, 1.8)
        .dpi(72)
        .pick(95.0, 52.5)
        .expect("pick")
        .expect("surface hit");
    assert_eq!(hit.primitive, ruviz::core::PickPrimitive3D::SurfaceTriangle);
    assert_eq!(hit.series_index, 0);
    assert!(hit.source_indices == [0, 1, 3] || hit.source_indices == [0, 3, 2]);
    assert!(hit.point.x.abs() <= 1.0e-4);
    assert!(hit.point.y.abs() <= 1.0e-4);
    assert!(hit.point.z.abs() <= 1.0e-4);
    assert!((hit.barycentric.iter().sum::<f32>() - 1.0).abs() <= 1.0e-5);

    assert!(
        surface(&[-1.0, 1.0], &[-1.0, 1.0], &[[0.0, 0.0], [0.0, 0.0]],)
            .figure_size(2.4, 1.8)
            .dpi(72)
            .pick(0.0, 0.0)
            .expect("outside pick")
            .is_none()
    );
}

#[test]
fn large_offsets_and_both_projection_modes_render_semantically() {
    let x = [1.0e12, 1.0e12 + 1.0];
    let y = [-1.0e12, -1.0e12 + 1.0];
    let z = [[4.0e12, 4.0e12 + 0.5], [4.0e12 + 0.5, 4.0e12 + 1.0]];
    let orthographic = surface(&x, &y, &z)
        .figure_size(2.0, 1.5)
        .dpi(72)
        .render()
        .expect("large-offset orthographic render");
    let perspective = surface(&x, &y, &z)
        .perspective_deg(45.0)
        .figure_size(2.0, 1.5)
        .dpi(72)
        .render()
        .expect("large-offset perspective render");
    assert_eq!(
        (orthographic.width, orthographic.height),
        (perspective.width, perspective.height)
    );
    assert_ne!(orthographic.pixels, perspective.pixels);
    assert!(
        orthographic
            .pixels
            .chunks_exact(4)
            .any(|pixel| pixel[..3] != [255, 255, 255])
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn save_selects_png_and_svg_from_the_extension() {
    let directory = tempfile::tempdir().expect("tempdir");
    let png = directory.path().join("plot.png");
    let svg = directory.path().join("plot.svg");
    scatter3d(&[0.0], &[0.0], &[0.0])
        .figure_size(1.5, 1.2)
        .dpi(72)
        .save(&png)
        .expect("save PNG");
    scatter3d(&[0.0], &[0.0], &[0.0])
        .figure_size(1.5, 1.2)
        .dpi(72)
        .save(&svg)
        .expect("save SVG");
    assert!(
        std::fs::read(png)
            .expect("read PNG")
            .starts_with(b"\x89PNG\r\n\x1a\n")
    );
    assert!(
        std::fs::read_to_string(svg)
            .expect("read SVG")
            .contains("data:image/png;base64,")
    );
}

#[cfg(all(feature = "pdf", not(target_arch = "wasm32")))]
#[test]
fn hybrid_pdf_save_uses_the_same_depth_tested_layer() {
    let directory = tempfile::tempdir().expect("tempdir");
    let pdf = directory.path().join("plot.pdf");
    surface(&[0.0, 1.0], &[0.0, 1.0], &[[0.0, 1.0], [1.0, 0.0]])
        .figure_size(2.0, 1.5)
        .dpi(72)
        .save(&pdf)
        .expect("save PDF");
    assert!(std::fs::read(pdf).expect("read PDF").starts_with(b"%PDF-"));
}
