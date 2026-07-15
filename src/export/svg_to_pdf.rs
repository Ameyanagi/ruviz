//! SVG to PDF conversion
//!
//! Uses svg2pdf and usvg to convert SVG to PDF format.
//! This provides publication-quality vector PDF output.

use crate::core::{PlottingError, Result};
#[cfg(feature = "pdf")]
use crate::render::font_registry;
use std::path::Path;

#[cfg(feature = "pdf")]
fn pdf_font_database() -> Result<svg2pdf::usvg::fontdb::Database> {
    let mut database = svg2pdf::usvg::fontdb::Database::new();
    #[cfg(not(target_arch = "wasm32"))]
    database.load_system_fonts();
    let registered_fonts = font_registry::snapshot()?;
    font_registry::load_with_registered_precedence(&mut database, &registered_fonts);
    Ok(database)
}

/// Convert SVG string to PDF bytes
#[cfg(feature = "pdf")]
pub fn svg_to_pdf(svg_data: &str) -> Result<Vec<u8>> {
    // Use svg2pdf's re-exported usvg to ensure version compatibility
    use svg2pdf::usvg;

    // Plain SVG stays portable-by-reference; registered bytes are supplied only
    // to the PDF conversion database where glyph outlines can be embedded.
    let fontdb = pdf_font_database()?;

    // Parse SVG with usvg
    let options = usvg::Options {
        fontdb: std::sync::Arc::new(fontdb),
        ..Default::default()
    };

    let tree = usvg::Tree::from_str(svg_data, &options)
        .map_err(|e| PlottingError::RenderError(format!("Failed to parse SVG: {}", e)))?;

    // Convert to PDF
    let pdf_data = svg2pdf::to_pdf(
        &tree,
        svg2pdf::ConversionOptions::default(),
        svg2pdf::PageOptions::default(),
    )
    .map_err(|e| PlottingError::RenderError(format!("Failed to convert SVG to PDF: {:?}", e)))?;

    Ok(pdf_data)
}

/// Convert SVG string to PDF and save to file
#[cfg(feature = "pdf")]
pub fn svg_to_pdf_file<P: AsRef<Path>>(svg_data: &str, path: P) -> Result<()> {
    let pdf_data = svg_to_pdf(svg_data)?;
    crate::export::write_bytes_atomic(path, &pdf_data)
}

/// Page sizes in millimeters
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

    /// Convert millimeters to pixels at 96 DPI
    pub fn mm_to_px(mm: f64) -> f32 {
        (mm * 96.0 / 25.4) as f32
    }

    /// Convert pixels to millimeters at 96 DPI
    pub fn px_to_mm(px: f32) -> f64 {
        px as f64 * 25.4 / 96.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "pdf")]
    fn test_svg_to_pdf_basic() {
        let svg = r#"<?xml version="1.0" encoding="UTF-8"?>
<svg width="200" height="150" xmlns="http://www.w3.org/2000/svg">
  <rect width="100%" height="100%" fill="white"/>
  <line x1="10" y1="10" x2="190" y2="140" stroke="black" stroke-width="2"/>
  <text x="100" y="75" text-anchor="middle" font-family="sans-serif" font-size="14">Test</text>
</svg>"#;

        let result = svg_to_pdf(svg);
        assert!(result.is_ok());

        let pdf_data = result.unwrap();
        // PDF files start with %PDF-
        assert!(pdf_data.starts_with(b"%PDF-"));
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn registered_font_is_loaded_into_pdf_font_database() {
        let Some(bytes) = crate::render::font_registry::renamed_test_font(b"PPdf") else {
            return;
        };
        crate::render::register_font_bytes(bytes).unwrap();
        let database = pdf_font_database().unwrap();
        assert!(database.faces().any(|face| {
            face.families
                .iter()
                .any(|(family, _)| family == "PPdf Sans")
        }));

        let svg = r#"<svg width="120" height="40" xmlns="http://www.w3.org/2000/svg">
  <text x="2" y="28" font-family="PPdf Sans" font-size="20">P12 PDF</text>
</svg>"#;
        assert!(svg_to_pdf(svg).unwrap().starts_with(b"%PDF-"));
    }

    #[test]
    fn test_page_size_conversion() {
        // 25.4mm = 96 pixels (at 96 DPI)
        let px = page_sizes::mm_to_px(25.4);
        assert!((px - 96.0).abs() < 0.1);

        let mm = page_sizes::px_to_mm(96.0);
        assert!((mm - 25.4).abs() < 0.1);
    }
}
