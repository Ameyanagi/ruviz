#![cfg(feature = "3d")]

use std::collections::BTreeSet;

use base64::Engine;
use ruviz::core::Surface3DBuilder;
use ruviz::prelude::*;

const SURFACE_COLOR: Color = Color {
    r: 17,
    g: 93,
    b: 211,
    a: 255,
};

fn parity_plot() -> Surface3DBuilder {
    surface(
        &[-1.0, 0.0, 1.0],
        &[-1.0, 0.0, 1.0],
        &[[-0.3, 0.2, 0.8], [0.1, 0.7, -0.2], [0.9, -0.4, 0.4]],
    )
    .color(SURFACE_COLOR)
    .shading(SurfaceShading::Unlit)
    .camera(
        Camera3D::default()
            .azimuth_deg(-43.0)
            .elevation_deg(29.0)
            .perspective_deg(48.0),
    )
    .title("3d parity")
    .xlabel("axis-x")
    .ylabel("axis-y")
    .zlabel("axis-z")
    .figure_size(3.2, 2.4)
    .dpi(80)
}

fn embedded_png(svg: &str) -> Vec<u8> {
    let encoded = svg
        .split_once("data:image/png;base64,")
        .expect("embedded PNG data URI")
        .1
        .split('"')
        .next()
        .expect("data URI terminator");
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .expect("valid embedded PNG")
}

fn exact_color_pixels(image: &image::RgbaImage, color: Color) -> BTreeSet<(u32, u32)> {
    image
        .enumerate_pixels()
        .filter_map(|(x, y, pixel)| {
            (pixel.0 == [color.r, color.g, color.b, color.a]).then_some((x, y))
        })
        .collect()
}

#[test]
fn png_and_hybrid_svg_share_the_same_depth_layer_geometry() {
    let rendered = parity_plot().render().expect("composited PNG image");
    let png_bytes = parity_plot().render_png_bytes().expect("PNG terminal");
    let decoded_png = image::load_from_memory(&png_bytes)
        .expect("decode PNG terminal")
        .to_rgba8();
    assert_eq!(decoded_png.dimensions(), (rendered.width, rendered.height));
    assert_eq!(decoded_png.as_raw(), &rendered.pixels);

    let svg = parity_plot().render_to_svg().expect("hybrid SVG");
    assert_eq!(svg.matches("<image ").count(), 1);
    assert!(svg.matches("<text ").count() >= 4);
    assert!(svg.contains("<line "));
    let depth_layer = image::load_from_memory(&embedded_png(&svg))
        .expect("decode SVG depth layer")
        .to_rgba8();
    assert_eq!(depth_layer.dimensions(), decoded_png.dimensions());

    let layer_geometry = exact_color_pixels(&depth_layer, SURFACE_COLOR);
    let png_geometry = exact_color_pixels(&decoded_png, SURFACE_COLOR);
    assert!(
        layer_geometry.len() > 100,
        "the semantic probe needs a substantial surface interior"
    );
    let shared_geometry = layer_geometry.intersection(&png_geometry).count();
    assert_eq!(
        shared_geometry,
        png_geometry.len(),
        "every unchanged surface-color pixel in the PNG must come from the embedded depth layer"
    );
    assert!(
        shared_geometry * 100 >= layer_geometry.len() * 80,
        "PNG composition moved or recolored the depth-tested surface: \
         {shared_geometry}/{} exact-color pixels survived",
        layer_geometry.len()
    );

    let repeated_svg = parity_plot().render_to_svg().expect("repeated hybrid SVG");
    assert_eq!(embedded_png(&svg), embedded_png(&repeated_svg));
}

#[test]
fn aggressive_camera_clipping_is_deterministic_for_both_projection_modes() {
    let x = [-1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, 0.0];
    let y = [-1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 0.0];
    let z = [-1.0, -1.0, -1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 0.0];

    for camera in [
        Camera3D::default().orthographic().zoom(12.0),
        Camera3D::default().perspective_deg(45.0).zoom(12.0),
    ] {
        let render = || {
            scatter3d(&x, &y, &z)
                .color(Color::RED)
                .marker_size(8.0)
                .camera(camera)
                .figure_size(2.0, 1.5)
                .dpi(72)
                .benchmark_render_with_diagnostics()
                .expect("clipped render")
        };
        let (first, diagnostics) = render();
        let (second, repeated_diagnostics) = render();
        assert_eq!(first.pixels, second.pixels);
        assert_eq!(
            diagnostics.primitives_culled,
            repeated_diagnostics.primitives_culled
        );
        assert!(
            diagnostics.primitives_culled >= 8,
            "the eight cube corners should be clipped at high zoom"
        );
        assert!(
            first
                .pixels
                .chunks_exact(4)
                .any(|pixel| pixel == [255, 0, 0, 255]),
            "the center marker should remain visible"
        );
    }
}

#[cfg(all(feature = "pdf", not(target_arch = "wasm32")))]
fn pdf_image_streams(pdf: &[u8]) -> Vec<Vec<u8>> {
    fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    let mut streams = Vec::new();
    let mut cursor = 0;
    while let Some(image_offset) = find(&pdf[cursor..], b"/Subtype /Image") {
        let image_offset = cursor + image_offset;
        let stream_marker = find(&pdf[image_offset..], b"stream\n").expect("PDF image stream");
        let stream_start = image_offset + stream_marker + 7;
        let stream_end = stream_start
            + find(&pdf[stream_start..], b"\nendstream").expect("PDF image stream terminator");
        streams.push(pdf[stream_start..stream_end].to_vec());
        cursor = stream_end;
    }
    streams.sort();
    streams
}

#[cfg(all(feature = "pdf", not(target_arch = "wasm32")))]
#[test]
fn pdf_save_preserves_the_hybrid_svg_image_payload() {
    let svg = parity_plot().render_to_svg().expect("hybrid SVG");
    let expected_pdf = ruviz::export::svg_to_pdf(&svg).expect("direct SVG-to-PDF conversion");
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("parity.pdf");
    parity_plot().save(&path).expect("PDF terminal");
    let saved_pdf = std::fs::read(path).expect("saved PDF");

    assert!(saved_pdf.starts_with(b"%PDF-"));
    let expected_streams = pdf_image_streams(&expected_pdf);
    let saved_streams = pdf_image_streams(&saved_pdf);
    assert!(
        !saved_streams.is_empty(),
        "the PDF must contain the hybrid depth-tested raster layer"
    );
    assert_eq!(saved_streams, expected_streams);
    assert_eq!(
        saved_pdf
            .windows(b"/Width 256".as_slice().len())
            .filter(|window| *window == b"/Width 256")
            .count(),
        expected_pdf
            .windows(b"/Width 256".as_slice().len())
            .filter(|window| *window == b"/Width 256")
            .count()
    );
    assert_eq!(
        saved_pdf
            .windows(b"/Height 192".as_slice().len())
            .filter(|window| *window == b"/Height 192")
            .count(),
        expected_pdf
            .windows(b"/Height 192".as_slice().len())
            .filter(|window| *window == b"/Height 192")
            .count()
    );
}
