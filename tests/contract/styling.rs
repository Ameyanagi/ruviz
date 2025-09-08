// Styling Contract Tests  
// Tests for themes, colors, line styles, text labels, and visual customization

use ruviz::prelude::*;

#[cfg(test)]
mod styling_contract_tests {
    use super::*;
    
    /// Contract: Custom colors must be applied correctly to plot elements
    #[test]
    fn test_color_customization_contract() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 4.0, 9.0];
        
        // Will fail until Color and color() method are implemented
        let plot = Plot::new()
            .line(&x, &y)
            .color(Color::RED);
            
        let image = plot.render();
        
        // Must apply specified color to line
        assert!(image.is_ok(), "Color customization failed: {:?}", image.err());
        
        // Test other colors
        let plot2 = Plot::new()
            .line(&x, &y)
            .color(Color::BLUE);
        assert!(plot2.render().is_ok());
        
        let plot3 = Plot::new()
            .line(&x, &y)
            .color(Color::from_hex("#FF5722"));
        assert!(plot3.render().is_ok());
    }
    
    /// Contract: Themes must change overall plot appearance consistently
    #[test]
    fn test_theme_application_contract() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 4.0, 9.0];
        
        // Will fail until Theme and Theme::builder() are implemented
        let dark_theme = Theme::builder()
            .background(Color::from_hex("#1e1e1e"))
            .foreground(Color::from_hex("#ffffff"))
            .grid_color(Color::from_hex("#333333"))
            .build();
            
        // Will fail until Plot::with_theme() is implemented
        let plot = Plot::with_theme(dark_theme)
            .line(&x, &y);
            
        let image = plot.render();
        
        // Must render with dark theme applied
        assert!(image.is_ok(), "Dark theme application failed: {:?}", image.err());
        
        // Test light theme
        let light_theme = Theme::builder()
            .background(Color::WHITE)
            .foreground(Color::BLACK)
            .grid_color(Color::GRAY)
            .build();
        
        let plot2 = Plot::with_theme(light_theme)
            .line(&x, &y);
        assert!(plot2.render().is_ok());
    }
    
    /// Contract: Line styles must be customizable (solid, dashed, dotted)
    #[test]
    fn test_line_style_contract() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 4.0, 9.0];
        
        // Will fail until LineStyle and style() method are implemented
        let plot1 = Plot::new()
            .line(&x, &y)
            .style(LineStyle::Dashed)
            .width(3.0);
            
        let image1 = plot1.render();
        assert!(image1.is_ok(), "Dashed line style failed: {:?}", image1.err());
        
        // Test different line styles
        let plot2 = Plot::new()
            .line(&x, &y)
            .style(LineStyle::Dotted)
            .width(2.0);
        assert!(plot2.render().is_ok());
        
        let plot3 = Plot::new()
            .line(&x, &y)
            .style(LineStyle::Solid)
            .width(1.5);
        assert!(plot3.render().is_ok());
    }
    
    /// Contract: Text labels and titles must be customizable
    #[test]
    fn test_text_customization_contract() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 4.0, 9.0];
        
        // Will fail until title(), xlabel(), ylabel() methods are implemented
        let plot = Plot::new()
            .line(&x, &y)
            .title("Custom Plot Title")
            .xlabel("Custom X Label")  
            .ylabel("Custom Y Label");
            
        let image = plot.render();
        
        // Must render with custom text labels visible
        assert!(image.is_ok(), "Text customization failed: {:?}", image.err());
        
        // Test with longer titles and special characters
        let plot2 = Plot::new()
            .line(&x, &y)
            .title("A Very Long Title That Should Wrap or Truncate Properly")
            .xlabel("X-axis with Special Characters: α, β, γ")
            .ylabel("Y-axis: f(x) = x²");
        assert!(plot2.render().is_ok());
    }
    
    /// Contract: Legend must be positionable and show correct labels
    #[test]
    fn test_legend_positioning_contract() {
        let x = vec![1.0, 2.0, 3.0, 4.0];
        let y1 = vec![1.0, 4.0, 9.0, 16.0];
        let y2 = vec![2.0, 5.0, 8.0, 11.0];
        
        // Will fail until label() and legend() methods are implemented
        let plot = Plot::new()
            .line(&x, &y1)
                .label("Series 1")
                .color(Color::RED)
            .line(&x, &y2)
                .label("Series 2")
                .color(Color::BLUE)
            .legend(Position::BottomLeft);
            
        let image = plot.render();
        
        // Must render legend in bottom-left position with correct labels
        assert!(image.is_ok(), "Legend positioning failed: {:?}", image.err());
        
        // Test different positions
        let positions = [
            Position::TopLeft,
            Position::TopRight,
            Position::BottomRight,
        ];
        
        for pos in positions.iter() {
            let plot_pos = Plot::new()
                .line(&x, &y1).label("Test")
                .legend(*pos);
            assert!(plot_pos.render().is_ok(), "Legend position {:?} failed", pos);
        }
    }
    
    /// Contract: Grid must be toggleable and customizable
    #[test]
    fn test_grid_customization_contract() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 4.0, 9.0];
        
        // Will fail until grid() method is implemented
        let plot_with_grid = Plot::new()
            .line(&x, &y)
            .grid(true);
            
        let image1 = plot_with_grid.render();
        assert!(image1.is_ok(), "Grid enabled failed: {:?}", image1.err());
        
        // Test grid disabled
        let plot_no_grid = Plot::new()
            .line(&x, &y)
            .grid(false);
        assert!(plot_no_grid.render().is_ok());
        
        // Test grid with custom style (if implemented)
        let plot_custom_grid = Plot::new()
            .line(&x, &y)
            .grid(true)
            .grid_color(Color::from_hex("#404040"))
            .grid_style(LineStyle::Dotted);
        assert!(plot_custom_grid.render().is_ok());
    }

    /// Contract: Font customization must work for all text elements
    #[test]
    fn test_font_customization_contract() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 4.0, 9.0];
        
        // Will fail until font methods are implemented
        let plot = Plot::new()
            .line(&x, &y)
            .title("Font Test")
            .font_size(16)
            .title_font_size(20)
            .axis_font_size(12);
            
        let image = plot.render();
        assert!(image.is_ok(), "Font customization failed: {:?}", image.err());
    }

    /// Contract: Alpha/transparency must work for overlapping elements
    #[test]
    fn test_alpha_transparency_contract() {
        let x = vec![1.0, 2.0, 3.0, 4.0];
        let y1 = vec![1.0, 4.0, 9.0, 16.0];
        let y2 = vec![2.0, 5.0, 8.0, 11.0];
        
        // Will fail until alpha() method is implemented
        let plot = Plot::new()
            .line(&x, &y1)
                .color(Color::RED)
                .alpha(0.7)
            .line(&x, &y2)
                .color(Color::BLUE)
                .alpha(0.5);
                
        let image = plot.render();
        assert!(image.is_ok(), "Alpha transparency failed: {:?}", image.err());
    }

    /// Contract: Plot dimensions must be customizable
    #[test]
    fn test_dimensions_contract() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 4.0, 9.0];
        
        // Will fail until dimensions() method is implemented
        let plot = Plot::new()
            .line(&x, &y)
            .dimensions(1200, 800);
            
        let image = plot.render();
        assert!(image.is_ok(), "Custom dimensions failed: {:?}", image.err());
        
        let img = image.unwrap();
        assert_eq!(img.width(), 1200, "Custom width not applied");
        assert_eq!(img.height(), 800, "Custom height not applied");
    }
}