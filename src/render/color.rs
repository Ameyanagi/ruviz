use std::fmt;

/// Color representation for plot elements
/// 
/// Supports predefined colors and custom RGB/RGBA values with hex parsing
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8, // Alpha channel (0-255)
}

impl Color {
    /// Create a new Color from RGB values (alpha = 255)
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    
    /// Create a new Color from RGBA values
    pub fn new_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    
    /// Create a Color from hex string (supports #RGB, #RRGGBB, #RRGGBBAA)
    pub fn from_hex(hex: &str) -> Result<Self, ColorError> {
        let hex = hex.trim_start_matches('#');
        
        match hex.len() {
            3 => {
                // #RGB format
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16)
                    .map_err(|_| ColorError::InvalidHex)?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16)
                    .map_err(|_| ColorError::InvalidHex)?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16)
                    .map_err(|_| ColorError::InvalidHex)?;
                Ok(Self::new(r, g, b))
            }
            6 => {
                // #RRGGBB format
                let r = u8::from_str_radix(&hex[0..2], 16)
                    .map_err(|_| ColorError::InvalidHex)?;
                let g = u8::from_str_radix(&hex[2..4], 16)
                    .map_err(|_| ColorError::InvalidHex)?;
                let b = u8::from_str_radix(&hex[4..6], 16)
                    .map_err(|_| ColorError::InvalidHex)?;
                Ok(Self::new(r, g, b))
            }
            8 => {
                // #RRGGBBAA format
                let r = u8::from_str_radix(&hex[0..2], 16)
                    .map_err(|_| ColorError::InvalidHex)?;
                let g = u8::from_str_radix(&hex[2..4], 16)
                    .map_err(|_| ColorError::InvalidHex)?;
                let b = u8::from_str_radix(&hex[4..6], 16)
                    .map_err(|_| ColorError::InvalidHex)?;
                let a = u8::from_str_radix(&hex[6..8], 16)
                    .map_err(|_| ColorError::InvalidHex)?;
                Ok(Self::new_rgba(r, g, b, a))
            }
            _ => Err(ColorError::InvalidLength),
        }
    }
    
    /// Create a Color with modified alpha value
    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.a = (alpha.clamp(0.0, 1.0) * 255.0) as u8;
        self
    }
    
    /// Convert to tiny-skia Color32 for rendering
    pub fn to_tiny_skia_color(&self) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba8(self.r, self.g, self.b, self.a)
    }
    
    /// Convert to f32 RGBA values (0.0-1.0 range)
    pub fn to_rgba_f32(&self) -> (f32, f32, f32, f32) {
        (
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        )
    }
}

// Predefined color constants
impl Color {
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 255 };
    pub const RED: Color = Color { r: 255, g: 0, b: 0, a: 255 };
    pub const GREEN: Color = Color { r: 0, g: 128, b: 0, a: 255 };
    pub const BLUE: Color = Color { r: 0, g: 0, b: 255, a: 255 };
    pub const YELLOW: Color = Color { r: 255, g: 255, b: 0, a: 255 };
    pub const ORANGE: Color = Color { r: 255, g: 165, b: 0, a: 255 };
    pub const PURPLE: Color = Color { r: 128, g: 0, b: 128, a: 255 };
    pub const CYAN: Color = Color { r: 0, g: 255, b: 255, a: 255 };
    pub const MAGENTA: Color = Color { r: 255, g: 0, b: 255, a: 255 };
    pub const GRAY: Color = Color { r: 128, g: 128, b: 128, a: 255 };
    pub const LIGHT_GRAY: Color = Color { r: 211, g: 211, b: 211, a: 255 };
    pub const DARK_GRAY: Color = Color { r: 64, g: 64, b: 64, a: 255 };
    pub const TRANSPARENT: Color = Color { r: 0, g: 0, b: 0, a: 0 };
}

/// Default color palette for automatic color cycling
impl Color {
    /// Get the default color palette (matplotlib-inspired)
    pub fn default_palette() -> &'static [Color] {
        static PALETTE: &[Color] = &[
            Color::from_rgb_u32(0x1f77b4), // Blue
            Color::from_rgb_u32(0xff7f0e), // Orange
            Color::from_rgb_u32(0x2ca02c), // Green
            Color::from_rgb_u32(0xd62728), // Red
            Color::from_rgb_u32(0x9467bd), // Purple
            Color::from_rgb_u32(0x8c564b), // Brown
            Color::from_rgb_u32(0xe377c2), // Pink
            Color::from_rgb_u32(0x7f7f7f), // Gray
            Color::from_rgb_u32(0xbcbd22), // Olive
            Color::from_rgb_u32(0x17becf), // Cyan
        ];
        PALETTE
    }
    
    /// Get a color from the default palette by index (cycles if index >= palette length)
    pub fn from_palette(index: usize) -> Self {
        let palette = Self::default_palette();
        palette[index % palette.len()]
    }
    
    /// Create Color from 24-bit RGB integer
    const fn from_rgb_u32(rgb: u32) -> Self {
        Self {
            r: ((rgb >> 16) & 0xFF) as u8,
            g: ((rgb >> 8) & 0xFF) as u8,
            b: (rgb & 0xFF) as u8,
            a: 255,
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.a == 255 {
            write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            write!(f, "#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColorError {
    InvalidHex,
    InvalidLength,
}

impl fmt::Display for ColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColorError::InvalidHex => write!(f, "Invalid hexadecimal color value"),
            ColorError::InvalidLength => write!(f, "Invalid color string length (expected 3, 6, or 8 characters)"),
        }
    }
}

impl std::error::Error for ColorError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_creation() {
        let color = Color::new(255, 128, 64);
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 128);
        assert_eq!(color.b, 64);
        assert_eq!(color.a, 255);
    }

    #[test]
    fn test_color_with_alpha() {
        let color = Color::RED.with_alpha(0.5);
        assert_eq!(color.a, 127); // 0.5 * 255 ≈ 127
    }

    #[test]
    fn test_hex_parsing() {
        // #RGB format
        assert_eq!(Color::from_hex("#f0a").unwrap(), Color::new(255, 0, 170));
        
        // #RRGGBB format
        assert_eq!(Color::from_hex("#ff8040").unwrap(), Color::new(255, 128, 64));
        assert_eq!(Color::from_hex("ff8040").unwrap(), Color::new(255, 128, 64)); // Without #
        
        // #RRGGBBAA format
        assert_eq!(Color::from_hex("#ff804080").unwrap(), Color::new_rgba(255, 128, 64, 128));
        
        // Invalid formats
        assert!(Color::from_hex("#12345").is_err());
        assert!(Color::from_hex("#gghhii").is_err());
        assert!(Color::from_hex("").is_err());
    }

    #[test]
    fn test_predefined_colors() {
        assert_eq!(Color::RED, Color::new(255, 0, 0));
        assert_eq!(Color::BLUE, Color::new(0, 0, 255));
        assert_eq!(Color::WHITE, Color::new(255, 255, 255));
        assert_eq!(Color::TRANSPARENT.a, 0);
    }

    #[test]
    fn test_color_palette() {
        let palette = Color::default_palette();
        assert_eq!(palette.len(), 10);
        
        // Test cycling
        assert_eq!(Color::from_palette(0), palette[0]);
        assert_eq!(Color::from_palette(10), palette[0]); // Should cycle
        assert_eq!(Color::from_palette(15), palette[5]);
    }

    #[test]
    fn test_rgba_f32_conversion() {
        let color = Color::new_rgba(255, 128, 64, 128);
        let (r, g, b, a) = color.to_rgba_f32();
        assert!((r - 1.0).abs() < f32::EPSILON);
        assert!((g - 0.502).abs() < 0.01);
        assert!((b - 0.251).abs() < 0.01);
        assert!((a - 0.502).abs() < 0.01);
    }

    #[test]
    fn test_color_display() {
        assert_eq!(Color::RED.to_string(), "#ff0000");
        assert_eq!(Color::new_rgba(255, 128, 64, 128).to_string(), "#ff804080");
    }
}