use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use crate::data::{get_memory_manager, MemoryStats};

/// Adaptive memory allocation strategy that adjusts pool sizes and behavior
/// based on usage patterns and memory pressure.
#[derive(Debug)]
pub struct AdaptiveMemoryStrategy {
    /// Usage tracking for different buffer sizes
    usage_tracker: Arc<Mutex<UsageTracker>>,
    /// Memory pressure monitor
    pressure_monitor: Arc<Mutex<MemoryPressureMonitor>>,
    /// Strategy configuration
    config: AdaptiveConfig,
    /// Last adaptation time
    last_adaptation: Arc<Mutex<Instant>>,
}

impl AdaptiveMemoryStrategy {
    /// Create new adaptive memory strategy with default configuration
    pub fn new() -> Self {
        Self::with_config(AdaptiveConfig::default())
    }
    
    /// Create adaptive memory strategy with custom configuration
    pub fn with_config(config: AdaptiveConfig) -> Self {
        Self {
            usage_tracker: Arc::new(Mutex::new(UsageTracker::new())),
            pressure_monitor: Arc::new(Mutex::new(MemoryPressureMonitor::new())),
            config,
            last_adaptation: Arc::new(Mutex::new(Instant::now())),
        }
    }
    
    /// Record buffer usage for adaptation
    pub fn record_buffer_usage(&self, buffer_type: BufferType, size: usize, hit: bool) {
        let mut tracker = self.usage_tracker.lock().unwrap();
        tracker.record_usage(buffer_type, size, hit);
    }
    
    /// Check if memory adaptation should be performed
    pub fn should_adapt(&self) -> bool {
        let last_adaptation = *self.last_adaptation.lock().unwrap();
        let elapsed = last_adaptation.elapsed();
        
        elapsed >= self.config.adaptation_interval ||
        self.is_memory_pressure_high() ||
        self.has_significant_usage_change()
    }
    
    /// Perform memory adaptation based on current usage patterns
    pub fn adapt_memory_pools(&self) -> AdaptationResult {
        if !self.should_adapt() {
            return AdaptationResult::NoActionNeeded;
        }
        
        let memory_manager = get_memory_manager();
        let current_stats = memory_manager.get_stats();
        
        let mut tracker = self.usage_tracker.lock().unwrap();
        let mut pressure_monitor = self.pressure_monitor.lock().unwrap();
        
        pressure_monitor.update_memory_stats(&current_stats);
        
        let adaptations = self.calculate_adaptations(&tracker, &pressure_monitor, &current_stats);
        
        // Update last adaptation time
        *self.last_adaptation.lock().unwrap() = Instant::now();
        
        // Reset usage tracking for next period
        tracker.reset_period();
        
        adaptations
    }
    
    /// Calculate what adaptations should be made
    fn calculate_adaptations(
        &self,
        tracker: &UsageTracker,
        pressure_monitor: &MemoryPressureMonitor,
        current_stats: &MemoryStats,
    ) -> AdaptationResult {
        let mut adaptations = Vec::new();
        
        // Analyze buffer usage patterns
        for (buffer_type, usage) in tracker.get_usage_summary() {
            if usage.total_requests == 0 {
                continue;
            }
            
            let hit_rate = usage.hits as f32 / usage.total_requests as f32;
            let pressure_level = pressure_monitor.get_pressure_level();
            
            // Determine if pools should be expanded or contracted
            if hit_rate < self.config.low_hit_rate_threshold && pressure_level != MemoryPressureLevel::High {
                // Low hit rate - expand pools
                let suggested_expansion = self.calculate_expansion_factor(&usage, pressure_level);
                adaptations.push(PoolAdaptation {
                    buffer_type,
                    action: AdaptationAction::ExpandPool,
                    factor: suggested_expansion,
                    reason: AdaptationReason::LowHitRate { hit_rate },
                });
            } else if hit_rate > self.config.high_hit_rate_threshold && pressure_level == MemoryPressureLevel::High {
                // High hit rate but memory pressure - selective contraction
                let contraction_factor = self.calculate_contraction_factor(&usage, pressure_level);
                adaptations.push(PoolAdaptation {
                    buffer_type,
                    action: AdaptationAction::ContractPool,
                    factor: contraction_factor,
                    reason: AdaptationReason::MemoryPressure,
                });
            }
            
            // Adjust pool sizes based on common request sizes
            if let Some(optimal_size) = usage.get_optimal_size() {
                adaptations.push(PoolAdaptation {
                    buffer_type,
                    action: AdaptationAction::AdjustTargetSize,
                    factor: optimal_size as f32,
                    reason: AdaptationReason::OptimalSizeDetected { size: optimal_size },
                });
            }
        }
        
        // Global memory pressure adaptations
        match pressure_monitor.get_pressure_level() {
            MemoryPressureLevel::High => {
                adaptations.push(PoolAdaptation {
                    buffer_type: BufferType::All,
                    action: AdaptationAction::ForceTrim,
                    factor: self.config.pressure_trim_factor,
                    reason: AdaptationReason::MemoryPressure,
                });
            },
            MemoryPressureLevel::Critical => {
                adaptations.push(PoolAdaptation {
                    buffer_type: BufferType::All,
                    action: AdaptationAction::EmergencyCleanup,
                    factor: 0.1, // Keep only 10% of current pools
                    reason: AdaptationReason::CriticalMemoryPressure,
                });
            },
            _ => {}
        }
        
        if adaptations.is_empty() {
            AdaptationResult::NoActionNeeded
        } else {
            AdaptationResult::Adaptations(adaptations)
        }
    }
    
    /// Calculate expansion factor based on usage patterns
    fn calculate_expansion_factor(&self, usage: &BufferUsage, pressure_level: MemoryPressureLevel) -> f32 {
        let base_expansion = match pressure_level {
            MemoryPressureLevel::Low => self.config.max_expansion_factor,
            MemoryPressureLevel::Normal => self.config.max_expansion_factor * 0.7,
            MemoryPressureLevel::High => self.config.max_expansion_factor * 0.3,
            MemoryPressureLevel::Critical => 1.0, // No expansion
        };
        
        let usage_intensity = (usage.total_requests as f32).ln() / 10.0; // Log scale
        (base_expansion * (1.0 + usage_intensity)).min(self.config.max_expansion_factor)
    }
    
    /// Calculate contraction factor based on memory pressure
    fn calculate_contraction_factor(&self, _usage: &BufferUsage, pressure_level: MemoryPressureLevel) -> f32 {
        match pressure_level {
            MemoryPressureLevel::High => 0.7,      // 30% reduction
            MemoryPressureLevel::Critical => 0.3,   // 70% reduction
            _ => 1.0, // No contraction
        }
    }
    
    /// Check if memory pressure is high
    fn is_memory_pressure_high(&self) -> bool {
        let pressure_monitor = self.pressure_monitor.lock().unwrap();
        matches!(
            pressure_monitor.get_pressure_level(),
            MemoryPressureLevel::High | MemoryPressureLevel::Critical
        )
    }
    
    /// Check if there has been significant usage change
    fn has_significant_usage_change(&self) -> bool {
        let tracker = self.usage_tracker.lock().unwrap();
        tracker.has_significant_change(self.config.significant_change_threshold)
    }
    
    /// Get current adaptation statistics
    pub fn get_stats(&self) -> AdaptiveStats {
        let tracker = self.usage_tracker.lock().unwrap();
        let pressure_monitor = self.pressure_monitor.lock().unwrap();
        
        AdaptiveStats {
            total_adaptations: tracker.total_adaptations,
            current_pressure_level: pressure_monitor.get_pressure_level(),
            buffer_usage_summary: tracker.get_usage_summary(),
            last_adaptation: *self.last_adaptation.lock().unwrap(),
        }
    }
}

impl Default for AdaptiveMemoryStrategy {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for adaptive memory allocation
#[derive(Debug, Clone)]
pub struct AdaptiveConfig {
    /// Minimum interval between adaptations
    pub adaptation_interval: Duration,
    /// Hit rate below which pools should be expanded
    pub low_hit_rate_threshold: f32,
    /// Hit rate above which pools might be contracted under pressure
    pub high_hit_rate_threshold: f32,
    /// Maximum expansion factor for pools
    pub max_expansion_factor: f32,
    /// Trim factor under memory pressure
    pub pressure_trim_factor: f32,
    /// Threshold for detecting significant usage changes
    pub significant_change_threshold: f32,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            adaptation_interval: Duration::from_secs(30), // Adapt every 30 seconds
            low_hit_rate_threshold: 0.6,  // 60% hit rate
            high_hit_rate_threshold: 0.9, // 90% hit rate
            max_expansion_factor: 2.0,    // At most double pool size
            pressure_trim_factor: 0.5,    // Reduce pools by 50% under pressure
            significant_change_threshold: 0.3, // 30% change in usage patterns
        }
    }
}

/// Tracks buffer usage patterns for different buffer types and sizes
#[derive(Debug)]
struct UsageTracker {
    /// Usage statistics per buffer type
    usage_stats: HashMap<BufferType, BufferUsage>,
    /// Total number of adaptations performed
    total_adaptations: usize,
    /// Previous period statistics for change detection
    previous_period: Option<HashMap<BufferType, BufferUsage>>,
}

impl UsageTracker {
    fn new() -> Self {
        Self {
            usage_stats: HashMap::new(),
            total_adaptations: 0,
            previous_period: None,
        }
    }
    
    fn record_usage(&mut self, buffer_type: BufferType, size: usize, hit: bool) {
        let usage = self.usage_stats.entry(buffer_type).or_default();
        usage.record_request(size, hit);
    }
    
    fn get_usage_summary(&self) -> Vec<(BufferType, BufferUsage)> {
        self.usage_stats.iter().map(|(&k, v)| (k, v.clone())).collect()
    }
    
    fn has_significant_change(&self, threshold: f32) -> bool {
        if let Some(ref previous) = self.previous_period {
            for (buffer_type, current_usage) in &self.usage_stats {
                if let Some(prev_usage) = previous.get(buffer_type) {
                    let change_rate = if prev_usage.total_requests > 0 {
                        (current_usage.total_requests as f32 - prev_usage.total_requests as f32).abs()
                            / prev_usage.total_requests as f32
                    } else {
                        1.0 // New buffer type is significant change
                    };
                    
                    if change_rate > threshold {
                        return true;
                    }
                }
            }
        }
        false
    }
    
    fn reset_period(&mut self) {
        self.previous_period = Some(self.usage_stats.clone());
        self.usage_stats.clear();
        self.total_adaptations += 1;
    }
}

/// Buffer usage statistics
#[derive(Debug, Clone, Default)]
pub struct BufferUsage {
    pub hits: usize,
    pub misses: usize,
    pub total_requests: usize,
    /// Size distribution of requests
    size_histogram: HashMap<usize, usize>,
}

impl BufferUsage {
    fn record_request(&mut self, size: usize, hit: bool) {
        self.total_requests += 1;
        if hit {
            self.hits += 1;
        } else {
            self.misses += 1;
        }
        
        *self.size_histogram.entry(size).or_default() += 1;
    }
    
    /// Get the most common requested size (optimal size for pre-allocation)
    fn get_optimal_size(&self) -> Option<usize> {
        self.size_histogram
            .iter()
            .max_by_key(|&(_, count)| count)
            .map(|(&size, _)| size)
    }
    
    pub fn hit_rate(&self) -> f32 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.hits as f32 / self.total_requests as f32
        }
    }
}

/// Memory pressure monitoring
#[derive(Debug)]
struct MemoryPressureMonitor {
    /// Recent memory usage samples
    memory_samples: Vec<MemorySample>,
    /// Current pressure level
    current_pressure_level: MemoryPressureLevel,
}

impl MemoryPressureMonitor {
    fn new() -> Self {
        Self {
            memory_samples: Vec::new(),
            current_pressure_level: MemoryPressureLevel::Low,
        }
    }
    
    fn update_memory_stats(&mut self, stats: &MemoryStats) {
        let sample = MemorySample {
            timestamp: Instant::now(),
            allocated_bytes: stats.total_allocated,
            pool_hits: (stats.pool_hit_rate * stats.active_allocations as f32) as usize,
            pool_misses: stats.active_allocations.saturating_sub(
                (stats.pool_hit_rate * stats.active_allocations as f32) as usize
            ),
        };
        
        self.memory_samples.push(sample);
        
        // Keep only recent samples (last 10 minutes)
        let cutoff = Instant::now() - Duration::from_secs(600);
        self.memory_samples.retain(|s| s.timestamp > cutoff);
        
        // Update pressure level
        self.current_pressure_level = self.calculate_pressure_level();
    }
    
    fn calculate_pressure_level(&self) -> MemoryPressureLevel {
        if self.memory_samples.is_empty() {
            return MemoryPressureLevel::Low;
        }
        
        let recent_growth = self.calculate_memory_growth_rate();
        let current_allocation = self.memory_samples.last().unwrap().allocated_bytes;
        let hit_rate = self.calculate_recent_hit_rate();
        
        // Determine pressure based on multiple factors
        if current_allocation > 100 * 1024 * 1024 || recent_growth > 0.5 || hit_rate < 0.3 {
            MemoryPressureLevel::Critical
        } else if current_allocation > 50 * 1024 * 1024 || recent_growth > 0.2 || hit_rate < 0.5 {
            MemoryPressureLevel::High
        } else if current_allocation > 20 * 1024 * 1024 || recent_growth > 0.1 {
            MemoryPressureLevel::Normal
        } else {
            MemoryPressureLevel::Low
        }
    }
    
    fn calculate_memory_growth_rate(&self) -> f32 {
        if self.memory_samples.len() < 2 {
            return 0.0;
        }
        
        let first = self.memory_samples.first().unwrap();
        let last = self.memory_samples.last().unwrap();
        
        if first.allocated_bytes == 0 {
            return 1.0; // 100% growth from zero
        }
        
        (last.allocated_bytes as f32 - first.allocated_bytes as f32) / first.allocated_bytes as f32
    }
    
    fn calculate_recent_hit_rate(&self) -> f32 {
        if self.memory_samples.is_empty() {
            return 1.0;
        }
        
        let total_hits: usize = self.memory_samples.iter().map(|s| s.pool_hits).sum();
        let total_misses: usize = self.memory_samples.iter().map(|s| s.pool_misses).sum();
        let total_requests = total_hits + total_misses;
        
        if total_requests == 0 {
            1.0
        } else {
            total_hits as f32 / total_requests as f32
        }
    }
    
    fn get_pressure_level(&self) -> MemoryPressureLevel {
        self.current_pressure_level
    }
}

/// Memory sample for trend analysis
#[derive(Debug, Clone)]
struct MemorySample {
    timestamp: Instant,
    allocated_bytes: usize,
    pool_hits: usize,
    pool_misses: usize,
}

/// Memory pressure levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPressureLevel {
    Low,
    Normal,
    High,
    Critical,
}

/// Buffer type categories for targeted adaptation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferType {
    Point2f,
    F32,
    F64,
    U8,
    U32,
    All, // Special type for global operations
}

/// Pool adaptation recommendation
#[derive(Debug, Clone)]
pub struct PoolAdaptation {
    pub buffer_type: BufferType,
    pub action: AdaptationAction,
    pub factor: f32,
    pub reason: AdaptationReason,
}

/// Adaptation actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdaptationAction {
    ExpandPool,
    ContractPool,
    AdjustTargetSize,
    ForceTrim,
    EmergencyCleanup,
}

/// Reasons for adaptation
#[derive(Debug, Clone)]
pub enum AdaptationReason {
    LowHitRate { hit_rate: f32 },
    HighHitRate { hit_rate: f32 },
    MemoryPressure,
    CriticalMemoryPressure,
    OptimalSizeDetected { size: usize },
    PeriodicMaintenance,
}

/// Result of adaptation analysis
#[derive(Debug)]
pub enum AdaptationResult {
    NoActionNeeded,
    Adaptations(Vec<PoolAdaptation>),
}

/// Statistics for the adaptive memory strategy
#[derive(Debug)]
pub struct AdaptiveStats {
    pub total_adaptations: usize,
    pub current_pressure_level: MemoryPressureLevel,
    pub buffer_usage_summary: Vec<(BufferType, BufferUsage)>,
    pub last_adaptation: Instant,
}

/// Global adaptive memory strategy instance
static ADAPTIVE_STRATEGY: std::sync::OnceLock<AdaptiveMemoryStrategy> = std::sync::OnceLock::new();

/// Get the global adaptive memory strategy
pub fn get_adaptive_strategy() -> &'static AdaptiveMemoryStrategy {
    ADAPTIVE_STRATEGY.get_or_init(AdaptiveMemoryStrategy::new)
}

/// Initialize adaptive strategy with custom configuration
pub fn initialize_adaptive_strategy(config: AdaptiveConfig) -> Result<(), String> {
    ADAPTIVE_STRATEGY.set(AdaptiveMemoryStrategy::with_config(config))
        .map_err(|_| "Adaptive strategy already initialized".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_tracker() {
        let mut tracker = UsageTracker::new();
        
        // Record some usage
        tracker.record_usage(BufferType::Point2f, 1000, true);
        tracker.record_usage(BufferType::Point2f, 1000, false);
        tracker.record_usage(BufferType::F32, 500, true);
        
        let summary = tracker.get_usage_summary();
        assert_eq!(summary.len(), 2);
        
        let point_usage = summary.iter().find(|(t, _)| *t == BufferType::Point2f).unwrap();
        assert_eq!(point_usage.1.total_requests, 2);
        assert_eq!(point_usage.1.hits, 1);
        assert_eq!(point_usage.1.hit_rate(), 0.5);
    }

    #[test]
    fn test_buffer_usage_optimal_size() {
        let mut usage = BufferUsage::default();
        
        // Record requests with different sizes
        usage.record_request(1000, true);
        usage.record_request(1000, true);
        usage.record_request(1000, false);
        usage.record_request(500, true);
        usage.record_request(2000, false);
        
        // 1000 should be the most common size
        assert_eq!(usage.get_optimal_size(), Some(1000));
    }

    #[test]
    fn test_adaptive_config_defaults() {
        let config = AdaptiveConfig::default();
        assert_eq!(config.adaptation_interval, Duration::from_secs(30));
        assert_eq!(config.low_hit_rate_threshold, 0.6);
        assert_eq!(config.max_expansion_factor, 2.0);
    }

    #[test]
    fn test_memory_pressure_levels() {
        let mut monitor = MemoryPressureMonitor::new();
        
        // Start with low pressure
        assert_eq!(monitor.get_pressure_level(), MemoryPressureLevel::Low);
        
        // Simulate memory growth
        let stats = MemoryStats {
            total_allocated: 30 * 1024 * 1024, // 30MB
            total_deallocated: 0,
            current_usage: 30 * 1024 * 1024,
            peak_usage: 30 * 1024 * 1024,
            active_allocations: 150,
            pool_hit_rate: 0.67, // 67% hit rate
            pool_stats: crate::data::memory::PoolStats {
                f32_pool_size: 1000,
                f64_pool_size: 1000,
                u8_pool_size: 1000,
                u32_pool_size: 1000,
                point_pool_size: 1000,
                block_pool_size: 10,
                total_pool_memory: 5000,
            },
        };
        
        monitor.update_memory_stats(&stats);
        assert_eq!(monitor.get_pressure_level(), MemoryPressureLevel::Normal);
    }

    #[test]
    fn test_expansion_factor_calculation() {
        let strategy = AdaptiveMemoryStrategy::new();
        let usage = BufferUsage {
            hits: 50,
            misses: 50,
            total_requests: 100,
            size_histogram: HashMap::new(),
        };
        
        let factor = strategy.calculate_expansion_factor(&usage, MemoryPressureLevel::Low);
        assert!(factor > 1.0);
        assert!(factor <= strategy.config.max_expansion_factor);
        
        let factor_pressure = strategy.calculate_expansion_factor(&usage, MemoryPressureLevel::High);
        assert!(factor_pressure < factor); // Less expansion under pressure
    }

    #[test]
    fn test_adaptation_should_occur() {
        let strategy = AdaptiveMemoryStrategy::new();
        
        // Record usage to trigger adaptation
        strategy.record_buffer_usage(BufferType::Point2f, 1000, false); // Low hit rate
        strategy.record_buffer_usage(BufferType::Point2f, 1000, false);
        strategy.record_buffer_usage(BufferType::Point2f, 1000, true);
        
        // Should detect need for adaptation due to usage patterns
        assert!(strategy.has_significant_usage_change());
    }
}