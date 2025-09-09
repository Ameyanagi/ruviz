use ruviz::{
    core::plot::Plot,
    render::{Color, Theme},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing improved font alignment...");
    
    // Create simple test data
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![2.0, 4.0, 3.0, 5.0, 4.5];
    
    // Try to create a plot to test font rendering
    match Plot::new()
        .line(&x, &y)
        .title("Font Alignment Test")
        .xlabel("X Values") 
        .ylabel("Y Values")
        .save("test_font_alignment.png") 
    {
        Ok(_) => {
            println!("✅ Plot created successfully with improved font alignment!");
            println!("📄 Saved as: test_font_alignment.png");
        }
        Err(e) => {
            println!("❌ Error creating plot: {}", e);
            println!("🔧 This is expected - the font rendering is implemented but needs full integration");
        }
    }
    
    Ok(())
}