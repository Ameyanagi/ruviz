use ruviz::prelude::*;

fn main() {
    let x = [0.0, 1.0];
    let y = [1.0, 2.0];
    let _ = Plot::new().line(&x, &y).typst(true);
}
