//! Render pipeline management for plots
//!
//! This module provides the [`RenderPipeline`] struct which handles
//! rendering configuration and backend selection for plots.

use super::BackendType;

/// Manages rendering configuration for plots
///
/// The RenderPipeline handles:
/// - Backend selection (Skia, etc.)
/// - Output sizing overrides used by internal render targets
/// - GPU acceleration settings
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::core::plot::RenderPipeline;
///
/// let mut pipeline = RenderPipeline::new();
/// pipeline.set_backend(BackendType::Skia);
/// ```
#[derive(Clone, Debug)]
pub struct RenderPipeline {
    /// Selected backend (None = auto-select)
    pub(crate) backend: Option<BackendType>,
    /// Whether auto-optimization has been applied
    pub(crate) auto_optimized: bool,
    /// Allow internally prepared interactive frames below the public minimum DPI.
    pub(crate) allow_subminimum_dpi: bool,
    /// Whether `dpi()` was called explicitly. `max_resolution` treats an
    /// explicit DPI as a request to keep, only reducing it to honour the
    /// bounds; a default DPI is scaled freely to fit them.
    pub(crate) explicit_dpi: bool,
    /// Exact output pixels requested by an internal rendering target.
    pub(crate) explicit_output_pixels: Option<(u32, u32)>,
    /// Allow positive child subplot canvases below the top-level dimension minimum.
    pub(crate) allow_subplot_dimensions: bool,
    /// Enable GPU acceleration for coordinate transformations
    #[cfg(feature = "gpu")]
    pub(crate) enable_gpu: bool,
}

impl Default for RenderPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderPipeline {
    /// Create a new render pipeline with default settings
    pub fn new() -> Self {
        Self {
            backend: None,
            auto_optimized: false,
            allow_subminimum_dpi: false,
            explicit_dpi: false,
            explicit_output_pixels: None,
            allow_subplot_dimensions: false,
            #[cfg(feature = "gpu")]
            enable_gpu: false,
        }
    }

    /// Set the rendering backend
    pub fn set_backend(&mut self, backend: BackendType) {
        self.backend = Some(backend);
    }

    /// Get the selected backend
    pub fn backend(&self) -> Option<BackendType> {
        self.backend
    }

    /// Mark that auto-optimization has been applied
    pub fn set_auto_optimized(&mut self, optimized: bool) {
        self.auto_optimized = optimized;
    }

    /// Check if auto-optimization has been applied
    pub fn is_auto_optimized(&self) -> bool {
        self.auto_optimized
    }

    /// Enable or disable GPU acceleration
    #[cfg(feature = "gpu")]
    pub fn set_gpu_enabled(&mut self, enabled: bool) {
        self.enable_gpu = enabled;
    }

    /// Check if GPU acceleration is enabled
    #[cfg(feature = "gpu")]
    pub fn gpu_enabled(&self) -> bool {
        self.enable_gpu
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_render_pipeline() {
        let pipeline = RenderPipeline::new();
        assert!(pipeline.backend().is_none());
        assert!(!pipeline.is_auto_optimized());
    }

    #[test]
    fn test_backend_selection() {
        let mut pipeline = RenderPipeline::new();
        pipeline.set_backend(BackendType::Skia);
        assert_eq!(pipeline.backend(), Some(BackendType::Skia));
    }

    #[test]
    fn test_auto_optimization() {
        let mut pipeline = RenderPipeline::new();
        assert!(!pipeline.is_auto_optimized());

        pipeline.set_auto_optimized(true);
        assert!(pipeline.is_auto_optimized());
    }
}
