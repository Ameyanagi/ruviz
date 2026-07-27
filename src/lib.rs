// Clippy configuration - allow some lints that are too strict for this codebase
// too_many_arguments: Many rendering functions require multiple parameters for
// flexibility. Consider config structs for future additions, but current API is ergonomic.
#![allow(clippy::too_many_arguments)]
// Allow unused code during development
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(unreachable_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! # Ruviz - High-Performance Rust Plotting Library
//!
//! A modern, high-performance 2D plotting library for Rust that combines matplotlib's
//! comprehensiveness with Makie's performance-oriented design, while maintaining Rust's
//! safety and ergonomics.
//!
//! ## Features
//!
//! - **Performance-Oriented**: Built for release-mode plotting workloads
//!   with benchmarkable output paths
//! - **Zero Unsafe Public API**: Memory safety without compromising performance
//! - **19 Plot Types (23 with the `3d` feature)**: basic, distribution, continuous,
//!   composition, polar and vector families, plus subplot layout helpers
//! - **Publication Quality**: PNG/SVG export with custom themes
//! - **Large Dataset Support**: Streaming-friendly data structures and
//!   practical downsampling workflows
//! - **Cross Platform**: Linux, macOS, Windows
//! - **Animation Tooling**: Frame-based recording plus signal-aware plot data APIs
//!
//! ## Quick Start
//!
//! Every plot is built the same way: `Plot::new()`, one series method, setters,
//! then `save`. There is no second entry point to learn.
//!
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! // Line plot
//! let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
//! let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
//! Plot::new().line(&x, &y).title("Sine Wave").save("sine.png")?;
//!
//! // Scatter plot
//! Plot::new().scatter(&x, &y).title("Points").marker(MarkerStyle::Circle).save("scatter.png")?;
//!
//! // Bar chart
//! let cats = ["A", "B", "C", "D"];
//! let vals = [10.0, 25.0, 15.0, 30.0];
//! Plot::new().bar(&cats, &vals).title("Sales").save("bar.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! `save` finishes the pending series for you, so `.end_series()` is only needed
//! when you want the [`Plot`] value itself — for example to hand it to
//! [`subplots`](crate::core::subplots).
//!
//! ## Typst Text Mode
//!
//! Enable Typst-backed labels and titles by turning on the `typst-math` feature:
//!
//! ```toml
//! [dependencies]
//! ruviz = { version = "0.4.16", features = ["typst-math"] }
//! ```
//!
//! Then opt into Typst text rendering per plot with `.typst(true)`:
//!
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
//! let y: Vec<f64> = x.iter().map(|&v| (-v).exp()).collect();
//!
//! let mut plot = Plot::new()
//!     .line(&x, &y)
//!     .title("$f(x) = e^(-x)$")
//!     .xlabel("$x$")
//!     .ylabel("$f(x)$");
//!
//! #[cfg(feature = "typst-math")]
//! {
//!     plot = plot.typst(true);
//! }
//!
//! plot.save("typst_plot.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! If `typst-math` is not enabled, `.typst(true)` is unavailable and the compiler reports:
//!
//! ```text
//! error[E0599]: no method named `typst` found for struct `ruviz::core::Plot` in the current scope
//! ```
//!
//! If Typst is optional in your own crate, define a local feature and forward it to
//! `ruviz/typst-math`:
//!
//! ```toml
//! [dependencies]
//! ruviz = { version = "0.4.16", default-features = false }
//!
//! [features]
//! default = []
//! typst-math = ["ruviz/typst-math"]
//! ```
//!
//! Then guard the call with `#[cfg(feature = "typst-math")]` in your crate. Selecting the text
//! engine directly follows the same rule: `TextEngineMode::Typst` is only available when
//! `typst-math` is enabled.
//!
//! ## Animation APIs
//!
//! Create smooth animations with `record!` closures today. Signal-backed plot data
//! and labels can also be attached to a plot. Plain `render()` and `save()`
//! sample temporal sources at `0.0`, while `render_at()` lets you choose the
//! sampling time before using the normal backend-selection path. Push-based
//! reactive sources use their latest value when the plot snapshot is built.
//!
//! ### Basic Animation with record! Macro
//!
//! ```rust,ignore
//! use ruviz::prelude::*;
//! use ruviz::record;
//!
//! // Frame-based animation
//! record!("wave.gif", 60, |t| {
//!     let phase = t.time * 2.0 * std::f64::consts::PI;
//!     let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
//!     let y: Vec<f64> = x.iter().map(|&xi| (xi + phase).sin()).collect();
//!     Plot::new().line(&x, &y).title(format!("t = {:.2}s", t.time))
//! })?;
//!
//! // Duration-based animation (2 seconds at 30fps)
//! record!("bounce.gif", 2 secs, |t| {
//!     let y = t.ease_over(easing::ease_out_bounce, 100.0, 0.0, 2.0);
//!     Plot::new().scatter(&[0.0], &[y]).title("Bouncing Ball")
//! })?;
//!
//! // Custom framerate
//! record!("smooth.gif", 3 secs @ 60 fps, |t| {
//!     let x = t.lerp_over(0.0, 10.0, 3.0);
//!     Plot::new().line(&[0.0, x], &[0.0, x]).title("Growing Line")
//! })?;
//! ```
//!
//! ### Signals Inside Animation Closures
//!
//! Use `Signal<T>` to build time-varying values, then sample them inside
//! `record!` closures for animated series data today:
//!
//! ```rust,ignore
//! use ruviz::prelude::*;
//! use ruviz::animation::signal;
//! use ruviz::record;
//!
//! // Create signals that vary over time
//! let amplitude = signal::lerp(0.0, 2.0, 3.0);  // 0 to 2 over 3 seconds
//! let frequency = signal::ease(easing::ease_in_out_quad, 1.0, 5.0, 3.0);
//!
//! // Compose signals
//! let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
//! let y_signal = signal::of(move |t| {
//!     let amp = amplitude.at(t);
//!     let freq = frequency.at(t);
//!     (0..100).map(|i| {
//!         let x = i as f64 * 0.1;
//!         amp * (x * freq).sin()
//!     }).collect::<Vec<f64>>()
//! });
//!
//! // Use a signal-backed title alongside the sampled series data
//! let title = signal::of(|t| format!("Wave Animation - t={:.2}s", t));
//!
//! record!("wave.gif", 3 secs, |t| {
//!     let y = y_signal.at(t);
//!     Plot::new()
//!         .title(title.at(t))
//!         .line(&x, &y)
//! })?;
//! ```
//!
//! ### Reactive Labels
//!
//! Attach signal-backed titles and axis labels:
//!
//! ```rust,ignore
//! use ruviz::prelude::*;
//! use ruviz::animation::signal;
//!
//! // Dynamic title showing current time
//! let title = signal::of(|t| format!("Simulation: {:.1}s", t));
//!
//! // Dynamic axis label
//! let ylabel = signal::of(|t| {
//!     if t < 1.0 { "Accelerating".to_string() }
//!     else if t < 2.0 { "Constant Velocity".to_string() }
//!     else { "Decelerating".to_string() }
//! });
//!
//! let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
//! let y: Vec<f64> = x.iter().map(|&xi| xi.sin()).collect();
//!
//! let plot = Plot::new()
//!     .title_signal(title)
//!     .xlabel("Time")
//!     .ylabel_signal(ylabel)
//!     .line(&x, &y);
//! ```
//!
//! ### Signal Composition
//!
//! Combine multiple signals for complex animations:
//!
//! ```rust,ignore
//! use ruviz::animation::signal;
//!
//! // Basic signal constructors
//! let constant = signal::constant(42.0);           // Always returns 42
//! let time = signal::time();                       // Returns current time
//! let linear = signal::lerp(0.0, 100.0, 2.0);     // Linear interpolation
//! let eased = signal::ease(easing::ease_out_bounce, 100.0, 0.0, 2.0);
//!
//! // Transform signals
//! let doubled = linear.map(|v| v * 2.0);
//!
//! // Combine two signals
//! let combined = signal::zip(linear.clone(), eased, |a, b| a + b);
//!
//! // Combine three signals
//! let rgb = signal::zip3(
//!     signal::lerp(0.0, 255.0, 1.0),
//!     signal::lerp(255.0, 0.0, 1.0),
//!     signal::constant(128.0),
//!     |r, g, b| (r as u8, g as u8, b as u8)
//! );
//!
//! // Custom signal from closure
//! let sine_wave = signal::of(|t| (t * std::f64::consts::TAU).sin());
//! ```
//!
//! ## Gallery
//!
//! Click any image to view full size. Expand sections below to see code examples.
//!
//! ### Plot Types
//!
//! | | | |
//! |:---:|:---:|:---:|
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/line_plot.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/line_plot.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/scatter_plot.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/scatter_plot.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/bar_chart.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/bar_chart.png" width="250"></a> |
//! | Line Plot | Scatter Plot | Bar Chart |
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/histogram.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/histogram.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/boxplot.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/boxplot.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/heatmap.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/heatmap.png" width="250"></a> |
//! | Histogram | Box Plot | Heatmap |
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/kde_plot.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/kde_plot.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/ecdf_plot.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/ecdf_plot.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/pie_chart.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/pie_chart.png" width="250"></a> |
//! | KDE Plot | ECDF Plot | Pie Chart |
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/errorbar_plot.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/errorbar_plot.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/violin_plot.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/violin_plot.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/contour_plot.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/contour_plot.png" width="250"></a> |
//! | Error Bar | Violin Plot | Contour Plot |
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/polar_plot.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/polar_plot.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/radar_chart.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/radar_chart.png" width="250"></a> | |
//! | Polar Plot | Radar Chart | |
//!
//! ### Additional Plot Types
//!
//! The top-level [`Plot`] builder covers the most common scientific chart types:
//!
//! | Category | Plot Types |
//! |----------|------------|
//! | **Basic** | Line, Scatter, Bar |
//! | **Distribution** | Histogram, Box Plot, Violin, Boxen, KDE, ECDF, Rug, Strip, Swarm |
//! | **Continuous** | Heatmap, Contour, Hexbin, Fill Between, Area |
//! | **Discrete** | Step, Stem |
//! | **Error** | Error Bars |
//! | **Composition** | Pie, Donut |
//! | **Polar** | Polar Plot, Radar/Spider Chart |
//! | **Vector** | Quiver |
//! | **Hierarchical** | Dendrogram |
//! | **3D** (`3d` feature) | Scatter3D, Line3D, Surface3D, Wireframe3D |
//!
//! That is the complete list: 26 types from [`Plot`], 4 more from `Plot3D`.
//! `src/plots/mod.rs::catalog_is_true` reads the builder's own source and fails
//! if this table drifts from the API.
//!
//! All of them except `fill_between` are *series* methods returning
//! [`PlotBuilder<C>`](core::PlotBuilder), so one chain works across every one:
//! `.<series>(..).label(..).color(..).legend_best().save(..)`. `fill_between`
//! is an annotation rather than a series — it returns the plot itself, so it
//! takes plot-level setters instead of series-level ones.
//!
//! [`plots`] also exposes compute helpers (grouped and stacked bar, stacked
//! area, 2D KDE, regression) that have **no** builder method and cannot be
//! drawn — see the [`plots`] module docs. See the
//! [Plot Types Guide](https://github.com/Ameyanagi/ruviz/blob/main/docs/guide/04_plot_types.md)
//! for details.
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
//!     .histogram(&data)
//!     .save("histogram.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! **Box Plot**
//! ```rust,no_run
//! use ruviz::prelude::*;
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
//!     .boxplot(&data)
//!     .show_mean(true)
//!     .save("boxplot.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! **Heatmap**
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! // Create 2D data (distance from center, shifted positive for log scaling)
//! let data: Vec<Vec<f64>> = (0..10).map(|i| {
//!     (0..10).map(|j| {
//!         ((i as f64 - 5.0).powi(2) + (j as f64 - 5.0).powi(2)).sqrt() + 1.0
//!     }).collect()
//! }).collect();
//!
//! Plot::new()
//!     .title("Log-Scaled Heatmap")
//!     .xlabel("X")
//!     .ylabel("Y")
//!     .heatmap(&data)
//!     .value_scale(AxisScale::Log)
//!     .colorbar_label("Distance")
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
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/line_styles.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/line_styles.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/marker_styles.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/marker_styles.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/colors.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/colors.png" width="250"></a> |
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
//!     .label("Solid").line_style(LineStyle::Solid)
//!     .line(&x, &x.iter().map(|&v| v.sin() + 3.0).collect::<Vec<_>>())
//!     .label("Dashed").line_style(LineStyle::Dashed)
//!     .line(&x, &x.iter().map(|&v| v.sin() + 2.0).collect::<Vec<_>>())
//!     .label("Dotted").line_style(LineStyle::Dotted)
//!     .line(&x, &x.iter().map(|&v| v.sin() + 1.0).collect::<Vec<_>>())
//!     .label("DashDot").line_style(LineStyle::DashDot)
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
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/theme_default.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/theme_default.png" width="200"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/theme_dark.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/theme_dark.png" width="200"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/theme_seaborn.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/theme_seaborn.png" width="200"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/theme_publication.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/theme_publication.png" width="200"></a> |
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
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/legend_positions.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/legend_positions.png" width="350"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/subplots.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/subplots.png" width="350"></a> |
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
//! ### Internationalization
//!
//! | | | |
//! |:---:|:---:|:---:|
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/international_japanese.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/international_japanese.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/international_chinese.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/international_chinese.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/international_korean.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/international_korean.png" width="250"></a> |
//! | 日本語 (Japanese) | 中文 (Chinese) | 한국어 (Korean) |
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/international_comparison.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/international_comparison.png" width="350"></a> | | |
//! | Multi-language Comparison | | |
//!
//! <details>
//! <summary>Internationalization Code Examples</summary>
//!
//! **Japanese Labels**
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
//! let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
//!
//! Plot::new()
//!     .title("サイン波 (Sine Wave)")
//!     .xlabel("時間 (s)")
//!     .ylabel("振幅")
//!     .line(&x, &y)
//!     .label("sin(x)")
//!     .legend_best()
//!     .save("japanese_plot.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! **Chinese Labels**
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! let categories = vec!["一月", "二月", "三月", "四月", "五月", "六月"];
//! let values = vec![28.0, 45.0, 38.0, 52.0, 47.0, 63.0];
//!
//! Plot::new()
//!     .title("月度销售数据")
//!     .xlabel("月份")
//!     .ylabel("销售额 (万元)")
//!     .bar(&categories, &values)
//!     .save("chinese_plot.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! </details>
//!
//! ### Animation
//!
//! Smooth animations with the `record!` macro (requires `animation` feature):
//!
//! | | | |
//! |:---:|:---:|:---:|
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/animation_sine_wave.gif"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/animation_sine_wave.gif" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/animation_bars.gif"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/animation_bars.gif" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/animation_spiral.gif"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/animation_spiral.gif" width="250"></a> |
//! | Traveling Wave | Animated Bars | Spiral Growth |
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/animation_easing.gif"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/animation_easing.gif" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/animation_interference.gif"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/animation_interference.gif" width="250"></a> | |
//! | Easing Functions | Wave Interference | |
//!
//! See [Animation Gallery](https://github.com/Ameyanagi/ruviz/blob/main/docs/gallery/animation/README.md) for more examples.
//!
//! ## Common Tasks
//!
//! Each of these is the same chain from [Quick Start](#quick-start) with one
//! extra setter.
//!
//! ### Axis Labels
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
//!     .ylabel("y = x^2")
//!     .save("plot.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### With Legend (matplotlib-style)
//!
//! Label each series, then enable the legend with `legend_best()` (the
//! equivalent of `plt.legend()`) or `legend_position(..)` to place it yourself.
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
//!     .legend_best()
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

#[cfg(target_os = "freebsd")]
fn setup_freebsd_fontconfig() {
    use std::env;

    // Set FONTCONFIG_FILE if not already set
    if env::var("FONTCONFIG_FILE").is_err() {
        let fontconfig_path = "/usr/local/etc/fonts/fonts.conf";
        if std::path::Path::new(fontconfig_path).exists() {
            // SAFETY: This runs in a process constructor before worker threads start.
            // Setting process env vars at that point avoids concurrent env mutation.
            unsafe {
                env::set_var("FONTCONFIG_FILE", fontconfig_path);
            }
        }
    }
}

// Call it using ctor to run before any cosmic-text initialization
#[cfg(target_os = "freebsd")]
#[ctor::ctor]
fn init_freebsd_fonts() {
    setup_freebsd_fontconfig();
}

pub mod axes;
pub mod core;
pub mod data;
pub mod export;
pub mod layout;
pub mod plots;
pub mod render;
#[cfg(not(target_arch = "wasm32"))]
pub mod simple;
pub mod stats;
pub mod style;
pub mod text;

#[cfg(all(feature = "interactive", not(target_arch = "wasm32")))]
pub mod interactive;

#[cfg(all(feature = "animation", not(target_arch = "wasm32")))]
pub mod animation;

/// Convenience re-exports for common usage
///
/// # What is deliberately *not* here
///
/// - [`crate::core::Result`] — a one-parameter alias. Glob-importing it shadows
///   [`std::result::Result`], so every ordinary `Result<T, E>` in the importing
///   scope fails with `E0107`. Use [`PlotResult`](crate::core::PlotResult), which
///   is exported here, or import `ruviz::core::Result` explicitly.
/// - `PlotInput` / `SeriesStyle` — internal representations of a half-built
///   series. They are crate-internal so that refactoring them is not a breaking
///   change.
pub mod prelude {
    pub use crate::axes::AxisScale;
    // `Position` is deprecated in favour of `LegendPosition` but stays in the
    // prelude so existing `use ruviz::prelude::*` code keeps resolving it.
    #[allow(deprecated)]
    pub use crate::core::{
        Annotation, AnnotationId, ArrowHead, ArrowStyle, BackendType, BuilderWhen, FillStyle,
        FramePacing, FrameStats, GridSpec, HatchPattern, HitResult, Image, ImageTarget,
        InsetAnchor, InsetLayout, InteractiveFrame, InteractivePlotSession,
        InteractiveViewportSnapshot, IntoPlot, LayerRenderState, Legend, LegendAnchor, LegendItem,
        LegendItemType, LegendPosition, Plot, PlotBuilder, PlotInputEvent, PlotResult, PlotSource,
        PlottingError, Position, PreparedPlot, QualityPolicy, ReactiveSubscription, ReactiveValue,
        RenderTargetKind, ShapeStyle, SubplotFigure, SurfaceCapability, SurfaceTarget, TextAlign,
        TextStyle, TextVAlign, TickDirection, TickSides, ViewportPoint, ViewportRect, subplots,
        subplots_default,
    };
    #[cfg(feature = "3d")]
    pub use crate::core::{
        AxisAspect3D, Bounds3D, Camera3D, CameraSnapshot3D, InputEvent3D, InteractionResult3D,
        InteractivePlot3DSession, Line3DBuilder, PickHit3D, PickPrimitive3D, Point3D,
        PointerButton3D, ProjectedPoint3D, Projection3D, RenderDiagnostics3D, Scatter3DBuilder,
        ScreenRay3D, Surface3DBuilder, Wireframe3DBuilder, release_3d_gpu_resources,
    };
    pub use crate::data::{
        Data1D, DataShader, DataShaderCanvas, NullPolicy, NumericData1D, NumericData2D,
    };
    #[allow(deprecated)]
    pub use crate::plots::{
        BandwidthMethod, BinMethod, BoxPlotConfig, BoxenConfig, BoxenOrientation, ContourConfig,
        ContourInterpolation, EcdfConfig, EcdfStat, HeatmapConfig, HeatmapOrigin, HistogramConfig,
        Interpolation, KdeConfig, OutlierMethod, PieConfig, PlotArea, PlotCompute, PlotConfig,
        PlotData, PlotRender, PolarPlotConfig, QuiverConfig, QuiverPivot, RadarConfig, StemMarker,
        StemOrientation, StepWhere, ViolinConfig, WhiskerMethod,
    };
    // Enum arguments to prelude-exported setters must themselves be nameable from the
    // prelude, or `.orientation(BoxOrientation::Vertical)` is E0433 and rustc suggests
    // the wrong sibling type. These two are not re-exported from `crate::plots`.
    pub use crate::plots::boxplot::BoxOrientation;
    pub use crate::plots::distribution::ViolinScale;
    #[cfg(feature = "3d")]
    pub use crate::plots::{
        Line3DConfig, Scatter3DConfig, Surface3DConfig, SurfaceSampling, SurfaceShading,
        Wireframe3DConfig,
    };
    pub use crate::render::{
        Color, ColorMap, ColorMapSpec, FontConfig, FontFamily, FontStyle, FontWeight, LineStyle,
        MarkerStyle, Theme,
    };

    // Deprecated 2D shortcuts, kept re-exported so existing `use
    // ruviz::prelude::*` code still resolves them and gets the migration note.
    // Use `Plot::new().line(..)` / `.scatter(..)` / `.bar(..)` instead.
    #[allow(deprecated)]
    pub use crate::{bar, line, scatter};
    // 3D entry points are *not* deprecated: there is no `Plot3D` builder, so
    // these free functions are the one obvious way to start a 3D plot.
    #[cfg(feature = "3d")]
    pub use crate::{line3d, scatter3d, surface, wireframe};

    #[cfg(all(feature = "interactive", not(target_arch = "wasm32")))]
    pub use crate::interactive::{
        event::{InteractionEvent, Point2D, Rectangle, Vector2D},
        renderer::RealTimeRenderer,
        state::InteractionState,
        window::{
            InteractiveContextMenuActionContext, InteractiveContextMenuConfig,
            InteractiveContextMenuItem, InteractiveWindow, InteractiveWindowBuilder,
            show_interactive,
        },
    };

    #[cfg(all(feature = "animation", not(target_arch = "wasm32")))]
    #[allow(deprecated)]
    pub use crate::animation::{
        DurationExt, RecordConfig, Signal, Tick, easing, record_plot, record_simple, signal,
    };
}

// =============================================================================
// Top-Level Convenience Functions
// =============================================================================

use core::{Plot, PlotBuilder};
use data::NumericData1D;
use plots::{BarConfig, LineConfig, ScatterConfig};

#[cfg(feature = "3d")]
use core::{Line3DBuilder, Scatter3DBuilder, Surface3DBuilder, Wireframe3DBuilder};
#[cfg(feature = "3d")]
use data::NumericData2D;

/// Create a 3D scatter plot from x, y, and z coordinates.
///
/// Enable it with `ruviz = { version = "...", features = ["3d"] }`.
#[cfg(feature = "3d")]
pub fn scatter3d<X, Y, Z>(x: &X, y: &Y, z: &Z) -> Scatter3DBuilder
where
    X: NumericData1D + ?Sized,
    Y: NumericData1D + ?Sized,
    Z: NumericData1D + ?Sized,
{
    Scatter3DBuilder::from_data(x, y, z)
}

/// Create a 3D line plot from x, y, and z coordinates.
///
/// Enable it with `ruviz = { version = "...", features = ["3d"] }`.
#[cfg(feature = "3d")]
pub fn line3d<X, Y, Z>(x: &X, y: &Y, z: &Z) -> Line3DBuilder
where
    X: NumericData1D + ?Sized,
    Y: NumericData1D + ?Sized,
    Z: NumericData1D + ?Sized,
{
    Line3DBuilder::from_data(x, y, z)
}

/// Create a regular-grid surface where `z.shape() == (y.len(), x.len())`.
///
/// Enable it with `ruviz = { version = "...", features = ["3d"] }`.
#[cfg(feature = "3d")]
pub fn surface<X, Y, Z>(x: &X, y: &Y, z: &Z) -> Surface3DBuilder
where
    X: NumericData1D + ?Sized,
    Y: NumericData1D + ?Sized,
    Z: NumericData2D + ?Sized,
{
    Surface3DBuilder::from_data(x, y, z)
}

/// Create a regular-grid wireframe where `z.shape() == (y.len(), x.len())`.
///
/// Enable it with `ruviz = { version = "...", features = ["3d"] }`.
#[cfg(feature = "3d")]
pub fn wireframe<X, Y, Z>(x: &X, y: &Y, z: &Z) -> Wireframe3DBuilder
where
    X: NumericData1D + ?Sized,
    Y: NumericData1D + ?Sized,
    Z: NumericData2D + ?Sized,
{
    Wireframe3DBuilder::from_data(x, y, z)
}

/// Create a line plot with the given data.
///
/// # Deprecated
///
/// This is a second spelling of `Plot::new().line(x, y)` that returns the exact
/// same [`PlotBuilder`]. Write the builder chain instead — it is the one entry
/// point that works for all 21 plot types, whereas only three of them ever had
/// a free function:
///
/// ```rust,no_run
/// use ruviz::prelude::*;
///
/// let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
/// let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
///
/// Plot::new()
///     .line(&x, &y)
///     .title("Sine Wave")
///     .xlabel("x")
///     .ylabel("sin(x)")
///     .save("sine.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[deprecated(
    since = "0.6.0",
    note = "use the Plot builder: Plot::new().line(x, y) — identical return type, and the same entry point every other plot type uses"
)]
pub fn line<X, Y>(x: &X, y: &Y) -> PlotBuilder<LineConfig>
where
    X: NumericData1D,
    Y: NumericData1D,
{
    Plot::new().line(x, y)
}

/// Create a scatter plot with the given data.
///
/// # Deprecated
///
/// This is a second spelling of `Plot::new().scatter(x, y)` that returns the
/// exact same [`PlotBuilder`]. Write the builder chain instead:
///
/// ```rust,no_run
/// use ruviz::prelude::*;
///
/// let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let y = vec![2.0, 4.0, 1.0, 5.0, 3.0];
///
/// Plot::new()
///     .scatter(&x, &y)
///     .title("Scatter Plot")
///     .marker(MarkerStyle::Circle)
///     .save("scatter.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[deprecated(
    since = "0.6.0",
    note = "use the Plot builder: Plot::new().scatter(x, y) — identical return type, and the same entry point every other plot type uses"
)]
pub fn scatter<X, Y>(x: &X, y: &Y) -> PlotBuilder<ScatterConfig>
where
    X: NumericData1D,
    Y: NumericData1D,
{
    Plot::new().scatter(x, y)
}

/// Create a bar plot with the given categories and values.
///
/// # Deprecated
///
/// This is a second spelling of `Plot::new().bar(categories, values)` that
/// returns the exact same [`PlotBuilder`]. Write the builder chain instead:
///
/// ```rust,no_run
/// use ruviz::prelude::*;
///
/// let categories = vec!["A", "B", "C", "D"];
/// let values = vec![10.0, 25.0, 15.0, 30.0];
///
/// Plot::new()
///     .bar(&categories, &values)
///     .title("Bar Chart")
///     .ylabel("Count")
///     .save("bar.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[deprecated(
    since = "0.6.0",
    note = "use the Plot builder: Plot::new().bar(categories, values) — identical return type, and the same entry point every other plot type uses"
)]
pub fn bar<S, V>(categories: &[S], values: &V) -> PlotBuilder<BarConfig>
where
    S: ToString,
    V: NumericData1D,
{
    Plot::new().bar(categories, values)
}

#[cfg(test)]
mod prelude_contract_tests {
    //! Pins the two properties the prelude has to keep: it must not shadow
    //! `std::result::Result`, and every config type named in a public signature
    //! must be reachable from it.
    use crate::prelude::*;

    /// If `crate::core::Result` (one generic parameter) ever re-enters the
    /// prelude glob, this fails to compile with `E0107`.
    fn two_parameter_result_still_resolves() -> Result<u32, std::num::ParseIntError> {
        "42".parse()
    }

    /// The non-colliding alias the prelude offers instead.
    fn plot_result_is_exported() -> PlotResult<()> {
        Ok(())
    }

    /// Every family config reachable from the prelude alone. The builder's own
    /// setters (`.histogram(&d).bins(30)`) are the primary way to configure a
    /// series, but `histogram_with`/`boxplot_with`/`heatmap_with` take a config
    /// by value, so a user whose only import is the prelude must be able to
    /// name them.
    fn family_configs_resolve(
        _histogram: HistogramConfig,
        _bin: BinMethod,
        _boxplot: BoxPlotConfig,
        _outlier: OutlierMethod,
        _whisker: WhiskerMethod,
        _kde: KdeConfig,
        _ecdf: EcdfConfig,
        _ecdf_stat: EcdfStat,
        _bandwidth: BandwidthMethod,
    ) {
    }

    /// The error type itself, not just its alias, has to be nameable so callers
    /// can `match` on what `PlotResult` returns.
    fn error_type_is_exported(err: PlottingError) -> &'static str {
        match err {
            PlottingError::EmptyDataSet => "empty",
            _ => "other",
        }
    }

    #[test]
    fn prelude_contract_holds() {
        assert_eq!(two_parameter_result_still_resolves().unwrap(), 42);
        assert!(plot_result_is_exported().is_ok());
        assert_eq!(error_type_is_exported(PlottingError::EmptyDataSet), "empty");
        family_configs_resolve(
            HistogramConfig::default(),
            BinMethod::Uniform,
            BoxPlotConfig::default(),
            OutlierMethod::IQR,
            WhiskerMethod::Tukey,
            KdeConfig::default(),
            EcdfConfig::default(),
            EcdfStat::Proportion,
            BandwidthMethod::Scott,
        );
    }
}

#[cfg(test)]
mod one_obvious_way_tests {
    //! Pins the "one obvious way" property: the deprecated 2D free functions are
    //! literally the builder chain, so the migration named in their
    //! `#[deprecated]` notes is a textual change with no type churn.
    use crate::plots::{BarConfig, LineConfig, ScatterConfig};
    use crate::prelude::*;

    #[test]
    #[allow(deprecated)]
    fn free_functions_return_the_same_builder_as_the_canonical_chain() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![0.0, 1.0, 4.0];
        let categories = ["A", "B", "C"];
        let values = vec![1.0, 2.0, 3.0];

        let _deprecated: PlotBuilder<LineConfig> = crate::line(&x, &y);
        let _canonical: PlotBuilder<LineConfig> = Plot::new().line(&x, &y);

        let _deprecated: PlotBuilder<ScatterConfig> = crate::scatter(&x, &y);
        let _canonical: PlotBuilder<ScatterConfig> = Plot::new().scatter(&x, &y);

        let _deprecated: PlotBuilder<BarConfig> = crate::bar(&categories, &values);
        let _canonical: PlotBuilder<BarConfig> = Plot::new().bar(&categories, &values);
    }

    /// `save`/`render` finalize the pending series, so the `.end_series()` the
    /// crate docs used to show before every `.save()` was pure noise. Both
    /// spellings have to keep producing the same image; the short one is the
    /// documented one.
    #[test]
    // `end_series` is itself the deprecated item under test here: the point of
    // the test is that the deprecated spelling stays pixel-identical to the
    // documented one, so it cannot be written without calling it.
    #[allow(deprecated)]
    fn render_does_not_need_an_explicit_end_series() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![0.0, 1.0, 4.0];

        let with_end_series = Plot::new()
            .size_px(200, 150)
            .line(&x, &y)
            .end_series()
            .render()
            .expect("explicit end_series should render");
        let without_end_series = Plot::new()
            .size_px(200, 150)
            .line(&x, &y)
            .render()
            .expect("implicit finalize should render");

        assert_eq!(
            (with_end_series.width, with_end_series.height),
            (without_end_series.width, without_end_series.height)
        );
        assert_eq!(
            with_end_series.pixels, without_end_series.pixels,
            "`.end_series()` before a terminal call must be a no-op"
        );
    }
}
