// Clippy configuration - allow some lints that are too strict for this codebase
// too_many_arguments: Many rendering functions require multiple parameters for
// flexibility. Consider config structs for future additions, but current API is ergonomic.
#![allow(clippy::too_many_arguments)]
#![allow(unconditional_recursion)]
// Allow unused code during development
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(unreachable_code)]
#![allow(unreachable_patterns)]

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
//! Click any image to view full size. Expand sections below to see code examples.
//!
//! ### Plot Types
//!
//! | | | |
//! |:---:|:---:|:---:|
//! | [![Line Plot](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/line_plot.png)](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/line_plot.png) | [![Scatter Plot](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/scatter_plot.png)](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/scatter_plot.png) | [![Bar Chart](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/bar_chart.png)](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/bar_chart.png) |
//! | Line Plot | Scatter Plot | Bar Chart |
//! | [![Histogram](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/histogram.png)](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/histogram.png) | [![Box Plot](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/boxplot.png)](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/boxplot.png) | [![Heatmap](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/heatmap.png)](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/heatmap.png) |
//! | Histogram | Box Plot | Heatmap |
//!
//! <details>
//! <summary>Plot Types Code Examples</summary>
//!
//! **Line Plot**
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
//! let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
//!
//! Plot::new()
//!     .title("Sine Wave")
//!     .xlabel("x")
//!     .ylabel("sin(x)")
//!     .line(&x, &y)
//!     .end_series()
//!     .save("line_plot.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! **Scatter Plot**
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.2).collect();
//! let y: Vec<f64> = x.iter().enumerate()
//!     .map(|(i, &v)| v.sin() + (i as f64 * 0.1).sin() * 0.3)
//!     .collect();
//!
//! Plot::new()
//!     .title("Scatter Plot")
//!     .xlabel("x")
//!     .ylabel("y")
//!     .scatter(&x, &y)
//!     .end_series()
//!     .save("scatter_plot.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! **Bar Chart**
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! let categories = vec!["A", "B", "C", "D", "E"];
//! let values = vec![23.0, 45.0, 56.0, 78.0, 32.0];
//!
//! Plot::new()
//!     .title("Bar Chart")
//!     .xlabel("Category")
//!     .ylabel("Value")
//!     .bar(&categories, &values)
//!     .end_series()
//!     .save("bar_chart.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! **Histogram**
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! // Generate sample data
//! let data: Vec<f64> = (0..1000).map(|i| {
//!     let u1 = ((i * 7 + 13) % 1000) as f64 / 1000.0;
//!     let u2 = ((i * 11 + 17) % 1000) as f64 / 1000.0;
//!     (-2.0 * u1.max(0.001).ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
//! }).collect();
//!
//! Plot::new()
//!     .title("Histogram")
//!     .xlabel("Value")
//!     .ylabel("Frequency")
//!     .histogram(&data, None)
//!     .end_series()
//!     .save("histogram.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! **Box Plot**
//! ```rust,no_run
//! use ruviz::prelude::*;
//! use ruviz::plots::boxplot::BoxPlotConfig;
//!
//! let data = vec![
//!     1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0,
//!     11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0,
//!     35.0, 40.0, -5.0,  // Outliers
//! ];
//!
//! Plot::new()
//!     .title("Box Plot")
//!     .xlabel("Distribution")
//!     .ylabel("Values")
//!     .boxplot(&data, Some(BoxPlotConfig::new()))
//!     .end_series()
//!     .save("boxplot.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! **Heatmap**
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! // Create 2D data (distance from center)
//! let data: Vec<Vec<f64>> = (0..10).map(|i| {
//!     (0..10).map(|j| {
//!         ((i as f64 - 5.0).powi(2) + (j as f64 - 5.0).powi(2)).sqrt()
//!     }).collect()
//! }).collect();
//!
//! Plot::new()
//!     .title("Heatmap")
//!     .xlabel("X")
//!     .ylabel("Y")
//!     .heatmap(&data, None)
//!     .end_series()
//!     .save("heatmap.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! </details>
//!
//! ### Styling Options
//!
//! | | | |
//! |:---:|:---:|:---:|
//! | [![Line Styles](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/line_styles.png)](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/line_styles.png) | [![Marker Styles](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/marker_styles.png)](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/marker_styles.png) | [![Colors](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/colors.png)](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/colors.png) |
//! | Line Styles | Marker Styles | Color Palette |
//!
//! <details>
//! <summary>Styling Code Examples</summary>
//!
//! **Line Styles**
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
//!
//! Plot::new()
//!     .title("Line Styles")
//!     .legend_position(LegendPosition::Best)
//!     .line(&x, &x.iter().map(|&v| v.sin() + 4.0).collect::<Vec<_>>())
//!     .label("Solid").style(LineStyle::Solid)
//!     .line(&x, &x.iter().map(|&v| v.sin() + 3.0).collect::<Vec<_>>())
//!     .label("Dashed").style(LineStyle::Dashed)
//!     .line(&x, &x.iter().map(|&v| v.sin() + 2.0).collect::<Vec<_>>())
//!     .label("Dotted").style(LineStyle::Dotted)
//!     .line(&x, &x.iter().map(|&v| v.sin() + 1.0).collect::<Vec<_>>())
//!     .label("DashDot").style(LineStyle::DashDot)
//!     .end_series()
//!     .save("line_styles.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! **Marker Styles**
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! let x: Vec<f64> = (0..5).map(|j| j as f64 * 2.0).collect();
//!
//! Plot::new()
//!     .title("Marker Styles")
//!     .legend_position(LegendPosition::Best)
//!     .scatter(&x, &vec![5.0; 5]).label("Circle").marker(MarkerStyle::Circle)
//!     .scatter(&x, &vec![4.0; 5]).label("Square").marker(MarkerStyle::Square)
//!     .scatter(&x, &vec![3.0; 5]).label("Triangle").marker(MarkerStyle::Triangle)
//!     .scatter(&x, &vec![2.0; 5]).label("Diamond").marker(MarkerStyle::Diamond)
//!     .scatter(&x, &vec![1.0; 5]).label("Star").marker(MarkerStyle::Star)
//!     .end_series()
//!     .save("marker_styles.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! **Color Palette**
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! let palette = Color::default_palette();
//! let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
//!
//! Plot::new()
//!     .title("Default Color Palette")
//!     .legend_position(LegendPosition::Best)
//!     .line(&x, &x.iter().map(|&v| v.sin() + 3.0).collect::<Vec<_>>())
//!     .label("Color 1").color(palette[0])
//!     .line(&x, &x.iter().map(|&v| v.sin() + 2.0).collect::<Vec<_>>())
//!     .label("Color 2").color(palette[1])
//!     .line(&x, &x.iter().map(|&v| v.sin() + 1.0).collect::<Vec<_>>())
//!     .label("Color 3").color(palette[2])
//!     .line(&x, &x.iter().map(|&v| v.sin()).collect::<Vec<_>>())
//!     .label("Color 4").color(palette[3])
//!     .end_series()
//!     .save("colors.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! </details>
//!
//! ### Themes
//!
//! | | | | |
//! |:---:|:---:|:---:|:---:|
//! | [![Default](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/theme_default.png)](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/theme_default.png) | [![Dark](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/theme_dark.png)](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/theme_dark.png) | [![Seaborn](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/theme_seaborn.png)](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/theme_seaborn.png) | [![Publication](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/theme_publication.png)](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/theme_publication.png) |
//! | Default | Dark | Seaborn | Publication |
//!
//! <details>
//! <summary>Themes Code Examples</summary>
//!
//! **Default Theme**
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
//! let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
//!
//! Plot::new()
//!     .title("Default Theme")
//!     .line(&x, &y)
//!     .end_series()
//!     .save("theme_default.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! **Dark Theme**
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
//! let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
//!
//! Plot::new()
//!     .title("Dark Theme")
//!     .theme(Theme::dark())
//!     .line(&x, &y)
//!     .end_series()
//!     .save("theme_dark.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! **Seaborn Theme**
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
//! let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
//!
//! Plot::new()
//!     .title("Seaborn Theme")
//!     .theme(Theme::seaborn())
//!     .line(&x, &y)
//!     .end_series()
//!     .save("theme_seaborn.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! **Publication Theme**
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
//! let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
//!
//! Plot::new()
//!     .title("Publication Theme")
//!     .theme(Theme::publication())
//!     .line(&x, &y)
//!     .end_series()
//!     .save("theme_publication.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! </details>
//!
//! ### Layout
//!
//! | | |
//! |:---:|:---:|
//! | [![Legend Positions](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/legend_positions.png)](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/legend_positions.png) | [![Subplots](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/subplots.png)](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/subplots.png) |
//! | Legend Positions | Subplots |
//!
//! <details>
//! <summary>Layout Code Examples</summary>
//!
//! **Legend Positions**
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
//! let y_sin: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
//! let y_cos: Vec<f64> = x.iter().map(|&v| v.cos()).collect();
//!
//! // Create plots with different legend positions
//! let plot_ul = Plot::new()
//!     .title("UpperLeft")
//!     .legend_position(LegendPosition::UpperLeft)
//!     .line(&x, &y_sin).label("sin(x)")
//!     .line(&x, &y_cos).label("cos(x)")
//!     .end_series();
//!
//! let plot_ur = Plot::new()
//!     .title("UpperRight")
//!     .legend_position(LegendPosition::UpperRight)
//!     .line(&x, &y_sin).label("sin(x)")
//!     .line(&x, &y_cos).label("cos(x)")
//!     .end_series();
//!
//! let plot_ll = Plot::new()
//!     .title("LowerLeft")
//!     .legend_position(LegendPosition::LowerLeft)
//!     .line(&x, &y_sin).label("sin(x)")
//!     .line(&x, &y_cos).label("cos(x)")
//!     .end_series();
//!
//! let plot_lr = Plot::new()
//!     .title("LowerRight")
//!     .legend_position(LegendPosition::LowerRight)
//!     .line(&x, &y_sin).label("sin(x)")
//!     .line(&x, &y_cos).label("cos(x)")
//!     .end_series();
//!
//! // Combine in 2x2 subplots
//! subplots(2, 2, 800, 600)?
//!     .suptitle("Legend Positions")
//!     .subplot_at(0, plot_ul)?
//!     .subplot_at(1, plot_ur)?
//!     .subplot_at(2, plot_ll)?
//!     .subplot_at(3, plot_lr)?
//!     .save("legend_positions.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! **Subplots**
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.2).collect();
//!
//! let plot_line = Plot::new()
//!     .title("Line Plot")
//!     .line(&x, &x.iter().map(|&v| v.sin()).collect::<Vec<_>>())
//!     .end_series();
//!
//! let plot_scatter = Plot::new()
//!     .title("Scatter Plot")
//!     .scatter(&x, &x.iter().map(|&v| v.cos()).collect::<Vec<_>>())
//!     .end_series();
//!
//! let plot_bar = Plot::new()
//!     .title("Bar Chart")
//!     .bar(&["Q1", "Q2", "Q3", "Q4"], &[28.0, 45.0, 38.0, 52.0])
//!     .end_series();
//!
//! let plot_multi = Plot::new()
//!     .title("Comparison")
//!     .legend_position(LegendPosition::UpperRight)
//!     .line(&x, &x.iter().map(|&v| v.sin()).collect::<Vec<_>>()).label("sin")
//!     .line(&x, &x.iter().map(|&v| v.cos()).collect::<Vec<_>>()).label("cos")
//!     .end_series();
//!
//! subplots(2, 2, 800, 600)?
//!     .suptitle("Subplot Gallery")
//!     .subplot_at(0, plot_line)?
//!     .subplot_at(1, plot_scatter)?
//!     .subplot_at(2, plot_bar)?
//!     .subplot_at(3, plot_multi)?
//!     .save("subplots.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! </details>
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
//!
//! ### With Legend (matplotlib-style)
//!
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
//! let sin_y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
//! let cos_y: Vec<f64> = x.iter().map(|&v| v.cos()).collect();
//!
//! Plot::new()
//!     .title("Trigonometric Functions")
//!     .line(&x, &sin_y).label("sin(x)")
//!     .line(&x, &cos_y).label("cos(x)")
//!     .end_series()     // Finish series chain
//!     .legend_best()    // Enable legend (like plt.legend())
//!     .save("trig.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Figure Size and DPI
//!
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
//! let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
//!
//! Plot::new()
//!     .size(8.0, 6.0)  // 8×6 inches
//!     .dpi(300)        // 300 DPI = 2400×1800 pixels
//!     .line(&x, &y)
//!     .save("high_res.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Named Colors
//!
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.2).collect();
//! let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
//!
//! // Use named colors (no unwrap needed!)
//! let color = Color::named("coral").unwrap_or(Color::RED);
//!
//! Plot::new()
//!     .line(&x, &y).color(color)
//!     .save("colored.png")?;
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
        Annotation, ArrowHead, ArrowStyle, BackendType, FillStyle, GridSpec, HatchPattern, Legend,
        LegendAnchor, LegendItem, LegendItemType, LegendPosition, Plot, Position, Result,
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
