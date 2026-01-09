use ruviz::core::Plot;
use ruviz::core::Position;
use ruviz::render::{Color, LineStyle, Theme};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing legend handles with different line styles...");

    // Generate test data
    let x_data: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y1: Vec<f64> = x_data.iter().map(|&x| x.sin()).collect();
    let y2: Vec<f64> = x_data.iter().map(|&x| x.cos()).collect();
    let y3: Vec<f64> = x_data.iter().map(|&x| (x * 0.5).sin()).collect();

    // Test with labeled series to show legend handles
    Plot::new()
        .title("Legend Handles Test")
        .xlabel("X")
        .ylabel("Y")
        .theme(Theme::publication())
        .line(&x_data, &y1)
        .label("sin(x) - solid")
        .color(Color::BLUE)
        .line(&x_data, &y2)
        .label("cos(x) - dashed")
        .color(Color::RED)
        .style(LineStyle::Dashed)
        .line(&x_data, &y3)
        .label("sin(x/2) - dotted")
        .color(Color::GREEN)
        .style(LineStyle::Dotted)
        .legend(Position::TopRight)
        .save("gallery/basic/legend_handles_test.png")?;

    println!("✅ Legend handles test completed!");
    println!("📂 Check ./gallery/basic/legend_handles_test.png");

    Ok(())
}
