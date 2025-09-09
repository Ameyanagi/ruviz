/// Basic geometric types for plotting

/// 2D point with floating-point coordinates
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2f {
    pub x: f32,
    pub y: f32,
}

impl Point2f {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    
    /// Create from f64 coordinates
    pub fn from_f64(x: f64, y: f64) -> Self {
        Self { x: x as f32, y: y as f32 }
    }
    
    /// Convert to array for SIMD operations
    pub fn to_array(&self) -> [f32; 2] {
        [self.x, self.y]
    }
    
    /// Create from array
    pub fn from_array(arr: [f32; 2]) -> Self {
        Self { x: arr[0], y: arr[1] }
    }
    
    /// Create zero point
    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// Axis-aligned bounding box
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
}

impl BoundingBox {
    pub fn new(min_x: f32, max_x: f32, min_y: f32, max_y: f32) -> Self {
        Self { min_x, max_x, min_y, max_y }
    }
    
    pub fn width(&self) -> f32 {
        self.max_x - self.min_x
    }
    
    pub fn height(&self) -> f32 {
        self.max_y - self.min_y
    }
}