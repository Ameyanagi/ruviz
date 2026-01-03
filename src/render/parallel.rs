use crate::{
    core::{PlottingError, Result, types::Point2f},
    data::{get_memory_manager, elements::LineSegment},
    render::{Color, LineStyle, MarkerStyle},
};

#[cfg(feature = "simd")]
use crate::render::simd::{CoordinateBounds, PixelViewport, SIMDTransformer};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

/// Parallel rendering engine for high-performance plot rendering
///
/// Provides series-level parallelization using rayon for concurrent processing
/// of multiple data series, coordinate transformations, and rendering operations.
#[derive(Debug, Clone)]
pub struct ParallelRenderer {
    /// Maximum number of threads to use for parallel operations
    max_threads: usize,
    /// Minimum series count required to activate parallel rendering
    parallel_threshold: usize,
    /// Enable chunked processing for large datasets
    chunked_processing: bool,
    /// Chunk size for processing large series
    chunk_size: usize,
    /// SIMD transformer for vectorized coordinate operations
    #[cfg(feature = "simd")]
    simd_transformer: SIMDTransformer,
}

impl ParallelRenderer {
    /// Create new parallel renderer with default settings
    pub fn new() -> Self {
        Self {
            max_threads: rayon::current_num_threads(),
            parallel_threshold: 2,
            chunked_processing: true,
            chunk_size: 10_000,
            #[cfg(feature = "simd")]
            simd_transformer: SIMDTransformer::new(),
        }
    }

    /// Create parallel renderer with custom thread count
    pub fn with_threads(threads: usize) -> Self {
        let mut renderer = Self::new();
        renderer.max_threads = threads.max(1);
        renderer
    }

    /// Create parallel renderer with SIMD configuration
    #[cfg(feature = "simd")]
    pub fn with_simd(mut self, simd_threshold: usize) -> Self {
        self.simd_transformer = SIMDTransformer::with_threshold(simd_threshold);
        self
    }

    /// Set parallel processing threshold
    pub fn with_threshold(mut self, threshold: usize) -> Self {
        self.parallel_threshold = threshold.max(1);
        self
    }

    /// Enable/disable chunked processing for large datasets
    pub fn with_chunking(mut self, enabled: bool, chunk_size: usize) -> Self {
        self.chunked_processing = enabled;
        self.chunk_size = chunk_size.max(1000);
        self
    }

    /// Get current thread pool configuration
    pub fn thread_config(&self) -> ParallelConfig {
        ParallelConfig {
            max_threads: self.max_threads,
            current_threads: rayon::current_num_threads(),
            parallel_threshold: self.parallel_threshold,
            chunked_processing: self.chunked_processing,
            chunk_size: self.chunk_size,
        }
    }

    /// Check if parallel processing should be activated
    pub fn should_use_parallel(&self, series_count: usize, total_points: usize) -> bool {
        series_count >= self.parallel_threshold
            || (self.chunked_processing && total_points > self.chunk_size * 2)
    }

    /// Process multiple series in parallel with coordinate transformation
    pub fn process_series_parallel<T, F>(
        &self,
        series_data: &[T],
        processor: F,
    ) -> Result<Vec<SeriesRenderData>>
    where
        T: Send + Sync,
        F: Fn(&T, usize) -> Result<SeriesRenderData> + Send + Sync,
    {
        if !self.should_use_parallel(series_data.len(), 0) {
            // Use sequential processing for small datasets
            return series_data
                .iter()
                .enumerate()
                .map(|(i, data)| processor(data, i))
                .collect();
        }

        // Configure thread pool if needed
        let pool_result = if rayon::current_num_threads() != self.max_threads {
            rayon::ThreadPoolBuilder::new()
                .num_threads(self.max_threads)
                .build_global()
        } else {
            Ok(())
        };

        let results: Result<Vec<SeriesRenderData>> = match pool_result {
            Ok(_) => {
                // Use parallel processing
                series_data
                    .par_iter()
                    .enumerate()
                    .map(|(i, data)| processor(data, i))
                    .collect()
            }
            Err(_) => {
                // Fallback to sequential if thread pool creation fails
                series_data
                    .iter()
                    .enumerate()
                    .map(|(i, data)| processor(data, i))
                    .collect()
            }
        };

        results
    }

    /// Transform coordinates in parallel chunks with SIMD acceleration and memory pooling
    pub fn transform_coordinates_parallel(
        &self,
        x_data: &[f64],
        y_data: &[f64],
        bounds: DataBounds,
        plot_area: PlotArea,
    ) -> Result<Vec<Point2f>> {
        self.transform_coordinates_parallel_pooled(x_data, y_data, bounds, plot_area)
    }

    /// Memory-optimized coordinate transformation using buffer pools
    pub fn transform_coordinates_parallel_pooled(
        &self,
        x_data: &[f64],
        y_data: &[f64],
        bounds: DataBounds,
        plot_area: PlotArea,
    ) -> Result<Vec<Point2f>> {
        if x_data.len() != y_data.len() {
            return Err(PlottingError::DataLengthMismatch {
                x_len: x_data.len(),
                y_len: y_data.len(),
            });
        }

        let point_count = x_data.len();
        let memory_manager = get_memory_manager();

        // Get managed buffer for output points (use memory Point2f type)
        let mut output_buffer = memory_manager.get_point_buffer(point_count);
        let output_vec = output_buffer.get_mut();
        output_vec.clear();
        output_vec.reserve(point_count);

        #[cfg(feature = "simd")]
        {
            // Convert to SIMD-compatible types
            let simd_bounds = CoordinateBounds {
                x_min: bounds.x_min,
                x_max: bounds.x_max,
                y_min: bounds.y_min,
                y_max: bounds.y_max,
            };

            let viewport = PixelViewport {
                left: plot_area.left,
                right: plot_area.right,
                top: plot_area.top,
                bottom: plot_area.bottom,
            };

            if !self.chunked_processing || point_count < self.chunk_size {
                // Use SIMD for small datasets (sequential) with memory pooling
                let simd_points = self.simd_transformer.transform_coordinates_simd(
                    x_data,
                    y_data,
                    simd_bounds,
                    viewport,
                )?;

                // Add SIMD points directly to managed buffer (same type now)
                output_vec.extend(simd_points);

                return Ok(output_buffer.into_inner());
            } else {
                // Parallel chunked processing with SIMD and memory management
                let chunks: Vec<&[f64]> = x_data.chunks(self.chunk_size).collect();
                let y_chunks: Vec<&[f64]> = y_data.chunks(self.chunk_size).collect();

                // Process chunks in parallel, each using memory-optimized SIMD
                let chunk_results: Result<Vec<Vec<Point2f>>> = chunks
                    .par_iter()
                    .zip(y_chunks.par_iter())
                    .map(|(x_chunk, y_chunk)| {
                        // Each thread gets its own memory manager access
                        self.simd_transformer.transform_coordinates_simd(
                            x_chunk,
                            y_chunk,
                            simd_bounds.clone(),
                            viewport.clone(),
                        )
                    })
                    .collect();

                match chunk_results {
                    Ok(results) => {
                        // Efficiently collect results using pre-allocated buffer
                        for chunk_result in results {
                            output_vec.extend(chunk_result);
                        }
                        return Ok(output_buffer.into_inner());
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        // Fallback: simple coordinate transformation without SIMD
        for i in 0..point_count {
            let x_norm = (x_data[i] - bounds.x_min as f64) / (bounds.x_max - bounds.x_min) as f64;
            let y_norm = (y_data[i] - bounds.y_min as f64) / (bounds.y_max - bounds.y_min) as f64;

            let pixel_x = plot_area.left + x_norm as f32 * (plot_area.right - plot_area.left);
            let pixel_y = plot_area.bottom - y_norm as f32 * (plot_area.bottom - plot_area.top);

            output_vec.push(Point2f::new(pixel_x, pixel_y));
        }

        Ok(output_buffer.into_inner())
    }

    /// Process polyline segments in parallel for large line plots
    pub fn process_polyline_parallel(
        &self,
        points: &[Point2f],
        line_style: LineStyle,
        color: Color,
        line_width: f32,
    ) -> Result<Vec<LineSegment>> {
        if points.len() < 2 {
            return Ok(Vec::new());
        }

        if !self.chunked_processing || points.len() < self.chunk_size {
            // Sequential processing for small datasets
            return Ok(points
                .windows(2)
                .map(|segment| LineSegment {
                    start: segment[0],
                    end: segment[1],
                    style: line_style.clone(),
                    color,
                    width: line_width,
                })
                .collect());
        }

        // Parallel processing with overlapping chunks
        let chunk_size = self.chunk_size;
        let chunk_count = (points.len() + chunk_size - 1) / chunk_size;

        let segments: Vec<LineSegment> = (0..chunk_count)
            .into_par_iter()
            .map(|chunk_idx| {
                let start_idx = chunk_idx * chunk_size;
                let end_idx = ((chunk_idx + 1) * chunk_size + 1).min(points.len());

                if start_idx >= points.len() - 1 {
                    return Vec::new();
                }

                let chunk = &points[start_idx..end_idx];
                chunk
                    .windows(2)
                    .map(|segment| LineSegment {
                        start: segment[0],
                        end: segment[1],
                        style: line_style.clone(),
                        color,
                        width: line_width,
                    })
                    .collect::<Vec<LineSegment>>()
            })
            .flatten()
            .collect();

        Ok(segments)
    }

    /// Process scatter markers in parallel
    pub fn process_markers_parallel(
        &self,
        points: &[Point2f],
        marker_style: MarkerStyle,
        color: Color,
        size: f32,
    ) -> Result<Vec<MarkerInstance>> {
        if !self.chunked_processing || points.len() < self.chunk_size {
            // Sequential processing for small datasets
            return Ok(points
                .iter()
                .map(|&point| MarkerInstance {
                    position: point,
                    style: marker_style,
                    color,
                    size,
                })
                .collect());
        }

        // Parallel processing
        let markers: Vec<MarkerInstance> = points
            .par_chunks(self.chunk_size)
            .map(|chunk| {
                chunk
                    .iter()
                    .map(|&point| MarkerInstance {
                        position: point,
                        style: marker_style,
                        color,
                        size,
                    })
                    .collect::<Vec<MarkerInstance>>()
            })
            .flatten()
            .collect();

        Ok(markers)
    }

    /// Get performance statistics for the current configuration
    pub fn performance_stats(&self) -> PerformanceStats {
        let parallel_speedup = self.estimate_speedup();

        #[cfg(feature = "simd")]
        let combined_speedup = {
            let simd_info = self.simd_transformer.performance_info();
            parallel_speedup * simd_info.estimated_speedup
        };

        #[cfg(not(feature = "simd"))]
        let combined_speedup = parallel_speedup;

        PerformanceStats {
            available_threads: num_cpus::get(),
            configured_threads: self.max_threads,
            active_threads: rayon::current_num_threads(),
            parallel_threshold: self.parallel_threshold,
            chunked_processing: self.chunked_processing,
            chunk_size: self.chunk_size,
            estimated_speedup: combined_speedup,
        }
    }

    /// Get detailed performance information including SIMD
    #[cfg(feature = "simd")]
    pub fn detailed_performance_info(&self) -> DetailedPerformanceInfo {
        let simd_info = self.simd_transformer.performance_info();
        let combined_speedup = self.estimate_speedup() * simd_info.estimated_speedup;

        DetailedPerformanceInfo {
            parallel_info: self.performance_stats(),
            simd_info,
            combined_speedup,
        }
    }

    /// Estimate potential speedup for parallel processing
    fn estimate_speedup(&self) -> f32 {
        // Simple Amdahl's law approximation
        // Assumes 80% of work can be parallelized
        let parallel_fraction = 0.8;
        let threads = self.max_threads as f32;
        1.0 / ((1.0 - parallel_fraction) + (parallel_fraction / threads))
    }
}

impl Default for ParallelRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration information for parallel renderer
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    pub max_threads: usize,
    pub current_threads: usize,
    pub parallel_threshold: usize,
    pub chunked_processing: bool,
    pub chunk_size: usize,
}

/// Performance statistics for parallel processing
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub available_threads: usize,
    pub configured_threads: usize,
    pub active_threads: usize,
    pub parallel_threshold: usize,
    pub chunked_processing: bool,
    pub chunk_size: usize,
    pub estimated_speedup: f32,
}

/// Detailed performance information including SIMD
#[derive(Debug, Clone)]
pub struct DetailedPerformanceInfo {
    pub parallel_info: PerformanceStats,
    #[cfg(feature = "simd")]
    pub simd_info: crate::render::simd::SIMDPerformanceInfo,
    pub combined_speedup: f32,
}

/// Processed series data ready for rendering
#[derive(Debug, Clone)]
pub struct SeriesRenderData {
    pub series_type: RenderSeriesType,
    pub color: Color,
    pub line_width: f32,
    pub alpha: f32,
    pub label: Option<String>,
}

/// Series types optimized for parallel rendering
#[derive(Debug, Clone)]
pub enum RenderSeriesType {
    Line {
        segments: Vec<LineSegment>,
    },
    Scatter {
        markers: Vec<MarkerInstance>,
    },
    Bar {
        bars: Vec<BarInstance>,
    },
    ErrorBars {
        points: Vec<Point2f>,
        error_lines: Vec<LineSegment>,
    },
    BoxPlot {
        box_data: BoxPlotRenderData,
    },
}

// LineSegment imported from crate::data::elements (canonical definition)

/// Marker instance for parallel scatter rendering
#[derive(Debug, Clone, Copy)]
pub struct MarkerInstance {
    pub position: Point2f,
    pub style: MarkerStyle,
    pub color: Color,
    pub size: f32,
}

/// Bar instance for parallel bar rendering
#[derive(Debug, Clone)]
pub struct BarInstance {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: Color,
}

/// Box plot render data for parallel rendering
#[derive(Debug, Clone)]
pub struct BoxPlotRenderData {
    pub x_center: f32,
    pub box_left: f32,
    pub box_right: f32,
    pub q1_y: f32,
    pub median_y: f32,
    pub q3_y: f32,
    pub lower_whisker_y: f32,
    pub upper_whisker_y: f32,
    pub outliers: Vec<Point2f>,
    pub box_color: Color,
    pub line_color: Color,
    pub outlier_color: Color,
}

/// Data bounds for coordinate transformation
#[derive(Debug, Clone)]
pub struct DataBounds {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

/// Plot area definition for coordinate mapping
#[derive(Debug, Clone)]
pub struct PlotArea {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl PlotArea {
    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    pub fn height(&self) -> f32 {
        self.bottom - self.top
    }
}

/// Fast coordinate transformation functions
#[inline]
fn map_x_coordinate(x: f64, x_min: f64, x_max: f64, left: f32, right: f32) -> f32 {
    let normalized = (x - x_min) / (x_max - x_min);
    left + (normalized as f32) * (right - left)
}

#[inline]
fn map_y_coordinate(y: f64, y_min: f64, y_max: f64, bottom: f32, top: f32) -> f32 {
    let normalized = (y - y_min) / (y_max - y_min);
    bottom + (normalized as f32) * (top - bottom)
}

/// Thread-safe rendering statistics collector
#[derive(Debug)]
pub struct RenderStats {
    series_processed: Arc<Mutex<usize>>,
    points_processed: Arc<Mutex<usize>>,
    processing_time: Arc<Mutex<std::time::Duration>>,
    parallel_efficiency: Arc<Mutex<f32>>,
}

impl RenderStats {
    pub fn new() -> Self {
        Self {
            series_processed: Arc::new(Mutex::new(0)),
            points_processed: Arc::new(Mutex::new(0)),
            processing_time: Arc::new(Mutex::new(std::time::Duration::ZERO)),
            parallel_efficiency: Arc::new(Mutex::new(1.0)),
        }
    }

    pub fn record_series(
        &self,
        series_count: usize,
        point_count: usize,
        duration: std::time::Duration,
    ) {
        if let (Ok(mut series), Ok(mut points), Ok(mut time)) = (
            self.series_processed.lock(),
            self.points_processed.lock(),
            self.processing_time.lock(),
        ) {
            *series += series_count;
            *points += point_count;
            *time += duration;
        }
    }

    pub fn get_stats(&self) -> (usize, usize, std::time::Duration) {
        let series = *self
            .series_processed
            .lock()
            .unwrap_or_else(|_| panic!("Mutex poisoned"));
        let points = *self
            .points_processed
            .lock()
            .unwrap_or_else(|_| panic!("Mutex poisoned"));
        let time = *self
            .processing_time
            .lock()
            .unwrap_or_else(|_| panic!("Mutex poisoned"));
        (series, points, time)
    }
}

impl Default for RenderStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_renderer_creation() {
        let renderer = ParallelRenderer::new();
        assert!(renderer.max_threads > 0);
        assert_eq!(renderer.parallel_threshold, 2);
        assert!(renderer.chunked_processing);
    }

    #[test]
    fn test_parallel_threshold() {
        let renderer = ParallelRenderer::new().with_threshold(5);
        assert!(!renderer.should_use_parallel(3, 1000));
        assert!(renderer.should_use_parallel(5, 1000));
        assert!(renderer.should_use_parallel(10, 1000));
    }

    #[test]
    fn test_chunking_threshold() {
        let renderer = ParallelRenderer::new().with_chunking(true, 1000);
        assert!(!renderer.should_use_parallel(1, 500));
        assert!(renderer.should_use_parallel(1, 2500));
    }

    #[test]
    fn test_coordinate_transformation() {
        let renderer = ParallelRenderer::new();
        let x_data = vec![1.0, 2.0, 3.0];
        let y_data = vec![10.0, 20.0, 30.0];

        let bounds = DataBounds {
            x_min: 1.0,
            x_max: 3.0,
            y_min: 10.0,
            y_max: 30.0,
        };

        let plot_area = PlotArea {
            left: 0.0,
            right: 100.0,
            top: 0.0,
            bottom: 100.0,
        };

        let result = renderer.transform_coordinates_parallel(&x_data, &y_data, bounds, plot_area);
        assert!(result.is_ok());

        let points = result.unwrap();
        assert_eq!(points.len(), 3);
        assert_eq!(points[0].x, 0.0); // x=1 maps to left edge
        assert_eq!(points[2].x, 100.0); // x=3 maps to right edge
    }

    #[test]
    fn test_performance_stats() {
        let renderer = ParallelRenderer::new();
        let stats = renderer.performance_stats();

        assert!(stats.available_threads > 0);
        assert!(stats.configured_threads > 0);
        assert!(stats.estimated_speedup >= 1.0);
    }

    #[test]
    fn test_render_stats() {
        let stats = RenderStats::new();
        let duration = std::time::Duration::from_millis(100);

        stats.record_series(3, 1000, duration);
        let (series, points, time) = stats.get_stats();

        assert_eq!(series, 3);
        assert_eq!(points, 1000);
        assert_eq!(time, duration);
    }
}
