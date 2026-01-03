use ruviz::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Precise Title Centering Test");
    println!("===============================\n");

    // Create test data
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y_data = vec![0.0, 1.0, 0.0, 1.0, 0.0];

    // Test with different titles to make centering issues obvious
    let test_cases = vec![
        ("A", "A very short title"),
        ("ABC", "ABC three chars"),
        ("CENTERED", "CENTERED test"),
        ("Hello World Test", "Hello World Test medium"),
        (
            "This Is A Very Long Title That Should Be Perfectly Centered",
            "This Is A Very Long Title That Should Be Perfectly Centered long",
        ),
    ];

    for (title, filename) in test_cases {
        println!("📊 Testing title centering: '{}'", title);

        let plot = Plot::new()
            .line(&x_data, &y_data)
            .title(title)
            .xlabel("X axis")
            .ylabel("Y axis")
            .dpi(96);

        let output_filename = format!(
            "precise_center_{}.png",
            filename.replace(" ", "_").replace(",", "").to_lowercase()
        );
        plot.save(&output_filename)?;
        println!("   ✅ Generated: {}", output_filename);
    }

    println!("\n🔍 Analysis Instructions:");
    println!("========================");
    println!("• Look carefully at each image");
    println!("• Check if titles are perfectly centered over the entire canvas");
    println!("• Note any visual asymmetry in spacing");
    println!("• Compare short vs long titles for consistency");

    Ok(())
}
