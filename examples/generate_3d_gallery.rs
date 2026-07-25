//! Generate deterministic fixed-camera images for the 3D gallery and goldens.

use std::path::Path;

use ruviz::prelude::*;

const GOLDEN_FONT_BYTES: &[u8] = include_bytes!("../src/dejavu-sans.ttf");
pub const GOLDEN_FONT_FAMILY: &str = "DejaVu Sans";

pub const THREE_D_GALLERY_FIXTURES: [&str; 8] = [
    "scatter3d.png",
    "line3d.png",
    "surface3d.png",
    "wireframe3d.png",
    "surface3d_dark.png",
    "surface3d_publication.png",
    "surface3d_high_view.png",
    "surface3d_perspective.png",
];

const FIGURE_WIDTH_IN: f32 = 6.4;
const FIGURE_HEIGHT_IN: f32 = 4.8;
const FIGURE_DPI: u32 = 100;

fn stable_theme(mut theme: Theme) -> Theme {
    theme.font_family = GOLDEN_FONT_FAMILY.to_string();
    theme
}

fn register_golden_font() -> ruviz::core::Result<()> {
    ruviz::render::register_font_bytes(GOLDEN_FONT_BYTES.to_vec())
}

fn output_path(directory: &Path, name: &str) -> std::path::PathBuf {
    directory.join(name)
}

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

fn base_camera() -> Camera3D {
    Camera3D::default()
        .azimuth_deg(-52.0)
        .elevation_deg(28.0)
        .axis_aspect(AxisAspect3D::Equal)
        .orthographic()
}

fn configure_surface(
    x: &[f64],
    y: &[f64],
    z: &[Vec<f64>],
    theme: Theme,
    camera: Camera3D,
) -> Surface3DBuilder {
    surface(x, y, z)
        .title("Damped radial surface")
        .xlabel("x")
        .ylabel("y")
        .zlabel("height")
        .cmap(ColorMap::viridis())
        .shading(SurfaceShading::Smooth)
        .sampling(SurfaceSampling::Full)
        .theme(stable_theme(theme))
        .camera(camera)
        .size(FIGURE_WIDTH_IN, FIGURE_HEIGHT_IN)
        .dpi(FIGURE_DPI)
}

pub fn generate_three_d_gallery(output_directory: &Path) -> ruviz::core::Result<()> {
    register_golden_font()?;
    std::fs::create_dir_all(output_directory)?;

    let first_t: Vec<f64> = (0..48)
        .map(|index| index as f64 * std::f64::consts::TAU / 47.0)
        .collect();
    let first_x: Vec<f64> = first_t.iter().map(|&t| t.cos()).collect();
    let first_y: Vec<f64> = first_t.iter().map(|&t| t.sin()).collect();
    let first_z: Vec<f64> = first_t
        .iter()
        .map(|&t| -1.0 + 2.0 * t / std::f64::consts::TAU)
        .collect();
    let second_x: Vec<f64> = first_x.iter().map(|&value| -value).collect();
    let second_y: Vec<f64> = first_y.iter().map(|&value| -value).collect();
    let second_z: Vec<f64> = first_z.iter().map(|&value| -value).collect();
    scatter3d(&first_x, &first_y, &first_z)
        .color(Color::from_rgb(0, 114, 178))
        .marker(MarkerStyle::Circle)
        .marker_size(7.0)
        .scatter3d(&second_x, &second_y, &second_z)
        .color(Color::from_rgb(230, 159, 0))
        .marker(MarkerStyle::Diamond)
        .marker_size(6.0)
        .title("Orthographic scatter")
        .xlabel("x")
        .ylabel("y")
        .zlabel("z")
        .theme(stable_theme(Theme::light()))
        .camera(base_camera())
        .size(FIGURE_WIDTH_IN, FIGURE_HEIGHT_IN)
        .dpi(FIGURE_DPI)
        .save(output_path(output_directory, "scatter3d.png"))?;

    let line_t: Vec<f64> = (0..181)
        .map(|index| index as f64 * 6.0 * std::f64::consts::PI / 180.0)
        .collect();
    let line_x: Vec<f64> = line_t.iter().map(|&t| t.cos()).collect();
    let line_y: Vec<f64> = line_t.iter().map(|&t| t.sin()).collect();
    let line_z: Vec<f64> = line_t
        .iter()
        .map(|&t| -1.0 + 2.0 * t / (6.0 * std::f64::consts::PI))
        .collect();
    line3d(&line_x, &line_y, &line_z)
        .color(Color::from_rgb(0, 114, 178))
        .line_width(2.0)
        .title("Perspective helix")
        .xlabel("x")
        .ylabel("y")
        .zlabel("z")
        .theme(stable_theme(Theme::light()))
        .camera(
            Camera3D::default()
                .azimuth_deg(-62.0)
                .elevation_deg(24.0)
                .axis_aspect(AxisAspect3D::Equal)
                .perspective_deg(38.0),
        )
        .size(FIGURE_WIDTH_IN, FIGURE_HEIGHT_IN)
        .dpi(FIGURE_DPI)
        .save(output_path(output_directory, "line3d.png"))?;

    let (surface_x, surface_y, surface_z) = surface_data(29);
    configure_surface(
        &surface_x,
        &surface_y,
        &surface_z,
        Theme::light(),
        base_camera(),
    )
    .save(output_path(output_directory, "surface3d.png"))?;

    wireframe(&surface_x, &surface_y, &surface_z)
        .title("Regular-grid wireframe")
        .xlabel("x")
        .ylabel("y")
        .zlabel("height")
        .color(Color::from_rgb(0, 114, 178))
        .line_width(0.8)
        .sampling(SurfaceSampling::MaxGrid {
            rows: 15,
            columns: 15,
        })
        .theme(stable_theme(Theme::light()))
        .camera(
            Camera3D::default()
                .azimuth_deg(-38.0)
                .elevation_deg(34.0)
                .axis_aspect(AxisAspect3D::Equal)
                .orthographic(),
        )
        .size(FIGURE_WIDTH_IN, FIGURE_HEIGHT_IN)
        .dpi(FIGURE_DPI)
        .save(output_path(output_directory, "wireframe3d.png"))?;

    configure_surface(
        &surface_x,
        &surface_y,
        &surface_z,
        Theme::dark(),
        base_camera(),
    )
    .title("Dark theme surface")
    .cmap(ColorMap::plasma())
    .save(output_path(output_directory, "surface3d_dark.png"))?;

    configure_surface(
        &surface_x,
        &surface_y,
        &surface_z,
        Theme::publication(),
        base_camera(),
    )
    .title("Publication theme surface")
    .cmap(ColorMap::coolwarm())
    .save(output_path(output_directory, "surface3d_publication.png"))?;

    configure_surface(
        &surface_x,
        &surface_y,
        &surface_z,
        Theme::light(),
        Camera3D::default()
            .azimuth_deg(32.0)
            .elevation_deg(58.0)
            .axis_aspect(AxisAspect3D::Equal)
            .orthographic(),
    )
    .title("High-elevation orthographic view")
    .save(output_path(output_directory, "surface3d_high_view.png"))?;

    configure_surface(
        &surface_x,
        &surface_y,
        &surface_z,
        Theme::light(),
        Camera3D::default()
            .azimuth_deg(-52.0)
            .elevation_deg(28.0)
            .axis_aspect(AxisAspect3D::Equal)
            .perspective_deg(40.0),
    )
    .title("Perspective surface view")
    .save(output_path(output_directory, "surface3d_perspective.png"))?;

    Ok(())
}

#[cfg(not(test))]
fn main() -> ruviz::core::Result<()> {
    let output = Path::new(env!("CARGO_MANIFEST_DIR")).join("generated/examples/3d");
    generate_three_d_gallery(&output)?;
    println!("Generated 3D gallery images in {}", output.display());
    Ok(())
}
