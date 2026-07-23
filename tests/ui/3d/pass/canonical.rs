use ruviz::prelude::*;

fn main() -> ruviz::core::Result<()> {
    let x = vec![0_f32, 1.0, 2.0];
    let y = [0_f64, 1.0, 2.0];
    let z = [0_i32, 1, 2];
    scatter3d(&x, &y, &z)
        .title("scatter")
        .camera(Camera3D::default().azimuth_deg(30.0))
        .line3d(&x, &y, &z)
        .validate()?;

    let grid_x = [0.0, 1.0, 2.0];
    let grid_y = [0.0, 1.0];
    let grid_z = [[0.0, 1.0, 2.0], [1.0, 2.0, 3.0]];
    surface(&grid_x, &grid_y, &grid_z)
        .xlim(0.0, 2.0)
        .ylim(0.0, 1.0)
        .zlim(0.0, 3.0)
        .wireframe(&grid_x, &grid_y, &grid_z)
        .validate()
}

#[allow(dead_code)]
fn terminal_contract() -> ruviz::core::Result<()> {
    let values = [0.0, 1.0];
    let _ = scatter3d(&values, &values, &values).render()?;
    let _ = scatter3d(&values, &values, &values).render_png_bytes()?;
    let _ = scatter3d(&values, &values, &values).render_to_svg()?;
    #[cfg(not(target_arch = "wasm32"))]
    {
        scatter3d(&values, &values, &values).save("plot.png")?;
        let _ = scatter3d(&values, &values, &values).show();
    }
    Ok(())
}
