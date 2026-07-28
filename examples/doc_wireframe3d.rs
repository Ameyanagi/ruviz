//! Minimal 3D wireframe plot.
//!
//! Run with:
//! `cargo run --example doc_wireframe3d --features 3d`

use ruviz::prelude::*;

fn main() -> PlotResult<()> {
    let x = [-1.5, -0.5, 0.5, 1.5];
    let y = [-1.0, 0.0, 1.0];
    let z = [
        [-0.4, 0.4, 0.4, -0.4],
        [0.1, 1.0, 1.0, 0.1],
        [-0.4, 0.4, 0.4, -0.4],
    ];

    wireframe(&x, &y, &z)
        .title("3D wireframe")
        .xlabel("x")
        .ylabel("y")
        .zlabel("z")
        .line_width(1.5)
        .color(Color::DARK_GRAY)
        .save("doc_wireframe3d.png")
}
