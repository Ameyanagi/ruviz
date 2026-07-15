//! Allocation-reuse and CPU coordinate-transform experiment.
//!
//! Despite the historical filename, this example does not execute or estimate a
//! GPU path. It measures only the CPU work present in this file.

use std::time::Instant;

fn main() {
    benchmark_allocation_reuse();
    benchmark_cpu_transform();
}

fn benchmark_allocation_reuse() {
    const ITERATIONS: usize = 1_000;
    const POINTS_PER_ITERATION: usize = 10_000;

    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let data: Vec<f32> = (0..POINTS_PER_ITERATION).map(|i| i as f32).collect();
        std::hint::black_box(data);
    }
    let fresh_allocation = start.elapsed();

    let start = Instant::now();
    let mut reusable = Vec::with_capacity(POINTS_PER_ITERATION);
    for _ in 0..ITERATIONS {
        reusable.clear();
        reusable.extend((0..POINTS_PER_ITERATION).map(|i| i as f32));
        std::hint::black_box(&reusable);
    }
    let reused_allocation = start.elapsed();

    println!("CPU allocation experiment");
    println!("  fresh allocation: {fresh_allocation:?}");
    println!("  reused allocation: {reused_allocation:?}");
}

fn benchmark_cpu_transform() {
    for size in [1_000, 10_000, 100_000, 1_000_000] {
        let x: Vec<f64> = (0..size).map(|i| i as f64 * 0.001).collect();
        let y: Vec<f64> = x.iter().map(|value| value.sin()).collect();

        let start = Instant::now();
        let result = transform_cpu(&x, &y);
        let elapsed = start.elapsed();

        assert_eq!(result.len(), size);
        println!("CPU transform for {size} points: {elapsed:?}");
    }
}

fn transform_cpu(x: &[f64], y: &[f64]) -> Vec<(f32, f32)> {
    let x_range = (0.0, x.len() as f64 * 0.001);
    let y_range = (-1.0, 1.0);
    let viewport = (0.0, 0.0, 800.0, 600.0);

    x.iter()
        .zip(y)
        .map(|(&x_value, &y_value)| {
            let screen_x = ((x_value - x_range.0) / (x_range.1 - x_range.0)
                * (viewport.2 - viewport.0)
                + viewport.0) as f32;
            let screen_y = ((y_value - y_range.0) / (y_range.1 - y_range.0)
                * (viewport.3 - viewport.1)
                + viewport.1) as f32;
            (screen_x, screen_y)
        })
        .collect()
}
