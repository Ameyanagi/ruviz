//! Memory performance benchmarks for ruviz
//!
//! These benchmarks measure memory allocation and pooling performance.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ruviz::data::{MemoryPool, PooledVec, SharedMemoryPool};
use ruviz::prelude::*;

fn bench_allocation_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocation_overhead");

    for size in [100, 1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        // Benchmark traditional Vec allocation
        group.bench_with_input(
            BenchmarkId::new("vec_allocation", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let _vec: Vec<f64> = vec![0.0; size];
                });
            },
        );

        // Benchmark pooled allocation
        group.bench_with_input(
            BenchmarkId::new("pool_allocation", size),
            size,
            |b, &size| {
                let mut pool = MemoryPool::<f64>::new(size * 2);
                b.iter(|| {
                    let buffer = pool.acquire(size);
                    pool.release(buffer);
                });
            },
        );
    }
    group.finish();
}

fn bench_plot_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("plot_rendering");

    // Create test output directory
    std::fs::create_dir_all("test_output").ok();

    for &points in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(points as u64));

        let x_data: Vec<f64> = (0..points).map(|i| i as f64 * 0.01).collect();
        let y_data: Vec<f64> = x_data.iter().map(|&x| x.sin()).collect();

        // Benchmark plot rendering
        group.bench_with_input(
            BenchmarkId::new("line_plot", points),
            &(x_data.clone(), y_data.clone()),
            |b, (x, y)| {
                b.iter(|| {
                    let _result = Plot::new()
                        .line(x, y)
                        .title("Performance Test")
                        .save(&format!("test_output/bench_{}.png", points));
                });
            },
        );
    }
    group.finish();
}

fn bench_steady_state_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("steady_state_rendering");
    group.sample_size(20); // Fewer samples for longer benchmark

    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y_data = vec![0.0, 1.0, 0.0, 1.0, 0.0];

    // Benchmark memory growth over time with many plots
    group.bench_function("steady_state_100_plots", |b| {
        b.iter(|| {
            for i in 0..100 {
                let _result = Plot::new()
                    .line(&x_data, &y_data)
                    .title(&format!("Plot {}", i))
                    .save(&format!("test_output/steady_state_{}.png", i));
            }
        });
    });

    group.finish();
}

fn bench_concurrent_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_rendering");

    for threads in [1, 2, 4].iter() {
        let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let y_data = vec![0.0, 1.0, 0.0, 1.0, 0.0, 1.0];

        group.bench_with_input(
            BenchmarkId::new("concurrent_plots", threads),
            threads,
            |b, &thread_count| {
                b.iter(|| {
                    let handles: Vec<_> = (0..thread_count)
                        .map(|thread_id| {
                            let x = x_data.clone();
                            let y = y_data.clone();
                            std::thread::spawn(move || {
                                for i in 0..10 {
                                    let _result = Plot::new()
                                        .line(&x, &y)
                                        .title(&format!("Thread {} Plot {}", thread_id, i))
                                        .save(&format!(
                                            "test_output/concurrent_{}_{}.png",
                                            thread_id, i
                                        ));
                                }
                            })
                        })
                        .collect();

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_memory_pool_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_pool_operations");

    // Benchmark acquire/release cycle
    group.bench_function("acquire_release_cycle", |b| {
        let mut pool = MemoryPool::<f64>::new(1000);

        b.iter(|| {
            let buffer = pool.acquire(1000);
            pool.release(buffer);
        });
    });

    // Benchmark PooledVec operations
    group.bench_function("pooled_vec_push", |b| {
        let pool = SharedMemoryPool::<f64>::new(10000);

        b.iter(|| {
            let mut vec = PooledVec::new(pool.clone());
            for i in 0..1000 {
                vec.push(i as f64);
            }

            // Simulate processing
            let sum: f64 = vec.iter().sum();
            criterion::black_box(sum);
        });
    });

    group.finish();
}

fn bench_vec_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("vec_operations");

    // Compare traditional Vec vs PooledVec
    group.bench_function("traditional_vec_push", |b| {
        b.iter(|| {
            let mut vec = Vec::with_capacity(1000);
            for i in 0..1000 {
                vec.push(i as f64);
            }
            let sum: f64 = vec.iter().sum();
            criterion::black_box(sum);
        });
    });

    group.bench_function("pooled_vec_iteration", |b| {
        let pool = SharedMemoryPool::<f64>::new(10000);
        let mut vec = PooledVec::new(pool);
        for i in 0..10000 {
            vec.push(i as f64);
        }

        b.iter(|| {
            let sum: f64 = vec.iter().sum();
            criterion::black_box(sum);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_allocation_overhead,
    bench_plot_rendering,
    bench_steady_state_rendering,
    bench_concurrent_rendering,
    bench_memory_pool_operations,
    bench_vec_operations,
);
criterion_main!(benches);
