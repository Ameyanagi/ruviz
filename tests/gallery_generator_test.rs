#[allow(dead_code)]
#[path = "../scripts/generate_gallery.rs"]
mod generate_gallery;

use generate_gallery::AssetSource;
use std::{collections::BTreeSet, fs, path::Path};

fn repository() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn production_catalog_is_valid() {
    let categories = generate_gallery::categories();
    let entries = generate_gallery::entries();
    generate_gallery::validate_catalog(repository(), &categories, &entries)
        .unwrap_or_else(|errors| panic!("catalog validation failed:\n{}", errors.join("\n")));
}

#[test]
fn production_categories_and_entry_mappings_are_exact() {
    let categories = generate_gallery::categories();
    let entries = generate_gallery::entries();
    let actual: BTreeSet<_> = categories.iter().map(|category| category.slug).collect();
    let expected = BTreeSet::from([
        "advanced",
        "animation",
        "basic",
        "internationalization",
        "performance",
        "publication",
        "statistical",
    ]);
    assert_eq!(actual, expected);
    assert!(
        entries.iter().all(|entry| actual.contains(entry.category)),
        "every production entry must map to a known category"
    );
}

#[test]
fn production_catalog_has_no_duplicate_destinations() {
    let entries = generate_gallery::entries();
    let destinations: BTreeSet<_> = entries
        .iter()
        .map(|entry| (entry.category, entry.asset_name))
        .collect();
    assert_eq!(destinations.len(), entries.len());
}

#[test]
fn generated_markdown_matches_production_catalog_exactly() {
    let categories = generate_gallery::categories();
    let entries = generate_gallery::entries();
    for category in &categories {
        let path = repository()
            .join(generate_gallery::GALLERY_DOCS_ROOT)
            .join(category.slug)
            .join("README.md");
        let expected = generate_gallery::render_category_page(
            category,
            &generate_gallery::entries_for_category(&entries, category.slug),
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            expected,
            "{}",
            path.display()
        );
    }

    let index = repository()
        .join(generate_gallery::GALLERY_DOCS_ROOT)
        .join("README.md");
    assert_eq!(
        fs::read_to_string(&index).unwrap(),
        generate_gallery::render_gallery_index(&categories, &entries),
        "{}",
        index.display()
    );
}

#[test]
fn every_copy_backed_asset_is_byte_identical() {
    for entry in generate_gallery::entries() {
        if !matches!(entry.source, AssetSource::Copy { .. }) {
            continue;
        }
        let source = generate_gallery::copy_source_path(repository(), &entry).unwrap();
        let destination = generate_gallery::gallery_asset_path(repository(), &entry);
        assert_eq!(
            fs::read(&source).unwrap(),
            fs::read(&destination).unwrap(),
            "{} is stale relative to {}",
            destination.display(),
            source.display()
        );
    }
}

#[test]
fn gallery_has_no_unexpected_managed_assets() {
    let categories = generate_gallery::categories();
    let entries = generate_gallery::entries();
    let unexpected =
        generate_gallery::unexpected_managed_assets(repository(), &categories, &entries).unwrap();
    assert!(unexpected.is_empty(), "unexpected assets: {unexpected:?}");
}

#[test]
fn source_and_guide_targets_exist() {
    let categories = generate_gallery::categories();
    let entries = generate_gallery::entries();
    let errors = generate_gallery::validate_catalog(repository(), &categories, &entries)
        .err()
        .unwrap_or_default();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}
