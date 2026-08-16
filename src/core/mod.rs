//! Core plotting functionality and main API

pub mod adapter;
pub mod annotation;
pub mod config;
pub mod constants;
pub mod context_menu;
pub mod error;
pub mod grid_style;
pub mod layout;
pub mod legend;
pub mod plot;
#[cfg(feature = "3d")]
pub mod plot3d;
pub mod position;
pub mod style;
pub mod style_utils;
pub mod subplot;
pub mod tick_formatter;
pub mod transform;
pub mod types;
pub mod units;

#[cfg(feature = "3d")]
pub use adapter::TryIntoPlot3DSession;
pub use adapter::{
    ImageFit, IntoPlotSession, LatestRequestScheduler, LogicalPoint, LogicalRect,
    RequestCompletion, ScheduledRequest, ScheduledRequestId, fitted_content_rect,
    logical_to_physical, physical_backing_size, sanitize_scale_factor,
};
pub use annotation::{
    Annotation, ArrowHead, ArrowStyle, FillStyle, HatchPattern, ShapeStyle, TextAlign, TextStyle,
    TextVAlign,
};
pub use config::{
    ComputedMargins, DEFAULT_AUTOSCALE_MARGIN, FigureConfig, LineConfig, MarginConfig, PlotConfig,
    SpacingConfig, SpineConfig, TypographyConfig,
};
pub use constants::{dimensions, dpi, font_scales, font_sizes, line_widths, margins, spacing};
pub use context_menu::PlotContextMenuAction;
pub use error::{PlotResult, PlottingError, Result};
pub use grid_style::GridStyle;
pub use layout::{
    ComputedMarginsPixels, LayoutCalculator, LayoutConfig, LayoutRect, MeasuredDimensions,
    PlotContent, PlotLayout, TextPosition,
};
pub(crate) use layout::{LayoutMeasurements, ResolvedLayout};
#[allow(deprecated)]
pub use legend::LegendFrame; // Deprecated alias for backward compatibility
pub use legend::{
    DIMMED_LEGEND_ALPHA, LEGEND_OCCUPANCY_RESOLUTION, Legend, LegendAnchor, LegendEntryLayout,
    LegendHitRegion, LegendItem, LegendItemType, LegendLayout, LegendOccupancy, LegendPlacement,
    LegendPosition, LegendSpacing, LegendSpacingPixels, LegendStyle, LegendTitleLayout,
    estimated_label_width, find_best_position, layout_legend, measure_legend_size,
};
pub use plot::{
    AlphaMode, AnnotationId, BackendFallbackReason, BackendOperation, BackendResolution,
    BackendType, BuilderWhen, DirtyDomain, DirtyDomains, FramePacing, FrameStats, HitResult, Image,
    ImageTarget, InsetAnchor, InsetLayout, InteractiveChangeRevision,
    InteractiveChangeSubscription, InteractiveFrame, InteractiveFrameWithGeneration,
    InteractivePlotSession, InteractiveRenderStamp, InteractiveViewportSnapshot, IntoPlot,
    LayerImages, LayerRenderState, Plot, PlotBuilder, PlotInputEvent, PlotSource, PreparedPlot,
    QualityPolicy, ReactiveSubscription, ReactiveValue, RenderTargetKind, RenderedLayer,
    StampedInteractiveFrame, StampedInteractiveLayers, SurfaceCapability, SurfaceTarget,
    TextEngineMode, TickDirection, TickSides, ViewportPoint, ViewportRect,
    source_over_straight_rgba,
};
// `PlotInput` and `SeriesStyle` are internal representations of a half-built series
// (`SeriesStyle` alone has 18 public fields, including reactive-animation plumbing).
// Re-exporting them froze every internal refactor into a breaking change, so they are
// crate-visible here and absent from `crate::prelude`.
pub(crate) use plot::{PlotInput, SeriesStyle};
#[cfg(all(feature = "3d", feature = "gpu", not(target_arch = "wasm32")))]
pub use plot3d::GpuBenchmarkSession3D;
#[cfg(feature = "3d")]
pub use plot3d::{
    AxisAspect3D, BackgroundRenderBackend3D, BackgroundRenderJob3D, BackgroundRenderOutcome3D,
    BackgroundRenderer3D, Bounds3D, Camera3D, CameraSnapshot3D, CameraView3D, InputEvent3D,
    InteractionResult3D, InteractivePlot3DSession, Line3DBuilder, PickHit3D, PickPrimitive3D,
    Point3D, PointerButton3D, ProjectedPoint3D, Projection3D, RenderDiagnostics3D, RenderStamp3D,
    RenderedImage3D, Scatter3DBuilder, ScreenRay3D, StampedPick3D, Surface3DBuilder, ViewStamp3D,
    Wireframe3DBuilder, release_3d_gpu_resources,
};
#[cfg(all(feature = "3d", feature = "gpu"))]
#[doc(hidden)]
pub use plot3d::{GpuSurfacePresentStatus3D, GpuSurfaceSession3D};
#[allow(deprecated)]
pub use position::Position;
pub use style::PlotStyle;
pub use style_utils::StyleResolver;
pub use subplot::{FigureRect, GridSpec, SubplotFigure, figure, subplots, subplots_default};
pub use tick_formatter::TickFormatter;
pub use transform::CoordinateTransform;
pub use types::{BoundingBox, Orientation, Point2f};
pub use units::{
    POINTS_PER_INCH, REFERENCE_DPI, RenderScale, in_to_pt, in_to_px, pt_to_in, pt_to_px, px_to_in,
    px_to_pt,
};

#[cfg(test)]
mod validation_test;
