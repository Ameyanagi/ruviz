//! GPU device management and selection

use crate::core::error::PlottingError;
use crate::render::gpu::GpuConfig;
use std::sync::Arc;

/// GPU device wrapper with enhanced functionality
pub struct GpuDevice {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    adapter: wgpu::Adapter,
    info: GpuDeviceInfo,
}

/// Information about the GPU device
#[derive(Debug, Clone)]
pub struct GpuDeviceInfo {
    pub name: String,
    pub vendor: String,
    pub device_type: wgpu::DeviceType,
    pub backend: wgpu::Backend,
    pub driver_name: String,
    pub driver_info: String,
    pub features: wgpu::Features,
    pub limits: wgpu::Limits,
}

/// Device selection criteria
pub struct DeviceSelector {
    power_preference: wgpu::PowerPreference,
    required_features: wgpu::Features,
    required_limits: wgpu::Limits,
    prefer_integrated: bool,
    prefer_discrete: bool,
}

impl Default for DeviceSelector {
    fn default() -> Self {
        Self {
            power_preference: wgpu::PowerPreference::HighPerformance,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            prefer_integrated: false,
            prefer_discrete: true, // Prefer discrete GPU for plotting
        }
    }
}

impl DeviceSelector {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn power_preference(mut self, preference: wgpu::PowerPreference) -> Self {
        self.power_preference = preference;
        self
    }
    
    pub fn require_features(mut self, features: wgpu::Features) -> Self {
        self.required_features = features;
        self
    }
    
    pub fn require_limits(mut self, limits: wgpu::Limits) -> Self {
        self.required_limits = limits;
        self
    }
    
    pub fn prefer_integrated(mut self) -> Self {
        self.prefer_integrated = true;
        self.prefer_discrete = false;
        self
    }
    
    pub fn prefer_discrete(mut self) -> Self {
        self.prefer_discrete = true;
        self.prefer_integrated = false;
        self
    }
    
    /// Select the best adapter based on criteria
    pub async fn select_adapter(&self, instance: &wgpu::Instance) -> Result<wgpu::Adapter, PlottingError> {
        let adapters: Vec<_> = instance.enumerate_adapters(wgpu::Backends::all()).collect();
        
        if adapters.is_empty() {
            return Err(PlottingError::GpuNotAvailable("No GPU adapters found".to_string()));
        }
        
        // Score each adapter
        let mut scored_adapters: Vec<_> = adapters
            .into_iter()
            .map(|adapter| {
                let info = adapter.get_info();
                let score = self.score_adapter(&adapter, &info);
                (adapter, info, score)
            })
            .collect();
        
        // Sort by score (highest first)
        scored_adapters.sort_by(|a, b| b.2.cmp(&a.2));
        
        // Return the highest scored adapter
        scored_adapters
            .into_iter()
            .next()
            .map(|(adapter, _, _)| adapter)
            .ok_or_else(|| PlottingError::GpuNotAvailable("No suitable GPU adapter found".to_string()))
    }
    
    /// Score an adapter based on selection criteria
    fn score_adapter(&self, adapter: &wgpu::Adapter, info: &wgpu::AdapterInfo) -> i32 {
        let mut score = 0;
        
        // Device type preference
        match info.device_type {
            wgpu::DeviceType::DiscreteGpu if self.prefer_discrete => score += 100,
            wgpu::DeviceType::IntegratedGpu if self.prefer_integrated => score += 100,
            wgpu::DeviceType::DiscreteGpu => score += 50,
            wgpu::DeviceType::IntegratedGpu => score += 30,
            wgpu::DeviceType::VirtualGpu => score += 10,
            wgpu::DeviceType::Cpu => score += 1,
            wgpu::DeviceType::Other => score += 5,
        }
        
        // Backend preference (platform-specific)
        match info.backend {
            #[cfg(target_os = "windows")]
            wgpu::Backend::Dx12 => score += 20,
            #[cfg(target_os = "macos")]
            wgpu::Backend::Metal => score += 20,
            #[cfg(target_os = "linux")]
            wgpu::Backend::Vulkan => score += 20,
            wgpu::Backend::Vulkan => score += 15,
            wgpu::Backend::Dx12 => score += 15,
            wgpu::Backend::Metal => score += 15,
            wgpu::Backend::Gl => score += 5,
            wgpu::Backend::BrowserWebGpu => score += 5,
            _ => score += 1,
        }
        
        // Check if adapter supports required features
        let features = adapter.features();
        if !features.contains(self.required_features) {
            return -1000; // Disqualify if requirements not met
        }
        
        // Bonus for additional useful features
        if features.contains(wgpu::Features::COMPUTE_SHADER) {
            score += 30;
        }
        if features.contains(wgpu::Features::STORAGE_RESOURCE_BINDING_ARRAY) {
            score += 20;
        }
        if features.contains(wgpu::Features::TIMESTAMP_QUERY) {
            score += 10;
        }
        if features.contains(wgpu::Features::PIPELINE_STATISTICS_QUERY) {
            score += 10;
        }
        
        // Check limits
        let limits = adapter.limits();
        
        // Prefer larger texture sizes
        if limits.max_texture_dimension_2d >= 16384 {
            score += 20;
        } else if limits.max_texture_dimension_2d >= 8192 {
            score += 10;
        } else if limits.max_texture_dimension_2d < 4096 {
            return -1000; // Disqualify if too small
        }
        
        // Prefer larger buffer sizes
        if limits.max_buffer_size >= 1024 * 1024 * 1024 {
            score += 15; // 1GB+
        } else if limits.max_buffer_size >= 512 * 1024 * 1024 {
            score += 10; // 512MB+
        } else if limits.max_buffer_size < 256 * 1024 * 1024 {
            return -1000; // Disqualify if too small
        }
        
        score
    }
}

impl GpuDevice {
    /// Create new GPU device with automatic adapter selection
    pub async fn new(instance: &wgpu::Instance, config: &GpuConfig) -> Result<Self, PlottingError> {
        let selector = DeviceSelector::default()
            .power_preference(config.power_preference)
            .require_features(config.required_features)
            .require_limits(config.required_limits.clone());
        
        let adapter = selector.select_adapter(instance).await?;
        Self::from_adapter(adapter, config).await
    }
    
    /// Create GPU device from specific adapter
    pub async fn from_adapter(adapter: wgpu::Adapter, config: &GpuConfig) -> Result<Self, PlottingError> {
        let adapter_info = adapter.get_info();
        log::info!(
            "Selected GPU: {} ({:?} on {:?})",
            adapter_info.name,
            adapter_info.device_type,
            adapter_info.backend
        );
        
        // Request device with required features and limits
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Plotting GPU Device"),
                    required_features: config.required_features,
                    required_limits: config.required_limits.clone(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None, // No trace path
            )
            .await
            .map_err(|e| PlottingError::GpuInitError {
                backend: format!("{:?}", adapter_info.backend),
                error: e.to_string(),
            })?;
        
        // Set up error handling
        device.on_uncaptured_error(Box::new(|error| {
            log::error!("GPU Error: {}", error);
        }));
        
        let info = GpuDeviceInfo {
            name: adapter_info.name.clone(),
            vendor: format!("{:?}", adapter_info.vendor),
            device_type: adapter_info.device_type,
            backend: adapter_info.backend,
            driver_name: adapter_info.driver.clone(),
            driver_info: adapter_info.driver_info.clone(),
            features: device.features(),
            limits: device.limits(),
        };
        
        log::info!("GPU device created successfully");
        log::debug!("Device features: {:?}", device.features());
        log::debug!("Device limits: {:?}", device.limits());
        
        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            adapter,
            info,
        })
    }
    
    /// Get wgpu device reference
    pub fn device(&self) -> &Arc<wgpu::Device> {
        &self.device
    }
    
    /// Get wgpu queue reference
    pub fn queue(&self) -> &Arc<wgpu::Queue> {
        &self.queue
    }
    
    /// Get wgpu adapter reference
    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }
    
    /// Get device information
    pub fn info(&self) -> &GpuDeviceInfo {
        &self.info
    }
    
    /// Get device features
    pub fn features(&self) -> wgpu::Features {
        self.device.features()
    }
    
    /// Get device limits
    pub fn limits(&self) -> wgpu::Limits {
        self.device.limits()
    }
    
    /// Check if device is still valid
    pub fn is_valid(&self) -> bool {
        // Simple validation - in practice you might want more checks
        !self.device.is_lost()
    }
    
    /// Create buffer with device extension utility
    pub fn create_buffer_init(&self, desc: &wgpu::util::BufferInitDescriptor) -> wgpu::Buffer {
        self.device.create_buffer_init(desc)
    }
    
    /// Create texture
    pub fn create_texture(&self, desc: &wgpu::TextureDescriptor) -> wgpu::Texture {
        self.device.create_texture(desc)
    }
    
    /// Create render pipeline
    pub fn create_render_pipeline(&self, desc: &wgpu::RenderPipelineDescriptor) -> wgpu::RenderPipeline {
        self.device.create_render_pipeline(desc)
    }
    
    /// Create compute pipeline
    pub fn create_compute_pipeline(&self, desc: &wgpu::ComputePipelineDescriptor) -> wgpu::ComputePipeline {
        self.device.create_compute_pipeline(desc)
    }
    
    /// Create shader module
    pub fn create_shader_module(&self, desc: wgpu::ShaderModuleDescriptor) -> wgpu::ShaderModule {
        self.device.create_shader_module(desc)
    }
    
    /// Create bind group layout
    pub fn create_bind_group_layout(&self, desc: &wgpu::BindGroupLayoutDescriptor) -> wgpu::BindGroupLayout {
        self.device.create_bind_group_layout(desc)
    }
    
    /// Create bind group
    pub fn create_bind_group(&self, desc: &wgpu::BindGroupDescriptor) -> wgpu::BindGroup {
        self.device.create_bind_group(desc)
    }
    
    /// Submit command buffer
    pub fn submit<I: IntoIterator<Item = wgpu::CommandBuffer>>(&self, command_buffers: I) -> wgpu::SubmissionIndex {
        self.queue.submit(command_buffers)
    }
    
    /// Write buffer data
    pub fn write_buffer(&self, buffer: &wgpu::Buffer, offset: wgpu::BufferAddress, data: &[u8]) {
        self.queue.write_buffer(buffer, offset, data);
    }
    
    /// Write texture data
    pub fn write_texture(
        &self,
        texture: wgpu::ImageCopyTexture,
        data: &[u8],
        data_layout: wgpu::ImageDataLayout,
        size: wgpu::Extent3d,
    ) {
        self.queue.write_texture(texture, data, data_layout, size);
    }
    
    /// Poll for completed operations
    pub fn poll(&self, maintain: wgpu::Maintain) -> wgpu::MaintainResult {
        self.device.poll(maintain)
    }
    
    /// Get memory usage (if supported)
    pub fn memory_usage(&self) -> Option<wgpu::MemoryReport> {
        // wgpu doesn't currently expose memory usage directly
        // This would require vendor-specific extensions
        None
    }
    
    /// Generate debug report
    pub fn debug_info(&self) -> String {
        format!(
            "GPU Device: {}\n\
             Vendor: {}\n\
             Type: {:?}\n\
             Backend: {:?}\n\
             Driver: {} ({})\n\
             Features: {:?}\n\
             Max Texture Size: {}x{}\n\
             Max Buffer Size: {} MB",
            self.info.name,
            self.info.vendor,
            self.info.device_type,
            self.info.backend,
            self.info.driver_name,
            self.info.driver_info,
            self.info.features,
            self.info.limits.max_texture_dimension_2d,
            self.info.limits.max_texture_dimension_2d,
            self.info.limits.max_buffer_size / (1024 * 1024)
        )
    }
}

// Implement Send and Sync for GpuDevice (wgpu types are thread-safe)
unsafe impl Send for GpuDevice {}
unsafe impl Sync for GpuDevice {}

impl std::ops::Deref for GpuDevice {
    type Target = wgpu::Device;
    
    fn deref(&self) -> &Self::Target {
        &*self.device
    }
}