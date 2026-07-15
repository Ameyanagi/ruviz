// Golden image generator for visual regression testing

use cosmic_text::fontdb::{Family, Query};
use ruviz::core::PlottingError;
use ruviz::prelude::*;

const GOLDEN_FONT_BYTES: &[u8] = include_bytes!("../src/dejavu-sans.ttf");
const GOLDEN_FONT_FAMILY: &str = "DejaVu Sans";

pub const GOLDEN_FIXTURES: [&str; 25] = [
    "01_basic_line.png",
    "02_multi_series.png",
    "03_scatter.png",
    "04_bar_chart.png",
    "05_histogram.png",
    "06_boxplot.png",
    "07_theme_light.png",
    "08_theme_dark.png",
    "09_theme_publication.png",
    "010_theme_seaborn.png",
    "11_dpi_72.png",
    "12_dpi_150.png",
    "13_dpi_300.png",
    "14_custom_dimensions.png",
    "15_subplots.png",
    "16_large_1k.png",
    "17_scientific.png",
    "18_negative.png",
    "19_zero_crossing.png",
    "20_dense_scatter.png",
    "21_wide_bar.png",
    "22_minimal.png",
    "23_long_title.png",
    "24_unicode.png",
    "25_complex_multi.png",
];

fn stable_plot() -> Plot {
    Plot::new().font_family(GOLDEN_FONT_FAMILY)
}

fn stable_theme(mut theme: Theme) -> Theme {
    theme.font_family = GOLDEN_FONT_FAMILY.to_string();
    theme
}

pub fn register_golden_font() -> Result<()> {
    ruviz::render::register_font_bytes(GOLDEN_FONT_BYTES.to_vec())?;

    let font_system = ruviz::render::get_font_system().lock().map_err(|_| {
        PlottingError::RenderError(
            "cannot verify the deterministic golden font because FontSystem is poisoned"
                .to_string(),
        )
    })?;
    let requested_families = [Family::Name(GOLDEN_FONT_FAMILY)];
    let selected = font_system
        .db()
        .query(&Query {
            families: &requested_families,
            ..Query::default()
        })
        .ok_or_else(|| {
            PlottingError::RenderError(format!(
                "registered golden font family {GOLDEN_FONT_FAMILY:?} cannot be selected"
            ))
        })?;
    let selected_face = font_system.db().face(selected).ok_or_else(|| {
        PlottingError::RenderError(format!(
            "selected golden font family {GOLDEN_FONT_FAMILY:?} has no face metadata"
        ))
    })?;
    if !selected_face
        .families
        .iter()
        .any(|(family, _)| family == GOLDEN_FONT_FAMILY)
    {
        return Err(PlottingError::RenderError(format!(
            "requested golden font family {GOLDEN_FONT_FAMILY:?} resolved to {:?}",
            selected_face.families
        )));
    }

    let selected_embedded_bytes = font_system
        .db()
        .with_face_data(selected, |bytes, face_index| {
            face_index == 0 && bytes == GOLDEN_FONT_BYTES
        })
        .unwrap_or(false);
    if !selected_embedded_bytes {
        return Err(PlottingError::RenderError(format!(
            "golden font family {GOLDEN_FONT_FAMILY:?} did not select the bundled src/dejavu-sans.ttf bytes"
        )));
    }

    Ok(())
}

pub fn generate_golden_images() -> Result<()> {
    register_golden_font()?;
    println!("Generating deterministic golden images with {GOLDEN_FONT_FAMILY}...\n");

    std::fs::create_dir_all("tests/fixtures/golden")?;
    let mut count = 0;

    // 1. Basic line plot
    println!("[{}/25] Basic line plot...", count + 1);
    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];
    stable_plot()
        .title("Basic Line Plot")
        .xlabel("X")
        .ylabel("Y")
        .line(&x, &y)
        .save("tests/fixtures/golden/01_basic_line.png")?;
    count += 1;

    // 2. Multi-series line plot
    println!("[{}/25] Multi-series plot...", count + 1);
    stable_plot()
        .title("Multi-Series Plot")
        .xlabel("X")
        .ylabel("Y")
        .legend(Position::TopLeft)
        .line(&x, &x.to_vec())
        .label("Linear")
        .line(&x, &x.iter().map(|&v| v * v).collect::<Vec<_>>())
        .label("Quadratic")
        .line(&x, &x.iter().map(|&v: &f64| v.powi(3)).collect::<Vec<_>>())
        .label("Cubic")
        .save("tests/fixtures/golden/02_multi_series.png")?;
    count += 1;

    // 3. Scatter plot
    println!("[{}/25] Scatter plot...", count + 1);
    let x_s = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_s = vec![2.3, 3.1, 2.8, 4.2, 3.9];
    stable_plot()
        .title("Scatter Plot")
        .xlabel("X")
        .ylabel("Y")
        .scatter(&x_s, &y_s)
        .marker(MarkerStyle::Circle)
        .marker_size(8.0)
        .save("tests/fixtures/golden/03_scatter.png")?;
    count += 1;

    // 4. Bar chart
    println!("[{}/25] Bar chart...", count + 1);
    let categories = vec!["A", "B", "C", "D", "E"];
    let values = vec![25.0, 40.0, 30.0, 55.0, 45.0];
    stable_plot()
        .title("Bar Chart")
        .ylabel("Value")
        .bar(&categories, &values)
        .save("tests/fixtures/golden/04_bar_chart.png")?;
    count += 1;

    // 5. Histogram
    println!("[{}/25] Histogram...", count + 1);
    let data = vec![
        1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0, 4.0, 5.0, 1.5, 2.5, 2.5, 3.5, 3.5, 3.5, 4.5, 4.5, 5.5,
    ];
    stable_plot()
        .title("Histogram")
        .xlabel("Value")
        .ylabel("Frequency")
        .histogram(&data, None)
        .save("tests/fixtures/golden/05_histogram.png")?;
    count += 1;

    // 6. Box plot
    println!("[{}/25] Box plot...", count + 1);
    let boxdata = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 25.0];
    stable_plot()
        .title("Box Plot")
        .ylabel("Value")
        .boxplot(&boxdata, None)
        .save("tests/fixtures/golden/06_boxplot.png")?;
    count += 1;

    // 7-10. Theme variations
    for (theme, name) in [
        (Theme::light(), "light"),
        (Theme::dark(), "dark"),
        (Theme::publication(), "publication"),
        (Theme::seaborn(), "seaborn"),
    ] {
        println!("[{}/25] {} theme...", count + 1, name);
        stable_plot()
            .title(format!("{} Theme", name.to_uppercase()))
            .xlabel("X")
            .ylabel("Y")
            .theme(stable_theme(theme))
            .line(&x, &y)
            .save(format!(
                "tests/fixtures/golden/0{}_theme_{}.png",
                count + 1,
                name
            ))?;
        count += 1;
    }

    // 11-13. DPI variations
    for dpi in [72, 150, 300] {
        println!("[{}/25] {} DPI...", count + 1, dpi);
        stable_plot()
            .title(format!("{} DPI", dpi))
            .dpi(dpi)
            .line(&x, &y)
            .save(format!(
                "tests/fixtures/golden/{}_dpi_{}.png",
                count + 1,
                dpi
            ))?;
        count += 1;
    }

    // 14. Custom dimensions
    println!("[{}/25] Custom dimensions...", count + 1);
    stable_plot()
        .title("Custom Dimensions")
        .size_px(1200, 900)
        .line(&x, &y)
        .save("tests/fixtures/golden/14_custom_dimensions.png")?;
    count += 1;

    // 15. Subplots
    println!("[{}/25] Subplots...", count + 1);
    let plot1: Plot = stable_plot().title("Linear").line(&x, &y).into();
    let plot2: Plot = stable_plot().title("Scatter").scatter(&x, &y).into();
    let cats_sub = vec!["A", "B", "C"];
    let vals_sub = vec![25.0, 40.0, 30.0];
    let plot3: Plot = stable_plot().title("Bar").bar(&cats_sub, &vals_sub).into();
    let plot4: Plot = stable_plot()
        .title("Histogram")
        .histogram(&data, None)
        .into();

    // Keep figure-level dark-theme behavior, including the light suptitle,
    // under the same exact-pixel contract as the rest of the golden suite.
    subplots(2, 2, 1200, 900)?
        .theme(stable_theme(Theme::dark()))
        .subplot(0, 0, plot1)?
        .subplot(0, 1, plot2)?
        .subplot(1, 0, plot3)?
        .subplot(1, 1, plot4)?
        .suptitle("Subplot Grid")
        .save("tests/fixtures/golden/15_subplots.png")?;
    count += 1;

    // 16. Large dataset
    println!("[{}/25] Large dataset (1K)...", count + 1);
    let x_large: Vec<f64> = (0..1000).map(|i| i as f64 * 0.01).collect();
    let y_large: Vec<f64> = x_large.iter().map(|&t| t.sin()).collect();
    stable_plot()
        .title("1K Points")
        .line(&x_large, &y_large)
        .save("tests/fixtures/golden/16_large_1k.png")?;
    count += 1;

    // 17. Scientific notation
    println!("[{}/25] Scientific notation...", count + 1);
    let x_sci: Vec<f64> = (0..50).map(|i| i as f64 * 100.0).collect();
    let y_sci: Vec<f64> = x_sci.iter().map(|&t| t * t).collect();
    stable_plot()
        .title("Scientific Notation")
        .xlabel("X (x100)")
        .ylabel("Y (x10000)")
        .line(&x_sci, &y_sci)
        .save("tests/fixtures/golden/17_scientific.png")?;
    count += 1;

    // 18. Negative values
    println!("[{}/25] Negative values...", count + 1);
    let x_neg = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
    let y_neg = vec![-4.0, -1.0, 0.0, 1.0, 4.0];
    stable_plot()
        .title("Negative Values")
        .xlabel("X")
        .ylabel("Y")
        .line(&x_neg, &y_neg)
        .save("tests/fixtures/golden/18_negative.png")?;
    count += 1;

    // 19. Zero-crossing
    println!("[{}/25] Zero-crossing...", count + 1);
    let x_zero: Vec<f64> = (0..100).map(|i| (i as f64 - 50.0) * 0.1).collect();
    let y_zero: Vec<f64> = x_zero.iter().map(|&t| t.sin()).collect();
    stable_plot()
        .title("Zero-Crossing")
        .line(&x_zero, &y_zero)
        .save("tests/fixtures/golden/19_zero_crossing.png")?;
    count += 1;

    // 20. Dense scatter
    println!("[{}/25] Dense scatter...", count + 1);
    let x_dense: Vec<f64> = (0..200).map(|i| i as f64 * 0.05).collect();
    let y_dense: Vec<f64> = x_dense.iter().map(|&t| t.sin() + (t * 0.5).cos()).collect();
    stable_plot()
        .title("Dense Scatter")
        .scatter(&x_dense, &y_dense)
        .marker(MarkerStyle::Circle)
        .marker_size(2.0)
        .save("tests/fixtures/golden/20_dense_scatter.png")?;
    count += 1;

    // 21. Wide bar chart
    println!("[{}/25] Wide bar chart...", count + 1);
    let cats: Vec<String> = (0..20).map(|i| format!("C{}", i)).collect();
    let vals: Vec<f64> = (0..20)
        .map(|i| (i as f64 * 1.5).sin() * 50.0 + 50.0)
        .collect();
    let cats_str: Vec<&str> = cats.iter().map(|s| s.as_str()).collect();
    stable_plot()
        .title("Wide Bar Chart")
        .bar(&cats_str, &vals)
        .save("tests/fixtures/golden/21_wide_bar.png")?;
    count += 1;

    // 22. Minimal plot
    println!("[{}/25] Minimal plot...", count + 1);
    let x_min = vec![0.0, 1.0];
    let y_min = vec![0.0, 1.0];
    stable_plot()
        .line(&x_min, &y_min)
        .save("tests/fixtures/golden/22_minimal.png")?;
    count += 1;

    // 23. Long title
    println!("[{}/25] Long title...", count + 1);
    stable_plot()
        .title("This is a Very Long Title That Tests Text Wrapping and Layout Behavior")
        .xlabel("X-axis with longer label")
        .ylabel("Y-axis with longer label")
        .line(&x, &y)
        .save("tests/fixtures/golden/23_long_title.png")?;
    count += 1;

    // 24. Unicode text
    println!("[{}/25] Unicode text...", count + 1);
    stable_plot()
        .title("Unicode: α β γ δ ε θ λ π σ ω")
        .xlabel("Température (°C)")
        .ylabel("Résultat")
        .line(&x, &y)
        .save("tests/fixtures/golden/24_unicode.png")?;
    count += 1;

    // 25. Complex multi-series
    println!("[{}/25] Complex multi-series...", count + 1);
    let x_comp: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    stable_plot()
        .title("Complex Multi-Series")
        .legend(Position::TopRight)
        .line(
            &x_comp,
            &x_comp.iter().map(|&t| t.sin()).collect::<Vec<_>>(),
        )
        .label("sin(x)")
        .line(
            &x_comp,
            &x_comp.iter().map(|&t| t.cos()).collect::<Vec<_>>(),
        )
        .label("cos(x)")
        .line(
            &x_comp,
            &x_comp.iter().map(|&t| (t * 0.5).sin()).collect::<Vec<_>>(),
        )
        .label("sin(x/2)")
        .save("tests/fixtures/golden/25_complex_multi.png")?;
    count += 1;

    println!("\nGenerated {} golden images successfully!", count);
    println!("Location: tests/fixtures/golden/");

    Ok(())
}

#[cfg(not(test))]
fn main() -> Result<()> {
    generate_golden_images()
}
