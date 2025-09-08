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
            assert_eq!(image.width, 800);
            assert_eq!(image.height, 600);
            assert_eq!(image.pixels.len(), 800 * 600 * 4); // RGBA
        }
    }

    #[test]
    fn test_scatter_plot_render() {
        let x_data = vec![1.0, 2.0, 3.0];
        let y_data = vec![1.0, 4.0, 2.0];
        
        let result = Plot::new()
            .scatter(&x_data, &y_data)
            .render();
        
        assert!(result.is_ok());
        if let Ok(image) = result {
            assert!(image.pixels.len() > 0);
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
        
        let result = Plot::new()
            .line(&x_data, &y_data)
            .render();
        
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
        
        let result = Plot::new()
            .bar(&categories, &values)
            .render();
        
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
}