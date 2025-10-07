// Baseline performance benchmarks - TDD approach
// These benchmarks define expected performance targets before optimization

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use ruviz::prelude::*;

/// Benchmark: Line plot with 1K points
/// Target: < 10ms
fn bench_line_plot_1k(c: &mut Criterion) {
    let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    c.bench_function("line_plot_1k", |b| {
        b.iter(|| {
            Plot::new()
                .line(black_box(&x), black_box(&y))
                .save("test_output/bench_line_1k.png")
                .expect("Failed to save plot");
        });
    });
}

/// Benchmark: Line plot with 100K points
/// Target: < 100ms
fn bench_line_plot_100k(c: &mut Criterion) {
    let x: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    c.bench_function("line_plot_100k", |b| {
        b.iter(|| {
            Plot::new()
                .line(black_box(&x), black_box(&y))
                .auto_optimize()
                .save("test_output/bench_line_100k.png")
                .expect("Failed to save plot");
        });
    });
}

/// Benchmark: Scatter plot with 10K points
/// Target: < 50ms
fn bench_scatter_plot_10k(c: &mut Criterion) {
    let x: Vec<f64> = (0..10_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0 + 10.0).collect();

    c.bench_function("scatter_plot_10k", |b| {
        b.iter(|| {
            Plot::new()
                .scatter(black_box(&x), black_box(&y))
                .save("test_output/bench_scatter_10k.png")
                .expect("Failed to save plot");
        });
    });
}

/// Benchmark: Histogram with 1M points
/// Target: < 500ms
fn bench_histogram_1m(c: &mut Criterion) {
    let data: Vec<f64> = (0..1_000_000).map(|i| (i as f64).sin() * 100.0).collect();

    c.bench_function("histogram_1m", |b| {
        b.iter(|| {
            Plot::new()
                .histogram(black_box(&data), None)
                .auto_optimize()
                .save("test_output/bench_histogram_1m.png")
                .expect("Failed to save plot");
        });
    });
}

/// Benchmark: Box plot with 100K points
/// Target: < 200ms
fn bench_boxplot_100k(c: &mut Criterion) {
    let data: Vec<f64> = (0..100_000).map(|i| (i as f64).sin() * 100.0).collect();

    c.bench_function("boxplot_100k", |b| {
        b.iter(|| {
            Plot::new()
                .boxplot(black_box(&data), None)
                .save("test_output/bench_boxplot_100k.png")
                .expect("Failed to save plot");
        });
    });
}

/// Benchmark: Multi-series plot (5 series, 10K points each)
/// Target: < 150ms
fn bench_multi_series_50k(c: &mut Criterion) {
    let x: Vec<f64> = (0..10_000).map(|i| i as f64).collect();
    let series: Vec<Vec<f64>> = (0..5)
        .map(|s| x.iter().map(|v| v * (s as f64 + 1.0)).collect())
        .collect();

    c.bench_function("multi_series_50k", |b| {
        b.iter(|| {
            let mut builder = Plot::new().line(black_box(&x), black_box(&series[0]));
            for y in series[1..].iter() {
                builder = builder.line(black_box(&x), black_box(y));
            }
            builder
                .auto_optimize()
                .save("test_output/bench_multi_series.png")
                .expect("Failed to save plot");
        });
    });
}

/// Benchmark: Auto-optimization decision speed
/// Target: < 1ms for decision logic
fn bench_auto_optimize_speed(c: &mut Criterion) {
    let mut group = c.benchmark_group("auto_optimize_decision");

    for size in [100, 1_000, 10_000, 100_000].iter() {
        let x: Vec<f64> = (0..*size).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let plot = Plot::new()
                    .line(black_box(&x), black_box(&y))
                    .auto_optimize();
                black_box(plot.get_backend_name());
            });
        });
    }
    group.finish();
}

/// Benchmark: Throughput measurement (points/second)
fn bench_throughput_measurement(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    group.throughput(criterion::Throughput::Elements(100_000));

    let x: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    group.bench_function("line_plot_throughput", |b| {
        b.iter(|| {
            Plot::new()
                .line(black_box(&x), black_box(&y))
                .auto_optimize()
                .save("test_output/bench_throughput.png")
                .expect("Failed to save plot");
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_line_plot_1k,
    bench_line_plot_100k,
    bench_scatter_plot_10k,
    bench_histogram_1m,
    bench_boxplot_100k,
    bench_multi_series_50k,
    bench_auto_optimize_speed,
    bench_throughput_measurement
);
criterion_main!(benches);
