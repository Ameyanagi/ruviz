//! Basic geometric types for plotting

/// Orientation for plots that support horizontal/vertical rendering
///
/// Many plot types (boxplot, violin, bar, etc.) can be rendered in either
/// horizontal or vertical orientation. This enum provides a unified way
/// to specify the orientation.
///
/// # Default
///
/// `Orientation::Vertical` is the default for most plots.
///
/// # Matplotlib/Seaborn Compatibility
///
/// - `Vertical`: Standard orientation (categories on x-axis, values on y-axis)
/// - `Horizontal`: Rotated orientation (categories on y-axis, values on x-axis)
///   - In matplotlib: `orient='h'` or `orientation='horizontal'`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    /// Vertical orientation (default)
    /// Categories on x-axis, values on y-axis
    #[default]
    Vertical,
    /// Horizontal orientation
    /// Categories on y-axis, values on x-axis
    Horizontal,
}

impl Orientation {
    /// Check if this is horizontal orientation
    pub fn is_horizontal(&self) -> bool {
        matches!(self, Orientation::Horizontal)
    }

    /// Check if this is vertical orientation
    pub fn is_vertical(&self) -> bool {
        matches!(self, Orientation::Vertical)
    }

    /// Swap x and y coordinates based on orientation
    ///
    /// For horizontal orientation, swaps the coordinates.
    /// For vertical orientation, returns unchanged.
    pub fn transform_xy<T: Copy>(&self, x: T, y: T) -> (T, T) {
        match self {
            Orientation::Vertical => (x, y),
            Orientation::Horizontal => (y, x),
        }
    }
}

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
        Self {
            x: x as f32,
            y: y as f32,
        }
    }

    /// Convert to array for SIMD operations
    pub fn to_array(self) -> [f32; 2] {
        [self.x, self.y]
    }

    /// Create from array
    pub fn from_array(arr: [f32; 2]) -> Self {
        Self {
            x: arr[0],
            y: arr[1],
        }
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
        Self {
            min_x,
            max_x,
            min_y,
            max_y,
        }
    }

    pub fn width(&self) -> f32 {
        self.max_x - self.min_x
    }

    pub fn height(&self) -> f32 {
        self.max_y - self.min_y
    }
}

/// 2D affine transformation matrix
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform2D {
    pub m11: f32,
    pub m12: f32,
    pub m21: f32,
    pub m22: f32,
    pub tx: f32,
    pub ty: f32,
}

impl Transform2D {
    /// Identity transformation
    pub fn identity() -> Self {
        Self {
            m11: 1.0,
            m12: 0.0,
            m21: 0.0,
            m22: 1.0,
            tx: 0.0,
            ty: 0.0,
        }
    }

    /// Translation transformation
    pub fn translation(tx: f32, ty: f32) -> Self {
        Self {
            m11: 1.0,
            m12: 0.0,
            m21: 0.0,
            m22: 1.0,
            tx,
            ty,
        }
    }

    /// Uniform scale transformation
    pub fn scale(scale: f32) -> Self {
        Self {
            m11: scale,
            m12: 0.0,
            m21: 0.0,
            m22: scale,
            tx: 0.0,
            ty: 0.0,
        }
    }
}
