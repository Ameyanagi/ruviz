//! Composition plot types
//!
//! Plots showing parts of a whole.
//! - Pie charts
//! - Donut charts

pub mod pie;

pub use pie::{PieConfig, PieData, render_pie};
