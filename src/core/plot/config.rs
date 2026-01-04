//! Plot configuration types

/// Backend types for rendering
#[allow(clippy::upper_case_acronyms)] // GPU is the standard industry acronym
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    /// Default Skia backend (CPU-based, good for <1K points)
    Skia,
    /// Parallel multi-threaded backend (good for 1K-100K points)
    Parallel,
    /// GPU-accelerated backend (good for >100K points)
    GPU,
    /// DataShader aggregation backend (good for >1M points)
    DataShader,
}

/// Tick direction configuration
#[derive(Clone, Debug, PartialEq, Default)]
pub enum TickDirection {
    /// Ticks point inward into the plot area (default)
    #[default]
    Inside,
    /// Ticks point outward from the plot area
    Outside,
}

/// Grid display mode for major and minor ticks
#[derive(Clone, Debug, PartialEq, Default)]
pub enum GridMode {
    /// Show grid lines only at major ticks
    #[default]
    MajorOnly,
    /// Show grid lines only at minor ticks
    MinorOnly,
    /// Show grid lines at both major and minor ticks
    Both,
}
