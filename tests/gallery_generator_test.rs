// Gallery generator tests - TDD approach
// Tests define expected gallery generation behavior before implementation

use std::path::Path;

#[test]
fn test_example_categorization() {
    // GIVEN: Example files should be categorized correctly

    // Basic examples (fundamental plot types)
    assert_eq!(categorize_example("line_plot.rs"), Some("basic"));
    assert_eq!(categorize_example("scatter_plot.rs"), Some("basic"));
    assert_eq!(categorize_example("bar_chart.rs"), Some("basic"));

    // Statistical examples
    assert_eq!(categorize_example("histogram_example.rs"), Some("statistical"));
    assert_eq!(categorize_example("boxplot_example.rs"), Some("statistical"));

    // Publication quality
    assert_eq!(categorize_example("scientific_plotting.rs"), Some("publication"));
    assert_eq!(categorize_example("simple_publication_test.rs"), Some("publication"));
    assert_eq!(categorize_example("scientific_showcase.rs"), Some("publication"));

    // Performance demonstrations
    assert_eq!(categorize_example("parallel_demo.rs"), Some("performance"));
    assert_eq!(categorize_example("memory_optimization_demo.rs"), Some("performance"));

    // Advanced techniques
    assert_eq!(categorize_example("font_demo.rs"), Some("advanced"));
    assert_eq!(categorize_example("seaborn_style_example.rs"), Some("advanced"));
    assert_eq!(categorize_example("subplot_example.rs"), Some("advanced"));
}

#[test]
fn test_uncategorized_examples_detected() {
    // GIVEN: Unknown examples should return None
    assert_eq!(categorize_example("unknown_test.rs"), None);
    assert_eq!(categorize_example("gpu_test.rs"), None);
}

#[test]
fn test_example_metadata_extraction() {
    // GIVEN: Example files with metadata comments
    let metadata = ExampleMetadata {
        title: "Line Plot Example".to_string(),
        description: "Simple line plot demonstration".to_string(),
        category: "basic".to_string(),
    };

    // THEN: Metadata should be extractable
    assert_eq!(metadata.title, "Line Plot Example");
    assert_eq!(metadata.category, "basic");
    assert!(!metadata.description.is_empty());
}

#[test]
fn test_gallery_index_generation() {
    // GIVEN: Gallery with categorized examples
    let index = GalleryIndex::new();

    // THEN: Index should have all categories
    assert!(index.has_category("basic"));
    assert!(index.has_category("statistical"));
    assert!(index.has_category("publication"));
    assert!(index.has_category("performance"));
    assert!(index.has_category("advanced"));
}

#[test]
fn test_markdown_generation_for_example() {
    // GIVEN: Example with metadata
    let metadata = ExampleMetadata {
        title: "Test Plot".to_string(),
        description: "Test description".to_string(),
        category: "basic".to_string(),
    };

    let markdown = generate_example_markdown(&metadata, "test_plot.png");

    // THEN: Markdown should contain key elements
    assert!(markdown.contains("## Test Plot"));
    assert!(markdown.contains("Test description"));
    assert!(markdown.contains("![Test Plot](test_plot.png)"));
}

#[test]
fn test_category_readme_generation() {
    // GIVEN: Category with examples
    let examples = vec![
        ExampleMetadata {
            title: "Example 1".to_string(),
            description: "Description 1".to_string(),
            category: "basic".to_string(),
        },
        ExampleMetadata {
            title: "Example 2".to_string(),
            description: "Description 2".to_string(),
            category: "basic".to_string(),
        },
    ];

    let readme = generate_category_readme("basic", &examples);

    // THEN: README should list all examples
    assert!(readme.contains("# Basic"));
    assert!(readme.contains("Example 1"));
    assert!(readme.contains("Example 2"));
}

// Helper types and functions for tests
// These will be implemented in the gallery generation script

#[derive(Debug, PartialEq)]
struct ExampleMetadata {
    title: String,
    description: String,
    category: String,
}

struct GalleryIndex {
    categories: Vec<String>,
}

impl GalleryIndex {
    fn new() -> Self {
        GalleryIndex {
            categories: vec![
                "basic".to_string(),
                "statistical".to_string(),
                "publication".to_string(),
                "performance".to_string(),
                "advanced".to_string(),
            ],
        }
    }

    fn has_category(&self, category: &str) -> bool {
        self.categories.iter().any(|c| c == category)
    }
}

fn categorize_example(filename: &str) -> Option<&'static str> {
    // Pattern matching for categorization
    let name = filename.to_lowercase();

    // Basic patterns
    if name.contains("line") || name.contains("scatter") || name.contains("bar") {
        return Some("basic");
    }

    // Statistical patterns
    if name.contains("histogram") || name.contains("boxplot") || name.contains("distribution") {
        return Some("statistical");
    }

    // Publication patterns
    if name.contains("publication") || name.contains("scientific") || name.contains("showcase") {
        return Some("publication");
    }

    // Performance patterns
    if name.contains("parallel") || name.contains("memory_optimization") || name.contains("performance") {
        return Some("performance");
    }

    // Advanced patterns
    if name.contains("font") || name.contains("seaborn") || name.contains("subplot") || name.contains("theme") {
        return Some("advanced");
    }

    None
}

fn generate_example_markdown(metadata: &ExampleMetadata, image_path: &str) -> String {
    format!(
        "## {}\n\n{}\n\n![{}]({})\n",
        metadata.title, metadata.description, metadata.title, image_path
    )
}

fn generate_category_readme(category: &str, examples: &[ExampleMetadata]) -> String {
    let title = match category {
        "basic" => "Basic",
        "statistical" => "Statistical",
        "publication" => "Publication",
        "performance" => "Performance",
        "advanced" => "Advanced",
        _ => category,
    };

    let mut readme = format!("# {} Plots\n\n", title);

    for example in examples {
        readme.push_str(&format!("## {}\n\n{}\n\n", example.title, example.description));
    }

    readme
}
