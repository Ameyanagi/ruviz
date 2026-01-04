//! Interactive plotting system with real-time zoom, pan, and data brushing
//!
//! This module provides interactive capabilities built on top of the existing
//! Plot system, using winit for windowing and leveraging the existing GPU
//! acceleration for smooth 60fps interactions.

pub mod event;
pub mod renderer;
pub mod state;
pub mod window;

/// Test utilities for interactive mode testing
#[doc(hidden)]
pub mod test_utils;

pub use event::{EventHandler, InteractionEvent};
pub use renderer::RealTimeRenderer;
pub use state::{AnimationState, InteractionState};
pub use window::InteractiveWindow;
