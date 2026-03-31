use super::*;

#[cfg(feature = "typst-math")]
#[test]
fn test_typst_toggle_mode_switch() {
    let plot = Plot::new().typst(true);
    assert_eq!(plot.display.text_engine, TextEngineMode::Typst);

    let plot = plot.typst(false);
    assert_eq!(plot.display.text_engine, TextEngineMode::Plain);
}

#[cfg(feature = "typst-math")]
#[test]
fn test_plot_builder_typst_forwarding() {
    let x = vec![0.0, 1.0, 2.0];
    let y = vec![1.0, 2.0, 3.0];

    let plot: Plot = Plot::new().line(&x, &y).typst(true).into();
    assert_eq!(plot.display.text_engine, TextEngineMode::Typst);
}

#[cfg(feature = "typst-math")]
#[test]
fn test_plot_series_builder_typst_forwarding() {
    let x = vec![0.0, 1.0, 2.0];
    let y = vec![1.0, 2.0, 3.0];

    let plot = Plot::new()
        .line(&x, &y)
        .label("Series")
        .typst(true)
        .end_series();
    assert_eq!(plot.display.text_engine, TextEngineMode::Typst);
}

#[cfg(feature = "typst-math")]
#[test]
fn test_invalid_typst_snippet_returns_typst_error() {
    let x = vec![0.0, 1.0, 2.0];
    let y = vec![1.0, 2.0, 3.0];

    let result = Plot::new()
        .line(&x, &y)
        .title("#let broken =")
        .typst(true)
        .render();

    assert!(matches!(result, Err(PlottingError::TypstError(_))));
}
