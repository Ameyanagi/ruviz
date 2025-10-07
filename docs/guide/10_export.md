# Export & Formats

Complete guide to exporting high-quality plots with DPI settings, custom dimensions, and file formats.

## Overview

ruviz currently supports PNG export with comprehensive control over quality and resolution. SVG support is planned for v0.2.

| Format | Status | Use Case |
|--------|--------|----------|
| **PNG** | ✅ Supported | Screen, web, print, universal compatibility |
| **SVG** | ⏳ Planned v0.2 | Vector graphics, scalable, web embedding |
| **PDF** | ⏳ Planned v0.3 | Publications, archival |
| **JPEG** | ⏳ Planned v0.4 | Photos, web (lossy compression) |

## PNG Export

### Basic Export

```rust
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y)
    .title("Basic Export")
    .save("plot.png")?;
```

**Default settings**:
- DPI: 96 (standard screen)
- Dimensions: 800×600 pixels
- Format: PNG with lossless compression

### Custom File Path

```rust
use ruviz::prelude::*;

// Absolute path
Plot::new()
    .line(&x, &y)
    .save("/home/user/plots/output.png")?;

// Relative path
Plot::new()
    .line(&x, &y)
    .save("../results/figure_1.png")?;

// Create directories if needed
std::fs::create_dir_all("output/plots")?;
Plot::new()
    .line(&x, &y)
    .save("output/plots/analysis.png")?;
```

## Resolution Control (DPI)

**DPI (Dots Per Inch)** controls image resolution and quality.

### Standard DPI Values

```rust
use ruviz::prelude::*;

// Screen (default)
Plot::new()
    .line(&x, &y)
    .dpi(96)  // Standard screen resolution
    .save("screen.png")?;

// High-quality screen (Retina/HiDPI)
Plot::new()
    .line(&x, &y)
    .dpi(150)
    .save("retina.png")?;

// Print quality (journals, publications)
Plot::new()
    .line(&x, &y)
    .dpi(300)  // Standard for IEEE, Nature, etc.
    .save("print.png")?;

// Premium print (archival, posters)
Plot::new()
    .line(&x, &y)
    .dpi(600)
    .save("premium.png")?;
```

### DPI Guidelines

| DPI | Use Case | File Size | Quality |
|-----|----------|-----------|---------|
| **72** | Web (legacy) | Smallest | Basic |
| **96** | Screen (default) | Small | Good |
| **150** | High-quality screen | Medium | Very good |
| **300** | Publication print | Large | Excellent |
| **600** | Premium print | Very large | Outstanding |

### DPI vs File Size

```rust
use ruviz::prelude::*;

// DPI affects both resolution and file size
Plot::new()
    .line(&x, &y)
    .dimensions(800, 600)
    .dpi(96)   // ~50KB
    .save("dpi_96.png")?;

Plot::new()
    .line(&x, &y)
    .dimensions(800, 600)
    .dpi(300)  // ~250KB (5x larger)
    .save("dpi_300.png")?;

Plot::new()
    .line(&x, &y)
    .dimensions(800, 600)
    .dpi(600)  // ~900KB (18x larger)
    .save("dpi_600.png")?;
```

**DPI scaling**:
- 150 DPI: ~2.5× file size vs 96 DPI
- 300 DPI: ~10× file size vs 96 DPI
- 600 DPI: ~40× file size vs 96 DPI

## Custom Dimensions

### Pixel Dimensions

```rust
use ruviz::prelude::*;

// Widescreen (16:9)
Plot::new()
    .dimensions(1920, 1080)
    .line(&x, &y)
    .save("widescreen.png")?;

// Square
Plot::new()
    .dimensions(1000, 1000)
    .line(&x, &y)
    .save("square.png")?;

// Portrait
Plot::new()
    .dimensions(600, 800)
    .line(&x, &y)
    .save("portrait.png")?;

// Ultra-wide
Plot::new()
    .dimensions(2560, 1080)
    .line(&x, &y)
    .save("ultrawide.png")?;
```

### Common Sizes

```rust
use ruviz::prelude::*;

// Presentation (4:3)
Plot::new()
    .dimensions(1024, 768)
    .line(&x, &y)
    .save("presentation_43.png")?;

// Presentation (16:9)
Plot::new()
    .dimensions(1920, 1080)
    .line(&x, &y)
    .save("presentation_169.png")?;

// Social media (Instagram post)
Plot::new()
    .dimensions(1080, 1080)
    .line(&x, &y)
    .save("instagram.png")?;

// Social media (Twitter card)
Plot::new()
    .dimensions(1200, 675)
    .line(&x, &y)
    .save("twitter.png")?;
```

## Publication-Quality Export

### IEEE Format

**IEEE requires specific dimensions for publication figures**.

#### Single-Column Figure

```rust
use ruviz::prelude::*;

// IEEE single-column: 3.5 inches wide
// At 300 DPI: 3.5" × 300 = 1050 pixels
Plot::new()
    .dimensions(1050, 787)  // 3.5" × 2.625" @ 300 DPI
    .dpi(300)
    .theme(Theme::publication())
    .line(&x, &y)
    .title("Figure 1: Experimental Results")
    .xlabel("Input Parameter")
    .ylabel("Output Response")
    .save("ieee_single_column.png")?;
```

#### Double-Column Figure

```rust
use ruviz::prelude::*;

// IEEE double-column: 7.25 inches wide
// At 300 DPI: 7.25" × 300 = 2175 pixels
Plot::new()
    .dimensions(2175, 1631)  // 7.25" × 5.44" @ 300 DPI
    .dpi(300)
    .theme(Theme::publication())
    .line(&x, &y)
    .title("Figure 2: Comprehensive Analysis")
    .xlabel("Time (s)")
    .ylabel("Amplitude (V)")
    .save("ieee_double_column.png")?;
```

### Nature Format

```rust
use ruviz::prelude::*;

// Nature single-column: 89mm = 3.5 inches
// At 300 DPI: 1050 pixels
Plot::new()
    .dimensions(1050, 1050)  // Square or custom height
    .dpi(300)
    .theme(Theme::publication())
    .line(&x, &y)
    .title("a")  // Nature uses lowercase panel letters
    .save("nature_panel_a.png")?;

// Nature two-column: 183mm = 7.2 inches
Plot::new()
    .dimensions(2160, 1440)
    .dpi(300)
    .theme(Theme::publication())
    .line(&x, &y)
    .save("nature_full_width.png")?;
```

### Science/Cell Format

```rust
use ruviz::prelude::*;

// Science single-column: 2.37 inches @ 300 DPI = 711 pixels
Plot::new()
    .dimensions(711, 533)
    .dpi(300)
    .theme(Theme::publication())
    .line(&x, &y)
    .save("science_single.png")?;

// Science double-column: 4.92 inches @ 300 DPI = 1476 pixels
Plot::new()
    .dimensions(1476, 1107)
    .dpi(300)
    .theme(Theme::publication())
    .line(&x, &y)
    .save("science_double.png")?;
```

## DPI Calculation Helper

```rust
fn inches_to_pixels(inches: f64, dpi: u32) -> u32 {
    (inches * dpi as f64) as u32
}

fn main() {
    // IEEE double-column at 300 DPI
    let width = inches_to_pixels(7.25, 300);   // 2175
    let height = inches_to_pixels(5.44, 300);  // 1632

    println!("IEEE double-column @ 300 DPI: {}×{}", width, height);
}
```

## Complete Publication Example

### Journal Article Figure

```rust
use ruviz::prelude::*;
use std::f64::consts::PI;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate publication-quality data
    let x: Vec<f64> = (0..200).map(|i| i as f64 * 0.05).collect();
    let y_exp: Vec<f64> = x.iter().map(|v| (-v).exp()).collect();
    let y_decay: Vec<f64> = x.iter()
        .map(|v| (-v * 0.5).exp() * (v * 2.0).sin())
        .collect();

    // IEEE double-column figure @ 300 DPI
    Plot::new()
        .dimensions(2175, 1500)
        .dpi(300)
        .theme(Theme::publication())

        .line(&x, &y_exp)
            .label("Exponential Decay")
            .color(Color::from_rgb(76, 114, 176))
            .line_width(2.0)

        .line(&x, &y_decay)
            .label("Damped Oscillation")
            .color(Color::from_rgb(221, 132, 82))
            .line_width(2.0)
            .line_style(LineStyle::Dashed)

        .title("Figure 1: Temporal Decay Patterns")
        .title_font("Arial", 16.0)
        .xlabel("Time (s)")
        .xlabel_font("Arial", 14.0)
        .ylabel("Amplitude (normalized)")
        .ylabel_font("Arial", 14.0)

        .xlim(0.0, 10.0)
        .ylim(-0.2, 1.0)
        .grid(true)
        .legend(Position::TopRight)

        .save("journal_figure_1.png")?;

    println!("✅ Publication figure saved at 300 DPI");
    println!("   Dimensions: 2175×1500 pixels (7.25\" × 5\")");
    println!("   Format: PNG, lossless");

    Ok(())
}
```

## Export Workflow

### Development Workflow

```rust
use ruviz::prelude::*;

// 1. Quick draft (low DPI, fast iteration)
Plot::new()
    .line(&x, &y)
    .dimensions(800, 600)
    .dpi(72)  // Fast rendering
    .save("draft.png")?;

// 2. Review version (screen quality)
Plot::new()
    .line(&x, &y)
    .dimensions(1200, 800)
    .dpi(96)
    .save("review.png")?;

// 3. Final version (publication quality)
Plot::new()
    .line(&x, &y)
    .dimensions(2175, 1500)
    .dpi(300)
    .theme(Theme::publication())
    .save("final.png")?;
```

### Batch Export

```rust
use ruviz::prelude::*;

fn export_plot(data: &[f64], filename: &str, quality: &str) -> Result<(), Box<dyn std::error::Error>> {
    let x: Vec<f64> = (0..data.len()).map(|i| i as f64).collect();

    let (dpi, dimensions) = match quality {
        "draft" => (72, (800, 600)),
        "screen" => (96, (1200, 800)),
        "print" => (300, (2175, 1500)),
        _ => (96, (800, 600)),
    };

    Plot::new()
        .line(&x, data)
        .dimensions(dimensions.0, dimensions.1)
        .dpi(dpi)
        .save(filename)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = vec![/* ... */];

    // Export at multiple quality levels
    export_plot(&data, "output/draft.png", "draft")?;
    export_plot(&data, "output/screen.png", "screen")?;
    export_plot(&data, "output/print.png", "print")?;

    Ok(())
}
```

## File Size Management

### Optimizing File Size

```rust
use ruviz::prelude::*;

// Large file (high DPI + large dimensions)
Plot::new()
    .dimensions(3000, 2000)
    .dpi(600)
    .line(&x, &y)
    .save("large.png")?;  // ~2-5 MB

// Optimized (balanced quality/size)
Plot::new()
    .dimensions(1600, 1200)
    .dpi(300)
    .line(&x, &y)
    .save("optimized.png")?;  // ~500 KB

// Minimal (web-ready)
Plot::new()
    .dimensions(800, 600)
    .dpi(96)
    .line(&x, &y)
    .save("web.png")?;  // ~100 KB
```

### File Size vs Quality Trade-offs

| Configuration | Typical Size | Use Case |
|---------------|--------------|----------|
| 800×600 @ 72 DPI | ~50 KB | Web thumbnails |
| 1200×800 @ 96 DPI | ~150 KB | Web full-size |
| 1600×1200 @ 150 DPI | ~400 KB | Presentations |
| 2175×1500 @ 300 DPI | ~800 KB | Journal submission |
| 3000×2000 @ 600 DPI | ~2.5 MB | Poster printing |

## Error Handling

### Robust Export

```rust
use ruviz::prelude::*;
use std::path::Path;

fn safe_export(plot: Plot, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Check parent directory exists
    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Try export
    match plot.save(path) {
        Ok(_) => {
            println!("✅ Saved: {}", path);
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Failed to save {}: {}", path, e);
            Err(e.into())
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plot = Plot::new()
        .line(&x, &y)
        .title("Safe Export");

    safe_export(plot, "output/plots/figure_1.png")?;
    Ok(())
}
```

## Future Export Formats

### SVG Export (Planned v0.2)

```rust
// Planned API for v0.2
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y)
    .save_svg("vector_plot.svg")?;  // Scalable vector graphics
```

**Benefits**:
- Infinite scalability
- Smaller file size for simple plots
- Editable in vector graphics software
- Perfect for web embedding

### PDF Export (Planned v0.3)

```rust
// Planned API for v0.3
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y)
    .save_pdf("publication.pdf")?;  // Direct PDF output
```

**Benefits**:
- Publication-ready format
- Embedded fonts
- Print-optimized
- Archival quality

## Best Practices

### ✅ DO

1. **Use 300 DPI for publications** - Standard for journals
2. **Match journal dimensions** - Check submission guidelines
3. **Create directory structure** - Organize output files
4. **Use descriptive filenames** - `figure_1_timeseries.png` not `plot.png`
5. **Test at draft quality first** - Iterate quickly with low DPI

### ❌ DON'T

1. **Don't use 72 DPI for print** - Too low resolution
2. **Don't ignore aspect ratios** - Check journal requirements
3. **Don't export huge files unnecessarily** - Balance quality vs size
4. **Don't forget error handling** - Always handle `Result` types
5. **Don't hardcode paths** - Use configurable output directories

## Export Checklist

**For publications**:
- [ ] Check journal DPI requirements (usually 300 DPI)
- [ ] Verify dimensions match submission guidelines
- [ ] Use `Theme::publication()` for professional styling
- [ ] Test file size (< 10 MB for most journals)
- [ ] Verify PNG compression (lossless)
- [ ] Check font sizes are readable at target size

**For presentations**:
- [ ] Use screen-appropriate DPI (96-150)
- [ ] Match presentation aspect ratio (16:9 or 4:3)
- [ ] Use `Theme::dark()` for projectors
- [ ] Ensure text is large enough for distant viewing
- [ ] Test file size for embedding in slides

**For web**:
- [ ] Optimize for file size (< 500 KB)
- [ ] Use 96 DPI for screen display
- [ ] Consider responsive dimensions
- [ ] Test loading speed

## Next Steps

- **[Advanced Techniques](11_advanced.md)** - Complex visualizations
- **[Styling Guide](05_styling.md)** - Professional appearance
- **[Performance Guide](08_performance.md)** - Large dataset optimization

---

**Ready for advanced topics?** → [Advanced Techniques](11_advanced.md)
