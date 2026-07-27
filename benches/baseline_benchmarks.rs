//! Baseline performance benchmarks.
//!
//! Every workload is declared exactly once, in [`workloads`], and each group
//! below iterates that one list. Adding a workload adds it to every group, so
//! the groups cannot drift out of sync with each other.
//!
//! The groups deliberately separate the two costs that used to be measured as
//! one number:
//!
//! - `render` — rasterization only, via `Plot::render()`.
//! - `encode_png` — PNG encoding of an already-rendered image.
//!
//! Neither writes to disk. Until 2026-07 every benchmark here called
//! `.save(path)` inside `b.iter`, so each reported "render time" was
//! rasterization + deflate + a filesystem write, and a rasterizer regression
//! could hide inside PNG/IO noise.
//!
//! For cross-runtime and feature-flag comparisons see `docs/benchmarks/`.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use ruviz::prelude::*;

/// Builds one plot to measure. Kept as a plain `fn` pointer so the workload
/// table stays a value, not a macro.
type BuildPlot = fn() -> Plot;

/// The single source of truth for what gets benchmarked.
fn workloads() -> Vec<(&'static str, BuildPlot)> {
    vec![
        ("line_1k", line_1k as BuildPlot),
        ("line_100k", line_100k as BuildPlot),
        ("scatter_10k", scatter_10k as BuildPlot),
        ("histogram_1m", histogram_1m as BuildPlot),
        ("boxplot_100k", boxplot_100k as BuildPlot),
        ("multi_series_50k", multi_series_50k as BuildPlot),
    ]
}

fn ramp(count: usize) -> (Vec<f64>, Vec<f64>) {
    let x: Vec<f64> = (0..count).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();
    (x, y)
}

fn wave(count: usize) -> Vec<f64> {
    (0..count).map(|i| (i as f64).sin() * 100.0).collect()
}

fn line_1k() -> Plot {
    let (x, y) = ramp(1_000);
    Plot::new().line(&x, &y).into()
}

fn line_100k() -> Plot {
    let (x, y) = ramp(100_000);
    Plot::new().line(&x, &y).auto_optimize().into()
}

fn scatter_10k() -> Plot {
    let x: Vec<f64> = (0..10_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0 + 10.0).collect();
    Plot::new().scatter(&x, &y).into()
}

fn histogram_1m() -> Plot {
    let data = wave(1_000_000);
    Plot::new().histogram(&data).auto_optimize().into()
}

fn boxplot_100k() -> Plot {
    let data = wave(100_000);
    Plot::new().boxplot(&data).into()
}

fn multi_series_50k() -> Plot {
    let x: Vec<f64> = (0..10_000).map(|i| i as f64).collect();
    let series: Vec<Vec<f64>> = (0..5)
        .map(|s| x.iter().map(|v| v * (s as f64 + 1.0)).collect())
        .collect();

    let mut builder = Plot::new().line(&x, &series[0]);
    for y in series[1..].iter() {
        builder = builder.line(&x, y);
    }
    builder.auto_optimize().into()
}

/// Rasterization only: no PNG encode, no disk.
fn bench_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("render");
    for (name, build) in workloads() {
        let plot = build();
        group.bench_function(name, |b| {
            b.iter(|| black_box(plot.render().expect("render failed")));
        });
    }
    group.finish();
}

/// PNG encoding only, on an image that was rasterized once up front.
fn bench_encode_png(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_png");
    for (name, build) in workloads() {
        let image = build().render().expect("render failed");
        group.bench_function(name, |b| {
            b.iter(|| black_box(image.encode_png().expect("PNG encode failed")));
        });
    }
    group.finish();
}

/// Points per second through the rasterizer, so the number can be compared
/// against other libraries without a PNG encoder in the denominator.
fn bench_render_throughput(c: &mut Criterion) {
    const POINTS: u64 = 100_000;

    let mut group = c.benchmark_group("render_throughput");
    group.throughput(criterion::Throughput::Elements(POINTS));

    let plot = line_100k();
    group.bench_function("line_100k", |b| {
        b.iter(|| black_box(plot.render().expect("render failed")));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_render,
    bench_encode_png,
    bench_render_throughput
);
criterion_main!(benches);
