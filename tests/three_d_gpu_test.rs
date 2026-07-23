#![cfg(all(feature = "3d", feature = "gpu", not(target_arch = "wasm32")))]

use ruviz::prelude::*;

#[test]
fn required_adapter_executes_direct_mesh_line_and_point_draws() {
    let _ = env_logger::builder().is_test(true).try_init();
    let x = [-1.0, 0.0, 1.0];
    let y = [-1.0, 0.0, 1.0];
    let z = [[0.0, 0.2, 0.0], [0.2, 1.0, 0.2], [0.0, 0.2, 0.0]];
    let (image, diagnostics) = surface(&x, &y, &z)
        .wireframe(&x, &y, &z)
        .line3d(&x, &y, &[0.0, 1.0, 0.0])
        .scatter3d(&x, &y, &[0.0, 1.0, 0.0])
        .figure_size(2.4, 1.8)
        .dpi(72)
        .benchmark_render_gpu_with_diagnostics()
        .expect("required direct 3d adapter");

    assert_eq!(diagnostics.actual_backend, "gpu3d");
    assert!(
        diagnostics
            .adapter_name
            .as_deref()
            .is_some_and(|name| !name.is_empty())
    );
    assert!(matches!(diagnostics.sample_count, 1 | 4));
    assert_eq!(diagnostics.fallback_reason, None);
    assert_eq!(diagnostics.camera_uniform_writes, 1);
    assert_eq!(diagnostics.draw_calls, 4);
    assert_eq!(diagnostics.points_submitted, 3);
    assert_eq!(diagnostics.triangles_submitted, 8);
    assert!(diagnostics.vertex_upload_bytes > 0);
    assert!(diagnostics.index_upload_bytes > 0);
    assert!(diagnostics.buffer_creations > 0);
    assert!(diagnostics.readback_bytes >= u64::from(image.width) * u64::from(image.height) * 4);
    assert!(
        image
            .pixels
            .chunks_exact(4)
            .any(|pixel| pixel[..3] != [255, 255, 255])
    );
    if let Some(path) = std::env::var_os("RUVIZ_3D_GPU_TEST_OUTPUT") {
        std::fs::write(path, image.encode_png().expect("encode GPU preview"))
            .expect("write GPU preview");
    }
}

#[test]
fn auto_render_reports_gpu_only_after_direct_wgpu_execution() {
    let (image, diagnostics) = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
        .render_auto_with_diagnostics()
        .expect("required direct 3d adapter");

    assert!(image.pixels.iter().any(|&channel| channel != 0));
    assert_eq!(diagnostics.actual_backend, "gpu3d");
    assert!(
        diagnostics
            .adapter_name
            .as_deref()
            .is_some_and(|name| !name.is_empty())
    );
    assert_eq!(diagnostics.fallback_reason, None);
    assert!(diagnostics.draw_calls > 0);
    assert!(diagnostics.readback_bytes > 0);
}

#[test]
fn explicit_gpu_save_keeps_png_and_hybrid_svg_simple() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let png = directory.path().join("surface.png");
    let svg = directory.path().join("surface.svg");
    let x = [-1.0, 1.0];
    let y = [-1.0, 1.0];
    let z = [[0.0, 0.0], [0.0, 1.0]];

    surface(&x, &y, &z)
        .save_gpu(&png)
        .expect("direct GPU PNG save");
    surface(&x, &y, &z)
        .save_gpu(&svg)
        .expect("direct GPU hybrid SVG save");

    assert!(
        std::fs::read(png)
            .expect("read PNG")
            .starts_with(b"\x89PNG\r\n\x1a\n")
    );
    let svg = std::fs::read_to_string(svg).expect("read SVG");
    assert!(svg.contains("<svg"));
    assert_eq!(svg.matches("<image ").count(), 1);
}

#[test]
fn each_direct_gpu_primitive_produces_visible_colored_fragments() {
    let x = [-1.0, 0.0, 1.0];
    let y = [-1.0, 0.0, 1.0];
    let points_z = [0.0, 1.0, 0.0];
    let grid_z = [[0.0, 0.2, 0.0], [0.2, 1.0, 0.2], [0.0, 0.2, 0.0]];
    let images = [
        scatter3d(&x, &y, &points_z)
            .color(Color::RED)
            .marker_size(12.0)
            .render_gpu()
            .expect("scatter GPU"),
        line3d(&x, &y, &points_z)
            .color(Color::RED)
            .line_width(3.0)
            .render_gpu()
            .expect("line GPU"),
        surface(&x, &y, &grid_z)
            .color(Color::RED)
            .render_gpu()
            .expect("surface GPU"),
        wireframe(&x, &y, &grid_z)
            .color(Color::RED)
            .line_width(3.0)
            .render_gpu()
            .expect("wireframe GPU"),
    ];
    for image in images {
        assert!(
            image.pixels.chunks_exact(4).any(|pixel| {
                pixel[0] > 100
                    && u16::from(pixel[0]) > u16::from(pixel[1]).saturating_mul(2)
                    && u16::from(pixel[0]) > u16::from(pixel[2]).saturating_mul(2)
            }),
            "direct GPU primitive produced no red-dominant fragments"
        );
    }
}

#[test]
fn retained_camera_session_has_no_readback_or_warm_uploads() {
    let x = [-1.0, 0.0, 1.0];
    let y = [-1.0, 0.0, 1.0];
    let z = [[0.0, 0.2, 0.0], [0.2, 1.0, 0.2], [0.0, 0.2, 0.0]];
    let mut session = surface(&x, &y, &z)
        .benchmark_gpu_session()
        .expect("retained GPU session");
    let first = session.render_no_readback().expect("first retained frame");
    assert_eq!(first.actual_backend, "gpu3d");
    assert_eq!(first.readback_bytes, 0);
    assert!(first.vertex_upload_bytes > 0);
    assert!(first.index_upload_bytes > 0);

    for frame_index in 0..128 {
        let warm = session
            .render_camera_no_readback(
                Camera3D::default().azimuth_deg(20.0 + frame_index as f32 * 0.25),
            )
            .expect("warm retained frame");
        assert_eq!(warm.actual_backend, "gpu3d");
        assert_eq!(warm.scene_compiles, 0);
        assert_eq!(warm.triangulations, 0);
        assert_eq!(warm.normal_recomputations, 0);
        assert_eq!(warm.vertex_upload_bytes, 0);
        assert_eq!(warm.index_upload_bytes, 0);
        assert_eq!(warm.buffer_creations, 0);
        assert_eq!(warm.camera_uniform_writes, 1);
        assert_eq!(warm.readback_bytes, 0);
    }
}

#[test]
fn interactive_gpu_image_path_is_truthfully_labeled_as_readback_fallback() {
    let mut session = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
        .interactive_session()
        .expect("interactive session");
    let (image, diagnostics) = session
        .render_gpu_readback()
        .expect("interactive GPU readback");
    assert!(image.pixels.iter().any(|&channel| channel != 0));
    assert_eq!(diagnostics.actual_backend, "gpu3d-readback-fallback");
    assert!(diagnostics.fallback_reason.is_some());
    assert!(diagnostics.readback_bytes > 0);
    assert!(diagnostics.adapter_name.is_some());
}
