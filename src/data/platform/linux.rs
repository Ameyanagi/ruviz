//! Linux-specific memory optimization implementations

use crate::core::error::PlottingError;
use std::fs;
use std::io::Read;

/// Get total system memory on Linux by reading /proc/meminfo
pub fn get_total_memory() -> Result<u64, PlottingError> {
    let mut contents = String::new();
    fs::File::open("/proc/meminfo")
        .and_then(|mut f| f.read_to_string(&mut contents))
        .map_err(|e| PlottingError::SystemError(format!("Failed to read /proc/meminfo: {}", e)))?;
    
    for line in contents.lines() {
        if line.starts_with("MemTotal:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let kb = parts[1].parse::<u64>()
                    .map_err(|e| PlottingError::SystemError(format!("Failed to parse memory size: {}", e)))?;
                return Ok(kb * 1024); // Convert KB to bytes
            }
        }
    }
    
    Err(PlottingError::SystemError("Could not find MemTotal in /proc/meminfo".to_string()))
}

/// Get available system memory on Linux
pub fn get_available_memory() -> Result<u64, PlottingError> {
    let mut contents = String::new();
    fs::File::open("/proc/meminfo")
        .and_then(|mut f| f.read_to_string(&mut contents))
        .map_err(|e| PlottingError::SystemError(format!("Failed to read /proc/meminfo: {}", e)))?;
    
    // Try to get MemAvailable first (more accurate on modern kernels)
    for line in contents.lines() {
        if line.starts_with("MemAvailable:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let kb = parts[1].parse::<u64>()
                    .map_err(|e| PlottingError::SystemError(format!("Failed to parse memory size: {}", e)))?;
                return Ok(kb * 1024); // Convert KB to bytes
            }
        }
    }
    
    // Fallback: calculate from MemFree + Buffers + Cached
    let mut mem_free = 0u64;
    let mut buffers = 0u64;
    let mut cached = 0u64;
    
    for line in contents.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let value = parts[1].parse::<u64>().unwrap_or(0);
            match parts[0] {
                "MemFree:" => mem_free = value,
                "Buffers:" => buffers = value,
                "Cached:" => cached = value,
                _ => {}
            }
        }
    }
    
    Ok((mem_free + buffers + cached) * 1024) // Convert KB to bytes
}

/// Get number of NUMA nodes on Linux
pub fn get_numa_nodes() -> usize {
    // Check /sys/devices/system/node/ for NUMA topology
    if let Ok(entries) = fs::read_dir("/sys/devices/system/node") {
        let node_count = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.file_name()
                    .to_string_lossy()
                    .starts_with("node")
                    && entry.file_name()
                        .to_string_lossy()
                        .chars()
                        .skip(4)
                        .all(|c| c.is_ascii_digit())
            })
            .count();
        
        if node_count > 0 {
            return node_count;
        }
    }
    
    // Fallback: check if NUMA is mentioned in /proc/cpuinfo
    if let Ok(contents) = fs::read_to_string("/proc/cpuinfo") {
        // Simple heuristic: if we see multiple physical id entries, assume NUMA
        let physical_ids: std::collections::HashSet<_> = contents
            .lines()
            .filter(|line| line.starts_with("physical id"))
            .map(|line| line.split(':').nth(1).unwrap_or("").trim())
            .collect();
        
        if physical_ids.len() > 1 {
            return physical_ids.len();
        }
    }
    
    1 // Default to single NUMA node
}

/// Check if huge pages are supported on Linux
pub fn check_hugepage_support() -> bool {
    // Check /proc/meminfo for HugePages entries
    if let Ok(contents) = fs::read_to_string("/proc/meminfo") {
        for line in contents.lines() {
            if line.starts_with("HugePages_Total:") {
                return true;
            }
        }
    }
    
    // Check if huge page file systems are mounted
    if let Ok(contents) = fs::read_to_string("/proc/mounts") {
        for line in contents.lines() {
            if line.contains("hugetlbfs") {
                return true;
            }
        }
    }
    
    // Check /sys/kernel/mm/hugepages/ directory
    if fs::metadata("/sys/kernel/mm/hugepages").is_ok() {
        if let Ok(entries) = fs::read_dir("/sys/kernel/mm/hugepages") {
            return entries.count() > 0;
        }
    }
    
    false
}

/// Check if memory mapping is supported on Linux
pub fn check_memory_mapping_support() -> Result<bool, PlottingError> {
    // On Linux, memory mapping is always supported
    Ok(true)
}

/// Get huge page information
pub fn get_hugepage_info() -> Option<HugepageInfo> {
    let mut info = HugepageInfo::default();
    
    if let Ok(contents) = fs::read_to_string("/proc/meminfo") {
        for line in contents.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let value = parts[1].parse::<u64>().unwrap_or(0);
                match parts[0] {
                    "HugePages_Total:" => info.total_pages = value,
                    "HugePages_Free:" => info.free_pages = value,
                    "Hugepagesize:" => info.page_size_kb = value,
                    _ => {}
                }
            }
        }
    }
    
    if info.total_pages > 0 {
        Some(info)
    } else {
        None
    }
}

/// Configure transparent huge pages
pub fn configure_transparent_hugepages(enabled: bool) -> Result<(), PlottingError> {
    let thp_path = "/sys/kernel/mm/transparent_hugepage/enabled";
    
    if !fs::metadata(thp_path).is_ok() {
        return Err(PlottingError::SystemError(
            "Transparent huge pages not supported".to_string()
        ));
    }
    
    let value = if enabled { "always" } else { "never" };
    
    fs::write(thp_path, value)
        .map_err(|e| PlottingError::SystemError(
            format!("Failed to configure transparent huge pages: {}", e)
        ))
}

/// Get memory pressure information from /proc/pressure/memory (if available)
pub fn get_memory_pressure() -> Option<MemoryPressure> {
    if let Ok(contents) = fs::read_to_string("/proc/pressure/memory") {
        return parse_pressure_stats(&contents);
    }
    None
}

/// Parse pressure stall information format
fn parse_pressure_stats(contents: &str) -> Option<MemoryPressure> {
    let mut pressure = MemoryPressure::default();
    
    for line in contents.lines() {
        if line.starts_with("some ") {
            if let Some(avg10) = extract_avg_value(line, "avg10=") {
                pressure.some_avg10 = avg10;
            }
            if let Some(avg60) = extract_avg_value(line, "avg60=") {
                pressure.some_avg60 = avg60;
            }
        } else if line.starts_with("full ") {
            if let Some(avg10) = extract_avg_value(line, "avg10=") {
                pressure.full_avg10 = avg10;
            }
            if let Some(avg60) = extract_avg_value(line, "avg60=") {
                pressure.full_avg60 = avg60;
            }
        }
    }
    
    Some(pressure)
}

fn extract_avg_value(line: &str, prefix: &str) -> Option<f32> {
    if let Some(start) = line.find(prefix) {
        let start_idx = start + prefix.len();
        if let Some(end) = line[start_idx..].find(' ') {
            line[start_idx..start_idx + end].parse().ok()
        } else {
            line[start_idx..].parse().ok()
        }
    } else {
        None
    }
}

/// Check and configure swappiness
pub fn configure_swappiness(value: u32) -> Result<(), PlottingError> {
    if value > 100 {
        return Err(PlottingError::InvalidInput("Swappiness must be between 0 and 100".to_string()));
    }
    
    fs::write("/proc/sys/vm/swappiness", value.to_string())
        .map_err(|e| PlottingError::SystemError(
            format!("Failed to configure swappiness: {}", e)
        ))
}

/// Get current swappiness value
pub fn get_swappiness() -> Result<u32, PlottingError> {
    let contents = fs::read_to_string("/proc/sys/vm/swappiness")
        .map_err(|e| PlottingError::SystemError(format!("Failed to read swappiness: {}", e)))?;
    
    contents.trim().parse()
        .map_err(|e| PlottingError::SystemError(format!("Failed to parse swappiness: {}", e)))
}

#[derive(Debug, Clone, Default)]
pub struct HugepageInfo {
    pub total_pages: u64,
    pub free_pages: u64,
    pub page_size_kb: u64,
}

#[derive(Debug, Clone, Default)]
pub struct MemoryPressure {
    pub some_avg10: f32,  // Some pressure (any process waiting) - 10 second average
    pub some_avg60: f32,  // Some pressure - 60 second average
    pub full_avg10: f32,  // Full pressure (all non-idle processes waiting) - 10 second average
    pub full_avg60: f32,  // Full pressure - 60 second average
}

/// Get detailed memory statistics from /proc/meminfo
pub fn get_detailed_memory_stats() -> Result<DetailedMemoryStats, PlottingError> {
    let mut contents = String::new();
    fs::File::open("/proc/meminfo")
        .and_then(|mut f| f.read_to_string(&mut contents))
        .map_err(|e| PlottingError::SystemError(format!("Failed to read /proc/meminfo: {}", e)))?;
    
    let mut stats = DetailedMemoryStats::default();
    
    for line in contents.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let value_kb = parts[1].parse::<u64>().unwrap_or(0);
            let value_bytes = value_kb * 1024;
            
            match parts[0] {
                "MemTotal:" => stats.total = value_bytes,
                "MemFree:" => stats.free = value_bytes,
                "MemAvailable:" => stats.available = value_bytes,
                "Buffers:" => stats.buffers = value_bytes,
                "Cached:" => stats.cached = value_bytes,
                "SwapTotal:" => stats.swap_total = value_bytes,
                "SwapFree:" => stats.swap_free = value_bytes,
                "Slab:" => stats.slab = value_bytes,
                "SReclaimable:" => stats.slab_reclaimable = value_bytes,
                "SUnreclaim:" => stats.slab_unreclaimable = value_bytes,
                "PageTables:" => stats.page_tables = value_bytes,
                "Mapped:" => stats.mapped = value_bytes,
                _ => {}
            }
        }
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