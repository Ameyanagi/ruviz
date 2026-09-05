use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use glam::Vec3;

use crate::core::{Bounds3D, FigureConfig, Image, PlottingError, Point3D, Result};
use crate::render::Theme;
use crate::render::three_d::overlay::compose_image;
use crate::render::three_d::scene::{Scene3D, SceneGeometry3D};
use crate::render::three_d::software::raster::{SoftwareRenderOptions3D, render_scene};

#[cfg(feature = "gpu")]
use crate::render::three_d::gpu::{
    GpuContext3D, PresentationCompositor3D, Wgpu3DRenderer, select_surface_format,
};
#[cfg(all(feature = "interactive-gpu", not(target_arch = "wasm32")))]
use crate::render::three_d::gpu::{SurfacePresentOutcome3D, SurfacePresenter3D};

use super::RenderDiagnostics3D;
use super::builder::Plot3D;
use super::layout::Axis3Layout;
use super::picking::{Bvh3D, PickHit3D, pick_scene};
use super::prepared::PreparedSceneCache3D;
use super::resolve::{CacheKey3D, FrameKeys3D, ResolvedFrame3D};
use super::{Camera3D, CameraView3D};

const DRAG_THRESHOLD_PX: f32 = 3.0;
const ORBIT_DEGREES_PER_PIXEL: f32 = 0.25;
const MIN_ZOOM: f32 = 0.02;
const MAX_ZOOM: f32 = 100.0;
const WHEEL_EXPONENT_PER_PIXEL: f32 = 0.0015;
const MAX_WHEEL_DELTA_PER_EVENT_PX: f32 = 120.0;
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

/// Opaque identity of one resolved 3d view.
///
/// Scene, camera, and render-target changes advance independently. Adapter
/// code can compare the whole value without coupling itself to generation
/// counters or assuming that a resize changed the camera.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ViewStamp3D {
    scene_generation: u64,
    camera_generation: u64,
    target_generation: u64,
}

impl ViewStamp3D {
    /// Whether both stamps refer to the same compiled scene.
    pub const fn same_scene(self, other: Self) -> bool {
        self.scene_generation == other.scene_generation
    }

    /// Whether both stamps refer to the same camera.
    pub const fn same_camera(self, other: Self) -> bool {
        self.camera_generation == other.camera_generation
    }

    /// Whether both stamps refer to the same physical render target.
    pub const fn same_target(self, other: Self) -> bool {
        self.target_generation == other.target_generation
    }
}

/// Opaque identity of one background image-render request.
///
/// In addition to the view identity, this contains a monotonically increasing
/// request identity. Requesting the same unchanged view twice therefore makes
/// the first request superseded, which gives adapters deterministic
/// latest-request-wins behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RenderStamp3D {
    view: ViewStamp3D,
    request_generation: u64,
}

impl RenderStamp3D {
    /// View that this render request captured.
    pub const fn view(self) -> ViewStamp3D {
        self.view
    }
}

/// A pick paired with the exact view that produced it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StampedPick3D {
    /// Pick result produced by the stamped view.
    pub hit: PickHit3D,
    /// Complete scene, camera, and target identity used for the pick.
    pub view: ViewStamp3D,
}

/// Completed image from a [`BackgroundRenderJob3D`].
#[derive(Clone, Debug)]
pub struct RenderedImage3D {
    /// Completed straight-alpha RGBA frame.
    pub image: Image,
    /// Request and view identity captured by the render job.
    pub stamp: RenderStamp3D,
}

/// Latest-wins classification of one completed background render.
#[derive(Clone, Debug)]
pub enum BackgroundRenderOutcome3D {
    /// The completed image is still the latest requested view.
    Current(RenderedImage3D),
    /// The completed image was superseded and must not be installed.
    Superseded {
        /// Identity of the completed obsolete render.
        rendered: RenderStamp3D,
        /// Identity representing the session's current requested view.
        current: RenderStamp3D,
    },
}

/// Owned, native-thread-safe snapshot for one image render.
///
/// Construct this on the UI thread with
/// [`InteractivePlot3DSession::background_render_job`], then move it to a
/// worker. It retains the already compiled scene and never borrows session or
/// frontend state.
#[derive(Clone)]
pub struct BackgroundRenderJob3D {
    frame: Arc<ResolvedFrame3D>,
    scene: Arc<Scene3D>,
    stamp: RenderStamp3D,
}

impl BackgroundRenderJob3D {
    /// Identity used to reject a superseded result.
    pub const fn stamp(&self) -> RenderStamp3D {
        self.stamp
    }

    /// Render this immutable snapshot to an image.
    pub fn render(self) -> Result<RenderedImage3D> {
        BackgroundRenderer3D::default().render(self)
    }
}

/// Worker-owned backend used to execute immutable 3d image jobs.
///
/// CPU rendering is the default. With native `gpu` support, selecting
/// [`BackgroundRenderBackend3D::GpuReadback`] retains one wgpu renderer across
/// jobs on the worker. GPU output is explicitly read back and composed into a
/// CPU image; this API does not claim or provide zero-copy presentation.
pub struct BackgroundRenderer3D {
    backend: BackgroundRenderBackend3D,
    #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
    gpu_renderer: Option<Wgpu3DRenderer>,
}

/// Backend preference for worker-owned 3d image rendering.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BackgroundRenderBackend3D {
    /// Software rasterization and CPU image composition.
    #[default]
    Cpu,
    /// Retained GPU rendering followed by readback into a CPU image.
    #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
    GpuReadback,
}

impl Default for BackgroundRenderer3D {
    fn default() -> Self {
        Self::new(BackgroundRenderBackend3D::Cpu)
    }
}

impl BackgroundRenderer3D {
    /// Create a worker renderer with the requested backend preference.
    ///
    /// GPU initialization is lazy and therefore occurs on the worker's first
    /// [`Self::render`] call rather than in the UI callback that queues a job.
    pub const fn new(backend: BackgroundRenderBackend3D) -> Self {
        Self {
            backend,
            #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
            gpu_renderer: None,
        }
    }

    /// Configured image-rendering backend.
    pub const fn backend(&self) -> BackgroundRenderBackend3D {
        self.backend
    }

    /// Execute one immutable job, retaining backend resources for later jobs.
    pub fn render(&mut self, job: BackgroundRenderJob3D) -> Result<RenderedImage3D> {
        self.render_with_diagnostics(job)
            .map(|(rendered, _)| rendered)
    }

    /// Execute one job and report the backend/readback behavior used.
    pub fn render_with_diagnostics(
        &mut self,
        job: BackgroundRenderJob3D,
    ) -> Result<(RenderedImage3D, RenderDiagnostics3D)> {
        match self.backend {
            BackgroundRenderBackend3D::Cpu => render_background_cpu(job),
            #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
            BackgroundRenderBackend3D::GpuReadback => {
                let renderer = match &mut self.gpu_renderer {
                    Some(renderer) => renderer,
                    None => self.gpu_renderer.insert(Wgpu3DRenderer::new()?),
                };
                render_background_gpu_readback(job, renderer)
            }
        }
    }
}

fn render_background_cpu(
    job: BackgroundRenderJob3D,
) -> Result<(RenderedImage3D, RenderDiagnostics3D)> {
    let layout = Axis3Layout::resolve(&job.frame)?;
    let output = render_scene(
        &job.scene,
        &layout,
        job.frame.figure.dpi,
        SoftwareRenderOptions3D::interactive(),
    )?;
    let image = compose_image(&layout, &job.frame.figure, &job.frame.theme, &output.layer)?;
    let diagnostics = RenderDiagnostics3D {
        draw_calls: output.draw_calls,
        points_submitted: job.scene.point_count() as u64,
        triangles_submitted: job.scene.triangle_count() as u64,
        primitives_culled: output.primitives_culled,
        actual_backend: "cpu3d-background".to_string(),
        ..RenderDiagnostics3D::default()
    };
    Ok((
        RenderedImage3D {
            image,
            stamp: job.stamp,
        },
        diagnostics,
    ))
}

#[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
fn render_background_gpu_readback(
    job: BackgroundRenderJob3D,
    renderer: &mut Wgpu3DRenderer,
) -> Result<(RenderedImage3D, RenderDiagnostics3D)> {
    let layout = Axis3Layout::resolve(&job.frame)?;
    let output = renderer.render_to_image(&job.scene, &layout, job.frame.figure.dpi)?;
    let image = compose_image(&layout, &job.frame.figure, &job.frame.theme, &output.layer)?;
    let mut diagnostics = RenderDiagnostics3D {
        points_submitted: job.scene.point_count() as u64,
        triangles_submitted: job.scene.triangle_count() as u64,
        actual_backend: "gpu3d-background-readback".to_string(),
        adapter_name: Some(output.adapter_name),
        sample_count: output.sample_count,
        fallback_reason: Some(
            "background image presentation requires GPU readback; zero-copy was not used"
                .to_string(),
        ),
        ..RenderDiagnostics3D::default()
    };
    diagnostics.vertex_upload_bytes = output.resource_update.vertex_upload_bytes;
    diagnostics.index_upload_bytes = output.resource_update.index_upload_bytes;
    diagnostics.texture_upload_bytes = output.resource_update.texture_upload_bytes;
    diagnostics.buffer_creations = output.resource_update.buffer_creations;
    diagnostics.buffer_evictions = output.resource_update.evictions;
    diagnostics.camera_uniform_writes = output.camera_uniform_writes;
    diagnostics.draw_calls = output.draw_calls;
    diagnostics.readback_bytes = output.readback_bytes;
    diagnostics.queue_waits = 1;
    Ok((
        RenderedImage3D {
            image,
            stamp: job.stamp,
        },
        diagnostics,
    ))
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
    // `ResolvedFrame3D` owns only small plot metadata plus Arc-backed series
    // data. Retaining it behind an Arc makes render-job construction strictly
    // O(1): interaction mutations use `Arc::make_mut`, whose shallow clone
    // retains the underlying datasets instead of copying their values.
    frame: Arc<ResolvedFrame3D>,
    scene: Arc<Scene3D>,
    bvh: Arc<Bvh3D>,
    initial_camera: Camera3D,
    scene_generation: u64,
    camera_generation: u64,
    target_generation: u64,
    request_generation: u64,
    active_drag: Option<ActiveDrag3D>,
    current_pick: Option<StampedPick3D>,
    #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
    gpu_renderer: Option<Wgpu3DRenderer>,
}

impl InteractivePlot3DSession {
    pub(super) fn new(plot: Plot3D) -> Result<Self> {
        let frame = plot.resolve()?;
        let (scene, _) = PreparedSceneCache3D::default().prepare(&frame)?;
        // Picking runs from GUI input callbacks, so all topology acceleration
        // must be built during this already-fallible construction phase.
        // Retaining it here guarantees the first click performs only layout
        // and traversal work rather than an unbounded scene-wide BVH build.
        let bvh = Arc::new(Bvh3D::build(&scene.geometry)?);
        let frame = Arc::new(frame);
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
            bvh,
            scene_generation,
            camera_generation: 0,
            target_generation: 0,
            request_generation: 0,
            active_drag: None,
            current_pick: None,
            #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
            gpu_renderer: None,
        })
    }

    /// Construct a non-rendering placeholder for adapters whose infallible
    /// compatibility builder could not resolve the supplied plot.
    ///
    /// Adapters must retain and report the original construction error and
    /// must not schedule render jobs for this placeholder. The reserved zero
    /// scene generation keeps it distinct from normally constructed sessions.
    #[doc(hidden)]
    pub fn error_placeholder() -> Self {
        let camera = Camera3D::default();
        let frame = Arc::new(ResolvedFrame3D {
            series: Vec::new(),
            bounds: Bounds3D {
                min: Point3D::new(-1.0, -1.0, -1.0),
                max: Point3D::new(1.0, 1.0, 1.0),
            },
            camera,
            figure: FigureConfig::default(),
            theme: Theme::default(),
            title: None,
            xlabel: None,
            ylabel: None,
            zlabel: None,
            legend: None,
            axes: true,
            keys: FrameKeys3D {
                geometry: CacheKey3D(0),
                appearance: CacheKey3D(0),
                layout: CacheKey3D(0),
                view: CacheKey3D(0),
            },
        });
        let geometry = Arc::new(SceneGeometry3D::default());
        Self {
            frame,
            scene: Arc::new(Scene3D {
                geometry,
                spheres: Vec::new(),
                points: Vec::new(),
                lines: Vec::new(),
                meshes: Vec::new(),
            }),
            bvh: Arc::new(Bvh3D::default()),
            initial_camera: camera,
            scene_generation: 0,
            camera_generation: 0,
            target_generation: 0,
            request_generation: 0,
            active_drag: None,
            current_pick: None,
            #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
            gpu_renderer: None,
        }
    }

    /// Current authoritative camera.
    pub fn camera(&self) -> Camera3D {
        self.frame.camera
    }

    /// Current camera and generation counters.
    pub fn camera_snapshot(&self) -> CameraSnapshot3D {
        CameraSnapshot3D {
            camera: self.frame.camera,
            scene_generation: self.scene_generation,
            camera_generation: self.camera_generation,
        }
    }

    /// Identity of the current scene, camera, and physical render target.
    pub fn view_stamp(&self) -> ViewStamp3D {
        ViewStamp3D {
            scene_generation: self.scene_generation,
            camera_generation: self.camera_generation,
            target_generation: self.target_generation,
        }
    }

    /// Whether a view identity still matches all authoritative session state.
    pub fn is_view_current(&self, stamp: ViewStamp3D) -> bool {
        self.view_stamp().scene_generation == stamp.scene_generation
            && self.view_stamp().camera_generation == stamp.camera_generation
            && self.view_stamp().target_generation == stamp.target_generation
    }

    /// Toggle lighting for every sphere series without changing geometry, camera,
    /// drag state, or the selected atom. Returns whether anything changed.
    pub fn set_sphere_shading(&mut self, enabled: bool) -> Result<bool> {
        if !self
            .scene
            .spheres
            .iter()
            .any(|batch| batch.style.shaded != enabled)
        {
            return Ok(false);
        }
        let generation = NEXT_SCENE_GENERATION
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| {
                PlottingError::RenderError("3D scene generation space was exhausted".into())
            })?;
        for series in &mut Arc::make_mut(&mut self.frame).series {
            if let super::builder::Series3D::Spheres { style, .. } = series {
                style.shaded = enabled;
            }
        }
        for batch in &mut Arc::make_mut(&mut self.scene).spheres {
            batch.style.shaded = enabled;
        }
        self.scene_generation = generation;
        let stamp = self.view_stamp();
        if let Some(pick) = &mut self.current_pick {
            pick.hit.scene_generation = generation;
            pick.view = stamp;
        }
        Ok(true)
    }

    /// Replace the current camera without rebuilding scene geometry.
    pub fn set_camera(&mut self, camera: Camera3D) -> Result<()> {
        camera.validate()?;
        if camera != self.frame.camera {
            let next_generation = self.camera_generation.checked_add(1).ok_or_else(|| {
                PlottingError::RenderError("3D camera generation space was exhausted".to_string())
            })?;
            Arc::make_mut(&mut self.frame).camera = camera;
            self.camera_generation = next_generation;
            self.current_pick = None;
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
        // `Camera3D::prepare` clamps the look-at target to the plotting box.
        // Store the clamped value too, so a long drag cannot accumulate a
        // target the view will never honour and that takes as many drags to
        // undo.
        let moved_target = clamp_to_bounds(moved_target, self.frame.bounds);
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

    /// Apply a named camera orientation without rebuilding scene geometry.
    ///
    /// Projection, axis aspect, zoom, and the current look-at target are
    /// preserved. Top and bottom use pole-safe elevations of `±89.9°`.
    pub fn apply_camera_view(&mut self, view: CameraView3D) -> Result<()> {
        self.set_camera(self.frame.camera.camera_view(view))
    }

    /// Recenter the current scene and restore unit zoom.
    ///
    /// Camera orientation, projection, and axis aspect are preserved.
    pub fn fit_to_content(&mut self) -> Result<()> {
        self.set_camera(self.frame.camera.fit_to_content(self.frame.bounds))
    }

    /// Resize the physical render target without rebuilding scene geometry.
    pub fn resize(&mut self, width_px: u32, height_px: u32, scale_factor: f32) -> Result<()> {
        if width_px == 0 || height_px == 0 {
            return Err(PlottingError::InvalidDimensions {
                width: width_px,
                height: height_px,
            });
        }
        if !scale_factor.is_finite() || scale_factor <= 0.0 {
            return Err(PlottingError::InvalidInput(format!(
                "3D window scale factor must be finite and greater than zero, got {scale_factor}"
            )));
        }
        let dpi = 72.0 * scale_factor;
        let current_size = self.frame.figure.canvas_size();
        let changed = current_size != (width_px, height_px)
            || self.frame.figure.dpi.to_bits() != dpi.to_bits();
        if changed {
            let next_generation = self.target_generation.checked_add(1).ok_or_else(|| {
                PlottingError::RenderError(
                    "3D render target generation space was exhausted".to_string(),
                )
            })?;
            let frame = Arc::make_mut(&mut self.frame);
            frame.figure.width = width_px as f32 / dpi;
            frame.figure.height = height_px as f32 / dpi;
            frame.figure.dpi = dpi;
            self.target_generation = next_generation;
            self.current_pick = None;
        }
        Ok(())
    }

    /// Current physical render-target size.
    pub fn size_px(&self) -> (u32, u32) {
        self.frame.figure.canvas_size()
    }

    /// Resolved plot title, if one was supplied.
    pub fn title(&self) -> Option<&str> {
        self.frame.title.as_deref()
    }

    /// Cancel an in-progress pointer drag, including release-outside drags.
    ///
    /// Returns `true` when an active drag was cleared. Frontends should call
    /// this on pointer-capture loss, focus loss, or pointer leave when they
    /// cannot guarantee delivery of the matching pointer-up event.
    pub fn cancel_drag(&mut self) -> bool {
        self.active_drag.take().is_some()
    }

    /// Whether a pointer drag is currently retained.
    pub const fn is_drag_active(&self) -> bool {
        self.active_drag.is_some()
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
                let generation = self.camera_generation;
                match drag.button {
                    PointerButton3D::Left => self.orbit(delta_x, delta_y)?,
                    PointerButton3D::Middle | PointerButton3D::Right => {
                        self.pan(delta_x, delta_y)?
                    }
                }
                let changed = self.camera_generation != generation;
                Ok(InteractionResult3D {
                    camera_changed: changed,
                    request_redraw: changed,
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
                let bounded_delta =
                    delta_y.clamp(-MAX_WHEEL_DELTA_PER_EVENT_PX, MAX_WHEEL_DELTA_PER_EVENT_PX);
                self.zoom_by((bounded_delta * WHEEL_EXPONENT_PER_PIXEL).exp())?;
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
                self.cancel_drag();
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
        let hit = pick_scene(
            &self.frame,
            &layout,
            &self.scene,
            &self.bvh,
            x,
            y,
            self.scene_generation,
            self.camera_generation,
        )?;
        self.current_pick = hit.map(|hit| StampedPick3D {
            hit,
            view: self.view_stamp(),
        });
        Ok(hit)
    }

    /// Current retained pick paired with its complete view identity.
    pub const fn current_pick(&self) -> Option<StampedPick3D> {
        self.current_pick
    }

    /// Clear the retained pick.
    ///
    /// Returns `true` when a pick was present.
    pub fn clear_pick(&mut self) -> bool {
        self.current_pick.take().is_some()
    }

    /// Whether a stamped pick still matches scene, camera, and target.
    pub fn is_stamped_pick_current(&self, pick: &StampedPick3D) -> bool {
        self.is_view_current(pick.view) && self.current_pick.is_some_and(|current| current == *pick)
    }

    /// Whether a previously returned pick still matches this scene and camera.
    ///
    /// New adapters should retain [`StampedPick3D`] from [`Self::current_pick`]
    /// and use [`Self::is_stamped_pick_current`] so target resizes are included.
    pub fn is_pick_current(&self, hit: &PickHit3D) -> bool {
        hit.scene_generation == self.scene_generation
            && hit.camera_generation == self.camera_generation
            && match self.current_pick {
                Some(current) => current.hit == *hit && self.is_view_current(current.view),
                None => false,
            }
    }

    /// Create an owned image render job for a background worker.
    ///
    /// Calling this again supersedes all previously created jobs, even when
    /// the view did not change.
    pub fn background_render_job(&mut self) -> Result<BackgroundRenderJob3D> {
        self.request_generation = self.request_generation.checked_add(1).ok_or_else(|| {
            PlottingError::RenderError("3D render request space was exhausted".to_string())
        })?;
        Ok(BackgroundRenderJob3D {
            frame: Arc::clone(&self.frame),
            scene: Arc::clone(&self.scene),
            stamp: RenderStamp3D {
                view: self.view_stamp(),
                request_generation: self.request_generation,
            },
        })
    }

    /// Whether a background render request is still the latest current view.
    pub fn is_render_current(&self, stamp: RenderStamp3D) -> bool {
        self.is_view_current(stamp.view) && stamp.request_generation == self.request_generation
    }

    /// Classify a completed background render without presenting stale pixels.
    pub fn classify_render(&self, rendered: RenderedImage3D) -> BackgroundRenderOutcome3D {
        if self.is_render_current(rendered.stamp) {
            BackgroundRenderOutcome3D::Current(rendered)
        } else {
            BackgroundRenderOutcome3D::Superseded {
                rendered: rendered.stamp,
                current: RenderStamp3D {
                    view: self.view_stamp(),
                    request_generation: self.request_generation,
                },
            }
        }
    }

    /// Replace the retained scene and reset to the replacement's own camera.
    ///
    /// Any retained GPU renderer is moved into the replacement session so
    /// adapters do not recreate a device for every plot replacement.
    pub fn replace(&mut self, replacement: Self) {
        // This compatibility API predates fallible replacement. If the
        // request counter is exhausted, retain the complete old session
        // instead of partially installing a replacement that cannot
        // invalidate its previously-created jobs.
        let _ = self.replace_inner(replacement);
    }

    /// Fallibly replace the retained scene and reset to its own camera.
    ///
    /// Unlike [`Self::replace`], this reports request-generation exhaustion.
    /// Failure is atomic: the current session and retained renderer are left
    /// unchanged.
    pub fn try_replace(&mut self, replacement: Self) -> Result<()> {
        self.replace_inner(replacement)
    }

    /// Replace the retained scene while keeping the current camera.
    pub fn replace_keep_camera(&mut self, mut replacement: Self) -> Result<()> {
        let camera = self.camera();
        camera.validate()?;
        if replacement.camera() != camera {
            let next_generation =
                replacement
                    .camera_generation
                    .checked_add(1)
                    .ok_or_else(|| {
                        PlottingError::RenderError(
                            "3D camera generation space was exhausted".to_string(),
                        )
                    })?;
            Arc::make_mut(&mut replacement.frame).camera = camera;
            replacement.camera_generation = next_generation;
            replacement.current_pick = None;
        }
        self.replace_inner(replacement)
    }

    /// Render one retained frame as PNG bytes.
    ///
    /// Stamps the frame's figure DPI into the `pHYs` chunk — the same stamp
    /// the non-interactive 3D `render_png_bytes` writes for the same figure,
    /// so a session export and a direct export cannot differ by a metadata
    /// chunk. The figure DPI already tracks the session's scale factor (see
    /// `resize`), so a scaled render stamps its denser resolution.
    pub fn render_png_bytes(&self) -> Result<Vec<u8>> {
        self.render()?.encode_png_with_dpi(self.frame.figure.dpi)
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
        compose_image(
            &layout,
            &self.frame.figure,
            &self.frame.theme,
            &output.layer,
        )
    }

    /// Render one retained GPU frame and read it back for image presentation.
    ///
    /// This is a diagnosed correctness fallback for CPU-image frontends, not
    /// direct presentation.
    #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
    pub fn render_gpu_readback(&mut self) -> Result<(Image, RenderDiagnostics3D)> {
        let layout = Axis3Layout::resolve(&self.frame)?;
        let renderer = match &mut self.gpu_renderer {
            Some(renderer) => renderer,
            None => self.gpu_renderer.insert(Wgpu3DRenderer::new()?),
        };
        let output = renderer.render_to_image(&self.scene, &layout, self.frame.figure.dpi)?;
        let image = compose_image(
            &layout,
            &self.frame.figure,
            &self.frame.theme,
            &output.layer,
        )?;
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
        diagnostics.texture_upload_bytes = output.resource_update.texture_upload_bytes;
        diagnostics.buffer_creations = output.resource_update.buffer_creations;
        diagnostics.buffer_evictions = output.resource_update.evictions;
        diagnostics.camera_uniform_writes = output.camera_uniform_writes;
        diagnostics.draw_calls = output.draw_calls;
        diagnostics.readback_bytes = output.readback_bytes;
        diagnostics.queue_waits = 1;
        Ok((image, diagnostics))
    }

    /// Present one frame directly to a native wgpu surface.
    ///
    /// `None` means the surface was temporarily unavailable or occluded. A
    /// presented frame always reports zero readback bytes; the retained text
    /// atlas reports zero texture uploads on camera-only warm frames.
    #[cfg(all(feature = "interactive-gpu", not(target_arch = "wasm32")))]
    pub(crate) fn present_direct(
        &mut self,
        presenter: &mut SurfacePresenter3D,
    ) -> Result<Option<RenderDiagnostics3D>> {
        let layout = Axis3Layout::resolve(&self.frame)?;
        let outcome =
            presenter.present(&self.scene, &layout, &self.frame.figure, &self.frame.theme)?;
        let SurfacePresentOutcome3D::Presented(output) = outcome else {
            return Ok(None);
        };
        let mut diagnostics = RenderDiagnostics3D {
            points_submitted: self.scene.point_count() as u64,
            triangles_submitted: self.scene.triangle_count() as u64,
            actual_backend: "gpu3d-surface".to_string(),
            adapter_name: Some(output.scene.adapter_name),
            sample_count: output.scene.sample_count,
            fallback_reason: None,
            readback_bytes: 0,
            presentation_vertex_upload_bytes: output.presentation.vertex_upload_bytes,
            presentation_texture_upload_bytes: output.presentation.texture_upload_bytes,
            surface_presents: 1,
            surface_reconfigurations: output.presentation.surface_reconfigurations,
            queue_waits: 0,
            ..RenderDiagnostics3D::default()
        };
        diagnostics.vertex_upload_bytes = output.scene.resource_update.vertex_upload_bytes;
        diagnostics.index_upload_bytes = output.scene.resource_update.index_upload_bytes;
        diagnostics.texture_upload_bytes = output.scene.resource_update.texture_upload_bytes;
        diagnostics.buffer_creations = output
            .scene
            .resource_update
            .buffer_creations
            .saturating_add(output.presentation.buffer_creations);
        diagnostics.camera_uniform_writes = output.scene.camera_uniform_writes;
        diagnostics.draw_calls = output
            .scene
            .draw_calls
            .saturating_add(output.presentation.draw_calls);
        Ok(Some(diagnostics))
    }

    fn replace_inner(&mut self, mut replacement: Self) -> Result<()> {
        // A replacement session may already have produced jobs, a selection,
        // or an in-progress drag before it is installed. None of that
        // frontend state is valid across the replacement boundary.
        replacement.request_generation =
            replacement
                .request_generation
                .checked_add(1)
                .ok_or_else(|| {
                    PlottingError::RenderError(
                        "3D render request space was exhausted during replacement".to_string(),
                    )
                })?;
        replacement.active_drag = None;
        replacement.current_pick = None;
        #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
        {
            replacement.gpu_renderer = self.gpu_renderer.take();
        }
        *self = replacement;
        Ok(())
    }
}

/// Result of one direct native or browser surface presentation attempt.
///
/// This integration enum is public only so platform adapter crates can own the
/// actual surface without exposing ruviz's retained scene internals.
#[cfg(feature = "gpu")]
#[doc(hidden)]
#[allow(
    clippy::large_enum_variant,
    reason = "boxing diagnostics would add a heap allocation to every presented frame"
)]
pub enum GpuSurfacePresentStatus3D {
    Presented(RenderDiagnostics3D),
    Skipped,
    RecreateSurface,
}

/// Cross-target retained 3d renderer for an adapter-owned wgpu surface.
///
/// Browser adapters construct this asynchronously after creating their canvas
/// surface. Interactive frames submit GPU work and present without readback,
/// blocking polls, or CPU framebuffer uploads.
#[cfg(feature = "gpu")]
#[doc(hidden)]
pub struct GpuSurfaceSession3D {
    session: InteractivePlot3DSession,
    renderer: Wgpu3DRenderer,
    compositor: PresentationCompositor3D,
    configuration: wgpu::SurfaceConfiguration,
    presentation_format: wgpu::TextureFormat,
    pending_surface_reconfigurations: u64,
}

#[cfg(feature = "gpu")]
impl GpuSurfaceSession3D {
    pub async fn new(
        session: InteractivePlot3DSession,
        instance: wgpu::Instance,
        surface: &wgpu::Surface<'_>,
    ) -> Result<Self> {
        let (width, height) = session.size_px();
        let context = GpuContext3D::from_instance_async(instance, Some(surface)).await?;
        let capabilities = surface.get_capabilities(context.adapter());
        let formats = select_surface_format(&capabilities.formats)?;
        let mut configuration = surface
            .get_default_config(context.adapter(), width.max(1), height.max(1))
            .ok_or_else(|| {
                PlottingError::UnsupportedGpuFeature(
                    "the selected adapter cannot present to this 3d canvas".to_string(),
                )
            })?;
        configuration.format = formats.surface;
        configuration.view_formats = if formats.view == formats.surface {
            Vec::new()
        } else {
            vec![formats.view]
        };
        configuration.present_mode = wgpu::PresentMode::AutoVsync;
        configuration.desired_maximum_frame_latency = 2;
        surface.configure(context.device(), &configuration);
        let renderer = Wgpu3DRenderer::from_context(context)?;
        let compositor = PresentationCompositor3D::new(renderer.context().device(), formats.view);
        Ok(Self {
            session,
            renderer,
            compositor,
            configuration,
            presentation_format: formats.view,
            pending_surface_reconfigurations: 1,
        })
    }

    pub fn handle_input(&mut self, event: InputEvent3D) -> Result<InteractionResult3D> {
        self.session.handle_input(event)
    }

    pub fn camera_snapshot(&self) -> CameraSnapshot3D {
        self.session.camera_snapshot()
    }

    pub fn resize(
        &mut self,
        surface: &wgpu::Surface<'_>,
        width: u32,
        height: u32,
        scale_factor: f32,
    ) -> Result<()> {
        self.session.resize(width, height, scale_factor)?;
        if self.configuration.width != width || self.configuration.height != height {
            self.configuration.width = width;
            self.configuration.height = height;
            surface.configure(self.renderer.context().device(), &self.configuration);
            self.pending_surface_reconfigurations =
                self.pending_surface_reconfigurations.saturating_add(1);
        }
        Ok(())
    }

    pub fn present(&mut self, surface: &wgpu::Surface<'_>) -> Result<GpuSurfacePresentStatus3D> {
        if self.renderer.context().is_lost() {
            return Ok(GpuSurfacePresentStatus3D::RecreateSurface);
        }
        let (surface_texture, reconfigure_after_present) = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => (texture, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => (texture, true),
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(GpuSurfacePresentStatus3D::Skipped);
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                surface.configure(self.renderer.context().device(), &self.configuration);
                self.pending_surface_reconfigurations =
                    self.pending_surface_reconfigurations.saturating_add(1);
                return Ok(GpuSurfacePresentStatus3D::Skipped);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                return Ok(GpuSurfacePresentStatus3D::RecreateSurface);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(PlottingError::RenderError(
                    "direct 3d surface acquisition hit a validation error".to_string(),
                ));
            }
        };
        let layout = Axis3Layout::resolve(&self.session.frame)?;
        let scene_output = match self.renderer.render_to_texture(
            &self.session.scene,
            &layout,
            self.session.frame.figure.dpi,
        ) {
            Ok(output) => output,
            Err(_) if self.renderer.context().is_lost() => {
                drop(surface_texture);
                return Ok(GpuSurfacePresentStatus3D::RecreateSurface);
            }
            Err(error) => return Err(error),
        };
        let target_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                label: Some("ruviz 3d sRGB surface view"),
                format: Some(self.presentation_format),
                ..Default::default()
            });
        let mut presentation = self.compositor.compose(
            self.renderer.context(),
            self.renderer.color_view()?,
            self.renderer.attachment_generation(),
            &target_view,
            &layout,
            &self.session.frame.figure,
            &self.session.frame.theme,
        )?;
        presentation.surface_reconfigurations = self.pending_surface_reconfigurations;
        self.pending_surface_reconfigurations = 0;
        surface_texture.present();
        if reconfigure_after_present {
            surface.configure(self.renderer.context().device(), &self.configuration);
            presentation.surface_reconfigurations =
                presentation.surface_reconfigurations.saturating_add(1);
        }
        let mut diagnostics = RenderDiagnostics3D {
            points_submitted: self.session.scene.point_count() as u64,
            triangles_submitted: self.session.scene.triangle_count() as u64,
            actual_backend: "gpu3d-surface".to_string(),
            adapter_name: Some(scene_output.adapter_name),
            sample_count: scene_output.sample_count,
            fallback_reason: None,
            readback_bytes: 0,
            presentation_vertex_upload_bytes: presentation.vertex_upload_bytes,
            presentation_texture_upload_bytes: presentation.texture_upload_bytes,
            surface_presents: 1,
            surface_reconfigurations: presentation.surface_reconfigurations,
            queue_waits: 0,
            ..RenderDiagnostics3D::default()
        };
        diagnostics.vertex_upload_bytes = scene_output.resource_update.vertex_upload_bytes;
        diagnostics.index_upload_bytes = scene_output.resource_update.index_upload_bytes;
        diagnostics.texture_upload_bytes = scene_output.resource_update.texture_upload_bytes;
        diagnostics.buffer_creations = scene_output
            .resource_update
            .buffer_creations
            .saturating_add(presentation.buffer_creations);
        diagnostics.camera_uniform_writes = scene_output.camera_uniform_writes;
        diagnostics.draw_calls = scene_output
            .draw_calls
            .saturating_add(presentation.draw_calls);
        Ok(GpuSurfacePresentStatus3D::Presented(diagnostics))
    }

    pub fn render_png_bytes(&self) -> Result<Vec<u8>> {
        self.session.render_png_bytes()
    }
}

/// Clamp a data-space point into the resolved plot bounds.
fn clamp_to_bounds(point: super::Point3D, bounds: super::Bounds3D) -> super::Point3D {
    super::Point3D::new(
        point.x.clamp(bounds.min.x, bounds.max.x),
        point.y.clamp(bounds.min.y, bounds.max.y),
        point.z.clamp(bounds.min.z, bounds.max.z),
    )
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
    fn one_wheel_event_has_a_bounded_zoom_step() {
        let mut session = scatter3d(&[0.0], &[0.0], &[0.0])
            .interactive_session()
            .expect("session");
        session
            .handle_input(InputEvent3D::Wheel { delta_y: 10_000.0 })
            .expect("wheel");
        let maximum_step = (MAX_WHEEL_DELTA_PER_EVENT_PX * WHEEL_EXPONENT_PER_PIXEL).exp();
        assert!((session.camera().get_zoom() - maximum_step).abs() <= f32::EPSILON);

        session.reset_view().expect("reset");
        session
            .handle_input(InputEvent3D::Wheel { delta_y: -10_000.0 })
            .expect("wheel");
        assert!((session.camera().get_zoom() - maximum_step.recip()).abs() <= f32::EPSILON);
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
    fn cancel_drag_handles_release_outside_without_a_synthetic_event() {
        let mut session = scatter3d(&[0.0], &[0.0], &[0.0])
            .interactive_session()
            .expect("session");
        let initial = session.camera();
        session
            .handle_input(InputEvent3D::PointerDown {
                x: 10.0,
                y: 10.0,
                button: PointerButton3D::Left,
            })
            .expect("down");
        assert!(session.is_drag_active());
        assert!(session.cancel_drag());
        assert!(!session.is_drag_active());
        assert!(!session.cancel_drag());

        let moved = session
            .handle_input(InputEvent3D::PointerMove { x: 80.0, y: 30.0 })
            .expect("move after cancellation");
        assert_eq!(moved, InteractionResult3D::default());
        assert_eq!(session.camera(), initial);
    }

    #[test]
    fn clamped_drag_reports_no_camera_change_and_escape_cancels_drag() {
        let mut session = scatter3d(&[0.0], &[0.0], &[0.0])
            .interactive_session()
            .expect("session");
        session
            .set_camera(session.camera().elevation_deg(89.9))
            .expect("set clamped elevation");
        session
            .handle_input(InputEvent3D::PointerDown {
                x: 10.0,
                y: 10.0,
                button: PointerButton3D::Left,
            })
            .expect("down");
        let clamped = session
            .handle_input(InputEvent3D::PointerMove { x: 10.0, y: -10.0 })
            .expect("clamped move");
        assert!(!clamped.camera_changed);
        assert!(!clamped.request_redraw);
        assert!(session.is_drag_active());

        session
            .handle_input(InputEvent3D::Escape)
            .expect("escape should reset and cancel");
        assert!(!session.is_drag_active());
    }

    #[test]
    fn named_camera_view_preserves_non_orientation_state() {
        let mut session = scatter3d(&[-2.0, 4.0], &[10.0, 30.0], &[5.0, 9.0])
            .interactive_session()
            .expect("session");
        let target = Point3D::new(1.0, 20.0, 7.0);
        let camera = session
            .camera()
            .perspective_deg(51.0)
            .axis_aspect(super::super::AxisAspect3D::Equal)
            .zoom(3.0)
            .look_at(target)
            .roll_deg(22.0);
        session.set_camera(camera).expect("custom camera");
        let before = session.camera_snapshot();

        session
            .apply_camera_view(CameraView3D::Top)
            .expect("top view");

        let after = session.camera_snapshot();
        assert_eq!(after.camera.get_azimuth_deg(), 0.0);
        assert_eq!(after.camera.get_elevation_deg(), 89.9);
        assert_eq!(after.camera.get_roll_deg(), 0.0);
        assert_eq!(after.camera.projection(), before.camera.projection());
        assert_eq!(
            after.camera.axis_aspect_value(),
            before.camera.axis_aspect_value()
        );
        assert_eq!(after.camera.get_zoom(), before.camera.get_zoom());
        assert_eq!(after.camera.target(), before.camera.target());
        assert_eq!(
            after.camera_generation,
            before.camera_generation.checked_add(1).expect("generation")
        );
    }

    #[test]
    fn fit_to_content_uses_scene_bounds_and_preserves_view_direction() {
        let mut session = scatter3d(&[-2.0, 4.0], &[10.0, 30.0], &[5.0, 9.0])
            .interactive_session()
            .expect("session");
        let camera = session
            .camera()
            .azimuth_deg(12.0)
            .elevation_deg(-34.0)
            .roll_deg(8.0)
            .perspective_deg(47.0)
            .axis_aspect(super::super::AxisAspect3D::Equal)
            .zoom(5.0)
            .look_at(Point3D::new(-2.0, 10.0, 5.0));
        session.set_camera(camera).expect("custom camera");
        let before = session.camera();
        let expected_center = session.frame.bounds.center();

        session.fit_to_content().expect("fit");

        let fitted = session.camera();
        assert_eq!(fitted.get_azimuth_deg(), before.get_azimuth_deg());
        assert_eq!(fitted.get_elevation_deg(), before.get_elevation_deg());
        assert_eq!(fitted.get_roll_deg(), before.get_roll_deg());
        assert_eq!(fitted.projection(), before.projection());
        assert_eq!(fitted.axis_aspect_value(), before.axis_aspect_value());
        assert_eq!(fitted.get_zoom(), 1.0);
        assert_eq!(fitted.target(), Some(expected_center));
    }

    #[test]
    fn view_stamp_distinguishes_scene_camera_and_target_changes() {
        let mut session = scatter3d(&[0.0], &[0.0], &[0.0])
            .interactive_session()
            .expect("session");
        let initial = session.view_stamp();
        session.orbit(4.0, 0.0).expect("orbit");
        let camera_changed = session.view_stamp();
        assert!(initial.same_scene(camera_changed));
        assert!(!initial.same_camera(camera_changed));
        assert!(initial.same_target(camera_changed));

        let (width, height) = session.size_px();
        session
            .resize(width + 1, height, 1.25)
            .expect("target resize");
        let target_changed = session.view_stamp();
        assert!(camera_changed.same_scene(target_changed));
        assert!(camera_changed.same_camera(target_changed));
        assert!(!camera_changed.same_target(target_changed));
        assert!(!session.is_view_current(initial));
        assert!(session.is_view_current(target_changed));
    }

    #[test]
    fn camera_generation_exhaustion_leaves_state_and_pick_unchanged() {
        let mut session = scatter3d(&[0.0], &[0.0], &[0.0])
            .interactive_session()
            .expect("session");
        session.camera_generation = u64::MAX;
        let layout = Axis3Layout::resolve(&session.frame).expect("layout");
        let center = layout.project_local(Vec3::ZERO).expect("center");
        session
            .pick(center.x, center.y)
            .expect("pick before exhaustion");

        let camera = session.camera();
        let stamp = session.view_stamp();
        let pick = session.current_pick();
        let error = session
            .set_camera(camera.azimuth_deg(camera.get_azimuth_deg() + 1.0))
            .expect_err("camera generation exhaustion must fail");

        assert!(matches!(error, PlottingError::RenderError(_)));
        assert_eq!(session.camera(), camera);
        assert_eq!(session.view_stamp(), stamp);
        assert_eq!(session.current_pick(), pick);
    }

    #[test]
    fn target_generation_exhaustion_leaves_state_and_pick_unchanged() {
        let mut session = scatter3d(&[0.0], &[0.0], &[0.0])
            .interactive_session()
            .expect("session");
        session.target_generation = u64::MAX;
        let layout = Axis3Layout::resolve(&session.frame).expect("layout");
        let center = layout.project_local(Vec3::ZERO).expect("center");
        session
            .pick(center.x, center.y)
            .expect("pick before exhaustion");

        let size = session.size_px();
        let dpi = session.frame.figure.dpi;
        let stamp = session.view_stamp();
        let pick = session.current_pick();
        let error = session
            .resize(size.0 + 1, size.1, dpi / 72.0)
            .expect_err("target generation exhaustion must fail");

        assert!(matches!(error, PlottingError::RenderError(_)));
        assert_eq!(session.size_px(), size);
        assert_eq!(session.frame.figure.dpi, dpi);
        assert_eq!(session.view_stamp(), stamp);
        assert_eq!(session.current_pick(), pick);
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
        let stamped = point.current_pick().expect("retained stamped pick");
        assert_eq!(hit.primitive, super::super::PickPrimitive3D::Point);
        assert_eq!(hit.sources(), &[0]);
        assert!(point.is_pick_current(&hit));
        assert!(point.is_stamped_pick_current(&stamped));
        point.orbit(4.0, 0.0).expect("orbit");
        assert!(!point.is_pick_current(&hit));
        assert!(!point.is_stamped_pick_current(&stamped));
        assert!(point.current_pick().is_none());

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
    fn session_construction_prebuilds_and_first_pick_reuses_the_bvh() {
        let mut session = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .interactive_session()
            .expect("session construction includes BVH preparation");
        let retained_bvh = Arc::clone(&session.bvh);
        let layout = Axis3Layout::resolve(&session.frame).expect("layout");
        let center = layout.project_local(Vec3::ZERO).expect("center");

        let _ = session.pick(center.x, center.y).expect("first pick");

        assert!(
            Arc::ptr_eq(&retained_bvh, &session.bvh),
            "the input path must reuse the BVH built by session construction"
        );
    }

    #[test]
    fn resize_clears_a_pick_without_claiming_the_camera_changed() {
        let mut session = scatter3d(&[0.0], &[0.0], &[0.0])
            .interactive_session()
            .expect("session");
        let layout = Axis3Layout::resolve(&session.frame).expect("layout");
        let center = layout.project_local(Vec3::ZERO).expect("center");
        session
            .pick(center.x, center.y)
            .expect("pick")
            .expect("hit");
        let pick = session.current_pick().expect("stamped pick");
        let before = session.view_stamp();
        let (width, height) = session.size_px();

        session.resize(width + 7, height + 5, 1.0).expect("resize");

        let after = session.view_stamp();
        assert!(before.same_camera(after));
        assert!(!before.same_target(after));
        assert!(session.current_pick().is_none());
        assert!(!session.is_stamped_pick_current(&pick));
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
    fn background_jobs_are_send_and_use_latest_request_wins() {
        fn assert_send<T: Send>() {}
        assert_send::<BackgroundRenderJob3D>();
        assert_send::<RenderedImage3D>();
        assert_send::<BackgroundRenderer3D>();

        let mut session = scatter3d(&[0.0], &[0.0], &[0.0])
            .size_px(48, 40)
            .interactive_session()
            .expect("session");
        let first = session.background_render_job().expect("first job");
        let second = session.background_render_job().expect("second job");
        assert!(Arc::ptr_eq(&first.frame, &session.frame));
        assert!(Arc::ptr_eq(&second.frame, &session.frame));
        assert!(!session.is_render_current(first.stamp()));
        assert!(session.is_render_current(second.stamp()));

        let first_frame = std::thread::spawn(move || first.render())
            .join()
            .expect("worker")
            .expect("first render");
        assert!(matches!(
            session.classify_render(first_frame),
            BackgroundRenderOutcome3D::Superseded { .. }
        ));

        let second_frame = second.render().expect("second render");
        let image_size = (second_frame.image.width, second_frame.image.height);
        assert!(matches!(
            session.classify_render(second_frame),
            BackgroundRenderOutcome3D::Current(_)
        ));
        assert_eq!(image_size, (48, 40));
    }

    #[test]
    fn a_view_change_supersedes_an_outstanding_background_job() {
        let mut session = scatter3d(&[0.0], &[0.0], &[0.0])
            .size_px(32, 32)
            .interactive_session()
            .expect("session");
        let job = session.background_render_job().expect("job");
        session.orbit(1.0, 0.0).expect("orbit");
        assert!(!session.is_render_current(job.stamp()));
        let rendered = job.render().expect("render");
        assert!(matches!(
            session.classify_render(rendered),
            BackgroundRenderOutcome3D::Superseded { .. }
        ));
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

    #[test]
    fn in_place_replacement_can_reset_or_keep_the_camera() {
        let mut session = scatter3d(&[0.0], &[0.0], &[0.0])
            .interactive_session()
            .expect("first");
        session.orbit(15.0, -4.0).expect("orbit");
        let kept_camera = session.camera();
        let first_scene = session.view_stamp();
        let replacement = scatter3d(&[1.0], &[2.0], &[3.0])
            .interactive_session()
            .expect("replacement");
        let replacement_default = replacement.camera();

        session
            .replace_keep_camera(replacement)
            .expect("keep-camera replacement");
        assert_eq!(session.camera(), kept_camera);
        assert!(!first_scene.same_scene(session.view_stamp()));

        let reset_replacement = scatter3d(&[4.0], &[5.0], &[6.0])
            .interactive_session()
            .expect("reset replacement");
        session.replace(reset_replacement);
        assert_eq!(session.camera(), replacement_default);
    }

    #[test]
    fn keep_camera_replacement_invalidates_replacement_frontend_state() {
        let mut current = scatter3d(&[0.0], &[0.0], &[0.0])
            .interactive_session()
            .expect("current");
        current.orbit(12.0, -3.0).expect("orbit");
        let kept_camera = current.camera();

        let mut replacement = scatter3d(&[1.0], &[2.0], &[3.0])
            .size_px(40, 32)
            .interactive_session()
            .expect("replacement");
        let layout = Axis3Layout::resolve(&replacement.frame).expect("layout");
        let center = layout.project_local(Vec3::ZERO).expect("center");
        replacement
            .pick(center.x, center.y)
            .expect("pick replacement");
        replacement
            .handle_input(InputEvent3D::PointerDown {
                x: center.x,
                y: center.y,
                button: PointerButton3D::Left,
            })
            .expect("start replacement drag");
        let stale_job = replacement
            .background_render_job()
            .expect("replacement job");
        assert!(replacement.current_pick().is_some());
        assert!(replacement.is_drag_active());
        assert!(replacement.is_render_current(stale_job.stamp()));

        current
            .replace_keep_camera(replacement)
            .expect("replace and keep camera");

        assert_eq!(current.camera(), kept_camera);
        assert!(current.current_pick().is_none());
        assert!(!current.is_drag_active());
        assert!(!current.is_render_current(stale_job.stamp()));
        let rendered = stale_job.render().expect("stale render remains executable");
        assert!(matches!(
            current.classify_render(rendered),
            BackgroundRenderOutcome3D::Superseded { .. }
        ));
    }

    #[test]
    fn replacement_invalidates_jobs_even_when_kept_camera_is_unchanged() {
        let mut current = scatter3d(&[0.0], &[0.0], &[0.0])
            .interactive_session()
            .expect("current");
        let mut replacement = scatter3d(&[1.0], &[1.0], &[1.0])
            .interactive_session()
            .expect("replacement");
        assert_eq!(current.camera(), replacement.camera());
        let layout = Axis3Layout::resolve(&replacement.frame).expect("layout");
        let center = layout.project_local(Vec3::ZERO).expect("center");
        replacement
            .pick(center.x, center.y)
            .expect("pick")
            .expect("replacement hit");
        let stale_pick = replacement.current_pick().expect("stamped pick");
        let stale_job = replacement.background_render_job().expect("job");

        current
            .replace_keep_camera(replacement)
            .expect("same-camera replacement");

        assert!(!current.is_render_current(stale_job.stamp()));
        assert!(!current.is_stamped_pick_current(&stale_pick));
    }

    #[test]
    fn replacement_generation_exhaustion_is_atomic_and_never_panics() {
        let mut current = scatter3d(&[0.0], &[0.0], &[0.0])
            .interactive_session()
            .expect("current");
        current.orbit(8.0, -2.0).expect("orbit");
        let original_view = current.view_stamp();
        let original_camera = current.camera();

        let mut replacement = scatter3d(&[1.0], &[1.0], &[1.0])
            .interactive_session()
            .expect("replacement");
        replacement.request_generation = u64::MAX;
        let error = current
            .try_replace(replacement)
            .expect_err("exhausted replacement must fail");
        assert!(error.to_string().contains("request space was exhausted"));
        assert_eq!(current.view_stamp(), original_view);
        assert_eq!(current.camera(), original_camera);

        let mut compatibility_replacement = scatter3d(&[2.0], &[2.0], &[2.0])
            .interactive_session()
            .expect("compatibility replacement");
        compatibility_replacement.request_generation = u64::MAX;
        current.replace(compatibility_replacement);
        assert_eq!(current.view_stamp(), original_view);
        assert_eq!(current.camera(), original_camera);
    }

    #[test]
    fn worker_renderer_defaults_to_cpu_and_reports_readback_truthfully() {
        let mut session = scatter3d(&[0.0], &[0.0], &[0.0])
            .size_px(24, 20)
            .interactive_session()
            .expect("session");
        let job = session.background_render_job().expect("job");
        let mut renderer = BackgroundRenderer3D::default();
        assert_eq!(renderer.backend(), BackgroundRenderBackend3D::Cpu);
        let (rendered, diagnostics) = renderer
            .render_with_diagnostics(job)
            .expect("background render");
        assert_eq!((rendered.image.width, rendered.image.height), (24, 20));
        assert_eq!(diagnostics.actual_backend, "cpu3d-background");
        assert_eq!(diagnostics.readback_bytes, 0);
        assert!(diagnostics.fallback_reason.is_none());
    }
}
