use std::path::Path;
use tiny_skia::*;
use fontdue::{Font, FontSettings};
use crate::{
    core::{PlottingError, Result, plot::Image},
    render::{Color, LineStyle, MarkerStyle, Theme, FontFamily, CosmicTextRenderer},
};

/// Tiny-skia based renderer with cosmic-text for professional typography
pub struct SkiaRenderer {
    width: u32,
    height: u32,
    pixmap: Pixmap,
    paint: Paint<'static>,
    theme: Theme,
    font: Font,  // Keep fontdue for fallback
    cosmic_text: CosmicTextRenderer,  // High-quality text rendering
}

impl SkiaRenderer {
    /// Create a new renderer with the given dimensions
    pub fn new(width: u32, height: u32, theme: Theme) -> Result<Self> {
        Self::with_font_family(width, height, theme, FontFamily::SansSerif)
    }
    
    /// Create a new renderer with specified font family
    pub fn with_font_family(width: u32, height: u32, theme: Theme, font_family: FontFamily) -> Result<Self> {
        let mut pixmap = Pixmap::new(width, height)
            .ok_or(PlottingError::OutOfMemory)?;
        
        // Fill background
        let bg_color = theme.background.to_tiny_skia_color();
        pixmap.fill(bg_color);
        
        let paint = Paint::default();
        
        // Load font using FontFamily system
        let font = font_family.load_font().unwrap_or_else(|_| {
            // If requested font fails, try SansSerif fallback
            FontFamily::SansSerif.load_font().unwrap_or_else(|_| {
                // Last resort: create simple font
                Self::create_simple_font()
            })
        });
        
        // Initialize cosmic-text renderer for professional typography
        let cosmic_text = CosmicTextRenderer::new()
            .map_err(|e| PlottingError::RenderError(format!("Failed to create cosmic-text renderer: {}", e)))?;

        Ok(Self {
            width,
            height,
            pixmap,
            paint,
            theme,
            font,
            cosmic_text,
        })
    }

    
    /// Load font with system fallbacks
    fn load_font() -> Result<Font> {
        // Try to load common system fonts
        let font_candidates = [
            // Arch Linux fonts (Adwaita, Noto, Nimbus)
            "/usr/share/fonts/Adwaita/AdwaitaSans-Regular.ttf",
            "/usr/share/fonts/noto/NotoSansGothic-Regular.ttf",
            "/usr/share/fonts/noto/NotoSansTelugu-Regular.ttf",
            "/usr/share/fonts/gsfonts/NimbusSans-Regular.otf",
            // Other Linux fonts
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
            // Windows fonts
            "C:/Windows/Fonts/arial.ttf",
            "C:/Windows/Fonts/calibri.ttf",
            // macOS fonts
            "/System/Library/Fonts/Arial.ttf",
        ];
        
        // Try each font path
        for font_path in &font_candidates {
            if let Ok(font_data) = std::fs::read(font_path) {
                if let Ok(font) = Font::from_bytes(font_data, FontSettings::default()) {
                    println!("✅ Loaded font: {}", font_path);
                    return Ok(font);
                }
            }
        }
        
        // If no system font found, create simple embedded font
        Self::create_embedded_font()
    }
    
    /// Create embedded font as last resort
    fn create_embedded_font() -> Result<Font> {
        // Create a minimal embedded font using a small TTF
        // For now, we'll create a simple font that handles basic ASCII
        
        println!("⚠️  No system fonts found, using fallback vector rendering");
        Err(PlottingError::FontError("No fonts available - using vector fallback".to_string()))
    }
    
    /// Create a simple programmatic font as ultimate fallback
    fn create_simple_font() -> Font {
        // This should never be called now, but kept for safety
        panic!("Font creation fallback should not be reached")
    }
    
    /// Clear the canvas with background color
    pub fn clear(&mut self) {
        let bg_color = self.theme.background.to_tiny_skia_color();
        self.pixmap.fill(bg_color);
    }
    
    /// Draw a line between two points
    pub fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: Color, width: f32, style: LineStyle) -> Result<()> {
        let mut paint = Paint::default();
        paint.set_color(color.to_tiny_skia_color());
        paint.anti_alias = true;
        
        let mut stroke = Stroke::default();
        stroke.width = width.max(0.1);
        
        // Apply line style
        if let Some(dash_pattern) = style.to_dash_array() {
            stroke.dash = StrokeDash::new(dash_pattern, 0.0);
        }
        
        paint.set_color_rgba8(color.r, color.g, color.b, color.a);
        
        let mut path = PathBuilder::new();
        path.move_to(x1, y1);
        path.line_to(x2, y2);
        let path = path.finish().ok_or(PlottingError::RenderError("Failed to create line path".to_string()))?;
        
        self.pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        
        Ok(())
    }
    
    /// Draw a series of connected lines (polyline)
    pub fn draw_polyline(&mut self, points: &[(f32, f32)], color: Color, width: f32, style: LineStyle) -> Result<()> {
        if points.len() < 2 {
            return Ok(());
        }
        
        let mut paint = Paint::default();
        paint.set_color(color.to_tiny_skia_color());
        paint.anti_alias = true;
        
        let mut stroke = Stroke::default();
        stroke.width = width.max(0.1);
        stroke.line_cap = LineCap::Round;
        stroke.line_join = LineJoin::Round;
        
        // Apply line style
        if let Some(dash_pattern) = style.to_dash_array() {
            stroke.dash = StrokeDash::new(dash_pattern, 0.0);
        }
        
        let mut path = PathBuilder::new();
        path.move_to(points[0].0, points[0].1);
        
        for &(x, y) in &points[1..] {
            path.line_to(x, y);
        }
        
        let path = path.finish().ok_or(PlottingError::RenderError("Failed to create polyline path".to_string()))?;
        
        self.pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        
        Ok(())
    }
    
    /// Draw a circle (for scatter plots)
    pub fn draw_circle(&mut self, x: f32, y: f32, radius: f32, color: Color, filled: bool) -> Result<()> {
        let mut paint = Paint::default();
        paint.set_color(color.to_tiny_skia_color());
        paint.anti_alias = true;
        
        let mut path = PathBuilder::new();
        path.push_circle(x, y, radius);
        let path = path.finish().ok_or(PlottingError::RenderError("Failed to create circle path".to_string()))?;
        
        if filled {
            self.pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        } else {
            let stroke = Stroke::default();
            self.pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
        
        Ok(())
    }
    
    /// Draw a rectangle (for bar plots)
    pub fn draw_rectangle(&mut self, x: f32, y: f32, width: f32, height: f32, color: Color, filled: bool) -> Result<()> {
        let mut paint = Paint::default();
        paint.set_color(color.to_tiny_skia_color());
        paint.anti_alias = true;
        
        let rect = Rect::from_xywh(x, y, width, height)
            .ok_or(PlottingError::RenderError("Invalid rectangle dimensions".to_string()))?;
        
        let mut path = PathBuilder::new();
        path.push_rect(rect);
        let path = path.finish().ok_or(PlottingError::RenderError("Failed to create rectangle path".to_string()))?;
        
        if filled {
            self.pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        } else {
            let stroke = Stroke::default();
            self.pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
        
        Ok(())
    }
    
    /// Draw a marker at the given position
    pub fn draw_marker(&mut self, x: f32, y: f32, size: f32, style: MarkerStyle, color: Color) -> Result<()> {
        let radius = size * 0.5;
        
        match style {
            MarkerStyle::Circle => {
                self.draw_circle(x, y, radius, color, true)?;
            }
            MarkerStyle::Square => {
                let half_size = radius;
                self.draw_rectangle(x - half_size, y - half_size, size, size, color, true)?;
            }
            MarkerStyle::Triangle => {
                let mut paint = Paint::default();
                paint.set_color(color.to_tiny_skia_color());
                paint.anti_alias = true;
                
                let mut path = PathBuilder::new();
                path.move_to(x, y - radius);
                path.line_to(x - radius * 0.866, y + radius * 0.5); // 60 degree angles
                path.line_to(x + radius * 0.866, y + radius * 0.5);
                path.close();
                
                let path = path.finish().ok_or(PlottingError::RenderError("Failed to create triangle path".to_string()))?;
                self.pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
            }
            MarkerStyle::Diamond => {
                let mut paint = Paint::default();
                paint.set_color(color.to_tiny_skia_color());
                paint.anti_alias = true;
                
                let mut path = PathBuilder::new();
                path.move_to(x, y - radius);
                path.line_to(x + radius, y);
                path.line_to(x, y + radius);
                path.line_to(x - radius, y);
                path.close();
                
                let path = path.finish().ok_or(PlottingError::RenderError("Failed to create diamond path".to_string()))?;
                self.pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
            }
            MarkerStyle::Plus => {
                // Draw cross with lines
                self.draw_line(x - radius, y, x + radius, y, color, 2.0, LineStyle::Solid)?;
                self.draw_line(x, y - radius, x, y + radius, color, 2.0, LineStyle::Solid)?;
            }
            MarkerStyle::Cross => {
                // Draw X with lines
                let offset = radius * 0.707; // sin(45°)
                self.draw_line(x - offset, y - offset, x + offset, y + offset, color, 2.0, LineStyle::Solid)?;
                self.draw_line(x - offset, y + offset, x + offset, y - offset, color, 2.0, LineStyle::Solid)?;
            }
            _ => {
                // For other marker types, fallback to circle
                self.draw_circle(x, y, radius, color, style.is_filled())?;
            }
        }
        
        Ok(())
    }
    
    /// Draw grid lines
    pub fn draw_grid(&mut self, x_ticks: &[f32], y_ticks: &[f32], plot_area: Rect, color: Color, style: LineStyle) -> Result<()> {
        let width = 1.0;
        
        // Vertical grid lines
        for &x in x_ticks {
            if x >= plot_area.left() && x <= plot_area.right() {
                self.draw_line(x, plot_area.top(), x, plot_area.bottom(), color, width, style.clone())?;
            }
        }
        
        // Horizontal grid lines
        for &y in y_ticks {
            if y >= plot_area.top() && y <= plot_area.bottom() {
                self.draw_line(plot_area.left(), y, plot_area.right(), y, color, width, style.clone())?;
            }
        }
        
        Ok(())
    }
    
    /// Draw axis lines and ticks
    pub fn draw_axes(&mut self, plot_area: Rect, x_ticks: &[f32], y_ticks: &[f32], color: Color) -> Result<()> {
        let axis_width = 1.5;
        let tick_size = 5.0;
        
        // Draw main axis lines
        // X-axis (bottom)
        self.draw_line(
            plot_area.left(), 
            plot_area.bottom(), 
            plot_area.right(), 
            plot_area.bottom(), 
            color, 
            axis_width, 
            LineStyle::Solid
        )?;
        
        // Y-axis (left)
        self.draw_line(
            plot_area.left(), 
            plot_area.top(), 
            plot_area.left(), 
            plot_area.bottom(), 
            color, 
            axis_width, 
            LineStyle::Solid
        )?;
        
        // Draw tick marks
        // X-axis ticks
        for &x in x_ticks {
            if x >= plot_area.left() && x <= plot_area.right() {
                self.draw_line(x, plot_area.bottom(), x, plot_area.bottom() + tick_size, color, 1.0, LineStyle::Solid)?;
            }
        }
        
        // Y-axis ticks
        for &y in y_ticks {
            if y >= plot_area.top() && y <= plot_area.bottom() {
                self.draw_line(plot_area.left() - tick_size, y, plot_area.left(), y, color, 1.0, LineStyle::Solid)?;
            }
        }
        
        Ok(())
    }

    
    /// Draw a DataShader aggregated image
    pub fn draw_datashader_image(&mut self, image: &crate::data::DataShaderImage, plot_area: Rect) -> Result<()> {
        // Create a pixmap from the DataShader image data
        let mut datashader_pixmap = Pixmap::new(image.width as u32, image.height as u32)
            .ok_or(PlottingError::OutOfMemory)?;
            
        // Copy the RGBA data from DataShader
        if image.pixels.len() != (image.width * image.height * 4) {
            return Err(PlottingError::RenderError(
                "Invalid DataShader image pixel data".to_string()
            ));
        }
        
        // Convert RGBA u8 data to tiny-skia's format
        let pixmap_data = datashader_pixmap.data_mut();
        for (i, chunk) in image.pixels.chunks_exact(4).enumerate() {
            let r = chunk[0];
            let g = chunk[1]; 
            let b = chunk[2];
            let a = chunk[3];
            
            // tiny-skia uses premultiplied alpha BGRA format
            let alpha_f = a as f32 / 255.0;
            let premult_r = ((r as f32 * alpha_f) as u8);
            let premult_g = ((g as f32 * alpha_f) as u8);
            let premult_b = ((b as f32 * alpha_f) as u8);
            
            // BGRA order for tiny-skia
            pixmap_data[i * 4] = premult_b;
            pixmap_data[i * 4 + 1] = premult_g;
            pixmap_data[i * 4 + 2] = premult_r;
            pixmap_data[i * 4 + 3] = a;
        }
        
        // Scale and draw the DataShader image onto the plot area
        let src_rect = Rect::from_xywh(0.0, 0.0, image.width as f32, image.height as f32)
            .ok_or(PlottingError::RenderError("Invalid source rect".to_string()))?;
            
        let transform = Transform::from_scale(
            plot_area.width() / image.width as f32,
            plot_area.height() / image.height as f32,
        ).post_translate(plot_area.x(), plot_area.y());
        
        self.pixmap.draw_pixmap(
            plot_area.x() as i32,
            plot_area.y() as i32,
            datashader_pixmap.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None
        );
        
        Ok(())
    }

    /// Draw text at the specified position using cosmic-text (professional quality)
    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color) -> Result<()> {
        // Use cosmic-text for professional typography with text shaping
        self.draw_text_cosmic(text, x, y, size, color)
    }
    
    /// Draw text using cosmic-text with professional typography and text shaping
    pub fn draw_text_cosmic(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color) -> Result<()> {
        println!("✅ Using cosmic-text for professional text rendering: '{}'", text);
        self.cosmic_text.render_text(&mut self.pixmap, text, x, y, size, color)
    }
    
    /// Draw text rotated 90 degrees counterclockwise using cosmic-text
    pub fn draw_text_rotated(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color) -> Result<()> {
        println!("🔄 Using cosmic-text for rotated text rendering: '{}'", text);
        self.cosmic_text.render_text_rotated(&mut self.pixmap, text, x, y, size, color)
    }
    
    /// Fallback text rendering using fontdue (for compatibility)
    pub fn draw_text_fallback(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color) -> Result<()> {
        // Use fontdue implementation with baseline alignment as fallback
        self.draw_text_fontdue(text, x, y, size, color)
    }
    
    /// Draw text using fontdue with proper baseline alignment
    pub fn draw_text_fontdue(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color) -> Result<()> {
        let mut current_x = x;
        
        // Treat 'y' as the baseline position (standard typography convention)
        // This is what most plotting systems expect
        let baseline_y = y;
        
        for ch in text.chars() {
            let (metrics, bitmap) = self.font.rasterize(ch, size);
            
            if !bitmap.is_empty() && metrics.width > 0 && metrics.height > 0 {
                // Create a tiny-skia pixmap from the fontdue bitmap with proper alpha blending
                if let Some(char_pixmap) = self.create_char_pixmap_aligned(&bitmap, metrics.width, metrics.height, color) {
                    // Position character correctly relative to baseline
                    // metrics.xmin: horizontal bearing (offset from current position)
                    // metrics.ymin: vertical bearing (distance from baseline to top of glyph, usually negative)
                    // The glyph bitmap top-left should be at (baseline_y + ymin)
                    let char_x = (current_x + metrics.xmin as f32).round() as i32;
                    let char_y = (baseline_y + metrics.ymin as f32).round() as i32;
                    
                    self.pixmap.draw_pixmap(
                        char_x,
                        char_y,
                        char_pixmap.as_ref(),
                        &PixmapPaint::default(),
                        Transform::identity(),
                        None,
                    );
                }
            }
            
            current_x += metrics.advance_width;
        }
        
        Ok(())
    }
    
    /// Create a tiny-skia pixmap from fontdue bitmap with proper alpha blending
    fn create_char_pixmap_aligned(&self, bitmap: &[u8], width: usize, height: usize, color: Color) -> Option<Pixmap> {
        let mut char_pixmap = Pixmap::new(width as u32, height as u32)?;
        
        let (r, g, b, a) = color.to_rgba_f32();
        let color_r = (r * 255.0) as u8;
        let color_g = (g * 255.0) as u8;
        let color_b = (b * 255.0) as u8;
        let color_a = (a * 255.0) as u8;
        
        // Convert fontdue grayscale bitmap to RGBA with proper gamma correction
        for (i, &alpha) in bitmap.iter().enumerate() {
            if alpha > 0 {
                let pixel_x = i % width;
                let pixel_y = i / width;
                
                if pixel_x < width && pixel_y < height {
                    // Apply gamma correction for better anti-aliasing
                    let gamma_corrected = ((alpha as f32 / 255.0).powf(1.0 / 2.2) * 255.0) as u8;
                    
                    // Calculate final alpha combining fontdue alpha with color alpha
                    let final_alpha = ((gamma_corrected as u32 * color_a as u32) / 255) as u8;
                    
                    // Premultiply color channels by final alpha
                    let premult_r = ((color_r as u32 * final_alpha as u32) / 255) as u8;
                    let premult_g = ((color_g as u32 * final_alpha as u32) / 255) as u8;
                    let premult_b = ((color_b as u32 * final_alpha as u32) / 255) as u8;
                    
                    let rgba_pixel = tiny_skia::PremultipliedColorU8::from_rgba(
                        premult_r, premult_g, premult_b, final_alpha
                    ).unwrap_or(tiny_skia::PremultipliedColorU8::TRANSPARENT);
                    
                    char_pixmap.pixels_mut()[(pixel_y * width + pixel_x)] = rgba_pixel;
                }
            }
        }
        
        Some(char_pixmap)
    }
    
    /// Create a tiny-skia pixmap from fontdue bitmap (legacy version)
    fn create_char_pixmap(&self, bitmap: &[u8], width: usize, height: usize, color: Color) -> Option<Pixmap> {
        self.create_char_pixmap_aligned(bitmap, width, height, color)
    }

    /// Clean vector-based text rendering using only tiny-skia rectangles
    pub fn draw_text_fontdue_improved(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color) -> Result<()> {
        use tiny_skia::{Paint, Rect, Transform};
        
        let mut paint = Paint::default();
        paint.set_color_rgba8(color.r, color.g, color.b, color.a);
        paint.anti_alias = true;
        
        // Use simple rectangles to create clean, aliased text (no paths)
        let char_width = size * 0.6;
        let char_height = size * 0.8;
        let mut current_x = x;
        
        for ch in text.chars() {
            match ch {
                ' ' => {
                    current_x += char_width * 0.5;
                }
                '0' => {
                    // Draw 0 as solid rectangle (simpler approach to avoid hairline issues)
                    if let Some(rect) = Rect::from_xywh(current_x, y - char_height, char_width * 0.7, char_height) {
                        self.pixmap.fill_rect(rect, &paint, Transform::identity(), None);
                    }
                    current_x += char_width;
                }
                '1' => {
                    // Draw 1 as thick vertical rectangle
                    if let Some(rect) = Rect::from_xywh(current_x + char_width * 0.4, y - char_height, size * 0.15, char_height) {
                        self.pixmap.fill_rect(rect, &paint, Transform::identity(), None);
                    }
                    current_x += char_width;
                }
                '2'..='9' => {
                    // Draw other numbers as solid rectangles
                    if let Some(rect) = Rect::from_xywh(current_x, y - char_height, char_width * 0.7, char_height) {
                        self.pixmap.fill_rect(rect, &paint, Transform::identity(), None);
                    }
                    current_x += char_width;
                }
                'A'..='Z' | 'a'..='z' => {
                    // Draw letters as rectangles
                    if let Some(rect) = Rect::from_xywh(current_x, y - char_height, char_width * 0.8, char_height) {
                        self.pixmap.fill_rect(rect, &paint, Transform::identity(), None);
                    }
                    current_x += char_width;
                }
                '.' => {
                    // Draw dot
                    let dot_size = size * 0.15;
                    if let Some(dot_rect) = Rect::from_xywh(current_x + char_width * 0.25, y - dot_size, dot_size, dot_size) {
                        self.pixmap.fill_rect(dot_rect, &paint, Transform::identity(), None);
                    }
                    current_x += char_width * 0.4;
                }
                _ => {
                    // Default rectangle for other characters
                    if let Some(rect) = Rect::from_xywh(current_x, y - char_height * 0.8, char_width * 0.7, char_height * 0.8) {
                        self.pixmap.fill_rect(rect, &paint, Transform::identity(), None);
                    }
                    current_x += char_width;
                }
            }
        }
        
        Ok(())
    }

    fn draw_vector_char(&mut self, ch: char, x: f32, y: f32, size: f32, paint: &tiny_skia::Paint) -> Result<()> {
        use tiny_skia::{PathBuilder, Transform, FillRule};
        
        let char_width = size * 0.6;
        let char_height = size * 0.8;
        let stroke_width = size * 0.1; // Thick enough to avoid hairline issues
        
        // Create simple filled shapes for common characters (avoid hairline strokes)
        let mut path_builder = PathBuilder::new();
        
        match ch {
            // Numbers
            '0' => {
                // Draw oval for 0
                let center_x = x + char_width * 0.5;
                let center_y = y - char_height * 0.5;
                let rx = char_width * 0.25;
                let ry = char_height * 0.35;
                
                if let Some(outer_rect) = tiny_skia::Rect::from_xywh(center_x - rx, center_y - ry, rx * 2.0, ry * 2.0) {
                    path_builder.push_oval(outer_rect);
                    // Create inner hole
                    let inner_rx = rx * 0.6;
                    let inner_ry = ry * 0.6;
                    if let Some(inner_rect) = tiny_skia::Rect::from_xywh(center_x - inner_rx, center_y - inner_ry, inner_rx * 2.0, inner_ry * 2.0) {
                        path_builder.push_oval(inner_rect);
                    }
                }
            }
            '1' => {
                // Draw thick rectangle for 1 (avoid hairline)
                if let Some(rect) = tiny_skia::Rect::from_xywh(x + char_width * 0.4, y - char_height, stroke_width * 2.0, char_height) {
                    path_builder.push_rect(rect);
                }
            }
            '2'..='9' => {
                // Draw filled rectangle for other numbers
                if let Some(rect) = tiny_skia::Rect::from_xywh(x, y - char_height, char_width * 0.7, char_height) {
                    path_builder.push_rect(rect);
                }
            }
            // Letters - simplified filled shapes
            'A' | 'a' => {
                // Triangle shape for A (filled)
                path_builder.move_to(x + char_width * 0.5, y - char_height);
                path_builder.line_to(x + char_width * 0.1, y);
                path_builder.line_to(x + char_width * 0.9, y);
                path_builder.close();
            }
            'B' | 'b' | 'D' | 'd' | 'O' | 'o' | 'P' | 'p' | 'Q' | 'q' | 'R' | 'r' => {
                // Filled oval for round letters
                if let Some(rect) = tiny_skia::Rect::from_xywh(x, y - char_height, char_width * 0.8, char_height) {
                    path_builder.push_oval(rect);
                }
            }
            'I' | 'i' | 'J' | 'j' | 'L' | 'l' => {
                // Thick rectangle for thin letters (avoid hairline)
                if let Some(rect) = tiny_skia::Rect::from_xywh(x + char_width * 0.4, y - char_height, stroke_width * 2.0, char_height) {
                    path_builder.push_rect(rect);
                }
            }
            _ => {
                // Default filled rectangle for other characters
                if let Some(rect) = tiny_skia::Rect::from_xywh(x, y - char_height, char_width * 0.7, char_height) {
                    path_builder.push_rect(rect);
                }
            }
        }
        
        if let Some(path) = path_builder.finish() {
            self.pixmap.fill_path(&path, paint, FillRule::EvenOdd, Transform::identity(), None);
        }
        
        Ok(())
    }
    
    fn draw_glyph_bitmap(&mut self, bitmap: &[u8], width: usize, height: usize, x: f32, y: f32, color: Color) -> Result<()> {
        let (r, g, b, _a) = color.to_rgba_f32();
        
        for (i, &alpha) in bitmap.iter().enumerate() {
            if alpha > 0 {
                let bitmap_x = i % width;
                let bitmap_y = i / width;
                
                let pixel_x = (x + bitmap_x as f32).round() as i32;
                let pixel_y = (y + bitmap_y as f32).round() as i32;
                
                if pixel_x >= 0 && pixel_y >= 0 && 
                   pixel_x < self.width as i32 && pixel_y < self.height as i32 {
                    
                    // Simple linear alpha blending - no gamma correction
                    let alpha_f = alpha as f32 / 255.0;
                    
                    // Get existing pixel
                    let existing = self.pixmap.pixel(pixel_x as u32, pixel_y as u32)
                        .unwrap_or(tiny_skia::PremultipliedColorU8::TRANSPARENT);
                    
                    // Simple alpha blend
                    let new_r = ((1.0 - alpha_f) * existing.red() as f32 + alpha_f * r * 255.0) as u8;
                    let new_g = ((1.0 - alpha_f) * existing.green() as f32 + alpha_f * g * 255.0) as u8;
                    let new_b = ((1.0 - alpha_f) * existing.blue() as f32 + alpha_f * b * 255.0) as u8;
                    let new_a = ((1.0 - alpha_f) * existing.alpha() as f32 + alpha_f * 255.0) as u8;
                    
                    // Create non-premultiplied color first, then convert
                    let final_color = tiny_skia::ColorU8::from_rgba(new_r, new_g, new_b, new_a);
                    self.pixmap.pixels_mut()[pixel_y as usize * self.width as usize + pixel_x as usize] = 
                        final_color.premultiply();
                }
            }
        }
        
        Ok(())
    }
    
    /// Draw character with proper gamma-corrected anti-aliasing
    fn draw_antialiased_char(&mut self, bitmap: &[u8], width: usize, height: usize, 
                           x: f32, y: f32, color: Color) -> Result<()> {
        let (r, g, b, a) = color.to_rgba_f32();
        
        for (i, &alpha) in bitmap.iter().enumerate() {
            if alpha > 0 {
                let bitmap_x = i % width;
                let bitmap_y = i / width;
                
                let pixel_x = (x + bitmap_x as f32).round() as i32;
                let pixel_y = (y + bitmap_y as f32).round() as i32;
                
                if pixel_x >= 0 && pixel_y >= 0 && 
                   pixel_x < self.width as i32 && pixel_y < self.height as i32 {
                    
                    // Apply gamma correction for better anti-aliasing
                    let gamma_corrected_alpha = (alpha as f32 / 255.0).powf(2.2);
                    
                    // Convert to tiny-skia color with proper alpha
                    let final_alpha = (gamma_corrected_alpha * a * 255.0) as u8;
                    let premult_r = ((r * gamma_corrected_alpha * a * 255.0) as u8).min(final_alpha);
                    let premult_g = ((g * gamma_corrected_alpha * a * 255.0) as u8).min(final_alpha);
                    let premult_b = ((b * gamma_corrected_alpha * a * 255.0) as u8).min(final_alpha);
                    
                    if let Some(pixel_color) = tiny_skia::PremultipliedColorU8::from_rgba(
                        premult_r, premult_g, premult_b, final_alpha
                    ) {
                        // Get existing pixel for proper blending
                        let existing_pixel = self.pixmap.pixel(pixel_x as u32, pixel_y as u32)
                            .unwrap_or(tiny_skia::PremultipliedColorU8::TRANSPARENT);
                        
                        // Blend with existing pixel
                        let blended = self.alpha_blend_pixels(existing_pixel, pixel_color);
                        
                        self.pixmap.pixels_mut()[(pixel_y as usize * self.width as usize + pixel_x as usize)] = blended;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Proper alpha blending of two pixels
    fn alpha_blend_pixels(&self, background: tiny_skia::PremultipliedColorU8, 
                         foreground: tiny_skia::PremultipliedColorU8) -> tiny_skia::PremultipliedColorU8 {
        let fg_a = foreground.alpha() as f32 / 255.0;
        let bg_a = background.alpha() as f32 / 255.0;
        let inv_fg_a = 1.0 - fg_a;
        
        let final_a = fg_a + bg_a * inv_fg_a;
        
        if final_a > 0.0 {
            let final_r = ((foreground.red() as f32 + background.red() as f32 * inv_fg_a) / final_a) as u8;
            let final_g = ((foreground.green() as f32 + background.green() as f32 * inv_fg_a) / final_a) as u8;
            let final_b = ((foreground.blue() as f32 + background.blue() as f32 * inv_fg_a) / final_a) as u8;
            let final_alpha = (final_a * 255.0) as u8;
            
            tiny_skia::PremultipliedColorU8::from_rgba(final_r, final_g, final_b, final_alpha)
                .unwrap_or(background)
        } else {
            background
        }
    }

    /// Draw text with rotation support (simplified for now)
    pub fn draw_text_with_rotation(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color, rotation_degrees: f32) -> Result<()> {
        if rotation_degrees.abs() < 0.01 {
            // No rotation needed, use regular text
            self.draw_text_fontdue_improved(text, x, y, size, color)
        } else {
            // For now, fall back to vector text for rotation
            let char_width = size * 0.6;
            let char_height = size;
            let mut current_x = x;
            
            for ch in text.chars() {
                self.draw_simple_char(ch, current_x, y, char_width, char_height, color)?;
                current_x += char_width;
            }
            Ok(())
        }
    }
    
    /// Draw a character bitmap onto the pixmap
    fn draw_character_bitmap(&mut self, bitmap: &[u8], width: usize, height: usize, 
                           x: f32, y: f32, color: Color, rotation_degrees: f32) -> Result<()> {
        let (r, g, b, a) = color.to_rgba_f32();
        let r = (r * 255.0) as u8;
        let g = (g * 255.0) as u8;
        let b = (b * 255.0) as u8;
        let a = (a * 255.0) as u8;
        // Color components will be used for blending
        
        // Calculate rotation if needed
        let cos_r = rotation_degrees.to_radians().cos();
        let sin_r = rotation_degrees.to_radians().sin();
        
        for (i, &alpha) in bitmap.iter().enumerate() {
            if alpha > 0 {
                let bitmap_x = i % width;
                let bitmap_y = i / width;
                
                // Apply rotation if needed
                let (final_x, final_y) = if rotation_degrees.abs() > 0.01 {
                    let centered_x = bitmap_x as f32 - width as f32 / 2.0;
                    let centered_y = bitmap_y as f32 - height as f32 / 2.0;
                    
                    let rotated_x = centered_x * cos_r - centered_y * sin_r;
                    let rotated_y = centered_x * sin_r + centered_y * cos_r;
                    
                    (x + rotated_x + width as f32 / 2.0, y + rotated_y + height as f32 / 2.0)
                } else {
                    (x + bitmap_x as f32, y + bitmap_y as f32)
                };
                
                let pixel_x = final_x as usize;
                let pixel_y = final_y as usize;
                
                if pixel_x < self.width as usize && pixel_y < self.height as usize {
                    // Alpha blend the character with the background
                    let alpha_f = alpha as f32 / 255.0;
                    let inv_alpha = 1.0 - alpha_f;
                    
                    // Get existing pixel data directly from pixmap data
                    let pixel_data = self.pixmap.data();
                    let pixel_index = (pixel_y * self.width as usize + pixel_x) * 4;
                    if pixel_index + 3 < pixel_data.len() {
                        let existing_b = pixel_data[pixel_index] as f32;     // B
                        let existing_g = pixel_data[pixel_index + 1] as f32; // G  
                        let existing_r = pixel_data[pixel_index + 2] as f32; // R
                        
                        let new_r = (r as f32 * alpha_f + existing_r * inv_alpha) as u8;
                        let new_g = (g as f32 * alpha_f + existing_g * inv_alpha) as u8;
                        let new_b = (b as f32 * alpha_f + existing_b * inv_alpha) as u8;
                        let new_a = 255u8; // Assume opaque background
                        
                        // Create new color for blending
                        
                        unsafe {
                            let pixel_data = self.pixmap.data_mut();
                            let pixel_index = (pixel_y * self.width as usize + pixel_x) * 4;
                            if pixel_index + 3 < pixel_data.len() {
                                pixel_data[pixel_index] = new_b;     // B
                                pixel_data[pixel_index + 1] = new_g; // G  
                                pixel_data[pixel_index + 2] = new_r; // R
                                pixel_data[pixel_index + 3] = new_a; // A
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Draw a simple character using basic paths
    fn draw_simple_char(&mut self, ch: char, x: f32, y: f32, width: f32, height: f32, color: Color) -> Result<()> {
        let mut path_builder = tiny_skia_path::PathBuilder::new();
        let stroke_width = width * 0.1;
        
        // Define simple character shapes using paths
        match ch {
            'A' | 'a' => {
                // Draw 'A' as triangle with crossbar
                path_builder.move_to(x + width * 0.5, y);
                path_builder.line_to(x, y + height);
                path_builder.move_to(x + width * 0.5, y);
                path_builder.line_to(x + width, y + height);
                path_builder.move_to(x + width * 0.25, y + height * 0.6);
                path_builder.line_to(x + width * 0.75, y + height * 0.6);
            }
            'B' | 'b' => {
                // Draw 'B' as vertical line with two bumps
                path_builder.move_to(x, y);
                path_builder.line_to(x, y + height);
                path_builder.move_to(x, y);
                path_builder.line_to(x + width * 0.7, y);
                path_builder.line_to(x + width * 0.7, y + height * 0.5);
                path_builder.line_to(x, y + height * 0.5);
                path_builder.move_to(x, y + height * 0.5);
                path_builder.line_to(x + width * 0.8, y + height * 0.5);
                path_builder.line_to(x + width * 0.8, y + height);
                path_builder.line_to(x, y + height);
            }
            'C' | 'c' => {
                // Draw 'C' as arc
                path_builder.move_to(x + width, y + height * 0.2);
                path_builder.line_to(x + width * 0.2, y + height * 0.2);
                path_builder.line_to(x + width * 0.2, y + height * 0.8);
                path_builder.line_to(x + width, y + height * 0.8);
            }
            'D' | 'd' => {
                // Draw 'D' as rectangle with curved right side
                path_builder.move_to(x, y);
                path_builder.line_to(x, y + height);
                path_builder.line_to(x + width * 0.7, y + height);
                path_builder.line_to(x + width * 0.7, y);
                path_builder.line_to(x, y);
            }
            'E' | 'e' => {
                // Draw 'E' as lines
                path_builder.move_to(x, y);
                path_builder.line_to(x, y + height);
                path_builder.move_to(x, y);
                path_builder.line_to(x + width, y);
                path_builder.move_to(x, y + height * 0.5);
                path_builder.line_to(x + width * 0.8, y + height * 0.5);
                path_builder.move_to(x, y + height);
                path_builder.line_to(x + width, y + height);
            }
            'F' | 'f' => {
                // Draw 'F' as lines
                path_builder.move_to(x, y);
                path_builder.line_to(x, y + height);
                path_builder.move_to(x, y);
                path_builder.line_to(x + width, y);
                path_builder.move_to(x, y + height * 0.5);
                path_builder.line_to(x + width * 0.8, y + height * 0.5);
            }
            'G' | 'g' => {
                // Draw 'G' as C with horizontal line
                path_builder.move_to(x + width, y + height * 0.2);
                path_builder.line_to(x + width * 0.2, y + height * 0.2);
                path_builder.line_to(x + width * 0.2, y + height * 0.8);
                path_builder.line_to(x + width, y + height * 0.8);
                path_builder.move_to(x + width * 0.6, y + height * 0.5);
                path_builder.line_to(x + width, y + height * 0.5);
            }
            'H' | 'h' => {
                // Draw 'H' as two vertical lines with crossbar
                path_builder.move_to(x, y);
                path_builder.line_to(x, y + height);
                path_builder.move_to(x + width, y);
                path_builder.line_to(x + width, y + height);
                path_builder.move_to(x, y + height * 0.5);
                path_builder.line_to(x + width, y + height * 0.5);
            }
            'I' | 'i' => {
                // Draw 'I' as vertical line with serifs
                path_builder.move_to(x + width * 0.2, y);
                path_builder.line_to(x + width * 0.8, y);
                path_builder.move_to(x + width * 0.5, y);
                path_builder.line_to(x + width * 0.5, y + height);
                path_builder.move_to(x + width * 0.2, y + height);
                path_builder.line_to(x + width * 0.8, y + height);
            }
            'L' | 'l' => {
                // Draw 'L' as two lines
                path_builder.move_to(x, y);
                path_builder.line_to(x, y + height);
                path_builder.line_to(x + width, y + height);
            }
            'M' | 'm' => {
                // Draw 'M' as mountain shape
                path_builder.move_to(x, y + height);
                path_builder.line_to(x, y);
                path_builder.line_to(x + width * 0.5, y + height * 0.4);
                path_builder.line_to(x + width, y);
                path_builder.line_to(x + width, y + height);
            }
            'N' | 'n' => {
                // Draw 'N' as two verticals with diagonal
                path_builder.move_to(x, y + height);
                path_builder.line_to(x, y);
                path_builder.line_to(x + width, y + height);
                path_builder.line_to(x + width, y);
            }
            'O' | 'o' => {
                // Draw 'O' as rectangle outline
                path_builder.move_to(x + width * 0.2, y + height * 0.2);
                path_builder.line_to(x + width * 0.8, y + height * 0.2);
                path_builder.line_to(x + width * 0.8, y + height * 0.8);
                path_builder.line_to(x + width * 0.2, y + height * 0.8);
                path_builder.line_to(x + width * 0.2, y + height * 0.2);
            }
            'P' | 'p' => {
                // Draw 'P' as vertical line with top bump
                path_builder.move_to(x, y);
                path_builder.line_to(x, y + height);
                path_builder.move_to(x, y);
                path_builder.line_to(x + width * 0.8, y);
                path_builder.line_to(x + width * 0.8, y + height * 0.5);
                path_builder.line_to(x, y + height * 0.5);
            }
            'R' | 'r' => {
                // Draw 'R' as P with diagonal leg
                path_builder.move_to(x, y);
                path_builder.line_to(x, y + height);
                path_builder.move_to(x, y);
                path_builder.line_to(x + width * 0.8, y);
                path_builder.line_to(x + width * 0.8, y + height * 0.5);
                path_builder.line_to(x, y + height * 0.5);
                path_builder.move_to(x + width * 0.4, y + height * 0.5);
                path_builder.line_to(x + width, y + height);
            }
            'S' | 's' => {
                // Draw 'S' as zigzag
                path_builder.move_to(x + width, y + height * 0.2);
                path_builder.line_to(x + width * 0.2, y + height * 0.2);
                path_builder.line_to(x + width * 0.2, y + height * 0.5);
                path_builder.line_to(x + width * 0.8, y + height * 0.5);
                path_builder.line_to(x + width * 0.8, y + height * 0.8);
                path_builder.line_to(x, y + height * 0.8);
            }
            'T' | 't' => {
                // Draw 'T' as horizontal and vertical lines
                path_builder.move_to(x, y);
                path_builder.line_to(x + width, y);
                path_builder.move_to(x + width * 0.5, y);
                path_builder.line_to(x + width * 0.5, y + height);
            }
            'U' | 'u' => {
                // Draw 'U' as arc
                path_builder.move_to(x, y);
                path_builder.line_to(x, y + height * 0.8);
                path_builder.line_to(x + width, y + height * 0.8);
                path_builder.line_to(x + width, y);
            }
            'V' | 'v' => {
                // Draw 'V' as two angled lines
                path_builder.move_to(x, y);
                path_builder.line_to(x + width * 0.5, y + height);
                path_builder.line_to(x + width, y);
            }
            'W' | 'w' => {
                // Draw 'W' as inverted mountain
                path_builder.move_to(x, y);
                path_builder.line_to(x + width * 0.25, y + height);
                path_builder.line_to(x + width * 0.5, y + height * 0.6);
                path_builder.line_to(x + width * 0.75, y + height);
                path_builder.line_to(x + width, y);
            }
            'X' | 'x' => {
                // Draw 'X' as two diagonal lines
                path_builder.move_to(x, y);
                path_builder.line_to(x + width, y + height);
                path_builder.move_to(x + width, y);
                path_builder.line_to(x, y + height);
            }
            'Y' | 'y' => {
                // Draw 'Y' as two angled lines meeting at center
                path_builder.move_to(x, y);
                path_builder.line_to(x + width * 0.5, y + height * 0.5);
                path_builder.line_to(x + width, y);
                path_builder.move_to(x + width * 0.5, y + height * 0.5);
                path_builder.line_to(x + width * 0.5, y + height);
            }
            'Z' | 'z' => {
                // Draw 'Z' as zigzag
                path_builder.move_to(x, y);
                path_builder.line_to(x + width, y);
                path_builder.line_to(x, y + height);
                path_builder.line_to(x + width, y + height);
            }
            '0'..='9' => {
                // Draw numbers as simple shapes
                match ch {
                    '0' => {
                        path_builder.move_to(x + width * 0.2, y + height * 0.2);
                        path_builder.line_to(x + width * 0.8, y + height * 0.2);
                        path_builder.line_to(x + width * 0.8, y + height * 0.8);
                        path_builder.line_to(x + width * 0.2, y + height * 0.8);
                        path_builder.line_to(x + width * 0.2, y + height * 0.2);
                    }
                    '1' => {
                        path_builder.move_to(x + width * 0.5, y);
                        path_builder.line_to(x + width * 0.5, y + height);
                    }
                    '2' => {
                        path_builder.move_to(x, y + height * 0.2);
                        path_builder.line_to(x + width, y + height * 0.2);
                        path_builder.line_to(x + width, y + height * 0.5);
                        path_builder.line_to(x, y + height * 0.5);
                        path_builder.line_to(x, y + height);
                        path_builder.line_to(x + width, y + height);
                    }
                    '3' => {
                        path_builder.move_to(x, y);
                        path_builder.line_to(x + width, y);
                        path_builder.line_to(x + width, y + height * 0.5);
                        path_builder.line_to(x + width * 0.5, y + height * 0.5);
                        path_builder.move_to(x + width, y + height * 0.5);
                        path_builder.line_to(x + width, y + height);
                        path_builder.line_to(x, y + height);
                    }
                    _ => {
                        // Default: draw as rectangle for other numbers
                        path_builder.move_to(x + width * 0.2, y + height * 0.2);
                        path_builder.line_to(x + width * 0.8, y + height * 0.2);
                        path_builder.line_to(x + width * 0.8, y + height * 0.8);
                        path_builder.line_to(x + width * 0.2, y + height * 0.8);
                        path_builder.line_to(x + width * 0.2, y + height * 0.2);
                    }
                }
            }
            ' ' => {
                // Space - draw nothing
                return Ok(());
            }
            '.' => {
                // Period - small rectangle at bottom
                path_builder.move_to(x + width * 0.4, y + height * 0.8);
                path_builder.line_to(x + width * 0.6, y + height * 0.8);
                path_builder.line_to(x + width * 0.6, y + height);
                path_builder.line_to(x + width * 0.4, y + height);
                path_builder.line_to(x + width * 0.4, y + height * 0.8);
            }
            ',' => {
                // Comma - small rectangle with tail
                path_builder.move_to(x + width * 0.4, y + height * 0.8);
                path_builder.line_to(x + width * 0.6, y + height * 0.8);
                path_builder.line_to(x + width * 0.5, y + height * 1.1);
            }
            ':' => {
                // Colon - two dots
                path_builder.move_to(x + width * 0.4, y + height * 0.3);
                path_builder.line_to(x + width * 0.6, y + height * 0.3);
                path_builder.line_to(x + width * 0.6, y + height * 0.4);
                path_builder.line_to(x + width * 0.4, y + height * 0.4);
                path_builder.line_to(x + width * 0.4, y + height * 0.3);
                path_builder.move_to(x + width * 0.4, y + height * 0.6);
                path_builder.line_to(x + width * 0.6, y + height * 0.6);
                path_builder.line_to(x + width * 0.6, y + height * 0.7);
                path_builder.line_to(x + width * 0.4, y + height * 0.7);
                path_builder.line_to(x + width * 0.4, y + height * 0.6);
            }
            '-' => {
                // Minus/hyphen - horizontal line
                path_builder.move_to(x + width * 0.2, y + height * 0.5);
                path_builder.line_to(x + width * 0.8, y + height * 0.5);
            }
            '+' => {
                // Plus - horizontal and vertical lines
                path_builder.move_to(x + width * 0.2, y + height * 0.5);
                path_builder.line_to(x + width * 0.8, y + height * 0.5);
                path_builder.move_to(x + width * 0.5, y + height * 0.2);
                path_builder.line_to(x + width * 0.5, y + height * 0.8);
            }
            '(' => {
                // Left parenthesis
                path_builder.move_to(x + width * 0.6, y);
                path_builder.line_to(x + width * 0.2, y + height * 0.5);
                path_builder.line_to(x + width * 0.6, y + height);
            }
            ')' => {
                // Right parenthesis
                path_builder.move_to(x + width * 0.4, y);
                path_builder.line_to(x + width * 0.8, y + height * 0.5);
                path_builder.line_to(x + width * 0.4, y + height);
            }
            _ => {
                // Unknown character - draw as small rectangle
                path_builder.move_to(x + width * 0.3, y + height * 0.3);
                path_builder.line_to(x + width * 0.7, y + height * 0.3);
                path_builder.line_to(x + width * 0.7, y + height * 0.7);
                path_builder.line_to(x + width * 0.3, y + height * 0.7);
                path_builder.line_to(x + width * 0.3, y + height * 0.3);
            }
        }
        
        if let Some(path) = path_builder.finish() {
            let mut paint = Paint::default();
            paint.set_color(color.to_tiny_skia_color());
            paint.anti_alias = true;
            
            let stroke = Stroke {
                width: stroke_width,
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..Stroke::default()
            };
            
            self.pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
        
        Ok(())
    }
    
    /// Draw axis labels and tick values
    pub fn draw_axis_labels(&mut self, plot_area: Rect, x_min: f64, x_max: f64, y_min: f64, y_max: f64, 
                           x_label: &str, y_label: &str, color: Color, label_size: f32, dpi_scale: f32) -> Result<()> {
        let tick_size = label_size * 0.7; // Tick labels slightly smaller than axis labels
        
        // Scale all positioning offsets with DPI
        let tick_offset_x = 15.0 * dpi_scale;
        let tick_offset_y = 20.0 * dpi_scale;
        let y_tick_offset = 50.0 * dpi_scale;
        let y_tick_baseline = 5.0 * dpi_scale;
        let x_label_offset = 50.0 * dpi_scale;
        let y_label_offset = 25.0 * dpi_scale;
        let char_width_estimate = 4.0 * dpi_scale; // Rough character width for centering
        
        // Generate and draw X-axis tick labels
        let x_ticks = generate_ticks(x_min, x_max, 5);
        for (i, &tick_value) in x_ticks.iter().enumerate() {
            let x_pixel = plot_area.left() + (tick_value - x_min) as f32 / (x_max - x_min) as f32 * plot_area.width();
            let label_text = if tick_value.abs() < 0.001 {
                "0".to_string()
            } else if tick_value.abs() > 1000.0 {
                format!("{:.0e}", tick_value)
            } else {
                format!("{:.1}", tick_value)
            };
            
            self.draw_text(&label_text, x_pixel - tick_offset_x, plot_area.bottom() + tick_offset_y, tick_size, color)?;
        }
        
        // Generate and draw Y-axis tick labels
        let y_ticks = generate_ticks(y_min, y_max, 5);
        for (i, &tick_value) in y_ticks.iter().enumerate() {
            let y_pixel = plot_area.bottom() - (tick_value - y_min) as f32 / (y_max - y_min) as f32 * plot_area.height();
            let label_text = if tick_value.abs() < 0.001 {
                "0".to_string()
            } else if tick_value.abs() > 1000.0 {
                format!("{:.0e}", tick_value)
            } else {
                format!("{:.1}", tick_value)
            };
            
            self.draw_text(&label_text, plot_area.left() - y_tick_offset, y_pixel + y_tick_baseline, tick_size, color)?;
        }
        
        // Draw X-axis label
        let x_label_x = plot_area.left() + plot_area.width() / 2.0 - x_label.len() as f32 * char_width_estimate;
        let x_label_y = plot_area.bottom() + x_label_offset;
        self.draw_text(x_label, x_label_x, x_label_y, label_size, color)?;
        
        // Draw Y-axis label (rotated 90 degrees counterclockwise)
        let y_label_x = y_label_offset;
        let y_label_y = plot_area.top() + plot_area.height() / 2.0;
        self.draw_text_rotated(y_label, y_label_x, y_label_y, label_size, color)?;
        
        Ok(())
    }
    
    /// Draw title
    pub fn draw_title(&mut self, title: &str, plot_area: Rect, color: Color, title_size: f32, dpi_scale: f32) -> Result<()> {
        let char_width_estimate = 5.0 * dpi_scale; // Rough character width for centering
        let title_offset = 20.0 * dpi_scale; // Offset from plot area top
        
        let title_x = plot_area.left() + plot_area.width() / 2.0 - title.len() as f32 * char_width_estimate;
        let title_y = plot_area.top() - title_offset;
        self.draw_text(title, title_x, title_y, title_size, color)
    }
    
    /// Draw legend
    pub fn draw_legend(&mut self, legend_items: &[(String, Color)], plot_area: Rect) -> Result<()> {
        if legend_items.is_empty() {
            return Ok(());
        }
        
        let legend_size = 12.0;
        let legend_spacing = 20.0;
        let legend_x = plot_area.right() - 150.0;
        let mut legend_y = plot_area.top() + 30.0;
        
        // Draw legend background (simple rectangle)
        let legend_bg = Rect::from_xywh(
            legend_x - 10.0, 
            legend_y - 15.0,
            140.0,
            legend_items.len() as f32 * legend_spacing + 10.0
        ).ok_or(PlottingError::InvalidData { 
            message: "Invalid legend dimensions".to_string(),
            position: None 
        })?;
        
        self.draw_rectangle(
            legend_bg.left(), 
            legend_bg.top(), 
            legend_bg.width(), 
            legend_bg.height(), 
            Color::new_rgba(255, 255, 255, 200), 
            true
        )?;
        
        // Draw legend items
        for (label, color) in legend_items {
            // Draw color square
            let color_rect = Rect::from_xywh(legend_x, legend_y - 8.0, 12.0, 12.0)
                .ok_or(PlottingError::InvalidData { 
                message: "Invalid legend item dimensions".to_string(),
                position: None 
            })?;
            self.draw_rectangle(
                color_rect.left(), 
                color_rect.top(), 
                color_rect.width(), 
                color_rect.height(), 
                *color, 
                true
            )?;
            
            // Draw label text
            self.draw_text(label, legend_x + 20.0, legend_y, legend_size, Color::new_rgba(0, 0, 0, 255))?;
            
            legend_y += legend_spacing;
        }
        
        Ok(())
    }

    /// Draw legend with configurable position
    pub fn draw_legend_positioned(&mut self, legend_items: &[(String, Color)], plot_area: Rect, position: crate::core::Position) -> Result<()> {
        if legend_items.is_empty() {
            return Ok(());
        }
        
        let legend_size = 12.0;
        let legend_spacing = 20.0;
        let legend_width = 140.0;
        let legend_height = legend_items.len() as f32 * legend_spacing + 10.0;
        
        // Calculate legend position based on position enum
        let center_x = plot_area.left() + plot_area.width() / 2.0;
        let center_y = plot_area.top() + plot_area.height() / 2.0;
        
        let (legend_x, legend_y) = match position {
            crate::core::Position::TopLeft => (plot_area.left() + 10.0, plot_area.top() + 10.0),
            crate::core::Position::TopCenter => (center_x - legend_width / 2.0, plot_area.top() + 10.0),
            crate::core::Position::TopRight => (plot_area.right() - legend_width - 10.0, plot_area.top() + 10.0),
            crate::core::Position::CenterLeft => (plot_area.left() + 10.0, center_y - legend_height / 2.0),
            crate::core::Position::Center => (center_x - legend_width / 2.0, center_y - legend_height / 2.0),
            crate::core::Position::CenterRight => (plot_area.right() - legend_width - 10.0, center_y - legend_height / 2.0),
            crate::core::Position::BottomLeft => (plot_area.left() + 10.0, plot_area.bottom() - legend_height - 10.0),
            crate::core::Position::BottomCenter => (center_x - legend_width / 2.0, plot_area.bottom() - legend_height - 10.0),
            crate::core::Position::BottomRight => (plot_area.right() - legend_width - 10.0, plot_area.bottom() - legend_height - 10.0),
            crate::core::Position::Custom { x, y } => (x, y),
        };
        
        // Draw legend background (simple rectangle)
        let legend_bg = Rect::from_xywh(
            legend_x - 10.0, 
            legend_y - 5.0,
            legend_width,
            legend_height
        ).ok_or(PlottingError::InvalidData { 
            message: "Invalid legend dimensions".to_string(),
            position: None 
        })?;
        
        self.draw_rectangle(
            legend_bg.left(), 
            legend_bg.top(), 
            legend_bg.width(), 
            legend_bg.height(), 
            Color::new_rgba(255, 255, 255, 200), 
            true
        )?;
        
        // Draw legend items
        let mut item_y = legend_y + 10.0;
        for (label, color) in legend_items {
            // Draw color square
            let color_rect = Rect::from_xywh(legend_x, item_y - 8.0, 12.0, 12.0)
                .ok_or(PlottingError::InvalidData { 
                message: "Invalid legend item dimensions".to_string(),
                position: None 
            })?;
            self.draw_rectangle(
                color_rect.left(), 
                color_rect.top(), 
                color_rect.width(), 
                color_rect.height(), 
                *color, 
                true
            )?;
            
            // Draw label text
            self.draw_text(label, legend_x + 20.0, item_y, legend_size, Color::new_rgba(0, 0, 0, 255))?;
            
            item_y += legend_spacing;
        }
        
        Ok(())
    }
    
    /// Get the current pixmap as an Image
    pub fn to_image(self) -> Image {
        Image {
            width: self.width,
            height: self.height,
            pixels: self.pixmap.data().to_vec(),
        }
    }
    
    /// Save the current pixmap as PNG
    pub fn save_png<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.pixmap.save_png(path)
            .map_err(|_| PlottingError::IoError(
                std::io::Error::new(std::io::ErrorKind::Other, "Failed to save PNG")
            ))
    }
    
    /// Export as SVG (simplified - tiny-skia doesn't directly support SVG export)
    pub fn export_svg<P: AsRef<Path>>(&self, path: P, width: u32, height: u32) -> Result<()> {
        // For now, create a basic SVG placeholder
        // In a real implementation, we'd need to track draw commands and convert to SVG
        let svg_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">
  <rect width="100%" height="100%" fill="{}"/>
  <text x="50%" y="50%" text-anchor="middle" font-family="Arial" font-size="16">
    Ruviz Plot ({} x {})
  </text>
</svg>"#,
            width, height,
            self.theme.background,
            width, height
        );
        
        std::fs::write(path, svg_content)
            .map_err(PlottingError::IoError)?;
        
        Ok(())
    }
}

/// Helper function to calculate plot area with margins
pub fn calculate_plot_area(canvas_width: u32, canvas_height: u32, margin_fraction: f32) -> Rect {
    let margin_x = (canvas_width as f32) * margin_fraction;
    let margin_y = (canvas_height as f32) * margin_fraction;
    
    Rect::from_xywh(
        margin_x,
        margin_y,
        (canvas_width as f32) - 2.0 * margin_x,
        (canvas_height as f32) - 2.0 * margin_y,
    ).unwrap_or_else(|| Rect::from_xywh(10.0, 10.0, (canvas_width as f32) - 20.0, (canvas_height as f32) - 20.0).unwrap())
}

/// Helper function to map data coordinates to pixel coordinates
pub fn map_data_to_pixels(
    data_x: f64, data_y: f64,
    data_x_min: f64, data_x_max: f64,
    data_y_min: f64, data_y_max: f64,
    plot_area: Rect,
) -> (f32, f32) {
    let pixel_x = plot_area.left() + 
        ((data_x - data_x_min) / (data_x_max - data_x_min)) as f32 * plot_area.width();
    
    // Note: Y axis is flipped in screen coordinates
    let pixel_y = plot_area.bottom() - 
        ((data_y - data_y_min) / (data_y_max - data_y_min)) as f32 * plot_area.height();
    
    (pixel_x, pixel_y)
}

/// Generate nice tick values for an axis
pub fn generate_ticks(min: f64, max: f64, target_count: usize) -> Vec<f64> {
    if min >= max || target_count == 0 {
        return vec![min, max];
    }
    
    let range = max - min;
    let rough_step = range / (target_count as f64);
    
    // Find a "nice" step size
    let magnitude = 10.0_f64.powf(rough_step.log10().floor());
    let normalized_step = rough_step / magnitude;
    
    let nice_step = if normalized_step <= 1.0 {
        1.0
    } else if normalized_step <= 2.0 {
        2.0
    } else if normalized_step <= 5.0 {
        5.0
    } else {
        10.0
    };
    
    let step = nice_step * magnitude;
    let start = (min / step).ceil() * step;
    
    let mut ticks = Vec::new();
    let mut tick = start;
    while tick <= max + step * 0.001 { // Small epsilon for floating point comparison
        ticks.push(tick);
        tick += step;
    }
    
    ticks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_creation() {
        let theme = Theme::default();
        let renderer = SkiaRenderer::new(800, 600, theme);
        assert!(renderer.is_ok());
        
        let renderer = renderer.unwrap();
        assert_eq!(renderer.width, 800);
        assert_eq!(renderer.height, 600);
    }

    #[test]
    fn test_plot_area_calculation() {
        let area = calculate_plot_area(800, 600, 0.1);
        assert_eq!(area.left(), 80.0);
        assert_eq!(area.top(), 60.0);
        assert_eq!(area.width(), 640.0);
        assert_eq!(area.height(), 480.0);
    }

    #[test]
    fn test_data_to_pixel_mapping() {
        let plot_area = Rect::from_xywh(100.0, 100.0, 600.0, 400.0).unwrap();
        let (px, py) = map_data_to_pixels(
            1.5, 2.5,  // data coordinates
            1.0, 2.0,  // data x range
            2.0, 3.0,  // data y range
            plot_area
        );
        
        assert_eq!(px, 400.0); // middle of x range
        assert_eq!(py, 300.0); // middle of y range (flipped)
    }

    #[test]
    fn test_tick_generation() {
        let ticks = generate_ticks(0.0, 10.0, 5);
        assert!(!ticks.is_empty());
        assert!(ticks[0] >= 0.0);
        assert!(ticks.last().unwrap() <= &10.0);
        
        // Test edge case
        let ticks = generate_ticks(5.0, 5.0, 3);
        assert_eq!(ticks, vec![5.0, 5.0]);
    }
}