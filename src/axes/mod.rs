//! Axis management and scaling
//!
//! This module provides axis configuration, tick generation, and scale transformations.

pub mod inset;
pub mod polar;
pub mod scale;
pub mod secondary;
pub mod tick_layout;
pub mod ticks;

pub use inset::{ConnectorStyle, InsetAxes};
pub use polar::PolarAxes;
pub use scale::{AxisScale, LinearScale, LogScale, Scale, SymLogScale};
pub use secondary::{AxisType, DualAxes, SecondaryAxis};
pub use tick_layout::TickLayout;
pub use ticks::{
    generate_log_minor_ticks, generate_log_ticks, generate_minor_ticks, generate_symlog_ticks,
    generate_ticks, generate_ticks_for_scale,
};
