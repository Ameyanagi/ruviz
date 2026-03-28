//! Test actual wgpu 29 API compatibility

#[tokio::main]
async fn main() {
    println!("🧪 Testing wgpu 29 API...");

    // Create instance
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        flags: wgpu::InstanceFlags::default(),
        memory_budget_thresholds: Default::default(),
        backend_options: Default::default(),
        display: None,
    });
    println!("✅ Instance created");

    // Test enumerate_adapters
    let adapters: Vec<_> = instance.enumerate_adapters(wgpu::Backends::all()).await;
    println!(
        "✅ enumerate_adapters() called and returned {} adapters",
        adapters.len()
    );

    if let Some(adapter) = adapters.first() {
        let info = adapter.get_info();
        println!("   First adapter: {} ({:?})", info.name, info.device_type);

        // Test device creation
        match adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Test Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
        {
            Ok((device, _queue)) => {
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
