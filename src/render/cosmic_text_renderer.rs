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
        text::{
            FontConfig, FontFamily, TextRenderer, blend_premultiplied_source_over, get_font_system,
            get_swash_cache, with_premultiplied_glyph_pixels,
        },
    },
};
use cosmic_text::{Attrs, Buffer, Family, Metrics, Shaping};
use tiny_skia::Pixmap;

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
        if color.a == 0 {
            return Ok(());
        }

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

        buffer.set_text(&mut font_system, text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut font_system, false);

        // Render each glyph
        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let physical_glyph = glyph.physical((x, y), 1.0);

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
                            && (pixel_x as u32) < pixmap.width()
                            && (pixel_y as u32) < pixmap.height()
                        {
                            let pixmap_idx =
                                (pixel_y as u32 * pixmap.width() + pixel_x as u32) as usize;
                            blend_premultiplied_source_over(
                                &mut pixmap.pixels_mut()[pixmap_idx],
                                source,
                            );
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

        buffer.set_text(&mut font_system, text, &attrs, Shaping::Advanced, None);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_renderer_preserves_destination_alpha() {
        let mut renderer = CosmicTextRenderer::new().unwrap();
        let color = Color::new_rgba(180, 90, 30, 96);

        let mut transparent = Pixmap::new(128, 96).unwrap();
        renderer
            .render_text(&mut transparent, "A", 24.0, 64.0, 32.0, color)
            .unwrap();
        let transparent_ink: Vec<_> = transparent
            .pixels()
            .iter()
            .copied()
            .filter(|pixel| pixel.alpha() > 0)
            .collect();
        assert!(!transparent_ink.is_empty());
        assert!(transparent_ink.iter().all(|pixel| pixel.alpha() <= color.a));
        assert!(transparent_ink.iter().any(|pixel| pixel.alpha() < color.a));
        assert!(transparent_ink.iter().all(|pixel| {
            pixel.red() <= pixel.alpha()
                && pixel.green() <= pixel.alpha()
                && pixel.blue() <= pixel.alpha()
        }));

        let mut translucent = Pixmap::new(128, 96).unwrap();
        translucent.fill(tiny_skia::Color::from_rgba8(40, 80, 120, 128));
        let before = translucent.pixels().to_vec();
        renderer
            .render_text(&mut translucent, "A", 24.0, 64.0, 32.0, color)
            .unwrap();
        let changed: Vec<_> = translucent
            .pixels()
            .iter()
            .zip(before.iter())
            .filter_map(|(after, before)| (after != before).then_some(*after))
            .collect();
        assert!(!changed.is_empty());
        assert!(
            changed
                .iter()
                .all(|pixel| pixel.alpha() >= 128 && pixel.alpha() < 255)
        );
        assert!(changed.iter().any(|pixel| pixel.alpha() > 128));
    }
}
