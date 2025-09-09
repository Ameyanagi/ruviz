//! GPU backend foundation with wgpu
//! High-performance GPU-accelerated rendering for massive datasets

use crate::core::error::PlottingError;
use crate::data::platform::get_platform_optimizer;
use std::sync::{Arc, Mutex, OnceLock};
use wgpu::util::DeviceExt;

pub mod device;
pub mod buffer;
pub mod pipeline;
pub mod compute;

pub use device::{GpuDevice, GpuDeviceInfo, DeviceSelector};
pub use buffer::{GpuBuffer, BufferUsage, BufferManager};
pub use pipeline::{RenderPipeline, ComputePipeline, PipelineCache};
pub use compute::{ComputeManager, ComputeStats, TransformParams, AggregationParams};

/// GPU backend capabilities and configuration
#[derive(Debug, Clone)]
pub struct GpuBackend {
    device: Arc<GpuDevice>,
    buffer_manager: Arc<Mutex<BufferManager>>,
    pipeline_cache: Arc<Mutex<PipelineCache>>,
    capabilities: GpuCapabilities,
    config: GpuConfig,
}

/// GPU capabilities detected at runtime
#[derive(Debug, Clone)]
pub struct GpuCapabilities {
    /// Maximum texture size (width/height)
    pub max_texture_size: u32,
    /// Maximum buffer size in bytes
    pub max_buffer_size: u64,
    /// Maximum number of compute workgroups
    pub max_compute_workgroups: [u32; 3],
    /// Available memory in bytes
    pub memory_size: Option<u64>,
    /// Supports compute shaders
    pub supports_compute: bool,
    /// Supports storage textures
    pub supports_storage_textures: bool,
    /// Supports timestamp queries
    pub supports_timestamps: bool,
    /// Maximum number of concurrent render passes
    pub max_render_targets: u32,
    /// Supported texture formats
    pub supported_formats: Vec<wgpu::TextureFormat>,
}

/// GPU backend configuration
#[derive(Debug, Clone)]
pub struct GpuConfig {
    /// Enable GPU acceleration
    pub enable_gpu: bool,
    /// Preferred backend (Vulkan, Metal, DX12, GL)
    pub preferred_backend: Option<wgpu::Backends>,
    /// Memory usage limit as fraction of total GPU memory
    pub memory_limit_fraction: f32,
    /// Enable debug validation layers
    pub enable_validation: bool,
    /// Enable GPU profiling
    pub enable_profiling: bool,
    /// Force specific power preference
    pub power_preference: wgpu::PowerPreference,
    /// Required features
    pub required_features: wgpu::Features,
    /// Required limits
    pub required_limits: wgpu::Limits,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            enable_gpu: true,
            preferred_backend: None, // Auto-detect best backend
            memory_limit_fraction: 0.8, // Use 80% of GPU memory
            enable_validation: cfg!(debug_assertions),
            enable_profiling: false,
            power_preference: wgpu::PowerPreference::HighPerformance,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
        }
    }
}

impl GpuBackend {
    /// Initialize GPU backend with automatic device selection
    pub async fn new() -> Result<Self, PlottingError> {
        Self::with_config(GpuConfig::default()).await
    }
    
    /// Initialize GPU backend with custom configuration
    pub async fn with_config(config: GpuConfig) -> Result<Self, PlottingError> {
        if !config.enable_gpu {
            return Err(PlottingError::FeatureNotEnabled {
                feature: "GPU acceleration".to_string(),
                operation: "GPU backend initialization".to_string(),
            });
        }
        
        // Create wgpu instance with appropriate backends
        let instance = Self::create_instance(&config)?;
        
        // Select and initialize device
        let device = GpuDevice::new(&instance, &config).await?;
        let capabilities = Self::detect_capabilities(&device)?;
        
        // Validate minimum requirements
        Self::validate_capabilities(&capabilities, &config)?;
        
        // Initialize buffer manager with platform-optimized settings
        let platform_optimizer = get_platform_optimizer();
        let hints = platform_optimizer.get_performance_hints();
        let buffer_manager = BufferManager::new(&device, &capabilities, &hints)?;
        
        // Initialize pipeline cache
        let pipeline_cache = PipelineCache::new();
        
        Ok(Self {
            device: Arc::new(device),
            buffer_manager: Arc::new(Mutex::new(buffer_manager)),
            pipeline_cache: Arc::new(Mutex::new(pipeline_cache)),
            capabilities,
            config,
        })
    }
    
    /// Create wgpu instance with platform-appropriate backends
    fn create_instance(config: &GpuConfig) -> Result<wgpu::Instance, PlottingError> {
        let backends = config.preferred_backend.unwrap_or_else(|| {
            // Select best backend for platform
            #[cfg(target_os = "windows")]
            return wgpu::Backends::DX12 | wgpu::Backends::VULKAN;
            
            #[cfg(target_os = "macos")]
            return wgpu::Backends::METAL;
            
            #[cfg(target_os = "linux")]
            return wgpu::Backends::VULKAN | wgpu::Backends::GL;
            
            #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
            return wgpu::Backends::all();
        });
        
        let instance_desc = wgpu::InstanceDescriptor {
            backends,
            flags: if config.enable_validation {
                wgpu::InstanceFlags::DEBUG | wgpu::InstanceFlags::VALIDATION
            } else {
                wgpu::InstanceFlags::default()
            },
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        };
        
        Ok(wgpu::Instance::new(instance_desc))
    }
    
    /// Detect GPU capabilities
    fn detect_capabilities(device: &GpuDevice) -> Result<GpuCapabilities, PlottingError> {
        let limits = device.limits();
        let features = device.features();
        
        Ok(GpuCapabilities {
            max_texture_size: limits.max_texture_dimension_2d,
            max_buffer_size: limits.max_buffer_size,
            max_compute_workgroups: [
                limits.max_compute_workgroups_per_dimension,
                limits.max_compute_workgroups_per_dimension,
                limits.max_compute_workgroups_per_dimension,
            ],
            memory_size: None, // wgpu doesn't expose memory info directly
            supports_compute: features.contains(wgpu::Features::COMPUTE_SHADER),
            supports_storage_textures: features.contains(wgpu::Features::STORAGE_RESOURCE_BINDING_ARRAY),
            supports_timestamps: features.contains(wgpu::Features::TIMESTAMP_QUERY),
            max_render_targets: limits.max_color_attachments,
            supported_formats: Self::get_supported_formats(device),
        })
    }
    
    /// Get list of supported texture formats
    fn get_supported_formats(device: &GpuDevice) -> Vec<wgpu::TextureFormat> {
        let common_formats = [
            wgpu::TextureFormat::R8Unorm,
            wgpu::TextureFormat::Rg8Unorm,
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureFormat::R16Float,
            wgpu::TextureFormat::Rg16Float,
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureFormat::R32Float,
            wgpu::TextureFormat::Rg32Float,
            wgpu::TextureFormat::Rgba32Float,
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        ];
        
        common_formats
            .into_iter()
            .filter(|&format| {
                device.adapter().get_texture_format_features(format).allowed_usages.contains(
                    wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING
                )
            })
            .collect()
    }
    
    /// Validate that GPU meets minimum requirements
    fn validate_capabilities(
        capabilities: &GpuCapabilities,
        config: &GpuConfig,
    ) -> Result<(), PlottingError> {
        // Check minimum texture size (need at least 4K for reasonable plots)
        if capabilities.max_texture_size < 4096 {
            return Err(PlottingError::UnsupportedGpuFeature(
                format!("Maximum texture size {} is too small (minimum 4096)", capabilities.max_texture_size)
            ));
        }
        
        // Check compute shader support if required
        if !capabilities.supports_compute && config.required_features.contains(wgpu::Features::COMPUTE_SHADER) {
            return Err(PlottingError::UnsupportedGpuFeature(
                "Compute shaders required but not supported".to_string()
            ));
        }
        
        // Check minimum buffer size (need at least 256MB for large datasets)
        const MIN_BUFFER_SIZE: u64 = 256 * 1024 * 1024; // 256MB
        if capabilities.max_buffer_size < MIN_BUFFER_SIZE {
            return Err(PlottingError::UnsupportedGpuFeature(
                format!("Maximum buffer size {} is too small (minimum {})", 
                    capabilities.max_buffer_size, MIN_BUFFER_SIZE)
            ));
        }
        
        Ok(())
    }
    
    /// Get GPU device reference
    pub fn device(&self) -> &Arc<GpuDevice> {
        &self.device
    }
    
    /// Get GPU capabilities
    pub fn capabilities(&self) -> &GpuCapabilities {
        &self.capabilities
    }
    
    /// Get GPU configuration
    pub fn config(&self) -> &GpuConfig {
        &self.config
    }
    
    /// Get buffer manager
    pub fn buffer_manager(&self) -> Arc<Mutex<BufferManager>> {
        Arc::clone(&self.buffer_manager)
    }
    
    /// Get pipeline cache
    pub fn pipeline_cache(&self) -> Arc<Mutex<PipelineCache>> {
        Arc::clone(&self.pipeline_cache)
    }
    
    /// Create render pass for plotting operations
    pub fn create_render_pass(
        &self,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Result<GpuRenderPass, PlottingError> {
        GpuRenderPass::new(&self.device, width, height, format, &self.capabilities)
    }
    
    /// Create compute manager for data processing
    pub fn create_compute_manager(&self) -> Result<ComputeManager, PlottingError> {
        if !self.capabilities.supports_compute {
            return Err(PlottingError::UnsupportedGpuFeature(
                "Compute shaders not supported".to_string()
            ));
        }
        
        Ok(ComputeManager::new(
            Arc::clone(self.device.device()),
            Arc::clone(self.device.queue()),
        ))
    }
    
    /// Check if GPU backend is available and functional
    pub fn is_available(&self) -> bool {
        self.device.is_valid()
    }
    
    /// Get performance statistics
    pub fn get_stats(&self) -> GpuStats {
        let buffer_manager = self.buffer_manager.lock().unwrap();
        let pipeline_cache = self.pipeline_cache.lock().unwrap();
        
        GpuStats {
            device_info: self.device.info().clone(),
            buffer_stats: buffer_manager.get_stats(),
            pipeline_stats: pipeline_cache.get_stats(),
            memory_usage: buffer_manager.get_memory_usage(),
            active_passes: 0, // TODO: track active render/compute passes
        }
    }
}

/// GPU render pass for drawing operations
pub struct GpuRenderPass {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    depth_texture: Option<wgpu::Texture>,
    depth_view: Option<wgpu::TextureView>,
}

impl GpuRenderPass {
    fn new(
        device: &GpuDevice,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        capabilities: &GpuCapabilities,
    ) -> Result<Self, PlottingError> {
        // Validate dimensions
        if width > capabilities.max_texture_size || height > capabilities.max_texture_size {
            return Err(PlottingError::GpuMemoryError {
                requested: (width * height * 4) as usize, // Assume RGBA format
                available: Some((capabilities.max_texture_size * capabilities.max_texture_size * 4) as usize),
            });
        }
        
        // Create main render texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Render Target"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        // Create depth buffer if supported
        let (depth_texture, depth_view) = if capabilities.supported_formats.contains(&wgpu::TextureFormat::Depth32Float) {
            let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth Buffer"),
                size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            
            let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
            (Some(depth_texture), Some(depth_view))
        } else {
            (None, None)
        };
        
        Ok(Self {
            texture,
            view,
            format,
            width,
            height,
            depth_texture,
            depth_view,
        })
    }
    
    pub fn color_view(&self) -> &wgpu::TextureView {
        &self.view
    }
    
    pub fn depth_view(&self) -> Option<&wgpu::TextureView> {
        self.depth_view.as_ref()
    }
    
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    
    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }
}

/// GPU performance statistics
#[derive(Debug, Clone)]
pub struct GpuStats {
    pub device_info: GpuDeviceInfo,
    pub buffer_stats: BufferStats,
    pub pipeline_stats: PipelineStats,
    pub memory_usage: u64,
    pub active_passes: u32,
}

#[derive(Debug, Clone)]
pub struct BufferStats {
    pub total_buffers: usize,
    pub total_memory: u64,
    pub active_buffers: usize,
    pub reused_buffers: usize,
}

#[derive(Debug, Clone)]
pub struct PipelineStats {
    pub total_pipelines: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

/// Global GPU backend instance
static GPU_BACKEND: OnceLock<Option<GpuBackend>> = OnceLock::new();

/// Initialize GPU backend (call once at startup)
pub async fn initialize_gpu_backend() -> Result<(), PlottingError> {
    let backend = match GpuBackend::new().await {
        Ok(backend) => Some(backend),
        Err(e) => {
            log::warn!("Failed to initialize GPU backend: {}", e);
            None
        }
    };
    
    GPU_BACKEND.set(backend).map_err(|_| {
        PlottingError::GpuInitError {
            backend: "wgpu".to_string(),
            error: "Backend already initialized".to_string(),
        }
    })?;
    
    Ok(())
}

/// Get global GPU backend instance
pub fn get_gpu_backend() -> Option<&'static GpuBackend> {
    GPU_BACKEND.get().and_then(|backend| backend.as_ref())
}

/// Check if GPU acceleration is available
pub fn is_gpu_available() -> bool {
    get_gpu_backend().map_or(false, |backend| backend.is_available())
}

/// Get GPU capabilities if available
pub fn get_gpu_capabilities() -> Option<&'static GpuCapabilities> {
    get_gpu_backend().map(|backend| backend.capabilities())
}