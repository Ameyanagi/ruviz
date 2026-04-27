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
        "animation",
        "internationalization",
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
        "animation",
        "internationalization",
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
            .unwrap_or_else(|_| panic!("Should read {} index", category));

        assert!(
            !content.is_empty(),
            "Category '{}' index should not be empty",
            category
        );
    }
}

#[test]
fn test_gallery_has_images() {
    // GIVEN: Gallery categories should have committed asset images
    let categories = vec![
        "basic",
        "statistical",
        "publication",
        "performance",
        "advanced",
        "animation",
        "internationalization",
    ];

    // THEN: At least one category has PNG files
    let mut total_images = 0;

    for category in categories {
        let category_path = Path::new("docs/assets/gallery/rust").join(category);

        if let Ok(entries) = std::fs::read_dir(&category_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                let extension = path.extension().and_then(|s| s.to_str());
                if extension == Some("png") || extension == Some("gif") {
                    total_images += 1;
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
    // GIVEN: Gallery asset categories should exist under docs/assets/gallery/rust
    let categories = vec![
        "basic",
        "statistical",
        "publication",
        "performance",
        "advanced",
        "animation",
        "internationalization",
    ];

    // THEN: Each category has a committed asset directory
    for category in categories {
        let category_path = Path::new("docs/assets/gallery/rust").join(category);
        assert!(
            category_path.exists(),
            "Gallery asset category '{}' should exist",
            category
        );
    }
}

#[test]
fn test_gallery_links_are_valid() {
    let markdown_files = [
        Path::new("docs/gallery/README.md").to_path_buf(),
        Path::new("docs/gallery/basic/README.md").to_path_buf(),
        Path::new("docs/gallery/statistical/README.md").to_path_buf(),
        Path::new("docs/gallery/publication/README.md").to_path_buf(),
        Path::new("docs/gallery/performance/README.md").to_path_buf(),
        Path::new("docs/gallery/advanced/README.md").to_path_buf(),
        Path::new("docs/gallery/animation/README.md").to_path_buf(),
        Path::new("docs/gallery/internationalization/README.md").to_path_buf(),
    ];
    let link_pattern = regex::Regex::new(r"!?\[.*?\]\((.*?)\)").unwrap();

    for markdown_path in markdown_files {
        let content = std::fs::read_to_string(&markdown_path)
            .unwrap_or_else(|_| panic!("should read {}", markdown_path.display()));
        for cap in link_pattern.captures_iter(&content) {
            let target = cap.get(1).unwrap().as_str();
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

#[test]
fn test_basic_category_has_expected_examples() {
    // GIVEN: Basic category should have fundamental plot types
    let basic_path = Path::new("docs/assets/gallery/rust/basic");

    // THEN: Check for expected basic plot types
    let expected_patterns = vec!["line", "scatter", "bar", "heatmap"];

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

            assert!(has_pattern, "Basic category missing '{pattern}' example");
        }
    }
}
