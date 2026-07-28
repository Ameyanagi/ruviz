//! Backend-neutral 3D scene data and renderers.

pub(crate) mod color;
#[cfg(feature = "gpu")]
pub(crate) mod gpu;
pub(crate) mod overlay;
pub(crate) mod scene;
pub(crate) mod software;

/// Release every GPU resource retained by the shared 3D renderer.
///
/// `render_gpu()` and `render_auto()` keep one process-wide wgpu device, its
/// offscreen attachments, and a small cache of scene buffers alive so repeated
/// renders stay warm. A single 2000x2000 surface can retain well over 100 MB
/// that way. Call this when a program is done with 3D output — the next render
/// transparently rebuilds everything it needs.
///
/// This is a no-op when the `gpu` feature is disabled.
///
/// ```
/// # #[cfg(feature = "3d")] {
/// use ruviz::core::plot3d::release_3d_gpu_resources;
///
/// release_3d_gpu_resources();
/// # }
/// ```
pub fn release_3d_gpu_resources() {
    #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
    gpu::release_shared_renderer();
}
