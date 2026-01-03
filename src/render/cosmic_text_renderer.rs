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
        text::{get_font_system, get_swash_cache},
    },
};
use cosmic_text::{
    Attrs, Buffer, Color as CosmicColor, Family, Metrics, Shaping, Stretch, Style, Weight,
};
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

    /// Render text rotated 90 degrees counterclockwise
    pub fn render_text_rotated(
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

        let metrics = Metrics::new(font_size, font_size * 1.2);
        let mut buffer = Buffer::new(&mut font_system, metrics);

        let dpi_scale = font_size / 12.0;
        let text_length_factor = (text.len() as f32).max(4.0);
        let buffer_width = (text_length_factor * font_size * 2.5 * dpi_scale).max(800.0);
        let buffer_height = (font_size * 6.0 * dpi_scale).max(180.0);

        buffer.set_size(&mut font_system, Some(buffer_width), Some(buffer_height));

        let attrs = Attrs::new()
            .family(Family::SansSerif)
            .stretch(Stretch::Normal)
            .style(Style::Normal)
            .weight(Weight::NORMAL);

        buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut font_system, false);

        // Calculate actual text bounds
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
            max_x = text.len() as f32 * font_size * 0.6;
            max_y = font_size * 1.2;
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
