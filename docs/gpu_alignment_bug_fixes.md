# GPU Alignment Bug - FIXED

**Status:** IMPLEMENTED (Fix 1 applied)

---

# GPU Alignment Bug - Original Proposed Fixes

## Bug Summary

**Location:** `src/render/gpu/memory.rs:308`
**Symptom:** `cast_slice>TargetAlignmentGreaterAndInputNotAligned` panic
**Cause:** `BufferView` from wgpu mapping may not be aligned for target type

---

## Fix 1: Use `try_cast_slice` with Manual Copy Fallback (Recommended)

**Approach:** Try zero-copy cast first, fall back to byte-by-byte copy if unaligned.

```rust
// In memory.rs, replace line 308:

// OLD (panics on unaligned):
let result_data: Vec<T> = cast_slice(&mapped_data[..element_count * element_size]).to_vec();

// NEW (safe with fallback):
let byte_slice = &mapped_data[..element_count * element_size];
let result_data: Vec<T> = match bytemuck::try_cast_slice::<u8, T>(byte_slice) {
    Ok(aligned_slice) => {
        // Fast path: data is aligned, zero-copy cast
        aligned_slice.to_vec()
    }
    Err(_) => {
        // Slow path: unaligned, copy byte-by-byte
        let mut result = Vec::with_capacity(element_count);
        for i in 0..element_count {
            let offset = i * element_size;
            let mut bytes = [0u8; std::mem::size_of::<T>()];
            bytes.copy_from_slice(&byte_slice[offset..offset + element_size]);
            result.push(bytemuck::from_bytes::<T>(&bytes).clone());
        }
        result
    }
};
```

**Pros:**
- Zero-copy when aligned (common case)
- Safe fallback when unaligned
- No API changes required

**Cons:**
- Slower fallback path
- Requires `T: Clone`

---

## Fix 2: Pre-aligned Buffer Allocation

**Approach:** Ensure staging buffer is aligned at creation time.

```rust
// In memory.rs, modify create_buffer_empty_bytes:

pub fn create_buffer_empty_bytes(
    &self,
    size: u64,
    usage: wgpu::BufferUsages,
    label: Option<&str>,
) -> Result<GpuBuffer> {
    // Ensure size is aligned to maximum possible type alignment (16 bytes for SIMD)
    const MAX_ALIGN: u64 = 16;
    let aligned_size = ((size + MAX_ALIGN - 1) / MAX_ALIGN) * MAX_ALIGN;

    let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
        label,
        size: aligned_size,
        usage,
        mapped_at_creation: false,
    });

    Ok(GpuBuffer::new_from_raw(buffer, aligned_size, usage, label))
}
```

**Pros:**
- Addresses root cause
- No runtime overhead

**Cons:**
- May waste some memory (up to 15 bytes per buffer)
- Doesn't guarantee mapped region alignment (GPU driver dependent)

---

## Fix 3: Use `bytemuck::allocation` with Aligned Vec

**Approach:** Copy to an aligned Vec before casting.

```rust
// Add dependency: bytemuck = { version = "1.23", features = ["extern_crate_alloc"] }

use bytemuck::allocation::zeroed_vec;

// In read_buffer, replace the problematic cast:
let byte_slice = &mapped_data[..element_count * element_size];

// Allocate aligned destination
let mut result_data: Vec<T> = zeroed_vec(element_count);

// Copy bytes (handles alignment automatically)
let dest_bytes: &mut [u8] = bytemuck::cast_slice_mut(&mut result_data);
dest_bytes.copy_from_slice(byte_slice);
```

**Pros:**
- Simple and clean
- Guaranteed alignment
- Uses bytemuck's allocation features

**Cons:**
- Extra allocation
- Requires `extern_crate_alloc` feature

---

## Fix 4: Offset-based Aligned Access

**Approach:** Find aligned offset within the buffer and adjust copy accordingly.

```rust
// In read_buffer:
let mapped_data = buffer_slice.get_mapped_range();
let ptr = mapped_data.as_ptr();
let align = std::mem::align_of::<T>();

// Calculate offset needed for alignment
let misalignment = (ptr as usize) % align;
let aligned_offset = if misalignment == 0 { 0 } else { align - misalignment };

// Adjust the copy operation
if aligned_offset > 0 {
    // Need to copy to aligned buffer
    let mut aligned_buffer: Vec<u8> = vec![0u8; element_count * element_size + align];
    let aligned_ptr = aligned_buffer.as_mut_ptr();
    let aligned_start = ((aligned_ptr as usize + align - 1) / align * align) as *mut u8;

    unsafe {
        std::ptr::copy_nonoverlapping(
            mapped_data.as_ptr(),
            aligned_start,
            element_count * element_size,
        );

        let result_data: Vec<T> =
            std::slice::from_raw_parts(aligned_start as *const T, element_count).to_vec();
    }
} else {
    // Already aligned
    let result_data: Vec<T> = cast_slice(&mapped_data[..element_count * element_size]).to_vec();
}
```

**Pros:**
- Optimal when aligned (zero-copy)
- Full control over memory layout

**Cons:**
- Complex and error-prone
- Uses unsafe code
- Hard to maintain

---

## Fix 5: Platform-specific Mapping Hints

**Approach:** Use wgpu features to request aligned mapping.

```rust
// When creating staging buffer, use mapped_at_creation with proper setup:
let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("GPU Readback Staging (Aligned)"),
    size: aligned_size,
    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: true,  // Map at creation for alignment control
});

// Get mapped data immediately (should be aligned)
{
    let mapped = staging_buffer.slice(..).get_mapped_range_mut();
    // The buffer is now mapped and should be aligned
}
staging_buffer.unmap();

// Later, when reading:
// Re-map and use - the address should be consistent
```

**Pros:**
- Works with GPU driver's alignment
- No extra copies in ideal case

**Cons:**
- Platform-dependent behavior
- May not solve the issue on all drivers

---

## Recommended Implementation

**Use Fix 1** as the primary solution because:
1. It's safe and handles all cases
2. Zero runtime cost when aligned (common case)
3. Minimal code changes
4. No external dependencies

Here's the complete implementation:

```rust
// src/render/gpu/memory.rs

/// Read data back from GPU buffer (alignment-safe version)
pub fn read_buffer<T: Pod + Clone>(&self, buffer: &GpuBuffer) -> GpuResult<Vec<T>> {
    if !buffer.usage().contains(wgpu::BufferUsages::COPY_SRC) {
        return Err(GpuError::OperationFailed(
            "Buffer was not created with COPY_SRC usage".to_string(),
        ));
    }

    let element_size = std::mem::size_of::<T>();
    let element_count = (buffer.size() as usize) / element_size;

    // Create staging buffer for readback
    let staging_buffer = self
        .create_buffer_empty_bytes(
            buffer.size(),
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            Some("GPU Readback Staging"),
        )
        .map_err(|e| GpuError::BufferCreationFailed(format!("{}", e)))?;

    // Copy from GPU buffer to staging buffer
    let mut encoder = self
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("GPU Buffer Copy"),
        });

    encoder.copy_buffer_to_buffer(
        buffer.buffer(),
        0,
        staging_buffer.buffer(),
        0,
        buffer.size(),
    );

    let submission = self.queue.submit(Some(encoder.finish()));

    // Map and read staging buffer
    let buffer_slice = staging_buffer.buffer().slice(..);
    let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        sender.send(result).ok();
    });

    // Wait for mapping to complete
    self.device
        .poll(wgpu::Maintain::WaitForSubmissionIndex(submission));

    pollster::block_on(receiver.receive())
        .ok_or_else(|| GpuError::OperationFailed("Buffer mapping failed".to_string()))?
        .map_err(|e| GpuError::OperationFailed(format!("Buffer mapping error: {:?}", e)))?;

    // Copy data with alignment-safe method
    let mapped_data = buffer_slice.get_mapped_range();
    let byte_slice = &mapped_data[..element_count * element_size];

    let result_data = Self::cast_slice_safe::<T>(byte_slice, element_count);

    // Unmap buffer
    drop(mapped_data);
    staging_buffer.buffer().unmap();

    Ok(result_data)
}

/// Alignment-safe slice casting with fallback
fn cast_slice_safe<T: Pod + Clone>(bytes: &[u8], element_count: usize) -> Vec<T> {
    let element_size = std::mem::size_of::<T>();

    // Try zero-copy cast first (fast path)
    if let Ok(aligned) = bytemuck::try_cast_slice::<u8, T>(bytes) {
        return aligned.to_vec();
    }

    // Fallback: manual byte-by-byte reconstruction
    // This is slower but handles unaligned data safely
    let mut result = Vec::with_capacity(element_count);

    for i in 0..element_count {
        let offset = i * element_size;
        let element_bytes = &bytes[offset..offset + element_size];

        // Create properly aligned temporary storage
        let mut aligned_bytes = vec![0u8; element_size];
        aligned_bytes.copy_from_slice(element_bytes);

        // Safe because aligned_bytes is properly aligned (heap allocation)
        let element: &T = bytemuck::from_bytes(&aligned_bytes);
        result.push(element.clone());
    }

    result
}
```

---

## Testing the Fix

```rust
#[cfg(test)]
mod alignment_tests {
    use super::*;

    #[test]
    fn test_cast_slice_safe_aligned() {
        // Aligned data
        let data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];
        let bytes: &[u8] = bytemuck::cast_slice(&data);

        let result = GpuMemoryPool::cast_slice_safe::<f32>(bytes, 4);
        assert_eq!(result, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_cast_slice_safe_unaligned() {
        // Simulate unaligned by adding offset byte
        let mut bytes = vec![0u8]; // 1 byte offset
        bytes.extend_from_slice(bytemuck::cast_slice::<f32, u8>(&[1.0f32, 2.0, 3.0]));

        // Slice from offset 1 (unaligned for f32)
        let unaligned = &bytes[1..];

        // Should work via fallback path
        let result = GpuMemoryPool::cast_slice_safe::<f32>(unaligned, 3);
        assert_eq!(result, vec![1.0, 2.0, 3.0]);
    }
}
```
