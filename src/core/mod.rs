//! Core plotting functionality and main API

pub mod annotation;
pub mod config;
pub mod constants;
pub mod error;
pub mod grid_style;
pub mod layout;
pub mod legend;
pub mod plot;
pub mod position;
pub mod style;
pub mod style_utils;
pub mod subplot;
pub mod tick_formatter;
pub mod transform;
pub mod types;
pub mod units;

pub use annotation::{
    Annotation, ArrowHead, ArrowStyle, FillStyle, HatchPattern, ShapeStyle, TextAlign, TextStyle,
    TextVAlign,
};
pub use config::{
    ComputedMargins, FigureConfig, LineConfig, MarginConfig, PlotConfig, SpacingConfig,
    SpineConfig, TypographyConfig,
};
pub use constants::{dimensions, dpi, font_scales, font_sizes, line_widths, margins, spacing};
pub use error::{PlottingError, Result};
pub use grid_style::GridStyle;
pub use layout::{
    ComputedMarginsPixels, LayoutCalculator, LayoutConfig, LayoutRect, PlotContent, PlotLayout,
    TextPosition,
};
#[allow(deprecated)]
pub use legend::LegendFrame; // Deprecated alias for backward compatibility
pub use legend::{
    Legend, LegendAnchor, LegendItem, LegendItemType, LegendPosition, LegendSpacing,
    LegendSpacingPixels, LegendStyle, find_best_position,
};
pub use plot::{BackendType, IntoPlot, Plot, PlotBuilder, PlotInput, SeriesStyle};
pub use position::Position;
pub use style::PlotStyle;
pub use style_utils::StyleResolver;
pub use subplot::{GridSpec, SubplotFigure, subplots, subplots_default};
pub use tick_formatter::TickFormatter;
pub use transform::CoordinateTransform;
pub use types::{BoundingBox, Orientation, Point2f};
pub use units::{
    POINTS_PER_INCH, REFERENCE_DPI, in_to_pt, in_to_px, pt_to_in, pt_to_px, px_to_in, px_to_pt,
};

#[cfg(test)]
mod validation_test;
