//! Tick layout computation for consistent axis rendering
//!
//! Provides a single source of truth for tick positions used by grid lines,
//! tick marks, and tick labels to ensure perfect alignment.

use super::{AxisScale, generate_ticks_for_scale};

/// Complete tick layout for an axis
///
/// This struct provides a single source of truth for tick positions,
/// ensuring that grid lines, tick marks, and labels are perfectly aligned.
#[derive(Debug, Clone)]
pub struct TickLayout {
    /// Tick positions in data coordinates
    pub data_positions: Vec<f64>,
    /// Tick positions in pixel coordinates
    pub pixel_positions: Vec<f32>,
    /// Formatted tick labels
    pub labels: Vec<String>,
    /// The data range (min, max)
    pub data_range: (f64, f64),
    /// The pixel range (min, max)
    pub pixel_range: (f32, f32),
}

impl TickLayout {
    /// Compute tick layout for an axis
    ///
    /// # Arguments
    /// * `data_min` - Minimum data value
    /// * `data_max` - Maximum data value
    /// * `pixel_min` - Minimum pixel coordinate
    /// * `pixel_max` - Maximum pixel coordinate
    /// * `scale` - The axis scale type
    /// * `target_ticks` - Target number of ticks (typically 5-7)
    ///
    /// # Returns
    /// A complete tick layout with positions and labels
    pub fn compute(
        data_min: f64,
        data_max: f64,
        pixel_min: f32,
        pixel_max: f32,
        scale: &AxisScale,
        target_ticks: usize,
    ) -> Self {
        // Generate tick positions in data coordinates
        let data_positions = generate_ticks_for_scale(data_min, data_max, target_ticks, scale);

        // Convert to pixel coordinates
        let data_range = data_max - data_min;
        let pixel_range = pixel_max - pixel_min;

        let pixel_positions: Vec<f32> = data_positions
            .iter()
            .map(|&data_pos| {
                if data_range.abs() < f64::EPSILON {
                    pixel_min
                } else {
                    let normalized = (data_pos - data_min) / data_range;
                    pixel_min + (normalized as f32) * pixel_range
                }
            })
            .collect();

        // Format labels with appropriate precision
        let labels = Self::format_labels(&data_positions, scale);

        Self {
            data_positions,
            pixel_positions,
            labels,
            data_range: (data_min, data_max),
            pixel_range: (pixel_min, pixel_max),
        }
    }

    /// Compute tick layout for Y-axis (inverted pixel coordinates)
    ///
    /// Y-axis typically has pixel coordinates inverted (0 at top, max at bottom)
    pub fn compute_y_axis(
        data_min: f64,
        data_max: f64,
        pixel_top: f32,
        pixel_bottom: f32,
        scale: &AxisScale,
        target_ticks: usize,
    ) -> Self {
        // Generate tick positions in data coordinates
        let data_positions = generate_ticks_for_scale(data_min, data_max, target_ticks, scale);

        // Convert to pixel coordinates (inverted for Y-axis)
        let data_range = data_max - data_min;
        let pixel_range = pixel_bottom - pixel_top;

        let pixel_positions: Vec<f32> = data_positions
            .iter()
            .map(|&data_pos| {
                if data_range.abs() < f64::EPSILON {
                    pixel_bottom
                } else {
                    // Invert: higher data values -> lower pixel values
                    let normalized = (data_pos - data_min) / data_range;
                    pixel_bottom - (normalized as f32) * pixel_range
                }
            })
            .collect();

        // Format labels with appropriate precision
        let labels = Self::format_labels(&data_positions, scale);

        Self {
            data_positions,
            pixel_positions,
            labels,
            data_range: (data_min, data_max),
            pixel_range: (pixel_top, pixel_bottom),
        }
    }

    /// Format tick labels with appropriate precision
    fn format_labels(positions: &[f64], scale: &AxisScale) -> Vec<String> {
        positions
            .iter()
            .map(|&pos| Self::format_tick_value(pos, scale))
            .collect()
    }

    /// Format a single tick value
    fn format_tick_value(value: f64, scale: &AxisScale) -> String {
        match scale {
            AxisScale::Log => {
                // For log scale, show as power of 10 if it's a clean power
                let log_val = value.log10();
                if (log_val.round() - log_val).abs() < 1e-10 {
                    let exp = log_val.round() as i32;
                    if exp == 0 {
                        "1".to_string()
                    } else if exp == 1 {
                        "10".to_string()
                    } else {
                        format!("10^{}", exp)
                    }
                } else {
                    Self::format_number(value)
                }
            }
            _ => Self::format_number(value),
        }
    }

    /// Format a number with appropriate precision
    fn format_number(value: f64) -> String {
        let abs_val = value.abs();

        if abs_val == 0.0 {
            return "0".to_string();
        }

        // Use scientific notation for very large or very small numbers
        if abs_val >= 1e5 || (abs_val < 1e-3 && abs_val > 0.0) {
            return format!("{:.1e}", value);
        }

        // Determine appropriate decimal places based on magnitude
        let magnitude = abs_val.log10().floor() as i32;
        let decimals = if magnitude >= 2 {
            0
        } else if magnitude >= 0 {
            1
        } else {
            (-magnitude + 1).min(4) as usize
        };

        let formatted = format!("{:.prec$}", value, prec = decimals);

        // Remove trailing zeros after decimal point
        if formatted.contains('.') {
            let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
            trimmed.to_string()
        } else {
            formatted
        }
    }

    /// Get the number of ticks
    pub fn len(&self) -> usize {
        self.data_positions.len()
    }

    /// Check if the layout is empty
    pub fn is_empty(&self) -> bool {
        self.data_positions.is_empty()
    }

    /// Convert a data value to pixel coordinate
    pub fn data_to_pixel(&self, data_value: f64) -> f32 {
        let (data_min, data_max) = self.data_range;
        let (pixel_min, pixel_max) = self.pixel_range;
        let data_range = data_max - data_min;
        let pixel_range = pixel_max - pixel_min;

        if data_range.abs() < f64::EPSILON {
            pixel_min
        } else {
            let normalized = (data_value - data_min) / data_range;
            pixel_min + (normalized as f32) * pixel_range
        }
    }

    /// Iterate over (pixel_position, label) pairs
    pub fn iter(&self) -> impl Iterator<Item = (f32, &str)> {
        self.pixel_positions
            .iter()
            .zip(self.labels.iter())
            .map(|(&pos, label)| (pos, label.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_layout_basic() {
        let layout = TickLayout::compute(0.0, 100.0, 0.0, 500.0, &AxisScale::Linear, 5);

        assert!(!layout.is_empty());
        assert_eq!(layout.data_positions.len(), layout.pixel_positions.len());
        assert_eq!(layout.data_positions.len(), layout.labels.len());
    }

    #[test]
    fn test_tick_layout_alignment() {
        let layout = TickLayout::compute(0.0, 100.0, 0.0, 500.0, &AxisScale::Linear, 6);

        // Verify pixel positions correspond correctly to data positions
        for (i, &data_pos) in layout.data_positions.iter().enumerate() {
            let expected_pixel = (data_pos / 100.0 * 500.0) as f32;
            let actual_pixel = layout.pixel_positions[i];
            assert!(
                (expected_pixel - actual_pixel).abs() < 0.1,
                "Pixel position mismatch at index {}: expected {}, got {}",
                i,
                expected_pixel,
                actual_pixel
            );
        }
    }

    #[test]
    fn test_tick_layout_y_axis_inverted() {
        let layout = TickLayout::compute_y_axis(0.0, 100.0, 0.0, 500.0, &AxisScale::Linear, 6);

        // Higher data values should have lower pixel values
        if layout.data_positions.len() >= 2 {
            let first_data = layout.data_positions[0];
            let last_data = layout.data_positions[layout.data_positions.len() - 1];
            let first_pixel = layout.pixel_positions[0];
            let last_pixel = layout.pixel_positions[layout.pixel_positions.len() - 1];

            if first_data < last_data {
                assert!(
                    first_pixel > last_pixel,
                    "Y-axis should be inverted: lower data = higher pixel"
                );
            }
        }
    }

    #[test]
    fn test_format_number() {
        assert_eq!(TickLayout::format_number(0.0), "0");
        assert_eq!(TickLayout::format_number(1.0), "1");
        assert_eq!(TickLayout::format_number(10.0), "10");
        assert_eq!(TickLayout::format_number(100.0), "100");
        assert_eq!(TickLayout::format_number(0.5), "0.5");
        assert_eq!(TickLayout::format_number(0.25), "0.25");
    }

    #[test]
    fn test_format_large_numbers() {
        let formatted = TickLayout::format_number(1000000.0);
        assert!(
            formatted.contains('e'),
            "Large numbers should use scientific notation"
        );
    }

    #[test]
    fn test_format_small_numbers() {
        let formatted = TickLayout::format_number(0.0001);
        assert!(
            formatted.contains('e'),
            "Small numbers should use scientific notation"
        );
    }

    #[test]
    fn test_tick_layout_labels_present() {
        let layout = TickLayout::compute(0.0, 100.0, 0.0, 500.0, &AxisScale::Linear, 5);

        for label in &layout.labels {
            assert!(!label.is_empty(), "Labels should not be empty");
        }
    }

    #[test]
    fn test_data_to_pixel() {
        let layout = TickLayout::compute(0.0, 100.0, 0.0, 500.0, &AxisScale::Linear, 5);

        assert!((layout.data_to_pixel(0.0) - 0.0).abs() < 0.1);
        assert!((layout.data_to_pixel(50.0) - 250.0).abs() < 0.1);
        assert!((layout.data_to_pixel(100.0) - 500.0).abs() < 0.1);
    }
}
