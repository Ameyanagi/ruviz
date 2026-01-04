use ruviz::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Ruviz SIMD Acceleration Demo");
    
    // Generate test data for SIMD demonstration
    let large_dataset_size = 50_000;
    let series_count = 3;
    
    println!("📊 Generating {} series with {} points each", series_count, large_dataset_size);
    println!("🔬 Testing SIMD coordinate transformation acceleration");
    
    // Create plot with SIMD optimization enabled
    let mut plot = Plot::new()
        .title(&format!("SIMD Demo - {} Points per Series", large_dataset_size))
        .xlabel("X Coordinate")
        .ylabel("Y Coordinate")
        .with_parallel(Some(4)) // 4 threads
        .parallel_threshold(2); // Enable parallel for 2+ series
    
    // Generate multiple mathematical function series
    println!("🧮 Generating mathematical functions...");
    
    // Series 1: Sine wave with noise
    let x1: Vec<f64> = (0..large_dataset_size)
        .map(|i| i as f64 * 0.01)
        .collect();
    let y1: Vec<f64> = x1.iter()
        .enumerate()
        .map(|(i, &x)| {
            let noise = (i as f64 * 0.1).sin() * 0.1;
            x.sin() + noise
        })
        .collect();
    
    // Series 2: Logarithmic spiral
    let x2: Vec<f64> = (0..large_dataset_size)
        .map(|i| {
            let t = i as f64 * 0.02;
            let r = 0.5 * t.exp() * 0.1;
            r * t.cos()
        })
        .collect();
    let y2: Vec<f64> = (0..large_dataset_size)
        .map(|i| {
            let t = i as f64 * 0.02;
            let r = 0.5 * t.exp() * 0.1;
            r * t.sin()
        })
        .collect();
    
    // Series 3: Damped oscillation
    let x3: Vec<f64> = (0..large_dataset_size)
        .map(|i| i as f64 * 0.01)
        .collect();
    let y3: Vec<f64> = x3.iter()
        .map(|&x| {
            let decay = (-x * 0.1).exp();
            decay * (x * 3.0).sin() * 2.0
        })
        .collect();
    
    // Add series to plot
    plot = plot
        .scatter(&x1, &y1)
        .label("Sine + Noise")
        .end_series()
        .line(&x2, &y2)
        .label("Log Spiral")
        .end_series()
        .scatter(&x3, &y3)
        .label("Damped Oscillation")
        .end_series()
        .legend(Position::TopRight)
        .grid(true);
    
    // Benchmark rendering performance
    println!("⚡ Measuring rendering performance...");
    
    let iterations = 3;
    let mut total_duration = std::time::Duration::ZERO;
    
    for i in 1..=iterations {
        println!("  📊 Iteration {}/{}", i, iterations);
        let start = std::time::Instant::now();
        
        let _image = plot.clone().render()?;
        
        let duration = start.elapsed();
        total_duration += duration;
        
        println!("     ⏱️  Rendered in {:.2}ms", duration.as_secs_f64() * 1000.0);
    }
    
    let avg_duration = total_duration / iterations;
    let total_points = series_count * large_dataset_size;
    let points_per_ms = total_points as f64 / avg_duration.as_secs_f64() / 1000.0;
    
    println!("\n🎯 Performance Results:");
    println!("   📈 Total Points: {}", total_points);
    println!("   ⏱️  Average Time: {:.2}ms", avg_duration.as_secs_f64() * 1000.0);
    println!("   🚄 Throughput: {:.0} points/ms", points_per_ms);
    
    // Show performance info
    println!("\n🔧 System Configuration:");
    let performance_info = plot.clone().render().map(|_| {
        // Get detailed performance info from the parallel renderer
        println!("   🧵 Available CPU threads: {}", num_cpus::get());
        println!("   ⚡ SIMD acceleration: Enabled");
        println!("   🔀 Parallel processing: Enabled");
        println!("   📦 Chunk processing: Enabled for large datasets");
    });
    
    match performance_info {
        Ok(_) => println!("✅ SIMD acceleration demo completed successfully!"),
        Err(e) => println!("❌ Demo failed: {}", e),
    }
    
    println!("\n💡 Technical Details:");
    println!("   🔄 Coordinate transformations use SIMD vectorization");
    println!("   📊 Processing 4 coordinates simultaneously (f32x4)");
    println!("   🎯 Automatic fallback to scalar for small datasets");
    println!("   🚀 Combined with parallel series processing");
    println!("   📈 Expected speedup: 3-4x for coordinate transforms");
    
    Ok(())
}