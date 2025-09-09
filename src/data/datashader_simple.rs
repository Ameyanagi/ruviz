//! Simple DataShader implementation without parallel features
//! This will be used to make tests pass initially, then enhanced

use crate::core::error::{PlottingError, Result};
use crate::core::types::{BoundingBox, Point2f};
use std::sync::atomic::{AtomicU32, Ordering};

/// Simple DataShader canvas for aggregation
pub struct DataShaderCanvas {
    width: usize,
    height: usize,
    canvas: Vec<AtomicU32>,
    bounds: BoundingBox,
    total_points: u64,
}

impl DataShaderCanvas {
    /// Create new DataShader canvas
    pub fn new(width: usize, height: usize) -> Self {
        let canvas_size = width * height;
        let canvas = (0..canvas_size)
            .map(|_| AtomicU32::new(0))
            .collect();
        
        Self {
            width,
            height,
            canvas,
            bounds: BoundingBox::new(0.0, 0.0, 1.0, 1.0),
            total_points: 0,
        }
    }

    
    /// Calculate bounds from a set of points
    pub fn calculate_bounds(points: &[(f64, f64)]) -> Result<BoundingBox> {
        if points.is_empty() {
            return Err(PlottingError::EmptyDataSet);
        }
        
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        
        for &(x, y) in points {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        
        Ok(BoundingBox::new(min_x as f32, min_y as f32, max_x as f32, max_y as f32))
    }

    
    /// Create a DataShaderCanvas with specific bounds
    pub fn with_bounds(width: usize, height: usize, bounds: BoundingBox) -> Self {
        let canvas_size = width * height;
        let canvas = (0..canvas_size)
            .map(|_| AtomicU32::new(0))
            .collect();
        
        Self {
            width,
            height,
            canvas,
            bounds,
            total_points: 0,
        }
    }
    
    /// Set the world coordinate bounds
    pub fn set_bounds(&mut self, bounds: BoundingBox) {
        self.bounds = bounds;
    }
    
    /// Get the bounds
    pub fn bounds(&self) -> BoundingBox {
        self.bounds
    }

    
    /// Get canvas width
    pub fn width(&self) -> usize {
        self.width
    }
    
    /// Get canvas height  
    pub fn height(&self) -> usize {
        self.height
    }
    
    /// Clear the canvas
    pub fn clear(&self) {
        for cell in &self.canvas {
            cell.store(0, Ordering::Relaxed);
        }
    }
    
    /// Convert world coordinates to grid coordinates
    fn world_to_grid(&self, point: &Point2f) -> Option<(usize, usize)> {
        // Check if point is within bounds
        if point.x < self.bounds.min_x || point.x > self.bounds.max_x ||
           point.y < self.bounds.min_y || point.y > self.bounds.max_y {
            return None;
        }
        
        // Convert to grid coordinates
        let x_norm = (point.x - self.bounds.min_x) / (self.bounds.max_x - self.bounds.min_x);
        let y_norm = (point.y - self.bounds.min_y) / (self.bounds.max_y - self.bounds.min_y);
        
        let grid_x = (x_norm * (self.width - 1) as f32) as usize;
        let grid_y = (y_norm * (self.height - 1) as f32) as usize;
        
        Some((grid_x, grid_y))
    }
    
    /// Aggregate points from (x, y) tuples
    pub fn aggregate(&mut self, points: &[(f64, f64)]) {
        // Convert to Point2f first
        let point2f_vec: Vec<Point2f> = points.iter()
            .map(|&(x, y)| Point2f::new(x as f32, y as f32))
            .collect();
            
        self.aggregate_points(&point2f_vec);
    }
    
    /// Aggregate points
    pub fn aggregate_points(&mut self, points: &[Point2f]) {
        for point in points {
            if let Some((grid_x, grid_y)) = self.world_to_grid(point) {
                let idx = grid_y * self.width + grid_x;
                if idx < self.canvas.len() {
                    self.canvas[idx].fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        
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
    
    /// Get maximum count in the canvas
    pub fn max_count(&self) -> u32 {
        self.canvas.iter()
            .map(|cell| cell.load(Ordering::Relaxed))
            .max()
            .unwrap_or(0)
    }
    
    /// Get statistics about the aggregated data
    pub fn statistics(&self) -> DataShaderStats {
        let counts: Vec<u32> = self.canvas.iter()
            .map(|cell| cell.load(Ordering::Relaxed))
            .collect();
            
        let non_zero_counts: Vec<u32> = counts.iter().filter(|&&c| c > 0).cloned().collect();
        let max_count = *non_zero_counts.iter().max().unwrap_or(&0);
        let min_count = *non_zero_counts.iter().min().unwrap_or(&0);
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
    
    /// Create image data as simple grayscale
    pub fn to_image_data(&self) -> Vec<u8> {
        let max_count = self.max_count();
        let mut pixels = Vec::with_capacity(self.width * self.height * 4);
        
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = y * self.width + x;
                let count = self.canvas[idx].load(Ordering::Relaxed);
                
                // Normalize to grayscale
                let intensity = if max_count > 0 {
                    ((count as f64 / max_count as f64) * 255.0) as u8
                } else {
                    0
                };
                
                // RGBA
                pixels.push(intensity); // R
                pixels.push(intensity); // G  
                pixels.push(intensity); // B
                pixels.push(255);       // A
            }
        }
        
        pixels
    }
}

/// Statistics about aggregated data
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

/// Simple image representation
pub struct DataShaderImage {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

impl DataShaderImage {
    pub fn new(width: usize, height: usize, pixels: Vec<u8>) -> Self {
        Self {
            width,
            height,
            pixels,
        }
    }
}

/// DataShader facade - simple aggregation for massive datasets
pub struct DataShader {
    canvas: DataShaderCanvas,
}

impl DataShader {
    /// Create new DataShader with default canvas size
    pub fn new() -> Self {
        Self::with_canvas_size(512, 512)
    }
    
    /// Create DataShader with specific canvas size
    pub fn with_canvas_size(width: usize, height: usize) -> Self {
        Self {
            canvas: DataShaderCanvas::new(width, height),
        }
    }

    
    /// Check if DataShader should be activated for the given point count
    pub fn should_activate(point_count: usize) -> bool {
        point_count > 100_000
    }

    
    /// Create canvas size recommendation based on data points and dimensions
    pub fn create_canvas(point_count: usize, width: usize, height: usize) -> (usize, usize) {
        // For simplicity, just return the given dimensions
        // In a more sophisticated implementation, this could adjust canvas size based on point density
        (width, height)
    }
    
    /// Set data bounds
    pub fn set_bounds(&mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) {
        let bounds = BoundingBox::new(min_x as f32, min_y as f32, max_x as f32, max_y as f32);
        self.canvas.set_bounds(bounds);
    }
    
    /// Aggregate data points
    pub fn aggregate(&mut self, x_data: &[f64], y_data: &[f64]) -> Result<()> {
        if x_data.len() != y_data.len() {
            return Err(PlottingError::DataLengthMismatch {
                x_len: x_data.len(),
                y_len: y_data.len(),
            });
        }
        
        if x_data.is_empty() {
            return Err(PlottingError::EmptyDataSet);
        }
        
        // Auto-calculate bounds if not set
        let x_min = x_data.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let x_max = x_data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let y_min = y_data.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let y_max = y_data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        
        self.set_bounds(x_min, y_min, x_max, y_max);
        
        // Create point tuples and aggregate
        let points: Vec<(f64, f64)> = x_data.iter().zip(y_data.iter()).map(|(&x, &y)| (x, y)).collect();
        self.canvas.aggregate(&points);
        
        Ok(())
    }
    
    /// Get statistics
    pub fn statistics(&self) -> DataShaderStats {
        self.canvas.statistics()
    }
    
    /// Render to image data
    pub fn render(&self) -> DataShaderImage {
        let pixels = self.canvas.to_image_data();
        DataShaderImage::new(self.canvas.width(), self.canvas.height(), pixels)
    }
}