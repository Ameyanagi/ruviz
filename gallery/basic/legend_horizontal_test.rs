use ruviz::core::Plot;
use ruviz::core::Position;
use ruviz::render::{Color, LineStyle, Theme};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("Testing legend horizontal (multi-column) layout...");

    // Generate test data
    let x_data: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y1: Vec<f64> = x_data.iter().map(|&x| x.sin()).collect();
    let y2: Vec<f64> = x_data.iter().map(|&x| x.cos()).collect();
    let y3: Vec<f64> = x_data.iter().map(|&x| (x * 0.5).sin()).collect();

    // Test with single-row horizontal layout (3 columns for 3 items)
    Plot::new()
        .title("Legend Horizontal Layout (Single Row)")
        .xlabel("X")
        .ylabel("Y")
        .theme(Theme::publication())
        .line(&x_data, &y1)
        .label("sin(x)")
        .color(Color::BLUE)
        .line(&x_data, &y2)
        .label("cos(x)")
        .color(Color::RED)
        .style(LineStyle::Dashed)
        .line(&x_data, &y3)
        .label("sin(x/2)")
        .color(Color::GREEN)
        .style(LineStyle::Dotted)
        .legend(Position::TopCenter)
        .legend_columns(3) // Single row with all 3 items
        .legend_corner_radius(4.0)
        .save("gallery/basic/legend_horizontal_test.png")?;

    println!("Legend horizontal layout test completed!");
    println!("Check ./gallery/basic/legend_horizontal_test.png");

    Ok(())
}
