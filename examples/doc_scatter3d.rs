//! Minimal 3D scatter plot.
//!
//! Run with:
//! `cargo run --example doc_scatter3d --features 3d`

use ruviz::prelude::*;

fn main() -> PlotResult<()> {
    let x = [0.0, 1.0, 2.0, 3.0, 4.0];
    let y = [0.2, 1.4, 0.8, 2.7, 2.1];
    let z = [0.5, 1.8, 1.1, 3.2, 2.6];

    scatter3d(&x, &y, &z)
        .title("3D scatter")
        .xlabel("x")
        .ylabel("y")
        .zlabel("z")
        .marker(MarkerStyle::Circle)
        .marker_size(8.0)
        .color(Color::BLUE)
        .save("doc_scatter3d.png")
}
