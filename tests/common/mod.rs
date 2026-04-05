//! Common test utilities for ruviz tests.
//!
//! Transient test artifacts live under `generated/tests/`, while committed
//! golden fixtures live under `tests/fixtures/golden/`.

use std::path::{Path, PathBuf};

fn ensure_dir(path: PathBuf, label: &str) -> PathBuf {
    if !path.exists() {
        std::fs::create_dir_all(&path).unwrap_or_else(|err| {
            panic!(
                "Failed to create {label} directory {}: {err}",
                path.display()
            )
        });
    }

    path
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[allow(dead_code)]
pub fn generated_tests_root() -> PathBuf {
    ensure_dir(
        project_root().join("generated").join("tests"),
        "generated tests root",
    )
}

#[allow(dead_code)]
pub fn golden_fixture_path(filename: &str) -> PathBuf {
    project_root()
        .join("tests")
        .join("fixtures")
        .join("golden")
        .join(filename)
}

/// Get the path for a test output file.
///
/// Creates the output directory if it doesn't exist.
///
/// # Arguments
/// * `filename` - The name of the output file (e.g., "line_plot.png")
///
/// # Returns
/// Full path to the output file in `generated/tests/render/`
///
/// # Example
/// ```ignore
/// let path = test_output_path("my_test_plot.png");
/// plot.save(&path)?;
/// ```
pub fn test_output_path(filename: &str) -> PathBuf {
    let output_dir = ensure_dir(
        generated_tests_root().join("render"),
        "generated test render output",
    );
    output_dir.join(filename)
}

/// Get the path for a test output file in a subdirectory.
///
/// Creates the subdirectory if it doesn't exist.
///
/// # Arguments
/// * `subdir` - Subdirectory name (e.g., "regression", "visual")
/// * `filename` - The name of the output file
///
/// # Returns
/// Full path to the output file in `generated/tests/render/<subdir>/`
pub fn test_output_path_in(subdir: &str, filename: &str) -> PathBuf {
    let output_dir = ensure_dir(
        generated_tests_root().join("render").join(subdir),
        "generated test render output subdirectory",
    );
    output_dir.join(filename)
}

#[allow(dead_code)]
pub fn visual_output_path(filename: &str) -> PathBuf {
    ensure_dir(
        generated_tests_root().join("visual"),
        "generated visual output",
    )
    .join(filename)
}

#[allow(dead_code)]
pub fn visual_diff_output_path(filename: &str) -> PathBuf {
    ensure_dir(
        generated_tests_root().join("visual-diff"),
        "generated visual diff output",
    )
    .join(filename)
}

#[allow(dead_code)]
pub fn export_output_path(subdir: &str, filename: &str) -> PathBuf {
    ensure_dir(
        generated_tests_root().join("export").join(subdir),
        "generated export output",
    )
    .join(filename)
}

/// Assert that a file exists and is non-empty.
#[allow(dead_code)]
pub fn assert_file_non_empty(path: impl AsRef<Path>) {
    let path = path.as_ref();
    assert!(path.exists(), "Expected file to exist: {}", path.display());
    let metadata = std::fs::metadata(path)
        .unwrap_or_else(|e| panic!("Failed to stat {}: {}", path.display(), e));
    assert!(
        metadata.len() > 0,
        "Expected non-empty file, got {} bytes: {}",
        metadata.len(),
        path.display()
    );
}

/// Assert that a PNG exists, decodes, and contains non-background pixels.
/// Optionally verifies exact dimensions.
#[allow(dead_code)]
pub fn assert_png_rendered(path: impl AsRef<Path>, expected_dims: Option<(u32, u32)>) {
    let path = path.as_ref();
    assert_file_non_empty(path);

    let img = image::open(path)
        .unwrap_or_else(|e| panic!("Expected valid PNG at {}: {}", path.display(), e))
        .to_rgba8();

    if let Some((expected_w, expected_h)) = expected_dims {
        assert_eq!(
            img.width(),
            expected_w,
            "Unexpected PNG width for {}",
            path.display()
        );
        assert_eq!(
            img.height(),
            expected_h,
            "Unexpected PNG height for {}",
            path.display()
        );
    }

    let bg = *img.get_pixel(0, 0);
    let has_non_bg = img.pixels().any(|p| *p != bg);
    assert!(
        has_non_bg,
        "Expected rendered non-background pixels in {}",
        path.display()
    );
}

/// Assert PNG dimensions with tolerance while also ensuring decode/content checks.
#[allow(dead_code)]
pub fn assert_png_dimensions_with_tolerance(
    path: impl AsRef<Path>,
    expected_dims: (u32, u32),
    tolerance: u32,
) {
    let path = path.as_ref();
    assert_file_non_empty(path);

    let img = image::open(path)
        .unwrap_or_else(|e| panic!("Expected valid PNG at {}: {}", path.display(), e))
        .to_rgba8();

    let (expected_w, expected_h) = expected_dims;
    let width_diff = img.width().abs_diff(expected_w);
    let height_diff = img.height().abs_diff(expected_h);

    assert!(
        width_diff <= tolerance,
        "Width mismatch for {}: got {}, expected {}, tolerance {}",
        path.display(),
        img.width(),
        expected_w,
        tolerance
    );
    assert!(
        height_diff <= tolerance,
        "Height mismatch for {}: got {}, expected {}, tolerance {}",
        path.display(),
        img.height(),
        expected_h,
        tolerance
    );

    let bg = *img.get_pixel(0, 0);
    let has_non_bg = img.pixels().any(|p| *p != bg);
    assert!(
        has_non_bg,
        "Expected rendered non-background pixels in {}",
        path.display()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_path_creates_directory() {
        let path = test_output_path("test_file.png");
        assert!(path.parent().unwrap().exists());
        assert!(path.to_string_lossy().contains("generated/tests/render"));
    }

    #[test]
    fn test_output_path_in_subdir() {
        let path = test_output_path_in("visual", "test_file.png");
        assert!(path.parent().unwrap().exists());
        assert!(
            path.to_string_lossy()
                .contains("generated/tests/render/visual")
        );
    }
}
