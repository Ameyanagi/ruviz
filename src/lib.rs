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
//! ## Gallery
//!
//! | Plot Types | | |
//! |:---:|:---:|:---:|
//! | ![Line Plot](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/line_plot.png) | ![Scatter Plot](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/scatter_plot.png) | ![Bar Chart](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/bar_chart.png) |
//! | Line Plot | Scatter Plot | Bar Chart |
//! | ![Histogram](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/histogram.png) | ![Box Plot](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/boxplot.png) | ![Heatmap](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/heatmap.png) |
//! | Histogram | Box Plot | Heatmap |
//!
//! | Styling Options | | |
//! |:---:|:---:|:---:|
//! | ![Line Styles](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/line_styles.png) | ![Marker Styles](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/marker_styles.png) | ![Colors](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/colors.png) |
//! | Line Styles | Marker Styles | Color Palette |
//!
//! | Themes | | | |
//! |:---:|:---:|:---:|:---:|
//! | ![Default](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/theme_default.png) | ![Dark](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/theme_dark.png) | ![Seaborn](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/theme_seaborn.png) | ![Publication](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/theme_publication.png) |
//! | Default | Dark | Seaborn | Publication |
//!
//! | Layout | |
//! |:---:|:---:|
//! | ![Legend Positions](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/legend_positions.png) | ![Subplots](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/subplots.png) |
//! | Legend Positions | Subplots |
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

pub mod axes;
pub mod core;
pub mod data;
pub mod export;
pub mod layout;
pub mod plots;
pub mod render;
pub mod simple;
pub mod text;

#[cfg(feature = "interactive")]
pub mod interactive;

/// Convenience re-exports for common usage
pub mod prelude {
    pub use crate::axes::AxisScale;
    pub use crate::core::{
        Annotation, ArrowHead, ArrowStyle, BackendType, FillStyle, GridSpec, HatchPattern,
        Legend, LegendAnchor, LegendItem, LegendItemType, LegendPosition, Plot, Position, Result,
        ShapeStyle, SubplotFigure, TextAlign, TextStyle, TextVAlign, subplots, subplots_default,
    };
    pub use crate::data::{Data1D, DataShader, DataShaderCanvas};
    pub use crate::plots::{HeatmapConfig, Interpolation};
    pub use crate::render::{
        Color, ColorMap, FontConfig, FontFamily, FontStyle, FontWeight, LineStyle, MarkerStyle,
        Theme,
    };

    #[cfg(feature = "interactive")]
    pub use crate::interactive::{
        event::{InteractionEvent, Point2D, Rectangle, Vector2D},
        renderer::RealTimeRenderer,
        state::InteractionState,
        window::{InteractiveWindow, InteractiveWindowBuilder, show_interactive},
    };
}
