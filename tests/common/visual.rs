use image::{Rgba, RgbaImage};
use std::{fs, path::Path};

fn write_diff_image(actual: &RgbaImage, golden: &RgbaImage, path: &Path) -> image::ImageResult<()> {
    let mut diff = RgbaImage::new(actual.width(), actual.height());
    for (x, y, pixel) in diff.enumerate_pixels_mut() {
        let actual_pixel = actual.get_pixel(x, y);
        let golden_pixel = golden.get_pixel(x, y);
        *pixel = Rgba([
            actual_pixel[0].abs_diff(golden_pixel[0]).saturating_mul(4),
            actual_pixel[1].abs_diff(golden_pixel[1]).saturating_mul(4),
            actual_pixel[2].abs_diff(golden_pixel[2]).saturating_mul(4),
            255,
        ]);
    }
    diff.save(path)
}

pub fn assert_exact_pixels(
    actual_path: &Path,
    golden_path: &Path,
    artifact_directory: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let actual = image::open(actual_path)?.to_rgba8();
    let golden = image::open(golden_path)?.to_rgba8();
    let fixture_name = golden_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("visual.png");
    let actual_artifact = artifact_directory.join(format!("actual_{fixture_name}"));

    if actual.dimensions() != golden.dimensions() {
        fs::create_dir_all(artifact_directory)?;
        actual.save(&actual_artifact)?;
        return Err(format!(
            "{} dimensions differ: actual {:?}, golden {:?}. Actual: {}",
            fixture_name,
            actual.dimensions(),
            golden.dimensions(),
            actual_artifact.display()
        )
        .into());
    }

    let changed_pixels = actual
        .pixels()
        .zip(golden.pixels())
        .filter(|(actual_pixel, golden_pixel)| actual_pixel != golden_pixel)
        .count();
    if changed_pixels == 0 {
        return Ok(());
    }

    fs::create_dir_all(artifact_directory)?;
    let diff_artifact = artifact_directory.join(format!("diff_{fixture_name}"));
    actual.save(&actual_artifact)?;
    write_diff_image(&actual, &golden, &diff_artifact)?;

    Err(format!(
        "{} changed {} of {} pixels; deterministic golden comparison requires an exact match. Actual: {}. Diff: {}",
        fixture_name,
        changed_pixels,
        actual.width() as u64 * actual.height() as u64,
        actual_artifact.display(),
        diff_artifact.display()
    )
    .into())
}
