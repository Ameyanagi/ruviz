//! Image representation for rendered plots

/// In-memory image representation
#[derive(Debug, Clone)]
pub struct Image {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Pixel data in RGBA format
    pub pixels: Vec<u8>,
}

impl Image {
    /// Create a new image
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        Self {
            width,
            height,
            pixels,
        }
    }

    /// Get image width
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get image height
    pub fn height(&self) -> u32 {
        self.height
    }
}
