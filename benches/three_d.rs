use std::hint::black_box;
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ruviz::{scatter3d, surface};

fn scatter_data(size: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let x: Vec<_> = (0..size)
        .map(|index| index as f64 / size.max(1) as f64)
        .collect();
    let y: Vec<_> = x
        .iter()
        .enumerate()
        .map(|(index, value)| (value * 31.0).sin() + (index % 17) as f64 * 0.002)
        .collect();
    let z: Vec<_> = x
        .iter()
        .enumerate()
        .map(|(index, value)| (value * 23.0).cos() + (index % 11) as f64 * 0.003)
        .collect();
    (x, y, z)
}

fn surface_data(side: usize) -> (Vec<f64>, Vec<f64>, Vec<Vec<f64>>) {
    let axis: Vec<_> = (0..side)
        .map(|index| -3.0 + 6.0 * index as f64 / side.saturating_sub(1).max(1) as f64)
        .collect();
    let z = axis
        .iter()
        .map(|&y| {
            axis.iter()
                .map(|&x| {
                    let radius = x.hypot(y);
                    if radius == 0.0 {
                        1.0
                    } else {
                        radius.sin() / radius
                    }
                })
                .collect()
        })
        .collect();
    (axis.clone(), axis, z)
}

fn benchmark_scatter(c: &mut Criterion) {
    let full = std::env::var_os("RUVIZ_3D_BENCH_FULL").is_some();
    let sizes: &[usize] = if full {
        &[100_000, 1_000_000]
    } else {
        &[10_000, 100_000]
    };
    let mut group = c.benchmark_group("3d/cpu/scatter");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(2));
    for &size in sizes {
        let (x, y, z) = scatter_data(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("compile", size), &size, |b, _| {
            b.iter(|| {
                black_box(
                    scatter3d(black_box(&x), black_box(&y), black_box(&z))
                        .benchmark_compile_scene_with_diagnostics()
                        .expect("compile scatter scene"),
                )
            });
        });
        group.bench_with_input(BenchmarkId::new("render-640x480", size), &size, |b, _| {
            b.iter(|| {
                black_box(
                    scatter3d(black_box(&x), black_box(&y), black_box(&z))
                        .render()
                        .expect("render scatter"),
                )
            });
        });
    }
    group.finish();
}

fn benchmark_surface(c: &mut Criterion) {
    let full = std::env::var_os("RUVIZ_3D_BENCH_FULL").is_some();
    let sizes: &[usize] = if full { &[100, 512, 1024] } else { &[32, 100] };
    let mut group = c.benchmark_group("3d/cpu/surface");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(2));
    for &side in sizes {
        let (x, y, z) = surface_data(side);
        let triangles = side.saturating_sub(1).pow(2).saturating_mul(2);
        group.throughput(Throughput::Elements(triangles as u64));
        group.bench_with_input(BenchmarkId::new("compile", side), &side, |b, _| {
            b.iter(|| {
                black_box(
                    surface(black_box(&x), black_box(&y), black_box(&z))
                        .benchmark_compile_scene_with_diagnostics()
                        .expect("compile surface scene"),
                )
            });
        });
        group.bench_with_input(BenchmarkId::new("render-640x480", side), &side, |b, _| {
            b.iter(|| {
                black_box(
                    surface(black_box(&x), black_box(&y), black_box(&z))
                        .render()
                        .expect("render surface"),
                )
            });
        });
    }
    group.finish();
}

criterion_group!(three_d_benches, benchmark_scatter, benchmark_surface);
criterion_main!(three_d_benches);
