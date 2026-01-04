//! Common test utilities for ruviz tests
//!
//! Provides helper functions for test output paths and other shared functionality.

use std::path::PathBuf;

/// Get the path for a test output file.
///
/// Creates the output directory if it doesn't exist.
///
/// # Arguments
/// * `filename` - The name of the output file (e.g., "line_plot.png")
///
/// # Returns
/// Full path to the output file in `tests/output/`
///
/// # Example
/// ```ignore
/// let path = test_output_path("my_test_plot.png");
/// plot.save(&path)?;
/// ```
pub fn test_output_path(filename: &str) -> PathBuf {
    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("output");

    // Create directory if it doesn't exist
    if !output_dir.exists() {
        std::fs::create_dir_all(&output_dir).expect("Failed to create tests/output directory");
    }

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
/// Full path to the output file in `tests/output/<subdir>/`
pub fn test_output_path_in(subdir: &str, filename: &str) -> PathBuf {
    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("output")
        .join(subdir);

    // Create directory if it doesn't exist
    if !output_dir.exists() {
        std::fs::create_dir_all(&output_dir).expect("Failed to create test output subdirectory");
    }

    output_dir.join(filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_path_creates_directory() {
        let path = test_output_path("test_file.png");
        assert!(path.parent().unwrap().exists());
        assert!(path.to_string_lossy().contains("tests/output"));
    }

    #[test]
    fn test_output_path_in_subdir() {
        let path = test_output_path_in("visual", "test_file.png");
        assert!(path.parent().unwrap().exists());
        assert!(path.to_string_lossy().contains("tests/output/visual"));
    }
}
