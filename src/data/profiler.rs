use std::sync::{Arc, Mutex, RwLock};
use std::collections::{HashMap, BTreeMap, VecDeque};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::thread;
use std::backtrace::Backtrace;
use std::alloc::{Layout, GlobalAlloc, System};
use std::ptr::NonNull;
use crate::data::{get_memory_manager, MemoryStats};

/// Comprehensive memory profiler for detecting leaks, tracking usage patterns,
/// and providing detailed allocation analysis.
#[derive(Debug)]
pub struct MemoryProfiler {
    /// Allocation tracking
    allocation_tracker: Arc<RwLock<AllocationTracker>>,
    /// Leak detector
    leak_detector: Arc<Mutex<LeakDetector>>,
    /// Usage pattern analyzer
    pattern_analyzer: Arc<Mutex<UsagePatternAnalyzer>>,
    /// Profiler configuration
    config: ProfilerConfig,
    /// Profiling state
    state: Arc<RwLock<ProfilerState>>,
}

impl MemoryProfiler {
    /// Create new memory profiler with default configuration
    pub fn new() -> Self {
        Self::with_config(ProfilerConfig::default())
    }
    
    /// Create memory profiler with custom configuration
    pub fn with_config(config: ProfilerConfig) -> Self {
        Self {
            allocation_tracker: Arc::new(RwLock::new(AllocationTracker::new())),
            leak_detector: Arc::new(Mutex::new(LeakDetector::new(config.leak_detection_threshold))),
            pattern_analyzer: Arc::new(Mutex::new(UsagePatternAnalyzer::new())),
            config,
            state: Arc::new(RwLock::new(ProfilerState::new())),
        }
    }
    
    /// Start memory profiling
    pub fn start_profiling(&self) -> Result<(), ProfilerError> {
        let mut state = self.state.write().unwrap();
        if state.is_running {
            return Err(ProfilerError::AlreadyRunning);
        }
        
        state.is_running = true;
        state.start_time = Instant::now();
        state.total_sessions += 1;
        
        // Start background monitoring thread if enabled
        if self.config.enable_background_monitoring {
            self.start_background_monitoring();
        }
        
        Ok(())
    }
    
    /// Stop memory profiling
    pub fn stop_profiling(&self) -> ProfilingReport {
        let mut state = self.state.write().unwrap();
        state.is_running = false;
        let session_duration = state.start_time.elapsed();
        
        // Generate comprehensive report
        self.generate_report(session_duration)
    }
    
    /// Record memory allocation for tracking
    pub fn record_allocation(&self, size: usize, layout: Layout, ptr: NonNull<u8>) {
        if !self.state.read().unwrap().is_running {
            return;
        }
        
        let allocation = AllocationRecord {
            ptr: ptr.as_ptr() as usize,
            size,
            layout,
            timestamp: Instant::now(),
            thread_id: thread::current().id(),
            backtrace: if self.config.capture_backtraces {
                Some(Backtrace::capture().to_string())
            } else {
                None
            },
        };
        
        // Track the allocation
        self.allocation_tracker.write().unwrap().record_allocation(allocation.clone());
        
        // Update pattern analysis
        self.pattern_analyzer.lock().unwrap().record_allocation(&allocation);
    }
    
    /// Record memory deallocation
    pub fn record_deallocation(&self, ptr: NonNull<u8>, layout: Layout) {
        if !self.state.read().unwrap().is_running {
            return;
        }
        
        let ptr_addr = ptr.as_ptr() as usize;
        let timestamp = Instant::now();
        
        // Remove from allocation tracker
        if let Some(allocation) = self.allocation_tracker.write().unwrap().record_deallocation(ptr_addr) {
            // Update pattern analysis with allocation lifetime
            let lifetime = timestamp.duration_since(allocation.timestamp);
            self.pattern_analyzer.lock().unwrap().record_deallocation(&allocation, lifetime);
            
            // Update leak detector
            self.leak_detector.lock().unwrap().record_deallocation(ptr_addr);
        }
    }
    
    /// Perform leak detection analysis
    pub fn detect_leaks(&self) -> LeakAnalysisReport {
        let tracker = self.allocation_tracker.read().unwrap();
        let mut detector = self.leak_detector.lock().unwrap();
        
        detector.analyze_leaks(&tracker.get_active_allocations(), self.config.leak_detection_threshold)
    }
    
    /// Get current memory usage snapshot
    pub fn get_memory_snapshot(&self) -> MemorySnapshot {
        let tracker = self.allocation_tracker.read().unwrap();
        let pattern_analyzer = self.pattern_analyzer.lock().unwrap();
        let manager = get_memory_manager();
        let memory_stats = manager.get_stats();
        
        MemorySnapshot {
            timestamp: Instant::now(),
            total_allocations: tracker.total_allocations(),
            active_allocations: tracker.active_allocations(),
            total_allocated_bytes: tracker.total_allocated_bytes(),
            active_allocated_bytes: tracker.active_allocated_bytes(),
            peak_allocated_bytes: tracker.peak_allocated_bytes(),
            allocation_rate: pattern_analyzer.get_allocation_rate(),
            deallocation_rate: pattern_analyzer.get_deallocation_rate(),
            memory_manager_stats: memory_stats,
        }
    }
    
    /// Generate detailed profiling report
    fn generate_report(&self, session_duration: Duration) -> ProfilingReport {
        let tracker = self.allocation_tracker.read().unwrap();
        let pattern_analyzer = self.pattern_analyzer.lock().unwrap();
        let leak_report = self.detect_leaks();
        let memory_snapshot = self.get_memory_snapshot();
        
        ProfilingReport {
            session_duration,
            memory_snapshot,
            leak_analysis: leak_report,
            allocation_patterns: pattern_analyzer.get_patterns_summary(),
            hotspots: tracker.get_allocation_hotspots(10),
            recommendations: self.generate_recommendations(&tracker, &pattern_analyzer),
        }
    }
    
    /// Generate optimization recommendations based on profiling data
    fn generate_recommendations(&self, tracker: &AllocationTracker, analyzer: &UsagePatternAnalyzer) -> Vec<MemoryRecommendation> {
        let mut recommendations = Vec::new();
        
        // Check for frequent small allocations
        let small_alloc_ratio = analyzer.get_small_allocation_ratio();
        if small_alloc_ratio > 0.7 {
            recommendations.push(MemoryRecommendation {
                category: RecommendationCategory::Pooling,
                severity: RecommendationSeverity::High,
                description: format!(
                    "High ratio of small allocations ({:.1}%). Consider using memory pools.",
                    small_alloc_ratio * 100.0
                ),
                estimated_impact: ImpactLevel::High,
            });
        }
        
        // Check for memory fragmentation
        let fragmentation_score = analyzer.get_fragmentation_score();
        if fragmentation_score > 0.6 {
            recommendations.push(MemoryRecommendation {
                category: RecommendationCategory::Fragmentation,
                severity: RecommendationSeverity::Medium,
                description: format!(
                    "Potential memory fragmentation detected (score: {:.2}). Consider larger block allocations.",
                    fragmentation_score
                ),
                estimated_impact: ImpactLevel::Medium,
            });
        }
        
        // Check for long-lived allocations
        let long_lived_ratio = analyzer.get_long_lived_allocation_ratio();
        if long_lived_ratio > 0.3 {
            recommendations.push(MemoryRecommendation {
                category: RecommendationCategory::Lifecycle,
                severity: RecommendationSeverity::Low,
                description: format!(
                    "Many long-lived allocations ({:.1}%). Consider pre-allocation strategies.",
                    long_lived_ratio * 100.0
                ),
                estimated_impact: ImpactLevel::Medium,
            });
        }
        
        // Check allocation hotspots
        let hotspots = tracker.get_allocation_hotspots(5);
        if let Some(top_hotspot) = hotspots.first() {
            if top_hotspot.allocation_count > 1000 {
                recommendations.push(MemoryRecommendation {
                    category: RecommendationCategory::Hotspot,
                    severity: RecommendationSeverity::High,
                    description: format!(
                        "Allocation hotspot detected: {} allocations at same location. Consider optimizing this code path.",
                        top_hotspot.allocation_count
                    ),
                    estimated_impact: ImpactLevel::High,
                });
            }
        }
        
        recommendations
    }
    
    /// Start background monitoring thread
    fn start_background_monitoring(&self) {
        let tracker = self.allocation_tracker.clone();
        let leak_detector = self.leak_detector.clone();
        let state = self.state.clone();
        let monitoring_interval = self.config.background_monitoring_interval;
        
        thread::spawn(move || {
            while state.read().unwrap().is_running {
                thread::sleep(monitoring_interval);
                
                // Perform periodic leak detection
                let active_allocations = tracker.read().unwrap().get_active_allocations();
                let mut detector = leak_detector.lock().unwrap();
                detector.update_allocation_ages(&active_allocations);
                
                // Clean up old allocation records if configured
                if active_allocations.len() > 10000 {
                    tracker.write().unwrap().cleanup_old_records(1000);
                }
            }
        });
    }
}

impl Default for MemoryProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for memory profiler
#[derive(Debug, Clone)]
pub struct ProfilerConfig {
    /// Capture stack backtraces for allocations
    pub capture_backtraces: bool,
    /// Enable background monitoring thread
    pub enable_background_monitoring: bool,
    /// Interval for background monitoring
    pub background_monitoring_interval: Duration,
    /// Threshold for considering an allocation a potential leak (in seconds)
    pub leak_detection_threshold: Duration,
    /// Maximum number of allocation records to keep
    pub max_allocation_records: usize,
}

impl Default for ProfilerConfig {
    fn default() -> Self {
        Self {
            capture_backtraces: false, // Expensive, only enable when needed
            enable_background_monitoring: true,
            background_monitoring_interval: Duration::from_secs(5),
            leak_detection_threshold: Duration::from_secs(300), // 5 minutes
            max_allocation_records: 50000,
        }
    }
}

/// Tracks all memory allocations and deallocations
#[derive(Debug)]
pub struct AllocationTracker {
    /// Active allocations (ptr_addr -> AllocationRecord)
    active_allocations: HashMap<usize, AllocationRecord>,
    /// Historical allocation statistics
    total_allocations: usize,
    total_deallocations: usize,
    total_allocated_bytes: usize,
    total_deallocated_bytes: usize,
    peak_allocated_bytes: usize,
    /// Allocation hotspots (location -> count)
    hotspots: HashMap<String, usize>,
}

impl AllocationTracker {
    fn new() -> Self {
        Self {
            active_allocations: HashMap::new(),
            total_allocations: 0,
            total_deallocations: 0,
            total_allocated_bytes: 0,
            total_deallocated_bytes: 0,
            peak_allocated_bytes: 0,
            hotspots: HashMap::new(),
        }
    }
    
    fn record_allocation(&mut self, allocation: AllocationRecord) {
        let ptr_addr = allocation.ptr;
        let size = allocation.size;
        
        self.active_allocations.insert(ptr_addr, allocation.clone());
        self.total_allocations += 1;
        self.total_allocated_bytes += size;
        
        // Update peak usage
        let current_active = self.active_allocated_bytes();
        if current_active > self.peak_allocated_bytes {
            self.peak_allocated_bytes = current_active;
        }
        
        // Track hotspot
        if let Some(backtrace) = &allocation.backtrace {
            let location = self.extract_allocation_location(backtrace);
            *self.hotspots.entry(location).or_default() += 1;
        }
    }
    
    fn record_deallocation(&mut self, ptr_addr: usize) -> Option<AllocationRecord> {
        if let Some(allocation) = self.active_allocations.remove(&ptr_addr) {
            self.total_deallocations += 1;
            self.total_deallocated_bytes += allocation.size;
            Some(allocation)
        } else {
            None
        }
    }
    
    fn get_active_allocations(&self) -> Vec<AllocationRecord> {
        self.active_allocations.values().cloned().collect()
    }
    
    fn total_allocations(&self) -> usize {
        self.total_allocations
    }
    
    fn active_allocations(&self) -> usize {
        self.active_allocations.len()
    }
    
    fn total_allocated_bytes(&self) -> usize {
        self.total_allocated_bytes
    }
    
    fn active_allocated_bytes(&self) -> usize {
        self.active_allocations.values().map(|a| a.size).sum()
    }
    
    fn peak_allocated_bytes(&self) -> usize {
        self.peak_allocated_bytes
    }
    
    fn get_allocation_hotspots(&self, limit: usize) -> Vec<AllocationHotspot> {
        let mut hotspots: Vec<_> = self.hotspots
            .iter()
            .map(|(location, count)| AllocationHotspot {
                location: location.clone(),
                allocation_count: *count,
            })
            .collect();
        
        hotspots.sort_by(|a, b| b.allocation_count.cmp(&a.allocation_count));
        hotspots.truncate(limit);
        hotspots
    }
    
    fn cleanup_old_records(&mut self, keep_count: usize) {
        if self.active_allocations.len() <= keep_count {
            return;
        }
        
        // Keep the most recent allocations
        let mut sorted_allocations: Vec<_> = self.active_allocations.values().collect();
        sorted_allocations.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        let to_keep: std::collections::HashSet<usize> = sorted_allocations
            .into_iter()
            .take(keep_count)
            .map(|a| a.ptr)
            .collect();
        
        self.active_allocations.retain(|&ptr, _| to_keep.contains(&ptr));
    }
    
    fn extract_allocation_location(&self, backtrace: &String) -> String {
        // Extract the first few frames of the backtrace to identify allocation location
        let backtrace_str = format!("{}", backtrace);
        let lines: Vec<&str> = backtrace_str.lines().take(3).collect();
        lines.join("; ")
    }
}

/// Individual allocation record
#[derive(Debug, Clone)]
pub struct AllocationRecord {
    pub ptr: usize,
    pub size: usize,
    pub layout: Layout,
    pub timestamp: Instant,
    pub thread_id: thread::ThreadId,
    pub backtrace: Option<String>, // Store backtrace as string to make it cloneable
}

/// Detects potential memory leaks
#[derive(Debug)]
pub struct LeakDetector {
    /// Threshold for considering an allocation a leak
    leak_threshold: Duration,
    /// Suspected leaks (ptr_addr -> first_seen)
    suspected_leaks: HashMap<usize, Instant>,
}

impl LeakDetector {
    fn new(leak_threshold: Duration) -> Self {
        Self {
            leak_threshold,
            suspected_leaks: HashMap::new(),
        }
    }
    
    fn analyze_leaks(&mut self, active_allocations: &[AllocationRecord], threshold: Duration) -> LeakAnalysisReport {
        let now = Instant::now();
        let mut potential_leaks = Vec::new();
        let mut leak_patterns = HashMap::new();
        
        for allocation in active_allocations {
            let age = now.duration_since(allocation.timestamp);
            
            if age >= threshold {
                // Track this as a potential leak
                self.suspected_leaks.entry(allocation.ptr)
                    .or_insert(allocation.timestamp);
                
                let leak_info = LeakInfo {
                    allocation: allocation.clone(),
                    age,
                    suspected_since: *self.suspected_leaks.get(&allocation.ptr).unwrap(),
                };
                
                potential_leaks.push(leak_info);
                
                // Analyze leak patterns
                if let Some(backtrace) = &allocation.backtrace {
                    let location = self.extract_leak_location(backtrace);
                    *leak_patterns.entry(location).or_default() += 1;
                }
            }
        }
        
        LeakAnalysisReport {
            total_suspected_leaks: potential_leaks.len(),
            total_leaked_bytes: potential_leaks.iter().map(|l| l.allocation.size).sum(),
            potential_leaks,
            leak_patterns,
            analysis_timestamp: now,
        }
    }
    
    fn record_deallocation(&mut self, ptr_addr: usize) {
        self.suspected_leaks.remove(&ptr_addr);
    }
    
    fn update_allocation_ages(&mut self, active_allocations: &[AllocationRecord]) {
        let now = Instant::now();
        
        // Clean up suspected leaks that have been deallocated
        let active_ptrs: std::collections::HashSet<usize> = 
            active_allocations.iter().map(|a| a.ptr).collect();
        
        self.suspected_leaks.retain(|&ptr, _| active_ptrs.contains(&ptr));
    }
    
    fn extract_leak_location(&self, backtrace: &String) -> String {
        // Similar to allocation tracker, but focus on the actual leak source
        let backtrace_str = format!("{}", backtrace);
        let lines: Vec<&str> = backtrace_str.lines()
            .filter(|line| !line.contains("alloc") && !line.contains("ruviz::data"))
            .take(2)
            .collect();
        lines.join("; ")
    }
}

/// Analyzes memory usage patterns
#[derive(Debug)]
pub struct UsagePatternAnalyzer {
    /// Allocation size distribution
    size_histogram: BTreeMap<usize, usize>,
    /// Allocation rate tracking (allocations per second)
    allocation_times: VecDeque<Instant>,
    /// Deallocation rate tracking
    deallocation_times: VecDeque<Instant>,
    /// Lifetime statistics
    lifetimes: Vec<Duration>,
    /// Total processed allocations
    total_processed: usize,
}

impl UsagePatternAnalyzer {
    fn new() -> Self {
        Self {
            size_histogram: BTreeMap::new(),
            allocation_times: VecDeque::new(),
            deallocation_times: VecDeque::new(),
            lifetimes: Vec::new(),
            total_processed: 0,
        }
    }
    
    fn record_allocation(&mut self, allocation: &AllocationRecord) {
        // Update size histogram
        *self.size_histogram.entry(allocation.size).or_default() += 1;
        
        // Track allocation rate
        self.allocation_times.push_back(allocation.timestamp);
        // Clone the times to avoid borrow checker issues
        let mut times = self.allocation_times.clone();
        self.cleanup_old_times(&mut times);
        self.allocation_times = times;
        
        self.total_processed += 1;
    }
    
    fn record_deallocation(&mut self, allocation: &AllocationRecord, lifetime: Duration) {
        // Track deallocation rate
        self.deallocation_times.push_back(Instant::now());
        // Clone the times to avoid borrow checker issues
        let mut times = self.deallocation_times.clone();
        self.cleanup_old_times(&mut times);
        self.deallocation_times = times;
        
        // Record lifetime
        self.lifetimes.push(lifetime);
        
        // Keep only recent lifetimes to avoid unbounded growth
        if self.lifetimes.len() > 10000 {
            self.lifetimes.drain(0..5000);
        }
    }
    
    fn get_allocation_rate(&self) -> f64 {
        self.calculate_rate(&self.allocation_times)
    }
    
    fn get_deallocation_rate(&self) -> f64 {
        self.calculate_rate(&self.deallocation_times)
    }
    
    fn get_small_allocation_ratio(&self) -> f32 {
        let small_threshold = 64; // bytes
        let small_allocations: usize = self.size_histogram
            .iter()
            .filter_map(|(&size, &count)| if size <= small_threshold { Some(count) } else { None })
            .sum();
        
        let total_allocations: usize = self.size_histogram.values().sum();
        
        if total_allocations == 0 {
            0.0
        } else {
            small_allocations as f32 / total_allocations as f32
        }
    }
    
    fn get_fragmentation_score(&self) -> f32 {
        // Simple fragmentation heuristic based on size distribution variance
        if self.size_histogram.is_empty() {
            return 0.0;
        }
        
        let sizes: Vec<usize> = self.size_histogram.keys().copied().collect();
        let mean_size = sizes.iter().sum::<usize>() as f32 / sizes.len() as f32;
        
        let variance: f32 = sizes.iter()
            .map(|&size| {
                let diff = size as f32 - mean_size;
                diff * diff
            })
            .sum::<f32>() / sizes.len() as f32;
        
        // Normalize to 0-1 range
        (variance.sqrt() / mean_size).min(1.0)
    }
    
    fn get_long_lived_allocation_ratio(&self) -> f32 {
        if self.lifetimes.is_empty() {
            return 0.0;
        }
        
        let long_lived_threshold = Duration::from_secs(60); // 1 minute
        let long_lived_count = self.lifetimes
            .iter()
            .filter(|&&lifetime| lifetime >= long_lived_threshold)
            .count();
        
        long_lived_count as f32 / self.lifetimes.len() as f32
    }
    
    fn get_patterns_summary(&self) -> UsagePatterns {
        UsagePatterns {
            allocation_rate: self.get_allocation_rate(),
            deallocation_rate: self.get_deallocation_rate(),
            small_allocation_ratio: self.get_small_allocation_ratio(),
            fragmentation_score: self.get_fragmentation_score(),
            long_lived_ratio: self.get_long_lived_allocation_ratio(),
            average_lifetime: self.calculate_average_lifetime(),
            size_distribution: self.size_histogram.clone(),
        }
    }
    
    fn calculate_rate(&self, times: &VecDeque<Instant>) -> f64 {
        if times.len() < 2 {
            return 0.0;
        }
        
        let duration = times.back().unwrap().duration_since(*times.front().unwrap());
        if duration.as_secs_f64() == 0.0 {
            return 0.0;
        }
        
        times.len() as f64 / duration.as_secs_f64()
    }
    
    fn calculate_average_lifetime(&self) -> Duration {
        if self.lifetimes.is_empty() {
            return Duration::ZERO;
        }
        
        let total_millis: u128 = self.lifetimes
            .iter()
            .map(|d| d.as_millis())
            .sum();
        
        Duration::from_millis((total_millis / self.lifetimes.len() as u128) as u64)
    }
    
    fn cleanup_old_times(&self, times: &mut VecDeque<Instant>) {
        let cutoff = Instant::now() - Duration::from_secs(60); // Keep last minute
        while let Some(&front_time) = times.front() {
            if front_time < cutoff {
                times.pop_front();
            } else {
                break;
            }
        }
    }
}

/// Profiler state
#[derive(Debug)]
struct ProfilerState {
    is_running: bool,
    start_time: Instant,
    total_sessions: usize,
}

impl ProfilerState {
    fn new() -> Self {
        Self {
            is_running: false,
            start_time: Instant::now(),
            total_sessions: 0,
        }
    }
}

// Report structures

/// Comprehensive profiling report
#[derive(Debug)]
pub struct ProfilingReport {
    pub session_duration: Duration,
    pub memory_snapshot: MemorySnapshot,
    pub leak_analysis: LeakAnalysisReport,
    pub allocation_patterns: UsagePatterns,
    pub hotspots: Vec<AllocationHotspot>,
    pub recommendations: Vec<MemoryRecommendation>,
}

/// Memory usage snapshot
#[derive(Debug)]
pub struct MemorySnapshot {
    pub timestamp: Instant,
    pub total_allocations: usize,
    pub active_allocations: usize,
    pub total_allocated_bytes: usize,
    pub active_allocated_bytes: usize,
    pub peak_allocated_bytes: usize,
    pub allocation_rate: f64,
    pub deallocation_rate: f64,
    pub memory_manager_stats: MemoryStats,
}

/// Leak analysis report
#[derive(Debug)]
pub struct LeakAnalysisReport {
    pub total_suspected_leaks: usize,
    pub total_leaked_bytes: usize,
    pub potential_leaks: Vec<LeakInfo>,
    pub leak_patterns: HashMap<String, usize>,
    pub analysis_timestamp: Instant,
}

/// Individual leak information
#[derive(Debug)]
pub struct LeakInfo {
    pub allocation: AllocationRecord,
    pub age: Duration,
    pub suspected_since: Instant,
}

/// Usage pattern analysis
#[derive(Debug)]
pub struct UsagePatterns {
    pub allocation_rate: f64,
    pub deallocation_rate: f64,
    pub small_allocation_ratio: f32,
    pub fragmentation_score: f32,
    pub long_lived_ratio: f32,
    pub average_lifetime: Duration,
    pub size_distribution: BTreeMap<usize, usize>,
}

/// Allocation hotspot
#[derive(Debug)]
pub struct AllocationHotspot {
    pub location: String,
    pub allocation_count: usize,
}

/// Memory optimization recommendation
#[derive(Debug)]
pub struct MemoryRecommendation {
    pub category: RecommendationCategory,
    pub severity: RecommendationSeverity,
    pub description: String,
    pub estimated_impact: ImpactLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecommendationCategory {
    Pooling,
    Fragmentation,
    Lifecycle,
    Hotspot,
    Leak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecommendationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImpactLevel {
    Low,
    Medium,
    High,
}

/// Profiler errors
#[derive(Debug, Clone)]
pub enum ProfilerError {
    AlreadyRunning,
    NotRunning,
    InvalidConfiguration,
}

impl std::fmt::Display for ProfilerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProfilerError::AlreadyRunning => write!(f, "Profiler is already running"),
            ProfilerError::NotRunning => write!(f, "Profiler is not running"),
            ProfilerError::InvalidConfiguration => write!(f, "Invalid profiler configuration"),
        }
    }
}

impl std::error::Error for ProfilerError {}

/// Global memory profiler instance
static MEMORY_PROFILER: std::sync::OnceLock<MemoryProfiler> = std::sync::OnceLock::new();

/// Get the global memory profiler
pub fn get_memory_profiler() -> &'static MemoryProfiler {
    MEMORY_PROFILER.get_or_init(MemoryProfiler::new)
}

/// Initialize memory profiler with custom configuration
pub fn initialize_memory_profiler(config: ProfilerConfig) -> Result<(), String> {
    MEMORY_PROFILER.set(MemoryProfiler::with_config(config))
        .map_err(|_| "Memory profiler already initialized".to_string())
}

/// Profiling allocator wrapper for automatic tracking
#[derive(Debug)]
pub struct ProfilingAllocator<A: GlobalAlloc> {
    inner: A,
}

impl<A: GlobalAlloc> ProfilingAllocator<A> {
    pub fn new(inner: A) -> Self {
        Self { inner }
    }
}

unsafe impl<A: GlobalAlloc> GlobalAlloc for ProfilingAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { self.inner.alloc(layout) };
        if !ptr.is_null() {
            if let Some(non_null_ptr) = NonNull::new(ptr) {
                get_memory_profiler().record_allocation(layout.size(), layout, non_null_ptr);
            }
        }
        ptr
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if let Some(non_null_ptr) = NonNull::new(ptr) {
            get_memory_profiler().record_deallocation(non_null_ptr, layout);
        }
        unsafe { self.inner.dealloc(ptr, layout) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_lifecycle() {
        let profiler = MemoryProfiler::new();
        
        // Test starting
        assert!(profiler.start_profiling().is_ok());
        assert!(profiler.state.read().unwrap().is_running);
        
        // Test can't start twice
        assert!(matches!(profiler.start_profiling(), Err(ProfilerError::AlreadyRunning)));
        
        // Test stopping
        let _report = profiler.stop_profiling();
        assert!(!profiler.state.read().unwrap().is_running);
    }

    #[test]
    fn test_allocation_tracking() {
        let profiler = MemoryProfiler::new();
        profiler.start_profiling().unwrap();
        
        let layout = Layout::new::<u64>();
        let ptr = NonNull::new(0x1000 as *mut u8).unwrap();
        
        profiler.record_allocation(8, layout, ptr);
        
        let snapshot = profiler.get_memory_snapshot();
        assert_eq!(snapshot.active_allocations, 1);
        assert_eq!(snapshot.active_allocated_bytes, 8);
        
        profiler.record_deallocation(ptr, layout);
        
        let snapshot2 = profiler.get_memory_snapshot();
        assert_eq!(snapshot2.active_allocations, 0);
        assert_eq!(snapshot2.active_allocated_bytes, 0);
    }

    #[test]
    fn test_leak_detection() {
        let profiler = MemoryProfiler::with_config(ProfilerConfig {
            leak_detection_threshold: Duration::from_millis(100),
            ..ProfilerConfig::default()
        });
        
        profiler.start_profiling().unwrap();
        
        let layout = Layout::new::<u64>();
        let ptr = NonNull::new(0x1000 as *mut u8).unwrap();
        
        profiler.record_allocation(8, layout, ptr);
        
        // Wait for leak threshold
        thread::sleep(Duration::from_millis(150));
        
        let leak_report = profiler.detect_leaks();
        assert_eq!(leak_report.total_suspected_leaks, 1);
        assert_eq!(leak_report.total_leaked_bytes, 8);
    }

    #[test]
    fn test_usage_patterns() {
        let mut analyzer = UsagePatternAnalyzer::new();
        
        // Record some allocations
        let layout = Layout::new::<u64>();
        for i in 0..10 {
            let allocation = AllocationRecord {
                ptr: i,
                size: if i < 5 { 32 } else { 1024 }, // Mix of small and large
                layout,
                timestamp: Instant::now(),
                thread_id: thread::current().id(),
                backtrace: None,
            };
            analyzer.record_allocation(&allocation);
        }
        
        let patterns = analyzer.get_patterns_summary();
        assert_eq!(patterns.small_allocation_ratio, 0.5); // 50% small allocations
        assert!(patterns.fragmentation_score > 0.0); // Should have some fragmentation
    }

    #[test]
    fn test_profiler_config() {
        let config = ProfilerConfig {
            capture_backtraces: true,
            leak_detection_threshold: Duration::from_secs(10),
            ..ProfilerConfig::default()
        };
        
        let profiler = MemoryProfiler::with_config(config);
        assert_eq!(profiler.config.leak_detection_threshold, Duration::from_secs(10));
        assert!(profiler.config.capture_backtraces);
    }
}