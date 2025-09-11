use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use ruviz::data::Data1D;
use ruviz::render::PooledRenderer;
use ruviz::core::Plot;

// Generate test data for benchmarking
fn generate_test_data(size: usize) -> Vec<f64> {
    (0..size).map(|i| (i as f64) * 0.1 + (i as f64 * 0.02).sin()).collect()
}

fn generate_position_data(size: usize) -> Vec<(f32, f32)> {
    (0..size).map(|i| (
        i as f32 * 0.5, 
        (i as f32 * 0.1).sin() * 10.0 
    )).collect()
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

// Benchmark tick generation with pooled vs traditional allocation
fn benchmark_tick_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("tick_generation");
    
    let renderer = PooledRenderer::new();
    
    for tick_count in [10, 25, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("pooled", tick_count),
            tick_count,
            |b, &count| {
                b.iter(|| {
                    let ticks = renderer.generate_ticks_pooled(
                        black_box(0.0),
                        black_box(100.0),
                        black_box(count)
                    );
                    black_box(ticks);
                })
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("traditional", tick_count),
            tick_count,
            |b, &count| {
                b.iter(|| {
                    let mut ticks = Vec::new();
                    let min = black_box(0.0);
                    let max = black_box(100.0);
                    
                    if count <= 1 {
                        ticks.push(min);
                    } else {
                        let step = (max - min) / (count - 1) as f64;
                        for i in 0..count {
                            ticks.push(min + i as f64 * step);
                        }
                    }
                    black_box(ticks);
                })
            },
        );
    }
    group.finish();
}

// Benchmark coordinate transformation memory reuse patterns  
fn benchmark_memory_reuse_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_reuse");
    
    let renderer = PooledRenderer::new();
    let test_data = generate_test_data(5000);
    
    // Test memory pool efficiency vs traditional allocation
    group.bench_function("pooled_coordinate_reuse", |b| {
        b.iter(|| {
            // Simulate multiple coordinate transformations that should reuse memory
            for _ in 0..5 {
                let result = renderer.transform_x_coordinates_pooled(
                    black_box(&test_data),
                    0.0, 
                    5000.0, 
                    0.0, 
                    800.0
                ).unwrap();
                black_box(result);
                // PooledVec should return memory to pool when dropped
            }
        })
    });
    
    group.bench_function("traditional_coordinate_allocation", |b| {
        b.iter(|| {
            // Traditional allocation - new Vec each time
            for _ in 0..5 {
                let mut result = Vec::with_capacity(test_data.len());
                let scale = 800.0 / 5000.0;
                
                for &x in &test_data {
                    result.push((x * scale) as f32);
                }
                black_box(result);
            }
        })
    });
    
    group.finish();
}



// Benchmark plotting pipeline with memory pooling enabled
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

// Memory allocation tracking benchmark
fn benchmark_allocation_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocation_tracking");
    
    let renderer = PooledRenderer::new();
    let test_data = generate_test_data(5000);
    
    group.bench_function("pooled_allocation_pattern", |b| {
        b.iter(|| {
            let start_stats = renderer.get_pool_stats();
            
            // Perform multiple operations
            let _coords = renderer.transform_x_coordinates_pooled(&test_data, 0.0, 5000.0, 0.0, 800.0).unwrap();
            let _ticks = renderer.generate_ticks_pooled(0.0, 100.0, 20);
            
            let end_stats = renderer.get_pool_stats();
            
            black_box((start_stats, end_stats));
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_coordinate_transformation,
    benchmark_tick_generation,
    benchmark_memory_reuse_patterns,
    benchmark_plot_pipeline,
    benchmark_allocation_patterns
);
criterion_main!(benches);