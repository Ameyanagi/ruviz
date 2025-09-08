use std::path::Path;
use tiny_skia::*;
// use fontdue::{Font, FontSettings}; // Temporarily disabled
use crate::{
    core::{PlottingError, Result, plot::Image},
    render::{Color, LineStyle, MarkerStyle, Theme},
};

/// Tiny-skia based renderer for high-performance plot rendering
pub struct SkiaRenderer {
    width: u32,
    height: u32,
    pixmap: Pixmap,
    paint: Paint<'static>,
    theme: Theme,
    // font: Font, // Temporarily disabled - will implement proper text rendering later
}

impl SkiaRenderer {
    /// Create a new renderer with the given dimensions
    pub fn new(width: u32, height: u32, theme: Theme) -> Result<Self> {
        let mut pixmap = Pixmap::new(width, height)
            .ok_or(PlottingError::OutOfMemory)?;
        
        // Fill background
        let bg_color = theme.background.to_tiny_skia_color();
        pixmap.fill(bg_color);
        
        let paint = Paint::default();
        
        // Load embedded DejaVu Sans font
        // Font loading temporarily disabled - will implement proper text rendering later
        
        Ok(Self {
            width,
            height,
            pixmap,
            paint,
            theme,
            // font, // Temporarily disabled
        })
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

    /// Draw text at the specified position (placeholder implementation)
    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color) -> Result<()> {
        // Temporary placeholder: draw a rectangle where text would be
        // This allows us to test the rest of the functionality
        let text_width = text.len() as f32 * size * 0.6; // Rough estimate
        let text_height = size;
        
        // Draw a simple rectangle as placeholder for text
        self.draw_rectangle(x, y - text_height, text_width, text_height, color, false)?;
        
        Ok(())
    }
    
    /// Draw axis labels and tick values
    pub fn draw_axis_labels(&mut self, plot_area: Rect, x_min: f64, x_max: f64, y_min: f64, y_max: f64, 
                           x_label: &str, y_label: &str, color: Color) -> Result<()> {
        let label_size = 14.0;
        let tick_size = 10.0;
        
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
            
            self.draw_text(&label_text, x_pixel - 15.0, plot_area.bottom() + 20.0, tick_size, color)?;
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
            
            self.draw_text(&label_text, plot_area.left() - 50.0, y_pixel + 5.0, tick_size, color)?;
        }
        
        // Draw X-axis label
        let x_label_x = plot_area.left() + plot_area.width() / 2.0 - x_label.len() as f32 * 4.0;
        let x_label_y = plot_area.bottom() + 50.0;
        self.draw_text(x_label, x_label_x, x_label_y, label_size, color)?;
        
        // Draw Y-axis label (rotated text simulation - just place it vertically)
        let y_label_x = 20.0;
        let y_label_y = plot_area.top() + plot_area.height() / 2.0;
        self.draw_text(y_label, y_label_x, y_label_y, label_size, color)?;
        
        Ok(())
    }
    
    /// Draw title
    pub fn draw_title(&mut self, title: &str, plot_area: Rect, color: Color) -> Result<()> {
        let title_size = 16.0;
        let title_x = plot_area.left() + plot_area.width() / 2.0 - title.len() as f32 * 5.0;
        let title_y = plot_area.top() - 20.0;
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