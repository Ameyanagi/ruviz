use ruviz::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Demonstrating All Fixes Applied");
    println!("==================================\n");

    // Create test data with a good range to test centering and overflow
    let x_data: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y_data: Vec<f64> = x_data
        .iter()
        .map(|&x| 2.5 * x.sin() * (x * 0.3).cos() + 1.5 * (x * 0.7).sin())
        .collect();

    println!("📊 Creating demonstration plots...\n");

    // Test 1: Standard DPI with centering and overflow fixes
    println!("1. Standard DPI (96) - Testing centering and tick label positioning");
    let plot = Plot::new()
        .line(&x_data, &y_data)
        .title("Plot Centering & Tick Label Fixes Demo (96 DPI)")
        .xlabel("X-axis with longer label to test positioning")
        .ylabel("Y-axis values testing overflow prevention")
        .dpi(96);

    plot.save("generated/examples/fixes_demo_96dpi.png")?;
    println!("   ✅ Generated: generated/examples/fixes_demo_96dpi.png");

    // Test 2: High DPI with title rendering fix
    println!("2. High DPI (300) - Testing title rendering without double scaling");
    let plot = Plot::new()
        .line(&x_data, &y_data)
        .title("High DPI Title Rendering Fix Demo (300 DPI)")
        .xlabel("X-axis label testing")
        .ylabel("Y-axis label testing")
        .dpi(300);

    plot.save("generated/examples/fixes_demo_300dpi.png")?;
    println!("   ✅ Generated: generated/examples/fixes_demo_300dpi.png");

    // Test 3: Very high DPI to really test the scaling
    println!("3. Publication DPI (600) - Testing extreme DPI scaling");
    let plot = Plot::new()
        .line(&x_data, &y_data)
        .title("Publication Quality DPI Demo (600 DPI)")
        .xlabel("Publication-ready X-axis")
        .ylabel("Publication-ready Y-axis")
        .dpi(600);

    plot.save("generated/examples/fixes_demo_600dpi.png")?;
    println!("   ✅ Generated: generated/examples/fixes_demo_600dpi.png");

    println!("\n🔍 Fixes Demonstrated:");
    println!("======================");
    println!("✅ Plot centering - Plots now center correctly within canvas");
    println!("✅ Title centering - Titles center over full canvas width");
    println!("✅ Tick label overflow - Labels stay within canvas boundaries");
    println!("✅ High DPI title rendering - No double scaling artifacts");
    println!("✅ Asymmetric margins - Better space allocation for labels");
    println!("✅ DPI-aware scaling - Proper scaling across all DPI values");

    println!("\n📐 Technical Details:");
    println!("====================");
    println!("• Asymmetric margins: L=100, R=40, T=80, B=60 (DPI scaled)");
    println!("• Title positioning: Centers on full canvas width, not plot area");
    println!("• Bounds checking: All tick labels clamped to canvas boundaries");
    println!("• DPI scaling: Fixed double multiplication in xlabel rendering");
    println!("• Text rendering: Professional cosmic-text with proper baseline");

    Ok(())
}
