//! Animated 3D surface orbit.
//!
//! Run with:
//! `cargo run --no-default-features --features 3d,animation --example doc_surface3d_animation`
//!
//! Pass an optional output path as the first argument.

use std::path::PathBuf;

use ruviz::animation::encoders::GifEncoder;
use ruviz::animation::{Encoder, Quality};
use ruviz::prelude::*;

const FONT_BYTES: &[u8] = include_bytes!("../src/dejavu-sans.ttf");
const FONT_FAMILY: &str = "DejaVu Sans";
const FRAME_COUNT: usize = 60;
const FRAMES_PER_SECOND: u64 = 20;

fn surface_data(side: usize) -> (Vec<f64>, Vec<f64>, Vec<Vec<f64>>) {
    let coordinate = |index: usize| -3.0 + 6.0 * index as f64 / (side - 1) as f64;
    let x: Vec<f64> = (0..side).map(coordinate).collect();
    let y = x.clone();
    let z = y
        .iter()
        .map(|&yv| {
            x.iter()
                .map(|&xv| {
                    let radius = (xv * xv + yv * yv).sqrt();
                    0.85 * (radius * 1.7).sin() / (1.0 + 0.14 * radius * radius) + 0.09 * xv
                        - 0.06 * yv
                })
                .collect()
        })
        .collect();
    (x, y, z)
}

fn rgb_pixels(image: &ruviz::core::Image) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(image.width as usize * image.height as usize * 3);
    for pixel in image.pixels.chunks_exact(4) {
        rgb.extend_from_slice(&pixel[..3]);
    }
    rgb
}

fn main() -> PlotResult<()> {
    ruviz::render::register_font_bytes(FONT_BYTES.to_vec())?;

    let output = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("generated/examples/3d/surface3d_orbit.gif"));
    let (x, y, z) = surface_data(41);

    let mut encoder =
        GifEncoder::new(&output, Quality::Medium)?.with_framerate(FRAMES_PER_SECOND as f64)?;
    encoder.init(640, 480)?;

    for frame_index in 0..FRAME_COUNT {
        let progress = frame_index as f32 / FRAME_COUNT as f32;
        let azimuth = -52.0 + progress * 360.0;
        let mut theme = Theme::publication();
        theme.font_family = FONT_FAMILY.to_string();

        let image = surface(&x, &y, &z)
            .title("Damped radial surface — 360° orbit")
            .cmap(ColorMap::coolwarm())
            .shading(SurfaceShading::Smooth)
            .sampling(SurfaceSampling::Full)
            .theme(theme)
            .camera(
                Camera3D::default()
                    .azimuth_deg(azimuth)
                    .elevation_deg(28.0)
                    .axis_aspect(AxisAspect3D::Equal)
                    .orthographic(),
            )
            .size(6.4, 4.8)
            .dpi(100)
            .render()?;

        let timestamp_ms = frame_index as u64 * 1_000 / FRAMES_PER_SECOND;
        encoder.encode_frame(&rgb_pixels(&image), timestamp_ms)?;
    }

    Box::new(encoder).finalize()?;
    println!("Saved {}", output.display());
    Ok(())
}
