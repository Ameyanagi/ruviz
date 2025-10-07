# Week 4: Visual Gallery - Detailed Plan

**Status**: In Progress
**Phase**: Phase 2 - User Experience
**Dates**: Following Week 3 completion

## Objective

Create a comprehensive visual gallery showcasing ruviz capabilities through categorized examples with automated generation and GitHub Pages deployment.

## TDD Approach

Following strict Test-Driven Development:
1. **Write failing tests first** for gallery script
2. **Implement minimal code** to pass tests
3. **Refactor** for clarity and maintainability
4. **Verify** gallery generates correctly

## Tasks Breakdown

### Task 1: Gallery Structure (TDD)

**Test First**:
```rust
// tests/gallery_structure_test.rs
#[test]
fn test_gallery_directory_structure() {
    let gallery_path = Path::new("docs/gallery");
    assert!(gallery_path.exists());
    assert!(gallery_path.join("basic").exists());
    assert!(gallery_path.join("statistical").exists());
    assert!(gallery_path.join("publication").exists());
    assert!(gallery_path.join("performance").exists());
    assert!(gallery_path.join("advanced").exists());
}

#[test]
fn test_gallery_index_exists() {
    let index = Path::new("docs/gallery/README.md");
    assert!(index.exists());
    let content = std::fs::read_to_string(index).unwrap();
    assert!(content.contains("# ruviz Gallery"));
}
```

**Implementation**:
- Create `docs/gallery/` directory structure
- Create category subdirectories
- Create `docs/gallery/README.md` template

### Task 2: Gallery Generation Script (TDD)

**Test First**:
```rust
// tests/gallery_generator_test.rs
use std::path::Path;

#[test]
fn test_generate_basic_examples() {
    let generator = GalleryGenerator::new();
    generator.generate_category("basic").unwrap();

    // Verify basic examples rendered
    assert!(Path::new("docs/gallery/basic/line_plot.png").exists());
    assert!(Path::new("docs/gallery/basic/scatter_plot.png").exists());
}

#[test]
fn test_markdown_generation() {
    let generator = GalleryGenerator::new();
    let markdown = generator.generate_category_index("basic").unwrap();

    assert!(markdown.contains("## Basic Plots"));
    assert!(markdown.contains("![Line Plot](line_plot.png)"));
}

#[test]
fn test_thumbnail_generation() {
    let generator = GalleryGenerator::new();
    generator.generate_thumbnail("docs/gallery/basic/line_plot.png").unwrap();

    assert!(Path::new("docs/gallery/basic/thumbnails/line_plot_thumb.png").exists());
}
```

**Implementation**:
- Create `scripts/generate_gallery.rs`
- Implement `GalleryGenerator` struct
- Implement category rendering
- Implement markdown index generation
- Implement thumbnail generation

### Task 3: Example Categorization (TDD)

**Test First**:
```rust
#[test]
fn test_example_categorization() {
    let categorizer = ExampleCategorizer::new();

    assert_eq!(categorizer.category_of("basic_example.rs"), "basic");
    assert_eq!(categorizer.category_of("scientific_plotting.rs"), "publication");
    assert_eq!(categorizer.category_of("parallel_demo.rs"), "performance");
}

#[test]
fn test_all_examples_categorized() {
    let categorizer = ExampleCategorizer::new();
    let examples = find_all_examples();

    for example in examples {
        let category = categorizer.category_of(&example);
        assert!(category.is_some(), "Example {} not categorized", example);
    }
}
```

**Implementation**:
- Create `ExampleCategorizer` with pattern matching
- Map all existing examples to categories
- Ensure no examples are uncategorized

### Task 4: Gallery Rendering (TDD)

**Test First**:
```rust
#[test]
fn test_render_all_examples() {
    let renderer = GalleryRenderer::new();
    let results = renderer.render_all().unwrap();

    assert_eq!(results.successful, results.total);
    assert!(results.errors.is_empty());
}

#[test]
fn test_example_metadata() {
    let example = Example::load("basic_example.rs").unwrap();

    assert!(example.title.len() > 0);
    assert!(example.description.len() > 0);
    assert!(example.code_snippet.len() > 0);
}
```

**Implementation**:
- Extract metadata from example files (comments)
- Render each example to PNG
- Generate code snippets for display
- Handle rendering errors gracefully

### Task 5: Index Generation (TDD)

**Test First**:
```rust
#[test]
fn test_main_index_generation() {
    let generator = IndexGenerator::new();
    let index = generator.generate_main_index().unwrap();

    assert!(index.contains("# ruviz Gallery"));
    assert!(index.contains("## Basic Plots"));
    assert!(index.contains("## Statistical Plots"));
    assert!(index.contains("thumbnails/"));
}

#[test]
fn test_category_index_generation() {
    let generator = IndexGenerator::new();
    let index = generator.generate_category_index("basic").unwrap();

    assert!(index.contains("## Basic Plots"));
    assert!(index.contains("[Back to Gallery]"));
}
```

**Implementation**:
- Generate main `docs/gallery/README.md`
- Generate category-specific README files
- Include thumbnails with links to full images
- Add code snippets and descriptions

## Gallery Categories

### 1. Basic Plots (`docs/gallery/basic/`)
- Line plots
- Scatter plots
- Bar charts
- Simple examples from getting started

**Examples to include**:
- `examples/basic_example.rs`
- Simple line, scatter, bar demonstrations

### 2. Statistical Plots (`docs/gallery/statistical/`)
- Histograms
- Box plots
- Distribution analysis
- Statistical summaries

**Examples to include**:
- `examples/histogram_example.rs`
- `examples/boxplot_example.rs`

### 3. Publication Quality (`docs/gallery/publication/`)
- IEEE-format figures
- Multi-panel layouts
- Professional themes
- High-DPI exports

**Examples to include**:
- `examples/scientific_plotting.rs`
- `examples/simple_publication_test.rs`
- `examples/scientific_showcase.rs`

### 4. Performance Demonstrations (`docs/gallery/performance/`)
- Large dataset handling
- Parallel rendering
- Memory optimization
- Benchmark visualizations

**Examples to include**:
- `examples/parallel_demo.rs`
- `examples/memory_optimization_demo.rs`

### 5. Advanced Techniques (`docs/gallery/advanced/`)
- Custom fonts
- Seaborn styling
- Complex subplots
- Advanced patterns

**Examples to include**:
- `examples/font_demo.rs`
- `examples/seaborn_style_example.rs`
- `examples/subplot_example.rs`

## Script Structure

```
scripts/
  generate_gallery.rs          # Main generation script
    ├── GalleryGenerator       # Core generator
    ├── ExampleCategorizer     # Example → category mapping
    ├── GalleryRenderer        # Renders examples to PNG
    ├── ThumbnailGenerator     # Creates thumbnails
    └── IndexGenerator         # Generates markdown indexes
```

## Deliverables

1. **Gallery directory structure** (`docs/gallery/`)
2. **Automated generation script** (`scripts/generate_gallery.rs`)
3. **Test suite** (`tests/gallery_*_test.rs`)
4. **Generated gallery** with:
   - Main index (`docs/gallery/README.md`)
   - Category indexes
   - ~25+ example images
   - Thumbnails for all images
   - Code snippets with descriptions

## Testing Strategy

### Unit Tests
- Gallery structure validation
- Categorization logic
- Markdown generation
- Thumbnail creation

### Integration Tests
- End-to-end gallery generation
- All examples render successfully
- Index links are valid
- Images exist and are non-empty

### Visual Validation
- Manual review of generated gallery
- Check image quality
- Verify thumbnails are appropriately sized
- Ensure markdown formatting is correct

## Success Criteria

- [ ] All tests pass
- [ ] Gallery script generates complete gallery
- [ ] All existing examples categorized and rendered
- [ ] Thumbnails generated for all images
- [ ] Main index and category indexes complete
- [ ] Gallery is visually appealing and well-organized
- [ ] Documentation links to gallery
- [ ] Gallery can be regenerated with single command

## Timeline

**Day 1**: Write tests + implement gallery structure
**Day 2**: Implement generation script (TDD)
**Day 3**: Generate gallery + create indexes
**Day 4**: Polish + documentation integration

**Total**: 3-4 days

## Next Steps After Week 4

Week 5: API Simplification
- Auto-optimization API
- Simple API module
- Documentation updates
