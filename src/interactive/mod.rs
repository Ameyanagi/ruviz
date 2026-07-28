//! Interactive plotting system with real-time zoom, pan, and data brushing
//!
//! This module provides interactive capabilities built on top of the existing
//! Plot system, using winit for windowing and leveraging the existing GPU
//! acceleration for smooth 60fps interactions.

pub mod event;
pub mod renderer;
pub mod state;
#[cfg(all(feature = "3d", feature = "gpu"))]
mod three_d_window;
pub mod window;

/// Test utilities for interactive mode testing
#[doc(hidden)]
pub mod test_utils;

pub use event::{EventHandler, InteractionEvent};
pub use renderer::RealTimeRenderer;
pub use state::{AnimationState, InteractionState};
#[cfg(all(feature = "3d", feature = "gpu"))]
pub use three_d_window::show_interactive_3d;
pub use window::{
    InteractiveContextMenuActionContext, InteractiveContextMenuConfig, InteractiveContextMenuItem,
    InteractiveWindow, InteractiveWindowBuilder, show_interactive,
};
