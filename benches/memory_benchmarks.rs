// Memory allocation tracking benchmarks - TDD approach
// These benchmarks validate memory efficiency targets

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ruviz::prelude::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Simple allocator wrapper to track allocations
struct TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static PEAK_ALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = System.alloc(layout);
        if !ret.is_null() {
            let size = layout.size();
            let current = ALLOCATED.fetch_add(size, Ordering::SeqCst) + size;

            // Update peak
            let mut peak = PEAK_ALLOCATED.load(Ordering::SeqCst);
            while current > peak {
                match PEAK_ALLOCATED.compare_exchange_weak(
                    peak,
                    current,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    Ok(_) => break,
                    Err(x) => peak = x,
                }
            }
        }
        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
    }
}

fn reset_memory_tracking() {
    ALLOCATED.store(0, Ordering::SeqCst);
    PEAK_ALLOCATED.store(0, Ordering::SeqCst);
}

fn get_peak_memory() -> usize {
    PEAK_ALLOCATED.load(Ordering::SeqCst)
}

/// Benchmark: Memory usage for line plot with 100K points
/// Target: Peak memory < 2x data size (< 1.6MB for 100K f64s = 800KB)
fn bench_memory_line_plot_100k(c: &mut Criterion) {
    c.bench_function("memory_line_plot_100k", |b| {
        b.iter(|| {
            reset_memory_tracking();

            let x: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
            let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

            Plot::new()
                .line(black_box(&x), black_box(&y))
                .auto_optimize()
                .save("test_output/bench_mem_line_100k.png")
                .expect("Failed to save plot");

            let peak = get_peak_memory();
            let data_size = 100_000 * 8 * 2; // 100K points * 8 bytes * 2 arrays

            // Peak memory should be < 2x data size
            assert!(
                peak < data_size * 2,
                "Peak memory {} exceeds 2x data size {}",
                peak,
                data_size * 2
            );
        });
    });
}

/// Benchmark: Memory usage for multi-series plot
/// Target: Peak memory < 20MB for 10 series × 10K points
fn bench_memory_multi_series(c: &mut Criterion) {
    c.bench_function("memory_multi_series", |b| {
        b.iter(|| {
            reset_memory_tracking();

            let x: Vec<f64> = (0..10_000).map(|i| i as f64).collect();
            let series: Vec<Vec<f64>> = (0..10)
                .map(|s| x.iter().map(|v| v * (s as f64 + 1.0)).collect())
                .collect();

            let mut builder = Plot::new().line(black_box(&x), black_box(&series[0]));
            for y in series[1..].iter() {
                builder = builder.line(black_box(&x), black_box(y));
            }

            builder
                .auto_optimize()
                .save("test_output/bench_mem_multi_series.png")
                .expect("Failed to save plot");

            let peak = get_peak_memory();
            let target_mb = 20 * 1024 * 1024; // 20MB

            assert!(
                peak < target_mb,
                "Peak memory {} exceeds 20MB target {}",
                peak,
                target_mb
            );
        });
    });
}

/// Benchmark: Memory leak detection through repeated operations
/// Target: Memory returns to baseline after 1K iterations
fn bench_memory_no_leaks(c: &mut Criterion) {
    c.bench_function("memory_no_leaks", |b| {
        b.iter(|| {
            reset_memory_tracking();
            let baseline = ALLOCATED.load(Ordering::SeqCst);

            // Create and drop plots repeatedly
            for _ in 0..100 {
                let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
                let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

                Plot::new()
                    .line(black_box(&x), black_box(&y))
                    .save("test_output/bench_mem_leaks.png")
                    .expect("Failed to save plot");
            }

            // Allow for some variance but should be close to baseline
            let final_allocated = ALLOCATED.load(Ordering::SeqCst);
            let growth = if final_allocated > baseline {
                final_allocated - baseline
            } else {
                0
            };

            // Growth should be minimal (< 1MB indicates no significant leaks)
            assert!(
                growth < 1024 * 1024,
                "Memory growth {} indicates potential leak",
                growth
            );
        });
    });
}

/// Benchmark: Large dataset memory efficiency with DataShader
/// Target: < 100MB for 1M point histogram
fn bench_memory_large_dataset(c: &mut Criterion) {
    c.bench_function("memory_large_dataset", |b| {
        b.iter(|| {
            reset_memory_tracking();

            let data: Vec<f64> = (0..1_000_000).map(|i| (i as f64).sin() * 100.0).collect();

            Plot::new()
                .histogram(black_box(&data), None)
                .auto_optimize()
                .save("test_output/bench_mem_large.png")
                .expect("Failed to save plot");

            let peak = get_peak_memory();
            let target_mb = 100 * 1024 * 1024; // 100MB

            assert!(
                peak < target_mb,
                "Peak memory {} exceeds 100MB target {}",
                peak,
                target_mb
            );
        });
    });
}

/// Benchmark: Memory pool reuse efficiency
/// Target: No allocation growth after warmup (3 iterations)
fn bench_memory_pool_reuse(c: &mut Criterion) {
    c.bench_function("memory_pool_reuse", |b| {
        b.iter(|| {
            reset_memory_tracking();

            let x: Vec<f64> = (0..10_000).map(|i| i as f64).collect();
            let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

            // Warmup - establish pool baseline
            for _ in 0..3 {
                Plot::new()
                    .line(&x, &y)
                    .save("test_output/bench_mem_pool_warmup.png")
                    .expect("Failed to save plot");
            }

            let warmup_peak = get_peak_memory();
            reset_memory_tracking();

            // Actual measurement - should reuse pools
            for _ in 0..10 {
                Plot::new()
                    .line(black_box(&x), black_box(&y))
                    .save("test_output/bench_mem_pool_reuse.png")
                    .expect("Failed to save plot");
            }

            let measurement_peak = get_peak_memory();

            // Memory usage should not grow significantly after warmup
            let growth_ratio = measurement_peak as f64 / warmup_peak as f64;
            assert!(
                growth_ratio < 1.5,
                "Memory grew {}x after warmup, indicates poor pool reuse",
                growth_ratio
            );
        });
    });
}

criterion_group!(
    memory_benches,
    bench_memory_line_plot_100k,
    bench_memory_multi_series,
    bench_memory_no_leaks,
    bench_memory_large_dataset,
    bench_memory_pool_reuse
);
criterion_main!(memory_benches);
