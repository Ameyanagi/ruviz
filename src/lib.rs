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
//! - **30+ Plot Types**: Distribution, categorical, polar, regression, composite plots
//! - **Publication Quality**: PNG/SVG export with custom themes
//! - **Large Dataset Support**: DataShader-style aggregation for 100M+ points
//! - **Cross Platform**: Linux, macOS, Windows, WASM support
//! - **Reactive Animation**: Signal-based animation with time-varying data
//!
//! ## Quick Start
//!
//! Create plots with minimal boilerplate using top-level convenience functions:
//!
//! ```rust,no_run
//! use ruviz::prelude::*;
//!
//! // Line plot - one line of code!
//! let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
//! let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
//! line(&x, &y).title("Sine Wave").save("sine.png")?;
//!
//! // Scatter plot
//! scatter(&x, &y).title("Points").marker(MarkerStyle::Circle).save("scatter.png")?;
//!
//! // Bar chart
//! let cats = ["A", "B", "C", "D"];
//! let vals = [10.0, 25.0, 15.0, 30.0];
//! bar(&cats, &vals).title("Sales").save("bar.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Reactive Animation (New!)
//!
//! Create smooth animations with Signal-based reactive data. Define your animation
//! once and render at any time point.
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
//!     line(&x, &y).title(format!("t = {:.2}s", t.time))
//! })?;
//!
//! // Duration-based animation (2 seconds at 30fps)
//! record!("bounce.gif", 2 secs, |t| {
//!     let y = t.ease_over(easing::ease_out_bounce, 100.0, 0.0, 2.0);
//!     scatter(&[0.0], &[y]).title("Bouncing Ball")
//! })?;
//!
//! // Custom framerate
//! record!("smooth.gif", 3 secs @ 60 fps, |t| {
//!     let x = t.lerp_over(0.0, 10.0, 3.0);
//!     line(&[0.0, x], &[0.0, x]).title("Growing Line")
//! })?;
//! ```
//!
//! ### Signal-Based Reactive Plots
//!
//! Use `Signal<T>` for pull-based animation values that are evaluated at render time:
//!
//! ```rust,ignore
//! use ruviz::prelude::*;
//! use ruviz::animation::signal;
//!
//! // Create signals that vary over time
//! let amplitude = signal::lerp(0.0, 2.0, 3.0);  // 0 to 2 over 3 seconds
//! let frequency = signal::ease(easing::ease_in_out_quad, 1.0, 5.0, 3.0);
//!
//! // Compose signals
//! let y_data = signal::of(move |t| {
//!     let amp = amplitude.at(t);
//!     let freq = frequency.at(t);
//!     (0..100).map(|i| {
//!         let x = i as f64 * 0.1;
//!         amp * (x * freq).sin()
//!     }).collect::<Vec<f64>>()
//! });
//!
//! // Use with reactive title
//! let title = signal::of(|t| format!("Wave Animation - t={:.2}s", t));
//!
//! // Create plot with reactive data (evaluated at render time)
//! let plot = Plot::new()
//!     .title_signal(title)
//!     .line_signal(&x_data, y_data);
//!
//! // Record using reactive plot
//! record!("reactive.gif", &plot, 3 secs)?;
//! ```
//!
//! ### Reactive Labels
//!
//! Make titles and axis labels change during animation:
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
//! Plot::new()
//!     .title_signal(title)
//!     .xlabel("Time")
//!     .ylabel_signal(ylabel)
//!     .line(&x, &y)
//!     .save("dynamic_labels.png")?;
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
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/line_plot.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/line_plot.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/scatter_plot.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/scatter_plot.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/bar_chart.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/bar_chart.png" width="250"></a> |
//! | Line Plot | Scatter Plot | Bar Chart |
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/histogram.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/histogram.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/boxplot.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/boxplot.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/heatmap.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/heatmap.png" width="250"></a> |
//! | Histogram | Box Plot | Heatmap |
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/kde_plot.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/kde_plot.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/ecdf_plot.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/ecdf_plot.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/pie_chart.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/pie_chart.png" width="250"></a> |
//! | KDE Plot | ECDF Plot | Pie Chart |
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/errorbar_plot.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/errorbar_plot.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/violin_plot.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/violin_plot.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/contour_plot.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/contour_plot.png" width="250"></a> |
//! | Error Bar | Violin Plot | Contour Plot |
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/polar_plot.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/polar_plot.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/radar_chart.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/radar_chart.png" width="250"></a> | |
//! | Polar Plot | Radar Chart | |
//!
//! ### Advanced Plot Types (30+ Total)
//!
//! ruviz provides comprehensive plot type coverage for scientific visualization:
//!
//! | Category | Plot Types |
//! |----------|------------|
//! | **Distribution** | Violin, KDE (1D/2D), Boxen, ECDF, Strip, Swarm |
//! | **Categorical** | Grouped Bar, Stacked Bar, Horizontal Bar |
//! | **Composition** | Pie, Donut, Area, Stacked Area |
//! | **Continuous** | Contour, Hexbin, Fill Between |
//! | **Error** | Error Bars (symmetric/asymmetric) |
//! | **Discrete** | Step, Stem |
//! | **Regression** | Regression Plot, Residual Plot |
//! | **Polar** | Polar Plot, Radar/Spider Chart |
//! | **Composite** | Joint Plot, Pair Plot |
//! | **Vector** | Quiver Plot |
//! | **Hierarchical** | Dendrogram |
//!
//! See [`plots`] module and [Plot Types Guide](https://github.com/Ameyanagi/ruviz/blob/main/docs/guide/04_plot_types.md) for details.
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
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/line_styles.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/line_styles.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/marker_styles.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/marker_styles.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/colors.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/colors.png" width="250"></a> |
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
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/theme_default.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/theme_default.png" width="200"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/theme_dark.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/theme_dark.png" width="200"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/theme_seaborn.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/theme_seaborn.png" width="200"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/theme_publication.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/theme_publication.png" width="200"></a> |
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
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/legend_positions.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/legend_positions.png" width="350"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/subplots.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/subplots.png" width="350"></a> |
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
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/international_japanese.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/international_japanese.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/international_chinese.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/international_chinese.png" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/international_korean.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/international_korean.png" width="250"></a> |
//! | 日本語 (Japanese) | 中文 (Chinese) | 한국어 (Korean) |
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/international_comparison.png"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/international_comparison.png" width="350"></a> | | |
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
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/animation_sine_wave.gif"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/animation_sine_wave.gif" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/animation_bars.gif"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/animation_bars.gif" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/animation_spiral.gif"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/animation_spiral.gif" width="250"></a> |
//! | Traveling Wave | Animated Bars | Spiral Growth |
//! | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/animation_easing.gif"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/animation_easing.gif" width="250"></a> | <a href="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/animation_interference.gif"><img src="https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/animation_interference.gif" width="250"></a> | |
//! | Easing Functions | Wave Interference | |
//!
//! See [Animation Gallery](https://github.com/Ameyanagi/ruviz/blob/main/docs/gallery/animation/README.md) for more examples.
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

#[cfg(target_os = "freebsd")]
fn setup_freebsd_fontconfig() {
    use std::env;

    // Set FONTCONFIG_FILE if not already set
    if env::var("FONTCONFIG_FILE").is_err() {
        let fontconfig_path = "/usr/local/etc/fonts/fonts.conf";
        if std::path::Path::new(fontconfig_path).exists() {
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
pub mod simple;
pub mod stats;
pub mod style;
pub mod text;

#[cfg(feature = "interactive")]
pub mod interactive;

#[cfg(feature = "animation")]
pub mod animation;

/// Convenience re-exports for common usage
pub mod prelude {
    pub use crate::axes::AxisScale;
    pub use crate::core::{
        Annotation, ArrowHead, ArrowStyle, BackendType, FillStyle, GridSpec, HatchPattern,
        IntoPlot, Legend, LegendAnchor, LegendItem, LegendItemType, LegendPosition, Plot,
        PlotBuilder, PlotInput, Position, Result, SeriesStyle, ShapeStyle, SubplotFigure,
        TextAlign, TextStyle, TextVAlign, subplots, subplots_default,
    };
    pub use crate::data::{Data1D, DataShader, DataShaderCanvas};
    pub use crate::plots::{
        ContourConfig, HeatmapConfig, Interpolation, PieConfig, PlotArea, PlotCompute, PlotConfig,
        PlotData, PlotRender, PolarPlotConfig, RadarConfig, ViolinConfig,
    };
    pub use crate::render::{
        Color, ColorMap, FontConfig, FontFamily, FontStyle, FontWeight, LineStyle, MarkerStyle,
        Theme,
    };

    // Top-level convenience functions
    pub use crate::{bar, line, scatter};

    #[cfg(feature = "interactive")]
    pub use crate::interactive::{
        event::{InteractionEvent, Point2D, Rectangle, Vector2D},
        renderer::RealTimeRenderer,
        state::InteractionState,
        window::{InteractiveWindow, InteractiveWindowBuilder, show_interactive},
    };

    #[cfg(feature = "animation")]
    #[allow(deprecated)]
    pub use crate::animation::{
        DurationExt, RecordConfig, Signal, Tick, easing, record_plot, record_simple, signal,
    };
}

// =============================================================================
// Top-Level Convenience Functions
// =============================================================================

use core::{Plot, PlotBuilder};
use data::Data1D;
use plots::{BarConfig, LineConfig, ScatterConfig};

/// Create a line plot with the given data.
///
/// This is a convenience function equivalent to `Plot::new().line(x, y)`.
///
/// # Arguments
///
/// * `x` - X-axis data (any type implementing `Data1D<f64>`)
/// * `y` - Y-axis data (any type implementing `Data1D<f64>`)
///
/// # Returns
///
/// A `PlotBuilder` that can be further configured.
///
/// # Example
///
/// ```rust,no_run
/// use ruviz::{line, prelude::*};
///
/// let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
/// let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
///
/// line(&x, &y)
///     .title("Sine Wave")
///     .xlabel("x")
///     .ylabel("sin(x)")
///     .save("sine.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn line<X, Y>(x: &X, y: &Y) -> PlotBuilder<LineConfig>
where
    X: Data1D<f64>,
    Y: Data1D<f64>,
{
    Plot::new().line(x, y)
}

/// Create a scatter plot with the given data.
///
/// This is a convenience function equivalent to `Plot::new().scatter(x, y)`.
///
/// # Arguments
///
/// * `x` - X-axis data (any type implementing `Data1D<f64>`)
/// * `y` - Y-axis data (any type implementing `Data1D<f64>`)
///
/// # Returns
///
/// A `PlotBuilder` that can be further configured.
///
/// # Example
///
/// ```rust,no_run
/// use ruviz::{scatter, prelude::*};
///
/// let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let y = vec![2.0, 4.0, 1.0, 5.0, 3.0];
///
/// scatter(&x, &y)
///     .title("Scatter Plot")
///     .marker(MarkerStyle::Circle)
///     .save("scatter.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn scatter<X, Y>(x: &X, y: &Y) -> PlotBuilder<ScatterConfig>
where
    X: Data1D<f64>,
    Y: Data1D<f64>,
{
    Plot::new().scatter(x, y)
}

/// Create a bar plot with the given categories and values.
///
/// This is a convenience function equivalent to `Plot::new().bar(categories, values)`.
///
/// # Arguments
///
/// * `categories` - Category labels for the bars
/// * `values` - Values for each bar (any type implementing `Data1D<f64>`)
///
/// # Returns
///
/// A `PlotBuilder` that can be further configured.
///
/// # Example
///
/// ```rust,no_run
/// use ruviz::{bar, prelude::*};
///
/// let categories = vec!["A", "B", "C", "D"];
/// let values = vec![10.0, 25.0, 15.0, 30.0];
///
/// bar(&categories, &values)
///     .title("Bar Chart")
///     .ylabel("Count")
///     .save("bar.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn bar<S, V>(categories: &[S], values: &V) -> PlotBuilder<BarConfig>
where
    S: ToString,
    V: Data1D<f64>,
{
    Plot::new().bar(categories, values)
}
