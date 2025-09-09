//! GPU pipeline management and caching

use crate::core::error::PlottingError;
use crate::render::gpu::{GpuDevice, PipelineStats};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Render pipeline configuration
#[derive(Debug, Clone)]
pub struct RenderPipelineConfig {
    pub vertex_shader: String,
    pub fragment_shader: String,
    pub vertex_attributes: Vec<wgpu::VertexAttribute>,
    pub color_format: wgpu::TextureFormat,
    pub depth_format: Option<wgpu::TextureFormat>,
    pub sample_count: u32,
    pub primitive_topology: wgpu::PrimitiveTopology,
    pub blend_state: Option<wgpu::BlendState>,
}

impl Hash for RenderPipelineConfig {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.vertex_shader.hash(state);
        self.fragment_shader.hash(state);
        // Note: Hashing vertex attributes and other fields would require custom implementations
        // For now, we hash the shaders which are the main differentiators
        self.color_format.hash(state);
        self.depth_format.hash(state);
        self.sample_count.hash(state);
        self.primitive_topology.hash(state);
    }
}

impl PartialEq for RenderPipelineConfig {
    fn eq(&self, other: &Self) -> bool {
        self.vertex_shader == other.vertex_shader
            && self.fragment_shader == other.fragment_shader
            && self.color_format == other.color_format
            && self.depth_format == other.depth_format
            && self.sample_count == other.sample_count
            && self.primitive_topology == other.primitive_topology
    }
}

impl Eq for RenderPipelineConfig {}

/// Compute pipeline configuration
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ComputePipelineConfig {
    pub compute_shader: String,
    pub workgroup_size: [u32; 3],
}

/// Cached render pipeline
pub struct RenderPipeline {
    pipeline: wgpu::RenderPipeline,
    config: RenderPipelineConfig,
    bind_group_layout: wgpu::BindGroupLayout,
    created_at: std::time::Instant,
    use_count: usize,
}

impl RenderPipeline {
    /// Create new render pipeline
    pub fn new(
        device: &GpuDevice,
        config: RenderPipelineConfig,
    ) -> Result<Self, PlottingError> {
        // Create vertex buffer layout
        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: config.vertex_attributes.iter().map(|attr| attr.format.size()).sum(),
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &config.vertex_attributes,
        };
        
        // Create bind group layout (basic uniform layout)
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Render Pipeline Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        
        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Create shaders
        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(config.vertex_shader.as_str().into()),
        });
        
        let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fragment Shader"),
            source: wgpu::ShaderSource::Wgsl(config.fragment_shader.as_str().into()),
        });
        
        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: "vs_main",
                buffers: &[vertex_buffer_layout],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.color_format,
                    blend: config.blend_state,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: config.primitive_topology,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: config.depth_format.map(|format| wgpu::DepthStencilState {
                format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: config.sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });
        
        Ok(Self {
            pipeline,
            config,
            bind_group_layout,
            created_at: std::time::Instant::now(),
            use_count: 0,
        })
    }
    
    /// Get wgpu render pipeline
    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }
    
    /// Get bind group layout
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
    
    /// Get configuration
    pub fn config(&self) -> &RenderPipelineConfig {
        &self.config
    }
    
    /// Increment use count
    pub fn use_pipeline(&mut self) {
        self.use_count += 1;
    }
    
    /// Get use count
    pub fn use_count(&self) -> usize {
        self.use_count
    }
    
    /// Get age since creation
    pub fn age(&self) -> std::time::Duration {
        self.created_at.elapsed()
    }
}

/// Cached compute pipeline
pub struct ComputePipeline {
    pipeline: wgpu::ComputePipeline,
    config: ComputePipelineConfig,
    bind_group_layout: wgpu::BindGroupLayout,
    created_at: std::time::Instant,
    use_count: usize,
}

impl ComputePipeline {
    /// Create new compute pipeline
    pub fn new(
        device: &GpuDevice,
        config: ComputePipelineConfig,
    ) -> Result<Self, PlottingError> {
        // Create bind group layout for compute operations
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Compute Pipeline Bind Group Layout"),
            entries: &[
                // Input buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Output buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        
        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compute Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Create compute shader
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(config.compute_shader.as_str().into()),
        });
        
        // Create compute pipeline
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &compute_shader,
            entry_point: "cs_main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        
        Ok(Self {
            pipeline,
            config,
            bind_group_layout,
            created_at: std::time::Instant::now(),
            use_count: 0,
        })
    }
    
    /// Get wgpu compute pipeline
    pub fn pipeline(&self) -> &wgpu::ComputePipeline {
        &self.pipeline
    }
    
    /// Get bind group layout
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
    
    /// Get configuration
    pub fn config(&self) -> &ComputePipelineConfig {
        &self.config
    }
    
    /// Increment use count
    pub fn use_pipeline(&mut self) {
        self.use_count += 1;
    }
    
    /// Get use count
    pub fn use_count(&self) -> usize {
        self.use_count
    }
    
    /// Get age since creation
    pub fn age(&self) -> std::time::Duration {
        self.created_at.elapsed()
    }
}

/// Pipeline cache for efficient reuse
pub struct PipelineCache {
    render_pipelines: HashMap<RenderPipelineConfig, RenderPipeline>,
    compute_pipelines: HashMap<ComputePipelineConfig, ComputePipeline>,
    cache_hits: usize,
    cache_misses: usize,
    max_cache_size: usize,
}

impl PipelineCache {
    /// Create new pipeline cache
    pub fn new() -> Self {
        Self {
            render_pipelines: HashMap::new(),
            compute_pipelines: HashMap::new(),
            cache_hits: 0,
            cache_misses: 0,
            max_cache_size: 32, // Reasonable limit for pipeline cache
        }
    }
    
    /// Get or create render pipeline
    pub fn get_render_pipeline(
        &mut self,
        device: &GpuDevice,
        config: RenderPipelineConfig,
    ) -> Result<&mut RenderPipeline, PlottingError> {
        if self.render_pipelines.contains_key(&config) {
            self.cache_hits += 1;
            let pipeline = self.render_pipelines.get_mut(&config).unwrap();
            pipeline.use_pipeline();
            Ok(pipeline)
        } else {
            self.cache_misses += 1;
            
            // Check cache size limit
            if self.render_pipelines.len() >= self.max_cache_size {
                self.evict_old_render_pipelines();
            }
            
            let pipeline = RenderPipeline::new(device, config.clone())?;
            self.render_pipelines.insert(config.clone(), pipeline);
            
            let pipeline = self.render_pipelines.get_mut(&config).unwrap();
            pipeline.use_pipeline();
            Ok(pipeline)
        }
    }
    
    /// Get or create compute pipeline
    pub fn get_compute_pipeline(
        &mut self,
        device: &GpuDevice,
        config: ComputePipelineConfig,
    ) -> Result<&mut ComputePipeline, PlottingError> {
        if self.compute_pipelines.contains_key(&config) {
            self.cache_hits += 1;
            let pipeline = self.compute_pipelines.get_mut(&config).unwrap();
            pipeline.use_pipeline();
            Ok(pipeline)
        } else {
            self.cache_misses += 1;
            
            // Check cache size limit
            if self.compute_pipelines.len() >= self.max_cache_size {
                self.evict_old_compute_pipelines();
            }
            
            let pipeline = ComputePipeline::new(device, config.clone())?;
            self.compute_pipelines.insert(config.clone(), pipeline);
            
            let pipeline = self.compute_pipelines.get_mut(&config).unwrap();
            pipeline.use_pipeline();
            Ok(pipeline)
        }
    }
    
    /// Evict old render pipelines
    fn evict_old_render_pipelines(&mut self) {
        if self.render_pipelines.len() <= self.max_cache_size / 2 {
            return;
        }
        
        // Find least recently used pipelines
        let mut pipelines_by_usage: Vec<_> = self.render_pipelines
            .iter()
            .map(|(config, pipeline)| (config.clone(), pipeline.use_count(), pipeline.age()))
            .collect();
        
        // Sort by use count (ascending) then by age (descending)
        pipelines_by_usage.sort_by(|a, b| {
            a.1.cmp(&b.1).then(b.2.cmp(&a.2))
        });
        
        // Remove least used pipelines
        let to_remove = pipelines_by_usage.len() / 4; // Remove 25%
        for (config, _, _) in pipelines_by_usage.into_iter().take(to_remove) {
            self.render_pipelines.remove(&config);
        }
    }
    
    /// Evict old compute pipelines
    fn evict_old_compute_pipelines(&mut self) {
        if self.compute_pipelines.len() <= self.max_cache_size / 2 {
            return;
        }
        
        // Find least recently used pipelines
        let mut pipelines_by_usage: Vec<_> = self.compute_pipelines
            .iter()
            .map(|(config, pipeline)| (config.clone(), pipeline.use_count(), pipeline.age()))
            .collect();
        
        // Sort by use count (ascending) then by age (descending)
        pipelines_by_usage.sort_by(|a, b| {
            a.1.cmp(&b.1).then(b.2.cmp(&a.2))
        });
        
        // Remove least used pipelines
        let to_remove = pipelines_by_usage.len() / 4; // Remove 25%
        for (config, _, _) in pipelines_by_usage.into_iter().take(to_remove) {
            self.compute_pipelines.remove(&config);
        }
    }
    
    /// Clear all cached pipelines
    pub fn clear(&mut self) {
        self.render_pipelines.clear();
        self.compute_pipelines.clear();
        self.cache_hits = 0;
        self.cache_misses = 0;
    }
    
    /// Get cache statistics
    pub fn get_stats(&self) -> PipelineStats {
        PipelineStats {
            total_pipelines: self.render_pipelines.len() + self.compute_pipelines.len(),
            cache_hits: self.cache_hits,
            cache_misses: self.cache_misses,
        }
    }
    
    /// Get cache efficiency (hit rate)
    pub fn cache_efficiency(&self) -> f32 {
        let total_requests = self.cache_hits + self.cache_misses;
        if total_requests > 0 {
            self.cache_hits as f32 / total_requests as f32
        } else {
            0.0
        }
    }
}

impl Default for PipelineCache {
    fn default() -> Self {
        Self::new()
    }
}