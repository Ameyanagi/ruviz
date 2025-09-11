use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use ruviz::data::Data1D;
use ruviz::render::pooled::PooledRenderer;
use ruviz::core::Plot;

// Generate test data for benchmarking
fn generate_test_data(size: usize) -> Vec<f64> {
    (0..size).map(|i| (i as f64) * 0.1 + (i as f64 * 0.02).sin()).collect()
}

// Benchmark coordinate transformation with and without pooling
fn benchmark_coordinate_transformation(c: &mut Criterion) {
    let mut group = c.benchmark_group("coordinate_transformation");
    
    for size in [1000, 5000, 10000, 50000].iter() {
        let test_data = generate_test_data(*size);
        let renderer = PooledRenderer::new();
        
        // Benchmark pooled transformation
        group.bench_with_input(
            BenchmarkId::new("pooled", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let result = renderer.transform_x_coordinates_pooled(
                        black_box(&test_data),
                        0.0, 
                        size as f64 * 0.1, 
                        0.0, 
                        800.0
                    ).unwrap();
                    black_box(result);
                })
            },
        );
        
        // Benchmark traditional Vec allocation
        group.bench_with_input(
            BenchmarkId::new("traditional", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut result = Vec::with_capacity(test_data.len());
                    let range = size as f64 * 0.1;
                    let scale = 800.0 / range;
                    
                    for &x in &test_data {
                        result.push((x * scale) as f32);
                    }
                    black_box(result);
                })
            },
        );
    }
    group.finish();
}

// Benchmark memory pool reuse efficiency
fn benchmark_pool_reuse(c: &mut Criterion) {
    let mut group = c.benchmark_group("pool_reuse");
    
    let renderer = PooledRenderer::new();
    let test_data = generate_test_data(10000);
    
    group.bench_function("multiple_allocations_pooled", |b| {
        b.iter(|| {
            // Simulate multiple sequential operations that reuse memory
            for _ in 0..10 {
                let result = renderer.transform_x_coordinates_pooled(
                    black_box(&test_data),
                    0.0, 
                    1000.0, 
                    0.0, 
                    800.0
                ).unwrap();
                black_box(result);
                // PooledVec will return memory to pool when dropped
            }
        })
    });
    
    group.bench_function("multiple_allocations_traditional", |b| {
        b.iter(|| {
            // Simulate multiple sequential operations with traditional allocation
            for _ in 0..10 {
                let mut result = Vec::with_capacity(test_data.len());
                let scale = 800.0 / 1000.0;
                
                for &x in &test_data {
                    result.push((x * scale) as f32);
                }
                black_box(result);
                // Vec will be deallocated on each iteration
            }
        })
    });
    
    group.finish();
}

// Benchmark plotting pipeline with memory pooling
fn benchmark_plot_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("plot_pipeline");
    
    for size in [1000, 5000, 10000].iter() {
        let x_data = generate_test_data(*size);
        let y_data = generate_test_data(*size);
        
        group.bench_with_input(
            BenchmarkId::new("with_pooling", size),
            size,
            |b, &_size| {
                b.iter(|| {
                    let plot = Plot::new()
                        .with_memory_pooling(true)
                        .line(black_box(&x_data), black_box(&y_data));
                    black_box(plot);
                })
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("without_pooling", size),
            size,
            |b, &_size| {
                b.iter(|| {
                    let plot = Plot::new()
                        .with_memory_pooling(false)
                        .line(black_box(&x_data), black_box(&y_data));
                    black_box(plot);
                })
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    benchmark_coordinate_transformation,
    benchmark_pool_reuse,
    benchmark_plot_pipeline
);
criterion_main!(benches);