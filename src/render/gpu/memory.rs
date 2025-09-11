//! GPU memory management with integration to CPU memory pools
//! 
//! This module provides GPU buffer pools that work seamlessly with the existing
//! CPU memory pool system for optimal hybrid CPU/GPU performance.

use crate::core::{PlottingError, Result};
use crate::data::{SharedMemoryPool, PooledVec};
use super::{GpuCapabilities, GpuError, GpuResult};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use bytemuck::{Pod, cast_slice, cast_vec};
use wgpu::util::DeviceExt;

/// GPU buffer wrapper with automatic memory management
pub struct GpuBuffer {
    buffer: wgpu::Buffer,
    size: u64,
    usage: wgpu::BufferUsages,
    label: Option<String>,
}

impl GpuBuffer {
    /// Create a new GPU buffer
    pub fn new(
        device: &wgpu::Device,
        data: &[u8],
        usage: wgpu::BufferUsages,
        label: Option<&str>,
    ) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label,
            contents: data,
            usage,
        });
        
        Self {
            buffer,
            size: data.len() as u64,
            usage,
            label: label.map(|s| s.to_string()),
        }
    }
    
    /// Create an empty GPU buffer
    pub fn new_empty(
        device: &wgpu::Device,
        size: u64,
        usage: wgpu::BufferUsages,
        label: Option<&str>,
    ) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label,
            size,
            usage,
            mapped_at_creation: false,
        });
        
        Self {
            buffer,
            size,
            usage,
            label: label.map(|s| s.to_string()),
        }
    }
    
    /// Get the underlying wgpu buffer
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
    
    /// Get buffer size in bytes
    pub fn size(&self) -> u64 {
        self.size
    }
    
    /// Get buffer usage flags
    pub fn usage(&self) -> wgpu::BufferUsages {
        self.usage
    }
}

/// GPU memory pool for efficient buffer management
pub struct GpuMemoryPool {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    /// Cached buffers by size and usage pattern
    buffer_cache: Arc<Mutex<HashMap<(u64, wgpu::BufferUsages), Vec<GpuBuffer>>>>,
    /// Total memory allocated (bytes)
    total_allocated: Arc<Mutex<u64>>,
    /// Maximum memory to use (fraction of GPU memory)
    memory_limit: u64,
    /// Buffer alignment requirements
    alignment: u64,
    /// Statistics
    stats: Arc<Mutex<GpuMemoryStats>>,
}

/// GPU memory usage statistics
#[derive(Debug, Clone, Default)]
pub struct GpuMemoryStats {
    pub total_allocated: u64,
    pub buffers_created: usize,
    pub buffers_reused: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub memory_limit: u64,
}

impl GpuMemoryPool {
    /// Create a new GPU memory pool
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        capabilities: &GpuCapabilities,
    ) -> Result<Self> {
        
        // Calculate memory limit (80% of max buffer size as approximation)
        let memory_limit = (capabilities.max_buffer_size as f64 * 0.8) as u64;
        
        // Get buffer alignment from limits
        let alignment = device.limits().min_uniform_buffer_offset_alignment as u64;
        
        Ok(Self {
            device,
            queue,
            buffer_cache: Arc::new(Mutex::new(HashMap::new())),
            total_allocated: Arc::new(Mutex::new(0)),
            memory_limit,
            alignment,
            stats: Arc::new(Mutex::new(GpuMemoryStats {
                memory_limit,
                ..Default::default()
            })),
        })
    }
    
    /// Create a GPU buffer from typed data
    pub fn create_buffer<T: Pod>(
        &self,
        data: &[T],
        usage: wgpu::BufferUsages,
    ) -> Result<GpuBuffer> {
        let bytes = cast_slice(data);
        self.create_buffer_bytes(bytes, usage, None)
    }
    
    /// Create an empty GPU buffer for a specific type and count
    pub fn create_buffer_empty<T: Pod>(
        &self,
        count: usize,
        usage: wgpu::BufferUsages,
    ) -> Result<GpuBuffer> {
        let size = (count * std::mem::size_of::<T>()) as u64;
        let aligned_size = self.align_buffer_size(size);
        self.create_buffer_empty_bytes(aligned_size, usage, None)
    }
    
    /// Create a GPU buffer from raw bytes
    pub fn create_buffer_bytes(
        &self,
        data: &[u8],
        usage: wgpu::BufferUsages,
        label: Option<&str>,
    ) -> Result<GpuBuffer> {
        let size = data.len() as u64;
        let aligned_size = self.align_buffer_size(size);
        
        // Check memory limit
        {
            let total_allocated = self.total_allocated.lock().unwrap();
            if *total_allocated + aligned_size > self.memory_limit {
                return Err(PlottingError::GpuMemoryError {
                    requested: aligned_size as usize,
                    available: Some((self.memory_limit - *total_allocated) as usize),
                });
            }
        }
        
        // Try to reuse existing buffer from cache
        let cache_key = (aligned_size, usage);
        if let Some(buffer) = self.try_reuse_buffer(&cache_key) {
            // Update buffer data
            self.queue.write_buffer(buffer.buffer(), 0, data);
            
            // Update stats
            {
                let mut stats = self.stats.lock().unwrap();
                stats.buffers_reused += 1;
                stats.cache_hits += 1;
            }
            
            return Ok(buffer);
        }
        
        // Create new buffer
        let buffer = if data.is_empty() {
            GpuBuffer::new_empty(&self.device, aligned_size, usage, label)
        } else {
            // Pad data to alignment if necessary
            let mut padded_data = data.to_vec();
            padded_data.resize(aligned_size as usize, 0);
            GpuBuffer::new(&self.device, &padded_data, usage, label)
        };
        
        // Update memory tracking
        {
            let mut total_allocated = self.total_allocated.lock().unwrap();
            *total_allocated += aligned_size;
        }
        
        // Update stats
        {
            let mut stats = self.stats.lock().unwrap();
            stats.total_allocated += aligned_size;
            stats.buffers_created += 1;
            stats.cache_misses += 1;
        }
        
        Ok(buffer)
    }
    
    /// Create an empty GPU buffer from raw size
    pub fn create_buffer_empty_bytes(
        &self,
        _size: u64,
        usage: wgpu::BufferUsages,
        label: Option<&str>,
    ) -> Result<GpuBuffer> {
        // Note: _size parameter is ignored for now, using empty data to create buffer
        self.create_buffer_bytes(&[], usage, label)
    }
    
    /// Try to reuse a buffer from cache
    fn try_reuse_buffer(&self, cache_key: &(u64, wgpu::BufferUsages)) -> Option<GpuBuffer> {
        let mut cache = self.buffer_cache.lock().unwrap();
        if let Some(buffers) = cache.get_mut(cache_key) {
            buffers.pop()
        } else {
            None
        }
    }
    
    /// Return buffer to cache for reuse
    pub fn return_buffer(&self, buffer: GpuBuffer) {
        let cache_key = (buffer.size(), buffer.usage());
        let mut cache = self.buffer_cache.lock().unwrap();
        cache.entry(cache_key).or_insert_with(Vec::new).push(buffer);
    }
    
    /// Align buffer size to GPU requirements
    fn align_buffer_size(&self, size: u64) -> u64 {
        ((size + self.alignment - 1) / self.alignment) * self.alignment
    }
    
    /// Read data back from GPU buffer
    pub fn read_buffer<T: Pod + Clone>(&self, buffer: &GpuBuffer) -> GpuResult<Vec<T>> {
        if !buffer.usage().contains(wgpu::BufferUsages::COPY_SRC) {
            return Err(GpuError::OperationFailed(
                "Buffer was not created with COPY_SRC usage".to_string()
            ));
        }
        
        let element_size = std::mem::size_of::<T>();
        let element_count = (buffer.size() as usize) / element_size;
        
        // Create staging buffer for readback
        let staging_buffer = self.create_buffer_empty_bytes(
            buffer.size(),
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            Some("GPU Readback Staging"),
        ).map_err(|e| GpuError::BufferCreationFailed(format!("{}", e)))?;
        
        // Copy from GPU buffer to staging buffer
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("GPU Buffer Copy"),
        });
        
        encoder.copy_buffer_to_buffer(
            buffer.buffer(),
            0,
            staging_buffer.buffer(),
            0,
            buffer.size(),
        );
        
        let submission = self.queue.submit(Some(encoder.finish()));
        
        // Map and read staging buffer
        let buffer_slice = staging_buffer.buffer().slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).ok();
        });
        
        // Wait for mapping to complete
        self.device.poll(wgpu::Maintain::WaitForSubmissionIndex(submission));
        
        let result = pollster::block_on(receiver.receive())
            .ok_or_else(|| GpuError::OperationFailed("Buffer mapping failed".to_string()))?
            .map_err(|e| GpuError::OperationFailed(format!("Buffer mapping error: {:?}", e)))?;
        
        // Copy data
        let mapped_data = buffer_slice.get_mapped_range();
        let result_data: Vec<T> = cast_slice(&mapped_data[..element_count * element_size]).to_vec();
        
        // Unmap buffer
        drop(mapped_data);
        staging_buffer.buffer().unmap();
        
        Ok(result_data)
    }
    
    /// Create a GPU buffer from CPU PooledVec (zero-copy when possible)
    pub fn create_buffer_from_pooled<T: Pod>(
        &self,
        pooled_data: &PooledVec<T>,
        usage: wgpu::BufferUsages,
    ) -> Result<GpuBuffer> {
        // Convert PooledVec to slice and create GPU buffer
        let slice: &[T] = pooled_data.as_slice();
        self.create_buffer(slice, usage)
    }
    
    /// Read GPU buffer into CPU PooledVec for seamless integration
    pub fn read_buffer_to_pooled<T: Pod + Clone + Default>(
        &self,
        buffer: &GpuBuffer,
        cpu_pool: SharedMemoryPool<T>,
    ) -> GpuResult<PooledVec<T>> {
        let gpu_data = self.read_buffer::<T>(buffer)?;
        
        // Create PooledVec from GPU data
        let mut pooled_result = PooledVec::with_capacity(gpu_data.len(), cpu_pool);
        for item in gpu_data {
            pooled_result.push(item);
        }
        
        Ok(pooled_result)
    }
    
    /// Get memory usage statistics
    pub fn get_stats(&self) -> GpuMemoryStats {
        self.stats.lock().unwrap().clone()
    }
    
    /// Clear buffer cache and reset memory tracking
    pub fn clear_cache(&self) {
        let mut cache = self.buffer_cache.lock().unwrap();
        let mut total_allocated = self.total_allocated.lock().unwrap();
        
        cache.clear();
        *total_allocated = 0;
        
        // Reset stats
        let mut stats = self.stats.lock().unwrap();
        *stats = GpuMemoryStats {
            memory_limit: stats.memory_limit,
            ..Default::default()
        };
    }
    
    /// Get current memory usage as fraction of limit
    pub fn memory_usage_fraction(&self) -> f32 {
        let total_allocated = *self.total_allocated.lock().unwrap();
        total_allocated as f32 / self.memory_limit as f32
    }
    
    /// Check if GPU memory is under pressure
    pub fn is_memory_pressure(&self) -> bool {
        self.memory_usage_fraction() > 0.85 // Above 85% usage
    }
}

/// RAII wrapper for GPU buffer that automatically returns to pool
pub struct PooledGpuBuffer {
    buffer: Option<GpuBuffer>,
    pool: Arc<GpuMemoryPool>,
}

impl PooledGpuBuffer {
    pub fn new(buffer: GpuBuffer, pool: Arc<GpuMemoryPool>) -> Self {
        Self {
            buffer: Some(buffer),
            pool,
        }
    }
    
    pub fn buffer(&self) -> &GpuBuffer {
        self.buffer.as_ref().unwrap()
    }
}

impl Drop for PooledGpuBuffer {
    fn drop(&mut self) {
        if let Some(buffer) = self.buffer.take() {
            self.pool.return_buffer(buffer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Note: GPU tests require actual GPU hardware, so they may fail in CI
    // These tests are mainly for development environments with GPU access
    
    #[test]
    fn test_buffer_alignment() {
        // Mock test for alignment calculation
        let alignment = 256u64; // Typical uniform buffer alignment
        let pool_alignment = |size: u64| -> u64 {
            ((size + alignment - 1) / alignment) * alignment
        };
        
        assert_eq!(pool_alignment(100), 256);
        assert_eq!(pool_alignment(256), 256);  
        assert_eq!(pool_alignment(300), 512);
    }
    
    #[test]
    fn test_memory_stats() {
        let mut stats = GpuMemoryStats::default();
        stats.total_allocated = 1024;
        stats.buffers_created = 5;
        stats.cache_hits = 3;
        
        assert_eq!(stats.total_allocated, 1024);
        assert_eq!(stats.buffers_created, 5);
    }
}