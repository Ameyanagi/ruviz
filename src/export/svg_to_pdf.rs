//! SVG to PDF conversion
//!
//! Uses svg2pdf and usvg to convert SVG to PDF format.
//! This provides publication-quality vector PDF output.

use crate::core::{PlottingError, Result};
use std::{borrow::Cow, path::Path};

/// The PDF converter's usvg treats start/end anchors as physical left/right, ignoring SVG's
/// direction-dependent anchor semantics. Preserve the physical alignment of
/// the explicit RTL text elements emitted by SvgRenderer during PDF conversion.
fn prepare_rtl_anchors(svg: &str) -> Result<Cow<'_, str>> {
    if !svg.contains("rtl") {
        return Ok(Cow::Borrowed(svg));
    }
    let document = svg2pdf::usvg::roxmltree::Document::parse_with_options(
        svg,
        svg2pdf::usvg::roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .map_err(|error| PlottingError::RenderError(format!("Failed to parse SVG: {error}")))?;
    let replacements: Vec<_> = document
        .descendants()
        .filter(|node| node.has_tag_name("text") && node.attribute("direction") == Some("rtl"))
        .filter_map(|node| {
            let anchor = node.attribute_node("text-anchor")?;
            let replacement = match anchor.value() {
                "start" => "end",
                "end" => "start",
                _ => return None,
            };
            Some((anchor.range_value(), replacement))
        })
        .collect();
    if replacements.is_empty() {
        return Ok(Cow::Borrowed(svg));
    }
    let mut prepared = svg.to_string();
    for (range, replacement) in replacements.into_iter().rev() {
        prepared.replace_range(range, replacement);
    }
    Ok(Cow::Owned(prepared))
}

#[cfg(feature = "pdf")]
fn pdf_font_database() -> Result<svg2pdf::usvg::fontdb::Database> {
    let system = crate::render::get_font_system().lock().map_err(|_| {
        PlottingError::RenderError(
            "Text rendering aborted because FontSystem lock is poisoned".into(),
        )
    })?;
    Ok(system.db().clone())
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

    let prepared = prepare_rtl_anchors(svg_data)?;
    let tree = usvg::Tree::from_str(&prepared, &options)
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
    fn rtl_pdf_text_keeps_left_edges_and_stays_inside_annotation_boxes() {
        use crate::{
            core::{TextAlign, TextStyle},
            export::SvgRenderer,
            render::{Color, FontFamily, TextOptions},
        };
        use svg2pdf::usvg;

        fn collect_bounds(
            group: &usvg::Group,
            text: &mut Vec<usvg::Rect>,
            boxes: &mut Vec<usvg::Rect>,
        ) {
            for node in group.children() {
                match node {
                    usvg::Node::Group(group) => collect_bounds(group, text, boxes),
                    usvg::Node::Text(node) => text.push(node.abs_bounding_box()),
                    usvg::Node::Path(node) => boxes.push(node.abs_bounding_box()),
                    _ => {}
                }
            }
        }

        crate::render::register_font_bytes(include_bytes!("../dejavu-sans.ttf").to_vec()).unwrap();
        let options = usvg::Options {
            fontdb: std::sync::Arc::new(pdf_font_database().unwrap()),
            ..Default::default()
        };
        let family = FontFamily::from("DejaVu Sans");
        for alignment in [TextAlign::Left, TextAlign::Center, TextAlign::Right] {
            for label in ["العربية", "العربية\nاختبار"] {
                let mut renderer = SvgRenderer::new(500.0, 200.0);
                renderer.set_font_family(family.clone());
                renderer.set_text_options(TextOptions::new().language("ar"));
                renderer
                    .draw_text("العربية", 100.0, 10.0, 20.0, Color::BLACK)
                    .unwrap();
                renderer
                    .draw_styled_text(
                        label,
                        250.0,
                        100.0,
                        &family,
                        &TextStyle::new()
                            .font_size(20.0)
                            .align(alignment)
                            .background(Color::WHITE)
                            .padding(6.0),
                    )
                    .unwrap();
                let svg = renderer.to_svg_string();
                let prepared = prepare_rtl_anchors(&svg).unwrap();
                let tree = usvg::Tree::from_str(&prepared, &options).unwrap();
                let mut text = Vec::new();
                let mut boxes = Vec::new();
                collect_bounds(tree.root(), &mut text, &mut boxes);
                assert_eq!(text.len(), 2);
                assert_eq!(boxes.len(), 1);
                assert!((text[0].left() - 100.0).abs() < 3.0, "{:?}", text[0]);
                let (ink, background) = (text[1], boxes[0]);
                assert!(
                    ink.left() >= background.left() - 1.0,
                    "{alignment:?}: {ink:?} {background:?}"
                );
                assert!(
                    ink.right() <= background.right() + 1.0,
                    "{alignment:?}: {ink:?} {background:?}"
                );
                assert!(svg_to_pdf(&svg).unwrap().starts_with(b"%PDF-"));
            }
        }
    }

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
