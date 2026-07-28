//! Pixel parity between the two 3D rasterizers.
//!
//! `render()` and `render_gpu()` are supposed to be the same picture. Nothing
//! checked that, and they had drifted: triangle markers were upside down on the
//! CPU relative to both the GPU and the 2D marker paths, point sprites were
//! culled on their centre by one backend and clipped per pixel by the other, and
//! lighting was applied in a different colour space on each side.
//!
//! These tests run against whatever adapter is available — including a software
//! adapter such as `llvmpipe` or WARP, which is what CI is expected to use. If
//! no adapter exists at all the tests report and skip rather than fail, so a
//! machine without a GPU does not turn into a red build.
#![cfg(all(feature = "3d", feature = "gpu", not(target_arch = "wasm32")))]

use base64::Engine;
use ruviz::core::Image;
use ruviz::prelude::*;

/// Every marker style, so a divergence in the shared glyph predicate between
/// `software/raster.rs` and `shaders/point.wgsl` shows up somewhere.
const ALL_MARKERS: [MarkerStyle; 12] = [
    MarkerStyle::Circle,
    MarkerStyle::CircleOpen,
    MarkerStyle::Square,
    MarkerStyle::SquareOpen,
    MarkerStyle::Diamond,
    MarkerStyle::DiamondOpen,
    MarkerStyle::Triangle,
    MarkerStyle::TriangleDown,
    MarkerStyle::TriangleOpen,
    MarkerStyle::Plus,
    MarkerStyle::Cross,
    MarkerStyle::Star,
];

/// A fixed orthographic camera keeps both backends on the same geometry, and
/// orthographic projection avoids depending on near/far precision.
fn parity_camera() -> Camera3D {
    Camera3D::default()
        .azimuth_deg(-45.0)
        .elevation_deg(25.0)
        .orthographic()
}

struct RenderedPair {
    cpu: Image,
    gpu: Image,
}

/// Render the same builder twice, or `None` when no adapter is available.
fn rendered_pair(build: impl Fn() -> Scatter3DBuilder, context: &str) -> Option<RenderedPair> {
    let cpu = build().render().expect("CPU 3D render");
    match build().render_gpu() {
        Ok(gpu) => Some(RenderedPair { cpu, gpu }),
        Err(error) => {
            eprintln!("skipping 3D parity ({context}): no usable wgpu adapter: {error}");
            None
        }
    }
}

struct Difference {
    mean: f64,
    beyond_tolerance: f64,
}

fn compare(pair: &RenderedPair, tolerance: u8) -> Difference {
    assert_eq!(
        (pair.cpu.width, pair.cpu.height),
        (pair.gpu.width, pair.gpu.height),
        "both backends must produce the same canvas"
    );
    assert_eq!(pair.cpu.pixels.len(), pair.gpu.pixels.len());
    let mut total = 0_u64;
    let mut beyond = 0_u64;
    let channels = pair.cpu.pixels.len() as u64;
    for (left, right) in pair.cpu.pixels.iter().zip(&pair.gpu.pixels) {
        let delta = u64::from(left.abs_diff(*right));
        total += delta;
        if delta > u64::from(tolerance) {
            beyond += 1;
        }
    }
    Difference {
        mean: total as f64 / channels as f64,
        beyond_tolerance: beyond as f64 / channels as f64,
    }
}

/// Vertical centre of mass of the pixels close to `color`, measured from the
/// middle of the glyph's own bounding box so it does not depend on where the
/// plot viewport happens to sit on the canvas.
///
/// A positive result means the glyph is heavier towards the bottom of the
/// screen, which is what an upward-pointing triangle looks like.
fn glyph_mass_offset(image: &Image, color: Color) -> Option<f64> {
    let mut weighted = 0.0;
    let mut count = 0.0;
    let mut top = usize::MAX;
    let mut bottom = 0_usize;
    for y in 0..image.height as usize {
        for x in 0..image.width as usize {
            let offset = (y * image.width as usize + x) * 4;
            let pixel = &image.pixels[offset..offset + 4];
            let close = pixel[0].abs_diff(color.r) < 40
                && pixel[1].abs_diff(color.g) < 40
                && pixel[2].abs_diff(color.b) < 40;
            if close {
                weighted += y as f64;
                count += 1.0;
                top = top.min(y);
                bottom = bottom.max(y);
            }
        }
    }
    (count > 32.0 && bottom > top).then(|| weighted / count - (top as f64 + bottom as f64) / 2.0)
}

#[test]
fn every_marker_style_matches_between_the_cpu_and_gpu_rasterizers() {
    for marker in ALL_MARKERS {
        let build = move || {
            scatter3d(&[-1.0, 0.0, 1.0], &[-1.0, 0.0, 1.0], &[-1.0, 0.0, 1.0])
                .color(Color::RED)
                .marker(marker)
                .marker_size(24.0)
                .camera(parity_camera())
                .size(3.0, 2.4)
                .dpi(72)
        };
        let Some(pair) = rendered_pair(build, &format!("{marker:?}")) else {
            return;
        };
        // Sample positions and coverage resolution genuinely differ between a
        // 4x MSAA GPU pass and the CPU's fixed 4-sample grid, and an adapter
        // without MSAA has no edge antialiasing at all. Bound the disagreement
        // rather than demanding equality; an inverted or missing glyph moves
        // far more area than any antialiasing seam can.
        let difference = compare(&pair, 48);
        assert!(
            difference.mean < 16.0,
            "{marker:?}: mean per-channel CPU/GPU difference {:.2} is too large",
            difference.mean
        );
        assert!(
            difference.beyond_tolerance < 0.10,
            "{marker:?}: {:.1}% of channels differ by more than 48",
            difference.beyond_tolerance * 100.0
        );
    }
}

#[test]
fn triangle_markers_point_the_same_way_on_both_rasterizers() {
    for (marker, mass_below_centre) in [
        (MarkerStyle::Triangle, true),
        (MarkerStyle::TriangleDown, false),
    ] {
        let build = move || {
            scatter3d(&[0.0], &[0.0], &[0.0])
                .color(Color::RED)
                .marker(marker)
                .marker_size(64.0)
                .camera(parity_camera())
                .size(3.0, 2.4)
                .dpi(72)
        };
        let Some(pair) = rendered_pair(build, &format!("{marker:?} orientation")) else {
            return;
        };
        let cpu = glyph_mass_offset(&pair.cpu, Color::RED)
            .expect("CPU marker must cover enough pixels to weigh");
        let gpu = glyph_mass_offset(&pair.gpu, Color::RED)
            .expect("GPU marker must cover enough pixels to weigh");
        // A triangle's mass sits toward its base, so an upward-pointing glyph
        // is heavier towards the bottom of the screen.
        assert_eq!(
            cpu > 0.0,
            mass_below_centre,
            "{marker:?}: the CPU glyph is drawn upside down (mass offset {cpu:.2})"
        );
        assert_eq!(
            gpu > 0.0,
            mass_below_centre,
            "{marker:?}: the GPU glyph is drawn upside down (mass offset {gpu:.2})"
        );
        assert!(
            (cpu - gpu).abs() < 4.0,
            "{marker:?}: CPU and GPU glyph mass disagree ({cpu:.2} vs {gpu:.2})"
        );
    }
}

#[test]
fn lit_surfaces_have_the_same_brightness_on_both_rasterizers() {
    // Lighting used to be applied to sRGB bytes on the CPU and to linear values
    // on the GPU, so the same figure was up to twice as bright depending on
    // which machine rendered it.
    let build = || {
        surface(
            &[-1.0, 0.0, 1.0],
            &[-1.0, 0.0, 1.0],
            &[[0.0, 0.6, 0.0], [0.6, 1.0, 0.6], [0.0, 0.6, 0.0]],
        )
        .color(Color::from_rgb(120, 180, 240))
        .shading(SurfaceShading::Smooth)
        .camera(parity_camera())
        .size(3.0, 2.4)
        .dpi(72)
    };
    let cpu = build().render().expect("CPU surface render");
    let gpu = match build().render_gpu() {
        Ok(image) => image,
        Err(error) => {
            eprintln!("skipping 3D shading parity: no usable wgpu adapter: {error}");
            return;
        }
    };
    let pair = RenderedPair { cpu, gpu };
    let difference = compare(&pair, 48);
    assert!(
        difference.mean < 16.0,
        "mean per-channel CPU/GPU difference {:.2} is too large for a lit surface",
        difference.mean
    );
    assert!(
        difference.beyond_tolerance < 0.10,
        "{:.1}% of channels differ by more than 48 on a lit surface",
        difference.beyond_tolerance * 100.0
    );
}

#[test]
fn the_exported_depth_layer_carries_straight_alpha_not_coverage_premultiplied_colour() {
    // The hybrid SVG embeds the raw depth layer as a PNG, and PNG is straight
    // alpha. A partly covered pixel of a pure red marker must therefore still
    // read (255, 0, 0) with a reduced alpha. Folding coverage into RGB gives
    // (127, 0, 0, 127) instead, which every compositor darkens a second time —
    // the halo around every silhouette.
    let svg = scatter3d(&[0.0], &[0.0], &[0.0])
        .color(Color::RED)
        .marker(MarkerStyle::Circle)
        .marker_size(48.0)
        .camera(parity_camera())
        .size(3.0, 2.4)
        .dpi(72)
        .render_to_svg()
        .expect("hybrid SVG");
    let encoded = svg
        .split_once("data:image/png;base64,")
        .expect("embedded PNG data URI")
        .1
        .split('"')
        .next()
        .expect("data URI terminator");
    let png = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .expect("decode embedded depth layer");
    let layer = image::load_from_memory(&png)
        .expect("decode embedded depth layer")
        .to_rgba8();

    let mut partial = 0_usize;
    let mut premultiplied_looking = 0_usize;
    for pixel in layer.pixels() {
        let [red, green, blue, alpha] = pixel.0;
        if !(1..255).contains(&alpha) {
            continue;
        }
        partial += 1;
        // Straight alpha keeps the source colour intact at every coverage.
        if red < 250 || green > 5 || blue > 5 {
            premultiplied_looking += 1;
        }
    }
    assert!(
        partial > 16,
        "an antialiased marker must produce partially covered pixels; got {partial}"
    );
    assert_eq!(
        premultiplied_looking, 0,
        "{premultiplied_looking} of {partial} antialiased pixels lost their source colour"
    );
}
