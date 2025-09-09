//! Data handling and trait definitions

pub mod traits;
pub mod impls;
pub mod transform;
pub mod datashader;
pub mod datashader_simple;
pub mod memory;
pub mod elements;
pub mod adaptive;
pub mod profiler;
pub mod platform;

pub use traits::Data1D;
// Use simple DataShader temporarily to fix compilation
pub use datashader_simple::{DataShader, DataShaderCanvas, DataShaderImage, DataShaderStats};
pub use memory::{MemoryManager, MemoryConfig, MemoryStats, ManagedBuffer, get_memory_manager, initialize_memory_manager};
pub use elements::{
    PlotElementStorage, LineSegment, MarkerInstance, Polygon, TextElement, ErrorBar,
    TextAlignment, PlotElementStats, PoolStats, get_plot_element_storage
};
pub use adaptive::{
    AdaptiveMemoryStrategy, AdaptiveConfig, AdaptationResult, PoolAdaptation,
    AdaptationAction, AdaptationReason, MemoryPressureLevel, BufferType,
    BufferUsage, AdaptiveStats, get_adaptive_strategy, initialize_adaptive_strategy
};
pub use profiler::{
    MemoryProfiler, AllocationTracker, LeakDetector, UsagePatternAnalyzer,
    AllocationRecord, get_memory_profiler
};
pub use platform::{
    PlatformOptimizer, PlatformInfo, OptimizationConfig, MemoryLimits, PerformanceHints,
    get_platform_optimizer, initialize_platform_optimization
};
// Future implementations will add more data types