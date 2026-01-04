use ruviz::data::{Data1D, DataView, MemoryPool, PooledVec};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(test)]
mod memory_pool_tests {
    use super::*;

    #[test]
    fn test_pool_basic_allocation() {
        let mut pool = MemoryPool::<f64>::new(1000);

        // Test basic allocation
        let buffer1 = pool.acquire(100);
        assert_eq!(buffer1.len(), 100);

        // Test multiple allocations
        let buffer2 = pool.acquire(200);
        assert_eq!(buffer2.len(), 200);

        // Buffers should be different
        assert_ne!(buffer1.as_ptr(), buffer2.as_ptr());

        // Test release and reuse
        let ptr1 = buffer1.as_ptr();
        pool.release(buffer1);

        let buffer3 = pool.acquire(100);
        // Should reuse the same memory
        assert_eq!(buffer3.as_ptr(), ptr1);
    }

    #[test]
    fn test_pool_reuse_efficiency() {
        let mut pool = MemoryPool::<f64>::new(1000);
        let mut pointers = Vec::new();

        // Allocate multiple buffers and track their pointers
        for _ in 0..5 {
            let buffer = pool.acquire(100);
            pointers.push(buffer.as_ptr());
            pool.release(buffer);
        }

        // All allocations should reuse the same memory
        for ptr in pointers.iter().skip(1) {
            assert_eq!(*ptr, pointers[0]);
        }

        // Pool should only contain one buffer after all releases
        assert_eq!(pool.available_count(), 1);
    }

    #[test]
    fn test_pool_growth_and_shrink() {
        let mut pool = MemoryPool::<f64>::new(100);
        let mut buffers = Vec::new();

        // Allocate more buffers than initial capacity
        for _ in 0..5 {
            buffers.push(pool.acquire(100));
        }

        // Pool should have grown to accommodate
        assert!(pool.total_capacity() >= 500);

        // Release all buffers
        for buffer in buffers.drain(..) {
            pool.release(buffer);
        }

        // Pool should shrink over time (simulate with manual shrink)
        pool.shrink_unused();
        assert!(pool.total_capacity() < 500);
    }

    #[test]
    fn test_pooled_vec_api_compatibility() {
        let pool = Arc::new(Mutex::new(MemoryPool::<f64>::new(1000)));
        let mut vec = PooledVec::new(pool.clone());

        // Test Vec-like operations
        vec.push(1.0);
        vec.push(2.0);
        vec.push(3.0);

        assert_eq!(vec.len(), 3);
        assert_eq!(vec[0], 1.0);
        assert_eq!(vec[1], 2.0);
        assert_eq!(vec[2], 3.0);

        // Test iteration
        let sum: f64 = vec.iter().sum();
        assert_eq!(sum, 6.0);

        // Test slice operations
        let slice = &vec[1..];
        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0], 2.0);

        // Test extend
        vec.extend_from_slice(&[4.0, 5.0]);
        assert_eq!(vec.len(), 5);
    }

    #[test]
    fn test_zero_copy_data_view() {
        let pool = Arc::new(Mutex::new(MemoryPool::<f64>::new(1000)));
        let mut pooled_data = PooledVec::new(pool.clone());
        pooled_data.extend_from_slice(&[1.0, 2.0, 3.0, 4.0, 5.0]);

        // Create zero-copy view
        let data_view = DataView::from_pooled_vec(&pooled_data);

        // Test Data1D trait implementation
        assert_eq!(data_view.len(), 5);
        assert_eq!(data_view.get(0).unwrap().into(), 1.0);
        assert_eq!(data_view.get(4).unwrap().into(), 5.0);
        assert!(data_view.get(5).is_none());

        // Verify zero-copy (same memory address)
        assert_eq!(data_view.as_ptr(), pooled_data.as_ptr());

        // Test iterator
        let collected: Vec<f64> = data_view.iter().map(|x| x.into()).collect();
        assert_eq!(collected, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn test_thread_safety() {
        let pool = Arc::new(Mutex::new(MemoryPool::<f64>::new(1000)));
        let mut handles = vec![];

        // Spawn multiple threads that allocate and release buffers
        for i in 0..10 {
            let pool_clone = Arc::clone(&pool);
            let handle = thread::spawn(move || {
                for j in 0..100 {
                    let buffer = {
                        let mut p = pool_clone.lock().unwrap();
                        p.acquire(100 + i + j)
                    };

                    // Do some work with the buffer
                    thread::sleep(Duration::from_millis(1));

                    // Release back to pool
                    let mut p = pool_clone.lock().unwrap();
                    p.release(buffer);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify pool is in consistent state
        let p = pool.lock().unwrap();
        assert!(p.available_count() > 0);
        assert!(p.total_capacity() >= 1000);
    }

    #[test]
    fn test_memory_leak_prevention() {
        let mut pool = MemoryPool::<f64>::new(100);
        let initial_capacity = pool.total_capacity();

        // Perform many allocation cycles
        for _ in 0..1000 {
            let buffer = pool.acquire(100);
            // Simulate work
            for i in 0..100 {
                unsafe {
                    *buffer.as_mut_ptr().add(i) = i as f64;
                }
            }
            pool.release(buffer);
        }

        // Memory should not have grown significantly
        let final_capacity = pool.total_capacity();
        assert!(final_capacity <= initial_capacity * 2);

        // All buffers should be available for reuse
        assert!(pool.available_count() > 0);
    }

    #[test]
    fn test_different_pool_types() {
        // Test f64 pool for coordinates
        let mut f64_pool = MemoryPool::<f64>::new(1000);
        let coord_buffer = f64_pool.acquire(1000);
        assert_eq!(std::mem::size_of_val(&*coord_buffer), 1000 * 8);
        f64_pool.release(coord_buffer);

        // Test u8 pool for pixel data
        let mut u8_pool = MemoryPool::<u8>::new(1000);
        let pixel_buffer = u8_pool.acquire(1920 * 1080 * 4);
        assert_eq!(pixel_buffer.len(), 1920 * 1080 * 4);
        u8_pool.release(pixel_buffer);

        // Test string pool for text data
        let mut string_pool = MemoryPool::<char>::new(1000);
        let text_buffer = string_pool.acquire(256);
        assert_eq!(text_buffer.len(), 256);
        string_pool.release(text_buffer);
    }

    #[test]
    fn test_pool_with_plot_integration() {
        use ruviz::prelude::*;

        // This test verifies pool integration with actual plotting
        let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let y_data = vec![0.0, 1.0, 0.0, 1.0, 0.0];

        // Create plot with pooled memory enabled using actual API
        let result = Plot::new()
            .with_memory_pooling(true)
            .line(&x_data, &y_data)
            .title("Pool Integration Test")
            .xlabel("X Values")
            .ylabel("Y Values")
            .save("tests/output/pool_integration_test.png");

        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;

    #[test]
    fn test_allocation_overhead_reduction() {
        const NUM_ITERATIONS: usize = 1000;
        const BUFFER_SIZE: usize = 10000;

        // Measure traditional allocation time
        let start = Instant::now();
        for _ in 0..NUM_ITERATIONS {
            let _vec: Vec<f64> = vec![0.0; BUFFER_SIZE];
        }
        let traditional_time = start.elapsed();

        // Measure pooled allocation time
        let mut pool = MemoryPool::<f64>::new(BUFFER_SIZE);
        let start = Instant::now();
        for _ in 0..NUM_ITERATIONS {
            let buffer = pool.acquire(BUFFER_SIZE);
            pool.release(buffer);
        }
        let pooled_time = start.elapsed();

        // Pooled allocation should be significantly faster
        println!(
            "Traditional: {:?}, Pooled: {:?}",
            traditional_time, pooled_time
        );
        assert!(pooled_time < traditional_time / 2);
    }

    #[test]
    fn test_large_plot_memory_efficiency() {
        use ruviz::prelude::*;

        // Generate large dataset
        let size = 100_000;
        let x_data: Vec<f64> = (0..size).map(|i| i as f64).collect();
        let y_data: Vec<f64> = (0..size).map(|i| (i as f64).sin()).collect();

        // Measure memory usage with pooling
        let plot = Plot::with_pool_config(PoolConfig::default());

        let start_memory = get_memory_usage();
        let _result = plot
            .line(&x_data, &y_data)
            .title("Large Dataset Test")
            .save("tests/output/large_plot_memory_test.png");
        let end_memory = get_memory_usage();

        let memory_growth = end_memory - start_memory;

        // Memory growth should be reasonable (< 3x data size)
        let data_size = (x_data.len() + y_data.len()) * std::mem::size_of::<f64>();
        assert!(memory_growth < data_size * 3);
    }

    #[test]
    fn test_steady_state_rendering() {
        use ruviz::prelude::*;

        const NUM_PLOTS: usize = 100;
        let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let y_data = vec![0.0, 1.0, 0.0, 1.0, 0.0];

        let plot = Plot::with_pool_config(PoolConfig::default());
        let start_memory = get_memory_usage();

        // Render many plots in succession
        for i in 0..NUM_PLOTS {
            let _result = plot
                .line(&x_data, &y_data)
                .title(&format!("Plot {}", i))
                .save(&format!("tests/output/steady_state_{}.png", i));
        }

        let end_memory = get_memory_usage();
        let memory_growth = end_memory - start_memory;

        // Memory should not grow significantly in steady state
        let expected_max_growth = 10 * 1024 * 1024; // 10MB max growth
        assert!(memory_growth < expected_max_growth);
    }

    // Helper function to measure memory usage (implementation depends on platform)
    #[cfg(target_os = "linux")]
    fn get_memory_usage() -> usize {
        use std::fs;
        let contents = fs::read_to_string("/proc/self/status").unwrap();
        for line in contents.lines() {
            if line.starts_with("VmRSS:") {
                let kb: usize = line.split_whitespace().nth(1).unwrap().parse().unwrap();
                return kb * 1024; // Convert to bytes
            }
        }
        0
    }

    #[cfg(not(target_os = "linux"))]
    fn get_memory_usage() -> usize {
        // Placeholder for other platforms
        0
    }
}

// Mock implementations for testing (these will be replaced by real implementations)
#[cfg(test)]
mod mocks {
    use super::*;

    pub struct MemoryPool<T> {
        _phantom: std::marker::PhantomData<T>,
    }

    impl<T> MemoryPool<T> {
        pub fn new(_capacity: usize) -> Self {
            panic!("MemoryPool not implemented yet");
        }

        pub fn acquire(&mut self, _len: usize) -> PooledBuffer<T> {
            panic!("MemoryPool::acquire not implemented yet");
        }

        pub fn release(&mut self, _buffer: PooledBuffer<T>) {
            panic!("MemoryPool::release not implemented yet");
        }

        pub fn available_count(&self) -> usize {
            panic!("MemoryPool::available_count not implemented yet");
        }

        pub fn total_capacity(&self) -> usize {
            panic!("MemoryPool::total_capacity not implemented yet");
        }

        pub fn shrink_unused(&mut self) {
            panic!("MemoryPool::shrink_unused not implemented yet");
        }
    }

    pub struct PooledBuffer<T> {
        _phantom: std::marker::PhantomData<T>,
    }

    impl<T> PooledBuffer<T> {
        pub fn len(&self) -> usize {
            panic!("PooledBuffer::len not implemented yet");
        }

        pub fn as_ptr(&self) -> *const T {
            panic!("PooledBuffer::as_ptr not implemented yet");
        }

        pub fn as_mut_ptr(&self) -> *mut T {
            panic!("PooledBuffer::as_mut_ptr not implemented yet");
        }
    }

    pub struct PooledVec<T> {
        _phantom: std::marker::PhantomData<T>,
    }

    impl<T> PooledVec<T> {
        pub fn new(_pool: Arc<Mutex<MemoryPool<T>>>) -> Self {
            panic!("PooledVec::new not implemented yet");
        }
    }

    pub struct DataView<T> {
        _phantom: std::marker::PhantomData<T>,
    }

    impl<T> DataView<T> {
        pub fn from_pooled_vec(_vec: &PooledVec<T>) -> Self {
            panic!("DataView::from_pooled_vec not implemented yet");
        }
    }

    pub struct PoolConfig {
        pub coordinate_pool_size: usize,
        pub pixel_pool_size: usize,
        pub text_pool_size: usize,
        pub max_pools_per_type: usize,
        pub enable_cross_thread_sharing: bool,
    }

    impl Default for PoolConfig {
        fn default() -> Self {
            panic!("PoolConfig::default not implemented yet");
        }
    }

    pub struct PoolStatistics {
        pub total_allocations: usize,
        pub pool_hits: usize,
        pub memory_leaks: usize,
    }
}
