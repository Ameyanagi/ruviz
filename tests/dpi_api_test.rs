//! Contract for the DPI-aware save API: `.dpi(n).save(path)`.
//!
//! `.dpi(n)` sets the export resolution of a figure whose size is held in
//! inches, so the saved PNG has to be `figure_inches * dpi` pixels. Until this
//! rewrite every test in this file saved a file and `println!`d, with no
//! assertion at all: a `.dpi()` that quietly became a no-op passed all six.
//! Each test now reads the dimensions back off the encoded PNG with
//! `image::image_dimensions`, which is the only evidence that the setting
//! reached the output.

mod common;

use ruviz::prelude::*;
use std::path::Path;

/// `FigureConfig` defaults, in inches.
const DEFAULT_FIGURE_INCHES: (f32, f32) = (6.4, 4.8);

/// `.dpi()` clamps anything below this to it.
const MIN_DPI: u32 = 72;

/// The pixel size a default-sized figure must be saved at for a given DPI.
fn expected_dimensions(dpi: u32) -> (u32, u32) {
    let dpi = dpi.max(MIN_DPI) as f32;
    let (width_in, height_in) = DEFAULT_FIGURE_INCHES;
    ((width_in * dpi) as u32, (height_in * dpi) as u32)
}

fn saved_dimensions(path: &Path) -> (u32, u32) {
    image::image_dimensions(path)
        .unwrap_or_else(|error| panic!("{} is not a readable PNG: {error}", path.display()))
}

/// Assert a saved PNG carries the resolution `.dpi(dpi)` asked for.
///
/// One pixel of slack absorbs the `inches * dpi` truncation, and nothing more:
/// a `.dpi()` that did nothing would leave a 640x480 file where 1920x1440 is
/// required, which is 1280 pixels of error, not one.
fn assert_saved_at_dpi(path: &Path, dpi: u32) {
    let (width, height) = saved_dimensions(path);
    let (expected_width, expected_height) = expected_dimensions(dpi);
    assert!(
        width.abs_diff(expected_width) <= 1 && height.abs_diff(expected_height) <= 1,
        "{} was saved at {width}x{height}, but .dpi({dpi}) on the default \
         {:?} inch figure must produce about {expected_width}x{expected_height}",
        path.display(),
        DEFAULT_FIGURE_INCHES
    );
}

/// Pin the DPI-to-pixels rule to evidence outside this file.
///
/// `tests/fixtures/golden/11_dpi_72.png`, `12_dpi_150.png` and
/// `13_dpi_300.png` are default-sized figures saved at those DPIs, and they are
/// byte-compared in CI, so their dimensions are an independent statement of
/// what `.dpi()` has to produce.
#[test]
fn dpi_to_pixel_rule_matches_the_committed_golden_fixtures() {
    assert_eq!(expected_dimensions(72), (460, 345));
    assert_eq!(expected_dimensions(150), (960, 720));
    assert_eq!(expected_dimensions(300), (1920, 1440));
}

#[test]
fn test_dpi_fluent_api_basic() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let path = common::test_output_path("dpi_300_test.png");

    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![2.0, 4.0, 1.0, 3.0, 5.0];

    Plot::new()
        .title("DPI Test - 300 DPI")
        .xlabel("X Axis")
        .ylabel("Y Axis")
        .line(&x_data, &y_data)
        .dpi(300)
        .save(&path)?;

    assert_eq!(saved_dimensions(&path), (1920, 1440));
    Ok(())
}

#[test]
fn test_ieee_publication_dpi() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let path = common::test_output_path("ieee_600_dpi_test.png");

    let x_data: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y_data: Vec<f64> = x_data.iter().map(|x| x.sin()).collect();

    // IEEE publication standard: 600 DPI.
    Plot::new()
        .title("IEEE Publication Quality")
        .xlabel("Time (s)")
        .ylabel("Amplitude")
        .line(&x_data, &y_data)
        .dpi(600)
        .save(&path)?;

    assert_saved_at_dpi(&path, 600);
    Ok(())
}

#[test]
fn test_multiple_dpi_outputs() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y_data = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    let base_plot = Plot::new()
        .title("Multi-DPI Test")
        .xlabel("X Values")
        .ylabel("X²")
        .line(&x_data, &y_data);

    let mut previous_width = 0;
    for dpi in [96, 150, 300] {
        let path = common::test_output_path(&format!("multi_dpi_{dpi}_test.png"));
        base_plot.clone().dpi(dpi).save(&path)?;
        assert_saved_at_dpi(&path, dpi);

        let (width, _) = saved_dimensions(&path);
        assert!(
            width > previous_width,
            "raising the DPI to {dpi} must widen the output, but {width} is not \
             wider than the previous {previous_width}"
        );
        previous_width = width;
    }
    Ok(())
}

#[test]
fn test_dpi_with_theme() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let path = common::test_output_path("theme_with_dpi_test.png");

    let x_data = vec![1.0, 2.0, 3.0];
    let y_data = vec![2.0, 4.0, 6.0];

    // A theme must not disturb the export resolution.
    Plot::new()
        .theme(Theme::publication())
        .title("Publication Theme with High DPI")
        .xlabel("Input")
        .ylabel("Output")
        .line(&x_data, &y_data)
        .dpi(300)
        .save(&path)?;

    assert_eq!(saved_dimensions(&path), (1920, 1440));
    Ok(())
}

#[test]
fn test_dpi_validation() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let below_minimum = common::test_output_path("dpi_validation_test.png");
    let at_minimum = common::test_output_path("dpi_minimum_test.png");

    let x_data = vec![1.0, 2.0];
    let y_data = vec![1.0, 2.0];

    let plot = Plot::new()
        .title("DPI Validation Test")
        .line(&x_data, &y_data);

    // 50 DPI is below the floor, so it must render exactly as 72 DPI does
    // rather than shrinking the output.
    plot.clone().dpi(50).save(&below_minimum)?;
    plot.dpi(MIN_DPI).save(&at_minimum)?;

    assert_eq!(
        saved_dimensions(&below_minimum),
        saved_dimensions(&at_minimum),
        ".dpi(50) must clamp to the {MIN_DPI} DPI floor"
    );
    assert_eq!(saved_dimensions(&at_minimum), expected_dimensions(MIN_DPI));
    Ok(())
}

#[test]
fn test_scientific_dpi_presets() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let x_data = vec![0.0, 0.5, 1.0, 1.5, 2.0];
    let y_data = vec![0.0, 0.25, 1.0, 2.25, 4.0];

    let plot = Plot::new()
        .title("Scientific DPI Presets")
        .xlabel("x")
        .ylabel("x²")
        .line(&x_data, &y_data);

    // Screen (96), web (150), print (300) and IEEE publication (600).
    for (label, dpi) in [("screen", 96), ("web", 150), ("print", 300), ("ieee", 600)] {
        let path = common::test_output_path(&format!("scientific_{label}_{dpi}_test.png"));
        plot.clone().dpi(dpi).save(&path)?;
        assert_saved_at_dpi(&path, dpi);
    }
    Ok(())
}
