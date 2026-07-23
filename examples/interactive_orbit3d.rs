use ruviz::prelude::*;

fn main() -> Result<()> {
    let points = 2_000;
    let t: Vec<f64> = (0..points)
        .map(|index| index as f64 * 12.0 * std::f64::consts::TAU / points as f64)
        .collect();
    let x: Vec<f64> = t.iter().copied().map(f64::cos).collect();
    let y: Vec<f64> = t.iter().copied().map(f64::sin).collect();
    let z: Vec<f64> = t.iter().map(|value| value / 20.0).collect();

    line3d(&x, &y, &z)
        .title("Drag to orbit · scroll to zoom · Esc to reset")
        .xlabel("x")
        .ylabel("y")
        .zlabel("z")
        .show()
}
