//! Rendering backend and styling

pub mod backend;
pub mod color;
pub mod cosmic_text_renderer;
#[cfg(feature = "gpu")]
pub mod gpu;
#[cfg(feature = "parallel")]
pub mod parallel;
pub mod pooled;
#[cfg(feature = "simd")]
pub mod simd;
pub mod skia;
pub mod style;
pub mod text;
pub mod theme;

pub use backend::Renderer;
pub use color::{Color, ColorError, ColorMap};
pub use cosmic_text_renderer::CosmicTextRenderer;
pub use text::{FontConfig, FontFamily, FontStyle, FontWeight};
#[cfg(feature = "gpu")]
pub use gpu::{GpuBackend, GpuRenderer, initialize_gpu_backend, is_gpu_available};
#[cfg(feature = "parallel")]
pub use parallel::{
    DetailedPerformanceInfo, ParallelConfig, ParallelRenderer, PerformanceStats, SeriesRenderData,
};
pub use pooled::{LineSegment, PooledRenderer, PooledRendererStats, get_pooled_renderer};
#[cfg(feature = "simd")]
pub use simd::{CoordinateBounds, PixelViewport, SIMDPerformanceInfo, SIMDTransformer};
pub use skia::SkiaRenderer;
pub use style::{LineStyle, MarkerStyle};
pub use text::{TextRenderer, get_font_system, get_swash_cache, initialize_text_system};
pub use theme::{Theme, ThemeBuilder, ThemeVariant};
