use ruviz::prelude::*;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎨 Testing Professional Histogram and Bar Plot Styling");
    println!("=====================================================");
    
    // Create test output directory
    fs::create_dir_all("test_output")?;
    println!("📁 Created test_output/ directory");
    
    // Test 1: Professional Histogram
    println!("\n📊 Test 1: Professional Histogram Styling");
    test_professional_histogram()?;
    
    // Test 2: Professional Bar Plot  
    println!("\n📊 Test 2: Professional Bar Plot Styling");
    test_professional_bar_plot()?;
    
    // Test 3: xlim/ylim functionality
    println!("\n📊 Test 3: Manual Axis Limits (xlim/ylim)");
    test_axis_limits()?;
    
    println!("\n✅ All tests completed! Check test_output/ directory for results:");
    println!("   - test_output/professional_histogram.png");
    println!("   - test_output/professional_bar_plot.png"); 
    println!("   - test_output/manual_axis_limits.png");
    
    println!("\n🎯 Key Improvements:");
    println!("   ✓ 85% transparency for better visual appeal");
    println!("   ✓ Dark borders for professional definition");
    println!("   ✓ Anti-aliased rendering for smooth edges");
    println!("   ✓ Manual axis control with xlim/ylim");
    
    Ok(())
}

fn test_professional_histogram() -> Result<(), Box<dyn std::error::Error>> {
    // Generate realistic histogram data
    let data = vec![
        1.2, 1.5, 1.8, 2.1, 2.3, 2.7, 2.9, 3.1, 3.4, 3.6,
        3.8, 4.0, 4.2, 4.5, 4.7, 4.9, 5.1, 5.3, 5.6, 5.8,
        6.0, 6.2, 6.5, 6.7, 6.9, 7.1, 7.4, 7.6, 7.8, 8.0,
        8.2, 8.5, 8.7, 8.9, 9.1, 9.4, 9.6, 9.8, 10.0, 10.2,
        10.5, 10.7, 10.9, 11.1, 11.4, 11.6, 11.8, 12.0
    ];

    Plot::new()
        .dimensions(1000, 700)
        .title("Professional Histogram - Improved Styling")
        .xlabel("Value Bins")
        .ylabel("Frequency")
        .histogram(&data, None)
        .end_series()
        .theme(Theme::publication())
        .save("test_output/professional_histogram.png")?;
    
    println!("   ✅ Professional histogram saved");
    Ok(())
}

fn test_professional_bar_plot() -> Result<(), Box<dyn std::error::Error>> {
    let categories = vec!["Product A", "Product B", "Product C", "Product D", "Product E"];
    let sales = vec![85.0, 120.0, 95.0, 140.0, 110.0];
    
    Plot::new()
        .dimensions(1000, 700)
        .title("Professional Bar Plot - Improved Styling")
        .xlabel("Products")
        .ylabel("Sales (thousands)")
        .bar(&categories, &sales)
        .end_series()
        .theme(Theme::publication())
        .save("test_output/professional_bar_plot.png")?;
    
    println!("   ✅ Professional bar plot saved");
    Ok(())
}

fn test_axis_limits() -> Result<(), Box<dyn std::error::Error>> {
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![2.0, 8.0, 18.0, 32.0, 50.0];
    
    Plot::new()
        .dimensions(1000, 700)
        .title("Manual Axis Limits - xlim/ylim Testing")
        .xlabel("X Values")
        .ylabel("Y Values")
        .xlim(0.0, 6.0)   // Manual X-axis limits
        .ylim(0.0, 60.0)  // Manual Y-axis limits
        .line(&x, &y)
            .label("Quadratic Growth")
        .end_series()
        .legend(Position::TopLeft)
        .theme(Theme::publication())
        .save("test_output/manual_axis_limits.png")?;
    
    println!("   ✅ Manual axis limits test saved");
    Ok(())
}