//! Layout management for plots
//!
//! This module provides the [`LayoutManager`] struct which handles
//! layout-related configuration for plots including legend, grid,
//! tick marks, margins, and axis settings.

use crate::axes::AxisScale;
use crate::core::{GridStyle, Position};

use super::{LegendConfig, TickConfig};

/// Manages layout configuration for plots
///
/// The LayoutManager handles:
/// - Legend configuration and positioning
/// - Grid styling
/// - Tick mark configuration
/// - Margins
/// - Axis limits and scales
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::core::plot::LayoutManager;
///
/// let mut layout = LayoutManager::new();
/// layout.set_xlim(0.0, 10.0);
/// layout.set_ylim(-5.0, 5.0);
/// layout.enable_legend(true);
/// ```
#[derive(Clone, Debug)]
pub struct LayoutManager {
    /// Legend configuration
    pub(crate) legend: LegendConfig,
    /// Grid styling configuration
    pub(crate) grid_style: GridStyle,
    /// Tick configuration
    pub(crate) tick_config: TickConfig,
    /// Margin around plot area (fraction of canvas)
    pub(crate) margin: Option<f32>,
    /// Whether to use scientific notation on axes
    pub(crate) scientific_notation: bool,
    /// Manual X-axis limits (min, max)
    pub(crate) x_limits: Option<(f64, f64)>,
    /// Manual Y-axis limits (min, max)
    pub(crate) y_limits: Option<(f64, f64)>,
    /// X-axis scale (linear, log, symlog)
    pub(crate) x_scale: AxisScale,
    /// Y-axis scale (linear, log, symlog)
    pub(crate) y_scale: AxisScale,
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutManager {
    /// Create a new layout manager with default settings
    pub fn new() -> Self {
        Self {
            legend: LegendConfig::default(),
            grid_style: GridStyle::default(),
            tick_config: TickConfig::default(),
            margin: None,
            scientific_notation: false,
            x_limits: None,
            y_limits: None,
            x_scale: AxisScale::Linear,
            y_scale: AxisScale::Linear,
        }
    }

    // Legend methods

    /// Enable or disable the legend
    pub fn enable_legend(&mut self, enabled: bool) {
        self.legend.enabled = enabled;
    }

    /// Check if legend is enabled
    pub fn legend_enabled(&self) -> bool {
        self.legend.enabled
    }

    /// Set legend position
    pub fn set_legend_position(&mut self, position: Position) {
        self.legend.position = position;
    }

    /// Get legend position
    pub fn legend_position(&self) -> Position {
        self.legend.position
    }

    /// Set legend font size
    pub fn set_legend_font_size(&mut self, size: f32) {
        self.legend.font_size = Some(size);
    }

    /// Set legend corner radius
    pub fn set_legend_corner_radius(&mut self, radius: f32) {
        self.legend.corner_radius = Some(radius);
    }

    /// Set number of legend columns
    pub fn set_legend_columns(&mut self, cols: usize) {
        self.legend.columns = Some(cols);
    }

    // Grid methods

    /// Set grid style
    pub fn set_grid_style(&mut self, style: GridStyle) {
        self.grid_style = style;
    }

    /// Get grid style
    pub fn grid_style(&self) -> &GridStyle {
        &self.grid_style
    }

    /// Get mutable reference to grid style
    pub fn grid_style_mut(&mut self) -> &mut GridStyle {
        &mut self.grid_style
    }

    // Axis limit methods

    /// Set X-axis limits
    pub fn set_xlim(&mut self, min: f64, max: f64) {
        self.x_limits = Some((min, max));
    }

    /// Set Y-axis limits
    pub fn set_ylim(&mut self, min: f64, max: f64) {
        self.y_limits = Some((min, max));
    }

    /// Get X-axis limits
    pub fn xlim(&self) -> Option<(f64, f64)> {
        self.x_limits
    }

    /// Get Y-axis limits
    pub fn ylim(&self) -> Option<(f64, f64)> {
        self.y_limits
    }

    // Axis scale methods

    /// Set X-axis scale
    pub fn set_x_scale(&mut self, scale: AxisScale) {
        self.x_scale = scale;
    }

    /// Set Y-axis scale
    pub fn set_y_scale(&mut self, scale: AxisScale) {
        self.y_scale = scale;
    }

    /// Get X-axis scale
    pub fn x_scale(&self) -> &AxisScale {
        &self.x_scale
    }

    /// Get Y-axis scale
    pub fn y_scale(&self) -> &AxisScale {
        &self.y_scale
    }

    // Margin methods

    /// Set margin as fraction of canvas
    pub fn set_margin(&mut self, margin: f32) {
        self.margin = Some(margin);
    }

    /// Get margin
    pub fn margin(&self) -> Option<f32> {
        self.margin
    }

    // Scientific notation

    /// Enable or disable scientific notation on axes
    pub fn set_scientific_notation(&mut self, enabled: bool) {
        self.scientific_notation = enabled;
    }

    /// Check if scientific notation is enabled
    pub fn scientific_notation(&self) -> bool {
        self.scientific_notation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_layout_manager() {
        let layout = LayoutManager::new();
        assert!(!layout.legend_enabled());
        assert!(layout.xlim().is_none());
        assert!(layout.ylim().is_none());
        assert!(layout.margin().is_none());
        assert!(!layout.scientific_notation());
    }

    #[test]
    fn test_axis_limits() {
        let mut layout = LayoutManager::new();
        layout.set_xlim(0.0, 100.0);
        layout.set_ylim(-50.0, 50.0);

        assert_eq!(layout.xlim(), Some((0.0, 100.0)));
        assert_eq!(layout.ylim(), Some((-50.0, 50.0)));
    }

    #[test]
    fn test_legend_config() {
        let mut layout = LayoutManager::new();
        layout.enable_legend(true);
        layout.set_legend_position(Position::BottomLeft);
        layout.set_legend_font_size(12.0);

        assert!(layout.legend_enabled());
        assert_eq!(layout.legend_position(), Position::BottomLeft);
        assert_eq!(layout.legend.font_size, Some(12.0));
    }

    #[test]
    fn test_axis_scales() {
        let mut layout = LayoutManager::new();
        layout.set_x_scale(AxisScale::Log);
        layout.set_y_scale(AxisScale::SymLog { linthresh: 1.0 });

        assert!(matches!(layout.x_scale(), AxisScale::Log));
        assert!(matches!(layout.y_scale(), AxisScale::SymLog { .. }));
    }
}
