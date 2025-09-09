# Ruviz Gallery

A showcase of plotting capabilities and examples for the Ruviz Rust plotting library.

## Directory Structure

```
gallery/
├── basic/           # Simple plots for beginners
├── advanced/        # Advanced styling and typography examples  
├── publication/     # Publication-ready plots with professional styling
├── scientific/      # Scientific and research-oriented visualizations
├── performance/     # Performance optimization and large dataset examples
└── utility/         # Development and testing utilities
```

## Gallery Categories

### 📈 Basic Examples
Simple, straightforward plots perfect for getting started.

**Location**: `gallery/basic/`

**Examples**:
- [basic_example.rs](basic/basic_example.rs) - Simple line plot demonstration
- [simple_visual_test.rs](basic/simple_visual_test.rs) - Basic visual rendering test
- [axis_legend_test.rs](basic/axis_legend_test.rs) - Axis labels and legend basics

### 🎨 Advanced Examples  
Advanced typography, font rendering, and text styling.

**Location**: `gallery/advanced/`

**Examples**:
- [font_demo.rs](advanced/font_demo.rs) - Font loading and rendering
- [cosmic_text_rotation_demo.rs](advanced/cosmic_text_rotation_demo.rs) - Text rotation with CosmicText
- [test_font_alignment.rs](advanced/test_font_alignment.rs) - Text alignment testing
- [test_font_families.rs](advanced/test_font_families.rs) - Different font family usage
- [test_plotters_style_fonts.rs](advanced/test_plotters_style_fonts.rs) - Plotters-style font rendering
- [test_text_rotation.rs](advanced/test_text_rotation.rs) - Text rotation capabilities

### 📊 Publication Examples
Professional, publication-ready plots with high-quality rendering.

**Location**: `gallery/publication/`

**Examples**:
- [simple_publication_test.rs](publication/simple_publication_test.rs) - Clean publication-ready plots
- [test_axis_labels.rs](publication/test_axis_labels.rs) - Professional axis labeling

### 🔬 Scientific Examples
Specialized plots for scientific and research applications.

**Location**: `gallery/scientific/`

**Examples**:
- [scientific_plotting.rs](scientific/scientific_plotting.rs) - Comprehensive scientific plotting with colormaps

### ⚡ Performance Examples
Performance optimization and large dataset handling.

**Location**: `gallery/performance/`

**Examples**:
- [memory_optimization_demo.rs](performance/memory_optimization_demo.rs) - Memory-efficient rendering
- [parallel_demo.rs](performance/parallel_demo.rs) - Parallel processing demonstration
- [simd_demo.rs](performance/simd_demo.rs) - SIMD optimization showcase

### 🔧 Utility Examples
Development utilities and testing tools.

**Location**: `gallery/utility/`

**Examples**:
- [generate_test_images.rs](utility/generate_test_images.rs) - Batch test image generation
- [image_gallery_generator.rs](utility/image_gallery_generator.rs) - Gallery creation utility
- [save_image_example.rs](utility/save_image_example.rs) - Basic save functionality demo
- [test_with_axes_and_grid.rs](utility/test_with_axes_and_grid.rs) - Axes and grid testing
- [verify_axes.rs](utility/verify_axes.rs) - Axis verification utility

## Running the Examples

Each example is a standalone Rust program. To run any example:

```bash
# Basic examples
cargo run --example basic_line
cargo run --example basic_scatter

# Advanced examples  
cargo run --example transparency_demo
cargo run --example line_styles

# Publication examples
cargo run --example publication_line
cargo run --example high_dpi

# Scientific examples
cargo run --example colormap_demo
cargo run --example large_dataset

# Interactive examples
cargo run --example animation
cargo run --example realtime_data
```

## Output Location

All example outputs are saved to the `test_output/` directory with descriptive names:

- `test_output/basic_line_demo.png`
- `test_output/transparency_effects.png`
- `test_output/publication_quality.png`
- `test_output/scientific_colormap.png`

## Features Demonstrated

### Core Features
- ✅ Line plots with multiple series
- ✅ Scatter plots with custom markers  
- ✅ Bar charts with categorical data
- ✅ Custom color palettes and themes
- ✅ Transparency and alpha blending
- ✅ Professional typography with TTF fonts
- ✅ High-DPI rendering (96, 150, 300 DPI)

### Advanced Features
- ✅ Line styles (solid, dashed, dotted, dash-dot)
- ✅ Enhanced tick system (major/minor ticks, inside/outside)
- ✅ Tight layout with automatic margin adjustment
- ✅ Legend positioning and styling
- ✅ Scientific colormaps (Viridis, Plasma, Inferno, Magma)
- ✅ Error bars and uncertainty visualization

### Performance Features
- ✅ Large dataset handling (100K+ points)
- ✅ Memory optimization and pooling
- ✅ Parallel rendering support
- ✅ DataShader integration for massive datasets

## API Examples

### Simple Line Plot
```rust
use ruviz::Plot;

let x = vec![1.0, 2.0, 3.0, 4.0];
let y = vec![1.0, 4.0, 2.0, 3.0];

Plot::new()
    .line(&x, &y)
    .title("Simple Line Plot")
    .xlabel("X Values")  
    .ylabel("Y Values")
    .save("output.png")?;
```

### Advanced Styling
```rust
use ruviz::{Plot, Color};
use ruviz::core::position::Position;

Plot::new()
    .scatter(&x, &y)
    .color(Color::new(255, 0, 0))
    .marker_size(8.0)
    .title("Custom Scatter Plot")
    .legend_top_right()
    .grid(true)
    .tight_layout(true)
    .save("styled_plot.png")?;
```

### Publication Quality
```rust
Plot::new()
    .line(&x, &y)
    .title("Publication Quality Plot")
    .xlabel("Time (seconds)")
    .ylabel("Amplitude (μV)")
    .dpi(300)  // High resolution for print
    .tight_layout(true)
    .save("publication.png")?;
```

## Contributing Examples

To add a new example:

1. Create the example file in the appropriate category directory
2. Add the example to the `Cargo.toml` examples section
3. Update this README with a description
4. Include sample output in `test_output/`

## Performance Benchmarks

The examples include performance demonstrations:

- **100K points**: <100ms rendering time
- **1M points**: <1s with optimization  
- **Memory usage**: <2x data size for efficient plots
- **Font rendering**: <50ms for system fonts, <100ms for web fonts

## Dependencies

Examples demonstrate usage with various data sources:

- `Vec<f64>` - Basic numeric vectors
- `ndarray` - Multi-dimensional arrays (optional)
- `polars` - DataFrames (optional)  
- Custom `Data1D` implementations

## Support

For questions about specific examples or to request new gallery entries:

- Check existing examples in this gallery
- Review the main documentation
- Open an issue for feature requests

---

*This gallery demonstrates the full capabilities of the Ruviz plotting library with practical, ready-to-use examples.*