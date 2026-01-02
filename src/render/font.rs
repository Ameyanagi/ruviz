//! Font configuration types for text rendering
//!
//! This module provides font configuration types that integrate with the
//! cosmic-text based text rendering system. Font discovery is handled
//! automatically by cosmic-text's fontdb integration.
//!
//! # Example
//!
//! ```rust,ignore
//! use ruviz::render::{FontFamily, FontConfig, FontWeight, FontStyle};
//!
//! // Create a bold sans-serif font configuration
//! let config = FontConfig::new(FontFamily::SansSerif, 14.0)
//!     .bold()
//!     .italic();
//!
//! // Or use a specific font by name
//! let roboto = FontConfig::new(FontFamily::Name("Roboto".into()), 12.0);
//! ```

// Re-export from the unified text module
pub use crate::render::text::{FontConfig, FontFamily, FontStyle, FontWeight};
