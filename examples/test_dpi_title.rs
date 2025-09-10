use ruviz::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Testing DPI Title Rendering");
    println!("==============================\n");

    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![2.0, 4.0, 6.0, 8.0, 10.0];

    // Test different DPI values for title rendering
    let dpi_values = vec![
        (96, "Standard DPI (96)"),
        (150, "High DPI (150)"),
        (300, "Publication DPI (300)"),
    ];

    for (dpi, description) in &dpi_values {
        println!("📊 Testing DPI: {} - {}", dpi, description);
        
        let filename = format!("dpi_title_test_{}.png", dpi);
        
        // Create plot with current DPI
        let plot = Plot::new()
            .line(&x_data, &y_data)
            .title(&format!("DPI {} Title Test - Should Render Correctly", dpi))
            .xlabel("X Values")
            .ylabel("Y Values")
            .dpi(*dpi);
            
        let result = plot.save(&filename);
        
        result?;
        println!("   ✅ Generated: {}", filename);
    }

    println!("\n🔍 DPI Title Analysis:");
    println!("=======================");
    println!("• Check title scaling at different DPI values");
    println!("• Title should be proportionally larger at higher DPI");
    println!("• Title should remain centered at all DPI values");
    println!("• No double scaling artifacts should be visible");
    
    Ok(())
}