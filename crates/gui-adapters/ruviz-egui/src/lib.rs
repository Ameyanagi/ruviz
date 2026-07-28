//! Native [`egui`] widgets for static and interactive [`ruviz`] plots.
//!
//! The widgets are app-owned retained values. Calling `show` only drains
//! already-completed frames, translates input, and schedules work; rendering
//! happens on background threads. The last good texture remains visible while
//! a newer frame is pending.
//!
//! This crate deliberately depends on `egui`, not an application shell.
//! `eframe` is used only by the runnable examples.

mod shared;
mod two_d;

#[cfg(feature = "3d")]
mod three_d;

pub use egui;
pub use ruviz;
pub use ruviz::core::ImageFit;
pub use shared::{AdapterError, AdapterErrorKind, PlotSize, ViewMode};
pub use two_d::{PlotEvent, PlotResponse, RuvizPlot, RuvizPlotBuilder, plot_builder};

#[cfg(feature = "3d")]
pub use three_d::{Plot3DEvent, Plot3DResponse, RuvizPlot3D, RuvizPlot3DBuilder, plot3d_builder};
