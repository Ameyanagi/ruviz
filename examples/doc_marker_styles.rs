//! Documentation example: Marker styles
//!
//! Generates docs/images/marker_styles.png for rustdoc

use ruviz::prelude::*;
use ruviz::render::MarkerStyle;

fn main() -> Result<()> {
    // X values for each row of markers
    let x: Vec<f64> = (0..5).map(|j| j as f64 * 2.0).collect();

    // Build plot with all marker styles, each on its own row
    Plot::new()
        .title("Marker Styles")
        .xlabel("x")
        .ylabel("y")
        .dpi(300)
        .legend_position(LegendPosition::Best)
        // Row 11: Circle
        .scatter(&x, &vec![11.0; 5])
        .label("Circle")
        .marker(MarkerStyle::Circle)
        .marker_size(10.0)
        // Row 10: Square
        .scatter(&x, &vec![10.0; 5])
        .label("Square")
        .marker(MarkerStyle::Square)
        .marker_size(10.0)
        // Row 9: Triangle
        .scatter(&x, &vec![9.0; 5])
        .label("Triangle")
        .marker(MarkerStyle::Triangle)
        .marker_size(10.0)
        // Row 8: Diamond
        .scatter(&x, &vec![8.0; 5])
        .label("Diamond")
        .marker(MarkerStyle::Diamond)
        .marker_size(10.0)
        // Row 7: Plus
        .scatter(&x, &vec![7.0; 5])
        .label("Plus")
        .marker(MarkerStyle::Plus)
        .marker_size(10.0)
        // Row 6: Cross
        .scatter(&x, &vec![6.0; 5])
        .label("Cross")
        .marker(MarkerStyle::Cross)
        .marker_size(10.0)
        // Row 5: Star
        .scatter(&x, &vec![5.0; 5])
        .label("Star")
        .marker(MarkerStyle::Star)
        .marker_size(10.0)
        // Row 4: CircleOpen
        .scatter(&x, &vec![4.0; 5])
        .label("CircleOpen")
        .marker(MarkerStyle::CircleOpen)
        .marker_size(10.0)
        // Row 3: SquareOpen
        .scatter(&x, &vec![3.0; 5])
        .label("SquareOpen")
        .marker(MarkerStyle::SquareOpen)
        .marker_size(10.0)
        // Row 2: TriangleOpen
        .scatter(&x, &vec![2.0; 5])
        .label("TriangleOpen")
        .marker(MarkerStyle::TriangleOpen)
        .marker_size(10.0)
        // Row 1: DiamondOpen
        .scatter(&x, &vec![1.0; 5])
        .label("DiamondOpen")
        .marker(MarkerStyle::DiamondOpen)
        .marker_size(10.0)
        .end_series()
        .save("docs/images/marker_styles.png")?;

    println!("✓ Generated docs/images/marker_styles.png");
    Ok(())
}
