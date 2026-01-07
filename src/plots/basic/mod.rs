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
