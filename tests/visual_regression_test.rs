// Deterministic visual regression tests using exact pixel comparison.
// Run with: cargo test --test visual_regression_test -- --ignored
// The shared generator registers repository-owned font bytes and selects that
// family for every plot and figure-level text surface.

#[path = "../examples/generate_golden_images.rs"]
mod golden_generator;

use image::{Rgba, RgbaImage};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

struct WorkingDirectory {
    original: PathBuf,
}

impl WorkingDirectory {
    fn enter(path: &Path) -> std::io::Result<Self> {
        let original = std::env::current_dir()?;
        std::env::set_current_dir(path)?;
        Ok(Self { original })
    }
}

impl Drop for WorkingDirectory {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.original)
            .expect("failed to restore visual test working directory");
    }
}

fn committed_fixture_names(directory: &Path) -> std::io::Result<BTreeSet<String>> {
    fs::read_dir(directory)?
        .filter_map(|entry| match entry {
            Ok(entry) if entry.path().extension().is_some_and(|ext| ext == "png") => {
                Some(Ok(entry.file_name().to_string_lossy().into_owned()))
            }
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

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

fn assert_exact_pixels(
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

#[test]
fn dimension_mismatch_preserves_the_actual_artifact() {
    let temp = tempfile::tempdir().expect("temporary visual test directory");
    let actual_path = temp.path().join("actual.png");
    let golden_path = temp.path().join("fixture.png");
    let artifact_directory = temp.path().join("artifacts");
    RgbaImage::new(2, 3)
        .save(&actual_path)
        .expect("save mismatched actual image");
    RgbaImage::new(3, 2)
        .save(&golden_path)
        .expect("save mismatched golden image");

    let error = assert_exact_pixels(&actual_path, &golden_path, &artifact_directory)
        .expect_err("dimension mismatch should fail");
    let actual_artifact = artifact_directory.join("actual_fixture.png");
    assert!(actual_artifact.is_file());
    assert!(
        error
            .to_string()
            .contains(&actual_artifact.display().to_string())
    );
}

#[test]
fn bundled_golden_font_registration_and_selection_are_verifiable() {
    golden_generator::register_golden_font()
        .expect("the exact golden font bytes should be registered and selected");
}

#[test]
#[ignore] // Regenerates and compares the committed golden image set.
fn deterministic_golden_images_match_exactly() -> Result<(), Box<dyn std::error::Error>> {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let committed_directory = repository.join("tests/fixtures/golden");
    let expected_names: BTreeSet<String> = golden_generator::GOLDEN_FIXTURES
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    let committed_names = committed_fixture_names(&committed_directory)?;
    if committed_names != expected_names {
        return Err(format!(
            "committed golden fixtures do not match GOLDEN_FIXTURES: expected {expected_names:?}, found {committed_names:?}"
        )
        .into());
    }

    let generated_root = tempfile::tempdir()?;
    {
        let _working_directory = WorkingDirectory::enter(generated_root.path())?;
        golden_generator::generate_golden_images()?;
    }

    let actual_directory = generated_root.path().join("tests/fixtures/golden");
    let artifact_directory = repository.join("generated/tests/render");
    for fixture in golden_generator::GOLDEN_FIXTURES {
        println!("Comparing {fixture}");
        assert_exact_pixels(
            &actual_directory.join(fixture),
            &committed_directory.join(fixture),
            &artifact_directory,
        )?;
    }

    Ok(())
}
