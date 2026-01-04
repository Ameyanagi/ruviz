//! GPU buffer management with automatic pooling and memory optimization

use crate::core::error::PlottingError;
use crate::data::platform::PerformanceHints;
use crate::render::gpu::{BufferStats, GpuCapabilities, GpuDevice};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// GPU buffer usage patterns for optimization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferUsage {
    /// Static data that rarely changes (vertex data)
    Static,
    /// Data that changes every frame (uniforms)
    Dynamic,
    /// Data used for compute operations
    Compute,
    /// Data transferred from CPU to GPU
    Upload,
    /// Data transferred from GPU to CPU
    Download,
    /// Staging buffer for transfers
    Staging,
}

impl BufferUsage {
    /// Convert to wgpu buffer usage flags
    pub fn to_wgpu_usage(self) -> wgpu::BufferUsages {
        match self {
            BufferUsage::Static => wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::INDEX,
            BufferUsage::Dynamic => wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            BufferUsage::Compute => {
                wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC
            }
            BufferUsage::Upload => wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::MAP_WRITE,
            BufferUsage::Download => wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            BufferUsage::Staging => wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        }
    }
}

/// Managed GPU buffer with automatic lifetime management
#[derive(Clone)]
pub struct GpuBuffer {
    buffer: Arc<wgpu::Buffer>,
    size: u64,
    usage: BufferUsage,
    label: String,
    mapped: bool,
    pool_id: Option<usize>,
}

impl GpuBuffer {
    /// Create new GPU buffer
    pub fn new(
        device: &GpuDevice,
        size: u64,
        usage: BufferUsage,
        label: &str,
    ) -> Result<Self, PlottingError> {
        if size > device.limits().max_buffer_size {
            return Err(PlottingError::GpuMemoryError {
                requested: size as usize,
                available: Some(device.limits().max_buffer_size as usize),
            });
        }

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: usage.to_wgpu_usage(),
            mapped_at_creation: false,
        });

        Ok(Self {
            buffer: Arc::new(buffer),
            size,
            usage,
            label: label.to_string(),
            mapped: false,
            pool_id: None,
        })
    }

    /// Create buffer with initial data
    pub fn with_data(
        device: &GpuDevice,
        data: &[u8],
        usage: BufferUsage,
        label: &str,
    ) -> Result<Self, PlottingError> {
        let size = data.len() as u64;

        if size > device.limits().max_buffer_size {
            return Err(PlottingError::GpuMemoryError {
                requested: size as usize,
                available: Some(device.limits().max_buffer_size as usize),
            });
        }

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: data,
            usage: usage.to_wgpu_usage(),
        });

        Ok(Self {
            buffer: Arc::new(buffer),
            size,
            usage,
            label: label.to_string(),
            mapped: false,
            pool_id: None,
        })
    }

    /// Get wgpu buffer reference
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Get buffer size
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Get buffer usage
    pub fn usage(&self) -> BufferUsage {
        self.usage
    }

    /// Get buffer label
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Check if buffer is mapped
    pub fn is_mapped(&self) -> bool {
        self.mapped
    }

    /// Map buffer for reading
    pub async fn map_read(&mut self) -> Result<&[u8], PlottingError> {
        if self.mapped {
            return Err(PlottingError::BufferError(
                "Buffer already mapped".to_string(),
            ));
        }

        let slice = self.buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |result| {
            if let Err(e) = result {
                log::error!("Failed to map buffer for reading: {}", e);
            }
        });

        // TODO: Handle async mapping properly
        self.mapped = true;

        // This is a placeholder - proper async handling would be needed
        Err(PlottingError::BufferError(
            "Async mapping not fully implemented".to_string(),
        ))
    }

    /// Unmap buffer
    pub fn unmap(&mut self) {
        if self.mapped {
            self.buffer.unmap();
            self.mapped = false;
        }
    }

    /// Write data to buffer
    pub fn write(&self, device: &GpuDevice, offset: u64, data: &[u8]) -> Result<(), PlottingError> {
        if offset + data.len() as u64 > self.size {
            return Err(PlottingError::BufferError(format!(
                "Write would exceed buffer size: {}+{} > {}",
                offset,
                data.len(),
                self.size
            )));
        }

        device.write_buffer(&self.buffer, offset, data);
        Ok(())
    }
}

impl Drop for GpuBuffer {
    fn drop(&mut self) {
        self.unmap();
    }
}

/// Buffer pool for efficient memory reuse
struct BufferPool {
    usage: BufferUsage,
    available: VecDeque<GpuBuffer>,
    in_use: HashMap<usize, GpuBuffer>,
    next_id: usize,
    total_allocated: u64,
    reuse_count: usize,
}

impl BufferPool {
    fn new(usage: BufferUsage) -> Self {
        Self {
            usage,
            available: VecDeque::new(),
            in_use: HashMap::new(),
            next_id: 0,
            total_allocated: 0,
            reuse_count: 0,
        }
    }

    /// Allocate buffer from pool or create new one
    fn allocate(
        &mut self,
        device: &GpuDevice,
        size: u64,
        label: &str,
    ) -> Result<(usize, GpuBuffer), PlottingError> {
        // Try to reuse existing buffer of appropriate size
        if let Some(mut buffer) = self.find_suitable_buffer(size) {
            buffer.label = format!("{} (reused)", label);
            let id = self.next_id;
            self.next_id += 1;
            self.in_use.insert(id, buffer);
            self.reuse_count += 1;
            return Ok((id, self.in_use.get(&id).unwrap().clone()));
        }

        // Create new buffer
        let mut buffer = GpuBuffer::new(device, size, self.usage, label)?;
        buffer.pool_id = Some(self.next_id);

        let id = self.next_id;
        self.next_id += 1;
        self.total_allocated += size;

        self.in_use.insert(id, buffer);
        Ok((id, self.in_use.get(&id).unwrap().clone()))
    }

    /// Return buffer to pool
    fn deallocate(&mut self, id: usize) -> bool {
        if let Some(buffer) = self.in_use.remove(&id) {
            // Only keep buffer if it's reasonably sized and pool isn't too full
            if buffer.size <= 64 * 1024 * 1024 && self.available.len() < 16 {
                self.available.push_back(buffer);
            }
            true
        } else {
            false
        }
    }

    /// Find suitable buffer for reuse
    fn find_suitable_buffer(&mut self, required_size: u64) -> Option<GpuBuffer> {
        // Look for buffer that's at least the required size but not too much larger
        let max_size = required_size * 2; // Don't waste more than 2x memory

        for i in 0..self.available.len() {
            if self.available[i].size >= required_size && self.available[i].size <= max_size {
                return Some(self.available.remove(i).unwrap());
            }
        }

        None
    }

    /// Get pool statistics
    fn stats(&self) -> BufferPoolStats {
        BufferPoolStats {
            usage: self.usage,
            available_buffers: self.available.len(),
            in_use_buffers: self.in_use.len(),
            total_allocated: self.total_allocated,
            reuse_count: self.reuse_count,
        }
    }
}

/// Buffer pool statistics
#[derive(Debug, Clone)]
pub struct BufferPoolStats {
    pub usage: BufferUsage,
    pub available_buffers: usize,
    pub in_use_buffers: usize,
    pub total_allocated: u64,
    pub reuse_count: usize,
}

/// Buffer manager with automatic pooling and optimization
pub struct BufferManager {
    pools: HashMap<BufferUsage, BufferPool>,
    total_memory: u64,
    memory_limit: u64,
    allocation_count: usize,
    deallocation_count: usize,
    capabilities: GpuCapabilities,
    performance_hints: PerformanceHints,
}

impl BufferManager {
    /// Create new buffer manager
    pub fn new(
        device: &GpuDevice,
        capabilities: &GpuCapabilities,
        hints: &PerformanceHints,
    ) -> Result<Self, PlottingError> {
        // Calculate memory limit (80% of max buffer size or platform hint)
        let memory_limit = capabilities.max_buffer_size.min(
            hints.optimal_chunk_size as u64 * 1024, // Conservative limit
        );

        let mut pools = HashMap::new();

        // Initialize pools for each usage type
        for usage in [
            BufferUsage::Static,
            BufferUsage::Dynamic,
            BufferUsage::Compute,
            BufferUsage::Upload,
            BufferUsage::Download,
            BufferUsage::Staging,
        ] {
            pools.insert(usage, BufferPool::new(usage));
        }

        Ok(Self {
            pools,
            total_memory: 0,
            memory_limit,
            allocation_count: 0,
            deallocation_count: 0,
            capabilities: capabilities.clone(),
            performance_hints: hints.clone(),
        })
    }

    /// Allocate buffer with automatic pooling
    pub fn allocate(
        &mut self,
        device: &GpuDevice,
        size: u64,
        usage: BufferUsage,
        label: &str,
    ) -> Result<BufferHandle, PlottingError> {
        // Check memory limits
        if self.total_memory + size > self.memory_limit {
            return Err(PlottingError::GpuMemoryError {
                requested: size as usize,
                available: Some((self.memory_limit - self.total_memory) as usize),
            });
        }

        let pool = self.pools.get_mut(&usage).ok_or_else(|| {
            PlottingError::BufferError(format!("No pool for usage type: {:?}", usage))
        })?;

        let (id, buffer) = pool.allocate(device, size, label)?;
        self.total_memory += size;
        self.allocation_count += 1;

        Ok(BufferHandle {
            id,
            usage,
            size,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Deallocate buffer and return to pool
    pub fn deallocate(&mut self, handle: BufferHandle) -> Result<(), PlottingError> {
        let pool = self.pools.get_mut(&handle.usage).ok_or_else(|| {
            PlottingError::BufferError(format!("No pool for usage type: {:?}", handle.usage))
        })?;

        if pool.deallocate(handle.id) {
            self.total_memory = self.total_memory.saturating_sub(handle.size);
            self.deallocation_count += 1;
            Ok(())
        } else {
            Err(PlottingError::BufferError(
                "Buffer not found in pool".to_string(),
            ))
        }
    }

    /// Get buffer from handle
    pub fn get_buffer(&self, handle: &BufferHandle) -> Option<&GpuBuffer> {
        self.pools.get(&handle.usage)?.in_use.get(&handle.id)
    }

    /// Create buffer with data (convenience method)
    pub fn create_with_data(
        &mut self,
        device: &GpuDevice,
        data: &[u8],
        usage: BufferUsage,
        label: &str,
    ) -> Result<(BufferHandle, GpuBuffer), PlottingError> {
        let handle = self.allocate(device, data.len() as u64, usage, label)?;
        let buffer = self.get_buffer(&handle).unwrap().clone();
        buffer.write(device, 0, data)?;
        Ok((handle, buffer))
    }

    /// Garbage collect unused buffers
    pub fn garbage_collect(&mut self) {
        for pool in self.pools.values_mut() {
            // Keep only recent buffers in available pool
            while pool.available.len() > 8 {
                if let Some(buffer) = pool.available.pop_front() {
                    self.total_memory = self.total_memory.saturating_sub(buffer.size);
                }
            }
        }
    }

    /// Get memory usage
    pub fn get_memory_usage(&self) -> u64 {
        self.total_memory
    }

    /// Get buffer statistics
    pub fn get_stats(&self) -> BufferStats {
        let mut total_buffers = 0;
        let mut active_buffers = 0;
        let mut reused_buffers = 0;

        for pool in self.pools.values() {
            let stats = pool.stats();
            total_buffers += stats.available_buffers + stats.in_use_buffers;
            active_buffers += stats.in_use_buffers;
            reused_buffers += stats.reuse_count;
        }

        BufferStats {
            total_buffers,
            total_memory: self.total_memory,
            active_buffers,
            reused_buffers,
        }
    }

    /// Get detailed pool statistics
    pub fn get_pool_stats(&self) -> Vec<BufferPoolStats> {
        self.pools.values().map(|pool| pool.stats()).collect()
    }

    /// Optimize buffer allocation based on usage patterns
    pub fn optimize(&mut self) {
        // Analyze usage patterns and adjust pool sizes
        for pool in self.pools.values_mut() {
            let stats = pool.stats();

            // If reuse rate is low, reduce available buffer count
            if stats.reuse_count < stats.available_buffers / 4 {
                while pool.available.len() > 4 {
                    if let Some(buffer) = pool.available.pop_front() {
                        self.total_memory = self.total_memory.saturating_sub(buffer.size);
                    }
                }
            }
        }
    }
}

/// Handle to a managed buffer
pub struct BufferHandle {
    id: usize,
    usage: BufferUsage,
    size: u64,
    _phantom: std::marker::PhantomData<GpuBuffer>,
}

impl BufferHandle {
    /// Get buffer ID
    pub fn id(&self) -> usize {
        self.id
    }

    /// Get buffer usage
    pub fn usage(&self) -> BufferUsage {
        self.usage
    }

    /// Get buffer size
    pub fn size(&self) -> u64 {
        self.size
    }
}

// Note: GpuBuffer doesn't implement Clone as wgpu::Buffer doesn't support cloning
// This is intentional to prevent accidental duplication of GPU resources
