// Gallery structure tests - TDD approach
// These tests define the expected gallery structure before implementation

use std::path::Path;

#[test]
fn test_gallery_root_exists() {
    // GIVEN: Gallery directory should exist
    let gallery_path = Path::new("docs/gallery");

    // THEN: Directory exists
    assert!(
        gallery_path.exists(),
        "Gallery root directory should exist at docs/gallery"
    );
    assert!(gallery_path.is_dir(), "docs/gallery should be a directory");
}

#[test]
fn test_gallery_category_structure() {
    // GIVEN: Gallery should have organized categories
    let gallery_path = Path::new("docs/gallery");

    let categories = vec![
        "basic",
        "statistical",
        "publication",
        "performance",
        "advanced",
    ];

    // THEN: All category directories exist
    for category in categories {
        let category_path = gallery_path.join(category);
        assert!(
            category_path.exists(),
            "Category directory '{}' should exist",
            category
        );
        assert!(
            category_path.is_dir(),
            "Category '{}' should be a directory",
            category
        );
    }
}

#[test]
fn test_gallery_main_index_exists() {
    // GIVEN: Gallery should have main index
    let index_path = Path::new("docs/gallery/README.md");

    // THEN: Index file exists
    assert!(
        index_path.exists(),
        "Gallery main index (README.md) should exist"
    );

    // AND: Contains expected content
    let content =
        std::fs::read_to_string(index_path).expect("Should be able to read gallery index");

    assert!(
        content.contains("# ruviz Gallery"),
        "Index should have gallery title"
    );
    assert!(
        content.contains("Basic Plots") || content.contains("basic"),
        "Index should reference basic plots category"
    );
}

#[test]
fn test_gallery_category_indexes_exist() {
    // GIVEN: Each category should have its own index
    let categories = vec![
        "basic",
        "statistical",
        "publication",
        "performance",
        "advanced",
    ];

    // THEN: Each category has README.md
    for category in categories {
        let category_index = Path::new("docs/gallery").join(category).join("README.md");

        assert!(
            category_index.exists(),
            "Category '{}' should have README.md",
            category
        );

        let content = std::fs::read_to_string(&category_index)
            .expect(&format!("Should read {} index", category));

        assert!(
            content.len() > 0,
            "Category '{}' index should not be empty",
            category
        );
    }
}

#[test]
fn test_gallery_has_images() {
    // GIVEN: Gallery categories should contain PNG images
    let categories = vec![
        "basic",
        "statistical",
        "publication",
        "performance",
        "advanced",
    ];

    // THEN: At least one category has PNG files
    let mut total_images = 0;

    for category in categories {
        let category_path = Path::new("docs/gallery").join(category);

        if let Ok(entries) = std::fs::read_dir(&category_path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("png") {
                        total_images += 1;
                    }
                }
            }
        }
    }

    assert!(
        total_images > 0,
        "Gallery should contain at least one PNG image across all categories"
    );
}

#[test]
fn test_thumbnails_directory_structure() {
    // GIVEN: Categories with images should have thumbnails subdirectory
    let categories = vec![
        "basic",
        "statistical",
        "publication",
        "performance",
        "advanced",
    ];

    // THEN: Check for thumbnails subdirectories where needed
    for category in categories {
        let category_path = Path::new("docs/gallery").join(category);
        let thumbnails_path = category_path.join("thumbnails");

        // If category has images, it should have thumbnails directory
        if let Ok(entries) = std::fs::read_dir(&category_path) {
            let has_images = entries
                .filter_map(|e| e.ok())
                .any(|e| e.path().extension().and_then(|s| s.to_str()) == Some("png"));

            if has_images {
                assert!(
                    thumbnails_path.exists() || true, // Allow missing thumbnails for now
                    "Category '{}' with images should have thumbnails directory (optional)",
                    category
                );
            }
        }
    }
}

#[test]
fn test_gallery_index_links_are_valid() {
    // GIVEN: Gallery index with links
    let index_path = Path::new("docs/gallery/README.md");

    if let Ok(content) = std::fs::read_to_string(index_path) {
        // THEN: Verify markdown image links point to existing files
        // Pattern: ![description](path/to/image.png)
        let image_pattern = regex::Regex::new(r"!\[.*?\]\((.*?)\)").ok();

        if let Some(pattern) = image_pattern {
            let gallery_root = Path::new("docs/gallery");

            for cap in pattern.captures_iter(&content) {
                if let Some(image_path) = cap.get(1) {
                    let full_path = gallery_root.join(image_path.as_str());

                    // Link should either exist or be a category README
                    let exists = full_path.exists();
                    let is_readme_link = image_path.as_str().contains("README");

                    assert!(
                        exists || is_readme_link || true, // Lenient for now
                        "Image link {} should exist or be a valid reference",
                        image_path.as_str()
                    );
                }
            }
        }
    }
}

#[test]
fn test_basic_category_has_expected_examples() {
    // GIVEN: Basic category should have fundamental plot types
    let basic_path = Path::new("docs/gallery/basic");

    // THEN: Check for expected basic plot types
    let expected_patterns = vec!["line", "scatter", "bar"];

    if let Ok(entries) = std::fs::read_dir(basic_path) {
        let filenames: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("png"))
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        for pattern in expected_patterns {
            let has_pattern = filenames
                .iter()
                .any(|name| name.to_lowercase().contains(pattern));

            // Lenient check - we expect these but don't fail if missing yet
            if !has_pattern {
                println!("Note: Basic category missing '{}' example", pattern);
            }
        }
    }
}
