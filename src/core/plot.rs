use std::path::Path;
use crate::{
    data::{Data1D, DataShader},
    render::{Color, LineStyle, MarkerStyle, Theme},
    render::skia::{SkiaRenderer, calculate_plot_area, map_data_to_pixels, generate_ticks},
    core::{Position, PlottingError, Result},
};

#[cfg(feature = "parallel")]
use crate::render::{ParallelRenderer, SeriesRenderData};

/// Main Plot struct - the core API entry point
/// 
/// Provides a fluent builder interface for creating plots with multiple data series,
/// styling options, and export capabilities.
#[derive(Clone)]
pub struct Plot {
    /// Plot title
    title: Option<String>,
    /// X-axis label
    xlabel: Option<String>,
    /// Y-axis label
    ylabel: Option<String>,
    /// Canvas dimensions (width, height)
    dimensions: (u32, u32),
    /// DPI for high-resolution export
    dpi: u32,
    /// Plot theme
    theme: Theme,
    /// Data series
    series: Vec<PlotSeries>,
    /// Legend configuration
    legend: LegendConfig,
    /// Grid configuration
    grid: GridConfig,
    /// Margin around plot area (fraction of canvas)
    margin: Option<f32>,
    /// Whether to use scientific notation on axes
    scientific_notation: bool,
    /// Auto-generate colors for series without explicit colors
    auto_color_index: usize,
    #[cfg(feature = "parallel")]
    /// Parallel renderer for performance optimization
    parallel_renderer: ParallelRenderer,
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

impl Plot {
    /// Create a new Plot with default settings
    pub fn new() -> Self {
        Self {
            title: None,
            xlabel: None,
            ylabel: None,
            dimensions: (800, 600),
            dpi: 96,
            theme: Theme::default(),
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
            margin: None,
            scientific_notation: false,
            auto_color_index: 0,
            #[cfg(feature = "parallel")]
            parallel_renderer: ParallelRenderer::new(),
        }
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
    
    /// Set canvas dimensions
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.dimensions = (width.max(100), height.max(100));
        self
    }
    
    /// Set DPI for export quality
    pub fn dpi(mut self, dpi: u32) -> Self {
        self.dpi = dpi.max(72);
        self
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
        y_errors: &EY
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
        let line_width = series.line_width.unwrap_or(2.0);
        let line_style = series.line_style.clone().unwrap_or(LineStyle::Solid);
        
        match &series.series_type {
            SeriesType::Line { x_data, y_data } => {
                let points: Vec<(f32, f32)> = x_data.iter()
                    .zip(y_data.iter())
                    .map(|(&x, &y)| {
                        crate::render::skia::map_data_to_pixels(x, y, x_min, x_max, y_min, y_max, plot_area)
                    })
                    .collect();
                
                renderer.draw_polyline(&points, color, line_width, line_style)?;
            }
            SeriesType::Scatter { x_data, y_data } => {
                let marker_size = 6.0; // Default marker size
                let marker_style = series.marker_style.unwrap_or(MarkerStyle::Circle);
                
                for (&x, &y) in x_data.iter().zip(y_data.iter()) {
                    let (px, py) = crate::render::skia::map_data_to_pixels(x, y, x_min, x_max, y_min, y_max, plot_area);
                    renderer.draw_marker(px, py, marker_size, marker_style, color)?;
                }
            }
            SeriesType::Bar { values, .. } => {
                // Simple bar rendering
                let bar_width = plot_area.width() / values.len() as f32 * 0.8;
                for (i, &value) in values.iter().enumerate() {
                    let x = i as f64;
                    let (px, py) = crate::render::skia::map_data_to_pixels(x, value, x_min, x_max, y_min, y_max, plot_area);
                    let (_, py_zero) = crate::render::skia::map_data_to_pixels(x, 0.0, x_min, x_max, y_min, y_max, plot_area);
                    renderer.draw_rectangle(
                        px - bar_width / 2.0, 
                        py.min(py_zero), 
                        bar_width, 
                        (py - py_zero).abs(), 
                        color, 
                        true
                    )?;
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
                SeriesType::Line { x_data, y_data } |
                SeriesType::Scatter { x_data, y_data } => {
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
                SeriesType::ErrorBars { x_data, y_data, y_errors } => {
                    if x_data.len() != y_data.len() || y_data.len() != y_errors.len() {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: x_data.len(),
                            y_len: y_data.len(),
                        });
                    }
                }
                SeriesType::ErrorBarsXY { x_data, y_data, x_errors, y_errors } => {
                    if x_data.len() != y_data.len() || 
                       x_data.len() != x_errors.len() || 
                       x_data.len() != y_errors.len() {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: x_data.len(),
                            y_len: y_data.len(),
                        });
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
            if self.parallel_renderer.should_use_parallel(series_count, total_points) {
                return self.render_with_parallel();
            }
        }
        
        // Create renderer for standard rendering
        let mut renderer = SkiaRenderer::new(self.dimensions.0, self.dimensions.1, self.theme.clone())?;
        
        // Calculate plot area with margins
        let plot_area = calculate_plot_area(self.dimensions.0, self.dimensions.1, 0.15);
        
        // Calculate data bounds across all series
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        
        for series in &self.series {
            match &series.series_type {
                SeriesType::Line { x_data, y_data } |
                SeriesType::Scatter { x_data, y_data } => {
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
                    // For bar charts, use category indices as x-values
                    x_min = x_min.min(0.0);
                    x_max = x_max.max((categories.len() - 1) as f64);
                    
                    for &val in values {
                        if val.is_finite() {
                            y_min = y_min.min(val.min(0.0)); // Include zero for bars
                            y_max = y_max.max(val.max(0.0));
                        }
                    }
                }
                SeriesType::ErrorBars { x_data, y_data, y_errors } => {
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
                SeriesType::ErrorBarsXY { x_data, y_data, x_errors, y_errors } => {
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
            }
        }
        
        // Handle edge case where all data is the same
        if (x_max - x_min).abs() < f64::EPSILON {
            x_min -= 1.0;
            x_max += 1.0;
        }
        if (y_max - y_min).abs() < f64::EPSILON {
            y_min -= 1.0;
            y_max += 1.0;
        }
        
        // Generate nice tick values
        let x_ticks = generate_ticks(x_min, x_max, 8);
        let y_ticks = generate_ticks(y_min, y_max, 6);
        
        // Convert ticks to pixel coordinates
        let x_tick_pixels: Vec<f32> = x_ticks.iter()
            .map(|&tick| map_data_to_pixels(tick, 0.0, x_min, x_max, y_min, y_max, plot_area).0)
            .collect();
        let y_tick_pixels: Vec<f32> = y_ticks.iter()
            .map(|&tick| map_data_to_pixels(0.0, tick, x_min, x_max, y_min, y_max, plot_area).1)
            .collect();
        
        // Draw grid if enabled
        if self.grid.enabled {
            renderer.draw_grid(&x_tick_pixels, &y_tick_pixels, plot_area, self.theme.grid_color, LineStyle::Solid)?;
        }
        
        // Draw axes
        renderer.draw_axes(plot_area, &x_tick_pixels, &y_tick_pixels, self.theme.foreground)?;
        
        // Render each data series
        for series in &self.series {
            // Get series styling with defaults
            let color = series.color.unwrap_or_else(|| {
                let palette = Color::default_palette();
                palette[self.auto_color_index % palette.len()]
            });
            let line_width = series.line_width.unwrap_or(self.theme.line_width);
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
                            let (px, py) = map_data_to_pixels(x_val, y_val, x_min, x_max, y_min, y_max, plot_area);
                            points.push((px, py));
                        }
                    }
                    
                    if points.len() >= 2 {
                        renderer.draw_polyline(&points, color, line_width, line_style)?;
                    }
                }
                SeriesType::Scatter { x_data, y_data } => {
                    // Draw individual markers
                    for i in 0..x_data.len() {
                        let x_val = x_data[i];
                        let y_val = y_data[i];
                        if x_val.is_finite() && y_val.is_finite() {
                            let (px, py) = map_data_to_pixels(x_val, y_val, x_min, x_max, y_min, y_max, plot_area);
                            renderer.draw_marker(px, py, 8.0, marker_style, color)?;
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
                    let baseline = map_data_to_pixels(0.0, 0.0, x_min, x_max, y_min, y_max, plot_area).1;
                    
                    for (i, &value) in values.iter().enumerate() {
                        if value.is_finite() {
                            let x_val = i as f64;
                            let (px, py) = map_data_to_pixels(x_val, value, x_min, x_max, y_min, y_max, plot_area);
                            let bar_height = (baseline - py).abs();
                            let bar_x = px - bar_width * 0.5;
                            
                            if value >= 0.0 {
                                renderer.draw_rectangle(bar_x, py, bar_width, bar_height, color, true)?;
                            } else {
                                renderer.draw_rectangle(bar_x, baseline, bar_width, bar_height, color, true)?;
                            }
                        }
                    }
                }
                _ => {
                    // For unsupported plot types (error bars), render as scatter points for now
                    // This is a placeholder - full implementation would handle error bars properly
                    match &series.series_type {
                        SeriesType::ErrorBars { x_data, y_data, .. } |
                        SeriesType::ErrorBarsXY { x_data, y_data, .. } => {
                            for i in 0..x_data.len() {
                                let x_val = x_data[i];
                                let y_val = y_data[i];
                                if x_val.is_finite() && y_val.is_finite() {
                                    let (px, py) = map_data_to_pixels(x_val, y_val, x_min, x_max, y_min, y_max, plot_area);
                                    renderer.draw_marker(px, py, 6.0, MarkerStyle::Circle, color)?;
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
    
    /// Calculate total number of data points across all series
    fn calculate_total_points(&self) -> usize {
        self.series.iter().map(|series| {
            match &series.series_type {
                SeriesType::Line { x_data, .. } |
                SeriesType::Scatter { x_data, .. } |
                SeriesType::ErrorBars { x_data, .. } |
                SeriesType::ErrorBarsXY { x_data, .. } => x_data.len(),
                SeriesType::Bar { categories, .. } => categories.len(),
            }
        }).sum()
    }
    
    /// Render plot using DataShader optimization for large datasets
    fn render_with_datashader(&self) -> Result<Image> {
        // Calculate combined data bounds across all series
        let mut all_points = Vec::new();
        
        // Collect all points from all series
        for series in &self.series {
            match &series.series_type {
                SeriesType::Line { x_data, y_data } |
                SeriesType::Scatter { x_data, y_data } => {
                    for i in 0..x_data.len() {
                        let x = x_data[i];
                        let y = y_data[i];
                        if x.is_finite() && y.is_finite() {
                            all_points.push(crate::core::types::Point2f::new(x as f32, y as f32));
                        }
                    }
                }
                SeriesType::ErrorBars { x_data, y_data, .. } |
                SeriesType::ErrorBarsXY { x_data, y_data, .. } => {
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
                            all_points.push(crate::core::types::Point2f::new(i as f32, value as f32));
                        }
                    }
                }
            }
        }
        
        if all_points.is_empty() {
            return Err(PlottingError::EmptyDataSet);
        }
        
        // Simple DataShader implementation - create basic aggregated image
        let mut datashader = DataShader::with_canvas_size(
            self.dimensions.0 as usize, 
            self.dimensions.1 as usize
        );
        
        // Convert points to (f64, f64) format for aggregation
        let points_f64: Vec<(f64, f64)> = all_points.iter()
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
        
        // Create renderer
        let mut renderer = SkiaRenderer::new(self.dimensions.0, self.dimensions.1, self.theme.clone())?;
        let plot_area = calculate_plot_area(self.dimensions.0, self.dimensions.1, 0.15);
        
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
        let x_tick_pixels: Vec<f32> = x_ticks.iter()
            .map(|&tick| map_data_to_pixels(tick, 0.0, bounds.0, bounds.1, bounds.2, bounds.3, plot_area).0)
            .collect();
        let y_tick_pixels: Vec<f32> = y_ticks.iter()
            .map(|&tick| map_data_to_pixels(0.0, tick, bounds.0, bounds.1, bounds.2, bounds.3, plot_area).1)
            .collect();
        
        // Draw grid if enabled (sequential - UI elements)
        if self.grid.enabled {
            renderer.draw_grid(&x_tick_pixels, &y_tick_pixels, plot_area, self.theme.grid_color, LineStyle::Solid)?;
        }
        
        // Draw axes (sequential - UI elements)
        renderer.draw_axes(plot_area, &x_tick_pixels, &y_tick_pixels, self.theme.foreground)?;
        
        // Process all series in parallel
        let processed_series = self.parallel_renderer.process_series_parallel(
            &self.series,
            |series, index| -> Result<SeriesRenderData> {
                // Get series styling with defaults
                let color = series.color.unwrap_or_else(|| {
                    self.theme.get_color(index)
                });
                let line_width = series.line_width.unwrap_or(self.theme.line_width);
                let alpha = series.alpha.unwrap_or(1.0);
                
                // Process each series type
                let render_series_type = match &series.series_type {
                    SeriesType::Line { x_data, y_data } => {
                        // Transform coordinates in parallel
                        let points = self.parallel_renderer.transform_coordinates_parallel(
                            x_data, 
                            y_data, 
                            data_bounds.clone(), 
                            parallel_plot_area.clone()
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
                            parallel_plot_area.clone()
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
                            parallel_plot_area.clone()
                        )?;
                        
                        // Create bar instances
                        let bar_width = if categories.len() > 1 {
                            let available_width = parallel_plot_area.width() * 0.8;
                            (available_width / categories.len() as f32).min(40.0)
                        } else {
                            40.0
                        };
                        
                        let baseline_y = map_data_to_pixels(0.0, 0.0, bounds.0, bounds.1, bounds.2, bounds.3, plot_area).1;
                        
                        let bars = points.iter().enumerate().map(|(i, point)| {
                            let height = (baseline_y - point.y).abs();
                            crate::render::parallel::BarInstance {
                                x: point.x - bar_width * 0.5,
                                y: if values[i] >= 0.0 { point.y } else { baseline_y },
                                width: bar_width,
                                height,
                                color,
                            }
                        }).collect();
                        
                        RenderSeriesType::Bar { bars }
                    }
                    SeriesType::ErrorBars { x_data, y_data, .. } |
                    SeriesType::ErrorBarsXY { x_data, y_data, .. } => {
                        // For now, render error bars as scatter points
                        // Full error bar implementation would be added here
                        let points = self.parallel_renderer.transform_coordinates_parallel(
                            x_data, 
                            y_data, 
                            data_bounds.clone(), 
                            parallel_plot_area.clone()
                        )?;
                        
                        let markers = self.parallel_renderer.process_markers_parallel(
                            &points,
                            MarkerStyle::Circle,
                            color,
                            6.0,
                        )?;
                        
                        RenderSeriesType::Scatter { markers }
                    }
                };
                
                Ok(SeriesRenderData {
                    series_type: render_series_type,
                    color,
                    line_width,
                    alpha,
                    label: series.label.clone(),
                })
            }
        )?;
        
        // Render processed series (sequential - final drawing)
        for processed in processed_series {
            match processed.series_type {
                RenderSeriesType::Line { segments } => {
                    // Draw all line segments
                    for segment in segments {
                        renderer.draw_polyline(
                            &[(segment.start.x, segment.start.y), (segment.end.x, segment.end.y)],
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
                        renderer.draw_rectangle(
                            bar.x,
                            bar.y,
                            bar.width,
                            bar.height,
                            bar.color,
                            true,
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
        println!("⚡ Parallel: {} series, {} points in {:.1}ms ({:.1}x speedup, {} threads)",
                self.series.len(),
                total_points,
                duration.as_millis(),
                stats.estimated_speedup,
                stats.configured_threads);
        
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
                SeriesType::Line { x_data, y_data } |
                SeriesType::Scatter { x_data, y_data } => {
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
                SeriesType::ErrorBars { x_data, y_data, y_errors } => {
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
                SeriesType::ErrorBarsXY { x_data, y_data, x_errors, y_errors } => {
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
    
    /// Save the plot to a PNG file
    pub fn save<P: AsRef<Path>>(self, path: P) -> Result<()> {
        use crate::render::skia::SkiaRenderer;
        
        // Create renderer and render the plot
        let mut renderer = SkiaRenderer::new(self.dimensions.0, self.dimensions.1, self.theme.clone())?;
        
        // Clear background
        renderer.clear();
        
        // Calculate plot area and data bounds
        let plot_area = crate::render::skia::calculate_plot_area(
            self.dimensions.0, 
            self.dimensions.1, 
            0.1
        );
        
        // Calculate data bounds across all series
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        
        for series in &self.series {
            match &series.series_type {
                SeriesType::Line { x_data, y_data } | 
                SeriesType::Scatter { x_data, y_data } |
                SeriesType::ErrorBars { x_data, y_data, .. } |
                SeriesType::ErrorBarsXY { x_data, y_data, .. } => {
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
            }
        }
        
        // Add padding to data bounds
        let x_range = x_max - x_min;
        let y_range = y_max - y_min;
        x_min -= x_range * 0.05;
        x_max += x_range * 0.05;
        y_min -= y_range * 0.05;
        y_max += y_range * 0.05;
        
        // Generate ticks for axes (always needed)
        let x_ticks = crate::render::skia::generate_ticks(x_min, x_max, 10);
        let y_ticks = crate::render::skia::generate_ticks(y_min, y_max, 8);
        
        let x_tick_pixels: Vec<f32> = x_ticks.iter().map(|&x| {
            crate::render::skia::map_data_to_pixels(x, 0.0, x_min, x_max, 0.0, 1.0, plot_area).0
        }).collect();
        let y_tick_pixels: Vec<f32> = y_ticks.iter().map(|&y| {
            crate::render::skia::map_data_to_pixels(0.0, y, 0.0, 1.0, y_min, y_max, plot_area).1
        }).collect();
        
        // Render grid if enabled
        if self.grid.enabled {
            renderer.draw_grid(&x_tick_pixels, &y_tick_pixels, plot_area, 
                             self.theme.grid_color, crate::render::LineStyle::Solid)?;
        }
        
        // Always draw axes
        renderer.draw_axes(plot_area, &x_tick_pixels, &y_tick_pixels, self.theme.foreground)?;
        
        // Draw axis labels and tick values
        let x_label = self.xlabel.as_deref().unwrap_or("X");
        let y_label = self.ylabel.as_deref().unwrap_or("Y");
        renderer.draw_axis_labels(plot_area, x_min, x_max, y_min, y_max, x_label, y_label, self.theme.foreground)?;
        
        // Draw title if present
        if let Some(ref title) = self.title {
            renderer.draw_title(title, plot_area, self.theme.foreground)?;
        }
        
        // Check if we should use DataShader for large datasets
        let total_points: usize = self.series.iter().map(|series| {
            match &series.series_type {
                SeriesType::Line { x_data, .. } |
                SeriesType::Scatter { x_data, .. } |
                SeriesType::ErrorBars { x_data, .. } |
                SeriesType::ErrorBarsXY { x_data, .. } => x_data.len(),
                SeriesType::Bar { categories, .. } => categories.len(),
            }
        }).sum();

        const DATASHADER_THRESHOLD: usize = 100_000; // Activate DataShader for >100K points

        if total_points > DATASHADER_THRESHOLD {
            // Use DataShader for massive datasets - simplified version
            use crate::data::DataShader;
            
            for series in &self.series {
                match &series.series_type {
                    SeriesType::Scatter { x_data, y_data } | 
                    SeriesType::Line { x_data, y_data } => {
                        let mut datashader = DataShader::with_canvas_size(
                            plot_area.width() as usize,
                            plot_area.height() as usize
                        );
                        
                        datashader.aggregate(x_data, y_data)?;
                        let image = datashader.render();
                        
                        // Draw the DataShader result
                        renderer.draw_datashader_image(&image, plot_area)?;
                    }
                    _ => {
                        // For other plot types, use normal rendering
                        self.render_series_normal(series, &mut renderer, plot_area, x_min, x_max, y_min, y_max)?;
                    }
                }
            }
        } else {
            // Use normal rendering for smaller datasets
            for series in &self.series {
                self.render_series_normal(series, &mut renderer, plot_area, x_min, x_max, y_min, y_max)?;
            }
        }
        
        // Collect legend items from series with labels
        let legend_items: Vec<(String, crate::render::Color)> = self.series.iter()
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
    pub fn save_with_size<P: AsRef<Path>>(mut self, path: P, width: u32, height: u32) -> Result<()> {
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
        
        std::fs::write(path, svg_content)
            .map_err(|e| PlottingError::IoError(e))?;
        
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
    pub fn save_with_size<P: AsRef<Path>>(mut self, path: P, width: u32, height: u32) -> Result<()> {
        self.plot.dimensions = (width, height);
        self.end_series().save(path)
    }
    
    /// Export to SVG
    pub fn export_svg<P: AsRef<Path>>(self, path: P) -> Result<()> {
        self.end_series().export_svg(path)
    }
}

/// In-memory image representation
#[derive(Debug, Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA format
}

impl Image {
    pub fn width(&self) -> u32 {
        self.width
    }
    
    pub fn height(&self) -> u32 {
        self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plot_creation() {
        let plot = Plot::new();
        assert_eq!(plot.dimensions, (800, 600));
        assert_eq!(plot.dpi, 96);
        assert!(plot.series.is_empty());
    }

    #[test]
    fn test_plot_configuration() {
        let plot = Plot::new()
            .title("Test Plot")
            .xlabel("X Axis")
            .ylabel("Y Axis")
            .dimensions(1000, 750)
            .dpi(300);
        
        assert_eq!(plot.title, Some("Test Plot".to_string()));
        assert_eq!(plot.xlabel, Some("X Axis".to_string()));
        assert_eq!(plot.ylabel, Some("Y Axis".to_string()));
        assert_eq!(plot.dimensions, (1000, 750));
        assert_eq!(plot.dpi, 300);
    }

    #[test]
    fn test_line_plot() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 4.0, 9.0];
        
        let plot = Plot::new()
            .line(&x, &y)
                .label("Test Series")
                .color(Color::RED)
            .title("Line Plot Test")
            .save("test_line.png");
        
        // In a real test, we'd verify the file was created
        // For now, we just verify the method chain works
        assert!(plot.is_ok());
    }

    #[test]
    fn test_multi_series() {
        let x1 = vec![1.0, 2.0, 3.0];
        let y1 = vec![1.0, 4.0, 9.0];
        let x2 = vec![1.0, 2.0, 3.0];
        let y2 = vec![2.0, 5.0, 10.0];
        
        let plot = Plot::new()
            .line(&x1, &y1)
                .label("Series 1")
                .color(Color::RED)
            .line(&x2, &y2)
                .label("Series 2")
                .color(Color::BLUE)
            .legend(Position::TopRight)
            .grid(true);
        
        let result = plot.render();
        assert!(result.is_ok());
    }

    #[test]
    fn test_scatter_plot() {
        let x = vec![1.0, 2.0, 3.0, 4.0];
        let y = vec![2.0, 5.0, 3.0, 8.0];
        
        let result = Plot::new()
            .scatter(&x, &y)
                .label("Scatter Data")
                .color(Color::GREEN)
                .marker(MarkerStyle::Circle)
            .render();
        
        assert!(result.is_ok());
        let image = result.unwrap();
        assert_eq!(image.width(), 800);
        assert_eq!(image.height(), 600);
    }

    #[test]
    fn test_validation_errors() {
        // Empty data
        let empty_x: Vec<f64> = vec![];
        let empty_y: Vec<f64> = vec![];
        
        let plot = Plot::new().line(&empty_x, &empty_y);
        let result = plot.render();
        assert!(result.is_err());
        
        // Mismatched lengths
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 2.0];
        
        let plot = Plot::new().line(&x, &y);
        let result = plot.render();
        assert!(result.is_err());
        
        // No series
        let plot = Plot::new();
        let result = plot.render();
        assert!(result.is_err());
    }
}