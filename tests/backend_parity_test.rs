// Backend parity tests - ensure all backends produce consistent output
// Tests that default, parallel, and SIMD backends render identically

mod common;

use common::{assert_png_dimensions_with_tolerance, assert_png_rendered};
use ruviz::prelude::*;

#[test]
fn test_backend_parity_basic_line() {
    // GIVEN: Simple line data
    let x: Vec<f64> = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    // WHEN: Rendering with default backend
    let result_default = Plot::new()
        .title("Backend Parity Test")
        .line(&x, &y)
        .save("generated/tests/render/backend_default_line.png");

    // THEN: Should succeed
    assert!(
        result_default.is_ok(),
        "Default backend failed: {:?}",
        result_default
    );

    assert_png_rendered(
        "generated/tests/render/backend_default_line.png",
        Some((640, 480)),
    );
}

#[test]
#[cfg(feature = "parallel")]
fn test_backend_parity_parallel() {
    // GIVEN: Larger dataset suitable for parallel rendering
    let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    // WHEN: Rendering with parallel backend (automatically used for large data)
    let result_parallel = Plot::new()
        .title("Parallel Backend Test")
        .line(&x, &y)
        .save("generated/tests/render/backend_parallel_line.png");

    // THEN: Should succeed
    assert!(
        result_parallel.is_ok(),
        "Parallel backend failed: {:?}",
        result_parallel
    );

    assert_png_rendered(
        "generated/tests/render/backend_parallel_line.png",
        Some((640, 480)),
    );
}

#[test]
fn test_backend_consistency_scatter() {
    // GIVEN: Scatter data
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![2.3, 3.1, 2.8, 4.2, 3.9];

    // WHEN: Rendering scatter plot
    let result = Plot::new()
        .title("Backend Consistency - Scatter")
        .scatter(&x, &y)
        .marker(MarkerStyle::Circle)
        .marker_size(8.0)
        .save("generated/tests/render/backend_scatter.png");

    // THEN: Should produce consistent output
    assert!(
        result.is_ok(),
        "expected operation to succeed: {:?}",
        result
    );
    assert_png_rendered(
        "generated/tests/render/backend_scatter.png",
        Some((640, 480)),
    );
}

#[test]
fn test_backend_consistency_bar() {
    // GIVEN: Bar chart data
    let categories = vec!["A", "B", "C", "D"];
    let values = vec![25.0, 40.0, 30.0, 55.0];

    // WHEN: Rendering bar chart
    let result = Plot::new()
        .title("Backend Consistency - Bar")
        .bar(&categories, &values)
        .save("generated/tests/render/backend_bar.png");

    // THEN: Should produce consistent output
    assert!(
        result.is_ok(),
        "expected operation to succeed: {:?}",
        result
    );
    assert_png_rendered("generated/tests/render/backend_bar.png", Some((640, 480)));
}

#[test]
fn test_backend_consistency_histogram() {
    // GIVEN: Distribution data
    let data = vec![
        1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0, 4.0, 5.0, 1.5, 2.5, 2.5, 3.5, 3.5, 3.5, 4.5, 4.5, 5.5,
    ];

    // WHEN: Rendering histogram
    let result = Plot::new()
        .title("Backend Consistency - Histogram")
        .histogram(&data)
        .save("generated/tests/render/backend_histogram.png");

    // THEN: Should produce consistent output
    assert!(
        result.is_ok(),
        "expected operation to succeed: {:?}",
        result
    );
    assert_png_rendered(
        "generated/tests/render/backend_histogram.png",
        Some((640, 480)),
    );
}

#[test]
fn test_backend_consistency_boxplot() {
    // GIVEN: Statistical data
    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 25.0];

    // WHEN: Rendering boxplot
    let result = Plot::new()
        .title("Backend Consistency - Boxplot")
        .boxplot(&data)
        .save("generated/tests/render/backend_boxplot.png");

    // THEN: Should produce consistent output
    assert!(
        result.is_ok(),
        "expected operation to succeed: {:?}",
        result
    );
    assert_png_rendered(
        "generated/tests/render/backend_boxplot.png",
        Some((640, 480)),
    );
}

#[test]
fn test_backend_consistency_multi_series() {
    // GIVEN: Multiple data series
    let x: Vec<f64> = vec![0.0, 1.0, 2.0, 3.0, 4.0];

    // WHEN: Rendering with multiple series
    let result = Plot::new()
        .title("Backend Consistency - Multi-Series")
        .legend(LegendPosition::UpperLeft)
        .line(&x, &x.to_vec())
        .label("Linear")
        .line(&x, &x.iter().map(|&v| v * v).collect::<Vec<_>>())
        .label("Quadratic")
        .line(&x, &x.iter().map(|&v| v.powi(3)).collect::<Vec<_>>())
        .label("Cubic")
        .save("generated/tests/render/backend_multi_series.png");

    // THEN: Should produce consistent output
    assert!(
        result.is_ok(),
        "expected operation to succeed: {:?}",
        result
    );
    assert_png_rendered(
        "generated/tests/render/backend_multi_series.png",
        Some((640, 480)),
    );
}

#[test]
fn test_backend_consistency_themes() {
    // GIVEN: Simple data
    let x = vec![0.0, 1.0, 2.0, 3.0];
    let y = vec![0.0, 1.0, 4.0, 9.0];

    // WHEN: Applying different themes
    for (theme, name) in [
        (Theme::light(), "light"),
        (Theme::dark(), "dark"),
        (Theme::publication(), "publication"),
        (Theme::seaborn(), "seaborn"),
    ] {
        let result = Plot::new()
            .theme(theme)
            .title(format!("Backend - {} Theme", name))
            .line(&x, &y)
            .save(format!("generated/tests/render/backend_theme_{}.png", name));

        // THEN: Should produce consistent output for all themes
        assert!(result.is_ok(), "{} theme failed", name);
        assert_png_rendered(
            format!("generated/tests/render/backend_theme_{}.png", name),
            Some((640, 480)),
        );
    }
}

#[test]
fn test_backend_consistency_dpi() {
    // GIVEN: Simple data
    let x = vec![0.0, 1.0, 2.0];
    let y = vec![0.0, 1.0, 4.0];

    // WHEN: Rendering at different DPIs
    for dpi in [72, 96, 150, 300] {
        let result = Plot::new()
            .dpi(dpi)
            .title(format!("Backend - {} DPI", dpi))
            .line(&x, &y)
            .save(format!("generated/tests/render/backend_dpi_{}.png", dpi));

        // THEN: Should succeed for all DPIs
        assert!(result.is_ok(), "{} DPI failed", dpi);

        // AND: Should produce appropriately sized output (±1 pixel for rounding)
        // Default figure size is 6.4 × 4.8 inches, so pixel dimensions = inches × DPI
        let expected_width = (6.4 * dpi as f32) as u32;
        let expected_height = (4.8 * dpi as f32) as u32;
        assert_png_dimensions_with_tolerance(
            format!("generated/tests/render/backend_dpi_{}.png", dpi),
            (expected_width, expected_height),
            1,
        );
    }
}

#[test]
fn test_backend_consistency_dimensions() {
    // GIVEN: Custom dimensions
    let x = vec![0.0, 1.0, 2.0, 3.0];
    let y = vec![0.0, 1.0, 4.0, 9.0];

    // WHEN: Using custom dimensions
    for (width, height) in [(400, 300), (800, 600), (1200, 900), (1600, 1200)] {
        let result = Plot::new()
            .size_px(width, height)
            .title(format!("{}x{}", width, height))
            .line(&x, &y)
            .save(format!(
                "generated/tests/render/backend_dim_{}x{}.png",
                width, height
            ));

        // THEN: Should produce correct dimensions
        assert!(result.is_ok(), "{}x{} failed", width, height);

        assert_png_dimensions_with_tolerance(
            format!(
                "generated/tests/render/backend_dim_{}x{}.png",
                width, height
            ),
            (width, height),
            1,
        );
    }
}

#[test]
fn test_backend_error_handling() {
    // GIVEN: Invalid data
    let empty_x: Vec<f64> = vec![];
    let empty_y: Vec<f64> = vec![];

    // WHEN: Attempting to plot empty data
    let result = Plot::new()
        .line(&empty_x, &empty_y)
        .save("generated/tests/render/backend_should_not_exist.png");

    // THEN: Should fail gracefully across all backends
    assert!(result.is_err(), "Empty data should produce error");

    // AND: Mismatched lengths
    let x = vec![1.0, 2.0, 3.0];
    let y = vec![1.0, 2.0];

    let result = Plot::new()
        .line(&x, &y)
        .save("generated/tests/render/backend_should_not_exist_2.png");

    assert!(result.is_err(), "Mismatched data should produce error");
}

/// The SVG bar layout used to be positional — evenly spaced across the plot
/// area at a 0.7 width fraction — while the raster path mapped category indices
/// through the real x-scale at 0.8. The same chart therefore put its bars in
/// different places depending on the output format.
#[test]
fn test_png_and_svg_bar_geometry_agree() {
    use std::io::Read;

    let directory = tempfile::tempdir().expect("temp dir");
    let png_path = directory.path().join("bars.png");
    let svg_path = directory.path().join("bars.svg");

    let chart = || {
        Plot::new()
            .size_px(800, 600)
            .ticks(false)
            .grid(false)
            .bar(&["A", "B", "C", "D"], &[3.0f64, 5.0, 2.0, 4.0])
            .color(Color::from_rgb(31, 119, 180))
    };
    chart().save(&png_path).expect("bar PNG should save");
    chart()
        .into_plot()
        .export_svg(&svg_path)
        .expect("bar SVG should export");

    let mut svg = String::new();
    std::fs::File::open(&svg_path)
        .expect("open SVG")
        .read_to_string(&mut svg)
        .expect("read SVG");

    // Every filled bar rect, in document order.
    let svg_bars: Vec<(f32, f32)> = svg
        .lines()
        .filter(|line| line.contains("<rect") && line.contains(r#"fill="rgb(31,119,180)""#))
        .map(|line| {
            let attribute = |name: &str| -> f32 {
                let needle = format!(r#"{name}=""#);
                let start = line.find(&needle).expect("attribute present") + needle.len();
                let rest = &line[start..];
                let end = rest.find('"').expect("attribute terminated");
                rest[..end].parse().expect("numeric attribute")
            };
            (attribute("x"), attribute("width"))
        })
        .collect();
    assert_eq!(svg_bars.len(), 4, "every bar must reach the SVG");

    let image = image::open(&png_path).expect("decode bar PNG").to_rgba8();
    let (width, height) = image.dimensions();
    let is_bar = |px: &image::Rgba<u8>| {
        // Fill or its auto-darkened edge.
        (px.0[0] == 31 && px.0[1] == 119 && px.0[2] == 180)
            || (px.0[0] == 21 && px.0[1] == 83 && px.0[2] == 126)
    };
    let mut png_bars: Vec<(f32, f32)> = Vec::new();
    let mut run: Option<(u32, u32)> = None;
    for x in 0..width {
        let occupied = (0..height).any(|y| is_bar(image.get_pixel(x, y)));
        match (&mut run, occupied) {
            (None, true) => run = Some((x, x)),
            (Some(span), true) => span.1 = x,
            (Some(span), false) => {
                png_bars.push((span.0 as f32, (span.1 - span.0 + 1) as f32));
                run = None;
            }
            (None, false) => {}
        }
    }
    if let Some(span) = run {
        png_bars.push((span.0 as f32, (span.1 - span.0 + 1) as f32));
    }
    assert_eq!(png_bars.len(), 4, "every bar must reach the PNG");

    // The SVG rect is the bar body; the PNG span includes the stroke, which
    // straddles the boundary by half its width on each side.
    for (index, ((svg_x, svg_width), (png_x, png_width))) in
        svg_bars.iter().zip(png_bars.iter()).enumerate()
    {
        let svg_centre = svg_x + svg_width / 2.0;
        let png_centre = png_x + png_width / 2.0;
        assert!(
            (svg_centre - png_centre).abs() <= 1.5,
            "bar {index} centre differs between PNG and SVG: {png_centre} vs {svg_centre}"
        );
        assert!(
            (svg_width - png_width).abs() <= 3.0,
            "bar {index} width differs between PNG and SVG: {png_width} vs {svg_width}"
        );
    }
}

/// The SVG backend drew no colorbar at all: a heatmap exported to SVG silently
/// lost its value scale while the PNG carried it. Both now go through one
/// routine, so the strip, its ticks and its labels must line up.
#[test]
fn test_png_and_svg_heatmap_colorbar_agree() {
    use std::io::Read;

    let directory = tempfile::tempdir().expect("temp dir");
    let png_path = directory.path().join("heatmap.png");
    let svg_path = directory.path().join("heatmap.svg");

    let values: Vec<Vec<f64>> = (0..4)
        .map(|row| (0..5).map(|col| (row * 5 + col) as f64).collect())
        .collect();
    let chart = || Plot::new().size_px(640, 480).heatmap(&values);
    chart().save(&png_path).expect("heatmap PNG should save");
    chart()
        .into_plot()
        .export_svg(&svg_path)
        .expect("heatmap SVG should export");

    let mut svg = String::new();
    std::fs::File::open(&svg_path)
        .expect("open SVG")
        .read_to_string(&mut svg)
        .expect("read SVG");

    // The colorbar's own border: the only stroked, unfilled rect in the figure.
    let outline_index = svg
        .lines()
        .position(|line| line.contains("<rect") && line.contains(r#"fill="none""#))
        .expect("the SVG must contain a colorbar outline");
    let outline = svg.lines().nth(outline_index).expect("outline line");

    // It must come after the data clip group closes: a colorbar sits beside the
    // plot area, and inside the clip the whole thing is invisible.
    let clip_close = svg
        .lines()
        .position(|line| line.trim() == "</g>")
        .expect("the SVG must close its data clip group");
    assert!(
        outline_index > clip_close,
        "the colorbar must be drawn outside the plot-area clip, or it is invisible"
    );
    let attribute = |line: &str, name: &str| -> f32 {
        let needle = format!(r#"{name}=""#);
        let start = line.find(&needle).expect("attribute present") + needle.len();
        let rest = &line[start..];
        let end = rest.find('"').expect("attribute terminated");
        rest[..end].parse().expect("numeric attribute")
    };
    let svg_left = attribute(outline, "x");
    let svg_right = svg_left + attribute(outline, "width");

    // Tick labels sit to the right of the strip.
    let label_count = svg
        .lines()
        .filter(|line| line.contains("<text") && attribute(line, "x") > svg_right)
        .count();
    assert!(
        label_count >= 2,
        "the SVG colorbar must be labelled, found {label_count} labels"
    );

    // The PNG's rightmost ink must reach the same strip.
    let image = image::open(&png_path)
        .expect("decode heatmap PNG")
        .to_rgba8();
    let (width, height) = image.dimensions();
    let inked_column = |x: u32| {
        (0..height).any(|y| {
            let px = image.get_pixel(x, y);
            px.0[0] != 255 || px.0[1] != 255 || px.0[2] != 255
        })
    };
    let png_right = (0..width)
        .rev()
        .find(|&x| inked_column(x))
        .expect("the PNG must have ink") as f32;

    // `png_right` is the far edge of the tick labels, which start beyond the
    // strip; the strip's own right edge must therefore sit just inside it.
    assert!(
        svg_right <= png_right + 1.0,
        "the SVG colorbar strip must end at or before the PNG's rightmost ink: \
         svg {svg_right}, png {png_right}"
    );
    assert!(
        svg_right > png_right - 60.0,
        "the SVG colorbar strip must be in the same place as the PNG's: \
         svg {svg_right}, png {png_right}"
    );
}

/// A filled contour is mostly fill. The SVG backend drew only the lines, so an
/// exported figure lost every band it was made of while the PNG carried them.
#[test]
fn test_png_and_svg_filled_contour_agree() {
    use std::io::Read;

    let directory = tempfile::tempdir().expect("temp dir");
    let png_path = directory.path().join("contour.png");
    let svg_path = directory.path().join("contour.svg");

    let x: Vec<f64> = (1..=20).map(f64::from).collect();
    let y: Vec<f64> = (1..=16).map(f64::from).collect();
    let z: Vec<f64> = y
        .iter()
        .flat_map(|&yy| x.iter().map(move |&xx| (xx * 0.2).sin() + (yy * 0.2).cos()))
        .collect();

    let chart = || Plot::new().size_px(640, 480).contour(&x, &y, &z);
    chart().save(&png_path).expect("contour PNG should save");
    chart()
        .into_plot()
        .export_svg(&svg_path)
        .expect("contour SVG should export");

    let mut svg = String::new();
    std::fs::File::open(&svg_path)
        .expect("open SVG")
        .read_to_string(&mut svg)
        .expect("read SVG");

    let band_count = svg.matches("shape-rendering=\"crispEdges\"").count();
    assert!(
        band_count > 50,
        "the SVG must carry the filled contour bands, found {band_count}"
    );

    // The PNG's plot area is dominated by fill, not by white background. The SVG
    // has to be too, which the band count above is a proxy for; here we only
    // confirm the PNG really is filled so the test cannot pass vacuously.
    let image = image::open(&png_path)
        .expect("decode contour PNG")
        .to_rgba8();
    let (width, height) = image.dimensions();
    let coloured = (0..width)
        .step_by(4)
        .flat_map(|x| (0..height).step_by(4).map(move |y| (x, y)))
        .filter(|&(x, y)| {
            let px = image.get_pixel(x, y);
            px.0[0] != 255 || px.0[1] != 255 || px.0[2] != 255
        })
        .count();
    assert!(
        coloured > (width as usize / 4) * (height as usize / 4) / 3,
        "the PNG contour must be filled, only {coloured} sampled pixels carried ink"
    );
}

/// Helper: pull every `<line ...>` element out of an SVG as attribute maps.
fn svg_lines(svg: &str) -> Vec<std::collections::HashMap<String, String>> {
    let mut out = Vec::new();
    let mut rest = svg;
    while let Some(start) = rest.find("<line ") {
        rest = &rest[start + "<line ".len()..];
        let Some(end) = rest.find('>') else { break };
        let body = &rest[..end];
        rest = &rest[end..];

        let mut attributes = std::collections::HashMap::new();
        let mut tail = body;
        while let Some(equals) = tail.find("=\"") {
            let name = tail[..equals]
                .rsplit(|c: char| c.is_whitespace())
                .next()
                .unwrap_or_default()
                .to_string();
            let value_start = equals + 2;
            let Some(value_end) = tail[value_start..].find('"') else {
                break;
            };
            attributes.insert(name, tail[value_start..value_start + value_end].to_string());
            tail = &tail[value_start + value_end + 1..];
        }
        out.push(attributes);
    }
    out
}

/// A linear axis whose topmost tick lands exactly on `y_max` must keep that
/// tick in both backends.
///
/// The tick's pixel used to be computed as `bottom - 1.0 * (bottom - top)`,
/// which in `f32` lands a quarter of a ULP *above* `plot_top`; every backend
/// then filters ticks with `pos >= plot_top`, so whichever one rounded the
/// wrong way silently dropped the mark, the gridline and the label. `1000000`
/// was missing from the SVG of this exact figure while the PNG drew it.
#[test]
fn test_png_and_svg_keep_the_topmost_tick_on_a_linear_axis() {
    use std::io::Read;

    let directory = tempfile::tempdir().expect("temp dir");
    let png_path = directory.path().join("million.png");
    let svg_path = directory.path().join("million.svg");

    // 800x600 at 125 DPI puts the plot area at 38.61..573.19 vertically, where
    // `573.19 - 1.0 * (573.19 - 38.61)` rounds to 38.6099854 in `f32` — a
    // quarter of a ULP outside the frame. The defect is a rounding accident, so
    // the figure has to pin the geometry that produces it.
    let chart = || {
        Plot::new()
            .size_px(800, 600)
            .dpi(125)
            .line(&[0.0f64, 9.0], &[10.0f64, 100.0])
            .xlim(0.0, 1e6)
            .ylim(0.0, 1e6)
    };
    chart().save(&png_path).expect("PNG should save");
    chart()
        .into_plot()
        .export_svg(&svg_path)
        .expect("SVG should export");

    let mut svg = String::new();
    std::fs::File::open(&svg_path)
        .expect("open SVG")
        .read_to_string(&mut svg)
        .expect("read SVG");

    // Both axes end on 1000000, so counting occurrences distinguishes "the y
    // axis kept its top label" from "the x axis happens to carry the string".
    assert_eq!(
        svg.matches(">1000000</text>").count(),
        2,
        "both axes must label the topmost tick that sits exactly on their max"
    );

    // Both axes run 0..1e6 with a tick every 200000, so six gridlines per
    // direction — including the pair that land exactly on the far spines.
    // (Seven-character labels cap the tick count well below the axis targets
    // of 10 and 8; eleven of them at 125 DPI genuinely overlap.)
    let grid: Vec<_> = svg_lines(&svg)
        .into_iter()
        .filter(|line| line.get("stroke").is_some_and(|s| s == "rgb(176,176,176)"))
        .collect();
    let horizontal: std::collections::BTreeSet<i64> = grid
        .iter()
        .filter(|l| l["y1"] == l["y2"])
        .map(|l| (l["y1"].parse::<f64>().unwrap() * 100.0).round() as i64)
        .collect();
    let vertical: std::collections::BTreeSet<i64> = grid
        .iter()
        .filter(|l| l["x1"] == l["x2"])
        .map(|l| (l["x1"].parse::<f64>().unwrap() * 100.0).round() as i64)
        .collect();
    assert_eq!(
        horizontal.len(),
        6,
        "SVG horizontal gridlines: {horizontal:?}"
    );
    assert_eq!(vertical.len(), 6, "SVG vertical gridlines: {vertical:?}");

    // The PNG must carry the same six horizontal rules, so the two backends
    // are not merely each self-consistent.
    let image = image::open(&png_path).expect("decode PNG").to_rgba8();
    let (width, height) = image.dimensions();
    let mut rows: Vec<u32> = Vec::new();
    for y in 0..height {
        let inked = (width / 8..width * 7 / 8)
            .filter(|&x| {
                let px = image.get_pixel(x, y);
                px.0[0] < 250 || px.0[1] < 250 || px.0[2] < 250
            })
            .count();
        // The plot area is narrower than the canvas, so a full-width rule
        // inks most — not all — of the sampled span.
        if inked as u32 > width * 5 / 8 {
            // A rule is 2-3 px of ink at this DPI; treat a near-adjacent row
            // as the same rule rather than a second one.
            if rows.last().is_some_and(|&prev| y - prev <= 3) {
                continue;
            }
            rows.push(y);
        }
    }
    assert_eq!(rows.len(), 6, "PNG horizontal rules at rows {rows:?}");
}

/// Error bars attached to a line with `with_yerr` must reach the SVG.
///
/// The SVG backend drew none of them while the bounds calculation still
/// reserved axis room for the whiskers, so an SVG of an error-bar plot showed a
/// stretched, empty axis. Both backends now go through one geometry routine.
#[test]
fn test_png_and_svg_attached_error_bars_agree() {
    use std::io::Read;

    let directory = tempfile::tempdir().expect("temp dir");
    let png_path = directory.path().join("yerr.png");
    let svg_path = directory.path().join("yerr.svg");

    let x = vec![1.0f64, 2.0, 3.0, 4.0, 5.0];
    let y = vec![10.0f64, 12.0, 15.0, 18.0, 20.0];
    let errors = vec![1.0f64, 1.0, 1.0, 1.0, 8.0];

    let chart = || {
        Plot::new()
            .size_px(640, 480)
            .line(&x, &y)
            .with_yerr(&errors)
            .color(Color::from_rgb(31, 119, 180))
    };
    chart().save(&png_path).expect("PNG should save");
    chart()
        .into_plot()
        .export_svg(&svg_path)
        .expect("SVG should export");

    let mut svg = String::new();
    std::fs::File::open(&svg_path)
        .expect("open SVG")
        .read_to_string(&mut svg)
        .expect("read SVG");

    // Five samples, each a stem plus two caps.
    let bars: Vec<_> = svg_lines(&svg)
        .into_iter()
        .filter(|line| line.get("stroke").is_some_and(|s| s == "rgb(31,119,180)"))
        .collect();
    assert_eq!(
        bars.len(),
        15,
        "five error bars are a stem and two caps each, found {}",
        bars.len()
    );

    // The tall whisker at x = 5 reaches 28 while the data stops at 20. Its top
    // cap must sit inside the frame, not clipped onto the spine.
    let stems: Vec<(f64, f64)> = bars
        .iter()
        .filter(|l| l["x1"] == l["x2"])
        .map(|l| {
            (
                l["y1"].parse::<f64>().unwrap(),
                l["y2"].parse::<f64>().unwrap(),
            )
        })
        .collect();
    let highest = stems
        .iter()
        .map(|&(a, b)| a.min(b))
        .fold(f64::INFINITY, f64::min);
    assert!(
        highest > 5.0,
        "the tallest whisker's top must clear the frame edge, got {highest}"
    );

    // The PNG must draw the same whisker, with a visible cap at the same place.
    let image = image::open(&png_path).expect("decode PNG").to_rgba8();
    let (width, height) = image.dimensions();
    let is_series =
        |px: &image::Rgba<u8>| px.0[0] == 31 && px.0[1] == 119 && px.0[2] == 180 && px.0[3] > 0;
    let tallest = (0..width)
        .filter_map(|x| {
            let rows: Vec<u32> = (0..height)
                .filter(|&y| is_series(image.get_pixel(x, y)))
                .collect();
            let first = *rows.first()?;
            let last = *rows.last()?;
            Some((last - first + 1, first))
        })
        .max_by_key(|&(span, _)| span)
        .expect("the PNG must carry series ink");
    let png_top = tallest.1 as f64;
    assert!(
        (png_top - highest).abs() < 3.0,
        "PNG whisker top row {png_top} must match the SVG's {highest}"
    );
}
