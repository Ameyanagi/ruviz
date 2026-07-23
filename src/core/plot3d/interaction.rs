use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use glam::Vec3;

use crate::core::{Image, PlottingError, Result};
use crate::render::three_d::overlay::compose_image;
use crate::render::three_d::scene::Scene3D;
use crate::render::three_d::software::raster::{SoftwareRenderOptions3D, render_scene};

#[cfg(feature = "gpu")]
use crate::render::three_d::gpu::Wgpu3DRenderer;

use super::Camera3D;
#[cfg(feature = "gpu")]
use super::RenderDiagnostics3D;
use super::builder::Plot3D;
use super::layout::Axis3Layout;
use super::picking::{Bvh3D, PickHit3D, pick_scene};
use super::prepared::PreparedSceneCache3D;
use super::resolve::ResolvedFrame3D;

const DRAG_THRESHOLD_PX: f32 = 3.0;
const ORBIT_DEGREES_PER_PIXEL: f32 = 0.25;
const MIN_ZOOM: f32 = 0.02;
const MAX_ZOOM: f32 = 100.0;
const WHEEL_EXPONENT_PER_PIXEL: f32 = 0.0015;
static NEXT_SCENE_GENERATION: AtomicU64 = AtomicU64::new(1);

/// Pointer button understood by the backend-neutral 3d controller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButton3D {
    Left,
    Middle,
    Right,
}

/// Small adapter-neutral input vocabulary for a 3d session.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputEvent3D {
    PointerDown {
        x: f32,
        y: f32,
        button: PointerButton3D,
    },
    PointerMove {
        x: f32,
        y: f32,
    },
    PointerUp {
        x: f32,
        y: f32,
        button: PointerButton3D,
    },
    /// Positive deltas zoom in and negative deltas zoom out.
    Wheel {
        delta_y: f32,
    },
    DoubleClick {
        x: f32,
        y: f32,
        button: PointerButton3D,
    },
    Escape,
}

/// Result of applying one adapter event to a 3d session.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InteractionResult3D {
    pub camera_changed: bool,
    pub request_redraw: bool,
    pub picked: Option<PickHit3D>,
}

/// Portable snapshot of the authoritative 3d camera.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraSnapshot3D {
    pub camera: Camera3D,
    pub scene_generation: u64,
    pub camera_generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ActiveDrag3D {
    button: PointerButton3D,
    anchor: (f32, f32),
    last: (f32, f32),
    crossed_threshold: bool,
}

/// Retained backend-neutral state for orbit, pan, zoom, reset, and picking.
///
/// Frontends translate their native events into [`InputEvent3D`]. The camera
/// lives only here, so native and web adapters cannot drift from each other.
pub struct InteractivePlot3DSession {
    frame: ResolvedFrame3D,
    scene: Arc<Scene3D>,
    bvh: Option<Arc<Bvh3D>>,
    initial_camera: Camera3D,
    scene_generation: u64,
    camera_generation: u64,
    active_drag: Option<ActiveDrag3D>,
    #[cfg(feature = "gpu")]
    gpu_renderer: Option<Wgpu3DRenderer>,
}

impl InteractivePlot3DSession {
    pub(super) fn new(plot: Plot3D) -> Result<Self> {
        let frame = plot.resolve()?;
        let (scene, _) = PreparedSceneCache3D::default().prepare(&frame)?;
        let scene_generation = NEXT_SCENE_GENERATION
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| {
                PlottingError::RenderError("3D scene generation space was exhausted".to_string())
            })?;
        Ok(Self {
            initial_camera: frame.camera,
            frame,
            scene,
            bvh: None,
            scene_generation,
            camera_generation: 0,
            active_drag: None,
            #[cfg(feature = "gpu")]
            gpu_renderer: None,
        })
    }

    /// Current authoritative camera.
    pub const fn camera(&self) -> Camera3D {
        self.frame.camera
    }

    /// Current camera and generation counters.
    pub const fn camera_snapshot(&self) -> CameraSnapshot3D {
        CameraSnapshot3D {
            camera: self.frame.camera,
            scene_generation: self.scene_generation,
            camera_generation: self.camera_generation,
        }
    }

    /// Replace the current camera without rebuilding scene geometry.
    pub fn set_camera(&mut self, camera: Camera3D) -> Result<()> {
        camera.validate()?;
        if camera != self.frame.camera {
            self.frame.camera = camera;
            self.advance_camera_generation()?;
        }
        Ok(())
    }

    /// Restore a saved camera. Snapshots may be reused for keep-view behavior.
    pub fn restore_camera(&mut self, snapshot: CameraSnapshot3D) -> Result<()> {
        self.set_camera(snapshot.camera)
    }

    /// Orbit by a screen-space drag delta in pixels.
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) -> Result<()> {
        validate_delta(delta_x, delta_y)?;
        let camera = self
            .frame
            .camera
            .azimuth_deg(self.frame.camera.get_azimuth_deg() + delta_x * ORBIT_DEGREES_PER_PIXEL)
            .elevation_deg(
                (self.frame.camera.get_elevation_deg() - delta_y * ORBIT_DEGREES_PER_PIXEL)
                    .clamp(-89.9, 89.9),
            );
        self.set_camera(camera)
    }

    /// Pan by a screen-space drag delta in pixels.
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) -> Result<()> {
        validate_delta(delta_x, delta_y)?;
        if delta_x == 0.0 && delta_y == 0.0 {
            return Ok(());
        }
        let layout = Axis3Layout::resolve(&self.frame)?;
        let target = self
            .frame
            .camera
            .target()
            .unwrap_or_else(|| self.frame.bounds.center());
        let target_local = self.frame.bounds.normalize(target, Vec3::ONE);
        let projected = layout.project_local(target_local)?;
        let moved_local = layout.unproject_local_at_depth(
            projected.x - delta_x,
            projected.y - delta_y,
            projected.depth,
        )?;
        let moved_target = self.frame.bounds.denormalize(moved_local, Vec3::ONE);
        self.set_camera(self.frame.camera.look_at(moved_target))
    }

    /// Multiply the current zoom by a positive factor.
    pub fn zoom_by(&mut self, factor: f32) -> Result<()> {
        if !factor.is_finite() || factor <= 0.0 {
            return Err(PlottingError::InvalidInput(format!(
                "3D zoom factor must be finite and greater than zero, got {factor}"
            )));
        }
        let zoom = (self.frame.camera.get_zoom() * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        self.set_camera(self.frame.camera.zoom(zoom))
    }

    /// Restore the camera supplied by the original builder.
    pub fn reset_view(&mut self) -> Result<()> {
        self.set_camera(self.initial_camera)
    }

    /// Apply one frontend event.
    pub fn handle_input(&mut self, event: InputEvent3D) -> Result<InteractionResult3D> {
        match event {
            InputEvent3D::PointerDown { x, y, button } => {
                validate_position(x, y)?;
                self.active_drag = Some(ActiveDrag3D {
                    button,
                    anchor: (x, y),
                    last: (x, y),
                    crossed_threshold: false,
                });
                Ok(InteractionResult3D::default())
            }
            InputEvent3D::PointerMove { x, y } => {
                validate_position(x, y)?;
                let Some(mut drag) = self.active_drag else {
                    return Ok(InteractionResult3D::default());
                };
                let total_x = x - drag.anchor.0;
                let total_y = y - drag.anchor.1;
                drag.crossed_threshold |= total_x.hypot(total_y) >= DRAG_THRESHOLD_PX;
                let delta_x = x - drag.last.0;
                let delta_y = y - drag.last.1;
                drag.last = (x, y);
                self.active_drag = Some(drag);
                if !drag.crossed_threshold || (delta_x == 0.0 && delta_y == 0.0) {
                    return Ok(InteractionResult3D::default());
                }
                match drag.button {
                    PointerButton3D::Left => self.orbit(delta_x, delta_y)?,
                    PointerButton3D::Middle | PointerButton3D::Right => {
                        self.pan(delta_x, delta_y)?
                    }
                }
                Ok(InteractionResult3D {
                    camera_changed: true,
                    request_redraw: true,
                    picked: None,
                })
            }
            InputEvent3D::PointerUp { x, y, button } => {
                validate_position(x, y)?;
                let drag = self.active_drag.take();
                let is_click = drag.is_some_and(|drag| {
                    drag.button == button
                        && !drag.crossed_threshold
                        && (x - drag.anchor.0).hypot(y - drag.anchor.1) < DRAG_THRESHOLD_PX
                });
                let picked = if is_click && button == PointerButton3D::Left {
                    self.pick(x, y)?
                } else {
                    None
                };
                Ok(InteractionResult3D {
                    camera_changed: false,
                    request_redraw: picked.is_some(),
                    picked,
                })
            }
            InputEvent3D::Wheel { delta_y } => {
                if !delta_y.is_finite() {
                    return Err(PlottingError::InvalidInput(
                        "3D wheel delta must be finite".to_string(),
                    ));
                }
                let generation = self.camera_generation;
                self.zoom_by((delta_y * WHEEL_EXPONENT_PER_PIXEL).exp())?;
                let changed = self.camera_generation != generation;
                Ok(InteractionResult3D {
                    camera_changed: changed,
                    request_redraw: changed,
                    picked: None,
                })
            }
            InputEvent3D::DoubleClick { x, y, button } => {
                validate_position(x, y)?;
                if button == PointerButton3D::Left {
                    let generation = self.camera_generation;
                    self.reset_view()?;
                    let changed = self.camera_generation != generation;
                    Ok(InteractionResult3D {
                        camera_changed: changed,
                        request_redraw: changed,
                        picked: None,
                    })
                } else {
                    Ok(InteractionResult3D::default())
                }
            }
            InputEvent3D::Escape => {
                let generation = self.camera_generation;
                self.reset_view()?;
                let changed = self.camera_generation != generation;
                Ok(InteractionResult3D {
                    camera_changed: changed,
                    request_redraw: changed,
                    picked: None,
                })
            }
        }
    }

    /// Pick the nearest visible point, line segment, or surface triangle.
    pub fn pick(&mut self, x: f32, y: f32) -> Result<Option<PickHit3D>> {
        let layout = Axis3Layout::resolve(&self.frame)?;
        if self.bvh.is_none() {
            self.bvh = Some(Arc::new(Bvh3D::build(&self.scene.geometry)?));
        }
        let bvh = self
            .bvh
            .as_ref()
            .ok_or_else(|| PlottingError::InvalidTopology3D {
                reason: "3D pick BVH was not retained".to_string(),
            })?;
        pick_scene(
            &self.frame,
            &layout,
            &self.scene,
            bvh,
            x,
            y,
            self.scene_generation,
            self.camera_generation,
        )
    }

    /// Whether a previously returned pick still matches this scene and camera.
    pub const fn is_pick_current(&self, hit: &PickHit3D) -> bool {
        hit.scene_generation == self.scene_generation
            && hit.camera_generation == self.camera_generation
    }

    /// Render one retained interactive-quality CPU frame.
    pub fn render(&self) -> Result<Image> {
        let layout = Axis3Layout::resolve(&self.frame)?;
        let output = render_scene(
            &self.scene,
            &layout,
            self.frame.figure.dpi,
            SoftwareRenderOptions3D::interactive(),
        )?;
        compose_image(&layout, &self.frame.figure, &self.frame.theme, output.layer)
    }

    /// Render one retained GPU frame and read it back for image presentation.
    ///
    /// This is a diagnosed correctness fallback for CPU-image frontends, not
    /// direct presentation.
    #[cfg(feature = "gpu")]
    pub fn render_gpu_readback(&mut self) -> Result<(Image, RenderDiagnostics3D)> {
        let layout = Axis3Layout::resolve(&self.frame)?;
        let renderer = match &mut self.gpu_renderer {
            Some(renderer) => renderer,
            None => self.gpu_renderer.insert(Wgpu3DRenderer::new()?),
        };
        let output = renderer.render_to_image(&self.scene, &layout, self.frame.figure.dpi)?;
        let image = compose_image(&layout, &self.frame.figure, &self.frame.theme, output.layer)?;
        let mut diagnostics = RenderDiagnostics3D {
            points_submitted: self.scene.point_count() as u64,
            triangles_submitted: self.scene.triangle_count() as u64,
            actual_backend: "gpu3d-readback-fallback".to_string(),
            adapter_name: Some(output.adapter_name),
            sample_count: output.sample_count,
            fallback_reason: Some(
                "frontend requires a CPU image; direct presentation was not used".to_string(),
            ),
            ..RenderDiagnostics3D::default()
        };
        diagnostics.vertex_upload_bytes = output.resource_update.vertex_upload_bytes;
        diagnostics.index_upload_bytes = output.resource_update.index_upload_bytes;
        diagnostics.buffer_creations = output.resource_update.buffer_creations;
        diagnostics.camera_uniform_writes = output.camera_uniform_writes;
        diagnostics.draw_calls = output.draw_calls;
        diagnostics.readback_bytes = output.readback_bytes;
        Ok((image, diagnostics))
    }

    fn advance_camera_generation(&mut self) -> Result<()> {
        self.camera_generation = self.camera_generation.checked_add(1).ok_or_else(|| {
            PlottingError::RenderError("3D camera generation space was exhausted".to_string())
        })?;
        Ok(())
    }
}

fn validate_delta(delta_x: f32, delta_y: f32) -> Result<()> {
    if delta_x.is_finite() && delta_y.is_finite() {
        Ok(())
    } else {
        Err(PlottingError::InvalidInput(
            "3D interaction deltas must be finite".to_string(),
        ))
    }
}

fn validate_position(x: f32, y: f32) -> Result<()> {
    if x.is_finite() && y.is_finite() {
        Ok(())
    } else {
        Err(PlottingError::InvalidInput(
            "3D pointer coordinates must be finite".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::{line3d, scatter3d, surface};

    use super::*;

    #[test]
    fn orbit_pan_zoom_reset_use_one_authoritative_camera() {
        let mut session = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .interactive_session()
            .expect("session");
        let initial = session.camera_snapshot();
        session.orbit(20.0, -8.0).expect("orbit");
        assert_ne!(session.camera(), initial.camera);
        let after_orbit = session.camera_generation;
        session.pan(12.0, -5.0).expect("pan");
        assert!(session.camera().target().is_some());
        session.zoom_by(1.25).expect("zoom");
        assert!(session.camera_generation > after_orbit);
        session.reset_view().expect("reset");
        assert_eq!(session.camera(), initial.camera);
        assert_eq!(session.scene_generation, initial.scene_generation);
    }

    #[test]
    fn event_drag_threshold_separates_orbit_from_click_pick() {
        let mut session = surface(
            &[-1.0, 0.0, 1.0],
            &[-1.0, 0.0, 1.0],
            &[[0.0, 0.0, 0.0], [0.0, 0.5, 0.0], [0.0, 0.0, 0.0]],
        )
        .interactive_session()
        .expect("session");
        let layout = Axis3Layout::resolve(&session.frame).expect("layout");
        let center = layout.project_local(Vec3::ZERO).expect("center");
        session
            .handle_input(InputEvent3D::PointerDown {
                x: center.x,
                y: center.y,
                button: PointerButton3D::Left,
            })
            .expect("down");
        let click = session
            .handle_input(InputEvent3D::PointerUp {
                x: center.x + 1.0,
                y: center.y,
                button: PointerButton3D::Left,
            })
            .expect("up");
        assert!(click.picked.is_some());

        session
            .handle_input(InputEvent3D::PointerDown {
                x: center.x,
                y: center.y,
                button: PointerButton3D::Left,
            })
            .expect("down");
        let drag = session
            .handle_input(InputEvent3D::PointerMove {
                x: center.x + 12.0,
                y: center.y + 4.0,
            })
            .expect("move");
        assert!(drag.camera_changed);
        let up = session
            .handle_input(InputEvent3D::PointerUp {
                x: center.x + 12.0,
                y: center.y + 4.0,
                button: PointerButton3D::Left,
            })
            .expect("up");
        assert!(up.picked.is_none());
    }

    #[test]
    fn point_line_and_surface_picks_carry_current_generations() {
        let mut point = scatter3d(&[0.0], &[0.0], &[0.0])
            .interactive_session()
            .expect("point session");
        let layout = Axis3Layout::resolve(&point.frame).expect("layout");
        let center = layout.project_local(Vec3::ZERO).expect("center");
        let hit = point
            .pick(center.x, center.y)
            .expect("point pick")
            .expect("point hit");
        assert_eq!(hit.primitive, super::super::PickPrimitive3D::Point);
        assert_eq!(hit.sources(), &[0]);
        assert!(point.is_pick_current(&hit));
        point.orbit(4.0, 0.0).expect("orbit");
        assert!(!point.is_pick_current(&hit));

        let mut line = line3d(&[-1.0, 1.0], &[0.0, 0.0], &[0.0, 0.0])
            .interactive_session()
            .expect("line session");
        let layout = Axis3Layout::resolve(&line.frame).expect("layout");
        let center = layout.project_local(Vec3::ZERO).expect("center");
        let hit = line
            .pick(center.x, center.y)
            .expect("line pick")
            .expect("line hit");
        assert_eq!(hit.primitive, super::super::PickPrimitive3D::LineSegment);
        assert_eq!(hit.sources(), &[0, 1]);
    }

    #[test]
    fn cpu_interactive_render_reuses_the_compiled_scene() {
        let session = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .interactive_session()
            .expect("session");
        let first = session.render().expect("first");
        let second = session.render().expect("second");
        assert_eq!((first.width, first.height), (second.width, second.height));
        assert_eq!(first.pixels, second.pixels);
    }

    #[test]
    fn replacement_session_keeps_view_but_gets_a_unique_scene_generation() {
        let mut first = scatter3d(&[0.0], &[0.0], &[0.0])
            .interactive_session()
            .expect("first");
        first.orbit(15.0, -4.0).expect("orbit");
        let snapshot = first.camera_snapshot();
        let second = scatter3d(&[1.0], &[2.0], &[3.0])
            .interactive_session_with_view(snapshot)
            .expect("second");
        assert_eq!(second.camera(), snapshot.camera);
        assert_ne!(
            second.camera_snapshot().scene_generation,
            snapshot.scene_generation
        );
    }
}
