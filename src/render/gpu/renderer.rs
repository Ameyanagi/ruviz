//! GPU-accelerated plotting renderer
//!
//! This module provides a high-performance GPU renderer that integrates seamlessly
//! with the existing memory pool system for optimal performance across all dataset sizes.

use super::GpuBackend;
use super::compute::{ComputeManager, TransformParams};
use super::memory::GpuMemoryPool;
use crate::core::{PlottingError, Result};
use crate::data::{Data1D, PooledVec, SharedMemoryPool};
use crate::render::backend::Renderer;
use crate::render::pooled::PooledRenderer;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;

/// High-performance GPU renderer with automatic CPU/GPU hybrid optimization
pub struct GpuRenderer {
    /// GPU backend for compute operations
    gpu_backend: Arc<GpuBackend>,
    /// CPU fallback renderer with memory pools
    cpu_fallback: PooledRenderer,
    /// GPU compute manager for coordinate transformations
    compute_manager: ComputeManager,
    /// GPU memory pools for buffer management
    gpu_memory_pool: GpuMemoryPool,
    /// Threshold for GPU vs CPU decision (number of points)
    gpu_threshold: usize,
    /// Statistics tracking
    stats: GpuRendererStats,
}

/// GPU-optimized vertex format
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GpuVertex {
    /// Position in screen coordinates
    position: [f32; 2],
    /// Color as packed RGBA (u8 components)
    color: u32,
    /// Point size for markers
    size: f32,
}

/// GPU renderer statistics
#[derive(Debug, Clone, Default)]
pub struct GpuRendererStats {
    /// Total points processed on GPU
    pub gpu_points_processed: u64,
    /// Total points processed on CPU  
    pub cpu_points_processed: u64,
    /// Number of GPU operations
    pub gpu_operations: u64,
    /// Number of CPU fallback operations
    pub cpu_operations: u64,
    /// Average GPU processing time (microseconds)
    pub avg_gpu_time: f32,
    /// Average CPU processing time (microseconds)
    pub avg_cpu_time: f32,
    /// GPU memory usage (bytes)
    pub gpu_memory_used: u64,
    /// CPU memory pool usage (bytes)
    pub cpu_memory_used: u64,
}

impl GpuRenderer {
    /// Create a new GPU renderer with automatic fallback capability
    pub async fn new() -> Result<Self> {
        let config = super::GpuConfig::default();
        Self::with_config(config).await
    }

    /// Create GPU renderer with custom configuration
    pub async fn with_config(config: super::GpuConfig) -> Result<Self> {
        // Initialize GPU backend
        let gpu_backend = Arc::new(GpuBackend::with_config(config).await.map_err(|e| {
            PlottingError::GpuInitError {
                backend: "wgpu".to_string(),
                error: format!("{}", e),
            }
        })?);

        // Create CPU fallback renderer (always available)
        let cpu_fallback = PooledRenderer::new();

        // Initialize compute manager
        let compute_manager =
            gpu_backend
                .create_compute_manager()
                .map_err(|e| PlottingError::GpuInitError {
                    backend: "compute".to_string(),
                    error: format!("{}", e),
                })?;

        // Initialize GPU memory pool
        let gpu_memory_pool = GpuMemoryPool::new(
            gpu_backend.device().device().clone(),
            gpu_backend.device().queue().clone(),
            gpu_backend.capabilities(),
        )?;

        // Set intelligent threshold based on GPU capabilities
        let gpu_threshold = Self::calculate_gpu_threshold(gpu_backend.capabilities());

        Ok(Self {
            gpu_backend,
            cpu_fallback,
            compute_manager,
            gpu_memory_pool,
            gpu_threshold,
            stats: GpuRendererStats::default(),
        })
    }

    /// Calculate optimal threshold for GPU vs CPU selection
    fn calculate_gpu_threshold(capabilities: &super::GpuCapabilities) -> usize {
        // Base threshold on GPU memory and compute capability
        let base_threshold = if capabilities.supports_compute {
            5_000 // Use GPU for 5K+ points if compute shaders available
        } else {
            50_000 // Higher threshold for render-only GPUs
        };

        // Adjust based on available memory
        if let Some(memory) = capabilities.memory_size {
            let memory_gb = memory / (1024 * 1024 * 1024);
            match memory_gb {
                0..=2 => base_threshold * 2, // Low memory: higher threshold
                3..=8 => base_threshold,     // Normal memory: base threshold
                _ => base_threshold / 2,     // High memory: lower threshold
            }
        } else {
            base_threshold
        }
    }

    /// Transform coordinates using optimal CPU/GPU path
    pub fn transform_coordinates_optimal<T>(
        &mut self,
        x_data: &T,
        y_data: &T,
        x_range: (f64, f64),
        y_range: (f64, f64),
        viewport: (f32, f32, f32, f32), // (left, top, right, bottom)
    ) -> Result<(PooledVec<f32>, PooledVec<f32>)>
    where
        T: Data1D<f64>,
    {
        let point_count = x_data.len().min(y_data.len());

        // Intelligent backend selection
        if point_count >= self.gpu_threshold && self.gpu_backend.is_available() {
            // Use GPU for large datasets
            self.transform_coordinates_gpu(x_data, y_data, x_range, y_range, viewport)
                .or_else(|_| {
                    // GPU failed, fallback to CPU with memory pools
                    log::warn!("GPU coordinate transformation failed, falling back to CPU");
                    self.stats.cpu_operations += 1;
                    self.transform_coordinates_cpu(x_data, y_data, x_range, y_range, viewport)
                })
        } else {
            // Use CPU with memory pools for small datasets
            self.transform_coordinates_cpu(x_data, y_data, x_range, y_range, viewport)
        }
    }

    /// GPU-accelerated coordinate transformation
    fn transform_coordinates_gpu<T>(
        &mut self,
        x_data: &T,
        y_data: &T,
        x_range: (f64, f64),
        y_range: (f64, f64),
        viewport: (f32, f32, f32, f32),
    ) -> Result<(PooledVec<f32>, PooledVec<f32>)>
    where
        T: Data1D<f64>,
    {
        let start_time = std::time::Instant::now();
        let point_count = x_data.len().min(y_data.len());

        // Prepare input data for GPU
        let mut input_points = Vec::with_capacity(point_count);
        for i in 0..point_count {
            let x = x_data.get(i).copied().unwrap_or(0.0) as f32;
            let y = y_data.get(i).copied().unwrap_or(0.0) as f32;
            input_points.push([x, y]);
        }

        // Create GPU buffers
        let input_buffer = self.gpu_memory_pool.create_buffer(
            &input_points,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        )?;

        let output_buffer = self.gpu_memory_pool.create_buffer_empty::<[f32; 2]>(
            point_count,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        )?;

        // Setup transformation parameters
        let (left, top, right, bottom) = viewport;
        let params = TransformParams {
            scale_x: (right - left) as f32 / (x_range.1 - x_range.0) as f32,
            scale_y: (bottom - top) as f32 / (y_range.1 - y_range.0) as f32,
            offset_x: left
                - (x_range.0 as f32 * (right - left) as f32 / (x_range.1 - x_range.0) as f32),
            offset_y: top
                - (y_range.0 as f32 * (bottom - top) as f32 / (y_range.1 - y_range.0) as f32),
            width: (right - left) as u32,
            height: (bottom - top) as u32,
            _padding: [0, 0],
        };

        // Execute GPU compute
        self.compute_manager.execute_transform(
            &input_buffer,
            &output_buffer,
            &params,
            point_count as u32,
        )?;

        // Read back results
        let output_data = self
            .gpu_memory_pool
            .read_buffer::<[f32; 2]>(&output_buffer)?;

        // Convert to separate X/Y arrays using CPU memory pools
        let x_pool = SharedMemoryPool::new(point_count);
        let y_pool = SharedMemoryPool::new(point_count);
        let mut x_result = PooledVec::with_capacity(point_count, x_pool);
        let mut y_result = PooledVec::with_capacity(point_count, y_pool);

        for point in output_data {
            x_result.push(point[0]);
            y_result.push(point[1]);
        }

        // Update statistics
        let elapsed = start_time.elapsed().as_micros() as f32;
        self.stats.gpu_operations += 1;
        self.stats.gpu_points_processed += point_count as u64;
        self.stats.avg_gpu_time =
            (self.stats.avg_gpu_time * (self.stats.gpu_operations - 1) as f32 + elapsed)
                / self.stats.gpu_operations as f32;

        Ok((x_result, y_result))
    }

    /// CPU fallback coordinate transformation with memory pools
    fn transform_coordinates_cpu<T>(
        &mut self,
        x_data: &T,
        y_data: &T,
        x_range: (f64, f64),
        y_range: (f64, f64),
        viewport: (f32, f32, f32, f32),
    ) -> Result<(PooledVec<f32>, PooledVec<f32>)>
    where
        T: Data1D<f64>,
    {
        let start_time = std::time::Instant::now();
        let point_count = x_data.len().min(y_data.len());

        // Use pooled renderer for CPU processing
        let (left, _top, right, bottom) = viewport;
        let x_result = self
            .cpu_fallback
            .transform_x_coordinates_pooled(x_data, x_range.0, x_range.1, left, right)?;
        let y_result = self.cpu_fallback.transform_y_coordinates_pooled(
            y_data, y_range.0, y_range.1, bottom, left, // Y is flipped
        )?;

        // Update statistics
        let elapsed = start_time.elapsed().as_micros() as f32;
        self.stats.cpu_operations += 1;
        self.stats.cpu_points_processed += point_count as u64;
        self.stats.avg_cpu_time =
            (self.stats.avg_cpu_time * (self.stats.cpu_operations - 1) as f32 + elapsed)
                / self.stats.cpu_operations as f32;

        Ok((x_result, y_result))
    }

    /// Check if GPU acceleration is available and recommended for given dataset size
    pub fn should_use_gpu(&self, point_count: usize) -> bool {
        point_count >= self.gpu_threshold && self.gpu_backend.is_available()
    }

    /// Get current GPU threshold
    pub fn gpu_threshold(&self) -> usize {
        self.gpu_threshold
    }

    /// Set GPU threshold manually
    pub fn set_gpu_threshold(&mut self, threshold: usize) {
        self.gpu_threshold = threshold;
    }

    /// Get performance statistics
    pub fn get_stats(&self) -> &GpuRendererStats {
        &self.stats
    }

    /// Reset performance statistics  
    pub fn reset_stats(&mut self) {
        self.stats = GpuRendererStats::default();
    }

    /// Get GPU capabilities
    pub fn gpu_capabilities(&self) -> &super::GpuCapabilities {
        self.gpu_backend.capabilities()
    }

    /// Check if GPU backend is available
    pub fn is_gpu_available(&self) -> bool {
        self.gpu_backend.is_available()
    }
}

impl Renderer for GpuRenderer {
    type Error = PlottingError;

    fn render(&self) -> std::result::Result<(), Self::Error> {
        // Basic render implementation - this would be expanded
        // for full GPU rendering pipeline
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gpu_renderer_creation() {
        // Should gracefully handle GPU not available
        let result = GpuRenderer::new().await;

        if result.is_ok() {
            let renderer = result.unwrap();
            assert!(renderer.gpu_threshold() > 0);
            println!(
                "GPU renderer created successfully with threshold: {}",
                renderer.gpu_threshold()
            );
        } else {
            println!("GPU not available, which is expected in CI environments");
        }
    }

    #[tokio::test]
    async fn test_coordinate_transformation() {
        if let Ok(mut renderer) = GpuRenderer::new().await {
            let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
            let y_data = vec![10.0, 20.0, 30.0, 40.0, 50.0];

            let result = renderer.transform_coordinates_optimal(
                &x_data,
                &y_data,
                (1.0, 5.0),               // x_range
                (10.0, 50.0),             // y_range
                (0.0, 0.0, 800.0, 600.0), // viewport
            );

            assert!(result.is_ok());
            let (x_transformed, y_transformed) = result.unwrap();
            assert_eq!(x_transformed.len(), 5);
            assert_eq!(y_transformed.len(), 5);
        }
    }
}
