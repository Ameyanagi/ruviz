//! Common utilities for ruviz examples.
//!
//! The repo keeps transient example artifacts under `generated/examples/` and
//! committed release-facing media under `docs/assets/`.

use std::path::PathBuf;

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
pub fn generated_root() -> PathBuf {
    ensure_dir(project_root().join("generated"), "generated root")
}

#[allow(dead_code)]
pub fn docs_assets_root() -> PathBuf {
    ensure_dir(
        project_root().join("docs").join("assets"),
        "docs assets root",
    )
}

/// Get the path for an example output file.
///
/// Creates the output directory if it doesn't exist.
///
/// # Arguments
/// * `filename` - The name of the output file (e.g., "line_plot.png")
///
/// # Returns
/// Full path to the output file in `generated/examples/`
///
/// # Example
/// ```ignore
/// let path = example_output_path("my_example_plot.png");
/// plot.save(&path)?;
/// ```
#[allow(dead_code)]
pub fn example_output_path(filename: &str) -> PathBuf {
    let output_dir = ensure_dir(
        generated_root().join("examples"),
        "generated examples output",
    );
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
/// Full path to the output file in `generated/examples/<subdir>/`
#[allow(dead_code)]
pub fn example_output_path_in(subdir: &str, filename: &str) -> PathBuf {
    let output_dir = ensure_dir(
        generated_root().join("examples").join(subdir),
        "generated example output subdirectory",
    );
    output_dir.join(filename)
}

/// Get the path for a committed README-facing asset under `docs/assets/readme/`.
#[allow(dead_code)]
pub fn readme_asset_path(filename: &str) -> PathBuf {
    ensure_dir(
        docs_assets_root().join("readme"),
        "docs readme asset output",
    )
    .join(filename)
}

/// Get the path for a committed rustdoc-facing asset under `docs/assets/rustdoc/`.
#[allow(dead_code)]
pub fn rustdoc_asset_path(filename: &str) -> PathBuf {
    ensure_dir(
        docs_assets_root().join("rustdoc"),
        "docs rustdoc asset output",
    )
    .join(filename)
}

/// Get the path for a committed Rust gallery asset under
/// `docs/assets/gallery/rust/<category>/`.
#[allow(dead_code)]
pub fn rust_gallery_asset_path(category: &str, filename: &str) -> PathBuf {
    ensure_dir(
        docs_assets_root()
            .join("gallery")
            .join("rust")
            .join(category),
        "docs Rust gallery asset output",
    )
    .join(filename)
}
