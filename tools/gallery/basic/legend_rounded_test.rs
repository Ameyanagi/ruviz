use ruviz::core::LegendPosition;
use ruviz::core::Plot;
use ruviz::render::{Color, LineStyle, Theme};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("Testing legend with rounded corners...");
    std::fs::create_dir_all("generated/bench")?;

    // Generate test data
    let x_data: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y1: Vec<f64> = x_data.iter().map(|&x| x.sin()).collect();
    let y2: Vec<f64> = x_data.iter().map(|&x| x.cos()).collect();
    let y3: Vec<f64> = x_data.iter().map(|&x| (x * 0.5).sin()).collect();

    // Test with rounded corners on legend frame
    Plot::new()
        .title("Legend with Rounded Corners")
        .xlabel("X")
        .ylabel("Y")
        .theme(Theme::publication())
        .line(&x_data, &y1)
        .label("sin(x)")
        .color(Color::BLUE)
        .line(&x_data, &y2)
        .label("cos(x)")
        .color(Color::RED)
        .line_style(LineStyle::Dashed)
        .line(&x_data, &y3)
        .label("sin(x/2)")
        .color(Color::GREEN)
        .line_style(LineStyle::Dotted)
        .legend(LegendPosition::UpperRight)
        .legend_corner_radius(6.0)
        .save("generated/bench/legend_rounded_test.png")?;

    println!("Legend rounded corners test completed!");
    println!("Check generated/bench/legend_rounded_test.png");

    Ok(())
}
