//! GPU compute shader support for accelerated data processing
//! 
//! This module provides compute pipeline management for GPU-accelerated operations
//! such as coordinate transformations, aggregations, and filtering.

use crate::core::error::{PlottingError, Result};
use std::sync::Arc;
use wgpu::*;

/// Compute shader manager for GPU-accelerated operations
pub struct ComputeManager {
    device: Arc<Device>,
    queue: Arc<Queue>,
    compute_pipelines: std::collections::HashMap<String, ComputePipeline>,
}

impl ComputeManager {
    /// Creates a new compute manager
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        Self {
            device,
            queue,
            compute_pipelines: std::collections::HashMap::new(),
        }
    }

    /// Creates a coordinate transformation compute pipeline
    pub fn create_transform_pipeline(&mut self) -> Result<()> {
        let shader_source = include_str!("shaders/transform.wgsl");
        let shader_module = self.device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Transform Compute Shader"),
            source: ShaderSource::Wgsl(shader_source.into()),
        });

        let bind_group_layout = self.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Transform Bind Group Layout"),
            entries: &[
                // Input data buffer
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Output buffer
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Transform parameters
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = self.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Transform Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = self.device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("Transform Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: "main",
            compilation_options: PipelineCompilationOptions::default(),
            cache: None,
        });

        self.compute_pipelines.insert("transform".to_string(), pipeline);
        Ok(())
    }

    /// Creates an aggregation compute pipeline for DataShader-style operations
    pub fn create_aggregation_pipeline(&mut self) -> Result<()> {
        let shader_source = include_str!("shaders/aggregate.wgsl");
        let shader_module = self.device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Aggregation Compute Shader"),
            source: ShaderSource::Wgsl(shader_source.into()),
        });

        let bind_group_layout = self.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Aggregation Bind Group Layout"),
            entries: &[
                // Input points buffer
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Canvas output buffer
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Aggregation parameters
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = self.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Aggregation Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = self.device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("Aggregation Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: "main",
            compilation_options: PipelineCompilationOptions::default(),
            cache: None,
        });

        self.compute_pipelines.insert("aggregate".to_string(), pipeline);
        Ok(())
    }

    /// Dispatches coordinate transformation on GPU
    pub async fn transform_coordinates(
        &self,
        input_buffer: &Buffer,
        output_buffer: &Buffer,
        transform_params: &Buffer,
        point_count: u32,
    ) -> Result<()> {
        let pipeline = self.compute_pipelines.get("transform")
            .ok_or_else(|| PlottingError::GpuInitError {
                backend: "wgpu".to_string(),
                error: "Transform pipeline not initialized".to_string(),
            })?;

        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Transform Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: transform_params.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Transform Command Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("Transform Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            // Use workgroups of 64 for optimal GPU utilization
            let workgroup_size = 64;
            let num_workgroups = (point_count + workgroup_size - 1) / workgroup_size;
            compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
        }

        let command_buffer = encoder.finish();
        self.queue.submit([command_buffer]);

        // Wait for completion
        self.device.poll(Maintain::Wait);

        Ok(())
    }

    /// Dispatches aggregation operation on GPU
    pub async fn aggregate_points(
        &self,
        points_buffer: &Buffer,
        canvas_buffer: &Buffer,
        params_buffer: &Buffer,
        point_count: u32,
    ) -> Result<()> {
        let pipeline = self.compute_pipelines.get("aggregate")
            .ok_or_else(|| PlottingError::GpuInitError {
                backend: "wgpu".to_string(),
                error: "Aggregation pipeline not initialized".to_string(),
            })?;

        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Aggregation Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: points_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: canvas_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Aggregation Command Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("Aggregation Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            // Use workgroups of 64 for optimal GPU utilization
            let workgroup_size = 64;
            let num_workgroups = (point_count + workgroup_size - 1) / workgroup_size;
            compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
        }

        let command_buffer = encoder.finish();
        self.queue.submit([command_buffer]);

        // Wait for completion
        self.device.poll(Maintain::Wait);

        Ok(())
    }

    /// Gets compute pipeline statistics
    pub fn get_stats(&self) -> ComputeStats {
        ComputeStats {
            pipeline_count: self.compute_pipelines.len(),
            available_pipelines: self.compute_pipelines.keys().cloned().collect(),
        }
    }
}

/// Compute pipeline statistics
#[derive(Debug)]
pub struct ComputeStats {
    pub pipeline_count: usize,
    pub available_pipelines: Vec<String>,
}

/// Transform parameters for coordinate transformation shader
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TransformParams {
    pub scale_x: f32,
    pub scale_y: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub width: u32,
    pub height: u32,
    pub _padding: [u32; 2], // Align to 32 bytes
}

/// Aggregation parameters for DataShader-style operations
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AggregationParams {
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub x_min: f32,
    pub x_max: f32,
    pub y_min: f32,
    pub y_max: f32,
    pub _padding: [u32; 2], // Align to 32 bytes
}

unsafe impl bytemuck::Pod for TransformParams {}
unsafe impl bytemuck::Zeroable for TransformParams {}

unsafe impl bytemuck::Pod for AggregationParams {}
unsafe impl bytemuck::Zeroable for AggregationParams {}