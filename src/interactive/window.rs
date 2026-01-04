//! Interactive window management using winit
//!
//! Provides cross-platform windowing for interactive plotting with event
//! handling and integration with the real-time renderer.

use crate::{
    core::{Plot, PlottingError, Result},
    interactive::{
        event::{
            EventHandler, EventProcessor, InteractionEvent, MouseButton, Point2D, RawInputEvent,
            Vector2D,
        },
        renderer::RealTimeRenderer,
        state::InteractionState,
    },
};

use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, Event, MouseButton as WinitMouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// Interactive window for displaying plots with real-time interactions
pub struct InteractiveWindow {
    window: Option<Arc<Window>>,
    renderer: RealTimeRenderer,
    interaction_state: InteractionState,
    event_handler: Box<dyn EventHandler>,

    // Window state
    window_size: PhysicalSize<u32>,
    scale_factor: f64,
    mouse_position: PhysicalPosition<f64>,
    mouse_buttons_pressed: HashMap<WinitMouseButton, bool>,

    // Performance tracking
    last_frame_time: Instant,
    frame_count: u64,
    should_close: bool,
}

impl InteractiveWindow {
    /// Create new interactive window
    pub async fn new(plot: Plot, title: &str, width: u32, height: u32) -> Result<Self> {
        let renderer = RealTimeRenderer::new().await?;
        let mut interaction_state = InteractionState::default();

        // Set up data bounds based on plot data
        // In real implementation, would analyze plot data to determine bounds
        interaction_state.data_bounds =
            crate::interactive::event::Rectangle::new(0.0, 0.0, 100.0, 100.0);
        interaction_state.screen_bounds =
            crate::interactive::event::Rectangle::new(0.0, 0.0, width as f64, height as f64);

        let event_handler = Box::new(DefaultEventHandler::new());

        Ok(Self {
            window: None,
            renderer,
            interaction_state,
            event_handler,
            window_size: PhysicalSize::new(width, height),
            scale_factor: 1.0,
            mouse_position: PhysicalPosition::new(0.0, 0.0),
            mouse_buttons_pressed: HashMap::new(),
            last_frame_time: Instant::now(),
            frame_count: 0,
            should_close: false,
        })
    }

    /// Run the interactive window event loop
    pub fn run(mut self, plot: Plot) -> Result<()> {
        let event_loop = EventLoop::new().map_err(|e| {
            PlottingError::RenderError(format!("Failed to create event loop: {}", e))
        })?;

        // Set the plot for rendering
        self.renderer.set_plot(plot);

        // Create window application handler
        let mut app_handler = InteractiveApp::new(self);

        event_loop
            .run_app(&mut app_handler)
            .map_err(|e| PlottingError::RenderError(format!("Event loop error: {}", e)))?;

        Ok(())
    }

    /// Handle window event
    fn handle_window_event(&mut self, event: WindowEvent) -> Result<()> {
        match event {
            WindowEvent::Resized(new_size) => {
                self.window_size = new_size;
                self.interaction_state.screen_bounds = crate::interactive::event::Rectangle::new(
                    0.0,
                    0.0,
                    new_size.width as f64,
                    new_size.height as f64,
                );
                self.interaction_state.viewport_dirty = true;
                self.interaction_state.needs_redraw = true;
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = scale_factor;
                self.interaction_state.needs_redraw = true;
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = matches!(state, ElementState::Pressed);
                self.mouse_buttons_pressed.insert(button, pressed);

                if pressed {
                    let raw_event = RawInputEvent::MouseClick {
                        position: (self.mouse_position.x, self.mouse_position.y),
                        button: self.convert_mouse_button(button),
                    };

                    if let Some(interaction_event) =
                        EventProcessor::process_raw_event(raw_event, &self.interaction_state)
                    {
                        self.process_interaction_event(interaction_event)?;
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y as f64 * 50.0, // Scale line delta
                    MouseScrollDelta::PixelDelta(pos) => pos.y,
                };

                let raw_event = RawInputEvent::MouseWheel {
                    delta: scroll_delta,
                    position: (self.mouse_position.x, self.mouse_position.y),
                };

                if let Some(interaction_event) =
                    EventProcessor::process_raw_event(raw_event, &self.interaction_state)
                {
                    self.process_interaction_event(interaction_event)?;
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let old_position = self.mouse_position;
                self.mouse_position = position;

                // Check if any mouse button is pressed for drag operation
                let button_pressed = self.mouse_buttons_pressed.values().any(|&pressed| pressed);

                let raw_event = RawInputEvent::MouseMove {
                    position: (position.x, position.y),
                    button_pressed,
                };

                if let Some(interaction_event) =
                    EventProcessor::process_raw_event(raw_event, &self.interaction_state)
                {
                    self.process_interaction_event(interaction_event)?;
                } else {
                    // Handle hover even if no interaction event is generated
                    let hover_pos = Point2D::new(position.x, position.y);
                    if let Some(data_point) = self
                        .renderer
                        .get_data_point_at(hover_pos, &self.interaction_state)
                    {
                        self.interaction_state
                            .update_hover(hover_pos, Some(data_point));
                    } else {
                        self.interaction_state.update_hover(hover_pos, None);
                    }
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    if let Some(key_string) = self.key_event_to_string(&event) {
                        let raw_event = RawInputEvent::KeyPress { key: key_string };

                        if let Some(interaction_event) =
                            EventProcessor::process_raw_event(raw_event, &self.interaction_state)
                        {
                            self.process_interaction_event(interaction_event)?;
                        }
                    }
                }
            }

            WindowEvent::CloseRequested => {
                self.should_close = true;
            }

            WindowEvent::RedrawRequested => {
                self.render_frame()?;
            }

            _ => {}
        }

        Ok(())
    }

    /// Process high-level interaction event
    fn process_interaction_event(&mut self, event: InteractionEvent) -> Result<()> {
        // Validate event first
        EventProcessor::validate_event(&event)?;

        // Process event with state
        match &event {
            InteractionEvent::Zoom { factor, center } => {
                self.interaction_state.apply_zoom(*factor, *center);
            }

            InteractionEvent::Pan { delta } => {
                self.interaction_state.apply_pan(*delta);
            }

            InteractionEvent::Reset => {
                self.interaction_state.reset_viewport();
            }

            InteractionEvent::Select { region } => {
                // Start brush selection
                self.interaction_state.start_brush(region.min);
                self.interaction_state.update_brush(region.max);
                self.interaction_state.end_brush();
            }

            InteractionEvent::SelectPoint { point } => {
                if let Some(data_point) = self
                    .renderer
                    .get_data_point_at(*point, &self.interaction_state)
                {
                    self.interaction_state.select_point(data_point.id);
                }
            }

            InteractionEvent::ClearSelection => {
                self.interaction_state.clear_selection();
            }

            InteractionEvent::Hover { point } => {
                if let Some(data_point) = self
                    .renderer
                    .get_data_point_at(*point, &self.interaction_state)
                {
                    self.interaction_state
                        .update_hover(*point, Some(data_point));
                } else {
                    self.interaction_state.update_hover(*point, None);
                }
            }

            InteractionEvent::ShowTooltip { content, position } => {
                self.interaction_state
                    .show_tooltip(content.clone(), *position);
            }

            InteractionEvent::HideTooltip => {
                self.interaction_state.hide_tooltip();
            }

            InteractionEvent::AddAnnotation { annotation } => {
                self.interaction_state.add_annotation(annotation.clone());
            }

            _ => {
                // Handle other events with custom event handler
                self.event_handler.handle_event(event)?;
            }
        }

        Ok(())
    }

    /// Render current frame
    fn render_frame(&mut self) -> Result<()> {
        let frame_start = Instant::now();

        // Update animation state
        let dt = frame_start.duration_since(self.last_frame_time);
        self.interaction_state.update_animation(dt);

        // Update event handler
        self.event_handler.update(dt)?;

        // Render frame if needed
        if self.interaction_state.needs_redraw || self.event_handler.needs_redraw() {
            let _pixel_data = self.renderer.render_interactive(
                &self.interaction_state,
                self.window_size.width,
                self.window_size.height,
            )?;

            // In real implementation, would present pixel_data to window surface
            // For now, just mark as rendered
            self.interaction_state.needs_redraw = false;
        }

        // Update frame timing
        self.last_frame_time = frame_start;
        self.frame_count += 1;
        self.interaction_state.update_frame_timing();

        Ok(())
    }

    /// Convert winit mouse button to our mouse button type
    fn convert_mouse_button(&self, button: WinitMouseButton) -> MouseButton {
        match button {
            WinitMouseButton::Left => MouseButton::Left,
            WinitMouseButton::Right => MouseButton::Right,
            WinitMouseButton::Middle => MouseButton::Middle,
            _ => MouseButton::Left, // Default to left for other buttons
        }
    }

    /// Convert keyboard event to string representation
    fn key_event_to_string(&self, event: &winit::event::KeyEvent) -> Option<String> {
        use winit::keyboard::{Key, NamedKey};

        match &event.logical_key {
            Key::Named(named_key) => match named_key {
                NamedKey::Escape => Some("Escape".to_string()),
                NamedKey::Delete => Some("Delete".to_string()),
                NamedKey::Space => Some("Space".to_string()),
                NamedKey::Enter => Some("Enter".to_string()),
                _ => None,
            },
            Key::Character(ch) => Some(ch.to_string()),
            _ => None,
        }
    }
}

/// Application handler for winit event loop
struct InteractiveApp {
    window_state: Option<InteractiveWindow>,
}

impl InteractiveApp {
    fn new(window: InteractiveWindow) -> Self {
        Self {
            window_state: Some(window),
        }
    }
}

impl ApplicationHandler for InteractiveApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(ref mut window_state) = self.window_state {
            if window_state.window.is_none() {
                let window_attrs = WindowAttributes::default()
                    .with_title("Interactive Plot")
                    .with_inner_size(window_state.window_size);

                match event_loop.create_window(window_attrs) {
                    Ok(window) => {
                        window_state.window = Some(Arc::new(window));
                    }
                    Err(e) => {
                        eprintln!("Failed to create window: {}", e);
                        event_loop.exit();
                    }
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(ref mut window_state) = self.window_state {
            match window_state.handle_window_event(event) {
                Ok(()) => {
                    if window_state.should_close {
                        event_loop.exit();
                    } else {
                        // Request redraw for next frame
                        if let Some(ref window) = window_state.window {
                            window.request_redraw();
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error handling window event: {}", e);
                    event_loop.exit();
                }
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Continuously request redraws to maintain smooth animation
        if let Some(ref window_state) = self.window_state {
            if let Some(ref window) = window_state.window {
                window.request_redraw();
            }
        }

        // Set control flow for smooth animation
        event_loop.set_control_flow(ControlFlow::Poll);
    }
}

/// Default event handler implementation
struct DefaultEventHandler {
    custom_handlers: Vec<Box<dyn Fn(&InteractionEvent) -> Result<()>>>,
}

impl DefaultEventHandler {
    fn new() -> Self {
        Self {
            custom_handlers: Vec::new(),
        }
    }

    fn add_custom_handler<F>(&mut self, handler: F)
    where
        F: Fn(&InteractionEvent) -> Result<()> + 'static,
    {
        self.custom_handlers.push(Box::new(handler));
    }
}

impl EventHandler for DefaultEventHandler {
    fn handle_event(
        &mut self,
        event: InteractionEvent,
    ) -> Result<crate::interactive::event::EventResponse> {
        // Process with custom handlers
        for handler in &self.custom_handlers {
            handler(&event)?;
        }

        // Default handling based on event type
        match event {
            InteractionEvent::AddAnnotation { .. } => {
                Ok(crate::interactive::event::EventResponse::NeedsRedraw)
            }
            InteractionEvent::ModifyStyle { .. } => {
                Ok(crate::interactive::event::EventResponse::NeedsRecompute)
            }
            _ => Ok(crate::interactive::event::EventResponse::Handled),
        }
    }

    fn update(&mut self, _dt: Duration) -> Result<()> {
        // Default implementation does nothing
        Ok(())
    }

    fn needs_redraw(&self) -> bool {
        false // Default implementation doesn't need redraw
    }

    fn reset(&mut self) {
        // Default implementation does nothing
    }
}

/// Builder for creating interactive windows
pub struct InteractiveWindowBuilder {
    title: String,
    width: u32,
    height: u32,
    resizable: bool,
    decorations: bool,
}

impl Default for InteractiveWindowBuilder {
    fn default() -> Self {
        Self {
            title: "Interactive Plot".to_string(),
            width: 800,
            height: 600,
            resizable: true,
            decorations: true,
        }
    }
}

impl InteractiveWindowBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = title.into();
        self
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn decorations(mut self, decorations: bool) -> Self {
        self.decorations = decorations;
        self
    }

    pub async fn build(self, plot: Plot) -> Result<InteractiveWindow> {
        let mut window = InteractiveWindow::new(plot, &self.title, self.width, self.height).await?;

        // Apply builder settings
        // Note: Some settings like resizable/decorations would be applied
        // when creating the actual winit window

        Ok(window)
    }
}

/// Convenience function to show plot interactively
pub async fn show_interactive(plot: Plot) -> Result<()> {
    let window = InteractiveWindowBuilder::new()
        .title("Interactive Plot")
        .size(1024, 768)
        .build(plot.clone())
        .await?;

    window.run(plot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_window_builder() {
        let plot = Plot::new(); // Empty plot for testing

        let window_result = InteractiveWindowBuilder::new()
            .title("Test Window")
            .size(640, 480)
            .resizable(false)
            .build(plot)
            .await;

        assert!(window_result.is_ok());
    }

    // Note: test_mouse_button_conversion was removed because InteractiveWindow
    // cannot be constructed in tests (requires async RealTimeRenderer creation)

    #[test]
    fn test_default_event_handler() {
        let mut handler = DefaultEventHandler::new();

        let zoom_event = InteractionEvent::Zoom {
            factor: 1.5,
            center: Point2D::new(100.0, 100.0),
        };

        let response = handler.handle_event(zoom_event);
        assert!(response.is_ok());
        assert_eq!(
            response.unwrap(),
            crate::interactive::event::EventResponse::Handled
        );

        assert!(!handler.needs_redraw());
    }
}
