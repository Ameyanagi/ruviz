//! Core Plot implementation and types

mod config;
mod image;

pub use config::{BackendType, GridMode, TickDirection};
pub use image::Image;

use crate::{
    core::{
        LayoutCalculator, LayoutConfig, MarginConfig, PlotConfig, PlotContent, PlotLayout,
        PlotStyle, PlottingError, Position, Result, pt_to_px, REFERENCE_DPI,
    },
    data::{Data1D, DataShader},
    plots::boxplot::BoxPlotConfig,
    plots::histogram::HistogramConfig,
    render::skia::{
        SkiaRenderer, calculate_plot_area_config, calculate_plot_area_dpi, generate_ticks,
        map_data_to_pixels,
    },
    render::{Color, LineStyle, MarkerStyle, Theme},
};
use std::path::Path;

#[cfg(feature = "parallel")]
use crate::render::{ParallelRenderer, SeriesRenderData};

/// Main Plot struct - the core API entry point
///
/// Provides a fluent builder interface for creating plots with multiple data series,
/// styling options, and export capabilities.
#[derive(Clone, Debug)]
pub struct Plot {
    /// Plot title
    title: Option<String>,
    /// X-axis label
    xlabel: Option<String>,
    /// Y-axis label
    ylabel: Option<String>,
    /// Canvas dimensions (width, height) - DEPRECATED: use config.figure instead
    dimensions: (u32, u32),
    /// DPI for high-resolution export - DEPRECATED: use config.figure.dpi instead
    dpi: u32,
    /// Plot theme
    theme: Theme,
    /// DPI-independent plot configuration
    config: PlotConfig,
    /// Data series
    series: Vec<PlotSeries>,
    /// Legend configuration
    legend: LegendConfig,
    /// Grid configuration
    grid: GridConfig,
    /// Tick configuration
    tick_config: TickConfig,
    /// Margin around plot area (fraction of canvas)
    margin: Option<f32>,
    /// Whether to use scientific notation on axes
    scientific_notation: bool,
    /// Auto-generate colors for series without explicit colors
    auto_color_index: usize,
    /// Manual X-axis limits (min, max)
    x_limits: Option<(f64, f64)>,
    /// Manual Y-axis limits (min, max)
    y_limits: Option<(f64, f64)>,
    #[cfg(feature = "parallel")]
    /// Parallel renderer for performance optimization
    parallel_renderer: ParallelRenderer,
    /// Memory pool renderer for allocation optimization
    pooled_renderer: Option<crate::render::PooledRenderer>,
    /// Enable memory pooled rendering for performance
    enable_pooled_rendering: bool,
    /// Selected backend (None = auto-select)
    backend: Option<BackendType>,
    /// Whether auto-optimization has been applied
    auto_optimized: bool,
}

/// Configuration for a single data series
#[derive(Clone, Debug)]
struct PlotSeries {
    /// Series type
    series_type: SeriesType,
    /// Series label for legend
    label: Option<String>,
    /// Series color (None for auto-color)
    color: Option<Color>,
    /// Line width override
    line_width: Option<f32>,
    /// Line style override
    line_style: Option<LineStyle>,
    /// Marker style for scatter plots
    marker_style: Option<MarkerStyle>,
    /// Marker size for scatter plots
    marker_size: Option<f32>,
    /// Alpha/transparency override
    alpha: Option<f32>,
}

/// Types of plot series
#[derive(Clone, Debug)]
enum SeriesType {
    Line {
        x_data: Vec<f64>,
        y_data: Vec<f64>,
    },
    Scatter {
        x_data: Vec<f64>,
        y_data: Vec<f64>,
    },
    Bar {
        categories: Vec<String>,
        values: Vec<f64>,
    },
    ErrorBars {
        x_data: Vec<f64>,
        y_data: Vec<f64>,
        y_errors: Vec<f64>,
    },
    ErrorBarsXY {
        x_data: Vec<f64>,
        y_data: Vec<f64>,
        x_errors: Vec<f64>,
        y_errors: Vec<f64>,
    },
    Histogram {
        data: Vec<f64>,
        config: crate::plots::histogram::HistogramConfig,
    },
    BoxPlot {
        data: Vec<f64>,
        config: crate::plots::boxplot::BoxPlotConfig,
    },
}

/// Legend configuration
#[derive(Clone, Debug)]
struct LegendConfig {
    /// Whether to show legend
    enabled: bool,
    /// Legend position
    position: Position,
    /// Font size override
    font_size: Option<f32>,
}

/// Grid configuration  
#[derive(Clone, Debug)]
struct GridConfig {
    /// Whether to show grid
    enabled: bool,
    /// Grid color override
    color: Option<Color>,
    /// Grid line style override
    style: Option<LineStyle>,
}

/// Tick configuration for axes
#[derive(Clone, Debug)]
struct TickConfig {
    /// Direction ticks point (inside or outside)
    direction: TickDirection,
    /// Number of major ticks on X axis
    major_ticks_x: usize,
    /// Number of minor ticks between major ticks on X axis
    minor_ticks_x: usize,
    /// Number of major ticks on Y axis
    major_ticks_y: usize,
    /// Number of minor ticks between major ticks on Y axis
    minor_ticks_y: usize,
    /// Grid display mode
    grid_mode: GridMode,
}

impl Default for TickConfig {
    fn default() -> Self {
        TickConfig {
            direction: TickDirection::Inside,
            major_ticks_x: 10,
            minor_ticks_x: 0,
            major_ticks_y: 8,
            minor_ticks_y: 0,
            grid_mode: GridMode::MajorOnly,
        }
    }
}

impl Plot {
    /// Create a new Plot with default settings
    ///
    /// Uses matplotlib-compatible defaults:
    /// - Figure size: 6.4 × 4.8 inches
    /// - DPI: 100 (640 × 480 pixels)
    /// - Font size: 10pt base
    /// - Line width: 1.5pt
    pub fn new() -> Self {
        let config = PlotConfig::default();
        let (width, height) = config.canvas_size();
        Self {
            title: None,
            xlabel: None,
            ylabel: None,
            dimensions: (width, height),
            dpi: config.figure.dpi as u32,
            theme: Theme::default(),
            config,
            series: Vec::new(),
            legend: LegendConfig {
                enabled: false,
                position: Position::TopRight,
                font_size: None,
            },
            grid: GridConfig {
                enabled: true,
                color: None,
                style: None,
            },
            tick_config: TickConfig::default(),
            margin: None,
            scientific_notation: false,
            auto_color_index: 0,
            x_limits: None,
            y_limits: None,
            #[cfg(feature = "parallel")]
            parallel_renderer: ParallelRenderer::new(),
            pooled_renderer: None,
            enable_pooled_rendering: false,
            backend: None,
            auto_optimized: false,
        }
    }

    /// Create a new Plot with a specific configuration
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = PlotConfig::builder()
    ///     .figure(8.0, 6.0)
    ///     .dpi(300.0)
    ///     .build();
    /// Plot::with_config(config).line(&x, &y).save("plot.png")?;
    /// ```
    pub fn with_config(config: PlotConfig) -> Self {
        let (width, height) = config.canvas_size();
        let mut plot = Self::new();
        plot.dimensions = (width, height);
        plot.dpi = config.figure.dpi as u32;
        plot.config = config;
        plot
    }

    /// Create a new Plot with a preset style
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::with_style(PlotStyle::Publication)
    ///     .line(&x, &y)
    ///     .save("paper_figure.png")?;
    /// ```
    pub fn with_style(style: PlotStyle) -> Self {
        Self::with_config(style.config())
    }

    /// Create a new Plot with a specific theme
    pub fn with_theme(theme: Theme) -> Self {
        let mut plot = Self::new();
        plot.theme = theme;
        plot
    }

    /// Set the theme for the plot (fluent API)
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Get the current theme
    pub fn get_theme(&self) -> Theme {
        self.theme.clone()
    }

    /// Configure parallel rendering settings
    #[cfg(feature = "parallel")]
    pub fn with_parallel(mut self, threads: Option<usize>) -> Self {
        if let Some(thread_count) = threads {
            self.parallel_renderer = ParallelRenderer::with_threads(thread_count);
        }
        self
    }

    /// Set parallel processing threshold
    #[cfg(feature = "parallel")]
    pub fn parallel_threshold(mut self, threshold: usize) -> Self {
        self.parallel_renderer = self.parallel_renderer.with_threshold(threshold);
        self
    }

    /// Enable memory pooled rendering for allocation optimization
    ///
    /// This reduces allocation overhead by 30-50% for large datasets by reusing
    /// memory buffers for coordinate transformations and rendering operations.
    pub fn with_memory_pooling(mut self, enable: bool) -> Self {
        self.enable_pooled_rendering = enable;
        if enable && self.pooled_renderer.is_none() {
            self.pooled_renderer = Some(crate::render::PooledRenderer::new());
        }
        self
    }

    /// Configure memory pool sizes for specific workloads
    ///
    /// # Arguments
    /// * `f32_pool_size` - Initial capacity for coordinate transformation pools
    /// * `position_pool_size` - Initial capacity for position/point pools  
    /// * `segment_pool_size` - Initial capacity for line segment pools
    pub fn with_pool_sizes(
        mut self,
        f32_pool_size: usize,
        position_pool_size: usize,
        segment_pool_size: usize,
    ) -> Self {
        self.pooled_renderer = Some(crate::render::PooledRenderer::with_pool_sizes(
            f32_pool_size,
            position_pool_size,
            segment_pool_size,
        ));
        self.enable_pooled_rendering = true;
        self
    }

    /// Get memory pool statistics for monitoring and optimization
    pub fn pool_stats(&self) -> Option<crate::render::PooledRendererStats> {
        self.pooled_renderer
            .as_ref()
            .map(|renderer| renderer.get_pool_stats())
    }

    /// Set the plot title
    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the X-axis label
    pub fn xlabel<S: Into<String>>(mut self, label: S) -> Self {
        self.xlabel = Some(label.into());
        self
    }

    /// Set the Y-axis label
    pub fn ylabel<S: Into<String>>(mut self, label: S) -> Self {
        self.ylabel = Some(label.into());
        self
    }

    /// Set X-axis limits (min, max)
    pub fn xlim(mut self, min: f64, max: f64) -> Self {
        if min < max && min.is_finite() && max.is_finite() {
            self.x_limits = Some((min, max));
        }
        self
    }

    /// Set Y-axis limits (min, max)
    pub fn ylim(mut self, min: f64, max: f64) -> Self {
        if min < max && min.is_finite() && max.is_finite() {
            self.y_limits = Some((min, max));
        }
        self
    }

    /// Set canvas dimensions in pixels
    ///
    /// This method automatically scales DPI based on canvas size to maintain
    /// proportional text and element sizes on larger canvases.
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.dimensions = (width.max(100), height.max(100));

        // Auto-scale DPI based on canvas size relative to reference (640x480)
        // This ensures text maintains proportional size on larger canvases
        let reference_diagonal = ((640.0_f32).powi(2) + (480.0_f32).powi(2)).sqrt();
        let canvas_diagonal = ((width as f32).powi(2) + (height as f32).powi(2)).sqrt();
        let scale_factor = (canvas_diagonal / reference_diagonal).max(1.0);
        let auto_dpi = (REFERENCE_DPI * scale_factor).round().max(100.0);

        self.dpi = auto_dpi as u32;
        self.config.figure.dpi = auto_dpi;
        self.config.figure.width = width as f32 / auto_dpi;
        self.config.figure.height = height as f32 / auto_dpi;
        self
    }

    /// Set figure size in inches
    ///
    /// This is the recommended way to set figure size for DPI-independent plots.
    /// Changing DPI will change resolution but not proportions.
    ///
    /// # Arguments
    ///
    /// * `width` - Figure width in inches
    /// * `height` - Figure height in inches
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .size(8.0, 6.0)  // 8×6 inches
    ///     .dpi(300)        // High resolution for print
    ///     .save("figure.png")?;
    /// ```
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.config.figure.width = width.max(1.0);
        self.config.figure.height = height.max(1.0);
        // Update legacy fields for backward compatibility
        let (w, h) = self.config.canvas_size();
        self.dimensions = (w, h);
        self
    }

    /// Set figure size in pixels
    ///
    /// Convenience method for users who prefer to think in pixels.
    /// Internally converts to inches using reference DPI (100).
    ///
    /// # Arguments
    ///
    /// * `width` - Figure width in pixels
    /// * `height` - Figure height in pixels
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .size_px(800, 600)  // 800×600 pixels at 100 DPI
    ///     .save("figure.png")?;
    /// ```
    pub fn size_px(mut self, width: u32, height: u32) -> Self {
        use crate::core::units::REFERENCE_DPI;
        self.config.figure.width = width as f32 / REFERENCE_DPI;
        self.config.figure.height = height as f32 / REFERENCE_DPI;
        // Update legacy fields
        let (w, h) = self.config.canvas_size();
        self.dimensions = (w, h);
        self
    }

    /// Set DPI for export quality
    ///
    /// DPI only affects output resolution, not layout proportions.
    /// Higher DPI produces larger files with more detail.
    ///
    /// # Common values
    ///
    /// * 72-100: Screen/web display
    /// * 150: Good quality print
    /// * 300: Publication quality
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .size(6.4, 4.8)  // Same size in inches
    ///     .dpi(300)        // High resolution: 1920×1440 pixels
    ///     .save("print.png")?;
    /// ```
    pub fn dpi(mut self, dpi: u32) -> Self {
        self.config.figure.dpi = dpi.max(72) as f32;
        self.dpi = dpi.max(72);
        // Update dimensions to reflect new DPI
        let (w, h) = self.config.canvas_size();
        self.dimensions = (w, h);
        self
    }

    /// Apply a style preset
    ///
    /// Style presets configure typography, line widths, and spacing
    /// for specific use cases.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .plot_style(PlotStyle::Publication)
    ///     .line(&x, &y)
    ///     .save("paper.png")?;
    /// ```
    pub fn plot_style(mut self, style: PlotStyle) -> Self {
        self.config = style.config();
        let (w, h) = self.config.canvas_size();
        self.dimensions = (w, h);
        self.dpi = self.config.figure.dpi as u32;
        self
    }

    /// Set the full plot configuration
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = PlotConfig::builder()
    ///     .figure(10.0, 7.5)
    ///     .font_size(14.0)
    ///     .build();
    /// Plot::new().plot_config(config).line(&x, &y).save("plot.png")?;
    /// ```
    pub fn plot_config(mut self, config: PlotConfig) -> Self {
        let (w, h) = config.canvas_size();
        self.dimensions = (w, h);
        self.dpi = config.figure.dpi as u32;
        self.config = config;
        self
    }

    /// Set the base font size in points
    ///
    /// All other font sizes (title, labels, ticks) scale relative to this.
    pub fn font_size(mut self, size: f32) -> Self {
        self.config.typography.base_size = size.max(4.0);
        self
    }

    /// Set the title font size in points (absolute)
    pub fn title_size(mut self, size: f32) -> Self {
        // Convert to scale factor
        self.config.typography.title_scale = size / self.config.typography.base_size;
        self
    }

    /// Set the data line width in points
    pub fn line_width_pt(mut self, width: f32) -> Self {
        self.config.lines.data_width = width.max(0.1);
        self
    }

    /// Get the current PlotConfig
    pub fn get_config(&self) -> &PlotConfig {
        &self.config
    }

    /// Adjust margins to tightly fit text with custom padding
    ///
    /// # Arguments
    ///
    /// * `pad` - Extra padding in points around text elements
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .tight_layout_pad(4.0)  // 4pt extra padding
    ///     .save("plot.png")?;
    /// ```
    pub fn tight_layout_pad(mut self, pad: f32) -> Self {
        let width = self.config.figure.width;
        let height = self.config.figure.height;

        // Estimate text sizes in inches
        let pt_to_in = |pt: f32| pt / 72.0;
        let pad_in = pt_to_in(pad);

        // Calculate required top margin (title)
        let top_margin = if self.title.is_some() {
            let title_size = self.config.typography.title_size();
            let title_pad = self.config.spacing.title_pad;
            pt_to_in(title_size) + pt_to_in(title_pad) + pad_in
        } else {
            pad_in.max(0.1) // Minimal margin
        };

        // Calculate required bottom margin (xlabel + tick labels)
        let tick_size = self.config.typography.tick_size();
        let label_size = self.config.typography.label_size();
        let tick_pad = self.config.spacing.tick_pad;
        let label_pad = self.config.spacing.label_pad;

        let bottom_margin = if self.xlabel.is_some() {
            pt_to_in(tick_size)
                + pt_to_in(tick_pad)
                + pt_to_in(label_size)
                + pt_to_in(label_pad)
                + pad_in
        } else {
            pt_to_in(tick_size) + pt_to_in(tick_pad) + pad_in
        };

        // Calculate required left margin (ylabel + tick labels)
        // Y-axis tick labels are typically 4-5 characters wide
        let estimated_tick_width = pt_to_in(tick_size) * 4.0;

        let left_margin = if self.ylabel.is_some() {
            estimated_tick_width
                + pt_to_in(tick_pad)
                + pt_to_in(label_size)
                + pt_to_in(label_pad)
                + pad_in
        } else {
            estimated_tick_width + pt_to_in(tick_pad) + pad_in
        };

        // Right margin is minimal (just padding)
        let right_margin = pad_in.max(0.1);

        // Ensure margins don't exceed half the figure size
        let max_horizontal = width * 0.4;
        let max_vertical = height * 0.4;

        self.config.margins = MarginConfig::Fixed {
            left: left_margin.min(max_horizontal),
            right: right_margin.min(max_horizontal),
            top: top_margin.min(max_vertical),
            bottom: bottom_margin.min(max_vertical),
        };

        self
    }

    /// Calculate canvas dimensions from config
    fn config_canvas_size(&self) -> (u32, u32) {
        self.config.canvas_size()
    }

    /// Get font size in pixels for rendering
    fn font_size_px(&self, points: f32) -> f32 {
        pt_to_px(points, self.config.figure.dpi)
    }

    /// Get line width in pixels for rendering
    fn line_width_px(&self, points: f32) -> f32 {
        pt_to_px(points, self.config.figure.dpi)
    }

    /// Calculate DPI-scaled canvas dimensions
    /// **Deprecated**: Use config_canvas_size() instead
    fn dpi_scaled_dimensions(&self) -> (u32, u32) {
        self.config_canvas_size()
    }

    /// Calculate DPI scaling factor
    /// **Deprecated**: Use config.figure.dpi with pt_to_px/in_to_px instead
    fn dpi_scale(&self) -> f32 {
        self.config.figure.dpi / 72.0 // Scale relative to 72 DPI (1pt = 1px)
    }

    /// Calculate DPI-scaled font size
    /// **Deprecated**: Use font_size_px() with config.typography instead
    pub fn dpi_scaled_font_size(&self, base_size: f32) -> f32 {
        pt_to_px(base_size, self.config.figure.dpi)
    }

    /// Calculate DPI-scaled line width
    /// **Deprecated**: Use line_width_px() with config.lines instead
    pub fn dpi_scaled_line_width(&self, base_width: f32) -> f32 {
        pt_to_px(base_width, self.config.figure.dpi)
    }

    /// Set margin around plot area
    pub fn margin(mut self, margin: f32) -> Self {
        self.margin = Some(margin.clamp(0.0, 0.5));
        self
    }

    /// Enable/disable scientific notation on axes
    pub fn scientific_notation(mut self, enabled: bool) -> Self {
        self.scientific_notation = enabled;
        self
    }

    /// Add a line plot series
    pub fn line<X, Y>(mut self, x_data: &X, y_data: &Y) -> PlotSeriesBuilder
    where
        X: Data1D<f64>,
        Y: Data1D<f64>,
    {
        // Validate data lengths match
        if x_data.len() != y_data.len() {
            // For now, we'll handle this in the builder
            // In a real implementation, we might want to return Result
        }

        let x_vec: Vec<f64> = x_data.iter().copied().collect();
        let y_vec: Vec<f64> = y_data.iter().copied().collect();

        let series = PlotSeries {
            series_type: SeriesType::Line {
                x_data: x_vec,
                y_data: y_vec,
            },
            label: None,
            color: None,
            line_width: None,
            line_style: None,
            marker_style: None,
            marker_size: None,
            alpha: None,
        };

        PlotSeriesBuilder::new(self, series)
    }

    /// Add a scatter plot series
    pub fn scatter<X, Y>(mut self, x_data: &X, y_data: &Y) -> PlotSeriesBuilder
    where
        X: Data1D<f64>,
        Y: Data1D<f64>,
    {
        let x_vec: Vec<f64> = x_data.iter().copied().collect();
        let y_vec: Vec<f64> = y_data.iter().copied().collect();

        let series = PlotSeries {
            series_type: SeriesType::Scatter {
                x_data: x_vec,
                y_data: y_vec,
            },
            label: None,
            color: None,
            line_width: None,
            line_style: None,
            marker_style: Some(MarkerStyle::Circle),
            marker_size: None,
            alpha: None,
        };

        PlotSeriesBuilder::new(self, series)
    }

    /// Add a bar plot series
    pub fn bar<S, V>(mut self, categories: &[S], values: &V) -> PlotSeriesBuilder
    where
        S: ToString,
        V: Data1D<f64>,
    {
        let cat_vec: Vec<String> = categories.iter().map(|s| s.to_string()).collect();
        let val_vec: Vec<f64> = values.iter().copied().collect();

        let series = PlotSeries {
            series_type: SeriesType::Bar {
                categories: cat_vec,
                values: val_vec,
            },
            label: None,
            color: None,
            line_width: None,
            line_style: None,
            marker_style: None,
            marker_size: None,
            alpha: None,
        };

        PlotSeriesBuilder::new(self, series)
    }

    /// Add a histogram plot series
    pub fn histogram<T, D: Data1D<T>>(
        self,
        data: &D,
        config: Option<HistogramConfig>,
    ) -> PlotSeriesBuilder
    where
        T: Into<f64> + Copy,
    {
        let mut data_vec = Vec::with_capacity(data.len());
        for i in 0..data.len() {
            if let Some(val) = data.get(i) {
                data_vec.push((*val).into());
            }
        }
        let hist_config = config.unwrap_or_else(|| HistogramConfig::new());

        let series = PlotSeries {
            series_type: SeriesType::Histogram {
                data: data_vec,
                config: hist_config,
            },
            label: None,
            color: None,
            line_width: None,
            line_style: None,
            marker_style: None,
            marker_size: None,
            alpha: None,
        };

        PlotSeriesBuilder::new(self, series)
    }

    /// Add a box plot series
    pub fn boxplot<T, D: Data1D<T>>(
        self,
        data: &D,
        config: Option<BoxPlotConfig>,
    ) -> PlotSeriesBuilder
    where
        T: Into<f64> + Copy,
    {
        let mut data_vec = Vec::with_capacity(data.len());
        for i in 0..data.len() {
            if let Some(val) = data.get(i) {
                data_vec.push((*val).into());
            }
        }
        let box_config = config.unwrap_or_else(|| BoxPlotConfig::new());

        let series = PlotSeries {
            series_type: SeriesType::BoxPlot {
                data: data_vec,
                config: box_config,
            },
            label: None,
            color: None,
            line_width: None,
            line_style: None,
            marker_style: None,
            marker_size: None,
            alpha: None,
        };

        PlotSeriesBuilder::new(self, series)
    }

    /// Add error bars (Y-direction only)
    pub fn error_bars<X, Y, E>(mut self, x_data: &X, y_data: &Y, y_errors: &E) -> PlotSeriesBuilder
    where
        X: Data1D<f64>,
        Y: Data1D<f64>,
        E: Data1D<f64>,
    {
        let x_vec: Vec<f64> = x_data.iter().copied().collect();
        let y_vec: Vec<f64> = y_data.iter().copied().collect();
        let e_vec: Vec<f64> = y_errors.iter().copied().collect();

        let series = PlotSeries {
            series_type: SeriesType::ErrorBars {
                x_data: x_vec,
                y_data: y_vec,
                y_errors: e_vec,
            },
            label: None,
            color: None,
            line_width: None,
            line_style: None,
            marker_style: None,
            marker_size: None,
            alpha: None,
        };

        PlotSeriesBuilder::new(self, series)
    }

    /// Add error bars in both X and Y directions
    pub fn error_bars_xy<X, Y, EX, EY>(
        mut self,
        x_data: &X,
        y_data: &Y,
        x_errors: &EX,
        y_errors: &EY,
    ) -> PlotSeriesBuilder
    where
        X: Data1D<f64>,
        Y: Data1D<f64>,
        EX: Data1D<f64>,
        EY: Data1D<f64>,
    {
        let x_vec: Vec<f64> = x_data.iter().copied().collect();
        let y_vec: Vec<f64> = y_data.iter().copied().collect();
        let ex_vec: Vec<f64> = x_errors.iter().copied().collect();
        let ey_vec: Vec<f64> = y_errors.iter().copied().collect();

        let series = PlotSeries {
            series_type: SeriesType::ErrorBarsXY {
                x_data: x_vec,
                y_data: y_vec,
                x_errors: ex_vec,
                y_errors: ey_vec,
            },
            label: None,
            color: None,
            line_width: None,
            line_style: None,
            marker_style: None,
            marker_size: None,
            alpha: None,
        };

        PlotSeriesBuilder::new(self, series)
    }

    /// Configure legend
    pub fn legend(mut self, position: Position) -> Self {
        self.legend.enabled = true;
        self.legend.position = position;
        self
    }

    /// Enable/disable grid
    pub fn grid(mut self, enabled: bool) -> Self {
        self.grid.enabled = enabled;
        self
    }

    /// Set tick direction to inside (default)
    pub fn tick_direction_inside(mut self) -> Self {
        self.tick_config.direction = TickDirection::Inside;
        self
    }

    /// Set tick direction to outside
    pub fn tick_direction_outside(mut self) -> Self {
        self.tick_config.direction = TickDirection::Outside;
        self
    }

    /// Set number of major ticks for both axes
    pub fn major_ticks(mut self, count: usize) -> Self {
        self.tick_config.major_ticks_x = count;
        self.tick_config.major_ticks_y = count;
        self
    }

    /// Set number of minor ticks between major ticks for both axes
    pub fn minor_ticks(mut self, count: usize) -> Self {
        self.tick_config.minor_ticks_x = count;
        self.tick_config.minor_ticks_y = count;
        self
    }

    /// Set number of major ticks for X axis
    pub fn major_ticks_x(mut self, count: usize) -> Self {
        self.tick_config.major_ticks_x = count;
        self
    }

    /// Set number of minor ticks between major ticks for X axis
    pub fn minor_ticks_x(mut self, count: usize) -> Self {
        self.tick_config.minor_ticks_x = count;
        self
    }

    /// Set number of major ticks for Y axis
    pub fn major_ticks_y(mut self, count: usize) -> Self {
        self.tick_config.major_ticks_y = count;
        self
    }

    /// Set number of minor ticks between major ticks for Y axis
    pub fn minor_ticks_y(mut self, count: usize) -> Self {
        self.tick_config.minor_ticks_y = count;
        self
    }

    /// Grid lines only at major ticks
    pub fn grid_major_only(mut self) -> Self {
        self.tick_config.grid_mode = GridMode::MajorOnly;
        self
    }

    /// Grid lines only at minor ticks
    pub fn grid_minor_only(mut self) -> Self {
        self.tick_config.grid_mode = GridMode::MinorOnly;
        self
    }

    /// Grid lines at both major and minor ticks
    pub fn grid_both(mut self) -> Self {
        self.tick_config.grid_mode = GridMode::Both;
        self
    }

    /// Enable tight layout (automatic margin adjustment like matplotlib)
    ///
    /// When enabled, computes minimum required margins based on:
    /// - Title dimensions (if present)
    /// - X-axis label and tick label dimensions
    /// - Y-axis label and tick label dimensions
    ///
    /// The result is `Fixed` margins that eliminate dead space while
    /// ensuring no text is clipped.
    ///
    /// # Arguments
    ///
    /// * `enabled` - If true, compute tight margins; if false, use default proportional margins
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .title("My Plot")
    ///     .xlabel("X Values")
    ///     .ylabel("Y Values")
    ///     .line(&x, &y)
    ///     .tight_layout(true)  // Compute optimal margins
    ///     .save("tight.png")?;
    /// ```
    pub fn tight_layout(self, enabled: bool) -> Self {
        if enabled {
            self.tight_layout_pad(2.0) // Default 2pt padding
        } else {
            // Reset to default proportional margins
            let mut s = self;
            s.config.margins = MarginConfig::default();
            s
        }
    }

    /// Set grid color
    pub fn grid_color(mut self, color: Color) -> Self {
        self.grid.color = Some(color);
        self
    }

    /// Set grid line style
    pub fn grid_style(mut self, style: LineStyle) -> Self {
        self.grid.style = Some(style);
        self
    }

    /// Enable LaTeX rendering (placeholder - requires latex feature)
    pub fn latex(mut self, _enabled: bool) -> Self {
        // Placeholder for future LaTeX support
        // Would require additional dependencies and rendering backend
        self
    }

    /// Set transparency for the next series
    pub fn alpha(self, _alpha: f32) -> Self {
        // This would be handled by the series builder
        // Keeping for API compatibility
        self
    }

    /// Add a new line to existing plot (for incremental updates)
    pub fn add_line<X, Y>(&mut self, x_data: &X, y_data: &Y) -> Result<()>
    where
        X: Data1D<f64>,
        Y: Data1D<f64>,
    {
        if x_data.len() != y_data.len() {
            return Err(PlottingError::DataLengthMismatch {
                x_len: x_data.len(),
                y_len: y_data.len(),
            });
        }

        if x_data.is_empty() {
            return Err(PlottingError::EmptyDataSet);
        }

        let x_vec: Vec<f64> = x_data.iter().copied().collect();
        let y_vec: Vec<f64> = y_data.iter().copied().collect();

        let series = PlotSeries {
            series_type: SeriesType::Line {
                x_data: x_vec,
                y_data: y_vec,
            },
            label: None,
            color: Some(self.theme.get_color(self.auto_color_index)),
            line_width: None,
            line_style: None,
            marker_style: None,
            marker_size: None,
            alpha: None,
        };

        self.series.push(series);
        self.auto_color_index += 1;

        Ok(())
    }

    /// Helper method to render a single series using normal (non-DataShader) rendering
    fn render_series_normal(
        &self,
        series: &PlotSeries,
        renderer: &mut SkiaRenderer,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> Result<()> {
        let color = series.color.unwrap_or(Color::new(0, 0, 0)); // Default black
        let line_width = self.dpi_scaled_line_width(series.line_width.unwrap_or(2.0));
        let line_style = series.line_style.clone().unwrap_or(LineStyle::Solid);

        match &series.series_type {
            SeriesType::Line { x_data, y_data } => {
                let points: Vec<(f32, f32)> = x_data
                    .iter()
                    .zip(y_data.iter())
                    .map(|(&x, &y)| {
                        crate::render::skia::map_data_to_pixels(
                            x, y, x_min, x_max, y_min, y_max, plot_area,
                        )
                    })
                    .collect();

                renderer.draw_polyline(&points, color, line_width, line_style)?;
            }
            SeriesType::Scatter { x_data, y_data } => {
                let marker_size = self.dpi_scaled_line_width(series.marker_size.unwrap_or(10.0)); // DPI-scaled marker size
                let marker_style = series.marker_style.unwrap_or(MarkerStyle::Circle);

                for (&x, &y) in x_data.iter().zip(y_data.iter()) {
                    let (px, py) = crate::render::skia::map_data_to_pixels(
                        x, y, x_min, x_max, y_min, y_max, plot_area,
                    );
                    renderer.draw_marker(px, py, marker_size, marker_style, color)?;
                }
            }
            SeriesType::Bar { values, .. } => {
                // Simple bar rendering
                let bar_width = plot_area.width() / values.len() as f32 * 0.8;
                for (i, &value) in values.iter().enumerate() {
                    let x = i as f64;
                    let (px, py) = crate::render::skia::map_data_to_pixels(
                        x, value, x_min, x_max, y_min, y_max, plot_area,
                    );
                    let (_, py_zero) = crate::render::skia::map_data_to_pixels(
                        x, 0.0, x_min, x_max, y_min, y_max, plot_area,
                    );
                    renderer.draw_rectangle(
                        px - bar_width / 2.0,
                        py.min(py_zero),
                        bar_width,
                        (py - py_zero).abs(),
                        color,
                        true,
                    )?;
                }
            }
            SeriesType::Histogram { data, config } => {
                // Calculate histogram data
                let hist_data = crate::plots::histogram::calculate_histogram(data, config)
                    .map_err(|e| {
                        PlottingError::RenderError(format!("Histogram calculation failed: {}", e))
                    })?;

                // Render histogram bars
                for (i, &count) in hist_data.counts.iter().enumerate() {
                    if count > 0.0 {
                        let x_left = hist_data.bin_edges[i];
                        let x_right = hist_data.bin_edges[i + 1];
                        let x_center = (x_left + x_right) / 2.0;

                        // Convert bar width from data coordinates to pixel coordinates
                        let (px_left, _) = crate::render::skia::map_data_to_pixels(
                            x_left, 0.0, x_min, x_max, y_min, y_max, plot_area,
                        );
                        let (px_right, _) = crate::render::skia::map_data_to_pixels(
                            x_right, 0.0, x_min, x_max, y_min, y_max, plot_area,
                        );
                        let bar_width_px = (px_right - px_left).abs();

                        let (px, py) = crate::render::skia::map_data_to_pixels(
                            x_center, count, x_min, x_max, y_min, y_max, plot_area,
                        );
                        let (_, py_zero) = crate::render::skia::map_data_to_pixels(
                            x_center, 0.0, x_min, x_max, y_min, y_max, plot_area,
                        );

                        renderer.draw_rectangle(
                            px - bar_width_px / 2.0,
                            py.min(py_zero),
                            bar_width_px,
                            (py - py_zero).abs(),
                            color,
                            true,
                        )?;
                    }
                }
            }
            SeriesType::BoxPlot { data, config } => {
                // Calculate box plot statistics
                let box_data =
                    crate::plots::boxplot::calculate_box_plot(data, config).map_err(|e| {
                        PlottingError::RenderError(format!("Box plot calculation failed: {}", e))
                    })?;

                // Box plot positioning
                let x_center = 0.5; // Center the box plot
                let box_width = 0.3; // Box width

                // Map coordinates to pixels
                let (x_center_px, _) = crate::render::skia::map_data_to_pixels(
                    x_center, 0.0, x_min, x_max, y_min, y_max, plot_area,
                );
                let (_, q1_y) = crate::render::skia::map_data_to_pixels(
                    0.0,
                    box_data.q1,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                );
                let (_, median_y) = crate::render::skia::map_data_to_pixels(
                    0.0,
                    box_data.median,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                );
                let (_, q3_y) = crate::render::skia::map_data_to_pixels(
                    0.0,
                    box_data.q3,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                );
                let (_, lower_whisker_y) = crate::render::skia::map_data_to_pixels(
                    0.0,
                    box_data.min,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                );
                let (_, upper_whisker_y) = crate::render::skia::map_data_to_pixels(
                    0.0,
                    box_data.max,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                );

                let box_half_width = box_width * plot_area.width() * 0.5;
                let box_left = x_center_px - box_half_width;
                let box_right = x_center_px + box_half_width;

                // Draw the box (IQR) - ensure positive dimensions
                let box_width = box_right - box_left;
                let box_height = (q1_y - q3_y).abs(); // Ensure positive height
                let box_top = q3_y.min(q1_y); // Use the smaller y value as top

                // Validate dimensions before drawing
                if box_width > 0.0
                    && box_height > 0.0
                    && box_width.is_finite()
                    && box_height.is_finite()
                {
                    renderer.draw_rectangle(
                        box_left, box_top, box_width, box_height, color, false, // outline only
                    )?;
                }

                // Draw median line - validate coordinates
                if box_left.is_finite() && median_y.is_finite() && box_right.is_finite() {
                    renderer.draw_line(
                        box_left,
                        median_y,
                        box_right,
                        median_y,
                        color,
                        line_width * 1.5, // thicker median line
                        line_style.clone(),
                    )?;
                }

                // Draw lower whisker - validate coordinates
                if x_center_px.is_finite() && q1_y.is_finite() && lower_whisker_y.is_finite() {
                    renderer.draw_line(
                        x_center_px,
                        q1_y,
                        x_center_px,
                        lower_whisker_y,
                        color,
                        line_width,
                        line_style.clone(),
                    )?;
                }

                // Draw upper whisker - validate coordinates
                if x_center_px.is_finite() && q3_y.is_finite() && upper_whisker_y.is_finite() {
                    renderer.draw_line(
                        x_center_px,
                        q3_y,
                        x_center_px,
                        upper_whisker_y,
                        color,
                        line_width,
                        line_style.clone(),
                    )?;
                }

                // Draw whisker caps - validate coordinates
                let cap_width = box_half_width * 0.6;
                if x_center_px.is_finite() && lower_whisker_y.is_finite() && cap_width.is_finite() {
                    renderer.draw_line(
                        x_center_px - cap_width,
                        lower_whisker_y,
                        x_center_px + cap_width,
                        lower_whisker_y,
                        color,
                        line_width,
                        line_style.clone(),
                    )?;
                }

                if x_center_px.is_finite() && upper_whisker_y.is_finite() && cap_width.is_finite() {
                    renderer.draw_line(
                        x_center_px - cap_width,
                        upper_whisker_y,
                        x_center_px + cap_width,
                        upper_whisker_y,
                        color,
                        line_width,
                        line_style.clone(),
                    )?;
                }

                // Draw outliers - validate coordinates
                for &outlier in &box_data.outliers {
                    let (_, outlier_y) = crate::render::skia::map_data_to_pixels(
                        0.0, outlier, x_min, x_max, y_min, y_max, plot_area,
                    );
                    if x_center_px.is_finite() && outlier_y.is_finite() {
                        renderer.draw_marker(
                            x_center_px,
                            outlier_y,
                            4.0, // outlier marker size
                            MarkerStyle::Circle,
                            color,
                        )?;
                    }
                }
            }
            _ => {} // Other plot types not implemented yet
        }

        Ok(())
    }

    /// Render the plot to an in-memory image
    pub fn render(&self) -> Result<Image> {
        // Validate we have at least one series
        if self.series.is_empty() {
            return Err(PlottingError::NoDataSeries);
        }

        // Validate all series data
        for (i, series) in self.series.iter().enumerate() {
            match &series.series_type {
                SeriesType::Line { x_data, y_data } | SeriesType::Scatter { x_data, y_data } => {
                    if x_data.len() != y_data.len() {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: x_data.len(),
                            y_len: y_data.len(),
                        });
                    }
                    if x_data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::Bar { categories, values } => {
                    if categories.len() != values.len() {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: categories.len(),
                            y_len: values.len(),
                        });
                    }
                    if categories.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::ErrorBars {
                    x_data,
                    y_data,
                    y_errors,
                } => {
                    if x_data.len() != y_data.len() || y_data.len() != y_errors.len() {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: x_data.len(),
                            y_len: y_data.len(),
                        });
                    }
                }
                SeriesType::ErrorBarsXY {
                    x_data,
                    y_data,
                    x_errors,
                    y_errors,
                } => {
                    if x_data.len() != y_data.len()
                        || x_data.len() != x_errors.len()
                        || x_data.len() != y_errors.len()
                    {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: x_data.len(),
                            y_len: y_data.len(),
                        });
                    }
                }
                SeriesType::Histogram { data, .. } => {
                    if data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::BoxPlot { data, .. } => {
                    if data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
            }
        }

        // Check if DataShader optimization should be used
        let total_points = self.calculate_total_points();
        let use_datashader = DataShader::should_activate(total_points);

        if use_datashader {
            // Use DataShader for large datasets
            return self.render_with_datashader();
        }

        // Check if parallel processing should be used
        #[cfg(feature = "parallel")]
        {
            let series_count = self.series.len();
            if self
                .parallel_renderer
                .should_use_parallel(series_count, total_points)
            {
                return self.render_with_parallel();
            }
        }

        // Create renderer for standard rendering with DPI scaling
        let (scaled_width, scaled_height) = self.config_canvas_size();
        let mut renderer = SkiaRenderer::new(scaled_width, scaled_height, self.theme.clone())?;
        let dpi = self.config.figure.dpi;

        // Calculate or use manual data bounds
        let (mut x_min, mut x_max, mut y_min, mut y_max) =
            if let (Some((x_min_manual, x_max_manual)), Some((y_min_manual, y_max_manual))) =
                (self.x_limits, self.y_limits)
            {
                // Use both manual limits
                (x_min_manual, x_max_manual, y_min_manual, y_max_manual)
            } else if let Some((x_min_manual, x_max_manual)) = self.x_limits {
                // Use manual X limits, calculate Y bounds from data
                let (_, _, y_min_calc, y_max_calc) = self.calculate_data_bounds()?;
                (x_min_manual, x_max_manual, y_min_calc, y_max_calc)
            } else if let Some((y_min_manual, y_max_manual)) = self.y_limits {
                // Use manual Y limits, calculate X bounds from data
                let (x_min_calc, x_max_calc, _, _) = self.calculate_data_bounds()?;
                (x_min_calc, x_max_calc, y_min_manual, y_max_manual)
            } else {
                self.calculate_data_bounds()?
            };

        // Handle edge case where all data is the same
        if (x_max - x_min).abs() < f64::EPSILON {
            x_min -= 1.0;
            x_max += 1.0;
        }
        if (y_max - y_min).abs() < f64::EPSILON {
            y_min -= 1.0;
            y_max += 1.0;
        }

        // Extract bar chart categories if present (for categorical x-axis labels)
        let bar_categories: Option<Vec<String>> = self.series.iter().find_map(|s| {
            if let SeriesType::Bar { categories, .. } = &s.series_type {
                Some(categories.clone())
            } else {
                None
            }
        });

        // Choose layout method based on MarginConfig
        let (plot_area, layout_opt): (tiny_skia::Rect, Option<PlotLayout>) =
            match &self.config.margins {
                MarginConfig::ContentDriven {
                    edge_buffer,
                    center_plot,
                } => {
                    // Use content-driven layout calculator
                    let content = self.create_plot_content(y_min, y_max);
                    let layout_config = LayoutConfig {
                        edge_buffer_pt: *edge_buffer,
                        center_plot: *center_plot,
                        ..Default::default()
                    };
                    let calculator = LayoutCalculator::new(layout_config);
                    let layout = calculator.compute(
                        (scaled_width, scaled_height),
                        &content,
                        &self.config.typography,
                        &self.config.spacing,
                        dpi,
                    );
                    let skia_rect = tiny_skia::Rect::from_ltrb(
                        layout.plot_area.left,
                        layout.plot_area.top,
                        layout.plot_area.right,
                        layout.plot_area.bottom,
                    )
                    .ok_or(PlottingError::InvalidData {
                        message: "Invalid plot area from layout".to_string(),
                        position: None,
                    })?;
                    (skia_rect, Some(layout))
                }
                _ => {
                    // Use legacy margin-based layout
                    let margins = self.config.compute_margins(
                        self.title.is_some(),
                        self.xlabel.is_some(),
                        self.ylabel.is_some(),
                    );
                    let plot_area =
                        calculate_plot_area_config(scaled_width, scaled_height, &margins, dpi);
                    (plot_area, None)
                }
            };

        // Generate nice tick values
        let x_ticks = generate_ticks(x_min, x_max, 8);
        let y_ticks = generate_ticks(y_min, y_max, 6);

        // Convert ticks to pixel coordinates
        let x_tick_pixels: Vec<f32> = x_ticks
            .iter()
            .map(|&tick| map_data_to_pixels(tick, 0.0, x_min, x_max, y_min, y_max, plot_area).0)
            .collect();
        let y_tick_pixels: Vec<f32> = y_ticks
            .iter()
            .map(|&tick| map_data_to_pixels(0.0, tick, x_min, x_max, y_min, y_max, plot_area).1)
            .collect();

        // Draw grid if enabled
        if self.grid.enabled {
            let grid_width_px = self.line_width_px(self.config.lines.grid_width);
            renderer.draw_grid(
                &x_tick_pixels,
                &y_tick_pixels,
                plot_area,
                self.theme.grid_color,
                LineStyle::Solid,
                grid_width_px,
            )?;
        }

        // Draw axes and labels based on layout method
        if let Some(ref layout) = layout_opt {
            // Content-driven layout: use computed positions
            let tick_size_px = pt_to_px(self.config.typography.tick_size(), dpi);

            // Draw tick labels using layout positions
            // Use categorical labels for bar charts, numeric for others
            if let Some(ref categories) = bar_categories {
                renderer.draw_axis_labels_at_categorical(
                    &layout.plot_area,
                    categories,
                    y_min,
                    y_max,
                    layout.xtick_baseline_y,
                    layout.ytick_right_x,
                    tick_size_px,
                    self.theme.foreground,
                    dpi,
                )?;
            } else {
                renderer.draw_axis_labels_at(
                    &layout.plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    layout.xtick_baseline_y,
                    layout.ytick_right_x,
                    tick_size_px,
                    self.theme.foreground,
                    dpi,
                )?;
            }

            // Draw title if present
            if let Some(ref pos) = layout.title_pos {
                if let Some(ref title) = self.title {
                    renderer.draw_title_at(pos, title, self.theme.foreground)?;
                }
            }

            // Draw xlabel if present
            if let Some(ref pos) = layout.xlabel_pos {
                if let Some(ref xlabel) = self.xlabel {
                    renderer.draw_xlabel_at(pos, xlabel, self.theme.foreground)?;
                }
            }

            // Draw ylabel if present
            if let Some(ref pos) = layout.ylabel_pos {
                if let Some(ref ylabel) = self.ylabel {
                    renderer.draw_ylabel_at(pos, ylabel, self.theme.foreground)?;
                }
            }
        } else {
            // Legacy layout: use old positioning logic
            renderer.draw_axes(
                plot_area,
                &x_tick_pixels,
                &y_tick_pixels,
                self.theme.foreground,
            )?;
        }

        // Render each data series
        for series in &self.series {
            // Get series styling with defaults
            let color = series.color.unwrap_or_else(|| {
                let palette = Color::default_palette();
                palette[self.auto_color_index % palette.len()]
            });
            // Use config data line width, or series override if specified
            let line_width_pt = series.line_width.unwrap_or(self.config.lines.data_width);
            let line_width = self.line_width_px(line_width_pt);
            let line_style = series.line_style.clone().unwrap_or(LineStyle::Solid);
            let marker_style = series.marker_style.unwrap_or(MarkerStyle::Circle);

            match &series.series_type {
                SeriesType::Line { x_data, y_data } => {
                    // Convert data to pixel coordinates
                    let mut points = Vec::new();
                    for i in 0..x_data.len() {
                        let x_val = x_data[i];
                        let y_val = y_data[i];
                        if x_val.is_finite() && y_val.is_finite() {
                            let (px, py) = map_data_to_pixels(
                                x_val, y_val, x_min, x_max, y_min, y_max, plot_area,
                            );
                            points.push((px, py));
                        }
                    }

                    if points.len() >= 2 {
                        renderer.draw_polyline(&points, color, line_width, line_style)?;
                    }
                }
                SeriesType::Scatter { x_data, y_data } => {
                    // Draw individual markers
                    let marker_size_px = self.line_width_px(series.marker_size.unwrap_or(8.0)); // 8pt default marker
                    for i in 0..x_data.len() {
                        let x_val = x_data[i];
                        let y_val = y_data[i];
                        if x_val.is_finite() && y_val.is_finite() {
                            let (px, py) = map_data_to_pixels(
                                x_val, y_val, x_min, x_max, y_min, y_max, plot_area,
                            );
                            renderer.draw_marker(px, py, marker_size_px, marker_style, color)?;
                        }
                    }
                }
                SeriesType::Bar { categories, values } => {
                    // Calculate bar width based on data density
                    let bar_width = if categories.len() > 1 {
                        let available_width = plot_area.width() * 0.8;
                        (available_width / categories.len() as f32).min(40.0)
                    } else {
                        40.0 // Default bar width
                    };

                    // Draw bars from baseline to data value
                    let baseline =
                        map_data_to_pixels(0.0, 0.0, x_min, x_max, y_min, y_max, plot_area).1;

                    for (i, &value) in values.iter().enumerate() {
                        if value.is_finite() {
                            let x_val = i as f64;
                            let (px, py) = map_data_to_pixels(
                                x_val, value, x_min, x_max, y_min, y_max, plot_area,
                            );
                            let bar_height = (baseline - py).abs();
                            let bar_x = px - bar_width * 0.5;

                            if value >= 0.0 {
                                renderer.draw_rectangle(
                                    bar_x, py, bar_width, bar_height, color, true,
                                )?;
                            } else {
                                renderer.draw_rectangle(
                                    bar_x, baseline, bar_width, bar_height, color, true,
                                )?;
                            }
                        }
                    }
                }
                SeriesType::Histogram { data, config } => {
                    // Calculate histogram data
                    let hist_data = crate::plots::histogram::calculate_histogram(data, config)
                        .map_err(|e| {
                            PlottingError::RenderError(format!(
                                "Histogram calculation failed: {}",
                                e
                            ))
                        })?;

                    // Calculate bar width from bin edges
                    let bar_width_data = if hist_data.bin_edges.len() > 1 {
                        hist_data.bin_edges[1] - hist_data.bin_edges[0]
                    } else {
                        1.0
                    };

                    // Convert to pixel width
                    let left_px = map_data_to_pixels(
                        hist_data.bin_edges[0],
                        0.0,
                        x_min,
                        x_max,
                        y_min,
                        y_max,
                        plot_area,
                    )
                    .0;
                    let right_px = map_data_to_pixels(
                        hist_data.bin_edges[0] + bar_width_data,
                        0.0,
                        x_min,
                        x_max,
                        y_min,
                        y_max,
                        plot_area,
                    )
                    .0;
                    let bar_width_px = (right_px - left_px).abs();

                    // Draw histogram bars
                    let baseline =
                        map_data_to_pixels(0.0, 0.0, x_min, x_max, y_min, y_max, plot_area).1;

                    for (i, &count) in hist_data.counts.iter().enumerate() {
                        if count > 0.0 && count.is_finite() {
                            // Use bin center for x position
                            let bin_center =
                                (hist_data.bin_edges[i] + hist_data.bin_edges[i + 1]) / 2.0;
                            let (px, py) = map_data_to_pixels(
                                bin_center, count, x_min, x_max, y_min, y_max, plot_area,
                            );
                            let bar_height = (baseline - py).abs();
                            let bar_x = px - bar_width_px * 0.5;

                            renderer.draw_rectangle(
                                bar_x,
                                py,
                                bar_width_px,
                                bar_height,
                                color,
                                true,
                            )?;
                        }
                    }
                }
                _ => {
                    // For unsupported plot types (error bars), render as scatter points for now
                    // This is a placeholder - full implementation would handle error bars properly
                    match &series.series_type {
                        SeriesType::ErrorBars { x_data, y_data, .. }
                        | SeriesType::ErrorBarsXY { x_data, y_data, .. } => {
                            let marker_size_px =
                                self.line_width_px(series.marker_size.unwrap_or(8.0));
                            for i in 0..x_data.len() {
                                let x_val = x_data[i];
                                let y_val = y_data[i];
                                if x_val.is_finite() && y_val.is_finite() {
                                    let (px, py) = map_data_to_pixels(
                                        x_val, y_val, x_min, x_max, y_min, y_max, plot_area,
                                    );
                                    renderer.draw_marker(
                                        px,
                                        py,
                                        marker_size_px,
                                        MarkerStyle::Circle,
                                        color,
                                    )?;
                                }
                            }
                        }
                        _ => {} // Already handled above
                    }
                }
            }
        }

        // Convert renderer output to Image
        Ok(renderer.to_image())
    }

    /// Render the plot to an external renderer (used for subplots)
    pub fn render_to_renderer(&self, renderer: &mut SkiaRenderer, dpi: f32) -> Result<()> {
        // Validate we have at least one series
        if self.series.is_empty() {
            return Err(PlottingError::NoDataSeries);
        }

        // Validate all series data (same validation as render method)
        for (_i, series) in self.series.iter().enumerate() {
            match &series.series_type {
                SeriesType::Line { x_data, y_data } | SeriesType::Scatter { x_data, y_data } => {
                    if x_data.len() != y_data.len() {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: x_data.len(),
                            y_len: y_data.len(),
                        });
                    }
                    if x_data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::Bar { categories, values } => {
                    if categories.len() != values.len() {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: categories.len(),
                            y_len: values.len(),
                        });
                    }
                    if categories.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::ErrorBars {
                    x_data,
                    y_data,
                    y_errors,
                } => {
                    if x_data.len() != y_data.len() || y_data.len() != y_errors.len() {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: x_data.len(),
                            y_len: y_data.len(),
                        });
                    }
                }
                SeriesType::ErrorBarsXY {
                    x_data,
                    y_data,
                    x_errors,
                    y_errors,
                } => {
                    if x_data.len() != y_data.len()
                        || x_data.len() != x_errors.len()
                        || x_data.len() != y_errors.len()
                    {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: x_data.len(),
                            y_len: y_data.len(),
                        });
                    }
                }
                SeriesType::Histogram { data, .. } => {
                    if data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::BoxPlot { data, .. } => {
                    if data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
            }
        }

        // Calculate data bounds across all series
        let (x_min, x_max, y_min, y_max) = self.calculate_data_bounds()?;

        // Extract bar chart categories if present (for categorical x-axis labels)
        let bar_categories: Option<Vec<String>> = self.series.iter().find_map(|s| {
            if let SeriesType::Bar { categories, .. } = &s.series_type {
                Some(categories.clone())
            } else {
                None
            }
        });

        // Choose layout method based on MarginConfig
        let (plot_area, layout_opt): (tiny_skia::Rect, Option<PlotLayout>) = match &self
            .config
            .margins
        {
            MarginConfig::ContentDriven {
                edge_buffer,
                center_plot,
            } => {
                // Use content-driven layout calculator
                let content = self.create_plot_content(y_min, y_max);
                let layout_config = LayoutConfig {
                    edge_buffer_pt: *edge_buffer,
                    center_plot: *center_plot,
                    ..Default::default()
                };
                let calculator = LayoutCalculator::new(layout_config);
                let layout = calculator.compute(
                    (renderer.width(), renderer.height()),
                    &content,
                    &self.config.typography,
                    &self.config.spacing,
                    dpi,
                );
                let skia_rect = tiny_skia::Rect::from_ltrb(
                    layout.plot_area.left,
                    layout.plot_area.top,
                    layout.plot_area.right,
                    layout.plot_area.bottom,
                )
                .ok_or(PlottingError::InvalidData {
                    message: "Invalid plot area from layout".to_string(),
                    position: None,
                })?;
                (skia_rect, Some(layout))
            }
            _ => {
                // Use legacy margin-based layout
                let margins = self.config.compute_margins(
                    self.title.is_some(),
                    self.xlabel.is_some(),
                    self.ylabel.is_some(),
                );
                let plot_area =
                    calculate_plot_area_config(renderer.width(), renderer.height(), &margins, dpi);
                (plot_area, None)
            }
        };

        // Generate nice tick values
        let x_ticks = generate_ticks(x_min, x_max, 8);
        let y_ticks = generate_ticks(y_min, y_max, 6);

        // Convert ticks to pixel coordinates
        let x_tick_pixels: Vec<f32> = x_ticks
            .iter()
            .map(|&tick| map_data_to_pixels(tick, 0.0, x_min, x_max, y_min, y_max, plot_area).0)
            .collect();
        let y_tick_pixels: Vec<f32> = y_ticks
            .iter()
            .map(|&tick| map_data_to_pixels(0.0, tick, x_min, x_max, y_min, y_max, plot_area).1)
            .collect();

        // Draw grid if enabled
        if self.grid.enabled {
            let grid_width_px = pt_to_px(self.config.lines.grid_width, dpi);
            renderer.draw_grid(
                &x_tick_pixels,
                &y_tick_pixels,
                plot_area,
                self.theme.grid_color,
                LineStyle::Solid,
                grid_width_px,
            )?;
        }

        // Draw axes and labels based on layout method
        if let Some(ref layout) = layout_opt {
            // Content-driven layout: use computed positions
            let tick_size_px = pt_to_px(self.config.typography.tick_size(), dpi);

            // Draw tick labels using layout positions
            // Use categorical labels for bar charts, numeric for others
            if let Some(ref categories) = bar_categories {
                renderer.draw_axis_labels_at_categorical(
                    &layout.plot_area,
                    categories,
                    y_min,
                    y_max,
                    layout.xtick_baseline_y,
                    layout.ytick_right_x,
                    tick_size_px,
                    self.theme.foreground,
                    dpi,
                )?;
            } else {
                renderer.draw_axis_labels_at(
                    &layout.plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    layout.xtick_baseline_y,
                    layout.ytick_right_x,
                    tick_size_px,
                    self.theme.foreground,
                    dpi,
                )?;
            }

            // Draw title if present
            if let Some(ref pos) = layout.title_pos {
                if let Some(ref title) = self.title {
                    renderer.draw_title_at(pos, title, self.theme.foreground)?;
                }
            }

            // Draw xlabel if present
            if let Some(ref pos) = layout.xlabel_pos {
                if let Some(ref xlabel) = self.xlabel {
                    renderer.draw_xlabel_at(pos, xlabel, self.theme.foreground)?;
                }
            }

            // Draw ylabel if present
            if let Some(ref pos) = layout.ylabel_pos {
                if let Some(ref ylabel) = self.ylabel {
                    renderer.draw_ylabel_at(pos, ylabel, self.theme.foreground)?;
                }
            }
        } else {
            // Legacy layout: use old positioning logic
            renderer.draw_axes(
                plot_area,
                &x_tick_pixels,
                &y_tick_pixels,
                self.theme.foreground,
            )?;

            // Draw title if present
            if let Some(ref title) = self.title {
                let title_size_px = pt_to_px(self.config.typography.title_size(), dpi);
                renderer.draw_title(
                    title,
                    plot_area,
                    self.theme.foreground,
                    title_size_px,
                    dpi,
                    &self.config.spacing,
                )?;
            }

            // Draw axis labels if present (legacy mode)
            let margins = self.config.compute_margins(
                self.title.is_some(),
                self.xlabel.is_some(),
                self.ylabel.is_some(),
            );

            if let Some(ref xlabel) = self.xlabel {
                let label_size = pt_to_px(self.config.typography.label_size(), dpi);
                let xlabel_y = renderer.height() as f32 - margins.bottom_px(dpi) * 0.3;
                renderer.draw_text_centered(
                    xlabel,
                    renderer.width() as f32 / 2.0,
                    xlabel_y,
                    label_size,
                    self.theme.foreground,
                )?;
            }

            if let Some(ref ylabel) = self.ylabel {
                let label_size = pt_to_px(self.config.typography.label_size(), dpi);
                let estimated_text_width = ylabel.len() as f32 * label_size * 0.8;
                let ylabel_x = (estimated_text_width * 0.6).max(margins.left_px(dpi) * 0.3);
                renderer.draw_text_rotated(
                    ylabel,
                    ylabel_x,
                    renderer.height() as f32 / 2.0,
                    label_size,
                    self.theme.foreground,
                )?;
            }
        }

        // Render each data series
        let mut color_index = 0;
        for series in &self.series {
            // Get series styling with defaults
            let color = series.color.unwrap_or_else(|| {
                let palette = Color::default_palette();
                palette[color_index % palette.len()]
            });
            // Use config data line width, or series override if specified
            let line_width_pt = series.line_width.unwrap_or(self.config.lines.data_width);
            let line_width = pt_to_px(line_width_pt, dpi);
            let line_style = series.line_style.clone().unwrap_or(LineStyle::Solid);
            let marker_style = series.marker_style.unwrap_or(MarkerStyle::Circle);

            match &series.series_type {
                SeriesType::Line { x_data, y_data } => {
                    // Convert data to pixel coordinates
                    let mut points = Vec::new();
                    for i in 0..x_data.len() {
                        let x_val = x_data[i];
                        let y_val = y_data[i];
                        if x_val.is_finite() && y_val.is_finite() {
                            let (px, py) = map_data_to_pixels(
                                x_val, y_val, x_min, x_max, y_min, y_max, plot_area,
                            );
                            points.push((px, py));
                        }
                    }

                    if points.len() >= 2 {
                        renderer.draw_polyline(&points, color, line_width, line_style)?;
                    }
                }
                SeriesType::Scatter { x_data, y_data } => {
                    // Draw individual markers
                    let marker_size_px = pt_to_px(series.marker_size.unwrap_or(8.0), dpi);
                    for i in 0..x_data.len() {
                        let x_val = x_data[i];
                        let y_val = y_data[i];
                        if x_val.is_finite() && y_val.is_finite() {
                            let (px, py) = map_data_to_pixels(
                                x_val, y_val, x_min, x_max, y_min, y_max, plot_area,
                            );
                            renderer.draw_marker(px, py, marker_size_px, marker_style, color)?;
                        }
                    }
                }
                SeriesType::Bar { categories, values } => {
                    // Calculate bar width based on data density
                    let bar_width = if categories.len() > 1 {
                        let available_width = plot_area.width() * 0.8;
                        (available_width / categories.len() as f32).min(pt_to_px(30.0, dpi))
                    } else {
                        pt_to_px(30.0, dpi) // Default bar width
                    };

                    // Draw bars from baseline to data value
                    let baseline =
                        map_data_to_pixels(0.0, 0.0, x_min, x_max, y_min, y_max, plot_area).1;

                    for (i, &value) in values.iter().enumerate() {
                        if value.is_finite() {
                            let x_val = i as f64;
                            let (px, py) = map_data_to_pixels(
                                x_val, value, x_min, x_max, y_min, y_max, plot_area,
                            );
                            let bar_height = (baseline - py).abs();
                            let bar_x = px - bar_width * 0.5;

                            if value >= 0.0 {
                                renderer.draw_rectangle(
                                    bar_x, py, bar_width, bar_height, color, true,
                                )?;
                            } else {
                                renderer.draw_rectangle(
                                    bar_x, baseline, bar_width, bar_height, color, true,
                                )?;
                            }
                        }
                    }
                }
                _ => {
                    // For unsupported plot types (error bars), render as scatter points
                    match &series.series_type {
                        SeriesType::ErrorBars { x_data, y_data, .. }
                        | SeriesType::ErrorBarsXY { x_data, y_data, .. } => {
                            let marker_size_px = pt_to_px(series.marker_size.unwrap_or(8.0), dpi);
                            for i in 0..x_data.len() {
                                let x_val = x_data[i];
                                let y_val = y_data[i];
                                if x_val.is_finite() && y_val.is_finite() {
                                    let (px, py) = map_data_to_pixels(
                                        x_val, y_val, x_min, x_max, y_min, y_max, plot_area,
                                    );
                                    renderer.draw_marker(
                                        px,
                                        py,
                                        marker_size_px,
                                        MarkerStyle::Circle,
                                        color,
                                    )?;
                                }
                            }
                        }
                        _ => {} // Already handled above
                    }
                }
            }
            color_index += 1;
        }

        Ok(())
    }

    /// Calculate total number of data points across all series
    fn calculate_total_points(&self) -> usize {
        self.series
            .iter()
            .map(|series| match &series.series_type {
                SeriesType::Line { x_data, .. }
                | SeriesType::Scatter { x_data, .. }
                | SeriesType::ErrorBars { x_data, .. }
                | SeriesType::ErrorBarsXY { x_data, .. } => x_data.len(),
                SeriesType::Bar { categories, .. } => categories.len(),
                SeriesType::Histogram { data, .. } => data.len(),
                SeriesType::BoxPlot { data, .. } => data.len(),
            })
            .sum()
    }

    /// Create PlotContent for layout calculation
    fn create_plot_content(&self, y_min: f64, y_max: f64) -> PlotContent {
        // Estimate max characters in y-tick labels
        let y_ticks = generate_ticks(y_min, y_max, 6);
        let max_ytick_chars = y_ticks
            .iter()
            .map(|&v| {
                if v.abs() < 0.001 {
                    1 // "0"
                } else if v.abs() > 1000.0 {
                    format!("{:.0e}", v).len()
                } else {
                    format!("{:.1}", v).len()
                }
            })
            .max()
            .unwrap_or(5);

        PlotContent {
            title: self.title.clone(),
            xlabel: self.xlabel.clone(),
            ylabel: self.ylabel.clone(),
            max_ytick_chars,
            max_xtick_chars: 5, // Reasonable default
        }
    }

    /// Render plot using DataShader optimization for large datasets
    fn render_with_datashader(&self) -> Result<Image> {
        // Calculate combined data bounds across all series
        let mut all_points = Vec::new();

        // Collect all points from all series
        for series in &self.series {
            match &series.series_type {
                SeriesType::Line { x_data, y_data } | SeriesType::Scatter { x_data, y_data } => {
                    for i in 0..x_data.len() {
                        let x = x_data[i];
                        let y = y_data[i];
                        if x.is_finite() && y.is_finite() {
                            all_points.push(crate::core::types::Point2f::new(x as f32, y as f32));
                        }
                    }
                }
                SeriesType::ErrorBars { x_data, y_data, .. }
                | SeriesType::ErrorBarsXY { x_data, y_data, .. } => {
                    for i in 0..x_data.len() {
                        let x = x_data[i];
                        let y = y_data[i];
                        if x.is_finite() && y.is_finite() {
                            all_points.push(crate::core::types::Point2f::new(x as f32, y as f32));
                        }
                    }
                }
                SeriesType::Bar { values, .. } => {
                    // For bar charts, convert category indices to points
                    for (i, &value) in values.iter().enumerate() {
                        if value.is_finite() {
                            all_points
                                .push(crate::core::types::Point2f::new(i as f32, value as f32));
                        }
                    }
                }
                SeriesType::Histogram { data, config } => {
                    // Calculate histogram and add bin center points
                    if let Ok(hist_data) =
                        crate::plots::histogram::calculate_histogram(data, config)
                    {
                        for (i, &count) in hist_data.counts.iter().enumerate() {
                            if count > 0.0 {
                                let x_center =
                                    (hist_data.bin_edges[i] + hist_data.bin_edges[i + 1]) / 2.0;
                                all_points.push(crate::core::types::Point2f::new(
                                    x_center as f32,
                                    count as f32,
                                ));
                            }
                        }
                    }
                }
                SeriesType::BoxPlot { data, .. } => {
                    if data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
            }
        }

        if all_points.is_empty() {
            return Err(PlottingError::EmptyDataSet);
        }

        // Simple DataShader implementation - create basic aggregated image
        let mut datashader =
            DataShader::with_canvas_size(self.dimensions.0 as usize, self.dimensions.1 as usize);

        // Convert points to (f64, f64) format for aggregation
        let points_f64: Vec<(f64, f64)> = all_points
            .iter()
            .map(|p| (p.x as f64, p.y as f64))
            .collect();

        // Aggregate points (this will auto-set bounds)
        let x_data: Vec<f64> = points_f64.iter().map(|p| p.0).collect();
        let y_data: Vec<f64> = points_f64.iter().map(|p| p.1).collect();

        datashader.aggregate(&x_data, &y_data)?;
        let ds_image = datashader.render();

        // Convert to Image format
        let image = Image {
            width: ds_image.width as u32,
            height: ds_image.height as u32,
            pixels: ds_image.pixels,
        };

        Ok(image)
    }

    /// Render plot using parallel processing for multiple series
    #[cfg(feature = "parallel")]
    fn render_with_parallel(&self) -> Result<Image> {
        use crate::render::parallel::{DataBounds, PlotArea, RenderSeriesType};

        // Start timing for performance measurement
        let start_time = std::time::Instant::now();

        // Create renderer with DPI scaling
        let (scaled_width, scaled_height) = self.dpi_scaled_dimensions();
        let mut renderer = SkiaRenderer::new(scaled_width, scaled_height, self.theme.clone())?;
        let plot_area = calculate_plot_area_dpi(scaled_width, scaled_height, self.dpi_scale());

        // Convert to parallel renderer format
        let parallel_plot_area = PlotArea {
            left: plot_area.left(),
            right: plot_area.right(),
            top: plot_area.top(),
            bottom: plot_area.bottom(),
        };

        // Calculate data bounds across all series (sequential - small operation)
        let bounds = self.calculate_data_bounds()?;
        let data_bounds = DataBounds {
            x_min: bounds.0,
            x_max: bounds.1,
            y_min: bounds.2,
            y_max: bounds.3,
        };

        // Generate nice tick values
        let x_ticks = generate_ticks(bounds.0, bounds.1, 8);
        let y_ticks = generate_ticks(bounds.2, bounds.3, 6);

        // Convert ticks to pixel coordinates
        let x_tick_pixels: Vec<f32> = x_ticks
            .iter()
            .map(|&tick| {
                map_data_to_pixels(tick, 0.0, bounds.0, bounds.1, bounds.2, bounds.3, plot_area).0
            })
            .collect();
        let y_tick_pixels: Vec<f32> = y_ticks
            .iter()
            .map(|&tick| {
                map_data_to_pixels(0.0, tick, bounds.0, bounds.1, bounds.2, bounds.3, plot_area).1
            })
            .collect();

        // Draw grid if enabled (sequential - UI elements)
        if self.grid.enabled {
            renderer.draw_grid(
                &x_tick_pixels,
                &y_tick_pixels,
                plot_area,
                self.theme.grid_color,
                LineStyle::Solid,
                self.dpi_scaled_line_width(1.0),
            )?;
        }

        // Draw axes (sequential - UI elements)
        renderer.draw_axes(
            plot_area,
            &x_tick_pixels,
            &y_tick_pixels,
            self.theme.foreground,
        )?;

        // Process all series in parallel
        let processed_series = self.parallel_renderer.process_series_parallel(
            &self.series,
            |series, index| -> Result<SeriesRenderData> {
                // Get series styling with defaults
                let color = series.color.unwrap_or_else(|| self.theme.get_color(index));
                let line_width =
                    self.dpi_scaled_line_width(series.line_width.unwrap_or(self.theme.line_width));
                let alpha = series.alpha.unwrap_or(1.0);

                // Process each series type
                let render_series_type = match &series.series_type {
                    SeriesType::Line { x_data, y_data } => {
                        // Transform coordinates in parallel
                        let points = self.parallel_renderer.transform_coordinates_parallel(
                            x_data,
                            y_data,
                            data_bounds.clone(),
                            parallel_plot_area.clone(),
                        )?;

                        // Process line segments in parallel
                        let segments = self.parallel_renderer.process_polyline_parallel(
                            &points,
                            series.line_style.clone().unwrap_or(LineStyle::Solid),
                            color,
                            line_width,
                        )?;

                        RenderSeriesType::Line { segments }
                    }
                    SeriesType::Scatter { x_data, y_data } => {
                        // Transform coordinates in parallel
                        let points = self.parallel_renderer.transform_coordinates_parallel(
                            x_data,
                            y_data,
                            data_bounds.clone(),
                            parallel_plot_area.clone(),
                        )?;

                        // Process markers in parallel
                        let markers = self.parallel_renderer.process_markers_parallel(
                            &points,
                            series.marker_style.unwrap_or(MarkerStyle::Circle),
                            color,
                            8.0, // Default marker size
                        )?;

                        RenderSeriesType::Scatter { markers }
                    }
                    SeriesType::Bar { categories, values } => {
                        // Convert categories to x-coordinates
                        let x_data: Vec<f64> = (0..categories.len()).map(|i| i as f64).collect();

                        // Transform coordinates
                        let points = self.parallel_renderer.transform_coordinates_parallel(
                            &x_data,
                            values,
                            data_bounds.clone(),
                            parallel_plot_area.clone(),
                        )?;

                        // Create bar instances
                        let bar_width = if categories.len() > 1 {
                            let available_width = parallel_plot_area.width() * 0.8;
                            (available_width / categories.len() as f32).min(40.0)
                        } else {
                            40.0
                        };

                        let baseline_y = map_data_to_pixels(
                            0.0, 0.0, bounds.0, bounds.1, bounds.2, bounds.3, plot_area,
                        )
                        .1;

                        let bars = points
                            .iter()
                            .enumerate()
                            .map(|(i, point)| {
                                let height = (baseline_y - point.y).abs();
                                crate::render::parallel::BarInstance {
                                    x: point.x - bar_width * 0.5,
                                    y: if values[i] >= 0.0 {
                                        point.y
                                    } else {
                                        baseline_y
                                    },
                                    width: bar_width,
                                    height,
                                    color,
                                }
                            })
                            .collect();

                        RenderSeriesType::Bar { bars }
                    }
                    SeriesType::ErrorBars { x_data, y_data, .. }
                    | SeriesType::ErrorBarsXY { x_data, y_data, .. } => {
                        // For now, render error bars as scatter points
                        // Full error bar implementation would be added here
                        let points = self.parallel_renderer.transform_coordinates_parallel(
                            x_data,
                            y_data,
                            data_bounds.clone(),
                            parallel_plot_area.clone(),
                        )?;

                        let markers = self.parallel_renderer.process_markers_parallel(
                            &points,
                            MarkerStyle::Circle,
                            color,
                            6.0,
                        )?;

                        RenderSeriesType::Scatter { markers }
                    }
                    SeriesType::Histogram { data, config } => {
                        // Calculate histogram data
                        let hist_data = crate::plots::histogram::calculate_histogram(data, config)
                            .map_err(|e| {
                                PlottingError::RenderError(format!(
                                    "Histogram calculation failed: {}",
                                    e
                                ))
                            })?;

                        // Convert histogram to bar format for parallel rendering
                        let x_data: Vec<f64> = hist_data
                            .bin_edges
                            .windows(2)
                            .map(|w| (w[0] + w[1]) / 2.0) // bin centers
                            .collect();

                        let points = self.parallel_renderer.transform_coordinates_parallel(
                            &x_data,
                            &hist_data.counts,
                            data_bounds.clone(),
                            parallel_plot_area.clone(),
                        )?;

                        // Create bar instances for histogram
                        let baseline_y = map_data_to_pixels(
                            0.0, 0.0, bounds.0, bounds.1, bounds.2, bounds.3, plot_area,
                        )
                        .1;

                        let bars = points
                            .iter()
                            .enumerate()
                            .map(|(i, point)| {
                                let bar_width =
                                    (hist_data.bin_edges[i + 1] - hist_data.bin_edges[i]) as f32;
                                let height = (baseline_y - point.y).abs();
                                crate::render::parallel::BarInstance {
                                    x: point.x - bar_width * 0.5,
                                    y: point.y,
                                    width: bar_width,
                                    height,
                                    color,
                                }
                            })
                            .collect();

                        RenderSeriesType::Bar { bars }
                    }
                    SeriesType::BoxPlot { data, config } => {
                        // Calculate box plot statistics
                        let box_data = crate::plots::boxplot::calculate_box_plot(data, config)
                            .map_err(|e| {
                                PlottingError::RenderError(format!(
                                    "Box plot calculation failed: {}",
                                    e
                                ))
                            })?;

                        // Transform coordinates for box plot elements
                        let x_center = 0.5; // Center the box plot
                        let box_width = 0.3; // Box width

                        // Map Y coordinates to plot area
                        let q1_y = map_data_to_pixels(
                            box_data.q1,
                            0.0,
                            bounds.0,
                            bounds.1,
                            bounds.2,
                            bounds.3,
                            plot_area,
                        )
                        .1;
                        let median_y = map_data_to_pixels(
                            box_data.median,
                            0.0,
                            bounds.0,
                            bounds.1,
                            bounds.2,
                            bounds.3,
                            plot_area,
                        )
                        .1;
                        let q3_y = map_data_to_pixels(
                            box_data.q3,
                            0.0,
                            bounds.0,
                            bounds.1,
                            bounds.2,
                            bounds.3,
                            plot_area,
                        )
                        .1;
                        let lower_whisker_y = map_data_to_pixels(
                            box_data.min,
                            0.0,
                            bounds.0,
                            bounds.1,
                            bounds.2,
                            bounds.3,
                            plot_area,
                        )
                        .1;
                        let upper_whisker_y = map_data_to_pixels(
                            box_data.max,
                            0.0,
                            bounds.0,
                            bounds.1,
                            bounds.2,
                            bounds.3,
                            plot_area,
                        )
                        .1;

                        // Map X coordinate
                        let x_center_px = map_data_to_pixels(
                            x_center, 0.0, bounds.0, bounds.1, bounds.2, bounds.3, plot_area,
                        )
                        .0;
                        let box_left = x_center_px - box_width * plot_area.width() * 0.5;
                        let box_right = x_center_px + box_width * plot_area.width() * 0.5;

                        // Transform outliers
                        let mut outliers = Vec::new();
                        for &outlier in &box_data.outliers {
                            let outlier_y = map_data_to_pixels(
                                outlier, 0.0, bounds.0, bounds.1, bounds.2, bounds.3, plot_area,
                            )
                            .1;
                            outliers.push(crate::core::types::Point2f {
                                x: x_center_px,
                                y: outlier_y,
                            });
                        }

                        let box_render_data = crate::render::parallel::BoxPlotRenderData {
                            x_center: x_center_px,
                            box_left,
                            box_right,
                            q1_y,
                            median_y,
                            q3_y,
                            lower_whisker_y,
                            upper_whisker_y,
                            outliers,
                            box_color: color,
                            line_color: color,
                            outlier_color: color,
                        };

                        RenderSeriesType::BoxPlot {
                            box_data: box_render_data,
                        }
                    }
                };

                Ok(SeriesRenderData {
                    series_type: render_series_type,
                    color,
                    line_width,
                    alpha,
                    label: series.label.clone(),
                })
            },
        )?;

        // Render processed series (sequential - final drawing)
        for processed in processed_series {
            match processed.series_type {
                RenderSeriesType::Line { segments } => {
                    // Draw all line segments
                    for segment in segments {
                        renderer.draw_polyline(
                            &[
                                (segment.start.x, segment.start.y),
                                (segment.end.x, segment.end.y),
                            ],
                            segment.color,
                            segment.width,
                            segment.style,
                        )?;
                    }
                }
                RenderSeriesType::Scatter { markers } => {
                    // Draw all markers
                    for marker in markers {
                        renderer.draw_marker(
                            marker.position.x,
                            marker.position.y,
                            marker.size,
                            marker.style,
                            marker.color,
                        )?;
                    }
                }
                RenderSeriesType::Bar { bars } => {
                    // Draw all bars
                    for bar in bars {
                        renderer
                            .draw_rectangle(bar.x, bar.y, bar.width, bar.height, bar.color, true)?;
                    }
                }
                RenderSeriesType::BoxPlot { box_data } => {
                    // Draw box plot components

                    // Draw the box (IQR)
                    renderer.draw_rectangle(
                        box_data.box_left,
                        box_data.q3_y,
                        box_data.box_right - box_data.box_left,
                        box_data.q1_y - box_data.q3_y,
                        box_data.box_color,
                        false, // outline only
                    )?;

                    // Draw median line
                    renderer.draw_line(
                        box_data.box_left,
                        box_data.median_y,
                        box_data.box_right,
                        box_data.median_y,
                        box_data.line_color,
                        2.0, // median line width
                        LineStyle::Solid,
                    )?;

                    // Draw lower whisker
                    renderer.draw_line(
                        box_data.x_center,
                        box_data.q1_y,
                        box_data.x_center,
                        box_data.lower_whisker_y,
                        box_data.line_color,
                        1.0,
                        LineStyle::Solid,
                    )?;

                    // Draw upper whisker
                    renderer.draw_line(
                        box_data.x_center,
                        box_data.q3_y,
                        box_data.x_center,
                        box_data.upper_whisker_y,
                        box_data.line_color,
                        1.0,
                        LineStyle::Solid,
                    )?;

                    // Draw whisker caps
                    let cap_width = (box_data.box_right - box_data.box_left) * 0.3;
                    renderer.draw_line(
                        box_data.x_center - cap_width,
                        box_data.lower_whisker_y,
                        box_data.x_center + cap_width,
                        box_data.lower_whisker_y,
                        box_data.line_color,
                        1.0,
                        LineStyle::Solid,
                    )?;

                    renderer.draw_line(
                        box_data.x_center - cap_width,
                        box_data.upper_whisker_y,
                        box_data.x_center + cap_width,
                        box_data.upper_whisker_y,
                        box_data.line_color,
                        1.0,
                        LineStyle::Solid,
                    )?;

                    // Draw outliers
                    for outlier in &box_data.outliers {
                        renderer.draw_marker(
                            outlier.x,
                            outlier.y,
                            4.0, // outlier marker size
                            MarkerStyle::Circle,
                            box_data.outlier_color,
                        )?;
                    }
                }
                RenderSeriesType::ErrorBars { .. } => {
                    // Error bars implementation would go here
                }
            }
        }

        // Record performance statistics
        let duration = start_time.elapsed();
        let total_points = self.calculate_total_points();

        // Log performance info (could be optional/debug in production)
        let stats = self.parallel_renderer.performance_stats();
        println!(
            "⚡ Parallel: {} series, {} points in {:.1}ms ({:.1}x speedup, {} threads)",
            self.series.len(),
            total_points,
            duration.as_millis(),
            stats.estimated_speedup,
            stats.configured_threads
        );

        // Convert renderer output to Image
        Ok(renderer.to_image())
    }

    /// Calculate data bounds across all series
    fn calculate_data_bounds(&self) -> Result<(f64, f64, f64, f64)> {
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for series in &self.series {
            match &series.series_type {
                SeriesType::Line { x_data, y_data } | SeriesType::Scatter { x_data, y_data } => {
                    for i in 0..x_data.len() {
                        let x_val = x_data[i];
                        let y_val = y_data[i];

                        if x_val.is_finite() {
                            x_min = x_min.min(x_val);
                            x_max = x_max.max(x_val);
                        }
                        if y_val.is_finite() {
                            y_min = y_min.min(y_val);
                            y_max = y_max.max(y_val);
                        }
                    }
                }
                SeriesType::Bar { categories, values } => {
                    x_min = x_min.min(0.0);
                    x_max = x_max.max((categories.len() - 1) as f64);

                    for &val in values {
                        if val.is_finite() {
                            y_min = y_min.min(val.min(0.0));
                            y_max = y_max.max(val.max(0.0));
                        }
                    }
                }
                SeriesType::ErrorBars {
                    x_data,
                    y_data,
                    y_errors,
                } => {
                    for i in 0..x_data.len() {
                        let x_val = x_data[i];
                        let y_val = y_data[i];
                        let y_err = y_errors[i];

                        if x_val.is_finite() {
                            x_min = x_min.min(x_val);
                            x_max = x_max.max(x_val);
                        }
                        if y_val.is_finite() && y_err.is_finite() {
                            y_min = y_min.min(y_val - y_err);
                            y_max = y_max.max(y_val + y_err);
                        }
                    }
                }
                SeriesType::ErrorBarsXY {
                    x_data,
                    y_data,
                    x_errors,
                    y_errors,
                } => {
                    for i in 0..x_data.len() {
                        let x_val = x_data[i];
                        let y_val = y_data[i];
                        let x_err = x_errors[i];
                        let y_err = y_errors[i];

                        if x_val.is_finite() && x_err.is_finite() {
                            x_min = x_min.min(x_val - x_err);
                            x_max = x_max.max(x_val + x_err);
                        }
                        if y_val.is_finite() && y_err.is_finite() {
                            y_min = y_min.min(y_val - y_err);
                            y_max = y_max.max(y_val + y_err);
                        }
                    }
                }
                SeriesType::Histogram { data, config } => {
                    // Calculate histogram to get data bounds
                    if let Ok(hist_data) =
                        crate::plots::histogram::calculate_histogram(data, config)
                    {
                        // X bounds from bin edges
                        if !hist_data.bin_edges.is_empty() {
                            x_min = x_min.min(*hist_data.bin_edges.first().unwrap());
                            x_max = x_max.max(*hist_data.bin_edges.last().unwrap());
                        }

                        // Y bounds from counts (include zero baseline)
                        y_min = y_min.min(0.0);
                        for &count in &hist_data.counts {
                            if count.is_finite() && count > 0.0 {
                                y_max = y_max.max(count);
                            }
                        }
                    }
                }
                SeriesType::BoxPlot { data, .. } => {
                    if data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }

                    // Set x bounds for box plot (centered at 0.5)
                    x_min = x_min.min(0.0);
                    x_max = x_max.max(1.0);

                    // Y bounds include all data values
                    for &value in data {
                        if value.is_finite() {
                            y_min = y_min.min(value);
                            y_max = y_max.max(value);
                        }
                    }
                }
            }
        }

        // Handle edge cases
        if (x_max - x_min).abs() < f64::EPSILON {
            x_min -= 1.0;
            x_max += 1.0;
        }
        if (y_max - y_min).abs() < f64::EPSILON {
            y_min -= 1.0;
            y_max += 1.0;
        }

        Ok((x_min, x_max, y_min, y_max))
    }

    /// Automatically optimize backend selection based on data size
    ///
    /// Selects the most appropriate rendering backend based on dataset characteristics:
    /// - < 1K points: Skia (simple, fast)
    /// - 1K-10K points: Parallel (multi-threaded)
    /// - 10K-100K points: Parallel (optimized)
    /// - > 100K points: GPU/DataShader (hardware acceleration)
    ///
    /// If a backend was explicitly set with `.backend()`, that choice is respected.
    pub fn auto_optimize(mut self) -> Self {
        // If backend already explicitly set, respect that choice
        if self.backend.is_some() {
            self.auto_optimized = true;
            return self;
        }

        // Count total data points across all series
        let total_points = self
            .series
            .iter()
            .map(|s| match &s.series_type {
                SeriesType::Line { x_data, .. } => x_data.len(),
                SeriesType::Scatter { x_data, .. } => x_data.len(),
                SeriesType::Bar { values, .. } => values.len(),
                SeriesType::Histogram { data, .. } => data.len(),
                SeriesType::BoxPlot { data, .. } => data.len(),
                SeriesType::ErrorBars { x_data, .. } => x_data.len(),
                SeriesType::ErrorBarsXY { x_data, .. } => x_data.len(),
            })
            .sum::<usize>();

        // Select backend based on data size
        let selected_backend = if total_points < 1000 {
            BackendType::Skia
        } else if total_points < 100_000 {
            #[cfg(feature = "parallel")]
            {
                BackendType::Parallel
            }
            #[cfg(not(feature = "parallel"))]
            {
                BackendType::Skia
            }
        } else {
            // For very large datasets, prefer GPU if available, else DataShader
            #[cfg(feature = "gpu")]
            {
                BackendType::GPU
            }
            #[cfg(not(feature = "gpu"))]
            {
                BackendType::DataShader
            }
        };

        self.backend = Some(selected_backend);
        self.auto_optimized = true;
        self
    }

    /// Set backend explicitly (overrides auto-optimization)
    pub fn backend(mut self, backend: BackendType) -> Self {
        self.backend = Some(backend);
        self
    }

    /// Get the current backend name (for testing)
    pub fn get_backend_name(&self) -> &'static str {
        match self.backend {
            Some(BackendType::Skia) => "skia",
            Some(BackendType::Parallel) => "parallel",
            Some(BackendType::GPU) => "gpu",
            Some(BackendType::DataShader) => "datashader",
            None => "auto",
        }
    }

    /// Save the plot to a PNG file
    pub fn save<P: AsRef<Path>>(self, path: P) -> Result<()> {
        use crate::render::skia::SkiaRenderer;

        // Validate data before rendering
        for series in &self.series {
            match &series.series_type {
                SeriesType::Line { x_data, y_data }
                | SeriesType::Scatter { x_data, y_data }
                | SeriesType::ErrorBars { x_data, y_data, .. }
                | SeriesType::ErrorBarsXY { x_data, y_data, .. } => {
                    // Check for empty data
                    if x_data.is_empty() || y_data.is_empty() {
                        return Err(crate::core::PlottingError::EmptyDataSet);
                    }
                    // Check for mismatched data lengths
                    if x_data.len() != y_data.len() {
                        return Err(crate::core::PlottingError::DataLengthMismatch {
                            x_len: x_data.len(),
                            y_len: y_data.len(),
                        });
                    }
                }
                SeriesType::Bar { categories, values } => {
                    // Check for empty data
                    if categories.is_empty() || values.is_empty() {
                        return Err(crate::core::PlottingError::EmptyDataSet);
                    }
                    // Check for mismatched data lengths
                    if categories.len() != values.len() {
                        return Err(crate::core::PlottingError::DataLengthMismatch {
                            x_len: categories.len(),
                            y_len: values.len(),
                        });
                    }
                }
                SeriesType::Histogram { data, .. } => {
                    // Check for empty data
                    if data.is_empty() {
                        return Err(crate::core::PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::BoxPlot { data, .. } => {
                    if data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
            }
        }

        // Create renderer and render the plot with DPI scaling
        let (scaled_width, scaled_height) = self.dpi_scaled_dimensions();
        let mut renderer = SkiaRenderer::new(scaled_width, scaled_height, self.theme.clone())?;

        // Clear background
        renderer.clear();

        // Calculate data bounds first (needed for layout calculation)
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for series in &self.series {
            match &series.series_type {
                SeriesType::Line { x_data, y_data }
                | SeriesType::Scatter { x_data, y_data }
                | SeriesType::ErrorBars { x_data, y_data, .. }
                | SeriesType::ErrorBarsXY { x_data, y_data, .. } => {
                    for (&x, &y) in x_data.iter().zip(y_data.iter()) {
                        x_min = x_min.min(x);
                        x_max = x_max.max(x);
                        y_min = y_min.min(y);
                        y_max = y_max.max(y);
                    }
                }
                SeriesType::Bar { values, .. } => {
                    for (i, &value) in values.iter().enumerate() {
                        let x = i as f64;
                        x_min = x_min.min(x);
                        x_max = x_max.max(x);
                        y_min = y_min.min(0.0).min(value);
                        y_max = y_max.max(value);
                    }
                }
                SeriesType::Histogram { data, config } => {
                    if let Ok(hist_data) =
                        crate::plots::histogram::calculate_histogram(data, config)
                    {
                        if let (Some(&first), Some(&last)) =
                            (hist_data.bin_edges.first(), hist_data.bin_edges.last())
                        {
                            x_min = x_min.min(first);
                            x_max = x_max.max(last);
                        }
                        y_min = y_min.min(0.0);
                        if let Some(&max_count) = hist_data
                            .counts
                            .iter()
                            .max_by(|a, b| a.partial_cmp(b).unwrap())
                        {
                            y_max = y_max.max(max_count);
                        }
                    }
                }
                SeriesType::BoxPlot { data, .. } => {
                    x_min = x_min.min(0.0);
                    x_max = x_max.max(1.0);
                    for &value in data {
                        if value.is_finite() {
                            y_min = y_min.min(value);
                            y_max = y_max.max(value);
                        }
                    }
                }
            }
        }

        // Add padding to data bounds
        let x_range = x_max - x_min;
        let y_range = y_max - y_min;
        x_min -= x_range * 0.05;
        x_max += x_range * 0.05;
        y_min -= y_range * 0.05;
        y_max += y_range * 0.05;

        // Extract bar chart categories if present (for categorical x-axis labels)
        let bar_categories: Option<Vec<String>> = self.series.iter().find_map(|s| {
            if let SeriesType::Bar { categories, .. } = &s.series_type {
                Some(categories.clone())
            } else {
                None
            }
        });

        let dpi = self.dpi as f32;

        // Calculate plot area based on MarginConfig
        let (plot_area, layout_opt): (tiny_skia::Rect, Option<PlotLayout>) =
            match &self.config.margins {
                MarginConfig::ContentDriven {
                    edge_buffer,
                    center_plot,
                } => {
                    // Use content-driven layout calculator
                    let content = self.create_plot_content(y_min, y_max);
                    let layout_config = LayoutConfig {
                        edge_buffer_pt: *edge_buffer,
                        center_plot: *center_plot,
                        ..Default::default()
                    };
                    let calculator = LayoutCalculator::new(layout_config);
                    let layout = calculator.compute(
                        (scaled_width, scaled_height),
                        &content,
                        &self.config.typography,
                        &self.config.spacing,
                        dpi,
                    );
                    let skia_rect = tiny_skia::Rect::from_ltrb(
                        layout.plot_area.left,
                        layout.plot_area.top,
                        layout.plot_area.right,
                        layout.plot_area.bottom,
                    )
                    .ok_or(PlottingError::InvalidData {
                        message: "Invalid plot area from layout".to_string(),
                        position: None,
                    })?;
                    (skia_rect, Some(layout))
                }
                _ => {
                    // Use legacy margin-based layout
                    let plot_area = crate::render::skia::calculate_plot_area_dpi(
                        scaled_width,
                        scaled_height,
                        self.dpi_scale(),
                    );
                    (plot_area, None)
                }
            };

        // Generate major and minor ticks for axes
        let x_major_ticks =
            crate::render::skia::generate_ticks(x_min, x_max, self.tick_config.major_ticks_x);
        let y_major_ticks =
            crate::render::skia::generate_ticks(y_min, y_max, self.tick_config.major_ticks_y);

        // Generate minor ticks if configured
        let x_minor_ticks = if self.tick_config.minor_ticks_x > 0 {
            crate::render::skia::generate_minor_ticks(
                &x_major_ticks,
                self.tick_config.minor_ticks_x,
            )
        } else {
            Vec::new()
        };
        let y_minor_ticks = if self.tick_config.minor_ticks_y > 0 {
            crate::render::skia::generate_minor_ticks(
                &y_major_ticks,
                self.tick_config.minor_ticks_y,
            )
        } else {
            Vec::new()
        };

        // Combine ticks for rendering based on grid mode
        let x_ticks = match self.tick_config.grid_mode {
            GridMode::MajorOnly => x_major_ticks.clone(),
            GridMode::MinorOnly => x_minor_ticks.clone(),
            GridMode::Both => {
                let mut combined = x_major_ticks.clone();
                combined.extend(x_minor_ticks.iter());
                combined.sort_by(|a, b| a.partial_cmp(b).unwrap());
                combined
            }
        };
        let y_ticks = match self.tick_config.grid_mode {
            GridMode::MajorOnly => y_major_ticks.clone(),
            GridMode::MinorOnly => y_minor_ticks.clone(),
            GridMode::Both => {
                let mut combined = y_major_ticks.clone();
                combined.extend(y_minor_ticks.iter());
                combined.sort_by(|a, b| a.partial_cmp(b).unwrap());
                combined
            }
        };

        let x_tick_pixels: Vec<f32> = x_ticks
            .iter()
            .map(|&x| {
                crate::render::skia::map_data_to_pixels(x, 0.0, x_min, x_max, 0.0, 1.0, plot_area).0
            })
            .collect();
        let y_tick_pixels: Vec<f32> = y_ticks
            .iter()
            .map(|&y| {
                crate::render::skia::map_data_to_pixels(0.0, y, 0.0, 1.0, y_min, y_max, plot_area).1
            })
            .collect();

        // Render grid if enabled
        if self.grid.enabled {
            renderer.draw_grid(
                &x_tick_pixels,
                &y_tick_pixels,
                plot_area,
                self.theme.grid_color,
                crate::render::LineStyle::Solid,
                self.dpi_scaled_line_width(1.0),
            )?;
        }

        // Convert tick values to pixel positions for major and minor ticks
        let x_major_tick_pixels: Vec<f32> = x_major_ticks
            .iter()
            .map(|&x| {
                crate::render::skia::map_data_to_pixels(x, 0.0, x_min, x_max, 0.0, 1.0, plot_area).0
            })
            .collect();
        let y_major_tick_pixels: Vec<f32> = y_major_ticks
            .iter()
            .map(|&y| {
                crate::render::skia::map_data_to_pixels(0.0, y, 0.0, 1.0, y_min, y_max, plot_area).1
            })
            .collect();

        let x_minor_tick_pixels: Vec<f32> = x_minor_ticks
            .iter()
            .map(|&x| {
                crate::render::skia::map_data_to_pixels(x, 0.0, x_min, x_max, 0.0, 1.0, plot_area).0
            })
            .collect();
        let y_minor_tick_pixels: Vec<f32> = y_minor_ticks
            .iter()
            .map(|&y| {
                crate::render::skia::map_data_to_pixels(0.0, y, 0.0, 1.0, y_min, y_max, plot_area).1
            })
            .collect();

        // Always draw axes with enhanced tick system
        renderer.draw_axes_with_config(
            plot_area,
            &x_major_tick_pixels,
            &y_major_tick_pixels,
            &x_minor_tick_pixels,
            &y_minor_tick_pixels,
            &self.tick_config.direction,
            self.theme.foreground,
        )?;

        // Draw axis labels, tick values, and title based on layout method
        if let Some(ref layout) = layout_opt {
            // Content-driven layout: use computed positions
            let tick_size_px = pt_to_px(self.config.typography.tick_size(), dpi);

            // Draw tick labels using layout positions
            // Use categorical labels for bar charts, numeric for others
            if let Some(ref categories) = bar_categories {
                renderer.draw_axis_labels_at_categorical(
                    &layout.plot_area,
                    categories,
                    y_min,
                    y_max,
                    layout.xtick_baseline_y,
                    layout.ytick_right_x,
                    tick_size_px,
                    self.theme.foreground,
                    dpi,
                )?;
            } else {
                renderer.draw_axis_labels_at(
                    &layout.plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    layout.xtick_baseline_y,
                    layout.ytick_right_x,
                    tick_size_px,
                    self.theme.foreground,
                    dpi,
                )?;
            }

            // Draw title if present
            if let Some(ref pos) = layout.title_pos {
                if let Some(ref title) = self.title {
                    renderer.draw_title_at(pos, title, self.theme.foreground)?;
                }
            }

            // Draw x-axis label if present
            if let Some(ref pos) = layout.xlabel_pos {
                if let Some(ref xlabel) = self.xlabel {
                    renderer.draw_xlabel_at(pos, xlabel, self.theme.foreground)?;
                }
            }

            // Draw y-axis label if present
            if let Some(ref pos) = layout.ylabel_pos {
                if let Some(ref ylabel) = self.ylabel {
                    renderer.draw_ylabel_at(pos, ylabel, self.theme.foreground)?;
                }
            }
        } else {
            // Legacy layout: use old methods
            let x_label = self.xlabel.as_deref().unwrap_or("X");
            let y_label = self.ylabel.as_deref().unwrap_or("Y");

            // Use categorical labels for bar charts, numeric for others
            if let Some(ref categories) = bar_categories {
                renderer.draw_axis_labels_with_categories(
                    plot_area,
                    categories,
                    y_min,
                    y_max,
                    &y_major_ticks,
                    x_label,
                    y_label,
                    self.theme.foreground,
                    self.dpi_scaled_font_size(14.0),
                    self.dpi_scale(),
                )?;
            } else {
                renderer.draw_axis_labels_with_ticks(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &x_major_ticks,
                    &y_major_ticks,
                    x_label,
                    y_label,
                    self.theme.foreground,
                    self.dpi_scaled_font_size(14.0),
                    self.dpi_scale(),
                )?;
            }

            // Draw title if present
            if let Some(ref title) = self.title {
                renderer.draw_title_legacy(
                    title,
                    plot_area,
                    self.theme.foreground,
                    self.dpi_scaled_font_size(16.0),
                    self.dpi_scale(),
                )?;
            }
        }

        // Check if we should use DataShader for large datasets
        let total_points: usize = self
            .series
            .iter()
            .map(|series| match &series.series_type {
                SeriesType::Line { x_data, .. }
                | SeriesType::Scatter { x_data, .. }
                | SeriesType::ErrorBars { x_data, .. }
                | SeriesType::ErrorBarsXY { x_data, .. } => x_data.len(),
                SeriesType::Bar { categories, .. } => categories.len(),
                SeriesType::Histogram { data, .. } => data.len(),
                SeriesType::BoxPlot { data, .. } => data.len(),
            })
            .sum();

        const DATASHADER_THRESHOLD: usize = 100_000; // Activate DataShader for >100K points

        if total_points > DATASHADER_THRESHOLD {
            // Use DataShader for massive datasets - simplified version
            use crate::data::DataShader;

            for series in &self.series {
                match &series.series_type {
                    SeriesType::Scatter { x_data, y_data }
                    | SeriesType::Line { x_data, y_data } => {
                        let mut datashader = DataShader::with_canvas_size(
                            plot_area.width() as usize,
                            plot_area.height() as usize,
                        );

                        datashader.aggregate(x_data, y_data)?;
                        let image = datashader.render();

                        // Draw the DataShader result
                        renderer.draw_datashader_image(&image, plot_area)?;
                    }
                    SeriesType::Histogram { data, config } => {
                        // For histograms, calculate bins and use DataShader for high density
                        let hist_data = crate::plots::histogram::calculate_histogram(data, config)
                            .map_err(|e| {
                                PlottingError::RenderError(format!(
                                    "Histogram calculation failed: {}",
                                    e
                                ))
                            })?;

                        // Convert histogram to x,y data for DataShader
                        let x_data: Vec<f64> = hist_data
                            .bin_edges
                            .windows(2)
                            .map(|w| (w[0] + w[1]) / 2.0)
                            .collect();
                        let y_data: Vec<f64> = hist_data.counts;

                        let mut datashader = DataShader::with_canvas_size(
                            plot_area.width() as usize,
                            plot_area.height() as usize,
                        );

                        datashader.aggregate(&x_data, &y_data)?;
                        let image = datashader.render();

                        // Draw the DataShader result
                        renderer.draw_datashader_image(&image, plot_area)?;
                    }
                    _ => {
                        // For other plot types, use normal rendering
                        self.render_series_normal(
                            series,
                            &mut renderer,
                            plot_area,
                            x_min,
                            x_max,
                            y_min,
                            y_max,
                        )?;
                    }
                }
            }
        } else {
            // Use normal rendering for smaller datasets
            for series in &self.series {
                self.render_series_normal(
                    series,
                    &mut renderer,
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                )?;
            }
        }

        // Collect legend items from series with labels
        let legend_items: Vec<(String, crate::render::Color)> = self
            .series
            .iter()
            .filter_map(|series| {
                series.label.as_ref().map(|label| {
                    let color = series.color.unwrap_or(self.theme.foreground);
                    (label.clone(), color)
                })
            })
            .collect();

        // Draw legend if there are labeled series and legend is enabled
        if !legend_items.is_empty() && self.legend.enabled {
            renderer.draw_legend_positioned(&legend_items, plot_area, self.legend.position)?;
        }

        // Save as PNG
        renderer.save_png(path)?;

        Ok(())
    }

    /// Save the plot to a PNG file with custom dimensions
    pub fn save_with_size<P: AsRef<Path>>(
        mut self,
        path: P,
        width: u32,
        height: u32,
    ) -> Result<()> {
        // Update dimensions
        self.dimensions = (width, height);
        self.save(path)
    }

    /// Export to SVG format
    pub fn export_svg<P: AsRef<Path>>(self, path: P) -> Result<()> {
        // Placeholder for SVG export
        let svg_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">
  <rect width="100%" height="100%" fill="white"/>
  <text x="50%" y="50%" text-anchor="middle">Ruviz Plot Placeholder</text>
</svg>"#,
            self.dimensions.0, self.dimensions.1
        );

        std::fs::write(path, svg_content).map_err(|e| PlottingError::IoError(e))?;

        Ok(())
    }
}

impl Default for Plot {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for configuring individual plot series
pub struct PlotSeriesBuilder {
    plot: Plot,
    series: PlotSeries,
}

impl PlotSeriesBuilder {
    fn new(plot: Plot, series: PlotSeries) -> Self {
        Self { plot, series }
    }

    /// Set series label for legend
    pub fn label<S: Into<String>>(mut self, label: S) -> Self {
        self.series.label = Some(label.into());
        self
    }

    /// Set series color
    pub fn color(mut self, color: Color) -> Self {
        self.series.color = Some(color);
        self
    }

    /// Set line width
    pub fn width(mut self, width: f32) -> Self {
        self.series.line_width = Some(width.max(0.1));
        self
    }

    /// Set line style
    pub fn style(mut self, style: LineStyle) -> Self {
        self.series.line_style = Some(style);
        self
    }

    /// Set marker style (for scatter plots)
    pub fn marker(mut self, marker: MarkerStyle) -> Self {
        self.series.marker_style = Some(marker);
        self
    }

    /// Set marker size (for scatter plots)
    pub fn marker_size(mut self, size: f32) -> Self {
        self.series.marker_size = Some(size.max(0.1));
        self
    }

    /// Set transparency
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.series.alpha = Some(alpha.clamp(0.0, 1.0));
        self
    }

    /// Finish configuring this series and return to the main Plot
    /// This consumes the builder and adds the series to the plot
    pub fn end_series(mut self) -> Plot {
        // Auto-assign color if none specified
        if self.series.color.is_none() {
            self.series.color = Some(self.plot.theme.get_color(self.plot.auto_color_index));
            self.plot.auto_color_index += 1;
        }

        self.plot.series.push(self.series);
        self.plot
    }
}

// Implement Deref so methods can be chained through the builder
impl std::ops::Deref for PlotSeriesBuilder {
    type Target = Plot;

    fn deref(&self) -> &Self::Target {
        &self.plot
    }
}

// Implement most Plot methods for PlotSeriesBuilder to allow chaining
impl PlotSeriesBuilder {
    /// Continue with a new line series
    pub fn line<X, Y>(mut self, x_data: &X, y_data: &Y) -> PlotSeriesBuilder
    where
        X: Data1D<f64>,
        Y: Data1D<f64>,
    {
        self.end_series().line(x_data, y_data)
    }

    /// Continue with a new scatter series  
    pub fn scatter<X, Y>(mut self, x_data: &X, y_data: &Y) -> PlotSeriesBuilder
    where
        X: Data1D<f64>,
        Y: Data1D<f64>,
    {
        self.end_series().scatter(x_data, y_data)
    }

    /// Continue with a new bar series
    pub fn bar<S, V>(mut self, categories: &[S], values: &V) -> PlotSeriesBuilder
    where
        S: ToString,
        V: Data1D<f64>,
    {
        self.end_series().bar(categories, values)
    }

    /// Set plot title
    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.plot.title = Some(title.into());
        self
    }

    /// Set X-axis label
    pub fn xlabel<S: Into<String>>(mut self, label: S) -> Self {
        self.plot.xlabel = Some(label.into());
        self
    }

    /// Set Y-axis label
    pub fn ylabel<S: Into<String>>(mut self, label: S) -> Self {
        self.plot.ylabel = Some(label.into());
        self
    }

    /// Configure legend
    pub fn legend(mut self, position: Position) -> Self {
        self.plot.legend.enabled = true;
        self.plot.legend.position = position;
        self
    }

    /// Enable/disable grid
    pub fn grid(mut self, enabled: bool) -> Self {
        self.plot.grid.enabled = enabled;
        self
    }

    /// Set DPI for export quality
    pub fn dpi(mut self, dpi: u32) -> Self {
        self.plot.dpi = dpi.max(72);
        self
    }

    /// Render the plot
    pub fn render(self) -> Result<Image> {
        self.end_series().render()
    }

    /// Save the plot to file
    pub fn save<P: AsRef<Path>>(self, path: P) -> Result<()> {
        self.end_series().save(path)
    }

    /// Save the plot to file with custom dimensions
    pub fn save_with_size<P: AsRef<Path>>(
        mut self,
        path: P,
        width: u32,
        height: u32,
    ) -> Result<()> {
        self.plot.dimensions = (width, height);
        self.end_series().save(path)
    }

    /// Export to SVG
    pub fn export_svg<P: AsRef<Path>>(self, path: P) -> Result<()> {
        self.end_series().export_svg(path)
    }

    /// Automatically optimize backend selection (fluent API)
    /// Note: This ends the current series before optimizing
    pub fn auto_optimize(self) -> Plot {
        self.end_series().auto_optimize()
    }

    /// Set backend explicitly (fluent API)
    /// Note: This ends the current series before setting backend
    pub fn backend(self, backend: BackendType) -> Plot {
        self.end_series().backend(backend)
    }

    /// Get current backend name (for testing)
    pub fn get_backend_name(&self) -> &'static str {
        self.plot.get_backend_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_theme_method() {
        use crate::render::Theme;

        let plot = Plot::new();
        let theme = plot.get_theme();

        // Should return a valid theme (can't compare directly since Theme doesn't implement PartialEq)
        // Just ensure the method works without panicking

        // Test with custom theme - just ensure get/set works
        let custom_theme = Theme::dark();
        let plot = Plot::new().theme(custom_theme);
        let _retrieved_theme = plot.get_theme();
        // Test passes if no panic occurs
    }

    #[test]
    fn test_render_to_renderer_basic() {
        use crate::render::{SkiaRenderer, Theme};

        let x_data = vec![1.0, 2.0, 3.0];
        let y_data = vec![2.0, 4.0, 3.0];

        let plot = Plot::new()
            .line(&x_data, &y_data)
            .title("Test Plot")
            .xlabel("X")
            .ylabel("Y")
            .end_series();

        let mut renderer = SkiaRenderer::new(400, 300, Theme::default()).unwrap();

        // Should render without error
        let result = plot.render_to_renderer(&mut renderer, 96.0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_to_renderer_empty_series() {
        use crate::render::{SkiaRenderer, Theme};

        let plot = Plot::new().title("Empty Plot");
        let mut renderer = SkiaRenderer::new(400, 300, Theme::default()).unwrap();

        // Should fail with no data series
        let result = plot.render_to_renderer(&mut renderer, 96.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_render_to_renderer_multiple_series() {
        use crate::render::{SkiaRenderer, Theme};

        let x1 = vec![1.0, 2.0, 3.0];
        let y1 = vec![2.0, 4.0, 3.0];
        let x2 = vec![1.5, 2.5, 3.5];
        let y2 = vec![1.0, 3.0, 2.0];

        let plot = Plot::new()
            .line(&x1, &y1)
            .label("Series 1")
            .line(&x2, &y2)
            .label("Series 2")
            .title("Multi-series Plot")
            .end_series();

        let mut renderer = SkiaRenderer::new(400, 300, Theme::default()).unwrap();

        // Should render multiple series without error
        let result = plot.render_to_renderer(&mut renderer, 96.0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_to_renderer_dpi_scaling() {
        use crate::render::{SkiaRenderer, Theme};

        let x_data = vec![1.0, 2.0, 3.0];
        let y_data = vec![2.0, 4.0, 3.0];

        let plot = Plot::new()
            .line(&x_data, &y_data)
            .title("DPI Test")
            .end_series();

        let mut renderer = SkiaRenderer::new(400, 300, Theme::default()).unwrap();

        // Test different DPI values
        let result_96 = plot.clone().render_to_renderer(&mut renderer, 96.0);
        assert!(result_96.is_ok());

        let result_144 = plot.clone().render_to_renderer(&mut renderer, 144.0);
        assert!(result_144.is_ok());

        let result_300 = plot.render_to_renderer(&mut renderer, 300.0);
        assert!(result_300.is_ok());
    }
}
