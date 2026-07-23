use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Mutex, OnceLock};

use bytemuck::{Pod, Zeroable};
use futures_intrusive::channel::shared::oneshot_channel;
use wgpu::util::DeviceExt;

use crate::core::plot::Image;
use crate::core::plot3d::layout::Axis3Layout;
use crate::core::{PlottingError, Result};
use crate::render::three_d::scene::Scene3D;

#[cfg(not(target_arch = "wasm32"))]
use super::context::validate_format;
use super::context::{COLOR_FORMAT, DEPTH_FORMAT, GpuContext3D};
use super::pipelines::PipelineLibrary3D;
use super::resources::{ResourceCache3D, ResourceUpdate3D};

const COPY_ROW_ALIGNMENT: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

#[derive(Debug)]
pub(crate) struct GpuRenderOutput3D {
    pub(crate) layer: Image,
    pub(crate) draw_calls: u64,
    pub(crate) readback_bytes: u64,
    pub(crate) resource_update: ResourceUpdate3D,
    pub(crate) camera_uniform_writes: u64,
    pub(crate) adapter_name: String,
    pub(crate) sample_count: u32,
}

#[derive(Debug)]
pub(crate) struct GpuFrameOutput3D {
    pub(crate) draw_calls: u64,
    pub(crate) resource_update: ResourceUpdate3D,
    pub(crate) camera_uniform_writes: u64,
    pub(crate) adapter_name: String,
    pub(crate) sample_count: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniformGpu {
    view_projection: [[f32; 4]; 4],
    axis_aspect: [f32; 4],
    viewport: [f32; 4],
}

pub(crate) struct Wgpu3DRenderer {
    context: GpuContext3D,
    pipelines: PipelineLibrary3D,
    resources: ResourceCache3D,
    attachments: Option<OffscreenAttachments3D>,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    initial_buffer_creations: u64,
    attachment_generation: u64,
    readback_enabled: bool,
}

#[cfg(not(target_arch = "wasm32"))]
static SHARED_RENDERER: OnceLock<Mutex<Option<Wgpu3DRenderer>>> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
fn with_locked_try_init<T, E, Output>(
    slot: &OnceLock<Mutex<Option<T>>>,
    initialize: impl FnOnce() -> std::result::Result<T, E>,
    lock_error: impl Fn() -> E,
    operation: impl FnOnce(&mut T) -> std::result::Result<Output, E>,
) -> std::result::Result<Output, E> {
    let mut slot = slot
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| lock_error())?;
    if slot.is_none() {
        *slot = Some(initialize()?);
    }
    let value = slot.as_mut().ok_or_else(lock_error)?;
    operation(value)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn render_with_shared_renderer(
    scene: &Arc<Scene3D>,
    layout: &Axis3Layout,
    dpi: f32,
) -> Result<GpuRenderOutput3D> {
    with_locked_try_init(
        &SHARED_RENDERER,
        Wgpu3DRenderer::new,
        || PlottingError::RenderError("direct 3d GPU renderer lock was poisoned".to_string()),
        |renderer| renderer.render_to_image(scene, layout, dpi),
    )
}

impl Wgpu3DRenderer {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn new() -> Result<Self> {
        let context = GpuContext3D::new()?;
        Self::from_context_with_readback(context)
    }

    pub(crate) fn from_context(context: GpuContext3D) -> Result<Self> {
        Self::from_context_internal(context, false)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn from_context_with_readback(context: GpuContext3D) -> Result<Self> {
        validate_format(
            context.adapter(),
            COLOR_FORMAT,
            wgpu::TextureUsages::COPY_SRC,
        )?;
        Self::from_context_internal(context, true)
    }

    fn from_context_internal(context: GpuContext3D, readback_enabled: bool) -> Result<Self> {
        let pipelines = PipelineLibrary3D::new(&context.device, context.sample_count);
        let camera_buffer = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ruviz 3d camera uniform"),
                contents: bytemuck::bytes_of(&CameraUniformGpu::zeroed()),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let camera_bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ruviz 3d camera bind group"),
                layout: &pipelines.camera_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }],
            });
        Ok(Self {
            context,
            pipelines,
            resources: ResourceCache3D::default(),
            attachments: None,
            camera_buffer,
            camera_bind_group,
            initial_buffer_creations: 1,
            attachment_generation: 0,
            readback_enabled,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn render_to_image(
        &mut self,
        scene: &Arc<Scene3D>,
        layout: &Axis3Layout,
        dpi: f32,
    ) -> Result<GpuRenderOutput3D> {
        let (layer, frame) = self.render_internal(scene, layout, dpi, true, true)?;
        let layer = layer.ok_or_else(|| {
            PlottingError::RenderError("direct 3d GPU export produced no image".to_string())
        })?;
        let attachments = self.attachments.as_ref().ok_or_else(|| {
            PlottingError::RenderError("missing direct 3d GPU attachments".to_string())
        })?;
        Ok(GpuRenderOutput3D {
            layer,
            draw_calls: frame.draw_calls,
            readback_bytes: u64::from(attachments.padded_bytes_per_row)
                .saturating_mul(u64::from(layout.canvas_height)),
            resource_update: frame.resource_update,
            camera_uniform_writes: frame.camera_uniform_writes,
            adapter_name: frame.adapter_name,
            sample_count: frame.sample_count,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn render_without_readback(
        &mut self,
        scene: &Arc<Scene3D>,
        layout: &Axis3Layout,
        dpi: f32,
    ) -> Result<GpuFrameOutput3D> {
        self.render_internal(scene, layout, dpi, false, true)
            .map(|(_, frame)| frame)
    }

    pub(crate) fn render_to_texture(
        &mut self,
        scene: &Arc<Scene3D>,
        layout: &Axis3Layout,
        dpi: f32,
    ) -> Result<GpuFrameOutput3D> {
        if self.context.is_lost() {
            return Err(PlottingError::GpuNotAvailable(
                "the direct 3d surface device was lost".to_string(),
            ));
        }
        self.render_internal(scene, layout, dpi, false, false)
            .map(|(_, frame)| frame)
    }

    pub(crate) fn context(&self) -> &GpuContext3D {
        &self.context
    }

    pub(crate) fn color_view(&self) -> Result<&wgpu::TextureView> {
        self.attachments
            .as_ref()
            .map(|attachments| &attachments.color_view)
            .ok_or_else(|| {
                PlottingError::RenderError(
                    "direct 3d presentation has no resolved color attachment".to_string(),
                )
            })
    }

    pub(crate) const fn attachment_generation(&self) -> u64 {
        self.attachment_generation
    }

    fn render_internal(
        &mut self,
        scene: &Arc<Scene3D>,
        layout: &Axis3Layout,
        dpi: f32,
        readback: bool,
        wait_for_completion: bool,
    ) -> Result<(Option<Image>, GpuFrameOutput3D)> {
        if self.context.is_lost() {
            #[cfg(not(target_arch = "wasm32"))]
            {
                *self = Self::new()?;
            }
            #[cfg(target_arch = "wasm32")]
            {
                return Err(PlottingError::GpuNotAvailable(
                    "the direct 3d WebGPU device was lost".to_string(),
                ));
            }
        }
        self.context.ensure_available()?;
        validate_dimensions(
            layout.canvas_width,
            layout.canvas_height,
            self.context.device.limits().max_texture_dimension_2d,
        )?;

        let mut resource_update = self.resources.ensure(
            &self.context.device,
            &self.context.queue,
            &self.pipelines,
            scene,
        )?;
        resource_update.buffer_creations = resource_update
            .buffer_creations
            .saturating_add(self.initial_buffer_creations);
        self.initial_buffer_creations = 0;

        let needs_attachments = self.attachments.as_ref().is_none_or(|attachments| {
            attachments.width != layout.canvas_width
                || attachments.height != layout.canvas_height
                || attachments.sample_count != self.context.sample_count
        });
        if needs_attachments {
            let attachments = OffscreenAttachments3D::new(
                &self.context.device,
                layout.canvas_width,
                layout.canvas_height,
                self.context.sample_count,
                self.readback_enabled,
            )?;
            resource_update.buffer_creations = resource_update
                .buffer_creations
                .saturating_add(attachments.creation_count);
            self.attachments = Some(attachments);
            self.attachment_generation =
                self.attachment_generation.checked_add(1).ok_or_else(|| {
                    PlottingError::RenderError(
                        "direct 3d attachment generation space was exhausted".to_string(),
                    )
                })?;
        }

        let camera = CameraUniformGpu {
            view_projection: layout.camera.view_projection.to_cols_array_2d(),
            axis_aspect: layout.camera.axis_aspect.extend(0.0).to_array(),
            viewport: [
                layout.viewport.width as f32,
                layout.viewport.height as f32,
                dpi,
                0.0,
            ],
        };
        self.context
            .queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&camera));

        let (geometry, appearance) = self.resources.get(scene)?;
        let attachments = self.attachments.as_ref().ok_or_else(|| {
            PlottingError::RenderError("missing direct 3d GPU attachments".to_string())
        })?;
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("ruviz direct 3d render encoder"),
                });
        let color_attachment = wgpu::RenderPassColorAttachment {
            view: attachments.render_color_view(),
            depth_slice: None,
            resolve_target: attachments.resolve_target(),
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        };
        let mut draw_calls = 0_u64;
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ruviz direct 3d render pass"),
                color_attachments: &[Some(color_attachment)],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &attachments.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_viewport(
                layout.viewport.x as f32,
                layout.viewport.y as f32,
                layout.viewport.width as f32,
                layout.viewport.height as f32,
                0.0,
                1.0,
            );
            pass.set_scissor_rect(
                layout.viewport.x,
                layout.viewport.y,
                layout.viewport.width,
                layout.viewport.height,
            );
            pass.set_bind_group(0, &self.camera_bind_group, &[]);

            pass.set_pipeline(&self.pipelines.mesh);
            for (geometry, material) in geometry.meshes.iter().zip(&appearance.meshes) {
                if geometry.index_count == 0 {
                    continue;
                }
                let (Some(vertex_buffer), Some(index_buffer)) =
                    (&geometry.vertex_buffer, &geometry.index_buffer)
                else {
                    return Err(PlottingError::RenderError(
                        "non-empty 3d mesh is missing retained GPU buffers".to_string(),
                    ));
                };
                pass.set_bind_group(1, &material.bind_group, &[]);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..geometry.index_count, 0, 0..1);
                draw_calls = draw_calls.saturating_add(1);
            }

            pass.set_pipeline(&self.pipelines.line);
            for (geometry, material) in geometry.lines.iter().zip(&appearance.lines) {
                if geometry.instance_count == 0 {
                    continue;
                }
                let Some(buffer) = &geometry.buffer else {
                    return Err(PlottingError::RenderError(
                        "non-empty 3d line batch is missing a retained GPU buffer".to_string(),
                    ));
                };
                pass.set_bind_group(1, &material.bind_group, &[]);
                pass.set_vertex_buffer(0, buffer.slice(..));
                pass.draw(0..4, 0..geometry.instance_count);
                draw_calls = draw_calls.saturating_add(1);
            }

            pass.set_pipeline(&self.pipelines.point);
            for (geometry, material) in geometry.points.iter().zip(&appearance.points) {
                if geometry.instance_count == 0 {
                    continue;
                }
                let Some(buffer) = &geometry.buffer else {
                    return Err(PlottingError::RenderError(
                        "non-empty 3d point batch is missing a retained GPU buffer".to_string(),
                    ));
                };
                pass.set_bind_group(1, &material.bind_group, &[]);
                pass.set_vertex_buffer(0, buffer.slice(..));
                pass.draw(0..4, 0..geometry.instance_count);
                draw_calls = draw_calls.saturating_add(1);
            }
        }

        if readback {
            let readback_buffer = attachments.readback.as_ref().ok_or_else(|| {
                PlottingError::RenderError(
                    "direct 3d renderer was created without readback support".to_string(),
                )
            })?;
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &attachments.color,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: readback_buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(attachments.padded_bytes_per_row),
                        rows_per_image: Some(layout.canvas_height),
                    },
                },
                wgpu::Extent3d {
                    width: layout.canvas_width,
                    height: layout.canvas_height,
                    depth_or_array_layers: 1,
                },
            );
        }
        let submission = self.context.queue.submit([encoder.finish()]);
        let layer = if readback {
            Some(readback_image(
                &self.context.device,
                attachments.readback.as_ref().ok_or_else(|| {
                    PlottingError::RenderError(
                        "direct 3d renderer was created without readback support".to_string(),
                    )
                })?,
                submission,
                layout.canvas_width,
                layout.canvas_height,
                attachments.padded_bytes_per_row,
            )?)
        } else if wait_for_completion {
            self.context
                .device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: None,
                })
                .map_err(|error| {
                    PlottingError::RenderError(format!("3d GPU poll failed: {error}"))
                })?;
            None
        } else {
            None
        };
        self.context.ensure_available()?;

        Ok((
            layer,
            GpuFrameOutput3D {
                draw_calls,
                resource_update,
                camera_uniform_writes: 1,
                adapter_name: self.context.adapter_name.clone(),
                sample_count: self.context.sample_count,
            },
        ))
    }

    #[cfg(test)]
    pub(crate) fn mark_device_lost_for_test(&self) {
        self.context.mark_lost_for_test();
    }
}

struct OffscreenAttachments3D {
    color: wgpu::Texture,
    color_view: wgpu::TextureView,
    msaa_color: Option<wgpu::Texture>,
    msaa_color_view: Option<wgpu::TextureView>,
    _depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
    readback: Option<wgpu::Buffer>,
    padded_bytes_per_row: u32,
    width: u32,
    height: u32,
    sample_count: u32,
    creation_count: u64,
}

impl OffscreenAttachments3D {
    fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        sample_count: u32,
        readback_enabled: bool,
    ) -> Result<Self> {
        let unpadded = width.checked_mul(4).ok_or(PlottingError::GpuMemoryError {
            requested: usize::MAX,
            available: None,
        })?;
        let padded_bytes_per_row = unpadded.div_ceil(COPY_ROW_ALIGNMENT) * COPY_ROW_ALIGNMENT;
        let readback_size = if readback_enabled {
            Some(
                u64::from(padded_bytes_per_row)
                    .checked_mul(u64::from(height))
                    .ok_or(PlottingError::GpuMemoryError {
                        requested: usize::MAX,
                        available: None,
                    })?,
            )
        } else {
            None
        };
        let extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ruviz direct 3d resolve target"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | if readback_enabled {
                    wgpu::TextureUsages::COPY_SRC
                } else {
                    wgpu::TextureUsages::empty()
                },
            view_formats: &[],
        });
        let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());
        let (msaa_color, msaa_color_view) = if sample_count > 1 {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("ruviz direct 3d multisample target"),
                size: extent,
                mip_level_count: 1,
                sample_count,
                dimension: wgpu::TextureDimension::D2,
                format: COLOR_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            (Some(texture), Some(view))
        } else {
            (None, None)
        };
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ruviz direct 3d depth target"),
            size: extent,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
        let readback = readback_size.map(|size| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ruviz direct 3d readback"),
                size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            })
        });
        Ok(Self {
            color,
            color_view,
            msaa_color,
            msaa_color_view,
            _depth: depth,
            depth_view,
            readback,
            padded_bytes_per_row,
            width,
            height,
            sample_count,
            creation_count: u64::from(readback_enabled),
        })
    }

    fn render_color_view(&self) -> &wgpu::TextureView {
        self.msaa_color_view.as_ref().unwrap_or(&self.color_view)
    }

    fn resolve_target(&self) -> Option<&wgpu::TextureView> {
        self.msaa_color.as_ref().map(|_| &self.color_view)
    }
}

fn readback_image(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
    submission: wgpu::SubmissionIndex,
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,
) -> Result<Image> {
    let slice = buffer.slice(..);
    let (sender, receiver) = oneshot_channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .map_err(|error| PlottingError::RenderError(format!("3d GPU poll failed: {error}")))?;
    pollster::block_on(receiver.receive())
        .ok_or_else(|| PlottingError::RenderError("3d GPU readback channel closed".to_string()))?
        .map_err(|error| PlottingError::RenderError(format!("3d GPU readback failed: {error}")))?;

    let mapped = slice.get_mapped_range();
    let row_bytes = width as usize * 4;
    let padded_row_bytes = padded_bytes_per_row as usize;
    let output_len =
        row_bytes
            .checked_mul(height as usize)
            .ok_or(PlottingError::GpuMemoryError {
                requested: usize::MAX,
                available: None,
            })?;
    let mut pixels = vec![0_u8; output_len];
    for row in 0..height as usize {
        let source_start = row * padded_row_bytes;
        let destination_start = row * row_bytes;
        pixels[destination_start..destination_start + row_bytes]
            .copy_from_slice(&mapped[source_start..source_start + row_bytes]);
    }
    drop(mapped);
    buffer.unmap();
    Ok(Image::new(width, height, pixels))
}

fn validate_dimensions(width: u32, height: u32, maximum: u32) -> Result<()> {
    if width == 0 || height == 0 {
        Err(PlottingError::InvalidDimensions { width, height })
    } else if width > maximum || height > maximum {
        let requested = u64::from(width)
            .saturating_mul(u64::from(height))
            .saturating_mul(4);
        let available = u64::from(maximum)
            .saturating_mul(u64::from(maximum))
            .saturating_mul(4);
        Err(PlottingError::GpuMemoryError {
            requested: usize::try_from(requested).unwrap_or(usize::MAX),
            available: Some(usize::try_from(available).unwrap_or(usize::MAX)),
        })
    } else {
        Ok(())
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};

    use super::*;

    #[test]
    fn shared_slot_initializes_once_under_contention() {
        let slot = Arc::new(OnceLock::new());
        let start = Arc::new(Barrier::new(16));
        let initializations = Arc::new(AtomicUsize::new(0));
        let mut threads = Vec::new();

        for _ in 0..16 {
            let slot = Arc::clone(&slot);
            let start = Arc::clone(&start);
            let initializations = Arc::clone(&initializations);
            threads.push(std::thread::spawn(move || {
                start.wait();
                with_locked_try_init(
                    &slot,
                    || {
                        initializations.fetch_add(1, Ordering::SeqCst);
                        std::thread::yield_now();
                        Ok::<_, ()>(42)
                    },
                    || (),
                    |value| {
                        assert_eq!(*value, 42);
                        Ok(())
                    },
                )
            }));
        }

        for thread in threads {
            assert_eq!(thread.join().expect("thread panicked"), Ok(()));
        }
        assert_eq!(initializations.load(Ordering::SeqCst), 1);
    }
}
