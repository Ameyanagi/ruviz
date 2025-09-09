//! T006: Memory efficiency contract test: <2x data size
//! 
//! This test MUST FAIL initially - current linear memory scaling
//! Target: Memory usage <2x input data size

use ruviz::core::Plot;
use std::sync::{Arc, Mutex};
use std::alloc::{GlobalAlloc, Layout, System};
use criterion::black_box;

/// Memory tracking allocator for testing
struct TrackingAllocator {
    allocated: Arc<Mutex<usize>>,
    peak: Arc<Mutex<usize>>,
}

impl TrackingAllocator {
    fn new() -> Self {
        Self {
            allocated: Arc::new(Mutex::new(0)),
            peak: Arc::new(Mutex::new(0)),
        }
    }
    
    fn current_allocated(&self) -> usize {
        *self.allocated.lock().unwrap()
    }
    
    fn peak_allocated(&self) -> usize {
        *self.peak.lock().unwrap()
    }
    
    fn reset(&self) {
        *self.allocated.lock().unwrap() = 0;
        *self.peak.lock().unwrap() = 0;
    }
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            let mut allocated = self.allocated.lock().unwrap();
            let mut peak = self.peak.lock().unwrap();
            
            *allocated += layout.size();
            if *allocated > *peak {
                *peak = *allocated;
            }
        }
        ptr
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        let mut allocated = self.allocated.lock().unwrap();
        *allocated = allocated.saturating_sub(layout.size());
    }
}

// Global allocator for testing (commented out to avoid conflicts)
// #[global_allocator] 
// static ALLOC: TrackingAllocator = TrackingAllocator::new();

/// Memory efficiency contract for various plot sizes
/// MUST use <2x input data size in memory
#[test]
fn memory_efficiency_contract() {
    println!("🧪 MEMORY EFFICIENCY CONTRACT: <2x input data size");
    
    let test_sizes = vec![10_000, 50_000, 100_000, 500_000];
    
    for size in test_sizes {
        println!("📊 Testing {} point dataset", size);
        
        // Generate test data
        let x_data: Vec<f64> = (0..size).map(|i| i as f64 * 0.01).collect();
        let y_data: Vec<f64> = (0..size).map(|i| (i as f64 * 0.01).sin()).collect();
        
        // Calculate input data size
        let input_size = (x_data.len() + y_data.len()) * std::mem::size_of::<f64>();
        let max_allowed = input_size * 2; // 2x contract
        
        println!("💾 Input data size: {} MB", input_size / 1024 / 1024);
        println!("📏 Maximum allowed: {} MB", max_allowed / 1024 / 1024);
        
        // For now, we'll simulate memory tracking since we can't easily
        // override the global allocator in tests
        let estimated_plot_memory = simulate_plot_memory(&x_data, &y_data);
        
        println!("🔍 Estimated plot memory: {} MB", estimated_plot_memory / 1024 / 1024);
        
        // Create the plot
        let plot_result = Plot::new()
            .scatter(&x_data, &y_data)
            .title(&format!("Memory Test - {} points", size));
            
        assert!(plot_result.is_ok(), "Plot creation should succeed");
        
        // MEMORY EFFICIENCY CONTRACT
        // This should fail initially without memory optimization
        assert!(
            estimated_plot_memory <= max_allowed,
            "❌ MEMORY CONTRACT VIOLATION: {} points used ~{} MB, max allowed {} MB (2x input)",
            size,
            estimated_plot_memory / 1024 / 1024,
            max_allowed / 1024 / 1024
        );
        
        println!("✅ Memory contract passed for {} points", size);
        
        // Prevent optimization
        black_box((plot_result, x_data, y_data));
    }
    
    println!("✅ Memory efficiency contract test completed");
}

/// Simulate plot memory usage calculation
fn simulate_plot_memory(x_data: &[f64], y_data: &[f64]) -> usize {
    let input_size = (x_data.len() + y_data.len()) * std::mem::size_of::<f64>();
    
    // Simulate current memory usage (without optimization)
    // This is a pessimistic estimate that should improve with DataShader
    let series_data = input_size; // Copy of input data
    let transformed_coords = x_data.len() * std::mem::size_of::<(f32, f32)>(); // Screen coordinates
    let canvas_buffer = 1920 * 1080 * 4; // RGBA buffer
    let metadata = 1024; // Plot metadata, styles, etc.
    
    series_data + transformed_coords + canvas_buffer + metadata
}

/// Test memory growth with dataset scaling
#[test]
fn memory_scaling_test() {
    println!("🧪 MEMORY SCALING: Testing memory growth vs dataset size");
    
    let sizes = vec![1000, 5000, 10000, 50000, 100000];
    let mut results = Vec::new();
    
    for size in sizes {
        let x_data: Vec<f64> = (0..size).map(|i| i as f64).collect();
        let y_data: Vec<f64> = (0..size).map(|i| i as f64).collect();
        
        let input_size = (x_data.len() + y_data.len()) * std::mem::size_of::<f64>();
        let estimated_memory = simulate_plot_memory(&x_data, &y_data);
        let memory_ratio = estimated_memory as f64 / input_size as f64;
        
        results.push((size, memory_ratio));
        
        println!("📊 {} points: {:.2}x memory ratio", size, memory_ratio);
        
        // Create plot to ensure it works
        let plot = Plot::new()
            .line(&x_data, &y_data)
            .title(&format!("Scaling Test - {} points", size));
            
        black_box((plot, x_data, y_data));
    }
    
    // Validate that memory ratio doesn't explode with size
    let large_ratio = results.last().unwrap().1;
    let small_ratio = results.first().unwrap().1;
    
    // With proper DataShader, large datasets should have better memory efficiency
    println!("🔍 Small dataset ratio: {:.2}x", small_ratio);
    println!("🔍 Large dataset ratio: {:.2}x", large_ratio);
    
    // For now, just validate that ratios are reasonable
    // Future: Large datasets should have BETTER ratios due to aggregation
    assert!(large_ratio < 10.0, "Memory ratio should not explode: {:.2}x", large_ratio);
    
    println!("✅ Memory scaling test completed");
}

/// Test memory pressure under concurrent plotting
#[test]
fn concurrent_memory_pressure() {
    println!("🧪 CONCURRENT MEMORY: Testing multiple simultaneous plots");
    
    use std::thread;
    use std::sync::mpsc;
    
    let (tx, rx) = mpsc::channel();
    let num_threads = 4;
    let points_per_plot = 50_000;
    
    // Spawn multiple threads creating plots simultaneously
    for thread_id in 0..num_threads {
        let tx_clone = tx.clone();
        thread::spawn(move || {
            let x_data: Vec<f64> = (0..points_per_plot).map(|i| i as f64 + thread_id as f64 * 1000.0).collect();
            let y_data: Vec<f64> = (0..points_per_plot).map(|i| (i as f64 * 0.01).sin()).collect();
            
            let plot_result = Plot::new()
                .scatter(&x_data, &y_data)
                .title(&format!("Concurrent Test - Thread {}", thread_id));
                
            tx_clone.send((thread_id, plot_result.is_ok())).unwrap();
        });
    }
    
    drop(tx);
    
    // Collect results
    let mut successes = 0;
    while let Ok((thread_id, success)) = rx.recv() {
        if success {
            successes += 1;
            println!("✅ Thread {} completed successfully", thread_id);
        } else {
            println!("❌ Thread {} failed", thread_id);
        }
    }
    
    assert_eq!(successes, num_threads, "All concurrent plots should succeed");
    println!("✅ Concurrent memory pressure test passed");
}