//! Validation tests for core Plot functionality

use crate::prelude::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plot_creation() {
        let plot = Plot::new();
        // Test that plot can be created without errors
        assert!(true); // Basic creation test
    }

    #[test]
    fn test_line_plot_render() {
        let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y_data = vec![2.0, 4.0, 1.0, 3.0, 5.0];

        let result = Plot::new()
            .title("Test Line Plot".to_string())
            .line(&x_data, &y_data)
            .render();

        assert!(result.is_ok());
        if let Ok(image) = result {
            // Default is 6.4×4.8 inches at 100 DPI = 640×480 pixels (matplotlib default)
            assert_eq!(image.width, 640);
            assert_eq!(image.height, 480);
            assert_eq!(image.pixels.len(), 640 * 480 * 4); // RGBA
        }
    }

    #[test]
    fn test_scatter_plot_render() {
        let x_data = vec![1.0, 2.0, 3.0];
        let y_data = vec![1.0, 4.0, 2.0];

        let result = Plot::new().scatter(&x_data, &y_data).render();

        assert!(result.is_ok());
        if let Ok(image) = result {
            assert!(!image.pixels.is_empty());
        }
    }

    #[test]
    fn test_empty_data_error() {
        let result = Plot::new().render();
        assert!(result.is_err());

        if let Err(e) = result {
            matches!(e, crate::core::PlottingError::NoDataSeries);
        }
    }

    #[test]
    fn test_mismatched_data_length() {
        let x_data = vec![1.0, 2.0, 3.0];
        let y_data = vec![1.0, 2.0]; // Different length

        let result = Plot::new().line(&x_data, &y_data).render();

        assert!(result.is_err());
    }

    #[test]
    fn test_color_parsing() {
        let red = Color::from_hex("#FF0000").unwrap();
        assert_eq!(red.r, 255);
        assert_eq!(red.g, 0);
        assert_eq!(red.b, 0);
        assert_eq!(red.a, 255);

        let with_alpha = Color::from_hex("#FF000080").unwrap();
        assert_eq!(with_alpha.a, 128);
    }

    #[test]
    fn test_data_validation() {
        let valid_data = vec![1.0, 2.0, 3.0, 4.0];
        let result = crate::core::PlottingError::validate_data(&valid_data);
        assert!(result.is_ok());

        let invalid_data = vec![1.0, f64::NAN, 3.0];
        let result = crate::core::PlottingError::validate_data(&invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_bar_plot_render() {
        let categories = vec!["A", "B", "C"];
        let values = vec![10.0, 15.0, 7.0];

        let result = Plot::new().bar(&categories, &values).render();

        assert!(result.is_ok());
    }

    #[test]
    fn test_theme_usage() {
        let x_data = vec![1.0, 2.0, 3.0];
        let y_data = vec![1.0, 4.0, 2.0];

        let result = Plot::with_theme(Theme::dark())
            .line(&x_data, &y_data)
            .render();

        assert!(result.is_ok());
    }

    #[test]
    fn test_dpi_independence_proportions() {
        // The ratio of font size to figure size should be constant across DPI values
        use crate::core::{PlotConfig, in_to_px, pt_to_px};

        let config = PlotConfig::default();

        // At 100 DPI
        let font_px_100 = pt_to_px(config.typography.base_size, 100.0);
        let figure_px_100 = in_to_px(config.figure.width, 100.0);
        let ratio_100 = font_px_100 / figure_px_100;

        // At 300 DPI
        let font_px_300 = pt_to_px(config.typography.base_size, 300.0);
        let figure_px_300 = in_to_px(config.figure.width, 300.0);
        let ratio_300 = font_px_300 / figure_px_300;

        // Ratios should be equal (DPI-independent)
        assert!(
            (ratio_100 - ratio_300).abs() < 0.0001,
            "Font-to-figure ratio should be constant across DPI: {} vs {}",
            ratio_100,
            ratio_300
        );
    }

    #[test]
    fn test_size_methods() {
        use crate::core::PlotConfig;

        // Test size() method sets inches correctly
        let plot = Plot::new().size(8.0, 6.0);
        let config = plot.get_config();
        assert!((config.figure.width - 8.0).abs() < 0.001);
        assert!((config.figure.height - 6.0).abs() < 0.001);

        // Test size_px() converts to inches at reference DPI
        let plot = Plot::new().size_px(800, 600);
        let config = plot.get_config();
        assert!((config.figure.width - 8.0).abs() < 0.001); // 800 / 100 = 8.0
        assert!((config.figure.height - 6.0).abs() < 0.001); // 600 / 100 = 6.0
    }

    #[test]
    fn test_dpi_method_updates_dimensions() {
        let plot = Plot::new().size(6.4, 4.8).dpi(200);
        let config = plot.get_config();

        // Canvas size should be width * dpi
        let (width, height) = config.canvas_size();
        assert_eq!(width, 1280); // 6.4 * 200
        assert_eq!(height, 960); // 4.8 * 200
    }

    #[test]
    fn test_plot_style_presets() {
        use crate::core::PlotStyle;

        // Test that all style presets produce valid configurations
        for style in PlotStyle::all() {
            let plot = Plot::with_style(*style);
            let config = plot.get_config();

            // Basic sanity checks
            assert!(config.figure.width > 0.0);
            assert!(config.figure.height > 0.0);
            assert!(config.figure.dpi > 0.0);
            assert!(config.typography.base_size > 0.0);
            assert!(config.lines.data_width > 0.0);
        }

        // Verify specific style properties
        let presentation = PlotStyle::Presentation.config();
        assert!((presentation.typography.base_size - 14.0).abs() < 0.01);
        assert!((presentation.lines.data_width - 2.5).abs() < 0.01);

        let ieee = PlotStyle::IEEE.config();
        assert!((ieee.typography.base_size - 8.0).abs() < 0.01);
        assert!((ieee.figure.width - 3.5).abs() < 0.01); // IEEE single column
    }

    #[test]
    fn test_computed_margins() {
        use crate::core::{MarginConfig, PlotConfig};

        // Test proportional margins - values are constant regardless of content
        let prop_config = PlotConfig {
            margins: MarginConfig::proportional(),
            ..PlotConfig::default()
        };
        let margins = prop_config.compute_margins(true, true, true);
        let margins_no_content = prop_config.compute_margins(false, false, false);

        // Proportional margins should be the same regardless of content
        assert!((margins.top - margins_no_content.top).abs() < 0.001);
        assert!((margins.left - margins_no_content.left).abs() < 0.001);

        // Verify proportional values match matplotlib defaults
        // Figure: 6.4 x 4.8 inches, margins: left=0.125, right=0.1, top=0.12, bottom=0.11
        assert!((margins.left - 6.4 * 0.125).abs() < 0.001); // 0.8 inches
        assert!((margins.right - 6.4 * 0.1).abs() < 0.001); // 0.64 inches
        assert!((margins.top - 4.8 * 0.12).abs() < 0.001); // 0.576 inches
        assert!((margins.bottom - 4.8 * 0.11).abs() < 0.001); // 0.528 inches

        // Test auto margins - these DO vary based on content
        let auto_config = PlotConfig {
            margins: MarginConfig::auto(),
            ..PlotConfig::default()
        };
        let auto_margins = auto_config.compute_margins(true, true, true);
        let auto_minimal = auto_config.compute_margins(false, false, false);

        // With Auto, margins should differ based on content
        assert!(auto_margins.top > auto_minimal.top); // More space for title
        assert!(auto_margins.left > auto_minimal.left); // More space for Y-axis labels

        // Test ContentDriven margins (default) - returns fallback values
        let content_config = PlotConfig::default();
        let content_margins = content_config.compute_margins(true, true, true);
        // ContentDriven returns fallback ComputedMargins (actual layout uses LayoutCalculator)
        assert!(content_margins.left > 0.0);
        assert!(content_margins.right > 0.0);
    }
}
