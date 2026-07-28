use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::{PlottingError, Result};

pub(super) const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
pub(super) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

pub(crate) struct GpuContext3D {
    pub(super) _instance: wgpu::Instance,
    pub(super) _adapter: wgpu::Adapter,
    pub(super) device: Arc<wgpu::Device>,
    pub(super) queue: Arc<wgpu::Queue>,
    pub(super) sample_count: u32,
    pub(super) adapter_name: String,
    lost: Arc<AtomicBool>,
}

impl GpuContext3D {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn new() -> Result<Self> {
        pollster::block_on(Self::from_instance_async(Self::create_instance(), None))
    }

    pub(crate) fn create_instance() -> wgpu::Instance {
        wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: if cfg!(debug_assertions) {
                wgpu::InstanceFlags::DEBUG | wgpu::InstanceFlags::VALIDATION
            } else {
                wgpu::InstanceFlags::default()
            },
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn for_surface(
        instance: wgpu::Instance,
        surface: &wgpu::Surface<'_>,
    ) -> Result<Self> {
        pollster::block_on(Self::from_instance_async(instance, Some(surface)))
    }

    pub(crate) async fn from_instance_async(
        instance: wgpu::Instance,
        compatible_surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<Self> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface,
            })
            .await
            .map_err(|error| PlottingError::GpuNotAvailable(error.to_string()))?;

        validate_format(
            &adapter,
            COLOR_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        )?;
        validate_format(
            &adapter,
            DEPTH_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        )?;

        let adapter_info = adapter.get_info();
        let required_limits = wgpu::Limits::default().using_resolution(adapter.limits());
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("ruviz direct 3d device"),
                required_features: wgpu::Features::empty(),
                required_limits,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|error| PlottingError::GpuInitError {
                backend: format!("{:?}", adapter_info.backend),
                error: error.to_string(),
            })?;
        let device = Arc::new(device);
        let queue = Arc::new(queue);
        let lost = Arc::new(AtomicBool::new(false));
        let lost_callback = Arc::clone(&lost);
        device.set_device_lost_callback(move |reason, message| {
            lost_callback.store(true, Ordering::Release);
            log::error!("ruviz 3d GPU device lost ({reason:?}): {message}");
        });
        device.on_uncaptured_error(Arc::new(|error| {
            log::error!("ruviz 3d GPU validation error: {error}");
        }));

        let color_features = adapter.get_texture_format_features(COLOR_FORMAT);
        let depth_features = adapter.get_texture_format_features(DEPTH_FORMAT);
        let sample_count = if color_features
            .flags
            .contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4)
            && color_features
                .flags
                .contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_RESOLVE)
            && depth_features
                .flags
                .contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4)
        {
            4
        } else {
            1
        };

        Ok(Self {
            _instance: instance,
            _adapter: adapter,
            device,
            queue,
            sample_count,
            adapter_name: adapter_info.name,
            lost,
        })
    }

    pub(crate) fn adapter(&self) -> &wgpu::Adapter {
        &self._adapter
    }

    pub(crate) fn instance(&self) -> &wgpu::Instance {
        &self._instance
    }

    pub(crate) fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub(crate) fn ensure_available(&self) -> Result<()> {
        if self.is_lost() {
            Err(PlottingError::GpuNotAvailable(
                "the direct 3d wgpu device was lost".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    pub(crate) fn is_lost(&self) -> bool {
        self.lost.load(Ordering::Acquire)
    }

    #[cfg(test)]
    pub(super) fn mark_lost_for_test(&self) {
        self.lost.store(true, Ordering::Release);
    }
}

pub(super) fn validate_format(
    adapter: &wgpu::Adapter,
    format: wgpu::TextureFormat,
    required_usage: wgpu::TextureUsages,
) -> Result<()> {
    let features = adapter.get_texture_format_features(format);
    if features.allowed_usages.contains(required_usage) {
        Ok(())
    } else {
        Err(PlottingError::UnsupportedGpuFeature(format!(
            "{format:?} does not support required usage {required_usage:?}"
        )))
    }
}
