/// Position enumeration for legend and label placement
/// 
/// Supports common positioning options for legends, labels, and other UI elements
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Position {
    /// Top-left corner
    TopLeft,
    /// Top-center
    TopCenter,
    /// Top-right corner
    TopRight,
    /// Center-left (middle-left)
    CenterLeft,
    /// Center (middle-center)
    Center,
    /// Center-right (middle-right)
    CenterRight,
    /// Bottom-left corner
    BottomLeft,
    /// Bottom-center
    BottomCenter,
    /// Bottom-right corner
    BottomRight,
    /// Custom position with exact coordinates
    Custom { x: f32, y: f32 },
}

impl Position {
    /// Convert position to normalized coordinates (0.0-1.0)
    /// Returns (x, y) where (0,0) is top-left and (1,1) is bottom-right
    pub fn to_normalized_coords(self) -> (f32, f32) {
        match self {
            Position::TopLeft => (0.0, 0.0),
            Position::TopCenter => (0.5, 0.0),
            Position::TopRight => (1.0, 0.0),
            Position::CenterLeft => (0.0, 0.5),
            Position::Center => (0.5, 0.5),
            Position::CenterRight => (1.0, 0.5),
            Position::BottomLeft => (0.0, 1.0),
            Position::BottomCenter => (0.5, 1.0),
            Position::BottomRight => (1.0, 1.0),
            Position::Custom { x, y } => (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)),
        }
    }
    
    /// Convert position to pixel coordinates given canvas dimensions
    /// Returns (x, y) in pixels from top-left corner
    pub fn to_pixel_coords(self, width: u32, height: u32) -> (f32, f32) {
        let (norm_x, norm_y) = self.to_normalized_coords();
        (norm_x * width as f32, norm_y * height as f32)
    }
    
    /// Get the anchor point for text/legend alignment
    /// Returns (horizontal_anchor, vertical_anchor) where:
    /// - horizontal: 0.0 = left-aligned, 0.5 = center-aligned, 1.0 = right-aligned
    /// - vertical: 0.0 = top-aligned, 0.5 = center-aligned, 1.0 = bottom-aligned
    pub fn get_anchor(self) -> (f32, f32) {
        match self {
            Position::TopLeft => (0.0, 0.0),
            Position::TopCenter => (0.5, 0.0),
            Position::TopRight => (1.0, 0.0),
            Position::CenterLeft => (0.0, 0.5),
            Position::Center => (0.5, 0.5),
            Position::CenterRight => (1.0, 0.5),
            Position::BottomLeft => (0.0, 1.0),
            Position::BottomCenter => (0.5, 1.0),
            Position::BottomRight => (1.0, 1.0),
            Position::Custom { .. } => (0.0, 0.0), // Default anchor for custom positions
        }
    }
    
    /// Create a custom position from normalized coordinates (0.0-1.0)
    pub fn custom(x: f32, y: f32) -> Self {
        Position::Custom { 
            x: x.clamp(0.0, 1.0), 
            y: y.clamp(0.0, 1.0) 
        }
    }
    
    /// Check if this is a corner position
    pub fn is_corner(self) -> bool {
        matches!(
            self,
            Position::TopLeft | Position::TopRight | Position::BottomLeft | Position::BottomRight
        )
    }
    
    /// Check if this is an edge position (not corner, not center)
    pub fn is_edge(self) -> bool {
        matches!(
            self,
            Position::TopCenter | Position::CenterLeft | Position::CenterRight | Position::BottomCenter
        )
    }
    
    /// Check if this is the center position
    pub fn is_center(self) -> bool {
        matches!(self, Position::Center)
    }
    
    /// Get the opposite position (useful for automatic legend placement)
    pub fn opposite(self) -> Self {
        match self {
            Position::TopLeft => Position::BottomRight,
            Position::TopCenter => Position::BottomCenter,
            Position::TopRight => Position::BottomLeft,
            Position::CenterLeft => Position::CenterRight,
            Position::Center => Position::Center, // Center is its own opposite
            Position::CenterRight => Position::CenterLeft,
            Position::BottomLeft => Position::TopRight,
            Position::BottomCenter => Position::TopCenter,
            Position::BottomRight => Position::TopLeft,
            Position::Custom { x, y } => Position::Custom { x: 1.0 - x, y: 1.0 - y },
        }
    }
    
    /// Get recommended padding/margin based on position
    /// Returns (horizontal_margin, vertical_margin) as fractions of canvas size
    pub fn get_recommended_margin(self) -> (f32, f32) {
        const CORNER_MARGIN: f32 = 0.05; // 5% margin for corners
        const EDGE_MARGIN: f32 = 0.03;   // 3% margin for edges
        const CENTER_MARGIN: f32 = 0.0;  // No margin for center
        
        match self {
            Position::TopLeft | Position::TopRight | Position::BottomLeft | Position::BottomRight => {
                (CORNER_MARGIN, CORNER_MARGIN)
            }
            Position::TopCenter | Position::BottomCenter => (0.0, EDGE_MARGIN),
            Position::CenterLeft | Position::CenterRight => (EDGE_MARGIN, 0.0),
            Position::Center => (CENTER_MARGIN, CENTER_MARGIN),
            Position::Custom { .. } => (0.0, 0.0), // No automatic margin for custom positions
        }
    }
}

impl Default for Position {
    /// Default position is TopRight (common for legends)
    fn default() -> Self {
        Position::TopRight
    }
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Position::TopLeft => write!(f, "top-left"),
            Position::TopCenter => write!(f, "top-center"),
            Position::TopRight => write!(f, "top-right"),
            Position::CenterLeft => write!(f, "center-left"),
            Position::Center => write!(f, "center"),
            Position::CenterRight => write!(f, "center-right"),
            Position::BottomLeft => write!(f, "bottom-left"),
            Position::BottomCenter => write!(f, "bottom-center"),
            Position::BottomRight => write!(f, "bottom-right"),
            Position::Custom { x, y } => write!(f, "custom({:.2}, {:.2})", x, y),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalized_coordinates() {
        assert_eq!(Position::TopLeft.to_normalized_coords(), (0.0, 0.0));
        assert_eq!(Position::Center.to_normalized_coords(), (0.5, 0.5));
        assert_eq!(Position::BottomRight.to_normalized_coords(), (1.0, 1.0));
        assert_eq!(Position::custom(0.25, 0.75).to_normalized_coords(), (0.25, 0.75));
        
        // Test clamping
        assert_eq!(Position::custom(-0.5, 1.5).to_normalized_coords(), (0.0, 1.0));
    }

    #[test]
    fn test_pixel_coordinates() {
        assert_eq!(Position::TopLeft.to_pixel_coords(800, 600), (0.0, 0.0));
        assert_eq!(Position::Center.to_pixel_coords(800, 600), (400.0, 300.0));
        assert_eq!(Position::BottomRight.to_pixel_coords(800, 600), (800.0, 600.0));
    }

    #[test]
    fn test_anchor_points() {
        assert_eq!(Position::TopLeft.get_anchor(), (0.0, 0.0));
        assert_eq!(Position::Center.get_anchor(), (0.5, 0.5));
        assert_eq!(Position::BottomRight.get_anchor(), (1.0, 1.0));
    }

    #[test]
    fn test_position_types() {
        assert!(Position::TopLeft.is_corner());
        assert!(!Position::TopCenter.is_corner());
        assert!(Position::TopCenter.is_edge());
        assert!(!Position::TopLeft.is_edge());
        assert!(Position::Center.is_center());
        assert!(!Position::TopLeft.is_center());
    }

    #[test]
    fn test_opposite_positions() {
        assert_eq!(Position::TopLeft.opposite(), Position::BottomRight);
        assert_eq!(Position::Center.opposite(), Position::Center);
        assert_eq!(Position::custom(0.25, 0.75).opposite(), Position::custom(0.75, 0.25));
    }

    #[test]
    fn test_margins() {
        let (h_margin, v_margin) = Position::TopLeft.get_recommended_margin();
        assert_eq!(h_margin, 0.05);
        assert_eq!(v_margin, 0.05);
        
        let (h_margin, v_margin) = Position::TopCenter.get_recommended_margin();
        assert_eq!(h_margin, 0.0);
        assert_eq!(v_margin, 0.03);
        
        let (h_margin, v_margin) = Position::Center.get_recommended_margin();
        assert_eq!(h_margin, 0.0);
        assert_eq!(v_margin, 0.0);
    }

    #[test]
    fn test_default_position() {
        assert_eq!(Position::default(), Position::TopRight);
    }

    #[test]
    fn test_display() {
        assert_eq!(Position::TopLeft.to_string(), "top-left");
        assert_eq!(Position::custom(0.25, 0.75).to_string(), "custom(0.25, 0.75)");
    }
}