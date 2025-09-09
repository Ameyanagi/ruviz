//! T008: Parallel rendering contract test: 4x speedup verification
//! 
//! This test MUST FAIL initially - sequential rendering only
//! Target: 4-8x speedup on multi-core systems

use ruviz::core::Plot;
use std::time::Instant;
use std::thread;
use criterion::black_box;

/// Parallel rendering contract for multi-threaded speedup
/// MUST achieve 4-8x speedup on multi-core systems
#[test]
fn parallel_rendering_speedup_contract() {
    println!("🧪 PARALLEL RENDERING CONTRACT: Multi-core speedup verification");
    
    let cpu_cores = num_cpus::get();
    println!("🖥️  Available CPU cores: {}", cpu_cores);
    
    if cpu_cores < 4 {
        println!("⏭️  Skipping parallel test - need at least 4 cores (found {})", cpu_cores);
        return;
    }
    
    // Generate test dataset suitable for parallel processing
    let point_count = 500_000;
    let series_count = 8; // Multiple series to test parallel series rendering
    
    println!("📊 Generating {} series with {} points each", series_count, point_count);
    
    let datasets: Vec<(Vec<f64>, Vec<f64>)> = (0..series_count)
        .map(|series_id| {
            let x_data: Vec<f64> = (0..point_count)
                .map(|i| i as f64 * 0.01)
                .collect();
            let y_data: Vec<f64> = (0..point_count)
                .map(|i| ((i as f64 * 0.01) + series_id as f64).sin())
                .collect();
            (x_data, y_data)
        })
        .collect();
    
    // Test 1: Sequential rendering (baseline)
    println!("🔄 Testing sequential rendering (baseline)");
    let start_sequential = Instant::now();
    
    let mut sequential_plot = Plot::new().title("Sequential Rendering Test");
    
    for (series_id, (x_data, y_data)) in datasets.iter().enumerate() {
        // This should use sequential rendering (current implementation)  
        sequential_plot = sequential_plot.line(x_data, y_data);
    }
    
    let _sequential_result = sequential_plot.save("test_output/contract_sequential.png");
    let sequential_duration = start_sequential.elapsed();
    
    println!("⏱️  Sequential rendering: {:?}", sequential_duration);
    
    // Test 2: Parallel rendering (target implementation)
    println!("🚀 Testing parallel rendering (target)");
    let start_parallel = Instant::now();
    
    let mut parallel_plot = Plot::new()
        .title("Parallel Rendering Test");
        // Future API: .parallel_threads(cpu_cores);
    
    for (series_id, (x_data, y_data)) in datasets.iter().enumerate() {
        // This should use parallel rendering when implemented
        parallel_plot = parallel_plot.line(x_data, y_data);
    }
    
    let _parallel_result = parallel_plot.save("test_output/contract_parallel.png");
    let parallel_duration = start_parallel.elapsed();
    
    println!("⏱️  Parallel rendering: {:?}", parallel_duration);
    
    // Calculate speedup ratio
    let speedup_ratio = sequential_duration.as_nanos() as f64 / parallel_duration.as_nanos() as f64;
    println!("📈 Speedup ratio: {:.2}x", speedup_ratio);
    
    let expected_min_speedup = (cpu_cores / 2) as f64; // Conservative expectation
    let expected_max_speedup = cpu_cores as f64 * 0.8; // 80% efficiency
    
    println!("🎯 Expected speedup: {:.1}x - {:.1}x", expected_min_speedup, expected_max_speedup);
    
    // CRITICAL PARALLEL RENDERING CONTRACT
    // This MUST fail initially because parallel rendering is not implemented
    assert!(
        speedup_ratio >= expected_min_speedup,
        "❌ PARALLEL RENDERING CONTRACT VIOLATION: Only {:.2}x speedup, expected at least {:.1}x. Multi-threaded rendering required!",
        speedup_ratio,
        expected_min_speedup
    );
    
    println!("✅ CONTRACT PASSED: Parallel rendering achieved {:.2}x speedup", speedup_ratio);
}

/// Test parallel workload distribution
#[test]
fn parallel_workload_distribution() {
    println!("🧪 PARALLEL WORKLOAD: Testing work distribution across cores");
    
    let thread_counts = vec![1, 2, 4, 8];
    let mut results = Vec::new();
    
    // Create workload with multiple series of different complexities
    let datasets = create_mixed_complexity_datasets();
    
    for thread_count in thread_counts {
        if thread_count > num_cpus::get() {
            println!("⏭️  Skipping {} threads (only {} cores available)", thread_count, num_cpus::get());
            continue;
        }
        
        println!("🧵 Testing with {} threads", thread_count);
        
        let start = Instant::now();
        
        let mut plot = Plot::new()
            .title(&format!("Workload Distribution - {} threads", thread_count));
            // Future API: .parallel_threads(thread_count);
        
        for (x_data, y_data) in &datasets {
            plot = plot.scatter(x_data, y_data);
        }
        
        let _result = plot.save(&format!("test_output/workload_{}_threads.png", thread_count));
        let duration = start.elapsed();
        
        results.push((thread_count, duration));
        println!("⏱️  {} threads: {:?}", thread_count, duration);
    }
    
    // Validate that performance scales with thread count
    if results.len() >= 2 {
        let single_thread = results[0].1;
        let multi_thread = results.last().unwrap().1;
        let improvement = single_thread.as_nanos() as f64 / multi_thread.as_nanos() as f64;
        
        println!("📊 Overall improvement: {:.2}x", improvement);
        
        // For now, just validate that it doesn't get worse
        assert!(improvement >= 0.8, "Multi-threading should not degrade performance significantly");
    }
    
    println!("✅ Parallel workload distribution test completed");
}

/// Create datasets with varying computational complexity
fn create_mixed_complexity_datasets() -> Vec<(Vec<f64>, Vec<f64>)> {
    let mut datasets = Vec::new();
    
    // Simple linear dataset
    let x1: Vec<f64> = (0..50_000).map(|i| i as f64).collect();
    let y1: Vec<f64> = (0..50_000).map(|i| i as f64 * 0.01).collect();
    datasets.push((x1, y1));
    
    // Sine wave (more complex)
    let x2: Vec<f64> = (0..50_000).map(|i| i as f64 * 0.01).collect();
    let y2: Vec<f64> = (0..50_000).map(|i| (i as f64 * 0.01).sin()).collect();
    datasets.push((x2, y2));
    
    // Complex mathematical function
    let x3: Vec<f64> = (0..50_000).map(|i| i as f64 * 0.001).collect();
    let y3: Vec<f64> = (0..50_000).map(|i| {
        let t = i as f64 * 0.001;
        t.sin() * t.cos() + (t * 10.0).sin() * 0.1
    }).collect();
    datasets.push((x3, y3));
    
    // High-frequency noise (challenging for rendering)
    let x4: Vec<f64> = (0..50_000).map(|i| i as f64 * 0.001).collect();
    let y4: Vec<f64> = (0..50_000).map(|i| {
        let t = i as f64 * 0.001;
        (t * 100.0).sin() + (t * 300.0).cos() * 0.3 + (t * 1000.0).sin() * 0.1
    }).collect();
    datasets.push((x4, y4));
    
    datasets
}

/// Test thread safety of parallel rendering
#[test]
fn parallel_thread_safety() {
    println!("🧪 THREAD SAFETY: Testing concurrent plot creation");
    
    use std::sync::mpsc;
    use std::sync::{Arc, Barrier};
    
    let num_threads = 4;
    let (tx, rx) = mpsc::channel();
    let barrier = Arc::new(Barrier::new(num_threads));
    
    // Spawn multiple threads creating plots concurrently
    for thread_id in 0..num_threads {
        let tx_clone = tx.clone();
        let barrier_clone = Arc::clone(&barrier);
        
        thread::spawn(move || {
            // Generate unique dataset for this thread
            let x_data: Vec<f64> = (0..100_000)
                .map(|i| i as f64 * 0.01 + thread_id as f64 * 10.0)
                .collect();
            let y_data: Vec<f64> = (0..100_000)
                .map(|i| ((i as f64 * 0.01) + thread_id as f64).sin())
                .collect();
            
            // Wait for all threads to be ready
            barrier_clone.wait();
            
            let start = Instant::now();
            
            // Create plot
            let plot_result = Plot::new()
                .line(&x_data, &y_data)
                .title(&format!("Thread Safety Test - Thread {}", thread_id))
                .save(&format!("test_output/thread_safety_{}.png", thread_id));
            
            let duration = start.elapsed();
            
            tx_clone.send((thread_id, plot_result.is_ok(), duration)).unwrap();
        });
    }
    
    drop(tx);
    
    // Collect results from all threads
    let mut results = Vec::new();
    while let Ok(result) = rx.recv() {
        results.push(result);
    }
    
    assert_eq!(results.len(), num_threads, "Should receive results from all threads");
    
    let successful_threads = results.iter().filter(|(_, success, _)| *success).count();
    let avg_duration: f64 = results.iter()
        .map(|(_, _, duration)| duration.as_millis() as f64)
        .sum::<f64>() / results.len() as f64;
    
    println!("✅ {}/{} threads completed successfully", successful_threads, num_threads);
    println!("⏱️  Average duration: {:.0}ms", avg_duration);
    
    assert_eq!(successful_threads, num_threads, "All threads should complete successfully");
    
    println!("✅ Thread safety test passed");
}

/// Test resource contention under parallel load
#[test] 
fn parallel_resource_contention() {
    println!("🧪 RESOURCE CONTENTION: Testing performance under parallel load");
    
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    
    let completed_plots = Arc::new(AtomicUsize::new(0));
    let num_concurrent = 4;
    let plots_per_thread = 3;
    
    let handles: Vec<_> = (0..num_concurrent)
        .map(|thread_id| {
            let completed_clone = Arc::clone(&completed_plots);
            
            thread::spawn(move || {
                let mut durations = Vec::new();
                
                for plot_id in 0..plots_per_thread {
                    let x_data: Vec<f64> = (0..75_000)
                        .map(|i| i as f64 * 0.01 + thread_id as f64 * 5.0)
                        .collect();
                    let y_data: Vec<f64> = (0..75_000)
                        .map(|i| ((i as f64 * 0.01) + plot_id as f64).cos())
                        .collect();
                    
                    let start = Instant::now();
                    
                    let plot_result = Plot::new()
                        .scatter(&x_data, &y_data)
                        .title(&format!("Contention Test - T{}P{}", thread_id, plot_id));
                    
                    let duration = start.elapsed();
                    durations.push(duration);
                    
                    if plot_result.is_ok() {
                        completed_clone.fetch_add(1, Ordering::Relaxed);
                    }
                    
                    // Prevent optimization
                    black_box((plot_result, x_data, y_data));
                }
                
                durations
            })
        })
        .collect();
    
    // Wait for all threads and collect timing data
    let mut all_durations = Vec::new();
    for handle in handles {
        let thread_durations = handle.join().unwrap();
        all_durations.extend(thread_durations);
    }
    
    let total_completed = completed_plots.load(Ordering::Relaxed);
    let expected_total = num_concurrent * plots_per_thread;
    
    let avg_duration: f64 = all_durations.iter()
        .map(|d| d.as_millis() as f64)
        .sum::<f64>() / all_durations.len() as f64;
    
    let max_duration = all_durations.iter().max().unwrap();
    let min_duration = all_durations.iter().min().unwrap();
    
    println!("📊 Completed: {}/{} plots", total_completed, expected_total);
    println!("⏱️  Duration - Avg: {:.0}ms, Min: {:?}, Max: {:?}", 
        avg_duration, min_duration, max_duration);
    
    // Validate that resource contention doesn't cause failures
    assert_eq!(total_completed, expected_total, "All plots should complete under load");
    
    // Validate reasonable performance consistency
    let duration_ratio = max_duration.as_millis() as f64 / min_duration.as_millis() as f64;
    assert!(duration_ratio < 5.0, "Performance should be reasonably consistent: {:.1}x variation", duration_ratio);
    
    println!("✅ Resource contention test passed");
}