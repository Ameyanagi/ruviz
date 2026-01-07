use ruviz::core::plot::Plot;
use ruviz::core::position::Position;
use ruviz::render::Color;
use ruviz::render::style::LineStyle;

#[test]
fn test_solid_line_style() {
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![0.0, 2.0, 1.0, 4.0, 3.0, 5.0];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Solid Line Style (Default)")
        .xlabel("X Values")
        .ylabel("Y Values")
        .line(&x_data, &y_data)
        .style(LineStyle::Solid)
        .save("tests/output/line_solid.png");

    assert!(result.is_ok(), "Solid line style should work");
}

#[test]
fn test_dashed_line_style() {
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![1.0, 3.0, 2.0, 5.0, 4.0, 6.0];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Dashed Line Style")
        .xlabel("X Values")
        .ylabel("Y Values")
        .line(&x_data, &y_data)
        .style(LineStyle::Dashed)
        .color(Color::new(255, 0, 0)) // Red dashed line
        .save("tests/output/line_dashed.png");

    assert!(result.is_ok(), "Dashed line style should work");
}

#[test]
fn test_dotted_line_style() {
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![0.5, 2.5, 1.5, 4.5, 3.5, 5.5];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Dotted Line Style")
        .xlabel("X Values")
        .ylabel("Y Values")
        .line(&x_data, &y_data)
        .style(LineStyle::Dotted)
        .color(Color::new(0, 128, 255)) // Blue dotted line
        .save("tests/output/line_dotted.png");

    assert!(result.is_ok(), "Dotted line style should work");
}

#[test]
fn test_dash_dot_line_style() {
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![2.0, 4.0, 3.0, 6.0, 5.0, 7.0];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Dash-Dot Line Style")
        .xlabel("X Values")
        .ylabel("Y Values")
        .line(&x_data, &y_data)
        .style(LineStyle::DashDot)
        .color(Color::new(0, 200, 0)) // Green dash-dot line
        .save("tests/output/line_dashdot.png");

    assert!(result.is_ok(), "Dash-dot line style should work");
}

#[test]
fn test_dash_dot_dot_line_style() {
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![1.5, 3.5, 2.5, 5.5, 4.5, 6.5];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Dash-Dot-Dot Line Style")
        .xlabel("X Values")
        .ylabel("Y Values")
        .line(&x_data, &y_data)
        .style(LineStyle::DashDotDot)
        .color(Color::new(128, 0, 128)) // Purple dash-dot-dot line
        .save("tests/output/line_dashdotdot.png");

    assert!(result.is_ok(), "Dash-dot-dot line style should work");
}

#[test]
fn test_custom_line_style() {
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![0.8, 2.8, 1.8, 4.8, 3.8, 5.8];

    // Custom pattern: long dash, short dash, long dash, short dash
    let custom_pattern = vec![20.0, 5.0, 10.0, 5.0];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Custom Line Style Pattern")
        .xlabel("X Values")
        .ylabel("Y Values")
        .line(&x_data, &y_data)
        .style(LineStyle::Custom(custom_pattern))
        .color(Color::new(255, 128, 0)) // Orange custom line
        .save("tests/output/line_custom.png");

    assert!(result.is_ok(), "Custom line style should work");
}

#[test]
fn test_multiple_line_styles() {
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let y1_data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let y2_data = vec![6.0, 5.0, 4.0, 3.0, 2.0, 1.0];
    let y3_data = vec![3.0, 3.5, 3.2, 3.8, 3.6, 4.0];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Multiple Line Styles")
        .xlabel("X Values")
        .ylabel("Y Values")
        .line(&x_data, &y1_data)
        .style(LineStyle::Solid)
        .color(Color::new(255, 0, 0))
        .label("Solid")
        .line(&x_data, &y2_data)
        .style(LineStyle::Dashed)
        .color(Color::new(0, 255, 0))
        .label("Dashed")
        .line(&x_data, &y3_data)
        .style(LineStyle::Dotted)
        .color(Color::new(0, 0, 255))
        .label("Dotted")
        .legend(Position::TopRight)
        .save("tests/output/multiple_line_styles.png");

    assert!(result.is_ok(), "Multiple line styles should work");
}

#[test]
fn test_line_width_with_styles() {
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y1_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y2_data = vec![5.0, 4.0, 3.0, 2.0, 1.0];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Line Styles with Different Widths")
        .xlabel("X Values")
        .ylabel("Y Values")
        .line(&x_data, &y1_data)
        .style(LineStyle::Dashed)
        .line_width(3.0)
        .color(Color::new(255, 0, 0))
        .label("Thick Dashed")
        .line(&x_data, &y2_data)
        .style(LineStyle::Dotted)
        .line_width(2.0)
        .color(Color::new(0, 0, 255))
        .label("Medium Dotted")
        .legend(Position::TopRight)
        .save("tests/output/line_styles_with_width.png");

    assert!(
        result.is_ok(),
        "Line styles with different widths should work"
    );
}
