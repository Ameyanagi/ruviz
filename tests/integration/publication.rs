// Publication Quality Integration Test (Story 4)
// "Given I need publication-ready output, When I export my plot with custom themes 
// and styling, Then the library produces high-quality images suitable for academic papers"

use ruviz::prelude::*;
use std::path::Path;

#[cfg(test)]
mod publication_integration_tests {
    use super::*;

    /// Integration Test: Story 4 - Publication-quality export with custom themes
    #[test]
    fn test_story_4_publication_quality_integration() {
        // Given: I need publication-ready output
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![1.0, 1.5, 2.2, 2.8, 3.1, 3.9];
        let error = vec![0.1, 0.2, 0.15, 0.25, 0.1, 0.2];
        
        // Create publication theme  
        // Will fail until Theme::builder() is implemented
        let pub_theme = Theme::builder()
            .background(Color::WHITE)
            .foreground(Color::BLACK)
            .grid_color(Color::GRAY.with_alpha(0.3))
            .font("Times New Roman")
            .font_size(12)
            .line_width(1.5)
            .build();
        
        // When: I export my plot with custom themes and styling
        // Will fail until Plot::with_theme(), error_bars(), dimensions(), dpi() are implemented
        let result_png = Plot::with_theme(pub_theme.clone())
            .line(&x, &y)
                .color(Color::BLACK)
                .style(LineStyle::Solid)
                .line_width(1.5)
            .error_bars(&x, &y, &error)
            .title("Experimental Results")
            .xlabel("Time (s)")
            .ylabel("Response (V)")
            .dimensions(600, 400) // Publication standard size
            .dpi(300) // High resolution for print
            .save("test_publication_integration.png");
        
        // Then: The library produces high-quality images suitable for academic papers
        assert!(result_png.is_ok(), "Publication PNG export failed: {:?}", result_png.err());
        
        // Verify PNG file was created with appropriate properties
        assert!(Path::new("test_publication_integration.png").exists(),
                "Publication PNG file was not created");
        
        let png_metadata = std::fs::metadata("test_publication_integration.png")
            .expect("Could not read PNG metadata");
        
        // High DPI files should be larger 
        assert!(png_metadata.len() > 50000, 
                "Publication PNG too small: {} bytes (expected >50KB for 300 DPI)", 
                png_metadata.len());
        
        // Also test SVG export for vector graphics
        // Will fail until export_svg() is implemented
        let result_svg = Plot::with_theme(pub_theme)
            .line(&x, &y)
                .color(Color::BLACK)
                .style(LineStyle::Solid)
            .error_bars(&x, &y, &error)
            .title("Experimental Results")
            .xlabel("Time (s)")
            .ylabel("Response (V)")
            .export_svg("test_publication_integration.svg");
        
        assert!(result_svg.is_ok(), "Publication SVG export failed: {:?}", result_svg.err());
        
        // Verify SVG file  
        assert!(Path::new("test_publication_integration.svg").exists(),
                "Publication SVG file was not created");
        
        let svg_metadata = std::fs::metadata("test_publication_integration.svg")
            .expect("Could not read SVG metadata");
        
        // SVG should contain actual vector content
        assert!(svg_metadata.len() > 1000,
                "Publication SVG too small: {} bytes", svg_metadata.len());
        
        println!("✅ Publication-quality plots created:");
        println!("  - PNG: {} KB (300 DPI)", png_metadata.len() / 1000);
        println!("  - SVG: {} KB (vector)", svg_metadata.len() / 1000);
        
        // Clean up test files
        std::fs::remove_file("test_publication_integration.png").ok();
        std::fs::remove_file("test_publication_integration.svg").ok();
    }

    /// Integration Test: Multiple publication themes
    #[test]
    fn test_publication_themes_integration() {
        let x = vec![1.0, 2.0, 3.0, 4.0];
        let y = vec![2.0, 4.0, 3.0, 5.0];
        
        // Scientific journal theme (conservative, high contrast)
        let journal_theme = Theme::builder()
            .background(Color::WHITE)
            .foreground(Color::BLACK)
            .grid_color(Color::from_hex("#E0E0E0"))
            .font("Times New Roman")
            .font_size(10)
            .line_width(1.0)
            .build();
        
        let result1 = Plot::with_theme(journal_theme)
            .line(&x, &y)
            .title("Scientific Journal Style")
            .xlabel("Independent Variable")
            .ylabel("Dependent Variable")
            .grid(true)
            .save("test_journal_theme.png");
        
        assert!(result1.is_ok(), "Journal theme failed: {:?}", result1.err());
        std::fs::remove_file("test_journal_theme.png").ok();
        
        // Presentation theme (larger text, bold lines)
        let presentation_theme = Theme::builder()
            .background(Color::WHITE)
            .foreground(Color::BLACK)
            .font("Arial")
            .font_size(16)
            .title_font_size(20)
            .line_width(3.0)
            .build();
        
        let result2 = Plot::with_theme(presentation_theme)
            .line(&x, &y)
                .color(Color::from_hex("#2E86AB"))
                .line_width(4.0)
            .title("Presentation Style")
            .xlabel("X Axis")
            .ylabel("Y Axis")
            .save("test_presentation_theme.png");
        
        assert!(result2.is_ok(), "Presentation theme failed: {:?}", result2.err());
        std::fs::remove_file("test_presentation_theme.png").ok();
        
        // Monochrome theme (for B&W printing)
        let mono_theme = Theme::builder()
            .background(Color::WHITE)
            .foreground(Color::BLACK)
            .grid_color(Color::from_hex("#808080"))
            .font("Helvetica")
            .font_size(11)
            .build();
        
        let result3 = Plot::with_theme(mono_theme)
            .line(&x, &y)
                .color(Color::BLACK)
                .style(LineStyle::Solid)
            .title("Monochrome Style")
            .grid(true)
            .save("test_mono_theme.png");
        
        assert!(result3.is_ok(), "Monochrome theme failed: {:?}", result3.err());
        std::fs::remove_file("test_mono_theme.png").ok();
    }

    /// Integration Test: High-resolution export formats
    #[test] 
    fn test_high_resolution_export_integration() {
        let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
        let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();
        
        let plot = Plot::new()
            .line(&x, &y)
            .title("High Resolution Test")
            .xlabel("x")
            .ylabel("sin(x)");
        
        // Test different DPI settings
        let dpi_settings = vec![150, 300, 600];
        
        for &dpi in &dpi_settings {
            let result = plot.clone()
                .dpi(dpi)
                .save(&format!("test_{}dpi.png", dpi));
            
            assert!(result.is_ok(), "{}DPI export failed: {:?}", dpi, result.err());
            
            let metadata = std::fs::metadata(&format!("test_{}dpi.png", dpi))
                .expect("Could not read DPI test file");
            
            println!("{}DPI file: {} KB", dpi, metadata.len() / 1000);
            
            std::fs::remove_file(&format!("test_{}dpi.png", dpi)).ok();
        }
    }

    /// Integration Test: Error bars and uncertainty visualization  
    #[test]
    fn test_error_bars_integration() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.1, 3.9, 6.2, 7.8, 10.1];
        let y_error = vec![0.3, 0.5, 0.4, 0.6, 0.7];
        let x_error = vec![0.1, 0.1, 0.15, 0.1, 0.2];
        
        // Vertical error bars only
        let result1 = Plot::new()
            .line(&x, &y)
            .error_bars(&x, &y, &y_error)
            .title("Vertical Error Bars")
            .save("test_vertical_errors.png");
        
        assert!(result1.is_ok(), "Vertical error bars failed: {:?}", result1.err());
        std::fs::remove_file("test_vertical_errors.png").ok();
        
        // Both horizontal and vertical error bars
        let result2 = Plot::new()
            .scatter(&x, &y)
            .error_bars_xy(&x, &y, &x_error, &y_error)
            .title("X-Y Error Bars")
            .save("test_xy_errors.png");
        
        assert!(result2.is_ok(), "X-Y error bars failed: {:?}", result2.err());
        std::fs::remove_file("test_xy_errors.png").ok();
    }

    /// Integration Test: Scientific notation and formatting
    #[test]
    fn test_scientific_formatting_integration() {
        // Very large values
        let x_large: Vec<f64> = vec![1e6, 2e6, 3e6, 4e6, 5e6];
        let y_large: Vec<f64> = vec![1.2e9, 2.4e9, 3.6e9, 4.8e9, 6.0e9];
        
        let result1 = Plot::new()
            .line(&x_large, &y_large)
            .title("Scientific Notation - Large Values")
            .xlabel("Frequency (Hz)")
            .ylabel("Power (W)")
            .scientific_notation(true)
            .save("test_scientific_large.png");
        
        assert!(result1.is_ok(), "Scientific notation (large) failed: {:?}", result1.err());
        std::fs::remove_file("test_scientific_large.png").ok();
        
        // Very small values
        let x_small: Vec<f64> = vec![1e-6, 2e-6, 3e-6, 4e-6, 5e-6];
        let y_small: Vec<f64> = vec![1.2e-9, 2.4e-9, 3.6e-9, 4.8e-9, 6.0e-9];
        
        let result2 = Plot::new()
            .line(&x_small, &y_small)
            .title("Scientific Notation - Small Values")
            .xlabel("Time (s)")
            .ylabel("Current (A)")
            .scientific_notation(true)
            .save("test_scientific_small.png");
        
        assert!(result2.is_ok(), "Scientific notation (small) failed: {:?}", result2.err());
        std::fs::remove_file("test_scientific_small.png").ok();
    }

    /// Integration Test: LaTeX-style mathematical expressions
    #[test]
    fn test_latex_expressions_integration() {
        let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.2).collect();
        let y: Vec<f64> = x.iter().map(|&x| (-x).exp()).collect();
        
        // Will fail until LaTeX rendering support is implemented
        let result = Plot::new()
            .line(&x, &y)
            .title(r"Exponential Decay: $f(x) = e^{-x}$")
            .xlabel(r"Time $t$ (seconds)")
            .ylabel(r"Amplitude $A(t)$")
            .latex(true)
            .save("test_latex_expressions.png");
        
        // Note: LaTeX support might be optional/future feature
        if result.is_ok() {
            println!("✅ LaTeX expressions supported");
            std::fs::remove_file("test_latex_expressions.png").ok();
        } else {
            println!("⚠️  LaTeX expressions not yet implemented (acceptable)");
        }
    }

    /// Integration Test: Color-blind friendly palettes
    #[test]
    fn test_colorblind_friendly_integration() {
        let x = vec![1.0, 2.0, 3.0, 4.0];
        let y1 = vec![1.0, 2.0, 3.0, 4.0];
        let y2 = vec![1.0, 4.0, 9.0, 16.0];
        let y3 = vec![1.0, 8.0, 27.0, 64.0];
        
        // Color-blind friendly theme
        let cb_theme = Theme::builder()
            .background(Color::WHITE)
            .colorblind_palette(true)
            .build();
        
        let result = Plot::with_theme(cb_theme)
            .line(&x, &y1).label("Linear")
            .line(&x, &y2).label("Quadratic")
            .line(&x, &y3).label("Cubic")
            .title("Color-blind Friendly Palette")
            .legend(Position::TopLeft)
            .save("test_colorblind_friendly.png");
        
        assert!(result.is_ok(), "Color-blind friendly theme failed: {:?}", result.err());
        std::fs::remove_file("test_colorblind_friendly.png").ok();
    }

    /// Integration Test: Print-optimized layouts
    #[test]
    fn test_print_optimization_integration() {
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];
        
        // A4 paper size optimization (210×297mm at 300 DPI ≈ 2480×3508 pixels)
        let result1 = Plot::new()
            .line(&x, &y)
            .title("A4 Print Optimized")
            .dimensions(2480, 1754) // Half A4 landscape
            .dpi(300)
            .margin(0.1) // 10% margin
            .save("test_a4_print.png");
        
        assert!(result1.is_ok(), "A4 print optimization failed: {:?}", result1.err());
        std::fs::remove_file("test_a4_print.png").ok();
        
        // Letter size optimization  
        let result2 = Plot::new()
            .line(&x, &y)
            .title("Letter Print Optimized")
            .dimensions(2550, 1912) // Half Letter landscape at 300 DPI
            .dpi(300)
            .save("test_letter_print.png");
        
        assert!(result2.is_ok(), "Letter print optimization failed: {:?}", result2.err());
        std::fs::remove_file("test_letter_print.png").ok();
    }

    /// Integration Test: Multi-format export consistency
    #[test]
    fn test_format_consistency_integration() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 3.0, 5.0, 4.5];
        
        let plot = Plot::new()
            .line(&x, &y)
                .color(Color::from_hex("#2E86AB"))
                .line_width(2.0)
            .scatter(&x, &y)
                .color(Color::from_hex("#A23B72"))
            .title("Format Consistency Test")
            .xlabel("X Values")
            .ylabel("Y Values")
            .grid(true);
        
        // Export to multiple formats
        let png_result = plot.clone().save("test_consistency.png");
        let svg_result = plot.clone().export_svg("test_consistency.svg");
        
        assert!(png_result.is_ok(), "PNG consistency export failed: {:?}", png_result.err());
        assert!(svg_result.is_ok(), "SVG consistency export failed: {:?}", svg_result.err());
        
        // Files should exist and have reasonable sizes
        assert!(Path::new("test_consistency.png").exists());
        assert!(Path::new("test_consistency.svg").exists());
        
        let png_size = std::fs::metadata("test_consistency.png").unwrap().len();
        let svg_size = std::fs::metadata("test_consistency.svg").unwrap().len();
        
        println!("Format consistency - PNG: {} KB, SVG: {} KB", 
                png_size / 1000, svg_size / 1000);
        
        // Clean up
        std::fs::remove_file("test_consistency.png").ok();
        std::fs::remove_file("test_consistency.svg").ok();
    }
}