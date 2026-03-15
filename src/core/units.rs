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

/// Shared render-scale context for converting logical units to output pixels.
///
/// `dpi` controls output pixel density. `device_scale` is reserved for host or
/// framebuffer scaling in interactive environments and is intentionally kept
/// separate from style semantics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderScale {
    figure_width_in: f32,
    figure_height_in: f32,
    dpi: f32,
    device_scale: f32,
}

impl RenderScale {
    fn sanitize_positive(value: f32, fallback: f32) -> f32 {
        if value.is_finite() && value > 0.0 {
            value
        } else {
            fallback
        }
    }

    /// Create a scale context with an explicit output DPI.
    pub fn new(dpi: f32) -> Self {
        Self {
            figure_width_in: 6.4,
            figure_height_in: 4.8,
            dpi: Self::sanitize_positive(dpi, REFERENCE_DPI),
            device_scale: 1.0,
        }
    }

    /// Return a copy with explicit figure dimensions in inches.
    pub fn with_figure(mut self, figure_width_in: f32, figure_height_in: f32) -> Self {
        self.figure_width_in = Self::sanitize_positive(figure_width_in, 6.4);
        self.figure_height_in = Self::sanitize_positive(figure_height_in, 4.8);
        self
    }

    /// Create a scale context with explicit output DPI and device scale.
    pub fn with_device_scale(dpi: f32, device_scale: f32) -> Self {
        Self::new(dpi).with_host_scale(device_scale)
    }

    /// Create a scale context from an existing pixel canvas and output DPI.
    pub fn from_canvas_size(width_px: u32, height_px: u32, dpi: f32) -> Self {
        let dpi = Self::sanitize_positive(dpi, REFERENCE_DPI);
        Self::new(dpi).with_figure(
            px_to_in(width_px as f32, dpi),
            px_to_in(height_px as f32, dpi),
        )
    }

    /// Return a copy with a separate host/device scale applied.
    pub fn with_host_scale(mut self, device_scale: f32) -> Self {
        self.device_scale = Self::sanitize_positive(device_scale, 1.0);
        self
    }

    /// Create a scale context from a legacy `dpi / REFERENCE_DPI` ratio.
    pub fn from_reference_scale(scale: f32) -> Self {
        Self::new(Self::sanitize_positive(scale, 1.0) * REFERENCE_DPI)
    }

    /// Figure width in inches.
    pub fn figure_width_in(self) -> f32 {
        self.figure_width_in
    }

    /// Figure height in inches.
    pub fn figure_height_in(self) -> f32 {
        self.figure_height_in
    }

    /// Output DPI used for physical-unit conversion.
    pub fn dpi(self) -> f32 {
        self.dpi
    }

    /// Host or framebuffer scale for interactive rendering.
    pub fn device_scale(self) -> f32 {
        self.device_scale
    }

    /// Effective device DPI after host/device scaling is applied.
    pub fn device_dpi(self) -> f32 {
        self.dpi * self.device_scale
    }

    /// Legacy `dpi / REFERENCE_DPI` ratio, retained only for compatibility.
    pub fn reference_scale(self) -> f32 {
        self.dpi / REFERENCE_DPI
    }

    /// Convert typographic points to output pixels.
    pub fn points_to_pixels(self, points: f32) -> f32 {
        pt_to_px(points, self.dpi)
    }

    /// Convert inches to output pixels.
    pub fn inches_to_pixels(self, inches: f32) -> f32 {
        in_to_px(inches, self.dpi)
    }

    /// Convert output pixels to inches.
    pub fn pixels_to_inches(self, pixels: f32) -> f32 {
        px_to_in(pixels, self.dpi)
    }

    /// Convert output pixels to points.
    pub fn pixels_to_points(self, pixels: f32) -> f32 {
        px_to_pt(pixels, self.dpi)
    }

    /// Convert logical pixels authored at `REFERENCE_DPI` to output pixels.
    pub fn logical_pixels_to_pixels(self, logical_pixels: f32) -> f32 {
        logical_pixels * self.reference_scale()
    }

    /// Convert output pixels back to logical pixels at `REFERENCE_DPI`.
    pub fn pixels_to_logical_pixels(self, pixels: f32) -> f32 {
        pixels / self.reference_scale()
    }

    /// Convert output pixels to device pixels using only the host/device scale.
    pub fn pixels_to_device_pixels(self, pixels: f32) -> f32 {
        pixels * self.device_scale
    }

    /// Convert logical/output pixels to device pixels using the host/device scale.
    pub fn logical_pixels_to_device_pixels(self, logical_pixels: f32) -> f32 {
        logical_pixels * self.device_scale
    }

    /// Convert device pixels back to logical/output pixels.
    pub fn device_pixels_to_logical_pixels(self, pixels: f32) -> f32 {
        pixels / self.device_scale
    }

    /// Convert reference pixels (100-DPI baseline) to output pixels.
    pub fn reference_pixels_to_pixels(self, pixels: f32) -> f32 {
        pixels * self.reference_scale()
    }

    /// Convert reference pixels (100-DPI baseline) directly to device pixels.
    pub fn reference_pixels_to_device_pixels(self, pixels: f32) -> f32 {
        self.pixels_to_device_pixels(self.reference_pixels_to_pixels(pixels))
    }

    /// Convert the configured figure size to output canvas pixels.
    pub fn canvas_size(self) -> (u32, u32) {
        (
            self.inches_to_pixels(self.figure_width_in) as u32,
            self.inches_to_pixels(self.figure_height_in) as u32,
        )
    }

    /// Convert the configured figure size to device canvas pixels.
    pub fn device_canvas_size(self) -> (u32, u32) {
        (
            self.pixels_to_device_pixels(self.inches_to_pixels(self.figure_width_in))
                .round() as u32,
            self.pixels_to_device_pixels(self.inches_to_pixels(self.figure_height_in))
                .round() as u32,
        )
    }

    /// Convert arbitrary figure dimensions in inches to output canvas pixels.
    pub fn canvas_size_pixels(self, width_in: f32, height_in: f32) -> (u32, u32) {
        (
            self.inches_to_pixels(width_in) as u32,
            self.inches_to_pixels(height_in) as u32,
        )
    }

    /// Convert arbitrary figure dimensions in inches to device canvas pixels.
    pub fn canvas_size_device_pixels(self, width_in: f32, height_in: f32) -> (u32, u32) {
        (
            self.pixels_to_device_pixels(self.inches_to_pixels(width_in))
                .round() as u32,
            self.pixels_to_device_pixels(self.inches_to_pixels(height_in))
                .round() as u32,
        )
    }
}

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
    fn test_render_scale_logical_and_device_conversion() {
        let scale = RenderScale::with_device_scale(150.0, 2.0);

        assert!((scale.points_to_pixels(12.0) - 25.0).abs() < 0.001);
        assert!((scale.pixels_to_device_pixels(25.0) - 50.0).abs() < 0.001);
        assert_eq!(scale.canvas_size_pixels(6.4, 4.8), (960, 720));
        assert_eq!(scale.canvas_size_device_pixels(6.4, 4.8), (1920, 1440));
    }

    #[test]
    fn test_render_scale_reference_baseline_conversion() {
        let scale = RenderScale::new(300.0);

        assert!((scale.logical_pixels_to_pixels(5.0) - 15.0).abs() < 0.001);
        assert!((scale.reference_scale() - 3.0).abs() < 0.001);
    }

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

    #[test]
    fn test_render_scale_points_and_inches() {
        let scale = RenderScale::new(144.0);

        assert!((scale.points_to_pixels(10.0) - 20.0).abs() < 0.001);
        assert!((scale.inches_to_pixels(2.0) - 288.0).abs() < 0.001);
        assert!((scale.pixels_to_points(20.0) - 10.0).abs() < 0.001);
        assert!((scale.pixels_to_inches(288.0) - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_render_scale_logical_pixels() {
        let scale = RenderScale::new(200.0);

        assert!((scale.logical_pixels_to_pixels(10.0) - 20.0).abs() < 0.001);
        assert!((scale.pixels_to_logical_pixels(20.0) - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_render_scale_device_scale_is_separate() {
        let scale = RenderScale::with_device_scale(150.0, 2.0);

        assert!((scale.points_to_pixels(12.0) - 25.0).abs() < 0.001);
        assert!((scale.logical_pixels_to_device_pixels(10.0) - 20.0).abs() < 0.001);
        assert!((scale.pixels_to_device_pixels(15.0) - 30.0).abs() < 0.001);
    }

    #[test]
    fn test_render_scale_sanitizes_invalid_inputs() {
        let scale = RenderScale::with_device_scale(f32::NAN, 0.0);

        assert!((scale.dpi() - REFERENCE_DPI).abs() < 0.001);
        assert!((scale.device_scale() - 1.0).abs() < 0.001);
    }
}
