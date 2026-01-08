//! Boxen (Letter-Value) plot implementations
//!
//! Provides enhanced box plots that show more quantile information.
//!
//! # Trait-Based API
//!
//! Boxen plots implement the core plot traits:
//! - [`PlotConfig`] for `BoxenConfig`
//! - [`PlotCompute`] for `Boxen` marker struct
//! - [`PlotData`] for `BoxenData`
//! - [`PlotRender`] for `BoxenData`

use crate::core::Result;
use crate::plots::traits::{PlotArea, PlotCompute, PlotConfig, PlotData, PlotRender};
use crate::render::skia::SkiaRenderer;
use crate::render::{Color, LineStyle, Theme};
use crate::stats::quantile::letter_values;

/// Configuration for boxen plot
#[derive(Debug, Clone)]
pub struct BoxenConfig {
    /// Maximum number of letter value levels to show
    pub k_depth: Option<usize>,
    /// Width of boxes (fraction of category spacing)
    pub width: f64,
    /// Colors for boxes (None for auto)
    pub color: Option<Color>,
    /// Saturation gradient (darken toward center)
    pub saturation: f32,
    /// Show outliers
    pub show_outliers: bool,
    /// Outlier marker size
    pub outlier_size: f32,
    /// Line width for box edges
    pub line_width: f32,
    /// Orientation
    pub orient: BoxenOrientation,
}

/// Orientation for boxen plots
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxenOrientation {
    Vertical,
    Horizontal,
}

impl Default for BoxenConfig {
    fn default() -> Self {
        Self {
            k_depth: None, // Auto-determine based on data size
            width: 0.8,
            color: None,
            saturation: 0.75,
            show_outliers: true,
            outlier_size: 4.0,
            line_width: 1.0,
            orient: BoxenOrientation::Vertical,
        }
    }
}

impl BoxenConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set k-depth
    pub fn k_depth(mut self, k: usize) -> Self {
        self.k_depth = Some(k.max(1));
        self
    }

    /// Set width
    pub fn width(mut self, width: f64) -> Self {
        self.width = width.clamp(0.1, 1.0);
        self
    }

    /// Set color
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Show outliers
    pub fn show_outliers(mut self, show: bool) -> Self {
        self.show_outliers = show;
        self
    }

    /// Set horizontal orientation
    pub fn horizontal(mut self) -> Self {
        self.orient = BoxenOrientation::Horizontal;
        self
    }
}

// Implement PlotConfig marker trait
impl PlotConfig for BoxenConfig {}

/// Marker struct for Boxen plot type (used with PlotCompute trait)
pub struct Boxen;

/// A single box in a boxen plot
#[derive(Debug, Clone)]
pub struct BoxenBox {
    /// Level (0 = outermost, increasing toward center)
    pub level: usize,
    /// Lower bound of box
    pub lower: f64,
    /// Upper bound of box
    pub upper: f64,
    /// Width (relative, decreases toward center)
    pub width: f64,
}

/// Computed boxen data for a single distribution
#[derive(Debug, Clone)]
pub struct BoxenData {
    /// All boxes from outer to inner
    pub boxes: Vec<BoxenBox>,
    /// Median value
    pub median: f64,
    /// Outlier values
    pub outliers: Vec<f64>,
    /// Original data range
    pub data_range: (f64, f64),
    /// Configuration used to compute this data
    pub(crate) config: BoxenConfig,
}

/// Compute boxen plot data
///
/// # Arguments
/// * `data` - Input data
/// * `config` - Boxen configuration
///
/// # Returns
/// BoxenData for rendering
pub fn compute_boxen(data: &[f64], config: &BoxenConfig) -> BoxenData {
    if data.is_empty() {
        return BoxenData {
            boxes: vec![],
            median: 0.0,
            outliers: vec![],
            data_range: (0.0, 1.0),
            config: config.clone(),
        };
    }

    let mut sorted: Vec<f64> = data.iter().copied().filter(|x| x.is_finite()).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    if sorted.is_empty() {
        return BoxenData {
            boxes: vec![],
            median: 0.0,
            outliers: vec![],
            data_range: (0.0, 1.0),
            config: config.clone(),
        };
    }

    let n = sorted.len();

    // Determine k depth
    let k = config.k_depth.unwrap_or_else(|| {
        // Tukey's criterion: use log2(n) levels
        ((n as f64).log2().floor() as usize).clamp(1, 10)
    });

    // Get letter values
    let lvs = letter_values(&sorted, Some(k));

    // Create boxes
    let mut boxes = Vec::new();
    let num_levels = lvs.len();

    for (level, (lower, upper)) in lvs.iter().enumerate() {
        // Width decreases toward center
        let width_factor = 1.0 - (level as f64 / (num_levels + 1) as f64) * 0.3;

        boxes.push(BoxenBox {
            level,
            lower: *lower,
            upper: *upper,
            width: config.width * width_factor,
        });
    }

    // Compute median
    let median = if n.is_multiple_of(2) {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    };

    // Find outliers (outside outermost box)
    let outliers = if config.show_outliers && !boxes.is_empty() {
        let outer_lower = boxes[0].lower;
        let outer_upper = boxes[0].upper;
        sorted
            .iter()
            .copied()
            .filter(|&x| x < outer_lower || x > outer_upper)
            .collect()
    } else {
        vec![]
    };

    let data_range = (*sorted.first().unwrap(), *sorted.last().unwrap());

    BoxenData {
        boxes,
        median,
        outliers,
        data_range,
        config: config.clone(),
    }
}

/// Generate box rectangle vertices
pub fn boxen_rect(boxen: &BoxenBox, center: f64, orient: BoxenOrientation) -> Vec<(f64, f64)> {
    let half_width = boxen.width / 2.0;

    match orient {
        BoxenOrientation::Vertical => {
            vec![
                (center - half_width, boxen.lower),
                (center + half_width, boxen.lower),
                (center + half_width, boxen.upper),
                (center - half_width, boxen.upper),
            ]
        }
        BoxenOrientation::Horizontal => {
            vec![
                (boxen.lower, center - half_width),
                (boxen.upper, center - half_width),
                (boxen.upper, center + half_width),
                (boxen.lower, center + half_width),
            ]
        }
    }
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl PlotCompute for Boxen {
    type Input<'a> = &'a [f64];
    type Config = BoxenConfig;
    type Output = BoxenData;

    fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output> {
        let result = compute_boxen(input, config);
        if result.boxes.is_empty() && input.iter().any(|x| x.is_finite()) {
            // Data exists but boxes couldn't be computed - very unusual
            Ok(result)
        } else if result.boxes.is_empty() {
            Err(crate::core::PlottingError::EmptyDataSet)
        } else {
            Ok(result)
        }
    }
}

impl PlotData for BoxenData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        // For vertical boxen: x is centered (use 0-1 range), y is data range
        // For horizontal boxen: x is data range, y is centered
        match self.config.orient {
            BoxenOrientation::Vertical => {
                let x_range = (0.0, 1.0); // Will be adjusted by position
                let y_range = self.data_range;
                (x_range, y_range)
            }
            BoxenOrientation::Horizontal => {
                let x_range = self.data_range;
                let y_range = (0.0, 1.0); // Will be adjusted by position
                (x_range, y_range)
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.boxes.is_empty()
    }
}

impl PlotRender for BoxenData {
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        _theme: &Theme,
        color: Color,
    ) -> Result<()> {
        if self.boxes.is_empty() {
            return Ok(());
        }

        let config = &self.config;
        let center = 0.5; // Centered position for single boxen

        // Draw boxes from outermost to innermost (so inner boxes overlay outer)
        for (i, boxen_box) in self.boxes.iter().enumerate() {
            // Generate saturation gradient (lighter toward outside)
            let saturation_factor = 1.0 - (i as f32 / self.boxes.len() as f32) * config.saturation;
            let box_color = config.color.unwrap_or(color);
            let adjusted_color = adjust_saturation(box_color, saturation_factor);

            // Get rectangle vertices
            let rect = boxen_rect(boxen_box, center, config.orient);

            // Convert to screen coordinates
            let screen_points: Vec<(f32, f32)> = rect
                .iter()
                .map(|(x, y)| area.data_to_screen(*x, *y))
                .collect();

            // Draw filled rectangle
            if screen_points.len() >= 3 {
                renderer.draw_filled_polygon(&screen_points, adjusted_color)?;
            }

            // Draw outline
            if config.line_width > 0.0 {
                let mut outline = screen_points.clone();
                outline.push(screen_points[0]); // Close the path
                renderer.draw_polyline(&outline, color, config.line_width, LineStyle::Solid)?;
            }
        }

        // Draw median line
        let median_half = config.width / 4.0;
        match config.orient {
            BoxenOrientation::Vertical => {
                let (x1, y) = area.data_to_screen(center - median_half, self.median);
                let (x2, _) = area.data_to_screen(center + median_half, self.median);
                renderer.draw_line(
                    x1,
                    y,
                    x2,
                    y,
                    Color::new(255, 255, 255),
                    2.0,
                    LineStyle::Solid,
                )?;
            }
            BoxenOrientation::Horizontal => {
                let (x, y1) = area.data_to_screen(self.median, center - median_half);
                let (_, y2) = area.data_to_screen(self.median, center + median_half);
                renderer.draw_line(
                    x,
                    y1,
                    x,
                    y2,
                    Color::new(255, 255, 255),
                    2.0,
                    LineStyle::Solid,
                )?;
            }
        }

        // Draw outliers
        if config.show_outliers {
            for &outlier in &self.outliers {
                let (px, py) = match config.orient {
                    BoxenOrientation::Vertical => area.data_to_screen(center, outlier),
                    BoxenOrientation::Horizontal => area.data_to_screen(outlier, center),
                };
                renderer.draw_marker(
                    px,
                    py,
                    config.outlier_size,
                    crate::render::MarkerStyle::Circle,
                    color,
                )?;
            }
        }

        Ok(())
    }
}

/// Adjust color saturation (simple approximation)
fn adjust_saturation(color: Color, factor: f32) -> Color {
    // Blend toward gray for lower saturation
    let gray = ((color.r as f32 + color.g as f32 + color.b as f32) / 3.0) as u8;
    let blend = |c: u8| -> u8 {
        ((c as f32 * factor + gray as f32 * (1.0 - factor)).clamp(0.0, 255.0)) as u8
    };
    Color::new_rgba(blend(color.r), blend(color.g), blend(color.b), color.a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boxen_basic() {
        let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let config = BoxenConfig::default();
        let boxen = compute_boxen(&data, &config);

        assert!(!boxen.boxes.is_empty());
        assert!((boxen.median - 49.5).abs() < 1e-10);
    }

    #[test]
    fn test_boxen_nested_boxes() {
        let data: Vec<f64> = (0..1000).map(|i| i as f64).collect();
        let config = BoxenConfig::default().k_depth(5);
        let boxen = compute_boxen(&data, &config);

        assert_eq!(boxen.boxes.len(), 5);

        // Each inner box should be narrower
        for i in 1..boxen.boxes.len() {
            assert!(boxen.boxes[i].width <= boxen.boxes[i - 1].width);
        }
    }

    #[test]
    fn test_boxen_rect_vertical() {
        let box_data = BoxenBox {
            level: 0,
            lower: 10.0,
            upper: 20.0,
            width: 0.8,
        };
        let rect = boxen_rect(&box_data, 0.0, BoxenOrientation::Vertical);

        assert_eq!(rect.len(), 4);
        // Check that rectangle covers the right range
        assert!((rect[0].1 - 10.0).abs() < 1e-10); // lower
        assert!((rect[2].1 - 20.0).abs() < 1e-10); // upper
    }

    #[test]
    fn test_boxen_empty() {
        let data: Vec<f64> = vec![];
        let config = BoxenConfig::default();
        let boxen = compute_boxen(&data, &config);

        assert!(boxen.boxes.is_empty());
    }

    #[test]
    fn test_boxen_config_implements_plot_config() {
        fn assert_plot_config<T: PlotConfig>() {}
        assert_plot_config::<BoxenConfig>();
    }

    #[test]
    fn test_boxen_plot_compute_trait() {
        use crate::plots::traits::PlotCompute;

        let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let config = BoxenConfig::default();
        let result = Boxen::compute(&data, &config);

        assert!(result.is_ok());
        let boxen_data = result.unwrap();
        assert!(!boxen_data.boxes.is_empty());
    }

    #[test]
    fn test_boxen_plot_compute_empty() {
        use crate::plots::traits::PlotCompute;

        let data: Vec<f64> = vec![];
        let config = BoxenConfig::default();
        let result = Boxen::compute(&data, &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_boxen_plot_data_trait() {
        use crate::plots::traits::PlotData;

        let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let config = BoxenConfig::default();
        let boxen_data = compute_boxen(&data, &config);

        // Test data_bounds
        let ((x_min, x_max), (y_min, y_max)) = boxen_data.data_bounds();
        assert!(x_min <= x_max);
        assert!(y_min <= y_max);

        // Test is_empty
        assert!(!boxen_data.is_empty());
    }

    #[test]
    fn test_adjust_saturation() {
        let color = Color::new(100, 150, 200);
        let adjusted = super::adjust_saturation(color, 0.5);
        // Should be blended toward gray
        assert!(adjusted.r > 0 && adjusted.r < 255);
        assert!(adjusted.g > 0 && adjusted.g < 255);
        assert!(adjusted.b > 0 && adjusted.b < 255);
    }
}
