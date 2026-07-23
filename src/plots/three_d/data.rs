use std::sync::Arc;

use crate::core::{PlottingError, Result};
use crate::data::{NumericData1D, NumericData2D, collect_numeric_data_2d};

/// Owned x/y/z coordinates shared by a resolved 3D point or line series.
#[derive(Clone, Debug)]
pub(crate) struct Points3DData {
    pub(crate) x: Arc<[f64]>,
    pub(crate) y: Arc<[f64]>,
    pub(crate) z: Arc<[f64]>,
}

impl Points3DData {
    pub(crate) fn collect<X, Y, Z>(
        series_name: &'static str,
        minimum_points: usize,
        x: &X,
        y: &Y,
        z: &Z,
    ) -> Result<Self>
    where
        X: NumericData1D + ?Sized,
        Y: NumericData1D + ?Sized,
        Z: NumericData1D + ?Sized,
    {
        let x = x.try_collect_f64()?;
        let y = y.try_collect_f64()?;
        let z = z.try_collect_f64()?;

        if x.len() != y.len() || x.len() != z.len() {
            return Err(PlottingError::DataLengthMismatch3D {
                operation: series_name,
                x_len: x.len(),
                y_len: y.len(),
                z_len: z.len(),
                series_index: None,
            });
        }
        if x.len() < minimum_points {
            return Err(PlottingError::InvalidInput(format!(
                "{}: data must contain at least {} point{}",
                series_name,
                minimum_points,
                if minimum_points == 1 { "" } else { "s" }
            )));
        }

        validate_coordinate_infinities(series_name, "x", &x)?;
        validate_coordinate_infinities(series_name, "y", &y)?;
        validate_coordinate_infinities(series_name, "z", &z)?;

        if !(0..x.len())
            .any(|index| x[index].is_finite() && y[index].is_finite() && z[index].is_finite())
        {
            return Err(PlottingError::InvalidInput(format!(
                "{}: data must contain at least one finite x/y/z point",
                series_name
            )));
        }

        Ok(Self {
            x: x.into(),
            y: y.into(),
            z: z.into(),
        })
    }
}

/// Owned regular grid where z rows correspond to y and z columns to x.
#[derive(Clone, Debug)]
pub(crate) struct Grid3DData {
    pub(crate) operation: &'static str,
    pub(crate) x: Arc<[f64]>,
    pub(crate) y: Arc<[f64]>,
    pub(crate) z: Arc<[f64]>,
    pub(crate) rows: usize,
    pub(crate) columns: usize,
}

impl Grid3DData {
    pub(crate) fn collect<X, Y, Z>(operation: &'static str, x: &X, y: &Y, z: &Z) -> Result<Self>
    where
        X: NumericData1D + ?Sized,
        Y: NumericData1D + ?Sized,
        Z: NumericData2D + ?Sized,
    {
        let x = x.try_collect_f64()?;
        let y = y.try_collect_f64()?;
        if x.len() < 2 || y.len() < 2 {
            return Err(PlottingError::InvalidTopology3D {
                reason: format!(
                    "{} x and y must each contain at least 2 values (x={}, y={})",
                    operation,
                    x.len(),
                    y.len()
                ),
            });
        }

        let (actual_rows, actual_columns) = z.shape();
        if actual_rows != y.len() || actual_columns != x.len() {
            return Err(PlottingError::GridShapeMismatch {
                operation,
                expected_rows: y.len(),
                expected_columns: x.len(),
                actual_rows,
                actual_columns,
            });
        }

        let (z, rows, columns) = collect_numeric_data_2d(z).map_err(|error| match error {
            PlottingError::RaggedData2D {
                row,
                expected_columns,
                actual_columns,
                ..
            } => PlottingError::RaggedData2D {
                context: operation,
                row,
                expected_columns,
                actual_columns,
            },
            other => other,
        })?;
        validate_coordinate_infinities(operation, "x", &x)?;
        validate_coordinate_infinities(operation, "y", &y)?;
        validate_coordinate_infinities(operation, "z", &z)?;

        if !z.iter().any(|value| value.is_finite()) {
            return Err(PlottingError::InvalidTopology3D {
                reason: format!("{operation} z grid must contain at least one finite value"),
            });
        }

        Ok(Self {
            operation,
            x: x.into(),
            y: y.into(),
            z: z.into(),
            rows,
            columns,
        })
    }

    pub(crate) fn triangle_capacity(&self) -> Result<usize> {
        self.rows
            .saturating_sub(1)
            .checked_mul(self.columns.saturating_sub(1))
            .and_then(|cells| cells.checked_mul(2))
            .ok_or_else(|| PlottingError::InvalidTopology3D {
                reason: format!(
                    "{} shape {}x{} overflows triangle capacity",
                    self.operation, self.rows, self.columns
                ),
            })
    }
}

fn validate_coordinate_infinities(
    series_name: &'static str,
    coordinate: &'static str,
    values: &[f64],
) -> Result<()> {
    if let Some((index, value)) = values
        .iter()
        .copied()
        .enumerate()
        .find(|(_, value)| value.is_infinite())
    {
        return Err(PlottingError::InvalidData {
            message: format!(
                "{}: {} contains an infinite value ({})",
                series_name, coordinate, value
            ),
            position: Some(index),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_data_rejects_mismatched_lengths() {
        let error = Points3DData::collect("scatter3d", 1, &[0.0, 1.0], &[0.0], &[0.0, 1.0])
            .expect_err("length mismatch must fail");
        assert!(matches!(
            error,
            PlottingError::DataLengthMismatch3D {
                x_len: 2,
                y_len: 1,
                z_len: 2,
                ..
            }
        ));
    }

    #[test]
    fn surface_shape_is_y_rows_by_x_columns() {
        let x = [0.0, 1.0, 2.0];
        let y = [0.0, 1.0];
        let z = vec![vec![0.0, 1.0], vec![1.0, 2.0], vec![2.0, 3.0]];
        let error =
            Grid3DData::collect("surface", &x, &y, &z).expect_err("transposed grid must fail");
        assert!(matches!(
            error,
            PlottingError::GridShapeMismatch {
                operation: "surface",
                expected_rows: 2,
                expected_columns: 3,
                actual_rows: 3,
                actual_columns: 2,
            }
        ));
    }

    #[test]
    fn regular_grid_reports_triangle_capacity() {
        let x = [0.0, 1.0, 2.0];
        let y = [0.0, 1.0];
        let z = vec![vec![0.0, 1.0, 2.0], vec![1.0, 2.0, 3.0]];
        let grid = Grid3DData::collect("surface", &x, &y, &z).expect("valid grid");
        assert_eq!(grid.triangle_capacity().expect("capacity"), 4);
    }
}
