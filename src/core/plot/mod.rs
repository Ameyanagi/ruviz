//! Core Plot implementation and types
//!
//! This module provides the main [`Plot`] struct and related types for creating
//! visualizations in ruviz.
//!
//! # Architecture
//!
//! The `Plot` struct is decomposed into focused component managers:
//!
//! - [`PlotConfiguration`] - Display settings (title, labels, dimensions, theme)
//! - [`SeriesManager`] - Data series storage and auto-coloring
//! - [`LayoutManager`] - Legend, grid, ticks, margins, axis limits/scales
//! - [`RenderPipeline`] - Backend selection, parallel/pooled rendering
//!
//! # Usage
//!
//! ```rust,ignore
//! use ruviz::prelude::*;
//!
//! // Simple plot
//! Plot::new()
//!     .line(&x, &y)
//!     .title("My Plot")
//!     .save("plot.png")?;
//!
//! // Multi-series with styling
//! Plot::new()
//!     .line(&x, &y1)
//!     .color(Color::RED)
//!     .label("Series 1")
//!     .line(&x, &y2)
//!     .color(Color::BLUE)
//!     .label("Series 2")
//!     .legend(Position::TopRight)
//!     .save("multi.png")?;
//! ```
//!
//! # Builder Pattern
//!
//! Series methods return [`PlotBuilder<C>`] which provides:
//! - Series-specific configuration (color, line_width, markers)
//! - Plot-level methods forwarded to inner Plot (title, xlabel, theme)
//! - Terminal methods (save, render) that auto-finalize series
//!
//! See [`PlotBuilder`] for details on the generic builder implementation.

macro_rules! impl_series_continuation_methods {
    ($self_:ident.$finalize:ident()) => {
        /// Continue with a new line series.
        pub fn line<X, Y>(
            $self_,
            x_data: &X,
            y_data: &Y,
        ) -> $crate::core::plot::PlotBuilder<$crate::plots::basic::LineConfig>
        where
            X: $crate::data::NumericData1D,
            Y: $crate::data::NumericData1D,
        {
            $self_.$finalize().line(x_data, y_data)
        }

        /// Continue with a new line series from source-backed data.
        pub fn line_source<X, Y>(
            $self_,
            x_data: X,
            y_data: Y,
        ) -> $crate::core::plot::PlotBuilder<$crate::plots::basic::LineConfig>
        where
            X: $crate::core::plot::IntoPlotData,
            Y: $crate::core::plot::IntoPlotData,
        {
            $self_.$finalize().line_source(x_data, y_data)
        }

        /// Continue with a new scatter series.
        pub fn scatter<X, Y>(
            $self_,
            x_data: &X,
            y_data: &Y,
        ) -> $crate::core::plot::PlotBuilder<$crate::plots::basic::ScatterConfig>
        where
            X: $crate::data::NumericData1D,
            Y: $crate::data::NumericData1D,
        {
            $self_.$finalize().scatter(x_data, y_data)
        }

        /// Continue with a new scatter series from source-backed data.
        pub fn scatter_source<X, Y>(
            $self_,
            x_data: X,
            y_data: Y,
        ) -> $crate::core::plot::PlotBuilder<$crate::plots::basic::ScatterConfig>
        where
            X: $crate::core::plot::IntoPlotData,
            Y: $crate::core::plot::IntoPlotData,
        {
            $self_.$finalize().scatter_source(x_data, y_data)
        }

        /// Continue with a new bar series.
        pub fn bar<S, V>(
            $self_,
            categories: &[S],
            values: &V,
        ) -> $crate::core::plot::PlotBuilder<$crate::plots::basic::BarConfig>
        where
            S: ToString,
            V: $crate::data::NumericData1D,
        {
            $self_.$finalize().bar(categories, values)
        }

        /// Continue with a new bar series from source-backed values.
        pub fn bar_source<S, V>(
            $self_,
            categories: &[S],
            values: V,
        ) -> $crate::core::plot::PlotBuilder<$crate::plots::basic::BarConfig>
        where
            S: ToString,
            V: $crate::core::plot::IntoPlotData,
        {
            $self_.$finalize().bar_source(categories, values)
        }

        /// Continue with a grouped-series scope after finalizing the current series.
        pub fn group<F>($self_, f: F) -> $crate::core::plot::Plot
        where
            F: FnOnce(
                $crate::core::plot::SeriesGroupBuilder,
            ) -> $crate::core::plot::SeriesGroupBuilder,
        {
            $self_.$finalize().group(f)
        }

        /// Continue with a histogram series.
        pub fn histogram<D: $crate::data::NumericData1D>(
            $self_,
            data: &D,
            config: Option<$crate::plots::HistogramConfig>,
        ) -> $crate::core::plot::PlotSeriesBuilder {
            $self_.$finalize().histogram(data, config)
        }

        /// Continue with a histogram series from source-backed values.
        pub fn histogram_source<D: $crate::core::plot::IntoPlotData>(
            $self_,
            data: D,
            config: Option<$crate::plots::HistogramConfig>,
        ) -> $crate::core::plot::PlotSeriesBuilder {
            $self_.$finalize().histogram_source(data, config)
        }

        /// Continue with a box plot series.
        pub fn boxplot<D: $crate::data::NumericData1D>(
            $self_,
            data: &D,
            config: Option<$crate::plots::BoxPlotConfig>,
        ) -> $crate::core::plot::PlotSeriesBuilder {
            $self_.$finalize().boxplot(data, config)
        }

        /// Continue with a box plot series from source-backed values.
        pub fn boxplot_source<D: $crate::core::plot::IntoPlotData>(
            $self_,
            data: D,
            config: Option<$crate::plots::BoxPlotConfig>,
        ) -> $crate::core::plot::PlotSeriesBuilder {
            $self_.$finalize().boxplot_source(data, config)
        }

        /// Continue with a heatmap series.
        pub fn heatmap<D>(
            $self_,
            data: &D,
            config: Option<$crate::plots::heatmap::HeatmapConfig>,
        ) -> $crate::core::plot::PlotSeriesBuilder
        where
            D: $crate::data::NumericData2D + ?Sized,
        {
            $self_.$finalize().heatmap(data, config)
        }

        /// Continue with a KDE series.
        pub fn kde<T, D: $crate::data::Data1D<T>>(
            $self_,
            data: &D,
        ) -> $crate::core::plot::PlotBuilder<$crate::plots::KdeConfig>
        where
            T: Into<f64> + Copy,
        {
            $self_.$finalize().kde(data)
        }

        /// Continue with an ECDF series.
        pub fn ecdf<T, D: $crate::data::Data1D<T>>(
            $self_,
            data: &D,
        ) -> $crate::core::plot::PlotBuilder<$crate::plots::EcdfConfig>
        where
            T: Into<f64> + Copy,
        {
            $self_.$finalize().ecdf(data)
        }

        /// Continue with a contour series.
        pub fn contour<X, Y, Z>(
            $self_,
            x: &X,
            y: &Y,
            z: &Z,
        ) -> $crate::core::plot::PlotBuilder<$crate::plots::ContourConfig>
        where
            X: $crate::data::Data1D<f64>,
            Y: $crate::data::Data1D<f64>,
            Z: $crate::data::Data1D<f64>,
        {
            $self_.$finalize().contour(x, y, z)
        }

        /// Continue with a pie series.
        pub fn pie<V>(
            $self_,
            values: &V,
        ) -> $crate::core::plot::PlotBuilder<$crate::plots::PieConfig>
        where
            V: $crate::data::Data1D<f64>,
        {
            $self_.$finalize().pie(values)
        }

        /// Continue with a radar series.
        pub fn radar<S: AsRef<str>>(
            $self_,
            labels: &[S],
        ) -> $crate::core::plot::PlotBuilder<$crate::plots::RadarConfig> {
            $self_.$finalize().radar(labels)
        }

        /// Continue with a polar line series.
        pub fn polar_line<R, T>(
            $self_,
            r: &R,
            theta: &T,
        ) -> $crate::core::plot::PlotBuilder<$crate::plots::PolarPlotConfig>
        where
            R: $crate::data::Data1D<f64>,
            T: $crate::data::Data1D<f64>,
        {
            $self_.$finalize().polar_line(r, theta)
        }

        /// Continue with a violin series.
        pub fn violin<T, D: $crate::data::Data1D<T>>(
            $self_,
            data: &D,
        ) -> $crate::core::plot::PlotBuilder<$crate::plots::ViolinConfig>
        where
            T: Into<f64> + Copy,
        {
            $self_.$finalize().violin(data)
        }

        /// Continue with a new streaming line series.
        pub fn line_streaming(
            $self_,
            stream: &$crate::data::StreamingXY,
        ) -> $crate::core::plot::PlotSeriesBuilder {
            $self_.$finalize().line_streaming(stream)
        }

        /// Continue with a new streaming scatter series.
        pub fn scatter_streaming(
            $self_,
            stream: &$crate::data::StreamingXY,
        ) -> $crate::core::plot::PlotSeriesBuilder {
            $self_.$finalize().scatter_streaming(stream)
        }

        /// Continue with a new error bar series (Y errors only).
        pub fn error_bars<X, Y, E>(
            $self_,
            x_data: &X,
            y_data: &Y,
            y_errors: &E,
        ) -> $crate::core::plot::PlotSeriesBuilder
        where
            X: $crate::data::NumericData1D,
            Y: $crate::data::NumericData1D,
            E: $crate::data::NumericData1D,
        {
            $self_.$finalize().error_bars(x_data, y_data, y_errors)
        }

        /// Continue with a new Y-error-bar series from source-backed data.
        pub fn error_bars_source<X, Y, E>(
            $self_,
            x_data: X,
            y_data: Y,
            y_errors: E,
        ) -> $crate::core::plot::PlotSeriesBuilder
        where
            X: $crate::core::plot::IntoPlotData,
            Y: $crate::core::plot::IntoPlotData,
            E: $crate::core::plot::IntoPlotData,
        {
            $self_.$finalize().error_bars_source(x_data, y_data, y_errors)
        }

        /// Continue with a new error bar series (both X and Y errors).
        pub fn error_bars_xy<X, Y, EX, EY>(
            $self_,
            x_data: &X,
            y_data: &Y,
            x_errors: &EX,
            y_errors: &EY,
        ) -> $crate::core::plot::PlotSeriesBuilder
        where
            X: $crate::data::NumericData1D,
            Y: $crate::data::NumericData1D,
            EX: $crate::data::NumericData1D,
            EY: $crate::data::NumericData1D,
        {
            $self_.$finalize().error_bars_xy(x_data, y_data, x_errors, y_errors)
        }

        /// Continue with a new X/Y error-bar series from source-backed data.
        pub fn error_bars_xy_source<X, Y, EX, EY>(
            $self_,
            x_data: X,
            y_data: Y,
            x_errors: EX,
            y_errors: EY,
        ) -> $crate::core::plot::PlotSeriesBuilder
        where
            X: $crate::core::plot::IntoPlotData,
            Y: $crate::core::plot::IntoPlotData,
            EX: $crate::core::plot::IntoPlotData,
            EY: $crate::core::plot::IntoPlotData,
        {
            $self_
                .$finalize()
                .error_bars_xy_source(x_data, y_data, x_errors, y_errors)
        }
    };
}

mod annotations;
mod builder;
mod config;
mod configuration;
mod construction;
pub mod data;
mod image;
mod interactive_session;
mod layout_manager;
mod mixed_render;
mod parallel_render;
mod prepared;
mod render;
mod render_pipeline;
mod series_api;
mod series_builders;
mod series_internal;
mod series_manager;
#[cfg(test)]
#[allow(deprecated)]
mod tests;
mod types;

pub use builder::{IntoPlot, PlotBuilder, PlotInput, SeriesStyle};
pub use config::{BackendType, GridMode, TickDirection, TickSides};
pub use configuration::{PlotConfiguration, TextEngineMode};
pub use data::{IntoPlotData, PlotData, PlotSource, PlotText, ReactiveValue};
pub use image::Image;
pub use interactive_session::{
    DirtyDomain, DirtyDomains, FramePacing, FrameStats, HitResult, ImageTarget, InteractiveFrame,
    InteractivePlotSession, InteractiveViewportSnapshot, LayerRenderState, PlotInputEvent,
    QualityPolicy, RenderTargetKind, SurfaceCapability, SurfaceTarget, ViewportPoint, ViewportRect,
};
pub use layout_manager::LayoutManager;
pub use prepared::{PreparedPlot, ReactiveSubscription};
pub use render_pipeline::RenderPipeline;
pub use series_builders::{PlotSeriesBuilder, SeriesGroupBuilder};
pub use series_manager::SeriesManager;
pub use types::{InsetAnchor, InsetLayout, Plot};

use crate::{
    axes::AxisScale,
    core::{
        Annotation, ArrowStyle, FillStyle, GridStyle, LayoutCalculator, LayoutConfig, Legend,
        LegendItem, LegendItemType, LegendPosition, MarginConfig, MeasuredDimensions, PlotConfig,
        PlotContent, PlotLayout, PlotStyle, PlottingError, Position, REFERENCE_DPI, RenderScale,
        Result, ShapeStyle, TextStyle, pt_to_px,
    },
    data::{
        Data1D, DataShader, NullPolicy, NumericData1D, NumericData2D, StreamingXY,
        collect_numeric_data_1d, collect_numeric_data_2d,
    },
    plots::boxplot::BoxPlotConfig,
    plots::error::errorbar::{ErrorBarConfig, ErrorValues},
    plots::histogram::HistogramConfig,
    plots::traits::PlotRender,
    render::skia::{
        SkiaRenderer, calculate_plot_area_config, calculate_plot_area_dpi, generate_ticks,
        map_data_to_pixels,
    },
    render::{Color, LineStyle, MarkerStyle, Theme},
};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    path::Path,
    sync::Arc,
};

use self::data::{ReactiveTeardown, SharedReactiveCallback};
pub(crate) use self::types::{
    LegendConfig, PendingIngestionError, PlotSeries, ResolvedSeries, SeriesGroupMeta, SeriesType,
    TickConfig,
};

#[cfg(feature = "parallel")]
use crate::render::{ParallelRenderer, SeriesRenderData};

#[cfg(feature = "gpu")]
use crate::render::gpu::GpuRenderer;
