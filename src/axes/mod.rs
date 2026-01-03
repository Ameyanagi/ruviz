//! Axis management and scaling
//!
//! This module provides axis configuration, tick generation, and scale transformations.

pub mod scale;
pub mod ticks;

pub use scale::{LinearScale, LogScale, Scale};
pub use ticks::{generate_minor_ticks, generate_ticks};
