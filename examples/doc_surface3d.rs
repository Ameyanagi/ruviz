//! Minimal 3D surface plot.
//!
//! Run with:
//! `cargo run --example doc_surface3d --features 3d`

use ruviz::prelude::*;

fn main() -> Result<()> {
    let x = [-1.5, -0.5, 0.5, 1.5];
    let y = [-1.0, 0.0, 1.0];
    let z = [
        [-0.4, 0.4, 0.4, -0.4],
        [0.1, 1.0, 1.0, 0.1],
        [-0.4, 0.4, 0.4, -0.4],
    ];

    surface(&x, &y, &z)
        .title("3D surface")
        .xlabel("x")
        .ylabel("y")
        .zlabel("z")
        .save("doc_surface3d.png")
}
