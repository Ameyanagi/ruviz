use std::collections::{HashSet, VecDeque};
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

/// A memory pool for efficient allocation and reuse of typed buffers.
///
/// The pool stores empty vectors with preserved capacity. This avoids unsafe
/// uninitialized `Vec<T>` construction while still enabling cheap capacity reuse.
pub struct MemoryPool<T> {
    /// Available raw buffers ready for reuse.
    available: VecDeque<Vec<MaybeUninit<T>>>,
    /// Pointers to buffers currently checked out (for diagnostics/statistics).
    in_use: HashSet<usize>,
    /// Default chunk size for new allocations.
    chunk_size: usize,
    /// Maximum number of buffers kept in the pool.
    max_pools: usize,
    /// Total capacity tracked across allocated raw buffers.
    total_capacity: usize,
}

enum BufferStorage<T> {
    Initialized(Vec<T>),
    Raw(Vec<MaybeUninit<T>>),
}

/// RAII wrapper for pooled memory that automatically returns buffer to pool on drop.
pub struct PooledBuffer<T> {
    storage: Option<BufferStorage<T>>,
    /// Logical length expected by callers.
    len: usize,
    /// Reference to the pool for returning managed buffers.
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
    /// Returned pooled buffers provide capacity for `len` elements. Logical length is set
    /// to `len`, but pooled storage is raw/uninitialized and intended for pointer-based writes.
    pub fn acquire(&mut self, len: usize) -> PooledBuffer<T> {
        let mut raw = if let Some(pos) = self
            .available
            .iter()
            .position(|buf| buf.capacity() >= len)
        {
            self.available.swap_remove_back(pos).unwrap()
        } else {
            let capacity = len.max(self.chunk_size);
            self.total_capacity += capacity;
            Vec::with_capacity(capacity)
        };

        raw.clear();
        self.in_use.insert(raw.as_ptr() as usize);

        PooledBuffer {
            storage: Some(BufferStorage::Raw(raw)),
            len,
            pool: None,
        }
    }

    /// Release a buffer back to the pool for reuse.
    pub fn release(&mut self, mut buffer: PooledBuffer<T>) {
        if let Some(raw) = buffer.take_raw_storage() {
            self.return_raw(raw);
        }
    }

    /// Get the number of available buffers in the pool.
    pub fn available_count(&self) -> usize {
        self.available.len()
    }

    /// Get the total tracked capacity across all buffers.
    pub fn total_capacity(&self) -> usize {
        self.total_capacity
    }

    /// Get the number of buffers currently in use.
    pub fn in_use_count(&self) -> usize {
        self.in_use.len()
    }

    /// Shrink the pool by removing unused buffers.
    pub fn shrink_unused(&mut self) {
        let target_size = self.available.len() / 2;
        while self.available.len() > target_size {
            if let Some(buffer) = self.available.pop_back() {
                self.total_capacity = self.total_capacity.saturating_sub(buffer.capacity());
            }
        }
    }

    fn return_raw(&mut self, mut raw: Vec<MaybeUninit<T>>) {
        let ptr_addr = raw.as_ptr() as usize;
        self.in_use.remove(&ptr_addr);
        raw.clear();

        if self.available.len() < self.max_pools {
            self.available.push_back(raw);
        } else {
            self.total_capacity = self.total_capacity.saturating_sub(raw.capacity());
        }
    }
}

impl<T> Default for MemoryPool<T> {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl<T> PooledBuffer<T> {
    /// Create a new unmanaged, initialized buffer.
    pub fn new_unmanaged(data: Box<[T]>) -> Self {
        let vec = data.into_vec();
        let len = vec.len();
        Self {
            storage: Some(BufferStorage::Initialized(vec)),
            len,
            pool: None,
        }
    }

    /// Create a managed buffer that will be returned to the pool on drop.
    pub fn new_managed(mut buffer: PooledBuffer<T>, pool: Arc<Mutex<MemoryPool<T>>>) -> Self {
        buffer.pool = Some(pool);
        buffer
    }

    /// Get the logical length of the buffer.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the logical length is zero.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the capacity of the underlying allocation.
    pub fn capacity(&self) -> usize {
        match self.storage.as_ref() {
            Some(BufferStorage::Initialized(vec)) => vec.capacity(),
            Some(BufferStorage::Raw(vec)) => vec.capacity(),
            None => 0,
        }
    }

    /// Get a raw pointer to the buffer data.
    pub fn as_ptr(&self) -> *const T {
        match self.storage.as_ref() {
            Some(BufferStorage::Initialized(vec)) => vec.as_ptr(),
            Some(BufferStorage::Raw(vec)) => vec.as_ptr() as *const T,
            None => std::ptr::null(),
        }
    }

    /// Get a mutable raw pointer to the buffer data.
    pub fn as_mut_ptr(&self) -> *mut T {
        match self.storage.as_ref() {
            Some(BufferStorage::Initialized(vec)) => vec.as_ptr() as *mut T,
            Some(BufferStorage::Raw(vec)) => vec.as_ptr() as *mut T,
            None => std::ptr::null_mut(),
        }
    }

    /// Resize logical length (must not exceed capacity).
    pub fn resize(&mut self, new_len: usize) {
        assert!(new_len <= self.capacity(), "Cannot resize beyond capacity");

        if let Some(BufferStorage::Initialized(vec)) = self.storage.as_mut() {
            if new_len < vec.len() {
                vec.truncate(new_len);
            } else {
                panic!("Cannot grow initialized unmanaged buffer without a fill value");
            }
        }

        self.len = new_len;
    }

    /// Convert to a slice.
    ///
    /// Panics for managed pooled buffers, because pooled storage may be uninitialized and
    /// must be accessed through explicit pointer operations by higher-level containers.
    pub fn as_slice(&self) -> &[T] {
        match self.storage.as_ref() {
            Some(BufferStorage::Initialized(vec)) => &vec[..self.len],
            Some(BufferStorage::Raw(_)) => {
                panic!("as_slice() is only valid for initialized unmanaged buffers")
            }
            None => &[],
        }
    }

    /// Convert to a mutable slice.
    ///
    /// Panics for managed pooled buffers, because pooled storage may be uninitialized and
    /// must be accessed through explicit pointer operations by higher-level containers.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        match self.storage.as_mut() {
            Some(BufferStorage::Initialized(vec)) => &mut vec[..self.len],
            Some(BufferStorage::Raw(_)) => {
                panic!("as_mut_slice() is only valid for initialized unmanaged buffers")
            }
            None => &mut [],
        }
    }

    fn take_raw_storage(&mut self) -> Option<Vec<MaybeUninit<T>>> {
        match self.storage.take() {
            Some(BufferStorage::Raw(raw)) => Some(raw),
            Some(initialized) => {
                self.storage = Some(initialized);
                None
            }
            None => None,
        }
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
        let pool = self.pool.clone();
        if let Some(pool) = pool {
            if let Some(raw) = self.take_raw_storage() {
                let mut guard = pool.lock().unwrap();
                guard.return_raw(raw);
            }
        }
    }
}

impl<T: Clone> Clone for PooledBuffer<T> {
    fn clone(&self) -> Self {
        match self.storage.as_ref() {
            Some(BufferStorage::Initialized(vec)) => {
                Self::new_unmanaged(vec[..self.len].to_vec().into_boxed_slice())
            }
            Some(BufferStorage::Raw(_)) => {
                panic!("cannot clone raw pooled buffer safely")
            }
            None => Self {
                storage: Some(BufferStorage::Initialized(Vec::new())),
                len: 0,
                pool: None,
            },
        }
    }
}

/// Thread-safe wrapper for cross-thread pool sharing.
#[derive(Clone)]
pub struct SharedMemoryPool<T> {
    inner: Arc<Mutex<MemoryPool<T>>>,
}

impl<T> SharedMemoryPool<T> {
    /// Create a new shared memory pool.
    pub fn new(initial_capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(MemoryPool::new(initial_capacity))),
        }
    }

    /// Acquire a buffer from the shared pool.
    pub fn acquire(&self, len: usize) -> PooledBuffer<T> {
        let buffer = {
            let mut pool = self.inner.lock().unwrap();
            pool.acquire(len)
        };
        PooledBuffer::new_managed(buffer, self.inner.clone())
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
        let buffer = PooledBuffer::new_unmanaged(data.into_boxed_slice());

        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer[0], 1.0);
        assert_eq!(buffer[4], 5.0);
        assert_eq!(buffer.as_slice(), &[1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn test_shared_pool_reuse_updates_in_use() {
        let shared_pool = SharedMemoryPool::<f64>::new(16);
        let ptr = {
            let buffer = shared_pool.acquire(8);
            assert_eq!(shared_pool.statistics().in_use_count, 1);
            buffer.as_ptr()
        };
        let after_drop = shared_pool.statistics();
        assert_eq!(after_drop.in_use_count, 0);
        assert!(after_drop.available_count >= 1);

        let reused = shared_pool.acquire(8);
        assert_eq!(reused.as_ptr(), ptr);
    }

    #[test]
    fn test_drop_blocks_on_lock_and_preserves_pool_return() {
        let shared_pool = SharedMemoryPool::<u64>::new(8);
        let buffer = shared_pool.acquire(8);
        let ptr = buffer.as_ptr();
        let pool_for_thread = shared_pool.clone();

        let holder = thread::spawn(move || {
            let _lock = pool_for_thread.inner.lock().unwrap();
            thread::sleep(Duration::from_millis(25));
        });

        drop(buffer);
        holder.join().unwrap();

        let reused = shared_pool.acquire(8);
        assert_eq!(reused.as_ptr(), ptr);
    }
}
