//! Native [`egui`] widgets for static and interactive [`ruviz`] plots.
//!
//! The widgets are app-owned retained values. Calling `show` only drains
//! already-completed frames, translates input, and schedules work; rendering
//! happens on one background thread per widget. The last good texture remains
//! visible while a newer frame is pending.
//!
//! A 2D frame is presented as its base and overlay layers stacked as two
//! textures, so an overlay-only redraw — a hover, tooltip, or brush — re-uploads
//! only the overlay and never composites a full frame on the CPU.
//!
//! This crate deliberately depends on `egui`, not an application shell.
//! `eframe` is used only by the runnable examples.

mod shared;
mod two_d;

#[cfg(feature = "3d")]
mod three_d;

pub use egui;
pub use ruviz;
pub use ruviz::core::{ImageFit, PlotContextMenuAction};
pub use shared::{AdapterError, AdapterErrorKind, PlotSize, ViewMode};
pub use two_d::{PlotEvent, PlotResponse, RuvizPlot, RuvizPlotBuilder, plot_builder};

#[cfg(feature = "3d")]
pub use ruviz::core::CameraView3D;
#[cfg(feature = "3d")]
pub use three_d::{Plot3DEvent, Plot3DResponse, RuvizPlot3D, RuvizPlot3DBuilder, plot3d_builder};
