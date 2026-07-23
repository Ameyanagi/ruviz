//! Direct retained wgpu renderer for 3d scenes.

mod context;
mod pipelines;
#[cfg(feature = "interactive-gpu")]
mod presenter;
mod renderer;
mod resources;

#[cfg(feature = "interactive-gpu")]
pub(crate) use presenter::{PresentedFrame3D, SurfacePresentOutcome3D, SurfacePresenter3D};
pub(crate) use renderer::{GpuFrameOutput3D, Wgpu3DRenderer, render_with_shared_renderer};
