# Font Rendering Research for Rust Plotting Library

## Executive Summary

Based on comprehensive research into Rust font rendering approaches, here are the key findings and recommendations for improving text quality in a tiny-skia + fontdue setup for the ruviz plotting library:

**Key Recommendation**: Consider migrating from fontdue to either `piet-tiny-skia` (with cosmic-text) or implementing a custom solution using `cosmic-text` directly for professional-quality text rendering.

## Current Font Rendering Landscape in Rust

### Library Comparison Matrix

| Library | Performance | Quality | Features | Anti-aliasing | Shaping | Sub-pixel |
|---------|-------------|---------|----------|---------------|---------|-----------|
| fontdue | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | Basic | Grayscale | ❌ | ❌ |
| ab_glyph | ⭐⭐⭐⭐ | ⭐⭐⭐ | Basic | Grayscale | ❌ | ❌ |
| cosmic-text | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | Complete | Advanced | ✅ | ✅ |
| rusttype | ⭐⭐ | ⭐⭐⭐ | Basic | Grayscale | ❌ | ✅ |

## Detailed Analysis

### 1. Plotters Crate Approach

**Font Backend**: Uses `ab_glyph` feature for pure Rust TTF/OTF support
- Requires `plotters::style::register_font` before use
- BackendTextStyle trait abstracts text styling from backend implementation
- Architecture separates font handling from rendering backend

**Key Pattern**:
```rust
// Register font first
plotters::style::register_font("sans-serif", FontStyle::Normal, font_data)?;

// Use in plotting
chart.configure_axes()
    .y_desc("Values")
    .x_desc("Time")
    .label_area_size(40)
    .draw()?;
```

### 2. egui Font Rendering

**Current Implementation**: Uses `ab_glyph` for font rendering
- Provides decent grayscale antialiasing
- Known issues with small fonts due to limited pixel resolution
- Considering migration to `cosmic-text` for improved quality

**Quality Issues**:
- Monospace fonts lose fixed-width due to rounding during text layout
- Legibility suffers with small fonts
- Limited antialiasing compared to system font rendering

### 3. wgpu Text Rendering

**Available Options**:
- `wgpu-text`: Wrapper over glyph-brush for simple text rendering
- `wgpu_glyph`: Fast text renderer for wgpu
- Challenge: GPU text rendering requires texture atlases and complex caching

### 4. Font Library Performance Benchmarks

**ab_glyph vs rusttype**:
- TTF fonts: ab_glyph 1.5x faster than rusttype
- OTF fonts: ab_glyph 9x faster than rusttype
- Layout performance: ab_glyph significantly outperforms rusttype

**fontdue**: Claims to be "fastest font renderer in the world"
- Designed as rusttype/ab_glyph replacement
- Focus on raw rasterization speed
- No text shaping support

## Text Quality Issues and Solutions

### 1. Anti-aliasing Problems

**Current State**:
- Most Rust libraries provide basic grayscale antialiasing
- Subpixel rendering (ClearType) support is limited or missing
- Gamma correction often not implemented properly

**Solutions**:
- Use `cosmic-text` with `swash` rasterizer for better quality
- Implement proper gamma correction in compositing
- Consider supersampling for better small text rendering

### 2. tiny-skia Text Rendering Limitations

**Critical Finding**: tiny-skia does NOT support text rendering and it's not planned
- Maintainer considers it "absurdly complex task"
- Would require: font parser, text shaper, font database, high-quality rasterization

**Recommended Solution**: Use `piet-tiny-skia` wrapper
- Provides text rendering via `cosmic-text` integration
- Maintains tiny-skia's excellent 2D rendering performance
- More familiar API compared to raw tiny-skia

### 3. Text Shaping Importance

**Missing Feature**: fontdue and ab_glyph don't support text shaping
- Text shaping handles complex scripts, ligatures, kerning
- Critical for professional-quality text rendering
- Only `cosmic-text` provides comprehensive shaping support

## Specific Recommendations for ruviz

### Option 1: Migrate to piet-tiny-skia (Recommended)

```rust
use piet_tiny_skia::{RenderContext, Cache};
use tiny_skia::PixmapMut;

// Basic setup
let mut pixmap = PixmapMut::new(width, height)?;
let mut cache = Cache::new();
let mut render_ctx = RenderContext::new(&mut cache, &mut pixmap);

// Text rendering with proper quality
let text_layout = render_ctx
    .text()
    .new_text_layout("Plot Title")
    .font(FontFamily::SERIF, 16.0)
    .text_color(Color::BLACK)
    .build()?;

render_ctx.draw_text(&text_layout, (x, y));
```

**Benefits**:
- Professional-quality text rendering via cosmic-text
- Maintains tiny-skia performance for graphics
- Proper text shaping, kerning, and advanced typography
- Sub-pixel rendering support

### Option 2: Custom Integration with cosmic-text

```rust
use cosmic_text::{FontSystem, SwashCache, TextArea, TextLayout};
use tiny_skia::{Pixmap, Paint, Transform};

pub struct TextRenderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
}

impl TextRenderer {
    pub fn render_text(&mut self, pixmap: &mut Pixmap, text: &str, x: f32, y: f32) {
        let mut text_layout = TextLayout::new();
        text_layout.set_text(&mut self.font_system, text, attrs, shaping);
        
        // Render glyphs to pixmap using cosmic-text + tiny-skia integration
        for glyph in text_layout.glyphs() {
            // Convert glyph bitmap to tiny-skia format and blit
        }
    }
}
```

### Option 3: Enhanced fontdue Implementation

If staying with fontdue, implement these improvements:

```rust
pub struct FontRenderer {
    font: fontdue::Font,
    glyph_cache: HashMap<(char, u32), fontdue::Metrics>,
}

impl FontRenderer {
    pub fn render_with_gamma_correction(&self, text: &str) -> Vec<u8> {
        let mut buffer = Vec::new();
        
        for ch in text.chars() {
            let (metrics, bitmap) = self.font.rasterize(ch, size);
            
            // Apply gamma correction
            let gamma_corrected: Vec<u8> = bitmap
                .iter()
                .map(|&pixel| {
                    let linear = (pixel as f32 / 255.0).powf(2.2);
                    let corrected = linear.powf(1.0 / 2.2);
                    (corrected * 255.0) as u8
                })
                .collect();
                
            buffer.extend(gamma_corrected);
        }
        
        buffer
    }
    
    pub fn apply_kerning(&self, text: &str) -> Vec<f32> {
        // Implement kerning using font metrics
        let mut positions = Vec::new();
        let mut x = 0.0;
        
        let chars: Vec<char> = text.chars().collect();
        for i in 0..chars.len() {
            positions.push(x);
            
            let ch = chars[i];
            let (metrics, _) = self.font.rasterize(ch, size);
            x += metrics.advance_width;
            
            // Apply kerning if next character exists
            if let Some(&next_ch) = chars.get(i + 1) {
                x += self.get_kerning(ch, next_ch);
            }
        }
        
        positions
    }
}
```

## Implementation Best Practices

### 1. Text Quality Optimization

```rust
pub struct TextConfig {
    pub gamma: f32,           // 2.2 for sRGB
    pub contrast: f32,        // 1.0 default
    pub use_subpixel: bool,   // ClearType-style rendering
    pub hinting: HintingLevel, // None, Light, Normal, Full
}

impl TextRenderer {
    pub fn render_high_quality(&self, config: &TextConfig) {
        // 1. Render at higher resolution
        let scale_factor = 2.0;
        let oversized_bitmap = self.rasterize_scaled(text, size * scale_factor);
        
        // 2. Apply gamma correction
        let gamma_corrected = self.apply_gamma(oversized_bitmap, config.gamma);
        
        // 3. Downsample with filtering
        let final_bitmap = self.downsample_with_filter(gamma_corrected, scale_factor);
        
        // 4. Composite onto target
        self.alpha_blend(final_bitmap, target_pixmap);
    }
}
```

### 2. Font Loading and Management

```rust
pub struct FontManager {
    system_fonts: HashMap<String, Vec<u8>>,
    fallback_chain: Vec<String>,
}

impl FontManager {
    pub fn new() -> Result<Self, FontError> {
        let mut manager = Self {
            system_fonts: HashMap::new(),
            fallback_chain: vec![
                "Arial".to_string(),
                "Liberation Sans".to_string(),
                "Noto Sans".to_string(),
            ],
        };
        
        manager.load_system_fonts()?;
        Ok(manager)
    }
    
    fn load_system_fonts(&mut self) -> Result<(), FontError> {
        // Load system fonts based on platform
        #[cfg(target_os = "windows")]
        self.load_windows_fonts()?;
        
        #[cfg(target_os = "macos")]
        self.load_macos_fonts()?;
        
        #[cfg(target_os = "linux")]
        self.load_linux_fonts()?;
        
        Ok(())
    }
}
```

### 3. Performance Optimization

```rust
pub struct GlyphCache {
    cache: HashMap<GlyphKey, CachedGlyph>,
    texture_atlas: TextureAtlas,
}

#[derive(Hash, PartialEq, Eq)]
struct GlyphKey {
    font_id: u32,
    character: char,
    size: u32,
    subpixel_offset: u8, // 0-3 for subpixel positioning
}

impl GlyphCache {
    pub fn get_or_render(&mut self, key: GlyphKey) -> &CachedGlyph {
        self.cache.entry(key).or_insert_with(|| {
            let bitmap = self.render_glyph(&key);
            let atlas_position = self.texture_atlas.allocate(bitmap.width, bitmap.height);
            
            CachedGlyph {
                bitmap,
                atlas_position,
                metrics: self.get_metrics(&key),
            }
        })
    }
}
```

## Common Text Rendering Mistakes to Avoid

1. **Ignoring Gamma Correction**: Always apply proper gamma correction for better contrast
2. **No Subpixel Positioning**: Results in uneven character spacing
3. **Missing Kerning**: Professional text requires proper kerning
4. **Poor Font Fallback**: Always implement font fallback chains
5. **No Text Shaping**: Required for complex scripts and ligatures
6. **Incorrect Alpha Blending**: Use premultiplied alpha for better quality
7. **No Hinting**: Small text becomes unreadable without proper hinting

## Recommended Dependencies for ruviz

```toml
[dependencies]
# Option 1: piet-tiny-skia approach
piet-tiny-skia = "0.1"
cosmic-text = "0.12"

# Option 2: Direct cosmic-text integration
cosmic-text = "0.12"
swash = "0.1"
fontdb = "0.16"

# Option 3: Enhanced fontdue (current approach)
fontdue = "0.7"
# Additional crates for kerning/advanced features
rustybuzz = "0.13"  # For text shaping
ttf-parser = "0.20" # For advanced font metrics
```

## Conclusion

For professional-quality text rendering in the ruviz plotting library, the recommended approach is to migrate from the current fontdue setup to either:

1. **piet-tiny-skia** (easiest migration): Provides high-quality text via cosmic-text while maintaining tiny-skia performance
2. **Direct cosmic-text integration** (best quality): Maximum control over text rendering quality and features

The current fontdue approach, while fast, lacks the text shaping, advanced antialiasing, and professional typography features needed for publication-quality plots. The migration effort would be significant but would result in substantially better text quality that matches or exceeds matplotlib's text rendering capabilities.

## Next Steps

1. Create proof-of-concept with piet-tiny-skia for text rendering
2. Benchmark performance impact compared to current fontdue approach
3. Implement font loading and management system
4. Add proper gamma correction and subpixel rendering support
5. Create comprehensive text rendering tests including visual regression testing