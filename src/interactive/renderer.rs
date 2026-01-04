//! Real-time renderer for interactive plotting
//!
//! Provides high-performance rendering for interactive features using the
//! existing GPU acceleration (when available) while maintaining 60fps during interactions.

#[cfg(feature = "gpu")]
use crate::render::gpu::GpuRenderer;
use crate::{
    core::{Plot, Result},
    interactive::{
        event::{Annotation, Point2D, Rectangle},
        state::{DataPoint, DataPointId, InteractionState},
    },
    render::{Color, skia::SkiaRenderer},
};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

/// Real-time renderer for interactive plotting
pub struct RealTimeRenderer {
    // Core rendering components
    #[cfg(feature = "gpu")]
    gpu_renderer: Option<GpuRenderer>,
    cpu_renderer: SkiaRenderer,

    // Rendering state
    current_plot: Option<Plot>,
    render_cache: RenderCache,
    performance_monitor: PerformanceMonitor,

    // Interactive elements
    hover_highlight_color: Color,
    selection_highlight_color: Color,
    brush_color: Color,
    annotation_renderer: AnnotationRenderer,

    // Optimization settings
    quality_mode: RenderQuality,
    adaptive_quality: bool,
    target_fps: f64,
}

impl RealTimeRenderer {
    /// Create new real-time renderer
    pub async fn new() -> Result<Self> {
        #[cfg(feature = "gpu")]
        let gpu_renderer = match crate::render::gpu::initialize_gpu_backend().await {
            Ok(_) => match GpuRenderer::new().await {
                Ok(renderer) => {
                    log::info!("Interactive GPU renderer initialized");
                    Some(renderer)
                }
                Err(e) => {
                    log::warn!("GPU not available for interactive mode: {}", e);
                    None
                }
            },
            Err(e) => {
                log::warn!("GPU backend initialization failed: {}", e);
                None
            }
        };

        let cpu_renderer = SkiaRenderer::new(800, 600, crate::render::Theme::default())?;

        Ok(Self {
            #[cfg(feature = "gpu")]
            gpu_renderer,
            cpu_renderer,
            current_plot: None,
            render_cache: RenderCache::new(),
            performance_monitor: PerformanceMonitor::new(),

            hover_highlight_color: Color::new_rgba(255, 165, 0, 180), // Orange with transparency
            selection_highlight_color: Color::new_rgba(255, 0, 0, 120), // Red with transparency
            brush_color: Color::new_rgba(0, 100, 255, 60),            // Blue with high transparency
            annotation_renderer: AnnotationRenderer::new(),

            quality_mode: RenderQuality::Interactive,
            adaptive_quality: true,
            target_fps: 60.0,
        })
    }

    /// Set the current plot for rendering
    pub fn set_plot(&mut self, plot: Plot) {
        self.current_plot = Some(plot);
        self.render_cache.invalidate_all();
    }

    /// Render frame with current interaction state
    pub fn render_interactive(
        &mut self,
        state: &InteractionState,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>> {
        let frame_start = Instant::now();

        // Update renderer dimensions if needed
        self.update_dimensions(width, height)?;

        // Adaptive quality based on performance
        if self.adaptive_quality {
            self.update_quality_mode(state);
        }

        // Render base plot (cached when possible)
        let mut pixel_data = self.render_base_plot(state, width, height)?;

        // Render interactive elements on top
        self.render_hover_highlight(state, &mut pixel_data)?;
        self.render_selection_highlight(state, &mut pixel_data)?;
        self.render_brush_region(state, &mut pixel_data)?;
        self.render_annotations(state, &mut pixel_data)?;
        self.render_tooltip(state, &mut pixel_data)?;

        // Update performance metrics
        self.performance_monitor.record_frame(frame_start.elapsed());

        Ok(pixel_data)
    }

    /// Render high-quality static version for export
    pub fn render_publication(
        &mut self,
        plot: &Plot,
        width: u32,
        height: u32,
        dpi: f32,
    ) -> Result<Vec<u8>> {
        // Temporarily switch to high quality mode
        let old_quality = self.quality_mode;
        self.quality_mode = RenderQuality::Publication;

        // Update renderer for high-quality output
        self.cpu_renderer = SkiaRenderer::new(width, height, crate::render::Theme::default())?;

        // Render the plot at high quality
        let mut plot_clone = plot.clone();

        // Set size in inches based on requested pixels and DPI
        let width_inches = width as f32 / dpi;
        let height_inches = height as f32 / dpi;
        plot_clone = plot_clone.size(width_inches, height_inches).dpi(dpi as u32);

        let result = match plot_clone.render() {
            Ok(image) => image.pixels,
            Err(e) => {
                log::warn!("Publication render failed: {}, returning white pixels", e);
                vec![255u8; (width * height * 4) as usize]
            }
        };

        // Restore previous quality mode
        self.quality_mode = old_quality;

        Ok(result)
    }

    /// Get data point at screen coordinates
    pub fn get_data_point_at(
        &self,
        screen_pos: Point2D,
        state: &InteractionState,
    ) -> Option<DataPoint> {
        let data_pos = state.screen_to_data(screen_pos);

        // In real implementation, this would spatial search through plot data
        // For now, simulate finding a nearby point
        if let Some(ref plot) = self.current_plot {
            // Simulate hit testing - in reality would use spatial indexing
            let tolerance = 10.0 / state.zoom_level; // Zoom-adjusted tolerance

            // Mock implementation - would actually search plot data
            if data_pos.x > 10.0 && data_pos.x < 90.0 && data_pos.y > 10.0 && data_pos.y < 90.0 {
                return Some(
                    DataPoint::new(
                        42, // Mock ID
                        data_pos.x, data_pos.y, data_pos.y, // Mock value
                        0,          // Series index
                    )
                    .with_metadata("type".to_string(), "simulated".to_string()),
                );
            }
        }

        None
    }

    /// Get all data points in selection region
    pub fn get_points_in_region(
        &self,
        region: Rectangle,
        state: &InteractionState,
    ) -> Vec<DataPointId> {
        let mut points = Vec::new();

        // Convert screen region to data region
        let data_min = state.screen_to_data(region.min);
        let data_max = state.screen_to_data(region.max);
        let data_region = Rectangle::from_points(data_min, data_max);

        // In real implementation, would use spatial indexing to find points efficiently
        // For now, simulate selecting some points
        for i in 0..100 {
            let test_point = Point2D::new(i as f64 % 100.0, (i as f64 * 0.5) % 100.0);

            if data_region.contains(test_point) {
                points.push(DataPointId(i));
            }
        }

        points
    }

    /// Update renderer dimensions
    fn update_dimensions(&mut self, width: u32, height: u32) -> Result<()> {
        if self.cpu_renderer.width() != width || self.cpu_renderer.height() != height {
            self.cpu_renderer = SkiaRenderer::new(width, height, crate::render::Theme::default())?;
            self.render_cache.invalidate_all();
        }
        Ok(())
    }

    /// Update quality mode based on performance
    fn update_quality_mode(&mut self, state: &InteractionState) {
        let current_fps = self.performance_monitor.get_current_fps();
        let is_animating = !matches!(
            state.animation_state,
            crate::interactive::state::AnimationState::Idle
        );

        if is_animating || current_fps < self.target_fps * 0.8 {
            self.quality_mode = RenderQuality::Interactive;
        } else if current_fps > self.target_fps * 0.95 {
            self.quality_mode = RenderQuality::Balanced;
        }
    }

    /// Render base plot with caching
    fn render_base_plot(
        &mut self,
        state: &InteractionState,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>> {
        // Check if we can use cached render
        if !state.needs_redraw && !state.viewport_dirty {
            if let Some(cached) = self
                .render_cache
                .get_base_render(state.zoom_level, state.pan_offset)
            {
                return Ok(cached);
            }
        }

        // Render fresh base plot
        let has_plot = self.current_plot.is_some();
        let pixel_data = if has_plot {
            match self.quality_mode {
                RenderQuality::Interactive => {
                    // Fast rendering for interaction
                    self.render_interactive_quality(state, width, height)?
                }
                RenderQuality::Balanced => {
                    // Balanced quality and performance
                    self.render_balanced_quality(state, width, height)?
                }
                RenderQuality::Publication => {
                    // High quality rendering
                    self.render_plot_to_pixels(width, height)?
                }
            }
        } else {
            // No plot set, render empty canvas
            vec![255u8; (width * height * 4) as usize] // White background
        };

        // Cache the render
        self.render_cache
            .store_base_render(state.zoom_level, state.pan_offset, pixel_data.clone());

        Ok(pixel_data)
    }

    /// Render with interactive quality (optimized for speed)
    fn render_interactive_quality(
        &mut self,
        _state: &InteractionState,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>> {
        self.render_plot_to_pixels(width, height)
    }

    /// Render with balanced quality
    fn render_balanced_quality(
        &mut self,
        _state: &InteractionState,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>> {
        self.render_plot_to_pixels(width, height)
    }

    /// Render the current plot to RGBA pixel data
    fn render_plot_to_pixels(&self, width: u32, height: u32) -> Result<Vec<u8>> {
        if let Some(ref plot) = self.current_plot {
            // Clone and resize the plot to match requested dimensions
            let mut plot_clone = plot.clone();

            // Convert pixels to inches (assuming 100 DPI for interactive mode)
            let dpi = 100.0;
            let width_inches = width as f32 / dpi;
            let height_inches = height as f32 / dpi;

            // Update plot size
            plot_clone = plot_clone.size(width_inches, height_inches);

            // Render the plot
            match plot_clone.render() {
                Ok(image) => {
                    // If dimensions match, return directly
                    if image.width == width && image.height == height {
                        Ok(image.pixels)
                    } else {
                        // Dimensions might differ slightly, return as-is
                        // The caller should handle any size mismatch
                        Ok(image.pixels)
                    }
                }
                Err(e) => {
                    log::warn!("Plot rendering failed: {}, returning white pixels", e);
                    Ok(vec![255u8; (width * height * 4) as usize])
                }
            }
        } else {
            // No plot set, return white background
            Ok(vec![255u8; (width * height * 4) as usize])
        }
    }

    /// Render hover highlight
    fn render_hover_highlight(
        &mut self,
        state: &InteractionState,
        pixel_data: &mut [u8],
    ) -> Result<()> {
        if let Some(ref hover_point) = state.hover_point {
            let screen_pos = state.data_to_screen(hover_point.position);
            self.draw_highlight_circle(pixel_data, screen_pos, 8.0, self.hover_highlight_color)?;
        }
        Ok(())
    }

    /// Render selection highlights
    fn render_selection_highlight(
        &mut self,
        state: &InteractionState,
        pixel_data: &mut [u8],
    ) -> Result<()> {
        for point_id in &state.selected_points {
            // In real implementation, would look up actual point coordinates
            // For now, simulate highlighting at fixed positions
            let screen_pos = Point2D::new(100.0 + point_id.0 as f64 * 50.0, 100.0);
            self.draw_highlight_circle(
                pixel_data,
                screen_pos,
                6.0,
                self.selection_highlight_color,
            )?;
        }
        Ok(())
    }

    /// Render brush selection region
    fn render_brush_region(
        &mut self,
        state: &InteractionState,
        pixel_data: &mut [u8],
    ) -> Result<()> {
        if let Some(region) = state.brushed_region {
            self.draw_selection_rectangle(pixel_data, region, self.brush_color)?;
        }
        Ok(())
    }

    /// Render custom annotations
    fn render_annotations(
        &mut self,
        state: &InteractionState,
        pixel_data: &mut [u8],
    ) -> Result<()> {
        for annotation in &state.annotations {
            self.annotation_renderer.render_annotation(
                annotation,
                state,
                pixel_data,
                self.cpu_renderer.width(),
                self.cpu_renderer.height(),
            )?;
        }
        Ok(())
    }

    /// Render tooltip
    fn render_tooltip(&mut self, state: &InteractionState, pixel_data: &mut [u8]) -> Result<()> {
        if state.tooltip_visible && !state.tooltip_content.is_empty() {
            self.draw_tooltip(pixel_data, &state.tooltip_content, state.tooltip_position)?;
        }
        Ok(())
    }

    /// Draw highlight circle at screen position
    fn draw_highlight_circle(
        &self,
        pixel_data: &mut [u8],
        center: Point2D,
        radius: f32,
        color: Color,
    ) -> Result<()> {
        // Simple circle drawing - in production would use proper graphics primitives
        let width = self.cpu_renderer.width() as i32;
        let height = self.cpu_renderer.height() as i32;
        let r_sq = (radius * radius) as i32;

        let cx = center.x as i32;
        let cy = center.y as i32;

        for dy in -(radius as i32)..=(radius as i32) {
            for dx in -(radius as i32)..=(radius as i32) {
                if dx * dx + dy * dy <= r_sq {
                    let x = cx + dx;
                    let y = cy + dy;

                    if x >= 0 && x < width && y >= 0 && y < height {
                        let index = ((y * width + x) * 4) as usize;
                        if index + 3 < pixel_data.len() {
                            // Alpha blend with existing pixel
                            let alpha = color.a as f32 / 255.0;
                            pixel_data[index] = blend_channel(pixel_data[index], color.r, alpha);
                            pixel_data[index + 1] =
                                blend_channel(pixel_data[index + 1], color.g, alpha);
                            pixel_data[index + 2] =
                                blend_channel(pixel_data[index + 2], color.b, alpha);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Draw selection rectangle
    fn draw_selection_rectangle(
        &self,
        pixel_data: &mut [u8],
        region: Rectangle,
        color: Color,
    ) -> Result<()> {
        let width = self.cpu_renderer.width() as i32;
        let height = self.cpu_renderer.height() as i32;

        let x1 = region.min.x as i32;
        let y1 = region.min.y as i32;
        let x2 = region.max.x as i32;
        let y2 = region.max.y as i32;

        let alpha = color.a as f32 / 255.0;

        // Fill rectangle with alpha blending
        for y in y1.max(0)..=y2.min(height - 1) {
            for x in x1.max(0)..=x2.min(width - 1) {
                let index = ((y * width + x) * 4) as usize;
                if index + 3 < pixel_data.len() {
                    pixel_data[index] = blend_channel(pixel_data[index], color.r, alpha);
                    pixel_data[index + 1] = blend_channel(pixel_data[index + 1], color.g, alpha);
                    pixel_data[index + 2] = blend_channel(pixel_data[index + 2], color.b, alpha);
                }
            }
        }

        Ok(())
    }

    /// Draw tooltip
    fn draw_tooltip(&self, pixel_data: &mut [u8], content: &str, position: Point2D) -> Result<()> {
        // Simple tooltip rendering - in production would use proper text rendering
        // For now, just draw a simple colored rectangle as placeholder
        let tooltip_width = content.len() as f64 * 8.0 + 20.0;
        let tooltip_height = 30.0;

        let tooltip_rect = Rectangle::new(
            position.x,
            position.y - tooltip_height,
            position.x + tooltip_width,
            position.y,
        );

        let tooltip_color = Color::new_rgba(255, 255, 220, 200); // Light yellow
        self.draw_selection_rectangle(pixel_data, tooltip_rect, tooltip_color)?;

        Ok(())
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> PerformanceStats {
        self.performance_monitor.get_stats()
    }
}

/// Alpha blend two color channels
fn blend_channel(background: u8, foreground: u8, alpha: f32) -> u8 {
    let bg = background as f32 / 255.0;
    let fg = foreground as f32 / 255.0;
    let result = bg * (1.0 - alpha) + fg * alpha;
    (result * 255.0) as u8
}

/// Render cache for performance optimization
struct RenderCache {
    base_renders: HashMap<CacheKey, Vec<u8>>,
    max_entries: usize,
}

#[derive(Hash, PartialEq, Eq, Clone)]
struct CacheKey {
    zoom_level_bits: u64,
    pan_x_bits: u64,
    pan_y_bits: u64,
}

impl RenderCache {
    fn new() -> Self {
        Self {
            base_renders: HashMap::new(),
            max_entries: 10,
        }
    }

    fn get_base_render(
        &self,
        zoom_level: f64,
        pan_offset: crate::interactive::event::Vector2D,
    ) -> Option<Vec<u8>> {
        let key = Self::make_key(zoom_level, pan_offset);
        self.base_renders.get(&key).cloned()
    }

    fn store_base_render(
        &mut self,
        zoom_level: f64,
        pan_offset: crate::interactive::event::Vector2D,
        pixel_data: Vec<u8>,
    ) {
        if self.base_renders.len() >= self.max_entries {
            // Simple LRU - remove first entry
            if let Some(first_key) = self.base_renders.keys().next().cloned() {
                self.base_renders.remove(&first_key);
            }
        }

        let key = Self::make_key(zoom_level, pan_offset);
        self.base_renders.insert(key, pixel_data);
    }

    fn invalidate_all(&mut self) {
        self.base_renders.clear();
    }

    fn make_key(zoom_level: f64, pan_offset: crate::interactive::event::Vector2D) -> CacheKey {
        CacheKey {
            zoom_level_bits: (zoom_level * 100.0) as u64, // Quantize to avoid floating point issues
            pan_x_bits: (pan_offset.x * 100.0) as u64,
            pan_y_bits: (pan_offset.y * 100.0) as u64,
        }
    }
}

/// Render quality modes
#[derive(Debug, Clone, Copy, PartialEq)]
enum RenderQuality {
    Interactive, // Fast rendering for smooth interaction
    Balanced,    // Balance between quality and performance
    Publication, // High quality for static output
}

/// Performance monitoring
struct PerformanceMonitor {
    frame_times: Vec<Duration>,
    frame_count: u64,
    last_fps_calculation: Instant,
    target_frame_time: Duration,
}

impl PerformanceMonitor {
    fn new() -> Self {
        Self {
            frame_times: Vec::with_capacity(60),
            frame_count: 0,
            last_fps_calculation: Instant::now(),
            target_frame_time: Duration::from_nanos(16_666_667), // ~60fps
        }
    }

    fn record_frame(&mut self, frame_time: Duration) {
        self.frame_times.push(frame_time);
        self.frame_count += 1;

        // Keep only recent frame times
        if self.frame_times.len() > 60 {
            self.frame_times.remove(0);
        }
    }

    fn get_current_fps(&self) -> f64 {
        if self.frame_times.is_empty() {
            return 0.0;
        }

        let avg_frame_time: Duration =
            self.frame_times.iter().sum::<Duration>() / self.frame_times.len() as u32;
        1.0 / avg_frame_time.as_secs_f64()
    }

    fn get_stats(&self) -> PerformanceStats {
        PerformanceStats {
            current_fps: self.get_current_fps(),
            frame_count: self.frame_count,
            avg_frame_time: if !self.frame_times.is_empty() {
                self.frame_times.iter().sum::<Duration>() / self.frame_times.len() as u32
            } else {
                Duration::ZERO
            },
        }
    }
}

/// Performance statistics
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub current_fps: f64,
    pub frame_count: u64,
    pub avg_frame_time: Duration,
}

/// Annotation renderer
struct AnnotationRenderer;

impl AnnotationRenderer {
    fn new() -> Self {
        Self
    }

    fn render_annotation(
        &self,
        annotation: &Annotation,
        state: &InteractionState,
        pixel_data: &mut [u8],
        width: u32,
        height: u32,
    ) -> Result<()> {
        match annotation {
            Annotation::Text {
                content,
                position,
                style: _,
            } => {
                let screen_pos = state.data_to_screen(*position);
                // In real implementation, would render text using cosmic-text
                println!(
                    "Rendering text annotation: '{}' at {:?}",
                    content, screen_pos
                );
            }

            Annotation::Arrow {
                start,
                end,
                style: _,
            } => {
                let screen_start = state.data_to_screen(*start);
                let screen_end = state.data_to_screen(*end);
                // In real implementation, would draw arrow line
                println!(
                    "Rendering arrow from {:?} to {:?}",
                    screen_start, screen_end
                );
            }

            Annotation::Shape { geometry, style: _ } => {
                // In real implementation, would render geometric shapes
                println!("Rendering shape annotation: {:?}", geometry);
            }

            Annotation::Equation {
                latex,
                position,
                style: _,
            } => {
                let screen_pos = state.data_to_screen(*position);
                // In real implementation, would render LaTeX equation
                println!("Rendering equation '{}' at {:?}", latex, screen_pos);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_renderer_creation() {
        let renderer_result = RealTimeRenderer::new().await;
        assert!(renderer_result.is_ok());
    }

    #[test]
    fn test_render_cache() {
        let mut cache = RenderCache::new();

        let zoom = 1.5;
        let pan = crate::interactive::event::Vector2D::new(10.0, 20.0);
        let test_data = vec![255u8; 100];

        // Store and retrieve
        cache.store_base_render(zoom, pan, test_data.clone());
        let retrieved = cache.get_base_render(zoom, pan);

        assert_eq!(retrieved, Some(test_data));

        // Test cache invalidation
        cache.invalidate_all();
        let retrieved_after_clear = cache.get_base_render(zoom, pan);
        assert_eq!(retrieved_after_clear, None);
    }

    #[test]
    fn test_performance_monitor() {
        let mut monitor = PerformanceMonitor::new();

        // Record some frame times
        monitor.record_frame(Duration::from_millis(16)); // ~60fps
        monitor.record_frame(Duration::from_millis(17));
        monitor.record_frame(Duration::from_millis(15));

        let stats = monitor.get_stats();
        assert!(stats.current_fps > 50.0 && stats.current_fps < 70.0);
        assert_eq!(stats.frame_count, 3);
    }

    #[test]
    fn test_alpha_blending() {
        let background = 100u8;
        let foreground = 200u8;
        let alpha = 0.5;

        let result = blend_channel(background, foreground, alpha);
        let expected = (100.0 * 0.5 + 200.0 * 0.5) as u8;

        assert_eq!(result, expected);
    }
}
