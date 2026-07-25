//! Configuration types for basic plot types
//!
//! This module provides configuration structs for fundamental plot types:
//! - [`LineConfig`] - Line plot configuration
//! - [`ScatterConfig`] - Scatter plot configuration
//! - [`BarConfig`] - Bar chart configuration
//!
//! These configs integrate with [`PlotBuilder<C>`](crate::core::PlotBuilder) to provide
//! a zero-ceremony API for basic plots.
//!
//! # Example
//!
//! ```rust,ignore
//! use ruviz::prelude::*;
//!
//! // Line plot with configuration
//! Plot::new()
//!     .line(&x, &y)
//!     .line_width(2.0)
//!     .color(Color::BLUE)
//!     .save("line.png")?;
//!
//! // Scatter plot with markers
//! Plot::new()
//!     .scatter(&x, &y)
//!     .marker(MarkerStyle::Circle)
//!     .marker_size(8.0)
//!     .save("scatter.png")?;
//! ```

mod bar;
mod line;
mod scatter;

pub use bar::{BarConfig, BarOrientation};
pub use line::LineConfig;
pub use scatter::ScatterConfig;

use crate::render::Color;

/// The one rule turning a marker-edge configuration into a renderer spec.
///
/// Shared by [`ScatterConfig`] and [`LineConfig`] so `.scatter().marker(..)` and
/// `.line().marker(..)` cannot drift apart on what counts as "no edge".
///
/// Returns `None` when nothing should be stroked — `show_edge` off, or a
/// non-positive width, since an invisible rim is no rim. A `None` colour in the
/// returned pair means "derive from the fill", which cannot be done here:
/// auto-palette series only learn their fill at render time.
///
/// The width stays in points; the renderer scales it to device pixels, so the
/// rim is the same physical thickness at every DPI.
pub(crate) fn marker_edge_spec(
    show_edge: bool,
    edge_color: Option<Color>,
    edge_width: f32,
) -> Option<(Option<Color>, f32)> {
    (show_edge && edge_width > 0.0).then_some((edge_color, edge_width))
}
