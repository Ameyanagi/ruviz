//! Lazy sphere-only pipelines and camera uniforms: flat plots pay none of this.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::core::plot3d::layout::Axis3Layout;
use crate::render::three_d::sphere::SphereCamera3D;

use super::pipelines::{PipelineLibrary3D, create_pipeline};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    matrix: [[f32; 4]; 4],
    inverse: [[f32; 4]; 4],
    normal_to_view: [[f32; 4]; 4],
    viewport: [f32; 4],
}

pub(super) struct SpherePipelines {
    pub(super) opaque: wgpu::RenderPipeline,
    pub(super) transparent: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    pub(super) camera_bind_group: wgpu::BindGroup,
}

impl SpherePipelines {
    pub(super) fn new(device: &wgpu::Device, library: &PipelineLibrary3D, samples: u32) -> Self {
        const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
            wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4];
        let buffers = [wgpu::VertexBufferLayout {
            array_stride: 48,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRIBUTES,
        }];
        let layouts = [&library.camera_layout, &library.point_material_layout];
        let make = |label, depth_write| {
            create_pipeline(
                device,
                label,
                include_str!("shaders/sphere.wgsl"),
                &layouts,
                &buffers,
                wgpu::PrimitiveTopology::TriangleStrip,
                samples,
                depth_write,
            )
        };
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ruviz sphere camera"),
            contents: bytemuck::bytes_of(&CameraUniform::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ruviz sphere camera"),
            layout: &library.camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        Self {
            opaque: make("ruviz opaque spheres", true),
            transparent: make("ruviz transparent spheres", false),
            camera_buffer,
            camera_bind_group,
        }
    }

    pub(super) fn update_camera(&self, queue: &wgpu::Queue, layout: &Axis3Layout) {
        let camera = SphereCamera3D::new(layout);
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::bytes_of(&CameraUniform {
                matrix: camera.matrix.to_cols_array_2d(),
                inverse: camera.inverse.to_cols_array_2d(),
                normal_to_view: camera.normal_to_view.to_cols_array_2d(),
                viewport: camera.viewport,
            }),
        );
    }
}
