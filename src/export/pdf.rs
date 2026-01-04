//! PDF export functionality
//!
//! Provides vector-based PDF export for publication-quality output.

use crate::core::{PlottingError, Result};
use crate::render::{Color, LineStyle};
use std::path::Path;

#[cfg(feature = "pdf")]
use printpdf::{
    Color as PdfColor, IndirectFontRef, Line, LineDashPattern, Mm, PdfDocumentReference,
    PdfLayerReference, Point, Pt, Rgb,
};

/// PDF renderer for vector-based plot export
#[cfg(feature = "pdf")]
pub struct PdfRenderer {
    width_mm: f32,
    height_mm: f32,
    doc: PdfDocumentReference,
    current_layer: PdfLayerReference,
    font: IndirectFontRef,
}

#[cfg(feature = "pdf")]
impl PdfRenderer {
    /// Create a new PDF renderer with specified dimensions in millimeters
    pub fn new(width_mm: f64, height_mm: f64, title: &str) -> Result<Self> {
        let w = width_mm as f32;
        let h = height_mm as f32;

        let (doc, page1, layer1) = printpdf::PdfDocument::new(title, Mm(w), Mm(h), "Plot Layer");

        let current_layer = doc.get_page(page1).get_layer(layer1);

        // Use built-in Helvetica font (always available)
        let font = doc
            .add_builtin_font(printpdf::BuiltinFont::Helvetica)
            .map_err(|e| PlottingError::RenderError(format!("Failed to add font: {}", e)))?;

        Ok(Self {
            width_mm: w,
            height_mm: h,
            doc,
            current_layer,
            font,
        })
    }

    /// Convert pixel coordinates to PDF millimeters
    /// Assumes 96 DPI for pixel-to-mm conversion
    fn px_to_mm(&self, px: f32) -> Mm {
        // 1 inch = 25.4 mm, 96 pixels = 1 inch
        Mm(px * 25.4 / 96.0)
    }

    /// Convert Y coordinate (flip for PDF coordinate system where origin is bottom-left)
    fn flip_y(&self, y: f32) -> f32 {
        (self.height_mm * 96.0 / 25.4) - y
    }

    /// Convert Color to PDF Rgb
    fn color_to_pdf(&self, color: Color) -> Rgb {
        Rgb::new(
            color.r as f32 / 255.0,
            color.g as f32 / 255.0,
            color.b as f32 / 255.0,
            None, // ICC profile
        )
    }

    /// Set line dash pattern based on LineStyle
    fn set_line_style(&self, style: &LineStyle) -> Option<LineDashPattern> {
        match style {
            LineStyle::Solid => None,
            LineStyle::Dashed => Some(LineDashPattern {
                dash_1: Some(6),
                gap_1: Some(3),
                dash_2: None,
                gap_2: None,
                dash_3: None,
                gap_3: None,
                offset: 0,
            }),
            LineStyle::Dotted => Some(LineDashPattern {
                dash_1: Some(2),
                gap_1: Some(2),
                dash_2: None,
                gap_2: None,
                dash_3: None,
                gap_3: None,
                offset: 0,
            }),
            LineStyle::DashDot => Some(LineDashPattern {
                dash_1: Some(6),
                gap_1: Some(2),
                dash_2: Some(2),
                gap_2: Some(2),
                dash_3: None,
                gap_3: None,
                offset: 0,
            }),
            LineStyle::DashDotDot => Some(LineDashPattern {
                dash_1: Some(6),
                gap_1: Some(2),
                dash_2: Some(2),
                gap_2: Some(2),
                dash_3: Some(2),
                gap_3: Some(2),
                offset: 0,
            }),
            LineStyle::Custom(_) => None, // Custom patterns not supported in PDF yet
        }
    }

    /// Draw a line from (x1, y1) to (x2, y2)
    pub fn draw_line(
        &self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: Color,
        width: f32,
        style: LineStyle,
    ) {
        let y1_flipped = self.flip_y(y1);
        let y2_flipped = self.flip_y(y2);

        let points = vec![
            (
                Point::new(self.px_to_mm(x1), self.px_to_mm(y1_flipped)),
                false,
            ),
            (
                Point::new(self.px_to_mm(x2), self.px_to_mm(y2_flipped)),
                false,
            ),
        ];

        let line = Line {
            points,
            is_closed: false,
        };

        self.current_layer
            .set_outline_color(PdfColor::Rgb(self.color_to_pdf(color)));
        self.current_layer
            .set_outline_thickness(self.px_to_mm(width).0);

        if let Some(dash) = self.set_line_style(&style) {
            self.current_layer.set_line_dash_pattern(dash);
        }

        self.current_layer.add_line(line);

        // Reset dash pattern
        self.current_layer
            .set_line_dash_pattern(LineDashPattern::default());
    }

    /// Draw a polyline (connected line segments)
    pub fn draw_polyline(&self, points: &[(f32, f32)], color: Color, width: f32, style: LineStyle) {
        if points.len() < 2 {
            return;
        }

        let pdf_points: Vec<(Point, bool)> = points
            .iter()
            .map(|(x, y)| {
                let y_flipped = self.flip_y(*y);
                (
                    Point::new(self.px_to_mm(*x), self.px_to_mm(y_flipped)),
                    false,
                )
            })
            .collect();

        let line = Line {
            points: pdf_points,
            is_closed: false,
        };

        self.current_layer
            .set_outline_color(PdfColor::Rgb(self.color_to_pdf(color)));
        self.current_layer
            .set_outline_thickness(self.px_to_mm(width).0);

        if let Some(dash) = self.set_line_style(&style) {
            self.current_layer.set_line_dash_pattern(dash);
        }

        self.current_layer.add_line(line);

        // Reset dash pattern
        self.current_layer
            .set_line_dash_pattern(LineDashPattern::default());
    }

    /// Draw a filled or stroked rectangle
    pub fn draw_rectangle(
        &self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
        filled: bool,
    ) {
        let y_flipped = self.flip_y(y + height); // Bottom-left corner in PDF coords

        let x_mm = self.px_to_mm(x);
        let y_mm = self.px_to_mm(y_flipped);
        let w_mm = self.px_to_mm(width);
        let h_mm = self.px_to_mm(height);

        let points = vec![
            (Point::new(x_mm, y_mm), false),
            (Point::new(Mm(x_mm.0 + w_mm.0), y_mm), false),
            (Point::new(Mm(x_mm.0 + w_mm.0), Mm(y_mm.0 + h_mm.0)), false),
            (Point::new(x_mm, Mm(y_mm.0 + h_mm.0)), false),
        ];

        let rect = Line {
            points,
            is_closed: true,
        };

        let pdf_color = PdfColor::Rgb(self.color_to_pdf(color));

        if filled {
            self.current_layer.set_fill_color(pdf_color);
        } else {
            self.current_layer.set_outline_color(pdf_color);
            self.current_layer.set_outline_thickness(1.0);
        }

        self.current_layer.add_line(rect);
    }

    /// Draw text at the specified position
    pub fn draw_text(&self, text: &str, x: f32, y: f32, size: f32, _color: Color) {
        let y_flipped = self.flip_y(y);

        self.current_layer.use_text(
            text,
            size,
            self.px_to_mm(x),
            self.px_to_mm(y_flipped),
            &self.font,
        );

        // Note: printpdf doesn't easily support text color per-text-block
        // Text will render in black by default
    }

    /// Draw a circle/marker at the specified position
    pub fn draw_marker(&self, x: f32, y: f32, size: f32, color: Color) {
        let y_flipped = self.flip_y(y);
        let radius = size / 2.0;

        // Approximate circle with polygon
        let num_segments = 16;
        let points: Vec<(Point, bool)> = (0..=num_segments)
            .map(|i| {
                let angle = 2.0 * std::f32::consts::PI * i as f32 / num_segments as f32;
                let px = x + radius * angle.cos();
                let py = y_flipped + radius * angle.sin();
                (Point::new(self.px_to_mm(px), self.px_to_mm(py)), false)
            })
            .collect();

        let circle = Line {
            points,
            is_closed: true,
        };

        self.current_layer
            .set_fill_color(PdfColor::Rgb(self.color_to_pdf(color)));
        self.current_layer.add_line(circle);
    }

    /// Save the PDF to a file
    pub fn save<P: AsRef<Path>>(self, path: P) -> Result<()> {
        let file = std::fs::File::create(path).map_err(PlottingError::IoError)?;

        let mut writer = std::io::BufWriter::new(file);
        self.doc
            .save(&mut writer)
            .map_err(|e| PlottingError::RenderError(format!("Failed to save PDF: {}", e)))?;

        Ok(())
    }

    /// Get width in pixels (at 96 DPI)
    pub fn width_px(&self) -> f32 {
        self.width_mm * 96.0 / 25.4
    }

    /// Get height in pixels (at 96 DPI)
    pub fn height_px(&self) -> f32 {
        self.height_mm * 96.0 / 25.4
    }
}

/// Default page sizes in millimeters
pub mod page_sizes {
    /// A4 paper size (210mm x 297mm)
    pub const A4: (f64, f64) = (210.0, 297.0);
    /// A4 landscape
    pub const A4_LANDSCAPE: (f64, f64) = (297.0, 210.0);
    /// Letter paper size (215.9mm x 279.4mm)
    pub const LETTER: (f64, f64) = (215.9, 279.4);
    /// Letter landscape
    pub const LETTER_LANDSCAPE: (f64, f64) = (279.4, 215.9);
    /// Default plot size (160mm x 120mm) - good for embedding
    pub const PLOT_DEFAULT: (f64, f64) = (160.0, 120.0);
}

#[cfg(test)]
#[cfg(feature = "pdf")]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_renderer_creation() {
        let renderer = PdfRenderer::new(160.0, 120.0, "Test Plot").unwrap();
        assert!(renderer.width_px() > 0.0);
        assert!(renderer.height_px() > 0.0);
    }

    #[test]
    fn test_px_to_mm_conversion() {
        let renderer = PdfRenderer::new(160.0, 120.0, "Test").unwrap();
        // 96 px = 1 inch = 25.4 mm
        let mm = renderer.px_to_mm(96.0);
        assert!((mm.0 - 25.4).abs() < 0.01);
    }

    #[test]
    fn test_color_conversion() {
        let renderer = PdfRenderer::new(160.0, 120.0, "Test").unwrap();
        let color = Color::new(255, 128, 0);
        let pdf_color = renderer.color_to_pdf(color);
        assert!((pdf_color.r - 1.0).abs() < 0.01);
        assert!((pdf_color.g - 0.5).abs() < 0.01);
        assert!((pdf_color.b - 0.0).abs() < 0.01);
    }
}
