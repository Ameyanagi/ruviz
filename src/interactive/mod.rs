//! Interactive plotting system with real-time zoom, pan, and data brushing
//! 
//! This module provides interactive capabilities built on top of the existing
//! Plot system, using winit for windowing and leveraging the existing GPU
//! acceleration for smooth 60fps interactions.

pub mod event;
pub mod state;
pub mod renderer;
pub mod window;

#[cfg(test)]
pub mod test_utils;

pub use event::{InteractionEvent, EventHandler};
pub use state::{InteractionState, AnimationState};
pub use renderer::RealTimeRenderer;
pub use window::InteractiveWindow;