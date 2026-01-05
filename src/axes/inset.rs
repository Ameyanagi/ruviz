//! Inset (zoom) axes support
//!
//! Provides inset axes for zoomed views of data regions.

/// Inset axes configuration
#[derive(Debug, Clone)]
pub struct InsetAxes {
    /// Bounds of the inset in parent axes coordinates (x, y, width, height)
    /// Values are fractions of parent axes (0.0 to 1.0)
    pub bounds: (f64, f64, f64, f64),
    /// Data limits for the inset (x_min, x_max, y_min, y_max)
    pub data_limits: Option<(f64, f64, f64, f64)>,
    /// Whether to draw connectors to zoom region
    pub draw_connectors: bool,
    /// Connector line style
    pub connector_style: ConnectorStyle,
    /// Border style for inset axes
    pub border_color: Option<String>,
    /// Border width
    pub border_width: f64,
    /// Background color (None for transparent)
    pub background: Option<String>,
}

/// Style of connector lines between inset and zoom region
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorStyle {
    /// Straight lines from corners
    Corners,
    /// Lines from edges
    Edges,
    /// No connectors
    None,
}

impl Default for InsetAxes {
    fn default() -> Self {
        Self {
            bounds: (0.5, 0.5, 0.4, 0.4), // Top-right quadrant
            data_limits: None,
            draw_connectors: true,
            connector_style: ConnectorStyle::Corners,
            border_color: Some("#333333".to_string()),
            border_width: 1.0,
            background: Some("#ffffff".to_string()),
        }
    }
}

impl InsetAxes {
    /// Create new inset axes at specified location
    ///
    /// # Arguments
    /// * `x` - X position (fraction of parent, 0.0 = left)
    /// * `y` - Y position (fraction of parent, 0.0 = bottom)
    /// * `width` - Width (fraction of parent)
    /// * `height` - Height (fraction of parent)
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            bounds: (
                x.clamp(0.0, 1.0),
                y.clamp(0.0, 1.0),
                width.clamp(0.0, 1.0),
                height.clamp(0.0, 1.0),
            ),
            ..Default::default()
        }
    }

    /// Set the data limits for the zoomed region
    pub fn data_limits(mut self, x_min: f64, x_max: f64, y_min: f64, y_max: f64) -> Self {
        self.data_limits = Some((x_min, x_max, y_min, y_max));
        self
    }

    /// Enable or disable connector lines
    pub fn connectors(mut self, draw: bool) -> Self {
        self.draw_connectors = draw;
        self
    }

    /// Set connector style
    pub fn connector_style(mut self, style: ConnectorStyle) -> Self {
        self.connector_style = style;
        self
    }

    /// Set border color
    pub fn border_color(mut self, color: impl Into<String>) -> Self {
        self.border_color = Some(color.into());
        self
    }

    /// Set border width
    pub fn border_width(mut self, width: f64) -> Self {
        self.border_width = width.max(0.0);
        self
    }

    /// Set background color
    pub fn background(mut self, color: impl Into<String>) -> Self {
        self.background = Some(color.into());
        self
    }

    /// Remove background (transparent)
    pub fn transparent(mut self) -> Self {
        self.background = None;
        self
    }

    /// Convert parent axes coordinates to inset axes coordinates
    ///
    /// # Arguments
    /// * `parent_x` - X coordinate in parent axes (0.0 to 1.0)
    /// * `parent_y` - Y coordinate in parent axes (0.0 to 1.0)
    ///
    /// # Returns
    /// (x, y) in inset axes coordinates, or None if outside inset
    pub fn parent_to_inset(&self, parent_x: f64, parent_y: f64) -> Option<(f64, f64)> {
        let (ix, iy, iw, ih) = self.bounds;

        if parent_x < ix || parent_x > ix + iw || parent_y < iy || parent_y > iy + ih {
            return None;
        }

        Some(((parent_x - ix) / iw, (parent_y - iy) / ih))
    }

    /// Convert inset axes coordinates to parent axes coordinates
    pub fn inset_to_parent(&self, inset_x: f64, inset_y: f64) -> (f64, f64) {
        let (ix, iy, iw, ih) = self.bounds;
        (ix + inset_x * iw, iy + inset_y * ih)
    }

    /// Get the zoom region rectangle in data coordinates
    pub fn zoom_region(&self) -> Option<(f64, f64, f64, f64)> {
        self.data_limits
    }

    /// Calculate connector line endpoints
    ///
    /// Returns pairs of (parent_point, inset_point) for each connector
    pub fn connector_lines(
        &self,
        zoom_region: (f64, f64, f64, f64),
    ) -> Vec<((f64, f64), (f64, f64))> {
        if !self.draw_connectors || self.connector_style == ConnectorStyle::None {
            return vec![];
        }

        let (zx_min, zx_max, zy_min, zy_max) = zoom_region;
        let (ix, iy, iw, ih) = self.bounds;

        match self.connector_style {
            ConnectorStyle::Corners => {
                // Connect zoom region corners to inset corners
                vec![
                    ((zx_min, zy_min), (ix, iy)),           // Bottom-left
                    ((zx_max, zy_max), (ix + iw, iy + ih)), // Top-right
                ]
            }
            ConnectorStyle::Edges => {
                // Connect zoom region edges to inset edges
                let zx_mid = (zx_min + zx_max) / 2.0;
                let zy_mid = (zy_min + zy_max) / 2.0;
                let ix_mid = ix + iw / 2.0;
                let iy_mid = iy + ih / 2.0;

                vec![
                    ((zx_mid, zy_max), (ix_mid, iy)), // Top edge
                    ((zx_max, zy_mid), (ix, iy_mid)), // Right edge
                ]
            }
            ConnectorStyle::None => vec![],
        }
    }

    /// Get the screen rectangle for the inset
    ///
    /// # Arguments
    /// * `parent_rect` - Parent axes rectangle (x, y, width, height) in screen coords
    ///
    /// # Returns
    /// Inset rectangle (x, y, width, height) in screen coordinates
    pub fn screen_rect(&self, parent_rect: (f64, f64, f64, f64)) -> (f64, f64, f64, f64) {
        let (px, py, pw, ph) = parent_rect;
        let (ix, iy, iw, ih) = self.bounds;

        (
            px + ix * pw,
            py + (1.0 - iy - ih) * ph, // Flip y for screen coords
            iw * pw,
            ih * ph,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inset_creation() {
        let inset = InsetAxes::new(0.6, 0.6, 0.35, 0.35);
        assert!((inset.bounds.0 - 0.6).abs() < 1e-10);
        assert!((inset.bounds.1 - 0.6).abs() < 1e-10);
    }

    #[test]
    fn test_coordinate_transform() {
        let inset = InsetAxes::new(0.5, 0.5, 0.4, 0.4);

        // Point at center of inset
        let (x, y) = inset.inset_to_parent(0.5, 0.5);
        assert!((x - 0.7).abs() < 1e-10);
        assert!((y - 0.7).abs() < 1e-10);

        // Reverse transform
        let result = inset.parent_to_inset(0.7, 0.7);
        assert!(result.is_some());
        let (ix, iy) = result.unwrap();
        assert!((ix - 0.5).abs() < 1e-10);
        assert!((iy - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_parent_to_inset_outside() {
        let inset = InsetAxes::new(0.5, 0.5, 0.4, 0.4);

        // Point outside inset
        let result = inset.parent_to_inset(0.1, 0.1);
        assert!(result.is_none());
    }

    #[test]
    fn test_screen_rect() {
        let inset = InsetAxes::new(0.5, 0.5, 0.4, 0.4);
        let parent = (100.0, 100.0, 400.0, 300.0);

        let (x, y, w, h) = inset.screen_rect(parent);
        assert!((x - 300.0).abs() < 1e-10); // 100 + 0.5 * 400
        assert!((w - 160.0).abs() < 1e-10); // 0.4 * 400
        assert!((h - 120.0).abs() < 1e-10); // 0.4 * 300
    }

    #[test]
    fn test_connector_lines() {
        let inset = InsetAxes::new(0.6, 0.6, 0.35, 0.35);
        let zoom = (0.1, 0.3, 0.2, 0.4);

        let connectors = inset.connector_lines(zoom);
        assert_eq!(connectors.len(), 2); // Corners style
    }
}
