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

/// ColorMap for mapping scalar values to colors (used by DataShader)
#[derive(Debug, Clone)]
pub struct ColorMap {
    colors: Vec<Color>,
    name: String,
}

impl ColorMap {
    /// Create a custom colormap from a vector of colors
    pub fn new(name: String, colors: Vec<Color>) -> Self {
        Self { name, colors }
    }
    
    /// Sample the colormap at position t (0.0 to 1.0)
    pub fn sample(&self, t: f64) -> Color {
        let t = t.clamp(0.0, 1.0);
        
        if self.colors.is_empty() {
            return Color::BLACK;
        }
        
        if self.colors.len() == 1 {
            return self.colors[0];
        }
        
        if t == 0.0 {
            return self.colors[0];
        }
        
        if t == 1.0 {
            return *self.colors.last().unwrap();
        }
        
        // Interpolate between colors
        let scaled = t * (self.colors.len() - 1) as f64;
        let index = scaled.floor() as usize;
        let frac = scaled - index as f64;
        
        if index >= self.colors.len() - 1 {
            return *self.colors.last().unwrap();
        }
        
        let c1 = self.colors[index];
        let c2 = self.colors[index + 1];
        
        // Linear interpolation
        Color::new_rgba(
            (c1.r as f64 + (c2.r as f64 - c1.r as f64) * frac) as u8,
            (c1.g as f64 + (c2.g as f64 - c1.g as f64) * frac) as u8,
            (c1.b as f64 + (c2.b as f64 - c1.b as f64) * frac) as u8,
            (c1.a as f64 + (c2.a as f64 - c1.a as f64) * frac) as u8,
        )
    }
    
    /// Get colormap name
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Get number of colors in the colormap
    pub fn len(&self) -> usize {
        self.colors.len()
    }
    
    /// Check if colormap is empty
    pub fn is_empty(&self) -> bool {
        self.colors.is_empty()
    }
}

// Predefined colormaps
impl ColorMap {
    /// Viridis colormap (perceptually uniform, colorblind-friendly)
    pub fn viridis() -> Self {
        Self::new("viridis".to_string(), vec![
            Color::from_rgb_u32(0x440154), // Dark purple
            Color::from_rgb_u32(0x482777), // Purple
            Color::from_rgb_u32(0x3f4a8a), // Dark blue
            Color::from_rgb_u32(0x31678e), // Blue
            Color::from_rgb_u32(0x26838f), // Teal
            Color::from_rgb_u32(0x1f9d8a), // Green-teal
            Color::from_rgb_u32(0x6cce5a), // Green
            Color::from_rgb_u32(0xb6de2b), // Yellow-green
            Color::from_rgb_u32(0xfee825), // Yellow
        ])
    }
    
    /// Plasma colormap (high contrast, visually appealing)
    pub fn plasma() -> Self {
        Self::new("plasma".to_string(), vec![
            Color::from_rgb_u32(0x0c0786), // Dark blue
            Color::from_rgb_u32(0x5b02a3), // Purple
            Color::from_rgb_u32(0x9a179b), // Magenta
            Color::from_rgb_u32(0xd53e4f), // Red
            Color::from_rgb_u32(0xf89441), // Orange
            Color::from_rgb_u32(0xfdbf6f), // Yellow-orange
            Color::from_rgb_u32(0xfeee65), // Yellow
        ])
    }
    
    /// Inferno colormap (dark background friendly)
    pub fn inferno() -> Self {
        Self::new("inferno".to_string(), vec![
            Color::from_rgb_u32(0x000003), // Black
            Color::from_rgb_u32(0x1f0c47), // Dark purple
            Color::from_rgb_u32(0x550f6d), // Purple
            Color::from_rgb_u32(0x88226a), // Magenta
            Color::from_rgb_u32(0xb83655), // Red
            Color::from_rgb_u32(0xe55c30), // Orange
            Color::from_rgb_u32(0xfb9b06), // Yellow-orange
            Color::from_rgb_u32(0xf7d746), // Yellow
            Color::from_rgb_u32(0xfcfdbf), // Light yellow
        ])
    }
    
    /// Magma colormap (scientific visualization)
    pub fn magma() -> Self {
        Self::new("magma".to_string(), vec![
            Color::from_rgb_u32(0x000003), // Black
            Color::from_rgb_u32(0x1c1044), // Dark purple
            Color::from_rgb_u32(0x4f127b), // Purple
            Color::from_rgb_u32(0x812581), // Magenta
            Color::from_rgb_u32(0xb5367a), // Red-magenta
            Color::from_rgb_u32(0xe55964), // Red
            Color::from_rgb_u32(0xfb8761), // Orange
            Color::from_rgb_u32(0xfec287), // Light orange
            Color::from_rgb_u32(0xfbfcbf), // Light yellow
        ])
    }
    
    /// Hot colormap (classic heat map)
    pub fn hot() -> Self {
        Self::new("hot".to_string(), vec![
            Color::new(0, 0, 0),       // Black
            Color::new(128, 0, 0),     // Dark red
            Color::new(255, 0, 0),     // Red
            Color::new(255, 128, 0),   // Orange
            Color::new(255, 255, 0),   // Yellow
            Color::new(255, 255, 128), // Light yellow
            Color::new(255, 255, 255), // White
        ])
    }
    
    /// Cool colormap (cyan to magenta)
    pub fn cool() -> Self {
        Self::new("cool".to_string(), vec![
            Color::new(0, 255, 255),   // Cyan
            Color::new(128, 128, 255), // Light blue
            Color::new(255, 0, 255),   // Magenta
        ])
    }
    
    /// Grayscale colormap
    pub fn gray() -> Self {
        Self::new("gray".to_string(), vec![
            Color::new(0, 0, 0),       // Black
            Color::new(64, 64, 64),    // Dark gray
            Color::new(128, 128, 128), // Gray
            Color::new(192, 192, 192), // Light gray
            Color::new(255, 255, 255), // White
        ])
    }
    
    /// Jet colormap (classic but not recommended for scientific use)
    pub fn jet() -> Self {
        Self::new("jet".to_string(), vec![
            Color::new(0, 0, 128),     // Dark blue
            Color::new(0, 0, 255),     // Blue
            Color::new(0, 128, 255),   // Light blue
            Color::new(0, 255, 255),   // Cyan
            Color::new(128, 255, 128), // Light green
            Color::new(255, 255, 0),   // Yellow
            Color::new(255, 128, 0),   // Orange
            Color::new(255, 0, 0),     // Red
            Color::new(128, 0, 0),     // Dark red
        ])
    }
    
    /// Get colormap by name
    pub fn by_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "viridis" => Some(Self::viridis()),
            "plasma" => Some(Self::plasma()),
            "inferno" => Some(Self::inferno()),
            "magma" => Some(Self::magma()),
            "hot" => Some(Self::hot()),
            "cool" => Some(Self::cool()),
            "gray" | "grey" => Some(Self::gray()),
            "jet" => Some(Self::jet()),
            _ => None,
        }
    }
    
    /// List all available colormap names
    pub fn available_names() -> Vec<&'static str> {
        vec!["viridis", "plasma", "inferno", "magma", "hot", "cool", "gray", "jet"]
    }
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

    #[test]
    fn test_colormap_creation() {
        let colors = vec![Color::RED, Color::GREEN, Color::BLUE];
        let cmap = ColorMap::new("test".to_string(), colors);
        assert_eq!(cmap.name(), "test");
        assert_eq!(cmap.len(), 3);
    }
    
    #[test]
    fn test_colormap_sampling() {
        let cmap = ColorMap::new("test".to_string(), vec![
            Color::new(0, 0, 0),     // Black
            Color::new(255, 255, 255), // White
        ]);
        
        // Test endpoints
        assert_eq!(cmap.sample(0.0), Color::new(0, 0, 0));
        assert_eq!(cmap.sample(1.0), Color::new(255, 255, 255));
        
        // Test middle (should be gray)
        let mid = cmap.sample(0.5);
        assert!(mid.r > 100 && mid.r < 200); // Should be grayish
    }
    
    #[test]
    fn test_predefined_colormaps() {
        let viridis = ColorMap::viridis();
        assert_eq!(viridis.name(), "viridis");
        assert!(viridis.len() > 0);
        
        let plasma = ColorMap::plasma();
        assert_eq!(plasma.name(), "plasma");
        assert!(plasma.len() > 0);
    }
    
    #[test]
    fn test_colormap_by_name() {
        assert!(ColorMap::by_name("viridis").is_some());
        assert!(ColorMap::by_name("plasma").is_some());
        assert!(ColorMap::by_name("nonexistent").is_none());
        
        // Case insensitive
        assert!(ColorMap::by_name("VIRIDIS").is_some());
        assert!(ColorMap::by_name("Plasma").is_some());
    }
    
    #[test]
    fn test_colormap_edge_cases() {
        // Empty colormap
        let empty = ColorMap::new("empty".to_string(), vec![]);
        assert_eq!(empty.sample(0.5), Color::BLACK);
        
        // Single color
        let single = ColorMap::new("single".to_string(), vec![Color::RED]);
        assert_eq!(single.sample(0.0), Color::RED);
        assert_eq!(single.sample(0.5), Color::RED);
        assert_eq!(single.sample(1.0), Color::RED);
    }
    
    #[test]
    fn test_colormap_clamping() {
        let cmap = ColorMap::viridis();
        
        // Values outside [0,1] should be clamped
        let below = cmap.sample(-0.5);
        let above = cmap.sample(1.5);
        let start = cmap.sample(0.0);
        let end = cmap.sample(1.0);
        
        assert_eq!(below, start);
        assert_eq!(above, end);
    }
}