//! Interactive multi-series line exploration example
//!
//! Demonstrates zooming into dense line data with several labeled signals.
//!
//! Run with: cargo run --features interactive --example interactive_multi_series

use ruviz::prelude::*;
use std::f64::consts::PI;

fn main() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to create current-thread Tokio runtime for interactive example")
        .block_on(async_main())
}

async fn async_main() -> Result<()> {
    println!("Starting interactive multi-series example...");
    println!("Controls:");
    println!("  - Mouse wheel: Zoom in/out");
    println!("  - Left click + drag: Box zoom");
    println!("  - Right click + drag: Pan");
    println!("  - Escape: Reset view");
    println!("  - Close window to exit");

    let sample_count = 4_000;
    let x: Vec<f64> = (0..sample_count).map(|i| i as f64 * 0.01).collect();
    let primary: Vec<f64> = x.iter().map(|&t| (t * PI * 0.7).sin()).collect();
    let harmonic: Vec<f64> = x
        .iter()
        .map(|&t| 0.55 * (t * PI * 2.1 + 0.4).sin())
        .collect();
    let damped: Vec<f64> = x
        .iter()
        .map(|&t| (-t * 0.04).exp() * (t * PI * 3.4).sin() + 0.12 * (t * PI * 0.2).cos())
        .collect();

    let plot: Plot = Plot::new()
        .title("Interactive Multi-Signal Explorer")
        .xlabel("Time (s)")
        .ylabel("Amplitude")
        .legend(Position::TopRight)
        .line(&x, &primary)
        .label("Primary oscillation")
        .line(&x, &harmonic)
        .label("Harmonic component")
        .line(&x, &damped)
        .label("Damped carrier")
        .into();

    println!("Plot created with {} samples per series", sample_count);

    #[cfg(feature = "interactive")]
    {
        println!("Opening interactive window...");
        show_interactive(plot).await?;
    }

    #[cfg(not(feature = "interactive"))]
    {
        println!("Interactive features not enabled.");
        println!("To enable: cargo run --features interactive --example interactive_multi_series");
        std::fs::create_dir_all("examples/output").ok();
        plot.save("examples/output/interactive_multi_series_static.png")?;
        println!("Saved static version as: examples/output/interactive_multi_series_static.png");
    }

    Ok(())
}
