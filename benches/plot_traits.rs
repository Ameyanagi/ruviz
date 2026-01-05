//! Benchmarks for plot traits infrastructure
//!
//! These benchmarks verify that trait-based plot types meet performance targets:
//! - KDE: <100ms for 100K points
//! - ECDF: <50ms for 100K points
//! - Step: <30ms for 100K points
//! - Contour: <200ms for 100x100 grid
//! - General: <100ms for 100K points
//!
//! **IMPORTANT**: Run benchmarks LOCALLY only, not in CI.
//! CI runners have inconsistent performance that invalidates results.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use std::time::Duration;

// ============================================================================
// Test Data Generation Helpers
// ============================================================================

/// Generate pseudo-random data using a simple LCG (Linear Congruential Generator)
///
/// This provides reproducible "random" data for benchmarks without
/// depending on external crates. The data is normally-distributed-ish
/// using Box-Muller transform approximation.
fn generate_normal_data(n: usize, seed: u64) -> Vec<f64> {
    let mut state = seed;
    let mut data = Vec::with_capacity(n);

    for _ in 0..n {
        // LCG: next = (a * state + c) mod m
        // Using parameters from Numerical Recipes
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let u1 = (state >> 33) as f64 / (1u64 << 31) as f64;

        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let u2 = (state >> 33) as f64 / (1u64 << 31) as f64;

        // Box-Muller-ish transform (simplified)
        let z = (-2.0 * u1.max(1e-10).ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        data.push(z);
    }

    data
}

/// Generate X-Y data for scatter/line plots
fn generate_xy_data(n: usize, seed: u64) -> (Vec<f64>, Vec<f64>) {
    let mut state = seed;
    let mut x = Vec::with_capacity(n);
    let mut y = Vec::with_capacity(n);

    for i in 0..n {
        // X is sequential with some noise
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let noise = ((state >> 33) as f64 / (1u64 << 31) as f64 - 0.5) * 0.1;
        x.push(i as f64 + noise);

        // Y is a function of X plus noise
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let y_noise = ((state >> 33) as f64 / (1u64 << 31) as f64 - 0.5) * 0.5;
        y.push((i as f64 * 0.1).sin() + y_noise);
    }

    (x, y)
}

/// Generate 2D grid data for contour/heatmap
fn generate_grid_data(nx: usize, ny: usize, seed: u64) -> (Vec<f64>, Vec<f64>, Vec<Vec<f64>>) {
    let mut state = seed;

    let x: Vec<f64> = (0..nx)
        .map(|i| i as f64 / (nx - 1) as f64 * 10.0 - 5.0)
        .collect();
    let y: Vec<f64> = (0..ny)
        .map(|i| i as f64 / (ny - 1) as f64 * 10.0 - 5.0)
        .collect();

    let mut z = Vec::with_capacity(ny);
    for j in 0..ny {
        let mut row = Vec::with_capacity(nx);
        for i in 0..nx {
            // Gaussian-like function with noise
            let xi = x[i];
            let yj = y[j];
            let base = (-0.1 * (xi * xi + yj * yj)).exp();

            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let noise = ((state >> 33) as f64 / (1u64 << 31) as f64 - 0.5) * 0.05;

            row.push(base + noise);
        }
        z.push(row);
    }

    (x, y, z)
}

/// Generate categorical data for bar charts
#[allow(dead_code)]
fn generate_categorical_data(n: usize, seed: u64) -> (Vec<String>, Vec<f64>) {
    let mut state = seed;

    let categories: Vec<String> = (0..n).map(|i| format!("Category {}", i)).collect();

    let values: Vec<f64> = (0..n)
        .map(|_| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            (state >> 33) as f64 / (1u64 << 31) as f64 * 100.0
        })
        .collect();

    (categories, values)
}

// ============================================================================
// Distribution Plot Benchmarks (KDE, ECDF, Violin, Boxen)
// ============================================================================

/// Benchmark KDE computation and rendering
///
/// Target: <100ms for 100K points
fn bench_kde(c: &mut Criterion) {
    let mut group = c.benchmark_group("kde");
    group.measurement_time(Duration::from_secs(10));

    let sizes = [1_000, 10_000, 100_000];

    for &size in &sizes {
        let data = generate_normal_data(size, 12345);

        group.throughput(Throughput::Elements(size as u64));

        // Benchmark computation only (when compute_kde is available)
        group.bench_with_input(BenchmarkId::new("compute", size), &data, |b, data| {
            b.iter(|| {
                // TODO: When KDE traits are implemented, benchmark compute_kde()
                // For now, benchmark a simple operation as placeholder
                let sum: f64 = black_box(data).iter().sum();
                black_box(sum)
            })
        });

        // Benchmark full render (when Plot::kde is available)
        // group.bench_with_input(BenchmarkId::new("full_render", size), &data, |b, data| {
        //     b.iter(|| {
        //         Plot::new()
        //             .kde(black_box(data))
        //             .render()
        //             .expect("Render should succeed")
        //     })
        // });
    }

    group.finish();
}

/// Benchmark ECDF computation and rendering
///
/// Target: <50ms for 100K points
fn bench_ecdf(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecdf");
    group.measurement_time(Duration::from_secs(10));

    let sizes = [1_000, 10_000, 100_000];

    for &size in &sizes {
        let data = generate_normal_data(size, 54321);

        group.throughput(Throughput::Elements(size as u64));

        // Benchmark computation only (when compute_ecdf is available)
        group.bench_with_input(BenchmarkId::new("compute", size), &data, |b, data| {
            b.iter(|| {
                // TODO: When ECDF traits are implemented, benchmark compute_ecdf()
                // ECDF is essentially sorting, so benchmark sort as proxy
                let mut sorted = black_box(data).clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                black_box(sorted)
            })
        });
    }

    group.finish();
}

// ============================================================================
// Discrete Plot Benchmarks (Step, Stem)
// ============================================================================

/// Benchmark step plot computation and rendering
///
/// Target: <30ms for 100K points
fn bench_step(c: &mut Criterion) {
    let mut group = c.benchmark_group("step");
    group.measurement_time(Duration::from_secs(10));

    let sizes = [1_000, 10_000, 100_000];

    for &size in &sizes {
        let (x, y) = generate_xy_data(size, 11111);

        group.throughput(Throughput::Elements(size as u64));

        // Benchmark step line computation
        group.bench_with_input(BenchmarkId::new("compute", size), &(&x, &y), |b, (x, y)| {
            b.iter(|| {
                // Step function doubles point count (each point creates horizontal + vertical segment)
                let mut step_x = Vec::with_capacity(x.len() * 2);
                let mut step_y = Vec::with_capacity(y.len() * 2);

                for i in 0..x.len() {
                    if i > 0 {
                        step_x.push(black_box(x)[i]);
                        step_y.push(black_box(y)[i - 1]);
                    }
                    step_x.push(x[i]);
                    step_y.push(y[i]);
                }

                black_box((step_x, step_y))
            })
        });
    }

    group.finish();
}

// ============================================================================
// Continuous Plot Benchmarks (Contour, Hexbin, Area)
// ============================================================================

/// Benchmark contour plot computation
///
/// Target: <200ms for 100x100 grid
fn bench_contour(c: &mut Criterion) {
    let mut group = c.benchmark_group("contour");
    group.measurement_time(Duration::from_secs(10));

    let grid_sizes = [50, 100, 200];

    for &size in &grid_sizes {
        let (_x, _y, z) = generate_grid_data(size, size, 99999);

        group.throughput(Throughput::Elements((size * size) as u64));

        // Benchmark grid generation (proxy for contour computation)
        group.bench_with_input(BenchmarkId::new("grid_gen", size), &size, |b, &size| {
            b.iter(|| {
                let (_x, _y, z) = generate_grid_data(black_box(size), size, 99999);
                black_box(z)
            })
        });

        // TODO: When contour traits are implemented, benchmark actual contour computation
        group.bench_with_input(BenchmarkId::new("data_access", size), &z, |b, z| {
            b.iter(|| {
                let sum: f64 = black_box(z).iter().flat_map(|row| row.iter()).sum();
                black_box(sum)
            })
        });
    }

    group.finish();
}

// ============================================================================
// Trait Infrastructure Benchmarks
// ============================================================================

/// Benchmark PlotArea coordinate transformations
///
/// These are called frequently during rendering, so should be very fast.
fn bench_plot_area(c: &mut Criterion) {
    use ruviz::prelude::PlotArea;

    let mut group = c.benchmark_group("plot_area");

    let area = PlotArea::new(100.0, 50.0, 600.0, 400.0, 0.0, 10.0, 0.0, 100.0);

    // Benchmark single coordinate transform
    group.bench_function("data_to_screen", |b| {
        b.iter(|| area.data_to_screen(black_box(5.0), black_box(50.0)))
    });

    group.bench_function("screen_to_data", |b| {
        b.iter(|| area.screen_to_data(black_box(400.0), black_box(250.0)))
    });

    group.bench_function("contains_data", |b| {
        b.iter(|| area.contains_data(black_box(5.0), black_box(50.0)))
    });

    // Benchmark batch coordinate transform (common pattern)
    let n = 10_000;
    let data_points: Vec<(f64, f64)> = (0..n)
        .map(|i| (i as f64 / 1000.0, i as f64 / 100.0))
        .collect();

    group.throughput(Throughput::Elements(n as u64));

    group.bench_with_input(
        BenchmarkId::new("batch_transform", n),
        &data_points,
        |b, points| {
            b.iter(|| {
                let screen_points: Vec<(f32, f32)> = black_box(points)
                    .iter()
                    .map(|&(x, y)| area.data_to_screen(x, y))
                    .collect();
                black_box(screen_points)
            })
        },
    );

    group.finish();
}

/// Benchmark PlotBuilder operations
fn bench_plot_builder(c: &mut Criterion) {
    use ruviz::prelude::*;

    let mut group = c.benchmark_group("plot_builder");

    // Benchmark builder creation and method chaining
    group.bench_function("method_chain", |b| {
        b.iter(|| {
            let plot = Plot::new()
                .title("Test Plot")
                .xlabel("X Axis")
                .ylabel("Y Axis")
                .size(8.0, 6.0)
                .dpi(100);
            black_box(plot)
        })
    });

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(distribution_benches, bench_kde, bench_ecdf,);

criterion_group!(discrete_benches, bench_step,);

criterion_group!(continuous_benches, bench_contour,);

criterion_group!(infrastructure_benches, bench_plot_area, bench_plot_builder,);

criterion_main!(
    distribution_benches,
    discrete_benches,
    continuous_benches,
    infrastructure_benches,
);
