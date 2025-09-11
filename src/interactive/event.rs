//! Event handling system for interactive plotting
//!
//! Provides a unified event system for handling user interactions like
//! zoom, pan, selection, and custom annotations.

use crate::core::{Result, PlottingError};
use std::time::Duration;

/// High-level interaction events
#[derive(Debug, Clone, PartialEq)]
pub enum InteractionEvent {
    // Navigation events
    Zoom {
        factor: f64,
        center: Point2D,
    },
    Pan {
        delta: Vector2D,
    },
    Reset,
    
    // Selection events
    Select {
        region: Rectangle,
    },
    SelectPoint {
        point: Point2D,
    },
    ClearSelection,
    
    // Data brushing events
    Brush {
        start: Point2D,
        end: Point2D,
    },
    LinkPlots {
        plot_ids: Vec<PlotId>,
    },
    
    // Information events
    Hover {
        point: Point2D,
    },
    ShowTooltip {
        content: String,
        position: Point2D,
    },
    HideTooltip,
    
    // Customization events
    AddAnnotation {
        annotation: Annotation,
    },
    ModifyStyle {
        element: StyleElement,
        style: Style,
    },
}

/// 2D point in screen or data coordinates
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl Point2D {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    
    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
    
    pub fn distance_to(&self, other: Point2D) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

/// 2D vector for representing deltas and directions
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector2D {
    pub x: f64,
    pub y: f64,
}

impl Vector2D {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    
    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
    
    pub fn magnitude(&self) -> f64 {
        (self.x.powi(2) + self.y.powi(2)).sqrt()
    }
}

/// Rectangular region for selection and bounds
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rectangle {
    pub min: Point2D,
    pub max: Point2D,
}

impl Rectangle {
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            min: Point2D::new(min_x, min_y),
            max: Point2D::new(max_x, max_y),
        }
    }
    
    pub fn from_points(p1: Point2D, p2: Point2D) -> Self {
        Self {
            min: Point2D::new(p1.x.min(p2.x), p1.y.min(p2.y)),
            max: Point2D::new(p1.x.max(p2.x), p1.y.max(p2.y)),
        }
    }
    
    pub fn contains(&self, point: Point2D) -> bool {
        point.x >= self.min.x && point.x <= self.max.x &&
        point.y >= self.min.y && point.y <= self.max.y
    }
    
    pub fn width(&self) -> f64 {
        self.max.x - self.min.x
    }
    
    pub fn height(&self) -> f64 {
        self.max.y - self.min.y
    }
    
    pub fn center(&self) -> Point2D {
        Point2D::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
        )
    }
}

/// Plot identifier for multi-plot linking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlotId(pub u64);

impl PlotId {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// Annotation types for custom plot elements
#[derive(Debug, Clone, PartialEq)]
pub enum Annotation {
    Text {
        content: String,
        position: Point2D,
        style: TextStyle,
    },
    Arrow {
        start: Point2D,
        end: Point2D,
        style: ArrowStyle,
    },
    Shape {
        geometry: ShapeGeometry,
        style: ShapeStyle,
    },
    Equation {
        latex: String,
        position: Point2D,
        style: EquationStyle,
    },
}

/// Text styling for annotations
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    pub font_size: f32,
    pub color: [u8; 4], // RGBA
    pub font_family: String,
    pub bold: bool,
    pub italic: bool,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: 12.0,
            color: [0, 0, 0, 255], // Black
            font_family: "Arial".to_string(),
            bold: false,
            italic: false,
        }
    }
}

/// Arrow styling for annotations
#[derive(Debug, Clone, PartialEq)]
pub struct ArrowStyle {
    pub color: [u8; 4],
    pub thickness: f32,
    pub head_size: f32,
}

impl Default for ArrowStyle {
    fn default() -> Self {
        Self {
            color: [0, 0, 0, 255], // Black
            thickness: 2.0,
            head_size: 8.0,
        }
    }
}

/// Shape geometry for geometric annotations
#[derive(Debug, Clone, PartialEq)]
pub enum ShapeGeometry {
    Circle {
        center: Point2D,
        radius: f64,
    },
    Rectangle(Rectangle),
    Polygon {
        points: Vec<Point2D>,
    },
}

/// Shape styling
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeStyle {
    pub fill_color: Option<[u8; 4]>,
    pub stroke_color: [u8; 4],
    pub stroke_width: f32,
}

impl Default for ShapeStyle {
    fn default() -> Self {
        Self {
            fill_color: None,
            stroke_color: [0, 0, 0, 255], // Black
            stroke_width: 1.0,
        }
    }
}

/// Equation styling for LaTeX math
#[derive(Debug, Clone, PartialEq)]
pub struct EquationStyle {
    pub font_size: f32,
    pub color: [u8; 4],
}

impl Default for EquationStyle {
    fn default() -> Self {
        Self {
            font_size: 14.0,
            color: [0, 0, 0, 255], // Black
        }
    }
}

/// Style elements that can be customized
#[derive(Debug, Clone, PartialEq)]
pub enum StyleElement {
    Axis,
    Grid,
    Legend,
    Title,
    DataSeries(usize),
}

/// Generic style container
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    pub color: Option<[u8; 4]>,
    pub thickness: Option<f32>,
    pub font_size: Option<f32>,
}

/// Response type for event processing
#[derive(Debug, Clone, PartialEq)]
pub enum EventResponse {
    Handled,
    NeedsRedraw,
    NeedsRecompute,
    PropagateTo(Vec<PlotId>),
    Error(String),
}

/// Event handler trait for processing interactions
pub trait EventHandler {
    /// Process a single interaction event
    fn handle_event(&mut self, event: InteractionEvent) -> Result<EventResponse>;
    
    /// Update state for animations and transitions
    fn update(&mut self, dt: Duration) -> Result<()>;
    
    /// Check if a redraw is needed
    fn needs_redraw(&self) -> bool;
    
    /// Reset handler to initial state
    fn reset(&mut self);
}

/// Event processing utilities
pub struct EventProcessor;

impl EventProcessor {
    /// Convert raw winit events to high-level interaction events
    pub fn process_raw_event(
        raw_event: RawInputEvent,
        current_state: &InteractionState,
    ) -> Option<InteractionEvent> {
        match raw_event {
            RawInputEvent::MouseWheel { delta, position } => {
                let factor = if delta > 0.0 { 1.2 } else { 1.0 / 1.2 };
                Some(InteractionEvent::Zoom {
                    factor,
                    center: Point2D::new(position.0, position.1),
                })
            }
            
            RawInputEvent::MouseMove { position, button_pressed } => {
                if button_pressed {
                    // Calculate pan delta from last position
                    let delta = Vector2D::new(
                        position.0 - current_state.last_mouse_pos.x,
                        position.1 - current_state.last_mouse_pos.y,
                    );
                    Some(InteractionEvent::Pan { delta })
                } else {
                    Some(InteractionEvent::Hover {
                        point: Point2D::new(position.0, position.1),
                    })
                }
            }
            
            RawInputEvent::MouseClick { position, button } => {
                match button {
                    MouseButton::Left => Some(InteractionEvent::SelectPoint {
                        point: Point2D::new(position.0, position.1),
                    }),
                    MouseButton::Right => Some(InteractionEvent::ShowTooltip {
                        content: "Right-click menu".to_string(),
                        position: Point2D::new(position.0, position.1),
                    }),
                    _ => None,
                }
            }
            
            RawInputEvent::KeyPress { key } => {
                match key.as_str() {
                    "Escape" => Some(InteractionEvent::Reset),
                    "Delete" => Some(InteractionEvent::ClearSelection),
                    _ => None,
                }
            }
        }
    }
    
    /// Validate event parameters
    pub fn validate_event(event: &InteractionEvent) -> Result<()> {
        match event {
            InteractionEvent::Zoom { factor, .. } => {
                if *factor <= 0.0 || factor.is_infinite() || factor.is_nan() {
                    return Err(PlottingError::InvalidParameter(
                        format!("Invalid zoom factor: {}", factor)
                    ));
                }
            }
            InteractionEvent::Pan { delta } => {
                if delta.x.is_infinite() || delta.x.is_nan() || 
                   delta.y.is_infinite() || delta.y.is_nan() {
                    return Err(PlottingError::InvalidParameter(
                        "Invalid pan delta".to_string()
                    ));
                }
            }
            _ => {} // Other events don't need validation
        }
        Ok(())
    }
}

/// Raw input events from windowing system (winit)
#[derive(Debug, Clone, PartialEq)]
pub enum RawInputEvent {
    MouseWheel {
        delta: f64,
        position: (f64, f64),
    },
    MouseMove {
        position: (f64, f64),
        button_pressed: bool,
    },
    MouseClick {
        position: (f64, f64),
        button: MouseButton,
    },
    KeyPress {
        key: String,
    },
}

/// Mouse button identification
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

// Import InteractionState from state module (forward declaration)
use super::state::InteractionState;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point2d_operations() {
        let p1 = Point2D::new(3.0, 4.0);
        let p2 = Point2D::new(0.0, 0.0);
        
        assert_eq!(p1.distance_to(p2), 5.0); // 3-4-5 triangle
    }

    #[test]
    fn test_rectangle_operations() {
        let rect = Rectangle::new(0.0, 0.0, 10.0, 10.0);
        
        assert!(rect.contains(Point2D::new(5.0, 5.0)));
        assert!(!rect.contains(Point2D::new(15.0, 5.0)));
        assert_eq!(rect.width(), 10.0);
        assert_eq!(rect.height(), 10.0);
        assert_eq!(rect.center(), Point2D::new(5.0, 5.0));
    }

    #[test]
    fn test_event_validation() {
        // Valid zoom
        let valid_zoom = InteractionEvent::Zoom {
            factor: 1.5,
            center: Point2D::new(100.0, 100.0),
        };
        assert!(EventProcessor::validate_event(&valid_zoom).is_ok());

        // Invalid zoom (negative factor)
        let invalid_zoom = InteractionEvent::Zoom {
            factor: -1.0,
            center: Point2D::new(100.0, 100.0),
        };
        assert!(EventProcessor::validate_event(&invalid_zoom).is_err());

        // Valid pan
        let valid_pan = InteractionEvent::Pan {
            delta: Vector2D::new(10.0, -5.0),
        };
        assert!(EventProcessor::validate_event(&valid_pan).is_ok());
    }

    #[test]
    fn test_plot_id_generation() {
        let id1 = PlotId::new();
        let id2 = PlotId::new();
        
        assert_ne!(id1, id2);
        assert!(id1.0 < id2.0); // IDs should be increasing
    }

    #[test]
    fn test_raw_event_processing() {
        let state = InteractionState::default(); // We'll define this in state.rs
        
        let mouse_wheel = RawInputEvent::MouseWheel {
            delta: 1.0,
            position: (50.0, 50.0),
        };
        
        let processed = EventProcessor::process_raw_event(mouse_wheel, &state);
        
        match processed {
            Some(InteractionEvent::Zoom { factor, center }) => {
                assert_eq!(factor, 1.2);
                assert_eq!(center, Point2D::new(50.0, 50.0));
            }
            _ => panic!("Expected zoom event"),
        }
    }
}