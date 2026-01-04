//! Common utilities for ruviz examples
//!
//! Provides helper functions for example output paths and other shared functionality.

use std::path::PathBuf;

/// Get the path for an example output file.
///
/// Creates the output directory if it doesn't exist.
///
/// # Arguments
/// * `filename` - The name of the output file (e.g., "line_plot.png")
///
/// # Returns
/// Full path to the output file in `examples/output/`
///
/// # Example
/// ```ignore
/// let path = example_output_path("my_example_plot.png");
/// plot.save(&path)?;
/// ```
pub fn example_output_path(filename: &str) -> PathBuf {
    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("output");

    // Create directory if it doesn't exist
    if !output_dir.exists() {
        std::fs::create_dir_all(&output_dir).expect("Failed to create examples/output directory");
    }

    output_dir.join(filename)
}

/// Get the path for an example output file in a subdirectory.
///
/// Creates the subdirectory if it doesn't exist.
///
/// # Arguments
/// * `subdir` - Subdirectory name (e.g., "performance", "basic")
/// * `filename` - The name of the output file
///
/// # Returns
/// Full path to the output file in `examples/output/<subdir>/`
pub fn example_output_path_in(subdir: &str, filename: &str) -> PathBuf {
    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("output")
        .join(subdir);

    // Create directory if it doesn't exist
    if !output_dir.exists() {
        std::fs::create_dir_all(&output_dir).expect("Failed to create example output subdirectory");
    }

    output_dir.join(filename)
}
