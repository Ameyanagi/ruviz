//! Data handling and trait definitions

pub mod datashader_simple;
pub mod elements;
pub mod impls;
pub mod memory;
pub mod memory_pool;
pub mod observable;
pub mod platform;
pub mod pooled_vec;
pub mod traits;
pub mod transform;
pub mod validation;
pub mod zero_copy;

pub use datashader_simple::{DataShader, DataShaderCanvas, DataShaderImage, DataShaderStats};
pub use elements::{
    ErrorBar, LineSegment, MarkerInstance, PlotElementStats, PlotElementStorage, Polygon,
    PoolStats, TextAlignment, TextElement, get_plot_element_storage,
};
pub use memory::{
    ManagedBuffer, MemoryConfig, MemoryManager, MemoryStats, get_memory_manager,
    initialize_memory_manager,
};
pub use memory_pool::{MemoryPool, PoolStatistics, PooledBuffer, SharedMemoryPool};
pub use observable::{
    BatchNotifier, BatchUpdate, IntoObservable, Observable, ReactiveDataHandle,
    SlidingWindowObservable, StreamingBuffer, StreamingBufferView, StreamingXY, SubscriberCallback,
    SubscriberId, WeakObservable, lift, lift2, map,
};
pub use platform::{
    MemoryLimits, OptimizationConfig, PerformanceHints, PlatformInfo, PlatformOptimizer,
    get_platform_optimizer, initialize_platform_optimization,
};
pub use pooled_vec::{PooledVec, PooledVecIntoIter};
pub use traits::Data1D;
pub use validation::{collect_finite_values, collect_finite_values_sorted};
pub use zero_copy::{DataView, DataViewIter, MappedDataView, MappedDataViewIter};
