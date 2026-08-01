//! GPU backend integration tests

#[cfg(feature = "gpu")]
mod gpu_tests {
    use ruviz::render::gpu::*;

    fn gpu_is_required() -> bool {
        std::env::var_os("RUVIZ_REQUIRE_GPU_TESTS").is_some()
    }

    async fn backend_or_skip() -> Option<GpuBackend> {
        match GpuBackend::new().await {
            Ok(backend) => Some(backend),
            Err(error) if gpu_is_required() => {
                panic!("RUVIZ_REQUIRE_GPU_TESTS requires a working adapter: {error}")
            }
            Err(error) => {
                println!("⚠️  GPU not available (expected outside required-adapter CI): {error}");
                None
            }
        }
    }

    #[tokio::test]
    async fn test_gpu_backend_initialization() {
        // Test GPU backend can be created
        match GpuBackend::new().await {
            Ok(backend) => {
                println!("✅ GPU backend initialized successfully");
                println!("Device: {}", backend.capabilities().max_texture_size);

                // Test basic functionality
                assert!(backend.is_available());

                let stats = backend.get_stats();
                println!("GPU Stats: {:?}", stats.device_info);
            }
            Err(e) => {
                println!("⚠️  GPU not available (expected in CI): {}", e);
                // This is expected in CI environments without GPU
            }
        }
    }

    #[tokio::test]
    async fn test_compute_manager() {
        match GpuBackend::new().await {
            Ok(backend) => {
                if backend.capabilities().supports_compute {
                    match backend.create_compute_manager() {
                        Ok(mut compute) => {
                            println!("✅ Compute manager created");

                            // Test pipeline creation
                            if let Err(e) = compute.create_transform_pipeline() {
                                println!("⚠️  Transform pipeline failed: {}", e);
                            }

                            if let Err(e) = compute.create_aggregation_pipeline() {
                                println!("⚠️  Aggregation pipeline failed: {}", e);
                            }

                            let stats = compute.get_stats();
                            println!("Compute stats: {:?}", stats);
                        }
                        Err(e) => println!("⚠️  Compute manager failed: {}", e),
                    }
                } else {
                    println!("⚠️  Compute shaders not supported on this device");
                }
            }
            Err(e) => {
                println!("⚠️  GPU backend not available: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_buffer_management() {
        match GpuBackend::new().await {
            Ok(backend) => {
                let buffer_manager = backend.buffer_manager();
                let stats = {
                    let manager = buffer_manager.lock().unwrap();
                    manager.get_stats()
                };

                println!("✅ Buffer manager accessible");
                println!(
                    "Buffer stats: total_memory={}, active_buffers={}",
                    stats.total_memory, stats.active_buffers
                );

                assert_eq!(stats.active_buffers, 0); // Should start empty
            }
            Err(e) => {
                println!("⚠️  GPU backend not available: {}", e);
            }
        }
    }

    #[test]
    fn test_buffer_usage_conversion() {
        use ruviz::render::gpu::BufferUsage;

        // Test buffer usage to wgpu usage conversion
        let static_usage = BufferUsage::Static.to_wgpu_usage();
        let dynamic_usage = BufferUsage::Dynamic.to_wgpu_usage();
        let compute_usage = BufferUsage::Compute.to_wgpu_usage();

        assert!(static_usage.contains(wgpu::BufferUsages::VERTEX));
        assert!(dynamic_usage.contains(wgpu::BufferUsages::UNIFORM));
        assert!(compute_usage.contains(wgpu::BufferUsages::STORAGE));

        println!("✅ Buffer usage conversions work correctly");
    }

    #[tokio::test]
    async fn test_render_pass_creation() {
        match GpuBackend::new().await {
            Ok(backend) => {
                match backend.create_render_pass(800, 600, wgpu::TextureFormat::Rgba8Unorm) {
                    Ok(render_pass) => {
                        println!("✅ Render pass created successfully");
                        assert_eq!(render_pass.dimensions(), (800, 600));
                        assert_eq!(render_pass.format(), wgpu::TextureFormat::Rgba8Unorm);
                    }
                    Err(e) => println!("⚠️  Render pass creation failed: {}", e),
                }
            }
            Err(e) => {
                println!("⚠️  GPU backend not available: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn required_adapter_exercises_buffer_readback_and_pool_accounting() {
        let Some(backend) = backend_or_skip().await else {
            return;
        };
        let manager = backend.buffer_manager();
        let payload = [3_u8, 1, 4, 1, 5, 9, 2, 6];

        let (handle, mut buffer) = {
            let mut manager = manager.lock().unwrap();
            manager
                .create_with_data(
                    backend.device(),
                    &payload,
                    BufferUsage::Download,
                    "required-adapter readback",
                )
                .expect("download buffer should be created")
        };
        assert_eq!(buffer.map_read().await.unwrap(), payload);
        assert!(!buffer.is_mapped());

        {
            let mut manager = manager.lock().unwrap();
            assert_eq!(manager.get_memory_usage(), payload.len() as u64);
            manager.deallocate(handle).unwrap();
            assert_eq!(
                manager.get_memory_usage(),
                payload.len() as u64,
                "cached buffers remain physically allocated"
            );

            let reused = manager
                .allocate(
                    backend.device(),
                    payload.len() as u64,
                    BufferUsage::Download,
                    "required-adapter reuse",
                )
                .unwrap();
            assert_eq!(manager.get_memory_usage(), payload.len() as u64);
            assert_eq!(manager.get_stats().reused_buffers, 1);
            manager.deallocate(reused).unwrap();
        }

        let typed_pool = GpuMemoryPool::new(
            backend.device().device().clone(),
            backend.device().queue().clone(),
            backend.capabilities(),
        )
        .unwrap();
        let typed_buffer = typed_pool
            .create_buffer(
                &payload,
                wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            )
            .unwrap();
        let tracked_bytes = typed_pool.get_stats().total_allocated;
        assert!(tracked_bytes >= payload.len() as u64);
        drop(typed_buffer);
        assert_eq!(typed_pool.get_stats().total_allocated, 0);

        let cached_buffer = typed_pool
            .create_buffer(
                &payload,
                wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            )
            .unwrap();
        typed_pool.return_buffer(cached_buffer);
        assert_eq!(typed_pool.get_stats().total_allocated, tracked_bytes);
        typed_pool.clear_cache();
        assert_eq!(typed_pool.get_stats().total_allocated, 0);
    }

    #[tokio::test]
    async fn required_adapter_rejects_impossible_feature_sets() {
        if !gpu_is_required() {
            return;
        }
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let selector = DeviceSelector::new().require_features(wgpu::Features::all());
        assert!(selector.select_adapter(&instance).await.is_err());
    }
}

#[cfg(not(feature = "gpu"))]
mod no_gpu_tests {
    #[test]
    fn test_gpu_feature_disabled() {
        println!("ℹ️  GPU feature not enabled - skipping GPU tests");
    }
}
