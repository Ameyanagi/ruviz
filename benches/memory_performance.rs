use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ruviz::data::{MemoryPool, PooledVec};
use ruviz::prelude::*;
use std::sync::{Arc, Mutex};

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
        
        // Benchmark pooled allocation with reuse
        group.bench_with_input(
            BenchmarkId::new("pool_allocation_reuse", size),
            size, 
            |b, &size| {
                let mut pool = MemoryPool::<f64>::new(size * 2);
                // Pre-warm the pool
                let buffer = pool.acquire(size);
                pool.release(buffer);
                
                b.iter(|| {
                    let buffer = pool.acquire(size);
                    pool.release(buffer);
                });
            },
        );
    }
    group.finish();
}

fn bench_large_plot_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_plot_memory");
    
    for points in [1000, 10000, 100000].iter() {
        let x_data: Vec<f64> = (0..*points).map(|i| i as f64).collect();
        let y_data: Vec<f64> = (0..*points).map(|i| (i as f64).sin()).collect();
        
        group.throughput(Throughput::Elements(*points as u64));
        
        // Benchmark without memory pooling
        group.bench_with_input(
            BenchmarkId::new("traditional_rendering", points),
            &(x_data.clone(), y_data.clone()),
            |b, (x, y)| {
                b.iter(|| {
                    let plot = Plot::new();
                    let _result = plot
                        .line(x, y)
                        .title("Performance Test")
                        .save(&format!("test_output/bench_traditional_{}.png", points));
                });
            },
        );
        
        // Benchmark with memory pooling
        group.bench_with_input(
            BenchmarkId::new("pooled_rendering", points),
            &(x_data, y_data),
            |b, (x, y)| {
                let pool_config = PoolConfig {
                    coordinate_pool_size: *points,
                    pixel_pool_size: 8 * 1024 * 1024, // 8MB
                    text_pool_size: 256 * 1024, // 256KB
                    max_pools_per_type: 5,
                    enable_cross_thread_sharing: false,
                };
                
                b.iter(|| {
                    let plot = Plot::with_pool_config(pool_config.clone());
                    let _result = plot
                        .line(x, y)
                        .title("Performance Test")
                        .save(&format!("test_output/bench_pooled_{}.png", points));
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
    
    // Benchmark memory growth over time
    group.bench_function("steady_state_100_plots", |b| {
        let pool_config = PoolConfig::default();
        let plot = Plot::with_pool_config(pool_config);
        
        b.iter(|| {
            for i in 0..100 {
                let _result = plot
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
    
    for threads in [1, 2, 4, 8].iter() {
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
                                let pool_config = PoolConfig {
                                    enable_cross_thread_sharing: true,
                                    ..PoolConfig::default()
                                };
                                let plot = Plot::with_pool_config(pool_config);
                                
                                for i in 0..10 {
                                    let _result = plot
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
    
    // Benchmark pool acquire/release patterns
    group.bench_function("acquire_release_cycle", |b| {
        let mut pool = MemoryPool::<f64>::new(1000);
        b.iter(|| {
            let buffer = pool.acquire(1000);
            // Simulate some work
            unsafe {
                for i in 0..1000 {
                    *buffer.as_mut_ptr().add(i) = i as f64;
                }
            }
            pool.release(buffer);
        });
    });
    
    // Benchmark pool growth behavior
    group.bench_function("pool_growth", |b| {
        b.iter(|| {
            let mut pool = MemoryPool::<f64>::new(100);
            let mut buffers = Vec::new();
            
            // Allocate more than initial capacity
            for _ in 0..10 {
                buffers.push(pool.acquire(100));
            }
            
            // Release all
            for buffer in buffers {
                pool.release(buffer);
            }
        });
    });
    
    // Benchmark PooledVec operations
    group.bench_function("pooled_vec_operations", |b| {
        let pool = Arc::new(Mutex::new(MemoryPool::<f64>::new(10000)));
        
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

fn bench_data_view_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_view_operations");
    
    let pool = Arc::new(Mutex::new(MemoryPool::<f64>::new(10000)));
    let mut pooled_data = PooledVec::new(pool);
    for i in 0..10000 {
        pooled_data.push(i as f64);
    }
    
    // Benchmark zero-copy view creation
    group.bench_function("create_data_view", |b| {
        b.iter(|| {
            let view = DataView::from_pooled_vec(&pooled_data);
            criterion::black_box(view);
        });
    });
    
    // Benchmark Data1D trait operations on view
    group.bench_function("data_view_iteration", |b| {
        let view = DataView::from_pooled_vec(&pooled_data);
        
        b.iter(|| {
            let sum: f64 = (0..view.len())
                .map(|i| view.get(i).unwrap().into())
                .sum();
            criterion::black_box(sum);
        });
    });
    
    // Compare with traditional Vec operations
    group.bench_function("vec_iteration", |b| {
        let vec_data: Vec<f64> = (0..10000).map(|i| i as f64).collect();
        
        b.iter(|| {
            let sum: f64 = vec_data.iter().sum();
            criterion::black_box(sum);
        });
    });
    
    group.finish();
}

// Mock implementations for benchmarking (will be replaced by real implementations)
#[cfg(bench)]
mod mock_implementations {
    use super::*;
    
    // These will be replaced by the actual implementations
    impl MemoryPool<f64> {
        pub fn new(_capacity: usize) -> Self {
            unimplemented!("MemoryPool not implemented yet")
        }
        
        pub fn acquire(&mut self, _len: usize) -> PooledBuffer<f64> {
            unimplemented!("MemoryPool::acquire not implemented yet")
        }
        
        pub fn release(&mut self, _buffer: PooledBuffer<f64>) {
            unimplemented!("MemoryPool::release not implemented yet")
        }
    }
    
    impl Plot {
        pub fn with_pool_config(_config: PoolConfig) -> Self {
            unimplemented!("Plot::with_pool_config not implemented yet")
        }
    }
    
    #[derive(Clone)]
    pub struct PoolConfig {
        pub coordinate_pool_size: usize,
        pub pixel_pool_size: usize,
        pub text_pool_size: usize,
        pub max_pools_per_type: usize,
        pub enable_cross_thread_sharing: bool,
    }
    
    impl Default for PoolConfig {
        fn default() -> Self {
            unimplemented!("PoolConfig::default not implemented yet")
        }
    }
}

criterion_group!(
    benches,
    bench_allocation_overhead,
    bench_large_plot_memory,
    bench_steady_state_rendering,
    bench_concurrent_rendering,
    bench_memory_pool_operations,
    bench_data_view_operations
);
criterion_main!(benches);