# Ruviz Scientific Plotting Improvement Plan

Based on comprehensive analysis of matplotlib, seaborn, and Makie design patterns, here's a detailed plan to elevate ruviz's scientific plotting capabilities to publication-quality standards.

## 🎯 Executive Summary

**Goal**: Transform ruviz into a publication-ready scientific plotting library that rivals matplotlib's professional quality while maintaining high performance.

**Key Issues Identified**:
- Typography hierarchy needs scientific standards
- Color palettes are not publication/accessibility optimized  
- Grid and axis styling lacks scientific polish
- Text scaling inconsistencies across resolutions
- Missing mathematical notation support
- No scientific themes (IEEE, Nature, etc.)

## 📊 Current Analysis

### Strengths ✅
- **Cosmic-text integration**: Professional typography foundation in place
- **Y-axis rotation**: Working correctly (major breakthrough!)
- **Theme system**: Flexible architecture ready for enhancement
- **Performance**: DataShader and parallel rendering foundations
- **UTF-8 support**: International character rendering

### Critical Gaps ❌
- **Scientific color palettes**: Basic red/blue vs. accessibility-tested schemes
- **Typography ratios**: No golden ratio/scientific spacing standards  
- **Grid sophistication**: Prominent grids vs. subtle scientific standards
- **Mathematical notation**: No LaTeX/equation support
- **Resolution scaling**: Text/line ratios inconsistent across DPI
- **Publication themes**: Missing IEEE, Nature, Science journal standards

## 🔬 Scientific Plotting Requirements

### 1. Typography Hierarchy (Matplotlib Standard)

**Professional Font Scaling**:
```rust
// Scientific typography hierarchy
pub struct ScientificTypography {
    base_size: f32,           // 10pt standard
    title_scale: 1.2,         // 12pt (golden ratio)
    axis_label_scale: 1.0,    // 10pt
    tick_label_scale: 0.833,  // 8.33pt
    legend_scale: 0.9,        // 9pt
    annotation_scale: 0.8,    // 8pt
}
```

**Font Family Standards**:
- **Publications**: Times New Roman, Computer Modern (serif with math symbols)
- **Presentations**: Helvetica, Arial (sans-serif, high legibility)
- **Posters**: Larger fonts with increased contrast

### 2. Scientific Color Systems

**Perceptually Uniform Palettes**:
```rust
// Replace current basic colors with scientific standards
pub const SCIENTIFIC_PALETTE: [&str; 10] = [
    "#1f77b4", // Blue (matplotlib v2+ accessibility tested)
    "#ff7f0e", // Orange  
    "#2ca02c", // Green
    "#d62728", // Red
    "#9467bd", // Purple
    "#8c564b", // Brown
    "#e377c2", // Pink
    "#7f7f7f", // Gray
    "#bcbd22", // Olive
    "#17becf", // Cyan
];

// Paul Tol's colorblind-friendly palettes
pub const WONG_PALETTE: [&str; 8] = [
    "#000000", // Black
    "#E69F00", // Orange
    "#56B4E9", // Sky blue  
    "#009E73", // Bluish green
    "#F0E442", // Yellow
    "#0072B2", // Blue
    "#D55E00", // Vermillion
    "#CC79A7", // Reddish purple
];
```

### 3. Grid and Axis Scientific Standards

**Professional Grid System**:
```rust
pub struct ScientificGrid {
    major_color: "#E0E0E0",      // Subtle gray (seaborn standard)
    major_width: 0.8,            // Thin lines
    major_alpha: 1.0,            // Full opacity
    minor_color: "#F0F0F0",      // Even more subtle
    minor_width: 0.5,            // Thinner
    minor_alpha: 0.6,            // Translucent
    z_order: -10,                // Behind data (matplotlib v2+ standard)
}
```

**Axis Enhancement**:
- **Smart despining**: Remove unnecessary top/right spines
- **Spine offsetting**: 10pt gap from plot area
- **Tick optimization**: 5-7 major ticks (MaxNLocator algorithm)
- **Minor ticks**: Automatic subdivision for log scales

### 4. Resolution Consistency System

**DPI-Aware Scaling**:
```rust
pub struct DPIScaling {
    screen_dpi: 96.0,           // Standard screen
    print_dpi: 300.0,           // Journal submission
    web_dpi: 150.0,             // High-quality web
    poster_dpi: 200.0,          // Large format
    
    // All elements scale proportionally
    font_scale: dpi / 96.0,
    line_scale: dpi / 96.0, 
    marker_scale: (dpi / 96.0).sqrt(), // Area scaling
}
```

**Text/Line Consistency Formula**:
```rust
// Ensure visual relationships remain constant
line_width = base_font_size / 12.0 * dpi_scale;
margin_size = base_font_size * 1.5 * dpi_scale;
tick_length = base_font_size * 0.5 * dpi_scale;
```

## 🏗️ Implementation Plan

### Phase 1: Enhanced Scientific Themes (Week 1)

**1.1 Add IEEE Publication Theme**
```rust
pub fn ieee() -> Theme {
    Theme {
        background: Color::WHITE,
        foreground: Color::BLACK,
        grid_color: Color::from_hex("#E5E5E5").unwrap(),
        line_width: 0.5,                    // Thinner for print
        font_family: "Times New Roman",     // IEEE standard
        font_size: 8.0,                     // IEEE column constraint
        title_font_size: 9.0,
        axis_label_font_size: 8.0,
        tick_label_font_size: 7.0,
        color_palette: WONG_PALETTE.to_vec(), // Accessibility first
        margin: 0.12,                       // IEEE spacing
    }
}
```

**1.2 Add Nature/Science Theme**
```rust
pub fn nature_journal() -> Theme {
    Theme {
        background: Color::WHITE,
        foreground: Color::BLACK,
        grid_color: Color::TRANSPARENT,     // No grid for Nature
        line_width: 0.75,
        font_family: "Arial",               // Nature standard
        font_size: 7.0,                     // Small for multi-panel
        color_palette: SCIENTIFIC_PALETTE.to_vec(),
        margin: 0.08,                       // Tight spacing
    }
}
```

**1.3 Add Presentation Theme**
```rust
pub fn presentation() -> Theme {
    Theme {
        font_size: 14.0,                    // Large for visibility
        title_font_size: 18.0,
        line_width: 3.0,                    // Thick for projectors
        color_palette: HIGH_CONTRAST_PALETTE.to_vec(),
        margin: 0.15,                       // Extra space
    }
}
```

### Phase 2: Typography Excellence (Week 2)

**2.1 Scientific Font Hierarchy**
```rust
pub struct FontHierarchy {
    pub base_size: f32,
    pub ratios: FontRatios,
}

pub struct FontRatios {
    pub title: f32,        // 1.2 (golden ratio)
    pub subtitle: f32,     // 1.1
    pub axis_label: f32,   // 1.0
    pub tick_label: f32,   // 0.833
    pub legend: f32,       // 0.9
    pub annotation: f32,   // 0.8
    pub caption: f32,      // 0.75
}

impl FontHierarchy {
    pub fn scale_all(&mut self, factor: f32) {
        self.base_size *= factor;
        // All ratios remain constant for visual consistency
    }
}
```

**2.2 Mathematical Notation System**
```rust
// Placeholder for LaTeX integration
pub trait MathRenderer {
    fn render_latex(&mut self, expr: &str, x: f32, y: f32, size: f32) -> Result<()>;
    fn measure_latex(&self, expr: &str, size: f32) -> (f32, f32);
}

// Basic implementation with Unicode math symbols
pub fn render_math_basic(text: &str) -> String {
    text.replace("alpha", "α")
        .replace("beta", "β")
        .replace("gamma", "γ")
        .replace("delta", "δ")
        .replace("pi", "π")
        .replace("theta", "θ")
        .replace("lambda", "λ")
        .replace("mu", "μ")
        .replace("sigma", "σ")
}
```

### Phase 3: Advanced Color Systems (Week 2)

**3.1 Perceptually Uniform Palettes**
```rust
pub mod scientific_colors {
    // ColorBrewer qualitative palettes (scientifically tested)
    pub const SET1: [&str; 9] = [
        "#E41A1C", "#377EB8", "#4DAF4A", "#984EA3",
        "#FF7F00", "#FFFF33", "#A65628", "#F781BF", "#999999"
    ];
    
    // Viridis family (perceptually uniform)
    pub const VIRIDIS: [&str; 10] = [
        "#440154", "#482777", "#3F4A8A", "#31678E",
        "#26838F", "#1F9D8A", "#6CCE5A", "#B6DE2B",
        "#FEE825", "#FFFF00"
    ];
    
    // Paul Tol's bright palette
    pub const TOL_BRIGHT: [&str; 7] = [
        "#EE6677", "#228833", "#4477AA", "#CCBB44",
        "#66CCEE", "#AA3377", "#BBBBBB"
    ];
}
```

**3.2 Accessibility Validation**
```rust
pub fn validate_accessibility(colors: &[Color]) -> AccessibilityReport {
    AccessibilityReport {
        deuteranopia_safe: true,  // Most common colorblindness
        protanopia_safe: true,
        tritanopia_safe: true,
        contrast_ratios: calculate_contrasts(colors),
        print_safe: validate_grayscale(colors),
    }
}
```

### Phase 4: Grid and Axis Excellence (Week 3)

**4.1 Smart Grid System**
```rust
pub enum GridStyle {
    Scientific {        // Subtle, behind data
        major_alpha: 0.8,
        minor_alpha: 0.4, 
        color: "#E0E0E0",
        z_order: -10,
    },
    Engineering {       // More prominent
        major_alpha: 1.0,
        color: "#C0C0C0",
        z_order: -5,
    },
    Minimal {          // Tick marks only
        show_grid: false,
        tick_length: 6.0,
    },
}
```

**4.2 Intelligent Tick Generation**
```rust
// Implement matplotlib's MaxNLocator algorithm
pub fn generate_scientific_ticks(min: f64, max: f64, max_ticks: usize) -> Vec<f64> {
    let range = max - min;
    let rough_step = range / (max_ticks - 1) as f64;
    
    // Round to nice numbers: 1, 2, 5, 10, 20, 50, 100, etc.
    let magnitude = 10.0_f64.powf(rough_step.log10().floor());
    let normalized_step = rough_step / magnitude;
    
    let nice_step = if normalized_step <= 1.0 { 1.0 }
    else if normalized_step <= 2.0 { 2.0 }
    else if normalized_step <= 5.0 { 5.0 }
    else { 10.0 };
    
    let step = nice_step * magnitude;
    let start = (min / step).ceil() * step;
    
    (0..max_ticks)
        .map(|i| start + i as f64 * step)
        .take_while(|&tick| tick <= max)
        .collect()
}
```

### Phase 5: Resolution and Export Excellence (Week 4)

**5.1 DPI Fluent API Design**
```rust
// Fluent API for scientific plotting DPI control
impl Plot {
    pub fn dpi(mut self, dpi: u32) -> Self {
        self.export_config.dpi = validate_dpi(dpi);
        self
    }
}

fn validate_dpi(dpi: u32) -> u32 {
    // Clamp to minimum 72 DPI (1 point = 1/72 inch standard)
    std::cmp::max(dpi, 72)
}

// Usage examples:
// Plot::new().line(&x, &y).dpi(300).save("output.png")?;    // Print quality
// Plot::new().scatter(&x, &y).dpi(96).save("web.png")?;     // Screen quality
// Plot::new().bar(&data).dpi(600).save("ieee.png")?;        // IEEE standard
```

**5.2 Scientific DPI Standards**
```rust
pub mod dpi_presets {
    pub const SCREEN: u32 = 96;        // Standard screen resolution
    pub const WEB: u32 = 150;          // High-quality web (1.5x screen)
    pub const PRINT: u32 = 300;        // Professional print quality
    pub const IEEE: u32 = 600;         // IEEE publication requirement
    pub const POSTER: u32 = 200;       // Large format printing
    
    // Convenience methods
    impl Plot {
        pub fn screen_quality(self) -> Self { self.dpi(SCREEN) }
        pub fn web_quality(self) -> Self { self.dpi(WEB) }
        pub fn print_quality(self) -> Self { self.dpi(PRINT) }
        pub fn ieee_standard(self) -> Self { self.dpi(IEEE) }
        pub fn poster_quality(self) -> Self { self.dpi(POSTER) }
    }
}
```

**5.3 DPI-Aware Scaling Implementation**
```rust
pub struct ExportConfig {
    pub dpi: u32,
    pub format: ExportFormat,
    pub embed_fonts: bool,
    pub transparent_background: bool,
    
    // DPI-aware scaling factors (computed automatically)
    pub font_scale: f32,        // dpi / 96.0
    pub line_scale: f32,        // dpi / 96.0  
    pub marker_scale: f32,      // sqrt(dpi / 96.0) for area scaling
    pub spacing_scale: f32,     // dpi / 96.0
}

impl ExportConfig {
    pub fn new(dpi: u32) -> Self {
        let base_scale = dpi as f32 / 96.0;
        
        Self {
            dpi: validate_dpi(dpi),
            format: ExportFormat::PNG { compression: 6 },
            embed_fonts: true,
            transparent_background: false,
            font_scale: base_scale,
            line_scale: base_scale,
            marker_scale: base_scale.sqrt(), // Area scales with sqrt for visual consistency
            spacing_scale: base_scale,
        }
    }
}

// Text/line ratio consistency formula
pub fn calculate_line_width(base_font_size: f32, dpi_scale: f32) -> f32 {
    // Maintains visual relationship: line_width = font_size / 12.0 * scale
    base_font_size / 12.0 * dpi_scale
}

pub fn calculate_margin_size(base_font_size: f32, dpi_scale: f32) -> f32 {
    // Standard margin: 1.5x font size
    base_font_size * 1.5 * dpi_scale
}

pub fn calculate_tick_length(base_font_size: f32, dpi_scale: f32) -> f32 {
    // Tick length: 0.5x font size  
    base_font_size * 0.5 * dpi_scale
}
```

**5.4 Multi-DPI Export Support**
```rust
pub enum ExportFormat {
    PNG { compression: u8 },
    SVG { embed_fonts: bool },
    PDF { embed_fonts: bool },
    EPS { embed_fonts: bool },
    TIFF { compression: TiffCompression },
}
```

**5.2 Journal-Specific Export Presets**
```rust
pub mod journal_presets {
    pub fn ieee_column() -> ExportConfig {
        ExportConfig {
            dpi: 600,                    // IEEE requirement
            width: 3.5,                  // inches
            height: 2.625,               // 4:3 ratio
            format: ExportFormat::TIFF,
            embed_fonts: true,
        }
    }
    
    pub fn nature_single() -> ExportConfig {
        ExportConfig {
            dpi: 300,
            width: 89.0,                 // mm
            height: 60.0,
            format: ExportFormat::PDF,
            embed_fonts: true,
        }
    }
}
```

## 🎨 Enhanced Theme System

### Complete Scientific Theme Catalog

```rust
impl Theme {
    // Journal-specific themes
    pub fn ieee() -> Self { /* IEEE standards */ }
    pub fn nature() -> Self { /* Nature journal */ }
    pub fn science() -> Self { /* Science journal */ }
    pub fn plos() -> Self { /* PLOS standards */ }
    
    // Context-specific themes  
    pub fn presentation() -> Self { /* Large fonts, high contrast */ }
    pub fn poster() -> Self { /* Extra large fonts */ }
    pub fn web() -> Self { /* Screen-optimized */ }
    pub fn print() -> Self { /* High-DPI, embedded fonts */ }
    
    // Accessibility themes
    pub fn high_contrast() -> Self { /* WCAG AAA compliance */ }
    pub fn large_text() -> Self { /* Accessibility fonts */ }
    pub fn monochrome() -> Self { /* Grayscale safe */ }
}
```

## 📐 Mathematical Standards Implementation

### Font Size Relationships
- **Base size**: 10pt (scientific standard)
- **Title**: 12pt (1.2× base)
- **Axis labels**: 10pt (1.0× base)  
- **Tick labels**: 8.33pt (0.833× base)
- **Legend**: 9pt (0.9× base)

### Spacing Standards
- **Margins**: 12.5% of canvas (publication standard)
- **Padding**: 1.5× base font size
- **Line weights**: base_font / 12
- **Tick length**: base_font × 0.5

### Color Standards
- **Accessibility**: WCAG 2.1 AA compliance minimum
- **Print safety**: 15% minimum brightness difference in grayscale
- **Colorblind**: Support for 8% of male population (deuteranopia)

## 🚀 Implementation Priority

### High Priority (Immediate)
1. **DPI Fluent API**: `.dpi(u32).save("")` method for resolution control
2. **IEEE Publication Theme**: Most requested for scientific papers
3. **Enhanced colorblind palette**: Accessibility compliance
4. **Grid sophistication**: Scientific plotting standard
5. **DPI consistency**: Publication-quality exports with text/line ratio preservation

### Medium Priority (Next Month)  
1. **LaTeX integration**: Mathematical notation support
2. **Journal presets**: Nature, Science, PLOS themes
3. **Presentation themes**: Large font variants
4. **Advanced typography**: Kerning, ligatures

### Future Enhancements
1. **Interactive annotations**: Hover text, clickable legends
2. **3D plot foundations**: Scientific visualization expansion
3. **Animation support**: Time-series scientific data
4. **Statistical overlays**: Error bars, confidence intervals

## 📊 Success Metrics

- **Visual Quality**: Plots indistinguishable from matplotlib publication outputs
- **Accessibility**: 100% WCAG AA compliance for scientific palettes  
- **Performance**: <100ms render time for typical scientific plots
- **Adoption**: Positive feedback from scientific community beta users

## 🧪 Test-Driven Development (TDD) Requirements

### **MANDATORY TDD APPROACH**
All features MUST follow strict Test-Driven Development:

1. **Write tests FIRST** - Test must fail before implementation
2. **Red-Green-Refactor cycle** - No code without failing tests first
3. **Test outputs** - All plot images go to `test_output/` directory ONLY
4. **Example outputs** - Example program outputs go to `gallery/` directory

### **Directory Structure Requirements**

```
ruviz/
├── test_output/          # Test-generated images (.gitignored)
├── gallery/              # Example program outputs (committed)
├── tests/
│   ├── scientific_themes_test.rs
│   ├── typography_test.rs
│   ├── color_palette_test.rs
│   └── dpi_consistency_test.rs
└── examples/
    ├── ieee_publication.rs
    ├── nature_journal.rs
    └── presentation_theme.rs
```

### **TDD Implementation Pattern**

**Step 1: Write Failing Test**
```rust
#[test]
fn test_ieee_publication_theme() -> Result<(), Box<dyn std::error::Error>> {
    setup_test_output_dir()?;
    
    // This should fail initially - ieee() doesn't exist yet
    let theme = Theme::ieee();
    
    let plot = Plot::new()
        .theme(theme)
        .title("IEEE Publication Test")
        .line(&[1.0, 2.0, 3.0], &[1.0, 4.0, 9.0])
        .save("test_output/ieee_publication_test.png")?;
    
    // Validate IEEE requirements
    assert_eq!(theme.font_family, "Times New Roman");
    assert_eq!(theme.font_size, 8.0);
    assert_eq!(theme.line_width, 0.5);
    Ok(())
}
```

**Step 2: Run Test (Must Fail)**
```bash
cargo test test_ieee_publication_theme
# Should fail with "no method named `ieee`" or similar
```

**Step 3: Implement Minimal Code**
```rust
impl Theme {
    pub fn ieee() -> Self {
        Self {
            font_family: "Times New Roman".to_string(),
            font_size: 8.0,
            line_width: 0.5,
            // ... minimal implementation to pass test
        }
    }
}
```

**Step 4: Verify Test Passes**
```bash
cargo test test_ieee_publication_theme
# Should now pass and create test_output/ieee_publication_test.png
```

### **DPI API TDD Example**

**Step 1: Write Failing DPI Test First**
```rust
#[test]
fn test_dpi_fluent_api_basic() -> Result<(), Box<dyn std::error::Error>> {
    setup_test_output_dir()?;
    
    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![2.0, 4.0, 1.0, 3.0, 5.0];
    
    // Test the fluent API: .dpi(300).save()
    Plot::new()
        .title("DPI Test - 300 DPI")
        .xlabel("X Axis")
        .ylabel("Y Axis")
        .line(&x_data, &y_data)
        .dpi(300)  // This should fail initially - method doesn't exist yet
        .save("test_output/dpi_300_test.png")?;
    
    println!("✓ Saved: test_output/dpi_300_test.png at 300 DPI");
    Ok(())
}
```

**Step 2: Run Test (Must Fail)**
```bash
cargo test test_dpi_fluent_api_basic
# Expected failure: "no method named `dpi` found for struct `Plot`"
```

**Step 3: Implement Minimal DPI API**
```rust
impl Plot {
    pub fn dpi(mut self, dpi: u32) -> Self {
        // Validate minimum DPI (72 = 1 point standard)
        let validated_dpi = std::cmp::max(dpi, 72);
        
        // Set DPI in export configuration
        self.export_config.dpi = validated_dpi;
        
        // Calculate DPI scaling factors for consistency
        let scale = validated_dpi as f32 / 96.0; // 96 DPI baseline
        self.export_config.font_scale = scale;
        self.export_config.line_scale = scale;
        self.export_config.spacing_scale = scale;
        
        self
    }
}
```

**Step 4: Verify DPI Test Passes and Creates Output**
```bash
cargo test test_dpi_fluent_api_basic
# Should pass and create test_output/dpi_300_test.png with 300 DPI
```

### **Test Output Management**

**Required Test Setup Function:**
```rust
/// MANDATORY: All tests must use this setup
fn setup_test_output_dir() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("test_output")?;
    Ok(())
}
```

**File Naming Convention:**
```rust
// Tests: test_output/{feature}_{variant}_test.png
"test_output/ieee_theme_basic_test.png"
"test_output/colorblind_palette_validation_test.png"
"test_output/dpi_300_typography_test.png"

// Examples: gallery/{purpose}_{description}.png  
"gallery/ieee_publication_demo.png"
"gallery/nature_journal_figure.png"
"gallery/presentation_slide_chart.png"
```

### **Required Test Categories**

**1. Theme Tests** (`tests/scientific_themes_test.rs`)
```rust
#[test] fn test_ieee_theme() -> Result<(), Box<dyn std::error::Error>>
#[test] fn test_nature_theme() -> Result<(), Box<dyn std::error::Error>>
#[test] fn test_science_theme() -> Result<(), Box<dyn std::error::Error>>
#[test] fn test_presentation_theme() -> Result<(), Box<dyn std::error::Error>>
```

**2. Typography Tests** (`tests/typography_test.rs`)
```rust
#[test] fn test_font_hierarchy_ratios() -> Result<(), Box<dyn std::error::Error>>
#[test] fn test_dpi_scaling_consistency() -> Result<(), Box<dyn std::error::Error>>
#[test] fn test_mathematical_symbols() -> Result<(), Box<dyn std::error::Error>>
```

**3. Color Palette Tests** (`tests/color_palette_test.rs`)
```rust
#[test] fn test_wong_palette_accessibility() -> Result<(), Box<dyn std::error::Error>>
#[test] fn test_scientific_palette() -> Result<(), Box<dyn std::error::Error>>
#[test] fn test_colorblind_validation() -> Result<(), Box<dyn std::error::Error>>
```

**4. DPI Fluent API Tests** (`tests/dpi_api_test.rs`)
```rust
#[test] fn test_dpi_fluent_api_basic() -> Result<(), Box<dyn std::error::Error>>
#[test] fn test_ieee_publication_dpi() -> Result<(), Box<dyn std::error::Error>>
#[test] fn test_multiple_dpi_outputs() -> Result<(), Box<dyn std::error::Error>>
#[test] fn test_dpi_with_theme() -> Result<(), Box<dyn std::error::Error>>
#[test] fn test_dpi_validation() -> Result<(), Box<dyn std::error::Error>>
#[test] fn test_scientific_dpi_presets() -> Result<(), Box<dyn std::error::Error>>
```

**5. DPI Consistency Tests** (`tests/dpi_consistency_test.rs`)
```rust
#[test] fn test_96_dpi_baseline() -> Result<(), Box<dyn std::error::Error>>
#[test] fn test_150_dpi_scaling() -> Result<(), Box<dyn std::error::Error>>
#[test] fn test_300_dpi_print_quality() -> Result<(), Box<dyn std::error::Error>>
#[test] fn test_600_dpi_ieee_standard() -> Result<(), Box<dyn std::error::Error>>
#[test] fn test_text_line_ratio_consistency() -> Result<(), Box<dyn std::error::Error>>
```

### **Visual Regression Testing**

**Automated Comparison with Reference Images:**
```rust
#[test]
fn test_matches_matplotlib_reference() -> Result<(), Box<dyn std::error::Error>> {
    setup_test_output_dir()?;
    
    // Generate ruviz plot
    let plot = Plot::new()
        .theme(Theme::ieee())
        .line(&reference_data_x(), &reference_data_y())
        .save("test_output/matplotlib_comparison_test.png")?;
    
    // Compare with reference (stored in tests/references/)
    let reference_path = "tests/references/matplotlib_ieee_baseline.png";
    let generated_path = "test_output/matplotlib_comparison_test.png";
    
    assert_images_similar(reference_path, generated_path, 0.95)?; // 95% similarity
    Ok(())
}
```

### **Accessibility Testing Requirements**

**Colorblind Validation:**
```rust
#[test]
fn test_deuteranopia_safe_palette() -> Result<(), Box<dyn std::error::Error>> {
    setup_test_output_dir()?;
    
    let theme = Theme::colorblind_friendly();
    
    // Test with simulated deuteranopia
    let simulated_colors = simulate_deuteranopia(&theme.color_palette);
    assert!(validate_color_discrimination(&simulated_colors, 0.15)); // 15% minimum difference
    
    // Generate test image
    create_colorblind_test_plot(&theme, "test_output/deuteranopia_safe_test.png")?;
    Ok(())
}
```

### **Performance Testing with TDD**

**Rendering Performance Validation:**
```rust
#[test]
fn test_scientific_plot_performance() -> Result<(), Box<dyn std::error::Error>> {
    setup_test_output_dir()?;
    
    let large_dataset_x: Vec<f64> = (0..10000).map(|i| i as f64 * 0.01).collect();
    let large_dataset_y: Vec<f64> = large_dataset_x.iter().map(|x| x.sin()).collect();
    
    let start = std::time::Instant::now();
    
    Plot::new()
        .theme(Theme::ieee())
        .line(&large_dataset_x, &large_dataset_y)
        .save("test_output/performance_10k_points_test.png")?;
    
    let duration = start.elapsed();
    assert!(duration.as_millis() < 100, "Rendering took {}ms, expected <100ms", duration.as_millis());
    
    println!("✓ Performance test: {}ms for 10K points", duration.as_millis());
    Ok(())
}
```

### **Git Integration Requirements**

**Gitignore Configuration:**
```
# Test outputs (not committed)
test_output/*

# Gallery outputs (committed for documentation)  
# gallery/ files are committed to show examples
```

**Pre-commit Testing:**
```bash
# All tests must pass before commits
cargo test
cargo test --test scientific_themes_test
cargo test --test typography_test
cargo test --test color_palette_test
cargo test --test dpi_consistency_test
```

### **Documentation Testing**

**Example Programs Must Generate Gallery:**
```rust
// examples/ieee_publication.rs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("gallery")?;
    
    Plot::new()
        .theme(Theme::ieee())
        .title("IEEE Publication Example")
        .save("gallery/ieee_publication_demo.png")?; // Gallery, not test_output
    
    println!("Gallery example created: gallery/ieee_publication_demo.png");
    Ok(())
}
```

### **Testing Compliance Checklist**

Before implementing ANY feature:

- [ ] **Test written first** - Must fail initially
- [ ] **Output directory correct** - `test_output/` for tests, `gallery/` for examples  
- [ ] **Gitignore updated** - `test_output/*` ignored, `gallery/` committed
- [ ] **TDD cycle followed** - Red → Green → Refactor
- [ ] **Performance tested** - Timing assertions included
- [ ] **Accessibility tested** - Colorblind validation included
- [ ] **Visual regression** - Reference image comparison
- [ ] **Documentation example** - Gallery image generated

**Failure to follow TDD will result in rejected implementation.**

---

This comprehensive plan transforms ruviz from a basic plotting library into a publication-ready scientific visualization tool that matches matplotlib's professional standards while maintaining superior performance and modern Rust safety guarantees.