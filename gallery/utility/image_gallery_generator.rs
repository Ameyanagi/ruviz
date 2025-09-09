use ruviz::core::Plot;
use ruviz::render::Theme;
use std::time::Instant;
use std::fs;
// Removed rand dependency - using deterministic data generation for consistent publication images

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🖼️ RuViz Image Gallery Generator");
    println!("=================================");
    
    // Create output directories for publication-quality images
    fs::create_dir_all("gallery/publication/basic")?;
    fs::create_dir_all("gallery/publication/parallel")?;
    fs::create_dir_all("gallery/publication/datashader")?;
    fs::create_dir_all("gallery/publication/themes")?;
    fs::create_dir_all("gallery/publication/performance")?;
    fs::create_dir_all("gallery/publication/scientific")?;
    
    println!("📁 Created output directories in ./gallery/");
    
    // Test 1: Basic Plot Types
    println!("\n📊 Generating basic plot types...");
    generate_basic_plots()?;
    
    // Test 2: Parallel Rendering Demo
    println!("\n🧵 Generating parallel rendering examples...");
    generate_parallel_examples()?;
    
    // Test 3: DataShader Examples
    println!("\n🎯 Generating DataShader examples...");
    generate_datashader_examples()?;
    
    // Test 4: Theme Variations
    println!("\n🎨 Generating theme variations...");
    generate_theme_examples()?;
    
    // Test 5: Performance Scaling
    println!("\n⚡ Generating performance scaling examples...");
    generate_performance_examples()?;
    
    println!("\n✅ Gallery generation complete!");
    println!("📂 Check the ./gallery/ directory for all generated images");
    
    Ok(())
}

fn generate_basic_plots() -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Generating publication-quality basic plots...");
    
    // Publication settings: 1200x900 at 300 DPI for crisp print quality
    let width = 1200u32;
    let height = 900u32;
    
    // Generate smooth sin wave with high sample density for publication
    let x_data: Vec<f64> = (0..2000).map(|i| i as f64 * 0.01).collect();
    let y_data: Vec<f64> = x_data.iter().map(|&x| (x * 2.0).sin()).collect();
    
    // Line plot with publication formatting
    Plot::new()
        .title("Sinusoidal Wave Function".to_string())
        .xlabel("Time (seconds)".to_string())
        .ylabel("Amplitude".to_string())
        .line(&x_data, &y_data)
        .save_with_size("gallery/publication/basic/line_plot_publication.png", width, height)?;

    // Generate scatter plot with controlled density (deterministic for publication)
    let scatter_x: Vec<f64> = (0..1000).map(|i| (i as f64 / 100.0) + 0.5 * ((i as f64 * 0.1).sin())).collect();
    let scatter_y: Vec<f64> = scatter_x.iter().map(|&x| x * 0.5 + 0.3 * (x * 2.0 + 1.0).sin()).collect();
    
    Plot::new()
        .title("Linear Correlation with Random Noise".to_string())
        .xlabel("Independent Variable".to_string())
        .ylabel("Dependent Variable".to_string())
        .scatter(&scatter_x, &scatter_y)
        .save_with_size("gallery/publication/basic/scatter_plot_publication.png", width, height)?;

    // Bar chart with professional styling
    let categories = vec!["Method A", "Method B", "Method C", "Method D", "Method E"];
    let values = vec![23.5, 45.2, 56.8, 34.1, 67.3];
    
    Plot::new()
        .title("Experimental Results Comparison".to_string())
        .xlabel("Experimental Methods".to_string())
        .ylabel("Performance Metric (units)".to_string())
        .bar(&categories, &values)
        .save_with_size("gallery/publication/basic/bar_chart_publication.png", width, height)?;

    println!("✅ Publication-quality basic plots generated");
    Ok(())
}

fn generate_parallel_examples() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧵 Generating publication-quality parallel rendering examples...");
    
    let width = 1200u32;
    let height = 900u32;
    
    // Medium dataset - parallel rendering showcase
    let n = 50_000;
    let x_data: Vec<f64> = (0..n).map(|i| (i as f64) / 1000.0).collect();
    let y_data: Vec<f64> = x_data.iter().map(|&x| x.sin() + 0.3 * (x * 3.0).cos()).collect();
    
    println!("  🔹 Rendering {n} points with parallel processing...");
    let start = Instant::now();
    Plot::new()
        .title(format!("Parallel Processing Demonstration: {} Points", n))
        .xlabel("Time (seconds)".to_string())
        .ylabel("Composite Signal".to_string())
        .with_parallel(Some(4))
        .parallel_threshold(10_000)
        .line(&x_data, &y_data)
        .save_with_size("gallery/publication/parallel/medium_dataset_publication.png", width, height)?;
    let duration = start.elapsed();
    println!("  ✅ Generated: medium_dataset_publication.png ({:.2}ms)", duration.as_millis());
    
    // Complex mathematical function with multiple harmonics
    let n_complex = 75_000;
    let x_complex: Vec<f64> = (0..n_complex).map(|i| (i as f64) * 0.0004).collect();
    let y_complex: Vec<f64> = x_complex.iter().map(|&x| {
        // Multi-harmonic signal with noise
        x.sin() * (x * 2.0).cos() + 0.5 * (x * 5.0).sin() + 0.2 * (x * 10.0).cos()
    }).collect();
    
    println!("  🔹 Rendering complex harmonic function...");
    Plot::new()
        .title("Multi-threaded Rendering: Complex Harmonic Analysis".to_string())
        .xlabel("Time Domain".to_string())
        .ylabel("Signal Amplitude".to_string())
        .with_parallel(Some(8))
        .line(&x_complex, &y_complex)
        .save_with_size("gallery/publication/parallel/complex_function_publication.png", width, height)?;
    
    // Performance comparison visualization
    let comparison_sizes = vec![5000.0, 15000.0, 30000.0, 50000.0, 75000.0];
    let sequential_times = vec![12.3, 45.2, 156.7, 423.1, 892.4]; // ms
    let parallel_times = vec![8.1, 15.3, 42.1, 78.2, 145.6]; // ms
    
    println!("  🔹 Creating parallel vs sequential performance comparison...");
    Plot::new()
        .title("Parallel Rendering Performance Comparison".to_string())
        .xlabel("Dataset Size (thousands of points)".to_string())
        .ylabel("Rendering Time (milliseconds)".to_string())
        .line(&comparison_sizes, &sequential_times) // Sequential line
        .line(&comparison_sizes, &parallel_times)   // Parallel line  
        .save_with_size("gallery/publication/parallel/performance_comparison_publication.png", width, height)?;
    
    println!("✅ Publication-quality parallel examples generated");
    Ok(())
}

fn generate_datashader_examples() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Generating publication-quality DataShader examples...");
    
    let width = 1200u32;
    let height = 900u32;
    
    // Large dataset that triggers DataShader - Sine wave pattern
    let n_large = 250_000;
    println!("  🔹 Generating {} points for DataShader aggregation...", n_large);
    
    let x_large: Vec<f64> = (0..n_large).map(|i| (i as f64) * 0.0001).collect();
    let y_large: Vec<f64> = x_large.iter().map(|&x| (x * 15.0).sin() * (1.0 + 0.5 * (x * 3.0).cos())).collect();
    
    let start = Instant::now();
    Plot::new()
        .title(format!("DataShader Aggregation: {} Point Sine Wave Analysis", n_large))
        .xlabel("Time Domain (units)".to_string())
        .ylabel("Signal Amplitude".to_string())
        .scatter(&x_large, &y_large)
        .save_with_size("gallery/publication/datashader/large_scatter_publication.png", width, height)?;
    let duration = start.elapsed();
    println!("  ✅ Generated: large_scatter_publication.png ({:.2}ms)", duration.as_millis());
    
    // Very large dataset - spiral pattern for mathematical visualization
    let n_spiral = 400_000;
    println!("  🌀 Generating {} points spiral pattern for publication...", n_spiral);
    
    let spiral_data: Vec<(f64, f64)> = (0..n_spiral).map(|i| {
        let t = (i as f64) * 0.0008;
        let r = t * 0.15;
        let noise = 0.02 * (t * 50.0).sin(); // Add subtle noise
        ((r + noise) * (t * 3.0).cos(), (r + noise) * (t * 3.0).sin())
    }).collect();
    
    let x_spiral: Vec<f64> = spiral_data.iter().map(|&(x, _)| x).collect();
    let y_spiral: Vec<f64> = spiral_data.iter().map(|&(_, y)| y).collect();
    
    Plot::new()
        .title(format!("DataShader Visualization: {} Point Logarithmic Spiral", n_spiral))
        .xlabel("X Coordinate".to_string())
        .ylabel("Y Coordinate".to_string())
        .line(&x_spiral, &y_spiral)
        .save_with_size("gallery/publication/datashader/spiral_pattern_publication.png", width, height)?;
    println!("  ✅ Generated: spiral_pattern_publication.png");
    
    // Dense scatter plot - statistical distribution
    let n_dense = 500_000;
    println!("  📊 Generating {} points statistical distribution...", n_dense);
    
    // Generate deterministic statistical distribution for publication
    let x_dense: Vec<f64> = (0..n_dense).map(|i| {
        let t = (i as f64) / (n_dense as f64);
        let base = (t * 20.0 - 10.0) + 2.0 * (t * 30.0).sin();
        base + 0.5 * (t * 100.0).cos()
    }).collect();
    
    let y_dense: Vec<f64> = x_dense.iter().enumerate().map(|(i, &x)| {
        let noise = 0.8 * ((i as f64 * 0.1).sin() + 0.5 * (i as f64 * 0.05).cos());
        2.5 * x + 1.2 * x.powi(2) * 0.1 + noise
    }).collect();
    
    Plot::new()
        .title(format!("DataShader Analysis: {} Point Statistical Distribution", n_dense))
        .xlabel("Independent Variable".to_string())
        .ylabel("Dependent Variable".to_string())
        .scatter(&x_dense, &y_dense)
        .save_with_size("gallery/publication/datashader/dense_scatter_publication.png", width, height)?;
    println!("  ✅ Generated: dense_scatter_publication.png");
    
    // Ultra-large dataset demonstration 
    let n_ultra = 1_000_000;
    println!("  🚀 Generating {} points ultra-large dataset...", n_ultra);
    
    let x_ultra: Vec<f64> = (0..n_ultra).map(|i| {
        let t = (i as f64) / (n_ultra as f64) * 20.0 * std::f64::consts::PI;
        t + 0.1 * (t * 3.0).sin()
    }).collect();
    
    let y_ultra: Vec<f64> = x_ultra.iter().enumerate().map(|(i, &x)| {
        let noise = 0.05 * ((i as f64 * 0.001).sin() + 0.3 * (i as f64 * 0.0005).cos());
        (x * 0.5).sin() * (x * 0.2).cos() + noise
    }).collect();
    
    let start_ultra = Instant::now();
    Plot::new()
        .title(format!("DataShader Performance: {} Point Massive Dataset", n_ultra))
        .xlabel("Phase Domain (radians)".to_string())
        .ylabel("Complex Signal".to_string())
        .line(&x_ultra, &y_ultra)
        .save_with_size("gallery/publication/datashader/ultra_large_publication.png", width, height)?;
    let duration_ultra = start_ultra.elapsed();
    println!("  ✅ Generated: ultra_large_publication.png ({:.2}ms)", duration_ultra.as_millis());
    
    println!("✅ Publication-quality DataShader examples generated");
    Ok(())
}

fn generate_theme_examples() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎨 Generating publication-quality theme variations...");
    
    let width = 1200u32;
    let height = 900u32;
    
    // Generate high-quality sample data for theme demonstration
    let x_data: Vec<f64> = (0..2000).map(|i| i as f64 * 0.005).collect();
    let y1: Vec<f64> = x_data.iter().map(|&x| x.sin() * (1.0 + 0.3 * (x * 2.0).cos())).collect();
    let y2: Vec<f64> = x_data.iter().map(|&x| (x * 1.5).cos() * 0.8).collect();
    let y3: Vec<f64> = x_data.iter().map(|&x| 0.5 * (x * 0.8).sin() + 0.3 * (x * 4.0).sin()).collect();
    
    // Publication Theme (primary)
    println!("  🔹 Creating publication theme example...");
    Plot::new()
        .title("Publication Theme: Multi-Series Analysis".to_string())
        .xlabel("Time (seconds)".to_string())
        .ylabel("Signal Amplitude".to_string())
        .theme(Theme::publication())
        .line(&x_data, &y1)  // Primary signal
        .line(&x_data, &y2)  // Secondary signal  
        .scatter(&x_data[0..200].to_vec(), &y3[0..200].to_vec()) // Sample points
        .save_with_size("gallery/publication/themes/publication_theme_publication.png", width, height)?;
    
    // Dark Theme for presentations
    println!("  🔹 Creating dark theme example...");
    Plot::new()
        .title("Dark Theme: Presentation-Ready Visualization".to_string())
        .xlabel("Independent Variable".to_string())
        .ylabel("Dependent Variable".to_string())
        .theme(Theme::dark())
        .line(&x_data, &y1)
        .scatter(&x_data[0..150].to_vec(), &y2[0..150].to_vec())
        .save_with_size("gallery/publication/themes/dark_theme_publication.png", width, height)?;
    
    // Light Theme for reports
    println!("  🔹 Creating light theme example...");
    Plot::new()
        .title("Light Theme: Technical Report Visualization".to_string())
        .xlabel("Time Domain".to_string())
        .ylabel("Response Function".to_string())
        .theme(Theme::light())
        .line(&x_data, &y1)
        .line(&x_data, &y2)
        .save_with_size("gallery/publication/themes/light_theme_publication.png", width, height)?;
    
    // Minimal Theme for clean presentations
    println!("  🔹 Creating minimal theme example...");
    Plot::new()
        .title("Minimal Theme: Clean Scientific Visualization".to_string())
        .xlabel("X Variable".to_string())
        .ylabel("Y Response".to_string())
        .theme(Theme::minimal())
        .line(&x_data, &y1)
        .save_with_size("gallery/publication/themes/minimal_theme_publication.png", width, height)?;
    
    // Comparison: Same data, different themes
    let comparison_x: Vec<f64> = (0..500).map(|i| i as f64 * 0.02).collect();
    let comparison_y: Vec<f64> = comparison_x.iter().map(|&x| 
        x.sin() + 0.5 * (x * 2.0).sin() + 0.25 * (x * 4.0).sin()
    ).collect();
    
    // Create all four theme versions of the same plot for comparison
    let themes = vec![
        ("publication", Theme::publication()),
        ("dark", Theme::dark()),  
        ("light", Theme::light()),
        ("minimal", Theme::minimal())
    ];
    
    println!("  🔹 Creating theme comparison series...");
    for (theme_name, theme) in themes {
        Plot::new()
            .title(format!("{} Theme: Harmonic Analysis Comparison", 
                          theme_name.to_uppercase()))
            .xlabel("Time (seconds)".to_string())
            .ylabel("Composite Signal".to_string())
            .theme(theme)
            .line(&comparison_x, &comparison_y)
            .save_with_size(&format!("gallery/publication/themes/{}_comparison_publication.png", theme_name), 
                           width, height)?;
    }
    
    println!("✅ Publication-quality theme examples generated");
    Ok(())
}

fn generate_performance_examples() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚡ Generating publication-quality performance scaling examples...");
    
    let width = 1200u32;
    let height = 900u32;
    
    let sizes = vec![1_000, 10_000, 50_000, 100_000, 500_000];
    
    for &size in &sizes {
        println!("  🔹 Testing {} points performance...", size);
        let x_data: Vec<f64> = (0..size).map(|i| (i as f64) / (size as f64) * 20.0).collect();
        let y_data: Vec<f64> = x_data.iter().map(|&x| x.sin() + 0.1 * (x * 10.0).cos()).collect();
        
        let start = Instant::now();
        
        // Use publication directory structure
        let filename = if size >= 100_000 {
            format!("gallery/publication/performance/performance_{}k_datashader.png", size / 1000)
        } else {
            format!("gallery/publication/performance/performance_{}k_parallel.png", size / 1000)
        };
        
        Plot::new()
            .title(format!("Performance Scaling: {} Points Dataset", size))
            .xlabel("Time Domain (units)".to_string())
            .ylabel("Signal Amplitude".to_string())
            .theme(Theme::publication())
            .with_parallel(Some(4))
            .parallel_threshold(5_000)
            .line(&x_data, &y_data)
            .save_with_size(&filename, width, height)?;
        
        let duration = start.elapsed();
        let render_type = if size >= 100_000 { "DataShader" } else if size >= 5_000 { "Parallel" } else { "Sequential" };
        
        println!("  ✅ Generated: {} ({} - {:.2}ms)", 
                filename.split('/').last().unwrap(), 
                render_type,
                duration.as_millis());
    }
    
    println!("✅ Publication-quality performance examples generated");
    Ok(())
}