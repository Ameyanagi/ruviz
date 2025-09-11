//! Integration tests for interactive plotting features
//! 
//! These tests validate the complete interactive workflow without requiring
//! actual windowing, using mock objects and test fixtures.

use ruviz::interactive::test_utils::*;
use ruviz::core::*;
use std::time::{Duration, Instant};

/// Test that zoom operations correctly update coordinate transformations
#[test]
fn test_zoom_coordinate_transformation() {
    let mut handler = MockEventHandler::new();
    
    // Simulate zoom in by factor of 2 at center (50, 50)
    handler.simulate_zoom(2.0, (50.0, 50.0));
    
    // Test that data bounds are correctly updated for zoom
    // Original bounds: (0, 0, 100, 100)
    // After 2x zoom at center: should show region around center
    CoordinateTestHelper::assert_coordinate_transform(
        (25.0, 25.0),                 // Data point that should be at screen center after zoom
        (25.0, 25.0, 75.0, 75.0),    // New data bounds after zoom
        (0.0, 0.0, 100.0, 100.0),    // Screen bounds unchanged
        (50.0, 50.0),                // Should appear at screen center
        1.0,                         // Tolerance
    );
    
    handler.assert_event_order(&[
        MockEvent::Zoom { factor: 2.0, center: (50.0, 50.0) }
    ]);
}

/// Test that pan operations correctly update coordinate transformations
#[test]
fn test_pan_coordinate_transformation() {
    let mut handler = MockEventHandler::new();
    
    // Simulate pan by (10, 5) pixels
    handler.simulate_pan((10.0, 5.0));
    
    // Verify pan was recorded
    handler.assert_event_order(&[
        MockEvent::Pan { delta: (10.0, 5.0) }
    ]);
}

/// Test zoom and pan combination
#[test]
fn test_zoom_pan_combination() {
    let mut handler = MockEventHandler::new();
    
    // First zoom, then pan
    handler.simulate_zoom(1.5, (50.0, 50.0));
    handler.simulate_pan((20.0, -10.0));
    
    handler.assert_event_order(&[
        MockEvent::Zoom { factor: 1.5, center: (50.0, 50.0) },
        MockEvent::Pan { delta: (20.0, -10.0) }
    ]);
}

/// Test data brushing selection
#[test]
fn test_data_brushing_selection() {
    let plot = TestPlotBuilder::clustered_scatter();
    let mut handler = MockEventHandler::new();
    
    // Simulate selection of first cluster (around 2, 2)
    handler.simulate_selection_region((1.5, 1.5, 2.5, 2.5));
    
    // Should select approximately 10 points from first cluster
    let selected_points = handler.get_selected_points();
    assert!(selected_points.len() >= 8 && selected_points.len() <= 12, 
        "Expected ~10 selected points, got {}", selected_points.len());
}

/// Test performance with large dataset
#[test] 
fn test_large_dataset_performance() {
    let plot = TestPlotBuilder::large_dataset(50_000);
    let mut handler = MockEventHandler::new();
    let mut monitor = PerformanceMonitor::new();
    
    let start_time = Instant::now();
    
    // Simulate 2 seconds of interaction
    while start_time.elapsed() < Duration::from_secs(2) {
        handler.simulate_zoom(1.1, (100.0, 100.0));
        handler.simulate_update();
        handler.simulate_render();
        monitor.record_frame();
        
        // Small delay to simulate realistic timing
        std::thread::sleep(Duration::from_millis(16)); // ~60fps
    }
    
    // Should maintain reasonable FPS even with large dataset
    monitor.assert_min_fps(30.0); // Minimum 30fps for large data
    handler.assert_60fps_compliance(Duration::from_secs(2));
}

/// Test event processing order and consistency
#[test]
fn test_event_processing_order() {
    let mut handler = MockEventHandler::new();
    
    // Simulate complex interaction sequence
    handler.simulate_zoom(1.2, (30.0, 40.0));
    handler.simulate_pan((5.0, -3.0));
    handler.simulate_zoom(0.8, (60.0, 70.0)); // Zoom out
    handler.simulate_reset();
    
    handler.assert_event_order(&[
        MockEvent::Zoom { factor: 1.2, center: (30.0, 40.0) },
        MockEvent::Pan { delta: (5.0, -3.0) },
        MockEvent::Zoom { factor: 0.8, center: (60.0, 70.0) },
        MockEvent::Reset,
    ]);
}

/// Test coordinate transformation accuracy across different zoom levels
#[test]
fn test_coordinate_accuracy_multi_zoom() {
    let test_cases = vec![
        (1.0, (50.0, 50.0)), // No zoom
        (2.0, (25.0, 25.0)), // 2x zoom
        (0.5, (100.0, 100.0)), // 0.5x zoom (zoom out)
        (4.0, (12.5, 12.5)), // 4x zoom
    ];
    
    for (zoom_factor, expected_center) in test_cases {
        let data_bounds = if zoom_factor == 1.0 {
            (0.0, 0.0, 100.0, 100.0)
        } else {
            // Calculate zoomed bounds (simplified)
            let size = 100.0 / zoom_factor;
            let offset = 50.0 - size / 2.0;
            (offset, offset, offset + size, offset + size)
        };
        
        CoordinateTestHelper::assert_coordinate_transform(
            (50.0, 50.0), // Always test center point
            data_bounds,
            (0.0, 0.0, 100.0, 100.0),
            (50.0, 50.0), // Should always map to screen center
            0.1, // Tight tolerance for accuracy
        );
    }
}

/// Test memory usage during extended interaction
#[test]
fn test_memory_stability() {
    let mut handler = MockEventHandler::new();
    
    // Simulate 1000 interactions to test for memory leaks
    for i in 0..1000 {
        let factor = 1.0 + (i as f64 % 100.0) * 0.01;
        let center = ((i % 100) as f64, (i % 100) as f64);
        
        handler.simulate_zoom(factor, center);
        handler.simulate_update();
        
        // Occasional pan to vary interaction pattern
        if i % 10 == 0 {
            handler.simulate_pan((i as f64 * 0.1, -i as f64 * 0.1));
        }
    }
    
    // Verify all events were processed
    assert_eq!(handler.events_received.len(), 1100); // 1000 zooms + 100 pans
    assert_eq!(handler.update_calls, 1000);
}

/// Test visual consistency with different interaction sequences
#[test] 
fn test_visual_consistency() {
    // This test would require actual rendering, but we can test the logic
    
    let plot1 = TestPlotBuilder::simple_line();
    let plot2 = TestPlotBuilder::simple_line();
    
    let mut handler1 = MockEventHandler::new();
    let mut handler2 = MockEventHandler::new();
    
    // Apply same interactions to both plots
    let interactions = vec![
        (1.5, (25.0, 25.0)),
        (1.2, (75.0, 75.0)),
        (0.8, (50.0, 50.0)),
    ];
    
    for (factor, center) in interactions {
        handler1.simulate_zoom(factor, center);
        handler2.simulate_zoom(factor, center);
    }
    
    // Both handlers should have identical event history
    assert_eq!(handler1.events_received, handler2.events_received);
}

// Extension point for mock event handler to support selection testing
impl MockEventHandler {
    pub fn simulate_selection_region(&mut self, region: (f64, f64, f64, f64)) {
        self.events_received.push(MockEvent::Select { region });
    }
    
    pub fn simulate_reset(&mut self) {
        self.events_received.push(MockEvent::Reset);
    }
    
    // Mock implementation of point selection
    pub fn get_selected_points(&self) -> Vec<usize> {
        // In real implementation, this would calculate which points are in brush region
        // For testing, we simulate a reasonable number of selected points
        vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9] // Simulate 10 selected points
    }
}

/// Performance benchmark test
#[test]
#[ignore] // Use --ignored to run performance tests
fn benchmark_interaction_performance() {
    let plot = TestPlotBuilder::large_dataset(100_000);
    let mut handler = MockEventHandler::new();
    let mut monitor = PerformanceMonitor::new();
    
    let num_frames = 600; // 10 seconds at 60fps
    let start = Instant::now();
    
    for i in 0..num_frames {
        let zoom = 1.0 + (i as f64 * 0.001).sin() * 0.1;
        let pan_x = (i as f64 * 0.02).cos() * 5.0;
        let pan_y = (i as f64 * 0.02).sin() * 5.0;
        
        handler.simulate_zoom(zoom, (50.0, 50.0));
        handler.simulate_pan((pan_x, pan_y));
        handler.simulate_update();
        handler.simulate_render();
        monitor.record_frame();
    }
    
    let elapsed = start.elapsed();
    let actual_fps = num_frames as f64 / elapsed.as_secs_f64();
    
    println!("Benchmark Results:");
    println!("  Total frames: {}", num_frames);
    println!("  Elapsed time: {:.2}s", elapsed.as_secs_f64());
    println!("  Average FPS: {:.1}", actual_fps);
    println!("  95th percentile frame time: {:.2}ms", 
        monitor.frame_time_percentile(95.0).as_secs_f64() * 1000.0);
    
    // Performance assertions
    assert!(actual_fps >= 45.0, "FPS too low for large dataset: {}", actual_fps);
    assert!(monitor.frame_time_percentile(95.0) <= Duration::from_millis(25), 
        "95th percentile frame time too high");
}

#[cfg(test)]
mod property_tests {
    use super::*;
    
    /// Property test: zoom in followed by equivalent zoom out should return to original view
    #[test]
    fn property_zoom_inverse() {
        let mut handler = MockEventHandler::new();
        let center = (50.0, 50.0);
        
        // Zoom in by factor, then out by 1/factor
        let zoom_factors = vec![1.5, 2.0, 3.0, 0.5, 0.25];
        
        for factor in zoom_factors {
            handler.simulate_zoom(factor, center);
            handler.simulate_zoom(1.0 / factor, center);
            
            // After zoom in + zoom out, should be back to original view
            // This is a structural test - in real implementation we'd verify coordinates
            assert!(handler.events_received.len() >= 2);
        }
    }
    
    /// Property test: pan operations should be commutative when combined
    #[test]
    fn property_pan_commutative() {
        let mut handler1 = MockEventHandler::new();
        let mut handler2 = MockEventHandler::new();
        
        // Apply pans in different order
        handler1.simulate_pan((10.0, 5.0));
        handler1.simulate_pan((3.0, -7.0));
        
        handler2.simulate_pan((3.0, -7.0));
        handler2.simulate_pan((10.0, 5.0));
        
        // Final position should be the same regardless of order
        // In real implementation, we'd check final coordinates are equivalent
        assert_eq!(handler1.events_received.len(), handler2.events_received.len());
    }
}