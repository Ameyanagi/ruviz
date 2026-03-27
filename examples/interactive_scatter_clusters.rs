//! Interactive scatter exploration example
//!
//! Demonstrates zoom and pan across clustered point clouds and outliers.
//!
//! Run with: cargo run --features interactive --example interactive_scatter_clusters

use ruviz::prelude::*;

fn main() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to create current-thread Tokio runtime for interactive example")
        .block_on(async_main())
}

async fn async_main() -> Result<()> {
    println!("Starting interactive scatter cluster example...");
    println!("Controls:");
    println!("  - Mouse wheel: Zoom in/out");
    println!("  - Left click + drag: Box zoom");
    println!("  - Right click + drag: Pan");
    println!("  - Escape: Reset view");
    println!("  - Close window to exit");

    let (cluster_a_x, cluster_a_y) = build_cluster(1.5, 2.0, 0.0, 110);
    let (cluster_b_x, cluster_b_y) = build_cluster(6.5, 6.2, 1.3, 140);
    let (cluster_c_x, cluster_c_y) = build_cluster(9.0, 2.8, 2.4, 90);

    let outlier_x = vec![-0.8, 0.4, 4.9, 7.7, 10.6, 11.3];
    let outlier_y = vec![5.3, 6.6, -0.4, 8.4, 1.3, 7.0];

    let plot: Plot = Plot::new()
        .title("Interactive Clustered Scatter")
        .xlabel("Feature 1")
        .ylabel("Feature 2")
        .legend(Position::TopLeft)
        .xlim(-1.5, 12.0)
        .ylim(-1.5, 9.5)
        .scatter(&cluster_a_x, &cluster_a_y)
        .marker_size(7.0)
        .label("Cluster A")
        .scatter(&cluster_b_x, &cluster_b_y)
        .marker_size(7.0)
        .label("Cluster B")
        .scatter(&cluster_c_x, &cluster_c_y)
        .marker_size(7.0)
        .label("Cluster C")
        .scatter(&outlier_x, &outlier_y)
        .marker_size(9.0)
        .label("Outliers")
        .into();

    println!(
        "Plot created with {} scatter points",
        cluster_a_x.len() + cluster_b_x.len() + cluster_c_x.len() + outlier_x.len()
    );

    #[cfg(feature = "interactive")]
    {
        println!("Opening interactive window...");
        show_interactive(plot).await?;
    }

    #[cfg(not(feature = "interactive"))]
    {
        println!("Interactive features not enabled.");
        println!(
            "To enable: cargo run --features interactive --example interactive_scatter_clusters"
        );
        std::fs::create_dir_all("examples/output").ok();
        plot.save("examples/output/interactive_scatter_clusters_static.png")?;
        println!(
            "Saved static version as: examples/output/interactive_scatter_clusters_static.png"
        );
    }

    Ok(())
}

fn build_cluster(center_x: f64, center_y: f64, phase: f64, count: usize) -> (Vec<f64>, Vec<f64>) {
    let mut x = Vec::with_capacity(count);
    let mut y = Vec::with_capacity(count);

    for i in 0..count {
        let angle = phase + i as f64 * 0.31;
        let ring = 0.18 + (i % 9) as f64 * 0.035;
        let jitter = (i as f64 * 0.17).sin() * 0.04;
        x.push(center_x + (ring + jitter) * angle.cos());
        y.push(center_y + (ring - jitter) * angle.sin());
    }

    (x, y)
}
