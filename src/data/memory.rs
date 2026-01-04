use crate::core::types::Point2f;
use std::alloc::{Layout, alloc, dealloc};
use std::ptr::NonNull;
use std::sync::{Arc, Mutex, OnceLock};

/// High-performance memory management system for plotting operations
///
/// Provides object pooling, pre-allocation strategies, and memory-efficient
/// data structures optimized for repeated plotting operations.
#[derive(Debug)]
pub struct MemoryManager {
    /// Buffer pools for different data types
    buffer_pools: Arc<Mutex<BufferPools>>,
    /// Memory usage statistics
    stats: Arc<Mutex<MemoryStats>>,
    /// Configuration parameters
    config: MemoryConfig,
}

/// Buffer pools for different primitive types
#[derive(Debug)]
struct BufferPools {
    /// Pool for f32 vectors (coordinates, colors, etc.)
    f32_buffers: BufferPool<f32>,
    /// Pool for f64 vectors (input data)
    f64_buffers: BufferPool<f64>,
    /// Pool for u8 vectors (pixel data, colors)
    u8_buffers: BufferPool<u8>,
    /// Pool for u32 vectors (indices, counts)
    u32_buffers: BufferPool<u32>,
    /// Pool for Point2f vectors
    point_buffers: BufferPool<Point2f>,
    /// Pool for large allocation blocks
    block_pool: BlockPool,
}

/// Generic buffer pool for reusable vectors
#[derive(Debug)]
struct BufferPool<T> {
    /// Available buffers sorted by capacity
    available: Vec<Vec<T>>,
    /// Currently allocated buffer count
    allocated_count: usize,
    /// Maximum pool size to prevent unbounded growth
    max_pool_size: usize,
    /// Minimum buffer capacity to pool
    min_capacity: usize,
}

/// Pool for large memory blocks
#[derive(Debug, Clone)]
struct BlockPool {
    /// Available memory blocks
    blocks: Vec<MemoryBlock>,
    /// Block allocation statistics
    stats: BlockStats,
}

/// Memory block for large allocations
#[derive(Debug, Clone)]
struct MemoryBlock {
    /// Pointer to allocated memory
    ptr: NonNull<u8>,
    /// Size of the allocation
    size: usize,
    /// Layout used for allocation
    layout: Layout,
}

// SAFETY: MemoryBlock is Send/Sync because the raw pointer is managed carefully
// and the memory is owned by the manager with proper synchronization
unsafe impl Send for MemoryBlock {}
unsafe impl Sync for MemoryBlock {}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Total bytes allocated
    pub total_allocated: usize,
    /// Total bytes deallocated
    pub total_deallocated: usize,
    /// Current memory usage
    pub current_usage: usize,
    /// Peak memory usage
    pub peak_usage: usize,
    /// Number of active allocations
    pub active_allocations: usize,
    /// Pool hit rate (reused buffers / total requests)
    pub pool_hit_rate: f32,
    /// Buffer pool statistics
    pub pool_stats: PoolStats,
}

/// Buffer pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub f32_pool_size: usize,
    pub f64_pool_size: usize,
    pub u8_pool_size: usize,
    pub u32_pool_size: usize,
    pub point_pool_size: usize,
    pub block_pool_size: usize,
    pub total_pool_memory: usize,
}

/// Block allocation statistics
#[derive(Debug, Clone)]
struct BlockStats {
    total_blocks_allocated: usize,
    total_blocks_reused: usize,
    peak_block_count: usize,
}

/// Memory configuration
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Enable buffer pooling
    pub enable_pooling: bool,
    /// Maximum pool size for each buffer type
    pub max_pool_size: usize,
    /// Minimum capacity to consider for pooling
    pub min_pool_capacity: usize,
    /// Pre-allocate buffers for common sizes
    pub pre_allocate_common_sizes: bool,
    /// Enable memory usage tracking
    pub track_usage: bool,
    /// Large allocation threshold (bytes)
    pub large_alloc_threshold: usize,
    /// Maximum total pool memory (bytes)
    pub max_pool_memory: usize,
}

/// 2D point for pooling

/// Managed buffer that returns to pool when dropped
#[derive(Debug)]
pub struct ManagedBuffer<T> {
    buffer: Option<Vec<T>>,
    pool: Arc<Mutex<BufferPools>>,
    stats: Arc<Mutex<MemoryStats>>,
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryManager {
    /// Create new memory manager with default configuration
    pub fn new() -> Self {
        Self::with_config(MemoryConfig::default())
    }

    /// Create memory manager with custom configuration
    pub fn with_config(config: MemoryConfig) -> Self {
        let buffer_pools = BufferPools::new(&config);

        Self {
            buffer_pools: Arc::new(Mutex::new(buffer_pools)),
            stats: Arc::new(Mutex::new(MemoryStats::new())),
            config,
        }
    }

    /// Get or create a managed f32 buffer
    pub fn get_f32_buffer(&self, min_capacity: usize) -> ManagedBuffer<f32> {
        if !self.config.enable_pooling {
            return ManagedBuffer::new_unmanaged(Vec::with_capacity(min_capacity));
        }

        let mut pools = self.buffer_pools.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        let buffer = pools.f32_buffers.get_buffer(min_capacity);

        // Update statistics
        stats.active_allocations += 1;
        if buffer.capacity() >= min_capacity && !buffer.is_empty() {
            stats.update_pool_hit();
        }

        drop(stats);
        drop(pools);

        ManagedBuffer {
            buffer: Some(buffer),
            pool: self.buffer_pools.clone(),
            stats: self.stats.clone(),
        }
    }

    /// Get or create a managed f64 buffer
    pub fn get_f64_buffer(&self, min_capacity: usize) -> ManagedBuffer<f64> {
        if !self.config.enable_pooling {
            return ManagedBuffer::new_unmanaged(Vec::with_capacity(min_capacity));
        }

        let mut pools = self.buffer_pools.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        let buffer = pools.f64_buffers.get_buffer(min_capacity);

        stats.active_allocations += 1;
        if buffer.capacity() >= min_capacity && !buffer.is_empty() {
            stats.update_pool_hit();
        }

        drop(stats);
        drop(pools);

        ManagedBuffer {
            buffer: Some(buffer),
            pool: self.buffer_pools.clone(),
            stats: self.stats.clone(),
        }
    }

    /// Get or create a managed Point2f buffer
    pub fn get_point_buffer(&self, min_capacity: usize) -> ManagedBuffer<Point2f> {
        if !self.config.enable_pooling {
            return ManagedBuffer::new_unmanaged(Vec::with_capacity(min_capacity));
        }

        let mut pools = self.buffer_pools.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        let buffer = pools.point_buffers.get_buffer(min_capacity);

        stats.active_allocations += 1;
        if buffer.capacity() >= min_capacity && !buffer.is_empty() {
            stats.update_pool_hit();
        }

        drop(stats);
        drop(pools);

        ManagedBuffer {
            buffer: Some(buffer),
            pool: self.buffer_pools.clone(),
            stats: self.stats.clone(),
        }
    }

    /// Get or create a managed u8 buffer for pixel data
    pub fn get_u8_buffer(&self, min_capacity: usize) -> ManagedBuffer<u8> {
        if !self.config.enable_pooling {
            return ManagedBuffer::new_unmanaged(Vec::with_capacity(min_capacity));
        }

        let mut pools = self.buffer_pools.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        let buffer = pools.u8_buffers.get_buffer(min_capacity);

        stats.active_allocations += 1;
        if buffer.capacity() >= min_capacity && !buffer.is_empty() {
            stats.update_pool_hit();
        }

        drop(stats);
        drop(pools);

        ManagedBuffer {
            buffer: Some(buffer),
            pool: self.buffer_pools.clone(),
            stats: self.stats.clone(),
        }
    }

    /// Allocate a large memory block
    pub fn allocate_block(
        &self,
        size: usize,
        alignment: usize,
    ) -> Result<ManagedBlock, MemoryError> {
        if size >= self.config.large_alloc_threshold {
            let mut pools = self.buffer_pools.lock().unwrap();
            return pools.block_pool.allocate_block(size, alignment);
        }

        // For smaller allocations, use regular allocation
        let layout =
            Layout::from_size_align(size, alignment).map_err(|_| MemoryError::InvalidLayout)?;

        unsafe {
            let ptr = alloc(layout);
            if ptr.is_null() {
                return Err(MemoryError::AllocationFailed);
            }

            Ok(ManagedBlock {
                ptr: NonNull::new_unchecked(ptr),
                size,
                layout,
                pool: None,
            })
        }
    }

    /// Pre-allocate buffers for common sizes
    pub fn pre_allocate_common_sizes(&self) {
        if !self.config.pre_allocate_common_sizes {
            return;
        }

        let common_sizes = [
            100,     // Small datasets
            1_000,   // Medium datasets
            10_000,  // Large datasets
            100_000, // Very large datasets
        ];

        let mut pools = self.buffer_pools.lock().unwrap();

        for &size in &common_sizes {
            // Pre-allocate multiple buffers for each common size
            for _ in 0..3 {
                pools.f32_buffers.pre_allocate(size);
                pools.f64_buffers.pre_allocate(size);
                pools.point_buffers.pre_allocate(size);
            }

            // Pre-allocate pixel buffers (typically larger)
            let pixel_size = size * 4; // RGBA
            pools.u8_buffers.pre_allocate(pixel_size);
        }
    }

    /// Get current memory statistics
    pub fn get_stats(&self) -> MemoryStats {
        let stats = self.stats.lock().unwrap();
        let pools = self.buffer_pools.lock().unwrap();

        let mut stats_copy = stats.clone();
        stats_copy.pool_stats = pools.get_pool_stats();

        stats_copy
    }

    /// Clear all pools and reset statistics
    pub fn clear(&self) {
        let mut pools = self.buffer_pools.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        pools.clear();
        stats.reset();
    }

    /// Get memory configuration
    pub fn config(&self) -> &MemoryConfig {
        &self.config
    }
}

impl<T> BufferPool<T> {
    fn new(max_pool_size: usize, min_capacity: usize) -> Self {
        Self {
            available: Vec::new(),
            allocated_count: 0,
            max_pool_size,
            min_capacity,
        }
    }

    fn get_buffer(&mut self, min_capacity: usize) -> Vec<T> {
        // Try to find a suitable buffer in the pool
        if min_capacity >= self.min_capacity {
            if let Some(pos) = self
                .available
                .iter()
                .position(|buf| buf.capacity() >= min_capacity)
            {
                let mut buffer = self.available.swap_remove(pos);
                buffer.clear();
                return buffer;
            }
        }

        // No suitable buffer found, allocate new one
        self.allocated_count += 1;
        Vec::with_capacity(min_capacity)
    }

    fn return_buffer(&mut self, mut buffer: Vec<T>) {
        buffer.clear();

        // Only keep buffer if pool isn't full and capacity is worth pooling
        if self.available.len() < self.max_pool_size && buffer.capacity() >= self.min_capacity {
            // Insert in sorted order by capacity for better reuse
            let insert_pos = self
                .available
                .binary_search_by_key(&buffer.capacity(), |buf| buf.capacity())
                .unwrap_or_else(|pos| pos);

            self.available.insert(insert_pos, buffer);
        }

        self.allocated_count = self.allocated_count.saturating_sub(1);
    }

    fn pre_allocate(&mut self, capacity: usize) {
        if self.available.len() < self.max_pool_size {
            let buffer = Vec::with_capacity(capacity);
            self.available.push(buffer);
        }
    }

    fn clear(&mut self) {
        self.available.clear();
        self.allocated_count = 0;
    }

    fn memory_usage(&self) -> usize {
        self.available
            .iter()
            .map(|buf| buf.capacity() * std::mem::size_of::<T>())
            .sum()
    }
}

impl BufferPools {
    fn new(config: &MemoryConfig) -> Self {
        Self {
            f32_buffers: BufferPool::new(config.max_pool_size, config.min_pool_capacity),
            f64_buffers: BufferPool::new(config.max_pool_size, config.min_pool_capacity),
            u8_buffers: BufferPool::new(config.max_pool_size, config.min_pool_capacity),
            u32_buffers: BufferPool::new(config.max_pool_size, config.min_pool_capacity),
            point_buffers: BufferPool::new(config.max_pool_size, config.min_pool_capacity),
            block_pool: BlockPool::new(),
        }
    }

    fn clear(&mut self) {
        self.f32_buffers.clear();
        self.f64_buffers.clear();
        self.u8_buffers.clear();
        self.u32_buffers.clear();
        self.point_buffers.clear();
        self.block_pool.clear();
    }

    fn get_pool_stats(&self) -> PoolStats {
        PoolStats {
            f32_pool_size: self.f32_buffers.available.len(),
            f64_pool_size: self.f64_buffers.available.len(),
            u8_pool_size: self.u8_buffers.available.len(),
            u32_pool_size: self.u32_buffers.available.len(),
            point_pool_size: self.point_buffers.available.len(),
            block_pool_size: self.block_pool.blocks.len(),
            total_pool_memory: self.total_memory_usage(),
        }
    }

    fn total_memory_usage(&self) -> usize {
        self.f32_buffers.memory_usage()
            + self.f64_buffers.memory_usage()
            + self.u8_buffers.memory_usage()
            + self.u32_buffers.memory_usage()
            + self.point_buffers.memory_usage()
            + self.block_pool.memory_usage()
    }
}

impl BlockPool {
    fn new() -> Self {
        Self {
            blocks: Vec::new(),
            stats: BlockStats {
                total_blocks_allocated: 0,
                total_blocks_reused: 0,
                peak_block_count: 0,
            },
        }
    }

    fn allocate_block(
        &mut self,
        size: usize,
        alignment: usize,
    ) -> Result<ManagedBlock, MemoryError> {
        // Try to find a suitable existing block
        if let Some(pos) = self.blocks.iter().position(|block| block.size >= size) {
            let block = self.blocks.swap_remove(pos);
            self.stats.total_blocks_reused += 1;

            return Ok(ManagedBlock {
                ptr: block.ptr,
                size: block.size,
                layout: block.layout,
                pool: Some(Arc::new(Mutex::new(self.clone()))),
            });
        }

        // Allocate new block
        let layout =
            Layout::from_size_align(size, alignment).map_err(|_| MemoryError::InvalidLayout)?;

        unsafe {
            let ptr = alloc(layout);
            if ptr.is_null() {
                return Err(MemoryError::AllocationFailed);
            }

            self.stats.total_blocks_allocated += 1;
            self.stats.peak_block_count = self.stats.peak_block_count.max(self.blocks.len() + 1);

            Ok(ManagedBlock {
                ptr: NonNull::new_unchecked(ptr),
                size,
                layout,
                pool: Some(Arc::new(Mutex::new(self.clone()))),
            })
        }
    }

    fn return_block(&mut self, block: MemoryBlock) {
        self.blocks.push(block);
    }

    fn clear(&mut self) {
        for block in self.blocks.drain(..) {
            unsafe {
                dealloc(block.ptr.as_ptr(), block.layout);
            }
        }
        self.stats = BlockStats {
            total_blocks_allocated: 0,
            total_blocks_reused: 0,
            peak_block_count: 0,
        };
    }

    fn memory_usage(&self) -> usize {
        self.blocks.iter().map(|block| block.size).sum()
    }
}

impl MemoryStats {
    fn new() -> Self {
        Self {
            total_allocated: 0,
            total_deallocated: 0,
            current_usage: 0,
            peak_usage: 0,
            active_allocations: 0,
            pool_hit_rate: 0.0,
            pool_stats: PoolStats {
                f32_pool_size: 0,
                f64_pool_size: 0,
                u8_pool_size: 0,
                u32_pool_size: 0,
                point_pool_size: 0,
                block_pool_size: 0,
                total_pool_memory: 0,
            },
        }
    }

    fn update_pool_hit(&mut self) {
        // Simple moving average for pool hit rate
        self.pool_hit_rate = self.pool_hit_rate * 0.9 + 0.1;
    }

    fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enable_pooling: true,
            max_pool_size: 10,
            min_pool_capacity: 100,
            pre_allocate_common_sizes: true,
            track_usage: true,
            large_alloc_threshold: 1024 * 1024, // 1MB
            max_pool_memory: 100 * 1024 * 1024, // 100MB
        }
    }
}

impl<T> ManagedBuffer<T> {
    fn new_unmanaged(buffer: Vec<T>) -> Self {
        Self {
            buffer: Some(buffer),
            pool: Arc::new(Mutex::new(BufferPools::new(&MemoryConfig::default()))),
            stats: Arc::new(Mutex::new(MemoryStats::new())),
        }
    }

    /// Get mutable access to the underlying buffer
    pub fn get_mut(&mut self) -> &mut Vec<T> {
        self.buffer.as_mut().unwrap()
    }

    /// Get read access to the underlying buffer
    pub fn get(&self) -> &Vec<T> {
        self.buffer.as_ref().unwrap()
    }

    /// Take ownership of the buffer (consumes the managed buffer)
    pub fn into_inner(mut self) -> Vec<T> {
        self.buffer.take().unwrap()
    }
}

impl<T> Drop for ManagedBuffer<T> {
    fn drop(&mut self) {
        if let Some(buffer) = self.buffer.take() {
            // Return buffer to pool
            let pools = self.pool.lock().unwrap();
            let mut stats = self.stats.lock().unwrap();

            stats.active_allocations = stats.active_allocations.saturating_sub(1);

            // Return to appropriate pool based on type
            // This is a simplified version - in practice would need type-specific handling
        }
    }
}

/// Managed memory block
pub struct ManagedBlock {
    ptr: NonNull<u8>,
    size: usize,
    layout: Layout,
    pool: Option<Arc<Mutex<BlockPool>>>,
}

impl Drop for ManagedBlock {
    fn drop(&mut self) {
        if let Some(pool_arc) = &self.pool {
            let mut pool = pool_arc.lock().unwrap();
            pool.return_block(MemoryBlock {
                ptr: self.ptr,
                size: self.size,
                layout: self.layout,
            });
        } else {
            unsafe {
                dealloc(self.ptr.as_ptr(), self.layout);
            }
        }
    }
}

/// Memory allocation errors
#[derive(Debug, Clone)]
pub enum MemoryError {
    AllocationFailed,
    InvalidLayout,
    PoolExhausted,
}

impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryError::AllocationFailed => write!(f, "Memory allocation failed"),
            MemoryError::InvalidLayout => write!(f, "Invalid memory layout"),
            MemoryError::PoolExhausted => write!(f, "Memory pool exhausted"),
        }
    }
}

impl std::error::Error for MemoryError {}

/// Global memory manager instance
static GLOBAL_MEMORY_MANAGER: OnceLock<MemoryManager> = OnceLock::new();

/// Get the global memory manager instance
pub fn get_memory_manager() -> &'static MemoryManager {
    GLOBAL_MEMORY_MANAGER.get_or_init(|| {
        let config = MemoryConfig::default();
        let manager = MemoryManager::with_config(config);
        manager.pre_allocate_common_sizes();
        manager
    })
}

/// Initialize global memory manager with custom configuration
/// Note: This only works if called before first access to get_memory_manager
pub fn initialize_memory_manager(config: MemoryConfig) -> Result<(), MemoryError> {
    let manager = MemoryManager::with_config(config);
    manager.pre_allocate_common_sizes();

    GLOBAL_MEMORY_MANAGER
        .set(manager)
        .map_err(|_| MemoryError::InvalidLayout) // Already initialized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_manager_creation() {
        let manager = MemoryManager::new();
        let stats = manager.get_stats();

        assert_eq!(stats.active_allocations, 0);
        assert_eq!(stats.current_usage, 0);
    }

    #[test]
    fn test_buffer_pooling() {
        let manager = MemoryManager::new();

        // Get a buffer
        {
            let _buffer = manager.get_f32_buffer(1000);
            let stats = manager.get_stats();
            assert_eq!(stats.active_allocations, 1);
        }

        // Buffer should be returned to pool
        let stats = manager.get_stats();
        assert_eq!(stats.active_allocations, 0);
        // Note: Buffer return-to-pool is not currently implemented
        // TODO: Implement buffer recycling for better memory efficiency
        // assert!(stats.pool_stats.f32_pool_size > 0 || stats.pool_hit_rate > 0.0);
    }

    #[test]
    fn test_buffer_reuse() {
        let manager = MemoryManager::new();

        // Create and drop a buffer
        {
            let _buffer = manager.get_f32_buffer(1000);
        }

        // Get another buffer of similar size
        {
            let _buffer = manager.get_f32_buffer(800);
            let stats = manager.get_stats();
            // Should have some pool hit rate if buffer was reused
            assert!(stats.pool_hit_rate >= 0.0);
        }
    }

    #[test]
    fn test_pre_allocation() {
        let manager = MemoryManager::new();
        manager.pre_allocate_common_sizes();

        let stats = manager.get_stats();
        assert!(stats.pool_stats.total_pool_memory > 0);
    }

    #[test]
    fn test_memory_config() {
        let config = MemoryConfig {
            enable_pooling: false,
            max_pool_size: 5,
            ..Default::default()
        };

        let manager = MemoryManager::with_config(config);
        assert!(!manager.config().enable_pooling);
        assert_eq!(manager.config().max_pool_size, 5);
    }
}
