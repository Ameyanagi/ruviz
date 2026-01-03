//! Quick test to verify wgpu API compatibility

#[tokio::main]
async fn main() {
    println!("Testing wgpu API compatibility...");

    // Test 1: Create instance
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        dx12_shader_compiler: Default::default(),
        flags: wgpu::InstanceFlags::default(),
        gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
    });
    println!("✅ Instance created");

    // Test 2: Enumerate adapters
    let adapters: Vec<_> = instance.enumerate_adapters(wgpu::Backends::all());
    println!("✅ Found {} adapters", adapters.len());

    if let Some(adapter) = adapters.first() {
        let info = adapter.get_info();
        println!("   First adapter: {} ({:?})", info.name, info.device_type);

        // Test 3: Request device
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
                println!("✅ Device and queue created");
                println!("   Device features: {:?}", device.features());
                println!("   Queue: {:?}", std::ptr::addr_of!(queue));
            }
            Err(e) => {
                println!("⚠️  Device creation failed: {}", e);
            }
        }
    } else {
        println!("❌ No adapters found");
    }
}
