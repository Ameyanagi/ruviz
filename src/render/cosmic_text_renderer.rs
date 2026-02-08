//! High-quality text renderer using cosmic-text for professional typography
//!
//! This module provides the CosmicTextRenderer which integrates with the
//! shared FontSystem and SwashCache singletons for efficient text rendering.
//!
//! Note: For new code, prefer using `TextRenderer` from `crate::render::text`
//! which provides a cleaner API with `FontConfig` support.

use crate::{
    core::error::{PlottingError, Result},
    render::{
        Color,
        text::{FontConfig, FontFamily, TextRenderer, get_font_system, get_swash_cache},
    },
};
use cosmic_text::{Attrs, Buffer, Color as CosmicColor, Family, Metrics, Shaping};
use tiny_skia::{Pixmap, PremultipliedColorU8};

/// High-quality text renderer using cosmic-text for professional typography
///
/// This renderer uses the global FontSystem and SwashCache singletons
/// to avoid redundant font discovery and glyph caching.
pub struct CosmicTextRenderer;

impl CosmicTextRenderer {
    /// Create a new cosmic-text renderer
    ///
    /// This is a lightweight operation as font discovery is handled by
    /// the global FontSystem singleton.
    pub fn new() -> Result<Self> {
        log::debug!("CosmicTextRenderer initialized (using shared FontSystem)");
        Ok(Self)
    }

    /// Render text with professional typography to a tiny-skia pixmap
    pub fn render_text(
        &mut self,
        pixmap: &mut Pixmap,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        color: Color,
    ) -> Result<()> {
        let mut font_system = get_font_system()
            .lock()
            .map_err(|e| PlottingError::RenderError(format!("Failed to lock FontSystem: {}", e)))?;

        let mut swash_cache = get_swash_cache()
            .lock()
            .map_err(|e| PlottingError::RenderError(format!("Failed to lock SwashCache: {}", e)))?;

        // Text metrics indicate the font size and line height of a buffer
        let metrics = Metrics::new(font_size, font_size * 1.2);

        // Create buffer for text layout
        let mut buffer = Buffer::new(&mut font_system, metrics);

        // Calculate generous buffer dimensions
        let dpi_scale = (font_size / 12.0).max(1.0);
        let text_length_factor = (text.len() as f32).max(8.0);
        let buffer_width = (text_length_factor * font_size * 2.0 * dpi_scale).max(3200.0);
        let buffer_height = (font_size * 6.0 * dpi_scale).max(600.0);

        buffer.set_size(&mut font_system, Some(buffer_width), Some(buffer_height));

        // Set text with sans-serif font fallback (system fonts are discovered automatically)
        let attrs = Attrs::new().family(Family::SansSerif);

        buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut font_system, false);

        // Convert color to cosmic-text format
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
                                let pixmap_idx =
                                    (pixel_y as u32 * pixmap.width() + pixel_x as u32) as usize;
                                let background = pixmap.pixels()[pixmap_idx];

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
                                    pixmap.pixels_mut()[pixmap_idx] = blended;
                                }
                            }
                        }
                    },
                );
            }
        }

        Ok(())
    }

    /// Render text rotated 90 degrees counterclockwise.
    pub fn render_text_rotated(
        &mut self,
        pixmap: &mut Pixmap,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        color: Color,
    ) -> Result<()> {
        let renderer = TextRenderer::new();
        let config = FontConfig::new(FontFamily::SansSerif, font_size);
        renderer.render_text_rotated(pixmap, text, x, y, &config, color)
    }

    /// Measure text dimensions for layout calculations
    pub fn measure_text(&mut self, text: &str, font_size: f32) -> Result<(f32, f32)> {
        let mut font_system = get_font_system()
            .lock()
            .map_err(|e| PlottingError::RenderError(format!("Failed to lock FontSystem: {}", e)))?;

        let metrics = Metrics::new(font_size, font_size * 1.2);
        let mut buffer = Buffer::new(&mut font_system, metrics);

        let dpi_scale = (font_size / 12.0).max(1.0);
        let text_length_factor = (text.len() as f32).max(8.0);
        let buffer_width = (text_length_factor * font_size * 2.0 * dpi_scale).max(3200.0);
        let buffer_height = (font_size * 6.0 * dpi_scale).max(600.0);

        buffer.set_size(&mut font_system, Some(buffer_width), Some(buffer_height));

        let attrs = Attrs::new().family(Family::SansSerif);

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

impl Default for CosmicTextRenderer {
    fn default() -> Self {
        Self::new().expect("Failed to create CosmicTextRenderer")
    }
}
