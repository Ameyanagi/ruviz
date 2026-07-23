use crate::render::{Color, ColorMap};

/// Surface lighting mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SurfaceShading {
    /// Preserve colormap colors without lighting.
    Unlit,
    /// One normal per triangle.
    Flat,
    /// Area-weighted vertex normals.
    #[default]
    Smooth,
}

/// Surface sampling policy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SurfaceSampling {
    /// Full topology for static output and diagnosed LOD for interaction.
    #[default]
    Auto,
    /// Always use the complete source grid.
    Full,
    /// Use at most the requested regular-grid dimensions.
    MaxGrid { rows: usize, columns: usize },
}

/// Styling and sampling for a regular-grid surface.
#[derive(Clone, Debug)]
pub struct Surface3DConfig {
    /// Optional fixed color. When absent, z is mapped through `colormap`.
    pub color: Option<Color>,
    /// Scalar colormap used when no fixed color is present.
    pub colormap: ColorMap,
    /// Lighting mode.
    pub shading: SurfaceShading,
    /// Static/interactive sampling policy.
    pub sampling: SurfaceSampling,
    /// Whether to add a colorbar.
    pub colorbar: bool,
}

impl Default for Surface3DConfig {
    fn default() -> Self {
        Self {
            color: None,
            colormap: ColorMap::viridis(),
            shading: SurfaceShading::Smooth,
            sampling: SurfaceSampling::Auto,
            colorbar: false,
        }
    }
}
