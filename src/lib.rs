//! # Ruviz - High-Performance Rust Plotting Library
//!
//! A modern, high-performance 2D plotting library for Rust that combines matplotlib's
//! comprehensiveness with Makie's performance-oriented design, while maintaining Rust's
//! safety and ergonomics.
//!
//! ## Features
//!
//! - **High Performance**: <100ms for 100K points, <1s for 1M points
//! - **Zero Unsafe Public API**: Memory safety without compromising performance
//! - **Multiple Plot Types**: Line, scatter, bar, histogram, heatmap
//! - **Publication Quality**: PNG/SVG export with custom themes
//! - **Large Dataset Support**: DataShader-style aggregation for 100M+ points
//! - **Cross Platform**: Linux, macOS, Windows, WASM support
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
//! let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];
//!
//! Plot::new()
//!     .line(&x, &y)
//!     .title("Quadratic Function")
//!     .xlabel("x")
//!     .ylabel("y = x²")
//!     .save("plot.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod core;
pub mod data;
pub mod render;
pub mod plots;
pub mod axes;
pub mod layout;
pub mod text;
pub mod export;

#[cfg(feature = "interactive")]
pub mod interactive;

/// Convenience re-exports for common usage
pub mod prelude {
    pub use crate::core::{Plot, Position, SubplotFigure, GridSpec, subplots, subplots_default, Result};
    pub use crate::data::{Data1D, DataShader, DataShaderCanvas};
    pub use crate::render::{Color, ColorMap, LineStyle, MarkerStyle, Theme, FontFamily, FontConfig, FontWeight, FontStyle};
    
    #[cfg(feature = "interactive")]
    pub use crate::interactive::{
        event::{InteractionEvent, Point2D, Vector2D, Rectangle},
        state::InteractionState,
        renderer::RealTimeRenderer,
        window::{InteractiveWindow, InteractiveWindowBuilder, show_interactive},
    };
}