use std::path::Path;
use tiny_skia::*;
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
        
        Ok(Self {
            width,
            height,
            pixmap,
            paint,
            theme,
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