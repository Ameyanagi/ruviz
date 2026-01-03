/// Gallery Example: Legend Features
///
/// This example demonstrates all legend features implemented in ruviz:
/// - Multiple position options
/// - Different series types with proper visual handles
/// - Rounded corners
/// - Multi-column (horizontal) layout
/// - Frame styling with background and border

use ruviz::core::Plot;
use ruviz::core::Position;
use ruviz::render::{Color, LineStyle, MarkerStyle, Theme};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Legend Features Gallery Example");
    println!("================================");

    // Generate sample data
    let x: Vec<f64> = (0..80).map(|i| i as f64 * 0.1).collect();
    let y_sin: Vec<f64> = x.iter().map(|&x| x.sin()).collect();
    let y_cos: Vec<f64> = x.iter().map(|&x| x.cos()).collect();
    let y_exp: Vec<f64> = x.iter().map(|&x| (-x * 0.1).exp() * x.sin()).collect();

    // Example 1: Standard legend with line styles
    println!("1. Creating legend with line styles...");
    Plot::new()
        .title("Line Styles in Legend")
        .xlabel("X")
        .ylabel("Y")
        .theme(Theme::publication())
        .line(&x, &y_sin)
        .label("Solid line")
        .color(Color::BLUE)
        .end_series()
        .line(&x, &y_cos)
        .label("Dashed line")
        .color(Color::RED)
        .style(LineStyle::Dashed)
        .end_series()
        .line(&x, &y_exp)
        .label("Dotted line")
        .color(Color::GREEN)
        .style(LineStyle::Dotted)
        .end_series()
        .legend(Position::TopRight)
        .legend_corner_radius(5.0)
        .save("gallery/publication/legend_line_styles.png")?;

    // Example 2: Scatter markers in legend
    println!("2. Creating legend with scatter markers...");
    let x_scatter: Vec<f64> = (0..15).map(|i| i as f64 * 0.5).collect();
    let y1: Vec<f64> = x_scatter.iter().map(|&x| x.sin() + 0.5).collect();
    let y2: Vec<f64> = x_scatter.iter().map(|&x| x.cos()).collect();
    let y3: Vec<f64> = x_scatter.iter().map(|&x| (x * 0.8).sin() - 0.3).collect();

    Plot::new()
        .title("Scatter Markers in Legend")
        .xlabel("X")
        .ylabel("Y")
        .theme(Theme::publication())
        .scatter(&x_scatter, &y1)
        .label("Circle markers")
        .color(Color::BLUE)
        .marker(MarkerStyle::Circle)
        .end_series()
        .scatter(&x_scatter, &y2)
        .label("Square markers")
        .color(Color::RED)
        .marker(MarkerStyle::Square)
        .end_series()
        .scatter(&x_scatter, &y3)
        .label("Triangle markers")
        .color(Color::GREEN)
        .marker(MarkerStyle::Triangle)
        .end_series()
        .legend(Position::TopLeft)
        .legend_corner_radius(4.0)
        .save("gallery/publication/legend_scatter_markers.png")?;

    // Example 3: Horizontal legend layout
    println!("3. Creating horizontal legend layout...");
    Plot::new()
        .title("Horizontal Legend Layout")
        .xlabel("X")
        .ylabel("Y")
        .theme(Theme::publication())
        .line(&x, &y_sin)
        .label("sin(x)")
        .color(Color::BLUE)
        .end_series()
        .line(&x, &y_cos)
        .label("cos(x)")
        .color(Color::RED)
        .style(LineStyle::Dashed)
        .end_series()
        .line(&x, &y_exp)
        .label("decay")
        .color(Color::GREEN)
        .style(LineStyle::Dotted)
        .end_series()
        .legend(Position::BottomCenter)
        .legend_columns(3)
        .legend_corner_radius(4.0)
        .save("gallery/publication/legend_horizontal.png")?;

    // Example 4: Mixed line and scatter with 2-column layout
    println!("4. Creating mixed series legend...");
    Plot::new()
        .title("Mixed Series Types")
        .xlabel("X")
        .ylabel("Y")
        .theme(Theme::publication())
        .line(&x, &y_sin)
        .label("Line A")
        .color(Color::BLUE)
        .end_series()
        .scatter(&x_scatter, &y1)
        .label("Scatter B")
        .color(Color::RED)
        .marker(MarkerStyle::Circle)
        .end_series()
        .line(&x, &y_cos)
        .label("Line C")
        .color(Color::GREEN)
        .style(LineStyle::Dashed)
        .end_series()
        .scatter(&x_scatter, &y2)
        .label("Scatter D")
        .color(Color::new(255, 165, 0))
        .marker(MarkerStyle::Diamond)
        .end_series()
        .legend(Position::TopRight)
        .legend_columns(2)
        .legend_corner_radius(6.0)
        .save("gallery/publication/legend_mixed.png")?;

    println!();
    println!("Gallery examples created!");
    println!("Check the following files:");
    println!("  - gallery/publication/legend_line_styles.png");
    println!("  - gallery/publication/legend_scatter_markers.png");
    println!("  - gallery/publication/legend_horizontal.png");
    println!("  - gallery/publication/legend_mixed.png");

    Ok(())
}
