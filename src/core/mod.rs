//! Core plotting functionality and main API

pub mod annotation;
pub mod config;
pub mod constants;
pub mod error;
pub mod layout;
pub mod legend;
pub mod plot;
pub mod position;
pub mod style;
pub mod subplot;
pub mod types;
pub mod units;

pub use annotation::{
    Annotation, ArrowHead, ArrowStyle, FillStyle, HatchPattern, ShapeStyle, TextAlign, TextStyle,
    TextVAlign,
};
pub use config::{
    ComputedMargins, FigureConfig, LineConfig, MarginConfig, PlotConfig, SpacingConfig,
    TypographyConfig,
};
pub use constants::{dimensions, dpi, font_scales, font_sizes, line_widths, margins, spacing};
pub use error::{PlottingError, Result};
pub use layout::{
    ComputedMarginsPixels, LayoutCalculator, LayoutConfig, LayoutRect, PlotContent, PlotLayout,
    TextPosition,
};
pub use legend::{
    Legend, LegendAnchor, LegendFrame, LegendItem, LegendItemType, LegendPosition, LegendSpacing,
    LegendSpacingPixels, find_best_position,
};
pub use plot::{BackendType, Plot, PlotBuilder, PlotInput, SeriesStyle};
pub use position::Position;
pub use style::PlotStyle;
pub use subplot::{GridSpec, SubplotFigure, subplots, subplots_default};
pub use types::{BoundingBox, Point2f};
pub use units::{
    POINTS_PER_INCH, REFERENCE_DPI, in_to_pt, in_to_px, pt_to_in, pt_to_px, px_to_in, px_to_pt,
};

#[cfg(test)]
mod validation_test;
