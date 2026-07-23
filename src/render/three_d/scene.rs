use std::sync::Arc;

use crate::plots::SurfaceShading;
use crate::render::{Color, ColorMap, LineStyle, MarkerStyle};

/// Camera-independent point geometry retained by CPU and GPU renderers.
#[derive(Clone, Debug)]
pub(crate) struct PointGeometryBatch3D {
    pub(crate) series_index: u32,
    pub(crate) positions: Arc<[[f32; 3]]>,
    pub(crate) source_indices: Arc<[u32]>,
}

/// Camera-independent segment geometry retained by CPU and GPU renderers.
#[derive(Clone, Debug)]
pub(crate) struct LineGeometryBatch3D {
    pub(crate) series_index: u32,
    pub(crate) positions: Arc<[[f32; 3]]>,
    pub(crate) source_indices: Arc<[u32]>,
    pub(crate) segments: Arc<[[u32; 2]]>,
}

/// A normalized mesh vertex with renderer-neutral interpolants.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub(crate) struct MeshVertex3D {
    pub(crate) position: [f32; 3],
    pub(crate) normal: [f32; 3],
    pub(crate) scalar: f32,
    pub(crate) source_index: u32,
}

/// Camera-independent indexed mesh geometry.
#[derive(Clone, Debug)]
pub(crate) struct MeshGeometryBatch3D {
    pub(crate) series_index: u32,
    pub(crate) vertices: Arc<[MeshVertex3D]>,
    pub(crate) indices: Arc<[u32]>,
}

/// Geometry retained independently from camera and appearance.
#[derive(Clone, Debug, Default)]
pub(crate) struct SceneGeometry3D {
    pub(crate) points: Vec<Arc<PointGeometryBatch3D>>,
    pub(crate) lines: Vec<Arc<LineGeometryBatch3D>>,
    pub(crate) meshes: Vec<Arc<MeshGeometryBatch3D>>,
}

#[derive(Clone, Debug)]
pub(crate) struct PointStyle3D {
    pub(crate) color: Color,
    pub(crate) marker: MarkerStyle,
    pub(crate) marker_size: f32,
    pub(crate) label: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct StrokeStyle3D {
    pub(crate) color: Color,
    pub(crate) line_width: f32,
    pub(crate) line_style: LineStyle,
    pub(crate) label: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) enum MeshColor3D {
    Solid(Color),
    Scalar {
        colormap: ColorMap,
        data_range: (f64, f64),
    },
}

#[derive(Clone, Debug)]
pub(crate) struct MeshStyle3D {
    pub(crate) color: MeshColor3D,
    pub(crate) shading: SurfaceShading,
    pub(crate) two_sided: bool,
    pub(crate) label: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct PointBatch3D {
    pub(crate) geometry: Arc<PointGeometryBatch3D>,
    pub(crate) style: PointStyle3D,
}

#[derive(Clone, Debug)]
pub(crate) struct LineBatch3D {
    pub(crate) geometry: Arc<LineGeometryBatch3D>,
    pub(crate) style: StrokeStyle3D,
}

#[derive(Clone, Debug)]
pub(crate) struct MeshBatch3D {
    pub(crate) geometry: Arc<MeshGeometryBatch3D>,
    pub(crate) style: MeshStyle3D,
}

/// Small backend-neutral primitive vocabulary produced by high-level plots.
#[derive(Clone, Debug)]
pub(crate) struct Scene3D {
    pub(crate) geometry: Arc<SceneGeometry3D>,
    pub(crate) points: Vec<PointBatch3D>,
    pub(crate) lines: Vec<LineBatch3D>,
    pub(crate) meshes: Vec<MeshBatch3D>,
}

impl Scene3D {
    pub(crate) fn point_count(&self) -> usize {
        self.points
            .iter()
            .map(|batch| batch.geometry.positions.len())
            .sum()
    }

    pub(crate) fn segment_count(&self) -> usize {
        self.lines
            .iter()
            .map(|batch| batch.geometry.segments.len())
            .sum()
    }

    pub(crate) fn triangle_count(&self) -> usize {
        self.meshes
            .iter()
            .map(|batch| batch.geometry.indices.len() / 3)
            .sum()
    }
}
