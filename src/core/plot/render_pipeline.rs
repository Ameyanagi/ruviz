//! Render pipeline management for plots
//!
//! This module provides the [`RenderPipeline`] struct which handles
//! rendering configuration and backend selection for plots.

use super::BackendType;

#[cfg(feature = "parallel")]
use crate::render::ParallelRenderer;

/// Manages rendering configuration for plots
///
/// The RenderPipeline handles:
/// - Backend selection (Skia, etc.)
/// - Parallel rendering configuration
/// - Memory pooling for performance
/// - GPU acceleration settings
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::core::plot::RenderPipeline;
///
/// let mut pipeline = RenderPipeline::new();
/// pipeline.set_backend(BackendType::Skia);
/// pipeline.enable_pooled_rendering(true);
/// ```
#[derive(Clone, Debug)]
pub struct RenderPipeline {
    /// Parallel renderer for performance optimization
    #[cfg(feature = "parallel")]
    pub(crate) parallel_renderer: ParallelRenderer,
    /// Memory pool renderer for allocation optimization
    pub(crate) pooled_renderer: Option<crate::render::PooledRenderer>,
    /// Enable memory pooled rendering for performance
    pub(crate) enable_pooled_rendering: bool,
    /// Selected backend (None = auto-select)
    pub(crate) backend: Option<BackendType>,
    /// Whether auto-optimization has been applied
    pub(crate) auto_optimized: bool,
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
            #[cfg(feature = "parallel")]
            parallel_renderer: ParallelRenderer::new(),
            pooled_renderer: None,
            enable_pooled_rendering: false,
            backend: None,
            auto_optimized: false,
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

    /// Enable or disable pooled rendering
    pub fn set_pooled_rendering(&mut self, enabled: bool) {
        self.enable_pooled_rendering = enabled;
    }

    /// Check if pooled rendering is enabled
    pub fn pooled_rendering_enabled(&self) -> bool {
        self.enable_pooled_rendering
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

    /// Get reference to parallel renderer
    #[cfg(feature = "parallel")]
    pub fn parallel_renderer(&self) -> &ParallelRenderer {
        &self.parallel_renderer
    }

    /// Get mutable reference to parallel renderer
    #[cfg(feature = "parallel")]
    pub fn parallel_renderer_mut(&mut self) -> &mut ParallelRenderer {
        &mut self.parallel_renderer
    }

    /// Set or clear the pooled renderer
    pub fn set_pooled_renderer(&mut self, renderer: Option<crate::render::PooledRenderer>) {
        self.pooled_renderer = renderer;
    }

    /// Get reference to pooled renderer
    pub fn pooled_renderer(&self) -> Option<&crate::render::PooledRenderer> {
        self.pooled_renderer.as_ref()
    }

    /// Get mutable reference to pooled renderer
    pub fn pooled_renderer_mut(&mut self) -> Option<&mut crate::render::PooledRenderer> {
        self.pooled_renderer.as_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_render_pipeline() {
        let pipeline = RenderPipeline::new();
        assert!(pipeline.backend().is_none());
        assert!(!pipeline.pooled_rendering_enabled());
        assert!(!pipeline.is_auto_optimized());
    }

    #[test]
    fn test_backend_selection() {
        let mut pipeline = RenderPipeline::new();
        pipeline.set_backend(BackendType::Skia);
        assert_eq!(pipeline.backend(), Some(BackendType::Skia));
    }

    #[test]
    fn test_pooled_rendering() {
        let mut pipeline = RenderPipeline::new();
        assert!(!pipeline.pooled_rendering_enabled());

        pipeline.set_pooled_rendering(true);
        assert!(pipeline.pooled_rendering_enabled());

        pipeline.set_pooled_rendering(false);
        assert!(!pipeline.pooled_rendering_enabled());
    }

    #[test]
    fn test_auto_optimization() {
        let mut pipeline = RenderPipeline::new();
        assert!(!pipeline.is_auto_optimized());

        pipeline.set_auto_optimized(true);
        assert!(pipeline.is_auto_optimized());
    }
}
