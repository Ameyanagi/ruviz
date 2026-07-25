//! Minimal 3D line plot.
//!
//! Run with:
//! `cargo run --example doc_line3d --features 3d`

use ruviz::prelude::*;

fn main() -> PlotResult<()> {
    let t: Vec<f64> = (0..200).map(|index| f64::from(index) * 0.08).collect();
    let x: Vec<f64> = t.iter().map(|value| value.cos()).collect();
    let y: Vec<f64> = t.iter().map(|value| value.sin()).collect();
    let z: Vec<f64> = t.iter().map(|value| value * 0.08).collect();

    line3d(&x, &y, &z)
        .title("3D helix")
        .xlabel("x")
        .ylabel("y")
        .zlabel("z")
        .line_width(2.0)
        .color(Color::BLUE)
        .save("doc_line3d.png")
}
