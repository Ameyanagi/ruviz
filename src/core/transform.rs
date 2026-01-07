//! Coordinate transformation utilities
//!
//! This module provides unified coordinate transformation between data space
//! and screen (pixel) space. It consolidates the coordinate mapping logic
//! that was previously duplicated in `PlotArea` and `map_data_to_pixels`.
//!
//! # Overview
//!
//! The [`CoordinateTransform`] struct handles the mapping between:
//! - **Data space**: The coordinate system of your data (e.g., x: 0.0..100.0, y: -10.0..50.0)
//! - **Screen space**: Pixel coordinates on the canvas (e.g., x: 50..750, y: 50..550)
//!
//! # Example
//!
//! ```rust,ignore
//! use ruviz::core::CoordinateTransform;
//!
//! let transform = CoordinateTransform::new(
//!     0.0..100.0,   // data x range
//!     0.0..50.0,    // data y range
//!     50.0..750.0,  // screen x range (pixels)
//!     50.0..550.0,  // screen y range (pixels)
//! );
//!
//! let (screen_x, screen_y) = transform.data_to_screen(50.0, 25.0);
//! let (data_x, data_y) = transform.screen_to_data(screen_x, screen_y);
//! ```

use std::ops::Range;

/// Unified coordinate transformation between data space and screen space.
///
/// This struct provides methods to convert coordinates between the data domain
/// (typically f64 values representing your plot data) and screen coordinates
/// (f32 pixel positions on the canvas).
///
/// # Y-axis Inversion
///
/// Screen coordinates typically have Y=0 at the top, while data coordinates
/// usually have Y increasing upward. This struct handles the inversion
/// automatically based on the `y_inverted` flag (true by default for standard plots).
#[derive(Debug, Clone)]
pub struct CoordinateTransform {
    /// Data bounds for x-axis (min..max)
    pub data_x: Range<f64>,
    /// Data bounds for y-axis (min..max)
    pub data_y: Range<f64>,
    /// Screen bounds for x-axis in pixels (left..right)
    pub screen_x: Range<f32>,
    /// Screen bounds for y-axis in pixels (top..bottom)
    pub screen_y: Range<f32>,
    /// Whether Y-axis should be inverted (true for standard screen coordinates)
    pub y_inverted: bool,
}

impl CoordinateTransform {
    /// Create a new coordinate transform with the given bounds.
    ///
    /// By default, Y-axis is inverted to match standard screen coordinates
    /// where Y=0 is at the top.
    ///
    /// # Arguments
    ///
    /// * `data_x` - Data x-axis range (min..max)
    /// * `data_y` - Data y-axis range (min..max)
    /// * `screen_x` - Screen x-axis range in pixels (left..right)
    /// * `screen_y` - Screen y-axis range in pixels (top..bottom)
    pub fn new(
        data_x: Range<f64>,
        data_y: Range<f64>,
        screen_x: Range<f32>,
        screen_y: Range<f32>,
    ) -> Self {
        Self {
            data_x,
            data_y,
            screen_x,
            screen_y,
            y_inverted: true,
        }
    }

    /// Create a coordinate transform without Y-axis inversion.
    ///
    /// Useful for coordinate systems where Y increases downward.
    pub fn new_non_inverted(
        data_x: Range<f64>,
        data_y: Range<f64>,
        screen_x: Range<f32>,
        screen_y: Range<f32>,
    ) -> Self {
        Self {
            data_x,
            data_y,
            screen_x,
            screen_y,
            y_inverted: false,
        }
    }

    /// Create a coordinate transform from plot area parameters.
    ///
    /// This is a convenience constructor for creating a transform from
    /// the typical plot area representation used in the crate.
    ///
    /// # Arguments
    ///
    /// * `area_x` - Left edge of plot area in pixels
    /// * `area_y` - Top edge of plot area in pixels
    /// * `area_width` - Width of plot area in pixels
    /// * `area_height` - Height of plot area in pixels
    /// * `x_min` - Minimum x value in data space
    /// * `x_max` - Maximum x value in data space
    /// * `y_min` - Minimum y value in data space
    /// * `y_max` - Maximum y value in data space
    pub fn from_plot_area(
        area_x: f32,
        area_y: f32,
        area_width: f32,
        area_height: f32,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> Self {
        Self::new(
            x_min..x_max,
            y_min..y_max,
            area_x..(area_x + area_width),
            area_y..(area_y + area_height),
        )
    }

    /// Transform data coordinates to screen coordinates.
    ///
    /// # Arguments
    ///
    /// * `data_x` - X coordinate in data space
    /// * `data_y` - Y coordinate in data space
    ///
    /// # Returns
    ///
    /// A tuple of (screen_x, screen_y) in pixel coordinates
    #[inline]
    pub fn data_to_screen(&self, data_x: f64, data_y: f64) -> (f32, f32) {
        let x_range = self.data_x.end - self.data_x.start;
        let y_range = self.data_y.end - self.data_y.start;

        // Normalize to [0, 1], handling division by zero
        let norm_x = if x_range.abs() > f64::EPSILON {
            (data_x - self.data_x.start) / x_range
        } else {
            0.5
        };

        let norm_y = if y_range.abs() > f64::EPSILON {
            (data_y - self.data_y.start) / y_range
        } else {
            0.5
        };

        let screen_width = self.screen_x.end - self.screen_x.start;
        let screen_height = self.screen_y.end - self.screen_y.start;

        let screen_x = self.screen_x.start + (norm_x as f32) * screen_width;
        let screen_y = if self.y_inverted {
            // Y is inverted in screen coordinates (0 at top)
            self.screen_y.start + (1.0 - norm_y as f32) * screen_height
        } else {
            self.screen_y.start + (norm_y as f32) * screen_height
        };

        (screen_x, screen_y)
    }

    /// Transform screen coordinates to data coordinates.
    ///
    /// # Arguments
    ///
    /// * `screen_x` - X coordinate in pixels
    /// * `screen_y` - Y coordinate in pixels
    ///
    /// # Returns
    ///
    /// A tuple of (data_x, data_y) in data space
    #[inline]
    pub fn screen_to_data(&self, screen_x: f32, screen_y: f32) -> (f64, f64) {
        let screen_width = self.screen_x.end - self.screen_x.start;
        let screen_height = self.screen_y.end - self.screen_y.start;

        let norm_x = (screen_x - self.screen_x.start) / screen_width;
        let norm_y = if self.y_inverted {
            1.0 - (screen_y - self.screen_y.start) / screen_height
        } else {
            (screen_y - self.screen_y.start) / screen_height
        };

        let data_x = self.data_x.start + (norm_x as f64) * (self.data_x.end - self.data_x.start);
        let data_y = self.data_y.start + (norm_y as f64) * (self.data_y.end - self.data_y.start);

        (data_x, data_y)
    }

    /// Check if a data point is within the data bounds.
    #[inline]
    pub fn contains_data(&self, data_x: f64, data_y: f64) -> bool {
        data_x >= self.data_x.start
            && data_x <= self.data_x.end
            && data_y >= self.data_y.start
            && data_y <= self.data_y.end
    }

    /// Check if a screen point is within the screen bounds.
    #[inline]
    pub fn contains_screen(&self, screen_x: f32, screen_y: f32) -> bool {
        screen_x >= self.screen_x.start
            && screen_x <= self.screen_x.end
            && screen_y >= self.screen_y.start
            && screen_y <= self.screen_y.end
    }

    /// Get the center point in data coordinates.
    pub fn data_center(&self) -> (f64, f64) {
        (
            (self.data_x.start + self.data_x.end) / 2.0,
            (self.data_y.start + self.data_y.end) / 2.0,
        )
    }

    /// Get the center point in screen coordinates.
    pub fn screen_center(&self) -> (f32, f32) {
        (
            (self.screen_x.start + self.screen_x.end) / 2.0,
            (self.screen_y.start + self.screen_y.end) / 2.0,
        )
    }

    /// Get the width of the screen area in pixels.
    pub fn screen_width(&self) -> f32 {
        self.screen_x.end - self.screen_x.start
    }

    /// Get the height of the screen area in pixels.
    pub fn screen_height(&self) -> f32 {
        self.screen_y.end - self.screen_y.start
    }

    /// Get the width of the data range.
    pub fn data_width(&self) -> f64 {
        self.data_x.end - self.data_x.start
    }

    /// Get the height of the data range.
    pub fn data_height(&self) -> f64 {
        self.data_y.end - self.data_y.start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_to_screen_basic() {
        let transform = CoordinateTransform::new(0.0..100.0, 0.0..100.0, 0.0..1000.0, 0.0..500.0);

        // Origin in data space
        let (x, y) = transform.data_to_screen(0.0, 0.0);
        assert!((x - 0.0).abs() < f32::EPSILON);
        assert!((y - 500.0).abs() < f32::EPSILON); // Y inverted: 0 in data -> bottom in screen

        // Max corner
        let (x, y) = transform.data_to_screen(100.0, 100.0);
        assert!((x - 1000.0).abs() < f32::EPSILON);
        assert!((y - 0.0).abs() < f32::EPSILON); // Y inverted: 100 in data -> top in screen

        // Center
        let (x, y) = transform.data_to_screen(50.0, 50.0);
        assert!((x - 500.0).abs() < f32::EPSILON);
        assert!((y - 250.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_screen_to_data_basic() {
        let transform = CoordinateTransform::new(0.0..100.0, 0.0..100.0, 0.0..1000.0, 0.0..500.0);

        // Top-left of screen
        let (x, y) = transform.screen_to_data(0.0, 0.0);
        assert!((x - 0.0).abs() < f64::EPSILON);
        assert!((y - 100.0).abs() < f64::EPSILON); // Y inverted

        // Bottom-right of screen
        let (x, y) = transform.screen_to_data(1000.0, 500.0);
        assert!((x - 100.0).abs() < f64::EPSILON);
        assert!((y - 0.0).abs() < f64::EPSILON); // Y inverted
    }

    #[test]
    fn test_roundtrip() {
        let transform =
            CoordinateTransform::new(-50.0..150.0, -10.0..90.0, 100.0..900.0, 50.0..550.0);

        let test_points = [(0.0, 0.0), (100.0, 50.0), (-25.0, 45.0), (75.0, -5.0)];

        // Note: tolerance is higher due to f64 -> f32 -> f64 conversion
        // f32 has ~7 significant digits, so we use 1e-4 relative tolerance
        let tolerance = 1e-4;

        for (data_x, data_y) in test_points {
            let (screen_x, screen_y) = transform.data_to_screen(data_x, data_y);
            let (recovered_x, recovered_y) = transform.screen_to_data(screen_x, screen_y);

            // Use relative tolerance for non-zero values, absolute for near-zero
            let x_tol = if data_x.abs() > 1.0 {
                data_x.abs() * tolerance
            } else {
                tolerance
            };
            let y_tol = if data_y.abs() > 1.0 {
                data_y.abs() * tolerance
            } else {
                tolerance
            };

            assert!(
                (data_x - recovered_x).abs() < x_tol,
                "X roundtrip failed: {} -> {} -> {} (tolerance: {})",
                data_x,
                screen_x,
                recovered_x,
                x_tol
            );
            assert!(
                (data_y - recovered_y).abs() < y_tol,
                "Y roundtrip failed: {} -> {} -> {} (tolerance: {})",
                data_y,
                screen_y,
                recovered_y,
                y_tol
            );
        }
    }

    #[test]
    fn test_from_plot_area() {
        let transform = CoordinateTransform::from_plot_area(
            50.0,  // area_x
            50.0,  // area_y
            700.0, // area_width
            500.0, // area_height
            0.0,   // x_min
            100.0, // x_max
            0.0,   // y_min
            100.0, // y_max
        );

        assert!((transform.screen_x.start - 50.0).abs() < f32::EPSILON);
        assert!((transform.screen_x.end - 750.0).abs() < f32::EPSILON);
        assert!((transform.screen_y.start - 50.0).abs() < f32::EPSILON);
        assert!((transform.screen_y.end - 550.0).abs() < f32::EPSILON);

        // Test a point
        let (x, y) = transform.data_to_screen(50.0, 50.0);
        assert!((x - 400.0).abs() < f32::EPSILON); // 50 + 700/2
        assert!((y - 300.0).abs() < f32::EPSILON); // 50 + 500/2
    }

    #[test]
    fn test_non_inverted() {
        let transform =
            CoordinateTransform::new_non_inverted(0.0..100.0, 0.0..100.0, 0.0..100.0, 0.0..100.0);

        // Without inversion, data Y=0 should map to screen Y=0
        let (_, y) = transform.data_to_screen(0.0, 0.0);
        assert!((y - 0.0).abs() < f32::EPSILON);

        let (_, y) = transform.data_to_screen(0.0, 100.0);
        assert!((y - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_contains_data() {
        let transform = CoordinateTransform::new(0.0..100.0, 0.0..100.0, 0.0..100.0, 0.0..100.0);

        assert!(transform.contains_data(50.0, 50.0));
        assert!(transform.contains_data(0.0, 0.0));
        assert!(transform.contains_data(100.0, 100.0));
        assert!(!transform.contains_data(-1.0, 50.0));
        assert!(!transform.contains_data(50.0, 101.0));
    }

    #[test]
    fn test_zero_range() {
        // Edge case: zero range should return center
        let transform = CoordinateTransform::new(
            50.0..50.0, // zero range
            50.0..50.0, // zero range
            0.0..100.0,
            0.0..100.0,
        );

        let (x, y) = transform.data_to_screen(50.0, 50.0);
        assert!((x - 50.0).abs() < f32::EPSILON); // Center of screen range
        assert!((y - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_helper_methods() {
        let transform = CoordinateTransform::new(0.0..200.0, 0.0..100.0, 50.0..850.0, 100.0..600.0);

        assert!((transform.screen_width() - 800.0).abs() < f32::EPSILON);
        assert!((transform.screen_height() - 500.0).abs() < f32::EPSILON);
        assert!((transform.data_width() - 200.0).abs() < f64::EPSILON);
        assert!((transform.data_height() - 100.0).abs() < f64::EPSILON);

        let (cx, cy) = transform.data_center();
        assert!((cx - 100.0).abs() < f64::EPSILON);
        assert!((cy - 50.0).abs() < f64::EPSILON);

        let (sx, sy) = transform.screen_center();
        assert!((sx - 450.0).abs() < f32::EPSILON);
        assert!((sy - 350.0).abs() < f32::EPSILON);
    }
}
