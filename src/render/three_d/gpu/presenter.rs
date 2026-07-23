use std::collections::HashMap;
use std::mem;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
#[cfg(all(feature = "interactive-gpu", not(target_arch = "wasm32")))]
use winit::window::Window;

use crate::core::plot3d::layout::{
    Axis3Layout, LegendGlyph3D, OverlayLine3D, OverlayRect3D, OverlayText3D,
};
use crate::core::{FigureConfig, PlottingError, Result};
use crate::render::three_d::scene::Scene3D;
use crate::render::{Color, SkiaRenderer, Theme};

use super::context::{COLOR_FORMAT, GpuContext3D};
use super::renderer::{GpuFrameOutput3D, Wgpu3DRenderer};

const TEXTURE_SHADER: &str = include_str!("shaders/present_texture.wgsl");
const SOLID_SHADER: &str = include_str!("shaders/present_solid.wgsl");
const TEXT_PADDING: u32 = 2;
const PREFERRED_ATLAS_WIDTH: u32 = 2048;
const COLORBAR_SEGMENTS: usize = 64;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PresentationUpdate3D {
    pub(crate) vertex_upload_bytes: u64,
    pub(crate) texture_upload_bytes: u64,
    pub(crate) buffer_creations: u64,
    pub(crate) draw_calls: u64,
    pub(crate) surface_reconfigurations: u64,
}

#[cfg(all(feature = "interactive-gpu", not(target_arch = "wasm32")))]
pub(crate) struct PresentedFrame3D {
    pub(crate) scene: GpuFrameOutput3D,
    pub(crate) presentation: PresentationUpdate3D,
}

#[cfg(all(feature = "interactive-gpu", not(target_arch = "wasm32")))]
pub(crate) enum SurfacePresentOutcome3D {
    Presented(PresentedFrame3D),
    Skipped,
}

#[cfg(all(feature = "interactive-gpu", not(target_arch = "wasm32")))]
pub(crate) struct SurfacePresenter3D {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    configuration: wgpu::SurfaceConfiguration,
    presentation_format: wgpu::TextureFormat,
    renderer: Wgpu3DRenderer,
    compositor: PresentationCompositor3D,
    pending_surface_reconfigurations: u64,
}

#[cfg(all(feature = "interactive-gpu", not(target_arch = "wasm32")))]
impl SurfacePresenter3D {
    pub(crate) fn new(window: Arc<Window>, width: u32, height: u32) -> Result<Self> {
        validate_surface_dimensions(width, height)?;
        let (surface, configuration, presentation_format, renderer, compositor) =
            create_surface_stack(Arc::clone(&window), width, height)?;
        Ok(Self {
            window,
            surface,
            configuration,
            presentation_format,
            renderer,
            compositor,
            pending_surface_reconfigurations: 1,
        })
    }

    fn rebuild_gpu(&mut self) -> Result<()> {
        let (surface, configuration, presentation_format, renderer, compositor) =
            create_surface_stack(
                Arc::clone(&self.window),
                self.configuration.width,
                self.configuration.height,
            )?;
        self.surface = surface;
        self.configuration = configuration;
        self.presentation_format = presentation_format;
        self.renderer = renderer;
        self.compositor = compositor;
        self.pending_surface_reconfigurations =
            self.pending_surface_reconfigurations.saturating_add(1);
        Ok(())
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        validate_surface_dimensions(width, height)?;
        if self.configuration.width == width && self.configuration.height == height {
            return Ok(());
        }
        self.configuration.width = width;
        self.configuration.height = height;
        self.surface
            .configure(&self.renderer.context().device, &self.configuration);
        self.pending_surface_reconfigurations =
            self.pending_surface_reconfigurations.saturating_add(1);
        Ok(())
    }

    pub(crate) fn present(
        &mut self,
        scene: &Arc<Scene3D>,
        layout: &Axis3Layout,
        figure: &FigureConfig,
        theme: &Theme,
    ) -> Result<SurfacePresentOutcome3D> {
        self.resize(layout.canvas_width, layout.canvas_height)?;
        if self.renderer.context().is_lost() {
            self.rebuild_gpu()?;
            return Ok(SurfacePresentOutcome3D::Skipped);
        }
        let Some(acquired) = self.acquire_surface_texture()? else {
            return Ok(SurfacePresentOutcome3D::Skipped);
        };
        let surface_texture = acquired.texture;
        let scene_output = match self.renderer.render_to_texture(scene, layout, figure.dpi) {
            Ok(output) => output,
            Err(_) if self.renderer.context().is_lost() => {
                drop(surface_texture);
                self.rebuild_gpu()?;
                return Ok(SurfacePresentOutcome3D::Skipped);
            }
            Err(error) => return Err(error),
        };
        let target_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                label: Some("ruviz 3d sRGB surface view"),
                format: Some(self.presentation_format),
                ..Default::default()
            });
        let mut update = self.compositor.compose(
            self.renderer.context(),
            self.renderer.color_view()?,
            self.renderer.attachment_generation(),
            &target_view,
            layout,
            figure,
            theme,
        )?;
        update.surface_reconfigurations = self.pending_surface_reconfigurations;
        self.pending_surface_reconfigurations = 0;
        surface_texture.present();
        if acquired.reconfigure_after_present {
            self.surface
                .configure(&self.renderer.context().device, &self.configuration);
            update.surface_reconfigurations = update.surface_reconfigurations.saturating_add(1);
        }
        Ok(SurfacePresentOutcome3D::Presented(PresentedFrame3D {
            scene: scene_output,
            presentation: update,
        }))
    }

    fn acquire_surface_texture(&mut self) -> Result<Option<AcquiredSurfaceTexture3D>> {
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => Ok(Some(AcquiredSurfaceTexture3D {
                texture,
                reconfigure_after_present: false,
            })),
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                Ok(Some(AcquiredSurfaceTexture3D {
                    texture,
                    reconfigure_after_present: true,
                }))
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                Ok(None)
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface
                    .configure(&self.renderer.context().device, &self.configuration);
                self.pending_surface_reconfigurations =
                    self.pending_surface_reconfigurations.saturating_add(1);
                Ok(None)
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self
                    .renderer
                    .context()
                    .instance()
                    .create_surface(Arc::clone(&self.window))
                    .map_err(|error| PlottingError::GpuInitError {
                        backend: "surface".to_string(),
                        error: error.to_string(),
                    })?;
                self.surface
                    .configure(&self.renderer.context().device, &self.configuration);
                self.pending_surface_reconfigurations =
                    self.pending_surface_reconfigurations.saturating_add(1);
                Ok(None)
            }
            wgpu::CurrentSurfaceTexture::Validation => Err(PlottingError::RenderError(
                "direct 3d surface acquisition hit a validation error".to_string(),
            )),
        }
    }
}

#[cfg(all(feature = "interactive-gpu", not(target_arch = "wasm32")))]
fn create_surface_stack(
    window: Arc<Window>,
    width: u32,
    height: u32,
) -> Result<(
    wgpu::Surface<'static>,
    wgpu::SurfaceConfiguration,
    wgpu::TextureFormat,
    Wgpu3DRenderer,
    PresentationCompositor3D,
)> {
    let instance = GpuContext3D::create_instance();
    let surface = instance
        .create_surface(window)
        .map_err(|error| PlottingError::GpuInitError {
            backend: "surface".to_string(),
            error: error.to_string(),
        })?;
    let context = GpuContext3D::for_surface(instance, &surface)?;
    let capabilities = surface.get_capabilities(context.adapter());
    let formats = select_surface_format(&capabilities.formats)?;
    let mut configuration = surface
        .get_default_config(context.adapter(), width, height)
        .ok_or_else(|| {
            PlottingError::UnsupportedGpuFeature(
                "the selected adapter cannot present to this 3d window".to_string(),
            )
        })?;
    configuration.format = formats.surface;
    configuration.view_formats = if formats.view == formats.surface {
        Vec::new()
    } else {
        vec![formats.view]
    };
    configuration.present_mode = wgpu::PresentMode::AutoVsync;
    configuration.desired_maximum_frame_latency = 2;
    surface.configure(&context.device, &configuration);
    let renderer = Wgpu3DRenderer::from_context(context)?;
    let compositor = PresentationCompositor3D::new(&renderer.context().device, formats.view);
    Ok((surface, configuration, formats.view, renderer, compositor))
}

#[cfg(all(feature = "interactive-gpu", not(target_arch = "wasm32")))]
struct AcquiredSurfaceTexture3D {
    texture: wgpu::SurfaceTexture,
    reconfigure_after_present: bool,
}

#[cfg(all(feature = "interactive-gpu", not(target_arch = "wasm32")))]
fn validate_surface_dimensions(width: u32, height: u32) -> Result<()> {
    if width == 0 || height == 0 {
        Err(PlottingError::InvalidDimensions { width, height })
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SurfaceFormatSelection3D {
    pub(crate) surface: wgpu::TextureFormat,
    pub(crate) view: wgpu::TextureFormat,
}

pub(crate) fn select_surface_format(
    formats: &[wgpu::TextureFormat],
) -> Result<SurfaceFormatSelection3D> {
    formats
        .iter()
        .copied()
        .find_map(|surface| {
            let view = surface.add_srgb_suffix();
            view.is_srgb()
                .then_some(SurfaceFormatSelection3D { surface, view })
        })
        .ok_or_else(|| {
            PlottingError::UnsupportedGpuFeature(
                "direct 3d presentation requires an sRGB-compatible surface format".to_string(),
            )
        })
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SolidVertex {
    position: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TextureVertex {
    position: [f32; 2],
    uv: [f32; 2],
}

#[derive(Default)]
struct VertexBufferState {
    buffer: Option<wgpu::Buffer>,
    capacity: u64,
}

impl VertexBufferState {
    fn upload<T: Pod>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &'static str,
        vertices: &[T],
    ) -> (u64, u64) {
        if vertices.is_empty() {
            return (0, 0);
        }
        let bytes = bytemuck::cast_slice(vertices);
        let required = bytes.len() as u64;
        let mut creations = 0;
        if self.buffer.is_none() || self.capacity < required {
            self.capacity = required.next_power_of_two().max(256);
            self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: self.capacity,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            creations = 1;
        }
        if let Some(buffer) = &self.buffer {
            queue.write_buffer(buffer, 0, bytes);
        }
        (required, creations)
    }
}

pub(crate) struct PresentationCompositor3D {
    solid_pipeline: wgpu::RenderPipeline,
    texture_pipeline: wgpu::RenderPipeline,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    scene_buffer: wgpu::Buffer,
    background_buffer: VertexBufferState,
    foreground_buffer: VertexBufferState,
    text_buffer: VertexBufferState,
    text_atlas: Option<TextAtlas>,
    scene_bind_group: Option<(u64, wgpu::BindGroup)>,
}

impl PresentationCompositor3D {
    pub(crate) fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ruviz 3d presentation texture layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ruviz 3d presentation sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        let solid_pipeline = create_solid_pipeline(device, target_format);
        let texture_pipeline = create_texture_pipeline(device, target_format, &texture_layout);
        let scene_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ruviz 3d presentation scene vertices"),
            contents: bytemuck::cast_slice(&full_screen_texture_vertices()),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Self {
            solid_pipeline,
            texture_pipeline,
            texture_layout,
            sampler,
            scene_buffer,
            background_buffer: VertexBufferState::default(),
            foreground_buffer: VertexBufferState::default(),
            text_buffer: VertexBufferState::default(),
            text_atlas: None,
            scene_bind_group: None,
        }
    }

    pub(crate) fn compose(
        &mut self,
        context: &GpuContext3D,
        scene_view: &wgpu::TextureView,
        scene_generation: u64,
        target_view: &wgpu::TextureView,
        layout: &Axis3Layout,
        figure: &FigureConfig,
        theme: &Theme,
    ) -> Result<PresentationUpdate3D> {
        let background = background_vertices(layout, figure, theme);
        let foreground = foreground_vertices(layout, figure, theme);
        let texture_upload_bytes =
            self.ensure_text_atlas(&context.device, &context.queue, layout, figure, theme)?;
        let text = self.text_atlas.as_ref().map_or_else(Vec::new, |atlas| {
            text_vertices(layout, figure, theme, atlas)
        });

        let (background_bytes, background_creations) = self.background_buffer.upload(
            &context.device,
            &context.queue,
            "ruviz 3d presentation background vertices",
            &background,
        );
        let (foreground_bytes, foreground_creations) = self.foreground_buffer.upload(
            &context.device,
            &context.queue,
            "ruviz 3d presentation foreground vertices",
            &foreground,
        );
        let (text_bytes, text_creations) = self.text_buffer.upload(
            &context.device,
            &context.queue,
            "ruviz 3d presentation text vertices",
            &text,
        );

        if self
            .scene_bind_group
            .as_ref()
            .is_none_or(|(generation, _)| *generation != scene_generation)
        {
            let bind_group = context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("ruviz 3d presentation scene bind group"),
                    layout: &self.texture_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(scene_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                    ],
                });
            self.scene_bind_group = Some((scene_generation, bind_group));
        }
        let scene_bind_group = self
            .scene_bind_group
            .as_ref()
            .map(|(_, bind_group)| bind_group)
            .ok_or_else(|| {
                PlottingError::RenderError(
                    "direct 3d presentation scene bind group was not retained".to_string(),
                )
            })?;
        let mut encoder = context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ruviz 3d presentation encoder"),
            });
        let mut draw_calls = 0_u64;
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ruviz 3d presentation pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(color_to_wgpu(theme.background)),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            if !background.is_empty() {
                pass.set_pipeline(&self.solid_pipeline);
                if let Some(buffer) = &self.background_buffer.buffer {
                    pass.set_vertex_buffer(0, buffer.slice(..));
                    pass.draw(0..background.len() as u32, 0..1);
                    draw_calls = draw_calls.saturating_add(1);
                }
            }

            pass.set_pipeline(&self.texture_pipeline);
            pass.set_bind_group(0, scene_bind_group, &[]);
            pass.set_vertex_buffer(0, self.scene_buffer.slice(..));
            pass.draw(0..6, 0..1);
            draw_calls = draw_calls.saturating_add(1);

            if !foreground.is_empty() {
                pass.set_pipeline(&self.solid_pipeline);
                if let Some(buffer) = &self.foreground_buffer.buffer {
                    pass.set_vertex_buffer(0, buffer.slice(..));
                    pass.draw(0..foreground.len() as u32, 0..1);
                    draw_calls = draw_calls.saturating_add(1);
                }
            }

            if !text.is_empty()
                && let (Some(buffer), Some(atlas)) = (&self.text_buffer.buffer, &self.text_atlas)
            {
                pass.set_pipeline(&self.texture_pipeline);
                pass.set_bind_group(0, &atlas.bind_group, &[]);
                pass.set_vertex_buffer(0, buffer.slice(..));
                pass.draw(0..text.len() as u32, 0..1);
                draw_calls = draw_calls.saturating_add(1);
            }
        }
        context.queue.submit([encoder.finish()]);

        Ok(PresentationUpdate3D {
            vertex_upload_bytes: background_bytes
                .saturating_add(foreground_bytes)
                .saturating_add(text_bytes),
            texture_upload_bytes,
            buffer_creations: background_creations
                .saturating_add(foreground_creations)
                .saturating_add(text_creations),
            draw_calls,
            surface_reconfigurations: 0,
        })
    }

    fn ensure_text_atlas(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &Axis3Layout,
        figure: &FigureConfig,
        theme: &Theme,
    ) -> Result<u64> {
        let key = TextAtlasKey::from_layout(layout, figure, theme);
        if !text_atlas_key_changed(self.text_atlas.as_ref().map(|atlas| &atlas.key), &key) {
            return Ok(0);
        }
        if key.entries.is_empty() {
            self.text_atlas = None;
            return Ok(0);
        }
        let (atlas, uploaded) = TextAtlas::build(
            device,
            queue,
            &self.texture_layout,
            &self.sampler,
            key,
            theme,
        )?;
        self.text_atlas = Some(atlas);
        Ok(uploaded)
    }
}

fn text_atlas_key_changed(existing: Option<&TextAtlasKey>, requested: &TextAtlasKey) -> bool {
    existing != Some(requested)
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TextKey {
    text: String,
    size_bits: u32,
}

impl TextKey {
    fn new(text: &str, size: f32) -> Self {
        Self {
            text: text.to_string(),
            size_bits: size.to_bits(),
        }
    }

    fn size(&self) -> f32 {
        f32::from_bits(self.size_bits)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TextAtlasKey {
    entries: Vec<TextKey>,
    font_family: String,
    foreground: [u8; 4],
}

impl TextAtlasKey {
    fn from_layout(layout: &Axis3Layout, figure: &FigureConfig, theme: &Theme) -> Self {
        let mut entries = Vec::new();
        for (text, size) in text_specs(layout, figure, theme) {
            let key = TextKey::new(&text.text, size);
            if !entries.contains(&key) {
                entries.push(key);
            }
        }
        Self {
            entries,
            font_family: theme.font_family.clone(),
            foreground: [
                theme.foreground.r,
                theme.foreground.g,
                theme.foreground.b,
                theme.foreground.a,
            ],
        }
    }
}

#[derive(Clone, Copy)]
struct TextRegion {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    content_width: f32,
}

struct TextAtlas {
    key: TextAtlasKey,
    _texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    regions: HashMap<TextKey, TextRegion>,
    width: u32,
    height: u32,
}

impl TextAtlas {
    fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        key: TextAtlasKey,
        theme: &Theme,
    ) -> Result<(Self, u64)> {
        let mut transparent_theme = theme.clone();
        transparent_theme.background = Color::TRANSPARENT;
        let measuring = SkiaRenderer::new(1, 1, transparent_theme.clone())?;
        let max_dimension = device.limits().max_texture_dimension_2d;
        let mut measured = Vec::with_capacity(key.entries.len());
        for entry in &key.entries {
            let (width, height) = measuring.measure_text(&entry.text, entry.size())?;
            let cell_width =
                (width.ceil().max(1.0) as u32).saturating_add(TEXT_PADDING.saturating_mul(2));
            let cell_height = (height.ceil().max(entry.size()).max(1.0) as u32)
                .saturating_add(TEXT_PADDING.saturating_mul(2));
            if cell_width > max_dimension || cell_height > max_dimension {
                return Err(PlottingError::GpuMemoryError {
                    requested: usize::MAX,
                    available: Some(max_dimension as usize),
                });
            }
            measured.push((entry.clone(), width.max(1.0), cell_width, cell_height));
        }
        let widest = measured.iter().map(|entry| entry.2).max().unwrap_or(1);
        let row_limit = PREFERRED_ATLAS_WIDTH.min(max_dimension).max(widest);
        let mut cursor_x = 0_u32;
        let mut cursor_y = 0_u32;
        let mut row_height = 0_u32;
        let mut atlas_width = 1_u32;
        let mut placements = Vec::with_capacity(measured.len());
        for (entry, content_width, width, height) in measured {
            if cursor_x > 0 && cursor_x.saturating_add(width) > row_limit {
                cursor_x = 0;
                cursor_y = cursor_y.saturating_add(row_height);
                row_height = 0;
            }
            let region = TextRegion {
                x: cursor_x,
                y: cursor_y,
                width,
                height,
                content_width,
            };
            placements.push((entry, region));
            cursor_x = cursor_x.saturating_add(width);
            row_height = row_height.max(height);
            atlas_width = atlas_width.max(cursor_x);
        }
        let atlas_height = cursor_y.saturating_add(row_height).max(1);
        if atlas_height > max_dimension {
            return Err(PlottingError::GpuMemoryError {
                requested: u64::from(atlas_width)
                    .saturating_mul(u64::from(atlas_height))
                    .saturating_mul(4)
                    .try_into()
                    .unwrap_or(usize::MAX),
                available: Some(
                    u64::from(max_dimension)
                        .saturating_mul(u64::from(max_dimension))
                        .saturating_mul(4)
                        .try_into()
                        .unwrap_or(usize::MAX),
                ),
            });
        }

        let mut renderer = SkiaRenderer::new(atlas_width, atlas_height, transparent_theme)?;
        let mut regions = HashMap::with_capacity(placements.len());
        for (entry, region) in placements {
            renderer.draw_text(
                &entry.text,
                region.x.saturating_add(TEXT_PADDING) as f32,
                region.y.saturating_add(TEXT_PADDING) as f32,
                entry.size(),
                theme.foreground,
            )?;
            regions.insert(entry, region);
        }
        let image = renderer.into_image_demultiplied();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ruviz 3d presentation text atlas"),
            size: wgpu::Extent3d {
                width: atlas_width,
                height: atlas_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &image.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(atlas_width.saturating_mul(4)),
                rows_per_image: Some(atlas_height),
            },
            wgpu::Extent3d {
                width: atlas_width,
                height: atlas_height,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ruviz 3d presentation text atlas bind group"),
            layout: texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        let uploaded = image.pixels.len() as u64;
        Ok((
            Self {
                key,
                _texture: texture,
                bind_group,
                regions,
                width: atlas_width,
                height: atlas_height,
            },
            uploaded,
        ))
    }
}

fn background_vertices(
    layout: &Axis3Layout,
    figure: &FigureConfig,
    theme: &Theme,
) -> Vec<SolidVertex> {
    let mut vertices = Vec::new();
    for pane in &layout.panes {
        let color = linear_color(Color::new_rgba(
            theme.grid_color.r,
            theme.grid_color.g,
            theme.grid_color.b,
            28,
        ));
        push_solid_triangle(&mut vertices, pane[0], pane[1], pane[2], color, layout);
        push_solid_triangle(&mut vertices, pane[0], pane[2], pane[3], color, layout);
    }
    let grid_color = linear_color(Color::new_rgba(
        theme.grid_color.r,
        theme.grid_color.g,
        theme.grid_color.b,
        110,
    ));
    let grid_width = (0.45 * figure.dpi / 72.0).max(0.5);
    for line in &layout.grid_lines {
        push_solid_line(&mut vertices, *line, grid_width, grid_color, layout);
    }
    vertices
}

fn foreground_vertices(
    layout: &Axis3Layout,
    figure: &FigureConfig,
    theme: &Theme,
) -> Vec<SolidVertex> {
    let mut vertices = Vec::new();
    let color = linear_color(theme.foreground);
    let width = (0.8 * figure.dpi / 72.0).max(0.75);
    for line in layout.box_edges.iter().chain(&layout.tick_marks) {
        push_solid_line(&mut vertices, *line, width, color, layout);
    }
    if let Some(legend) = &layout.legend {
        push_solid_rect(
            &mut vertices,
            legend.bounds,
            linear_color(theme.background),
            layout,
        );
        push_solid_rect_outline(
            &mut vertices,
            legend.bounds,
            width,
            linear_color(theme.grid_color),
            layout,
        );
        for item in &legend.items {
            match item.glyph {
                LegendGlyph3D::Marker => {
                    let size = item.glyph_rect.height.min(item.glyph_rect.width) * 0.72;
                    push_solid_rect(
                        &mut vertices,
                        OverlayRect3D {
                            x: item.glyph_rect.x + (item.glyph_rect.width - size) * 0.5,
                            y: item.glyph_rect.y + (item.glyph_rect.height - size) * 0.5,
                            width: size,
                            height: size,
                        },
                        linear_color(item.color),
                        layout,
                    );
                }
                LegendGlyph3D::Line => push_solid_line(
                    &mut vertices,
                    OverlayLine3D {
                        start: glam::Vec2::new(
                            item.glyph_rect.x,
                            item.glyph_rect.y + item.glyph_rect.height * 0.5,
                        ),
                        end: glam::Vec2::new(
                            item.glyph_rect.right(),
                            item.glyph_rect.y + item.glyph_rect.height * 0.5,
                        ),
                    },
                    width.max(1.5),
                    linear_color(item.color),
                    layout,
                ),
                LegendGlyph3D::Fill => push_solid_rect(
                    &mut vertices,
                    item.glyph_rect,
                    linear_color(item.color),
                    layout,
                ),
            }
        }
    }
    for colorbar in &layout.colorbars {
        let segment_height = colorbar.bounds.height / COLORBAR_SEGMENTS as f32;
        for index in 0..COLORBAR_SEGMENTS {
            let normalized = 1.0 - index as f64 / COLORBAR_SEGMENTS.saturating_sub(1).max(1) as f64;
            push_solid_rect(
                &mut vertices,
                OverlayRect3D {
                    x: colorbar.bounds.x,
                    y: colorbar.bounds.y + index as f32 * segment_height,
                    width: colorbar.bounds.width,
                    height: segment_height + 0.5,
                },
                linear_color(colorbar.colormap.sample(normalized)),
                layout,
            );
        }
        push_solid_rect_outline(&mut vertices, colorbar.bounds, width, color, layout);
        for line in &colorbar.tick_marks {
            push_solid_line(&mut vertices, *line, width, color, layout);
        }
    }
    vertices
}

fn push_solid_rect(
    output: &mut Vec<SolidVertex>,
    rect: OverlayRect3D,
    color: [f32; 4],
    layout: &Axis3Layout,
) {
    let top_left = glam::Vec2::new(rect.x, rect.y);
    let top_right = glam::Vec2::new(rect.right(), rect.y);
    let bottom_right = glam::Vec2::new(rect.right(), rect.bottom());
    let bottom_left = glam::Vec2::new(rect.x, rect.bottom());
    push_solid_triangle(output, top_left, top_right, bottom_right, color, layout);
    push_solid_triangle(output, top_left, bottom_right, bottom_left, color, layout);
}

fn push_solid_rect_outline(
    output: &mut Vec<SolidVertex>,
    rect: OverlayRect3D,
    width: f32,
    color: [f32; 4],
    layout: &Axis3Layout,
) {
    let top_left = glam::Vec2::new(rect.x, rect.y);
    let top_right = glam::Vec2::new(rect.right(), rect.y);
    let bottom_right = glam::Vec2::new(rect.right(), rect.bottom());
    let bottom_left = glam::Vec2::new(rect.x, rect.bottom());
    for line in [
        OverlayLine3D {
            start: top_left,
            end: top_right,
        },
        OverlayLine3D {
            start: top_right,
            end: bottom_right,
        },
        OverlayLine3D {
            start: bottom_right,
            end: bottom_left,
        },
        OverlayLine3D {
            start: bottom_left,
            end: top_left,
        },
    ] {
        push_solid_line(output, line, width, color, layout);
    }
}

fn push_solid_triangle(
    output: &mut Vec<SolidVertex>,
    a: glam::Vec2,
    b: glam::Vec2,
    c: glam::Vec2,
    color: [f32; 4],
    layout: &Axis3Layout,
) {
    for point in [a, b, c] {
        output.push(SolidVertex {
            position: to_ndc(point.x, point.y, layout),
            color,
        });
    }
}

fn push_solid_line(
    output: &mut Vec<SolidVertex>,
    line: OverlayLine3D,
    width: f32,
    color: [f32; 4],
    layout: &Axis3Layout,
) {
    let direction = line.end - line.start;
    if !direction.is_finite() || direction.length_squared() <= f32::EPSILON {
        return;
    }
    let normal = glam::Vec2::new(-direction.y, direction.x).normalize() * (width * 0.5);
    let corners = [
        line.start - normal,
        line.start + normal,
        line.end + normal,
        line.end - normal,
    ];
    for index in [0, 1, 2, 0, 2, 3] {
        let point = corners[index];
        output.push(SolidVertex {
            position: to_ndc(point.x, point.y, layout),
            color,
        });
    }
}

fn text_vertices(
    layout: &Axis3Layout,
    figure: &FigureConfig,
    theme: &Theme,
    atlas: &TextAtlas,
) -> Vec<TextureVertex> {
    let mut output = Vec::new();
    for (text, size) in text_specs(layout, figure, theme) {
        let key = TextKey::new(&text.text, size);
        let Some(region) = atlas.regions.get(&key).copied() else {
            continue;
        };
        let (anchor_x, top_y) = clamped_text_position(text, size, layout);
        let content_left = if text.centered {
            anchor_x - region.content_width * 0.5
        } else {
            anchor_x
        };
        let left = content_left - TEXT_PADDING as f32;
        let top = top_y - TEXT_PADDING as f32;
        let right = left + region.width as f32;
        let bottom = top + region.height as f32;
        let u0 = region.x as f32 / atlas.width as f32;
        let v0 = region.y as f32 / atlas.height as f32;
        let u1 = region.x.saturating_add(region.width) as f32 / atlas.width as f32;
        let v1 = region.y.saturating_add(region.height) as f32 / atlas.height as f32;
        push_texture_quad(
            &mut output,
            [left, top, right, bottom],
            [u0, v0, u1, v1],
            layout,
        );
    }
    output
}

fn text_specs<'a>(
    layout: &'a Axis3Layout,
    figure: &FigureConfig,
    theme: &Theme,
) -> Vec<(&'a OverlayText3D, f32)> {
    let dpi_scale = figure.dpi / 72.0;
    let mut specs = Vec::with_capacity(
        layout.tick_labels.len()
            + layout.axis_labels.len()
            + usize::from(layout.title.is_some())
            + layout
                .legend
                .as_ref()
                .map_or(0, |legend| legend.items.len())
            + layout
                .colorbars
                .iter()
                .map(|colorbar| colorbar.tick_labels.len())
                .sum::<usize>(),
    );
    specs.extend(
        layout
            .tick_labels
            .iter()
            .map(|text| (text, theme.tick_label_font_size * dpi_scale)),
    );
    specs.extend(
        layout
            .axis_labels
            .iter()
            .map(|text| (text, theme.axis_label_font_size * dpi_scale)),
    );
    if let Some(title) = &layout.title {
        specs.push((title, theme.title_font_size * dpi_scale));
    }
    if let Some(legend) = &layout.legend {
        specs.extend(
            legend
                .items
                .iter()
                .map(|item| (&item.label, theme.legend_font_size * dpi_scale)),
        );
    }
    specs.extend(layout.colorbars.iter().flat_map(|colorbar| {
        colorbar
            .tick_labels
            .iter()
            .map(|text| (text, theme.tick_label_font_size * dpi_scale))
    }));
    specs
}

fn clamped_text_position(text: &OverlayText3D, font_size: f32, layout: &Axis3Layout) -> (f32, f32) {
    let x = text
        .position
        .x
        .clamp(2.0, layout.canvas_width.saturating_sub(2) as f32);
    let y = (text.position.y - font_size * 0.5)
        .clamp(0.0, (layout.canvas_height as f32 - font_size).max(0.0));
    (x, y)
}

fn full_screen_texture_vertices() -> [TextureVertex; 6] {
    [
        TextureVertex {
            position: [-1.0, 1.0],
            uv: [0.0, 0.0],
        },
        TextureVertex {
            position: [1.0, 1.0],
            uv: [1.0, 0.0],
        },
        TextureVertex {
            position: [1.0, -1.0],
            uv: [1.0, 1.0],
        },
        TextureVertex {
            position: [-1.0, 1.0],
            uv: [0.0, 0.0],
        },
        TextureVertex {
            position: [1.0, -1.0],
            uv: [1.0, 1.0],
        },
        TextureVertex {
            position: [-1.0, -1.0],
            uv: [0.0, 1.0],
        },
    ]
}

fn push_texture_quad(
    output: &mut Vec<TextureVertex>,
    rect: [f32; 4],
    uv: [f32; 4],
    layout: &Axis3Layout,
) {
    let [left, top, right, bottom] = rect;
    let [u0, v0, u1, v1] = uv;
    let vertices = [
        (left, top, u0, v0),
        (right, top, u1, v0),
        (right, bottom, u1, v1),
        (left, top, u0, v0),
        (right, bottom, u1, v1),
        (left, bottom, u0, v1),
    ];
    output.extend(vertices.map(|(x, y, u, v)| TextureVertex {
        position: to_ndc(x, y, layout),
        uv: [u, v],
    }));
}

fn to_ndc(x: f32, y: f32, layout: &Axis3Layout) -> [f32; 2] {
    [
        x / layout.canvas_width as f32 * 2.0 - 1.0,
        1.0 - y / layout.canvas_height as f32 * 2.0,
    ]
}

fn color_to_wgpu(color: Color) -> wgpu::Color {
    let [r, g, b, a] = linear_color(color);
    wgpu::Color {
        r: f64::from(r),
        g: f64::from(g),
        b: f64::from(b),
        a: f64::from(a),
    }
}

fn linear_color(color: Color) -> [f32; 4] {
    let convert = |value: u8| {
        let value = f32::from(value) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    [
        convert(color.r),
        convert(color.g),
        convert(color.b),
        f32::from(color.a) / 255.0,
    ]
}

fn create_solid_pipeline(
    device: &wgpu::Device,
    target_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ruviz 3d presentation solid shader"),
        source: wgpu::ShaderSource::Wgsl(SOLID_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("ruviz 3d presentation solid pipeline layout"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("ruviz 3d presentation solid pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: mem::size_of::<SolidVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &ATTRIBUTES,
            }],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

fn create_texture_pipeline(
    device: &wgpu::Device,
    target_format: wgpu::TextureFormat,
    texture_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ruviz 3d presentation texture shader"),
        source: wgpu::ShaderSource::Wgsl(TEXTURE_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("ruviz 3d presentation texture pipeline layout"),
        bind_group_layouts: &[Some(texture_layout)],
        immediate_size: 0,
    });
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("ruviz 3d presentation texture pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: mem::size_of::<TextureVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &ATTRIBUTES,
            }],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use crate::core::plot3d::layout::Colorbar3D;
    use crate::render::ColorMap;
    use crate::scatter3d;

    use super::*;

    #[test]
    fn camera_layout_generates_bounded_direct_overlay_geometry() {
        let frame = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .title("direct")
            .xlabel("x")
            .ylabel("y")
            .zlabel("z")
            .finalize()
            .resolve()
            .expect("frame");
        let layout = Axis3Layout::resolve(&frame).expect("layout");
        let background = background_vertices(&layout, &frame.figure, &frame.theme);
        let foreground = foreground_vertices(&layout, &frame.figure, &frame.theme);
        assert!(!background.is_empty());
        assert!(!foreground.is_empty());
        assert!(background.iter().chain(&foreground).all(|vertex| {
            vertex.position[0].is_finite()
                && vertex.position[1].is_finite()
                && vertex.color.iter().all(|component| component.is_finite())
        }));
    }

    #[test]
    fn texture_quads_keep_top_left_texture_orientation() {
        let vertices = full_screen_texture_vertices();
        assert_eq!(vertices[0].position, [-1.0, 1.0]);
        assert_eq!(vertices[0].uv, [0.0, 0.0]);
        assert_eq!(vertices[2].position, [1.0, -1.0]);
        assert_eq!(vertices[2].uv, [1.0, 1.0]);
    }

    #[test]
    fn direct_colorbar_uses_solid_vertices_instead_of_a_texture() {
        let frame = scatter3d(&[0.0], &[0.0], &[0.0])
            .finalize()
            .resolve()
            .expect("frame");
        let mut layout = Axis3Layout::resolve(&frame).expect("layout");
        let baseline = foreground_vertices(&layout, &frame.figure, &frame.theme).len();
        layout.colorbars.push(Colorbar3D {
            bounds: OverlayRect3D {
                x: 550.0,
                y: 80.0,
                width: 14.0,
                height: 240.0,
            },
            colormap: ColorMap::viridis(),
            data_range: (0.0, 1.0),
            tick_marks: Vec::new(),
            tick_labels: Vec::new(),
        });
        let foreground = foreground_vertices(&layout, &frame.figure, &frame.theme);
        assert!(foreground.len() >= baseline + COLORBAR_SEGMENTS * 6);
        assert!(foreground.iter().all(|vertex| {
            vertex
                .position
                .iter()
                .all(|component| component.is_finite())
                && vertex.color.iter().all(|component| component.is_finite())
        }));
    }

    #[test]
    fn surface_format_selection_uses_preferred_base_with_srgb_view() {
        assert_eq!(
            select_surface_format(&[
                wgpu::TextureFormat::Bgra8Unorm,
                wgpu::TextureFormat::Bgra8UnormSrgb,
            ])
            .expect("sRGB"),
            SurfaceFormatSelection3D {
                surface: wgpu::TextureFormat::Bgra8Unorm,
                view: wgpu::TextureFormat::Bgra8UnormSrgb,
            }
        );
        assert_eq!(
            select_surface_format(&[
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureFormat::Bgra8Unorm,
                wgpu::TextureFormat::Rgba16Float,
            ])
            .expect("WebGPU"),
            SurfaceFormatSelection3D {
                surface: wgpu::TextureFormat::Rgba8Unorm,
                view: wgpu::TextureFormat::Rgba8UnormSrgb,
            }
        );
        assert!(select_surface_format(&[wgpu::TextureFormat::Rgba16Float]).is_err());
    }

    #[test]
    fn camera_only_layout_change_reuses_the_text_atlas_key() {
        let first = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .label("terrain")
            .xlabel("x")
            .ylabel("y")
            .zlabel("z")
            .azimuth_deg(-60.0)
            .finalize()
            .resolve()
            .expect("first");
        let second = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .label("terrain")
            .xlabel("x")
            .ylabel("y")
            .zlabel("z")
            .azimuth_deg(20.0)
            .finalize()
            .resolve()
            .expect("second");
        let first_layout = Axis3Layout::resolve(&first).expect("first layout");
        let second_layout = Axis3Layout::resolve(&second).expect("second layout");
        assert_ne!(first_layout.box_edges, second_layout.box_edges);
        let first_key = TextAtlasKey::from_layout(&first_layout, &first.figure, &first.theme);
        assert!(
            first_key
                .entries
                .iter()
                .any(|entry| entry.text == "terrain")
        );
        assert_eq!(
            first_key,
            TextAtlasKey::from_layout(&second_layout, &second.figure, &second.theme)
        );
        let second_key = TextAtlasKey::from_layout(&second_layout, &second.figure, &second.theme);
        assert!(
            !text_atlas_key_changed(Some(&first_key), &second_key),
            "camera-only decoration changes must report zero warm text-atlas upload bytes"
        );
    }
}
