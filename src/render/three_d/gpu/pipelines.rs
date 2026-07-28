use std::mem;

use super::context::{COLOR_FORMAT, DEPTH_FORMAT};

pub(super) struct PipelineLibrary3D {
    pub(super) camera_layout: wgpu::BindGroupLayout,
    pub(super) mesh_material_layout: wgpu::BindGroupLayout,
    pub(super) line_material_layout: wgpu::BindGroupLayout,
    pub(super) point_material_layout: wgpu::BindGroupLayout,
    pub(super) mesh: wgpu::RenderPipeline,
    pub(super) line: wgpu::RenderPipeline,
    pub(super) point: wgpu::RenderPipeline,
}

impl PipelineLibrary3D {
    pub(super) fn new(device: &wgpu::Device, sample_count: u32) -> Self {
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ruviz 3d camera layout"),
            entries: &[uniform_layout_entry(
                0,
                wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            )],
        });
        let mesh_material_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ruviz 3d mesh material layout"),
                entries: &[
                    uniform_layout_entry(0, wgpu::ShaderStages::FRAGMENT),
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let line_material_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ruviz 3d line material layout"),
                entries: &[uniform_layout_entry(
                    0,
                    wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                )],
            });
        let point_material_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ruviz 3d point material layout"),
                entries: &[uniform_layout_entry(
                    0,
                    wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                )],
            });

        let mesh = create_pipeline(
            device,
            "ruviz 3d mesh pipeline",
            include_str!("shaders/mesh.wgsl"),
            &[&camera_layout, &mesh_material_layout],
            &[mesh_vertex_layout()],
            wgpu::PrimitiveTopology::TriangleList,
            sample_count,
        );
        let line = create_pipeline(
            device,
            "ruviz 3d line pipeline",
            include_str!("shaders/line.wgsl"),
            &[&camera_layout, &line_material_layout],
            &[line_instance_layout()],
            wgpu::PrimitiveTopology::TriangleStrip,
            sample_count,
        );
        let point = create_pipeline(
            device,
            "ruviz 3d point pipeline",
            include_str!("shaders/point.wgsl"),
            &[&camera_layout, &point_material_layout],
            &[point_instance_layout()],
            wgpu::PrimitiveTopology::TriangleStrip,
            sample_count,
        );

        Self {
            camera_layout,
            mesh_material_layout,
            line_material_layout,
            point_material_layout,
            mesh,
            line,
            point,
        }
    }
}

fn uniform_layout_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn mesh_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x3,
        2 => Float32
    ];
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<crate::render::three_d::scene::MeshVertex3D>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ATTRIBUTES,
    }
}

fn line_instance_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4];
    wgpu::VertexBufferLayout {
        array_stride: 32,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &ATTRIBUTES,
    }
}

fn point_instance_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRIBUTES: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x4];
    wgpu::VertexBufferLayout {
        array_stride: 16,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &ATTRIBUTES,
    }
}

fn create_pipeline(
    device: &wgpu::Device,
    label: &'static str,
    shader_source: &'static str,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
    buffers: &[wgpu::VertexBufferLayout<'_>],
    topology: wgpu::PrimitiveTopology,
    sample_count: u32,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &bind_group_layouts
            .iter()
            .map(|layout| Some(*layout))
            .collect::<Vec<_>>(),
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: COLOR_FORMAT,
                // Every scene shader emits premultiplied colour, so translucent
                // fragments composite correctly and the MSAA resolve produces a
                // coverage-premultiplied value the readback can divide back out.
                // `blend: None` silently made partly covered silhouettes read as
                // straight alpha, haloing every edge.
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview_mask: None,
        cache: None,
    })
}
