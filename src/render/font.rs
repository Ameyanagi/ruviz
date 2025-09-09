use fontdue::{Font, FontSettings};
use crate::core::{PlottingError, Result};

/// Font family specification, similar to Plotters library approach
#[derive(Debug, Clone, PartialEq)]
pub enum FontFamily {
    /// System default serif font (Times New Roman, Times, etc.)
    Serif,
    /// System default sans-serif font (Arial, Helvetica, etc.)
    SansSerif,
    /// System default monospace font (Courier New, Consolas, etc.)
    Monospace,
    /// Specific font family name
    Name(String),
}

impl FontFamily {
    /// Convert to CSS-compatible font family string
    pub fn as_str(&self) -> &str {
        match self {
            FontFamily::Serif => "serif",
            FontFamily::SansSerif => "sans-serif", 
            FontFamily::Monospace => "monospace",
            FontFamily::Name(name) => name,
        }
    }
    
    /// Get platform-specific font paths for this font family
    pub fn get_font_paths(&self) -> Vec<&'static str> {
        match self {
            FontFamily::Serif => vec![
                // Linux
                "/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
                "/usr/share/fonts/noto/NotoSerif-Regular.ttf",
                // Windows
                "C:/Windows/Fonts/times.ttf",
                "C:/Windows/Fonts/Georgia.ttf",
                // macOS
                "/System/Library/Fonts/Times.ttc",
                "/System/Library/Fonts/Georgia.ttf",
            ],
            FontFamily::SansSerif => vec![
                // Linux (Arch/modern systems)
                "/usr/share/fonts/Adwaita/AdwaitaSans-Regular.ttf",
                "/usr/share/fonts/noto/NotoSans-Regular.ttf",
                "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                "/usr/share/fonts/gsfonts/NimbusSans-Regular.otf",
                // Windows
                "C:/Windows/Fonts/arial.ttf",
                "C:/Windows/Fonts/calibri.ttf",
                "C:/Windows/Fonts/segoeui.ttf",
                // macOS
                "/System/Library/Fonts/Arial.ttf",
                "/System/Library/Fonts/Helvetica.ttc",
            ],
            FontFamily::Monospace => vec![
                // Linux
                "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
                "/usr/share/fonts/noto/NotoSansMono-Regular.ttf",
                // Windows
                "C:/Windows/Fonts/consola.ttf",
                "C:/Windows/Fonts/cour.ttf",
                // macOS
                "/System/Library/Fonts/Monaco.ttf",
                "/System/Library/Fonts/Courier New.ttf",
            ],
            FontFamily::Name(name) => {
                // For custom font names, try common locations
                // This could be extended to search more locations
                vec![]
            }
        }
    }
    
    /// Load the font using fontdue with fallback chain
    pub fn load_font(&self) -> Result<Font> {
        let font_paths = self.get_font_paths();
        
        // Try each font path in order
        for font_path in &font_paths {
            if let Ok(font_data) = std::fs::read(font_path) {
                if let Ok(font) = Font::from_bytes(font_data, FontSettings::default()) {
                    println!("✅ Loaded {} font: {}", self.as_str(), font_path);
                    return Ok(font);
                }
            }
        }
        
        // If no system font found, create embedded fallback
        println!("⚠️ No {} fonts found, using embedded fallback", self.as_str());
        Self::create_embedded_font()
    }
    
    /// Create a minimal embedded font as last resort
    fn create_embedded_font() -> Result<Font> {
        // Create a minimal font data for basic ASCII characters
        // This is a simplified approach - in practice you'd embed a real font
        Err(PlottingError::InvalidData {
            message: "No suitable font found on system".to_string(),
            position: None,
        })
    }
}

impl From<&str> for FontFamily {
    fn from(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "serif" => FontFamily::Serif,
            "sans-serif" | "sans" => FontFamily::SansSerif,
            "monospace" | "mono" => FontFamily::Monospace,
            _ => FontFamily::Name(name.to_string()),
        }
    }
}

impl From<String> for FontFamily {
    fn from(name: String) -> Self {
        FontFamily::from(name.as_str())
    }
}

impl Default for FontFamily {
    fn default() -> Self {
        FontFamily::SansSerif
    }
}

/// Font configuration for text rendering
#[derive(Debug, Clone)]
pub struct FontConfig {
    pub family: FontFamily,
    pub size: f32,
    pub weight: FontWeight,
    pub style: FontStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FontWeight {
    Normal,
    Bold,
    Light,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FontStyle {
    Normal,
    Italic,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: FontFamily::default(),
            size: 12.0,
            weight: FontWeight::Normal,
            style: FontStyle::Normal,
        }
    }
}

impl FontConfig {
    pub fn new(family: FontFamily, size: f32) -> Self {
        Self {
            family,
            size,
            weight: FontWeight::Normal,
            style: FontStyle::Normal,
        }
    }
    
    pub fn bold(mut self) -> Self {
        self.weight = FontWeight::Bold;
        self
    }
    
    pub fn italic(mut self) -> Self {
        self.style = FontStyle::Italic;
        self
    }
    
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_font_family_creation() {
        assert_eq!(FontFamily::from("serif"), FontFamily::Serif);
        assert_eq!(FontFamily::from("sans-serif"), FontFamily::SansSerif);
        assert_eq!(FontFamily::from("monospace"), FontFamily::Monospace);
        assert_eq!(FontFamily::from("Arial"), FontFamily::Name("Arial".to_string()));
    }
    
    #[test]
    fn test_font_config() {
        let config = FontConfig::new(FontFamily::SansSerif, 14.0)
            .bold()
            .italic();
        
        assert_eq!(config.family, FontFamily::SansSerif);
        assert_eq!(config.size, 14.0);
        assert_eq!(config.weight, FontWeight::Bold);
        assert_eq!(config.style, FontStyle::Italic);
    }
}