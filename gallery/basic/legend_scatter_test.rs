use ruviz::core::Plot;
use ruviz::core::Position;
use ruviz::render::{Color, MarkerStyle, Theme};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing legend handles with scatter markers (auto-positioning)...");

    // Generate test data
    let x1: Vec<f64> = (0..20).map(|i| i as f64 * 0.5).collect();
    let y1: Vec<f64> = x1.iter().map(|&x| x.sin() + 0.1 * (x * 3.0).cos()).collect();
    let x2: Vec<f64> = (0..15).map(|i| i as f64 * 0.7).collect();
    let y2: Vec<f64> = x2.iter().map(|&x| x.cos() - 0.1 * x).collect();
    let x3: Vec<f64> = (0..25).map(|i| i as f64 * 0.4).collect();
    let y3: Vec<f64> = x3.iter().map(|&x| 0.5 * x.sin() + 0.5).collect();

    // Test with scatter series using auto-positioning (Position::Best)
    // The legend will automatically find the position with minimum data overlap
    Plot::new()
        .title("Legend Scatter Markers Test (Auto-Position)")
        .xlabel("X")
        .ylabel("Y")
        .theme(Theme::publication())
        .scatter(&x1, &y1)
        .label("Data A - circles")
        .color(Color::BLUE)
        .marker(MarkerStyle::Circle)
        .end_series()
        .scatter(&x2, &y2)
        .label("Data B - squares")
        .color(Color::RED)
        .marker(MarkerStyle::Square)
        .end_series()
        .scatter(&x3, &y3)
        .label("Data C - triangles")
        .color(Color::GREEN)
        .marker(MarkerStyle::Triangle)
        .end_series()
        .legend(Position::Best)  // Auto-positioning to minimize overlap
        .save("gallery/basic/legend_scatter_test.png")?;

    println!("✅ Legend scatter test completed!");
    println!("📂 Check ./gallery/basic/legend_scatter_test.png");
    println!("   Legend should auto-position to avoid data overlap");

    Ok(())
}
