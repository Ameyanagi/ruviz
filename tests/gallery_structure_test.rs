#[allow(dead_code)]
#[path = "../scripts/generate_gallery.rs"]
mod generate_gallery;

use image::GenericImageView;
use std::{collections::BTreeSet, fs, path::Path};

fn managed_asset_names(directory: &Path) -> BTreeSet<String> {
    fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()))
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            matches!(
                path.extension().and_then(|value| value.to_str()),
                Some("png" | "gif")
            )
            .then(|| entry.file_name().to_string_lossy().into_owned())
        })
        .collect()
}

#[test]
fn gallery_layout_and_asset_sets_match_the_production_catalog() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let categories = generate_gallery::categories();
    let entries = generate_gallery::entries();
    let expected = generate_gallery::expected_assets_by_category(&entries);

    assert!(
        repository
            .join(generate_gallery::GALLERY_DOCS_ROOT)
            .is_dir()
    );
    assert!(
        repository
            .join(generate_gallery::GALLERY_DOCS_ROOT)
            .join("README.md")
            .is_file()
    );

    for category in &categories {
        let docs = repository
            .join(generate_gallery::GALLERY_DOCS_ROOT)
            .join(category.slug);
        let assets = repository
            .join(generate_gallery::GALLERY_ASSETS_ROOT)
            .join(category.slug);
        assert!(docs.join("README.md").is_file(), "{}", docs.display());
        assert!(assets.is_dir(), "{}", assets.display());
        let actual = managed_asset_names(&assets);
        let expected: BTreeSet<String> = expected
            .get(category.slug)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(str::to_string)
            .collect();
        assert_eq!(actual, expected, "asset set for {}", category.slug);
    }
}

#[test]
fn key_committed_gallery_assets_have_stable_dimensions() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (relative, dimensions) in [
        ("publication/subplots.png", (800, 600)),
        ("basic/line_plot.png", (1920, 1440)),
        ("advanced/radar_chart.png", (640, 480)),
    ] {
        let path = repository
            .join(generate_gallery::GALLERY_ASSETS_ROOT)
            .join(relative);
        assert_eq!(
            image::open(&path).unwrap().dimensions(),
            dimensions,
            "{}",
            path.display()
        );
    }
}

#[test]
fn gallery_links_are_valid() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut markdown_files = vec![
        repository
            .join(generate_gallery::GALLERY_DOCS_ROOT)
            .join("README.md"),
    ];
    markdown_files.extend(generate_gallery::categories().iter().map(|category| {
        repository
            .join(generate_gallery::GALLERY_DOCS_ROOT)
            .join(category.slug)
            .join("README.md")
    }));
    let link_pattern = regex::Regex::new(r"!?\[.*?\]\((.*?)\)").unwrap();

    for markdown_path in markdown_files {
        let content = fs::read_to_string(&markdown_path)
            .unwrap_or_else(|_| panic!("should read {}", markdown_path.display()));
        for capture in link_pattern.captures_iter(&content) {
            let target = capture.get(1).unwrap().as_str();
            if target.starts_with("http://") || target.starts_with("https://") {
                continue;
            }
            let target_path = target.split('#').next().unwrap_or(target);
            if target_path.is_empty() {
                continue;
            }
            let full_path = markdown_path.parent().unwrap().join(target_path);
            assert!(
                full_path.exists(),
                "{} points to missing gallery target {}",
                markdown_path.display(),
                target
            );
        }
    }
}
