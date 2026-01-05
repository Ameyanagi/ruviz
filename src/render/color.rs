use std::fmt;

/// Color representation for plot elements
///
/// Supports predefined colors and custom RGB/RGBA values with hex parsing.
///
/// # Default Palette
///
/// Use [`Color::default_palette()`] to get the 8-color default palette.
///
/// # Example
///
/// ```rust,no_run
/// use ruviz::prelude::*;
/// use ruviz::render::Color;
///
/// let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
/// let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
///
/// Plot::new()
///     .line(&x, &y)
///     .color(Color::new(255, 0, 128)) // Custom pink
///     .end_series()
///     .save("custom_color.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// ![Default color palette](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/colors.png)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8, // Alpha channel (0-255)
}

impl Color {
    /// Create a new Color from RGB values (alpha = 255)
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::render::Color;
    ///
    /// let red = Color::new(255, 0, 0);
    /// let purple = Color::new(128, 0, 128);
    /// ```
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Create a new Color from RGBA values
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::render::Color;
    ///
    /// // Semi-transparent blue
    /// let translucent_blue = Color::new_rgba(0, 0, 255, 128);
    /// ```
    pub fn new_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create a Color from hex string (supports #RGB, #RRGGBB, #RRGGBBAA)
    ///
    /// Returns `None` for invalid hex strings. This is a convenience wrapper
    /// around `from_hex()` that returns `Option` instead of `Result`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::render::Color;
    ///
    /// // Valid hex colors
    /// let coral = Color::hex("#FF7F50");
    /// assert!(coral.is_some());
    ///
    /// // Invalid hex returns None (no panic!)
    /// let invalid = Color::hex("not-a-color");
    /// assert!(invalid.is_none());
    /// ```
    pub fn hex(hex: &str) -> Option<Self> {
        Self::from_hex(hex).ok()
    }

    /// Create a Color from a named color string
    ///
    /// Supports common color names like "red", "blue", "green", etc.
    /// Case-insensitive. Returns `None` for unrecognized names.
    ///
    /// # Supported Colors
    ///
    /// - Primary: "red", "green", "blue"
    /// - Secondary: "cyan", "magenta", "yellow"
    /// - Neutral: "black", "white", "gray" (or "grey")
    /// - Other: "orange", "purple", "pink", "brown"
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::render::Color;
    ///
    /// // Valid named colors
    /// let blue = Color::named("blue");
    /// assert!(blue.is_some());
    ///
    /// // Case insensitive
    /// let red = Color::named("RED");
    /// assert!(red.is_some());
    ///
    /// // Unknown color returns None (no panic!)
    /// let unknown = Color::named("notacolor");
    /// assert!(unknown.is_none());
    /// ```
    pub fn named(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "red" => Some(Self::RED),
            "green" => Some(Self::GREEN),
            "blue" => Some(Self::BLUE),
            "yellow" => Some(Self::YELLOW),
            "orange" => Some(Self::ORANGE),
            "purple" => Some(Self::PURPLE),
            "cyan" => Some(Self::CYAN),
            "magenta" => Some(Self::MAGENTA),
            "black" => Some(Self::BLACK),
            "white" => Some(Self::WHITE),
            "gray" | "grey" => Some(Self::GRAY),
            "lightgray" | "lightgrey" | "light_gray" | "light_grey" => Some(Self::LIGHT_GRAY),
            "darkgray" | "darkgrey" | "dark_gray" | "dark_grey" => Some(Self::DARK_GRAY),
            "pink" => Some(Self::new(255, 192, 203)),
            "brown" => Some(Self::new(139, 69, 19)),
            "lime" => Some(Self::new(0, 255, 0)),
            "navy" => Some(Self::new(0, 0, 128)),
            "teal" => Some(Self::new(0, 128, 128)),
            "olive" => Some(Self::new(128, 128, 0)),
            "maroon" => Some(Self::new(128, 0, 0)),
            "aqua" => Some(Self::CYAN),
            "fuchsia" => Some(Self::MAGENTA),
            "silver" => Some(Self::new(192, 192, 192)),
            "coral" => Some(Self::new(255, 127, 80)),
            "salmon" => Some(Self::new(250, 128, 114)),
            "gold" => Some(Self::new(255, 215, 0)),
            "indigo" => Some(Self::new(75, 0, 130)),
            "violet" => Some(Self::new(238, 130, 238)),
            "crimson" => Some(Self::new(220, 20, 60)),
            _ => None,
        }
    }

    /// Suggest similar color names for typos
    ///
    /// Returns a suggestion if the input is close to a known color name.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::render::Color;
    ///
    /// let suggestion = Color::suggest_named("blu");
    /// assert_eq!(suggestion, Some("blue"));
    /// ```
    pub fn suggest_named(name: &str) -> Option<&'static str> {
        let name_lower = name.to_lowercase();
        let known_colors = [
            "red", "green", "blue", "yellow", "orange", "purple", "cyan", "magenta", "black",
            "white", "gray", "grey", "pink", "brown", "lime", "navy", "teal", "olive", "maroon",
            "aqua", "fuchsia", "silver", "coral", "salmon", "gold", "indigo", "violet", "crimson",
        ];

        // Check for common typos (simple edit distance heuristic)
        for color in &known_colors {
            // Check if input is a prefix (e.g., "blu" -> "blue")
            if color.starts_with(&name_lower) && color.len() <= name_lower.len() + 2 {
                return Some(color);
            }
            // Check if color is a prefix of input (e.g., "bluee" -> "blue")
            if name_lower.starts_with(color) && name_lower.len() <= color.len() + 2 {
                return Some(color);
            }
            // Check for single character difference
            if name_lower.len() == color.len() {
                let diff_count = name_lower
                    .chars()
                    .zip(color.chars())
                    .filter(|(a, b)| a != b)
                    .count();
                if diff_count <= 1 {
                    return Some(color);
                }
            }
        }
        None
    }

    /// Create a Color from hex string (supports #RGB, #RRGGBB, #RRGGBBAA)
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::render::Color;
    ///
    /// // 6-digit hex
    /// let coral = Color::from_hex("#FF7F50").unwrap();
    ///
    /// // 3-digit hex (shorthand)
    /// let cyan = Color::from_hex("#0FF").unwrap();
    ///
    /// // 8-digit hex with alpha
    /// let semi_red = Color::from_hex("#FF000080").unwrap();
    /// ```
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
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| ColorError::InvalidHex)?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| ColorError::InvalidHex)?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| ColorError::InvalidHex)?;
                Ok(Self::new(r, g, b))
            }
            8 => {
                // #RRGGBBAA format
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| ColorError::InvalidHex)?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| ColorError::InvalidHex)?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| ColorError::InvalidHex)?;
                let a = u8::from_str_radix(&hex[6..8], 16).map_err(|_| ColorError::InvalidHex)?;
                Ok(Self::new_rgba(r, g, b, a))
            }
            _ => Err(ColorError::InvalidLength),
        }
    }

    /// Create a Color with modified alpha value
    ///
    /// Alpha ranges from 0.0 (fully transparent) to 1.0 (fully opaque).
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::render::Color;
    ///
    /// // Make red 50% transparent
    /// let semi_red = Color::RED.with_alpha(0.5);
    /// assert_eq!(semi_red.a, 127);
    /// ```
    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.a = (alpha.clamp(0.0, 1.0) * 255.0) as u8;
        self
    }

    /// Convert to tiny-skia Color32 for rendering
    pub fn to_tiny_skia_color(self) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba8(self.r, self.g, self.b, self.a)
    }

    /// Convert to f32 RGBA values (0.0-1.0 range)
    pub fn to_rgba_f32(self) -> (f32, f32, f32, f32) {
        (
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        )
    }

    /// Create a darker version of this color
    ///
    /// The factor controls how much to darken (0.0 = unchanged, 1.0 = black).
    /// Commonly used for generating edge colors from fill colors.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::render::Color;
    ///
    /// let blue = Color::new(100, 150, 200);
    /// let dark_blue = blue.darken(0.3); // 30% darker
    /// assert!(dark_blue.r < blue.r);
    /// assert!(dark_blue.g < blue.g);
    /// assert!(dark_blue.b < blue.b);
    /// ```
    #[inline]
    pub fn darken(self, factor: f32) -> Self {
        let factor = factor.clamp(0.0, 1.0);
        let mult = 1.0 - factor;
        Self {
            r: ((self.r as f32) * mult) as u8,
            g: ((self.g as f32) * mult) as u8,
            b: ((self.b as f32) * mult) as u8,
            a: self.a,
        }
    }

    /// Create a lighter version of this color
    ///
    /// The factor controls how much to lighten (0.0 = unchanged, 1.0 = white).
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::render::Color;
    ///
    /// let blue = Color::new(100, 150, 200);
    /// let light_blue = blue.lighten(0.3); // 30% lighter
    /// assert!(light_blue.r > blue.r);
    /// assert!(light_blue.g > blue.g);
    /// assert!(light_blue.b > blue.b);
    /// ```
    #[inline]
    pub fn lighten(self, factor: f32) -> Self {
        let factor = factor.clamp(0.0, 1.0);
        Self {
            r: (self.r as f32 + (255.0 - self.r as f32) * factor) as u8,
            g: (self.g as f32 + (255.0 - self.g as f32) * factor) as u8,
            b: (self.b as f32 + (255.0 - self.b as f32) * factor) as u8,
            a: self.a,
        }
    }
}

// Predefined color constants
impl Color {
    pub const BLACK: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const WHITE: Color = Color {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const RED: Color = Color {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const GREEN: Color = Color {
        r: 0,
        g: 128,
        b: 0,
        a: 255,
    };
    pub const BLUE: Color = Color {
        r: 0,
        g: 0,
        b: 255,
        a: 255,
    };
    pub const YELLOW: Color = Color {
        r: 255,
        g: 255,
        b: 0,
        a: 255,
    };
    pub const ORANGE: Color = Color {
        r: 255,
        g: 165,
        b: 0,
        a: 255,
    };
    pub const PURPLE: Color = Color {
        r: 128,
        g: 0,
        b: 128,
        a: 255,
    };
    pub const CYAN: Color = Color {
        r: 0,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const MAGENTA: Color = Color {
        r: 255,
        g: 0,
        b: 255,
        a: 255,
    };
    pub const GRAY: Color = Color {
        r: 128,
        g: 128,
        b: 128,
        a: 255,
    };
    pub const LIGHT_GRAY: Color = Color {
        r: 211,
        g: 211,
        b: 211,
        a: 255,
    };
    pub const DARK_GRAY: Color = Color {
        r: 64,
        g: 64,
        b: 64,
        a: 255,
    };
    pub const TRANSPARENT: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
}

/// Default color palette for automatic color cycling
impl Color {
    /// Get the default color palette (matplotlib-inspired)
    ///
    /// Returns a 10-color palette suitable for distinguishing data series.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::render::Color;
    ///
    /// let palette = Color::default_palette();
    /// assert_eq!(palette.len(), 10);
    ///
    /// // Access individual colors
    /// let blue = palette[0];
    /// let orange = palette[1];
    /// ```
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
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::render::Color;
    ///
    /// let first = Color::from_palette(0);  // Blue
    /// let second = Color::from_palette(1); // Orange
    ///
    /// // Colors cycle after 10
    /// let eleventh = Color::from_palette(10);
    /// assert_eq!(first, eleventh);
    /// ```
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
            write!(
                f,
                "#{:02x}{:02x}{:02x}{:02x}",
                self.r, self.g, self.b, self.a
            )
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
        Self::new(
            "viridis".to_string(),
            vec![
                Color::from_rgb_u32(0x440154), // Dark purple
                Color::from_rgb_u32(0x482777), // Purple
                Color::from_rgb_u32(0x3f4a8a), // Dark blue
                Color::from_rgb_u32(0x31678e), // Blue
                Color::from_rgb_u32(0x26838f), // Teal
                Color::from_rgb_u32(0x1f9d8a), // Green-teal
                Color::from_rgb_u32(0x6cce5a), // Green
                Color::from_rgb_u32(0xb6de2b), // Yellow-green
                Color::from_rgb_u32(0xfee825), // Yellow
            ],
        )
    }

    /// Plasma colormap (high contrast, visually appealing)
    pub fn plasma() -> Self {
        Self::new(
            "plasma".to_string(),
            vec![
                Color::from_rgb_u32(0x0c0786), // Dark blue
                Color::from_rgb_u32(0x5b02a3), // Purple
                Color::from_rgb_u32(0x9a179b), // Magenta
                Color::from_rgb_u32(0xd53e4f), // Red
                Color::from_rgb_u32(0xf89441), // Orange
                Color::from_rgb_u32(0xfdbf6f), // Yellow-orange
                Color::from_rgb_u32(0xfeee65), // Yellow
            ],
        )
    }

    /// Inferno colormap (dark background friendly)
    pub fn inferno() -> Self {
        Self::new(
            "inferno".to_string(),
            vec![
                Color::from_rgb_u32(0x000003), // Black
                Color::from_rgb_u32(0x1f0c47), // Dark purple
                Color::from_rgb_u32(0x550f6d), // Purple
                Color::from_rgb_u32(0x88226a), // Magenta
                Color::from_rgb_u32(0xb83655), // Red
                Color::from_rgb_u32(0xe55c30), // Orange
                Color::from_rgb_u32(0xfb9b06), // Yellow-orange
                Color::from_rgb_u32(0xf7d746), // Yellow
                Color::from_rgb_u32(0xfcfdbf), // Light yellow
            ],
        )
    }

    /// Magma colormap (scientific visualization)
    pub fn magma() -> Self {
        Self::new(
            "magma".to_string(),
            vec![
                Color::from_rgb_u32(0x000003), // Black
                Color::from_rgb_u32(0x1c1044), // Dark purple
                Color::from_rgb_u32(0x4f127b), // Purple
                Color::from_rgb_u32(0x812581), // Magenta
                Color::from_rgb_u32(0xb5367a), // Red-magenta
                Color::from_rgb_u32(0xe55964), // Red
                Color::from_rgb_u32(0xfb8761), // Orange
                Color::from_rgb_u32(0xfec287), // Light orange
                Color::from_rgb_u32(0xfbfcbf), // Light yellow
            ],
        )
    }

    /// Hot colormap (classic heat map)
    pub fn hot() -> Self {
        Self::new(
            "hot".to_string(),
            vec![
                Color::new(0, 0, 0),       // Black
                Color::new(128, 0, 0),     // Dark red
                Color::new(255, 0, 0),     // Red
                Color::new(255, 128, 0),   // Orange
                Color::new(255, 255, 0),   // Yellow
                Color::new(255, 255, 128), // Light yellow
                Color::new(255, 255, 255), // White
            ],
        )
    }

    /// Cool colormap (cyan to magenta)
    pub fn cool() -> Self {
        Self::new(
            "cool".to_string(),
            vec![
                Color::new(0, 255, 255),   // Cyan
                Color::new(128, 128, 255), // Light blue
                Color::new(255, 0, 255),   // Magenta
            ],
        )
    }

    /// Grayscale colormap
    pub fn gray() -> Self {
        Self::new(
            "gray".to_string(),
            vec![
                Color::new(0, 0, 0),       // Black
                Color::new(64, 64, 64),    // Dark gray
                Color::new(128, 128, 128), // Gray
                Color::new(192, 192, 192), // Light gray
                Color::new(255, 255, 255), // White
            ],
        )
    }

    /// Jet colormap (classic but not recommended for scientific use)
    pub fn jet() -> Self {
        Self::new(
            "jet".to_string(),
            vec![
                Color::new(0, 0, 128),     // Dark blue
                Color::new(0, 0, 255),     // Blue
                Color::new(0, 128, 255),   // Light blue
                Color::new(0, 255, 255),   // Cyan
                Color::new(128, 255, 128), // Light green
                Color::new(255, 255, 0),   // Yellow
                Color::new(255, 128, 0),   // Orange
                Color::new(255, 0, 0),     // Red
                Color::new(128, 0, 0),     // Dark red
            ],
        )
    }

    /// Coolwarm diverging colormap (blue-white-red, centered at zero)
    pub fn coolwarm() -> Self {
        Self::new(
            "coolwarm".to_string(),
            vec![
                Color::from_rgb_u32(0x3b4cc0), // Blue
                Color::from_rgb_u32(0x6788ee), // Light blue
                Color::from_rgb_u32(0x9abbff), // Lighter blue
                Color::from_rgb_u32(0xc9d7f0), // Very light blue
                Color::from_rgb_u32(0xf7f7f7), // White (center)
                Color::from_rgb_u32(0xf6cfa5), // Very light red
                Color::from_rgb_u32(0xf08a6d), // Light red
                Color::from_rgb_u32(0xd8412d), // Red
                Color::from_rgb_u32(0xb40426), // Dark red
            ],
        )
    }

    /// RdBu diverging colormap (red-white-blue)
    pub fn rdbu() -> Self {
        Self::new(
            "rdbu".to_string(),
            vec![
                Color::from_rgb_u32(0x67001f), // Dark red
                Color::from_rgb_u32(0xb2182b), // Red
                Color::from_rgb_u32(0xd6604d), // Light red
                Color::from_rgb_u32(0xf4a582), // Salmon
                Color::from_rgb_u32(0xfddbc7), // Light salmon
                Color::from_rgb_u32(0xf7f7f7), // White (center)
                Color::from_rgb_u32(0xd1e5f0), // Light blue
                Color::from_rgb_u32(0x92c5de), // Light blue
                Color::from_rgb_u32(0x4393c3), // Blue
                Color::from_rgb_u32(0x2166ac), // Dark blue
                Color::from_rgb_u32(0x053061), // Very dark blue
            ],
        )
    }

    /// Create a colormap from a slice of colors
    pub fn from_colors(colors: &[Color]) -> Self {
        Self::new("custom".to_string(), colors.to_vec())
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
            "coolwarm" => Some(Self::coolwarm()),
            "rdbu" => Some(Self::rdbu()),
            _ => None,
        }
    }

    /// List all available colormap names
    pub fn available_names() -> Vec<&'static str> {
        vec![
            "viridis", "plasma", "inferno", "magma", "hot", "cool", "gray", "jet", "coolwarm",
            "rdbu",
        ]
    }
}

impl fmt::Display for ColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColorError::InvalidHex => write!(f, "Invalid hexadecimal color value"),
            ColorError::InvalidLength => write!(
                f,
                "Invalid color string length (expected 3, 6, or 8 characters)"
            ),
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
        assert_eq!(
            Color::from_hex("#ff8040").unwrap(),
            Color::new(255, 128, 64)
        );
        assert_eq!(Color::from_hex("ff8040").unwrap(), Color::new(255, 128, 64)); // Without #

        // #RRGGBBAA format
        assert_eq!(
            Color::from_hex("#ff804080").unwrap(),
            Color::new_rgba(255, 128, 64, 128)
        );

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
        let cmap = ColorMap::new(
            "test".to_string(),
            vec![
                Color::new(0, 0, 0),       // Black
                Color::new(255, 255, 255), // White
            ],
        );

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
        assert!(!viridis.is_empty());

        let plasma = ColorMap::plasma();
        assert_eq!(plasma.name(), "plasma");
        assert!(!plasma.is_empty());
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

    #[test]
    fn test_named_colors() {
        // Basic colors
        assert_eq!(Color::named("red"), Some(Color::RED));
        assert_eq!(Color::named("blue"), Some(Color::BLUE));
        assert_eq!(Color::named("green"), Some(Color::GREEN));

        // Case insensitive
        assert_eq!(Color::named("RED"), Some(Color::RED));
        assert_eq!(Color::named("Blue"), Some(Color::BLUE));

        // Gray variants
        assert_eq!(Color::named("gray"), Some(Color::GRAY));
        assert_eq!(Color::named("grey"), Some(Color::GRAY));

        // Extended colors
        assert!(Color::named("coral").is_some());
        assert!(Color::named("salmon").is_some());
        assert!(Color::named("gold").is_some());

        // Unknown color
        assert_eq!(Color::named("notacolor"), None);
    }

    #[test]
    fn test_hex_convenience() {
        // Valid hex
        assert!(Color::hex("#FF0000").is_some());
        assert_eq!(Color::hex("#FF0000"), Some(Color::RED));

        // Invalid hex returns None instead of panicking
        assert_eq!(Color::hex("invalid"), None);
        assert_eq!(Color::hex("#GGGGGG"), None);
    }

    #[test]
    fn test_color_suggestions() {
        // Common typos - prefix matching
        assert_eq!(Color::suggest_named("blu"), Some("blue"));
        assert_eq!(Color::suggest_named("re"), Some("red"));
        assert_eq!(Color::suggest_named("gree"), Some("green"));

        // Extra character
        assert_eq!(Color::suggest_named("bluee"), Some("blue"));

        // Single character difference
        assert_eq!(Color::suggest_named("blua"), Some("blue"));
        assert_eq!(Color::suggest_named("rad"), Some("red"));

        // No suggestion for very different strings
        assert_eq!(Color::suggest_named("xyz"), None);
    }

    #[test]
    fn test_color_darken() {
        let color = Color::new(100, 150, 200);

        // 0% darkening = unchanged
        let same = color.darken(0.0);
        assert_eq!(same.r, 100);
        assert_eq!(same.g, 150);
        assert_eq!(same.b, 200);

        // 30% darkening
        let darker = color.darken(0.3);
        assert_eq!(darker.r, 70); // 100 * 0.7 = 70
        assert_eq!(darker.g, 105); // 150 * 0.7 = 105
        assert_eq!(darker.b, 140); // 200 * 0.7 = 140

        // 100% darkening = black
        let black = color.darken(1.0);
        assert_eq!(black.r, 0);
        assert_eq!(black.g, 0);
        assert_eq!(black.b, 0);

        // Alpha preserved
        let with_alpha = Color::new_rgba(100, 150, 200, 128).darken(0.5);
        assert_eq!(with_alpha.a, 128);
    }

    #[test]
    fn test_color_lighten() {
        let color = Color::new(100, 150, 200);

        // 0% lightening = unchanged
        let same = color.lighten(0.0);
        assert_eq!(same.r, 100);
        assert_eq!(same.g, 150);
        assert_eq!(same.b, 200);

        // 50% lightening
        let lighter = color.lighten(0.5);
        assert_eq!(lighter.r, 177); // 100 + (255-100)*0.5 = 177.5
        assert_eq!(lighter.g, 202); // 150 + (255-150)*0.5 = 202.5
        assert_eq!(lighter.b, 227); // 200 + (255-200)*0.5 = 227.5

        // 100% lightening = white
        let white = color.lighten(1.0);
        assert_eq!(white.r, 255);
        assert_eq!(white.g, 255);
        assert_eq!(white.b, 255);

        // Alpha preserved
        let with_alpha = Color::new_rgba(100, 150, 200, 128).lighten(0.5);
        assert_eq!(with_alpha.a, 128);
    }
}
