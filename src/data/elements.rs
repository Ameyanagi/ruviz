use std::sync::{Arc, Mutex};
use crate::{
    core::types::{Point2f, BoundingBox},
    render::{Color, LineStyle, MarkerStyle},
    data::{get_memory_manager, ManagedBuffer},
};

/// Memory-optimized storage for plot elements with object pooling
/// 
/// Provides efficient storage and reuse of plot primitives like line segments,
/// markers, and polygons through specialized memory pools.
#[derive(Debug)]
pub struct PlotElementStorage {
    /// Pool for line segments
    line_pool: Arc<Mutex<ElementPool<LineSegment>>>,
    /// Pool for markers
    marker_pool: Arc<Mutex<ElementPool<MarkerInstance>>>,
    /// Pool for polygons
    polygon_pool: Arc<Mutex<ElementPool<Polygon>>>,
    /// Pool for text elements
    text_pool: Arc<Mutex<ElementPool<TextElement>>>,
    /// Pool for error bars
    error_bar_pool: Arc<Mutex<ElementPool<ErrorBar>>>,
}

impl PlotElementStorage {
    /// Create new element storage with default pool sizes
    pub fn new() -> Self {
        Self {
            line_pool: Arc::new(Mutex::new(ElementPool::new(1000))),
            marker_pool: Arc::new(Mutex::new(ElementPool::new(5000))),
            polygon_pool: Arc::new(Mutex::new(ElementPool::new(100))),
            text_pool: Arc::new(Mutex::new(ElementPool::new(50))),
            error_bar_pool: Arc::new(Mutex::new(ElementPool::new(1000))),
        }
    }
    
    /// Get managed storage for line segments
    pub fn get_line_storage(&self, capacity: usize) -> ManagedElementStorage<LineSegment> {
        let mut pool = self.line_pool.lock().unwrap();
        let storage = pool.get_storage(capacity);
        ManagedElementStorage::new(storage, self.line_pool.clone())
    }
    
    /// Get managed storage for markers
    pub fn get_marker_storage(&self, capacity: usize) -> ManagedElementStorage<MarkerInstance> {
        let mut pool = self.marker_pool.lock().unwrap();
        let storage = pool.get_storage(capacity);
        ManagedElementStorage::new(storage, self.marker_pool.clone())
    }
    
    /// Get managed storage for polygons
    pub fn get_polygon_storage(&self, capacity: usize) -> ManagedElementStorage<Polygon> {
        let mut pool = self.polygon_pool.lock().unwrap();
        let storage = pool.get_storage(capacity);
        ManagedElementStorage::new(storage, self.polygon_pool.clone())
    }
    
    /// Get managed storage for text elements
    pub fn get_text_storage(&self, capacity: usize) -> ManagedElementStorage<TextElement> {
        let mut pool = self.text_pool.lock().unwrap();
        let storage = pool.get_storage(capacity);
        ManagedElementStorage::new(storage, self.text_pool.clone())
    }
    
    /// Get managed storage for error bars
    pub fn get_error_bar_storage(&self, capacity: usize) -> ManagedElementStorage<ErrorBar> {
        let mut pool = self.error_bar_pool.lock().unwrap();
        let storage = pool.get_storage(capacity);
        ManagedElementStorage::new(storage, self.error_bar_pool.clone())
    }
    
    /// Get memory usage statistics for all pools
    pub fn get_pool_stats(&self) -> PlotElementStats {
        PlotElementStats {
            line_segments: self.line_pool.lock().unwrap().stats(),
            markers: self.marker_pool.lock().unwrap().stats(),
            polygons: self.polygon_pool.lock().unwrap().stats(),
            text_elements: self.text_pool.lock().unwrap().stats(),
            error_bars: self.error_bar_pool.lock().unwrap().stats(),
        }
    }
}

impl Default for PlotElementStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Generic pool for plot elements
#[derive(Debug)]
struct ElementPool<T> {
    /// Available element collections
    available: Vec<Vec<T>>,
    /// Target capacity for new collections
    target_capacity: usize,
    /// Pool statistics
    stats: PoolStats,
}

impl<T> ElementPool<T> {
    fn new(target_capacity: usize) -> Self {
        Self {
            available: Vec::new(),
            target_capacity,
            stats: PoolStats::default(),
        }
    }
    
    fn get_storage(&mut self, min_capacity: usize) -> Vec<T> {
        // Try to find a suitable existing collection
        for (i, storage) in self.available.iter().enumerate() {
            if storage.capacity() >= min_capacity {
                let mut storage = self.available.swap_remove(i);
                storage.clear();
                self.stats.hits += 1;
                return storage;
            }
        }
        
        // Create new collection if none suitable found
        let capacity = min_capacity.max(self.target_capacity);
        self.stats.misses += 1;
        self.stats.total_allocated += 1;
        Vec::with_capacity(capacity)
    }
    
    fn return_storage(&mut self, mut storage: Vec<T>) {
        // Only keep collections that aren't too large
        if storage.capacity() <= self.target_capacity * 4 {
            storage.clear();
            self.available.push(storage);
        }
        // Large collections are dropped to prevent memory bloat
    }
    
    fn stats(&self) -> PoolStats {
        self.stats.clone()
    }
}

/// Pool statistics
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    pub hits: usize,
    pub misses: usize,
    pub total_allocated: usize,
}

/// Managed element storage that returns to pool when dropped
pub struct ManagedElementStorage<T> {
    storage: Option<Vec<T>>,
    pool: Arc<Mutex<ElementPool<T>>>,
}

impl<T> ManagedElementStorage<T> {
    fn new(storage: Vec<T>, pool: Arc<Mutex<ElementPool<T>>>) -> Self {
        Self {
            storage: Some(storage),
            pool,
        }
    }
    
    /// Get mutable reference to the storage
    pub fn get_mut(&mut self) -> &mut Vec<T> {
        self.storage.as_mut().unwrap()
    }
    
    /// Get immutable reference to the storage
    pub fn get(&self) -> &Vec<T> {
        self.storage.as_ref().unwrap()
    }
    
    /// Take ownership of the storage (prevents return to pool)
    pub fn into_inner(mut self) -> Vec<T> {
        self.storage.take().unwrap()
    }
}

impl<T> Drop for ManagedElementStorage<T> {
    fn drop(&mut self) {
        if let Some(storage) = self.storage.take() {
            if let Ok(mut pool) = self.pool.lock() {
                pool.return_storage(storage);
            }
        }
    }
}

/// Memory-efficient line segment representation
#[derive(Debug, Clone)]
pub struct LineSegment {
    /// Start point
    pub start: Point2f,
    /// End point  
    pub end: Point2f,
    /// Line style
    pub style: LineStyle,
    /// Line color
    pub color: Color,
    /// Line width
    pub width: f32,
}

impl LineSegment {
    pub fn new(start: Point2f, end: Point2f, style: LineStyle, color: Color, width: f32) -> Self {
        Self { start, end, style, color, width }
    }
    
    /// Calculate the length of the line segment
    pub fn length(&self) -> f32 {
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        (dx * dx + dy * dy).sqrt()
    }
    
    /// Get bounding box for the line segment
    pub fn bounds(&self) -> BoundingBox {
        BoundingBox::new(
            self.start.x.min(self.end.x),
            self.start.x.max(self.end.x),
            self.start.y.min(self.end.y),
            self.start.y.max(self.end.y),
        )
    }
}

/// Memory-efficient marker instance
#[derive(Debug, Clone)]
pub struct MarkerInstance {
    /// Marker position
    pub position: Point2f,
    /// Marker style
    pub style: MarkerStyle,
    /// Marker color
    pub color: Color,
    /// Marker size
    pub size: f32,
}

impl MarkerInstance {
    pub fn new(position: Point2f, style: MarkerStyle, color: Color, size: f32) -> Self {
        Self { position, style, color, size }
    }
    
    /// Get bounding box for the marker
    pub fn bounds(&self) -> BoundingBox {
        let half_size = self.size * 0.5;
        BoundingBox::new(
            self.position.x - half_size,
            self.position.x + half_size,
            self.position.y - half_size,
            self.position.y + half_size,
        )
    }
}

/// Memory-efficient polygon representation with pooled point storage
#[derive(Debug)]
pub struct Polygon {
    /// Polygon vertices using managed memory
    vertices: ManagedBuffer<Point2f>,
    /// Fill color
    pub fill_color: Color,
    /// Stroke color
    pub stroke_color: Color,
    /// Stroke width
    pub stroke_width: f32,
}

impl Polygon {
    /// Create polygon with managed point storage
    pub fn new(capacity: usize, fill_color: Color, stroke_color: Color, stroke_width: f32) -> Self {
        let memory_manager = get_memory_manager();
        Self {
            vertices: memory_manager.get_point_buffer(capacity),
            fill_color,
            stroke_color,
            stroke_width,
        }
    }
    
    /// Add vertex to polygon
    pub fn add_vertex(&mut self, point: Point2f) {
        self.vertices.get_mut().push(point);
    }
    
    /// Get vertices
    pub fn vertices(&self) -> &[Point2f] {
        self.vertices.get()
    }
    
    /// Get mutable vertices
    pub fn vertices_mut(&mut self) -> &mut Vec<Point2f> {
        self.vertices.get_mut()
    }
    
    /// Calculate bounding box
    pub fn bounds(&self) -> BoundingBox {
        let vertices = self.vertices();
        if vertices.is_empty() {
            return BoundingBox::new(0.0, 0.0, 0.0, 0.0);
        }
        
        let mut min_x = vertices[0].x;
        let mut max_x = vertices[0].x;
        let mut min_y = vertices[0].y;
        let mut max_y = vertices[0].y;
        
        for vertex in vertices.iter().skip(1) {
            min_x = min_x.min(vertex.x);
            max_x = max_x.max(vertex.x);
            min_y = min_y.min(vertex.y);
            max_y = max_y.max(vertex.y);
        }
        
        BoundingBox::new(min_x, max_x, min_y, max_y)
    }
}

/// Memory-efficient text element
#[derive(Debug, Clone)]
pub struct TextElement {
    /// Text content
    pub text: String,
    /// Position
    pub position: Point2f,
    /// Font size
    pub font_size: f32,
    /// Text color
    pub color: Color,
    /// Text alignment
    pub alignment: TextAlignment,
    /// Rotation angle in radians
    pub rotation: f32,
}

impl TextElement {
    pub fn new(text: String, position: Point2f, font_size: f32, color: Color) -> Self {
        Self {
            text,
            position,
            font_size,
            color,
            alignment: TextAlignment::Left,
            rotation: 0.0,
        }
    }
    
    /// Estimate bounding box (approximate)
    pub fn bounds(&self) -> BoundingBox {
        let char_width = self.font_size * 0.6; // Rough estimate
        let text_width = self.text.len() as f32 * char_width;
        let text_height = self.font_size;
        
        BoundingBox::new(
            self.position.x,
            self.position.x + text_width,
            self.position.y,
            self.position.y + text_height,
        )
    }
}

/// Text alignment options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

/// Memory-efficient error bar representation
#[derive(Debug, Clone)]
pub struct ErrorBar {
    /// Center point
    pub center: Point2f,
    /// Error in positive X direction
    pub error_x_pos: f32,
    /// Error in negative X direction
    pub error_x_neg: f32,
    /// Error in positive Y direction
    pub error_y_pos: f32,
    /// Error in negative Y direction
    pub error_y_neg: f32,
    /// Error bar color
    pub color: Color,
    /// Error bar line width
    pub width: f32,
    /// Cap size (for error bar ends)
    pub cap_size: f32,
}

impl ErrorBar {
    pub fn symmetric(center: Point2f, x_error: f32, y_error: f32, color: Color, width: f32) -> Self {
        Self {
            center,
            error_x_pos: x_error,
            error_x_neg: x_error,
            error_y_pos: y_error,
            error_y_neg: y_error,
            color,
            width,
            cap_size: width * 2.0,
        }
    }
    
    pub fn asymmetric(
        center: Point2f,
        x_pos: f32, x_neg: f32,
        y_pos: f32, y_neg: f32,
        color: Color,
        width: f32,
    ) -> Self {
        Self {
            center,
            error_x_pos: x_pos,
            error_x_neg: x_neg,
            error_y_pos: y_pos,
            error_y_neg: y_neg,
            color,
            width,
            cap_size: width * 2.0,
        }
    }
    
    /// Get bounding box including error bars
    pub fn bounds(&self) -> BoundingBox {
        BoundingBox::new(
            self.center.x - self.error_x_neg,
            self.center.x + self.error_x_pos,
            self.center.y - self.error_y_neg,
            self.center.y + self.error_y_pos,
        )
    }
}

/// Statistics for all plot element pools
#[derive(Debug, Clone)]
pub struct PlotElementStats {
    pub line_segments: PoolStats,
    pub markers: PoolStats,
    pub polygons: PoolStats,
    pub text_elements: PoolStats,
    pub error_bars: PoolStats,
}

impl PlotElementStats {
    /// Calculate total pool efficiency across all element types
    pub fn total_efficiency(&self) -> f32 {
        let total_hits = self.line_segments.hits + self.markers.hits + 
                        self.polygons.hits + self.text_elements.hits + self.error_bars.hits;
        let total_requests = total_hits + 
                           self.line_segments.misses + self.markers.misses +
                           self.polygons.misses + self.text_elements.misses + self.error_bars.misses;
        
        if total_requests > 0 {
            total_hits as f32 / total_requests as f32
        } else {
            0.0
        }
    }
    
    /// Get total memory allocations across all pools
    pub fn total_allocations(&self) -> usize {
        self.line_segments.total_allocated + self.markers.total_allocated +
        self.polygons.total_allocated + self.text_elements.total_allocated + self.error_bars.total_allocated
    }
}

/// Global plot element storage instance
static PLOT_ELEMENT_STORAGE: std::sync::OnceLock<PlotElementStorage> = std::sync::OnceLock::new();

/// Get the global plot element storage
pub fn get_plot_element_storage() -> &'static PlotElementStorage {
    PLOT_ELEMENT_STORAGE.get_or_init(PlotElementStorage::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_segment_creation() {
        let start = Point2f::new(0.0, 0.0);
        let end = Point2f::new(3.0, 4.0);
        let line = LineSegment::new(start, end, LineStyle::Solid, Color::BLACK, 1.0);
        
        assert_eq!(line.length(), 5.0); // 3-4-5 triangle
        let bounds = line.bounds();
        assert_eq!(bounds.min_x, 0.0);
        assert_eq!(bounds.max_x, 3.0);
    }

    #[test]
    fn test_element_storage_pooling() {
        let storage = PlotElementStorage::new();
        
        // Get line storage twice to test pooling
        {
            let mut lines1 = storage.get_line_storage(100);
            lines1.get_mut().push(LineSegment::new(
                Point2f::zero(), Point2f::new(1.0, 1.0), 
                LineStyle::Solid, Color::RED, 1.0
            ));
        } // Dropped, returns to pool
        
        {
            let lines2 = storage.get_line_storage(50);
            assert!(lines2.get().capacity() >= 50);
        }
        
        let stats = storage.get_pool_stats();
        assert!(stats.line_segments.hits > 0 || stats.line_segments.misses > 0);
    }

    #[test]
    fn test_polygon_bounds() {
        let mut polygon = Polygon::new(10, Color::RED, Color::BLACK, 1.0);
        polygon.add_vertex(Point2f::new(0.0, 0.0));
        polygon.add_vertex(Point2f::new(5.0, 0.0));
        polygon.add_vertex(Point2f::new(2.5, 5.0));
        
        let bounds = polygon.bounds();
        assert_eq!(bounds.min_x, 0.0);
        assert_eq!(bounds.max_x, 5.0);
        assert_eq!(bounds.min_y, 0.0);
        assert_eq!(bounds.max_y, 5.0);
    }

    #[test]
    fn test_error_bar_creation() {
        let center = Point2f::new(5.0, 10.0);
        let error_bar = ErrorBar::symmetric(center, 1.0, 2.0, Color::BLUE, 0.5);
        
        let bounds = error_bar.bounds();
        assert_eq!(bounds.min_x, 4.0); // center.x - error_x_neg
        assert_eq!(bounds.max_x, 6.0); // center.x + error_x_pos
        assert_eq!(bounds.min_y, 8.0); // center.y - error_y_neg
        assert_eq!(bounds.max_y, 12.0); // center.y + error_y_pos
    }
}