#!/usr/bin/env rust-script
//! Gallery generation script
//! Categorizes examples, renders to PNG, generates thumbnails and markdown indexes
//!
//! Usage: cargo run --bin generate_gallery

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎨 ruviz Gallery Generator");
    println!("==========================\n");

    // Step 1: Discover all example files
    let examples = discover_examples("examples")?;
    println!("📂 Found {} example files", examples.len());

    // Step 2: Categorize examples
    let categorized = categorize_examples(&examples);
    print_categorization_summary(&categorized);

    // Step 3: Generate images by running examples
    println!("\n📸 Generating images from examples...");
    generate_images(&categorized)?;

    // Step 4: Update category README files
    println!("\n📝 Updating category README files...");
    update_category_readmes(&categorized)?;

    // Step 5: Update main gallery index
    println!("\n📋 Updating main gallery index...");
    update_main_index(&categorized)?;

    println!("\n✅ Gallery generation complete!");
    println!("   View at: docs/gallery/README.md");

    Ok(())
}

fn discover_examples(dir: &str) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut examples = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
            // Skip test files and internal utilities
            let filename = path.file_name().unwrap().to_str().unwrap();
            if !filename.contains("test")
                && !filename.contains("debug")
                && !filename.starts_with("generate_")
            {
                examples.push(path);
            }
        }
    }

    examples.sort();
    Ok(examples)
}

fn categorize_examples(examples: &[PathBuf]) -> CategoryMap {
    let mut map = CategoryMap::new();

    for example in examples {
        if let Some(filename) = example.file_name().and_then(|s| s.to_str()) {
            if let Some(category) = categorize_filename(filename) {
                map.add(category.to_string(), example.clone());
            }
        }
    }

    map
}

fn categorize_filename(filename: &str) -> Option<&'static str> {
    let name = filename.to_lowercase();

    // Basic patterns
    if name.contains("basic") || name.contains("simple") || name.contains("first") {
        return Some("basic");
    }

    // Statistical patterns
    if name.contains("histogram")
        || name.contains("boxplot")
        || name.contains("distribution")
        || name.contains("statistical")
    {
        return Some("statistical");
    }

    // Publication patterns
    if name.contains("publication") || name.contains("scientific") || name.contains("showcase") {
        return Some("publication");
    }

    // Performance patterns
    if name.contains("parallel")
        || name.contains("memory_optimization")
        || name.contains("performance")
        || name.contains("benchmark")
    {
        return Some("performance");
    }

    // Advanced patterns
    if name.contains("font")
        || name.contains("seaborn")
        || name.contains("subplot")
        || name.contains("theme")
        || name.contains("advanced")
    {
        return Some("advanced");
    }

    None
}

fn print_categorization_summary(map: &CategoryMap) {
    println!("\n📊 Categorization Summary:");
    for category in &[
        "basic",
        "statistical",
        "publication",
        "performance",
        "advanced",
    ] {
        let count = map.get(category).map(|v| v.len()).unwrap_or(0);
        println!("   {} {} example(s)", category, count);
    }
}

fn generate_images(map: &CategoryMap) -> Result<(), Box<dyn std::error::Error>> {
    for (category, examples) in map.iter() {
        println!("   📁 {}/", category);

        for example in examples {
            let example_name = example.file_stem().unwrap().to_str().unwrap();

            // Run the example
            print!("      ▶ {} ... ", example_name);

            let output = Command::new("cargo")
                .args(&["run", "--example", example_name, "--quiet"])
                .output()?;

            if output.status.success() {
                println!("✅");
            } else {
                println!("⚠️  (skipped)");
            }
        }
    }

    Ok(())
}

fn update_category_readmes(map: &CategoryMap) -> Result<(), Box<dyn std::error::Error>> {
    for (category, examples) in map.iter() {
        let category_path = Path::new("docs/gallery").join(category);
        let readme_path = category_path.join("README.md");

        let title = match category.as_str() {
            "basic" => "Basic Plots",
            "statistical" => "Statistical Plots",
            "publication" => "Publication Quality",
            "performance" => "Performance Demonstrations",
            "advanced" => "Advanced Techniques",
            _ => category,
        };

        let description = match category.as_str() {
            "basic" => "Fundamental plot types for everyday data visualization.",
            "statistical" => "Statistical analysis and distribution visualization.",
            "publication" => {
                "Professional figures meeting journal standards (IEEE, Nature, Science)."
            }
            "performance" => "Large dataset handling with parallel rendering and optimization.",
            "advanced" => "Complex visualizations and advanced customization.",
            _ => "",
        };

        let mut content = format!("# {}\n\n{}\n\n## Examples\n\n", title, description);

        // Add images for each example
        for example in examples {
            let example_name = example.file_stem().unwrap().to_str().unwrap();
            let image_name = format!("{}.png", example_name);

            if category_path.join(&image_name).exists() {
                let title = example_name.replace('_', " ").replace(".rs", "");
                content.push_str(&format!("### {}\n\n", title));
                content.push_str(&format!("![{}]({})\n\n", title, image_name));
            }
        }

        content.push_str("\n[← Back to Gallery](../README.md)\n");

        fs::write(&readme_path, content)?;
        println!("   ✅ {}/README.md", category);
    }

    Ok(())
}

fn update_main_index(map: &CategoryMap) -> Result<(), Box<dyn std::error::Error>> {
    let mut content = String::from("# ruviz Gallery\n\n");
    content.push_str("Comprehensive visual showcase of ruviz plotting capabilities.\n\n");

    // Summary statistics
    let total_examples: usize = map.iter().map(|(_, v)| v.len()).sum();
    content.push_str(&format!("**Total Examples**: {}\n\n", total_examples));

    content.push_str("## Gallery Categories\n\n");

    // Add each category
    for category in &[
        "basic",
        "statistical",
        "publication",
        "performance",
        "advanced",
    ] {
        let count = map.get(*category).map(|v| v.len()).unwrap_or(0);

        let (title, desc, icon) = match *category {
            "basic" => (
                "Basic Plots",
                "Fundamental plot types for everyday visualization",
                "📊",
            ),
            "statistical" => (
                "Statistical Plots",
                "Statistical analysis and distributions",
                "📈",
            ),
            "publication" => (
                "Publication Quality",
                "Professional figures for journals",
                "📄",
            ),
            "performance" => (
                "Performance",
                "Large dataset handling and optimization",
                "⚡",
            ),
            "advanced" => (
                "Advanced Techniques",
                "Complex visualizations and customization",
                "🎨",
            ),
            _ => (*category, "", "📁"),
        };

        content.push_str(&format!("### {} {} ({} examples)\n\n", icon, title, count));
        content.push_str(&format!("{}\n\n", desc));
        content.push_str(&format!(
            "[View {} Examples →]({}/README.md)\n\n",
            title, category
        ));
    }

    content.push_str("---\n\n");
    content
        .push_str("All examples are automatically generated from the `examples/` directory.\n\n");
    content.push_str("To regenerate the gallery:\n\n");
    content.push_str("```bash\n");
    content.push_str("cargo run --bin generate_gallery\n");
    content.push_str("```\n");

    fs::write("docs/gallery/README.md", content)?;
    println!("   ✅ docs/gallery/README.md");

    Ok(())
}

// Helper struct for category mapping
struct CategoryMap {
    map: std::collections::HashMap<String, Vec<PathBuf>>,
}

impl CategoryMap {
    fn new() -> Self {
        CategoryMap {
            map: std::collections::HashMap::new(),
        }
    }

    fn add(&mut self, category: String, path: PathBuf) {
        self.map.entry(category).or_insert_with(Vec::new).push(path);
    }

    fn get(&self, category: &str) -> Option<&Vec<PathBuf>> {
        self.map.get(category)
    }

    fn iter(&self) -> impl Iterator<Item = (&String, &Vec<PathBuf>)> {
        self.map.iter()
    }
}
