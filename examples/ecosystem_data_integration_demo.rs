use ruviz::prelude::*;

#[cfg(any(feature = "nalgebra_support", feature = "nalgebra"))]
use nalgebra::{DMatrix, DVector};
#[cfg(feature = "ndarray_support")]
use ndarray::{Array1, Array2};
#[cfg(feature = "polars_support")]
use polars::prelude::*;

mod util;

fn main() -> Result<()> {
    #[allow(unused_mut)]
    let mut outputs: Vec<std::path::PathBuf> = Vec::new();

    #[cfg(feature = "ndarray_support")]
    {
        let x = Array1::linspace(0.0, 4.0, 5);
        let y = x.mapv(|v| v * v);
        let line_path = util::example_output_path("ecosystem_ndarray_line.png");
        Plot::new()
            .line(&x.view(), &y.view())
            .title("ndarray line")
            .save(line_path.to_string_lossy().as_ref())?;
        outputs.push(line_path);

        let z = Array2::from_shape_vec((2, 3), vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
        let heatmap_path = util::example_output_path("ecosystem_ndarray_heatmap.png");
        Plot::new()
            .heatmap(&z.view(), None)
            .title("ndarray heatmap")
            .save(heatmap_path.to_string_lossy().as_ref())?;
        outputs.push(heatmap_path);
    }

    #[cfg(any(feature = "nalgebra_support", feature = "nalgebra"))]
    {
        let x = DVector::from_vec(vec![0.0, 1.0, 2.0, 3.0]);
        let y = DVector::from_vec(vec![0.0, 1.0, 4.0, 9.0]);
        let line_path = util::example_output_path("ecosystem_nalgebra_line.png");
        Plot::new()
            .line(&x, &y)
            .title("nalgebra line")
            .save(line_path.to_string_lossy().as_ref())?;
        outputs.push(line_path);

        let z = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        let heatmap_path = util::example_output_path("ecosystem_nalgebra_heatmap.png");
        Plot::new()
            .heatmap(&z, None)
            .title("nalgebra heatmap")
            .save(heatmap_path.to_string_lossy().as_ref())?;
        outputs.push(heatmap_path);
    }

    #[cfg(feature = "polars_support")]
    {
        let x = Series::new("x", &[Some(1.0), None, Some(3.0), Some(4.0)]);
        let y = Series::new("y", &[Some(2.0), None, Some(6.0), Some(8.0)]);

        let strict_is_error = Plot::new().line(&x, &y).render().is_err();
        println!("polars strict null policy errors: {strict_is_error}");

        let line_path = util::example_output_path("ecosystem_polars_drop_line.png");
        Plot::new()
            .null_policy(NullPolicy::Drop)
            .line(&x, &y)
            .title("polars line (NullPolicy::Drop)")
            .save(line_path.to_string_lossy().as_ref())?;
        outputs.push(line_path);
    }

    if outputs.is_empty() {
        println!(
            "No ecosystem feature enabled. Run with --features \"ndarray_support polars_support nalgebra_support\"."
        );
    } else {
        println!("Generated files:");
        for path in outputs {
            println!("  {}", path.display());
        }
    }

    Ok(())
}
