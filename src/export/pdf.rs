//! PDF export functionality
//!
//! Provides vector-based PDF export for publication-quality output.

use crate::core::{PlottingError, Result};
use crate::render::{Color, LineStyle};
use std::{cell::RefCell, fmt::Write as _, path::Path};

#[cfg(feature = "pdf")]
#[derive(Debug, Clone, Copy)]
struct PdfRgb {
    r: f32,
    g: f32,
    b: f32,
}

#[cfg(feature = "pdf")]
#[derive(Debug, Clone)]
enum PdfCommand {
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: Color,
        width: f32,
        style: LineStyle,
    },
    Polyline {
        points: Vec<(f32, f32)>,
        color: Color,
        width: f32,
        style: LineStyle,
    },
    Rectangle {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
        filled: bool,
    },
    Text {
        text: String,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
    },
    Marker {
        x: f32,
        y: f32,
        size: f32,
        color: Color,
    },
}

/// PDF renderer for vector-based plot export
#[cfg(feature = "pdf")]
pub struct PdfRenderer {
    width_mm: f32,
    height_mm: f32,
    title: String,
    commands: RefCell<Vec<PdfCommand>>,
}

#[cfg(feature = "pdf")]
impl PdfRenderer {
    /// Create a new PDF renderer with specified dimensions in millimeters
    pub fn new(width_mm: f64, height_mm: f64, title: &str) -> Result<Self> {
        Ok(Self {
            width_mm: width_mm as f32,
            height_mm: height_mm as f32,
            title: title.to_string(),
            commands: RefCell::new(Vec::new()),
        })
    }

    /// Convert pixel coordinates to millimeters.
    /// Assumes 96 DPI for pixel-to-mm conversion.
    fn px_to_mm(&self, px: f32) -> f32 {
        px * 25.4 / 96.0
    }

    /// Convert Y coordinate from top-left origin to bottom-left origin.
    fn flip_y(&self, y: f32) -> f32 {
        self.height_px() - y
    }

    /// Convert Color to normalized RGB components.
    fn color_to_pdf(&self, color: Color) -> PdfRgb {
        PdfRgb {
            r: color.r as f32 / 255.0,
            g: color.g as f32 / 255.0,
            b: color.b as f32 / 255.0,
        }
    }

    fn color_to_svg(&self, color: Color) -> String {
        format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b)
    }

    fn dash_pattern(&self, style: &LineStyle) -> Option<String> {
        match style {
            LineStyle::Solid => None,
            LineStyle::Dashed => Some("6 3".to_string()),
            LineStyle::Dotted => Some("2 2".to_string()),
            LineStyle::DashDot => Some("6 2 2 2".to_string()),
            LineStyle::DashDotDot => Some("6 2 2 2 2 2".to_string()),
            LineStyle::Custom(pattern) if !pattern.is_empty() => Some(
                pattern
                    .iter()
                    .map(|value| format!("{value:.2}"))
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            LineStyle::Custom(_) => None,
        }
    }

    fn escape_xml(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    fn build_svg(&self) -> String {
        let width_px = self.width_px();
        let height_px = self.height_px();
        let mut svg = String::new();
        let _ = writeln!(
            svg,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width_px:.2}\" height=\"{height_px:.2}\" viewBox=\"0 0 {width_px:.2} {height_px:.2}\">"
        );
        let _ = writeln!(svg, "<title>{}</title>", Self::escape_xml(&self.title));

        for command in self.commands.borrow().iter() {
            match command {
                PdfCommand::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    color,
                    width,
                    style,
                } => {
                    let dash = self
                        .dash_pattern(style)
                        .map(|pattern| format!(" stroke-dasharray=\"{pattern}\""))
                        .unwrap_or_default();
                    let _ = writeln!(
                        svg,
                        "<line x1=\"{x1:.2}\" y1=\"{y1:.2}\" x2=\"{x2:.2}\" y2=\"{y2:.2}\" stroke=\"{}\" stroke-width=\"{width:.2}\" stroke-linecap=\"round\"{dash} />",
                        self.color_to_svg(*color),
                    );
                }
                PdfCommand::Polyline {
                    points,
                    color,
                    width,
                    style,
                } => {
                    let point_list = points
                        .iter()
                        .map(|(x, y)| format!("{x:.2},{y:.2}"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let dash = self
                        .dash_pattern(style)
                        .map(|pattern| format!(" stroke-dasharray=\"{pattern}\""))
                        .unwrap_or_default();
                    let _ = writeln!(
                        svg,
                        "<polyline points=\"{point_list}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{width:.2}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"{dash} />",
                        self.color_to_svg(*color),
                    );
                }
                PdfCommand::Rectangle {
                    x,
                    y,
                    width,
                    height,
                    color,
                    filled,
                } => {
                    let fill = if *filled {
                        self.color_to_svg(*color)
                    } else {
                        "none".to_string()
                    };
                    let stroke = if *filled {
                        "none".to_string()
                    } else {
                        self.color_to_svg(*color)
                    };
                    let _ = writeln!(
                        svg,
                        "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{width:.2}\" height=\"{height:.2}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.00\" />"
                    );
                }
                PdfCommand::Text {
                    text,
                    x,
                    y,
                    size,
                    color,
                } => {
                    let _ = writeln!(
                        svg,
                        "<text x=\"{x:.2}\" y=\"{y:.2}\" fill=\"{}\" font-family=\"Helvetica, Arial, sans-serif\" font-size=\"{size:.2}\" dominant-baseline=\"hanging\">{}</text>",
                        self.color_to_svg(*color),
                        Self::escape_xml(text),
                    );
                }
                PdfCommand::Marker { x, y, size, color } => {
                    let radius = size / 2.0;
                    let _ = writeln!(
                        svg,
                        "<circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"{radius:.2}\" fill=\"{}\" />",
                        self.color_to_svg(*color),
                    );
                }
            }
        }

        svg.push_str("</svg>\n");
        svg
    }

    /// Draw a line from `(x1, y1)` to `(x2, y2)`.
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
        self.commands.borrow_mut().push(PdfCommand::Line {
            x1,
            y1,
            x2,
            y2,
            color,
            width,
            style,
        });
    }

    /// Draw a polyline (connected line segments).
    pub fn draw_polyline(&self, points: &[(f32, f32)], color: Color, width: f32, style: LineStyle) {
        if points.len() < 2 {
            return;
        }

        self.commands.borrow_mut().push(PdfCommand::Polyline {
            points: points.to_vec(),
            color,
            width,
            style,
        });
    }

    /// Draw a filled or stroked rectangle.
    pub fn draw_rectangle(
        &self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
        filled: bool,
    ) {
        self.commands.borrow_mut().push(PdfCommand::Rectangle {
            x,
            y,
            width,
            height,
            color,
            filled,
        });
    }

    /// Draw text at the specified position.
    pub fn draw_text(&self, text: &str, x: f32, y: f32, size: f32, color: Color) {
        self.commands.borrow_mut().push(PdfCommand::Text {
            text: text.to_string(),
            x,
            y,
            size,
            color,
        });
    }

    /// Draw a circle/marker at the specified position.
    pub fn draw_marker(&self, x: f32, y: f32, size: f32, color: Color) {
        self.commands
            .borrow_mut()
            .push(PdfCommand::Marker { x, y, size, color });
    }

    /// Save the PDF to a file.
    pub fn save<P: AsRef<Path>>(self, path: P) -> Result<()> {
        let svg = self.build_svg();
        let pdf_data = crate::export::svg_to_pdf(&svg)?;
        crate::export::write_bytes_atomic(path, &pdf_data)
    }

    /// Get width in pixels (at 96 DPI).
    pub fn width_px(&self) -> f32 {
        self.width_mm * 96.0 / 25.4
    }

    /// Get height in pixels (at 96 DPI).
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
        let mm = renderer.px_to_mm(96.0);
        assert!((mm - 25.4).abs() < 0.01);
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

    #[test]
    fn test_flip_y_uses_top_left_coordinates() {
        let renderer = PdfRenderer::new(160.0, 120.0, "Test").unwrap();
        assert!((renderer.flip_y(0.0) - renderer.height_px()).abs() < 0.01);
    }
}
