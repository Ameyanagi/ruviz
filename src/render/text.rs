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

use std::sync::{Mutex, MutexGuard, OnceLock};

use cosmic_text::{
    Attrs, Buffer, CacheKey, Color as CosmicColor, Family, FontSystem, Metrics, Shaping,
    Style as CosmicStyle, SwashCache, SwashContent, Weight as CosmicWeight,
};
use swash::scale::Source as SwashSource;
use tiny_skia::{Pixmap, PixmapMut, PremultipliedColorU8};

use crate::core::error::{PlottingError, Result};
use crate::render::text_anchor::TextPlacementMetrics;
use crate::render::{
    Color,
    color::{premultiply_rgba, scale_premultiplied_rgba, source_over_premultiplied_rgba},
};

const MAX_TEXT_RASTER_DIMENSION: u32 = 8_192;
const MAX_TEXT_RASTER_BYTES: usize = 128 * 1024 * 1024;

trait PixmapTarget {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn pixels_mut(&mut self) -> &mut [PremultipliedColorU8];
}

impl PixmapTarget for Pixmap {
    fn width(&self) -> u32 {
        self.width()
    }

    fn height(&self) -> u32 {
        self.height()
    }

    fn pixels_mut(&mut self) -> &mut [PremultipliedColorU8] {
        self.pixels_mut()
    }
}

impl PixmapTarget for PixmapMut<'_> {
    fn width(&self) -> u32 {
        self.width()
    }

    fn height(&self) -> u32 {
        self.height()
    }

    fn pixels_mut(&mut self) -> &mut [PremultipliedColorU8] {
        self.pixels_mut()
    }
}

fn validate_text_raster_size(width: u32, height: u32, context: &str) -> Result<()> {
    if width > MAX_TEXT_RASTER_DIMENSION || height > MAX_TEXT_RASTER_DIMENSION {
        return Err(PlottingError::PerformanceLimit {
            limit_type: format!("{context} raster dimension"),
            actual: width.max(height) as usize,
            maximum: MAX_TEXT_RASTER_DIMENSION as usize,
        });
    }

    let bytes = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| PlottingError::PerformanceLimit {
            limit_type: format!("{context} raster bytes"),
            actual: usize::MAX,
            maximum: MAX_TEXT_RASTER_BYTES,
        })?;

    if bytes > MAX_TEXT_RASTER_BYTES {
        return Err(PlottingError::PerformanceLimit {
            limit_type: format!("{context} raster bytes"),
            actual: bytes,
            maximum: MAX_TEXT_RASTER_BYTES,
        });
    }

    Ok(())
}

#[derive(Clone, Copy)]
enum GlyphPixel {
    Straight([u8; 4]),
    Premultiplied([u8; 4]),
}

fn premultiplied_glyph_pixel(
    glyph_pixel: GlyphPixel,
    requested_alpha: u8,
) -> Option<PremultipliedColorU8> {
    let [red, green, blue, alpha] = match glyph_pixel {
        GlyphPixel::Straight([red, green, blue, alpha]) => {
            let effective_alpha = crate::render::color::mul_div_255(alpha, requested_alpha);
            premultiply_rgba(red, green, blue, effective_alpha)
        }
        GlyphPixel::Premultiplied(rgba) => scale_premultiplied_rgba(rgba, requested_alpha),
    };

    if alpha == 0 {
        return None;
    }

    PremultipliedColorU8::from_rgba(red, green, blue, alpha)
}

pub(crate) fn with_premultiplied_glyph_pixels<F: FnMut(i32, i32, PremultipliedColorU8)>(
    swash_cache: &mut SwashCache,
    font_system: &mut FontSystem,
    cache_key: CacheKey,
    color: Color,
    mut callback: F,
) {
    let Some(image) = swash_cache.get_image(font_system, cache_key) else {
        return;
    };

    let origin_x = image.placement.left;
    let origin_y = -image.placement.top;
    match image.content {
        SwashContent::Mask => {
            for (index, coverage) in image.data.iter().copied().enumerate() {
                let x = index as u32 % image.placement.width;
                let y = index as u32 / image.placement.width;
                let glyph_pixel = GlyphPixel::Straight([color.r, color.g, color.b, coverage]);
                if let Some(source) = premultiplied_glyph_pixel(glyph_pixel, color.a) {
                    callback(origin_x + x as i32, origin_y + y as i32, source);
                }
            }
        }
        SwashContent::Color => {
            let is_premultiplied = matches!(image.source, SwashSource::ColorOutline(_));
            for (index, rgba) in image.data.chunks_exact(4).enumerate() {
                let x = index as u32 % image.placement.width;
                let y = index as u32 / image.placement.width;
                let rgba = [rgba[0], rgba[1], rgba[2], rgba[3]];
                let glyph_pixel = if is_premultiplied {
                    GlyphPixel::Premultiplied(rgba)
                } else {
                    GlyphPixel::Straight(rgba)
                };
                if let Some(source) = premultiplied_glyph_pixel(glyph_pixel, color.a) {
                    callback(origin_x + x as i32, origin_y + y as i32, source);
                }
            }
        }
        SwashContent::SubpixelMask => {
            log::warn!("Subpixel glyph masks are not supported");
        }
    }
}

pub(crate) fn blend_premultiplied_source_over(
    destination: &mut PremultipliedColorU8,
    source: PremultipliedColorU8,
) {
    let [red, green, blue, alpha] = source_over_premultiplied_rgba(
        [
            destination.red(),
            destination.green(),
            destination.blue(),
            destination.alpha(),
        ],
        [source.red(), source.green(), source.blue(), source.alpha()],
    );

    if let Some(blended) = PremultipliedColorU8::from_rgba(red, green, blue, alpha) {
        *destination = blended;
    }
}

fn is_renderable_text(text: &str) -> bool {
    !text.trim().is_empty()
}

fn estimate_text_metrics(text: &str, config: &FontConfig) -> TextPlacementMetrics {
    let char_count = text.chars().count() as f32;
    let height = (config.size * 1.2).max(config.size);
    let width = char_count * config.size * 0.6;
    TextPlacementMetrics::new(width, height, config.size)
}

#[derive(Debug, Clone, Copy)]
struct InkBoxMetrics {
    width: f32,
    height: f32,
    min_y_from_top: f32,
    max_y_from_top: f32,
    baseline_from_top: f32,
}

impl InkBoxMetrics {
    fn center_y_from_top(self) -> f32 {
        (self.min_y_from_top + self.max_y_from_top) / 2.0
    }
}

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

fn lock_text_resource<'a, T>(
    mutex: &'a Mutex<T>,
    resource_name: &str,
) -> Result<MutexGuard<'a, T>> {
    mutex.lock().map_err(|_| {
        PlottingError::RenderError(format!(
            "Text rendering aborted because {resource_name} lock is poisoned"
        ))
    })
}

fn lock_font_system() -> Result<MutexGuard<'static, FontSystem>> {
    lock_text_resource(get_font_system(), "FontSystem")
}

fn lock_swash_cache() -> Result<MutexGuard<'static, SwashCache>> {
    lock_text_resource(get_swash_cache(), "SwashCache")
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

/// Register a font from raw bytes with the global text system.
pub fn register_font_bytes(bytes: Vec<u8>) -> Result<()> {
    let mut font_system = lock_font_system()?;
    font_system.db_mut().load_font_data(bytes);
    Ok(())
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
    /// * `y` - Y coordinate for text top origin
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
        self.render_text_impl(pixmap, text, x, y, config, color)
    }

    /// Render text to a mutable pixmap view without cloning the frame buffer.
    pub fn render_text_mut(
        &self,
        pixmap: &mut PixmapMut<'_>,
        text: &str,
        x: f32,
        y: f32,
        config: &FontConfig,
        color: Color,
    ) -> Result<()> {
        self.render_text_impl(pixmap, text, x, y, config, color)
    }

    fn render_text_impl<T: PixmapTarget>(
        &self,
        pixmap: &mut T,
        text: &str,
        x: f32,
        y: f32,
        config: &FontConfig,
        color: Color,
    ) -> Result<()> {
        if !is_renderable_text(text) || color.a == 0 {
            return Ok(());
        }

        let mut font_system = lock_font_system()?;
        if font_system.db().is_empty() {
            log::debug!("Skipping text render because no fonts are registered");
            return Ok(());
        }
        let mut swash_cache = lock_swash_cache()?;

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
        buffer.set_text(&mut font_system, text, &attrs, Shaping::Advanced, None);

        // Shape the text
        buffer.shape_until_scroll(&mut font_system, false);

        let width = pixmap.width();
        let height = pixmap.height();
        let pixels = pixmap.pixels_mut();

        // Render each glyph
        for run in buffer.layout_runs() {
            // Add line_y offset for multiline text support
            let line_y = run.line_y;
            for glyph in run.glyphs.iter() {
                let physical_glyph = glyph.physical((x, y + line_y), 1.0);

                with_premultiplied_glyph_pixels(
                    &mut swash_cache,
                    &mut font_system,
                    physical_glyph.cache_key,
                    color,
                    |glyph_x, glyph_y, source| {
                        let pixel_x = physical_glyph.x + glyph_x;
                        let pixel_y = physical_glyph.y + glyph_y;

                        if pixel_x >= 0
                            && pixel_y >= 0
                            && (pixel_x as u32) < width
                            && (pixel_y as u32) < height
                        {
                            let idx = (pixel_y as u32 * width + pixel_x as u32) as usize;
                            blend_premultiplied_source_over(&mut pixels[idx], source);
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
    /// * `y` - Y coordinate for text top origin
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
        if !is_renderable_text(text) || color.a == 0 {
            return Ok(());
        }

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
        if !is_renderable_text(text) || color.a == 0 {
            return Ok(());
        }

        let mut font_system = lock_font_system()?;
        if font_system.db().is_empty() {
            log::debug!("Skipping rotated text render because no fonts are registered");
            return Ok(());
        }
        let mut swash_cache = lock_swash_cache()?;

        let metrics = Metrics::new(config.size, config.size * 1.2);
        let mut buffer = Buffer::new(&mut font_system, metrics);

        // Use a generous shaping buffer. Tight placement bounds are computed from
        // rasterized glyph pixels rather than heuristic constants.
        let buffer_width = (text.len() as f32 * config.size * 3.0).max(800.0);
        let buffer_height = (config.size * 6.0).max(180.0);

        buffer.set_size(&mut font_system, Some(buffer_width), Some(buffer_height));

        let attrs = config.to_cosmic_attrs();
        buffer.set_text(&mut font_system, text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut font_system, false);

        // Compute tight bounds from rasterized glyph pixels.
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        for run in buffer.layout_runs() {
            let line_y = run.line_y;
            for glyph in run.glyphs.iter() {
                let physical_glyph = glyph.physical((0., line_y), 1.0);
                with_premultiplied_glyph_pixels(
                    &mut swash_cache,
                    &mut font_system,
                    physical_glyph.cache_key,
                    color,
                    |dx, dy, _source| {
                        let px = physical_glyph.x + dx;
                        let py = physical_glyph.y + dy;
                        min_x = min_x.min(px);
                        min_y = min_y.min(py);
                        max_x = max_x.max(px);
                        max_y = max_y.max(py);
                    },
                );
            }
        }

        if min_x == i32::MAX || min_y == i32::MAX {
            return Ok(());
        }

        let text_width = (max_x - min_x + 1).max(1) as u32;
        let text_height = (max_y - min_y + 1).max(1) as u32;
        validate_text_raster_size(text_width, text_height, "Rotated text")?;

        // Create temporary pixmap for horizontal text
        let mut temp_pixmap = Pixmap::new(text_width, text_height).ok_or_else(|| {
            PlottingError::RenderError("Failed to create temp pixmap".to_string())
        })?;
        temp_pixmap.fill(tiny_skia::Color::TRANSPARENT);

        // Render glyphs to tight temporary pixmap.
        for run in buffer.layout_runs() {
            let line_y = run.line_y;
            for glyph in run.glyphs.iter() {
                let physical_glyph = glyph.physical((0., line_y), 1.0);

                with_premultiplied_glyph_pixels(
                    &mut swash_cache,
                    &mut font_system,
                    physical_glyph.cache_key,
                    color,
                    |dx, dy, source| {
                        let glyph_x = (physical_glyph.x + dx - min_x) as u32;
                        let glyph_y = (physical_glyph.y + dy - min_y) as u32;

                        if glyph_x < text_width && glyph_y < text_height {
                            let idx = glyph_y as usize * text_width as usize + glyph_x as usize;
                            if idx < temp_pixmap.pixels().len() {
                                blend_premultiplied_source_over(
                                    &mut temp_pixmap.pixels_mut()[idx],
                                    source,
                                );
                            }
                        }
                    },
                );
            }
        }

        // Apply 90° counterclockwise rotation (lossless pixel swap)
        let rotated_width = text_height;
        let rotated_height = text_width;
        validate_text_raster_size(rotated_width, rotated_height, "Rotated text")?;

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

        // Composite to main pixmap with alpha blending. Keep center anchor
        // stable by avoiding clamp-based position adjustments.
        let canvas_width = pixmap.width();
        let canvas_height = pixmap.height();

        let target_x = (x - rotated_width as f32 / 2.0).floor() as i32;
        let target_y = (y - rotated_height as f32 / 2.0).floor() as i32;

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
                            blend_premultiplied_source_over(
                                &mut pixmap.pixels_mut()[pixmap_idx],
                                src_pixel,
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Measure text placement metrics for layout/anchor conversion.
    ///
    /// Returns width/height and baseline offset from top origin.
    pub(crate) fn measure_text_placement(
        &self,
        text: &str,
        config: &FontConfig,
    ) -> Result<TextPlacementMetrics> {
        if !is_renderable_text(text) {
            return Ok(TextPlacementMetrics::new(0.0, config.size, config.size));
        }

        let mut font_system = lock_font_system()?;
        if font_system.db().is_empty() {
            log::debug!("Estimating text metrics because no fonts are registered");
            return Ok(estimate_text_metrics(text, config));
        }

        let metrics = Metrics::new(config.size, config.size * 1.2);
        let mut buffer = Buffer::new(&mut font_system, metrics);

        let buffer_width = (text.len() as f32 * config.size * 2.0).max(800.0);
        let buffer_height = (config.size * 4.0).max(100.0);
        buffer.set_size(&mut font_system, Some(buffer_width), Some(buffer_height));

        let attrs = config.to_cosmic_attrs();
        buffer.set_text(&mut font_system, text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut font_system, false);

        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;
        let mut baseline_from_top: Option<f32> = None;

        for run in buffer.layout_runs() {
            width = width.max(run.line_w);
            height = height.max(run.line_height);
            if baseline_from_top.is_none() {
                baseline_from_top = Some(run.line_y);
            }
        }

        let baseline_from_top = baseline_from_top.unwrap_or(height);
        Ok(TextPlacementMetrics::new(width, height, baseline_from_top))
    }

    /// Measure tight ink bounds for shaped text.
    ///
    /// Unlike `measure_text_placement`, this returns the bounds of the rasterized
    /// glyph ink rather than the full line box. This is useful for visually
    /// centering labels against ticks.
    fn measure_text_ink_box(&self, text: &str, config: &FontConfig) -> Result<InkBoxMetrics> {
        if !is_renderable_text(text) {
            return Ok(InkBoxMetrics {
                width: 0.0,
                height: config.size,
                min_y_from_top: 0.0,
                max_y_from_top: config.size,
                baseline_from_top: config.size,
            });
        }

        let mut font_system = lock_font_system()?;
        if font_system.db().is_empty() {
            log::debug!("Estimating text ink metrics because no fonts are registered");
            let estimated = estimate_text_metrics(text, config);
            return Ok(InkBoxMetrics {
                width: estimated.width,
                height: estimated.height,
                min_y_from_top: 0.0,
                max_y_from_top: estimated.height,
                baseline_from_top: estimated.baseline_from_top,
            });
        }

        let mut swash_cache = lock_swash_cache()?;
        let metrics = Metrics::new(config.size, config.size * 1.2);
        let mut buffer = Buffer::new(&mut font_system, metrics);

        let buffer_width = (text.len() as f32 * config.size * 2.0).max(800.0);
        let buffer_height = (config.size * 4.0).max(100.0);
        buffer.set_size(&mut font_system, Some(buffer_width), Some(buffer_height));

        let attrs = config.to_cosmic_attrs();
        buffer.set_text(&mut font_system, text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut font_system, false);

        let cosmic_color = CosmicColor::rgba(0, 0, 0, 255);
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        let mut baseline_from_top: Option<f32> = None;

        for run in buffer.layout_runs() {
            baseline_from_top.get_or_insert(run.line_y);
            let line_y = run.line_y;

            for glyph in run.glyphs.iter() {
                let physical_glyph = glyph.physical((0.0, line_y), 1.0);
                swash_cache.with_pixels(
                    &mut font_system,
                    physical_glyph.cache_key,
                    cosmic_color,
                    |dx, dy, glyph_color| {
                        if glyph_color.a() == 0 {
                            return;
                        }

                        let px = physical_glyph.x + dx;
                        let py = physical_glyph.y + dy;
                        min_x = min_x.min(px);
                        min_y = min_y.min(py);
                        max_x = max_x.max(px);
                        max_y = max_y.max(py);
                    },
                );
            }
        }

        if min_x == i32::MAX || min_y == i32::MAX {
            let placement = self.measure_text_placement(text, config)?;
            return Ok(InkBoxMetrics {
                width: placement.width,
                height: placement.height,
                min_y_from_top: 0.0,
                max_y_from_top: placement.height,
                baseline_from_top: placement.baseline_from_top,
            });
        }

        let width = (max_x - min_x + 1).max(1) as f32;
        let height = (max_y - min_y + 1).max(1) as f32;
        let baseline_from_top = baseline_from_top.unwrap_or(height) - min_y as f32;

        Ok(InkBoxMetrics {
            width,
            height,
            min_y_from_top: min_y as f32,
            max_y_from_top: max_y as f32,
            baseline_from_top,
        })
    }

    pub(crate) fn measure_text_ink_placement(
        &self,
        text: &str,
        config: &FontConfig,
    ) -> Result<TextPlacementMetrics> {
        let ink_box = self.measure_text_ink_box(text, config)?;
        Ok(TextPlacementMetrics::new(
            ink_box.width,
            ink_box.height,
            ink_box.baseline_from_top,
        ))
    }

    pub(crate) fn measure_text_ink_center_from_top(
        &self,
        text: &str,
        config: &FontConfig,
    ) -> Result<f32> {
        Ok(self.measure_text_ink_box(text, config)?.center_y_from_top())
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
        let placement = self.measure_text_placement(text, config)?;
        Ok((placement.width, placement.height))
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
        assert_eq!(FontFamily::from("cursive"), FontFamily::Cursive);
        assert_eq!(FontFamily::from("fantasy"), FontFamily::Fantasy);
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
        assert_eq!(FontFamily::Cursive.as_str(), "cursive");
        assert_eq!(FontFamily::Fantasy.as_str(), "fantasy");
        assert_eq!(FontFamily::Name("Roboto".to_string()).as_str(), "Roboto");
    }

    #[test]
    fn test_font_family_to_cosmic_generic_mapping() {
        assert!(matches!(
            FontFamily::Cursive.to_cosmic_family(),
            Family::Cursive
        ));
        assert!(matches!(
            FontFamily::Fantasy.to_cosmic_family(),
            Family::Fantasy
        ));
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
    fn poisoned_text_lock_returns_error() {
        let mutex = Mutex::new(0_u8);
        let _ = std::panic::catch_unwind(|| {
            let _guard = mutex.lock().unwrap();
            panic!("poison text lock");
        });

        let err = lock_text_resource(&mutex, "test resource").unwrap_err();
        assert!(matches!(err, PlottingError::RenderError(_)));
        assert!(err.to_string().contains("test resource lock is poisoned"));
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

    #[test]
    fn whitespace_text_is_treated_as_empty() {
        let renderer = TextRenderer::new();
        let config = FontConfig::new(FontFamily::SansSerif, 12.0);

        let (w, h) = renderer.measure_text("   \n\t", &config).unwrap();
        assert_eq!(w, 0.0);
        assert_eq!(h, 12.0);
    }

    fn pixel_rgba(pixel: PremultipliedColorU8) -> [u8; 4] {
        [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()]
    }

    fn composite_test_glyph(
        destination: &mut PremultipliedColorU8,
        glyph_pixel: GlyphPixel,
        requested_alpha: u8,
    ) {
        if let Some(source) = premultiplied_glyph_pixel(glyph_pixel, requested_alpha) {
            blend_premultiplied_source_over(destination, source);
        }
    }

    #[test]
    fn glyph_compositing_combines_coverage_and_requested_alpha() {
        let glyph = GlyphPixel::Straight([200, 100, 50, 128]);
        let mut destination = PremultipliedColorU8::TRANSPARENT;

        composite_test_glyph(&mut destination, glyph, 128);

        assert_eq!(pixel_rgba(destination), [50, 25, 13, 64]);
    }

    #[test]
    fn premultiplied_color_glyphs_are_not_premultiplied_twice() {
        let color_outline = GlyphPixel::Premultiplied([128, 0, 0, 128]);
        let color_bitmap = GlyphPixel::Straight([255, 0, 0, 128]);

        let outline_source = premultiplied_glyph_pixel(color_outline, 255).unwrap();
        let bitmap_source = premultiplied_glyph_pixel(color_bitmap, 255).unwrap();
        assert_eq!(pixel_rgba(outline_source), [128, 0, 0, 128]);
        assert_eq!(outline_source, bitmap_source);

        let translucent_outline = premultiplied_glyph_pixel(color_outline, 128).unwrap();
        assert_eq!(pixel_rgba(translucent_outline), [64, 0, 0, 64]);
    }

    #[test]
    fn glyph_source_over_preserves_transparent_translucent_and_opaque_alpha() {
        let glyph = GlyphPixel::Straight([200, 100, 50, 128]);
        let cases = [
            ([0, 0, 0, 0], [50, 25, 13, 64]),
            ([20, 40, 60, 128], [65, 55, 58, 160]),
            ([10, 20, 30, 255], [57, 40, 35, 255]),
        ];

        for (destination, expected) in cases {
            let mut destination = PremultipliedColorU8::from_rgba(
                destination[0],
                destination[1],
                destination[2],
                destination[3],
            )
            .unwrap();
            composite_test_glyph(&mut destination, glyph, 128);
            assert_eq!(pixel_rgba(destination), expected);
        }
    }

    #[test]
    fn transparent_requested_text_is_a_no_op_and_opaque_text_replaces() {
        let original = PremultipliedColorU8::from_rgba(20, 40, 60, 128).unwrap();
        let glyph = GlyphPixel::Straight([200, 100, 50, 255]);
        let mut destination = original;

        composite_test_glyph(&mut destination, glyph, 0);
        assert_eq!(destination, original);

        composite_test_glyph(&mut destination, glyph, 255);
        assert_eq!(pixel_rgba(destination), [200, 100, 50, 255]);
    }

    #[test]
    fn transparent_centered_text_is_a_no_op() {
        let renderer = TextRenderer::new();
        let config = FontConfig::new(FontFamily::SansSerif, 24.0);
        let mut pixmap = Pixmap::new(32, 32).unwrap();
        pixmap.fill(tiny_skia::Color::from_rgba8(40, 80, 120, 128));
        let before = pixmap.data().to_vec();

        renderer
            .render_text_centered(
                &mut pixmap,
                "centered",
                16.0,
                8.0,
                &config,
                Color::new_rgba(200, 100, 50, 0),
            )
            .unwrap();

        assert_eq!(pixmap.data(), before);
    }

    fn nontransparent_bounds(pixmap: &Pixmap) -> Option<(u32, u32, u32, u32)> {
        let mut min_x = pixmap.width();
        let mut min_y = pixmap.height();
        let mut max_x = 0;
        let mut max_y = 0;
        let mut found = false;

        for y in 0..pixmap.height() {
            for x in 0..pixmap.width() {
                let pixel = pixmap.pixels()[(y * pixmap.width() + x) as usize];
                if pixel.alpha() > 0 {
                    found = true;
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }

        found.then_some((min_x, min_y, max_x, max_y))
    }

    fn cropped_pixels(pixmap: &Pixmap, bounds: (u32, u32, u32, u32)) -> Vec<[u8; 4]> {
        let (min_x, min_y, max_x, max_y) = bounds;
        let mut pixels = Vec::new();
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                pixels.push(pixel_rgba(
                    pixmap.pixels()[(y * pixmap.width() + x) as usize],
                ));
            }
        }
        pixels
    }

    #[test]
    fn rotated_text_is_pixel_exact_counterclockwise_parity() {
        register_font_bytes(include_bytes!("../../assets/NotoSans-Regular.ttf").to_vec()).unwrap();

        let renderer = TextRenderer::new();
        let config = FontConfig::new(FontFamily::Name("Noto Sans".to_string()), 32.0);
        let color = Color::new_rgba(180, 90, 30, 128);
        let mut normal = Pixmap::new(128, 128).unwrap();
        let mut rotated = Pixmap::new(128, 128).unwrap();

        renderer
            .render_text(&mut normal, "A", 24.0, 24.0, &config, color)
            .unwrap();
        renderer
            .render_text_rotated(&mut rotated, "A", 64.0, 64.0, &config, color)
            .unwrap();

        let normal_bounds = nontransparent_bounds(&normal).expect("normal text rendered no pixels");
        let rotated_bounds =
            nontransparent_bounds(&rotated).expect("rotated text rendered no pixels");
        let normal_width = normal_bounds.2 - normal_bounds.0 + 1;
        let normal_height = normal_bounds.3 - normal_bounds.1 + 1;
        let rotated_width = rotated_bounds.2 - rotated_bounds.0 + 1;
        let rotated_height = rotated_bounds.3 - rotated_bounds.1 + 1;
        assert_eq!(
            (rotated_width, rotated_height),
            (normal_height, normal_width)
        );

        let normal_pixels = cropped_pixels(&normal, normal_bounds);
        let rotated_pixels = cropped_pixels(&rotated, rotated_bounds);
        for y in 0..normal_height {
            for x in 0..normal_width {
                let normal_index = (y * normal_width + x) as usize;
                let rotated_x = y;
                let rotated_y = normal_width - 1 - x;
                let rotated_index = (rotated_y * rotated_width + rotated_x) as usize;
                assert_eq!(rotated_pixels[rotated_index], normal_pixels[normal_index]);
            }
        }

        let mut alphas = normal_pixels.iter().map(|pixel| pixel[3]);
        assert!(alphas.clone().any(|alpha| alpha == color.a));
        assert!(alphas.any(|alpha| alpha > 0 && alpha < color.a));
    }
}
