//! Rendering backend and styling

pub mod backend;
pub mod skia;
pub mod cosmic_text_renderer;
pub mod primitives;
pub mod style;
pub mod color;
pub mod theme;
pub mod font;
#[cfg(feature = "parallel")]
pub mod parallel;
#[cfg(feature = "simd")]
pub mod simd;
pub mod pooled;

pub use backend::Renderer;
pub use skia::SkiaRenderer;
pub use cosmic_text_renderer::CosmicTextRenderer;
pub use style::{LineStyle, MarkerStyle};
pub use color::{Color, ColorError, ColorMap};
pub use theme::{Theme, ThemeBuilder, ThemeVariant};
pub use primitives::Primitive;
pub use font::{FontFamily, FontConfig, FontWeight, FontStyle};
#[cfg(feature = "parallel")]
pub use parallel::{ParallelRenderer, ParallelConfig, PerformanceStats, SeriesRenderData, DetailedPerformanceInfo};
#[cfg(feature = "simd")]
pub use simd::{SIMDTransformer, SIMDPerformanceInfo, CoordinateBounds, PixelViewport};
pub use pooled::{PooledRenderer, PooledRendererStats, LineSegment, get_pooled_renderer};