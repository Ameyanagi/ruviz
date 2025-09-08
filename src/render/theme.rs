use crate::render::{Color, LineStyle};

/// Comprehensive theme system for consistent plot styling
/// 
/// Themes control the visual appearance of all plot elements including colors,
/// fonts, spacing, and other visual properties
#[derive(Debug, Clone)]
pub struct Theme {
    /// Background color of the plot area
    pub background: Color,
    /// Foreground color for text and axes
    pub foreground: Color,
    /// Grid line color
    pub grid_color: Color,
    /// Default line width for plot elements
    pub line_width: f32,
    /// Default line style
    pub line_style: LineStyle,
    /// Primary font family name
    pub font_family: String,
    /// Default font size for labels and text
    pub font_size: f32,
    /// Title font size
    pub title_font_size: f32,
    /// Legend font size
    pub legend_font_size: f32,
    /// Axis label font size
    pub axis_label_font_size: f32,
    /// Tick label font size
    pub tick_label_font_size: f32,
    /// Default color palette for automatic color cycling
    pub color_palette: Vec<Color>,
    /// Margin around the plot (as fraction of canvas size)
    pub margin: f32,
    /// Padding between plot elements
    pub padding: f32,
    /// Use colorblind-friendly palette
    pub colorblind_friendly: bool,
}

impl Theme {
    /// Create a new theme builder
    pub fn builder() -> ThemeBuilder {
        ThemeBuilder::default()
    }
    
    /// Create default light theme
    pub fn light() -> Self {
        Self {
            background: Color::WHITE,
            foreground: Color::BLACK,
            grid_color: Color::LIGHT_GRAY,
            line_width: 2.0,
            line_style: LineStyle::Solid,
            font_family: "Arial".to_string(),
            font_size: 12.0,
            title_font_size: 16.0,
            legend_font_size: 10.0,
            axis_label_font_size: 12.0,
            tick_label_font_size: 10.0,
            color_palette: Color::default_palette().to_vec(),
            margin: 0.1,
            padding: 8.0,
            colorblind_friendly: false,
        }
    }
    
    /// Create dark theme
    pub fn dark() -> Self {
        Self {
            background: Color::from_hex("#1e1e1e").unwrap(),
            foreground: Color::WHITE,
            grid_color: Color::DARK_GRAY,
            line_width: 2.0,
            line_style: LineStyle::Solid,
            font_family: "Arial".to_string(),
            font_size: 12.0,
            title_font_size: 16.0,
            legend_font_size: 10.0,
            axis_label_font_size: 12.0,
            tick_label_font_size: 10.0,
            color_palette: Self::dark_palette(),
            margin: 0.1,
            padding: 8.0,
            colorblind_friendly: false,
        }
    }
    
    /// Create publication-ready theme (high contrast, clean)
    pub fn publication() -> Self {
        Self {
            background: Color::WHITE,
            foreground: Color::BLACK,
            grid_color: Color::from_hex("#E0E0E0").unwrap(),
            line_width: 1.5,
            line_style: LineStyle::Solid,
            font_family: "Times New Roman".to_string(),
            font_size: 10.0,
            title_font_size: 12.0,
            legend_font_size: 9.0,
            axis_label_font_size: 10.0,
            tick_label_font_size: 8.0,
            color_palette: Self::publication_palette(),
            margin: 0.08,
            padding: 6.0,
            colorblind_friendly: false,
        }
    }
    
    /// Create minimal theme (minimal visual elements)
    pub fn minimal() -> Self {
        Self {
            background: Color::WHITE,
            foreground: Color::BLACK,
            grid_color: Color::TRANSPARENT,
            line_width: 1.5,
            line_style: LineStyle::Solid,
            font_family: "Helvetica".to_string(),
            font_size: 11.0,
            title_font_size: 14.0,
            legend_font_size: 10.0,
            axis_label_font_size: 11.0,
            tick_label_font_size: 9.0,
            color_palette: Self::minimal_palette(),
            margin: 0.05,
            padding: 4.0,
            colorblind_friendly: false,
        }
    }
    
    /// Create colorblind-friendly theme
    pub fn colorblind_friendly() -> Self {
        let mut theme = Self::light();
        theme.color_palette = Self::colorblind_palette();
        theme.colorblind_friendly = true;
        theme
    }
    
    /// Get a color from the theme's palette by index (cycles if needed)
    pub fn get_color(&self, index: usize) -> Color {
        if self.color_palette.is_empty() {
            Color::BLACK
        } else {
            self.color_palette[index % self.color_palette.len()]
        }
    }
    
    /// Get the effective grid color (returns transparent if grid should be hidden)
    pub fn effective_grid_color(&self) -> Color {
        self.grid_color
    }
    
    // Color palettes for different themes
    
    fn dark_palette() -> Vec<Color> {
        vec![
            Color::from_hex("#8dd3c7").unwrap(), // Light cyan
            Color::from_hex("#ffffb3").unwrap(), // Light yellow
            Color::from_hex("#bebada").unwrap(), // Light purple
            Color::from_hex("#fb8072").unwrap(), // Light red
            Color::from_hex("#80b1d3").unwrap(), // Light blue
            Color::from_hex("#fdb462").unwrap(), // Light orange
            Color::from_hex("#b3de69").unwrap(), // Light green
            Color::from_hex("#fccde5").unwrap(), // Light pink
        ]
    }
    
    fn publication_palette() -> Vec<Color> {
        vec![
            Color::BLACK,
            Color::DARK_GRAY,
            Color::from_hex("#404040").unwrap(),
            Color::from_hex("#606060").unwrap(),
            Color::from_hex("#808080").unwrap(),
            Color::from_hex("#A0A0A0").unwrap(),
        ]
    }
    
    fn minimal_palette() -> Vec<Color> {
        vec![
            Color::BLACK,
            Color::from_hex("#666666").unwrap(),
            Color::from_hex("#999999").unwrap(),
            Color::from_hex("#CCCCCC").unwrap(),
        ]
    }
    
    fn colorblind_palette() -> Vec<Color> {
        // Optimized for deuteranopia/protanopia (most common color blindness)
        vec![
            Color::from_hex("#1f77b4").unwrap(), // Blue
            Color::from_hex("#ff7f0e").unwrap(), // Orange
            Color::from_hex("#2ca02c").unwrap(), // Green
            Color::from_hex("#d62728").unwrap(), // Red
            Color::from_hex("#9467bd").unwrap(), // Purple
            Color::from_hex("#8c564b").unwrap(), // Brown
            Color::from_hex("#e377c2").unwrap(), // Pink
            Color::from_hex("#bcbd22").unwrap(), // Olive
            Color::from_hex("#17becf").unwrap(), // Cyan
        ]
    }
}

impl Default for Theme {
    /// Default theme is the light theme
    fn default() -> Self {
        Self::light()
    }
}

/// Builder pattern for creating custom themes
#[derive(Debug, Clone)]
pub struct ThemeBuilder {
    theme: Theme,
}

impl ThemeBuilder {
    /// Set background color
    pub fn background(mut self, color: Color) -> Self {
        self.theme.background = color;
        self
    }
    
    /// Set foreground color
    pub fn foreground(mut self, color: Color) -> Self {
        self.theme.foreground = color;
        self
    }
    
    /// Set grid color
    pub fn grid_color(mut self, color: Color) -> Self {
        self.theme.grid_color = color;
        self
    }
    
    /// Set default line width
    pub fn line_width(mut self, width: f32) -> Self {
        self.theme.line_width = width.max(0.1);
        self
    }
    
    /// Set default line style
    pub fn line_style(mut self, style: LineStyle) -> Self {
        self.theme.line_style = style;
        self
    }
    
    /// Set font family
    pub fn font<S: Into<String>>(mut self, font_family: S) -> Self {
        self.theme.font_family = font_family.into();
        self
    }
    
    /// Set default font size
    pub fn font_size(mut self, size: f32) -> Self {
        self.theme.font_size = size.max(6.0);
        self
    }
    
    /// Set title font size
    pub fn title_font_size(mut self, size: f32) -> Self {
        self.theme.title_font_size = size.max(6.0);
        self
    }
    
    /// Set legend font size
    pub fn legend_font_size(mut self, size: f32) -> Self {
        self.theme.legend_font_size = size.max(6.0);
        self
    }
    
    /// Set color palette
    pub fn palette<I>(mut self, colors: I) -> Self 
    where 
        I: IntoIterator<Item = Color>
    {
        self.theme.color_palette = colors.into_iter().collect();
        self
    }
    
    /// Enable colorblind-friendly palette
    pub fn colorblind_palette(mut self, enabled: bool) -> Self {
        self.theme.colorblind_friendly = enabled;
        if enabled {
            self.theme.color_palette = Theme::colorblind_palette();
        }
        self
    }
    
    /// Set margin
    pub fn margin(mut self, margin: f32) -> Self {
        self.theme.margin = margin.clamp(0.0, 0.5);
        self
    }
    
    /// Set padding
    pub fn padding(mut self, padding: f32) -> Self {
        self.theme.padding = padding.max(0.0);
        self
    }
    
    /// Build the theme
    pub fn build(self) -> Theme {
        self.theme
    }
}

impl Default for ThemeBuilder {
    fn default() -> Self {
        Self {
            theme: Theme::light(),
        }
    }
}

/// Predefined theme variants
pub enum ThemeVariant {
    Light,
    Dark,
    Publication,
    Minimal,
    ColorblindFriendly,
}

impl ThemeVariant {
    /// Convert theme variant to actual theme
    pub fn to_theme(&self) -> Theme {
        match self {
            ThemeVariant::Light => Theme::light(),
            ThemeVariant::Dark => Theme::dark(),
            ThemeVariant::Publication => Theme::publication(),
            ThemeVariant::Minimal => Theme::minimal(),
            ThemeVariant::ColorblindFriendly => Theme::colorblind_friendly(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_theme() {
        let theme = Theme::default();
        assert_eq!(theme.background, Color::WHITE);
        assert_eq!(theme.foreground, Color::BLACK);
        assert!(theme.font_size > 0.0);
        assert!(!theme.color_palette.is_empty());
    }

    #[test]
    fn test_theme_variants() {
        let light = Theme::light();
        let dark = Theme::dark();
        let publication = Theme::publication();
        let minimal = Theme::minimal();
        let colorblind = Theme::colorblind_friendly();
        
        assert_eq!(light.background, Color::WHITE);
        assert_eq!(dark.background, Color::from_hex("#1e1e1e").unwrap());
        assert_eq!(publication.font_family, "Times New Roman");
        assert_eq!(minimal.font_family, "Helvetica");
        assert!(colorblind.colorblind_friendly);
    }

    #[test]
    fn test_theme_builder() {
        let theme = Theme::builder()
            .background(Color::BLUE)
            .foreground(Color::WHITE)
            .font("Helvetica")
            .font_size(14.0)
            .line_width(3.0)
            .margin(0.05)
            .colorblind_palette(true)
            .build();
        
        assert_eq!(theme.background, Color::BLUE);
        assert_eq!(theme.foreground, Color::WHITE);
        assert_eq!(theme.font_family, "Helvetica");
        assert_eq!(theme.font_size, 14.0);
        assert_eq!(theme.line_width, 3.0);
        assert_eq!(theme.margin, 0.05);
        assert!(theme.colorblind_friendly);
    }

    #[test]
    fn test_color_cycling() {
        let theme = Theme::light();
        let color0 = theme.get_color(0);
        let color1 = theme.get_color(1);
        let color_cycle = theme.get_color(theme.color_palette.len());
        
        assert_eq!(color0, color_cycle); // Should cycle back to first color
        assert_ne!(color0, color1); // Different colors
    }

    #[test]
    fn test_builder_validation() {
        let theme = Theme::builder()
            .font_size(-5.0) // Invalid, should be clamped
            .line_width(-1.0) // Invalid, should be clamped
            .margin(-0.1) // Invalid, should be clamped
            .build();
        
        assert!(theme.font_size >= 6.0); // Minimum font size
        assert!(theme.line_width >= 0.1); // Minimum line width
        assert!(theme.margin >= 0.0); // Minimum margin
    }

    #[test]
    fn test_theme_variant_conversion() {
        let light = ThemeVariant::Light.to_theme();
        let dark = ThemeVariant::Dark.to_theme();
        
        assert_eq!(light.background, Color::WHITE);
        assert_ne!(dark.background, Color::WHITE);
    }

    #[test]
    fn test_empty_palette() {
        let mut theme = Theme::light();
        theme.color_palette.clear();
        
        // Should return black for empty palette
        assert_eq!(theme.get_color(0), Color::BLACK);
        assert_eq!(theme.get_color(5), Color::BLACK);
    }
}