use ruviz::scatter3d;

fn main() {
    let x = [0.0, 1.0];
    let y = [0.0, 1.0];
    let z = [[0.0, 1.0], [1.0, 2.0]];
    let _ = scatter3d(&x, &y, &z);
}
