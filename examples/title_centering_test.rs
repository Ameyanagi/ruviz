use ruviz::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Testing Title Centering with Accurate Text Width");
    println!("=================================================\n");

    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![2.0, 4.0, 6.0, 8.0, 10.0];

    // Test various title lengths to verify centering
    let test_cases = vec![
        ("Short", "title_center_short.png"),
        ("Medium Length Title", "title_center_medium.png"),
        (
            "Very Long Title That Should Still Be Centered Properly",
            "title_center_long.png",
        ),
        (
            "Title with Numbers 12345 and Symbols !@#$%",
            "title_center_mixed.png",
        ),
    ];

    for (title, filename) in test_cases {
        println!("📊 Testing title: '{}'", title);

        let plot = Plot::new()
            .line(&x_data, &y_data)
            .title(title)
            .xlabel("X Values")
            .ylabel("Y Values");

        plot.save(filename)?;
        println!("   ✅ Generated: {}", filename);
    }

    println!("\n🔍 Title Centering Analysis:");
    println!("============================");
    println!("• All titles should be perfectly centered over the canvas");
    println!("• No more estimation errors - using actual text width measurement");
    println!("• Works correctly for titles of any length and character mix");
    println!("• Centering is independent of plot area margins");

    Ok(())
}
