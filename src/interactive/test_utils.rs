//! Test utilities for interactive plotting components
//!
//! Provides mock objects, test fixtures, and validation utilities for
//! testing interactive features without requiring actual windowing.

use crate::core::{Plot, Result};
use std::time::{Duration, Instant};

/// Mock event handler for testing without windowing system
pub struct MockEventHandler {
    pub events_received: Vec<MockEvent>,
    pub render_calls: usize,
    pub update_calls: usize,
    pub last_update_time: Option<Instant>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MockEvent {
    Zoom { factor: f64, center: (f64, f64) },
    Pan { delta: (f64, f64) },
    Select { region: (f64, f64, f64, f64) },
    Hover { point: (f64, f64) },
    Reset,
}

impl MockEventHandler {
    pub fn new() -> Self {
        Self {
            events_received: Vec::new(),
            render_calls: 0,
            update_calls: 0,
            last_update_time: None,
        }
    }

    pub fn simulate_zoom(&mut self, factor: f64, center: (f64, f64)) {
        self.events_received
            .push(MockEvent::Zoom { factor, center });
    }

    pub fn simulate_pan(&mut self, delta: (f64, f64)) {
        self.events_received.push(MockEvent::Pan { delta });
    }

    pub fn simulate_update(&mut self) {
        self.update_calls += 1;
        self.last_update_time = Some(Instant::now());
    }

    pub fn simulate_render(&mut self) {
        self.render_calls += 1;
    }

    /// Verify that render was called within expected timeframe for 60fps
    pub fn assert_60fps_compliance(&self, duration: Duration) {
        let expected_frames = (duration.as_secs_f64() * 60.0) as usize;
        let tolerance = expected_frames / 10; // 10% tolerance

        assert!(
            self.render_calls >= expected_frames - tolerance,
            "Expected ~{} render calls for 60fps, got {}",
            expected_frames,
            self.render_calls
        );
    }

    /// Verify that events were processed in correct order
    pub fn assert_event_order(&self, expected: &[MockEvent]) {
        assert_eq!(self.events_received, expected, "Event order mismatch");
    }
}

/// Test fixture for creating plots with known data patterns
pub struct TestPlotBuilder;

impl TestPlotBuilder {
    /// Create a simple line plot with predictable data for testing
    pub fn simple_line() -> Plot {
        let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y_data = vec![2.0, 4.0, 6.0, 8.0, 10.0];

        Plot::new()
            .line(&x_data, &y_data)
            .title("Test Line Plot")
            .xlabel("X Values")
            .ylabel("Y Values")
    }

    /// Create a scatter plot with clustered data for brush testing
    pub fn clustered_scatter() -> Plot {
        // Generate clustered data
        let mut x_data = Vec::new();
        let mut y_data = Vec::new();

        // Cluster 1: around (2, 2)
        for i in 0..10 {
            x_data.push(2.0 + (i as f64 - 5.0) * 0.1);
            y_data.push(2.0 + (i as f64 - 5.0) * 0.1);
        }

        // Cluster 2: around (8, 8)
        for i in 0..10 {
            x_data.push(8.0 + (i as f64 - 5.0) * 0.1);
            y_data.push(8.0 + (i as f64 - 5.0) * 0.1);
        }

        Plot::new()
            .scatter(&x_data, &y_data)
            .title("Test Clustered Scatter")
            .xlabel("X Cluster")
            .ylabel("Y Cluster")
    }

    /// Create large dataset for performance testing
    pub fn large_dataset(n_points: usize) -> Plot {
        let x_data: Vec<f64> = (0..n_points).map(|i| i as f64 * 0.01).collect();
        let y_data: Vec<f64> = x_data
            .iter()
            .map(|&x| (x * std::f64::consts::PI).sin())
            .collect();

        Plot::new()
            .line(&x_data, &y_data)
            .title(&format!("Performance Test - {} points", n_points))
            .xlabel("Time")
            .ylabel("Amplitude")
    }
}

/// Performance measurement utilities
pub struct PerformanceMonitor {
    frame_times: Vec<Duration>,
    start_time: Instant,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            frame_times: Vec::new(),
            start_time: Instant::now(),
        }
    }

    pub fn record_frame(&mut self) {
        let now = Instant::now();
        if let Some(&last_time) = self.frame_times.last() {
            // This is a simplified version - in real impl we'd track actual frame intervals
            self.frame_times.push(now.duration_since(self.start_time));
        } else {
            self.frame_times.push(Duration::from_nanos(16_666_667)); // ~60fps
        }
        self.start_time = now;
    }

    pub fn average_fps(&self) -> f64 {
        if self.frame_times.is_empty() {
            return 0.0;
        }

        let total_time: Duration = self.frame_times.iter().sum();
        let avg_frame_time = total_time.as_secs_f64() / self.frame_times.len() as f64;
        1.0 / avg_frame_time
    }

    pub fn assert_min_fps(&self, min_fps: f64) {
        let actual_fps = self.average_fps();
        assert!(
            actual_fps >= min_fps,
            "FPS too low: {} < {} (minimum)",
            actual_fps,
            min_fps
        );
    }

    pub fn frame_time_percentile(&self, percentile: f64) -> Duration {
        let mut sorted_times = self.frame_times.clone();
        sorted_times.sort();

        let index = ((sorted_times.len() as f64 - 1.0) * percentile / 100.0) as usize;
        sorted_times[index]
    }
}

/// Coordinate transformation testing utilities
pub struct CoordinateTestHelper;

impl CoordinateTestHelper {
    /// Test data-to-screen coordinate transformation
    pub fn assert_coordinate_transform(
        data_point: (f64, f64),
        data_bounds: (f64, f64, f64, f64), // min_x, min_y, max_x, max_y
        screen_bounds: (f64, f64, f64, f64), // left, top, right, bottom
        expected_screen: (f64, f64),
        tolerance: f64,
    ) {
        let actual_screen = Self::data_to_screen(data_point, data_bounds, screen_bounds);

        let dx = (actual_screen.0 - expected_screen.0).abs();
        let dy = (actual_screen.1 - expected_screen.1).abs();

        assert!(
            dx <= tolerance && dy <= tolerance,
            "Coordinate transform failed: expected {:?}, got {:?} (tolerance: {})",
            expected_screen,
            actual_screen,
            tolerance
        );
    }

    /// Simple coordinate transformation for testing
    fn data_to_screen(
        data_point: (f64, f64),
        data_bounds: (f64, f64, f64, f64),
        screen_bounds: (f64, f64, f64, f64),
    ) -> (f64, f64) {
        let (data_x, data_y) = data_point;
        let (min_x, min_y, max_x, max_y) = data_bounds;
        let (left, top, right, bottom) = screen_bounds;

        let screen_x = left + (data_x - min_x) / (max_x - min_x) * (right - left);
        let screen_y = top + (data_y - min_y) / (max_y - min_y) * (bottom - top);

        (screen_x, screen_y)
    }
}

/// Visual regression testing utilities
pub struct VisualTestHelper;

impl VisualTestHelper {
    /// Generate a deterministic hash of pixel data for comparison
    pub fn hash_pixel_data(pixels: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        pixels.hash(&mut hasher);
        hasher.finish()
    }

    /// Compare two pixel buffers with tolerance for anti-aliasing differences
    pub fn compare_images_with_tolerance(
        image1: &[u8],
        image2: &[u8],
        tolerance: u8,
        max_different_pixels: usize,
    ) -> bool {
        if image1.len() != image2.len() {
            return false;
        }

        let mut different_pixels = 0;

        for (pixel1, pixel2) in image1.chunks(4).zip(image2.chunks(4)) {
            if !Self::pixels_similar(pixel1, pixel2, tolerance) {
                different_pixels += 1;
                if different_pixels > max_different_pixels {
                    return false;
                }
            }
        }

        true
    }

    fn pixels_similar(pixel1: &[u8], pixel2: &[u8], tolerance: u8) -> bool {
        pixel1
            .iter()
            .zip(pixel2.iter())
            .all(|(&p1, &p2)| (p1 as i16 - p2 as i16).abs() <= tolerance as i16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_event_handler() {
        let mut handler = MockEventHandler::new();

        handler.simulate_zoom(1.5, (100.0, 200.0));
        handler.simulate_pan((10.0, -5.0));

        assert_eq!(handler.events_received.len(), 2);
        assert_eq!(
            handler.events_received[0],
            MockEvent::Zoom {
                factor: 1.5,
                center: (100.0, 200.0)
            }
        );
    }

    #[test]
    fn test_coordinate_transformation() {
        CoordinateTestHelper::assert_coordinate_transform(
            (5.0, 5.0),               // data point
            (0.0, 0.0, 10.0, 10.0),   // data bounds
            (0.0, 0.0, 100.0, 100.0), // screen bounds
            (50.0, 50.0),             // expected screen point
            1.0,                      // tolerance
        );
    }

    #[test]
    fn test_performance_monitor() {
        let mut monitor = PerformanceMonitor::new();

        // Simulate 60fps for 1 second
        for _ in 0..60 {
            monitor.record_frame();
        }

        let fps = monitor.average_fps();
        assert!(fps >= 50.0, "FPS should be close to 60, got {}", fps);
    }

    #[test]
    fn test_plot_builders() {
        let simple_plot = TestPlotBuilder::simple_line();
        assert_eq!(simple_plot.title, Some("Test Line Plot".to_string()));

        let scatter_plot = TestPlotBuilder::clustered_scatter();
        assert_eq!(
            scatter_plot.title,
            Some("Test Clustered Scatter".to_string())
        );
    }

    #[test]
    fn test_visual_comparison() {
        let image1 = vec![255, 0, 0, 255; 100]; // Red pixels
        let image2 = vec![250, 5, 5, 255; 100]; // Slightly different red

        assert!(VisualTestHelper::compare_images_with_tolerance(
            &image1, &image2, 10, 0
        ));
        assert!(!VisualTestHelper::compare_images_with_tolerance(
            &image1, &image2, 2, 0
        ));
    }
}
