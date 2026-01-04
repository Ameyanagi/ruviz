//! Export functionality
//!
//! Provides export capabilities for various formats:
//! - PNG: Raster export via `Plot::save()`
//! - SVG: Vector export via `Plot::to_svg()` or `Plot::render_to_svg()`
//! - PDF: Vector export via `Plot::save_pdf()` (requires `pdf` feature)
//!
//! The PDF export uses an SVG → PDF pipeline for high-quality vector output.

pub mod svg;

#[cfg(feature = "pdf")]
pub mod pdf;

#[cfg(feature = "pdf")]
pub mod svg_to_pdf;

pub use svg::SvgRenderer;

#[cfg(feature = "pdf")]
pub use pdf::PdfRenderer;

#[cfg(feature = "pdf")]
pub use svg_to_pdf::{page_sizes, svg_to_pdf, svg_to_pdf_file};
