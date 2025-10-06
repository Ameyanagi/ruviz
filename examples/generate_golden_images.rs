// Golden image generator for visual regression testing
// This program generates a comprehensive set of reference images
// that serve as the baseline for visual regression tests

use ruviz::prelude::*;

fn main() -> Result<()> {
    println!("🎨 Generating golden images for visual regression testing...\n");

    // Create output directory
    std::fs::create_dir_all("tests/golden_images")?;

    let mut count = 0;

    // 1. Basic line plot
    println!("[{}/25] Generating basic line plot...", count + 1);
    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];
    Plot::new()
        .line(&x, &y)
        .title("Basic Line Plot")
        .xlabel("X")
        .ylabel("Y")
        .save("tests/golden_images/01_basic_line.png")?;
    count += 1;

    // 2. Multi-series line plot
    println!("[{}/25] Generating multi-series plot...", count + 1);
    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    Plot::new()
        .line(&x, &x.iter().map(|&v| v).collect::<Vec<_>>())
        .label("Linear")
        .line(&x, &x.iter().map(|&v| v * v).collect::<Vec<_>>())
        .label("Quadratic")
        .line(&x, &x.iter().map(|&v| v.powi(3)).collect::<Vec<_>>())
        .label("Cubic")
        .title("Multi-Series Plot")
        .xlabel("X")
        .ylabel("Y")
        .legend(Position::TopLeft)
        .save("tests/golden_images/02_multi_series.png")?;
    count += 1;

    // 3. Scatter plot
    println!("[{}/25] Generating scatter plot...", count + 1);
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![2.3, 3.1, 2.8, 4.2, 3.9];
    Plot::new()
        .scatter(&x, &y)
        .marker(MarkerStyle::Circle)
        .marker_size(8.0)
        .title("Scatter Plot")
        .xlabel("X")
        .ylabel("Y")
        .save("tests/golden_images/03_scatter.png")?;
    count += 1;

    // 4. Bar chart
    println!("[{}/25] Generating bar chart...", count + 1);
    let categories = vec!["A", "B", "C", "D", "E"];
    let values = vec![25.0, 40.0, 30.0, 55.0, 45.0];
    Plot::new()
        .bar(&categories, &values)
        .title("Bar Chart")
        .ylabel("Value")
        .save("tests/golden_images/04_bar_chart.png")?;
    count += 1;

    // 5. Histogram
    println!("[{}/25] Generating histogram...", count + 1);
    let data = vec![
        1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0, 4.0, 5.0,
        1.5, 2.5, 2.5, 3.5, 3.5, 3.5, 4.5, 4.5, 5.5,
    ];
    Plot::new()
        .histogram(&data, None)
        .title("Histogram")
        .xlabel("Value")
        .ylabel("Frequency")
        .save("tests/golden_images/05_histogram.png")?;
    count += 1;

    // 6. Box plot
    println!("[{}/25] Generating box plot...", count + 1);
    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 25.0];
    Plot::new()
        .boxplot(&data, None)
        .title("Box Plot")
        .ylabel("Value")
        .save("tests/golden_images/06_boxplot.png")?;
    count += 1;

    // 7-10. Theme variations
    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    for (theme, name) in [
        (Theme::light(), "light"),
        (Theme::dark(), "dark"),
        (Theme::publication(), "publication"),
        (Theme::seaborn(), "seaborn"),
    ] {
        println!("[{}/25] Generating {} theme...", count + 1, name);
        Plot::new()
            .theme(theme)
            .line(&x, &y)
            .title(&format!("{} Theme", name.to_uppercase()))
            .xlabel("X")
            .ylabel("Y")
            .save(&format!("tests/golden_images/0{}_theme_{}.png", count + 1, name))?;
        count += 1;
    }

    // 11-13. DPI variations
    for dpi in [72, 150, 300] {
        println!("[{}/25] Generating {} DPI image...", count + 1, dpi);
        Plot::new()
            .line(&x, &y)
            .dpi(dpi)
            .title(&format!("{} DPI", dpi))
            .save(&format!("tests/golden_images/{}_dpi_{}.png", count + 1, dpi))?;
        count += 1;
    }

    // 14. Custom dimensions
    println!("[{}/25] Generating custom dimensions...", count + 1);
    Plot::new()
        .dimensions(1200, 900)
        .line(&x, &y)
        .title("Custom Dimensions")
        .save("tests/golden_images/14_custom_dimensions.png")?;
    count += 1;

    // 15. Subplots
    println!("[{}/25] Generating subplots...", count + 1);
    let plot1 = Plot::new()
        .line(&x, &y)
        .title("Linear")
        .end_series();
    let plot2 = Plot::new()
        .scatter(&x, &y)
        .title("Scatter")
        .end_series();
    let cats_sub = vec!["A", "B", "C"];
    let vals_sub = vec![25.0, 40.0, 30.0];
    let plot3 = Plot::new()
        .bar(&cats_sub, &vals_sub)
        .title("Bar")
        .end_series();
    let plot4 = Plot::new()
        .histogram(&data, None)
        .title("Histogram")
        .end_series();

    subplots(2, 2, 1200, 900)?
        .subplot(0, 0, plot1)?
        .subplot(0, 1, plot2)?
        .subplot(1, 0, plot3)?
        .subplot(1, 1, plot4)?
        .suptitle("Subplot Grid")
        .save("tests/golden_images/15_subplots.png")?;
    count += 1;

    // 16. Large dataset (1K points)
    println!("[{}/25] Generating large dataset (1K points)...", count + 1);
    let x_large: Vec<f64> = (0..1000).map(|i| i as f64 * 0.01).collect();
    let y_large: Vec<f64> = x_large.iter().map(|&x| x.sin()).collect();
    Plot::new()
        .line(&x_large, &y_large)
        .title("1K Points")
        .save("tests/golden_images/16_large_1k.png")?;
    count += 1;

    // 17. Scientific notation axes
    println!("[{}/25] Generating scientific notation...", count + 1);
    let x_sci: Vec<f64> = (0..50).map(|i| i as f64 * 100.0).collect();
    let y_sci: Vec<f64> = x_sci.iter().map(|&x| x * x).collect();
    Plot::new()
        .line(&x_sci, &y_sci)
        .title("Scientific Notation")
        .xlabel("X (×100)")
        .ylabel("Y (×10000)")
        .save("tests/golden_images/17_scientific.png")?;
    count += 1;

    // 18. Negative values
    println!("[{}/25] Generating negative values...", count + 1);
    let x_neg = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
    let y_neg = vec![-4.0, -1.0, 0.0, 1.0, 4.0];
    Plot::new()
        .line(&x_neg, &y_neg)
        .title("Negative Values")
        .xlabel("X")
        .ylabel("Y")
        .save("tests/golden_images/18_negative.png")?;
    count += 1;

    // 19. Zero-crossing
    println!("[{}/25] Generating zero-crossing...", count + 1);
    let x_zero: Vec<f64> = (0..100).map(|i| (i as f64 - 50.0) * 0.1).collect();
    let y_zero: Vec<f64> = x_zero.iter().map(|&x| x.sin()).collect();
    Plot::new()
        .line(&x_zero, &y_zero)
        .title("Zero-Crossing")
        .save("tests/golden_images/19_zero_crossing.png")?;
    count += 1;

    // 20. Dense scatter
    println!("[{}/25] Generating dense scatter...", count + 1);
    let x_dense: Vec<f64> = (0..200).map(|i| i as f64 * 0.05).collect();
    let y_dense: Vec<f64> = x_dense.iter().map(|&x| x.sin() + (x * 0.5).cos()).collect();
    Plot::new()
        .scatter(&x_dense, &y_dense)
        .marker(MarkerStyle::Circle)
        .marker_size(2.0)
        .title("Dense Scatter")
        .save("tests/golden_images/20_dense_scatter.png")?;
    count += 1;

    // 21. Wide bar chart
    println!("[{}/25] Generating wide bar chart...", count + 1);
    let cats: Vec<String> = (0..20).map(|i| format!("C{}", i)).collect();
    let vals: Vec<f64> = (0..20).map(|i| (i as f64 * 1.5).sin() * 50.0 + 50.0).collect();
    let cats_str: Vec<&str> = cats.iter().map(|s| s.as_str()).collect();
    Plot::new()
        .bar(&cats_str, &vals)
        .title("Wide Bar Chart")
        .save("tests/golden_images/21_wide_bar.png")?;
    count += 1;

    // 22. Empty plot (minimal)
    println!("[{}/25] Generating minimal plot...", count + 1);
    let x_min = vec![0.0, 1.0];
    let y_min = vec![0.0, 1.0];
    Plot::new()
        .line(&x_min, &y_min)
        .save("tests/golden_images/22_minimal.png")?;
    count += 1;

    // 23. Long title
    println!("[{}/25] Generating long title...", count + 1);
    Plot::new()
        .line(&x, &y)
        .title("This is a Very Long Title That Tests Text Wrapping and Layout Behavior")
        .xlabel("X-axis with longer label")
        .ylabel("Y-axis with longer label")
        .save("tests/golden_images/23_long_title.png")?;
    count += 1;

    // 24. Unicode text
    println!("[{}/25] Generating unicode text...", count + 1);
    Plot::new()
        .line(&x, &y)
        .title("Unicode: α β γ δ ε θ λ π σ ω")
        .xlabel("Température (°C)")
        .ylabel("Résultat")
        .save("tests/golden_images/24_unicode.png")?;
    count += 1;

    // 25. Complex multi-series
    println!("[{}/25] Generating complex multi-series...", count + 1);
    let x_comp: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    Plot::new()
        .line(&x_comp, &x_comp.iter().map(|&x| x.sin()).collect::<Vec<_>>())
        .label("sin(x)")
        .line(&x_comp, &x_comp.iter().map(|&x| x.cos()).collect::<Vec<_>>())
        .label("cos(x)")
        .line(&x_comp, &x_comp.iter().map(|&x| (x * 0.5).sin()).collect::<Vec<_>>())
        .label("sin(x/2)")
        .title("Complex Multi-Series")
        .legend(Position::TopRight)
        .save("tests/golden_images/25_complex_multi.png")?;
    count += 1;

    println!("\n✅ Generated {} golden images successfully!", count);
    println!("📁 Location: tests/golden_images/");

    Ok(())
}
