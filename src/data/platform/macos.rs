//! macOS-specific memory optimization implementations

use crate::core::error::PlottingError;
use std::ffi::CStr;
use std::mem;
use std::ptr;

// macOS sysctl constants
const CTL_HW: i32 = 6;
const HW_MEMSIZE: i32 = 24;
const HW_USERMEM: i32 = 25;
const CTL_VM: i32 = 2;
const VM_SWAPUSAGE: i32 = 5;

#[repr(C)]
struct VMSwapUsage {
    used: u64,
    total: u64,
    encrypted: u32,
}

extern "C" {
    fn sysctl(
        name: *const i32,
        namelen: u32,
        oldp: *mut std::ffi::c_void,
        oldlenp: *mut usize,
        newp: *mut std::ffi::c_void,
        newlen: usize,
    ) -> i32;
    
    fn sysctlbyname(
        name: *const i8,
        oldp: *mut std::ffi::c_void,
        oldlenp: *mut usize,
        newp: *mut std::ffi::c_void,
        newlen: usize,
    ) -> i32;
    
    fn mach_host_self() -> u32;
    fn host_statistics64(
        host_priv: u32,
        flavor: i32,
        host_info_out: *mut std::ffi::c_void,
        host_info_outCnt: *mut u32,
    ) -> i32;
}

// Mach constants
const HOST_VM_INFO64: i32 = 4;
const HOST_VM_INFO64_COUNT: u32 = mem::size_of::<VMStatistics64>() as u32 / 4;

#[repr(C)]
struct VMStatistics64 {
    free_count: u32,
    active_count: u32,
    inactive_count: u32,
    wire_count: u32,
    zero_fill_count: u64,
    reactivations: u64,
    pageins: u64,
    pageouts: u64,
    faults: u64,
    cow_faults: u64,
    lookups: u64,
    hits: u64,
    purges: u64,
    purgeable_count: u32,
    speculative_count: u32,
    decompressions: u64,
    compressions: u64,
    swapins: u64,
    swapouts: u64,
    compressor_page_count: u32,
    throttled_count: u32,
    external_page_count: u32,
    internal_page_count: u32,
    total_uncompressed_pages_in_compressor: u64,
}

/// Get total system memory on macOS using sysctl
pub fn get_total_memory() -> Result<u64, PlottingError> {
    unsafe {
        let mut mem_size: u64 = 0;
        let mut size = mem::size_of::<u64>();
        let mut mib = [CTL_HW, HW_MEMSIZE];
        
        let result = sysctl(
            mib.as_ptr(),
            2,
            &mut mem_size as *mut u64 as *mut std::ffi::c_void,
            &mut size,
            ptr::null_mut(),
            0,
        );
        
        if result == 0 {
            Ok(mem_size)
        } else {
            Err(PlottingError::SystemError("Failed to get total memory via sysctl".to_string()))
        }
    }
}

/// Get available memory on macOS by calculating from VM statistics
pub fn get_available_memory() -> Result<u64, PlottingError> {
    unsafe {
        let mut vm_stat: VMStatistics64 = mem::zeroed();
        let mut count = HOST_VM_INFO64_COUNT;
        
        let result = host_statistics64(
            mach_host_self(),
            HOST_VM_INFO64,
            &mut vm_stat as *mut VMStatistics64 as *mut std::ffi::c_void,
            &mut count,
        );
        
        if result == 0 {
            let page_size = get_page_size() as u64;
            
            // Available = free + inactive + speculative + purgeable - compressor
            let available_pages = vm_stat.free_count as u64
                + vm_stat.inactive_count as u64
                + vm_stat.speculative_count as u64
                + vm_stat.purgeable_count as u64;
            
            Ok(available_pages * page_size)
        } else {
            Err(PlottingError::SystemError("Failed to get VM statistics".to_string()))
        }
    }
}

/// Get detailed VM statistics on macOS
pub fn get_vm_statistics() -> Result<MacOSVMStats, PlottingError> {
    unsafe {
        let mut vm_stat: VMStatistics64 = mem::zeroed();
        let mut count = HOST_VM_INFO64_COUNT;
        
        let result = host_statistics64(
            mach_host_self(),
            HOST_VM_INFO64,
            &mut vm_stat as *mut VMStatistics64 as *mut std::ffi::c_void,
            &mut count,
        );
        
        if result == 0 {
            let page_size = get_page_size() as u64;
            
            Ok(MacOSVMStats {
                free_pages: vm_stat.free_count as u64,
                active_pages: vm_stat.active_count as u64,
                inactive_pages: vm_stat.inactive_count as u64,
                wired_pages: vm_stat.wire_count as u64,
                compressed_pages: vm_stat.compressor_page_count as u64,
                speculative_pages: vm_stat.speculative_count as u64,
                purgeable_pages: vm_stat.purgeable_count as u64,
                external_pages: vm_stat.external_page_count as u64,
                internal_pages: vm_stat.internal_page_count as u64,
                total_uncompressed_pages_in_compressor: vm_stat.total_uncompressed_pages_in_compressor,
                page_size,
                compressions: vm_stat.compressions,
                decompressions: vm_stat.decompressions,
                swapins: vm_stat.swapins,
                swapouts: vm_stat.swapouts,
            })
        } else {
            Err(PlottingError::SystemError("Failed to get VM statistics".to_string()))
        }
    }
}

/// Get swap usage on macOS
pub fn get_swap_usage() -> Result<SwapUsage, PlottingError> {
    unsafe {
        let mut swap_usage: VMSwapUsage = mem::zeroed();
        let mut size = mem::size_of::<VMSwapUsage>();
        let mut mib = [CTL_VM, VM_SWAPUSAGE];
        
        let result = sysctl(
            mib.as_ptr(),
            2,
            &mut swap_usage as *mut VMSwapUsage as *mut std::ffi::c_void,
            &mut size,
            ptr::null_mut(),
            0,
        );
        
        if result == 0 {
            Ok(SwapUsage {
                used: swap_usage.used,
                total: swap_usage.total,
                encrypted: swap_usage.encrypted != 0,
            })
        } else {
            Err(PlottingError::SystemError("Failed to get swap usage".to_string()))
        }
    }
}

/// Get memory pressure level on macOS
pub fn get_memory_pressure() -> Result<MemoryPressureLevel, PlottingError> {
    // macOS provides memory pressure through dispatch sources, but we can approximate
    // it by looking at VM statistics
    let vm_stats = get_vm_statistics()?;
    let swap_usage = get_swap_usage().unwrap_or_default();
    
    let total_memory = get_total_memory()?;
    let available = vm_stats.free_pages * vm_stats.page_size;
    let memory_usage_ratio = 1.0 - (available as f64 / total_memory as f64);
    
    // Consider swap usage and compression activity
    let compression_ratio = if vm_stats.compressions > 0 {
        vm_stats.decompressions as f64 / vm_stats.compressions as f64
    } else {
        0.0
    };
    
    let pressure_level = if memory_usage_ratio > 0.95 || swap_usage.used > swap_usage.total / 2 {
        MemoryPressureLevel::Critical
    } else if memory_usage_ratio > 0.8 || compression_ratio > 2.0 {
        MemoryPressureLevel::High
    } else if memory_usage_ratio > 0.6 || compression_ratio > 1.0 {
        MemoryPressureLevel::Medium
    } else {
        MemoryPressureLevel::Low
    };
    
    Ok(pressure_level)
}

/// Get system page size on macOS
pub fn get_page_size() -> usize {
    unsafe {
        let mut page_size: i32 = 0;
        let mut size = mem::size_of::<i32>();
        
        let name = CStr::from_bytes_with_nul(b"hw.pagesize\0").unwrap();
        let result = sysctlbyname(
            name.as_ptr(),
            &mut page_size as *mut i32 as *mut std::ffi::c_void,
            &mut size,
            ptr::null_mut(),
            0,
        );
        
        if result == 0 && page_size > 0 {
            page_size as usize
        } else {
            4096 // Default page size
        }
    }
}

/// Check if large pages (super pages) are supported on macOS
pub fn check_large_page_support() -> bool {
    unsafe {
        let mut super_page_size: i32 = 0;
        let mut size = mem::size_of::<i32>();
        
        let name = CStr::from_bytes_with_nul(b"vm.superpages_size\0").unwrap();
        let result = sysctlbyname(
            name.as_ptr(),
            &mut super_page_size as *mut i32 as *mut std::ffi::c_void,
            &mut size,
            ptr::null_mut(),
            0,
        );
        
        result == 0 && super_page_size > 0
    }
}

/// Check if memory mapping is supported on macOS
pub fn check_memory_mapping_support() -> Result<bool, PlottingError> {
    // On macOS, memory mapping is always supported
    Ok(true)
}

/// Get CPU cache information on macOS
pub fn get_cache_info() -> Result<CacheInfo, PlottingError> {
    unsafe {
        let mut l1_cache_size: i32 = 0;
        let mut l2_cache_size: i32 = 0;
        let mut l3_cache_size: i32 = 0;
        let mut cache_line_size: i32 = 0;
        let mut size = mem::size_of::<i32>();
        
        let names = [
            (CStr::from_bytes_with_nul(b"hw.l1dcachesize\0").unwrap(), &mut l1_cache_size),
            (CStr::from_bytes_with_nul(b"hw.l2cachesize\0").unwrap(), &mut l2_cache_size),
            (CStr::from_bytes_with_nul(b"hw.l3cachesize\0").unwrap(), &mut l3_cache_size),
            (CStr::from_bytes_with_nul(b"hw.cachelinesize\0").unwrap(), &mut cache_line_size),
        ];
        
        for (name, value) in names.iter() {
            sysctlbyname(
                name.as_ptr(),
                *value as *mut i32 as *mut std::ffi::c_void,
                &mut size,
                ptr::null_mut(),
                0,
            );
        }
        
        Ok(CacheInfo {
            l1_size: if l1_cache_size > 0 { Some(l1_cache_size as usize) } else { None },
            l2_size: if l2_cache_size > 0 { Some(l2_cache_size as usize) } else { None },
            l3_size: if l3_cache_size > 0 { Some(l3_cache_size as usize) } else { None },
            line_size: if cache_line_size > 0 { cache_line_size as usize } else { 64 },
        })
    }
}

/// Configure memory pressure handling on macOS
pub fn configure_memory_pressure_handling(enable_swap: bool) -> Result<(), PlottingError> {
    // On macOS, we can suggest VM parameters but most are read-only
    // This is more of a configuration hint for the application
    if !enable_swap {
        // Suggest disabling swap for performance-critical applications
        // This would typically be done at the application level
        return Ok(());
    }
    
    Ok(())
}

#[derive(Debug, Clone)]
pub struct MacOSVMStats {
    pub free_pages: u64,
    pub active_pages: u64,
    pub inactive_pages: u64,
    pub wired_pages: u64,
    pub compressed_pages: u64,
    pub speculative_pages: u64,
    pub purgeable_pages: u64,
    pub external_pages: u64,
    pub internal_pages: u64,
    pub total_uncompressed_pages_in_compressor: u64,
    pub page_size: u64,
    pub compressions: u64,
    pub decompressions: u64,
    pub swapins: u64,
    pub swapouts: u64,
}

#[derive(Debug, Clone, Default)]
pub struct SwapUsage {
    pub used: u64,
    pub total: u64,
    pub encrypted: bool,
}

#[derive(Debug, Clone)]
pub enum MemoryPressureLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub struct CacheInfo {
    pub l1_size: Option<usize>,
    pub l2_size: Option<usize>,
    pub l3_size: Option<usize>,
    pub line_size: usize,
}

/// Get thermal state (relevant for performance scaling)
pub fn get_thermal_state() -> Result<ThermalState, PlottingError> {
    unsafe {
        let mut thermal_state: i32 = 0;
        let mut size = mem::size_of::<i32>();
        
        let name = CStr::from_bytes_with_nul(b"machdep.xcpm.cpu_thermal_level\0").unwrap();
        let result = sysctlbyname(
            name.as_ptr(),
            &mut thermal_state as *mut i32 as *mut std::ffi::c_void,
            &mut size,
            ptr::null_mut(),
            0,
        );
        
        if result == 0 {
            match thermal_state {
                0 => Ok(ThermalState::Normal),
                1 => Ok(ThermalState::Fair),
                2 => Ok(ThermalState::Serious),
                3 => Ok(ThermalState::Critical),
                _ => Ok(ThermalState::Unknown),
            }
        } else {
            Ok(ThermalState::Unknown)
        }
    }
}

#[derive(Debug, Clone)]
pub enum ThermalState {
    Normal,
    Fair,
    Serious,
    Critical,
    Unknown,
}

/// macOS-specific memory optimization recommendations
pub fn get_macos_optimization_recommendations(vm_stats: &MacOSVMStats) -> Vec<String> {
    let mut recommendations = Vec::new();
    
    let total_pages = vm_stats.free_pages + vm_stats.active_pages + 
                     vm_stats.inactive_pages + vm_stats.wired_pages;
    
    if total_pages == 0 {
        return recommendations;
    }
    
    let memory_pressure = (vm_stats.active_pages + vm_stats.wired_pages) as f64 / total_pages as f64;
    
    if memory_pressure > 0.8 {
        recommendations.push("High memory pressure detected. Consider reducing memory usage.".to_string());
    }
    
    let compression_ratio = if vm_stats.compressions > 0 {
        vm_stats.decompressions as f64 / vm_stats.compressions as f64
    } else {
        0.0
    };
    
    if compression_ratio > 2.0 {
        recommendations.push("High memory compression activity. Consider increasing available memory.".to_string());
    }
    
    if vm_stats.swapouts > vm_stats.swapins * 2 {
        recommendations.push("Excessive swap activity detected. System may be memory constrained.".to_string());
    }
    
    if vm_stats.purgeable_pages > total_pages / 10 {
        recommendations.push("Large amount of purgeable memory available for reclamation.".to_string());
    }
    
    recommendations
}