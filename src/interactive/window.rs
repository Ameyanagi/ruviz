//! Interactive window management using winit
//!
//! Provides cross-platform windowing for interactive plotting with event
//! handling and integration with the real-time renderer.

use crate::{
    core::{Plot, PlotInputEvent, PlottingError, ReactiveSubscription, Result, ViewportPoint},
    interactive::{
        event::{EventHandler, EventProcessor, InteractionEvent, Point2D, Rectangle, Vector2D},
        renderer::{InteractiveRenderOutput, RealTimeRenderer},
        state::InteractionState,
    },
};

use softbuffer::{Context as SoftbufferContext, Surface as SoftbufferSurface};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton as WinitMouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy, OwnedDisplayHandle},
    window::{Window, WindowAttributes, WindowId},
};

use std::{
    num::NonZeroU32,
    sync::Arc,
    time::{Duration, Instant},
};

const DRAG_THRESHOLD_PX: f64 = 3.0;
const LINE_SCROLL_DELTA_PX: f64 = 50.0;
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

type WindowSurface = SoftbufferSurface<OwnedDisplayHandle, Arc<Window>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InteractiveAppEvent {
    ReactiveUpdate,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ActiveDrag {
    LeftZoom {
        anchor_px: Point2D,
        current_px: Point2D,
        crossed_threshold: bool,
    },
    RightPan {
        anchor_px: Point2D,
        last_px: Point2D,
        crossed_threshold: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PendingHover {
    Hover(Point2D),
    Clear,
}

/// Interactive window for displaying plots with real-time interactions
pub struct InteractiveWindow {
    window: Option<Arc<Window>>,
    surface_context: Option<SoftbufferContext<OwnedDisplayHandle>>,
    surface: Option<WindowSurface>,
    renderer: RealTimeRenderer,
    interaction_state: InteractionState,
    event_handler: Box<dyn EventHandler>,
    title: String,
    resizable: bool,
    decorations: bool,

    // Window state
    window_size: PhysicalSize<u32>,
    scale_factor: f64,
    mouse_position: PhysicalPosition<f64>,
    active_drag: Option<ActiveDrag>,
    pending_hover: Option<PendingHover>,
    surface_size: Option<(u32, u32)>,
    reactive_subscription: Option<ReactiveSubscription>,

    // Performance tracking
    last_frame_time: Instant,
    frame_count: u64,
    should_close: bool,
}

impl InteractiveWindow {
    /// Create new interactive window
    pub async fn new(plot: Plot, title: &str, width: u32, height: u32) -> Result<Self> {
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
            title: title.to_string(),
            resizable: true,
            decorations: true,
            window_size: PhysicalSize::new(width, height),
            scale_factor: 1.0,
            mouse_position: PhysicalPosition::new(0.0, 0.0),
            active_drag: None,
            pending_hover: None,
            surface_size: None,
            reactive_subscription: None,
            last_frame_time: Instant::now(),
            frame_count: 0,
            should_close: false,
        })
    }

    /// Run the interactive window event loop
    pub fn run(mut self, plot: Plot) -> Result<()> {
        let mut event_loop_builder = EventLoop::<InteractiveAppEvent>::with_user_event();
        #[cfg(target_os = "macos")]
        {
            use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};

            event_loop_builder
                .with_activation_policy(ActivationPolicy::Regular)
                .with_activate_ignoring_other_apps(true);
        }
        let event_loop = event_loop_builder.build().map_err(|e| {
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
        self.install_reactive_wakeup(event_loop.create_proxy());

        // Set the plot for rendering
        self.renderer.set_plot(plot);

        // Create window application handler
        let mut app_handler = InteractiveApp::new(self);

        event_loop
            .run_app(&mut app_handler)
            .map_err(|e| PlottingError::RenderError(format!("Event loop error: {}", e)))?;

        Ok(())
    }

    fn install_reactive_wakeup(&mut self, proxy: EventLoopProxy<InteractiveAppEvent>) {
        self.reactive_subscription = self.renderer.subscribe_reactive(move || {
            let _ = proxy.send_event(InteractiveAppEvent::ReactiveUpdate);
        });
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
                let position = self.current_pointer_position();
                match (button, state) {
                    (WinitMouseButton::Left, ElementState::Pressed) => {
                        self.handle_left_button_pressed(position)?
                    }
                    (WinitMouseButton::Left, ElementState::Released) => {
                        self.handle_left_button_released(position)?
                    }
                    (WinitMouseButton::Right, ElementState::Pressed) => {
                        self.handle_right_button_pressed(position)?
                    }
                    (WinitMouseButton::Right, ElementState::Released) => {
                        self.handle_right_button_released(position)?
                    }
                    _ => {}
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
                self.queue_hover_clear();
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
        self.surface_size = None;
        Ok(())
    }

    fn present_frame(&mut self, frame: &InteractiveRenderOutput) -> Result<()> {
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
        let surface_size = (width.get(), height.get());
        if self.surface_size != Some(surface_size) {
            surface.resize(width, height).map_err(|e| {
                PlottingError::RenderError(format!("Failed to resize window surface: {}", e))
            })?;
            self.surface_size = Some(surface_size);
        }

        let mut buffer = surface.buffer_mut().map_err(|e| {
            PlottingError::RenderError(format!("Failed to acquire window buffer: {}", e))
        })?;
        match frame {
            InteractiveRenderOutput::Pixels(pixel_data) => {
                copy_rgba_to_softbuffer(pixel_data, &mut buffer)
            }
            InteractiveRenderOutput::Layers(layers) => {
                copy_rgba_to_softbuffer(&layers.base.pixels, &mut buffer);
                for overlay in &layers.overlays {
                    blend_rgba_into_softbuffer(&overlay.pixels, &mut buffer);
                }
            }
        }
        buffer.present().map_err(|e| {
            PlottingError::RenderError(format!("Failed to present window buffer: {}", e))
        })
    }

    fn reset_pointer_state(&mut self) {
        let had_drag = self.active_drag.take().is_some();
        self.active_drag = None;
        self.interaction_state.mouse_button_pressed = false;
        if had_drag {
            self.interaction_state.needs_redraw = true;
        }
        self.clear_pending_hover();
        self.clear_brush_overlay();
    }

    fn clear_brush_overlay(&mut self) {
        let had_brush = self.interaction_state.brushed_region.take().is_some();
        self.interaction_state.brushed_region = None;
        self.interaction_state.brush_active = false;
        self.interaction_state.brush_start = None;
        if had_brush {
            self.interaction_state.needs_redraw = true;
        }
    }

    fn box_zoom_drag_active(&self) -> bool {
        matches!(self.active_drag, Some(ActiveDrag::LeftZoom { .. }))
    }

    fn apply_plot_input(&mut self, event: PlotInputEvent, viewport_dirty: bool) -> Result<()> {
        let session_changed = self.renderer.apply_session_input(
            event,
            (self.window_size.width, self.window_size.height),
            self.scale_factor as f32,
        );
        if session_changed || viewport_dirty {
            self.sync_interaction_state_from_session()?;
            self.interaction_state.viewport_dirty = viewport_dirty;
            self.interaction_state.needs_redraw = true;
        }
        Ok(())
    }

    fn clear_pending_hover(&mut self) {
        self.pending_hover = None;
    }

    fn queue_hover(&mut self, position: Point2D) {
        self.pending_hover = Some(PendingHover::Hover(position));
        self.interaction_state.needs_redraw = true;
    }

    fn queue_hover_clear(&mut self) {
        self.pending_hover = Some(PendingHover::Clear);
        self.interaction_state.needs_redraw = true;
    }

    fn flush_pending_hover(&mut self) -> Result<()> {
        let Some(pending_hover) = self.pending_hover.take() else {
            return Ok(());
        };

        match pending_hover {
            PendingHover::Hover(position) => self.apply_plot_input(
                PlotInputEvent::Hover {
                    position_px: ViewportPoint::new(position.x, position.y),
                },
                false,
            ),
            PendingHover::Clear => self.apply_plot_input(PlotInputEvent::ClearHover, false),
        }
    }

    fn sync_interaction_state_from_session(&mut self) -> Result<()> {
        let Some(snapshot) = self.renderer.viewport_snapshot()? else {
            return Ok(());
        };
        let preserve_brush_overlay = self.box_zoom_drag_active();
        let brushed_region = preserve_brush_overlay
            .then_some(self.interaction_state.brushed_region)
            .flatten();
        let brush_active = preserve_brush_overlay && self.interaction_state.brush_active;
        let brush_start = preserve_brush_overlay
            .then_some(self.interaction_state.brush_start)
            .flatten();

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
        self.interaction_state.brushed_region = brushed_region;
        self.interaction_state.brush_active = brush_active;
        self.interaction_state.brush_start = brush_start;
        self.interaction_state.tooltip_visible = false;
        self.interaction_state.tooltip_content.clear();
        self.interaction_state.tooltip_position = Point2D::zero();
        Ok(())
    }

    fn plot_area_rect(&self) -> Result<Option<Rectangle>> {
        Ok(self.renderer.viewport_snapshot()?.map(|snapshot| {
            Rectangle::new(
                snapshot.plot_area.min.x,
                snapshot.plot_area.min.y,
                snapshot.plot_area.max.x,
                snapshot.plot_area.max.y,
            )
        }))
    }

    fn plot_area_contains(&self, position: Point2D) -> Result<bool> {
        let Some(plot_area) = self.plot_area_rect()? else {
            return Ok(false);
        };
        Ok(plot_area.contains(position))
    }

    fn clamp_to_plot_area(&self, position: Point2D) -> Result<Option<Point2D>> {
        let Some(plot_area) = self.plot_area_rect()? else {
            return Ok(None);
        };
        Ok(Some(Point2D::new(
            position.x.clamp(plot_area.min.x, plot_area.max.x),
            position.y.clamp(plot_area.min.y, plot_area.max.y),
        )))
    }

    fn handle_left_button_pressed(&mut self, position: Point2D) -> Result<()> {
        if !self.plot_area_contains(position)? {
            self.reset_pointer_state();
            return Ok(());
        }

        self.clear_pending_hover();
        self.active_drag = Some(ActiveDrag::LeftZoom {
            anchor_px: position,
            current_px: position,
            crossed_threshold: false,
        });
        self.interaction_state.last_mouse_pos = position;
        self.interaction_state.mouse_button_pressed = true;
        self.interaction_state.brush_start = Some(position);
        Ok(())
    }

    fn handle_left_button_released(&mut self, position: Point2D) -> Result<()> {
        self.interaction_state.last_mouse_pos = position;
        let Some(active_drag) = self.active_drag else {
            return Ok(());
        };
        self.reset_pointer_state();

        if let ActiveDrag::LeftZoom {
            anchor_px,
            current_px,
            crossed_threshold,
        } = active_drag
        {
            if crossed_threshold {
                let release_px = self.clamp_to_plot_area(position)?.unwrap_or(current_px);
                let region_px = crate::core::ViewportRect::from_points(
                    ViewportPoint::new(anchor_px.x, anchor_px.y),
                    ViewportPoint::new(release_px.x, release_px.y),
                );
                if region_px.width() > f64::EPSILON && region_px.height() > f64::EPSILON {
                    self.apply_plot_input(PlotInputEvent::ZoomRect { region_px }, true)?;
                }
            } else if self.plot_area_contains(position)? {
                self.apply_plot_input(
                    PlotInputEvent::SelectAt {
                        position_px: ViewportPoint::new(position.x, position.y),
                    },
                    false,
                )?;
            }
        }

        Ok(())
    }

    fn handle_right_button_pressed(&mut self, position: Point2D) -> Result<()> {
        if !self.plot_area_contains(position)? {
            self.reset_pointer_state();
            return Ok(());
        }

        self.clear_pending_hover();
        self.active_drag = Some(ActiveDrag::RightPan {
            anchor_px: position,
            last_px: position,
            crossed_threshold: false,
        });
        self.interaction_state.last_mouse_pos = position;
        self.interaction_state.mouse_button_pressed = true;
        Ok(())
    }

    fn handle_right_button_released(&mut self, position: Point2D) -> Result<()> {
        self.interaction_state.last_mouse_pos = position;
        if matches!(self.active_drag, Some(ActiveDrag::RightPan { .. })) {
            self.reset_pointer_state();
        }
        Ok(())
    }

    fn handle_pointer_moved(&mut self, position: Point2D) -> Result<()> {
        self.interaction_state.last_mouse_pos = position;

        if let Some(active_drag) = self.active_drag {
            self.clear_pending_hover();
            match active_drag {
                ActiveDrag::LeftZoom {
                    anchor_px,
                    current_px: _,
                    crossed_threshold,
                } => {
                    let clamped_position = self.clamp_to_plot_area(position)?.unwrap_or(anchor_px);
                    let crossed_threshold_now =
                        crossed_threshold || anchor_px.distance_to(position) > DRAG_THRESHOLD_PX;
                    self.active_drag = Some(ActiveDrag::LeftZoom {
                        anchor_px,
                        current_px: clamped_position,
                        crossed_threshold: crossed_threshold_now,
                    });
                    if crossed_threshold_now {
                        if !crossed_threshold {
                            self.apply_plot_input(PlotInputEvent::ClearHover, false)?;
                        }
                        self.interaction_state.brush_start = Some(anchor_px);
                        self.interaction_state.brush_active = true;
                        self.interaction_state.brushed_region =
                            Some(Rectangle::from_points(anchor_px, clamped_position));
                        self.interaction_state.needs_redraw = true;
                    }
                    return Ok(());
                }
                ActiveDrag::RightPan {
                    anchor_px,
                    last_px,
                    crossed_threshold,
                } => {
                    let crossed_threshold_now =
                        crossed_threshold || anchor_px.distance_to(position) > DRAG_THRESHOLD_PX;
                    let previous = if crossed_threshold {
                        last_px
                    } else {
                        anchor_px
                    };
                    if crossed_threshold_now {
                        if !crossed_threshold {
                            self.apply_plot_input(PlotInputEvent::ClearHover, false)?;
                        }
                        let delta_x = position.x - previous.x;
                        let delta_y = position.y - previous.y;
                        if delta_x.abs() > f64::EPSILON || delta_y.abs() > f64::EPSILON {
                            self.active_drag = Some(ActiveDrag::RightPan {
                                anchor_px,
                                last_px: position,
                                crossed_threshold: true,
                            });
                            self.apply_plot_input(
                                PlotInputEvent::Pan {
                                    delta_px: ViewportPoint::new(delta_x, delta_y),
                                },
                                true,
                            )?;
                        } else {
                            self.active_drag = Some(ActiveDrag::RightPan {
                                anchor_px,
                                last_px,
                                crossed_threshold: true,
                            });
                        }
                    } else {
                        self.active_drag = Some(ActiveDrag::RightPan {
                            anchor_px,
                            last_px,
                            crossed_threshold: false,
                        });
                    }
                    return Ok(());
                }
            }
        }

        if self.plot_area_contains(position)? {
            self.queue_hover(position);
        } else {
            self.queue_hover_clear();
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
            "Escape" => {
                self.reset_pointer_state();
                self.apply_plot_input(PlotInputEvent::ResetView, true)
            }
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
        self.flush_pending_hover()?;

        // Render frame if needed
        if self.has_pending_redraw() {
            let frame = self.renderer.render_interactive(
                &self.interaction_state,
                self.window_size.width,
                self.window_size.height,
                self.scale_factor as f32,
            )?;

            self.present_frame(&frame)?;
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

    fn has_pending_redraw(&self) -> bool {
        self.pending_hover.is_some()
            || self.interaction_state.needs_redraw
            || self.event_handler.needs_redraw()
    }

    fn needs_frame_timer(&self) -> bool {
        !matches!(
            self.interaction_state.animation_state,
            crate::interactive::state::AnimationState::Idle
        ) || self.event_handler.needs_redraw()
    }

    fn request_redraw_if_needed(&self) {
        if !self.has_pending_redraw() && !self.needs_frame_timer() {
            return;
        }

        if let Some(ref window) = self.window {
            window.request_redraw();
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

impl ApplicationHandler<InteractiveAppEvent> for InteractiveApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(ref mut window_state) = self.window_state {
            if window_state.window.is_none() {
                let window_attrs = WindowAttributes::default()
                    .with_title(window_state.title.clone())
                    .with_inner_size(window_state.window_size)
                    .with_resizable(window_state.resizable)
                    .with_decorations(window_state.decorations)
                    .with_visible(true)
                    .with_active(true);

                match event_loop.create_window(window_attrs) {
                    Ok(window) => {
                        window.set_visible(true);
                        window.focus_window();
                        window.request_user_attention(Some(
                            winit::window::UserAttentionType::Informational,
                        ));
                        window_state.window = Some(Arc::new(window));
                        if let Err(err) = window_state.ensure_surface_initialized() {
                            eprintln!("Failed to initialize window surface: {}", err);
                            event_loop.exit();
                        } else {
                            window_state.request_redraw_if_needed();
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
                        window_state.request_redraw_if_needed();
                    }
                }
                Err(e) => {
                    eprintln!("Error handling window event: {}", e);
                    event_loop.exit();
                }
            }
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: InteractiveAppEvent) {
        if let (InteractiveAppEvent::ReactiveUpdate, Some(window_state)) =
            (event, self.window_state.as_mut())
        {
            window_state.interaction_state.needs_redraw = true;
            window_state.request_redraw_if_needed();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(ref window_state) = self.window_state {
            window_state.request_redraw_if_needed();
            if window_state.needs_frame_timer() {
                event_loop.set_control_flow(ControlFlow::wait_duration(FRAME_INTERVAL));
                return;
            }
        }

        event_loop.set_control_flow(ControlFlow::Wait);
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

fn blend_rgba_into_softbuffer(src_rgba: &[u8], dst_rgbx: &mut [u32]) {
    for (dst, rgba) in dst_rgbx.iter_mut().zip(src_rgba.chunks_exact(4)) {
        let alpha = rgba[3];
        if alpha == 0 {
            continue;
        }
        if alpha == u8::MAX {
            let red = rgba[0] as u32;
            let green = rgba[1] as u32;
            let blue = rgba[2] as u32;
            *dst = (red << 16) | (green << 8) | blue;
            continue;
        }

        let dst_red = ((*dst >> 16) & 0xff) as u8;
        let dst_green = ((*dst >> 8) & 0xff) as u8;
        let dst_blue = (*dst & 0xff) as u8;
        let alpha = alpha as f32 / 255.0;
        let red = blend_channel(dst_red, rgba[0], alpha) as u32;
        let green = blend_channel(dst_green, rgba[1], alpha) as u32;
        let blue = blend_channel(dst_blue, rgba[2], alpha) as u32;
        *dst = (red << 16) | (green << 8) | blue;
    }
}

fn blend_channel(background: u8, foreground: u8, alpha: f32) -> u8 {
    let bg = background as f32 / 255.0;
    let fg = foreground as f32 / 255.0;
    ((bg * (1.0 - alpha) + fg * alpha) * 255.0) as u8
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
        let mut window = InteractiveWindow::new(plot, &self.title, self.width, self.height).await?;

        window.resizable = self.resizable;
        window.decorations = self.decorations;

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

    fn assert_visible_bounds_close(
        actual: crate::core::ViewportRect,
        expected: crate::core::ViewportRect,
    ) {
        assert!(
            (actual.min.x - expected.min.x).abs() < 1e-6,
            "min.x mismatch: actual={actual:?}, expected={expected:?}"
        );
        assert!(
            (actual.max.x - expected.max.x).abs() < 1e-6,
            "max.x mismatch: actual={actual:?}, expected={expected:?}"
        );
        assert!(
            (actual.min.y - expected.min.y).abs() < 1e-6,
            "min.y mismatch: actual={actual:?}, expected={expected:?}"
        );
        assert!(
            (actual.max.y - expected.max.y).abs() < 1e-6,
            "max.y mismatch: actual={actual:?}, expected={expected:?}"
        );
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

    #[test]
    fn test_blend_rgba_into_softbuffer() {
        let src = [
            0xff, 0x00, 0x00, 0x80, //
            0x00, 0xff, 0x00, 0xff,
        ];
        let mut dst = [0x000000ff, 0x000000ff];

        blend_rgba_into_softbuffer(&src, &mut dst);

        assert_eq!(dst[1], 0x0000ff00);
        assert_ne!(dst[0], 0x000000ff);
        assert_ne!(dst[0], 0x00ff0000);
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
    async fn test_left_drag_zoom_rect_updates_visible_bounds() {
        let mut window = interactive_window_for_test().await;
        let before = viewport_snapshot(&window);
        let start = Point2D::new(before.plot_area.min.x + 40.0, before.plot_area.min.y + 36.0);
        let end = Point2D::new(
            before.plot_area.min.x + 220.0,
            before.plot_area.min.y + 168.0,
        );
        let start_data = screen_to_data(before.visible_bounds, before.plot_area, start);
        let end_data = screen_to_data(before.visible_bounds, before.plot_area, end);
        let expected = crate::core::ViewportRect::from_points(
            ViewportPoint::new(start_data.x, start_data.y),
            ViewportPoint::new(end_data.x, end_data.y),
        );

        window
            .handle_left_button_pressed(start)
            .expect("mouse down should succeed");
        window
            .handle_pointer_moved(end)
            .expect("box zoom drag should succeed");
        assert_eq!(
            window.interaction_state.brushed_region,
            Some(Rectangle::from_points(start, end))
        );
        window
            .handle_left_button_released(end)
            .expect("box zoom release should succeed");
        window
            .render_frame()
            .expect("render after box zoom should succeed");

        let after = viewport_snapshot(&window);
        assert_visible_bounds_close(after.visible_bounds, expected);
        assert_eq!(after.selected_count, 0);
        assert!(window.interaction_state.brushed_region.is_none());
    }

    #[tokio::test]
    async fn test_right_drag_pan_uses_incremental_pointer_deltas() {
        let mut window = interactive_window_for_test().await;
        let before = viewport_snapshot(&window);
        let start = plot_area_center(&window);
        let end = Point2D::new(start.x + 40.0, start.y + 24.0);

        window
            .handle_right_button_pressed(start)
            .expect("mouse down should succeed");
        window
            .handle_pointer_moved(end)
            .expect("drag move should succeed");
        window
            .handle_right_button_released(end)
            .expect("mouse up should succeed");
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
            "max.y delta mismatch: before={before:?}, after={after:?}, expected_dx={expected_dx}"
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

    #[tokio::test]
    async fn test_hover_updates_are_batched_until_render() {
        let mut window = interactive_window_for_test().await;
        let center = plot_area_center(&window);

        window
            .handle_pointer_moved(center)
            .expect("hover move should queue successfully");

        assert!(matches!(window.pending_hover, Some(PendingHover::Hover(_))));
        assert!(window.has_pending_redraw());

        window
            .render_frame()
            .expect("render after queued hover should succeed");

        assert!(window.pending_hover.is_none());
    }

    #[tokio::test]
    async fn test_static_window_uses_demand_driven_redraws() {
        let window = interactive_window_for_test().await;

        assert!(!window.has_pending_redraw());
        assert!(!window.needs_frame_timer());
    }
}
