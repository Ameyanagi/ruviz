use ruviz::core::Plot;
use std::time::Instant;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Parallel Rendering Demo");
    println!("=========================");
    
    // Test 1: Medium dataset with parallel rendering
    let n = 75_000;
    println!("\n📊 Test 1: {} points with parallel rendering", n);
    
    let x_data: Vec<f64> = (0..n).map(|i| (i as f64) / 1000.0).collect();
    let y_data: Vec<f64> = x_data.iter().map(|&x| x.sin() + 0.5 * (x * 3.0).cos()).collect();
    
    let start = Instant::now();
    
    let plot_result = Plot::new()
        .title("Parallel Rendering Demo".to_string())
        .with_parallel(Some(4)) // Use 4 threads
        .parallel_threshold(20_000) // Lower threshold for demo
        .line(&x_data, &y_data)
        .render();
    
    let duration = start.elapsed();
    
    match plot_result {
        Ok(image) => {
            println!("✅ Successfully rendered {} points", n);
            println!("   ⏱️ Render time: {:.2}ms", duration.as_millis());
            println!("   🖼️ Image size: {}x{}", image.width, image.height);
            println!("   🧵 Used parallel rendering with 4 threads");
        }
        Err(e) => {
            println!("❌ Rendering failed: {}", e);
            return Err(e.into());
        }
    }
    
    // Test 2: Large dataset that triggers DataShader
    let n_large = 150_000;
    println!("\n📊 Test 2: {} points (DataShader + parallel)", n_large);
    
    let x_large: Vec<f64> = (0..n_large).map(|i| (i as f64) * 0.0001).collect();
    let y_large: Vec<f64> = x_large.iter().map(|&x| (x * 10.0).sin() * (x * 2.0).cos()).collect();
    
    let start_large = Instant::now();
    
    let large_result = Plot::new()
        .title("Large Dataset with DataShader".to_string())
        .scatter(&x_large, &y_large)
        .render();
    
    let duration_large = start_large.elapsed();
    
    match large_result {
        Ok(image) => {
            println!("✅ Successfully rendered {} points with DataShader", n_large);
            println!("   ⏱️ Render time: {:.2}ms", duration_large.as_millis());
            println!("   🖼️ Image size: {}x{}", image.width, image.height);
            println!("   🎯 Automatically used DataShader for >100K points");
        }
        Err(e) => {
            println!("❌ Large dataset rendering failed: {}", e);
            return Err(e.into());
        }
    }
    
    // Performance comparison
    println!("\n📈 Performance Summary");
    println!("   Medium dataset ({} pts): {:.2}ms", n, duration.as_millis());
    println!("   Large dataset  ({} pts): {:.2}ms", n_large, duration_large.as_millis());
    
    let efficiency = (n_large as f64 / n as f64) / (duration_large.as_millis() as f64 / duration.as_millis() as f64);
    println!("   Scaling efficiency: {:.1}x", efficiency);
    
    if duration.as_millis() < 200 && duration_large.as_millis() < 500 {
        println!("✅ Performance targets met!");
    } else {
        println!("⚠️ Performance could be improved");
    }
    
    Ok(())
}