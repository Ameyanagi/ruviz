use ruviz::core::PlottingError;
use ruviz::prelude::*;

#[test]
fn render_rejects_oversized_dimensions_before_rendering() {
    let err = Plot::new()
        .size_px(20_000, 20_000)
        .line(&[0.0, 1.0], &[1.0, 2.0])
        .render()
        .expect_err("oversized dimensions should fail validation");

    assert!(matches!(
        err,
        PlottingError::PerformanceLimit {
            ref limit_type,
            actual: 20_000,
            maximum: 16_384,
        } if limit_type == "Image dimension"
    ));
}

#[test]
fn render_rejects_excessive_dpi_before_rendering() {
    let err = Plot::new()
        .dpi(5_000)
        .line(&[0.0, 1.0], &[1.0, 2.0])
        .render()
        .expect_err("excessive DPI should fail validation");

    assert!(matches!(
        err,
        PlottingError::PerformanceLimit {
            ref limit_type,
            actual: 5_000,
            maximum: 2_400,
        } if limit_type == "DPI"
    ));
}

#[test]
fn render_rejects_fill_between_length_mismatch() {
    let err = Plot::new()
        .line(&[0.0, 1.0, 2.0], &[1.0, 2.0, 3.0])
        .fill_between(&[0.0, 1.0, 2.0], &[0.5, 1.5], &[1.0, 2.0, 3.0])
        .render()
        .expect_err("mismatched fill_between inputs should fail validation");

    assert!(matches!(err, PlottingError::DataLengthMismatch { .. }));
}

#[test]
fn render_rejects_nan_data_before_bounds_calculation() {
    let err = Plot::new()
        .line(&[0.0, f64::NAN], &[1.0, 2.0])
        .render()
        .expect_err("NaN data should fail validation");

    assert!(matches!(err, PlottingError::InvalidData { .. }));
}
