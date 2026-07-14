use ruviz::core::{BackendFallbackReason, BackendOperation};
use ruviz::prelude::*;

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
#[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
fn public_parallel_render_honors_pending_ingestion_errors() {
    let x: Vec<f64> = (0..5_000).map(|index| index as f64).collect();
    let y: Vec<f64> = x.iter().map(|value| value.sin()).collect();
    let bad_x = vec![0.0, 1.0];
    let bad_y = vec![0.0];

    let err = Plot::new()
        .parallel_threshold(1)
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
        .width(0.7)
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
        .scale(0.25)
        .width(1.2)
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
#[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
fn high_level_quiver_api_stays_on_reference_renderer_with_parallel_available() {
    let x: Vec<f64> = (0..2_500).map(|index| (index % 50) as f64).collect();
    let y: Vec<f64> = (0..2_500).map(|index| (index / 50) as f64).collect();
    let u: Vec<f64> = x.iter().map(|value| (value * 0.1).sin()).collect();
    let v: Vec<f64> = y.iter().map(|value| (value * 0.1).cos()).collect();

    let (_, diagnostics) = Plot::new()
        .parallel_threshold(1)
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
    #[cfg(feature = "parallel")]
    assert_eq!(
        resolution.fallback_reason(),
        Some(BackendFallbackReason::UnsupportedOperation)
    );
    #[cfg(not(feature = "parallel"))]
    assert_eq!(
        resolution.fallback_reason(),
        Some(BackendFallbackReason::FeatureDisabled)
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
