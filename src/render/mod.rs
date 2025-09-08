//! Rendering backend and styling

pub mod backend;
pub mod skia;
pub mod primitives;
pub mod style;
pub mod color;
pub mod theme;

pub use backend::Renderer;
pub use style::{LineStyle, MarkerStyle};
pub use color::{Color, ColorError};
pub use theme::{Theme, ThemeBuilder, ThemeVariant};
pub use primitives::Primitive;