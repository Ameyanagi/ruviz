use std::collections::HashMap;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::core::{PlottingError, Result};
use crate::plots::SurfaceShading;
use crate::render::three_d::scene::{MeshColor3D, Scene3D, SceneGeometry3D};
use crate::render::{Color, MarkerStyle};

use super::pipelines::PipelineLibrary3D;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ResourceUpdate3D {
    pub(crate) vertex_upload_bytes: u64,
    pub(crate) index_upload_bytes: u64,
    pub(crate) texture_upload_bytes: u64,
    pub(crate) buffer_creations: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PointInstanceGpu {
    position: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LineInstanceGpu {
    start: [f32; 4],
    end: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MeshMaterialUniformGpu {
    color: [f32; 4],
    parameters: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LineMaterialUniformGpu {
    color: [f32; 4],
    parameters: [f32; 4],
    dash0: [f32; 4],
    dash1: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PointMaterialUniformGpu {
    color: [f32; 4],
    parameters: [f32; 4],
}

pub(super) struct PointGeometryGpu {
    pub(super) buffer: Option<wgpu::Buffer>,
    pub(super) instance_count: u32,
}

pub(super) struct LineGeometryGpu {
    pub(super) buffer: Option<wgpu::Buffer>,
    pub(super) instance_count: u32,
}

pub(super) struct MeshGeometryGpu {
    pub(super) vertex_buffer: Option<wgpu::Buffer>,
    pub(super) index_buffer: Option<wgpu::Buffer>,
    pub(super) index_count: u32,
}

pub(super) struct GeometryResources3D {
    _geometry: Arc<SceneGeometry3D>,
    pub(super) points: Vec<PointGeometryGpu>,
    pub(super) lines: Vec<LineGeometryGpu>,
    pub(super) meshes: Vec<MeshGeometryGpu>,
}

pub(super) struct MaterialGpu {
    _uniform: wgpu::Buffer,
    _texture: Option<wgpu::Texture>,
    _sampler: Option<wgpu::Sampler>,
    pub(super) bind_group: wgpu::BindGroup,
}

pub(super) struct AppearanceResources3D {
    _scene: Arc<Scene3D>,
    pub(super) points: Vec<MaterialGpu>,
    pub(super) lines: Vec<MaterialGpu>,
    pub(super) meshes: Vec<MaterialGpu>,
}

#[derive(Default)]
pub(super) struct ResourceCache3D {
    geometries: HashMap<usize, GeometryResources3D>,
    appearances: HashMap<usize, AppearanceResources3D>,
}

impl ResourceCache3D {
    pub(super) fn ensure(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pipelines: &PipelineLibrary3D,
        scene: &Arc<Scene3D>,
    ) -> Result<ResourceUpdate3D> {
        let geometry_key = arc_key(&scene.geometry);
        let mut update = ResourceUpdate3D::default();
        if !self.geometries.contains_key(&geometry_key) {
            let (resources, geometry_update) =
                create_geometry_resources(device, Arc::clone(&scene.geometry))?;
            self.geometries.clear();
            self.geometries.insert(geometry_key, resources);
            update.vertex_upload_bytes = update
                .vertex_upload_bytes
                .saturating_add(geometry_update.vertex_upload_bytes);
            update.index_upload_bytes = update
                .index_upload_bytes
                .saturating_add(geometry_update.index_upload_bytes);
            update.buffer_creations = update
                .buffer_creations
                .saturating_add(geometry_update.buffer_creations);
        }

        let appearance_key = arc_key(scene);
        if !self.appearances.contains_key(&appearance_key) {
            let (resources, appearance_update) =
                create_appearance_resources(device, queue, pipelines, Arc::clone(scene))?;
            self.appearances.clear();
            self.appearances.insert(appearance_key, resources);
            update.texture_upload_bytes = update
                .texture_upload_bytes
                .saturating_add(appearance_update.texture_upload_bytes);
            update.buffer_creations = update
                .buffer_creations
                .saturating_add(appearance_update.buffer_creations);
        }
        Ok(update)
    }

    pub(super) fn get(
        &self,
        scene: &Arc<Scene3D>,
    ) -> Result<(&GeometryResources3D, &AppearanceResources3D)> {
        let geometry = self
            .geometries
            .get(&arc_key(&scene.geometry))
            .ok_or_else(|| PlottingError::RenderError("missing retained 3d GPU geometry".into()))?;
        let appearance = self.appearances.get(&arc_key(scene)).ok_or_else(|| {
            PlottingError::RenderError("missing retained 3d GPU appearance".into())
        })?;
        Ok((geometry, appearance))
    }
}

fn create_geometry_resources(
    device: &wgpu::Device,
    geometry: Arc<SceneGeometry3D>,
) -> Result<(GeometryResources3D, ResourceUpdate3D)> {
    let mut update = ResourceUpdate3D::default();
    let mut points = Vec::with_capacity(geometry.points.len());
    for batch in &geometry.points {
        let instances: Vec<_> = batch
            .positions
            .iter()
            .map(|position| PointInstanceGpu {
                position: [position[0], position[1], position[2], 0.0],
            })
            .collect();
        let buffer = create_vertex_buffer(device, "ruviz 3d point instances", &instances);
        if buffer.is_some() {
            update.buffer_creations = update.buffer_creations.saturating_add(1);
            update.vertex_upload_bytes = update
                .vertex_upload_bytes
                .saturating_add(byte_len(&instances)?);
        }
        points.push(PointGeometryGpu {
            buffer,
            instance_count: checked_u32(instances.len(), "3d point instance count")?,
        });
    }

    let mut lines = Vec::with_capacity(geometry.lines.len());
    for batch in &geometry.lines {
        let mut instances = Vec::with_capacity(batch.segments.len());
        for &[start, end] in batch.segments.iter() {
            let start = batch.positions.get(start as usize).ok_or_else(|| {
                PlottingError::InvalidTopology3D {
                    reason: "3d GPU line references an out-of-range start vertex".to_string(),
                }
            })?;
            let end = batch.positions.get(end as usize).ok_or_else(|| {
                PlottingError::InvalidTopology3D {
                    reason: "3d GPU line references an out-of-range end vertex".to_string(),
                }
            })?;
            instances.push(LineInstanceGpu {
                start: [start[0], start[1], start[2], 0.0],
                end: [end[0], end[1], end[2], 0.0],
            });
        }
        let buffer = create_vertex_buffer(device, "ruviz 3d line instances", &instances);
        if buffer.is_some() {
            update.buffer_creations = update.buffer_creations.saturating_add(1);
            update.vertex_upload_bytes = update
                .vertex_upload_bytes
                .saturating_add(byte_len(&instances)?);
        }
        lines.push(LineGeometryGpu {
            buffer,
            instance_count: checked_u32(instances.len(), "3d line instance count")?,
        });
    }

    let mut meshes = Vec::with_capacity(geometry.meshes.len());
    for batch in &geometry.meshes {
        let vertex_buffer = if batch.vertices.is_empty() {
            None
        } else {
            update.buffer_creations = update.buffer_creations.saturating_add(1);
            update.vertex_upload_bytes = update
                .vertex_upload_bytes
                .saturating_add(byte_len(batch.vertices.as_ref())?);
            Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("ruviz 3d mesh vertices"),
                    contents: bytemuck::cast_slice(batch.vertices.as_ref()),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            )
        };
        let index_buffer = if batch.indices.is_empty() {
            None
        } else {
            update.buffer_creations = update.buffer_creations.saturating_add(1);
            update.index_upload_bytes = update
                .index_upload_bytes
                .saturating_add(byte_len(batch.indices.as_ref())?);
            Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("ruviz 3d mesh indices"),
                    contents: bytemuck::cast_slice(batch.indices.as_ref()),
                    usage: wgpu::BufferUsages::INDEX,
                }),
            )
        };
        meshes.push(MeshGeometryGpu {
            vertex_buffer,
            index_buffer,
            index_count: checked_u32(batch.indices.len(), "3d mesh index count")?,
        });
    }

    Ok((
        GeometryResources3D {
            _geometry: geometry,
            points,
            lines,
            meshes,
        },
        update,
    ))
}

fn create_appearance_resources(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipelines: &PipelineLibrary3D,
    scene: Arc<Scene3D>,
) -> Result<(AppearanceResources3D, ResourceUpdate3D)> {
    let mut update = ResourceUpdate3D::default();
    let mut points = Vec::with_capacity(scene.points.len());
    for batch in &scene.points {
        let uniform = PointMaterialUniformGpu {
            color: linear_color(batch.style.color),
            parameters: [
                batch.style.marker_size,
                marker_code(batch.style.marker) as f32,
                0.0,
                0.0,
            ],
        };
        points.push(create_uniform_material(
            device,
            "ruviz 3d point material",
            &pipelines.point_material_layout,
            &uniform,
        ));
        update.buffer_creations = update.buffer_creations.saturating_add(1);
    }

    let mut lines = Vec::with_capacity(scene.lines.len());
    for batch in &scene.lines {
        let mut dashes = [0.0_f32; 8];
        let pattern = batch.style.line_style.to_dash_array().unwrap_or_default();
        if pattern.len() > dashes.len() {
            return Err(PlottingError::UnsupportedGpuFeature(format!(
                "direct 3d GPU dash patterns support at most {} entries, got {}; use render() for the CPU reference",
                dashes.len(),
                pattern.len()
            )));
        }
        let dash_count = pattern.len().min(dashes.len());
        for (destination, source) in dashes.iter_mut().zip(pattern) {
            *destination = source;
        }
        let uniform = LineMaterialUniformGpu {
            color: linear_color(batch.style.color),
            parameters: [
                batch.style.line_width,
                dash_count as f32,
                dashes[..dash_count].iter().sum(),
                0.0,
            ],
            dash0: dashes[..4].try_into().unwrap_or([0.0; 4]),
            dash1: dashes[4..].try_into().unwrap_or([0.0; 4]),
        };
        lines.push(create_uniform_material(
            device,
            "ruviz 3d line material",
            &pipelines.line_material_layout,
            &uniform,
        ));
        update.buffer_creations = update.buffer_creations.saturating_add(1);
    }

    let mut meshes = Vec::with_capacity(scene.meshes.len());
    for batch in &scene.meshes {
        let (color, use_colormap, texels) = match &batch.style.color {
            MeshColor3D::Solid(color) => (
                linear_color(*color),
                0.0,
                vec![*color; COLORMAP_TEXEL_COUNT],
            ),
            MeshColor3D::Scalar { colormap, .. } => (
                [1.0; 4],
                1.0,
                (0..COLORMAP_TEXEL_COUNT)
                    .map(|index| colormap.sample(index as f64 / (COLORMAP_TEXEL_COUNT - 1) as f64))
                    .collect(),
            ),
        };
        let uniform = MeshMaterialUniformGpu {
            color,
            parameters: [
                shading_code(batch.style.shading) as f32,
                use_colormap,
                if batch.style.two_sided { 1.0 } else { 0.0 },
                0.0,
            ],
        };
        meshes.push(create_mesh_material(
            device,
            queue,
            &pipelines.mesh_material_layout,
            &uniform,
            &texels,
        ));
        update.buffer_creations = update.buffer_creations.saturating_add(1);
        update.texture_upload_bytes = update
            .texture_upload_bytes
            .saturating_add(COLORMAP_UPLOAD_BYTES);
    }

    Ok((
        AppearanceResources3D {
            _scene: scene,
            points,
            lines,
            meshes,
        },
        update,
    ))
}

const COLORMAP_TEXEL_COUNT: usize = 256;
const COLORMAP_UPLOAD_BYTES: u64 = (COLORMAP_TEXEL_COUNT * 4) as u64;

fn create_mesh_material(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    uniform: &MeshMaterialUniformGpu,
    texels: &[Color],
) -> MaterialGpu {
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("ruviz 3d mesh material"),
        contents: bytemuck::bytes_of(uniform),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ruviz 3d colormap"),
        size: wgpu::Extent3d {
            width: COLORMAP_TEXEL_COUNT as u32,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let bytes: Vec<_> = texels
        .iter()
        .flat_map(|color| [color.r, color.g, color.b, color.a])
        .collect();
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some((COLORMAP_TEXEL_COUNT * 4) as u32),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: COLORMAP_TEXEL_COUNT as u32,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("ruviz 3d colormap sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ruviz 3d mesh material bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    MaterialGpu {
        _uniform: uniform_buffer,
        _texture: Some(texture),
        _sampler: Some(sampler),
        bind_group,
    }
}

fn create_uniform_material<T: Pod>(
    device: &wgpu::Device,
    label: &'static str,
    layout: &wgpu::BindGroupLayout,
    uniform: &T,
) -> MaterialGpu {
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::bytes_of(uniform),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });
    MaterialGpu {
        _uniform: uniform_buffer,
        _texture: None,
        _sampler: None,
        bind_group,
    }
}

fn create_vertex_buffer<T: Pod>(
    device: &wgpu::Device,
    label: &'static str,
    values: &[T],
) -> Option<wgpu::Buffer> {
    (!values.is_empty()).then(|| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(values),
            usage: wgpu::BufferUsages::VERTEX,
        })
    })
}

fn linear_color(color: Color) -> [f32; 4] {
    [
        srgb_to_linear(f32::from(color.r) / 255.0),
        srgb_to_linear(f32::from(color.g) / 255.0),
        srgb_to_linear(f32::from(color.b) / 255.0),
        f32::from(color.a) / 255.0,
    ]
}

fn srgb_to_linear(channel: f32) -> f32 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

fn marker_code(marker: MarkerStyle) -> u32 {
    match marker {
        MarkerStyle::Circle => 0,
        MarkerStyle::Square => 1,
        MarkerStyle::Triangle => 2,
        MarkerStyle::TriangleDown => 3,
        MarkerStyle::Diamond => 4,
        MarkerStyle::Plus => 5,
        MarkerStyle::Cross => 6,
        MarkerStyle::Star => 7,
        MarkerStyle::CircleOpen => 8,
        MarkerStyle::SquareOpen => 9,
        MarkerStyle::TriangleOpen => 10,
        MarkerStyle::DiamondOpen => 11,
    }
}

fn shading_code(shading: SurfaceShading) -> u32 {
    match shading {
        SurfaceShading::Unlit => 0,
        SurfaceShading::Flat => 1,
        SurfaceShading::Smooth => 2,
    }
}

fn arc_key<T: ?Sized>(value: &Arc<T>) -> usize {
    Arc::as_ptr(value) as *const () as usize
}

fn byte_len<T>(values: &[T]) -> Result<u64> {
    let bytes = values.len().checked_mul(std::mem::size_of::<T>()).ok_or(
        PlottingError::GpuMemoryError {
            requested: usize::MAX,
            available: None,
        },
    )?;
    u64::try_from(bytes).map_err(|_| PlottingError::GpuMemoryError {
        requested: bytes,
        available: None,
    })
}

fn checked_u32(value: usize, context: &str) -> Result<u32> {
    u32::try_from(value).map_err(|_| PlottingError::InvalidTopology3D {
        reason: format!("{context} exceeds u32 indexing"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_codes_are_stable_and_distinct() {
        let markers = [
            MarkerStyle::Circle,
            MarkerStyle::Square,
            MarkerStyle::Triangle,
            MarkerStyle::TriangleDown,
            MarkerStyle::Diamond,
            MarkerStyle::Plus,
            MarkerStyle::Cross,
            MarkerStyle::Star,
            MarkerStyle::CircleOpen,
            MarkerStyle::SquareOpen,
            MarkerStyle::TriangleOpen,
            MarkerStyle::DiamondOpen,
        ];
        let mut codes: Vec<_> = markers.into_iter().map(marker_code).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), markers.len());
    }

    #[test]
    fn srgb_endpoints_are_preserved() {
        assert_eq!(srgb_to_linear(0.0), 0.0);
        assert_eq!(srgb_to_linear(1.0), 1.0);
    }
}
