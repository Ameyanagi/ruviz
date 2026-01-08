//! Documentation example: International text support
//!
//! Generates docs/images/international_*.png for gallery
//! Demonstrates support for Japanese, Chinese, Korean, and mixed scripts.

use ruviz::prelude::*;

fn main() -> Result<()> {
    // Ensure output directory exists
    std::fs::create_dir_all("docs/images")?;

    // Example 1: Japanese plot
    generate_japanese_plot()?;

    // Example 2: Chinese plot
    generate_chinese_plot()?;

    // Example 3: Korean plot
    generate_korean_plot()?;

    // Example 4: Multi-language comparison
    generate_multilang_comparison()?;

    println!("✓ Generated all international text examples");
    Ok(())
}

/// Generate a plot with Japanese labels
fn generate_japanese_plot() -> Result<()> {
    let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();

    Plot::new()
        .title("サイン波 (Sine Wave)")
        .xlabel("時間 (s)")
        .ylabel("振幅")
        .max_resolution(1920, 1440)
        .line(&x, &y)
        .label("sin(x)")
        .legend_best()
        .save("docs/images/international_japanese.png")?;

    println!("  ✓ Generated docs/images/international_japanese.png");
    Ok(())
}

/// Generate a plot with Chinese labels
fn generate_chinese_plot() -> Result<()> {
    let categories = vec!["一月", "二月", "三月", "四月", "五月", "六月"];
    let values = vec![28.0, 45.0, 38.0, 52.0, 47.0, 63.0];

    Plot::new()
        .title("月度销售数据")
        .xlabel("月份")
        .ylabel("销售额 (万元)")
        .max_resolution(1920, 1440)
        .bar(&categories, &values)
        .label("2024年")
        .legend_best()
        .save("docs/images/international_chinese.png")?;

    println!("  ✓ Generated docs/images/international_chinese.png");
    Ok(())
}

/// Generate a plot with Korean labels
fn generate_korean_plot() -> Result<()> {
    let x: Vec<f64> = (0..50).map(|i| i as f64).collect();
    let y1: Vec<f64> = x.iter().map(|&v| v * 1.5 + 10.0).collect();
    let y2: Vec<f64> = x.iter().map(|&v| v * 2.0 + 5.0).collect();

    Plot::new()
        .title("성장 추이 분석")
        .xlabel("기간 (일)")
        .ylabel("성장률 (%)")
        .max_resolution(1920, 1440)
        .line(&x, &y1)
        .label("A 그룹")
        .line(&x, &y2)
        .label("B 그룹")
        .legend_best()
        .save("docs/images/international_korean.png")?;

    println!("  ✓ Generated docs/images/international_korean.png");
    Ok(())
}

/// Generate a subplot comparing multiple languages
fn generate_multilang_comparison() -> Result<()> {
    let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.2).collect();
    let y_sin: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
    let y_cos: Vec<f64> = x.iter().map(|&v| v.cos()).collect();

    // Japanese plot
    let plot_jp = Plot::new()
        .title("日本語")
        .xlabel("横軸")
        .ylabel("縦軸")
        .legend_position(LegendPosition::UpperRight)
        .line(&x, &y_sin)
        .label("正弦")
        .line(&x, &y_cos)
        .label("余弦");

    // Chinese plot
    let plot_cn = Plot::new()
        .title("中文")
        .xlabel("X轴")
        .ylabel("Y轴")
        .legend_position(LegendPosition::UpperRight)
        .line(&x, &y_sin)
        .label("正弦")
        .line(&x, &y_cos)
        .label("余弦");

    // Korean plot
    let plot_kr = Plot::new()
        .title("한국어")
        .xlabel("X축")
        .ylabel("Y축")
        .legend_position(LegendPosition::UpperRight)
        .line(&x, &y_sin)
        .label("사인")
        .line(&x, &y_cos)
        .label("코사인");

    // Mixed script plot - simplified labels for cleaner rendering
    let plot_mixed = Plot::new()
        .title("Mixed")
        .xlabel("Time")
        .ylabel("Value")
        .legend_position(LegendPosition::UpperRight)
        .line(&x, &y_sin)
        .label("Wave");

    // Create 2x2 subplot
    subplots(2, 2, 1000, 800)?
        .suptitle("International Text Support")
        .subplot_at(0, plot_jp.into())?
        .subplot_at(1, plot_cn.into())?
        .subplot_at(2, plot_kr.into())?
        .subplot_at(3, plot_mixed.into())?
        .save("docs/images/international_comparison.png")?;

    println!("  ✓ Generated docs/images/international_comparison.png");
    Ok(())
}
