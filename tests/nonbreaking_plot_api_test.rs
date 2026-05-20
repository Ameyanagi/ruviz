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
