// Profiling example for small dataset rendering
// Used to identify performance bottlenecks for optimization

use ruviz::prelude::*;
use std::time::Instant;

fn main() -> Result<()> {
    println!("Profiling small dataset rendering (1K points)");
    println!("==============================================\n");

    // Prepare data
    let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    // Warmup run
    println!("Warmup run...");
    Plot::new()
        .line(&x, &y)
        .title("Warmup")
        .save("test_output/profile_warmup.png")?;

    // Timed runs
    println!("\nRunning 10 iterations to measure performance:");
    let mut times = Vec::new();

    for i in 0..10 {
        let start = Instant::now();

        Plot::new()
            .line(&x, &y)
            .title(&format!("Small Dataset Test {}", i))
            .xlabel("X Axis")
            .ylabel("Y Axis")
            .save("test_output/profile_small.png")?;

        let duration = start.elapsed();
        times.push(duration);
        println!("  Run {}: {:?}", i + 1, duration);
    }

    // Statistics
    let total: std::time::Duration = times.iter().sum();
    let avg = total / times.len() as u32;
    let min = times.iter().min().unwrap();
    let max = times.iter().max().unwrap();

    println!("\nStatistics:");
    println!("  Average: {:?}", avg);
    println!("  Min:     {:?}", min);
    println!("  Max:     {:?}", max);
    println!("  Target:  < 10ms");

    if avg.as_millis() > 10 {
        println!(
            "  Status:  ❌ NEEDS OPTIMIZATION ({:.1}x slower than target)",
            avg.as_millis() as f64 / 10.0
        );
    } else {
        println!("  Status:  ✅ Meets target");
    }

    println!("\nBottleneck analysis:");
    println!(
        "  - Run with 'cargo flamegraph --example profile_small_dataset' for detailed profiling"
    );
    println!("  - Key areas to investigate:");
    println!("    1. Font loading and caching");
    println!("    2. Canvas allocation");
    println!("    3. Coordinate transformation");
    println!("    4. Rendering operations");
    println!("    5. File I/O");

    Ok(())
}
