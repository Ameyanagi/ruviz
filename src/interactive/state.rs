//! State management for interactive plotting
//!
//! Tracks the current state of user interactions including zoom level,
//! pan offset, selected data points, and animation state.

use crate::interactive::event::{Point2D, Vector2D, Rectangle, PlotId, Annotation};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Complete interaction state for a plot
#[derive(Debug, Clone)]
pub struct InteractionState {
    // Viewport management
    pub zoom_level: f64,
    pub pan_offset: Vector2D,
    pub data_bounds: Rectangle,
    pub screen_bounds: Rectangle,
    pub viewport_dirty: bool,
    
    // Mouse tracking
    pub last_mouse_pos: Point2D,
    pub mouse_button_pressed: bool,
    pub mouse_in_window: bool,
    
    // Selection and brushing
    pub selected_points: HashSet<DataPointId>,
    pub brushed_region: Option<Rectangle>,
    pub brush_active: bool,
    pub brush_start: Option<Point2D>,
    pub linked_plots: Vec<PlotId>,
    
    // Hover and tooltip
    pub hover_point: Option<DataPoint>,
    pub tooltip_visible: bool,
    pub tooltip_content: String,
    pub tooltip_position: Point2D,
    
    // Animation and transitions
    pub animation_state: AnimationState,
    pub transition_duration: Duration,
    pub animation_start_time: Option<Instant>,
    
    // Custom annotations
    pub annotations: Vec<Annotation>,
    pub annotation_dirty: bool,
    
    // Performance tracking
    pub last_frame_time: Instant,
    pub frame_count: u64,
    pub needs_redraw: bool,
}

impl Default for InteractionState {
    fn default() -> Self {
        Self {
            zoom_level: 1.0,
            pan_offset: Vector2D::zero(),
            data_bounds: Rectangle::new(0.0, 0.0, 100.0, 100.0),
            screen_bounds: Rectangle::new(0.0, 0.0, 800.0, 600.0),
            viewport_dirty: true,
            
            last_mouse_pos: Point2D::zero(),
            mouse_button_pressed: false,
            mouse_in_window: false,
            
            selected_points: HashSet::new(),
            brushed_region: None,
            brush_active: false,
            brush_start: None,
            linked_plots: Vec::new(),
            
            hover_point: None,
            tooltip_visible: false,
            tooltip_content: String::new(),
            tooltip_position: Point2D::zero(),
            
            animation_state: AnimationState::Idle,
            transition_duration: Duration::from_millis(200),
            animation_start_time: None,
            
            annotations: Vec::new(),
            annotation_dirty: false,
            
            last_frame_time: Instant::now(),
            frame_count: 0,
            needs_redraw: true,
        }
    }
}

impl InteractionState {
    /// Create new interaction state with custom bounds
    pub fn new(data_bounds: Rectangle, screen_bounds: Rectangle) -> Self {
        Self {
            data_bounds,
            screen_bounds,
            ..Default::default()
        }
    }
    
    /// Apply zoom transformation
    pub fn apply_zoom(&mut self, factor: f64, center: Point2D) {
        let old_zoom = self.zoom_level;
        self.zoom_level *= factor;
        
        // Clamp zoom to reasonable bounds
        self.zoom_level = self.zoom_level.clamp(0.1, 100.0);
        
        let actual_factor = self.zoom_level / old_zoom;
        
        // Adjust pan to zoom around the specified center
        if actual_factor != 1.0 {
            let data_center = self.screen_to_data(center);
            let zoom_adjust = 1.0 - 1.0 / actual_factor;
            
            self.pan_offset.x += (data_center.x - self.get_data_center().x) * zoom_adjust;
            self.pan_offset.y += (data_center.y - self.get_data_center().y) * zoom_adjust;
        }
        
        self.viewport_dirty = true;
        self.needs_redraw = true;
        
        // Start smooth zoom animation
        if factor != 1.0 {
            self.start_animation(AnimationState::Zooming { 
                start_zoom: old_zoom,
                target_zoom: self.zoom_level,
                center,
            });
        }
    }
    
    /// Apply pan transformation
    pub fn apply_pan(&mut self, delta: Vector2D) {
        // Convert screen delta to data delta
        let data_delta = self.screen_delta_to_data_delta(delta);
        
        self.pan_offset.x -= data_delta.x;
        self.pan_offset.y -= data_delta.y;
        
        self.viewport_dirty = true;
        self.needs_redraw = true;
        
        // Start smooth pan animation for large movements
        if data_delta.magnitude() > 0.1 {
            self.start_animation(AnimationState::Panning {
                start_offset: Vector2D::new(
                    self.pan_offset.x + data_delta.x,
                    self.pan_offset.y + data_delta.y,
                ),
                target_offset: self.pan_offset,
            });
        }
    }
    
    /// Reset viewport to show all data
    pub fn reset_viewport(&mut self) {
        self.zoom_level = 1.0;
        self.pan_offset = Vector2D::zero();
        self.viewport_dirty = true;
        self.needs_redraw = true;
        
        self.start_animation(AnimationState::Resetting);
    }
    
    /// Update brush selection region
    pub fn update_brush(&mut self, current_pos: Point2D) {
        if let Some(start) = self.brush_start {
            self.brushed_region = Some(Rectangle::from_points(start, current_pos));
            self.brush_active = true;
            self.needs_redraw = true;
        }
    }
    
    /// Start brush selection
    pub fn start_brush(&mut self, start_pos: Point2D) {
        self.brush_start = Some(start_pos);
        self.brush_active = true;
        self.clear_selection();
    }
    
    /// End brush selection and finalize point selection
    pub fn end_brush(&mut self) {
        if let Some(region) = self.brushed_region.take() {
            // In real implementation, this would select points within the region
            // For now, we'll simulate selecting some points
            for i in 0..10 {
                self.selected_points.insert(DataPointId(i));
            }
        }
        
        self.brush_active = false;
        self.brush_start = None;
        self.needs_redraw = true;
    }
    
    /// Clear current selection
    pub fn clear_selection(&mut self) {
        self.selected_points.clear();
        self.brushed_region = None;
        self.needs_redraw = true;
    }
    
    /// Add point to selection
    pub fn select_point(&mut self, point_id: DataPointId) {
        self.selected_points.insert(point_id);
        self.needs_redraw = true;
    }
    
    /// Update hover state
    pub fn update_hover(&mut self, screen_pos: Point2D, data_point: Option<DataPoint>) {
        self.last_mouse_pos = screen_pos;
        
        let hover_changed = match (&self.hover_point, &data_point) {
            (None, None) => false,
            (Some(_), None) | (None, Some(_)) => true,
            (Some(old), Some(new)) => old.id != new.id,
        };
        
        if hover_changed {
            self.hover_point = data_point;
            self.needs_redraw = true;
        }
    }
    
    /// Show tooltip at position
    pub fn show_tooltip(&mut self, content: String, position: Point2D) {
        self.tooltip_content = content;
        self.tooltip_position = position;
        self.tooltip_visible = true;
        self.needs_redraw = true;
    }
    
    /// Hide tooltip
    pub fn hide_tooltip(&mut self) {
        if self.tooltip_visible {
            self.tooltip_visible = false;
            self.needs_redraw = true;
        }
    }
    
    /// Add annotation
    pub fn add_annotation(&mut self, annotation: Annotation) {
        self.annotations.push(annotation);
        self.annotation_dirty = true;
        self.needs_redraw = true;
    }
    
    /// Remove annotation by index
    pub fn remove_annotation(&mut self, index: usize) {
        if index < self.annotations.len() {
            self.annotations.remove(index);
            self.annotation_dirty = true;
            self.needs_redraw = true;
        }
    }
    
    /// Update animation state
    pub fn update_animation(&mut self, dt: Duration) -> bool {
        match &self.animation_state {
            AnimationState::Idle => false,
            
            AnimationState::Zooming { start_zoom, target_zoom, center: _ } => {
                if let Some(start_time) = self.animation_start_time {
                    let elapsed = start_time.elapsed();
                    let progress = (elapsed.as_secs_f64() / self.transition_duration.as_secs_f64()).min(1.0);
                    
                    if progress >= 1.0 {
                        self.zoom_level = *target_zoom;
                        self.animation_state = AnimationState::Idle;
                        self.animation_start_time = None;
                    } else {
                        // Smooth interpolation with easing
                        let eased_progress = ease_in_out_cubic(progress);
                        self.zoom_level = start_zoom + (target_zoom - start_zoom) * eased_progress;
                        self.viewport_dirty = true;
                        self.needs_redraw = true;
                    }
                    
                    true
                } else {
                    false
                }
            }
            
            AnimationState::Panning { start_offset, target_offset } => {
                if let Some(start_time) = self.animation_start_time {
                    let elapsed = start_time.elapsed();
                    let progress = (elapsed.as_secs_f64() / self.transition_duration.as_secs_f64()).min(1.0);
                    
                    if progress >= 1.0 {
                        self.pan_offset = *target_offset;
                        self.animation_state = AnimationState::Idle;
                        self.animation_start_time = None;
                    } else {
                        let eased_progress = ease_in_out_cubic(progress);
                        self.pan_offset.x = start_offset.x + (target_offset.x - start_offset.x) * eased_progress;
                        self.pan_offset.y = start_offset.y + (target_offset.y - start_offset.y) * eased_progress;
                        self.viewport_dirty = true;
                        self.needs_redraw = true;
                    }
                    
                    true
                } else {
                    false
                }
            }
            
            AnimationState::Resetting => {
                if let Some(start_time) = self.animation_start_time {
                    let elapsed = start_time.elapsed();
                    if elapsed >= self.transition_duration {
                        self.animation_state = AnimationState::Idle;
                        self.animation_start_time = None;
                    } else {
                        self.needs_redraw = true;
                    }
                    true
                } else {
                    false
                }
            }
        }
    }
    
    /// Start animation with given state
    fn start_animation(&mut self, state: AnimationState) {
        self.animation_state = state;
        self.animation_start_time = Some(Instant::now());
    }
    
    /// Convert screen coordinates to data coordinates
    pub fn screen_to_data(&self, screen_pos: Point2D) -> Point2D {
        let screen_width = self.screen_bounds.width();
        let screen_height = self.screen_bounds.height();
        
        let data_width = self.data_bounds.width() / self.zoom_level;
        let data_height = self.data_bounds.height() / self.zoom_level;
        
        let data_center = self.get_data_center();
        
        let norm_x = (screen_pos.x - self.screen_bounds.min.x) / screen_width - 0.5;
        let norm_y = (screen_pos.y - self.screen_bounds.min.y) / screen_height - 0.5;
        
        Point2D::new(
            data_center.x + norm_x * data_width,
            data_center.y + norm_y * data_height,
        )
    }
    
    /// Convert data coordinates to screen coordinates  
    pub fn data_to_screen(&self, data_pos: Point2D) -> Point2D {
        let screen_width = self.screen_bounds.width();
        let screen_height = self.screen_bounds.height();
        
        let data_width = self.data_bounds.width() / self.zoom_level;
        let data_height = self.data_bounds.height() / self.zoom_level;
        
        let data_center = self.get_data_center();
        
        let norm_x = (data_pos.x - data_center.x) / data_width + 0.5;
        let norm_y = (data_pos.y - data_center.y) / data_height + 0.5;
        
        Point2D::new(
            self.screen_bounds.min.x + norm_x * screen_width,
            self.screen_bounds.min.y + norm_y * screen_height,
        )
    }
    
    /// Convert screen delta to data delta
    fn screen_delta_to_data_delta(&self, screen_delta: Vector2D) -> Vector2D {
        let screen_width = self.screen_bounds.width();
        let screen_height = self.screen_bounds.height();
        
        let data_width = self.data_bounds.width() / self.zoom_level;
        let data_height = self.data_bounds.height() / self.zoom_level;
        
        Vector2D::new(
            (screen_delta.x / screen_width) * data_width,
            (screen_delta.y / screen_height) * data_height,
        )
    }
    
    /// Get current data center point (including pan offset)
    fn get_data_center(&self) -> Point2D {
        Point2D::new(
            self.data_bounds.center().x + self.pan_offset.x,
            self.data_bounds.center().y + self.pan_offset.y,
        )
    }
    
    /// Update frame timing
    pub fn update_frame_timing(&mut self) {
        let now = Instant::now();
        self.last_frame_time = now;
        self.frame_count += 1;
    }
    
    /// Get current FPS estimate
    pub fn get_fps_estimate(&self) -> f64 {
        // Simple estimate based on frame count and elapsed time
        // In real implementation, this would use a rolling average
        60.0 // Placeholder
    }
    
    /// Check if viewport needs recalculation
    pub fn needs_viewport_update(&self) -> bool {
        self.viewport_dirty
    }
    
    /// Mark viewport as updated
    pub fn mark_viewport_clean(&mut self) {
        self.viewport_dirty = false;
    }
}

/// Animation state for smooth transitions
#[derive(Debug, Clone, PartialEq)]
pub enum AnimationState {
    Idle,
    Zooming {
        start_zoom: f64,
        target_zoom: f64,
        center: Point2D,
    },
    Panning {
        start_offset: Vector2D,
        target_offset: Vector2D,
    },
    Resetting,
}

/// Data point identifier for selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DataPointId(pub usize);

/// Data point information for hover and selection
#[derive(Debug, Clone, PartialEq)]
pub struct DataPoint {
    pub id: DataPointId,
    pub position: Point2D,
    pub value: f64,
    pub series_index: usize,
    pub metadata: HashMap<String, String>,
}

impl DataPoint {
    pub fn new(id: usize, x: f64, y: f64, value: f64, series_index: usize) -> Self {
        Self {
            id: DataPointId(id),
            position: Point2D::new(x, y),
            value,
            series_index,
            metadata: HashMap::new(),
        }
    }
    
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Smooth easing function for animations
fn ease_in_out_cubic(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - 4.0 * (1.0 - t).powi(3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_transformation() {
        let state = InteractionState::new(
            Rectangle::new(0.0, 0.0, 100.0, 100.0),
            Rectangle::new(0.0, 0.0, 800.0, 600.0),
        );
        
        // Center of screen should map to center of data
        let screen_center = Point2D::new(400.0, 300.0);
        let data_center = state.screen_to_data(screen_center);
        
        assert!((data_center.x - 50.0).abs() < 0.1);
        assert!((data_center.y - 50.0).abs() < 0.1);
        
        // Round trip should work
        let back_to_screen = state.data_to_screen(data_center);
        assert!((back_to_screen.x - screen_center.x).abs() < 0.1);
        assert!((back_to_screen.y - screen_center.y).abs() < 0.1);
    }

    #[test]
    fn test_zoom_application() {
        let mut state = InteractionState::default();
        let initial_zoom = state.zoom_level;
        
        state.apply_zoom(2.0, Point2D::new(400.0, 300.0));
        
        assert_eq!(state.zoom_level, initial_zoom * 2.0);
        assert!(state.viewport_dirty);
        assert!(state.needs_redraw);
    }

    #[test]
    fn test_zoom_clamping() {
        let mut state = InteractionState::default();
        
        // Test zoom out limit
        state.apply_zoom(0.01, Point2D::zero()); // Very small zoom
        assert!(state.zoom_level >= 0.1);
        
        // Test zoom in limit  
        state.zoom_level = 50.0;
        state.apply_zoom(10.0, Point2D::zero()); // Large zoom
        assert!(state.zoom_level <= 100.0);
    }

    #[test]
    fn test_brush_selection() {
        let mut state = InteractionState::default();
        
        state.start_brush(Point2D::new(10.0, 10.0));
        assert!(state.brush_active);
        assert_eq!(state.brush_start, Some(Point2D::new(10.0, 10.0)));
        
        state.update_brush(Point2D::new(50.0, 50.0));
        assert!(state.brushed_region.is_some());
        
        state.end_brush();
        assert!(!state.brush_active);
        assert!(!state.selected_points.is_empty());
    }

    #[test]
    fn test_animation_update() {
        let mut state = InteractionState::default();
        
        // Start zoom animation
        state.apply_zoom(2.0, Point2D::zero());
        assert!(matches!(state.animation_state, AnimationState::Zooming { .. }));
        
        // Update animation
        let animated = state.update_animation(Duration::from_millis(100));
        assert!(animated);
        
        // Should still be animating
        assert!(!matches!(state.animation_state, AnimationState::Idle));
        
        // Complete animation
        let complete_duration = state.transition_duration + Duration::from_millis(1);
        let animated = state.update_animation(complete_duration);
        assert!(!animated || matches!(state.animation_state, AnimationState::Idle));
    }

    #[test]
    fn test_easing_function() {
        assert_eq!(ease_in_out_cubic(0.0), 0.0);
        assert_eq!(ease_in_out_cubic(1.0), 1.0);
        assert!(ease_in_out_cubic(0.5) > 0.4 && ease_in_out_cubic(0.5) < 0.6);
    }

    #[test]
    fn test_tooltip_management() {
        let mut state = InteractionState::default();
        
        state.show_tooltip("Test tooltip".to_string(), Point2D::new(100.0, 100.0));
        assert!(state.tooltip_visible);
        assert_eq!(state.tooltip_content, "Test tooltip");
        
        state.hide_tooltip();
        assert!(!state.tooltip_visible);
    }

    #[test]
    fn test_annotation_management() {
        let mut state = InteractionState::default();
        
        let annotation = Annotation::Text {
            content: "Test annotation".to_string(),
            position: Point2D::new(50.0, 50.0),
            style: crate::interactive::event::TextStyle::default(),
        };
        
        state.add_annotation(annotation);
        assert_eq!(state.annotations.len(), 1);
        assert!(state.annotation_dirty);
        
        state.remove_annotation(0);
        assert_eq!(state.annotations.len(), 0);
    }
}