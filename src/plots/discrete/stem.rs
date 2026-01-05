//! Stem plot implementations
//!
//! Provides stem plots (lollipop charts) for discrete data visualization.

use crate::render::Color;

/// Configuration for stem plot
#[derive(Debug, Clone)]
pub struct StemConfig {
    /// Stem line color (None for auto)
    pub line_color: Option<Color>,
    /// Stem line width
    pub line_width: f32,
    /// Marker color (None for auto)
    pub marker_color: Option<Color>,
    /// Marker size
    pub marker_size: f32,
    /// Marker style
    pub marker: StemMarker,
    /// Baseline value
    pub baseline: f64,
    /// Bottom marker (at baseline)
    pub bottom_marker: bool,
    /// Orientation
    pub orientation: StemOrientation,
}

/// Marker style for stem heads
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StemMarker {
    Circle,
    Square,
    Diamond,
    Triangle,
    None,
}

/// Orientation for stem plots
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StemOrientation {
    Vertical,
    Horizontal,
}

impl Default for StemConfig {
    fn default() -> Self {
        Self {
            line_color: None,
            line_width: 1.0,
            marker_color: None,
            marker_size: 6.0,
            marker: StemMarker::Circle,
            baseline: 0.0,
            bottom_marker: false,
            orientation: StemOrientation::Vertical,
        }
    }
}

impl StemConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set line color
    pub fn line_color(mut self, color: Color) -> Self {
        self.line_color = Some(color);
        self
    }

    /// Set line width
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width.max(0.1);
        self
    }

    /// Set marker color
    pub fn marker_color(mut self, color: Color) -> Self {
        self.marker_color = Some(color);
        self
    }

    /// Set marker size
    pub fn marker_size(mut self, size: f32) -> Self {
        self.marker_size = size.max(0.0);
        self
    }

    /// Set marker style
    pub fn marker(mut self, marker: StemMarker) -> Self {
        self.marker = marker;
        self
    }

    /// Set baseline
    pub fn baseline(mut self, baseline: f64) -> Self {
        self.baseline = baseline;
        self
    }

    /// Show markers at baseline
    pub fn bottom_marker(mut self, show: bool) -> Self {
        self.bottom_marker = show;
        self
    }

    /// Set horizontal orientation
    pub fn horizontal(mut self) -> Self {
        self.orientation = StemOrientation::Horizontal;
        self
    }
}

/// A single stem element
#[derive(Debug, Clone, Copy)]
pub struct StemElement {
    /// X position of stem
    pub x: f64,
    /// Y position of marker (top of stem for vertical)
    pub y: f64,
    /// Baseline Y position (bottom of stem for vertical)
    pub baseline: f64,
    /// Index in original data
    pub index: usize,
}

impl StemElement {
    /// Get stem line endpoints (for vertical orientation)
    pub fn vertical_line(&self) -> ((f64, f64), (f64, f64)) {
        ((self.x, self.baseline), (self.x, self.y))
    }

    /// Get stem line endpoints (for horizontal orientation)
    pub fn horizontal_line(&self) -> ((f64, f64), (f64, f64)) {
        ((self.baseline, self.x), (self.y, self.x))
    }

    /// Get marker position
    pub fn marker_position(&self, orientation: StemOrientation) -> (f64, f64) {
        match orientation {
            StemOrientation::Vertical => (self.x, self.y),
            StemOrientation::Horizontal => (self.y, self.x),
        }
    }

    /// Get baseline marker position
    pub fn baseline_marker_position(&self, orientation: StemOrientation) -> (f64, f64) {
        match orientation {
            StemOrientation::Vertical => (self.x, self.baseline),
            StemOrientation::Horizontal => (self.baseline, self.x),
        }
    }
}

/// Compute stem elements from data
///
/// # Arguments
/// * `x` - X coordinates (or positions for horizontal)
/// * `y` - Y coordinates (values)
/// * `config` - Stem configuration
///
/// # Returns
/// Vec of StemElement
pub fn compute_stems(x: &[f64], y: &[f64], config: &StemConfig) -> Vec<StemElement> {
    let n = x.len().min(y.len());
    let mut stems = Vec::with_capacity(n);

    for i in 0..n {
        stems.push(StemElement {
            x: x[i],
            y: y[i],
            baseline: config.baseline,
            index: i,
        });
    }

    stems
}

/// Compute data range for stem plot
pub fn stem_range(x: &[f64], y: &[f64], config: &StemConfig) -> ((f64, f64), (f64, f64)) {
    if x.is_empty() || y.is_empty() {
        return ((0.0, 1.0), (0.0, 1.0));
    }

    let x_min = x.iter().copied().fold(f64::INFINITY, f64::min);
    let x_max = x.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    let y_min = y
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min)
        .min(config.baseline);
    let y_max = y
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max)
        .max(config.baseline);

    // Add margin for markers
    let x_margin = (x_max - x_min) * 0.05;

    match config.orientation {
        StemOrientation::Vertical => ((x_min - x_margin, x_max + x_margin), (y_min, y_max)),
        StemOrientation::Horizontal => ((y_min, y_max), (x_min - x_margin, x_max + x_margin)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_stems() {
        let x = vec![0.0, 1.0, 2.0, 3.0];
        let y = vec![2.0, 5.0, 3.0, 4.0];
        let config = StemConfig::default();
        let stems = compute_stems(&x, &y, &config);

        assert_eq!(stems.len(), 4);
        assert!((stems[0].x - 0.0).abs() < 1e-10);
        assert!((stems[0].y - 2.0).abs() < 1e-10);
        assert!((stems[0].baseline - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_stem_element_vertical_line() {
        let stem = StemElement {
            x: 1.0,
            y: 5.0,
            baseline: 0.0,
            index: 0,
        };

        let ((x1, y1), (x2, y2)) = stem.vertical_line();
        assert!((x1 - 1.0).abs() < 1e-10);
        assert!((y1 - 0.0).abs() < 1e-10);
        assert!((x2 - 1.0).abs() < 1e-10);
        assert!((y2 - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_stem_marker_position() {
        let stem = StemElement {
            x: 1.0,
            y: 5.0,
            baseline: 0.0,
            index: 0,
        };

        let (mx, my) = stem.marker_position(StemOrientation::Vertical);
        assert!((mx - 1.0).abs() < 1e-10);
        assert!((my - 5.0).abs() < 1e-10);

        let (mx, my) = stem.marker_position(StemOrientation::Horizontal);
        assert!((mx - 5.0).abs() < 1e-10);
        assert!((my - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_stem_range() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![1.0, 3.0, 2.0];
        let config = StemConfig::default();
        let ((x_min, x_max), (y_min, y_max)) = stem_range(&x, &y, &config);

        assert!(x_min < 0.0); // margin
        assert!(x_max > 2.0); // margin
        assert!((y_min - 0.0).abs() < 1e-10); // baseline
        assert!((y_max - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_stem_with_baseline() {
        let x = vec![0.0, 1.0];
        let y = vec![2.0, 4.0];
        let config = StemConfig::default().baseline(1.0);
        let stems = compute_stems(&x, &y, &config);

        assert!((stems[0].baseline - 1.0).abs() < 1e-10);
    }
}
