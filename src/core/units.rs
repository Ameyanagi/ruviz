//! Unit conversion utilities for DPI-independent rendering
//!
//! This module provides functions to convert between physical units (inches, points)
//! and pixels at a given DPI. This enables DPI-independent plot configuration where
//! changing DPI only affects output resolution, not proportions.
//!
//! # Units
//!
//! - **Points (pt)**: Typographic unit, 1 point = 1/72 inch
//! - **Inches (in)**: Physical measurement unit
//! - **Pixels (px)**: Screen/output unit, depends on DPI
//!
//! # Example
//!
//! ```rust
//! use ruviz::core::units::{pt_to_px, in_to_px, POINTS_PER_INCH};
//!
//! // Convert 10pt font to pixels at 100 DPI
//! let font_px = pt_to_px(10.0, 100.0);  // ≈ 13.89 px
//!
//! // Convert 6.4 inches to pixels at 100 DPI
//! let width_px = in_to_px(6.4, 100.0);  // = 640 px
//! ```

/// Number of points per inch (standard typographic definition)
pub const POINTS_PER_INCH: f32 = 72.0;

/// Reference DPI used for pixel-to-inch conversions in `size_px()` method
pub const REFERENCE_DPI: f32 = 100.0;

/// Convert points to pixels at the given DPI
///
/// Points are a typographic unit where 1 point = 1/72 inch.
/// This is used for font sizes and line widths.
///
/// # Arguments
///
/// * `points` - Size in points
/// * `dpi` - Output resolution in dots per inch
///
/// # Returns
///
/// Size in pixels
///
/// # Example
///
/// ```rust
/// use ruviz::core::units::pt_to_px;
///
/// // 10pt font at 72 DPI = 10 pixels
/// assert_eq!(pt_to_px(10.0, 72.0), 10.0);
///
/// // 10pt font at 144 DPI = 20 pixels
/// assert_eq!(pt_to_px(10.0, 144.0), 20.0);
/// ```
#[inline]
pub fn pt_to_px(points: f32, dpi: f32) -> f32 {
    points * dpi / POINTS_PER_INCH
}

/// Convert inches to pixels at the given DPI
///
/// This is used for figure dimensions and margins.
///
/// # Arguments
///
/// * `inches` - Size in inches
/// * `dpi` - Output resolution in dots per inch
///
/// # Returns
///
/// Size in pixels
///
/// # Example
///
/// ```rust
/// use ruviz::core::units::in_to_px;
///
/// // 6.4 inches at 100 DPI = 640 pixels
/// assert_eq!(in_to_px(6.4, 100.0), 640.0);
///
/// // 6.4 inches at 300 DPI = 1920 pixels
/// assert_eq!(in_to_px(6.4, 300.0), 1920.0);
/// ```
#[inline]
pub fn in_to_px(inches: f32, dpi: f32) -> f32 {
    inches * dpi
}

/// Convert pixels to inches at the given DPI
///
/// This is useful for the `size_px()` convenience method which
/// accepts pixel dimensions and converts them to inches.
///
/// # Arguments
///
/// * `pixels` - Size in pixels
/// * `dpi` - Reference DPI for conversion
///
/// # Returns
///
/// Size in inches
///
/// # Example
///
/// ```rust
/// use ruviz::core::units::px_to_in;
///
/// // 640 pixels at 100 DPI = 6.4 inches
/// assert_eq!(px_to_in(640.0, 100.0), 6.4);
/// ```
#[inline]
pub fn px_to_in(pixels: f32, dpi: f32) -> f32 {
    pixels / dpi
}

/// Convert pixels to points at the given DPI
///
/// This is useful for converting existing pixel-based configurations
/// to the new point-based system.
///
/// # Arguments
///
/// * `pixels` - Size in pixels
/// * `dpi` - Reference DPI for conversion
///
/// # Returns
///
/// Size in points
///
/// # Example
///
/// ```rust
/// use ruviz::core::units::px_to_pt;
///
/// // 10 pixels at 72 DPI = 10 points
/// assert_eq!(px_to_pt(10.0, 72.0), 10.0);
///
/// // 20 pixels at 144 DPI = 10 points
/// assert_eq!(px_to_pt(20.0, 144.0), 10.0);
/// ```
#[inline]
pub fn px_to_pt(pixels: f32, dpi: f32) -> f32 {
    pixels * POINTS_PER_INCH / dpi
}

/// Convert points to inches
///
/// # Arguments
///
/// * `points` - Size in points
///
/// # Returns
///
/// Size in inches
#[inline]
pub fn pt_to_in(points: f32) -> f32 {
    points / POINTS_PER_INCH
}

/// Convert inches to points
///
/// # Arguments
///
/// * `inches` - Size in inches
///
/// # Returns
///
/// Size in points
#[inline]
pub fn in_to_pt(inches: f32) -> f32 {
    inches * POINTS_PER_INCH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pt_to_px() {
        // At 72 DPI, 1 point = 1 pixel
        assert!((pt_to_px(10.0, 72.0) - 10.0).abs() < 0.001);

        // At 144 DPI, 1 point = 2 pixels
        assert!((pt_to_px(10.0, 144.0) - 20.0).abs() < 0.001);

        // At 100 DPI, 10 points = 10 * 100 / 72 ≈ 13.89 pixels
        assert!((pt_to_px(10.0, 100.0) - 13.889).abs() < 0.01);
    }

    #[test]
    fn test_in_to_px() {
        // At 100 DPI, 6.4 inches = 640 pixels
        assert!((in_to_px(6.4, 100.0) - 640.0).abs() < 0.001);

        // At 300 DPI, 6.4 inches = 1920 pixels
        assert!((in_to_px(6.4, 300.0) - 1920.0).abs() < 0.001);
    }

    #[test]
    fn test_px_to_in() {
        // 640 pixels at 100 DPI = 6.4 inches
        assert!((px_to_in(640.0, 100.0) - 6.4).abs() < 0.001);

        // 1920 pixels at 300 DPI = 6.4 inches
        assert!((px_to_in(1920.0, 300.0) - 6.4).abs() < 0.001);
    }

    #[test]
    fn test_px_to_pt() {
        // At 72 DPI, 10 pixels = 10 points
        assert!((px_to_pt(10.0, 72.0) - 10.0).abs() < 0.001);

        // At 144 DPI, 20 pixels = 10 points
        assert!((px_to_pt(20.0, 144.0) - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_pt_in_roundtrip() {
        let original_pt = 36.0;
        let inches = pt_to_in(original_pt);
        let back_to_pt = in_to_pt(inches);
        assert!((original_pt - back_to_pt).abs() < 0.001);
    }

    #[test]
    fn test_dpi_independence() {
        // The ratio of font_px to figure_px should be constant regardless of DPI
        let font_pt = 10.0;
        let figure_in = 6.4;

        // At 100 DPI
        let font_px_100 = pt_to_px(font_pt, 100.0);
        let figure_px_100 = in_to_px(figure_in, 100.0);
        let ratio_100 = font_px_100 / figure_px_100;

        // At 300 DPI
        let font_px_300 = pt_to_px(font_pt, 300.0);
        let figure_px_300 = in_to_px(figure_in, 300.0);
        let ratio_300 = font_px_300 / figure_px_300;

        // Ratios should be equal (DPI-independent)
        assert!((ratio_100 - ratio_300).abs() < 0.0001);
    }
}
