//! Backend-neutral 3D scene data and renderers.

#[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
pub(crate) mod gpu;
pub(crate) mod overlay;
pub(crate) mod scene;
pub(crate) mod software;
