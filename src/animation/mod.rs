//! Animation export system for ruviz
//!
//! This module provides animation recording and video export capabilities,
//! modeled after Makie.jl's proven animation architecture.
//!
//! # Features
//!
//! - **Tick-based timing**: Deterministic frame timing with `Tick` struct
//! - **Iterator-based recording**: `record()` function for frame generation
//! - **Multiple formats**: GIF (default), MP4/WebM via AV1 (optional)
//! - **Observable integration**: Reactive animations with `AnimatedObservable`
//! - **Smooth transitions**: Easing functions and plot morphing
//! - **Simplified API**: `record_simple()` with built-in interpolation helpers
//!
//! # Simplified API (Recommended)
//!
//! ```rust,ignore
//! use ruviz::animation::{record_simple, DurationExt, easing};
//!
//! // Record with frame count and tick interpolation helpers
//! record_simple("bounce.gif", 60, |t| {
//!     let y = t.ease_over(easing::ease_out_bounce, 100.0, 0.0, 2.0);
//!     Plot::new().scatter(&[0.0], &[y])
//! })?;
//!
//! // Or use duration syntax
//! record_simple("wave.gif", 2.0.secs(), |t| {
//!     let x = t.lerp_over(0.0, 10.0, 2.0);
//!     Plot::new().line(&[0.0, x], &[0.0, x])
//! })?;
//! ```
//!
//! # Animation Builder API
//!
//! For multi-value animations with custom easing:
//!
//! ```rust,ignore
//! use ruviz::animation::{Animation, easing};
//!
//! Animation::build()
//!     .value("x", 0.0).to(100.0).duration_secs(2.0)
//!     .value("y", 50.0).to(0.0).ease(easing::ease_out_bounce)
//!     .record("output.gif", |values, tick| {
//!         Plot::new().scatter(&[values["x"]], &[values["y"]])
//!     })?;
//! ```
//!
//! # Original API
//!
//! ```rust,ignore
//! use ruviz::prelude::*;
//! use ruviz::animation::record;
//!
//! let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
//!
//! record("wave.gif", 0..60, |frame, tick| {
//!     let phase = tick.time * 2.0 * std::f64::consts::PI;
//!     let y: Vec<f64> = x.iter().map(|&xi| (xi + phase).sin()).collect();
//!     #[allow(deprecated)]
//!     Plot::new()
//!         .line(&x, &y)
//!         .end_series()
//!         .title(format!("t = {:.2}s", tick.time))
//! }).unwrap();
//! ```
//!
//! # Feature Flags
//!
//! - `animation` - Core animation system with GIF export
//! - `animation-hq-gif` - High-quality GIF via gifski
//! - `animation-video` - MP4/WebM via pure Rust AV1 (rav1e)

mod builder;
mod interpolation;
mod macros;
mod observable_ext;
mod recorder;
mod stream;
mod tick;

pub mod encoders;

// Re-export core types
pub use encoders::{Codec, Encoder, Quality};
pub use interpolation::{Interpolate, easing};
pub use observable_ext::{AnimatedObservable, AnimationGroup, Tickable};
pub use recorder::{
    _record_duration,
    _record_duration_fps,
    // Internal functions for record! macro
    _record_frames,
    _record_frames_config,
    _record_reactive,
    _record_reactive_config,
    DurationExt,
    IntoFrameCount,
    RecordConfig,
    // Original API
    record,
    record_animated,
    record_animated_with_config,
    record_duration,
    record_duration_with_config,
    // Reactive plot recording
    record_plot,
    record_plot_with_config,
    // Simplified API
    record_simple,
    record_simple_with_config,
    record_with_config,
};
pub use stream::{FrameCapture, VideoConfig, VideoStream};
pub use tick::{Tick, TickGenerator, TickState};
// Animation Builder API
pub use builder::{Animation, AnimationBuilder, AnimationValues};
// Re-export Signal from data module for convenience
pub use crate::data::signal::{self, Signal};
