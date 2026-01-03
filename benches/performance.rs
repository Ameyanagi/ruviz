use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use ruviz::prelude::*;
use std::time::Duration;

/// Benchmark different dataset sizes for scatter plots
fn bench_scatter_plots(c: &mut Criterion) {
    let mut group = c.benchmark_group("scatter_plots");

    // Test various dataset sizes
    let sizes = [1_000, 10_000, 50_000, 100_000, 500_000, 1_000_000];

    for &size in &sizes {
        // Generate test data
        let x_data: Vec<f64> = (0..size).map(|i| i as f64 * 0.01).collect();
        let y_data: Vec<f64> = (0..size).map(|i| (i as f64 * 0.01).sin()).collect();

        group.throughput(Throughput::Elements(size as u64));
        group.measurement_time(Duration::from_secs(10));

        group.bench_with_input(BenchmarkId::new("scatter", size), &size, |b, _| {
            b.iter(|| {
                let plot = Plot::new()
                    .scatter(black_box(&x_data), black_box(&y_data))
                    .title(&format!("Scatter Plot - {} points", size));

                black_box(plot.render()).expect("Render should succeed")
            })
        });
    }

    group.finish();
}

/// Benchmark different dataset sizes for line plots
fn bench_line_plots(c: &mut Criterion) {
    let mut group = c.benchmark_group("line_plots");

    let sizes = [1_000, 10_000, 50_000, 100_000, 500_000, 1_000_000];

    for &size in &sizes {
        // Generate realistic time series data
        let x_data: Vec<f64> = (0..size).map(|i| i as f64 / 100.0).collect();
        let y_data: Vec<f64> = x_data
            .iter()
            .map(|&t| t.sin() + (t * 0.1).cos() * 0.5 + (t * 0.05).sin() * 0.2)
            .collect();

        group.throughput(Throughput::Elements(size as u64));
        group.measurement_time(Duration::from_secs(10));

        group.bench_with_input(BenchmarkId::new("line", size), &size, |b, _| {
            b.iter(|| {
                let plot = Plot::new()
                    .line(black_box(&x_data), black_box(&y_data))
                    .title(&format!("Line Plot - {} points", size))
                    .xlabel("Time")
                    .ylabel("Signal");

                black_box(plot.render()).expect("Render should succeed")
            })
        });
    }

    group.finish();
}

/// Benchmark multi-series plots with varying complexity
fn bench_multi_series(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_series");

    let base_size = 10_000;
    let series_counts = [1, 2, 4, 8, 16];

    for &series_count in &series_counts {
        // Generate multiple data series
        let mut all_series_data = Vec::new();
        for i in 0..series_count {
            let x: Vec<f64> = (0..base_size).map(|j| j as f64 * 0.01).collect();
            let y: Vec<f64> = (0..base_size)
                .map(|j| ((j as f64 * 0.01) + (i as f64 * 0.5)).sin())
                .collect();
            all_series_data.push((x, y));
        }

        group.throughput(Throughput::Elements((base_size * series_count) as u64));

        group.bench_with_input(
            BenchmarkId::new("multi_series", series_count),
            &series_count,
            |b, _| {
                b.iter(|| {
                    let mut plot =
                        Plot::new().title(&format!("Multi-Series Plot - {} series", series_count));

                    // Add all series to the plot
                    for (i, (x, y)) in all_series_data.iter().enumerate() {
                        plot = plot
                            .line(black_box(x), black_box(y))
                            .label(&format!("Series {}", i + 1))
                            .end_series();
                    }

                    plot = plot.legend(Position::TopRight);

                    black_box(plot.render()).expect("Render should succeed")
                })
            },
        );
    }

    group.finish();
}

/// Benchmark different plot types with same dataset
fn bench_plot_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("plot_types");

    let size = 50_000;
    let x_data: Vec<f64> = (0..size).map(|i| i as f64).collect();
    let y_data: Vec<f64> = (0..size).map(|i| (i as f64 * 0.01).sin() * 100.0).collect();

    // Categories for bar chart
    let categories: Vec<String> = (0..100).map(|i| format!("Cat{}", i)).collect();
    let values: Vec<f64> = (0..100).map(|i| (i as f64).sin() * 50.0 + 50.0).collect();

    group.throughput(Throughput::Elements(size as u64));

    // Line plot benchmark
    group.bench_function("line_plot", |b| {
        b.iter(|| {
            let plot = Plot::new()
                .line(black_box(&x_data), black_box(&y_data))
                .title("Line Plot Benchmark");
            black_box(plot.render()).expect("Render should succeed")
        })
    });

    // Scatter plot benchmark
    group.bench_function("scatter_plot", |b| {
        b.iter(|| {
            let plot = Plot::new()
                .scatter(black_box(&x_data), black_box(&y_data))
                .title("Scatter Plot Benchmark");
            black_box(plot.render()).expect("Render should succeed")
        })
    });

    // Bar chart benchmark (smaller dataset)
    group.bench_function("bar_chart", |b| {
        b.iter(|| {
            let plot = Plot::new()
                .bar(black_box(&categories), black_box(&values))
                .title("Bar Chart Benchmark");
            black_box(plot.render()).expect("Render should succeed")
        })
    });

    group.finish();
}

/// Benchmark rendering at different resolutions
fn bench_resolutions(c: &mut Criterion) {
    let mut group = c.benchmark_group("resolutions");

    let size = 25_000;
    let x_data: Vec<f64> = (0..size).map(|i| i as f64 * 0.01).collect();
    let y_data: Vec<f64> = (0..size).map(|i| (i as f64 * 0.01).sin()).collect();

    let resolutions = [
        (400, 300),   // Small
        (800, 600),   // Standard
        (1920, 1080), // HD
        (3840, 2160), // 4K
    ];

    for &(width, height) in &resolutions {
        let resolution_name = format!("{}x{}", width, height);

        group.bench_with_input(
            BenchmarkId::new("resolution", &resolution_name),
            &(width, height),
            |b, &(w, h)| {
                b.iter(|| {
                    let plot = Plot::new()
                        .scatter(black_box(&x_data), black_box(&y_data))
                        .end_series()
                        .dimensions(w, h)
                        .title(&format!("Resolution Test {}x{}", w, h));

                    black_box(plot.render()).expect("Render should succeed")
                })
            },
        );
    }

    group.finish();
}

/// Benchmark memory efficiency - track memory usage patterns
fn bench_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_patterns");

    // Test memory scaling with dataset size
    let sizes = [10_000, 50_000, 100_000, 500_000];

    for &size in &sizes {
        group.bench_with_input(BenchmarkId::new("memory_scaling", size), &size, |b, &s| {
            b.iter_custom(|iters| {
                let start = std::time::Instant::now();

                for _ in 0..iters {
                    // Generate fresh data each iteration to test allocation patterns
                    let x_data: Vec<f64> = (0..s).map(|i| i as f64).collect();
                    let y_data: Vec<f64> = (0..s).map(|i| (i as f64).sin()).collect();

                    let plot = Plot::new()
                        .line(black_box(&x_data), black_box(&y_data))
                        .title(&format!("Memory Test - {} points", s));

                    black_box(plot.render()).expect("Render should succeed");

                    // Ensure data is dropped
                    drop(x_data);
                    drop(y_data);
                }

                start.elapsed()
            })
        });
    }

    group.finish();
}

/// Benchmark theme and styling overhead
fn bench_styling(c: &mut Criterion) {
    let mut group = c.benchmark_group("styling");

    let size = 25_000;
    let x_data: Vec<f64> = (0..size).map(|i| i as f64).collect();
    let y_data: Vec<f64> = (0..size).map(|i| (i as f64).sin()).collect();

    // Minimal styling
    group.bench_function("minimal_style", |b| {
        b.iter(|| {
            let plot = Plot::new().line(black_box(&x_data), black_box(&y_data));
            black_box(plot.render()).expect("Render should succeed")
        })
    });

    // Heavy styling
    group.bench_function("heavy_style", |b| {
        b.iter(|| {
            let plot = Plot::new()
                .line(black_box(&x_data), black_box(&y_data))
                .title("Complex Styled Plot")
                .xlabel("X Axis Label")
                .ylabel("Y Axis Label")
                .end_series()
                .grid(true)
                .legend(Position::TopRight);
            black_box(plot.render()).expect("Render should succeed")
        })
    });

    group.finish();
}

// Criterion configuration and group registration
criterion_group!(
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(50)
        .warm_up_time(Duration::from_secs(3));
    targets =
        bench_scatter_plots,
        bench_line_plots,
        bench_multi_series,
        bench_plot_types,
        bench_resolutions,
        bench_memory_patterns,
        bench_styling
);

criterion_main!(benches);
