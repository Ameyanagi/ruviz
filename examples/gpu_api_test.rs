//! Test actual wgpu 22.1 API compatibility

use pollster;

#[tokio::main]
async fn main() {
    println!("🧪 Testing wgpu 22.1 API...");

    // Create instance
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    println!("✅ Instance created");

    // Test enumerate_adapters - it returns Vec directly
    let adapters: Vec<_> = instance.enumerate_adapters(wgpu::Backends::all());
    println!(
        "✅ enumerate_adapters() called and returned {} adapters",
        adapters.len()
    );

    if let Some(adapter) = adapters.first() {
        let info = adapter.get_info();
        println!("   First adapter: {} ({:?})", info.name, info.device_type);

        // Test device creation
        match adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Test Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
        {
            Ok((device, queue)) => {
                println!("✅ Device and queue created successfully");

                // Test basic buffer creation
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("test buffer"),
                    size: 256,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                println!("✅ Buffer created: {} bytes", buffer.size());

                // Test features
                println!("Device features: {:?}", device.features());
                println!(
                    "Device limits: max_buffer_size = {}",
                    device.limits().max_buffer_size
                );
            }
            Err(e) => {
                println!("❌ Device creation failed: {}", e);
            }
        }
    } else {
        println!("❌ No adapters found");
    }

    println!("🎯 API test completed");
}
