//! Basic interactive plotting example
//!
//! Demonstrates zoom, pan, and reset functionality with a simple line plot.
//!
//! Controls:
//! - Mouse wheel: Zoom in/out
//! - Left click + drag: Pan
//! - Double click: Reset view
//! - Escape: Exit

use ruviz::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🎮 Starting basic interactive plot example...");
    println!("Controls:");
    println!("  - Mouse wheel: Zoom in/out");
    println!("  - Left click + drag: Pan");
    println!("  - Escape: Reset view");
    println!("  - Close window to exit");

    // Generate sample data - sine wave
    let n_points = 1000;
    let x_data: Vec<f64> = (0..n_points).map(|i| i as f64 * 0.02).collect();
    let y_data: Vec<f64> = x_data
        .iter()
        .map(|&x| (x * std::f64::consts::PI).sin())
        .collect();

    // Create plot
    let plot = Plot::new()
        .line(&x_data, &y_data)
        .title("Interactive Sine Wave - Basic Example")
        .xlabel("Time (s)")
        .ylabel("Amplitude")
        .legend(Position::TopRight);

    println!("📊 Plot created with {} data points", n_points);

    // Show interactive window
    #[cfg(feature = "interactive")]
    {
        println!("🚀 Opening interactive window...");
        show_interactive(plot).await?;
    }

    #[cfg(not(feature = "interactive"))]
    {
        println!("⚠️ Interactive features not enabled.");
        println!("To enable: cargo run --features interactive --example basic_interaction");

        // Fallback to static plot
        plot.save("examples/output/basic_interaction_static.png")?;
        println!("💾 Saved static version as: examples/output/basic_interaction_static.png");
    }

    println!("✅ Example completed!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_interaction_example() {
        // Test that the example can create a plot without crashing
        let x_data = vec![0.0, 1.0, 2.0, 3.0];
        let y_data = vec![0.0, 1.0, 0.0, -1.0];

        let plot = Plot::new()
            .line(&x_data, &y_data)
            .title("Test Plot")
            .xlabel("X")
            .ylabel("Y");

        // Test static save
        let result = plot.save("examples/output/test_basic_interaction.png");
        assert!(result.is_ok());

        // Clean up
        std::fs::remove_file("examples/output/test_basic_interaction.png").ok();
    }
}
