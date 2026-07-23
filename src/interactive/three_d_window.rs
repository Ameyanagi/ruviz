use std::sync::Arc;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::core::{InputEvent3D, InteractivePlot3DSession, PlottingError, PointerButton3D, Result};
use crate::render::three_d::gpu::SurfacePresenter3D;

const DOUBLE_CLICK_INTERVAL: Duration = Duration::from_millis(350);
const DOUBLE_CLICK_DISTANCE_PX: f64 = 5.0;
const LINE_SCROLL_DELTA_PX: f32 = 50.0;

/// Show a retained 3d session in the native winit adapter.
///
/// Geometry and Axis3 are composed directly into a retained wgpu surface. No
/// presented frame performs GPU readback or uploads a CPU-rendered frame.
pub fn show_interactive_3d(session: InteractivePlot3DSession) -> Result<()> {
    let (width, height) = session.size_px();
    let title = session.title().unwrap_or("ruviz 3d").to_string();
    let event_loop = EventLoop::new()
        .map_err(|error| PlottingError::RenderError(format!("3D event loop: {error}")))?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = ThreeDWindowApp {
        session,
        title,
        requested_size: PhysicalSize::new(width.max(1), height.max(1)),
        window: None,
        presenter: None,
        cursor: PhysicalPosition::new(0.0, 0.0),
        last_left_click: None,
        error: None,
    };
    event_loop
        .run_app(&mut app)
        .map_err(|error| PlottingError::RenderError(format!("3D event loop: {error}")))?;
    app.error.map_or(Ok(()), Err)
}

struct ThreeDWindowApp {
    session: InteractivePlot3DSession,
    title: String,
    requested_size: PhysicalSize<u32>,
    window: Option<Arc<Window>>,
    presenter: Option<SurfacePresenter3D>,
    cursor: PhysicalPosition<f64>,
    last_left_click: Option<(Instant, PhysicalPosition<f64>)>,
    error: Option<PlottingError>,
}

impl ThreeDWindowApp {
    fn handle_event(&mut self, event: InputEvent3D) -> Result<()> {
        let result = self.session.handle_input(event)?;
        if result.request_redraw {
            self.request_redraw();
        }
        Ok(())
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>, scale_factor: f64) -> Result<()> {
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        self.requested_size = size;
        self.session
            .resize(size.width, size.height, scale_factor as f32)?;
        if let Some(presenter) = &mut self.presenter {
            presenter.resize(size.width, size.height)?;
        }
        self.request_redraw();
        Ok(())
    }

    fn render(&mut self) -> Result<()> {
        let Some(window) = self.window.as_ref() else {
            return Ok(());
        };
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        self.session
            .resize(size.width, size.height, window.scale_factor() as f32)?;
        let presenter = self.presenter.as_mut().ok_or_else(|| {
            PlottingError::RenderError("direct 3d surface presenter was not retained".to_string())
        })?;
        let diagnostics = self.session.present_direct(presenter)?;
        if let Some(diagnostics) = diagnostics {
            debug_assert_eq!(diagnostics.actual_backend, "gpu3d-surface");
            debug_assert_eq!(diagnostics.readback_bytes, 0);
        }
        Ok(())
    }

    fn pointer_button(button: MouseButton) -> Option<PointerButton3D> {
        match button {
            MouseButton::Left => Some(PointerButton3D::Left),
            MouseButton::Middle => Some(PointerButton3D::Middle),
            MouseButton::Right => Some(PointerButton3D::Right),
            _ => None,
        }
    }

    fn wheel_delta(delta: MouseScrollDelta) -> f32 {
        match delta {
            MouseScrollDelta::LineDelta(_, y) => y * LINE_SCROLL_DELTA_PX,
            MouseScrollDelta::PixelDelta(position) => -(position.y as f32),
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: PlottingError) {
        self.error = Some(error);
        event_loop.exit();
    }
}

impl ApplicationHandler for ThreeDWindowApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = WindowAttributes::default()
            .with_title(self.title.clone())
            .with_inner_size(self.requested_size);
        match event_loop.create_window(attributes) {
            Ok(window) => {
                let window = Arc::new(window);
                self.requested_size = window.inner_size();
                match SurfacePresenter3D::new(
                    Arc::clone(&window),
                    self.requested_size.width.max(1),
                    self.requested_size.height.max(1),
                ) {
                    Ok(presenter) => {
                        self.presenter = Some(presenter);
                        self.window = Some(window);
                    }
                    Err(error) => {
                        self.fail(event_loop, error);
                        return;
                    }
                }
                self.request_redraw();
            }
            Err(error) => self.fail(
                event_loop,
                PlottingError::RenderError(format!("3D window creation: {error}")),
            ),
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let result = match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                Ok(())
            }
            WindowEvent::Resized(size) => {
                let scale = self
                    .window
                    .as_ref()
                    .map_or(1.0, |window| window.scale_factor());
                self.resize(size, scale)
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let size = self
                    .window
                    .as_ref()
                    .map_or(self.requested_size, |window| window.inner_size());
                self.resize(size, scale_factor)
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = position;
                self.handle_event(InputEvent3D::PointerMove {
                    x: position.x as f32,
                    y: position.y as f32,
                })
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let Some(button3d) = Self::pointer_button(button) else {
                    return;
                };
                let x = self.cursor.x as f32;
                let y = self.cursor.y as f32;
                match state {
                    ElementState::Pressed => self.handle_event(InputEvent3D::PointerDown {
                        x,
                        y,
                        button: button3d,
                    }),
                    ElementState::Released => {
                        let pointer_up = self.handle_event(InputEvent3D::PointerUp {
                            x,
                            y,
                            button: button3d,
                        });
                        if pointer_up.is_err() || button3d != PointerButton3D::Left {
                            pointer_up
                        } else {
                            let now = Instant::now();
                            let is_double = self.last_left_click.is_some_and(|(when, position)| {
                                now.duration_since(when) <= DOUBLE_CLICK_INTERVAL
                                    && (self.cursor.x - position.x)
                                        .hypot(self.cursor.y - position.y)
                                        <= DOUBLE_CLICK_DISTANCE_PX
                            });
                            self.last_left_click = if is_double {
                                None
                            } else {
                                Some((now, self.cursor))
                            };
                            if is_double {
                                self.handle_event(InputEvent3D::DoubleClick {
                                    x,
                                    y,
                                    button: PointerButton3D::Left,
                                })
                            } else {
                                Ok(())
                            }
                        }
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => self.handle_event(InputEvent3D::Wheel {
                delta_y: Self::wheel_delta(delta),
            }),
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed
                    && matches!(event.logical_key, Key::Named(NamedKey::Escape)) =>
            {
                self.handle_event(InputEvent3D::Escape)
            }
            WindowEvent::RedrawRequested => self.render(),
            _ => Ok(()),
        };
        if let Err(error) = result {
            self.fail(event_loop, error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wheel_normalization_has_one_zoom_direction() {
        assert_eq!(
            ThreeDWindowApp::wheel_delta(MouseScrollDelta::LineDelta(0.0, 2.0)),
            100.0
        );
        assert_eq!(
            ThreeDWindowApp::wheel_delta(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
                0.0, -12.0
            ))),
            12.0
        );
    }

    #[test]
    fn only_mvp_pointer_buttons_are_mapped() {
        assert_eq!(
            ThreeDWindowApp::pointer_button(MouseButton::Left),
            Some(PointerButton3D::Left)
        );
        assert_eq!(ThreeDWindowApp::pointer_button(MouseButton::Back), None);
    }
}
