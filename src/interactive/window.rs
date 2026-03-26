//! Interactive window management using winit
//!
//! Provides cross-platform windowing for interactive plotting with event
//! handling and integration with the real-time renderer.

use crate::{
    core::{Plot, PlotInputEvent, PlottingError, Result, ViewportPoint},
    interactive::{
        event::{EventHandler, EventProcessor, InteractionEvent, Point2D, Rectangle, Vector2D},
        renderer::RealTimeRenderer,
        state::InteractionState,
    },
};

use softbuffer::{Context as SoftbufferContext, Surface as SoftbufferSurface};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton as WinitMouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle},
    window::{Window, WindowAttributes, WindowId},
};

use std::{
    num::NonZeroU32,
    sync::Arc,
    time::{Duration, Instant},
};

const DRAG_THRESHOLD_PX: f64 = 3.0;
const LINE_SCROLL_DELTA_PX: f64 = 50.0;

type WindowSurface = SoftbufferSurface<OwnedDisplayHandle, Arc<Window>>;

/// Interactive window for displaying plots with real-time interactions
pub struct InteractiveWindow {
    window: Option<Arc<Window>>,
    surface_context: Option<SoftbufferContext<OwnedDisplayHandle>>,
    surface: Option<WindowSurface>,
    renderer: RealTimeRenderer,
    interaction_state: InteractionState,
    event_handler: Box<dyn EventHandler>,

    // Window state
    window_size: PhysicalSize<u32>,
    scale_factor: f64,
    mouse_position: PhysicalPosition<f64>,
    pointer_down_px: Option<Point2D>,
    last_pointer_px: Option<Point2D>,

    // Performance tracking
    last_frame_time: Instant,
    frame_count: u64,
    should_close: bool,
}

impl InteractiveWindow {
    /// Create new interactive window
    pub async fn new(plot: Plot, _title: &str, width: u32, height: u32) -> Result<Self> {
        let renderer = RealTimeRenderer::new().await?;
        // Set up data bounds based on plot data
        // In real implementation, would analyze plot data to determine bounds
        let interaction_state = InteractionState {
            data_bounds: crate::interactive::event::Rectangle::new(0.0, 0.0, 100.0, 100.0),
            screen_bounds: crate::interactive::event::Rectangle::new(
                0.0,
                0.0,
                width as f64,
                height as f64,
            ),
            ..Default::default()
        };

        let event_handler = Box::new(DefaultEventHandler::new());

        Ok(Self {
            window: None,
            surface_context: None,
            surface: None,
            renderer,
            interaction_state,
            event_handler,
            window_size: PhysicalSize::new(width, height),
            scale_factor: 1.0,
            mouse_position: PhysicalPosition::new(0.0, 0.0),
            pointer_down_px: None,
            last_pointer_px: None,
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
        self.surface_context = Some(
            SoftbufferContext::new(event_loop.owned_display_handle()).map_err(|e| {
                PlottingError::RenderError(format!(
                    "Failed to create window surface context: {}",
                    e
                ))
            })?,
        );

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
                self.apply_plot_input(
                    PlotInputEvent::Resize {
                        size_px: (new_size.width, new_size.height),
                        scale_factor: self.scale_factor as f32,
                    },
                    true,
                )?;
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = scale_factor;
                self.apply_plot_input(
                    PlotInputEvent::Resize {
                        size_px: (self.window_size.width, self.window_size.height),
                        scale_factor: scale_factor as f32,
                    },
                    true,
                )?;
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if button == WinitMouseButton::Left {
                    let position = self.current_pointer_position();
                    match state {
                        ElementState::Pressed => self.handle_left_button_pressed(position)?,
                        ElementState::Released => self.handle_left_button_released(position)?,
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y as f64 * LINE_SCROLL_DELTA_PX,
                    MouseScrollDelta::PixelDelta(pos) => pos.y,
                };
                self.handle_scroll_delta(scroll_delta)?;
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = position;
                self.interaction_state.mouse_in_window = true;
                self.handle_pointer_moved(Point2D::new(position.x, position.y))?;
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    if let Some(key_string) = self.key_event_to_string(&event) {
                        self.handle_key_string(&key_string)?;
                    }
                }
            }

            WindowEvent::CursorLeft { .. } => {
                self.interaction_state.mouse_in_window = false;
                self.reset_pointer_state();
                self.apply_plot_input(PlotInputEvent::ClearHover, false)?;
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

    fn current_pointer_position(&self) -> Point2D {
        Point2D::new(self.mouse_position.x, self.mouse_position.y)
    }

    fn ensure_surface_initialized(&mut self) -> Result<()> {
        if self.surface.is_some() {
            return Ok(());
        }

        let window = self.window.as_ref().ok_or_else(|| {
            PlottingError::RenderError(
                "Window surface requested before window creation".to_string(),
            )
        })?;
        let context = self.surface_context.as_ref().ok_or_else(|| {
            PlottingError::RenderError(
                "Window surface requested before context creation".to_string(),
            )
        })?;

        self.surface = Some(
            SoftbufferSurface::new(context, Arc::clone(window)).map_err(|e| {
                PlottingError::RenderError(format!("Failed to create window surface: {}", e))
            })?,
        );
        Ok(())
    }

    fn present_frame(&mut self, pixel_data: &[u8]) -> Result<()> {
        if self.window.is_none() || self.surface_context.is_none() {
            return Ok(());
        }

        self.ensure_surface_initialized()?;

        let width = NonZeroU32::new(self.window_size.width.max(1))
            .expect("window width is clamped to non-zero");
        let height = NonZeroU32::new(self.window_size.height.max(1))
            .expect("window height is clamped to non-zero");
        let surface = self
            .surface
            .as_mut()
            .expect("surface should be initialized");
        surface.resize(width, height).map_err(|e| {
            PlottingError::RenderError(format!("Failed to resize window surface: {}", e))
        })?;

        let mut buffer = surface.buffer_mut().map_err(|e| {
            PlottingError::RenderError(format!("Failed to acquire window buffer: {}", e))
        })?;
        copy_rgba_to_softbuffer(pixel_data, &mut buffer);
        buffer.present().map_err(|e| {
            PlottingError::RenderError(format!("Failed to present window buffer: {}", e))
        })
    }

    fn reset_pointer_state(&mut self) {
        self.pointer_down_px = None;
        self.last_pointer_px = None;
        self.interaction_state.mouse_button_pressed = false;
    }

    fn apply_plot_input(&mut self, event: PlotInputEvent, viewport_dirty: bool) -> Result<()> {
        self.renderer.apply_session_input(
            event,
            (self.window_size.width, self.window_size.height),
            self.scale_factor as f32,
        );
        self.sync_interaction_state_from_session()?;
        self.interaction_state.viewport_dirty = viewport_dirty;
        self.interaction_state.needs_redraw = true;
        Ok(())
    }

    fn sync_interaction_state_from_session(&mut self) -> Result<()> {
        let Some(snapshot) = self.renderer.viewport_snapshot()? else {
            return Ok(());
        };

        self.interaction_state.zoom_level = snapshot.zoom_level;
        self.interaction_state.pan_offset =
            Vector2D::new(snapshot.pan_offset.x, snapshot.pan_offset.y);
        self.interaction_state.data_bounds = Rectangle::new(
            snapshot.base_bounds.min.x,
            snapshot.base_bounds.min.y,
            snapshot.base_bounds.max.x,
            snapshot.base_bounds.max.y,
        );
        self.interaction_state.screen_bounds = Rectangle::new(
            snapshot.plot_area.min.x,
            snapshot.plot_area.min.y,
            snapshot.plot_area.max.x,
            snapshot.plot_area.max.y,
        );
        self.interaction_state.last_mouse_pos = self.current_pointer_position();
        self.interaction_state.hover_point = None;
        self.interaction_state.selected_points.clear();
        self.interaction_state.brushed_region = None;
        self.interaction_state.tooltip_visible = false;
        self.interaction_state.tooltip_content.clear();
        self.interaction_state.tooltip_position = Point2D::zero();
        Ok(())
    }

    fn plot_area_contains(&self, position: Point2D) -> Result<bool> {
        let Some(snapshot) = self.renderer.viewport_snapshot()? else {
            return Ok(false);
        };
        Ok(snapshot
            .plot_area
            .contains(ViewportPoint::new(position.x, position.y)))
    }

    fn handle_left_button_pressed(&mut self, position: Point2D) -> Result<()> {
        if !self.plot_area_contains(position)? {
            self.reset_pointer_state();
            return Ok(());
        }

        self.pointer_down_px = Some(position);
        self.last_pointer_px = Some(position);
        self.interaction_state.last_mouse_pos = position;
        self.interaction_state.mouse_button_pressed = true;
        Ok(())
    }

    fn handle_left_button_released(&mut self, position: Point2D) -> Result<()> {
        let pointer_down = self.pointer_down_px;
        self.interaction_state.last_mouse_pos = position;
        self.reset_pointer_state();

        let Some(pointer_down) = pointer_down else {
            return Ok(());
        };

        if pointer_down.distance_to(position) <= DRAG_THRESHOLD_PX
            && self.plot_area_contains(position)?
        {
            self.apply_plot_input(
                PlotInputEvent::SelectAt {
                    position_px: ViewportPoint::new(position.x, position.y),
                },
                false,
            )?;
        }

        Ok(())
    }

    fn handle_pointer_moved(&mut self, position: Point2D) -> Result<()> {
        self.interaction_state.last_mouse_pos = position;

        if let Some(pointer_down) = self.pointer_down_px {
            if pointer_down.distance_to(position) > DRAG_THRESHOLD_PX {
                let previous = self.last_pointer_px.unwrap_or(pointer_down);
                let delta_x = position.x - previous.x;
                let delta_y = position.y - previous.y;
                if delta_x.abs() > f64::EPSILON || delta_y.abs() > f64::EPSILON {
                    self.apply_plot_input(
                        PlotInputEvent::Pan {
                            delta_px: ViewportPoint::new(delta_x, delta_y),
                        },
                        true,
                    )?;
                    self.last_pointer_px = Some(position);
                }
            }
            return Ok(());
        }

        if self.plot_area_contains(position)? {
            self.apply_plot_input(
                PlotInputEvent::Hover {
                    position_px: ViewportPoint::new(position.x, position.y),
                },
                false,
            )?;
        } else {
            self.apply_plot_input(PlotInputEvent::ClearHover, false)?;
        }

        Ok(())
    }

    fn handle_scroll_delta(&mut self, delta_y: f64) -> Result<()> {
        if delta_y.abs() <= f64::EPSILON {
            return Ok(());
        }

        let position = self.current_pointer_position();
        if !self.plot_area_contains(position)? {
            return Ok(());
        }

        let factor = (1.0025f64).powf(-delta_y).clamp(0.25, 4.0);
        self.apply_plot_input(
            PlotInputEvent::Zoom {
                factor,
                center_px: ViewportPoint::new(position.x, position.y),
            },
            true,
        )
    }

    fn handle_key_string(&mut self, key: &str) -> Result<()> {
        match key {
            "Escape" => self.apply_plot_input(PlotInputEvent::ResetView, true),
            "Delete" => self.apply_plot_input(PlotInputEvent::ClearSelection, false),
            _ => Ok(()),
        }
    }

    /// Process high-level interaction event
    fn process_interaction_event(&mut self, event: InteractionEvent) -> Result<()> {
        // Validate event first
        EventProcessor::validate_event(&event)?;

        // Process event with state
        match event {
            InteractionEvent::Zoom { factor, center } => self.apply_plot_input(
                PlotInputEvent::Zoom {
                    factor,
                    center_px: ViewportPoint::new(center.x, center.y),
                },
                true,
            )?,
            InteractionEvent::Pan { delta } => self.apply_plot_input(
                PlotInputEvent::Pan {
                    delta_px: ViewportPoint::new(delta.x, delta.y),
                },
                true,
            )?,
            InteractionEvent::Reset => self.apply_plot_input(PlotInputEvent::ResetView, true)?,
            InteractionEvent::Select { region } => {
                self.apply_plot_input(
                    PlotInputEvent::BrushStart {
                        position_px: ViewportPoint::new(region.min.x, region.min.y),
                    },
                    false,
                )?;
                self.apply_plot_input(
                    PlotInputEvent::BrushMove {
                        position_px: ViewportPoint::new(region.max.x, region.max.y),
                    },
                    false,
                )?;
                self.apply_plot_input(
                    PlotInputEvent::BrushEnd {
                        position_px: ViewportPoint::new(region.max.x, region.max.y),
                    },
                    false,
                )?;
            }
            InteractionEvent::SelectPoint { point } => self.apply_plot_input(
                PlotInputEvent::SelectAt {
                    position_px: ViewportPoint::new(point.x, point.y),
                },
                false,
            )?,
            InteractionEvent::ClearSelection => {
                self.apply_plot_input(PlotInputEvent::ClearSelection, false)?
            }
            InteractionEvent::Hover { point } => {
                if self.plot_area_contains(point)? {
                    self.apply_plot_input(
                        PlotInputEvent::Hover {
                            position_px: ViewportPoint::new(point.x, point.y),
                        },
                        false,
                    )?;
                } else {
                    self.apply_plot_input(PlotInputEvent::ClearHover, false)?;
                }
            }
            InteractionEvent::ShowTooltip { content, position } => self.apply_plot_input(
                PlotInputEvent::ShowTooltip {
                    content,
                    position_px: ViewportPoint::new(position.x, position.y),
                },
                false,
            )?,
            InteractionEvent::HideTooltip => {
                self.apply_plot_input(PlotInputEvent::HideTooltip, false)?
            }
            InteractionEvent::AddAnnotation { annotation } => {
                self.interaction_state.add_annotation(annotation);
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
            let pixel_data = self.renderer.render_interactive(
                &self.interaction_state,
                self.window_size.width,
                self.window_size.height,
                self.scale_factor as f32,
            )?;

            self.present_frame(&pixel_data)?;
            self.sync_interaction_state_from_session()?;
            self.interaction_state.needs_redraw = false;
            self.interaction_state.mark_viewport_clean();
        }

        // Update frame timing
        self.last_frame_time = frame_start;
        self.frame_count += 1;
        self.interaction_state.update_frame_timing();

        Ok(())
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
                        if let Err(err) = window_state.ensure_surface_initialized() {
                            eprintln!("Failed to initialize window surface: {}", err);
                            event_loop.exit();
                        }
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

fn copy_rgba_to_softbuffer(src_rgba: &[u8], dst_rgbx: &mut [u32]) {
    for (dst, rgba) in dst_rgbx.iter_mut().zip(src_rgba.chunks_exact(4)) {
        let red = rgba[0] as u32;
        let green = rgba[1] as u32;
        let blue = rgba[2] as u32;
        *dst = (red << 16) | (green << 8) | blue;
    }
}

/// Default event handler implementation
#[allow(clippy::type_complexity)] // Event handler callbacks need this flexibility
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
        let window = InteractiveWindow::new(plot, &self.title, self.width, self.height).await?;

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

    fn plot_area_center(window: &InteractiveWindow) -> Point2D {
        Point2D::new(
            (window.interaction_state.screen_bounds.min.x
                + window.interaction_state.screen_bounds.max.x)
                * 0.5,
            (window.interaction_state.screen_bounds.min.y
                + window.interaction_state.screen_bounds.max.y)
                * 0.5,
        )
    }

    fn viewport_snapshot(
        window: &InteractiveWindow,
    ) -> crate::core::plot::InteractiveViewportSnapshot {
        window
            .renderer
            .viewport_snapshot()
            .expect("viewport snapshot should succeed")
            .expect("interactive session should be available")
    }

    fn screen_to_data(
        visible_bounds: crate::core::ViewportRect,
        plot_area: crate::core::ViewportRect,
        position: Point2D,
    ) -> Point2D {
        let normalized_x = ((position.x - plot_area.min.x) / plot_area.width()).clamp(0.0, 1.0);
        let normalized_y = ((position.y - plot_area.min.y) / plot_area.height()).clamp(0.0, 1.0);
        Point2D::new(
            visible_bounds.min.x + visible_bounds.width() * normalized_x,
            visible_bounds.max.y - visible_bounds.height() * normalized_y,
        )
    }

    async fn interactive_window_for_test() -> InteractiveWindow {
        let plot: Plot = Plot::new()
            .line(&[0.0, 5.0, 10.0], &[0.0, 5.0, 10.0])
            .title("Interactive Test")
            .xlabel("X")
            .ylabel("Y")
            .xlim(0.0, 10.0)
            .ylim(0.0, 10.0)
            .into();
        let mut window = InteractiveWindow::new(plot.clone(), "Test Window", 640, 480)
            .await
            .expect("window should build");
        window.renderer.set_plot(plot);
        window
            .render_frame()
            .expect("initial render should populate session geometry");
        window
    }

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

    #[test]
    fn test_copy_rgba_to_softbuffer() {
        let src = [
            0x12, 0x34, 0x56, 0xff, //
            0xab, 0xcd, 0xef, 0x40,
        ];
        let mut dst = [0u32; 2];

        copy_rgba_to_softbuffer(&src, &mut dst);

        assert_eq!(dst, [0x00123456, 0x00abcdef]);
    }

    #[tokio::test]
    async fn test_scroll_zoom_keeps_cursor_anchor_stable() {
        let mut window = interactive_window_for_test().await;
        let before = viewport_snapshot(&window);
        let anchor = Point2D::new(before.plot_area.min.x, plot_area_center(&window).y);
        let anchor_before = screen_to_data(before.visible_bounds, before.plot_area, anchor);
        let expected_factor = (1.0025f64).powf(-LINE_SCROLL_DELTA_PX).clamp(0.25, 4.0);

        window.mouse_position = PhysicalPosition::new(anchor.x, anchor.y);
        window
            .handle_scroll_delta(LINE_SCROLL_DELTA_PX)
            .expect("scroll zoom should succeed");
        window
            .render_frame()
            .expect("render after zoom should succeed");

        let after = viewport_snapshot(&window);
        let anchor_after = screen_to_data(after.visible_bounds, after.plot_area, anchor);

        assert!(
            (after.zoom_level - expected_factor).abs() < 1e-9,
            "expected zoom {}, got {}",
            expected_factor,
            after.zoom_level
        );
        assert!(
            (anchor_before.x - anchor_after.x).abs() < 1e-6,
            "expected stable x anchor, before={anchor_before:?}, after={anchor_after:?}"
        );
        assert!(
            (anchor_before.y - anchor_after.y).abs() < 1e-6,
            "expected stable y anchor, before={anchor_before:?}, after={anchor_after:?}, before_snapshot={before:?}, after_snapshot={after:?}"
        );
    }

    #[tokio::test]
    async fn test_drag_pan_uses_incremental_pointer_deltas() {
        let mut window = interactive_window_for_test().await;
        let before = viewport_snapshot(&window);
        let start = plot_area_center(&window);

        window
            .handle_left_button_pressed(start)
            .expect("mouse down should succeed");
        window
            .handle_pointer_moved(Point2D::new(start.x + 40.0, start.y + 24.0))
            .expect("drag move should succeed");
        window
            .render_frame()
            .expect("render after pan should succeed");

        let after = viewport_snapshot(&window);
        let expected_dx = -(40.0 / before.plot_area.width()) * before.visible_bounds.width();
        let expected_dy = (24.0 / before.plot_area.height()) * before.visible_bounds.height();

        assert!(
            ((after.visible_bounds.min.x - before.visible_bounds.min.x) - expected_dx).abs() < 1e-6,
            "min.x delta mismatch: before={before:?}, after={after:?}, expected_dx={expected_dx}"
        );
        assert!(
            ((after.visible_bounds.max.x - before.visible_bounds.max.x) - expected_dx).abs() < 1e-6,
            "max.x delta mismatch: before={before:?}, after={after:?}, expected_dx={expected_dx}"
        );
        assert!(
            ((after.visible_bounds.min.y - before.visible_bounds.min.y) - expected_dy).abs() < 1e-6,
            "min.y delta mismatch: before={before:?}, after={after:?}, expected_dy={expected_dy}"
        );
        assert!(
            ((after.visible_bounds.max.y - before.visible_bounds.max.y) - expected_dy).abs() < 1e-6,
            "max.y delta mismatch: before={before:?}, after={after:?}, expected_dy={expected_dy}"
        );
    }

    #[tokio::test]
    async fn test_small_pointer_motion_selects_without_panning() {
        let mut window = interactive_window_for_test().await;
        let before = viewport_snapshot(&window);
        let center = plot_area_center(&window);

        window
            .handle_left_button_pressed(center)
            .expect("mouse down should succeed");
        window
            .handle_pointer_moved(Point2D::new(center.x + 2.0, center.y + 1.0))
            .expect("small move should succeed");
        window
            .handle_left_button_released(Point2D::new(center.x + 2.0, center.y + 1.0))
            .expect("mouse up should succeed");
        window
            .render_frame()
            .expect("render after click selection should succeed");

        let after = viewport_snapshot(&window);

        assert_eq!(after.visible_bounds, before.visible_bounds);
        assert_eq!(after.selected_count, 1);
    }
}
