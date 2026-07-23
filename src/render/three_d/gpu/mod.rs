//! Direct retained wgpu renderer for 3d scenes.

mod context;
mod pipelines;
mod renderer;
mod resources;

pub(crate) use renderer::{GpuFrameOutput3D, Wgpu3DRenderer, render_with_shared_renderer};
