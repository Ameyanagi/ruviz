//! Secondary (twin) axes support
//!
//! Provides dual y-axis (twinx) and dual x-axis (twiny) functionality.

use super::scale::AxisScale;

/// Secondary axis configuration
#[derive(Debug, Clone)]
pub struct SecondaryAxis {
    /// Which axis this secondary axis mirrors
    pub axis: AxisType,
    /// Label for the secondary axis
    pub label: Option<String>,
    /// Scale for the secondary axis
    pub scale: AxisScale,
    /// Data range (min, max)
    pub range: Option<(f64, f64)>,
    /// Whether to show grid lines from this axis
    pub show_grid: bool,
    /// Color for axis line and ticks
    pub color: Option<String>,
}

/// Type of axis being twinned
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisType {
    /// X axis (horizontal)
    X,
    /// Y axis (vertical)
    Y,
}

impl SecondaryAxis {
    /// Create a secondary y-axis (twinx)
    pub fn twinx() -> Self {
        Self {
            axis: AxisType::Y,
            label: None,
            scale: AxisScale::Linear,
            range: None,
            show_grid: false,
            color: None,
        }
    }

    /// Create a secondary x-axis (twiny)
    pub fn twiny() -> Self {
        Self {
            axis: AxisType::X,
            label: None,
            scale: AxisScale::Linear,
            range: None,
            show_grid: false,
            color: None,
        }
    }

    /// Set axis label
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set axis scale (linear, log, etc.)
    pub fn scale(mut self, scale: AxisScale) -> Self {
        self.scale = scale;
        self
    }

    /// Set data range
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.range = Some((min, max));
        self
    }

    /// Enable grid lines from this axis
    pub fn show_grid(mut self, show: bool) -> Self {
        self.show_grid = show;
        self
    }

    /// Set axis color
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Calculate tick positions and labels
    ///
    /// # Arguments
    /// * `range` - The data range to generate ticks for
    /// * `n_ticks` - Approximate number of ticks
    ///
    /// # Returns
    /// Vec of (position, label) pairs
    pub fn generate_ticks(&self, range: (f64, f64), n_ticks: usize) -> Vec<(f64, String)> {
        let (min, max) = range;
        if max - min <= 0.0 || n_ticks == 0 {
            return vec![];
        }

        // Positions and labels both come from `axes::ticks`, the single tick
        // generator and the single tick formatter. This used to carry its own
        // 1/2/5/10 ladder and its own `{:.N}` formatter, so a secondary axis
        // could be ticked and labelled differently from the primary axis it
        // sits beside.
        super::ticks::generate_ticks(min, max, n_ticks)
            .into_iter()
            .map(|tick| (tick, super::ticks::format_tick_label(tick)))
            .collect()
    }

    /// Transform a value from data coordinates to normalized [0, 1] position
    pub fn normalize(&self, value: f64) -> f64 {
        let (min, max) = self.range.unwrap_or((0.0, 1.0));

        match self.scale {
            AxisScale::Linear => (value - min) / (max - min),
            AxisScale::Log => {
                let log_min = min.max(1e-10).log10();
                let log_max = max.log10();
                let log_val = value.max(1e-10).log10();
                (log_val - log_min) / (log_max - log_min)
            }
            _ => (value - min) / (max - min), // Default to linear
        }
    }

    /// Transform a normalized [0, 1] position to data coordinates
    pub fn denormalize(&self, norm: f64) -> f64 {
        let (min, max) = self.range.unwrap_or((0.0, 1.0));

        match self.scale {
            AxisScale::Linear => min + norm * (max - min),
            AxisScale::Log => {
                let log_min = min.max(1e-10).log10();
                let log_max = max.log10();
                10.0_f64.powf(log_min + norm * (log_max - log_min))
            }
            _ => min + norm * (max - min),
        }
    }
}

/// Dual axis configuration for a plot
#[derive(Debug, Clone)]
pub struct DualAxes {
    /// Primary y-axis range
    pub primary_y: (f64, f64),
    /// Secondary y-axis (if any)
    pub secondary_y: Option<SecondaryAxis>,
    /// Primary x-axis range
    pub primary_x: (f64, f64),
    /// Secondary x-axis (if any)
    pub secondary_x: Option<SecondaryAxis>,
}

impl Default for DualAxes {
    fn default() -> Self {
        Self {
            primary_y: (0.0, 1.0),
            secondary_y: None,
            primary_x: (0.0, 1.0),
            secondary_x: None,
        }
    }
}

impl DualAxes {
    /// Create with primary ranges
    pub fn new(x_range: (f64, f64), y_range: (f64, f64)) -> Self {
        Self {
            primary_x: x_range,
            primary_y: y_range,
            ..Default::default()
        }
    }

    /// Add a secondary y-axis
    pub fn twinx(mut self, config: SecondaryAxis) -> Self {
        self.secondary_y = Some(config);
        self
    }

    /// Add a secondary x-axis
    pub fn twiny(mut self, config: SecondaryAxis) -> Self {
        self.secondary_x = Some(config);
        self
    }

    /// Check if plot has secondary y-axis
    pub fn has_secondary_y(&self) -> bool {
        self.secondary_y.is_some()
    }

    /// Check if plot has secondary x-axis
    pub fn has_secondary_x(&self) -> bool {
        self.secondary_x.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secondary_axis_ticks() {
        let axis = SecondaryAxis::twinx().range(0.0, 100.0);
        let ticks = axis.generate_ticks((0.0, 100.0), 5);

        assert!(!ticks.is_empty());
        // First tick should be at or near 0
        assert!(ticks[0].0 >= 0.0);
        // Last tick should be at or near 100
        assert!(ticks.last().unwrap().0 <= 100.0);
    }

    /// A secondary axis sits directly beside a primary one, so it must be
    /// ticked and labelled by the same code. It used to carry its own 1/2/5/10
    /// ladder and its own `{:.N}` formatter and could disagree with the axis
    /// next to it.
    #[test]
    fn test_secondary_axis_uses_the_canonical_generator_and_formatter() {
        let axis = SecondaryAxis::twinx().range(0.0, 100.0);

        for (min, max, count) in [(0.0, 100.0, 5), (0.7, 9.3, 6), (-5.0, 5.0, 8)] {
            let expected: Vec<(f64, String)> = super::super::ticks::generate_ticks(min, max, count)
                .into_iter()
                .map(|tick| (tick, super::super::ticks::format_tick_label(tick)))
                .collect();
            assert_eq!(
                axis.generate_ticks((min, max), count),
                expected,
                "secondary axis diverged from the canonical tick pipeline for ({min}, {max}, {count})"
            );
        }
    }

    #[test]
    fn test_normalize_linear() {
        let axis = SecondaryAxis::twinx().range(0.0, 100.0);

        assert!((axis.normalize(0.0) - 0.0).abs() < 1e-10);
        assert!((axis.normalize(50.0) - 0.5).abs() < 1e-10);
        assert!((axis.normalize(100.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_denormalize() {
        let axis = SecondaryAxis::twinx().range(0.0, 100.0);

        assert!((axis.denormalize(0.0) - 0.0).abs() < 1e-10);
        assert!((axis.denormalize(0.5) - 50.0).abs() < 1e-10);
        assert!((axis.denormalize(1.0) - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_dual_axes() {
        let dual = DualAxes::new((0.0, 10.0), (0.0, 100.0))
            .twinx(SecondaryAxis::twinx().range(0.0, 1.0).label("Secondary"));

        assert!(dual.has_secondary_y());
        assert!(!dual.has_secondary_x());
    }
}
