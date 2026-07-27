//! Direct retained wgpu renderer for 3d scenes.
#![cfg_attr(target_arch = "wasm32", allow(clippy::arc_with_non_send_sync))]

mod context;
mod pipelines;
mod presenter;
mod renderer;
mod resources;

pub(crate) use context::GpuContext3D;
pub(crate) use presenter::{PresentationCompositor3D, select_surface_format};
#[cfg(all(feature = "interactive-gpu", not(target_arch = "wasm32")))]
pub(crate) use presenter::{PresentedFrame3D, SurfacePresentOutcome3D, SurfacePresenter3D};
pub(crate) use renderer::{GpuFrameOutput3D, Wgpu3DRenderer};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use renderer::{release_shared_renderer, render_with_shared_renderer};
