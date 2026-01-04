//! Unified text rendering system using cosmic-text
//!
//! This module provides a single, coherent text rendering system that leverages
//! cosmic-text for cross-platform font discovery, international script support,
//! and high-quality text shaping.
//!
//! # Architecture
//!
//! - `FontSystem` singleton: Discovers and caches system fonts (created once)
//! - `SwashCache` singleton: Caches rasterized glyphs for performance
//! - `TextRenderer`: Main interface for rendering text to pixmaps
//!
//! # Example
//!
//! ```rust,ignore
//! use ruviz::render::{TextRenderer, FontConfig, FontFamily, Color};
//!
//! let renderer = TextRenderer::new();
//! let config = FontConfig::new(FontFamily::SansSerif, 14.0);
//! renderer.render_text(&mut pixmap, "Hello 日本語", 10.0, 50.0, &config, Color::BLACK)?;
//! ```

use std::sync::{Mutex, OnceLock};

use cosmic_text::{
    Attrs, Buffer, Color as CosmicColor, Family, FontSystem, Metrics, Shaping,
    Style as CosmicStyle, SwashCache, Weight as CosmicWeight,
};
use tiny_skia::{Pixmap, PremultipliedColorU8};

use crate::core::error::{PlottingError, Result};
use crate::render::Color;

// =============================================================================
// Global Singletons
// =============================================================================

/// Global FontSystem singleton - discovers and caches system fonts
static FONT_SYSTEM: OnceLock<Mutex<FontSystem>> = OnceLock::new();

/// Global SwashCache singleton - caches rasterized glyphs
static SWASH_CACHE: OnceLock<Mutex<SwashCache>> = OnceLock::new();

/// Get or initialize the global FontSystem
///
/// The FontSystem is created lazily on first access and reused for all
/// subsequent text rendering operations. This avoids redundant font
/// discovery which can be expensive.
pub fn get_font_system() -> &'static Mutex<FontSystem> {
    FONT_SYSTEM.get_or_init(|| {
        log::debug!("Initializing global FontSystem with system font discovery");
        Mutex::new(FontSystem::new())
    })
}

/// Get or initialize the global SwashCache
///
/// The SwashCache stores rasterized glyphs to avoid re-rasterizing the same
/// glyphs repeatedly. It implements LRU eviction when memory limits are reached.
pub fn get_swash_cache() -> &'static Mutex<SwashCache> {
    SWASH_CACHE.get_or_init(|| {
        log::debug!("Initializing global SwashCache for glyph caching");
        Mutex::new(SwashCache::new())
    })
}

/// Initialize the text rendering system
///
/// This function eagerly initializes the FontSystem and SwashCache singletons.
/// Calling this at application startup can avoid latency on first text render.
///
/// # Example
///
/// ```rust,ignore
/// // Call at startup to pre-warm font discovery
/// ruviz::render::text::initialize_text_system();
/// ```
pub fn initialize_text_system() {
    let _ = get_font_system();
    let _ = get_swash_cache();
    log::info!("Text rendering system initialized");
}

// =============================================================================
// Font Configuration
// =============================================================================

/// Font family specification
///
/// Represents the font family to use for text rendering. The system will
/// automatically fall back to available fonts if the requested family
/// is not available.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum FontFamily {
    /// System default serif font (Times, Georgia, etc.)
    Serif,
    /// System default sans-serif font (Arial, Helvetica, etc.)
    #[default]
    SansSerif,
    /// System default monospace font (Courier, Consolas, etc.)
    Monospace,
    /// Cursive/script font family
    Cursive,
    /// Fantasy/decorative font family
    Fantasy,
    /// Specific font family by name
    Name(String),
}

impl FontFamily {
    /// Convert to cosmic-text Family type
    pub fn to_cosmic_family(&self) -> Family<'_> {
        match self {
            FontFamily::Serif => Family::Serif,
            FontFamily::SansSerif => Family::SansSerif,
            FontFamily::Monospace => Family::Monospace,
            FontFamily::Cursive => Family::Cursive,
            FontFamily::Fantasy => Family::Fantasy,
            FontFamily::Name(name) => Family::Name(name),
        }
    }

    /// Get a CSS-compatible font family string
    pub fn as_str(&self) -> &str {
        match self {
            FontFamily::Serif => "serif",
            FontFamily::SansSerif => "sans-serif",
            FontFamily::Monospace => "monospace",
            FontFamily::Cursive => "cursive",
            FontFamily::Fantasy => "fantasy",
            FontFamily::Name(name) => name,
        }
    }
}

impl From<&str> for FontFamily {
    fn from(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "serif" => FontFamily::Serif,
            "sans-serif" | "sans" => FontFamily::SansSerif,
            "monospace" | "mono" => FontFamily::Monospace,
            "cursive" => FontFamily::Cursive,
            "fantasy" => FontFamily::Fantasy,
            _ => FontFamily::Name(name.to_string()),
        }
    }
}

impl From<String> for FontFamily {
    fn from(name: String) -> Self {
        FontFamily::from(name.as_str())
    }
}

/// Font weight specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontWeight {
    /// Thin weight (100)
    Thin,
    /// Extra-light weight (200)
    ExtraLight,
    /// Light weight (300)
    Light,
    /// Normal/regular weight (400)
    #[default]
    Normal,
    /// Medium weight (500)
    Medium,
    /// Semi-bold weight (600)
    SemiBold,
    /// Bold weight (700)
    Bold,
    /// Extra-bold weight (800)
    ExtraBold,
    /// Black/heavy weight (900)
    Black,
}

impl FontWeight {
    /// Convert to cosmic-text Weight type
    pub fn to_cosmic_weight(self) -> CosmicWeight {
        match self {
            FontWeight::Thin => CosmicWeight::THIN,
            FontWeight::ExtraLight => CosmicWeight::EXTRA_LIGHT,
            FontWeight::Light => CosmicWeight::LIGHT,
            FontWeight::Normal => CosmicWeight::NORMAL,
            FontWeight::Medium => CosmicWeight::MEDIUM,
            FontWeight::SemiBold => CosmicWeight::SEMIBOLD,
            FontWeight::Bold => CosmicWeight::BOLD,
            FontWeight::ExtraBold => CosmicWeight::EXTRA_BOLD,
            FontWeight::Black => CosmicWeight::BLACK,
        }
    }
}

/// Font style specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontStyle {
    /// Normal upright style
    #[default]
    Normal,
    /// Italic style
    Italic,
    /// Oblique style (slanted)
    Oblique,
}

impl FontStyle {
    /// Convert to cosmic-text Style type
    pub fn to_cosmic_style(self) -> CosmicStyle {
        match self {
            FontStyle::Normal => CosmicStyle::Normal,
            FontStyle::Italic => CosmicStyle::Italic,
            FontStyle::Oblique => CosmicStyle::Oblique,
        }
    }
}

/// Complete font configuration for text rendering
#[derive(Debug, Clone)]
pub struct FontConfig {
    /// Font family (e.g., SansSerif, Serif, or specific name)
    pub family: FontFamily,
    /// Font size in pixels
    pub size: f32,
    /// Font weight (Normal, Bold, etc.)
    pub weight: FontWeight,
    /// Font style (Normal, Italic, Oblique)
    pub style: FontStyle,
}

impl FontConfig {
    /// Create a new font configuration with family and size
    pub fn new(family: FontFamily, size: f32) -> Self {
        Self {
            family,
            size,
            weight: FontWeight::Normal,
            style: FontStyle::Normal,
        }
    }

    /// Set the font weight
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Set the font to bold
    pub fn bold(mut self) -> Self {
        self.weight = FontWeight::Bold;
        self
    }

    /// Set the font style
    pub fn style(mut self, style: FontStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the font to italic
    pub fn italic(mut self) -> Self {
        self.style = FontStyle::Italic;
        self
    }

    /// Set the font size
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Convert to cosmic-text Attrs for text shaping
    pub fn to_cosmic_attrs(&self) -> Attrs<'_> {
        Attrs::new()
            .family(self.family.to_cosmic_family())
            .weight(self.weight.to_cosmic_weight())
            .style(self.style.to_cosmic_style())
    }
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: FontFamily::default(),
            size: 12.0,
            weight: FontWeight::default(),
            style: FontStyle::default(),
        }
    }
}

// =============================================================================
// Text Renderer
// =============================================================================

/// High-quality text renderer using cosmic-text
///
/// TextRenderer provides a unified interface for rendering text with:
/// - Automatic font discovery on all platforms
/// - International script support (CJK, Arabic, Hebrew, etc.)
/// - Font fallback for missing glyphs
/// - Glyph caching for performance
///
/// # Thread Safety
///
/// TextRenderer uses global singletons for FontSystem and SwashCache,
/// which are protected by Mutex. Multiple TextRenderer instances can
/// be used safely from multiple threads.
pub struct TextRenderer;

impl TextRenderer {
    /// Create a new TextRenderer
    ///
    /// This is a lightweight operation as the heavy lifting (font discovery)
    /// is done lazily by the global FontSystem singleton.
    pub fn new() -> Self {
        Self
    }

    /// Render text to a pixmap at the specified position
    ///
    /// # Arguments
    ///
    /// * `pixmap` - Target pixmap to render into
    /// * `text` - Text string to render (supports Unicode)
    /// * `x` - X coordinate for text origin
    /// * `y` - Y coordinate for text baseline
    /// * `config` - Font configuration (family, size, weight, style)
    /// * `color` - Text color
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if rendering fails.
    pub fn render_text(
        &self,
        pixmap: &mut Pixmap,
        text: &str,
        x: f32,
        y: f32,
        config: &FontConfig,
        color: Color,
    ) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        let mut font_system = get_font_system()
            .lock()
            .map_err(|e| PlottingError::RenderError(format!("Failed to lock FontSystem: {}", e)))?;

        let mut swash_cache = get_swash_cache()
            .lock()
            .map_err(|e| PlottingError::RenderError(format!("Failed to lock SwashCache: {}", e)))?;

        // Create metrics for the buffer
        let metrics = Metrics::new(config.size, config.size * 1.2);

        // Create buffer for text layout
        let mut buffer = Buffer::new(&mut font_system, metrics);

        // Calculate buffer dimensions
        let buffer_width = (text.len() as f32 * config.size * 2.0).max(800.0);
        let buffer_height = (config.size * 4.0).max(100.0);
        buffer.set_size(&mut font_system, Some(buffer_width), Some(buffer_height));

        // Set text with font attributes
        let attrs = config.to_cosmic_attrs();
        buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);

        // Shape the text
        buffer.shape_until_scroll(&mut font_system, false);

        // Convert color
        let cosmic_color = CosmicColor::rgba(color.r, color.g, color.b, color.a);

        // Render each glyph
        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let physical_glyph = glyph.physical((x, y), 1.0);

                swash_cache.with_pixels(
                    &mut font_system,
                    physical_glyph.cache_key,
                    cosmic_color,
                    |glyph_x, glyph_y, glyph_color| {
                        let pixel_x = physical_glyph.x + glyph_x;
                        let pixel_y = physical_glyph.y + glyph_y;

                        if pixel_x >= 0
                            && pixel_y >= 0
                            && (pixel_x as u32) < pixmap.width()
                            && (pixel_y as u32) < pixmap.height()
                        {
                            let alpha = glyph_color.a();
                            if alpha > 0 {
                                let idx =
                                    (pixel_y as u32 * pixmap.width() + pixel_x as u32) as usize;
                                let background = pixmap.pixels()[idx];

                                // Alpha blend
                                let alpha_f = alpha as f32 / 255.0;
                                let inv_alpha = 1.0 - alpha_f;

                                let blended_r = (glyph_color.r() as f32 * alpha_f
                                    + background.red() as f32 * inv_alpha)
                                    as u8;
                                let blended_g = (glyph_color.g() as f32 * alpha_f
                                    + background.green() as f32 * inv_alpha)
                                    as u8;
                                let blended_b = (glyph_color.b() as f32 * alpha_f
                                    + background.blue() as f32 * inv_alpha)
                                    as u8;

                                if let Some(blended) = PremultipliedColorU8::from_rgba(
                                    blended_r, blended_g, blended_b, 255,
                                ) {
                                    pixmap.pixels_mut()[idx] = blended;
                                }
                            }
                        }
                    },
                );
            }
        }

        Ok(())
    }

    /// Render text centered horizontally at the given position
    ///
    /// # Arguments
    ///
    /// * `pixmap` - Target pixmap to render into
    /// * `text` - Text string to render
    /// * `center_x` - X coordinate for horizontal center
    /// * `y` - Y coordinate for text baseline
    /// * `config` - Font configuration
    /// * `color` - Text color
    pub fn render_text_centered(
        &self,
        pixmap: &mut Pixmap,
        text: &str,
        center_x: f32,
        y: f32,
        config: &FontConfig,
        color: Color,
    ) -> Result<()> {
        let (width, _) = self.measure_text(text, config)?;
        let x = center_x - width / 2.0;
        self.render_text(pixmap, text, x, y, config, color)
    }

    /// Render text rotated 90 degrees counterclockwise
    ///
    /// Useful for Y-axis labels in plots.
    ///
    /// # Arguments
    ///
    /// * `pixmap` - Target pixmap to render into
    /// * `text` - Text string to render
    /// * `x` - X coordinate for rotation center
    /// * `y` - Y coordinate for rotation center
    /// * `config` - Font configuration
    /// * `color` - Text color
    pub fn render_text_rotated(
        &self,
        pixmap: &mut Pixmap,
        text: &str,
        x: f32,
        y: f32,
        config: &FontConfig,
        color: Color,
    ) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        let mut font_system = get_font_system()
            .lock()
            .map_err(|e| PlottingError::RenderError(format!("Failed to lock FontSystem: {}", e)))?;

        let mut swash_cache = get_swash_cache()
            .lock()
            .map_err(|e| PlottingError::RenderError(format!("Failed to lock SwashCache: {}", e)))?;

        let metrics = Metrics::new(config.size, config.size * 1.2);
        let mut buffer = Buffer::new(&mut font_system, metrics);

        let dpi_scale = config.size / 12.0;
        let buffer_width = (text.len() as f32 * config.size * 2.5 * dpi_scale).max(800.0);
        let buffer_height = (config.size * 6.0 * dpi_scale).max(180.0);

        buffer.set_size(&mut font_system, Some(buffer_width), Some(buffer_height));

        let attrs = config.to_cosmic_attrs();
        buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut font_system, false);

        // Calculate text bounds
        let mut max_x = 0.0f32;
        let mut max_y = 0.0f32;
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;

        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let physical_glyph = glyph.physical((0., 0.), 1.0);
                let gx = physical_glyph.x as f32;
                let gy = physical_glyph.y as f32;

                min_x = min_x.min(gx);
                min_y = min_y.min(gy);
                max_x = max_x.max(gx + 20.0);
                max_y = max_y.max(gy + run.line_height);
            }
        }

        if min_x == f32::MAX {
            min_x = 0.0;
            min_y = 0.0;
            max_x = text.len() as f32 * config.size * 0.6;
            max_y = config.size * 1.2;
        }

        let padding = 30.0 * dpi_scale;
        min_x -= padding;
        min_y -= padding;
        max_x += padding;
        max_y += padding;

        let text_width = (max_x - min_x).ceil().max(1.0) as u32;
        let text_height = (max_y - min_y).ceil().max(1.0) as u32;

        // Create temporary pixmap for horizontal text
        let mut temp_pixmap = Pixmap::new(text_width, text_height).ok_or_else(|| {
            PlottingError::RenderError("Failed to create temp pixmap".to_string())
        })?;
        temp_pixmap.fill(tiny_skia::Color::TRANSPARENT);

        let cosmic_color = CosmicColor::rgba(color.r, color.g, color.b, color.a);

        // Render to temporary pixmap
        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let physical_glyph = glyph.physical((0., 0.), 1.0);

                swash_cache.with_pixels(
                    &mut font_system,
                    physical_glyph.cache_key,
                    cosmic_color,
                    |dx, dy, glyph_color| {
                        let glyph_x_calc = (physical_glyph.x as f32 - min_x) as i32 + dx;
                        let glyph_y_calc = (physical_glyph.y as f32 - min_y) as i32 + dy;

                        if glyph_x_calc < 0 || glyph_y_calc < 0 {
                            return;
                        }

                        let glyph_x = glyph_x_calc as u32;
                        let glyph_y = glyph_y_calc as u32;

                        if glyph_x < text_width && glyph_y < text_height {
                            let idx = glyph_y as usize * text_width as usize + glyph_x as usize;
                            if idx < temp_pixmap.pixels().len() {
                                if let Some(rgba_pixel) = PremultipliedColorU8::from_rgba(
                                    glyph_color.r(),
                                    glyph_color.g(),
                                    glyph_color.b(),
                                    glyph_color.a(),
                                ) {
                                    if rgba_pixel.alpha() > 0 {
                                        temp_pixmap.pixels_mut()[idx] = rgba_pixel;
                                    }
                                }
                            }
                        }
                    },
                );
            }
        }

        // Apply 90° counterclockwise rotation
        let rotated_width = text_height;
        let rotated_height = text_width;

        let mut rotated_pixmap = Pixmap::new(rotated_width, rotated_height).ok_or_else(|| {
            PlottingError::RenderError("Failed to create rotated pixmap".to_string())
        })?;
        rotated_pixmap.fill(tiny_skia::Color::TRANSPARENT);

        for orig_y in 0..text_height {
            for orig_x in 0..text_width {
                let src_pixel =
                    temp_pixmap.pixels()[orig_y as usize * text_width as usize + orig_x as usize];
                if src_pixel.alpha() > 0 {
                    let new_x = orig_y;
                    let new_y = text_width - 1 - orig_x;

                    if new_x < rotated_width && new_y < rotated_height {
                        let new_idx = new_y as usize * rotated_width as usize + new_x as usize;
                        if new_idx < rotated_pixmap.pixels().len() {
                            rotated_pixmap.pixels_mut()[new_idx] = src_pixel;
                        }
                    }
                }
            }
        }

        // Draw rotated text to main pixmap
        let canvas_width = pixmap.width();
        let canvas_height = pixmap.height();

        let margin_x = (rotated_width / 2) as i32;
        let margin_y = (rotated_height / 2) as i32;

        let target_x = (x as i32 - margin_x)
            .max(0)
            .min((canvas_width as i32) - (rotated_width as i32));
        let target_y = (y as i32 - margin_y)
            .max(0)
            .min((canvas_height as i32) - (rotated_height as i32));

        for py in 0..rotated_height {
            for px in 0..rotated_width {
                let src_pixel =
                    rotated_pixmap.pixels()[py as usize * rotated_width as usize + px as usize];
                if src_pixel.alpha() > 0 {
                    let final_x = target_x + px as i32;
                    let final_y = target_y + py as i32;

                    if final_x >= 0
                        && final_y >= 0
                        && final_x < canvas_width as i32
                        && final_y < canvas_height as i32
                    {
                        let pixmap_idx = (final_y as u32 * canvas_width + final_x as u32) as usize;
                        if pixmap_idx < pixmap.pixels().len() {
                            pixmap.pixels_mut()[pixmap_idx] = src_pixel;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Measure text dimensions for layout calculations
    ///
    /// # Arguments
    ///
    /// * `text` - Text string to measure
    /// * `config` - Font configuration
    ///
    /// # Returns
    ///
    /// Returns `(width, height)` in pixels.
    pub fn measure_text(&self, text: &str, config: &FontConfig) -> Result<(f32, f32)> {
        if text.is_empty() {
            return Ok((0.0, config.size));
        }

        let mut font_system = get_font_system()
            .lock()
            .map_err(|e| PlottingError::RenderError(format!("Failed to lock FontSystem: {}", e)))?;

        let metrics = Metrics::new(config.size, config.size * 1.2);
        let mut buffer = Buffer::new(&mut font_system, metrics);

        let buffer_width = (text.len() as f32 * config.size * 2.0).max(800.0);
        let buffer_height = (config.size * 4.0).max(100.0);
        buffer.set_size(&mut font_system, Some(buffer_width), Some(buffer_height));

        let attrs = config.to_cosmic_attrs();
        buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut font_system, false);

        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;

        for run in buffer.layout_runs() {
            width = width.max(run.line_w);
            height = height.max(run.line_height);
        }

        Ok((width, height))
    }
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_family_from_str() {
        assert_eq!(FontFamily::from("serif"), FontFamily::Serif);
        assert_eq!(FontFamily::from("sans-serif"), FontFamily::SansSerif);
        assert_eq!(FontFamily::from("sans"), FontFamily::SansSerif);
        assert_eq!(FontFamily::from("monospace"), FontFamily::Monospace);
        assert_eq!(FontFamily::from("mono"), FontFamily::Monospace);
        assert_eq!(
            FontFamily::from("Arial"),
            FontFamily::Name("Arial".to_string())
        );
    }

    #[test]
    fn test_font_family_as_str() {
        assert_eq!(FontFamily::Serif.as_str(), "serif");
        assert_eq!(FontFamily::SansSerif.as_str(), "sans-serif");
        assert_eq!(FontFamily::Monospace.as_str(), "monospace");
        assert_eq!(FontFamily::Name("Roboto".to_string()).as_str(), "Roboto");
    }

    #[test]
    fn test_font_config_builder() {
        let config = FontConfig::new(FontFamily::SansSerif, 14.0).bold().italic();

        assert_eq!(config.family, FontFamily::SansSerif);
        assert_eq!(config.size, 14.0);
        assert_eq!(config.weight, FontWeight::Bold);
        assert_eq!(config.style, FontStyle::Italic);
    }

    #[test]
    fn test_font_config_to_cosmic_attrs() {
        let config = FontConfig::new(FontFamily::Serif, 16.0).bold();
        let attrs = config.to_cosmic_attrs();
        // Just verify it doesn't panic - cosmic-text attrs are opaque
        let _ = attrs;
    }

    #[test]
    fn test_font_weight_to_cosmic() {
        // Verify all weights convert without panic
        let weights = [
            FontWeight::Thin,
            FontWeight::ExtraLight,
            FontWeight::Light,
            FontWeight::Normal,
            FontWeight::Medium,
            FontWeight::SemiBold,
            FontWeight::Bold,
            FontWeight::ExtraBold,
            FontWeight::Black,
        ];

        for weight in weights {
            let _ = weight.to_cosmic_weight();
        }
    }

    #[test]
    fn test_font_style_to_cosmic() {
        assert!(matches!(
            FontStyle::Normal.to_cosmic_style(),
            CosmicStyle::Normal
        ));
        assert!(matches!(
            FontStyle::Italic.to_cosmic_style(),
            CosmicStyle::Italic
        ));
        assert!(matches!(
            FontStyle::Oblique.to_cosmic_style(),
            CosmicStyle::Oblique
        ));
    }

    #[test]
    fn test_text_renderer_creation() {
        let renderer = TextRenderer::new();
        let _ = renderer; // Just verify it creates without panic
    }

    #[test]
    fn test_singleton_initialization() {
        // First access initializes
        let fs1 = get_font_system();
        let sc1 = get_swash_cache();

        // Second access returns same instance
        let fs2 = get_font_system();
        let sc2 = get_swash_cache();

        assert!(std::ptr::eq(fs1, fs2));
        assert!(std::ptr::eq(sc1, sc2));
    }

    #[test]
    fn test_measure_text() {
        let renderer = TextRenderer::new();
        let config = FontConfig::new(FontFamily::SansSerif, 12.0);

        // Empty string
        let (w, h) = renderer.measure_text("", &config).unwrap();
        assert_eq!(w, 0.0);
        assert_eq!(h, 12.0);

        // Non-empty string should have positive width
        let (w, _h) = renderer.measure_text("Hello", &config).unwrap();
        assert!(w > 0.0);
    }
}
