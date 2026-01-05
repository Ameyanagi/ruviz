//! Pie chart implementation
//!
//! Provides pie and donut chart functionality with labels, percentages, and explode.

use crate::render::primitives::{Wedge, pie_wedges};
use crate::render::{Color, SkiaRenderer, Theme};
use std::f64::consts::PI;

/// Configuration for pie chart
#[derive(Debug, Clone)]
pub struct PieConfig {
    /// Labels for each wedge
    pub labels: Vec<String>,
    /// Colors for each wedge (None for auto-colors from palette)
    pub colors: Option<Vec<Color>>,
    /// Explode offset for each wedge (0.0 = no explode)
    pub explode: Vec<f64>,
    /// Show percentage labels
    pub show_percentages: bool,
    /// Show value labels
    pub show_values: bool,
    /// Show labels (category names)
    pub show_labels: bool,
    /// Inner radius for donut chart (0.0 = full pie)
    pub inner_radius: f64,
    /// Start angle in degrees (0 = 3 o'clock, 90 = 12 o'clock)
    pub start_angle: f64,
    /// Whether to go counter-clockwise
    pub counter_clockwise: bool,
    /// Text color for labels
    pub text_color: Color,
    /// Font size for labels
    pub label_font_size: f32,
    /// Distance from center for labels (as fraction of radius)
    pub label_distance: f64,
    /// Shadow offset (0 = no shadow)
    pub shadow: f64,
    /// Edge color for wedges (None for no edge)
    pub edge_color: Option<Color>,
    /// Edge width
    pub edge_width: f32,
}

impl Default for PieConfig {
    fn default() -> Self {
        Self {
            labels: vec![],
            colors: None,
            explode: vec![],
            show_percentages: true,
            show_values: false,
            show_labels: true,
            inner_radius: 0.0,
            start_angle: 90.0, // Start at top (12 o'clock)
            counter_clockwise: true,
            text_color: Color::new(0, 0, 0),
            label_font_size: 10.0,
            label_distance: 0.6,
            shadow: 0.0,
            edge_color: Some(Color::new(255, 255, 255)),
            edge_width: 1.0,
        }
    }
}

impl PieConfig {
    /// Create a new pie config with labels
    pub fn new(labels: Vec<String>) -> Self {
        Self {
            labels,
            ..Default::default()
        }
    }

    /// Set colors for wedges
    pub fn colors(mut self, colors: Vec<Color>) -> Self {
        self.colors = Some(colors);
        self
    }

    /// Set explode offsets for wedges
    pub fn explode(mut self, explode: Vec<f64>) -> Self {
        self.explode = explode;
        self
    }

    /// Create a donut chart with inner radius
    pub fn donut(mut self, inner_radius: f64) -> Self {
        self.inner_radius = inner_radius.clamp(0.0, 0.95);
        self
    }

    /// Set start angle in degrees
    pub fn start_angle(mut self, angle: f64) -> Self {
        self.start_angle = angle;
        self
    }

    /// Go clockwise instead of counter-clockwise
    pub fn clockwise(mut self) -> Self {
        self.counter_clockwise = false;
        self
    }

    /// Show/hide percentage labels
    pub fn percentages(mut self, show: bool) -> Self {
        self.show_percentages = show;
        self
    }

    /// Show/hide value labels
    pub fn values(mut self, show: bool) -> Self {
        self.show_values = show;
        self
    }

    /// Show/hide category labels
    pub fn labels(mut self, show: bool) -> Self {
        self.show_labels = show;
        self
    }

    /// Set label font size
    pub fn font_size(mut self, size: f32) -> Self {
        self.label_font_size = size;
        self
    }

    /// Set label distance from center
    pub fn label_distance(mut self, distance: f64) -> Self {
        self.label_distance = distance;
        self
    }

    /// Set edge color
    pub fn edge_color(mut self, color: Color) -> Self {
        self.edge_color = Some(color);
        self
    }

    /// Remove edge
    pub fn no_edge(mut self) -> Self {
        self.edge_color = None;
        self
    }
}

/// Pie chart data and computed wedges
#[derive(Debug, Clone)]
pub struct PieData {
    /// Original values
    pub values: Vec<f64>,
    /// Computed wedges
    pub wedges: Vec<Wedge>,
    /// Total sum of values
    pub total: f64,
    /// Percentages for each wedge
    pub percentages: Vec<f64>,
}

impl PieData {
    /// Create pie data from values
    pub fn from_values(values: &[f64], cx: f64, cy: f64, radius: f64, config: &PieConfig) -> Self {
        let positive_values: Vec<f64> = values.iter().filter(|&&v| v > 0.0).copied().collect();
        let total: f64 = positive_values.iter().sum();

        let percentages: Vec<f64> = if total > 0.0 {
            positive_values.iter().map(|v| v / total * 100.0).collect()
        } else {
            vec![0.0; positive_values.len()]
        };

        // Convert start angle to radians and adjust for convention
        let start_angle_rad = config.start_angle * PI / 180.0 - PI / 2.0;
        let direction = if config.counter_clockwise { 1.0 } else { -1.0 };

        let mut wedges = pie_wedges(&positive_values, cx, cy, radius, Some(start_angle_rad));

        // Apply configuration to wedges
        for (i, wedge) in wedges.iter_mut().enumerate() {
            // Apply inner radius for donut
            if config.inner_radius > 0.0 {
                *wedge = wedge.inner_radius(radius * config.inner_radius);
            }

            // Apply explode
            if i < config.explode.len() && config.explode[i] > 0.0 {
                *wedge = wedge.explode(config.explode[i] * radius * 0.1);
            }
        }

        Self {
            values: positive_values,
            wedges,
            total,
            percentages,
        }
    }
}

/// Render a pie chart
///
/// # Arguments
/// * `renderer` - The Skia renderer
/// * `values` - Numeric values for each wedge
/// * `cx` - Center X coordinate
/// * `cy` - Center Y coordinate
/// * `radius` - Outer radius
/// * `config` - Pie chart configuration
/// * `theme` - Color theme
///
/// # Returns
/// Result with PieData containing computed wedges
pub fn render_pie(
    renderer: &mut SkiaRenderer,
    values: &[f64],
    cx: f64,
    cy: f64,
    radius: f64,
    config: &PieConfig,
    theme: &Theme,
) -> crate::core::Result<PieData> {
    let pie_data = PieData::from_values(values, cx, cy, radius, config);

    if pie_data.wedges.is_empty() {
        return Ok(pie_data);
    }

    // Get colors from config or theme palette
    let colors = if let Some(ref colors) = config.colors {
        colors.clone()
    } else {
        let palette = theme.color_palette.clone();
        (0..pie_data.wedges.len())
            .map(|i| palette[i % palette.len()])
            .collect()
    };

    // Number of segments per arc for smooth curves
    let segments = 64;

    // Draw shadow if configured
    if config.shadow > 0.0 {
        let shadow_color = Color::new(100, 100, 100).with_alpha(0.3);
        let offset = config.shadow;
        for wedge in &pie_data.wedges {
            let mut shadow_wedge = *wedge;
            // Offset shadow
            let polygon = shadow_wedge.as_polygon(segments);
            let shadow_polygon: Vec<(f32, f32)> = polygon
                .iter()
                .map(|(x, y)| ((x + offset) as f32, (y + offset) as f32))
                .collect();
            renderer.draw_filled_polygon(&shadow_polygon, shadow_color)?;
        }
    }

    // Draw wedges
    for (i, wedge) in pie_data.wedges.iter().enumerate() {
        let color = colors[i % colors.len()];
        let polygon = wedge.as_polygon(segments);
        let polygon_f32: Vec<(f32, f32)> = polygon
            .iter()
            .map(|(x, y)| (*x as f32, *y as f32))
            .collect();

        // Draw filled wedge
        renderer.draw_filled_polygon(&polygon_f32, color)?;

        // Draw edge if configured
        if let Some(edge_color) = config.edge_color {
            renderer.draw_polygon_outline(&polygon_f32, edge_color, config.edge_width)?;
        }
    }

    // Draw labels
    if config.show_labels || config.show_percentages || config.show_values {
        for (i, wedge) in pie_data.wedges.iter().enumerate() {
            let label_parts: Vec<String> = [
                if config.show_labels && i < config.labels.len() {
                    Some(config.labels[i].clone())
                } else {
                    None
                },
                if config.show_percentages {
                    Some(format!("{:.1}%", pie_data.percentages[i]))
                } else {
                    None
                },
                if config.show_values {
                    Some(format!("{:.1}", pie_data.values[i]))
                } else {
                    None
                },
            ]
            .into_iter()
            .flatten()
            .collect();

            if !label_parts.is_empty() {
                let label = label_parts.join("\n");

                // Calculate label position
                let label_r = if config.inner_radius > 0.0 {
                    radius * (1.0 + config.inner_radius) / 2.0 * config.label_distance
                } else {
                    radius * config.label_distance
                };

                let (lx, ly) = wedge.centroid();
                let mid_angle = (wedge.start_angle + wedge.end_angle) / 2.0;
                let label_x = cx + label_r * mid_angle.cos();
                let label_y = cy + label_r * mid_angle.sin();

                renderer.draw_text_centered(
                    &label,
                    label_x as f32,
                    label_y as f32,
                    config.label_font_size,
                    config.text_color,
                )?;
            }
        }
    }

    Ok(pie_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pie_data_basic() {
        let values = vec![30.0, 20.0, 50.0];
        let config = PieConfig::default();
        let data = PieData::from_values(&values, 100.0, 100.0, 50.0, &config);

        assert_eq!(data.wedges.len(), 3);
        assert!((data.total - 100.0).abs() < 1e-10);
        assert!((data.percentages[0] - 30.0).abs() < 1e-10);
        assert!((data.percentages[1] - 20.0).abs() < 1e-10);
        assert!((data.percentages[2] - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_pie_config_donut() {
        let config = PieConfig::default().donut(0.5);
        assert!((config.inner_radius - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_pie_data_with_explode() {
        let values = vec![25.0, 25.0, 50.0];
        let config = PieConfig::default().explode(vec![0.1, 0.0, 0.0]);
        let data = PieData::from_values(&values, 100.0, 100.0, 50.0, &config);

        assert_eq!(data.wedges.len(), 3);
        // First wedge should be exploded
        assert!(data.wedges[0].explode > 0.0);
        assert!((data.wedges[1].explode - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_pie_ignores_negative() {
        let values = vec![30.0, -10.0, 20.0];
        let config = PieConfig::default();
        let data = PieData::from_values(&values, 100.0, 100.0, 50.0, &config);

        // Should only have 2 wedges (negative value filtered)
        assert_eq!(data.wedges.len(), 2);
    }
}
