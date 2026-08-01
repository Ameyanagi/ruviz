//! GPU buffer management with automatic pooling and memory optimization

use crate::core::error::PlottingError;
use crate::data::platform::PerformanceHints;
use crate::render::gpu::{BufferStats, GpuCapabilities, GpuDevice};
use std::collections::{HashMap, VecDeque};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
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
            BufferUsage::Static => {
                wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::INDEX
                    | wgpu::BufferUsages::COPY_DST
            }
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
    mapped: Arc<AtomicBool>,
    pool_id: Option<usize>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
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
            mapped: Arc::new(AtomicBool::new(false)),
            pool_id: None,
            device: Arc::clone(device.device()),
            queue: Arc::clone(device.queue()),
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
            mapped: Arc::new(AtomicBool::new(false)),
            pool_id: None,
            device: Arc::clone(device.device()),
            queue: Arc::clone(device.queue()),
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
        self.mapped.load(Ordering::Acquire)
    }

    /// Map buffer for reading
    pub async fn map_read(&mut self) -> Result<Vec<u8>, PlottingError> {
        if !self
            .usage
            .to_wgpu_usage()
            .contains(wgpu::BufferUsages::MAP_READ)
        {
            return Err(PlottingError::BufferError(
                "Buffer was not created with MAP_READ usage".to_string(),
            ));
        }
        if self.mapped.swap(true, Ordering::AcqRel) {
            return Err(PlottingError::BufferError(
                "Buffer already mapped".to_string(),
            ));
        }

        let slice = self.buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        let submission = self.queue.submit(std::iter::empty());
        if let Err(error) = self.device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        }) {
            self.mapped.store(false, Ordering::Release);
            self.buffer.unmap();
            return Err(PlottingError::BufferError(format!(
                "Failed to poll mapped buffer: {error}"
            )));
        }
        let Some(map_result) = receiver.receive().await else {
            self.mapped.store(false, Ordering::Release);
            self.buffer.unmap();
            return Err(PlottingError::BufferError(
                "Buffer mapping callback was dropped".to_string(),
            ));
        };
        if let Err(error) = map_result {
            self.mapped.store(false, Ordering::Release);
            self.buffer.unmap();
            return Err(PlottingError::BufferError(format!(
                "Failed to map buffer for reading: {error}"
            )));
        }

        let bytes = slice.get_mapped_range().to_vec();
        self.unmap();
        Ok(bytes)
    }

    /// Unmap buffer
    pub fn unmap(&mut self) {
        if self.mapped.swap(false, Ordering::AcqRel) {
            self.buffer.unmap();
        }
    }

    /// Write data to buffer
    pub fn write(&self, device: &GpuDevice, offset: u64, data: &[u8]) -> Result<(), PlottingError> {
        let end = offset.checked_add(data.len() as u64).ok_or_else(|| {
            PlottingError::BufferError("Buffer write offset overflowed".to_string())
        })?;
        if end > self.size {
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
        max_new_bytes: u64,
    ) -> Result<(usize, u64, u64), PlottingError> {
        // Try to reuse existing buffer of appropriate size
        if let Some(mut buffer) = self.find_suitable_buffer(size) {
            buffer.label = format!("{} (reused)", label);
            let id = self.next_id;
            self.next_id += 1;
            self.in_use.insert(id, buffer);
            self.reuse_count += 1;
            let actual_size = self.in_use.get(&id).map_or(size, |buffer| buffer.size);
            return Ok((id, actual_size, 0));
        }

        if size > max_new_bytes {
            return Err(PlottingError::GpuMemoryError {
                requested: size as usize,
                available: Some(max_new_bytes as usize),
            });
        }

        // Create new buffer
        let mut buffer = GpuBuffer::new(device, size, self.usage, label)?;
        buffer.pool_id = Some(self.next_id);

        let id = self.next_id;
        self.next_id += 1;
        self.total_allocated += size;

        self.in_use.insert(id, buffer);
        Ok((id, size, size))
    }

    /// Return buffer to pool
    fn deallocate(&mut self, id: usize) -> Option<u64> {
        if let Some(buffer) = self.in_use.remove(&id) {
            // Only keep buffer if it's reasonably sized and pool isn't too full
            if buffer.size <= 64 * 1024 * 1024 && self.available.len() < 16 {
                self.available.push_back(buffer);
                Some(0)
            } else {
                self.total_allocated = self.total_allocated.saturating_sub(buffer.size);
                Some(buffer.size)
            }
        } else {
            None
        }
    }

    /// Find suitable buffer for reuse
    fn find_suitable_buffer(&mut self, required_size: u64) -> Option<GpuBuffer> {
        // Look for buffer that's at least the required size but not too much larger
        let max_size = required_size.saturating_mul(2); // Don't waste more than 2x memory

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
        Self::with_memory_limit_fraction(device, capabilities, hints, 0.8)
    }

    /// Create a buffer manager with an explicit fraction of the adapter limit.
    pub fn with_memory_limit_fraction(
        _device: &GpuDevice,
        capabilities: &GpuCapabilities,
        hints: &PerformanceHints,
        memory_limit_fraction: f32,
    ) -> Result<Self, PlottingError> {
        if !memory_limit_fraction.is_finite() || !(0.0..=1.0).contains(&memory_limit_fraction) {
            return Err(PlottingError::InvalidInput(
                "GPU memory_limit_fraction must be finite and between 0 and 1".into(),
            ));
        }
        let memory_limit =
            (capabilities.max_buffer_size as f64 * memory_limit_fraction as f64) as u64;

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
        let pool = self.pools.get_mut(&usage).ok_or_else(|| {
            PlottingError::BufferError(format!("No pool for usage type: {:?}", usage))
        })?;

        let available = self.memory_limit.saturating_sub(self.total_memory);
        let (id, actual_size, newly_allocated) = pool.allocate(device, size, label, available)?;
        self.total_memory = self.total_memory.checked_add(newly_allocated).ok_or(
            PlottingError::GpuMemoryError {
                requested: newly_allocated as usize,
                available: Some(available as usize),
            },
        )?;
        self.allocation_count += 1;

        Ok(BufferHandle {
            id,
            usage,
            size: actual_size,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Deallocate buffer and return to pool
    pub fn deallocate(&mut self, handle: BufferHandle) -> Result<(), PlottingError> {
        let pool = self.pools.get_mut(&handle.usage).ok_or_else(|| {
            PlottingError::BufferError(format!("No pool for usage type: {:?}", handle.usage))
        })?;

        match pool.deallocate(handle.id) {
            Some(freed) => {
                self.total_memory = self.total_memory.saturating_sub(freed);
                self.deallocation_count += 1;
                Ok(())
            }
            None => Err(PlottingError::BufferError(
                "Buffer not found in pool".to_string(),
            )),
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
        if !usage.to_wgpu_usage().contains(wgpu::BufferUsages::COPY_DST) {
            return Err(PlottingError::BufferError(format!(
                "Buffer usage {usage:?} cannot be initialized with queue writes"
            )));
        }
        let handle = self.allocate(device, data.len() as u64, usage, label)?;
        let buffer = self.get_buffer(&handle).cloned().ok_or_else(|| {
            PlottingError::BufferError("Newly allocated buffer was not retained".to_string())
        })?;
        buffer.write(device, 0, data)?;
        Ok((handle, buffer))
    }

    /// Garbage collect unused buffers
    pub fn garbage_collect(&mut self) {
        for pool in self.pools.values_mut() {
            // Keep only recent buffers in available pool
            while pool.available.len() > 8 {
                if let Some(buffer) = pool.available.pop_front() {
                    pool.total_allocated = pool.total_allocated.saturating_sub(buffer.size);
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
                        pool.total_allocated = pool.total_allocated.saturating_sub(buffer.size);
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

// Handles are move-only; cloned `GpuBuffer` values share the same physical buffer.
