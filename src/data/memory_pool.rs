use std::collections::{HashSet, VecDeque};
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

/// A memory pool for efficient allocation and reuse of typed buffers
/// Reduces allocation overhead by reusing previously allocated memory
pub struct MemoryPool<T> {
    /// Available buffers ready for reuse
    available: VecDeque<Box<[T]>>,
    /// Pointers to buffers currently in use (for leak detection)
    in_use: HashSet<usize>,
    /// Default chunk size for new allocations
    chunk_size: usize,
    /// Maximum number of buffers to keep in the pool
    max_pools: usize,
    /// Total capacity across all buffers
    total_capacity: usize,
}

/// RAII wrapper for pooled memory that automatically returns buffer to pool on drop
pub struct PooledBuffer<T> {
    /// Non-null pointer to the buffer data
    data: NonNull<T>,
    /// Length of the buffer
    len: usize,
    /// Capacity of the underlying allocation
    capacity: usize,
    /// Reference to the pool for returning the buffer
    pool: Option<Arc<Mutex<MemoryPool<T>>>>,
    /// Underlying box for memory management
    _box: Option<Box<[T]>>,
}

unsafe impl<T: Send> Send for PooledBuffer<T> {}
unsafe impl<T: Sync> Sync for PooledBuffer<T> {}

impl<T> MemoryPool<T> {
    /// Create a new memory pool with the specified initial capacity
    pub fn new(initial_capacity: usize) -> Self {
        Self {
            available: VecDeque::new(),
            in_use: HashSet::new(),
            chunk_size: initial_capacity,
            max_pools: 10, // Default maximum of 10 buffers
            total_capacity: 0,
        }
    }

    /// Create a memory pool with custom configuration
    pub fn with_config(chunk_size: usize, max_pools: usize) -> Self {
        Self {
            available: VecDeque::new(),
            in_use: HashSet::new(),
            chunk_size,
            max_pools,
            total_capacity: 0,
        }
    }

    /// Acquire a buffer from the pool or allocate a new one
    pub fn acquire(&mut self, len: usize) -> PooledBuffer<T> {
        // Try to reuse an existing buffer that's large enough
        for i in 0..self.available.len() {
            if self.available[i].len() >= len {
                let buffer = self.available.remove(i).unwrap();
                let ptr = NonNull::new(buffer.as_ptr() as *mut T).unwrap();
                self.in_use.insert(ptr.as_ptr() as usize);

                return PooledBuffer {
                    data: ptr,
                    len,
                    capacity: buffer.len(),
                    pool: None, // Set when converting to managed buffer
                    _box: Some(buffer),
                };
            }
        }

        // No suitable buffer found, allocate a new one
        self.grow_pool(len);

        // Try again after growing
        if let Some(buffer) = self.available.pop_front() {
            let ptr = NonNull::new(buffer.as_ptr() as *mut T).unwrap();
            self.in_use.insert(ptr.as_ptr() as usize);

            PooledBuffer {
                data: ptr,
                len,
                capacity: buffer.len(),
                pool: None,
                _box: Some(buffer),
            }
        } else {
            panic!("Failed to allocate buffer after growing pool");
        }
    }

    /// Release a buffer back to the pool for reuse
    pub fn release(&mut self, mut buffer: PooledBuffer<T>) {
        if let Some(boxed) = buffer._box.take() {
            let ptr_addr = boxed.as_ptr() as usize;
            self.in_use.remove(&ptr_addr);

            // Only keep the buffer if we haven't exceeded max_pools
            if self.available.len() < self.max_pools {
                self.available.push_back(boxed);
            } else {
                // Let the buffer drop naturally, reducing total capacity
                self.total_capacity -= boxed.len();
            }
        }
    }

    /// Get the number of available buffers in the pool
    pub fn available_count(&self) -> usize {
        self.available.len()
    }

    /// Get the total capacity across all buffers
    pub fn total_capacity(&self) -> usize {
        self.total_capacity
    }

    /// Get the number of buffers currently in use
    pub fn in_use_count(&self) -> usize {
        self.in_use.len()
    }

    /// Shrink the pool by removing unused buffers
    pub fn shrink_unused(&mut self) {
        // Keep only half of the available buffers
        let target_size = (self.available.len() / 2).max(1);

        while self.available.len() > target_size {
            if let Some(buffer) = self.available.pop_back() {
                self.total_capacity -= buffer.len();
            }
        }
    }

    /// Grow the pool by adding a new buffer
    #[allow(clippy::uninit_vec)]
    fn grow_pool(&mut self, min_size: usize) {
        let size = min_size.max(self.chunk_size);
        // Allocate uninitialized memory - users must initialize before use
        // Safety: callers must ensure they initialize the buffer before reading
        let mut vec = Vec::with_capacity(size);
        unsafe {
            vec.set_len(size);
        }
        let buffer = vec.into_boxed_slice();
        self.total_capacity += buffer.len();
        self.available.push_back(buffer);
    }
}

impl<T> Default for MemoryPool<T> {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl<T> PooledBuffer<T> {
    /// Create a new unmanaged buffer (for direct usage)
    pub fn new_unmanaged(data: Box<[T]>) -> Self {
        let len = data.len();
        let ptr = NonNull::new(data.as_ptr() as *mut T).unwrap();

        Self {
            data: ptr,
            len,
            capacity: len,
            pool: None,
            _box: Some(data),
        }
    }

    /// Create a managed buffer that will be returned to the pool on drop
    pub fn new_managed(mut buffer: PooledBuffer<T>, pool: Arc<Mutex<MemoryPool<T>>>) -> Self {
        let boxed = buffer._box.take();
        Self {
            data: buffer.data,
            len: buffer.len,
            capacity: buffer.capacity,
            pool: Some(pool),
            _box: boxed,
        }
    }

    /// Get the length of the buffer
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the capacity of the buffer
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get a raw pointer to the buffer data
    pub fn as_ptr(&self) -> *const T {
        self.data.as_ptr()
    }

    /// Get a mutable raw pointer to the buffer data
    pub fn as_mut_ptr(&self) -> *mut T {
        self.data.as_ptr()
    }

    /// Resize the logical length of the buffer (must be <= capacity)
    pub fn resize(&mut self, new_len: usize) {
        assert!(new_len <= self.capacity, "Cannot resize beyond capacity");
        self.len = new_len;
    }

    /// Convert to a slice
    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.data.as_ptr(), self.len) }
    }

    /// Convert to a mutable slice
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.data.as_ptr(), self.len) }
    }
}

impl<T> Deref for PooledBuffer<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T> DerefMut for PooledBuffer<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T> Drop for PooledBuffer<T> {
    fn drop(&mut self) {
        // If this buffer is managed by a pool, return it
        if let Some(pool) = &self.pool {
            if let Ok(mut p) = pool.try_lock() {
                // Move the box out and release it
                if let Some(boxed) = self._box.take() {
                    let ptr_addr = boxed.as_ptr() as usize;
                    p.in_use.remove(&ptr_addr);

                    if p.available.len() < p.max_pools {
                        p.available.push_back(boxed);
                    } else {
                        p.total_capacity -= boxed.len();
                    }
                }
            }
            // If pool is locked, just let the buffer drop naturally
        }
        // For unmanaged buffers, _box will drop automatically
    }
}

impl<T: Clone> Clone for PooledBuffer<T> {
    fn clone(&self) -> Self {
        // Create a new unmanaged buffer with cloned data
        let cloned_data: Vec<T> = self.as_slice().to_vec();
        Self::new_unmanaged(cloned_data.into_boxed_slice())
    }
}

// Thread-safe wrapper for cross-thread pool sharing
#[derive(Clone)]
pub struct SharedMemoryPool<T> {
    inner: Arc<Mutex<MemoryPool<T>>>,
}

impl<T> SharedMemoryPool<T> {
    /// Create a new shared memory pool
    pub fn new(initial_capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(MemoryPool::new(initial_capacity))),
        }
    }

    /// Acquire a buffer from the shared pool
    pub fn acquire(&self, len: usize) -> PooledBuffer<T> {
        let buffer = {
            let mut pool = self.inner.lock().unwrap();
            pool.acquire(len)
        };

        // Convert to managed buffer
        PooledBuffer::new_managed(buffer, self.inner.clone())
    }

    /// Get pool statistics
    pub fn statistics(&self) -> PoolStatistics {
        let pool = self.inner.lock().unwrap();
        PoolStatistics {
            available_count: pool.available_count(),
            in_use_count: pool.in_use_count(),
            total_capacity: pool.total_capacity(),
        }
    }

    /// Shrink the pool
    pub fn shrink(&self) {
        let mut pool = self.inner.lock().unwrap();
        pool.shrink_unused();
    }
}

/// Statistics about memory pool usage
#[derive(Debug, Clone)]
pub struct PoolStatistics {
    pub available_count: usize,
    pub in_use_count: usize,
    pub total_capacity: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_creation() {
        let pool = MemoryPool::<f64>::new(1000);
        assert_eq!(pool.available_count(), 0);
        assert_eq!(pool.total_capacity(), 0);
        assert_eq!(pool.in_use_count(), 0);
    }

    #[test]
    fn test_buffer_operations() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let buffer = PooledBuffer::new_unmanaged(data.into_boxed_slice());

        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer[0], 1.0);
        assert_eq!(buffer[4], 5.0);

        let slice = buffer.as_slice();
        assert_eq!(slice.len(), 5);
        assert_eq!(slice, &[1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn test_shared_pool() {
        let shared_pool = SharedMemoryPool::<f64>::new(1000);
        let buffer = shared_pool.acquire(100);

        assert_eq!(buffer.len(), 100);

        let stats = shared_pool.statistics();
        assert_eq!(stats.in_use_count, 1);
    }
}
