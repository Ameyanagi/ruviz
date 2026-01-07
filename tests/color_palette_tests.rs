use ruviz::core::plot::Plot;
use ruviz::render::color::{Color, ColorMap};

#[test]
fn test_default_color_palette() {
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0];

    // Test multiple series with automatic color cycling
    let result = Plot::new()
        .dimensions(800, 600)
        .title("Default Color Palette Test")
        .xlabel("X Values")
        .ylabel("Y Values")
        .line(&x_data, &vec![1.0, 2.0, 3.0, 4.0, 5.0])
        .label("Series 1")
        .line(&x_data, &vec![2.0, 3.0, 4.0, 5.0, 6.0])
        .label("Series 2")
        .line(&x_data, &vec![3.0, 4.0, 5.0, 6.0, 7.0])
        .label("Series 3")
        .line(&x_data, &vec![4.0, 5.0, 6.0, 7.0, 8.0])
        .label("Series 4")
        .line(&x_data, &vec![5.0, 6.0, 7.0, 8.0, 9.0])
        .label("Series 5")
        .legend(ruviz::core::position::Position::TopRight)
        .save("tests/output/default_palette.png");

    assert!(result.is_ok(), "Default palette should work");
}

#[test]
fn test_custom_color_sequence() {
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Custom Color Sequence")
        .xlabel("X Values")
        .ylabel("Y Values")
        .line(&x_data, &vec![1.0, 2.0, 3.0, 4.0, 5.0])
        .color(Color::RED)
        .label("Red Line")
        .line(&x_data, &vec![2.0, 3.0, 4.0, 5.0, 6.0])
        .color(Color::from_hex("#00FF00").unwrap()) // Green
        .label("Green Line")
        .line(&x_data, &vec![3.0, 4.0, 5.0, 6.0, 7.0])
        .color(Color::from_hex("#0080FF").unwrap()) // Blue
        .label("Blue Line")
        .legend(ruviz::core::position::Position::TopRight)
        .save("tests/output/custom_colors.png");

    assert!(result.is_ok(), "Custom colors should work");
}

#[test]
fn test_scatter_with_different_colors() {
    let x_data1 = vec![1.0, 2.0, 3.0, 4.0];
    let y_data1 = vec![1.0, 4.0, 2.0, 5.0];

    let x_data2 = vec![1.5, 2.5, 3.5, 4.5];
    let y_data2 = vec![2.0, 3.0, 4.0, 3.0];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Scatter Plot with Different Colors")
        .xlabel("X Position")
        .ylabel("Y Position")
        .scatter(&x_data1, &y_data1)
        .color(Color::from_palette(0)) // First palette color
        .marker_size(12.0)
        .label("Group A")
        .scatter(&x_data2, &y_data2)
        .color(Color::from_palette(1)) // Second palette color
        .marker_size(12.0)
        .label("Group B")
        .legend(ruviz::core::position::Position::TopRight)
        .save("tests/output/scatter_colors.png");

    assert!(result.is_ok(), "Scatter with colors should work");
}

#[test]
fn test_transparency_effects() {
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let y1_data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let y2_data = vec![2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    let y3_data = vec![1.5, 2.5, 3.5, 4.5, 5.5, 6.5];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Transparency Effects")
        .xlabel("X Values")
        .ylabel("Y Values")
        .line(&x_data, &y1_data)
        .color(Color::RED.with_alpha(1.0)) // Opaque
        .line_width(3.0)
        .label("Opaque Red")
        .line(&x_data, &y2_data)
        .color(Color::BLUE.with_alpha(0.7)) // Semi-transparent
        .line_width(3.0)
        .label("70% Blue")
        .line(&x_data, &y3_data)
        .color(Color::GREEN.with_alpha(0.4)) // Very transparent
        .line_width(3.0)
        .label("40% Green")
        .legend(ruviz::core::position::Position::TopRight)
        .save("tests/output/transparency.png");

    assert!(result.is_ok(), "Transparency effects should work");
}

#[test]
fn test_colormap_visualization() {
    // Create a simple heatmap-like visualization using colormaps
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let colormap = ColorMap::viridis();

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Viridis Colormap Demonstration")
        .xlabel("X Values")
        .ylabel("Y Values")
        // Plot multiple lines with colors sampled from the colormap
        .line(
            &x_data,
            &x_data.iter().map(|&x| 1.0 + x * 0.5).collect::<Vec<_>>(),
        )
        .color(colormap.sample(0.0))
        .line_width(3.0)
        .label("Line 1 (t=0.0)")
        .line(
            &x_data,
            &x_data.iter().map(|&x| 2.0 + x * 0.5).collect::<Vec<_>>(),
        )
        .color(colormap.sample(0.25))
        .line_width(3.0)
        .label("Line 2 (t=0.3)")
        .line(
            &x_data,
            &x_data.iter().map(|&x| 3.0 + x * 0.5).collect::<Vec<_>>(),
        )
        .color(colormap.sample(0.5))
        .line_width(3.0)
        .label("Line 3 (t=0.5)")
        .line(
            &x_data,
            &x_data.iter().map(|&x| 4.0 + x * 0.5).collect::<Vec<_>>(),
        )
        .color(colormap.sample(0.75))
        .line_width(3.0)
        .label("Line 4 (t=0.8)")
        .line(
            &x_data,
            &x_data.iter().map(|&x| 5.0 + x * 0.5).collect::<Vec<_>>(),
        )
        .color(colormap.sample(1.0))
        .line_width(3.0)
        .label("Line 5 (t=1.0)")
        .legend(ruviz::core::position::Position::TopRight)
        .save("tests/output/colormap_viridis.png");

    assert!(result.is_ok(), "Colormap visualization should work");
}

#[test]
fn test_different_colormaps() {
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let colormaps = vec![
        ("plasma", ColorMap::plasma()),
        ("inferno", ColorMap::inferno()),
        ("hot", ColorMap::hot()),
        ("cool", ColorMap::cool()),
    ];

    for (name, colormap) in colormaps {
        let result = Plot::new()
            .dimensions(800, 600)
            .title(&format!("{} Colormap", name.to_uppercase()))
            .xlabel("X Values")
            .ylabel("Y Values")
            // Plot lines with colors from the colormap
            .line(
                &x_data,
                &x_data.iter().map(|&x| 1.0 + x * 0.3).collect::<Vec<_>>(),
            )
            .color(colormap.sample(0.0))
            .line_width(2.0)
            .label("t=0.00")
            .line(
                &x_data,
                &x_data.iter().map(|&x| 2.0 + x * 0.3).collect::<Vec<_>>(),
            )
            .color(colormap.sample(0.25))
            .line_width(2.0)
            .label("t=0.25")
            .line(
                &x_data,
                &x_data.iter().map(|&x| 3.0 + x * 0.3).collect::<Vec<_>>(),
            )
            .color(colormap.sample(0.5))
            .line_width(2.0)
            .label("t=0.50")
            .line(
                &x_data,
                &x_data.iter().map(|&x| 4.0 + x * 0.3).collect::<Vec<_>>(),
            )
            .color(colormap.sample(0.75))
            .line_width(2.0)
            .label("t=0.75")
            .line(
                &x_data,
                &x_data.iter().map(|&x| 5.0 + x * 0.3).collect::<Vec<_>>(),
            )
            .color(colormap.sample(1.0))
            .line_width(2.0)
            .label("t=1.00")
            .legend(ruviz::core::position::Position::TopRight)
            .save(&format!("tests/output/colormap_{}.png", name));

        assert!(result.is_ok(), "{} colormap should work", name);
    }
}

#[test]
fn test_hex_color_parsing() {
    let x_data = vec![0.0, 1.0, 2.0, 3.0];
    let y_data = vec![1.0, 2.0, 3.0, 4.0];

    // Test various hex formats
    let hex_colors = vec![
        "#FF0000", // Red
        "#00ff00", // Green (lowercase)
        "#0080FF", // Blue
        "#ff8040", // Orange
        "#8040ff", // Purple
    ];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Hex Color Parsing Test")
        .xlabel("X Values")
        .ylabel("Y Values")
        .line(
            &x_data,
            &y_data.iter().map(|&y| y + 0.0 * 0.5).collect::<Vec<_>>(),
        )
        .color(Color::from_hex("#FF0000").unwrap())
        .line_width(2.0)
        .label("#FF0000")
        .line(
            &x_data,
            &y_data.iter().map(|&y| y + 1.0 * 0.5).collect::<Vec<_>>(),
        )
        .color(Color::from_hex("#00ff00").unwrap())
        .line_width(2.0)
        .label("#00ff00")
        .line(
            &x_data,
            &y_data.iter().map(|&y| y + 2.0 * 0.5).collect::<Vec<_>>(),
        )
        .color(Color::from_hex("#0080FF").unwrap())
        .line_width(2.0)
        .label("#0080FF")
        .line(
            &x_data,
            &y_data.iter().map(|&y| y + 3.0 * 0.5).collect::<Vec<_>>(),
        )
        .color(Color::from_hex("#ff8040").unwrap())
        .line_width(2.0)
        .label("#ff8040")
        .line(
            &x_data,
            &y_data.iter().map(|&y| y + 4.0 * 0.5).collect::<Vec<_>>(),
        )
        .color(Color::from_hex("#8040ff").unwrap())
        .line_width(2.0)
        .label("#8040ff")
        .legend(ruviz::core::position::Position::TopRight)
        .save("tests/output/hex_colors.png");

    assert!(result.is_ok(), "Hex color parsing should work");
}

#[test]
fn test_color_palette_cycling() {
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0];

    // Test that colors cycle correctly when we have more series than palette colors
    let result = Plot::new()
        .dimensions(800, 600)
        .title("Color Palette Cycling Test (12 series)")
        .xlabel("X Values")
        .ylabel("Y Values")
        // Create 12 series to test cycling (default palette has 10 colors)
        .line(
            &x_data,
            &x_data.iter().map(|&x| 1.0 + x * 0.1).collect::<Vec<_>>(),
        )
        .color(Color::from_palette(0))
        .line_width(1.5)
        .label("Series 1")
        .line(
            &x_data,
            &x_data.iter().map(|&x| 2.0 + x * 0.1).collect::<Vec<_>>(),
        )
        .color(Color::from_palette(1))
        .line_width(1.5)
        .label("Series 2")
        .line(
            &x_data,
            &x_data.iter().map(|&x| 3.0 + x * 0.1).collect::<Vec<_>>(),
        )
        .color(Color::from_palette(2))
        .line_width(1.5)
        .label("Series 3")
        .line(
            &x_data,
            &x_data.iter().map(|&x| 4.0 + x * 0.1).collect::<Vec<_>>(),
        )
        .color(Color::from_palette(3))
        .line_width(1.5)
        .label("Series 4")
        .line(
            &x_data,
            &x_data.iter().map(|&x| 5.0 + x * 0.1).collect::<Vec<_>>(),
        )
        .color(Color::from_palette(4))
        .line_width(1.5)
        .label("Series 5")
        .line(
            &x_data,
            &x_data.iter().map(|&x| 6.0 + x * 0.1).collect::<Vec<_>>(),
        )
        .color(Color::from_palette(5))
        .line_width(1.5)
        .label("Series 6")
        .line(
            &x_data,
            &x_data.iter().map(|&x| 7.0 + x * 0.1).collect::<Vec<_>>(),
        )
        .color(Color::from_palette(6))
        .line_width(1.5)
        .label("Series 7")
        .line(
            &x_data,
            &x_data.iter().map(|&x| 8.0 + x * 0.1).collect::<Vec<_>>(),
        )
        .color(Color::from_palette(7))
        .line_width(1.5)
        .label("Series 8")
        .line(
            &x_data,
            &x_data.iter().map(|&x| 9.0 + x * 0.1).collect::<Vec<_>>(),
        )
        .color(Color::from_palette(8))
        .line_width(1.5)
        .label("Series 9")
        .line(
            &x_data,
            &x_data.iter().map(|&x| 10.0 + x * 0.1).collect::<Vec<_>>(),
        )
        .color(Color::from_palette(9))
        .line_width(1.5)
        .label("Series 10")
        .line(
            &x_data,
            &x_data.iter().map(|&x| 11.0 + x * 0.1).collect::<Vec<_>>(),
        )
        .color(Color::from_palette(10)) // Should cycle back to palette[0]
        .line_width(1.5)
        .label("Series 11")
        .line(
            &x_data,
            &x_data.iter().map(|&x| 12.0 + x * 0.1).collect::<Vec<_>>(),
        )
        .color(Color::from_palette(11)) // Should cycle to palette[1]
        .line_width(1.5)
        .label("Series 12")
        .legend(ruviz::core::position::Position::TopRight)
        .save("tests/output/palette_cycling.png");

    assert!(result.is_ok(), "Palette cycling should work");
}

#[test]
fn test_scientific_colormaps() {
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0];

    // Test scientific colormaps (perceptually uniform)
    let scientific_maps = vec![
        ("viridis", ColorMap::viridis()),
        ("plasma", ColorMap::plasma()),
        ("inferno", ColorMap::inferno()),
        ("magma", ColorMap::magma()),
    ];

    for (name, colormap) in scientific_maps {
        let result = Plot::new()
            .dimensions(800, 600)
            .title(&format!("Scientific Colormap: {}", name.to_uppercase()))
            .xlabel("Data Points")
            .ylabel("Value")
            // Create a gradient effect
            .line(
                &x_data,
                &x_data.iter().map(|&x| 0.0 + x * 0.2).collect::<Vec<_>>(),
            )
            .color(colormap.sample(0.0))
            .line_width(3.0)
            .label("0.0")
            .line(
                &x_data,
                &x_data.iter().map(|&x| 1.0 + x * 0.2).collect::<Vec<_>>(),
            )
            .color(colormap.sample(0.111))
            .line_width(3.0)
            .label("0.1")
            .line(
                &x_data,
                &x_data.iter().map(|&x| 2.0 + x * 0.2).collect::<Vec<_>>(),
            )
            .color(colormap.sample(0.222))
            .line_width(3.0)
            .label("0.2")
            .line(
                &x_data,
                &x_data.iter().map(|&x| 3.0 + x * 0.2).collect::<Vec<_>>(),
            )
            .color(colormap.sample(0.333))
            .line_width(3.0)
            .label("0.3")
            .line(
                &x_data,
                &x_data.iter().map(|&x| 4.0 + x * 0.2).collect::<Vec<_>>(),
            )
            .color(colormap.sample(0.444))
            .line_width(3.0)
            .label("0.4")
            .line(
                &x_data,
                &x_data.iter().map(|&x| 5.0 + x * 0.2).collect::<Vec<_>>(),
            )
            .color(colormap.sample(0.556))
            .line_width(3.0)
            .label("0.6")
            .line(
                &x_data,
                &x_data.iter().map(|&x| 6.0 + x * 0.2).collect::<Vec<_>>(),
            )
            .color(colormap.sample(0.667))
            .line_width(3.0)
            .label("0.7")
            .line(
                &x_data,
                &x_data.iter().map(|&x| 7.0 + x * 0.2).collect::<Vec<_>>(),
            )
            .color(colormap.sample(0.778))
            .line_width(3.0)
            .label("0.8")
            .line(
                &x_data,
                &x_data.iter().map(|&x| 8.0 + x * 0.2).collect::<Vec<_>>(),
            )
            .color(colormap.sample(0.889))
            .line_width(3.0)
            .label("0.9")
            .line(
                &x_data,
                &x_data.iter().map(|&x| 9.0 + x * 0.2).collect::<Vec<_>>(),
            )
            .color(colormap.sample(1.0))
            .line_width(3.0)
            .label("1.0")
            .legend(ruviz::core::position::Position::TopRight)
            .save(&format!("tests/output/scientific_{}.png", name));

        assert!(result.is_ok(), "Scientific colormap {} should work", name);
    }
}
