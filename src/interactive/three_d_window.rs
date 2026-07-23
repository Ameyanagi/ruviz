use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use softbuffer::{Context as SoftbufferContext, Surface as SoftbufferSurface};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::core::{InputEvent3D, InteractivePlot3DSession, PlottingError, PointerButton3D, Result};

type WindowSurface = SoftbufferSurface<OwnedDisplayHandle, Arc<Window>>;

const DOUBLE_CLICK_INTERVAL: Duration = Duration::from_millis(350);
const DOUBLE_CLICK_DISTANCE_PX: f64 = 5.0;
const LINE_SCROLL_DELTA_PX: f32 = 50.0;

/// Show a retained 3d session in the native winit adapter.
///
/// This adapter currently presents the diagnosed GPU-readback fallback through
/// softbuffer. The direct-surface adapter is a separate performance gate.
pub fn show_interactive_3d(session: InteractivePlot3DSession) -> Result<()> {
    let (width, height) = session.size_px();
    let title = session.title().unwrap_or("ruviz 3d").to_string();
    let event_loop = EventLoop::new()
        .map_err(|error| PlottingError::RenderError(format!("3D event loop: {error}")))?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let context = SoftbufferContext::new(event_loop.owned_display_handle()).map_err(|error| {
        PlottingError::RenderError(format!("3D softbuffer display context: {error}"))
    })?;
    let mut app = ThreeDWindowApp {
        session,
        title,
        requested_size: PhysicalSize::new(width.max(1), height.max(1)),
        context,
        window: None,
        surface: None,
        surface_size: None,
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
    context: SoftbufferContext<OwnedDisplayHandle>,
    window: Option<Arc<Window>>,
    surface: Option<WindowSurface>,
    surface_size: Option<(u32, u32)>,
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
        self.surface_size = None;
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
        let (image, diagnostics) = self.session.render_gpu_readback()?;
        debug_assert_eq!(diagnostics.actual_backend, "gpu3d-readback-fallback");

        if self.surface.is_none() {
            self.surface = Some(
                SoftbufferSurface::new(&self.context, Arc::clone(window)).map_err(|error| {
                    PlottingError::RenderError(format!("3D softbuffer surface: {error}"))
                })?,
            );
        }
        let width = NonZeroU32::new(size.width).ok_or(PlottingError::InvalidDimensions {
            width: size.width,
            height: size.height,
        })?;
        let height = NonZeroU32::new(size.height).ok_or(PlottingError::InvalidDimensions {
            width: size.width,
            height: size.height,
        })?;
        let surface = self.surface.as_mut().ok_or_else(|| {
            PlottingError::RenderError("3D softbuffer surface was not retained".to_string())
        })?;
        if self.surface_size != Some((size.width, size.height)) {
            surface.resize(width, height).map_err(|error| {
                PlottingError::RenderError(format!("3D softbuffer resize: {error}"))
            })?;
            self.surface_size = Some((size.width, size.height));
        }
        let mut buffer = surface.buffer_mut().map_err(|error| {
            PlottingError::RenderError(format!("3D softbuffer acquire: {error}"))
        })?;
        if buffer.len() != image.pixels.len() / 4 {
            return Err(PlottingError::RenderError(format!(
                "3D presentation size mismatch: {} pixels for {} surface entries",
                image.pixels.len() / 4,
                buffer.len()
            )));
        }
        for (destination, rgba) in buffer.iter_mut().zip(image.pixels.chunks_exact(4)) {
            *destination =
                u32::from(rgba[2]) | (u32::from(rgba[1]) << 8) | (u32::from(rgba[0]) << 16);
        }
        buffer
            .present()
            .map_err(|error| PlottingError::RenderError(format!("3D softbuffer present: {error}")))
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
                self.window = Some(window);
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
