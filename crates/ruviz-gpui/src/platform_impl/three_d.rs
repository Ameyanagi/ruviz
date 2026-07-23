use super::*;

use ruviz::core::{InputEvent3D, InteractivePlot3DSession, PickHit3D, PointerButton3D};

/// GPUI image-backed adapter for a retained ruviz 3d session.
///
/// With the `3d-gpu` feature this uses the truthfully diagnosed GPU-readback
/// fallback. GPUI direct texture interop remains a separate platform gate.
pub struct RuvizPlot3D {
    session: InteractivePlot3DSession,
    cached_image: Option<Arc<RenderImage>>,
    cached_snapshot: Option<ruviz::core::CameraSnapshot3D>,
    frame_size_px: (u32, u32),
    component_bounds: Option<Bounds<Pixels>>,
    focus_handle: FocusHandle,
    selected: Option<PickHit3D>,
}

impl RuvizPlot3D {
    fn new(session: InteractivePlot3DSession, cx: &mut Context<Self>) -> Self {
        Self {
            frame_size_px: session.size_px(),
            session,
            cached_image: None,
            cached_snapshot: None,
            component_bounds: None,
            focus_handle: cx.focus_handle(),
            selected: None,
        }
    }

    /// Current retained core session.
    pub fn session(&self) -> &InteractivePlot3DSession {
        &self.session
    }

    /// Mutable retained core session for host-driven camera changes.
    pub fn session_mut(&mut self) -> &mut InteractivePlot3DSession {
        &mut self.session
    }

    /// Most recent click selection.
    pub const fn selected(&self) -> Option<PickHit3D> {
        self.selected
    }

    fn render_image(&mut self) -> Result<Arc<RenderImage>> {
        #[cfg(feature = "gpu")]
        let image = self.session.render_gpu_readback()?.0;
        #[cfg(not(feature = "gpu"))]
        let image = self.session.render()?;
        Ok(render_image_from_ruviz(image))
    }

    fn prepaint(
        &mut self,
        bounds: Bounds<Pixels>,
        window: &mut Window,
    ) -> Result<Option<Arc<RenderImage>>> {
        let scale_factor = window.scale_factor();
        let width = (f64::from(bounds.size.width).max(1.0) * f64::from(scale_factor))
            .round()
            .clamp(1.0, f64::from(u32::MAX)) as u32;
        let height = (f64::from(bounds.size.height).max(1.0) * f64::from(scale_factor))
            .round()
            .clamp(1.0, f64::from(u32::MAX)) as u32;
        self.session.resize(width, height, scale_factor)?;
        self.frame_size_px = (width, height);
        self.component_bounds = Some(bounds);
        let snapshot = self.session.camera_snapshot();
        if self.cached_image.is_none() || self.cached_snapshot != Some(snapshot) {
            self.cached_image = Some(self.render_image()?);
            self.cached_snapshot = Some(snapshot);
        }
        Ok(self.cached_image.as_ref().map(Arc::clone))
    }

    fn local_position(&self, position: Point<Pixels>) -> Option<(f32, f32)> {
        let bounds = self.component_bounds?;
        if !bounds.contains(&position) {
            return None;
        }
        let width = f64::from(bounds.size.width).max(1.0);
        let height = f64::from(bounds.size.height).max(1.0);
        let x = f64::from(position.x - bounds.origin.x) / width * f64::from(self.frame_size_px.0);
        let y = f64::from(position.y - bounds.origin.y) / height * f64::from(self.frame_size_px.1);
        Some((x as f32, y as f32))
    }

    fn apply(&mut self, event: InputEvent3D, cx: &mut Context<Self>) -> Result<()> {
        let result = self.session.handle_input(event)?;
        if let Some(hit) = result.picked {
            self.selected = Some(hit);
        }
        if result.request_redraw {
            self.cached_image = None;
            cx.notify();
        }
        Ok(())
    }

    fn pointer_button(button: MouseButton) -> Option<PointerButton3D> {
        match button {
            MouseButton::Left => Some(PointerButton3D::Left),
            MouseButton::Middle => Some(PointerButton3D::Middle),
            MouseButton::Right => Some(PointerButton3D::Right),
            MouseButton::Navigate(_) => None,
        }
    }

    fn pointer_down(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) -> Result<()> {
        let Some((x, y)) = self.local_position(event.position) else {
            return Ok(());
        };
        let Some(button) = Self::pointer_button(event.button) else {
            return Ok(());
        };
        self.apply(InputEvent3D::PointerDown { x, y, button }, cx)
    }

    fn pointer_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) -> Result<()> {
        let Some((x, y)) = self.local_position(event.position) else {
            return Ok(());
        };
        self.apply(InputEvent3D::PointerMove { x, y }, cx)
    }

    fn pointer_up(&mut self, event: &MouseUpEvent, cx: &mut Context<Self>) -> Result<()> {
        let Some((x, y)) = self.local_position(event.position) else {
            return Ok(());
        };
        let Some(button) = Self::pointer_button(event.button) else {
            return Ok(());
        };
        self.apply(InputEvent3D::PointerUp { x, y, button }, cx)?;
        if button == PointerButton3D::Left && event.click_count >= 2 {
            self.apply(InputEvent3D::DoubleClick { x, y, button }, cx)?;
        }
        Ok(())
    }

    fn scroll(&mut self, event: &ScrollWheelEvent, cx: &mut Context<Self>) -> Result<()> {
        let delta_y = match event.delta {
            ScrollDelta::Pixels(point) => -f32::from(point.y),
            ScrollDelta::Lines(point) => point.y * LINE_SCROLL_DELTA_PX,
        };
        self.apply(InputEvent3D::Wheel { delta_y }, cx)
    }
}

impl Focusable for RuvizPlot3D {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for RuvizPlot3D {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let plot_canvas = canvas::<Option<Arc<RenderImage>>>(
            {
                let entity = entity.clone();
                move |bounds, window, cx| {
                    entity.update(cx, |view, _| {
                        view.prepaint(bounds, window).unwrap_or_else(|error| {
                            eprintln!("ruviz-gpui 3d render failed: {error}");
                            None
                        })
                    })
                }
            },
            move |bounds, image: Option<Arc<RenderImage>>, window, _cx| {
                if let Some(image) = image {
                    let _ = window.paint_image(bounds, Corners::default(), image, 0, false);
                }
            },
        )
        .size_full();

        div()
            .size_full()
            .track_focus(&self.focus_handle)
            .child(plot_canvas)
            .on_mouse_down(MouseButton::Left, pointer_down_handler(entity.clone()))
            .on_mouse_down(MouseButton::Middle, pointer_down_handler(entity.clone()))
            .on_mouse_down(MouseButton::Right, pointer_down_handler(entity.clone()))
            .on_mouse_move({
                let entity = entity.clone();
                move |event, _, cx| {
                    entity.update(cx, |view, cx| {
                        if let Err(error) = view.pointer_move(event, cx) {
                            eprintln!("ruviz-gpui 3d pointer move failed: {error}");
                        }
                    });
                }
            })
            .on_mouse_up(MouseButton::Left, pointer_up_handler(entity.clone()))
            .on_mouse_up(MouseButton::Middle, pointer_up_handler(entity.clone()))
            .on_mouse_up(MouseButton::Right, pointer_up_handler(entity.clone()))
            .on_scroll_wheel({
                let entity = entity.clone();
                move |event, _, cx| {
                    entity.update(cx, |view, cx| {
                        if let Err(error) = view.scroll(event, cx) {
                            eprintln!("ruviz-gpui 3d scroll failed: {error}");
                        }
                    });
                }
            })
            .on_key_down({
                let entity = entity.clone();
                move |event, _, cx| {
                    if event.keystroke.key.as_str() == "escape" {
                        entity.update(cx, |view, cx| {
                            if let Err(error) = view.apply(InputEvent3D::Escape, cx) {
                                eprintln!("ruviz-gpui 3d reset failed: {error}");
                            }
                        });
                    }
                }
            })
    }
}

fn pointer_down_handler(
    entity: Entity<RuvizPlot3D>,
) -> impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static {
    move |event, _, cx| {
        entity.update(cx, |view, cx| {
            if let Err(error) = view.pointer_down(event, cx) {
                eprintln!("ruviz-gpui 3d pointer down failed: {error}");
            }
        });
    }
}

fn pointer_up_handler(
    entity: Entity<RuvizPlot3D>,
) -> impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static {
    move |event, _, cx| {
        entity.update(cx, |view, cx| {
            if let Err(error) = view.pointer_up(event, cx) {
                eprintln!("ruviz-gpui 3d pointer up failed: {error}");
            }
        });
    }
}

/// Build a GPUI entity from a retained 3d session.
pub fn plot3d<V>(session: InteractivePlot3DSession, cx: &mut Context<V>) -> Entity<RuvizPlot3D>
where
    V: 'static,
{
    cx.new(|cx| RuvizPlot3D::new(session, cx))
}
