//! FreeBSD-specific memory optimization implementations

use crate::core::error::PlottingError;
use std::process::Command;

/// Get total system memory on FreeBSD using sysctl
pub fn get_total_memory() -> Result<u64, PlottingError> {
    let output = Command::new("sysctl")
        .arg("-n")
        .arg("hw.physmem")
        .output()
        .map_err(|e| PlottingError::SystemError(format!("Failed to run sysctl: {}", e)))?;

    let mem_str = String::from_utf8_lossy(&output.stdout);
    mem_str
        .trim()
        .parse::<u64>()
        .map_err(|e| PlottingError::SystemError(format!("Failed to parse memory size: {}", e)))
}

/// Get available system memory on FreeBSD
pub fn get_available_memory() -> Result<u64, PlottingError> {
    // Get page size
    let pagesize = get_pagesize()?;

    // Get free pages
    let free_pages = get_sysctl_u64("vm.stats.vm.v_free_count")?;

    // Get inactive pages (can be reclaimed)
    let inactive_pages = get_sysctl_u64("vm.stats.vm.v_inactive_count")?;

    // Get cache pages (can be reclaimed)
    let cache_pages = get_sysctl_u64("vm.stats.vm.v_cache_count").unwrap_or(0); // Cache queue removed in FreeBSD 12+

    let available_pages = free_pages + inactive_pages + cache_pages;
    Ok(available_pages * pagesize)
}

/// Get page size from sysctl
fn get_pagesize() -> Result<u64, PlottingError> {
    get_sysctl_u64("hw.pagesize")
}

/// Helper function to get u64 value from sysctl
fn get_sysctl_u64(name: &str) -> Result<u64, PlottingError> {
    let output = Command::new("sysctl")
        .arg("-n")
        .arg(name)
        .output()
        .map_err(|e| PlottingError::SystemError(format!("Failed to run sysctl {}: {}", name, e)))?;

    let value_str = String::from_utf8_lossy(&output.stdout);
    value_str
        .trim()
        .parse::<u64>()
        .map_err(|e| PlottingError::SystemError(format!("Failed to parse sysctl {}: {}", name, e)))
}

/// Get number of NUMA nodes on FreeBSD
pub fn get_numa_nodes() -> usize {
    // Try to get NUMA domain count
    if let Ok(domains) = get_sysctl_u64("vm.ndomains") {
        if domains > 0 {
            return domains as usize;
        }
    }

    // Fallback: check CPU topology
    if let Ok(output) = Command::new("sysctl")
        .arg("-n")
        .arg("kern.sched.topology_spec")
        .output()
    {
        let topology = String::from_utf8_lossy(&output.stdout);
        // Parse topology XML-like structure for NUMA domains
        let domain_count = topology.matches("<group level=\"1\"").count();
        if domain_count > 1 {
            return domain_count;
        }
    }

    1 // Default to single NUMA node
}

/// Check if huge pages (superpages) are supported on FreeBSD
pub fn check_hugepage_support() -> bool {
    // Check if superpage support is enabled
    if let Ok(enabled) = get_sysctl_u64("vm.pmap.pg_ps_enabled") {
        return enabled != 0;
    }

    // Alternative check for older FreeBSD versions
    if let Ok(output) = Command::new("sysctl").arg("vm.pmap").output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        return output_str.contains("superpage") || output_str.contains("pagesizes");
    }

    false
}

/// Check if memory mapping is supported on FreeBSD
pub fn check_memory_mapping_support() -> Result<bool, PlottingError> {
    // On FreeBSD, memory mapping is always supported
    Ok(true)
}

/// Get huge page (superpage) information
pub fn get_hugepage_info() -> Option<HugepageInfo> {
    let mut info = HugepageInfo::default();

    // Get superpage size
    if let Ok(output) = Command::new("sysctl")
        .arg("-n")
        .arg("vm.pmap.pagesizes")
        .output()
    {
        let sizes_str = String::from_utf8_lossy(&output.stdout);
        // Format is typically "4096 2097152" (4KB, 2MB)
        let sizes: Vec<u64> = sizes_str
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        if sizes.len() > 1 {
            info.page_size_kb = sizes[1] / 1024; // Convert to KB
        }
    }

    // Get superpage statistics
    if let Ok(promotions) = get_sysctl_u64("vm.stats.vm.v_page_promotions") {
        info.total_pages = promotions;
    }

    // Check if we got meaningful data
    if info.page_size_kb > 0 || info.total_pages > 0 {
        Some(info)
    } else {
        None
    }
}

/// Configure transparent huge pages (superpages on FreeBSD)
pub fn configure_transparent_hugepages(enabled: bool) -> Result<(), PlottingError> {
    let value = if enabled { "1" } else { "0" };

    let output = Command::new("sysctl")
        .arg(format!("vm.pmap.pg_ps_enabled={}", value))
        .output()
        .map_err(|e| {
            PlottingError::SystemError(format!("Failed to configure superpages: {}", e))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PlottingError::SystemError(format!(
            "Failed to configure superpages: {}",
            stderr
        )));
    }

    Ok(())
}

/// Get memory pressure information from vmstat
pub fn get_memory_pressure() -> Option<MemoryPressure> {
    let output = Command::new("vmstat").arg("-s").output().ok()?;

    let vmstat_str = String::from_utf8_lossy(&output.stdout);
    let mut pressure = MemoryPressure::default();

    // Parse vmstat output for memory pressure indicators
    for line in vmstat_str.lines() {
        let line = line.trim();

        if line.contains("pages paged out") {
            if let Some(num_str) = line.split_whitespace().next() {
                if let Ok(pages) = num_str.parse::<u64>() {
                    // Convert to a pressure percentage (simplified heuristic)
                    pressure.some_avg10 = (pages as f32 / 1000.0).min(100.0);
                }
            }
        }

        if line.contains("pages freed by daemon") {
            if let Some(num_str) = line.split_whitespace().next() {
                if let Ok(pages) = num_str.parse::<u64>() {
                    pressure.full_avg10 = (pages as f32 / 1000.0).min(100.0);
                }
            }
        }
    }

    // Get recent averages from vm.stats
    if let Ok(shortage) = get_sysctl_u64("vm.stats.vm.v_page_shortage") {
        pressure.some_avg60 = (shortage as f32 / 10.0).min(100.0);
    }

    Some(pressure)
}

/// Configure swappiness equivalent on FreeBSD
pub fn configure_swappiness(value: u32) -> Result<(), PlottingError> {
    if value > 100 {
        return Err(PlottingError::InvalidInput(
            "Swappiness must be between 0 and 100".to_string(),
        ));
    }

    // FreeBSD uses vm.swap_idle_threshold2 (range 0-100)
    // Lower values = less aggressive swapping
    let output = Command::new("sysctl")
        .arg(format!("vm.swap_idle_threshold2={}", value))
        .output()
        .map_err(|e| {
            PlottingError::SystemError(format!("Failed to configure swap threshold: {}", e))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PlottingError::SystemError(format!(
            "Failed to configure swap threshold: {}",
            stderr
        )));
    }

    Ok(())
}

/// Get current swappiness value
pub fn get_swappiness() -> Result<u32, PlottingError> {
    get_sysctl_u64("vm.swap_idle_threshold2")
        .map(|v| v as u32)
        .or_else(|_| {
            // Fallback to swap_idle_threshold1 on older systems
            get_sysctl_u64("vm.swap_idle_threshold1").map(|v| v as u32)
        })
}

#[derive(Debug, Clone, Default)]
pub struct HugepageInfo {
    pub total_pages: u64,
    pub free_pages: u64,
    pub page_size_kb: u64,
}

#[derive(Debug, Clone, Default)]
pub struct MemoryPressure {
    pub some_avg10: f32, // Some pressure - 10 second average
    pub some_avg60: f32, // Some pressure - 60 second average
    pub full_avg10: f32, // Full pressure - 10 second average
    pub full_avg60: f32, // Full pressure - 60 second average
}

/// Get detailed memory statistics from sysctl
pub fn get_detailed_memory_stats() -> Result<DetailedMemoryStats, PlottingError> {
    let pagesize = get_pagesize()?;

    let mut stats = DetailedMemoryStats::default();

    // Get total memory
    stats.total = get_total_memory()?;

    // Get page counts
    let free_count = get_sysctl_u64("vm.stats.vm.v_free_count")?;
    let active_count = get_sysctl_u64("vm.stats.vm.v_active_count")?;
    let inactive_count = get_sysctl_u64("vm.stats.vm.v_inactive_count")?;
    let cache_count = get_sysctl_u64("vm.stats.vm.v_cache_count").unwrap_or(0);
    let wire_count = get_sysctl_u64("vm.stats.vm.v_wire_count")?;

    stats.free = free_count * pagesize;
    stats.available = (free_count + inactive_count + cache_count) * pagesize;

    // Calculate used memory
    let used_pages = active_count + wire_count;

    // Get buffer and cache info
    stats.buffers = get_sysctl_u64("vfs.bufspace").unwrap_or(0);
    stats.cached = cache_count * pagesize;

    // Get swap info
    if let Ok(swap_total) = get_sysctl_u64("vm.swap_total") {
        stats.swap_total = swap_total;
        if let Ok(swap_used) = get_sysctl_u64("vm.swap_reserved") {
            stats.swap_free = swap_total.saturating_sub(swap_used);
        }
    }

    // Get zone memory allocator stats
    stats.slab = wire_count * pagesize; // Approximation using wired memory

    // Get mapped memory
    if let Ok(resident) = get_sysctl_u64("vm.stats.vm.v_user_wire_count") {
        stats.mapped = resident * pagesize;
    }

    Ok(stats)
}

#[derive(Debug, Clone, Default)]
pub struct DetailedMemoryStats {
    pub total: u64,
    pub free: u64,
    pub available: u64,
    pub buffers: u64,
    pub cached: u64,
    pub swap_total: u64,
    pub swap_free: u64,
    pub slab: u64,
    pub slab_reclaimable: u64,
    pub slab_unreclaimable: u64,
    pub page_tables: u64,
    pub mapped: u64,
}

/// Get memory zone information (FreeBSD-specific)
pub fn get_memory_zones() -> Result<Vec<MemoryZone>, PlottingError> {
    let output = Command::new("vmstat")
        .arg("-z")
        .output()
        .map_err(|e| PlottingError::SystemError(format!("Failed to run vmstat -z: {}", e)))?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut zones = Vec::new();

    for line in output_str.lines().skip(1) {
        // Skip header
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 5 {
            zones.push(MemoryZone {
                name: parts[0].trim_end_matches(':').to_string(),
                size: parts[1].parse().unwrap_or(0),
                used: parts[3].parse().unwrap_or(0),
                free: parts[4].parse().unwrap_or(0),
            });
        }
    }

    Ok(zones)
}

#[derive(Debug, Clone)]
pub struct MemoryZone {
    pub name: String,
    pub size: u64,
    pub used: u64,
    pub free: u64,
}
