//! Separate retained-mode foundation for three-dimensional plots.

mod builder;
mod diagnostics;
mod interaction;
pub(crate) mod layout;
mod picking;
mod prepared;
mod resolve;
mod types;

pub use crate::render::three_d::release_3d_gpu_resources;
pub use builder::{Line3DBuilder, Scatter3DBuilder, Surface3DBuilder, Wireframe3DBuilder};
pub use diagnostics::RenderDiagnostics3D;
pub use interaction::{
    BackgroundRenderBackend3D, BackgroundRenderJob3D, BackgroundRenderOutcome3D,
    BackgroundRenderer3D, CameraSnapshot3D, InputEvent3D, InteractionResult3D,
    InteractivePlot3DSession, PointerButton3D, RenderStamp3D, RenderedImage3D, StampedPick3D,
    ViewStamp3D,
};
#[cfg(feature = "gpu")]
#[doc(hidden)]
pub use interaction::{GpuSurfacePresentStatus3D, GpuSurfaceSession3D};
pub use picking::{PickHit3D, PickPrimitive3D};
#[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
pub use prepared::GpuBenchmarkSession3D;
pub use types::{
    AxisAspect3D, Bounds3D, Camera3D, Point3D, ProjectedPoint3D, Projection3D, ScreenRay3D,
};
