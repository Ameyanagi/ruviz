use crate::core::error::{PlottingError, Result};
use crate::core::types::{BoundingBox, Point2f};
use crate::render::color::ColorMap;
use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// DataShader canvas for high-performance aggregation of large datasets
/// Implements parallel canvas-based aggregation for 100M+ points
pub struct DataShaderCanvas {
    width: usize,
    height: usize,
    canvas: Vec<AtomicU32>,
    bounds: BoundingBox,
    total_points: u64,
}

impl DataShaderCanvas {
    /// Create new DataShader canvas with specified dimensions
    pub fn new(width: usize, height: usize) -> Self {
        let canvas_size = width * height;
        let canvas = (0..canvas_size)
            .map(|_| AtomicU32::new(0))
            .collect();
            
        Self {
            width,
            height,
            canvas,
            bounds: BoundingBox::new(0.0, 1.0, 0.0, 1.0), // Default unit bounds
            total_points: 0,
        }
    }
    
    /// Create canvas with specified world bounds
    pub fn with_bounds(width: usize, height: usize, bounds: BoundingBox) -> Self {
        let mut canvas = Self::new(width, height);
        canvas.bounds = bounds;
        canvas
    }
    
    /// Set world coordinate bounds for the canvas
    pub fn set_bounds(&mut self, bounds: BoundingBox) {
        self.bounds = bounds;
    }
    
    /// Get canvas dimensions
    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }
    
    /// Get world bounds
    pub fn bounds(&self) -> BoundingBox {
        self.bounds
    }
    
    /// Clear the canvas (reset all bins to zero)
    pub fn clear(&self) {
        #[cfg(feature = "parallel")]
        {
            self.canvas.par_iter().for_each(|cell| {
                cell.store(0, Ordering::Relaxed);
            });
        }
        #[cfg(not(feature = "parallel"))]
        {
            self.canvas.iter().for_each(|cell| {
                cell.store(0, Ordering::Relaxed);
            });
        }
    }
    
    /// Convert world coordinates to grid coordinates
    fn world_to_grid(&self, point: &Point2f) -> Option<(usize, usize)> {
        // Check bounds
        if point.x < self.bounds.min_x || point.x > self.bounds.max_x ||
           point.y < self.bounds.min_y || point.y > self.bounds.max_y {
            return None;
        }
        
        // Normalize to [0,1]
        let norm_x = (point.x - self.bounds.min_x) / self.bounds.width();
        let norm_y = (point.y - self.bounds.min_y) / self.bounds.height();
        
        // Convert to grid coordinates (flip Y for image coordinates)
        let grid_x = ((norm_x * self.width as f32) as usize).min(self.width - 1);
        let grid_y = (((1.0 - norm_y) * self.height as f32) as usize).min(self.height - 1);
        
        Some((grid_x, grid_y))
    }
    
    /// Aggregate a chunk of points in parallel
    pub fn aggregate_chunk(&mut self, points: &[(f64, f64)]) {
        // Convert to Point2f for processing
        let point2f_vec: Vec<Point2f> = points.iter()
            .map(|&(x, y)| Point2f::new(x as f32, y as f32))
            .collect();
            
        self.aggregate_points_parallel(&point2f_vec);
    }
    
    /// Aggregate points using parallel processing
    pub fn aggregate_points_parallel(&mut self, points: &[Point2f]) {
        // Process in parallel chunks for optimal performance
        const CHUNK_SIZE: usize = 10_000;
        
        #[cfg(feature = "parallel")]
        {
            points.par_chunks(CHUNK_SIZE).for_each(|chunk| {
                for point in chunk {
                    if let Some((grid_x, grid_y)) = self.world_to_grid(point) {
                        let idx = grid_y * self.width + grid_x;
                        // Atomic increment - thread-safe
                        self.canvas[idx].fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
        }
        #[cfg(not(feature = "parallel"))]
        {
            for point in points.chunks(CHUNK_SIZE).flat_map(|chunk| chunk.iter()) {
                if let Some((grid_x, grid_y)) = self.world_to_grid(point) {
                    let idx = grid_y * self.width + grid_x;
                    // Atomic increment - still thread-safe
                    self.canvas[idx].fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        
        // Update total point count (approximate, but good enough)
        self.total_points += points.len() as u64;
    }
    
    /// Get aggregated count at grid position
    pub fn get_count(&self, grid_x: usize, grid_y: usize) -> Option<u32> {
        if grid_x >= self.width || grid_y >= self.height {
            return None;
        }
        
        let idx = grid_y * self.width + grid_x;
        Some(self.canvas[idx].load(Ordering::Relaxed))
    }
    
    /// Get maximum count in the canvas (for normalization)
    pub fn max_count(&self) -> u32 {
        #[cfg(feature = "parallel")]
        {
            self.canvas.par_iter()
                .map(|cell| cell.load(Ordering::Relaxed))
                .max()
                .unwrap_or(0)
        }
        #[cfg(not(feature = "parallel"))]
        {
            self.canvas.iter()
                .map(|cell| cell.load(Ordering::Relaxed))
                .max()
                .unwrap_or(0)
        }
    }
    
    /// Get statistics about the aggregated data
    pub fn statistics(&self) -> DataShaderStats {
        let counts: Vec<u32> = self.canvas.iter()
            .map(|cell| cell.load(Ordering::Relaxed))
            .collect();
            
        let non_zero_counts: Vec<u32> = counts.iter()
            .filter(|&&count| count > 0)
            .cloned()
            .collect();
            
        let max_count = counts.iter().max().cloned().unwrap_or(0);
        let min_count = non_zero_counts.iter().min().cloned().unwrap_or(0);
        let total_count: u64 = counts.iter().map(|&c| c as u64).sum();
        let filled_pixels = non_zero_counts.len();
        
        DataShaderStats {
            total_points: self.total_points,
            filled_pixels,
            total_pixels: self.width * self.height,
            max_count,
            min_count,
            total_count,
            canvas_utilization: filled_pixels as f64 / (self.width * self.height) as f64,
        }
    }
    
    /// Render canvas to image using a colormap
    pub fn render_with_colormap(&self, colormap: &ColorMap) -> Result<DataShaderImage> {
        let max_count = self.max_count();
        if max_count == 0 {
            return Err(PlottingError::DataShaderError {
                message: "Cannot render empty canvas".to_string(),
                cause: Some("No points were aggregated".to_string()),
            });
        }
        
        // Generate image data in parallel
        let pixels: Vec<u8> = {
            #[cfg(feature = "parallel")]
            {
                (0..self.height).into_par_iter()
                    .flat_map(|y| {
                        (0..self.width).into_par_iter()
                            .flat_map(move |x| {
                                let idx = y * self.width + x;
                                let count = self.canvas[idx].load(Ordering::Relaxed);
                                
                                // Normalize count to [0,1] range
                                let normalized = if max_count > 0 {
                                    count as f64 / max_count as f64
                                } else {
                                    0.0
                                };
                                
                                // Apply colormap
                                let color = colormap.sample(normalized);
                                
                                // Return RGBA bytes
                                vec![color.r, color.g, color.b, color.a]
                            })
                    })
                    .collect()
            }
            #[cfg(not(feature = "parallel"))]
            {
                (0..self.height).into_iter()
                    .flat_map(|y| {
                        (0..self.width).into_iter()
                            .flat_map(move |x| {
                                let idx = y * self.width + x;
                                let count = self.canvas[idx].load(Ordering::Relaxed);
                                
                                // Normalize count to [0,1] range
                                let normalized = if max_count > 0 {
                                    count as f64 / max_count as f64
                                } else {
                                    0.0
                                };
                                
                                // Apply colormap
                                let color = colormap.sample(normalized);
                                
                                // Return RGBA bytes
                                vec![color.r, color.g, color.b, color.a]
                            })
                    })
                    .collect()
            }
        };
            
        Ok(DataShaderImage {
            width: self.width,
            height: self.height,
            pixels,
        })
    }
    
    /// Auto-calculate optimal bounds from data sample
    pub fn calculate_bounds(data: &[Point2f]) -> Result<BoundingBox> {
        if data.is_empty() {
            return Err(PlottingError::EmptyDataSet);
        }
        
        let mut min_x = data[0].x;
        let mut max_x = data[0].x;
        let mut min_y = data[0].y;
        let mut max_y = data[0].y;
        
        for point in data {
            min_x = min_x.min(point.x);
            max_x = max_x.max(point.x);
            min_y = min_y.min(point.y);
            max_y = max_y.max(point.y);
        }
        
        // Add 5% padding
        let x_range = max_x - min_x;
        let y_range = max_y - min_y;
        let x_pad = x_range * 0.05;
        let y_pad = y_range * 0.05;
        
        Ok(BoundingBox::new(
            min_x - x_pad,
            max_x + x_pad,
            min_y - y_pad,
            max_y + y_pad,
        ))
    }
}

/// Statistics about DataShader aggregation
#[derive(Debug, Clone)]
pub struct DataShaderStats {
    pub total_points: u64,
    pub filled_pixels: usize,
    pub total_pixels: usize,
    pub max_count: u32,
    pub min_count: u32,
    pub total_count: u64,
    pub canvas_utilization: f64,
}

impl DataShaderStats {
    /// Get compression ratio (original points / filled pixels)
    pub fn compression_ratio(&self) -> f64 {
        if self.filled_pixels > 0 {
            self.total_points as f64 / self.filled_pixels as f64
        } else {
            0.0
        }
    }
    
    /// Get average points per filled pixel
    pub fn average_density(&self) -> f64 {
        if self.filled_pixels > 0 {
            self.total_count as f64 / self.filled_pixels as f64
        } else {
            0.0
        }
    }
}

/// DataShader rendered image
#[derive(Debug, Clone)]
pub struct DataShaderImage {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>, // RGBA format
}

impl DataShaderImage {
    /// Get pixel count
    pub fn pixel_count(&self) -> usize {
        self.pixels.len() / 4 // RGBA = 4 bytes per pixel
    }
    
    /// Check if image has content (not all transparent)
    pub fn has_content(&self) -> bool {
        self.pixels.chunks(4)
            .any(|rgba| rgba[3] > 0) // Check alpha channel
    }
}

/// DataShader factory for creating optimized renderers
pub struct DataShader;

impl DataShader {
    /// Determine if DataShader should be used based on data size
    pub fn should_activate(data_size: usize) -> bool {
        // Activate DataShader for datasets larger than 100K points
        data_size > 100_000
    }
    
    /// Create optimal canvas for given data size and target resolution
    pub fn create_canvas(data_size: usize, target_width: usize, target_height: usize) -> DataShaderCanvas {
        // For very large datasets, use higher resolution canvas
        let (width, height) = if data_size > 10_000_000 {
            // 10M+ points: Use higher resolution
            (target_width * 2, target_height * 2)
        } else if data_size > 1_000_000 {
            // 1M+ points: Use 1.5x resolution  
            (target_width * 3 / 2, target_height * 3 / 2)
        } else {
            // Default resolution for moderate datasets
            (target_width, target_height)
        };
        
        DataShaderCanvas::new(width, height)
    }
    
    /// Process large dataset with automatic chunking and progress reporting
    pub fn process_large_dataset<F>(
        canvas: &mut DataShaderCanvas,
        total_size: usize,
        chunk_size: usize,
        mut data_generator: F,
    ) -> Result<()>
    where
        F: FnMut(usize, usize) -> Vec<Point2f>,
    {
        let num_chunks = (total_size + chunk_size - 1) / chunk_size;
        
        for chunk_idx in 0..num_chunks {
            let start_idx = chunk_idx * chunk_size;
            let end_idx = (start_idx + chunk_size).min(total_size);
            
            // Generate data chunk
            let chunk_data = data_generator(start_idx, end_idx);
            
            // Validate data
            for point in &chunk_data {
                if point.x.is_nan() || point.x.is_infinite() || 
                   point.y.is_nan() || point.y.is_infinite() {
                    return Err(PlottingError::InvalidData {
                        message: "DataShader cannot process NaN or infinite values".to_string(),
                        position: Some(start_idx + chunk_data.iter()
                            .position(|p| p.x.is_nan() || p.x.is_infinite() || 
                                          p.y.is_nan() || p.y.is_infinite())
                            .unwrap_or(0)),
                    });
                }
            }
            
            // Aggregate chunk
            canvas.aggregate_points_parallel(&chunk_data);
            
            // Optional: Report progress for very large datasets
            if total_size > 10_000_000 && chunk_idx % 100 == 0 {
                let progress = (chunk_idx as f64 / num_chunks as f64) * 100.0;
                println!("DataShader progress: {:.1}%", progress);
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::color::ColorMap;

    #[test]
    fn test_datashader_canvas_creation() {
        let canvas = DataShaderCanvas::new(800, 600);
        assert_eq!(canvas.dimensions(), (800, 600));
        assert_eq!(canvas.total_points, 0);
    }
    
    #[test]
    fn test_world_to_grid_conversion() {
        let mut canvas = DataShaderCanvas::new(100, 100);
        canvas.set_bounds(BoundingBox::new(0.0, 10.0, 0.0, 10.0));
        
        // Test center point
        let center = Point2f::new(5.0, 5.0);
        let grid_pos = canvas.world_to_grid(&center);
        assert_eq!(grid_pos, Some((50, 49))); // Y is flipped
        
        // Test corner points
        let bottom_left = Point2f::new(0.0, 0.0);
        assert_eq!(canvas.world_to_grid(&bottom_left), Some((0, 99)));
        
        let top_right = Point2f::new(10.0, 10.0);
        assert_eq!(canvas.world_to_grid(&top_right), Some((99, 0)));
    }
    
    #[test]
    fn test_point_aggregation() {
        let mut canvas = DataShaderCanvas::new(10, 10);
        canvas.set_bounds(BoundingBox::new(0.0, 1.0, 0.0, 1.0));
        
        // Add points to the same grid cell
        let points = vec![
            Point2f::new(0.1, 0.1),
            Point2f::new(0.12, 0.12),
            Point2f::new(0.08, 0.08),
        ];
        
        canvas.aggregate_points_parallel(&points);
        
        // Check that points were aggregated
        assert!(canvas.max_count() > 0);
        assert_eq!(canvas.total_points, 3);
    }
    
    #[test]
    fn test_should_activate() {
        assert!(!DataShader::should_activate(1000));
        assert!(!DataShader::should_activate(50000));
        assert!(DataShader::should_activate(150000));
        assert!(DataShader::should_activate(1000000));
    }
    
    #[test]
    fn test_bounds_calculation() {
        let points = vec![
            Point2f::new(1.0, 2.0),
            Point2f::new(5.0, 8.0),
            Point2f::new(3.0, 4.0),
        ];
        
        let bounds = DataShaderCanvas::calculate_bounds(&points).unwrap();
        
        // Should include all points with padding
        assert!(bounds.min_x < 1.0);
        assert!(bounds.max_x > 5.0);
        assert!(bounds.min_y < 2.0);
        assert!(bounds.max_y > 8.0);
    }
    
    #[test]
    fn test_statistics() {
        let mut canvas = DataShaderCanvas::new(10, 10);
        canvas.set_bounds(BoundingBox::new(0.0, 1.0, 0.0, 1.0));
        
        // Add some test points
        let points = vec![
            Point2f::new(0.1, 0.1),
            Point2f::new(0.9, 0.9),
        ];
        
        canvas.aggregate_points_parallel(&points);
        let stats = canvas.statistics();
        
        assert_eq!(stats.total_points, 2);
        assert!(stats.filled_pixels > 0);
        assert!(stats.total_pixels == 100);
        assert!(stats.canvas_utilization > 0.0);
    }
    
    #[test]
    fn test_empty_canvas_render() {
        let canvas = DataShaderCanvas::new(10, 10);
        let colormap = ColorMap::viridis();
        
        let result = canvas.render_with_colormap(&colormap);
        assert!(result.is_err());
        
        if let Err(PlottingError::DataShaderError { message, .. }) = result {
            assert!(message.contains("empty canvas"));
        }
    }
}