//! Tick layout computation for consistent axis rendering
//!
//! Provides a single source of truth for tick positions used by grid lines,
//! tick marks, and tick labels to ensure perfect alignment.
//!
//! Both positions and labels come from the canonical implementations in
//! [`super::ticks`], so a layout computed here matches what any backend draws.

use super::{AxisScale, format_tick_labels_for_scale, generate_ticks_for_scale};

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

/// Place a normalised axis position between two pixel bounds, exactly.
///
/// `at_zero + t * (at_one - at_zero)` drifts below `at_one` at `t == 1.0`:
/// with a plot area from 38.61 to 573.19, `573.19 - 1.0 * (573.19 - 38.61)`
/// evaluates to 38.6099854 in `f32`, which is *outside* the plot area by a
/// quarter of a ULP. Every backend then filters its ticks with
/// `pos >= plot_top`, so an axis whose topmost tick sits exactly on `data_max`
/// silently lost that tick — mark, gridline and label — in whichever backend
/// happened to round the wrong way.
///
/// Pinning the two endpoints makes the mapping exact where it matters and
/// leaves the interior to ordinary interpolation.
fn place_normalized(normalized: f64, at_zero: f32, at_one: f32) -> f32 {
    if normalized <= 0.0 {
        at_zero
    } else if normalized >= 1.0 {
        at_one
    } else {
        at_zero + (normalized as f32) * (at_one - at_zero)
    }
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

        let pixel_positions: Vec<f32> = data_positions
            .iter()
            .map(|&data_pos| {
                if scale_range_is_degenerate(data_min, data_max, scale) {
                    pixel_min
                } else {
                    let normalized = scale.normalized_position(data_pos, data_min, data_max);
                    place_normalized(normalized, pixel_min, pixel_max)
                }
            })
            .collect();

        // Format labels through the canonical, per-axis formatter
        let labels = format_tick_labels_for_scale(&data_positions, scale);

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

        let pixel_positions: Vec<f32> = data_positions
            .iter()
            .map(|&data_pos| {
                if scale_range_is_degenerate(data_min, data_max, scale) {
                    pixel_bottom
                } else {
                    // Invert: higher data values sit at LOWER pixel values, so
                    // normalised 1.0 lands on `pixel_top`.
                    let normalized = scale.normalized_position(data_pos, data_min, data_max);
                    place_normalized(normalized, pixel_bottom, pixel_top)
                }
            })
            .collect();

        // Format labels through the canonical, per-axis formatter
        let labels = format_tick_labels_for_scale(&data_positions, scale);

        Self {
            data_positions,
            pixel_positions,
            labels,
            data_range: (data_min, data_max),
            pixel_range: (pixel_top, pixel_bottom),
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

fn scale_range_is_degenerate(data_min: f64, data_max: f64, scale: &AxisScale) -> bool {
    match scale {
        AxisScale::Log => {
            data_min <= 0.0
                || data_max <= 0.0
                || !data_min.is_finite()
                || !data_max.is_finite()
                || data_min == data_max
        }
        _ => (data_max - data_min).abs() < f64::EPSILON,
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
    fn test_log_tick_layout_uses_log_pixel_positions() {
        let layout = TickLayout::compute(1.0, 1000.0, 0.0, 300.0, &AxisScale::Log, 4);

        assert_eq!(layout.data_positions, vec![1.0, 10.0, 100.0, 1000.0]);
        assert!((layout.pixel_positions[0] - 0.0).abs() < 0.1);
        assert!((layout.pixel_positions[1] - 100.0).abs() < 0.1);
        assert!((layout.pixel_positions[2] - 200.0).abs() < 0.1);
        assert!((layout.pixel_positions[3] - 300.0).abs() < 0.1);
    }

    #[test]
    fn test_log_y_tick_layout_uses_inverted_log_pixel_positions() {
        let layout = TickLayout::compute_y_axis(1.0, 1000.0, 0.0, 300.0, &AxisScale::Log, 4);

        assert_eq!(layout.data_positions, vec![1.0, 10.0, 100.0, 1000.0]);
        assert!((layout.pixel_positions[0] - 300.0).abs() < 0.1);
        assert!((layout.pixel_positions[1] - 200.0).abs() < 0.1);
        assert!((layout.pixel_positions[2] - 100.0).abs() < 0.1);
        assert!((layout.pixel_positions[3] - 0.0).abs() < 0.1);
    }

    #[test]
    fn test_log_tick_layout_spreads_sub_epsilon_ticks() {
        let min = f64::EPSILON / 1024.0;
        let max = f64::EPSILON / 16.0;
        let layout = TickLayout::compute(min, max, 0.0, 300.0, &AxisScale::Log, 8);

        assert!(layout.pixel_positions.len() >= 2);
        assert!(
            layout
                .pixel_positions
                .windows(2)
                .all(|pair| pair[0] < pair[1]),
            "expected distinct log-space positions: {:?}",
            layout.pixel_positions
        );
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
    fn test_layout_labels_come_from_the_canonical_formatter() {
        // `TickLayout` used to carry its own `format_number`, which switched to
        // scientific notation per value. Labels now come from the same
        // formatter the raster backend uses, so PNG and SVG cannot disagree.
        for (min, max, scale) in [
            (0.0, 100.0, AxisScale::Linear),
            (0.0, 1e6, AxisScale::Linear),
            (0.0, 0.001, AxisScale::Linear),
            (1.0, 1000.0, AxisScale::Log),
        ] {
            let layout = TickLayout::compute(min, max, 0.0, 500.0, &scale, 6);
            assert_eq!(
                layout.labels,
                format_tick_labels_for_scale(&layout.data_positions, &scale),
                "layout labels diverged from the canonical formatter for ({min}, {max})"
            );
        }
    }

    #[test]
    fn test_layout_labels_never_mix_notations() {
        for (min, max, scale) in [
            (0.0, 1e6, AxisScale::Linear),
            (0.0, 0.001, AxisScale::Linear),
            (99000.0, 101000.0, AxisScale::Linear),
        ] {
            let layout = TickLayout::compute(min, max, 0.0, 500.0, &scale, 6);
            let scientific = layout
                .labels
                .iter()
                .filter(|label| label.contains('e'))
                .count();
            assert!(
                scientific == 0 || scientific == layout.labels.len(),
                "({min}, {max}) mixed notations: {:?}",
                layout.labels
            );
        }
    }

    #[test]
    fn test_log_layout_labels_use_superscript_decades() {
        let layout = TickLayout::compute(1.0, 1000.0, 0.0, 300.0, &AxisScale::Log, 4);

        // Was "1", "10", "10^2", "10^3" here and "10⁰"… in the raster backend.
        assert_eq!(layout.labels, vec!["10⁰", "10¹", "10²", "10³"]);
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
