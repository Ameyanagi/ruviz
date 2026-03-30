#[cfg(feature = "polars_support")]
use ruviz::core::PlottingError;
#[cfg(any(
    feature = "ndarray_support",
    feature = "polars_support",
    feature = "nalgebra_support",
    feature = "nalgebra"
))]
use ruviz::prelude::*;

#[cfg(feature = "ndarray_support")]
#[test]
fn test_ndarray_view_line_and_heatmap() {
    use ndarray::{Array1, Array2};

    let x = Array1::linspace(0.0, 4.0, 5);
    let y = x.mapv(|v| v * v);

    let line_result = Plot::new().line(&x.view(), &y.view()).render();
    assert!(line_result.is_ok(), "ndarray ArrayView1 line failed");

    let matrix = Array2::from_shape_vec((2, 3), vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
    let heatmap_result = Plot::new().heatmap(&matrix.view(), None).render();
    assert!(heatmap_result.is_ok(), "ndarray ArrayView2 heatmap failed");
}

#[cfg(any(feature = "nalgebra_support", feature = "nalgebra"))]
#[test]
fn test_nalgebra_vector_and_matrix() {
    let x = nalgebra::DVector::from_vec(vec![0.0, 1.0, 2.0, 3.0]);
    let y = nalgebra::SVector::<f64, 4>::new(0.0, 1.0, 4.0, 9.0);

    let line_result = Plot::new().line(&x, &y).render();
    assert!(line_result.is_ok(), "nalgebra vector line failed");

    let matrix = nalgebra::DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);
    let heatmap_result = Plot::new().heatmap(&matrix, None).render();
    assert!(heatmap_result.is_ok(), "nalgebra matrix heatmap failed");
}

#[cfg(feature = "polars_support")]
#[test]
fn test_polars_series_and_chunked_input() {
    use polars::prelude::*;

    let df = df! {
        "x" => [1.0, 2.0, 3.0, 4.0],
        "y" => [2.0, 3.0, 5.0, 8.0],
    }
    .unwrap();

    let x_column = df.column("x").unwrap();
    let y_column = df.column("y").unwrap();
    let x_series = x_column.as_materialized_series();
    let y_series = y_column.as_materialized_series();

    let series_result = Plot::new().line(x_series, y_series).render();
    assert!(series_result.is_ok(), "polars Series ingestion failed");

    let x_chunked = x_column.f64().unwrap();
    let y_chunked = y_column.f64().unwrap();

    let chunked_result = Plot::new().scatter(x_chunked, y_chunked).render();
    assert!(
        chunked_result.is_ok(),
        "polars Float64Chunked ingestion failed"
    );
}

#[cfg(feature = "polars_support")]
#[test]
fn test_polars_null_policy_default_error() {
    use polars::prelude::*;

    let x = Series::new("x".into(), &[Some(1.0), None, Some(3.0)]);
    let y = Series::new("y".into(), &[Some(2.0), Some(4.0), Some(6.0)]);

    let result = Plot::new().line(&x, &y).render();
    assert!(result.is_err(), "strict null policy should reject nulls");

    let err = result.unwrap_err();
    assert!(
        matches!(err, PlottingError::NullValueNotAllowed { .. }),
        "unexpected error type: {err:?}"
    );
}

#[cfg(feature = "polars_support")]
#[test]
fn test_polars_null_policy_drop() {
    use polars::prelude::*;

    let x = Series::new("x".into(), &[Some(1.0), None, Some(3.0)]);
    let y = Series::new("y".into(), &[Some(2.0), None, Some(6.0)]);

    let result = Plot::new()
        .null_policy(NullPolicy::Drop)
        .line(&x, &y)
        .render();

    assert!(result.is_ok(), "drop null policy should allow plotting");
}
