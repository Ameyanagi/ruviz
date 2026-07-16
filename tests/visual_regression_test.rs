// Deterministic visual regression tests using exact pixel comparison.
// Run with: cargo test --test visual_regression_test -- --ignored
// The shared generator registers repository-owned font bytes and selects that
// family for every plot and figure-level text surface.

#[path = "../examples/generate_golden_images.rs"]
mod golden_generator;
#[path = "common/visual.rs"]
mod visual;

use image::RgbaImage;
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

    let error = visual::assert_exact_pixels(&actual_path, &golden_path, &artifact_directory)
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
        visual::assert_exact_pixels(
            &actual_directory.join(fixture),
            &committed_directory.join(fixture),
            &artifact_directory,
        )?;
    }

    Ok(())
}
