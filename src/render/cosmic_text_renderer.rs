use crate::{
    core::error::{PlottingError, Result},
    render::Color,
};
use cosmic_text::{
    Attrs, Buffer, Color as CosmicColor, Family, FontSystem, Metrics, Shaping, Stretch, Style,
    SwashCache, Weight,
};
use tiny_skia::{Pixmap, PremultipliedColorU8};

/// High-quality text renderer using cosmic-text for professional typography
/// Based on official cosmic-text documentation and API
pub struct CosmicTextRenderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
}

impl CosmicTextRenderer {
    /// Create a new cosmic-text renderer with system font discovery
    /// This follows the cosmic-text documentation pattern
    pub fn new() -> Result<Self> {
        // A FontSystem provides access to detected system fonts, create one per application
        let font_system = FontSystem::new();

        // A SwashCache stores rasterized glyphs, create one per application
        let swash_cache = SwashCache::new();

        println!("✅ Cosmic-text renderer initialized with system fonts");

        Ok(Self {
            font_system,
            swash_cache,
        })
    }

    /// Render text with professional typography to a tiny-skia pixmap
    /// Following cosmic-text documentation patterns
    pub fn render_text(
        &mut self,
        pixmap: &mut Pixmap,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        color: Color,
    ) -> Result<()> {
        // Text metrics indicate the font size and line height of a buffer
        let metrics = Metrics::new(font_size, font_size * 1.2);

        // A Buffer provides shaping and layout for a UTF-8 string, create one per text widget
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        // Set buffer dimensions - allow text to flow naturally
        buffer.set_size(&mut self.font_system, Some(800.0), Some(200.0));

        // Set text with professional typography - use high-quality font
        let attrs = Attrs::new().family(Family::Name("Roboto")); // Use Roboto font directly

        buffer.set_text(
            &mut self.font_system,
            text,
            attrs,
            Shaping::Advanced, // Enable advanced text shaping (ligatures, kerning)
        );

        // Shape the text - this performs the advanced typography processing
        buffer.shape_until_scroll(&mut self.font_system, false);

        // Convert our color to cosmic-text format
        let cosmic_color = CosmicColor::rgba(color.r, color.g, color.b, color.a);

        // Render each text run with professional rasterization
        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                // Get physical glyph placement
                let physical_glyph = glyph.physical((x, y), 1.0);

                // Rasterize glyph with cosmic-text's high-quality renderer
                self.swash_cache.with_pixels(
                    &mut self.font_system,
                    physical_glyph.cache_key,
                    cosmic_color,
                    |x, y, color| {
                        // x, y are pixel coordinates, color has alpha channel
                        let pixel_x = physical_glyph.x + x;
                        let pixel_y = physical_glyph.y + y;

                        // Check bounds before blending
                        if pixel_x >= 0
                            && pixel_y >= 0
                            && (pixel_x as u32) < pixmap.width()
                            && (pixel_y as u32) < pixmap.height()
                        {
                            let alpha = color.a();
                            if alpha > 0 {
                                let pixmap_idx =
                                    (pixel_y as u32 * pixmap.width() + pixel_x as u32) as usize;
                                let background = pixmap.pixels()[pixmap_idx];

                                // Alpha blend text color with background
                                let alpha_f = alpha as f32 / 255.0;
                                let inv_alpha = 1.0 - alpha_f;

                                let blended_r = (color.r() as f32 * alpha_f
                                    + background.red() as f32 * inv_alpha)
                                    as u8;
                                let blended_g = (color.g() as f32 * alpha_f
                                    + background.green() as f32 * inv_alpha)
                                    as u8;
                                let blended_b = (color.b() as f32 * alpha_f
                                    + background.blue() as f32 * inv_alpha)
                                    as u8;

                                let blended = PremultipliedColorU8::from_rgba(
                                    blended_r, blended_g, blended_b, 255,
                                )
                                .unwrap_or(background);

                                pixmap.pixels_mut()[pixmap_idx] = blended;
                            }
                        }
                    },
                );
            }
        }

        Ok(())
    }

    pub fn render_text_rotated(
        &mut self,
        pixmap: &mut Pixmap,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        color: Color,
    ) -> Result<()> {
        // Create buffer and configure for text rendering with the provided font size
        let metrics = Metrics::new(font_size, font_size * 1.2);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        // Calculate DPI-aware buffer dimensions based on text content and font size
        let dpi_scale = font_size / 12.0; // Relative to base font size (12pt)
        let text_length_factor = (text.len() as f32).max(4.0); // Minimum width for short text
        let buffer_width = (text_length_factor * font_size * 2.5 * dpi_scale).max(800.0); // Further increased for more space
        let buffer_height = (font_size * 6.0 * dpi_scale).max(180.0); // Further increased DPI-scaled height
        
        buffer.set_size(
            &mut self.font_system,
            Some(buffer_width),
            Some(buffer_height),
        );

        let attrs = Attrs::new()
            .family(Family::SansSerif)
            .stretch(Stretch::Normal)
            .style(Style::Normal)
            .weight(Weight::NORMAL);

        buffer.set_text(&mut self.font_system, text, attrs, Shaping::Advanced);

        buffer.shape_until_scroll(&mut self.font_system, false);

        // Calculate actual text bounds
        let mut max_x = 0.0f32;
        let mut max_y = 0.0f32;
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;

        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let physical_glyph = glyph.physical((0., 0.), 1.0);
                let x = physical_glyph.x as f32;
                let y = physical_glyph.y as f32;

                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x + 20.0); // Rough glyph width estimate
                max_y = max_y.max(y + run.line_height);
            }
        }

        // If no glyphs found, use font metrics
        if min_x == f32::MAX {
            min_x = 0.0;
            min_y = 0.0;
            max_x = text.len() as f32 * font_size * 0.6; // rough estimate
            max_y = font_size * 1.2;
        }

        // DPI-scaled padding for glyph rendering margins  
        let padding = 30.0 * dpi_scale; // Further increased padding for glyph rendering
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

        // Render text to temporary pixmap
        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let physical_glyph = glyph.physical((0., 0.), 1.0);

                self.swash_cache.with_pixels(
                    &mut self.font_system,
                    physical_glyph.cache_key,
                    CosmicColor::rgba(color.r, color.g, color.b, color.a),
                    |dx: i32, dy: i32, glyph_color: CosmicColor| {
                        let glyph_x_calc = (physical_glyph.x as f32 - min_x) as i32 + dx;
                        let glyph_y_calc = (physical_glyph.y as f32 - min_y) as i32 + dy;

                        // Skip negative coordinates (glyph rendering margins)
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

        // Apply 90° counterclockwise rotation: (x,y) -> (y, height-1-x)
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
                    // 90° CCW rotation: (x,y) -> (y, height-1-x)
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

        // Calculate target position with bounds validation
        let canvas_width = pixmap.width();
        let canvas_height = pixmap.height();
        
        // Calculate required margin for rotated text
        let margin_needed_x = (rotated_width / 2) as i32;
        let margin_needed_y = (rotated_height / 2) as i32;
        
        // Ensure text stays within canvas bounds
        let target_x = (x as i32 - margin_needed_x)
            .max(0)
            .min((canvas_width as i32) - (rotated_width as i32));
        let target_y = (y as i32 - margin_needed_y)
            .max(0) 
            .min((canvas_height as i32) - (rotated_height as i32));

        // Draw rotated text to main pixmap with bounds checking
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
                        let pixmap_idx =
                            (final_y as u32 * canvas_width + final_x as u32) as usize;
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
        let metrics = Metrics::new(font_size, font_size * 1.2);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        // Set large dimensions for measurement
        buffer.set_size(&mut self.font_system, Some(f32::MAX), Some(f32::MAX));

        let attrs = Attrs::new()
            .family(Family::Name("DejaVu Sans"))
            .family(Family::Name("Roboto"))
            .family(Family::SansSerif);

        buffer.set_text(&mut self.font_system, text, attrs, Shaping::Advanced);

        buffer.shape_until_scroll(&mut self.font_system, false);

        // Calculate actual text dimensions
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

