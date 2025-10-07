# Styling & Themes

Complete guide to customizing plot appearance with colors, markers, themes, and publication-quality output.

## Colors

### RGB Colors

```rust
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y)
    .color(Color::from_rgb(255, 0, 0))  // Red
    .save("red_line.png")?;
```

**Common Colors**:
```rust
Color::from_rgb(255, 0, 0)      // Red
Color::from_rgb(0, 255, 0)      // Green
Color::from_rgb(0, 0, 255)      // Blue
Color::from_rgb(255, 255, 0)    // Yellow
Color::from_rgb(255, 0, 255)    // Magenta
Color::from_rgb(0, 255, 255)    // Cyan
Color::from_rgb(0, 0, 0)        // Black
Color::from_rgb(255, 255, 255)  // White
Color::from_rgb(128, 128, 128)  // Gray
```

### Hex Colors

```rust
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y)
    .color(Color::from_hex("#FF5733"))  // Coral
    .save("hex_color.png")?;
```

### seaborn Color Palettes

**Muted Palette** (professional, readable):
```rust
// Blue
Color::from_rgb(76, 114, 176)   // #4C72B0

// Orange
Color::from_rgb(221, 132, 82)   // #DD8452

// Green
Color::from_rgb(85, 168, 104)   // #55A868

// Red
Color::from_rgb(196, 78, 82)    // #C44E52

// Purple
Color::from_rgb(129, 114, 179)  // #8172B3
```

**Example with seaborn colors**:
```rust
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y1)
        .label("Series 1")
        .color(Color::from_rgb(76, 114, 176))   // Muted blue
    .line(&x, &y2)
        .label("Series 2")
        .color(Color::from_rgb(221, 132, 82))   // Muted orange
    .line(&x, &y3)
        .label("Series 3")
        .color(Color::from_rgb(85, 168, 104))   // Muted green
    .legend(Position::TopRight)
    .save("seaborn_colors.png")?;
```

### Alpha (Transparency)

```rust
use ruviz::prelude::*;

// Semi-transparent colors
Color::from_rgba(255, 0, 0, 128)  // 50% transparent red

Plot::new()
    .scatter(&x, &y)
    .color(Color::from_rgba(0, 0, 255, 100))  // Transparent blue
    .marker_size(10.0)
    .save("transparent_scatter.png")?;
```

## Line Styling

### Line Width

```rust
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y)
    .line_width(0.5)   // Thin line
    .save("thin_line.png")?;

Plot::new()
    .line(&x, &y)
    .line_width(3.0)   // Thick line
    .save("thick_line.png")?;
```

### Line Styles

```rust
use ruviz::prelude::*;

// Solid line (default)
Plot::new()
    .line(&x, &y1)
    .line_style(LineStyle::Solid)
    .label("Solid")
    .save("line_solid.png")?;

// Dashed line
Plot::new()
    .line(&x, &y2)
    .line_style(LineStyle::Dashed)
    .label("Dashed")
    .save("line_dashed.png")?;

// Dotted line
Plot::new()
    .line(&x, &y3)
    .line_style(LineStyle::Dotted)
    .label("Dotted")
    .save("line_dotted.png")?;

// Dash-dot line
Plot::new()
    .line(&x, &y4)
    .line_style(LineStyle::DashDot)
    .label("DashDot")
    .save("line_dashdot.png")?;
```

### Combined Line Styling

```rust
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y)
    .line_width(2.5)
    .line_style(LineStyle::Dashed)
    .color(Color::from_rgb(255, 100, 0))
    .label("Styled Line")
    .legend(Position::TopRight)
    .save("line_fully_styled.png")?;
```

## Marker Styling

### Marker Shapes

```rust
use ruviz::prelude::*;

// Circle (most common)
Plot::new()
    .scatter(&x, &y)
    .marker(MarkerStyle::Circle)
    .save("marker_circle.png")?;

// Square
Plot::new()
    .scatter(&x, &y)
    .marker(MarkerStyle::Square)
    .save("marker_square.png")?;

// Triangle
Plot::new()
    .scatter(&x, &y)
    .marker(MarkerStyle::Triangle)
    .save("marker_triangle.png")?;

// Diamond
Plot::new()
    .scatter(&x, &y)
    .marker(MarkerStyle::Diamond)
    .save("marker_diamond.png")?;

// Cross
Plot::new()
    .scatter(&x, &y)
    .marker(MarkerStyle::Cross)
    .save("marker_cross.png")?;

// Plus
Plot::new()
    .scatter(&x, &y)
    .marker(MarkerStyle::Plus)
    .save("marker_plus.png")?;
```

### Marker Size

```rust
use ruviz::prelude::*;

Plot::new()
    .scatter(&x, &y)
    .marker(MarkerStyle::Circle)
    .marker_size(4.0)   // Small
    .save("marker_small.png")?;

Plot::new()
    .scatter(&x, &y)
    .marker(MarkerStyle::Circle)
    .marker_size(12.0)  // Large
    .save("marker_large.png")?;
```

### Marker Colors

```rust
use ruviz::prelude::*;

Plot::new()
    .scatter(&x, &y)
    .marker(MarkerStyle::Circle)
    .marker_size(8.0)
    .color(Color::from_rgb(255, 0, 128))  // Pink markers
    .save("colored_markers.png")?;
```

### Differentiated Series

```rust
use ruviz::prelude::*;

Plot::new()
    .scatter(&x1, &y1)
        .label("Group A")
        .marker(MarkerStyle::Circle)
        .marker_size(8.0)
        .color(Color::from_rgb(255, 0, 0))
    .scatter(&x2, &y2)
        .label("Group B")
        .marker(MarkerStyle::Square)
        .marker_size(8.0)
        .color(Color::from_rgb(0, 0, 255))
    .scatter(&x3, &y3)
        .label("Group C")
        .marker(MarkerStyle::Triangle)
        .marker_size(10.0)
        .color(Color::from_rgb(0, 128, 0))
    .legend(Position::TopRight)
    .save("differentiated_series.png")?;
```

## Themes

ruviz includes professional themes for different use cases.

### Light Theme (Default)

```rust
use ruviz::prelude::*;

Plot::new()
    .theme(Theme::light())  // White background, dark text
    .line(&x, &y)
    .title("Light Theme")
    .save("theme_light.png")?;
```

**Characteristics**:
- White background
- Black text and axes
- Clean, minimal styling
- Ideal for: General use, presentations

### Dark Theme

```rust
use ruviz::prelude::*;

Plot::new()
    .theme(Theme::dark())  // Dark background, light text
    .line(&x, &y)
    .title("Dark Theme")
    .save("theme_dark.png")?;
```

**Characteristics**:
- Dark background
- Light text and axes
- Reduced eye strain
- Ideal for: Screens, dark mode applications

### Publication Theme

```rust
use ruviz::prelude::*;

Plot::new()
    .theme(Theme::publication())  // IEEE/Nature quality
    .line(&x, &y)
    .title("Publication Theme")
    .save("theme_publication.png")?;
```

**Characteristics**:
- Optimized for print
- Professional typography
- High-contrast elements
- Clean, minimal design
- Ideal for: Journal articles, academic papers, theses

### seaborn Theme

```rust
use ruviz::prelude::*;

Plot::new()
    .theme(Theme::seaborn())  // matplotlib seaborn style
    .line(&x, &y)
    .title("seaborn Theme")
    .save("theme_seaborn.png")?;
```

**Characteristics**:
- Muted color palette
- Grid by default
- Clean, minimal styling
- Optimized for readability
- Ideal for: Statistical analysis, data science, Python migration

## Typography

### Title Font

```rust
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y)
    .title("Custom Title Font")
    .title_font("Arial", 18.0)  // Font name, size
    .save("custom_title.png")?;
```

### Axis Label Fonts

```rust
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y)
    .xlabel("X Axis")
    .xlabel_font("Times New Roman", 14.0)
    .ylabel("Y Axis")
    .ylabel_font("Times New Roman", 14.0)
    .save("custom_axis_fonts.png")?;
```

### System Fonts

**Common system fonts**:
- **Arial** (Windows, macOS, Linux)
- **Times New Roman** (Windows, macOS)
- **Helvetica** (macOS)
- **Liberation Sans** (Linux)
- **DejaVu Sans** (Linux)

### Open Fonts (Auto-Download)

**Google Fonts support**:
```rust
Plot::new()
    .line(&x, &y)
    .title("Open Sans Title")
    .title_font("Open Sans", 16.0)  // Auto-downloads from Google Fonts
    .save("open_font.png")?;
```

**Popular open fonts**:
- **Open Sans** - Clean, professional
- **Roboto** - Modern, geometric
- **Lato** - Humanist, warm
- **Montserrat** - Geometric, elegant
- **Source Sans Pro** - Technical, clean

### Custom TTF Files

```rust
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y)
    .title("Custom Font Title")
    .title_font_file("path/to/custom_font.ttf", 16.0)
    .save("custom_ttf_font.png")?;
```

## Grid

### Basic Grid

```rust
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y)
    .grid(true)  // Enable grid
    .save("with_grid.png")?;
```

### No Grid

```rust
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y)
    .grid(false)  // Disable grid (default for most themes)
    .save("no_grid.png")?;
```

**Note**: seaborn theme enables grid by default.

## Legend

### Legend Position

```rust
use ruviz::prelude::*;

// Top right (most common)
Plot::new()
    .line(&x, &y)
    .label("Data")
    .legend(Position::TopRight)
    .save("legend_top_right.png")?;

// Top left
Plot::new()
    .line(&x, &y)
    .label("Data")
    .legend(Position::TopLeft)
    .save("legend_top_left.png")?;

// Bottom right
Plot::new()
    .line(&x, &y)
    .label("Data")
    .legend(Position::BottomRight)
    .save("legend_bottom_right.png")?;

// Bottom left
Plot::new()
    .line(&x, &y)
    .label("Data")
    .legend(Position::BottomLeft)
    .save("legend_bottom_left.png")?;
```

**Available positions**:
- `Position::TopLeft`
- `Position::TopRight`
- `Position::BottomLeft`
- `Position::BottomRight`

### Legend with Multiple Series

```rust
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y1)
        .label("Linear")
        .color(Color::from_rgb(255, 0, 0))
    .line(&x, &y2)
        .label("Quadratic")
        .color(Color::from_rgb(0, 0, 255))
    .line(&x, &y3)
        .label("Cubic")
        .color(Color::from_rgb(0, 128, 0))
    .legend(Position::TopLeft)
    .save("multi_series_legend.png")?;
```

## Axes

### Axis Limits

```rust
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y)
    .xlim(0.0, 10.0)  // X axis from 0 to 10
    .ylim(-5.0, 5.0)  // Y axis from -5 to 5
    .save("custom_limits.png")?;
```

### Axis Labels

```rust
use ruviz::prelude::*;

Plot::new()
    .line(&x, &y)
    .xlabel("Time (seconds)")
    .ylabel("Amplitude (mV)")
    .save("axis_labels.png")?;
```

## Figure Dimensions

### Custom Size

```rust
use ruviz::prelude::*;

Plot::new()
    .dimensions(1200, 800)  // Width x Height in pixels
    .line(&x, &y)
    .title("Custom Dimensions")
    .save("custom_size.png")?;
```

**Common sizes**:
- **800 × 600** - Standard screen
- **1200 × 800** - Large screen
- **1920 × 1080** - Full HD
- **1000 × 600** - Wide presentation
- **600 × 600** - Square

### Publication Sizes

```rust
use ruviz::prelude::*;

// Single column (IEEE)
Plot::new()
    .dimensions(252, 189)  // 3.5" × 2.625" @ 72 DPI
    .dpi(300)              // High resolution
    .theme(Theme::publication())
    .line(&x, &y)
    .save("ieee_single_column.png")?;

// Double column (IEEE)
Plot::new()
    .dimensions(523, 392)  // 7.25" × 5.44" @ 72 DPI
    .dpi(300)
    .theme(Theme::publication())
    .line(&x, &y)
    .save("ieee_double_column.png")?;
```

## Resolution (DPI)

### Standard DPI

```rust
use ruviz::prelude::*;

// Screen (default)
Plot::new()
    .dpi(96)  // Standard screen resolution
    .line(&x, &y)
    .save("screen_dpi.png")?;

// High-quality screen
Plot::new()
    .dpi(150)  // Retina/HiDPI
    .line(&x, &y)
    .save("high_screen_dpi.png")?;
```

### Publication DPI

```rust
use ruviz::prelude::*;

// Print quality
Plot::new()
    .dpi(300)  // Standard print
    .line(&x, &y)
    .save("print_dpi.png")?;

// High-quality print
Plot::new()
    .dpi(600)  // Premium print, archival
    .line(&x, &y)
    .save("premium_dpi.png")?;
```

**DPI Guidelines**:
- **72-96 DPI**: Web, presentations
- **150 DPI**: High-quality screens
- **300 DPI**: Journals, publications, print
- **600 DPI**: Premium print, archival

## Complete Styling Example

### Scientific Publication Plot

```rust
use ruviz::prelude::*;
use std::f64::consts::PI;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate data
    let x: Vec<f64> = (0..200).map(|i| i as f64 * 0.05).collect();
    let y_exp: Vec<f64> = x.iter().map(|v| (-v).exp()).collect();
    let y_sin: Vec<f64> = x.iter().map(|v| v.sin() * (-v/5.0).exp()).collect();

    // Create publication-quality plot
    Plot::new()
        // Figure setup
        .dimensions(1000, 700)
        .dpi(300)
        .theme(Theme::publication())

        // Data series
        .line(&x, &y_exp)
            .label("Exponential Decay")
            .color(Color::from_rgb(76, 114, 176))   // Muted blue
            .line_width(2.0)
            .line_style(LineStyle::Solid)

        .line(&x, &y_sin)
            .label("Damped Oscillation")
            .color(Color::from_rgb(221, 132, 82))   // Muted orange
            .line_width(2.0)
            .line_style(LineStyle::Dashed)

        // Labels and formatting
        .title("Exponential Functions")
        .title_font("Arial", 16.0)
        .xlabel("Time (s)")
        .xlabel_font("Arial", 14.0)
        .ylabel("Amplitude")
        .ylabel_font("Arial", 14.0)

        // Layout
        .xlim(0.0, 10.0)
        .ylim(-0.5, 1.0)
        .grid(true)
        .legend(Position::TopRight)

        .save("publication_plot.png")?;

    println!("✅ Publication-quality plot saved");
    Ok(())
}
```

### Presentation Plot

```rust
use ruviz::prelude::*;

Plot::new()
    .dimensions(1920, 1080)  // Full HD
    .dpi(150)
    .theme(Theme::dark())     // Dark for projector

    .line(&x, &y)
    .color(Color::from_rgb(100, 200, 255))  // Bright cyan
    .line_width(4.0)  // Thick for visibility

    .title("Presentation Plot")
    .title_font("Arial", 24.0)  // Large font
    .xlabel("X Axis")
    .xlabel_font("Arial", 20.0)
    .ylabel("Y Axis")
    .ylabel_font("Arial", 20.0)

    .grid(true)
    .save("presentation.png")?;
```

## Style Best Practices

### Accessibility

1. **Color Contrast**: Use high-contrast colors for readability
2. **Color Blindness**: Avoid red-green only distinctions
3. **Line Styles**: Combine colors with line styles/markers
4. **Font Size**: Minimum 10pt for print, 12pt for screen

### Publication Standards

1. **DPI**: Use 300 DPI minimum
2. **Theme**: Use `Theme::publication()`
3. **Colors**: Muted, professional palette
4. **Fonts**: Standard fonts (Arial, Times New Roman)
5. **Size**: Match journal requirements

### Data Visualization Principles

1. **Maximize Data-Ink Ratio**: Remove unnecessary elements
2. **Clear Hierarchy**: Title > labels > legend > grid
3. **Consistent Styling**: Same style across related plots
4. **Readable Labels**: Descriptive, concise labels
5. **Appropriate Colors**: Meaningful color choices

## Next Steps

- **[Subplots & Composition](06_subplots.md)** - Multi-panel figures
- **[Backend Selection](07_backends.md)** - Rendering backends
- **[Performance](08_performance.md)** - Optimization strategies
- **[Export Formats](10_export.md)** - File formats and settings

---

**Ready for multi-panel figures?** → [Subplots Guide](06_subplots.md)
