use ruviz::prelude::*;

fn main() -> ruviz::core::Result<()> {
    let x = [-2.0, -1.0, 0.0, 1.0, 2.0];
    let y = [-2.0, -1.0, 0.0, 1.0, 2.0];
    let z = [
        [0.00, 0.25, 0.50, 0.25, 0.00],
        [0.25, 0.75, 1.00, 0.75, 0.25],
        [0.50, 1.00, 1.50, 1.00, 0.50],
        [0.25, 0.75, 1.00, 0.75, 0.25],
        [0.00, 0.25, 0.50, 0.25, 0.00],
    ];

    surface(&x, &y, &z)
        .title("Simple 3d surface")
        .xlabel("x")
        .ylabel("y")
        .zlabel("z")
        .perspective_deg(42.0)
        .save("ruviz-3d-surface.png")
}
