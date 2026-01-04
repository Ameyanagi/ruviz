//! Platform-specific memory optimizations

use crate::data::memory::MemoryConfig;
use std::sync::{Arc, RwLock};

/// Platform-specific memory optimization strategies
#[derive(Debug, Clone)]
pub struct PlatformOptimizer {
    platform_info: PlatformInfo,
    optimization_config: OptimizationConfig,
    memory_limits: MemoryLimits,
    performance_hints: Arc<RwLock<PerformanceHints>>,
}

/// Platform information and capabilities
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    pub os_type: OSType,
    pub total_memory: u64,
    pub available_memory: u64,
    pub cpu_cores: usize,
    pub cache_line_size: usize,
    pub page_size: usize,
    pub numa_nodes: usize,
    pub supports_hugepages: bool,
    pub supports_memory_mapping: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OSType {
    Linux,
    MacOS,
    Windows,
    Other(String),
}

/// Platform-specific optimization configuration
#[derive(Debug, Clone)]
pub struct OptimizationConfig {
    pub use_hugepages: bool,
    pub use_memory_mapping: bool,
    pub prefer_numa_local: bool,
    pub align_to_cache_lines: bool,
    pub use_transparent_hugepages: bool,
    pub memory_prefault: bool,
}

/// Memory limits and thresholds
#[derive(Debug, Clone)]
pub struct MemoryLimits {
    pub max_heap_size: Option<u64>,
    pub warning_threshold: f32,
    pub critical_threshold: f32,
    pub swap_threshold: f32,
    pub gc_trigger_threshold: f32,
}

/// Platform-specific performance hints
#[derive(Debug, Clone)]
pub struct PerformanceHints {
    pub optimal_chunk_size: usize,
    pub recommended_alignment: usize,
    pub prefetch_distance: usize,
    pub concurrent_allocations: usize,
    pub memory_bandwidth: Option<u64>,
    pub latency_sensitive: bool,
}

impl PlatformOptimizer {
    pub fn new() -> Result<Self, crate::core::error::PlottingError> {
        let platform_info = detect_platform_info()?;
        let optimization_config = create_optimization_config(&platform_info);
        let memory_limits = calculate_memory_limits(&platform_info);
        let performance_hints = generate_performance_hints(&platform_info);

        Ok(Self {
            platform_info,
            optimization_config,
            memory_limits,
            performance_hints: Arc::new(RwLock::new(performance_hints)),
        })
    }

    pub fn optimize_config(&self, base_config: &MemoryConfig) -> MemoryConfig {
        let mut optimized = base_config.clone();

        let memory_ratio =
            (self.platform_info.available_memory as f64) / (8u64 * 1024 * 1024 * 1024) as f64;
        // Adjust max pool size based on available memory
        optimized.max_pool_size = (optimized.max_pool_size as f64 * memory_ratio.min(8.0)) as usize;

        optimized
    }

    pub fn get_performance_hints(&self) -> PerformanceHints {
        self.performance_hints.read().unwrap().clone()
    }
}

fn detect_platform_info() -> Result<PlatformInfo, crate::core::error::PlottingError> {
    let os_type = detect_os_type();
    let total_memory = platform::get_total_memory()?;
    let available_memory = platform::get_available_memory()?;
    let cpu_cores = num_cpus::get();
    let numa_nodes = platform::get_numa_nodes();
    let supports_hugepages = platform::check_hugepage_support();
    let supports_memory_mapping = platform::check_memory_mapping_support()?;

    Ok(PlatformInfo {
        os_type,
        total_memory,
        available_memory,
        cpu_cores,
        cache_line_size: 64,
        page_size: 4096,
        numa_nodes,
        supports_hugepages,
        supports_memory_mapping,
    })
}

fn detect_os_type() -> OSType {
    #[cfg(target_os = "linux")]
    return OSType::Linux;

    #[cfg(target_os = "macos")]
    return OSType::MacOS;

    #[cfg(target_os = "windows")]
    return OSType::Windows;

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return OSType::Other(std::env::consts::OS.to_string());
}

fn create_optimization_config(platform_info: &PlatformInfo) -> OptimizationConfig {
    OptimizationConfig {
        use_hugepages: platform_info.supports_hugepages,
        use_memory_mapping: platform_info.supports_memory_mapping,
        prefer_numa_local: platform_info.numa_nodes > 1,
        align_to_cache_lines: true,
        use_transparent_hugepages: platform_info.os_type == OSType::Linux,
        memory_prefault: platform_info.os_type != OSType::MacOS,
    }
}

fn calculate_memory_limits(platform_info: &PlatformInfo) -> MemoryLimits {
    let total_gb = platform_info.total_memory as f32 / (1024.0 * 1024.0 * 1024.0);

    MemoryLimits {
        max_heap_size: Some((platform_info.total_memory as f32 * 0.8) as u64),
        warning_threshold: if total_gb > 16.0 { 0.7 } else { 0.6 },
        critical_threshold: if total_gb > 16.0 { 0.85 } else { 0.8 },
        swap_threshold: if total_gb > 8.0 { 0.9 } else { 0.85 },
        gc_trigger_threshold: if total_gb > 32.0 { 0.75 } else { 0.65 },
    }
}

fn generate_performance_hints(platform_info: &PlatformInfo) -> PerformanceHints {
    let total_gb = platform_info.total_memory as f32 / (1024.0 * 1024.0 * 1024.0);

    PerformanceHints {
        optimal_chunk_size: if total_gb > 16.0 {
            64 * 1024
        } else {
            32 * 1024
        },
        recommended_alignment: platform_info.cache_line_size,
        prefetch_distance: platform_info.cache_line_size * 8,
        concurrent_allocations: platform_info.cpu_cores.min(8),
        memory_bandwidth: None,
        latency_sensitive: total_gb <= 4.0,
    }
}

static PLATFORM_OPTIMIZER: std::sync::OnceLock<PlatformOptimizer> = std::sync::OnceLock::new();

pub fn get_platform_optimizer() -> &'static PlatformOptimizer {
    PLATFORM_OPTIMIZER.get_or_init(|| {
        PlatformOptimizer::new().unwrap_or_else(|_| PlatformOptimizer {
            platform_info: PlatformInfo {
                os_type: detect_os_type(),
                total_memory: 8 * 1024 * 1024 * 1024,
                available_memory: 4 * 1024 * 1024 * 1024,
                cpu_cores: num_cpus::get(),
                cache_line_size: 64,
                page_size: 4096,
                numa_nodes: 1,
                supports_hugepages: false,
                supports_memory_mapping: true,
            },
            optimization_config: OptimizationConfig {
                use_hugepages: false,
                use_memory_mapping: true,
                prefer_numa_local: false,
                align_to_cache_lines: true,
                use_transparent_hugepages: false,
                memory_prefault: false,
            },
            memory_limits: MemoryLimits {
                max_heap_size: Some(6 * 1024 * 1024 * 1024),
                warning_threshold: 0.7,
                critical_threshold: 0.85,
                swap_threshold: 0.9,
                gc_trigger_threshold: 0.75,
            },
            performance_hints: Arc::new(RwLock::new(PerformanceHints {
                optimal_chunk_size: 32 * 1024,
                recommended_alignment: 64,
                prefetch_distance: 512,
                concurrent_allocations: 4,
                memory_bandwidth: None,
                latency_sensitive: false,
            })),
        })
    })
}

pub fn initialize_platform_optimization() -> Result<(), crate::core::error::PlottingError> {
    let _optimizer = get_platform_optimizer();
    Ok(())
}

#[allow(clippy::module_inception)] // Platform-specific module pattern using #[path]
#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod platform;

#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod platform;

#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod platform;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod platform {
    use crate::core::error::PlottingError;

    pub fn get_total_memory() -> Result<u64, RuvizError> {
        Ok(8 * 1024 * 1024 * 1024) // 8GB default
    }

    pub fn get_available_memory() -> Result<u64, RuvizError> {
        Ok(4 * 1024 * 1024 * 1024) // 4GB default
    }

    pub fn get_numa_nodes() -> usize {
        1
    }

    pub fn check_hugepage_support() -> bool {
        false
    }

    pub fn check_large_page_support() -> bool {
        false
    }

    pub fn check_memory_mapping_support() -> Result<bool, RuvizError> {
        Ok(false)
    }
}
