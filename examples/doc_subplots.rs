//! Documentation example: Subplots
//!
//! Generates docs/assets/rustdoc/subplots.png for rustdoc

use ruviz::prelude::*;

fn plot_with_font(font_family: Option<&str>) -> Plot {
    match font_family {
        Some(font_family) => Plot::new().font_family(font_family),
        None => Plot::new(),
    }
}

pub fn build_subplots_figure(font_family: Option<&str>) -> Result<SubplotFigure> {
    let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.2).collect();
    let y_sin: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
    let y_cos: Vec<f64> = x.iter().map(|&v| v.cos()).collect();

    // Subplot 0: Line plot with styling
    let plot_line = plot_with_font(font_family)
        .title("Line Plot")
        .xlabel("x")
        .ylabel("y")
        .line(&x, &y_sin)
        .color(Color::from_palette(0))
        .line_width(2.0);

    // Subplot 1: Scatter plot
    let x_scatter: Vec<f64> = (0..30).map(|i| i as f64 * 0.3).collect();
    let y_scatter: Vec<f64> = x_scatter
        .iter()
        .map(|&v| v.cos() + 0.1 * (v * 3.0).sin())
        .collect();
    let plot_scatter = plot_with_font(font_family)
        .title("Scatter Plot")
        .xlabel("x")
        .ylabel("y")
        .ylim(-1.3, 1.3)
        .scatter(&x_scatter, &y_scatter)
        .color(Color::from_palette(1))
        .marker_size(6.0);

    // Subplot 2: Bar chart
    let categories = vec!["Q1", "Q2", "Q3", "Q4"];
    let values = vec![28.0, 45.0, 38.0, 52.0];
    let plot_bar = plot_with_font(font_family)
        .title("Bar Chart")
        .xlabel("Quarter")
        .ylabel("Sales")
        .ylim(0.0, 60.0)
        .bar(&categories, &values)
        .color(Color::from_palette(2));

    // Subplot 3: Multiple series with legend
    let plot_multi = plot_with_font(font_family)
        .title("Comparison")
        .xlabel("x")
        .ylabel("y")
        .legend_position(LegendPosition::UpperRight)
        .line(&x, &y_sin)
        .label("sin(x)")
        .color(Color::from_palette(0))
        .line(&x, &y_cos)
        .label("cos(x)")
        .color(Color::from_palette(1));

    // Create a 2x2 subplot figure
    let mut figure = subplots(2, 2, 800, 600)?
        .suptitle("Mixed Plots in a 2×2 Grid")
        .subplot_at(0, plot_line.into())?
        .subplot_at(1, plot_scatter.into())?
        .subplot_at(2, plot_bar.into())?
        .subplot_at(3, plot_multi.into())?;
    if let Some(font_family) = font_family {
        let theme = Theme {
            font_family: font_family.to_string(),
            ..Theme::default()
        };
        figure = figure.theme(theme);
    }
    Ok(figure)
}

fn main() -> Result<()> {
    build_subplots_figure(None)?.save("docs/assets/rustdoc/subplots.png")?;

    println!("✓ Generated docs/assets/rustdoc/subplots.png");
    Ok(())
}
