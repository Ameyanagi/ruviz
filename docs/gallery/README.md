# ruviz Gallery

Comprehensive visual showcase of ruviz plotting capabilities.

**Total Examples**: 9 curated visualizations across 5 categories

## Gallery Categories

### 📊 Basic Plots (3 examples)

Fundamental plot types for everyday visualization

- Box plots with quartiles and outliers
- Histograms with automatic binning
- Line plots for simple data series

[View Basic Examples →](basic/README.md)

### 📈 Statistical Plots (2 examples)

Statistical analysis and distributions

- Seaborn-style box plots
- Distribution histograms

[View Statistical Examples →](statistical/README.md)

### 📄 Publication Quality (1 example)

Professional figures for journals

- Multi-panel scientific figures
- IEEE/Nature/Science compliant
- High-DPI export ready

[View Publication Examples →](publication/README.md)

### ⚡ Performance (2 examples)

Large dataset handling and optimization

- 100K point parallel rendering
- Memory-optimized workflows
- Sub-second performance

[View Performance Examples →](performance/README.md)

### 🎨 Advanced Techniques (1 example)

Complex visualizations and customization

- Seaborn color palettes
- Professional theming
- Custom styling

[View Advanced Examples →](advanced/README.md)

---

## Features Demonstrated

✅ **Plot Types**: Line, scatter, bar, histogram, box plot
✅ **Styling**: Seaborn themes, publication quality, professional typography
✅ **Performance**: Parallel rendering, memory optimization, large datasets
✅ **Quality**: High-DPI export, IEEE compliance, journal-ready figures
✅ **Advanced**: Custom palettes, multi-panel layouts, statistical analysis

## Generate Gallery

All examples are automatically generated from the `examples/` directory and can be regenerated using:

```bash
cargo run --bin generate_gallery
```

Or manually run specific examples:

```bash
cargo run --example boxplot_example
cargo run --example histogram_example
cargo run --example scientific_showcase
cargo run --example parallel_demo
cargo run --example memory_optimization_demo
cargo run --example seaborn_style_example
```
