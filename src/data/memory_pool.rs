use std::collections::{HashSet, VecDeque};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

/// A memory pool for efficient allocation and reuse of typed buffers.
///
/// The pool stores empty vectors with preserved capacity. This avoids unsafe
/// uninitialized element handling while still reducing allocation churn.
pub struct MemoryPool<T> {
    /// Available buffers ready for reuse.
    available: VecDeque<Vec<T>>,
    /// Pointers to buffers currently in use (for leak/debug statistics).
    in_use: HashSet<usize>,
    /// Default chunk size for new allocations.
    chunk_size: usize,
    /// Maximum number of buffers to keep in the pool.
    max_pools: usize,
    /// Total capacity across all tracked buffers.
    total_capacity: usize,
}

/// RAII wrapper for pooled memory that automatically returns buffer to pool on drop.
pub struct PooledBuffer<T> {
    vec: Option<Vec<T>>,
    pool: Option<Arc<Mutex<MemoryPool<T>>>>,
}

impl<T> MemoryPool<T> {
    /// Create a new memory pool with the specified initial capacity.
    pub fn new(initial_capacity: usize) -> Self {
        Self {
            available: VecDeque::new(),
            in_use: HashSet::new(),
            chunk_size: initial_capacity,
            max_pools: 10,
            total_capacity: 0,
        }
    }

    /// Create a memory pool with custom configuration.
    pub fn with_config(chunk_size: usize, max_pools: usize) -> Self {
        Self {
            available: VecDeque::new(),
            in_use: HashSet::new(),
            chunk_size,
            max_pools,
            total_capacity: 0,
        }
    }

    /// Acquire a buffer from the pool or allocate a new one.
    ///
    /// The returned vector has `len == 0` and `capacity >= min_capacity`.
    pub fn acquire(&mut self, min_capacity: usize) -> PooledBuffer<T> {
        let mut vec = self
            .take_buffer(min_capacity)
            .unwrap_or_else(|| self.allocate_buffer(min_capacity));

        vec.clear();
        self.in_use.insert(vec.as_ptr() as usize);

        PooledBuffer {
            vec: Some(vec),
            pool: None,
        }
    }

    /// Release a buffer back to the pool for reuse.
    pub fn release(&mut self, mut buffer: PooledBuffer<T>) {
        if let Some(vec) = buffer.vec.take() {
            self.release_vec(vec);
        }
    }

    /// Release a raw vector back to the pool.
    pub fn release_vec(&mut self, mut vec: Vec<T>) {
        self.in_use.remove(&(vec.as_ptr() as usize));
        vec.clear();

        if self.available.len() < self.max_pools {
            self.available.push_back(vec);
        } else {
            self.total_capacity = self.total_capacity.saturating_sub(vec.capacity());
        }
    }

    /// Get the number of available buffers in the pool.
    pub fn available_count(&self) -> usize {
        self.available.len()
    }

    /// Get the total capacity across all buffers.
    pub fn total_capacity(&self) -> usize {
        self.total_capacity
    }

    /// Get the number of buffers currently in use.
    pub fn in_use_count(&self) -> usize {
        self.in_use.len()
    }

    /// Shrink the pool by removing unused buffers.
    pub fn shrink_unused(&mut self) {
        if self.available.is_empty() {
            return;
        }

        let target_size = (self.available.len() / 2).max(1);
        while self.available.len() > target_size {
            if let Some(vec) = self.available.pop_back() {
                self.total_capacity = self.total_capacity.saturating_sub(vec.capacity());
            }
        }
    }

    fn take_buffer(&mut self, min_capacity: usize) -> Option<Vec<T>> {
        for i in 0..self.available.len() {
            if self.available[i].capacity() >= min_capacity {
                return self.available.remove(i);
            }
        }
        None
    }

    fn allocate_buffer(&mut self, min_capacity: usize) -> Vec<T> {
        let capacity = min_capacity.max(self.chunk_size);
        self.total_capacity += capacity;
        Vec::with_capacity(capacity)
    }
}

impl<T> Default for MemoryPool<T> {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl<T> PooledBuffer<T> {
    /// Create a new unmanaged buffer from owned vector data.
    pub fn new_unmanaged(data: Vec<T>) -> Self {
        Self {
            vec: Some(data),
            pool: None,
        }
    }

    /// Create a managed buffer that will be returned to the pool on drop.
    ///
    /// This is crate-internal to keep managed construction on lock-order-safe paths.
    pub(crate) fn new_managed(
        mut buffer: PooledBuffer<T>,
        pool: Arc<Mutex<MemoryPool<T>>>,
    ) -> Self {
        let vec = buffer.vec.take();
        Self {
            vec,
            pool: Some(pool),
        }
    }

    /// Get the length of the buffer.
    pub fn len(&self) -> usize {
        self.vec.as_ref().map_or(0, Vec::len)
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the capacity of the buffer.
    pub fn capacity(&self) -> usize {
        self.vec.as_ref().map_or(0, Vec::capacity)
    }

    /// Get a raw pointer to the buffer data.
    pub fn as_ptr(&self) -> *const T {
        self.vec.as_ref().map_or(std::ptr::null(), Vec::as_ptr)
    }

    /// Get a mutable raw pointer to the buffer data.
    pub fn as_mut_ptr(&self) -> *mut T {
        self.as_ptr() as *mut T
    }

    /// Resize the logical length of the buffer.
    pub fn resize(&mut self, new_len: usize, value: T)
    where
        T: Clone,
    {
        if let Some(vec) = self.vec.as_mut() {
            assert!(
                new_len <= vec.capacity(),
                "requested length {new_len} exceeds pooled buffer capacity {}",
                vec.capacity()
            );
            vec.resize(new_len, value);
        }
    }

    /// Resize the logical length of the buffer, clamping to current capacity.
    ///
    /// This keeps pointer identity stable while the buffer is tracked in `in_use`.
    pub fn resize_clamped(&mut self, new_len: usize, value: T)
    where
        T: Clone,
    {
        if let Some(vec) = self.vec.as_mut() {
            let target_len = new_len.min(vec.capacity());
            vec.resize(target_len, value);
        }
    }

    /// Convert to a slice.
    pub fn as_slice(&self) -> &[T] {
        self.vec.as_deref().unwrap_or(&[])
    }

    /// Convert to a mutable slice.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.vec.as_deref_mut().unwrap_or(&mut [])
    }

    /// Extract the underlying vector and disable pool return on drop.
    pub fn into_inner(mut self) -> Vec<T> {
        self.pool = None;
        self.vec.take().unwrap_or_default()
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
        if let Some(pool) = &self.pool {
            if let Some(vec) = self.vec.take() {
                let mut p = pool.lock().unwrap_or_else(|e| e.into_inner());
                p.release_vec(vec);
            }
        }
    }
}

impl<T: Clone> Clone for PooledBuffer<T> {
    fn clone(&self) -> Self {
        Self::new_unmanaged(self.as_slice().to_vec())
    }
}

/// Thread-safe wrapper for cross-thread pool sharing.
pub struct SharedMemoryPool<T> {
    inner: Arc<Mutex<MemoryPool<T>>>,
}

impl<T> Clone for SharedMemoryPool<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> SharedMemoryPool<T> {
    /// Create a new shared memory pool.
    pub fn new(initial_capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(MemoryPool::new(initial_capacity))),
        }
    }

    /// Acquire a buffer from the shared pool.
    pub fn acquire(&self, min_capacity: usize) -> PooledBuffer<T> {
        let buffer = {
            let mut pool = self.inner.lock().unwrap();
            pool.acquire(min_capacity)
        };
        PooledBuffer::new_managed(buffer, self.inner.clone())
    }

    /// Release a vector back to the shared pool.
    pub fn release_vec(&self, vec: Vec<T>) {
        let mut pool = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        pool.release_vec(vec);
    }

    /// Get pool statistics.
    pub fn statistics(&self) -> PoolStatistics {
        let pool = self.inner.lock().unwrap();
        PoolStatistics {
            available_count: pool.available_count(),
            in_use_count: pool.in_use_count(),
            total_capacity: pool.total_capacity(),
        }
    }

    /// Shrink the pool.
    pub fn shrink(&self) {
        let mut pool = self.inner.lock().unwrap();
        pool.shrink_unused();
    }
}

/// Statistics about memory pool usage.
#[derive(Debug, Clone)]
pub struct PoolStatistics {
    pub available_count: usize,
    pub in_use_count: usize,
    pub total_capacity: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;
    use std::thread;
    use std::time::Duration;

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
        let buffer = PooledBuffer::new_unmanaged(data);

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

        assert_eq!(buffer.len(), 0);
        assert!(buffer.capacity() >= 100);

        let stats = shared_pool.statistics();
        assert_eq!(stats.in_use_count, 1);
    }

    #[test]
    fn test_resize_clamped_to_capacity_without_reallocation() {
        let shared_pool = SharedMemoryPool::<u8>::new(8);
        let mut buffer = shared_pool.acquire(8);
        let ptr_before = buffer.as_ptr();
        let cap = buffer.capacity();

        buffer.resize_clamped(cap + 32, 7);

        assert_eq!(buffer.len(), cap);
        assert_eq!(buffer.as_ptr(), ptr_before);
    }

    #[test]
    fn test_resize_preserves_requested_len_within_capacity() {
        let shared_pool = SharedMemoryPool::<u8>::new(8);
        let mut buffer = shared_pool.acquire(8);

        buffer.resize(6, 3);

        assert_eq!(buffer.len(), 6);
        assert_eq!(buffer.as_slice(), &[3, 3, 3, 3, 3, 3]);
    }

    #[test]
    #[should_panic(expected = "requested length")]
    fn test_resize_panics_when_requested_len_exceeds_capacity() {
        let shared_pool = SharedMemoryPool::<u8>::new(8);
        let mut buffer = shared_pool.acquire(8);
        let cap = buffer.capacity();

        buffer.resize(cap + 1, 9);
    }

    #[test]
    fn test_drop_waits_for_lock_and_cleans_in_use() {
        let pool = Arc::new(Mutex::new(MemoryPool::<u8>::new(16)));
        let managed = {
            let mut guard = pool.lock().unwrap();
            let raw = guard.acquire(16);
            PooledBuffer::new_managed(raw, pool.clone())
        };

        let hold_lock = pool.lock().unwrap();
        let (tx, rx) = channel();
        let handle = thread::spawn(move || {
            drop(managed);
            tx.send(()).unwrap();
        });

        thread::sleep(Duration::from_millis(25));
        assert!(rx.try_recv().is_err());
        drop(hold_lock);

        rx.recv_timeout(Duration::from_secs(1)).unwrap();
        handle.join().unwrap();

        let stats = pool.lock().unwrap();
        assert_eq!(stats.in_use_count(), 0);
        assert!(stats.available_count() >= 1);
    }
}
