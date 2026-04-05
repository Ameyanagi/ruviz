//! Basic interactive plotting example
//!
//! Demonstrates zoom, pan, and reset functionality with a simple line plot.
//!
//! Controls:
//! - Mouse wheel: Zoom in/out
//! - Left click + drag: Pan
//! - Right click: Context menu
//! - Right click + drag: Box zoom
//! - Escape: Reset view
//! - Cmd/Ctrl+S: Save PNG
//! - Cmd/Ctrl+C: Copy image
//! - Close window: Exit

use ruviz::prelude::*;

fn main() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to create current-thread Tokio runtime for interactive example")
        .block_on(async_main())
}

async fn async_main() -> Result<()> {
    println!("Starting basic interactive plot example...");
    println!("Controls:");
    println!("  - Mouse wheel: Zoom in/out");
    println!("  - Left click + drag: Pan");
    println!("  - Right click: Context menu");
    println!("  - Right click + drag: Box zoom");
    println!("  - Escape: Reset view");
    println!("  - Cmd/Ctrl+S: Save PNG");
    println!("  - Cmd/Ctrl+C: Copy image");
    println!("  - Close window to exit");

    // Generate sample data - sine wave
    let n_points = 1000;
    let x: Vec<f64> = (0..n_points).map(|i| i as f64 * 0.02).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&t| (t * std::f64::consts::PI).sin())
        .collect();

    // Create plot - no end_series() needed
    let plot = Plot::new()
        .title("Interactive Sine Wave")
        .xlabel("Time (s)")
        .ylabel("Amplitude")
        .legend(Position::TopRight)
        .line(&x, &y)
        .label("sin(x)")
        .into();

    println!("Plot created with {} data points", n_points);

    #[cfg(feature = "interactive")]
    {
        println!("Opening interactive window...");
        show_interactive(plot).await?;
    }

    #[cfg(not(feature = "interactive"))]
    {
        println!("Interactive features not enabled.");
        println!("To enable: cargo run --features interactive --example basic_interaction");
        std::fs::create_dir_all("generated/examples").ok();
        plot.save("generated/examples/basic_interaction_static.png")?;
        println!("Saved static version as: generated/examples/basic_interaction_static.png");
    }

    println!("Example completed!");
    Ok(())
}
