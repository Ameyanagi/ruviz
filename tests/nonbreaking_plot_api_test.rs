use ruviz::core::{
    BackendFallbackReason, BackendOperation, ComputedMarginsPixels, LayoutRect, MeasuredDimensions,
    PlotLayout, TextPosition,
};
use ruviz::prelude::*;

#[test]
fn image_struct_literals_remain_source_compatible() {
    let image = Image {
        width: 1,
        height: 1,
        pixels: vec![20, 40, 60, 128],
    };

    assert_eq!(image.alpha_mode(), AlphaMode::Straight);
    assert_eq!(
        image
            .pixels_in_alpha_mode(AlphaMode::Premultiplied)
            .as_ref(),
        &[10, 20, 30, 128]
    );
}

#[test]
fn image_alpha_conversion_preserves_the_straight_storage_contract() {
    let image = Image::from_premultiplied_rgba(1, 1, vec![64, 32, 16, 128]);

    assert_eq!(image.alpha_mode(), AlphaMode::Straight);
    assert_eq!(image.pixels, vec![128, 64, 32, 128]);
    assert_eq!(
        image
            .pixels_in_alpha_mode(AlphaMode::Premultiplied)
            .as_ref(),
        &[64, 32, 16, 128]
    );
}

#[test]
fn layout_structs_support_exhaustive_external_construction() {
    let plot_area = LayoutRect {
        left: 10.0,
        top: 20.0,
        right: 310.0,
        bottom: 220.0,
    };
    let layout = PlotLayout {
        plot_area,
        title_pos: Some(TextPosition {
            x: 160.0,
            y: 5.0,
            size: 18.0,
        }),
        xlabel_pos: None,
        ylabel_pos: None,
        xtick_baseline_y: 225.0,
        ytick_right_x: 5.0,
        margins: ComputedMarginsPixels {
            left: 10.0,
            right: 10.0,
            top: 20.0,
            bottom: 20.0,
        },
    };
    let measured = MeasuredDimensions {
        title: Some((100.0, 20.0)),
        xlabel: None,
        ylabel: None,
        xtick: Some((30.0, 12.0)),
        ytick: Some((24.0, 12.0)),
        right_margin: Some(10.0),
    };

    assert_eq!(layout.plot_area, plot_area);
    assert_eq!(measured.title, Some((100.0, 20.0)));
}

#[test]
fn builder_when_compiles_across_public_builder_families() {
    let x = vec![0.0, 1.0, 2.0];
    let y = vec![0.0, 1.0, 0.5];
    let data = vec![1.0, 2.0, 3.0, 4.0];

    let _plot = Plot::new()
        .when(true, |plot| plot.title("conditional title"))
        .grid(false);

    let _generic_builder = Plot::new()
        .line(&x, &y)
        .when(true, |builder| builder.label("line"))
        .line_width(2.0);

    let _series_builder = Plot::new()
        .histogram(&data)
        .when(true, |builder| builder.label("histogram"))
        .line_width(2.0);

    let _grouped_plot = Plot::new().group(|group| {
        group
            .when(true, |group| group.group_label("group"))
            .line_width(2.0)
            .line(&x, &y)
    });

    let figure = SubplotFigure::new(1, 1, 400, 300)
        .expect("subplot figure should be constructed")
        .when(true, |figure| figure.suptitle("conditional title"))
        .margin(0.1);
    assert_eq!(figure.grid_spec().total_subplots(), 1);
}

#[cfg(all(feature = "interactive", not(target_arch = "wasm32")))]
#[test]
fn builder_when_compiles_for_interactive_window_builder() {
    let _builder = InteractiveWindowBuilder::new()
        .when(true, |window| window.resizable(false))
        .title("conditional");
}

#[test]
fn high_level_step_area_and_stem_apis_render() {
    let x = vec![0.0, 1.0, 2.0, 3.0];
    let y = vec![1.0, 3.0, 2.0, 4.0];

    let step = Plot::new()
        .step(&x, &y, StepWhere::Post)
        .line_width(2.0)
        .render();
    assert!(step.is_ok());

    let area = Plot::new().area(&x, &y, 0.0).color(Color::BLUE).render();
    assert!(area.is_ok());

    let stem = Plot::new().stem(&x, &y, 0.0).marker_size(5.0).render();
    assert!(stem.is_ok());
}

#[test]
fn high_level_area_and_stem_apis_emit_svg_geometry() {
    let x = vec![0.0, 1.0, 2.0, 3.0];
    let y = vec![1.0, 3.0, 2.0, 4.0];

    let line_svg = Plot::new()
        .grid(false)
        .ticks(false)
        .line(&x, &y)
        .render_to_svg()
        .unwrap();
    let area_svg = Plot::new()
        .grid(false)
        .ticks(false)
        .area(&x, &y, 0.0)
        .render_to_svg()
        .unwrap();
    assert!(area_svg.matches("<polygon").count() > line_svg.matches("<polygon").count());

    let scatter_svg = Plot::new()
        .grid(false)
        .ticks(false)
        .scatter(&x, &y)
        .render_to_svg()
        .unwrap();
    let stem_svg = Plot::new()
        .grid(false)
        .ticks(false)
        .stem(&x, &y, 0.0)
        .render_to_svg()
        .unwrap();
    assert!(stem_svg.matches("<line ").count() >= scatter_svg.matches("<line ").count() + x.len());
}

#[test]
fn high_level_step_api_reports_length_mismatch() {
    let x = vec![0.0, 1.0, 2.0];
    let y = vec![1.0, 3.0];

    let result = Plot::new().step(&x, &y, StepWhere::Post).render();
    assert!(matches!(
        result,
        Err(ruviz::core::PlottingError::DataLengthMismatch { .. })
    ));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn public_render_honors_pending_ingestion_errors() {
    let x: Vec<f64> = (0..5_000).map(|index| index as f64).collect();
    let y: Vec<f64> = x.iter().map(|value| value.sin()).collect();
    let bad_x = vec![0.0, 1.0];
    let bad_y = vec![0.0];

    let err = Plot::new()
        .scatter(&x, &y)
        .into_plot()
        .step(&bad_x, &bad_y, StepWhere::Post)
        .render()
        .unwrap_err();

    assert!(matches!(
        err,
        ruviz::core::PlottingError::DataLengthMismatch { .. }
    ));
}

#[test]
fn high_level_boxen_api_renders() {
    let data: Vec<f64> = (0..128)
        .map(|i| (i as f64 * 0.2).sin() + i as f64 / 64.0)
        .collect();

    let boxen = Plot::new()
        .boxen(&data)
        .k_depth(4)
        .box_width(0.7)
        .saturation(0.6)
        .edge_width(1.5)
        .outlier_size(3.0)
        .render();

    assert!(boxen.is_ok());
}

#[test]
fn high_level_boxen_api_emits_svg_geometry() {
    let data: Vec<f64> = (0..128)
        .map(|i| (i as f64 * 0.2).sin() + i as f64 / 64.0)
        .collect();

    let vertical_svg = Plot::new()
        .boxen(&data)
        .render_to_svg()
        .expect("vertical boxen SVG should render");
    assert!(
        vertical_svg.matches("<polygon").count() >= 1,
        "expected vertical boxen polygons in SVG: {vertical_svg}"
    );

    let horizontal_svg = Plot::new()
        .boxen(&data)
        .horizontal()
        .render_to_svg()
        .expect("horizontal boxen SVG should render");
    assert!(
        horizontal_svg.matches("<polygon").count() >= 1,
        "expected horizontal boxen polygons in SVG: {horizontal_svg}"
    );
}

/// `.boxplot(&d).horizontal()` had no spelling at all: bar, violin and boxen
/// each carried `.horizontal()`, and a box plot could only be turned through
/// `BoxPlotConfig::orientation`, which nothing downstream read. Setting it
/// produced a vertical box.
#[test]
fn high_level_boxplot_api_turns_a_quarter_turn() {
    let data: Vec<f64> = (0..128)
        .map(|i| (i as f64 * 0.2).sin() + i as f64 / 64.0)
        .collect();

    let vertical = Plot::new()
        .boxplot(&data)
        .color(BOX_STROKE)
        .category("sample")
        .render_to_svg()
        .expect("vertical box plot SVG should render");
    let horizontal = Plot::new()
        .boxplot(&data)
        .horizontal()
        .color(BOX_STROKE)
        .category("sample")
        .render_to_svg()
        .expect("horizontal box plot SVG should render");

    assert_ne!(
        vertical, horizontal,
        "a horizontal box plot must not render identically to a vertical one"
    );

    // A box plot draws five straight lines: the median and two whisker caps
    // run across the category axis, the two whiskers along the value axis.
    // Turning the plot swaps which screen axis each of those holds constant.
    assert_eq!(
        box_stroke_axes(&vertical),
        (2, 3),
        "a vertical box plot must hold x on its whiskers and y on its median \
         and caps: {vertical}"
    );
    assert_eq!(
        box_stroke_axes(&horizontal),
        (3, 2),
        "a horizontal box plot must hold x on its median and caps and y on \
         its whiskers: {horizontal}"
    );

    // And the category label follows the categories into the left gutter.
    assert!(
        horizontal.contains("sample"),
        "a horizontal box plot must still label its category: {horizontal}"
    );
}

/// A colour no theme picks, so the box's own strokes can be told from the
/// grid and the spines.
const BOX_STROKE: Color = Color {
    r: 220,
    g: 20,
    b: 60,
    a: 255,
};

/// How many of the box's strokes hold x constant, and how many hold y.
fn box_stroke_axes(svg: &str) -> (usize, usize) {
    let stroke = format!(
        "stroke=\"rgb({},{},{})\"",
        BOX_STROKE.r, BOX_STROKE.g, BOX_STROKE.b
    );
    let mut vertical_strokes = 0;
    let mut horizontal_strokes = 0;
    for line in svg.lines() {
        let line = line.trim_start();
        if !line.starts_with("<line") || !line.contains(&stroke) {
            continue;
        }
        let coordinate = |name: &str| -> f32 {
            let start = line.find(&format!("{name}=\"")).expect("coordinate") + name.len() + 2;
            line[start..]
                .split('"')
                .next()
                .expect("coordinate value")
                .parse()
                .expect("numeric coordinate")
        };
        if coordinate("x1") == coordinate("x2") {
            vertical_strokes += 1;
        } else if coordinate("y1") == coordinate("y2") {
            horizontal_strokes += 1;
        }
    }
    (vertical_strokes, horizontal_strokes)
}

/// `.show_mean(true)` reached only the legacy `PlotRender` path, so through
/// `Plot::boxplot(..)` — the path the docs demonstrate — it drew nothing.
#[test]
fn high_level_boxplot_show_mean_draws_a_marker() {
    let data: Vec<f64> = (0..64)
        .map(|i| (i as f64 * 0.3).sin() * 2.0 + 5.0)
        .collect();
    let polygons = |svg: &str| svg.matches("<polygon").count();

    let without = Plot::new()
        .boxplot(&data)
        .render_to_svg()
        .expect("box plot without mean should render");
    let with_mean = Plot::new()
        .boxplot(&data)
        .show_mean(true)
        .render_to_svg()
        .expect("box plot with mean should render");
    assert_eq!(
        polygons(&with_mean),
        polygons(&without) + 1,
        "show_mean(true) must add exactly one diamond: {with_mean}"
    );

    let horizontal = Plot::new()
        .boxplot(&data)
        .horizontal()
        .show_mean(true)
        .render_to_svg()
        .expect("horizontal box plot with mean should render");
    assert_eq!(
        polygons(&horizontal),
        polygons(&without) + 1,
        "the mean diamond must survive the quarter turn: {horizontal}"
    );
}

/// `invert_y()` flips the resolved range, so a horizontal bar chart lists its
/// first category at the top — without hand-computing the descending `ylim`
/// that was previously the only spelling.
#[test]
fn invert_y_puts_the_first_category_on_top() {
    let cats = ["First", "Second", "Third"];
    let vals = [3.0, 1.0, 2.0];

    let label_y = |svg: &str, name: &str| -> f32 {
        svg.lines()
            .find(|line| line.contains(&format!(">{name}<")))
            .and_then(|line| {
                let start = line.find("y=\"")? + 3;
                line[start..].split('"').next()?.parse().ok()
            })
            .unwrap_or_else(|| panic!("no <text> for {name}: {svg}"))
    };

    let natural = Plot::new()
        .bar(&cats, &vals)
        .horizontal()
        .render_to_svg()
        .expect("horizontal bars should render");
    assert!(
        label_y(&natural, "First") > label_y(&natural, "Third"),
        "without inversion the first category sits at the bottom"
    );

    let inverted = Plot::new()
        .bar(&cats, &vals)
        .horizontal()
        .invert_y()
        .render_to_svg()
        .expect("inverted horizontal bars should render");
    assert!(
        label_y(&inverted, "First") < label_y(&inverted, "Third"),
        "invert_y() must put the first category on top: {inverted}"
    );
}

#[test]
fn high_level_boxplot_api_reports_empty_data_when_horizontal() {
    let empty: Vec<f64> = vec![];
    let err = Plot::new()
        .boxplot(&empty)
        .horizontal()
        .render()
        .unwrap_err();
    assert!(matches!(err, ruviz::core::PlottingError::EmptyDataSet));
}

#[test]
fn high_level_boxen_api_reports_empty_data() {
    let empty_data: Vec<f64> = vec![];
    let err = Plot::new().boxen(&empty_data).render().unwrap_err();
    assert!(matches!(err, ruviz::core::PlottingError::EmptyDataSet));

    let non_finite_data = vec![f64::NAN, f64::INFINITY];
    let err = Plot::new().boxen(&non_finite_data).render().unwrap_err();
    assert!(matches!(err, ruviz::core::PlottingError::EmptyDataSet));
}

#[test]
fn high_level_quiver_api_renders() {
    let x = vec![0.0, 1.0, 0.0, 1.0];
    let y = vec![0.0, 0.0, 1.0, 1.0];
    let u = vec![1.0, 0.4, -0.2, -0.8];
    let v = vec![0.2, 0.9, 0.7, -0.1];

    let quiver = Plot::new()
        .quiver(&x, &y, &u, &v)
        .arrow_scale(0.25)
        .arrow_width(1.2)
        .pivot(QuiverPivot::Middle)
        .color_by_magnitude(true)
        .render();

    assert!(quiver.is_ok());
}

#[test]
fn high_level_quiver_api_reports_length_mismatch() {
    let x = vec![0.0, 1.0];
    let y = vec![0.0];
    let u = vec![1.0, 1.0];
    let v = vec![0.0, 0.0];

    let err = Plot::new().quiver(&x, &y, &u, &v).render().unwrap_err();

    assert!(matches!(
        err,
        ruviz::core::PlottingError::DataLengthMismatch { .. }
    ));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn high_level_quiver_api_stays_on_reference_renderer() {
    let x: Vec<f64> = (0..2_500).map(|index| (index % 50) as f64).collect();
    let y: Vec<f64> = (0..2_500).map(|index| (index / 50) as f64).collect();
    let u: Vec<f64> = x.iter().map(|value| (value * 0.1).sin()).collect();
    let v: Vec<f64> = y.iter().map(|value| (value * 0.1).cos()).collect();

    let (_, diagnostics) = Plot::new()
        .quiver(&x, &y, &u, &v)
        .color_by_magnitude(true)
        .into_plot()
        .benchmark_render_png_bytes_with_diagnostics()
        .expect("quiver should render through the public PNG path");

    assert!(!diagnostics.used_parallel);
}

#[test]
fn resolved_backend_reports_actual_public_png_path() {
    let x = vec![0.0, 1.0, 2.0];
    let y = vec![0.0, 1.0, 4.0];

    let plot = Plot::new()
        .backend(BackendType::DataShader)
        .line(&x, &y)
        .into_plot();
    assert_eq!(plot.get_backend_name(), "datashader");
    assert_eq!(plot.resolved_backend_name(), "skia");
    #[cfg(not(target_arch = "wasm32"))]
    assert_eq!(
        plot.backend_resolution(BackendOperation::Png)
            .fallback_reason(),
        Some(BackendFallbackReason::UnsupportedSeries)
    );
    #[cfg(target_arch = "wasm32")]
    assert_eq!(
        plot.backend_resolution(BackendOperation::Png)
            .fallback_reason(),
        Some(BackendFallbackReason::UnsupportedTarget)
    );
}

#[test]
#[cfg(target_arch = "wasm32")]
fn wasm_resolved_backend_reports_skia_for_explicit_datashader_scatter() {
    let x = vec![0.0, 1.0, 2.0];
    let y = vec![0.0, 1.0, 4.0];

    let plot = Plot::new()
        .backend(BackendType::DataShader)
        .scatter(&x, &y)
        .into_plot();

    assert_eq!(plot.get_backend_name(), "datashader");
    assert_eq!(plot.resolved_backend_name(), "skia");
    assert_eq!(
        plot.backend_resolution(BackendOperation::Png)
            .fallback_reason(),
        Some(BackendFallbackReason::UnsupportedTarget)
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn explicit_datashader_backend_overrides_auto_optimize_ordering() {
    let x: Vec<f64> = (0..100_000).map(|index| index as f64 * 0.00001).collect();
    let y: Vec<f64> = x.iter().map(|value| value.sin()).collect();

    let auto_then_backend = Plot::new()
        .scatter(&x, &y)
        .auto_optimize()
        .backend(BackendType::DataShader)
        .into_plot();
    assert_eq!(auto_then_backend.resolved_backend_name(), "datashader");

    let backend_then_auto = Plot::new()
        .backend(BackendType::DataShader)
        .scatter(&x, &y)
        .auto_optimize()
        .into_plot();
    assert_eq!(backend_then_auto.resolved_backend_name(), "datashader");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn explicit_datashader_backend_falls_back_on_non_linear_axes() {
    let x = vec![1.0, 10.0, 100.0, 1000.0];
    let y = vec![1.0, 2.0, 3.0, 4.0];

    let plot = Plot::new()
        .backend(BackendType::DataShader)
        .xscale(AxisScale::Log)
        .scatter(&x, &y)
        .into_plot();

    assert_eq!(plot.get_backend_name(), "datashader");
    assert_eq!(plot.resolved_backend_name(), "skia");
    let resolution = plot.backend_resolution(BackendOperation::Png);
    assert_eq!(resolution.actual_backend(), BackendType::Skia);
    assert_eq!(
        resolution.fallback_reason(),
        Some(BackendFallbackReason::UnsupportedAxisScale)
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn explicit_datashader_reversed_manual_limits_fall_back_and_save_png() {
    let output_dir = tempfile::tempdir().expect("temporary output directory should exist");
    let output = output_dir.path().join("reversed-datashader.png");
    let plot = Plot::new()
        .backend(BackendType::DataShader)
        .xlim(4.0, 0.0)
        .ylim(4.0, 0.0)
        .scatter(&[0.0, 1.0, 2.0, 3.0, 4.0], &[0.0, 1.0, 4.0, 2.0, 3.0])
        .into_plot();

    let resolution = plot.backend_resolution(BackendOperation::Png);
    assert_eq!(resolution.actual_backend(), BackendType::Skia);
    assert_eq!(
        resolution.fallback_reason(),
        Some(BackendFallbackReason::ReversedAxisLimits)
    );

    let (_, backend, diagnostics, executed_resolution) = plot
        .benchmark_save_png_bytes_with_backend_resolution()
        .expect("reversed axes should execute through the Skia PNG path");
    assert_eq!(backend, "skia");
    assert_eq!(diagnostics.actual_backend(), BackendType::Skia);
    assert_eq!(diagnostics.render_mode, "reference");
    assert!(!diagnostics.used_auto_datashader);
    assert_eq!(executed_resolution, resolution);

    plot.save(&output)
        .expect("reversed-axis fallback should save a PNG successfully");
    let png = std::fs::read(output).expect("saved PNG should be readable");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    image::load_from_memory(&png).expect("saved output should decode as PNG");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn explicit_datashader_scatter_error_variants_fall_back_with_full_skia_output() {
    let x = [0.0, 1.0, 2.0, 3.0];
    let y = [1.0, 3.0, 2.0, 4.0];
    let symmetric_x = [0.1, 0.2, 0.15, 0.25];
    let symmetric_y = [0.3, 0.4, 0.2, 0.35];
    let lower = [0.1, 0.2, 0.1, 0.15];
    let upper = [0.2, 0.3, 0.25, 0.4];

    let cases = [
        (
            "symmetric y errors",
            Plot::new()
                .backend(BackendType::DataShader)
                .scatter(&x, &y)
                .with_yerr(&symmetric_y)
                .into_plot(),
            Plot::new()
                .scatter(&x, &y)
                .with_yerr(&symmetric_y)
                .into_plot(),
        ),
        (
            "symmetric x errors",
            Plot::new()
                .backend(BackendType::DataShader)
                .scatter(&x, &y)
                .with_xerr(&symmetric_x)
                .into_plot(),
            Plot::new()
                .scatter(&x, &y)
                .with_xerr(&symmetric_x)
                .into_plot(),
        ),
        (
            "combined x and y errors",
            Plot::new()
                .backend(BackendType::DataShader)
                .scatter(&x, &y)
                .with_xerr(&symmetric_x)
                .with_yerr(&symmetric_y)
                .into_plot(),
            Plot::new()
                .scatter(&x, &y)
                .with_xerr(&symmetric_x)
                .with_yerr(&symmetric_y)
                .into_plot(),
        ),
        (
            "asymmetric y errors",
            Plot::new()
                .backend(BackendType::DataShader)
                .scatter(&x, &y)
                .with_yerr_asymmetric(&lower, &upper)
                .into_plot(),
            Plot::new()
                .scatter(&x, &y)
                .with_yerr_asymmetric(&lower, &upper)
                .into_plot(),
        ),
        (
            "asymmetric x errors",
            Plot::new()
                .backend(BackendType::DataShader)
                .scatter(&x, &y)
                .with_xerr_asymmetric(&lower, &upper)
                .into_plot(),
            Plot::new()
                .scatter(&x, &y)
                .with_xerr_asymmetric(&lower, &upper)
                .into_plot(),
        ),
    ];

    for (name, fallback_plot, reference_plot) in cases {
        let resolution = fallback_plot.backend_resolution(BackendOperation::Png);
        assert_eq!(resolution.actual_backend(), BackendType::Skia, "{name}");
        assert_eq!(
            resolution.fallback_reason(),
            Some(BackendFallbackReason::UnsupportedSeries),
            "{name}"
        );

        let (fallback_png, backend, diagnostics, executed_resolution) = fallback_plot
            .benchmark_save_png_bytes_with_backend_resolution()
            .unwrap_or_else(|error| panic!("{name} fallback should render: {error}"));
        let (reference_png, reference_backend, reference_diagnostics) = reference_plot
            .benchmark_save_png_bytes_with_diagnostics()
            .unwrap_or_else(|error| panic!("{name} reference should render: {error}"));

        assert_eq!(backend, "skia", "{name}");
        assert_eq!(reference_backend, "skia", "{name}");
        assert_eq!(diagnostics.actual_backend(), BackendType::Skia, "{name}");
        assert_eq!(diagnostics.render_mode, "reference", "{name}");
        assert!(!diagnostics.used_auto_datashader, "{name}");
        assert_eq!(executed_resolution, resolution, "{name}");
        assert_eq!(
            reference_diagnostics.actual_backend(),
            BackendType::Skia,
            "{name}"
        );
        assert_eq!(
            fallback_png, reference_png,
            "{name} should preserve error bars"
        );
    }
}

#[test]
fn explicit_parallel_backend_reports_truthful_fallback() {
    let plot = Plot::new()
        .backend(BackendType::Parallel)
        .line(&[0.0, 1.0], &[0.0, 1.0])
        .into_plot();
    let resolution = plot.backend_resolution(BackendOperation::Png);

    assert_eq!(resolution.requested_backend(), Some(BackendType::Parallel));
    assert_eq!(resolution.actual_backend(), BackendType::Skia);
    // There is no 2D series-parallel raster path in any build configuration, so
    // the reason is the same with and without the `parallel` cargo feature.
    assert_eq!(
        resolution.fallback_reason(),
        Some(BackendFallbackReason::UnsupportedOperation)
    );
}

#[test]
fn explicit_gpu_backend_reports_truthful_fallback() {
    let plot = Plot::new()
        .backend(BackendType::GPU)
        .scatter(&[0.0, 1.0], &[0.0, 1.0])
        .into_plot();
    for operation in [
        BackendOperation::RasterImage,
        BackendOperation::Png,
        BackendOperation::Interactive,
    ] {
        let resolution = plot.backend_resolution(operation);
        assert_eq!(resolution.requested_backend(), Some(BackendType::GPU));
        assert_eq!(resolution.actual_backend(), BackendType::Skia);
        #[cfg(feature = "gpu")]
        assert_eq!(
            resolution.fallback_reason(),
            Some(BackendFallbackReason::UnsupportedOperation)
        );
        #[cfg(not(feature = "gpu"))]
        assert_eq!(
            resolution.fallback_reason(),
            Some(BackendFallbackReason::FeatureDisabled)
        );
    }
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn explicit_datashader_reports_operation_and_series_fallbacks() {
    let scatter = Plot::new()
        .backend(BackendType::DataShader)
        .scatter(&[0.0, 1.0], &[0.0, 1.0])
        .into_plot();
    let raster_resolution = scatter.backend_resolution(BackendOperation::RasterImage);
    assert_eq!(raster_resolution.actual_backend(), BackendType::Skia);
    assert_eq!(
        raster_resolution.fallback_reason(),
        Some(BackendFallbackReason::UnsupportedOperation)
    );

    let line = Plot::new()
        .backend(BackendType::DataShader)
        .line(&[0.0, 1.0], &[0.0, 1.0])
        .into_plot();
    let png_resolution = line.backend_resolution(BackendOperation::Png);
    assert_eq!(png_resolution.actual_backend(), BackendType::Skia);
    assert_eq!(
        png_resolution.fallback_reason(),
        Some(BackendFallbackReason::UnsupportedSeries)
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn explicit_datashader_reports_mixed_coordinate_fallback() {
    let plot = Plot::new()
        .backend(BackendType::DataShader)
        .scatter(&[0.0, 1.0], &[0.0, 1.0])
        .polar_line(&[1.0, 2.0], &[0.0, std::f64::consts::PI])
        .into_plot();
    let resolution = plot.backend_resolution(BackendOperation::Png);

    assert_eq!(resolution.actual_backend(), BackendType::Skia);
    assert_eq!(
        resolution.fallback_reason(),
        Some(BackendFallbackReason::MixedCoordinateSystems)
    );
}

#[test]
fn prepared_plot_diagnostics_match_raster_image_resolution() {
    let plot = Plot::new()
        .backend(BackendType::DataShader)
        .scatter(&[0.0, 1.0], &[0.0, 1.0])
        .into_plot();
    let resolution = plot.backend_resolution(BackendOperation::RasterImage);
    let (_, diagnostics) = plot
        .prepare()
        .render_png_bytes_uncached_with_diagnostics()
        .expect("prepared PNG bytes should use the raster-image policy");

    assert_eq!(resolution.actual_backend(), BackendType::Skia);
    assert_eq!(
        resolution.fallback_reason(),
        Some(BackendFallbackReason::UnsupportedOperation)
    );
    assert_eq!(diagnostics.actual_backend(), resolution.actual_backend());
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn backend_resolution_and_diagnostics_agree_on_executed_renderer() {
    let plot = Plot::new()
        .backend(BackendType::Parallel)
        .line(&[0.0, 1.0], &[0.0, 1.0])
        .into_plot();
    let (_, backend, diagnostics, resolution) = plot
        .benchmark_save_png_bytes_with_backend_resolution()
        .expect("explicit parallel preference should fall back to a Skia PNG");

    assert_eq!(backend, "skia");
    assert_eq!(diagnostics.actual_backend(), BackendType::Skia);
    assert_eq!(resolution.actual_backend(), diagnostics.actual_backend());
    assert!(resolution.used_fallback());
}

mod legend_customization {
    use super::*;

    const LABEL: &str = "Frame-free legend";
    const FONT_SIZE: f32 = 14.0;
    const X: [f64; 2] = [0.1, 0.9];
    const Y: [f64; 2] = [0.1, 0.2];

    fn base_plot() -> Plot {
        Plot::new()
            .size(8.0, 5.0)
            .dpi(72)
            .grid(false)
            .ticks(false)
            .xlim(0.0, 1.0)
            .ylim(0.0, 1.0)
            .legend_font_size(FONT_SIZE)
    }

    fn labeled_plot() -> Plot {
        // Keep the data well below the upper-right legend so pixel differences
        // in that region come only from the frame, handle and label.
        base_plot()
            .line(&X, &Y)
            .color(Color::RED)
            .line_width(2.0)
            .label(LABEL)
            .into_plot()
    }

    fn attribute(element: &str, name: &str) -> f32 {
        let prefix = format!(" {name}=\"");
        let value = element
            .split_once(&prefix)
            .unwrap_or_else(|| panic!("missing {name} attribute: {element}"))
            .1
            .split('"')
            .next()
            .expect("attribute value");
        value
            .parse()
            .unwrap_or_else(|_| panic!("non-numeric {name} attribute: {element}"))
    }

    fn frame_element(svg: &str) -> &str {
        // This line-only fixture has no other rounded rectangles.
        svg.lines()
            .find(|line| line.trim_start().starts_with("<rect ") && line.contains(" rx=\""))
            .unwrap_or_else(|| panic!("missing legend frame: {svg}"))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn exported_png(plot: Plot, name: &str) -> image::RgbaImage {
        let directory = tempfile::tempdir().expect("temporary PNG directory should exist");
        let path = directory.path().join(format!("{name}.png"));
        plot.save(&path).expect("legend plot should save as PNG");
        image::open(path)
            .expect("exported legend PNG should decode")
            .to_rgba8()
    }

    #[test]
    fn frameless_style_omits_svg_frame_and_keeps_labels() {
        let disabled = labeled_plot().render_to_svg().unwrap();
        let framed = labeled_plot()
            .legend_position(LegendPosition::UpperRight)
            .render_to_svg()
            .unwrap();
        let frameless = base_plot()
            .line(&X, &Y)
            .legend_style(LegendStyle::invisible())
            .legend_spacing(LegendSpacing::default())
            .color(Color::RED)
            .line_width(2.0)
            .label(LABEL)
            .legend_position(LegendPosition::UpperRight)
            .render_to_svg()
            .unwrap();

        assert!(!disabled.contains(LABEL));
        assert!(framed.contains(LABEL));
        assert!(
            frameless.contains(LABEL),
            "hiding the frame must retain its label"
        );
        assert_eq!(
            frameless.matches("<rect ").count(),
            disabled.matches("<rect ").count(),
            "a frameless line legend must emit no frame rectangles"
        );
        assert_eq!(
            framed.matches("<rect ").count(),
            frameless.matches("<rect ").count() + 2,
            "the default legend adds a face and an edge"
        );
        assert_eq!(
            frameless.matches("<line ").count(),
            disabled.matches("<line ").count() + 1,
            "the line handle must survive alongside the label"
        );
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn frameless_style_omits_png_frame_and_fill_and_keeps_label_pixels() {
        // A tinted canvas makes even the default white frame fill detectable.
        let theme = Theme {
            background: Color::from_rgb(215, 230, 245),
            ..Theme::default()
        };
        let framed_plot = labeled_plot()
            .theme(theme.clone())
            .legend_style(LegendStyle::default())
            .legend_position(LegendPosition::UpperRight);
        let svg = framed_plot.render_to_svg().unwrap();
        let frame = frame_element(&svg);
        let left = attribute(frame, "x").ceil() as u32;
        let top = attribute(frame, "y").ceil() as u32;
        let right = (attribute(frame, "x") + attribute(frame, "width")).floor() as u32;
        let bottom = (attribute(frame, "y") + attribute(frame, "height")).floor() as u32;
        let label = svg
            .lines()
            .find(|line| line.contains(&format!(">{LABEL}<")))
            .expect("legend label should be SVG text");
        let label_left = attribute(label, "x").ceil() as u32;

        let disabled = exported_png(labeled_plot().theme(theme.clone()), "disabled");
        let framed = exported_png(framed_plot, "framed");
        let frameless = exported_png(
            labeled_plot()
                .theme(theme)
                .legend_style(LegendStyle::invisible())
                .legend_position(LegendPosition::UpperRight),
            "frameless",
        );
        assert_eq!(disabled.dimensions(), framed.dimensions());
        assert_eq!(disabled.dimensions(), frameless.dimensions());
        assert!(right < framed.width() && bottom < framed.height());

        let mut visible_frame_pixels = 0;
        let mut label_pixels = 0;
        // The outer four pixels lie within the configured border padding,
        // outside the label and handle. They sample both edge and face paint.
        for y in top.saturating_sub(2)..=bottom + 2 {
            for x in left.saturating_sub(2)..=right + 2 {
                let in_frame_band = x < left + 4 || x > right - 4 || y < top + 4 || y > bottom - 4;
                if in_frame_band {
                    assert_eq!(
                        frameless.get_pixel(x, y),
                        disabled.get_pixel(x, y),
                        "frameless PNG retained frame/fill paint at ({x}, {y})"
                    );
                    if framed.get_pixel(x, y) != disabled.get_pixel(x, y) {
                        visible_frame_pixels += 1;
                    }
                }
                if x >= label_left
                    && x <= right
                    && y >= top
                    && y <= bottom
                    && frameless.get_pixel(x, y) != disabled.get_pixel(x, y)
                {
                    label_pixels += 1;
                }
            }
        }
        assert!(
            visible_frame_pixels > 100,
            "the framed control must visibly paint its frame"
        );
        assert!(
            label_pixels > 20,
            "the frameless PNG must retain label pixels"
        );
    }

    #[test]
    fn shorter_handle_length_reduces_measured_svg_legend_width() {
        let default = labeled_plot()
            .legend_position(LegendPosition::UpperRight)
            .render_to_svg()
            .unwrap();
        let spacing = LegendSpacing {
            handle_length: 0.5,
            ..LegendSpacing::default()
        };
        let plot_api = labeled_plot()
            .legend_spacing(spacing)
            .legend_position(LegendPosition::UpperRight)
            .render_to_svg()
            .unwrap();
        let builder_api = base_plot()
            .line(&X, &Y)
            .legend_spacing(spacing)
            .color(Color::RED)
            .line_width(2.0)
            .label(LABEL)
            .legend_position(LegendPosition::UpperRight)
            .render_to_svg()
            .unwrap();
        assert_eq!(
            plot_api, builder_api,
            "Plot and PlotBuilder must apply identical spacing"
        );

        let default_frame = frame_element(&default);
        let short_frame = frame_element(&plot_api);
        let width_reduction = attribute(default_frame, "width") - attribute(short_frame, "width");
        let expected = (LegendSpacing::default().handle_length - spacing.handle_length) * FONT_SIZE;
        assert!(
            (width_reduction - expected).abs() < 0.02,
            "handle length should reduce measured frame width by {expected}, got {width_reduction}"
        );
        assert_eq!(
            attribute(default_frame, "height"),
            attribute(short_frame, "height")
        );
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn explicit_themed_defaults_preserve_svg_and_exported_png() {
        let theme = Theme::default();
        // The existing high-level legend inherits its colors from the theme.
        // Explicit styles own their colors, so spell out those same defaults.
        let style = LegendStyle::default()
            .face_color(theme.background)
            .edge_color(Some(theme.grid_color));
        let baseline = labeled_plot().legend_position(LegendPosition::UpperRight);
        let spacing_only = labeled_plot()
            .legend_spacing(LegendSpacing::default())
            .legend_position(LegendPosition::UpperRight);
        let plot_api = labeled_plot()
            .legend_style(style.clone())
            .legend_spacing(LegendSpacing::default())
            .legend_position(LegendPosition::UpperRight);
        let builder_api = base_plot()
            .line(&X, &Y)
            .legend_style(style)
            .legend_spacing(LegendSpacing::default())
            .color(Color::RED)
            .line_width(2.0)
            .label(LABEL)
            .legend_position(LegendPosition::UpperRight)
            .into_plot();
        let baseline_svg = baseline.render_to_svg().unwrap();
        assert_eq!(baseline_svg, spacing_only.render_to_svg().unwrap());
        assert_eq!(baseline_svg, plot_api.render_to_svg().unwrap());
        assert_eq!(baseline_svg, builder_api.render_to_svg().unwrap());

        let baseline_png = exported_png(baseline, "default-baseline");
        assert_eq!(
            baseline_png,
            exported_png(spacing_only, "default-spacing-only")
        );
        assert_eq!(baseline_png, exported_png(plot_api, "default-plot-api"));
        assert_eq!(
            baseline_png,
            exported_png(builder_api, "default-builder-api")
        );
    }

    #[test]
    fn corner_radius_overrides_custom_style_in_either_setter_order() {
        let style = LegendStyle::rounded(1.0)
            .alpha(1.0)
            .face_color(Color::from_rgb(17, 93, 151))
            .edge_color(Some(Color::from_rgb(211, 37, 89)));
        let radius_first = labeled_plot()
            .legend_corner_radius(9.0)
            .legend_style(style.clone())
            .legend_position(LegendPosition::UpperRight)
            .render_to_svg()
            .unwrap();
        let style_first = base_plot()
            .line(&X, &Y)
            .legend_style(style)
            .legend_corner_radius(9.0)
            .color(Color::RED)
            .line_width(2.0)
            .label(LABEL)
            .legend_position(LegendPosition::UpperRight)
            .render_to_svg()
            .unwrap();

        assert_eq!(radius_first, style_first);
        assert_eq!(attribute(frame_element(&radius_first), "rx"), 9.0);
        assert!(radius_first.contains("fill=\"rgb(17,93,151)\""));
        assert!(radius_first.contains("stroke=\"rgb(211,37,89)\""));
    }
}
